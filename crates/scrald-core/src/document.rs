//! The document model sent to the frontend, and the pipeline that builds it
//! (DESIGN.md §3, "Data flow when opening a document").

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use comrak::nodes::{ListType, Node, NodeValue};
use serde::Serialize;

use crate::DocumentError;
use crate::frontmatter::{self, FrontMatter};
use crate::render::{self, Renderer};
use crate::source::{self, LineEnding, LineIndex, SourceRange};
use crate::toc::{Slugger, TocEntry};

/// Everything the reader needs to display a document.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentModel {
    pub path: PathBuf,
    pub has_bom: bool,
    pub line_ending: LineEnding,
    pub front_matter: Option<FrontMatter>,
    pub blocks: Vec<Block>,
    pub sections: Vec<Section>,
    pub toc: Vec<TocEntry>,
    pub word_count: usize,
    pub features: FeatureFlags,
}

/// One top-level Markdown block, rendered on its own.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// Position in `DocumentModel::blocks`; stable within one parse.
    pub id: u32,
    pub kind: BlockKind,
    /// Sanitized HTML for this block alone.
    pub html: String,
    /// The block's bytes in the original file.
    pub source: SourceRange,
    /// Hash of the block's source text, used to re-render only changed
    /// blocks after an edit. Only comparable within one app session.
    pub hash: u64,
    /// Index into `DocumentModel::sections`.
    pub section: u32,
}

/// What kind of block this is. Serialized as `{ "type": "heading", "level": 2 }`.
// Rust note: Rust enums can carry data per variant (a tagged union, like
// `std::variant` in C++). `match` must handle every variant, so adding one
// makes the compiler point at every place that needs updating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BlockKind {
    Heading {
        level: u8,
    },
    Paragraph,
    List {
        ordered: bool,
    },
    BlockQuote,
    CodeBlock {
        language: Option<String>,
    },
    Table,
    ThematicBreak,
    Html,
    FootnoteDefinition,
    DescriptionList,
    /// Anything else comrak produces (kept generic so new extensions still render).
    Other,
}

/// A run of blocks starting at a heading (or the start of the document).
/// The reader gives each section its own containment box (DESIGN.md §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub id: u32,
    /// The heading block that starts this section; `None` for text before
    /// the first heading.
    pub heading_block: Option<u32>,
    /// Heading level, or 0 for the preamble.
    pub level: u8,
    pub first_block: u32,
    pub block_count: u32,
}

/// Content the UI should lazy-load support for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlags {
    pub has_math: bool,
    pub has_mermaid: bool,
    pub has_code: bool,
}

/// Reads and parses a document from disk.
pub fn load_document(path: &Path) -> Result<DocumentModel, DocumentError> {
    let bytes = std::fs::read(path).map_err(|e| DocumentError::Io {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    parse_document(path.to_path_buf(), &bytes)
}

/// Comrak options for the GFM profile. Flavor profiles arrive in M4.
pub fn gfm_options() -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.math_dollars = true;
    options.extension.math_code = true;
    // Raw HTML is passed through to ammonia, which removes anything unsafe.
    // Without this, comrak would replace raw HTML with a comment instead.
    options.render.r#unsafe = true;
    options
}

/// Parses document bytes into the model. Never fails on Markdown content;
/// only invalid UTF-8 is an error.
pub fn parse_document(path: PathBuf, bytes: &[u8]) -> Result<DocumentModel, DocumentError> {
    let decoded = source::decode(bytes)?;
    let text = &decoded.text;
    let bom_len = decoded.bom_len();

    // Front matter, then the body that follows it.
    let split = frontmatter::split(text);
    let body_start = split.as_ref().map_or(0, |s| s.block.end);
    let front_matter = split.map(|s| {
        let range = SourceRange::new(bom_len + s.block.start, bom_len + s.block.end);
        frontmatter::parse(&text[s.yaml], range)
    });
    let body = &text[body_start..];

    let options = gfm_options();
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, body, &options);
    let index = LineIndex::new(body, bom_len + body_start);

    let renderer = Renderer::new();
    let mut slugger = Slugger::new();
    let mut blocks = Vec::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut toc = Vec::new();
    let mut word_count = 0;
    let mut features = FeatureFlags::default();

    // comrak moves footnote definitions to the end of the document. Put
    // blocks back in source order so ranges tile the file and edits splice
    // into the right place; the UI gathers footnotes into endnotes itself.
    // Rust note: `collect()` builds a Vec from the iterator; `sort_by_key`
    // then sorts in place by the key the closure returns (a tuple here).
    let mut nodes: Vec<Node<'_>> = root.children().collect();
    nodes.sort_by_key(|n| {
        let start = n.data().sourcepos.start;
        (start.line, start.column)
    });

    for node in nodes {
        let id = blocks.len() as u32;
        let kind = block_kind(node);
        let source = block_range(node, &index);

        // Original-file offsets include the BOM; subtract it to slice `text`.
        let source_text = &text[source.start - bom_len..source.end - bom_len];
        let mut html = renderer.render(node, &options);

        if let BlockKind::Heading { level } = kind {
            let heading = plain_text(node);
            let slug = slugger.slug(&heading);
            html = render::add_heading_id(&html, level, &slug);
            toc.push(TocEntry {
                level,
                text: heading,
                slug,
                block_id: id,
            });
            sections.push(Section {
                id: sections.len() as u32,
                heading_block: Some(id),
                level,
                first_block: id,
                block_count: 0,
            });
        } else if sections.is_empty() {
            sections.push(Section {
                id: 0,
                heading_block: None,
                level: 0,
                first_block: id,
                block_count: 0,
            });
        }

        // Rust note: `last_mut()` gives `Option<&mut Section>`, a mutable
        // reference we can update in place. `sections` is never empty here,
        // but matching on the Option avoids an `unwrap()`.
        let section = match sections.last_mut() {
            Some(section) => {
                section.block_count += 1;
                section.id
            }
            None => 0,
        };

        word_count += count_words(node);
        update_features(node, &mut features);

        blocks.push(Block {
            id,
            kind,
            html,
            source,
            hash: hash_text(source_text),
            section,
        });
    }

    Ok(DocumentModel {
        path,
        has_bom: decoded.has_bom,
        line_ending: decoded.line_ending,
        front_matter,
        blocks,
        sections,
        toc,
        word_count,
        features,
    })
}

fn block_kind(node: Node<'_>) -> BlockKind {
    // Rust note: `node.data()` borrows the node's contents (comrak keeps them
    // in a `RefCell`, which checks borrows at runtime instead of compile time).
    match &node.data().value {
        NodeValue::Heading(h) => BlockKind::Heading { level: h.level },
        NodeValue::Paragraph => BlockKind::Paragraph,
        NodeValue::List(list) => BlockKind::List {
            ordered: list.list_type == ListType::Ordered,
        },
        NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => BlockKind::BlockQuote,
        NodeValue::CodeBlock(code) => BlockKind::CodeBlock {
            language: code_language(&code.info),
        },
        NodeValue::Table(_) => BlockKind::Table,
        NodeValue::ThematicBreak => BlockKind::ThematicBreak,
        NodeValue::HtmlBlock(_) => BlockKind::Html,
        NodeValue::FootnoteDefinition(_) => BlockKind::FootnoteDefinition,
        NodeValue::DescriptionList => BlockKind::DescriptionList,
        _ => BlockKind::Other,
    }
}

/// The language word from a fence info string: "rust ignore" → "rust".
fn code_language(info: &str) -> Option<String> {
    info.split_whitespace().next().map(str::to_string)
}

/// Converts comrak's inclusive line/column span into a file byte range.
/// Top-level blocks always begin a line, so the range starts at column 1:
/// that keeps the indentation of indented code blocks (and the up-to-three
/// spaces any block may have) inside the block, where edits need it.
fn block_range(node: Node<'_>, index: &LineIndex) -> SourceRange {
    let pos = node.data().sourcepos;
    let start = index.start_offset(pos.start.line, 1);
    let end = index.end_offset(pos.end.line, pos.end.column);
    SourceRange::new(start, end.max(start))
}

/// The text content of a node, as shown to the reader (no markup).
fn plain_text(node: Node<'_>) -> String {
    let mut out = String::new();
    // Rust note: `descendants()` walks the subtree depth-first, starting with
    // `node` itself; it borrows the tree rather than copying it.
    for n in node.descendants() {
        match &n.data().value {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::Code(c) => out.push_str(&c.literal),
            NodeValue::Math(m) => out.push_str(&m.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => out.push(' '),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Words in prose and inline code. Code blocks, raw HTML, and math don't count.
fn count_words(node: Node<'_>) -> usize {
    node.descendants()
        .map(|n| match &n.data().value {
            NodeValue::Text(t) => words_in(t),
            NodeValue::Code(c) => words_in(&c.literal),
            _ => 0,
        })
        .sum()
}

/// A word is a whitespace-separated run containing at least one letter or digit.
fn words_in(text: &str) -> usize {
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

fn update_features(node: Node<'_>, features: &mut FeatureFlags) {
    for n in node.descendants() {
        match &n.data().value {
            NodeValue::Math(_) => features.has_math = true,
            NodeValue::CodeBlock(code) => {
                features.has_code = true;
                if code_language(&code.info).as_deref() == Some("mermaid") {
                    features.has_mermaid = true;
                }
            }
            _ => {}
        }
    }
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> DocumentModel {
        parse_document(PathBuf::from("test.md"), text.as_bytes()).unwrap()
    }

    fn slices(doc: &DocumentModel, bytes: &[u8]) -> Vec<String> {
        doc.blocks
            .iter()
            .map(|b| String::from_utf8_lossy(&bytes[b.source.as_range()]).into_owned())
            .collect()
    }

    #[test]
    fn blocks_and_ranges_lf() {
        let text = "# Title\n\nSome *text*\nmore.\n\n- a\n- b\n";
        let doc = parse(text);
        assert_eq!(
            slices(&doc, text.as_bytes()),
            ["# Title", "Some *text*\nmore.", "- a\n- b"]
        );
        assert_eq!(doc.blocks[0].kind, BlockKind::Heading { level: 1 });
        assert_eq!(doc.blocks[2].kind, BlockKind::List { ordered: false });
    }

    #[test]
    fn ranges_with_bom_crlf_front_matter_and_multibyte() {
        let bytes =
            b"\xEF\xBB\xBF---\r\ntitle: Sk\xC3\xA5l\r\n---\r\n# H\xC3\xA9\r\n\r\npara \xC3\xA5\r\n";
        let doc = parse_document(PathBuf::from("t.md"), bytes).unwrap();
        assert!(doc.has_bom);
        assert_eq!(doc.line_ending, LineEnding::Crlf);
        let fm = doc.front_matter.as_ref().unwrap();
        assert_eq!(fm.title.as_deref(), Some("Skål"));
        assert_eq!(fm.range, SourceRange::new(3, 27));
        assert_eq!(slices(&doc, bytes), ["# Hé", "para å"]);
    }

    #[test]
    fn sections_follow_headings() {
        let doc = parse("intro\n\n# One\n\na\n\nb\n\n## Two\n\nc\n");
        let s: Vec<(Option<u32>, u8, u32, u32)> = doc
            .sections
            .iter()
            .map(|s| (s.heading_block, s.level, s.first_block, s.block_count))
            .collect();
        assert_eq!(s, [(None, 0, 0, 1), (Some(1), 1, 1, 3), (Some(4), 2, 4, 2)]);
        let block_sections: Vec<u32> = doc.blocks.iter().map(|b| b.section).collect();
        assert_eq!(block_sections, [0, 1, 1, 1, 2, 2]);
    }

    #[test]
    fn toc_and_heading_ids() {
        let doc = parse("# Intro\n\n## Intro\n\n## `code` & *more*\n");
        let toc: Vec<(&str, &str)> = doc
            .toc
            .iter()
            .map(|t| (t.text.as_str(), t.slug.as_str()))
            .collect();
        assert_eq!(
            toc,
            [
                ("Intro", "intro"),
                ("Intro", "intro-1"),
                ("code & more", "code--more")
            ]
        );
        assert!(doc.blocks[1].html.starts_with("<h2 id=\"intro-1\">"));
    }

    #[test]
    fn word_count_skips_code_blocks() {
        let doc = parse("Hello brave new world.\n\n```\nnot counted here\n```\n\nUse `x` - ok\n");
        assert_eq!(doc.word_count, 4 + 2 + 1);
    }

    #[test]
    fn feature_flags() {
        assert_eq!(parse("plain").features, FeatureFlags::default());
        let doc = parse("$x^2$\n\n```mermaid\ngraph TD\n```\n");
        assert!(doc.features.has_math && doc.features.has_mermaid && doc.features.has_code);
    }

    #[test]
    fn raw_html_is_sanitized() {
        let doc = parse("<div onclick=\"x()\">hi</div>\n\n<script>alert(1)</script>\n");
        assert!(
            doc.blocks
                .iter()
                .all(|b| !b.html.contains("script") && !b.html.contains("onclick"))
        );
    }

    #[test]
    fn hash_changes_with_source() {
        let a = parse("one\n\ntwo\n");
        let b = parse("one\n\nTWO\n");
        assert_eq!(a.blocks[0].hash, b.blocks[0].hash);
        assert_ne!(a.blocks[1].hash, b.blocks[1].hash);
    }

    #[test]
    fn invalid_utf8_is_an_error() {
        assert!(parse_document(PathBuf::from("x.md"), b"ok \xFF").is_err());
    }
}
