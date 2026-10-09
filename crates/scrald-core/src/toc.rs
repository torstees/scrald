//! Table of contents and heading IDs.

use std::collections::HashSet;

use serde::Serialize;

/// One heading in the table of contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TocEntry {
    /// Heading level, 1–6.
    pub level: u8,
    /// Plain text of the heading.
    pub text: String,
    /// The heading's HTML `id`, unique within the document.
    pub slug: String,
    /// The `Block::id` of the heading.
    pub block_id: u32,
    /// Outline number like "2.1", for themes that number headings. `None`
    /// for a document title (a lone top-level heading at the start).
    pub number: Option<String>,
}

/// Outline numbers for headings at the given levels, in document order.
///
/// Numbers follow nesting, not raw levels: a heading's depth is how many
/// enclosing headings are open above it, so an H4 directly under an H2 is
/// numbered like an H3 would be. A lone top-level heading that comes first
/// is the document title and is left unnumbered; numbering then starts
/// below it.
pub fn number_headings(levels: &[u8]) -> Vec<Option<String>> {
    let Some(&top) = levels.iter().min() else {
        return Vec::new();
    };
    let top_count = levels.iter().filter(|&&l| l == top).count();
    let has_title = top_count == 1 && levels.first() == Some(&top);

    // Levels of the headings currently "open" above the one being numbered.
    let mut open: Vec<u8> = Vec::new();
    // counts[d] is the running count at depth d.
    let mut counts: Vec<u32> = Vec::new();
    levels
        .iter()
        .enumerate()
        .map(|(i, &level)| {
            while open.last().is_some_and(|&l| l >= level) {
                open.pop();
            }
            let depth = open.len();
            open.push(level);
            if has_title && i == 0 {
                return None;
            }
            counts.resize(depth + 1, 0);
            counts[depth] += 1;
            let first = usize::from(has_title);
            let parts: Vec<String> = counts[first.min(depth)..=depth]
                .iter()
                .map(u32::to_string)
                .collect();
            Some(parts.join("."))
        })
        .collect()
}

/// Hands out unique, URL-friendly heading IDs, GitHub style: lowercase,
/// spaces become `-`, punctuation is dropped, and repeats get `-1`, `-2`, ...
#[derive(Debug, Default)]
pub struct Slugger {
    used: HashSet<String>,
}

impl Slugger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a slug for `text` that hasn't been returned before.
    pub fn slug(&mut self, text: &str) -> String {
        let base = slugify(text);
        let mut candidate = base.clone();
        let mut n = 1;
        // Loop, rather than trust a counter, in case a heading's own text is
        // already "foo-1".
        while self.used.contains(&candidate) {
            candidate = format!("{base}-{n}");
            n += 1;
        }
        self.used.insert(candidate.clone());
        candidate
    }
}

/// The slug for `text`, before deduplication. Letters and digits from any
/// script are kept, so non-English headings get readable IDs.
pub fn slugify(text: &str) -> String {
    let slug: String = text
        .trim()
        .chars()
        // Rust note: `flat_map` maps each char to an iterator (lowercasing
        // can yield several chars, e.g. 'İ') and flattens the results.
        .flat_map(char::to_lowercase)
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect();
    if slug.is_empty() {
        "section".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("  Chapter 1: The Fjord  "), "chapter-1-the-fjord");
        assert_eq!(slugify("snake_case and-dash"), "snake_case-and-dash");
    }

    #[test]
    fn slugify_keeps_non_ascii_letters() {
        assert_eq!(slugify("Skål!"), "skål");
        assert_eq!(slugify("Über Straße"), "über-straße");
    }

    #[test]
    fn slugify_empty_falls_back() {
        assert_eq!(slugify("!!!"), "section");
        assert_eq!(slugify(""), "section");
    }

    #[test]
    fn numbers_with_a_title() {
        // # Title / ## A / ### A.1 / ### A.2 / ## B
        let n = number_headings(&[1, 2, 3, 3, 2]);
        assert_eq!(
            n,
            [
                None,
                Some("1".into()),
                Some("1.1".into()),
                Some("1.2".into()),
                Some("2".into())
            ]
        );
    }

    #[test]
    fn numbers_chapters_when_there_are_several_top_headings() {
        let n = number_headings(&[1, 2, 1, 2, 2]);
        let n: Vec<&str> = n.iter().map(|x| x.as_deref().unwrap_or("-")).collect();
        assert_eq!(n, ["1", "1.1", "2", "2.1", "2.2"]);
    }

    #[test]
    fn skipped_levels_have_no_zeros() {
        let n = number_headings(&[2, 4, 3, 4]);
        let n: Vec<&str> = n.iter().map(|x| x.as_deref().unwrap_or("-")).collect();
        assert_eq!(n, ["-", "1", "2", "2.1"]);
    }

    #[test]
    fn single_heading_is_a_title_and_empty_is_empty() {
        assert_eq!(number_headings(&[2]), [None]);
        assert!(number_headings(&[]).is_empty());
    }

    #[test]
    fn slugger_deduplicates() {
        let mut s = Slugger::new();
        assert_eq!(s.slug("Notes"), "notes");
        assert_eq!(s.slug("Notes"), "notes-1");
        assert_eq!(s.slug("Notes"), "notes-2");
    }

    #[test]
    fn slugger_avoids_clash_with_literal_suffix() {
        let mut s = Slugger::new();
        assert_eq!(s.slug("Notes 1"), "notes-1");
        assert_eq!(s.slug("Notes"), "notes");
        assert_eq!(s.slug("Notes"), "notes-2");
    }
}
