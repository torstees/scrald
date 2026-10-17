//! Sending a re-parsed document to the UI without the HTML it already has
//! (DESIGN.md §9.1). After an edit most blocks render exactly as before, and
//! their HTML is most of the model's size, so only changed blocks carry it.

use std::collections::HashMap;

use crate::document::Block;

/// What the UI holds of the last document sent to it: each block's source
/// hash and HTML, by block id.
pub type ShownBlocks = Vec<(u64, String)>;

/// The HTML of every block, to remember as shown after sending them all.
pub fn shown_blocks(blocks: &[Block]) -> ShownBlocks {
    blocks
        .iter()
        .map(|block| (block.hash, block.html.clone()))
        .collect()
}

/// Empties the HTML of each block whose HTML a block in `shown` already has.
/// Returns, for each block, the id of the shown block to copy it from
/// (`None` if it was kept), and what the UI holds once it has copied them.
///
/// A shown block is matched by source hash and then by identical HTML (the
/// same text can render differently elsewhere, say a heading with another
/// number), and is used at most once.
pub fn reuse_html(blocks: &mut [Block], shown: &ShownBlocks) -> (Vec<Option<u32>>, ShownBlocks) {
    // Shown block ids by hash, in order.
    let mut by_hash: HashMap<u64, Vec<u32>> = HashMap::new();
    for (id, (hash, _)) in shown.iter().enumerate() {
        by_hash.entry(*hash).or_default().push(id as u32);
    }

    let mut from = Vec::with_capacity(blocks.len());
    let mut next = Vec::with_capacity(blocks.len());
    for block in blocks.iter_mut() {
        let found = by_hash.get_mut(&block.hash).and_then(|ids| {
            let position = ids
                .iter()
                .position(|&id| shown[id as usize].1 == block.html)?;
            Some(ids.remove(position))
        });
        let html = std::mem::take(&mut block.html);
        if found.is_none() {
            block.html = html.clone();
        }
        next.push((block.hash, html));
        from.push(found);
    }
    (from, next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_document;

    fn blocks(text: &str) -> Vec<Block> {
        parse_document("a.md".into(), text.as_bytes())
            .expect("parse")
            .blocks
    }

    #[test]
    fn only_changed_blocks_keep_their_html() {
        let shown = shown_blocks(&blocks("# Title\n\nOne.\n\nTwo.\n"));
        let mut edited = blocks("# Title\n\nOne, changed.\n\nTwo.\n");
        let (from, next) = reuse_html(&mut edited, &shown);
        assert_eq!(from, vec![Some(0), None, Some(2)]);
        assert_eq!(edited[0].html, "");
        assert!(edited[1].html.contains("changed"));
        assert_eq!(edited[2].html, "");
        // What the UI holds afterwards is the full HTML of the new blocks.
        assert_eq!(
            next,
            shown_blocks(&blocks("# Title\n\nOne, changed.\n\nTwo.\n"))
        );
    }

    #[test]
    fn blocks_that_moved_copy_from_their_old_place() {
        let shown = shown_blocks(&blocks("One.\n\nTwo.\n\nThree.\n"));
        // The first block is removed: the others are found one id earlier.
        let mut edited = blocks("Two.\n\nThree.\n");
        let (from, _) = reuse_html(&mut edited, &shown);
        assert_eq!(from, vec![Some(1), Some(2)]);
    }

    #[test]
    fn repeated_blocks_are_each_matched_once() {
        let shown = shown_blocks(&blocks("***\n\nA.\n\n***\n"));
        let mut edited = blocks("***\n\n***\n\n***\n");
        let (from, _) = reuse_html(&mut edited, &shown);
        // Two shown rules to copy from; the third is sent.
        assert_eq!(from, vec![Some(0), Some(2), None]);
        assert!(!edited[2].html.is_empty());
    }

    #[test]
    fn same_source_with_different_html_is_sent() {
        let mut shown = shown_blocks(&blocks("Text.\n"));
        shown[0].1 = "<p>rendered differently</p>".into();
        let mut edited = blocks("Text.\n");
        let (from, _) = reuse_html(&mut edited, &shown);
        assert_eq!(from, vec![None]);
        assert!(edited[0].html.contains("Text."));
    }
}
