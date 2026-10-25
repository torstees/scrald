//! What the reader may show before a document's first block (#189): its
//! cover image, or its title when the text doesn't start with it. Display
//! only; the file is never changed. Whether either is shown is up to the
//! theme and the user's settings, so core only says what's available.

use crate::toc::TocEntry;

/// How many blocks from the start count as "near the beginning" when
/// looking for the title: room for a subtitle, an epigraph, or a byline
/// before the heading.
pub const NEAR_START_BLOCKS: u32 = 3;

/// The front matter's title, if the text doesn't already show it near the
/// start. It counts as shown when one of the first few blocks is a heading
/// with the same text, or a Pandoc `% Title` line does: compared ignoring
/// case, punctuation, and spacing, and either may contain the other
/// ("Chapter 1: The Long Winter" shows "The Long Winter").
///
/// `toc` gives the headings and their blocks; `first_block_text` is the
/// source of the document's first block, for `% Title`.
pub fn missing_title(
    title: Option<&str>,
    toc: &[TocEntry],
    first_block_text: Option<&str>,
) -> Option<String> {
    let title = title.map(str::trim).filter(|t| !t.is_empty())?;
    let wanted = normalize(title);
    if wanted.is_empty() {
        return None;
    }
    let shown = |text: &str| {
        let text = normalize(text);
        !text.is_empty() && (text.contains(&wanted) || wanted.contains(&text))
    };
    // Rust note: `take_while` stops at the first heading past the first few
    // blocks; the TOC is in document order, so nothing later can be nearer.
    let in_heading = toc
        .iter()
        .take_while(|entry| entry.block_id < NEAR_START_BLOCKS)
        .any(|entry| shown(&entry.text));
    let in_pandoc_title = first_block_text
        .and_then(|text| text.trim_start().strip_prefix('%'))
        .and_then(|rest| rest.lines().next())
        .is_some_and(shown);
    (!in_heading && !in_pandoc_title).then(|| title.to_string())
}

/// Lowercase letters and digits, with every other run of characters as one
/// space: "The Long  Winter!" becomes "the long winter".
fn normalize(text: &str) -> String {
    let spaced: String = text
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_lowercase().next().unwrap_or(c)
            } else {
                ' '
            }
        })
        .collect();
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(text: &str, block_id: u32) -> TocEntry {
        TocEntry {
            level: 1,
            text: text.to_string(),
            slug: String::new(),
            block_id,
            offset: 0,
            number: None,
        }
    }

    #[test]
    fn a_title_shown_near_the_start_is_not_missing() {
        let toc = [heading("The Long Winter", 0)];
        assert_eq!(missing_title(Some("The Long Winter"), &toc, None), None);
        // Case, punctuation, and spacing don't matter; either may contain the other.
        let toc = [heading("Chapter 1: the long  winter", 2)];
        assert_eq!(missing_title(Some("The Long Winter!"), &toc, None), None);
        let toc = [heading("Winter", 1)];
        assert_eq!(missing_title(Some("The Long Winter"), &toc, None), None);
    }

    #[test]
    fn a_title_not_near_the_start_is_missing() {
        assert_eq!(
            missing_title(Some("Notes"), &[], None),
            Some("Notes".to_string())
        );
        let toc = [heading("Introduction", 0), heading("Notes", 5)];
        assert_eq!(
            missing_title(Some("Notes"), &toc, None),
            Some("Notes".to_string())
        );
        // A different first heading doesn't count.
        let toc = [heading("Meeting", 0)];
        assert_eq!(
            missing_title(Some("Notes from Oslo"), &toc, None),
            Some("Notes from Oslo".to_string())
        );
    }

    #[test]
    fn a_pandoc_title_line_counts() {
        let first = "% The Long Winter\n% A. Writer\n";
        assert_eq!(
            missing_title(Some("The Long Winter"), &[], Some(first)),
            None
        );
        assert!(missing_title(Some("The Long Winter"), &[], Some("Plain text")).is_some());
    }

    #[test]
    fn no_title_means_nothing_to_add() {
        assert_eq!(missing_title(None, &[], None), None);
        assert_eq!(missing_title(Some("  "), &[], None), None);
        assert_eq!(missing_title(Some("!!!"), &[], None), None);
    }

    #[test]
    fn titles_in_other_scripts() {
        let toc = [heading("Långa vintern", 0)];
        assert_eq!(missing_title(Some("LÅNGA VINTERN"), &toc, None), None);
        assert_eq!(
            missing_title(Some("Сага"), &[], None),
            Some("Сага".to_string())
        );
    }
}
