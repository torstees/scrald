//! YAML front matter: finding it, and reading it into typed properties
//! (DESIGN.md §8.1). Editing is in `yaml_edit.rs` (§8.3) and never re-serializes.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::source::SourceRange;
use crate::yaml_edit::{FieldInfo, field_info};

/// Where the front matter block sits in a document's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatterSplit {
    /// The YAML between the fences.
    pub yaml: std::ops::Range<usize>,
    /// The whole block, from the opening `---` through the closing fence's
    /// line ending. The Markdown body starts at `block.end`.
    pub block: std::ops::Range<usize>,
}

/// Finds a front matter block: `---` on the very first line, closed by a line
/// that is exactly `---` or `...` (trailing spaces allowed). Without a closing
/// fence there is no front matter, and the `---` is ordinary Markdown.
pub fn split(text: &str) -> Option<FrontMatterSplit> {
    let first_line_end = text.find('\n')?;
    if trim_line(&text[..first_line_end]) != "---" {
        return None;
    }
    let yaml_start = first_line_end + 1;
    let mut line_start = yaml_start;
    while line_start <= text.len() {
        // Rust note: `map_or(default, f)` unwraps an Option, applying `f` if
        // present: here "end of this line, or end of text if no more '\n'".
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |i| line_start + i);
        let line = trim_line(&text[line_start..line_end]);
        if line == "---" || line == "..." {
            let block_end = (line_end + 1).min(text.len());
            return Some(FrontMatterSplit {
                yaml: yaml_start..line_start,
                block: 0..block_end,
            });
        }
        if line_end == text.len() {
            break;
        }
        line_start = line_end + 1;
    }
    None
}

/// A line without its `\r` (from CRLF) or trailing spaces and tabs.
fn trim_line(line: &str) -> &str {
    line.trim_end_matches(['\r', ' ', '\t'])
}

/// Front matter properties, with the aliases from DESIGN.md §8.1 resolved.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontMatter {
    /// The whole block, fences included, in original-file byte offsets.
    pub range: SourceRange,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub source: Option<String>,
    pub assets: Option<String>,
    /// `scrald-theme` (or nested `scrald.theme`).
    pub theme: Option<String>,
    /// `scrald-flavor` (or nested `scrald.flavor`).
    pub flavor: Option<String>,
    /// Every key not consumed above, in file order, so nothing is hidden.
    pub extra: Map<String, Value>,
    /// The key each property above was read from (`author` or `authors`,
    /// say), so an edit changes that key rather than adding another.
    pub keys: PropertyKeys,
    /// Every top-level key, in file order, and whether the properties panel
    /// can edit it (DESIGN.md §8.3).
    pub fields: Vec<FieldInfo>,
    /// Set when the YAML couldn't be read. The document still opens.
    pub error: Option<String>,
}

/// For each recognized property, the key it was read from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyKeys {
    pub title: Option<String>,
    pub authors: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<String>,
    pub notes: Option<String>,
    pub source: Option<String>,
    pub assets: Option<String>,
    pub theme: Option<String>,
    pub flavor: Option<String>,
}

/// Recognized keys and their aliases, in priority order. When several
/// aliases are present, the first one listed wins and the rest stay in `extra`.
const TITLE: &[&str] = &["title"];
const AUTHORS: &[&str] = &["author", "authors"];
const SUMMARY: &[&str] = &["summary", "synopsis", "description", "abstract"];
const TAGS: &[&str] = &["tags", "tag", "keywords"];
const NOTES: &[&str] = &["notes"];
const SOURCE: &[&str] = &["source", "url"];
const ASSETS: &[&str] = &["assets", "asset_dir", "attachments"];
const THEME: &[&str] = &["scrald-theme"];
const FLAVOR: &[&str] = &["scrald-flavor"];

/// Parses the YAML text of a front matter block. `range` is the block's
/// position in the original file.
pub fn parse(yaml: &str, range: SourceRange) -> FrontMatter {
    let mut fm = FrontMatter {
        range,
        fields: field_info(yaml),
        ..FrontMatter::default()
    };

    // An empty block (`---` immediately followed by `---`) is valid and empty.
    if yaml.trim().is_empty() {
        return fm;
    }

    let mut map = match serde_saphyr::from_str::<Value>(yaml) {
        Ok(Value::Object(map)) => map,
        Ok(Value::Null) => return fm,
        Ok(_) => {
            fm.error = Some("front matter is not a list of key: value pairs".to_string());
            return fm;
        }
        Err(e) => {
            fm.error = Some(e.to_string());
            // YAML that doesn't parse can't be edited safely anywhere.
            for field in &mut fm.fields {
                field.editable = false;
                field.reason = Some("the front matter has a YAML error".to_string());
            }
            return fm;
        }
    };

    // Rust note: `.unzip()` splits an iterator of pairs into two values:
    // here each `Option<(value, key)>` becomes the value and the key's name.
    (fm.title, fm.keys.title) = take_string(&mut map, TITLE).unzip();
    let (authors, authors_key) = take_list(&mut map, AUTHORS).unzip();
    fm.authors = authors.unwrap_or_default();
    fm.keys.authors = authors_key;
    (fm.summary, fm.keys.summary) = take_string(&mut map, SUMMARY).unzip();
    let (tags, tags_key) = take_list(&mut map, TAGS).unzip();
    fm.tags = tags
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.trim_start_matches('#').to_string())
        .filter(|t| !t.is_empty())
        .collect();
    fm.keys.tags = tags_key;
    (fm.notes, fm.keys.notes) = take_string(&mut map, NOTES).unzip();
    (fm.source, fm.keys.source) = take_string(&mut map, SOURCE).unzip();
    (fm.assets, fm.keys.assets) = take_string(&mut map, ASSETS).unzip();
    (fm.theme, fm.keys.theme) = take_string(&mut map, THEME).unzip();
    (fm.flavor, fm.keys.flavor) = take_string(&mut map, FLAVOR).unzip();

    // Lenient read of the nested form `scrald: { theme, flavor }`. The flat
    // keys win, and the `scrald` map itself stays in `extra` untouched.
    if let Some(Value::Object(nested)) = map.get("scrald") {
        if fm.theme.is_none() {
            fm.theme = nested.get("theme").and_then(scalar_to_string);
        }
        if fm.flavor.is_none() {
            fm.flavor = nested.get("flavor").and_then(scalar_to_string);
        }
    }

    fm.extra = map;
    fm
}

/// Removes and returns the first alias holding a scalar, and that alias. A
/// value of the wrong shape (say, a map under `title`) is left in place so
/// it shows in `extra`.
// Rust note: `&mut Map` is an exclusive (mutable) borrow: we can change the
// caller's map, and nothing else can touch it while we hold the borrow.
fn take_string(map: &mut Map<String, Value>, keys: &[&str]) -> Option<(String, String)> {
    for key in keys {
        if let Some(text) = map.get(*key).and_then(scalar_to_string) {
            map.shift_remove(*key);
            return Some((text, key.to_string()));
        }
    }
    None
}

/// Like `take_string`, but accepts a list of scalars or a single scalar. A
/// single string is split on commas, so `tags: a, b` gives two tags.
fn take_list(map: &mut Map<String, Value>, keys: &[&str]) -> Option<(Vec<String>, String)> {
    for key in keys {
        let items = match map.get(*key) {
            Some(Value::Array(values)) if values.iter().all(is_scalar) => values
                .iter()
                .filter_map(scalar_to_string)
                .collect::<Vec<_>>(),
            Some(value) => match scalar_to_string(value) {
                Some(text) => text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                None => continue,
            },
            None => continue,
        };
        map.shift_remove(*key);
        return Some((items, key.to_string()));
    }
    None
}

fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

/// Strings, numbers, and booleans as text. Null, lists, and maps give `None`.
fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(yaml: &str) -> FrontMatter {
        parse(yaml, SourceRange::default())
    }

    #[test]
    fn split_finds_block_and_body() {
        let text = "---\ntitle: T\n---\n# Body\n";
        let s = split(text).unwrap();
        assert_eq!(&text[s.yaml.clone()], "title: T\n");
        assert_eq!(&text[s.block.end..], "# Body\n");
    }

    #[test]
    fn split_handles_crlf_and_dots_closer() {
        let text = "---\r\ntitle: T\r\n...\r\nbody";
        let s = split(text).unwrap();
        assert_eq!(&text[s.yaml.clone()], "title: T\r\n");
        assert_eq!(&text[s.block.end..], "body");
    }

    #[test]
    fn split_closing_fence_at_end_of_file() {
        let text = "---\na: 1\n---";
        let s = split(text).unwrap();
        assert_eq!(s.block, 0..text.len());
    }

    #[test]
    fn split_requires_fence_on_first_line_and_a_closer() {
        assert_eq!(split("# Title\n---\na: 1\n---\n"), None);
        assert_eq!(split("---\na: 1\nno closer\n"), None);
        assert_eq!(split("----\na: 1\n----\n"), None);
        assert_eq!(split("---"), None);
    }

    #[test]
    fn split_empty_block() {
        let s = split("---\n---\nbody").unwrap();
        assert!(s.yaml.is_empty());
    }

    #[test]
    fn recognized_keys_and_aliases() {
        let fm = parse_str(
            "title: The Long Winter\nauthors: [A. Writer, B. Writer]\nsynopsis: Cold.\n\
             keywords: [novel, '#draft']\nnotes: Fix ch. 12\nurl: https://example.com\n\
             attachments: ../img\n",
        );
        assert_eq!(fm.title.as_deref(), Some("The Long Winter"));
        assert_eq!(fm.authors, ["A. Writer", "B. Writer"]);
        assert_eq!(fm.summary.as_deref(), Some("Cold."));
        assert_eq!(fm.tags, ["novel", "draft"]);
        assert_eq!(fm.notes.as_deref(), Some("Fix ch. 12"));
        assert_eq!(fm.source.as_deref(), Some("https://example.com"));
        assert_eq!(fm.assets.as_deref(), Some("../img"));
        assert!(fm.extra.is_empty());
        assert_eq!(fm.error, None);
    }

    #[test]
    fn single_string_lists_split_on_commas() {
        let fm = parse_str("author: Solo\ntags: a, b ,c\n");
        assert_eq!(fm.authors, ["Solo"]);
        assert_eq!(fm.tags, ["a", "b", "c"]);
    }

    #[test]
    fn first_alias_wins_and_others_are_kept() {
        let fm = parse_str("description: second\nsummary: first\n");
        assert_eq!(fm.summary.as_deref(), Some("first"));
        assert_eq!(fm.extra.get("description"), Some(&Value::from("second")));
    }

    #[test]
    fn unknown_keys_preserved_in_order() {
        let fm = parse_str("zeta: 1\ntitle: T\nalpha: [x]\ncreated: 2024-01-02\n");
        let keys: Vec<&String> = fm.extra.keys().collect();
        assert_eq!(keys, ["zeta", "alpha", "created"]);
    }

    #[test]
    fn wrong_shape_stays_in_extra() {
        let fm = parse_str("title:\n  nested: map\n");
        assert_eq!(fm.title, None);
        assert!(fm.extra.contains_key("title"));
    }

    #[test]
    fn flat_scrald_keys() {
        let fm = parse_str("scrald-theme: nordic-night\nscrald-flavor: obsidian\n");
        assert_eq!(fm.theme.as_deref(), Some("nordic-night"));
        assert_eq!(fm.flavor.as_deref(), Some("obsidian"));
    }

    #[test]
    fn nested_scrald_keys_are_read() {
        let fm = parse_str("scrald:\n  theme: sepia\n  flavor: pandoc\n");
        assert_eq!(fm.theme.as_deref(), Some("sepia"));
        assert_eq!(fm.flavor.as_deref(), Some("pandoc"));
        // The nested map is left as-is for display and minimal editing.
        assert!(fm.extra.contains_key("scrald"));
    }

    #[test]
    fn flat_scrald_keys_win_over_nested() {
        let fm = parse_str("scrald:\n  theme: sepia\n  flavor: pandoc\nscrald-theme: dark\n");
        assert_eq!(fm.theme.as_deref(), Some("dark"));
        assert_eq!(fm.flavor.as_deref(), Some("pandoc"));
    }

    #[test]
    fn invalid_yaml_reports_error_without_failing() {
        let fm = parse_str("title: [unclosed\n");
        assert!(fm.error.is_some());
        assert_eq!(fm.title, None);
    }

    #[test]
    fn non_map_yaml_reports_error() {
        let fm = parse_str("- just\n- a list\n");
        assert!(fm.error.is_some());
    }

    #[test]
    fn empty_yaml_is_empty() {
        let fm = parse_str("\n");
        assert_eq!(fm, FrontMatter::default());
    }
}
