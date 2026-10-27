//! Note transclusion (#117): an Obsidian embed of a note on a line of its
//! own (`![[Other note]]`, `![[Note#Heading]]`, `![[Note#^block-id]]`) shows
//! that note's content in place, read-only (DESIGN.md §5.3). The note is
//! parsed on its own (with its own flavor, images, and embeds) and its HTML
//! is sanitized again for its new home: ids are dropped, links are made
//! absolute, and image ids are renumbered into the document's.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::assets::ImageCollector;
use crate::document::{BlockKind, DocumentModel, FeatureFlags, ParseOptions, parse_document_with};
use crate::links::{self, LinkTarget};
use crate::obsidian::{self, Piece, VaultIndex};
use crate::render::{self, escape_text};

/// How deeply embeds nest: a note embedded in a note embedded in the
/// document is two levels. Deeper ones stay link cards.
pub const MAX_DEPTH: usize = 3;

/// At most this many notes are shown per document, counting nested ones,
/// so a note that embeds many notes that embed many more can't stall a parse.
pub const MAX_PER_DOCUMENT: usize = 30;

/// The embed a paragraph consists of, if that's all it holds: its target
/// (`Note#Heading`) and the text after `|`, if any.
pub fn lone_embed(paragraph_source: &str) -> Option<(String, Option<String>)> {
    match obsidian::split_inline(paragraph_source.trim(), false).as_slice() {
        [Piece::Embed { target, options }] => Some((target.clone(), options.clone())),
        _ => None,
    }
}

/// Whether an embed's target names a note rather than an image or a PDF.
fn is_note(name: &str) -> bool {
    match Path::new(name.trim()).extension() {
        None => true,
        Some(ext) => links::is_markdown(Path::new(&format!("x.{}", ext.to_string_lossy()))),
    }
}

/// What the embedding document knows when it meets an embed.
pub struct EmbedContext<'c> {
    pub doc_path: &'c Path,
    pub vault: &'c Arc<VaultIndex>,
    pub allow_remote_images: bool,
    /// The document and the notes it's embedded in, innermost last.
    pub chain: &'c [PathBuf],
}

/// A note shown in place.
#[derive(Debug, Default)]
pub struct Embedded {
    /// The block's HTML: the note's content in a titled box, or a notice.
    pub html: String,
    /// What the note's content needs loaded (math, diagrams, ...).
    pub features: FeatureFlags,
    /// Every note file shown, nested ones included, for live reload.
    pub files: Vec<PathBuf>,
    /// How many notes were shown, nested ones included.
    pub count: usize,
}

/// The note `target` names, ready to show in place of the embed, or `None`
/// when it stays a link card: not a note, not found, nested too deeply, or
/// past the document's limit (`budget` is how many it may still show).
/// Local images in it are added to `images`, so the document serves them.
pub fn transclude(
    target: &str,
    label: Option<&str>,
    ctx: &EmbedContext<'_>,
    budget: usize,
    images: &mut ImageCollector,
) -> Option<Embedded> {
    let (name, fragment) = obsidian::split_target(target);
    if !is_note(name) || budget == 0 || ctx.chain.len() > MAX_DEPTH {
        return None;
    }
    let path = ctx.vault.resolve(name, ctx.doc_path)?;
    let title = label.map_or_else(|| obsidian::display_label(target), str::to_string);

    // Rust note: `Iterator::any` stops at the first item that passes, like
    // Python's `any()` over a generator.
    if ctx.chain.iter().any(|p| same_file(p, &path)) {
        return Some(notice(&format!(
            "\u{201c}{title}\u{201d} is already shown around this, so it isn't shown again inside itself"
        )));
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Some(notice(&format!(
                "Couldn't read \u{201c}{title}\u{201d}: {error}"
            )));
        }
    };
    let mut chain = ctx.chain.to_vec();
    chain.push(path.clone());
    let options = ParseOptions {
        allow_remote_images: ctx.allow_remote_images,
        flavor: None,
        vault: Some(Arc::clone(ctx.vault)),
        embedded_in: chain,
        embed_budget: Some(budget - 1),
    };
    let note = match parse_document_with(path.clone(), &bytes, &options) {
        Ok(note) => note,
        Err(error) => {
            return Some(notice(&format!(
                "Couldn't show \u{201c}{title}\u{201d}: {error}"
            )));
        }
    };

    let Some(range) = selected_blocks(&note, &bytes, fragment) else {
        let part = fragment.unwrap_or_default().trim_start_matches('^');
        return Some(notice(&format!(
            "\u{201c}{name}\u{201d} has no \u{201c}{part}\u{201d}"
        )));
    };
    let mut body = String::new();
    for block in &note.blocks[range] {
        if block.kind != BlockKind::FootnoteDefinition {
            body.push_str(&block.html);
        }
    }
    // The footnotes its references point to, as a short list at the end.
    let notes: Vec<_> = note
        .footnotes
        .iter()
        .filter(|f| body.contains(&format!("href=\"#fn-{}\"", f.name)))
        .collect();

    // Images it uses get ids in the document's list.
    let mut image_ids = HashMap::new();
    for image in &note.images {
        if body.contains(&format!("data-asset=\"{}\"", image.id))
            || notes
                .iter()
                .any(|f| f.html.contains(&format!("data-asset=\"{}\"", image.id)))
        {
            image_ids.insert(image.id, images.asset_for(image.path.clone()).id);
        }
    }
    images.remote_count += note.remote_images;

    let sanitizer = embed_sanitizer(path.clone(), image_ids, ctx.allow_remote_images);
    let mut notes_html = String::new();
    if !notes.is_empty() {
        notes_html.push_str("<ol class=\"sk-embed-notes\">");
        for footnote in &notes {
            let html = sanitizer.clean(&footnote.html).to_string();
            notes_html.push_str(&format!("<li value=\"{}\">{html}</li>", footnote.number));
        }
        notes_html.push_str("</ol>");
    }
    let body = sanitizer.clean(&body).to_string();
    let open = obsidian::file_url(&path, fragment.map(obsidian::fragment_id).as_deref());
    let html = format!(
        "<div class=\"sk-embed\"><div class=\"sk-embed-title\"><a href=\"{}\" title=\"Open this note\">{}</a></div>\
         <div class=\"sk-embed-body\">{body}{notes_html}</div></div>",
        escape_text(&open),
        escape_text(&title),
    );

    let mut files = vec![path];
    files.extend(note.embeds);
    Some(Embedded {
        html,
        features: note.features,
        files,
        count: 1 + note.transclusions,
    })
}

/// The blocks `fragment` picks out of a note: all of them, a heading's
/// section (up to the next heading of the same or a higher level), or the
/// block a `^block-id` marks. `None` when the heading or block isn't there.
/// `bytes` is the note's file, which block source ranges point into.
fn selected_blocks(
    note: &DocumentModel,
    bytes: &[u8],
    fragment: Option<&str>,
) -> Option<std::ops::Range<usize>> {
    let all = 0..note.blocks.len();
    let Some(fragment) = fragment.map(str::trim).filter(|f| !f.is_empty()) else {
        return Some(all);
    };
    if let Some(block_id) = fragment.strip_prefix('^') {
        // Read from the source: the marker ends the block's text.
        let marker = format!("^{block_id}");
        let i = note.blocks.iter().position(|b| {
            let text = String::from_utf8_lossy(
                bytes.get(b.source.start..b.source.end).unwrap_or_default(),
            );
            text.trim_end()
                .strip_suffix(&marker)
                .is_some_and(|before| before.is_empty() || before.ends_with(char::is_whitespace))
        })?;
        return Some(i..i + 1);
    }
    let slug = obsidian::fragment_id(fragment);
    let at = note
        .toc
        .iter()
        .position(|e| e.slug == slug || e.text.eq_ignore_ascii_case(fragment))?;
    let entry = &note.toc[at];
    let start = entry.block_id as usize;
    let end = note.toc[at + 1..]
        .iter()
        .find(|e| e.level <= entry.level && e.block_id as usize > start)
        .map_or(all.end, |e| e.block_id as usize);
    Some(start..end)
}

/// Re-sanitizes a note's HTML for the document it's shown in: the same
/// allowlist, plus a filter that drops ids (they'd clash with the
/// document's, and its Contents leaves the note out), makes links relative
/// to the note absolute, and renumbers images.
fn embed_sanitizer(
    note_path: PathBuf,
    image_ids: HashMap<u32, u32>,
    allow_remote_images: bool,
) -> ammonia::Builder<'static> {
    let mut sanitizer = render::allowlist();
    // Rust note: the closure must be `'static` (ammonia keeps it), so `move`
    // gives it its own copies of the path and the id map.
    sanitizer.attribute_filter(move |element, attribute, value| {
        let value = render::filter_attribute(element, attribute, value, allow_remote_images)?;
        match (element, attribute) {
            (_, "id") => None,
            ("img", "data-asset") => {
                let id = value.parse::<u32>().ok()?;
                image_ids.get(&id).map(|new| Cow::Owned(new.to_string()))
            }
            ("a", "href") => Some(absolute_link(&note_path, value)),
            _ => Some(Cow::Borrowed(value)),
        }
    });
    sanitizer
}

/// A link written in the note at `note_path`, as one that works from
/// anywhere: `#part` and relative paths become `file://` URLs, so a click
/// opens the note (at that heading) or the file it names.
// Rust note: `Cow` ("clone on write") is either a borrowed `&str` or an
// owned `String`, so links that need no change are returned without a copy.
fn absolute_link<'v>(note_path: &Path, href: &'v str) -> Cow<'v, str> {
    if links::scheme_of(href.trim()).is_some() {
        return Cow::Borrowed(href);
    }
    match links::resolve_link(note_path, href) {
        LinkTarget::Fragment { id } => Cow::Owned(obsidian::file_url(note_path, Some(&id))),
        LinkTarget::Document { path, fragment } => {
            Cow::Owned(obsidian::file_url(&path, fragment.as_deref()))
        }
        LinkTarget::LocalFile { path } => Cow::Owned(obsidian::file_url(&path, None)),
        _ => Cow::Borrowed(href),
    }
}

/// A box saying why a note isn't shown.
fn notice(text: &str) -> Embedded {
    Embedded {
        html: format!(
            "<div class=\"sk-embed sk-embed-notice\">{}</div>",
            escape_text(text)
        ),
        ..Embedded::default()
    }
}

/// Whether two paths name the same file (the same once resolved, or
/// identical when either can't be).
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse_document;

    /// A vault in a temp folder with these notes; returns its root.
    fn vault(name: &str, notes: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("scrald-embed-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".obsidian")).unwrap();
        for (file, text) in notes {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        root
    }

    fn open(root: &Path, file: &str) -> DocumentModel {
        let path = root.join(file);
        let bytes = std::fs::read(&path).unwrap();
        parse_document(path, &bytes).unwrap()
    }

    #[test]
    fn a_lone_embed_is_recognized() {
        assert_eq!(
            lone_embed("  ![[Note#Part|Seen]] \n"),
            Some(("Note#Part".into(), Some("Seen".into())))
        );
        assert_eq!(lone_embed("See ![[Note]]"), None);
        assert_eq!(lone_embed("![[A]] ![[B]]"), None);
        assert!(is_note("Note") && is_note("Note.md") && is_note("dir/Note.markdown"));
        assert!(!is_note("pic.png") && !is_note("paper.pdf"));
    }

    #[test]
    fn a_note_is_shown_in_place() {
        let root = vault(
            "whole",
            &[
                ("Index.md", "# Index\n\n![[Saga]]\n\nAfter.\n"),
                (
                    "Saga.md",
                    "## The ships\n\nLongships sailed [home](Home.md) and [down](#the-ships).\n",
                ),
            ],
        );
        let doc = open(&root, "Index.md");
        let html = &doc.blocks[1].html;
        assert!(html.contains("class=\"sk-embed\""), "{html}");
        assert!(html.contains("Longships sailed"));
        // The note's heading has no id, so the document's Contents leaves it out.
        assert!(html.contains("<h2>The ships</h2>"), "{html}");
        assert_eq!(doc.toc.len(), 1);
        // Links from the note work from the document.
        assert!(html.contains("Home.md\""), "{html}");
        assert!(html.contains("Saga.md#the-ships\""), "{html}");
        // The block is still the embed line, for editing.
        assert_eq!(doc.blocks[1].kind, BlockKind::Paragraph);
        assert_eq!(doc.embeds, vec![root.join("Saga.md")]);
    }

    #[test]
    fn a_heading_or_block_is_shown_alone() {
        let root = vault(
            "parts",
            &[
                (
                    "Index.md",
                    "![[Saga#Second]]\n\n![[Saga#^key]]\n\n![[Saga#Missing]]\n",
                ),
                (
                    "Saga.md",
                    "# First\n\nOne.\n\n# Second\n\nTwo.\n\n## Inside\n\nStill two.\n\n# Third\n\nThe key line. ^key\n\nThree.\n",
                ),
            ],
        );
        let doc = open(&root, "Index.md");
        let second = &doc.blocks[0].html;
        assert!(
            second.contains("Two.") && second.contains("Still two."),
            "{second}"
        );
        assert!(
            !second.contains("One.") && !second.contains("Three."),
            "{second}"
        );
        let key = &doc.blocks[1].html;
        assert!(
            key.contains("The key line.") && !key.contains("Three."),
            "{key}"
        );
        assert!(
            doc.blocks[2].html.contains("sk-embed-notice"),
            "{}",
            doc.blocks[2].html
        );
    }

    #[test]
    fn loops_and_depth_are_limited() {
        let root = vault(
            "loops",
            &[
                ("A.md", "A text.\n\n![[B]]\n"),
                ("B.md", "B text.\n\n![[A]]\n"),
                ("Deep1.md", "One.\n\n![[Deep2]]\n"),
                ("Deep2.md", "Two.\n\n![[Deep3]]\n"),
                ("Deep3.md", "Three.\n\n![[Deep4]]\n"),
                ("Deep4.md", "Four.\n\n![[Deep5]]\n"),
                ("Deep5.md", "Five.\n"),
            ],
        );
        let a = open(&root, "A.md");
        let html = &a.blocks[1].html;
        assert!(
            html.contains("B text.") && html.contains("shown again inside itself"),
            "{html}"
        );

        let deep = open(&root, "Deep1.md");
        let html = &deep.blocks[1].html;
        assert!(html.contains("Two.") && html.contains("Three."), "{html}");
        assert!(html.contains("Four."), "{html}");
        // A fourth level, Deep5, is a link card.
        assert!(
            !html.contains("Five.") && html.contains("sk-embed-card"),
            "{html}"
        );
    }

    #[test]
    fn images_and_footnotes_come_along() {
        let root = vault(
            "images",
            &[
                ("Index.md", "![[pic.png]]\n\n![[notes/Saga]]\n"),
                ("pic.png", ""),
                (
                    "notes/Saga.md",
                    "A ship.[^1] ![[boat.png]]\n\n[^1]: Built of oak.\n",
                ),
                ("notes/boat.png", ""),
            ],
        );
        let doc = open(&root, "Index.md");
        let html = &doc.blocks[1].html;
        let boat = doc
            .images
            .iter()
            .find(|i| i.path.ends_with("boat.png"))
            .unwrap();
        assert!(
            html.contains(&format!("data-asset=\"{}\"", boat.id)),
            "{html}"
        );
        assert!(
            html.contains("sk-embed-notes") && html.contains("Built of oak."),
            "{html}"
        );
    }

    #[test]
    fn mid_sentence_embeds_and_other_flavors_stay_cards() {
        let root = vault(
            "inline",
            &[
                ("Index.md", "See ![[Saga]] here.\n"),
                ("Saga.md", "Saga text.\n"),
            ],
        );
        let doc = open(&root, "Index.md");
        assert!(doc.blocks[0].html.contains("sk-embed-card"));
        assert!(!doc.blocks[0].html.contains("Saga text."));
    }
}
