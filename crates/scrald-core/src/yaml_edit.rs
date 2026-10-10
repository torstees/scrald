//! Minimal text edits to YAML front matter (DESIGN.md §8.3). Re-serializing
//! YAML through a parser loses comments, key order, and quoting, so instead
//! each edit finds one top-level key's lines and rewrites only its value,
//! keeping the key's existing style: plain or quoted scalars, block scalars
//! (`|`), flow lists (`[a, b]`), and block lists (`- a`). Anything harder to
//! edit safely (nested maps, anchors, multi-line flow) is reported read-only.

use serde::Serialize;

// --- Scanning -------------------------------------------------------------------

/// How a top-level key's value is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueStyle {
    /// `key:` with nothing after it.
    Empty,
    Plain,
    SingleQuoted,
    DoubleQuoted,
    /// `key: |` or `key: >`, with the header as written (`|-`, `>+`, ...).
    BlockScalar {
        header: String,
    },
    /// `key: [a, b]` on one line.
    FlowList,
    /// `key:` followed by `- item` lines, indented by `indent` spaces.
    BlockList {
        indent: usize,
    },
    /// Not safe to edit; the reason is shown to the user.
    ReadOnly(String),
}

/// One top-level `key: value` entry. Offsets are bytes into the YAML text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    /// Start of the key's line.
    pub start: usize,
    /// Just after `key:` on that line.
    pub after_colon: usize,
    /// End of the entry's last line, before its line break.
    pub end: usize,
    /// A trailing ` # comment` on the key's line (with its leading spaces).
    pub comment: String,
    pub style: ValueStyle,
}

/// Finds every top-level entry, in order. Lines that aren't part of an entry
/// (blank lines, full-line comments) are skipped.
pub fn scan(yaml: &str) -> Vec<Entry> {
    let lines = lines_with_offsets(yaml);
    let mut entries: Vec<Entry> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let (start, line) = lines[i];
        let Some((key, colon)) = top_level_key(line) else {
            i += 1;
            continue;
        };
        // Continuation lines: indented, or `-` items, until the next key.
        let mut j = i + 1;
        let mut last_content = i;
        while j < lines.len() {
            let next = lines[j].1;
            let trimmed = next.trim();
            if trimmed.is_empty() {
                j += 1;
                continue;
            }
            let indented = next.starts_with([' ', '\t']);
            let dash_item = next.starts_with('-') && !next.starts_with("---");
            if !(indented || dash_item) {
                break;
            }
            last_content = j;
            j += 1;
        }
        let (last_start, last_line) = lines[last_content];
        let end = last_start + last_line.len();

        let rest = &line[colon + 1..];
        let (value, comment) = split_comment(rest);
        let continuation: Vec<&str> = lines[i + 1..=last_content]
            .iter()
            .map(|(_, l)| *l)
            .filter(|l| !l.trim().is_empty())
            .collect();
        let style = classify(value.trim(), &continuation);
        entries.push(Entry {
            key,
            start,
            after_colon: start + colon + 1,
            end,
            comment: comment.to_string(),
            style,
        });
        i = last_content + 1;
    }

    // Duplicate keys: which one a parser keeps is unclear, so edit neither.
    let keys: Vec<String> = entries.iter().map(|e| e.key.clone()).collect();
    for entry in &mut entries {
        if keys.iter().filter(|k| **k == entry.key).count() > 1 {
            entry.style = ValueStyle::ReadOnly("the key appears more than once".to_string());
        }
    }
    entries
}

/// Each line (without its line break, `\r` included in the break) and the
/// byte offset where it starts.
fn lines_with_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut offset = 0;
    for raw in text.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\n', '\r']);
        out.push((offset, line));
        offset += raw.len();
    }
    out
}

/// `key: ...` at the start of a line: the key and the index of its colon.
/// Quoted keys (`"my key": x`) are accepted; comments, list items, and
/// indented lines are not keys.
fn top_level_key(line: &str) -> Option<(String, usize)> {
    let first = line.chars().next()?;
    if first.is_whitespace()
        || matches!(
            first,
            '#' | '-' | '[' | '{' | '?' | '&' | '*' | '!' | '|' | '>'
        )
    {
        return None;
    }
    let (key, after_key) = if first == '"' || first == '\'' {
        let close = line[1..].find(first)? + 1;
        (line[1..close].to_string(), close + 1)
    } else {
        // The colon must be followed by a space or the end of the line.
        // Rust note: `is_none_or(f)` is true for `None`, or when `f` holds
        // for the value inside `Some`.
        let mut found = None;
        for (i, c) in line.char_indices() {
            if c == ':'
                && line[i + 1..]
                    .chars()
                    .next()
                    .is_none_or(|n| n == ' ' || n == '\t')
            {
                found = Some(i);
                break;
            }
        }
        let colon = found?;
        (line[..colon].trim_end().to_string(), colon)
    };
    let colon = after_key + line[after_key..].find(':')?;
    // Only spaces may sit between a quoted key and its colon.
    if !line[after_key..colon].trim().is_empty() || key.is_empty() {
        return None;
    }
    Some((key, colon))
}

/// Splits a value from a trailing ` # comment`, ignoring `#` inside quotes.
fn split_comment(rest: &str) -> (&str, &str) {
    let mut quote: Option<char> = None;
    let mut prev = ' ';
    for (i, c) in rest.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if (c == '"' || c == '\'') && prev.is_whitespace() => quote = Some(c),
            None if c == '#' && prev.is_whitespace() => {
                let value = rest[..i].trim_end();
                let comment_start = value.len();
                return (&rest[..comment_start], &rest[comment_start..]);
            }
            None => {}
        }
        prev = c;
    }
    (rest, "")
}

fn read_only(reason: &str) -> ValueStyle {
    ValueStyle::ReadOnly(reason.to_string())
}

/// Classifies a value from the text after `key:` and its continuation lines.
fn classify(value: &str, continuation: &[&str]) -> ValueStyle {
    if value.starts_with('&') || value.starts_with('*') {
        return read_only("it uses a YAML anchor or alias");
    }
    if value.starts_with('!') {
        return read_only("it has a YAML type tag");
    }
    if value.starts_with('|') || value.starts_with('>') {
        let indicator_ok = value[1..]
            .chars()
            .all(|c| matches!(c, '-' | '+' | '1'..='9'));
        return if indicator_ok {
            ValueStyle::BlockScalar {
                header: value.to_string(),
            }
        } else {
            read_only("its block scalar header isn't recognized")
        };
    }
    if value.is_empty() {
        return match continuation.first() {
            None => ValueStyle::Empty,
            Some(first) if first.trim_start().starts_with('-') => block_list_style(continuation),
            Some(_) => read_only("it's a nested map"),
        };
    }
    if !continuation.is_empty() {
        return read_only("its value spans several lines");
    }
    if value.starts_with('[') {
        return if value.ends_with(']') && flow_items(value).is_some() {
            ValueStyle::FlowList
        } else {
            read_only("it's a list written in a form Scrald can't edit")
        };
    }
    if value.starts_with('{') {
        return read_only("it's an inline map");
    }
    if value.starts_with('\'') {
        return if value.len() >= 2 && value.ends_with('\'') {
            ValueStyle::SingleQuoted
        } else {
            read_only("its quoting isn't closed on the line")
        };
    }
    if value.starts_with('"') {
        return if value.len() >= 2 && value.ends_with('"') && !value.ends_with("\\\"") {
            ValueStyle::DoubleQuoted
        } else {
            read_only("its quoting isn't closed on the line")
        };
    }
    ValueStyle::Plain
}

/// A block list is editable when every item is `- scalar` at one indent.
fn block_list_style(continuation: &[&str]) -> ValueStyle {
    let indent = continuation[0].len() - continuation[0].trim_start().len();
    for line in continuation {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let this_indent = line.len() - trimmed.len();
        let Some(item) = trimmed.strip_prefix('-') else {
            return read_only("a list item spans several lines");
        };
        if this_indent != indent || !(item.is_empty() || item.starts_with(' ')) {
            return read_only("its list items are nested or uneven");
        }
        let (value, _) = split_comment(item);
        let value = value.trim();
        if value.starts_with(['-', '[', '{', '&', '*', '!', '|', '>']) || looks_like_map_item(value)
        {
            return read_only("its list items aren't simple values");
        }
    }
    ValueStyle::BlockList { indent }
}

fn looks_like_map_item(value: &str) -> bool {
    !value.starts_with(['"', '\'']) && (value.contains(": ") || value.ends_with(':'))
}

/// The items of a one-line flow list, unquoted; `None` if it nests.
fn flow_items(value: &str) -> Option<Vec<String>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in inner.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None => match c {
                '"' | '\'' if current.trim().is_empty() => {
                    current.clear();
                    quote = Some(c);
                }
                ',' => items.push(std::mem::take(&mut current).trim().to_string()),
                '[' | ']' | '{' | '}' => return None,
                _ => current.push(c),
            },
        }
    }
    if quote.is_some() {
        return None;
    }
    if !current.trim().is_empty() || !items.is_empty() {
        items.push(current.trim().to_string());
    }
    Some(items)
}

// --- What the panel may edit ----------------------------------------------------

/// Whether a key can be edited, for the properties panel (§8.2–8.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldInfo {
    pub key: String,
    pub editable: bool,
    /// Why not, when it isn't (shown with an "edit in raw mode" link).
    pub reason: Option<String>,
}

/// Editability of every top-level key, in file order.
pub fn field_info(yaml: &str) -> Vec<FieldInfo> {
    scan(yaml)
        .into_iter()
        .map(|e| match e.style {
            ValueStyle::ReadOnly(reason) => FieldInfo {
                key: e.key,
                editable: false,
                reason: Some(reason),
            },
            _ => FieldInfo {
                key: e.key,
                editable: true,
                reason: None,
            },
        })
        .collect()
}

// --- Editing --------------------------------------------------------------------

/// Why an edit wasn't made.
// Rust note: `thiserror::Error` derives the standard `Error` trait; each
// `#[error(...)]` is the variant's message, like `__str__` in Python.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("`{key}` can't be edited here because {reason}; edit it in raw mode")]
    ReadOnly { key: String, reason: String },
    #[error("`{0}` isn't a valid key")]
    InvalidKey(String),
}

/// A new value for a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A text value; text with line breaks is written as a `|` block.
    Text(String),
    List(Vec<String>),
    Remove,
}

/// Applies `change` to `key` in a front matter's YAML text, touching only
/// that key's lines. A missing key is added at the end (lists as block
/// lists, the form Obsidian writes). `line_ending` is used for new lines.
pub fn edit(
    yaml: &str,
    key: &str,
    change: &Change,
    line_ending: &str,
) -> Result<String, EditError> {
    if key.is_empty() || key.contains(['\n', '\r']) || key.starts_with(['#', '-', ' ']) {
        return Err(EditError::InvalidKey(key.to_string()));
    }
    let entries = scan(yaml);
    let Some(entry) = entries.iter().find(|e| e.key == key) else {
        return Ok(add_entry(yaml, key, change, line_ending));
    };
    if let ValueStyle::ReadOnly(reason) = &entry.style {
        return Err(EditError::ReadOnly {
            key: key.to_string(),
            reason: reason.clone(),
        });
    }

    if *change == Change::Remove {
        // The whole entry, with its line break.
        let end = line_break_end(yaml, entry.end);
        let mut out = yaml.to_string();
        out.replace_range(entry.start..end, "");
        return Ok(out);
    }

    let value = match change {
        Change::Text(text) => text_value(text, &entry.style, &entry.comment, line_ending),
        Change::List(items) => list_value(items, &entry.style, &entry.comment, line_ending),
        Change::Remove => unreachable!("handled above"),
    };
    let mut out = yaml.to_string();
    out.replace_range(entry.after_colon..entry.end, &value);
    Ok(out)
}

/// The offset just past the line break that follows `end` (if any).
fn line_break_end(text: &str, end: usize) -> usize {
    let rest = &text[end..];
    if rest.starts_with("\r\n") {
        end + 2
    } else if rest.starts_with('\n') {
        end + 1
    } else {
        end
    }
}

fn add_entry(yaml: &str, key: &str, change: &Change, line_ending: &str) -> String {
    if *change == Change::Remove {
        return yaml.to_string();
    }
    let key_text = if needs_quotes(key, false) {
        double_quoted(key)
    } else {
        key.to_string()
    };
    let value = match change {
        Change::Text(text) => text_value(text, &ValueStyle::Empty, "", line_ending),
        Change::List(items) => {
            list_value(items, &ValueStyle::BlockList { indent: 2 }, "", line_ending)
        }
        Change::Remove => String::new(),
    };
    let mut out = yaml.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(line_ending);
    }
    out.push_str(&format!("{key_text}:{value}{line_ending}"));
    out
}

/// The text after `key:` for a text value, keeping the existing style.
fn text_value(text: &str, style: &ValueStyle, comment: &str, line_ending: &str) -> String {
    if text.contains('\n') {
        // A literal block keeps line breaks exactly; `|-` drops the final one.
        let body = text.strip_suffix('\n').unwrap_or(text);
        let header = if text.ends_with('\n') { "|" } else { "|-" };
        let mut out = format!(" {header}{comment}");
        for line in body.split('\n') {
            out.push_str(line_ending);
            if !line.is_empty() {
                out.push_str("  ");
                out.push_str(line.trim_end_matches('\r'));
            }
        }
        return out;
    }
    let scalar = match style {
        ValueStyle::SingleQuoted => format!("'{}'", text.replace('\'', "''")),
        ValueStyle::DoubleQuoted => double_quoted(text),
        _ if text.is_empty() => String::new(),
        _ if needs_quotes(text, false) => double_quoted(text),
        _ => text.to_string(),
    };
    if scalar.is_empty() {
        comment.to_string()
    } else {
        format!(" {scalar}{comment}")
    }
}

/// The text after `key:` for a list, keeping the existing style.
fn list_value(items: &[String], style: &ValueStyle, comment: &str, line_ending: &str) -> String {
    match style {
        ValueStyle::BlockList { indent } if !items.is_empty() => {
            let pad = " ".repeat(*indent);
            let mut out = comment.to_string();
            for item in items {
                out.push_str(line_ending);
                out.push_str(&format!("{pad}- {}", list_item(item, false)));
            }
            out
        }
        // A single plain value stays a single value.
        ValueStyle::Plain | ValueStyle::SingleQuoted | ValueStyle::DoubleQuoted
            if items.len() == 1 =>
        {
            text_value(&items[0], style, comment, line_ending)
        }
        _ => {
            let inner: Vec<String> = items.iter().map(|i| list_item(i, true)).collect();
            format!(" [{}]{comment}", inner.join(", "))
        }
    }
}

fn list_item(item: &str, in_flow: bool) -> String {
    let item = item.replace(['\n', '\r'], " ");
    if needs_quotes(&item, in_flow) {
        double_quoted(&item)
    } else {
        item
    }
}

/// Whether a scalar must be quoted to stay the same string: empty, padded,
/// starting with a YAML indicator, containing `: ` or ` #`, or reading as
/// another type (`true`, `null`, `2024`). Inside flow lists, `,[]{}` too.
pub fn needs_quotes(text: &str, in_flow: bool) -> bool {
    if text.is_empty() || text != text.trim() {
        return true;
    }
    let first = text.chars().next().unwrap_or(' ');
    if matches!(
        first,
        '-' | '?'
            | ':'
            | ','
            | '['
            | ']'
            | '{'
            | '}'
            | '#'
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
    ) {
        return true;
    }
    if text.contains(": ") || text.contains(" #") || text.ends_with(':') || text.contains('\t') {
        return true;
    }
    if in_flow && text.contains([',', '[', ']', '{', '}']) {
        return true;
    }
    // Would it read as a boolean, null, or number instead of a string?
    let lower = text.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off" | "null" | "~" | "y" | "n"
    ) || text.parse::<f64>().is_ok()
        || lower.starts_with("0x")
        || lower.starts_with("0o")
}

fn double_quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// --- Whole documents ------------------------------------------------------------

/// Applies a change to a document's front matter and returns the new
/// document text. A document without front matter gets a new block at the
/// top. The line ending for new lines follows the document.
pub fn edit_document(text: &str, key: &str, change: &Change) -> Result<String, EditError> {
    let line_ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
    match crate::frontmatter::split(text) {
        Some(split) => {
            let yaml = &text[split.yaml.clone()];
            let edited = edit(yaml, key, change, line_ending)?;
            let mut out = String::with_capacity(text.len() + edited.len());
            out.push_str(&text[..split.yaml.start]);
            out.push_str(&edited);
            out.push_str(&text[split.yaml.end..]);
            Ok(out)
        }
        None => {
            if *change == Change::Remove {
                return Ok(text.to_string());
            }
            let yaml = edit("", key, change, line_ending)?;
            Ok(format!("---{line_ending}{yaml}---{line_ending}{text}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LF: &str = "\n";

    fn set(yaml: &str, key: &str, change: Change) -> String {
        edit(yaml, key, &change, LF).unwrap()
    }

    fn text(s: &str) -> Change {
        Change::Text(s.to_string())
    }

    fn list(items: &[&str]) -> Change {
        Change::List(items.iter().map(|s| s.to_string()).collect())
    }

    const SAMPLE: &str = "\
# Document settings
title: The Long Winter   # working title
author: 'A. Writer'
summary: \"A family survives.\"
tags: [novel, draft]
keywords:
  - norse
  - winter
notes: |
  Chapter 12 needs a rewrite.
  Also chapter 3.

scrald-theme: nordic-night
";

    #[test]
    fn scan_finds_entries_and_styles() {
        let styles: Vec<(String, ValueStyle)> =
            scan(SAMPLE).into_iter().map(|e| (e.key, e.style)).collect();
        assert_eq!(
            styles,
            vec![
                ("title".to_string(), ValueStyle::Plain),
                ("author".to_string(), ValueStyle::SingleQuoted),
                ("summary".to_string(), ValueStyle::DoubleQuoted),
                ("tags".to_string(), ValueStyle::FlowList),
                ("keywords".to_string(), ValueStyle::BlockList { indent: 2 }),
                (
                    "notes".to_string(),
                    ValueStyle::BlockScalar {
                        header: "|".to_string()
                    }
                ),
                ("scrald-theme".to_string(), ValueStyle::Plain),
            ]
        );
    }

    #[test]
    fn no_op_edits_round_trip() {
        assert_eq!(set(SAMPLE, "author", text("A. Writer")), SAMPLE);
        assert_eq!(set(SAMPLE, "summary", text("A family survives.")), SAMPLE);
        assert_eq!(set(SAMPLE, "tags", list(&["novel", "draft"])), SAMPLE);
        assert_eq!(set(SAMPLE, "keywords", list(&["norse", "winter"])), SAMPLE);
        assert_eq!(set(SAMPLE, "scrald-theme", text("nordic-night")), SAMPLE);
    }

    #[test]
    fn scalars_keep_their_style_and_comments() {
        let out = set(SAMPLE, "title", text("The Longest Winter"));
        assert!(out.contains("title: The Longest Winter   # working title\n"));
        let out = set(SAMPLE, "author", text("O'Brien"));
        assert!(out.contains("author: 'O''Brien'\n"));
        let out = set(SAMPLE, "summary", text("Say \"hi\""));
        assert!(out.contains("summary: \"Say \\\"hi\\\"\"\n"));
        // Everything else is untouched.
        let before: Vec<&str> = SAMPLE.lines().filter(|l| !l.starts_with("title")).collect();
        // Rust note: the edited String must outlive the `&str` lines borrowed
        // from it, so it gets a name instead of being a temporary.
        let edited = set(SAMPLE, "title", text("X"));
        let after: Vec<&str> = edited.lines().filter(|l| !l.starts_with("title")).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn plain_values_are_quoted_when_needed() {
        for (value, written) in [
            ("2024", "\"2024\""),
            ("true", "\"true\""),
            ("Part 1: Arrival", "\"Part 1: Arrival\""),
            ("#hashtag", "\"#hashtag\""),
            ("- dash", "\"- dash\""),
            ("plain words", "plain words"),
            ("C:\\path", "C:\\path"),
        ] {
            let out = set("title: x\n", "title", text(value));
            assert_eq!(out, format!("title: {written}\n"), "{value}");
        }
    }

    #[test]
    fn lists_keep_their_style() {
        let out = set(SAMPLE, "tags", list(&["novel", "final, really", "2024"]));
        assert!(out.contains("tags: [novel, \"final, really\", \"2024\"]\n"));
        let out = set(SAMPLE, "keywords", list(&["norse", "sea"]));
        assert!(out.contains("keywords:\n  - norse\n  - sea\nnotes: |"));
        let out = set("tags:\n- a\n- b\n", "tags", list(&["c"]));
        assert_eq!(out, "tags:\n- c\n");
        assert_eq!(set("tags: [a]\n", "tags", list(&[])), "tags: []\n");
        assert_eq!(set("tags:\n  - a\n", "tags", list(&[])), "tags: []\n");
        // A single plain value stays plain when it stays single.
        assert_eq!(
            set("tags: novel\n", "tags", list(&["poem"])),
            "tags: poem\n"
        );
        assert_eq!(
            set("tags: novel\n", "tags", list(&["a", "b"])),
            "tags: [a, b]\n"
        );
    }

    #[test]
    fn multi_line_text_becomes_a_literal_block() {
        let out = set(SAMPLE, "notes", text("Line one.\n\nLine three."));
        assert!(out.contains("notes: |-\n  Line one.\n\n  Line three.\n\nscrald-theme"));
        let out = set("notes: short\n", "notes", text("a\nb\n"));
        assert_eq!(out, "notes: |\n  a\n  b\n");
        // And back to one line.
        assert_eq!(
            set("notes: |\n  a\n  b\n", "notes", text("one")),
            "notes: one\n"
        );
    }

    #[test]
    fn missing_keys_are_added_and_keys_removed() {
        assert_eq!(
            set("title: x\n", "scrald-flavor", text("pandoc")),
            "title: x\nscrald-flavor: pandoc\n"
        );
        assert_eq!(
            set("title: x", "tags", list(&["a", "b"])),
            "title: x\ntags:\n  - a\n  - b\n"
        );
        assert!(!set(SAMPLE, "keywords", Change::Remove).contains("norse"));
        assert_eq!(set("a: 1\nb: 2\n", "a", Change::Remove), "b: 2\n");
        assert_eq!(set("a: 1\n", "missing", Change::Remove), "a: 1\n");
        assert_eq!(set("", "title", text("New")), "title: New\n");
    }

    #[test]
    fn crlf_is_kept() {
        let yaml = "title: x\r\ntags:\r\n  - a\r\n";
        let out = edit(yaml, "tags", &list(&["a", "b"]), "\r\n").unwrap();
        assert_eq!(out, "title: x\r\ntags:\r\n  - a\r\n  - b\r\n");
        let out = edit(yaml, "title", &text("z"), "\r\n").unwrap();
        assert_eq!(out, "title: z\r\ntags:\r\n  - a\r\n");
    }

    #[test]
    fn complex_values_are_read_only() {
        let yaml = "\
base: &base x
copy: *base
nested:
  a: 1
inline: {a: 1}
multi: [a,
  b]
tagged: !custom x
dup: 1
dup: 2
deep:
  - - a
";
        for info in field_info(yaml) {
            assert!(!info.editable, "{} should be read-only", info.key);
            assert!(info.reason.is_some());
        }
        let err = edit(yaml, "nested", &text("x"), LF).unwrap_err();
        assert!(matches!(err, EditError::ReadOnly { .. }));
        // Other keys in the same block can still be edited.
        assert!(edit("nested:\n  a: 1\ntitle: x\n", "title", &text("y"), LF).is_ok());
    }

    #[test]
    fn colons_in_values_and_urls() {
        let entries = scan("source: https://example.com/a:b\ntime: 12:30\n");
        assert_eq!(entries[0].key, "source");
        assert_eq!(entries[0].style, ValueStyle::Plain);
        assert_eq!(entries[1].key, "time");
        let out = set(
            "source: https://a.com\n",
            "source",
            text("https://b.com/x?y=1#frag"),
        );
        assert_eq!(out, "source: https://b.com/x?y=1#frag\n");
    }

    /// Tricky values, for checking that edits parse back exactly.
    const TRICKY: &[&str] = &[
        "plain",
        "2024",
        "1e3",
        "true",
        "null",
        "~",
        "yes",
        "Part 1: Arrival",
        "a #not comment",
        "#tag",
        "- dash",
        "it's",
        "say \"hi\"",
        "back\\slash",
        "trailing:",
        " padded ",
        "comma, inside",
        "[brackets]",
        "{braces}",
        "*star",
        "&amp",
        "!bang",
        "%percent",
        "@at",
        "`tick",
        "ünïcödé — ✓",
        "",
    ];

    fn parse(yaml: &str) -> serde_json::Map<String, serde_json::Value> {
        match serde_saphyr::from_str::<serde_json::Value>(yaml) {
            Ok(serde_json::Value::Object(map)) => map,
            other => panic!("edited YAML doesn't parse as a map: {other:?}\n{yaml}"),
        }
    }

    #[test]
    fn edits_parse_back_to_exactly_the_new_value() {
        let styles = [
            "key: old\nother: kept # c\n",
            "key: 'old'\nother: kept # c\n",
            "key: \"old\"\nother: kept # c\n",
            "key: |\n  old\nother: kept # c\n",
            "key:\nother: kept # c\n",
            "other: kept # c\n",
        ];
        for yaml in styles {
            for value in TRICKY {
                let out = set(yaml, "key", text(value));
                let map = parse(&out);
                let got = map.get("key");
                let expected = if value.is_empty() {
                    // An empty value reads as null (plain) or "" (quoted);
                    // the panel shows both as empty.
                    let empty = matches!(got, Some(serde_json::Value::Null) | None)
                        || got == Some(&serde_json::Value::String(String::new()));
                    assert!(empty, "{yaml:?} {value:?}: {out}");
                    continue;
                } else {
                    serde_json::Value::String(value.to_string())
                };
                assert_eq!(got, Some(&expected), "{yaml:?} <- {value:?}:\n{out}");
                assert_eq!(
                    map.get("other"),
                    Some(&serde_json::Value::String("kept".to_string()))
                );
            }
        }
    }

    #[test]
    fn list_edits_parse_back_exactly() {
        let items: Vec<String> = TRICKY
            .iter()
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .collect();
        for yaml in [
            "key: [a, b]\nother: kept\n",
            "key:\n  - a\nother: kept\n",
            "other: kept\n",
        ] {
            let out = set(yaml, "key", Change::List(items.clone()));
            let map = parse(&out);
            let expected = serde_json::Value::Array(
                items
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            );
            assert_eq!(map.get("key"), Some(&expected), "{yaml:?}:\n{out}");
            assert_eq!(
                map.get("other"),
                Some(&serde_json::Value::String("kept".to_string()))
            );
        }
    }

    #[test]
    fn multi_line_text_parses_back() {
        for value in [
            "a\nb",
            "a\nb\n",
            "first\n\n  indented\nlast",
            "x: y\n# not a comment",
        ] {
            for yaml in ["key: old\nother: kept\n", "key: |\n  old\nother: kept\n"] {
                let out = set(yaml, "key", text(value));
                let map = parse(&out);
                assert_eq!(
                    map.get("key"),
                    Some(&serde_json::Value::String(value.to_string())),
                    "{value:?}:\n{out}"
                );
                assert_eq!(
                    map.get("other"),
                    Some(&serde_json::Value::String("kept".to_string()))
                );
            }
        }
    }

    #[test]
    fn documents_with_and_without_front_matter() {
        let doc = "---\r\ntitle: x\r\n---\r\n# Body\r\n";
        assert_eq!(
            edit_document(doc, "scrald-theme", &text("sepia")).unwrap(),
            "---\r\ntitle: x\r\nscrald-theme: sepia\r\n---\r\n# Body\r\n"
        );
        assert_eq!(
            edit_document("# Body\n", "title", &text("Saga")).unwrap(),
            "---\ntitle: Saga\n---\n# Body\n"
        );
        assert_eq!(
            edit_document("# Body\n", "title", &Change::Remove).unwrap(),
            "# Body\n"
        );
        // Only the front matter changes; the body is byte-for-byte the same.
        let body = "\n# Heading\n\ntags: [not, front, matter]\n";
        let doc = format!("---\ntags: [a]\n---{body}");
        let out = edit_document(&doc, "tags", &list(&["b"])).unwrap();
        assert_eq!(out, format!("---\ntags: [b]\n---{body}"));
    }
}
