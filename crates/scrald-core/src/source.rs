//! Raw file bytes → text, plus the one place where line/column positions are
//! converted to byte offsets (AGENTS.md: "convert in exactly one place").

use serde::Serialize;

use crate::DocumentError;

/// The UTF-8 byte order mark some Windows editors put at the start of a file.
const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// A half-open byte range `[start, end)` into the ORIGINAL file bytes
/// (including any BOM and front matter).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
}

impl SourceRange {
    pub fn new(start: usize, end: usize) -> Self {
        SourceRange { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// The range as a standard library `Range`, for slicing: `&bytes[r.as_range()]`.
    // Rust note: `std::ops::Range<usize>` is what `a..b` builds. Slicing a
    // `str` or `[u8]` with it is bounds-checked and panics if out of range.
    pub fn as_range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// The dominant line ending of a file, preserved when saving.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    Lf,
    Crlf,
}

/// A file's text, with the BOM removed and its encoding details recorded.
#[derive(Debug, Clone)]
pub struct DecodedSource {
    /// File contents without the BOM.
    pub text: String,
    /// Whether the file started with a UTF-8 BOM.
    pub has_bom: bool,
    pub line_ending: LineEnding,
}

impl DecodedSource {
    /// Number of bytes before `text` in the original file (the BOM, if any).
    pub fn bom_len(&self) -> usize {
        body_offset(self.has_bom)
    }
}

/// Decodes raw file bytes. Fails on invalid UTF-8 rather than guessing, so a
/// later save can never silently change bytes we didn't understand.
pub fn decode(bytes: &[u8]) -> Result<DecodedSource, DocumentError> {
    // Rust note: `strip_prefix` returns `Some(rest)` if the slice starts with
    // the prefix, else `None`; `match` handles both cases and binds `rest`.
    let (has_bom, body) = match bytes.strip_prefix(UTF8_BOM) {
        Some(rest) => (true, rest),
        None => (false, bytes),
    };
    // Rust note: `map_err` converts one error type into another, and `?`
    // returns early with it, like `raise NewError from e` in Python.
    let text = std::str::from_utf8(body)
        .map_err(|e| DocumentError::NotUtf8 {
            offset: e.valid_up_to() + body_offset(has_bom),
        })?
        .to_string();
    let line_ending = detect_line_ending(&text);
    Ok(DecodedSource {
        text,
        has_bom,
        line_ending,
    })
}

fn body_offset(has_bom: bool) -> usize {
    if has_bom { UTF8_BOM.len() } else { 0 }
}

/// Picks the more common line ending. Files without any newline count as LF.
pub fn detect_line_ending(text: &str) -> LineEnding {
    let total_lf = text.matches('\n').count();
    let crlf = text.matches("\r\n").count();
    let lone_lf = total_lf - crlf;
    if crlf > lone_lf {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}

/// Maps 1-based (line, column) positions in a piece of text to byte offsets
/// in the original file. Columns are byte columns, as comrak reports them.
///
/// `base` is where the indexed text starts in the original file, so offsets
/// come out file-relative even when the text is only the Markdown body.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offset (within the indexed text) where each line starts.
    line_starts: Vec<usize>,
    len: usize,
    base: usize,
}

impl LineIndex {
    pub fn new(text: &str, base: usize) -> Self {
        let mut line_starts = vec![0];
        // Rust note: `match_indices` is a lazy iterator over (offset, match)
        // pairs; `map` keeps the offset after each '\n'. Nothing runs until
        // `extend` pulls the values, much like a Python generator.
        line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex {
            line_starts,
            len: text.len(),
            base,
        }
    }

    /// File offset of the byte at 1-based `(line, column)`.
    pub fn start_offset(&self, line: usize, column: usize) -> usize {
        self.base + self.clamp(self.line_start(line) + column.saturating_sub(1))
    }

    /// File offset just past the byte at 1-based `(line, column)`, for the
    /// inclusive end positions comrak reports. Column 0 means "before the
    /// first byte of the line".
    pub fn end_offset(&self, line: usize, column: usize) -> usize {
        self.base + self.clamp(self.line_start(line) + column)
    }

    fn line_start(&self, line: usize) -> usize {
        // Lines past the end map to the end of the text rather than panicking.
        let index = line.saturating_sub(1);
        self.line_starts.get(index).copied().unwrap_or(self.len)
    }

    fn clamp(&self, offset: usize) -> usize {
        offset.min(self.len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_plain_lf() {
        let d = decode(b"a\nb\n").unwrap();
        assert_eq!(d.text, "a\nb\n");
        assert!(!d.has_bom);
        assert_eq!(d.line_ending, LineEnding::Lf);
        assert_eq!(d.bom_len(), 0);
    }

    #[test]
    fn decode_strips_bom_and_detects_crlf() {
        let d = decode(b"\xEF\xBB\xBFa\r\nb\r\n").unwrap();
        assert_eq!(d.text, "a\r\nb\r\n");
        assert!(d.has_bom);
        assert_eq!(d.line_ending, LineEnding::Crlf);
        assert_eq!(d.bom_len(), 3);
    }

    #[test]
    fn decode_rejects_invalid_utf8_with_file_offset() {
        let err = decode(b"\xEF\xBB\xBFok\xFF").unwrap_err();
        assert_eq!(err, DocumentError::NotUtf8 { offset: 5 });
    }

    #[test]
    fn line_ending_majority_wins() {
        assert_eq!(detect_line_ending("a\r\nb\r\nc\n"), LineEnding::Crlf);
        assert_eq!(detect_line_ending("a\nb\nc\r\n"), LineEnding::Lf);
        assert_eq!(detect_line_ending("no newline"), LineEnding::Lf);
    }

    #[test]
    fn offsets_lf() {
        let text = "ab\ncd\n";
        let idx = LineIndex::new(text, 0);
        assert_eq!(idx.start_offset(1, 1), 0);
        assert_eq!(idx.end_offset(1, 2), 2);
        assert_eq!(idx.start_offset(2, 1), 3);
        assert_eq!(&text[idx.start_offset(2, 1)..idx.end_offset(2, 2)], "cd");
    }

    #[test]
    fn offsets_crlf_exclude_the_line_ending() {
        let text = "ab\r\ncd\r\n";
        let idx = LineIndex::new(text, 0);
        assert_eq!(idx.start_offset(2, 1), 4);
        assert_eq!(&text[idx.start_offset(1, 1)..idx.end_offset(1, 2)], "ab");
        assert_eq!(&text[idx.start_offset(2, 1)..idx.end_offset(2, 2)], "cd");
    }

    #[test]
    fn offsets_multi_byte_end_on_char_boundary() {
        // "é" is 2 bytes; comrak reports the end column as its last byte.
        let text = "# Hé\nskål\n";
        let idx = LineIndex::new(text, 0);
        assert_eq!(&text[idx.start_offset(1, 1)..idx.end_offset(1, 5)], "# Hé");
        assert_eq!(&text[idx.start_offset(2, 1)..idx.end_offset(2, 5)], "skål");
    }

    #[test]
    fn offsets_include_base() {
        let idx = LineIndex::new("x\ny", 10);
        assert_eq!(idx.start_offset(2, 1), 12);
        assert_eq!(idx.end_offset(2, 0), 12);
    }

    #[test]
    fn offsets_past_the_end_are_clamped() {
        let idx = LineIndex::new("abc", 0);
        assert_eq!(idx.end_offset(1, 99), 3);
        assert_eq!(idx.start_offset(9, 1), 3);
    }
}
