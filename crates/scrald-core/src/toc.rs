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
