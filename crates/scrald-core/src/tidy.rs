//! Tidying hard-wrapped text (#219): text pasted from a PDF, an email, or a
//! plain-text book has a line break at the end of every printed line, and
//! often no blank line between paragraphs. `unwrap_paragraphs` joins the
//! lines of each paragraph and puts a blank line where a paragraph seems to
//! end.
//!
//! Only top-level paragraphs are touched, found from the parse's blocks, so
//! front matter, code, tables, headings, lists, quotes, and HTML are left
//! alone by construction. The rules are guesses, so the app shows the result
//! beside the text before it's kept.

use crate::document::{BlockKind, DocumentModel};
use crate::source::{self, SourceRange};

/// The tidied text, and what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tidied {
    pub text: String,
    /// Line breaks replaced by a space (or removed after a hyphen).
    pub joined: usize,
    /// Line breaks made into paragraph breaks (a blank line).
    pub split: usize,
}

/// Tidies the top-level paragraphs of `doc`, which was parsed from `text`
/// (the document's text without its BOM). With `within` (offsets in
/// `text`), only line breaks inside that range change.
pub fn tidy_document(doc: &DocumentModel, text: &str, within: Option<SourceRange>) -> Tidied {
    let bom_len = if doc.has_bom { 3 } else { 0 };
    let paragraphs: Vec<SourceRange> = doc
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::Paragraph)
        .map(|b| SourceRange::new(b.source.start - bom_len, b.source.end - bom_len))
        .collect();
    unwrap_paragraphs(text, &paragraphs, within)
}

/// One line of a paragraph: where it starts, and where its text ends
/// (before the line break).
#[derive(Debug, Clone, Copy)]
struct Line {
    start: usize,
    end: usize,
}

/// A change to the text: `start..end` becomes `with`.
struct Change {
    start: usize,
    end: usize,
    with: &'static str,
}

/// Joins the lines of each paragraph in `paragraphs` (ranges in `text`),
/// except where a paragraph seems to end, which gets a blank line.
pub fn unwrap_paragraphs(
    text: &str,
    paragraphs: &[SourceRange],
    within: Option<SourceRange>,
) -> Tidied {
    let line_break = source::line_break(source::detect_line_ending(text));
    let mut changes: Vec<Change> = Vec::new();
    let mut joined = 0;
    let mut split = 0;
    // Rust note: a `OnceCell` is filled the first time it's asked for, so
    // the lowercased copy of the text is made only if a word is broken.
    let lowered = std::cell::OnceCell::new();
    for range in paragraphs {
        let Some(paragraph) = text.get(range.start..range.end) else {
            continue;
        };
        if !can_tidy(paragraph) {
            continue;
        }
        let lines = lines_of(text, *range);
        // The longest line, in characters: a PDF's lines are about this
        // long, except the last line of each paragraph.
        let longest = lines
            .iter()
            .map(|l| text[l.start..l.end].trim().chars().count())
            .max()
            .unwrap_or(0);
        // Rust note: `windows(2)` yields overlapping pairs of neighbours
        // (`[a, b]`, `[b, c]`, ...) as slices that borrow the vector.
        for pair in lines.windows(2) {
            let (line, next) = (pair[0], pair[1]);
            let this = &text[line.start..line.end];
            if within.is_some_and(|w| line.end < w.start || line.end >= w.end) {
                continue;
            }
            // Markdown hard breaks stay as they are.
            if this.ends_with("  ") || this.ends_with('\\') || this.ends_with('\t') {
                continue;
            }
            let kept = this.trim_end();
            let following = text[next.start..next.end].trim_start();
            // From the end of this line's text to the start of the next's.
            let gap_start = line.start + kept.len();
            let gap_end = next.end - following.len();
            if ends_paragraph(kept, following, longest) {
                changes.push(Change {
                    start: gap_start,
                    end: gap_end,
                    with: if line_break == "\r\n" {
                        "\r\n\r\n"
                    } else {
                        "\n\n"
                    },
                });
                split += 1;
            } else if let Some(word) = broken_word(kept, following) {
                // Keep the hyphen if the text has the hyphenated word
                // elsewhere (`well-known`); otherwise remove it.
                let lower = lowered.get_or_init(|| text.to_lowercase());
                let hyphen = if lower.contains(&word) { 0 } else { 1 };
                changes.push(Change {
                    start: gap_start - hyphen,
                    end: gap_end,
                    with: "",
                });
                joined += 1;
            } else {
                changes.push(Change {
                    start: gap_start,
                    end: gap_end,
                    with: " ",
                });
                joined += 1;
            }
        }
    }
    let mut tidied = String::with_capacity(text.len());
    let mut at = 0;
    for change in &changes {
        tidied.push_str(&text[at..change.start]);
        tidied.push_str(change.with);
        at = change.end;
    }
    tidied.push_str(&text[at..]);
    Tidied {
        text: tidied,
        joined,
        split,
    }
}

/// Paragraphs whose line breaks mean something are left alone: Pandoc line
/// blocks (`| ` lines) and display math. Obsidian comments too, to be safe.
fn can_tidy(paragraph: &str) -> bool {
    let first = paragraph.trim_start();
    !(first.starts_with('|') || paragraph.contains("$$") || paragraph.contains("%%"))
}

/// The paragraph's lines, without their line breaks.
fn lines_of(text: &str, range: SourceRange) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = range.start;
    while start < range.end {
        let end = text[start..range.end]
            .find('\n')
            .map_or(range.end, |i| start + i);
        // Rust note: `strip_suffix` gives the text without the suffix if it
        // has it (`Some`), else `None`; `map_or` picks the length either way.
        let content = text[start..end]
            .strip_suffix('\r')
            .map_or(end - start, str::len);
        lines.push(Line {
            start,
            end: start + content,
        });
        start = end + 1;
    }
    lines
}

/// Whether the line break after `line` (trimmed) ends a paragraph: the line
/// ends a sentence and is short, and the next starts like a sentence.
fn ends_paragraph(line: &str, next: &str, longest: usize) -> bool {
    let short = line.chars().count() * 100 < longest * 85;
    short && ends_sentence(line) && starts_sentence(next) && can_start_paragraph(next)
}

/// Closing quotes, brackets, and emphasis that can follow a sentence's end.
const CLOSERS: &[char] = &['"', '\'', '”', '’', '»', ')', ']', '*', '_'];

/// Abbreviations that end in a full stop without ending a sentence.
const ABBREVIATIONS: &[&str] = &[
    "mr.", "mrs.", "ms.", "dr.", "st.", "prof.", "vs.", "e.g.", "i.e.", "cf.",
];

fn ends_sentence(line: &str) -> bool {
    let stripped = line.trim_end_matches(CLOSERS);
    let last_word = stripped.rsplit(' ').next().unwrap_or("").to_lowercase();
    stripped.ends_with(['.', '?', '!', ':', '…']) && !ABBREVIATIONS.contains(&last_word.as_str())
}

fn starts_sentence(line: &str) -> bool {
    let unemphasized = line.trim_start_matches(['*', '_']);
    unemphasized.chars().next().is_some_and(|c| {
        c.is_uppercase()
            || c.is_ascii_digit()
            || ['"', '“', '\'', '‘', '«', '(', '—', '–', '-'].contains(&c)
    })
}

/// A line that would start something other than a paragraph after a blank
/// line (an ordered list, a link definition, HTML) keeps its line break.
/// Inside a paragraph these are plain text, since they can't interrupt one.
fn can_start_paragraph(line: &str) -> bool {
    if line.starts_with(['<', '[', '|', ':', '>', '#', '+', '~', '`', '$']) {
        return false;
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let after = &line[digits..];
    let ordered_marker = (1..=9).contains(&digits)
        && (after.starts_with(['.', ')']))
        && after[1..].chars().next().is_none_or(char::is_whitespace);
    let bullet =
        line.starts_with(['-', '*']) && line[1..].chars().next().is_none_or(char::is_whitespace);
    !ordered_marker && !bullet
}

/// For a word broken across lines (`exam-` / `ple`), the word with its
/// hyphen, lowercased (`exam-ple`). `None` if the line doesn't end in a
/// broken word.
fn broken_word(line: &str, next: &str) -> Option<String> {
    let before = line.strip_suffix('-')?;
    let head_len: usize = before
        .chars()
        .rev()
        .take_while(|c| c.is_alphabetic())
        .map(char::len_utf8)
        .sum();
    let head = &before[before.len() - head_len..];
    let tail: String = next.chars().take_while(|c| c.is_alphabetic()).collect();
    if head.is_empty() || !tail.chars().next().is_some_and(char::is_lowercase) {
        return None;
    }
    Some(format!("{head}-{tail}").to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tidy(text: &str) -> String {
        let doc = crate::parse_document(PathBuf::from("t.md"), text.as_bytes()).unwrap();
        tidy_document(&doc, text, None).text
    }

    const PDF: &str = "The ships left the fjord at dawn, their sails heavy with\n\
                       rain, and the men at the oars said nothing for a long time.\n\
                       Nobody looked back.\n\
                       \"We'll not see it again,\" said Ragnar, when the coast had\n\
                       gone grey behind them.\n";

    #[test]
    fn joins_lines_and_finds_paragraph_ends() {
        assert_eq!(
            tidy(PDF),
            "The ships left the fjord at dawn, their sails heavy with rain, and the men at the oars said nothing for a long time. Nobody looked back.\n\n\
             \"We'll not see it again,\" said Ragnar, when the coast had gone grey behind them.\n"
        );
    }

    #[test]
    fn keeps_crlf_and_multibyte_text() {
        let text = "Æsir and Vanir made peace after their long war and swapped\r\nhostages: Njörðr went to Ásgarðr.\r\n“Then all was well,” the poem says.\r\n";
        assert_eq!(
            tidy(text),
            "Æsir and Vanir made peace after their long war and swapped hostages: Njörðr went to Ásgarðr.\r\n\r\n“Then all was well,” the poem says.\r\n"
        );
    }

    #[test]
    fn tidy_text_comes_back_unchanged() {
        let text = "---\ntitle: Tidy\n---\n# Heading\n\nOne line.\n\nAnother line.\n\n- a list\n  item\n\n```\ncode\nlines\n```\n\n> a quote\n> here\n";
        assert_eq!(tidy(text), text);
        let crlf = text.replace('\n', "\r\n");
        assert_eq!(tidy(&crlf), crlf);
    }

    #[test]
    fn rejoins_broken_words() {
        let text = "It was an exam-\nple of the well-known kind, well-\nknown to all of them.\n";
        assert_eq!(
            tidy(text),
            "It was an example of the well-known kind, well-known to all of them.\n"
        );
    }

    #[test]
    fn leaves_hard_breaks_and_line_blocks() {
        let hard = "First line  \nsecond line\\\nthird line\n";
        assert_eq!(tidy(hard), hard);
        let line_block = "| The Lord of the Rings\n| by J. R. R. Tolkien\n";
        assert_eq!(tidy(line_block), line_block);
    }

    #[test]
    fn breaks_only_where_a_paragraph_can_start() {
        // After a blank line, "2. Then" would become a list, so it's joined.
        let text =
            "Some long line of text that sets the usual width here.\nDone.\n2. Then more text.\n";
        assert_eq!(
            tidy(text),
            "Some long line of text that sets the usual width here. Done. 2. Then more text.\n"
        );
        // An abbreviation doesn't end a sentence.
        let text =
            "Some long line of text that sets the usual width here.\nAsk Dr.\nSmith about it.\n";
        assert_eq!(
            tidy(text),
            "Some long line of text that sets the usual width here. Ask Dr. Smith about it.\n"
        );
    }

    #[test]
    fn only_inside_the_given_range() {
        let text = "one\ntwo\n\nthree\nfour\n";
        let doc = crate::parse_document(PathBuf::from("t.md"), text.as_bytes()).unwrap();
        let second = text.find("three").unwrap();
        let tidied = tidy_document(&doc, text, Some(SourceRange::new(second, text.len())));
        assert_eq!(tidied.text, "one\ntwo\n\nthree four\n");
        assert_eq!((tidied.joined, tidied.split), (1, 0));
    }
}
