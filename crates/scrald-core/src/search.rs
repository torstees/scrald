//! Search in a document (DESIGN.md §10).
//!
//! Search looks at the text the reader shows, not the Markdown source: a
//! match in `**Ragnar** Lothbrok` should be found as "Ragnar Lothbrok", and a
//! match inside a link's URL or other syntax would have nothing on screen to
//! highlight. Each block's visible text is read from its sanitized HTML.
//!
//! Core only counts the matches in each block and footnote. The reader finds
//! the same matches in the block's DOM text to highlight them, so both sides
//! must read text and match it the same way (`ui/src/search/matcher.ts`):
//! - every run of whitespace counts as one space, and `<br>` as whitespace;
//! - without "match case", each character is lowercased on its own, and kept
//!   as it is if its lowercase form is more than one character;
//! - matches don't overlap, searching from left to right;
//! - "whole word" means no letter or digit right before or after a match;
//! - math, diagrams, and music are left out, since the reader draws them.

use serde::Serialize;

use crate::document::{BlockKind, DocumentModel};

/// How to compare the query with the text.
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchOptions {
    pub match_case: bool,
    pub whole_word: bool,
}

/// How many matches one block or footnote has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hits {
    /// The block id, or the footnote's index in `DocumentModel::footnotes`.
    pub index: u32,
    pub count: u32,
}

/// Matches in a document: blocks in order, then footnotes (shown as endnotes
/// after the last block). Only blocks and footnotes with matches are listed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SearchResults {
    pub total: u32,
    pub blocks: Vec<Hits>,
    pub footnotes: Vec<Hits>,
}

/// One block's or footnote's text, ready to search.
#[derive(Debug, Clone)]
struct Text {
    /// Whitespace collapsed.
    plain: String,
    /// `plain` lowercased character by character.
    lower: String,
}

impl Text {
    fn new(html: &str) -> Self {
        let plain = collapse_whitespace(&visible_text(html));
        let lower = lowercase(&plain);
        Text { plain, lower }
    }
}

/// The searchable text of one document. Reading text out of HTML is done on
/// the first search, so documents that are never searched don't pay for it.
#[derive(Debug, Clone, Default)]
pub struct SearchIndex {
    /// Each block's HTML (`None` for blocks the reader shows no text for),
    /// then each footnote's; emptied once `texts` is built.
    blocks_html: Vec<Option<String>>,
    footnotes_html: Vec<String>,
    texts: Option<(Vec<Option<Text>>, Vec<Text>)>,
}

impl SearchIndex {
    pub fn new(doc: &DocumentModel) -> Self {
        let blocks_html = doc
            .blocks
            .iter()
            .map(|block| match block.kind {
                // Shown in the endnotes, from `footnotes`, not in place.
                BlockKind::FootnoteDefinition => None,
                _ => Some(block.html.clone()),
            })
            .collect();
        let footnotes_html = doc.footnotes.iter().map(|f| f.html.clone()).collect();
        SearchIndex {
            blocks_html,
            footnotes_html,
            texts: None,
        }
    }

    pub fn search(&mut self, query: &str, options: SearchOptions) -> SearchResults {
        let needle = collapse_whitespace(query.trim());
        if needle.is_empty() {
            return SearchResults::default();
        }
        let needle = if options.match_case {
            needle
        } else {
            lowercase(&needle)
        };
        // Rust note: `get_or_insert_with` fills an `Option` the first time
        // and hands back a reference to what's inside, like a lazily
        // initialized member.
        let blocks_html = &mut self.blocks_html;
        let footnotes_html = &mut self.footnotes_html;
        let (blocks, footnotes) = self.texts.get_or_insert_with(|| {
            let blocks = std::mem::take(blocks_html)
                .iter()
                .map(|html| html.as_deref().map(Text::new))
                .collect();
            let footnotes = std::mem::take(footnotes_html)
                .iter()
                .map(|html| Text::new(html))
                .collect();
            (blocks, footnotes)
        });

        let count_in = |text: &Text| {
            let haystack = if options.match_case {
                &text.plain
            } else {
                &text.lower
            };
            count_matches(haystack, &needle, options.whole_word)
        };
        let mut results = SearchResults::default();
        for (index, text) in blocks.iter().enumerate() {
            let count = text.as_ref().map_or(0, count_in);
            if count > 0 {
                results.blocks.push(Hits {
                    index: index as u32,
                    count,
                });
                results.total += count;
            }
        }
        for (index, text) in footnotes.iter().enumerate() {
            let count = count_in(text);
            if count > 0 {
                results.footnotes.push(Hits {
                    index: index as u32,
                    count,
                });
                results.total += count;
            }
        }
        results
    }
}

/// Counts the matches of `needle` in `haystack`, left to right, without
/// overlaps. With `whole_word`, a match next to a letter or digit is skipped
/// and the search goes on from the next character.
pub fn count_matches(haystack: &str, needle: &str, whole_word: bool) -> u32 {
    // An empty needle would match everywhere, without moving on.
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut from = 0;
    // Rust note: `while let` loops as long as the pattern matches, here
    // until `find` returns `None`, like `while ((pos = find(...)) != npos)`.
    while let Some(found) = haystack[from..].find(needle) {
        let start = from + found;
        let end = start + needle.len();
        if whole_word && !(is_boundary_before(haystack, start) && is_boundary_after(haystack, end))
        {
            // Rust note: `chars().next()` is the character starting at
            // `start`; stepping by its UTF-8 length keeps `from` on a
            // character boundary, which slicing a `str` requires.
            let step = haystack[start..].chars().next().map_or(1, char::len_utf8);
            from = start + step;
            continue;
        }
        count += 1;
        from = end;
    }
    count
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

fn is_boundary_before(text: &str, at: usize) -> bool {
    !text[..at].chars().next_back().is_some_and(is_word_char)
}

fn is_boundary_after(text: &str, at: usize) -> bool {
    !text[at..].chars().next().is_some_and(is_word_char)
}

/// Every run of whitespace becomes one space.
pub fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

/// Lowercases each character on its own, keeping it as it is when its
/// lowercase form is more than one character (`İ`), so the text keeps one
/// character per character and the reader can map matches back to it.
pub fn lowercase(text: &str) -> String {
    text.chars()
        .map(|c| {
            let mut lower = c.to_lowercase();
            match (lower.next(), lower.next()) {
                (Some(single), None) => single,
                _ => c,
            }
        })
        .collect()
}

/// The text the reader shows for sanitized HTML: tags removed, entities
/// decoded, `<br>` as a space, and math, diagrams, and music left out.
pub fn visible_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    // While inside a left-out element: its tag name and how deeply that tag
    // is nested inside itself.
    let mut skipping: Option<(String, usize)> = None;
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        if skipping.is_none() {
            push_decoded(&mut out, &rest[..open]);
        }
        let after = &rest[open + 1..];
        if let Some(comment) = after.strip_prefix("!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            continue;
        }
        let Some(len) = tag_length(after) else {
            // A stray `<` with no tag after it: it's text.
            if skipping.is_none() {
                out.push('<');
            }
            rest = after;
            continue;
        };
        let tag = Tag::parse(&after[..len]);
        rest = &after[len + 1..];

        if let Some((name, depth)) = &mut skipping {
            if tag.name == *name {
                if tag.closing {
                    *depth -= 1;
                    if *depth == 0 {
                        skipping = None;
                    }
                } else if !tag.self_closing {
                    *depth += 1;
                }
            }
            continue;
        }
        if tag.closing {
            continue;
        }
        if tag.name == "br" {
            out.push(' ');
        } else if tag.is_drawn_by_reader() && !tag.self_closing && !is_void(&tag.name) {
            skipping = Some((tag.name, 1));
        }
    }
    if skipping.is_none() {
        push_decoded(&mut out, rest);
    }
    out
}

/// The length of the tag at the start of `text` (just after its `<`), up
/// to its `>`, skipping `>` inside quoted attribute values. `None` if this
/// isn't a tag.
fn tag_length(text: &str) -> Option<usize> {
    let first = text.chars().next()?;
    if !(first.is_ascii_alphabetic() || first == '/' || first == '!') {
        return None;
    }
    let mut quote: Option<char> = None;
    for (i, c) in text.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '>' => return Some(i),
            None => {}
        }
    }
    None
}

/// An HTML tag, read only as far as search needs.
struct Tag {
    name: String,
    closing: bool,
    self_closing: bool,
    attributes: Vec<(String, String)>,
}

impl Tag {
    /// `text` is everything between `<` and `>`.
    fn parse(text: &str) -> Self {
        let closing = text.starts_with('/');
        let body = text.trim_start_matches('/');
        let self_closing = body.trim_end().ends_with('/');
        let name_len = body
            .find(|c: char| c.is_whitespace() || c == '/')
            .unwrap_or(body.len());
        let name = body[..name_len].to_ascii_lowercase();
        let attributes = parse_attributes(&body[name_len..]);
        Tag {
            name,
            closing,
            self_closing,
            attributes,
        }
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Math (KaTeX), diagrams (Mermaid), and music (abcjs): the reader
    /// replaces these with drawings, so their source text isn't on screen.
    /// The selectors match `ui/src/render/renderers.ts`.
    fn is_drawn_by_reader(&self) -> bool {
        if self.attribute("data-math-style").is_some() {
            return true;
        }
        self.name == "code"
            && self.attribute("class").is_some_and(|class| {
                class
                    .split_whitespace()
                    .any(|c| c == "language-mermaid" || c == "language-abc")
            })
    }
}

/// `name="value"`, `name='value'`, `name=value`, and bare `name` attributes.
fn parse_attributes(text: &str) -> Vec<(String, String)> {
    let mut attributes = Vec::new();
    let mut rest = text;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        if rest.is_empty() {
            return attributes;
        }
        let name_len = rest
            .find(|c: char| c.is_whitespace() || c == '=' || c == '/')
            .unwrap_or(rest.len());
        let name = rest[..name_len].to_ascii_lowercase();
        rest = rest[name_len..].trim_start();
        let mut value = String::new();
        if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            let (raw, remaining) = match after.chars().next() {
                // Rust note: `q @ pattern` matches the pattern and also binds
                // the matched value to `q`, here whichever quote it was.
                Some(q @ ('"' | '\'')) => {
                    let inner = &after[1..];
                    let end = inner.find(q).unwrap_or(inner.len());
                    (&inner[..end], inner.get(end + 1..).unwrap_or(""))
                }
                _ => {
                    let end = after.find(char::is_whitespace).unwrap_or(after.len());
                    (&after[..end], &after[end..])
                }
            };
            push_decoded(&mut value, raw);
            rest = remaining;
        }
        attributes.push((name, value));
    }
}

/// Elements that never have content or a closing tag.
fn is_void(name: &str) -> bool {
    // Rust note: `matches!` is `true` if the value fits the pattern, a
    // compact way to write a `match` that only answers yes or no.
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Appends `text` with its character references decoded. The sanitizer
/// writes only `&amp;`, `&lt;`, `&gt;`, `&quot;`, and `&nbsp;`, but numeric
/// references and `&apos;` are decoded too; anything else stays as written.
fn push_decoded(out: &mut String, text: &str) {
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        let decoded = after.find(';').filter(|&end| end <= 10).and_then(|end| {
            let name = &after[..end];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                _ => {
                    let number = name.strip_prefix('#')?;
                    let code = match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => number.parse().ok()?,
                    };
                    char::from_u32(code)
                }
            }?;
            Some((c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ParseOptions, parse_document_with};
    use std::path::PathBuf;

    fn index(markdown: &str) -> SearchIndex {
        let doc = parse_document_with(
            PathBuf::from("test.md"),
            markdown.as_bytes(),
            &ParseOptions::default(),
        )
        .unwrap();
        SearchIndex::new(&doc)
    }

    fn search(markdown: &str, query: &str) -> SearchResults {
        index(markdown).search(query, SearchOptions::default())
    }

    #[test]
    fn text_is_what_the_reader_shows() {
        assert_eq!(
            visible_text("<p><strong>Ragnar</strong> Lothbrok &amp; sons<br>x &lt;3</p>"),
            "Ragnar Lothbrok & sons x <3"
        );
        assert_eq!(
            visible_text("<p>a &#233; &#x263A; &bogus; & b</p>"),
            "a é ☺ &bogus; & b"
        );
        assert_eq!(
            visible_text(r#"<p title="a > b">t</p><!-- hidden <p>no</p> -->"#),
            "t"
        );
        assert_eq!(visible_text("<p>1 < 2</p>"), "1 < 2");
    }

    #[test]
    fn math_diagrams_and_music_are_left_out() {
        assert_eq!(
            visible_text(r#"<p>Energy <span data-math-style="inline">E = mc^2</span> here</p>"#),
            "Energy  here"
        );
        assert_eq!(
            visible_text(
                r#"<pre><code class="language-mermaid">graph TD; <span>A</span></code></pre><p>after</p>"#
            ),
            "after"
        );
        // Nested elements with the same tag name don't end the skip early.
        assert_eq!(
            visible_text(r#"<span data-math-style="inline"><span>x</span>y</span>z"#),
            "z"
        );
        // Other code is searchable.
        assert_eq!(
            visible_text(r#"<pre><code class="language-rust">fn main()</code></pre>"#),
            "fn main()"
        );
    }

    #[test]
    fn whitespace_and_case() {
        assert_eq!(collapse_whitespace(" a \n\t b\u{a0}c "), " a b c ");
        assert_eq!(lowercase("ÅSA Straße İ"), "åsa straße İ");
    }

    #[test]
    fn counting_matches() {
        assert_eq!(count_matches("aaaa", "aa", false), 2);
        assert_eq!(count_matches("the theme then", "the", false), 3);
        assert_eq!(count_matches("the theme then", "the", true), 1);
        assert_eq!(count_matches("thethe the", "the", true), 1);
        assert_eq!(count_matches("skål, skål!", "skål", true), 2);
        assert_eq!(count_matches("x", "", false), 0);
    }

    #[test]
    fn searches_blocks_across_formatting() {
        let results = search(
            "# The Saga\n\n**Ragnar** Lothbrok sailed.\n\nNo match here.\n\nragnar\nlothbrok again\n",
            "ragnar lothbrok",
        );
        assert_eq!(results.total, 2);
        assert_eq!(
            results.blocks,
            vec![Hits { index: 1, count: 1 }, Hits { index: 3, count: 1 }]
        );
    }

    #[test]
    fn match_case_and_whole_word() {
        let mut idx = index("Theme the THE them\n");
        let case = SearchOptions {
            match_case: true,
            whole_word: false,
        };
        assert_eq!(idx.search("THE", case).total, 1);
        let word = SearchOptions {
            match_case: false,
            whole_word: true,
        };
        assert_eq!(idx.search("the", word).total, 2);
        assert_eq!(idx.search("the", SearchOptions::default()).total, 4);
        assert_eq!(idx.search("   ", SearchOptions::default()).total, 0);
    }

    #[test]
    fn footnotes_are_searched_as_endnotes() {
        let results = search("A claim.[^1]\n\n[^1]: The source of the claim.\n", "claim");
        // The definition block shows nothing in place; its text is found once,
        // in the endnote.
        assert_eq!(results.blocks, vec![Hits { index: 0, count: 1 }]);
        assert_eq!(results.footnotes, vec![Hits { index: 0, count: 1 }]);
        assert_eq!(results.total, 2);
    }

    #[test]
    fn link_urls_are_not_searched() {
        let results = search("[the docs](https://example.com/longship)\n", "longship");
        assert_eq!(results.total, 0);
    }
}
