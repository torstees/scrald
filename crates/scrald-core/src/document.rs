//! The document model sent to the frontend, and the pipeline that builds it
//! (DESIGN.md §3, "Data flow when opening a document").

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use comrak::nodes::{Attributes, ListType, Node, NodeHtmlBlock, NodeValue};
use serde::Serialize;

use crate::DocumentError;
use crate::assets::{AssetContext, ImageAsset, ImageCollector};
use crate::flavor::{self, Flavor, FlavorSource};
use crate::frontmatter::{self, FrontMatter};
use crate::highlight;
use crate::obsidian::{self, VaultIndex};
use crate::pandoc;
use crate::render::{self, Renderer};
use crate::source::{self, LineEnding, LineIndex, SourceRange};
use crate::toc::{self, Slugger, TocEntry};

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
    /// Local image files the document uses. Block HTML refers to them as
    /// `<img data-asset="id">`; only these files may be served (§6.2).
    pub images: Vec<ImageAsset>,
    /// How many remote (`http(s)`) images the document contains.
    pub remote_images: usize,
    /// Whether remote images were allowed to load in this parse.
    pub remote_images_allowed: bool,
    /// The Markdown flavor the document was parsed as.
    pub flavor: Flavor,
    /// Why that flavor was chosen.
    pub flavor_source: FlavorSource,
    /// Every footnote, in numbered order, for popovers and the endnotes
    /// section. Includes inline footnotes (`^[...]`, Pandoc), which have no
    /// block of their own.
    pub footnotes: Vec<Footnote>,
}

/// A footnote, rendered for display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Footnote {
    /// The footnote's name: `source` for `[^source]`, or comrak's generated
    /// `__inline_1`. References link to `#fn-<name>`, and the first
    /// reference has the id `fnref-<name>`.
    pub name: String,
    /// The number shown at its references (1, 2, ...).
    pub number: u32,
    /// Sanitized HTML of the footnote's contents, without back-links.
    pub html: String,
    /// The block that defines it, or `None` for an inline footnote.
    pub block_id: Option<u32>,
}

/// Choices that change how a document is parsed and rendered.
#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    /// Load remote images (off by default, for privacy; DESIGN.md §6.1).
    pub allow_remote_images: bool,
    /// The flavor the user chose for this document, overriding front
    /// matter, folder config, and detection (DESIGN.md §5.2).
    pub flavor: Option<Flavor>,
    /// A prebuilt vault index (the app caches one per vault), used when it
    /// covers this document; otherwise one is built during the parse.
    pub vault: Option<Arc<VaultIndex>>,
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
    /// ABC music notation in ```abc fences (rendered with abcjs).
    pub has_abc: bool,
}

/// Reads and parses a document from disk.
pub fn load_document(path: &Path, options: &ParseOptions) -> Result<DocumentModel, DocumentError> {
    let bytes = std::fs::read(path).map_err(|e| DocumentError::Io {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    parse_document_with(path.to_path_buf(), &bytes, options)
}

/// Parses document bytes with default options. See `parse_document_with`.
pub fn parse_document(path: PathBuf, bytes: &[u8]) -> Result<DocumentModel, DocumentError> {
    parse_document_with(path, bytes, &ParseOptions::default())
}

/// Parses document bytes into the model. Never fails on Markdown content;
/// only invalid UTF-8 is an error. Image paths are resolved relative to
/// `path`, so it should be the document's real location.
pub fn parse_document_with(
    path: PathBuf,
    bytes: &[u8],
    parse_options: &ParseOptions,
) -> Result<DocumentModel, DocumentError> {
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

    let fm_flavor = front_matter.as_ref().and_then(|fm| fm.flavor.clone());
    let folder_flavor = crate::config::find_folder_config(&path)
        .and_then(|found| found.config.ok())
        .and_then(|config| config.flavor);
    let (flavor, flavor_source) = flavor::resolve(
        parse_options.flavor,
        fm_flavor.as_deref(),
        folder_flavor.as_deref(),
        || flavor::detect(text, flavor::in_obsidian_vault(&path)).0,
    );

    let options = flavor::comrak_options(flavor);
    let obsidian = flavor == Flavor::Obsidian;
    let pandoc = flavor == Flavor::Pandoc;
    // Obsidian comments are blanked to spaces of the same byte length, so the
    // parser never sees them but every offset stays the same.
    // Pandoc `:::` div fences are blanked the same way, and the divs kept
    // aside (with file offsets) to wrap the blocks inside them.
    let mut divs = Vec::new();
    let parse_text = if obsidian {
        obsidian::blank_comments(body)
    } else if pandoc {
        let (blanked, spans) = pandoc::extract_divs(body);
        let base = bom_len + body_start;
        // Rust note: `extend` with a `map` appends each converted span; the
        // closure takes each `DivSpan` by value (`into_iter` moves them).
        divs.extend(spans.into_iter().map(|mut span| {
            span.start += base;
            span.end += base;
            span
        }));
        blanked
    } else {
        body.to_string()
    };
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, &parse_text, &options);
    let index = LineIndex::new(body, bom_len + body_start);

    let assets_dir = front_matter.as_ref().and_then(|fm| fm.assets.as_deref());
    let mut asset_ctx =
        AssetContext::for_document(&path, assets_dir, parse_options.allow_remote_images);
    // The vault index is only built when something could use it.
    let vault = (obsidian && (body.contains("[[") || body.contains("![") || body.contains("<img")))
        .then(|| match &parse_options.vault {
            Some(cached) if cached.serves(&path) => Arc::clone(cached),
            _ => Arc::new(VaultIndex::for_document(&path)),
        });
    asset_ctx.vault = vault.clone();
    let mut images = ImageCollector::default();

    let renderer = Renderer::new(parse_options.allow_remote_images);
    let mut slugger = Slugger::new();
    let mut blocks = Vec::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut toc = Vec::new();
    let mut word_count = 0;
    let mut features = FeatureFlags::default();
    // Headings marked `{-}` or `{.unnumbered}` (Pandoc) get no outline number.
    let mut unnumbered: Vec<u32> = Vec::new();

    // comrak moves footnote definitions to the end of the document. Put
    // blocks back in source order so ranges tile the file and edits splice
    // into the right place; the UI gathers footnotes into endnotes itself.
    // Rust note: `collect()` builds a Vec from the iterator; `sort_by_key`
    // then sorts in place by the key the closure returns (a tuple here).
    // Pandoc's inline footnotes (`^[...]`) make comrak add a definition for
    // each at the end of the document, positioned inside the paragraph that
    // holds the note. They have no source text of their own, so they aren't
    // blocks (they would overlap that paragraph's range); their rendered
    // HTML is kept separately for footnote popovers and endnotes.
    // Rust note: `partition` splits one iterator into two collections by a
    // test, like two list comprehensions in one pass.
    let (generated, mut nodes): (Vec<Node<'_>>, Vec<Node<'_>>) =
        root.children().partition(|n| is_generated_footnote(n));
    let footnote_numbers = footnote_numbers(root);
    let mut footnotes: Vec<Footnote> = Vec::new();
    nodes.sort_by_key(|n| {
        let start = n.data().sourcepos.start;
        (start.line, start.column)
    });

    for group in group_blocks(&nodes, pandoc, &divs, &index) {
        let id = blocks.len() as u32;
        // Rust note: slice patterns: `[only]` matches a one-element slice and
        // binds its element; `[table, caption]` binds both of a pair; `_`
        // takes every other length.
        let mut table_attrs = None;
        let first_start = block_range(group[0], &index).start;
        let outer_div = divs
            .iter()
            .find(|d| d.start <= first_start && first_start <= d.end);
        let (kind, source) = match group {
            // A fenced div (and everything in it) is one block, fences included.
            _ if outer_div.is_some() => {
                let last_end = block_range(group[group.len() - 1], &index).end;
                let div_range = outer_div.map_or((first_start, last_end), |d| (d.start, d.end));
                (
                    BlockKind::Other,
                    SourceRange::new(div_range.0.min(first_start), div_range.1.max(last_end)),
                )
            }
            [only] => (block_kind(only), block_range(only, &index)),
            [table, caption] if pandoc && pandoc::is_table_caption(caption) => {
                table_attrs = Some(pandoc::take_table_caption(caption));
                let range = SourceRange::new(
                    block_range(table, &index).start,
                    block_range(caption, &index).end,
                );
                (BlockKind::Table, range)
            }
            _ => {
                let first = block_range(group[0], &index);
                let last = block_range(group[group.len() - 1], &index);
                (BlockKind::Html, SourceRange::new(first.start, last.end))
            }
        };

        // Original-file offsets include the BOM; subtract it to slice `text`.
        let source_text = &text[source.start - bom_len..source.end - bom_len];

        if pandoc
            && let [only] = group
            && matches!(only.data().value, NodeValue::Heading(_))
        {
            pandoc::take_heading_attributes(only);
        }
        // Read the heading text first: rewriting images replaces their alt
        // text nodes with HTML.
        let heading_text = match group {
            [only] if matches!(kind, BlockKind::Heading { .. }) => Some(plain_text(only)),
            _ => None,
        };
        let heading_attrs = match group {
            [only] if pandoc && heading_text.is_some() => pandoc::attributes_of(only),
            _ => None,
        };
        for &node in group {
            word_count += count_words(node);
            update_features(node, &mut features);
            if let Some(vault) = &vault {
                let link_ctx = obsidian::LinkContext {
                    doc_path: &path,
                    vault,
                };
                obsidian::transform_links(&arena, node, &link_ctx, &mut images);
            }
            let figures = if pandoc {
                pandoc::transform_inlines(&arena, node);
                pandoc::plan_figures(node)
            } else {
                Vec::new()
            };
            images.rewrite(node, &asset_ctx);
            pandoc::finish_figures(figures, &options);
            highlight_code_blocks(node);
            if obsidian {
                obsidian::transform_callouts(node, &options);
            }
        }
        let mut html = if outer_div.is_some() {
            let wraps = div_wraps(group, &divs, &index);
            renderer.render_group_wrapped(group, &options, &wraps)
        } else {
            renderer.render_group(group, &options)
        };
        if kind == BlockKind::Table {
            // A Pandoc caption (`Table: ... {.center}`) goes inside the
            // table; its id goes on the table and its classes on the box.
            let attrs = table_attrs.unwrap_or_default();
            html = pandoc::move_caption_into_table(&html);
            let id_only = Attributes {
                id: attrs.id.clone(),
                ..Attributes::default()
            };
            html = pandoc::add_to_first_tag(&html, &id_only);
            let classes = Attributes {
                classes: attrs.classes,
                ..Attributes::default()
            };
            // Wide tables scroll sideways in their own box instead of
            // widening the page (DESIGN.md §6.3).
            html = format!(
                "<div{}>{html}</div>",
                pandoc::attribute_html(&classes, Some("sk-table"), "")
            );
        }

        if let (BlockKind::Heading { level }, Some(heading)) = (&kind, heading_text) {
            let level = *level;
            // A Pandoc `{#id}` replaces the generated slug.
            let attrs = heading_attrs.unwrap_or_default();
            let slug = match &attrs.id {
                Some(id) => id.clone(),
                None => slugger.slug(&heading),
            };
            html = render::add_heading_id(&html, level, &slug);
            let classes = Attributes {
                classes: attrs.classes,
                ..Attributes::default()
            };
            if classes.classes.iter().any(|c| c == "unnumbered") {
                unnumbered.push(id);
            }
            html = pandoc::add_to_first_tag(&html, &classes);
            toc.push(TocEntry {
                level,
                text: heading,
                slug,
                block_id: id,
                number: None,
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

        if let [only] = group
            && let NodeValue::FootnoteDefinition(def) = &only.data().value
        {
            footnotes.push(Footnote {
                name: def.name.clone(),
                number: footnote_numbers.get(&def.name).copied().unwrap_or(0),
                html: footnote_html(&renderer, only, &options),
                block_id: Some(id),
            });
        }

        blocks.push(Block {
            id,
            kind,
            html,
            source,
            hash: hash_text(source_text),
            section,
        });
    }

    // Outline numbers need every heading, so they're added afterwards.
    // Rust note: `filter` on `iter_mut()` yields `&mut TocEntry` items, so the
    // loop below can update the entries it numbers.
    let levels: Vec<u8> = toc
        .iter()
        .filter(|entry| !unnumbered.contains(&entry.block_id))
        .map(|entry| entry.level)
        .collect();
    let numbered = toc
        .iter_mut()
        .filter(|entry| !unnumbered.contains(&entry.block_id));
    for (entry, number) in numbered.zip(toc::number_headings(&levels)) {
        if let (Some(n), Some(block)) = (&number, blocks.get_mut(entry.block_id as usize)) {
            block.html = render::add_heading_number(&block.html, entry.level, n);
        }
        entry.number = number;
    }

    // Inline footnotes have no block; their definitions still get the
    // image pass, so a local image inside one resolves like any other.
    for node in generated {
        images.rewrite(node, &asset_ctx);
        if let NodeValue::FootnoteDefinition(def) = &node.data().value {
            footnotes.push(Footnote {
                name: def.name.clone(),
                number: footnote_numbers.get(&def.name).copied().unwrap_or(0),
                html: footnote_html(&renderer, node, &options),
                block_id: None,
            });
        }
    }
    footnotes.sort_by_key(|f| f.number);

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
        images: images.assets,
        remote_images: images.remote_count,
        remote_images_allowed: parse_options.allow_remote_images,
        flavor,
        flavor_source,
        footnotes,
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

/// Container tags whose open/close balance is tracked across HTML blocks.
const CONTAINER_TAGS: &[&str] = &[
    "details",
    "div",
    "section",
    "aside",
    "figure",
    "article",
    "blockquote",
    "center",
];

/// Groups top-level nodes so that a raw HTML block which opens a container
/// (`<details>`, `<div>`, ...) without closing it is joined with the blocks
/// that follow, up to the block that closes it. Each group becomes one
/// `Block`, sanitized as a whole, so the container wraps its Markdown
/// content instead of being closed early by the sanitizer. Every other node
/// is a group of one.
// Rust note: the result borrows from `nodes` (the slices point into it), so
// it can't outlive it; Rust infers that link from the single borrowed input.
fn group_html_runs<'n, 'a>(nodes: &'n [Node<'a>]) -> Vec<&'n [Node<'a>]> {
    let mut groups = Vec::new();
    let mut start = 0;
    let mut depth: i32 = 0;
    for (i, node) in nodes.iter().enumerate() {
        if let NodeValue::HtmlBlock(html) = &node.data().value {
            depth = (depth + container_depth_change(&html.literal)).max(0);
        }
        if depth == 0 {
            groups.push(&nodes[start..=i]);
            start = i + 1;
        }
    }
    // A container never closed runs to the end of the document.
    if start < nodes.len() {
        groups.push(&nodes[start..]);
    }
    groups
}

/// Net number of container tags opened (minus closed) in raw HTML.
fn container_depth_change(html: &str) -> i32 {
    let lower = html.to_ascii_lowercase();
    let mut change = 0;
    for tag in CONTAINER_TAGS {
        change += count_tags(&lower, &format!("<{tag}")) - count_tags(&lower, &format!("</{tag}"));
    }
    change
}

/// Occurrences of `prefix` that are followed by the end of a tag name
/// (so `<div` doesn't also count `<divider`).
fn count_tags(html: &str, prefix: &str) -> i32 {
    html.match_indices(prefix)
        .filter(|(i, _)| {
            html[i + prefix.len()..]
                .chars()
                .next()
                .is_none_or(|c| c == '>' || c == '/' || c.is_whitespace())
        })
        .count() as i32
}

/// Replaces fenced code in a known language with syntax-highlighted HTML
/// (class-based; colors come from the theme). Unknown languages, frontend-
/// rendered fences (math, mermaid, abc), and very large blocks stay as they are.
fn highlight_code_blocks(node: Node<'_>) {
    let code_nodes: Vec<Node<'_>> = node
        .descendants()
        .filter(|n| matches!(n.data().value, NodeValue::CodeBlock(_)))
        .collect();
    for code_node in code_nodes {
        // Pandoc attributes (```` ```rust {#id .class} ````) go on `<pre>`.
        let extra = crate::pandoc::attributes_of(code_node)
            .map(|attrs| crate::pandoc::attribute_html(&attrs, None, ""))
            .unwrap_or_default();
        let highlighted = match &code_node.data().value {
            NodeValue::CodeBlock(code) => code_language(&code.info).and_then(|language| {
                highlight::highlight(&code.literal, &language).map(|spans| (language, spans))
            }),
            _ => None,
        };
        let literal = match (highlighted, &code_node.data().value) {
            (Some((language, spans)), _) => Some(format!(
                "<pre{extra}><code class=\"language-{} sk-highlighted\">{spans}</code></pre>\n",
                render::escape_text(&language)
            )),
            // Not highlighted, but it has attributes to show.
            (None, NodeValue::CodeBlock(code)) if !extra.is_empty() => {
                let class = code_language(&code.info)
                    .map(|l| format!(" class=\"language-{}\"", render::escape_text(&l)))
                    .unwrap_or_default();
                Some(format!(
                    "<pre{extra}><code{class}>{}</code></pre>\n",
                    render::escape_text(&code.literal)
                ))
            }
            _ => None,
        };
        if let Some(literal) = literal {
            code_node.data_mut().value = NodeValue::HtmlBlock(NodeHtmlBlock {
                block_type: 0,
                literal,
            });
        }
    }
}

/// Groups top-level nodes into blocks: raw HTML containers with their
/// contents (`group_html_runs`), and, in Pandoc documents, a table with the
/// `Table: caption` paragraph that follows it.
fn group_blocks<'n, 'a>(
    nodes: &'n [Node<'a>],
    pandoc: bool,
    divs: &[pandoc::DivSpan],
    index: &LineIndex,
) -> Vec<&'n [Node<'a>]> {
    let html_groups = group_html_runs(nodes);
    if !pandoc {
        return html_groups;
    }
    let html_groups = join_div_contents(nodes, html_groups, divs, index);
    // The groups tile `nodes` in order, so `start` tracks where each begins.
    let mut groups = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < html_groups.len() {
        let len = html_groups[i].len();
        let joins = len == 1
            && matches!(nodes[start].data().value, NodeValue::Table(_))
            && html_groups.get(i + 1).is_some_and(|next| next.len() == 1)
            && pandoc::is_table_caption(nodes[start + 1]);
        if joins {
            groups.push(&nodes[start..start + 2]);
            start += 2;
            i += 2;
        } else {
            groups.push(html_groups[i]);
            start += len;
            i += 1;
        }
    }
    groups
}

/// A footnote definition's contents as HTML. comrak leaves a space where
/// its back-link would go; the reader adds its own back-link.
fn footnote_html(renderer: &Renderer, definition: Node<'_>, options: &comrak::Options) -> String {
    let children: Vec<Node<'_>> = definition.children().collect();
    renderer
        .render_group(&children, options)
        .replace(" </p>", "</p>")
}

/// Joins consecutive groups that start inside the same outermost fenced div
/// into one group, so the whole div becomes one block.
fn join_div_contents<'n, 'a>(
    nodes: &'n [Node<'a>],
    groups: Vec<&'n [Node<'a>]>,
    divs: &[pandoc::DivSpan],
    index: &LineIndex,
) -> Vec<&'n [Node<'a>]> {
    // The outermost div (if any) each group starts in.
    let outer = |group: &[Node<'_>]| {
        let start = block_range(group[0], index).start;
        divs.iter().position(|d| d.start <= start && start <= d.end)
    };
    let mut joined: Vec<&'n [Node<'a>]> = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < groups.len() {
        let div = outer(groups[i]);
        let mut len = groups[i].len();
        let mut j = i + 1;
        while div.is_some() && j < groups.len() && outer(groups[j]) == div {
            len += groups[j].len();
            j += 1;
        }
        joined.push(&nodes[start..start + len]);
        start += len;
        i = j;
    }
    joined
}

/// For each node of a div block, the `<div>` tags to write before it and
/// the `</div>` tags after it, so nested divs wrap exactly their contents.
fn div_wraps(
    group: &[Node<'_>],
    divs: &[pandoc::DivSpan],
    index: &LineIndex,
) -> Vec<(String, String)> {
    let starts: Vec<usize> = group.iter().map(|n| block_range(n, index).start).collect();
    let first = starts.first().copied().unwrap_or(0);
    let last = group.last().map_or(0, |n| block_range(n, index).end);
    // The divs in this block, outermost first (`divs` is sorted by start).
    let mut pending = divs
        .iter()
        .filter(|d| d.end >= first && d.start <= last)
        .peekable();
    let mut open: Vec<&pandoc::DivSpan> = Vec::new();
    let mut wraps = Vec::with_capacity(group.len());
    for (i, &start) in starts.iter().enumerate() {
        let mut before = String::new();
        // Rust note: `peekable()` lets an iterator look one item ahead;
        // `next_if` takes the next item only when the test passes.
        while let Some(div) = pending.next_if(|d| d.start <= start) {
            // A div that ended before this node is empty; skip it.
            if div.end >= start {
                before.push_str(&pandoc::div_open_tag(div));
                open.push(div);
            }
        }
        let next_start = starts.get(i + 1).copied().unwrap_or(usize::MAX);
        let mut after = String::new();
        while open.last().is_some_and(|d| d.end <= next_start) {
            open.pop();
            after.push_str("</div>\n");
        }
        wraps.push((before, after));
    }
    wraps
}

/// Each footnote's number, from its references (`[^name]` or `^[...]`).
fn footnote_numbers(root: Node<'_>) -> std::collections::HashMap<String, u32> {
    let mut numbers = std::collections::HashMap::new();
    for node in root.descendants() {
        if let NodeValue::FootnoteReference(r) = &node.data().value {
            numbers.entry(r.name.clone()).or_insert(r.ix);
        }
    }
    numbers
}

/// A footnote definition comrak generated for an inline footnote (`^[...]`).
fn is_generated_footnote(node: Node<'_>) -> bool {
    matches!(&node.data().value, NodeValue::FootnoteDefinition(def) if def.name.starts_with("__inline_"))
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
pub(crate) fn plain_text(node: Node<'_>) -> String {
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
                match code_language(&code.info).as_deref() {
                    Some("mermaid") => features.has_mermaid = true,
                    Some("abc") => features.has_abc = true,
                    _ => {}
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
        // The lone H1 is the title (unnumbered); the H2s are 1 and 2.
        assert!(
            doc.blocks[0]
                .html
                .starts_with("<h1 id=\"user-content-intro\">")
        );
        assert!(
            doc.blocks[1]
                .html
                .starts_with("<h2 data-number=\"1\" id=\"user-content-intro-1\">")
        );
        let numbers: Vec<Option<&str>> = doc.toc.iter().map(|t| t.number.as_deref()).collect();
        assert_eq!(numbers, [None, Some("1"), Some("2")]);
    }

    #[test]
    fn html_container_wraps_following_markdown() {
        let text =
            "<details>\n<summary>More</summary>\n\nHidden **text**.\n\n</details>\n\nAfter.\n";
        let doc = parse(text);
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].kind, BlockKind::Html);
        assert_eq!(
            slices(&doc, text.as_bytes())[0],
            "<details>\n<summary>More</summary>\n\nHidden **text**.\n\n</details>"
        );
        let html = &doc.blocks[0].html;
        assert!(html.contains("<strong>text</strong>"));
        assert!(html.trim_end().ends_with("</details>"));
        assert_eq!(doc.blocks[1].kind, BlockKind::Paragraph);
    }

    #[test]
    fn unclosed_container_runs_to_the_end() {
        let doc = parse("<div>\n\nA\n\nB\n");
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn container_tag_counting() {
        assert_eq!(container_depth_change("<details><summary>x</summary>"), 1);
        assert_eq!(container_depth_change("<div class=\"a\"><div>\n</div>"), 1);
        assert_eq!(container_depth_change("</DIV>"), -1);
        assert_eq!(container_depth_change("<divider><detailsx>"), 0);
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
