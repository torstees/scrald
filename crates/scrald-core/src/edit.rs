//! Splicing edits into a document's source (DESIGN.md §9.1). A block edit
//! replaces exactly the block's byte range, so everything outside it stays
//! byte-for-byte the same, and the whole document is then re-parsed.
//! Also the small diffs that document-level undo and redo are built from.

use crate::source::SourceRange;

/// Why an edit wasn't applied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// The document changed since the block was read (say, a reload), so
    /// the range no longer holds the text being replaced.
    #[error("the document changed while the block was being edited; nothing was changed")]
    Stale,
    #[error("the block's range is outside the document")]
    OutOfRange,
}

/// Replaces a block's source with `replacement`.
///
/// - `text` is the document without its BOM; `range` is the block's range in
///   the original file, which counts the BOM, so `bom_len` (0 or 3) is
///   subtracted to find it in `text`.
/// - `original` must equal the text currently in that range, or the edit is
///   refused (`Stale`) rather than spliced into the wrong place.
/// - Line breaks in `replacement` are converted to the document's own
///   (`\r\n` if the document uses CRLF), and trailing line breaks are
///   dropped: the line break after a block isn't part of its range.
/// - An empty replacement removes the block together with the blank line
///   after it, so the blocks around it stay one blank line apart.
pub fn splice_block(
    text: &str,
    range: SourceRange,
    bom_len: usize,
    original: &str,
    replacement: &str,
) -> Result<String, EditError> {
    let start = range
        .start
        .checked_sub(bom_len)
        .ok_or(EditError::OutOfRange)?;
    let end = range
        .end
        .checked_sub(bom_len)
        .ok_or(EditError::OutOfRange)?;
    let current = text.get(start..end).ok_or(EditError::OutOfRange)?;
    if current != original {
        return Err(EditError::Stale);
    }
    let crlf = text.contains("\r\n");
    let normalized = replacement.replace("\r\n", "\n");
    let trimmed = normalized.trim_end_matches('\n');
    // Some blocks' ranges end with their line break (comrak includes it for
    // a few kinds, such as description lists); keep exactly those.
    let trailing = &original[original.trim_end_matches(['\r', '\n']).len()..];
    let new_text = if crlf {
        trimmed.replace('\n', "\r\n")
    } else {
        trimmed.to_string()
    };

    let mut end = end;
    if new_text.trim().is_empty() {
        // Remove up to two line breaks after the block: its own, and the
        // blank line that separated it from the next block.
        for _ in 0..2 {
            let rest = &text[end..];
            if rest.starts_with("\r\n") {
                end += 2;
            } else if rest.starts_with('\n') {
                end += 1;
            }
        }
        return Ok(format!("{}{}", &text[..start], &text[end..]));
    }
    Ok(format!(
        "{}{}{}{}",
        &text[..start],
        new_text,
        trailing,
        &text[end..]
    ))
}

/// The change between two versions of a text: at byte `at`, `removed` was
/// replaced by `inserted`. The smallest such change (common prefix and
/// suffix left out), cut on character boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDiff {
    pub at: usize,
    pub removed: String,
    pub inserted: String,
}

impl TextDiff {
    /// The diff from `before` to `after`.
    pub fn between(before: &str, after: &str) -> TextDiff {
        let mut prefix = before
            .bytes()
            .zip(after.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !before.is_char_boundary(prefix) || !after.is_char_boundary(prefix) {
            prefix -= 1;
        }
        let max_suffix = before.len().min(after.len()) - prefix;
        let mut suffix = before
            .bytes()
            .rev()
            .zip(after.bytes().rev())
            .take(max_suffix)
            .take_while(|(a, b)| a == b)
            .count();
        while !before.is_char_boundary(before.len() - suffix)
            || !after.is_char_boundary(after.len() - suffix)
        {
            suffix -= 1;
        }
        TextDiff {
            at: prefix,
            removed: before[prefix..before.len() - suffix].to_string(),
            inserted: after[prefix..after.len() - suffix].to_string(),
        }
    }

    /// Applies the diff to the text it was made from (`before` -> `after`).
    pub fn apply(&self, text: &str) -> Option<String> {
        let end = self.at + self.removed.len();
        (text.get(self.at..end)? == self.removed)
            .then(|| format!("{}{}{}", &text[..self.at], self.inserted, &text[end..]))
    }

    /// Undoes the diff on the text it produced (`after` -> `before`).
    pub fn revert(&self, text: &str) -> Option<String> {
        let end = self.at + self.inserted.len();
        (text.get(self.at..end)? == self.inserted)
            .then(|| format!("{}{}{}", &text[..self.at], self.removed, &text[end..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range_of(text: &str, part: &str, bom: usize) -> SourceRange {
        let start = text.find(part).unwrap() + bom;
        SourceRange::new(start, start + part.len())
    }

    #[test]
    fn a_block_is_replaced_and_nothing_else() {
        let text = "# Title\n\nOld paragraph.\n\nNext.\n";
        let r = range_of(text, "Old paragraph.", 0);
        let out = splice_block(text, r, 0, "Old paragraph.", "New *paragraph*.\n\n").unwrap();
        assert_eq!(out, "# Title\n\nNew *paragraph*.\n\nNext.\n");
    }

    #[test]
    fn crlf_bom_and_multibyte_text() {
        let text = "# Tíde\r\n\r\nÅse and Þór.\r\n\r\nEnd.\r\n";
        // The file had a BOM, so ranges are 3 bytes further on.
        let r = range_of(text, "Åse and Þór.", 3);
        let out = splice_block(text, r, 3, "Åse and Þór.", "Line one\nline two").unwrap();
        assert_eq!(out, "# Tíde\r\n\r\nLine one\r\nline two\r\n\r\nEnd.\r\n");
    }

    #[test]
    fn a_no_op_edit_round_trips() {
        let text = "a\r\n\r\n- one\r\n- two\r\n\r\nb";
        let r = range_of(text, "- one\r\n- two", 0);
        assert_eq!(
            splice_block(text, r, 0, "- one\r\n- two", "- one\n- two").unwrap(),
            text
        );
    }

    #[test]
    fn a_range_that_ends_with_its_line_break_keeps_it() {
        let text = "Term\n: Definition.\n\nNext.\n";
        let original = "Term\n: Definition.\n";
        let r = range_of(text, original, 0);
        assert_eq!(splice_block(text, r, 0, original, original).unwrap(), text);
        assert_eq!(
            splice_block(text, r, 0, original, "Word\n: Meaning.").unwrap(),
            "Word\n: Meaning.\n\nNext.\n"
        );
    }

    #[test]
    fn stale_edits_are_refused() {
        let text = "First.\n\nSecond.\n";
        let r = range_of(text, "First.", 0);
        assert_eq!(
            splice_block(text, r, 0, "Frist.", "x"),
            Err(EditError::Stale)
        );
        let far = SourceRange::new(100, 120);
        assert_eq!(
            splice_block(text, far, 0, "", "x"),
            Err(EditError::OutOfRange)
        );
    }

    #[test]
    fn emptying_a_block_removes_it_and_its_blank_line() {
        let text = "One.\n\nTwo.\n\nThree.\n";
        let r = range_of(text, "Two.", 0);
        assert_eq!(
            splice_block(text, r, 0, "Two.", "  \n").unwrap(),
            "One.\n\nThree.\n"
        );
    }

    #[test]
    fn diffs_apply_and_revert() {
        for (before, after) in [
            ("hello world", "hello brave world"),
            ("abc", ""),
            ("", "new"),
            ("Þór sails", "Þóra sails"),
            ("same", "same"),
            ("aaa", "aaaa"),
        ] {
            let diff = TextDiff::between(before, after);
            assert_eq!(
                diff.apply(before).as_deref(),
                Some(after),
                "{before:?} -> {after:?}"
            );
            assert_eq!(
                diff.revert(after).as_deref(),
                Some(before),
                "{after:?} -> {before:?}"
            );
        }
        let diff = TextDiff::between("hello world", "hello brave world");
        assert_eq!(
            (diff.at, diff.removed.as_str(), diff.inserted.as_str()),
            (6, "", "brave ")
        );
        // A diff that doesn't fit the text isn't applied.
        assert_eq!(diff.revert("unrelated"), None);
    }
}
