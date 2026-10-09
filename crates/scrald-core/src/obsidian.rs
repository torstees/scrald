//! Obsidian syntax (DESIGN.md §5.3–5.4): comments, callouts, wikilinks and
//! embeds resolved within the vault, inline tags, and block references.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use comrak::nodes::{Node, NodeHtmlBlock, NodeLink, NodeValue};

use crate::assets::{self, ImageCollector};
use crate::render::escape_text;
use crate::toc::slugify;

// --- %%comments%% ---------------------------------------------------------

/// Replaces every `%%comment%%` with spaces of the same byte length (line
/// breaks kept), so comments vanish from the reading view while every byte
/// offset, and so every block's source range, stays exactly the same.
/// Fenced code and inline code are left alone; an unclosed `%%` hides
/// everything after it, as in Obsidian.
pub fn blank_comments(text: &str) -> String {
    if !text.contains("%%") {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut in_comment = false;
    let mut fence: Option<&str> = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if !in_comment {
            let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
            match (fence, marker) {
                (None, Some(m)) => {
                    fence = Some(m);
                    out.push_str(line);
                    continue;
                }
                (Some(open), Some(m)) if open == m => {
                    fence = None;
                    out.push_str(line);
                    continue;
                }
                (Some(_), _) => {
                    out.push_str(line);
                    continue;
                }
                (None, None) => {}
            }
        }
        let mut in_code = false;
        let mut rest = line;
        while !rest.is_empty() {
            if !in_comment && rest.starts_with('`') {
                in_code = !in_code;
                out.push('`');
                rest = &rest[1..];
            } else if !in_code && rest.starts_with("%%") {
                in_comment = !in_comment;
                out.push_str("  ");
                rest = &rest[2..];
            } else {
                let c = rest.chars().next().unwrap_or(' ');
                if in_comment && c != '\n' && c != '\r' {
                    // Same byte length, so offsets don't move.
                    out.extend(std::iter::repeat_n(' ', c.len_utf8()));
                } else {
                    out.push(c);
                }
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

// --- Callouts -------------------------------------------------------------

/// A callout's color family, from its type. Unknown types look like notes.
fn callout_family(kind: &str) -> &'static str {
    match kind {
        "abstract" | "summary" | "tldr" => "abstract",
        "info" | "todo" => "info",
        "tip" | "hint" | "important" => "tip",
        "success" | "check" | "done" => "success",
        "question" | "help" | "faq" => "question",
        "warning" | "caution" | "attention" => "warning",
        "failure" | "fail" | "missing" => "failure",
        "danger" | "error" | "bug" => "danger",
        "example" => "example",
        "quote" | "cite" => "quote",
        _ => "note",
    }
}

/// The parts of a callout's first line: `[!type]` (case-insensitive), an
/// optional `+` (foldable, open) or `-` (foldable, closed), and an optional
/// title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalloutMarker {
    pub kind: String,
    /// `None`: not foldable. `Some(true)`: open. `Some(false)`: collapsed.
    pub fold: Option<bool>,
    pub title: String,
}

pub fn parse_callout_marker(line: &str) -> Option<CalloutMarker> {
    let rest = line.trim_start().strip_prefix("[!")?;
    let end = rest.find(']')?;
    let kind = rest[..end].trim().to_lowercase();
    if kind.is_empty() || kind.contains(char::is_whitespace) {
        return None;
    }
    let mut after = &rest[end + 1..];
    let fold = match after.chars().next() {
        Some('+') => Some(true),
        Some('-') => Some(false),
        _ => None,
    };
    if fold.is_some() {
        after = &after[1..];
    }
    Some(CalloutMarker {
        kind,
        fold,
        title: after.trim().to_string(),
    })
}

/// Turns blockquotes that start with `[!type]` into callouts: a titled box
/// colored by type, or a `<details>` element when foldable. The blockquote
/// is replaced by one HTML block holding the callout markup and its rendered
/// contents. Run it after the other passes, since it renders the contents.
pub fn transform_callouts(node: Node<'_>, options: &comrak::Options) {
    // Rust note: `.rev()` walks the list backwards, so nested callouts are
    // rendered before the callouts that contain them.
    let quotes: Vec<Node<'_>> = node
        .descendants()
        .filter(|n| matches!(n.data().value, NodeValue::BlockQuote))
        .collect();
    for quote in quotes.into_iter().rev() {
        let Some(first_para) = quote
            .first_child()
            .filter(|p| matches!(p.data().value, NodeValue::Paragraph))
        else {
            continue;
        };
        let Some(first_text) = first_para.first_child() else {
            continue;
        };
        let line = match &first_text.data().value {
            NodeValue::Text(t) => t.to_string(),
            _ => continue,
        };
        let Some(marker) = parse_callout_marker(&line) else {
            continue;
        };

        // The marker's line is the title; drop it (up to the first line break).
        let mut cursor = Some(first_text);
        while let Some(n) = cursor {
            cursor = n.next_sibling();
            let is_break = matches!(n.data().value, NodeValue::SoftBreak | NodeValue::LineBreak);
            n.detach();
            if is_break {
                break;
            }
        }
        if first_para.first_child().is_none() {
            first_para.detach();
        }

        let title = if marker.title.is_empty() {
            capitalize(&marker.kind)
        } else {
            marker.title.clone()
        };
        let classes = format!("sk-callout sk-callout-{}", callout_family(&marker.kind));
        let title = escape_text(&title);
        let (open, close) = match marker.fold {
            Some(expanded) => (
                format!(
                    "<details class=\"{classes}\"{}><summary class=\"sk-callout-title\">{title}</summary><div class=\"sk-callout-body\">\n",
                    if expanded { " open" } else { "" }
                ),
                "</div></details>\n".to_string(),
            ),
            None => (
                format!(
                    "<div class=\"{classes}\"><div class=\"sk-callout-title\">{title}</div><div class=\"sk-callout-body\">\n"
                ),
                "</div></div>\n".to_string(),
            ),
        };
        let mut literal = open;
        let children: Vec<Node<'_>> = quote.children().collect();
        for child in children {
            // Writing into a String can't fail; on the off chance comrak
            // reports an error, the child is left out.
            let _ = comrak::format_html(child, options, &mut literal);
            child.detach();
        }
        literal.push_str(&close);
        quote.data_mut().value = NodeValue::HtmlBlock(NodeHtmlBlock {
            block_type: 0,
            literal,
        });
    }
}

/// A new node allocated in the parse's arena, so it lives as long as the tree.
// Rust note: `'a` ties the new node's lifetime to the arena's: comrak keeps
// every node in an arena that outlives the whole tree, and the compiler
// checks that no node can outlive it. Explicit lifetimes are unavoidable with
// comrak's API; the `Node<'a>` alias hides most of them.
fn new_node<'a>(arena: &'a comrak::Arena<'a>, value: NodeValue) -> Node<'a> {
    arena.alloc(value.into())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

// --- Vault index ------------------------------------------------------------

/// Files in an Obsidian vault, for resolving `[[links]]` and `![[embeds]]`
/// the way Obsidian does: by name anywhere in the vault.
#[derive(Debug, Clone, Default)]
pub struct VaultIndex {
    pub root: PathBuf,
    /// All files, relative to `root`.
    files: Vec<PathBuf>,
    /// Lowercased file name (with extension) to indices into `files`.
    by_name: HashMap<String, Vec<usize>>,
    /// The vault's configured attachment folder, relative to `root`.
    attachments: Option<PathBuf>,
}

/// More than this many files and the index stops growing: enough for large
/// vaults, while a document opened from a huge folder can't stall.
const MAX_FILES: usize = 100_000;

impl VaultIndex {
    /// The index for a document: its vault (nearest folder with `.obsidian`,
    /// scanned recursively), or just its own folder if it isn't in a vault.
    pub fn for_document(doc_path: &Path) -> VaultIndex {
        let doc_dir = doc_path.parent().unwrap_or(Path::new("")).to_path_buf();
        match doc_dir.ancestors().find(|a| a.join(".obsidian").is_dir()) {
            Some(root) => VaultIndex::build(root, true),
            None => VaultIndex::build(&doc_dir, false),
        }
    }

    pub fn build(root: &Path, recursive: bool) -> VaultIndex {
        let mut index = VaultIndex {
            root: root.to_path_buf(),
            attachments: read_attachment_folder(root),
            ..VaultIndex::default()
        };
        let mut pending = vec![PathBuf::new()];
        while let Some(rel_dir) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(root.join(&rel_dir)) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok) {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') || name == "node_modules" {
                    continue;
                }
                let rel = rel_dir.join(&name);
                match entry.file_type() {
                    Ok(t) if t.is_dir() && recursive => pending.push(rel),
                    Ok(t) if t.is_file() => {
                        if index.files.len() >= MAX_FILES {
                            return index;
                        }
                        index
                            .by_name
                            .entry(name.to_lowercase())
                            .or_default()
                            .push(index.files.len());
                        index.files.push(rel);
                    }
                    _ => {}
                }
            }
        }
        index
    }

    /// Resolves a link target like `Note`, `Folder/Note`, `Note.md`, or
    /// `diagram.pdf`. A bare name gets `.md`. When several files match, one
    /// in the linking document's folder wins, then the shortest path.
    pub fn resolve(&self, target: &str, from_doc: &Path) -> Option<PathBuf> {
        let target = target.trim().trim_start_matches('/');
        if target.is_empty() {
            return None;
        }
        let with_ext = if Path::new(target).extension().is_some() {
            target.to_string()
        } else {
            format!("{target}.md")
        };
        let file_name = Path::new(&with_ext)
            .file_name()?
            .to_string_lossy()
            .to_lowercase();
        let candidates = self.by_name.get(&file_name)?;
        let wanted_suffix: Vec<String> = Path::new(&with_ext)
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
                _ => None,
            })
            .collect();
        let from_dir = from_doc.parent().unwrap_or(Path::new(""));
        let mut matches: Vec<&PathBuf> = candidates
            .iter()
            .map(|&i| &self.files[i])
            .filter(|rel| ends_with_components(rel, &wanted_suffix))
            .collect();
        if matches.is_empty() {
            return None;
        }
        matches.sort_by_key(|rel| {
            let same_folder = self.root.join(rel).parent() == Some(from_dir);
            (
                !same_folder,
                rel.components().count(),
                rel.to_string_lossy().len(),
            )
        });
        Some(self.root.join(matches[0]))
    }

    /// Resolves an attachment (image, PDF) by file name: the vault's
    /// attachment folder first, then anywhere in the vault.
    pub fn resolve_attachment(&self, name: &str, from_doc: &Path) -> Option<PathBuf> {
        if let Some(folder) = &self.attachments {
            let candidate = self.root.join(folder).join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        self.resolve(name, from_doc)
    }
}

/// Whether `rel`'s trailing components equal `suffix` (case-insensitive).
fn ends_with_components(rel: &Path, suffix: &[String]) -> bool {
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    parts.len() >= suffix.len() && parts[parts.len() - suffix.len()..] == *suffix
}

/// `attachmentFolderPath` from `.obsidian/app.json`, if it names a folder
/// inside the vault (not "/" or "./", which mean the vault root or the
/// note's own folder).
fn read_attachment_folder(root: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(root.join(".obsidian").join("app.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let folder = value.get("attachmentFolderPath")?.as_str()?.trim();
    let folder = folder.trim_start_matches("./").trim_matches('/');
    (!folder.is_empty() && !folder.contains("..")).then(|| PathBuf::from(folder))
}

/// A `file://` URL for a path, which Scrald's link handler opens in place.
pub fn file_url(path: &Path, fragment: Option<&str>) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut url = String::from("file://");
    if !text.starts_with('/') {
        url.push('/');
    }
    for c in text.chars() {
        match c {
            ' ' => url.push_str("%20"),
            '#' => url.push_str("%23"),
            '%' => url.push_str("%25"),
            '?' => url.push_str("%3F"),
            '"' => url.push_str("%22"),
            _ => url.push(c),
        }
    }
    if let Some(f) = fragment.filter(|f| !f.is_empty()) {
        url.push('#');
        url.push_str(f);
    }
    url
}

/// The HTML id an Obsidian link fragment points at: a heading (`#Heading`,
/// slugified) or a block reference (`#^id`).
pub fn fragment_id(fragment: &str) -> String {
    match fragment.strip_prefix('^') {
        Some(block) => format!("block-{block}"),
        None => slugify(fragment),
    }
}

// --- Inline syntax: embeds, leftover wikilinks, tags, block references ------

/// A piece of a text node after scanning for Obsidian inline syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    /// `![[target|options]]`
    Embed {
        target: String,
        options: Option<String>,
    },
    /// `[[target|alias]]` that comrak didn't already turn into a link.
    Wiki {
        target: String,
        alias: Option<String>,
    },
    /// `#tag` (without the `#`).
    Tag(String),
    /// `^block-id` at the end of a block (without the `^`).
    BlockRef(String),
}

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}

/// Splits text into plain runs and Obsidian inline syntax. `at_block_end`
/// says whether this text ends its block, where `^block-id` may appear.
pub fn split_inline(text: &str, at_block_end: bool) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let bracket = if rest.starts_with("![[") {
            Some(3)
        } else if rest.starts_with("[[") {
            Some(2)
        } else {
            None
        };
        if let Some(open) = bracket
            && let Some(close) = rest[open..].find("]]")
        {
            let inner = &rest[open..open + close];
            if !inner.is_empty() && !inner.contains('\n') {
                flush(&mut plain, &mut pieces);
                let (target, extra) = match inner.split_once('|') {
                    Some((t, e)) => (t.trim().to_string(), Some(e.trim().to_string())),
                    None => (inner.trim().to_string(), None),
                };
                pieces.push(if open == 3 {
                    Piece::Embed {
                        target,
                        options: extra,
                    }
                } else {
                    Piece::Wiki {
                        target,
                        alias: extra,
                    }
                });
                i += open + close + 2;
                continue;
            }
        }
        let c = rest.chars().next().unwrap_or(' ');
        let starts_word = i == 0
            || text[..i]
                .chars()
                .next_back()
                .is_some_and(|p| p.is_whitespace() || p == '(');
        if c == '#' && starts_word {
            let tag: String = rest[1..].chars().take_while(|&c| is_tag_char(c)).collect();
            if !tag.is_empty() && !tag.chars().all(|c| c.is_ascii_digit()) && !tag.ends_with('/') {
                flush(&mut plain, &mut pieces);
                i += 1 + tag.len();
                pieces.push(Piece::Tag(tag));
                continue;
            }
        }
        plain.push(c);
        i += c.len_utf8();
    }
    flush(&mut plain, &mut pieces);

    if at_block_end
        && let Some(Piece::Text(last)) = pieces.last()
        && let Some((before, id)) = trailing_block_ref(last)
    {
        let before = before.to_string();
        let id = id.to_string();
        pieces.pop();
        if !before.is_empty() {
            pieces.push(Piece::Text(before));
        }
        pieces.push(Piece::BlockRef(id));
    }
    pieces
}

fn flush(plain: &mut String, pieces: &mut Vec<Piece>) {
    if !plain.is_empty() {
        pieces.push(Piece::Text(std::mem::take(plain)));
    }
}

/// `"text ^abc-123"` -> `("text", "abc-123")`.
fn trailing_block_ref(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim_end();
    let caret = trimmed.rfind('^')?;
    let id = &trimmed[caret + 1..];
    let before = &trimmed[..caret];
    let valid = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let separated = before.is_empty() || before.ends_with(char::is_whitespace);
    (valid && separated).then(|| (before.trim_end(), id))
}

/// File extensions embedded as images (`![[pic.png]]`).
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif"];

/// `300` -> (300, none); `300x200` -> (300, 200).
fn parse_size(options: &str) -> Option<(u32, Option<u32>)> {
    match options.split_once('x') {
        Some((w, h)) => Some((w.trim().parse().ok()?, Some(h.trim().parse().ok()?))),
        None => Some((options.trim().parse().ok()?, None)),
    }
}

/// Splits `Note#Heading` into the note and the fragment.
fn split_target(target: &str) -> (&str, Option<&str>) {
    match target.split_once('#') {
        Some((note, fragment)) => (note, Some(fragment)),
        None => (target, None),
    }
}

/// The text Obsidian shows for a link without an alias: `Note`,
/// `Note > Heading`, or just `Heading` for a link within the note.
fn display_label(target: &str) -> String {
    match split_target(target) {
        ("", Some(fragment)) => fragment.trim_start_matches('^').to_string(),
        (note, Some(fragment)) => format!("{note} > {}", fragment.trim_start_matches('^')),
        (note, None) => note.to_string(),
    }
}

/// What the link transform needs about the document and its vault.
pub struct LinkContext<'c> {
    pub doc_path: &'c Path,
    pub vault: &'c VaultIndex,
}

/// Resolves wikilinks, renders embeds, and styles tags and block references
/// under `node` (Obsidian flavor only).
pub fn transform_links<'a>(
    arena: &'a comrak::Arena<'a>,
    node: Node<'a>,
    ctx: &LinkContext<'_>,
    images: &mut ImageCollector,
) {
    // [[wikilinks]] that comrak parsed: point them at the resolved file.
    let wikilinks: Vec<Node<'a>> = node
        .descendants()
        .filter(|n| matches!(n.data().value, NodeValue::WikiLink(_)))
        .collect();
    for link in wikilinks {
        let url = match &link.data().value {
            NodeValue::WikiLink(w) => w.url.clone(),
            _ => continue,
        };
        let mut label = crate::document::plain_text(link);
        if label == url {
            // No alias: show what Obsidian shows.
            label = display_label(&url);
            let children: Vec<Node<'a>> = link.children().collect();
            for child in children {
                child.detach();
            }
            link.append(new_node(arena, NodeValue::Text(label.clone().into())));
        }
        let (note, fragment) = split_target(&url);
        if note.is_empty() {
            // `[[#Heading]]`: a link within this document.
            let href = format!("#{}", fragment_id(fragment.unwrap_or_default()));
            link.data_mut().value = NodeValue::Link(Box::new(NodeLink {
                url: href,
                title: String::new(),
            }));
            continue;
        }
        match ctx.vault.resolve(note, ctx.doc_path) {
            Some(path) => {
                let href = file_url(&path, fragment.map(fragment_id).as_deref());
                link.data_mut().value = NodeValue::Link(Box::new(NodeLink {
                    url: href,
                    title: String::new(),
                }));
            }
            None => {
                let children: Vec<Node<'a>> = link.children().collect();
                for child in children {
                    child.detach();
                }
                link.data_mut().value = NodeValue::HtmlInline(missing_link_html(note, &label));
            }
        }
    }

    // Text with ![[embeds]], leftover [[links]], #tags, and ^block-ids.
    let texts: Vec<Node<'a>> = node
        .descendants()
        .filter(|n| matches!(n.data().value, NodeValue::Text(_)) && !inside_link(n))
        .collect();
    for text_node in texts {
        let text = match &text_node.data().value {
            NodeValue::Text(t) => t.to_string(),
            _ => continue,
        };
        let at_block_end = text_node.next_sibling().is_none()
            && text_node
                .parent()
                .is_some_and(|p| matches!(p.data().value, NodeValue::Paragraph));
        let pieces = split_inline(&text, at_block_end);
        if matches!(pieces.as_slice(), [Piece::Text(_)] | []) {
            continue;
        }
        for piece in pieces {
            let new = match piece {
                Piece::Text(s) => new_node(arena, NodeValue::Text(s.into())),
                Piece::Tag(tag) => new_node(
                    arena,
                    NodeValue::HtmlInline(format!(
                        "<span class=\"sk-tag\">#{}</span>",
                        escape_text(&tag)
                    )),
                ),
                Piece::BlockRef(id) => new_node(
                    arena,
                    NodeValue::HtmlInline(format!(
                        "<a id=\"block-{}\" class=\"sk-block-ref\"></a>",
                        escape_text(&id)
                    )),
                ),
                Piece::Wiki { target, alias } => {
                    let (note, fragment) = split_target(&target);
                    let label = alias.unwrap_or_else(|| display_label(&target));
                    match ctx.vault.resolve(note, ctx.doc_path) {
                        Some(path) => {
                            let href = file_url(&path, fragment.map(fragment_id).as_deref());
                            let link = new_node(
                                arena,
                                NodeValue::Link(Box::new(NodeLink {
                                    url: href,
                                    title: String::new(),
                                })),
                            );
                            link.append(new_node(arena, NodeValue::Text(label.into())));
                            link
                        }
                        None => new_node(
                            arena,
                            NodeValue::HtmlInline(missing_link_html(note, &label)),
                        ),
                    }
                }
                Piece::Embed { target, options } => new_node(
                    arena,
                    NodeValue::HtmlInline(embed_html(&target, options.as_deref(), ctx, images)),
                ),
            };
            text_node.insert_before(new);
        }
        text_node.detach();
    }
}

fn inside_link(node: Node<'_>) -> bool {
    node.ancestors().skip(1).any(|a| {
        matches!(
            a.data().value,
            NodeValue::Link(_) | NodeValue::Image(_) | NodeValue::WikiLink(_)
        )
    })
}

fn missing_link_html(target: &str, label: &str) -> String {
    format!(
        "<span class=\"sk-link-missing\" title=\"{}\">{}</span>",
        escape_text(&format!(
            "No note named \u{201c}{target}\u{201d} in this vault"
        )),
        escape_text(label)
    )
}

/// `![[pic.png|300]]` becomes an image; `![[Other note]]` (or a PDF) a link
/// card that opens it (transclusion is post-v1, DESIGN.md §15).
fn embed_html(
    target: &str,
    options: Option<&str>,
    ctx: &LinkContext<'_>,
    images: &mut ImageCollector,
) -> String {
    let (name, fragment) = split_target(target);
    let extension = Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        let size = options.and_then(parse_size);
        return match ctx.vault.resolve_attachment(name, ctx.doc_path) {
            Some(path) => {
                let alt = options.filter(|_| size.is_none()).unwrap_or(name);
                images.embed_html(path, alt, size.map(|(w, _)| w), size.and_then(|(_, h)| h))
            }
            None => assets::missing_image_html(
                name,
                "",
                &[ctx
                    .vault
                    .root
                    .join(format!("(anywhere in the vault)/{name}"))],
            ),
        };
    }
    let label = options.unwrap_or(target);
    match ctx.vault.resolve(name, ctx.doc_path) {
        Some(path) => format!(
            "<a class=\"sk-embed-card\" href=\"{}\" title=\"Embedded note (opens it)\">{}</a>",
            escape_text(&file_url(&path, fragment.map(fragment_id).as_deref())),
            escape_text(label)
        ),
        None => format!(
            "<span class=\"sk-embed-card sk-link-missing\" title=\"{}\">{}</span>",
            escape_text(&format!(
                "No note named \u{201c}{name}\u{201d} in this vault"
            )),
            escape_text(label)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("scrald-vault-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in [".obsidian", "people", "projects/old", "attachments"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        for file in [
            "Index.md",
            "people/Sigrid.md",
            "projects/Plan.md",
            "projects/old/Plan.md",
            "attachments/fjord.png",
        ] {
            std::fs::write(root.join(file), "x").unwrap();
        }
        std::fs::write(
            root.join(".obsidian/app.json"),
            r#"{"attachmentFolderPath": "attachments"}"#,
        )
        .unwrap();
        root
    }

    #[test]
    fn comments_are_blanked_without_moving_bytes() {
        let text = "Keep %%hidden \u{e5}%% this.\n%%\nmulti\nline\n%%\nAfter.\n";
        let out = blank_comments(text);
        assert_eq!(out.len(), text.len());
        assert!(!out.contains("hidden") && !out.contains("multi"));
        assert!(out.starts_with("Keep ") && out.contains(" this.\n") && out.ends_with("After.\n"));
        assert_eq!(out.matches('\n').count(), text.matches('\n').count());
    }

    #[test]
    fn comments_in_code_are_kept() {
        let text = "`%%not a comment%%`\n```\n%% code %%\n```\n";
        assert_eq!(blank_comments(text), text);
    }

    #[test]
    fn callout_markers() {
        assert_eq!(
            parse_callout_marker("[!todo]- Tasks for today"),
            Some(CalloutMarker {
                kind: "todo".into(),
                fold: Some(false),
                title: "Tasks for today".into()
            })
        );
        assert_eq!(
            parse_callout_marker("[!NOTE]").map(|m| (m.kind, m.fold)),
            Some(("note".into(), None))
        );
        assert_eq!(
            parse_callout_marker("[!tip]+").map(|m| m.fold),
            Some(Some(true))
        );
        assert_eq!(parse_callout_marker("[not a callout]"), None);
        assert_eq!(parse_callout_marker("[!two words]"), None);
    }

    #[test]
    fn inline_pieces() {
        let pieces = split_inline(
            "See ![[pic.png|300]] and [[Note#Part|that]] #idea #2024 #a/b ^ref-1",
            true,
        );
        assert_eq!(
            pieces,
            vec![
                Piece::Text("See ".into()),
                Piece::Embed {
                    target: "pic.png".into(),
                    options: Some("300".into())
                },
                Piece::Text(" and ".into()),
                Piece::Wiki {
                    target: "Note#Part".into(),
                    alias: Some("that".into())
                },
                Piece::Text(" ".into()),
                Piece::Tag("idea".into()),
                Piece::Text(" #2024 ".into()),
                Piece::Tag("a/b".into()),
                Piece::BlockRef("ref-1".into()),
            ]
        );
    }

    #[test]
    fn tags_need_a_word_start_and_block_refs_the_block_end() {
        assert_eq!(
            split_inline("issue#5 x", false),
            vec![Piece::Text("issue#5 x".into())]
        );
        assert_eq!(
            split_inline("ends ^id", false),
            vec![Piece::Text("ends ^id".into())]
        );
        assert_eq!(split_inline("x^2", true), vec![Piece::Text("x^2".into())]);
    }

    #[test]
    fn vault_resolution_like_obsidian() {
        let root = temp_vault("resolve");
        let vault = VaultIndex::for_document(&root.join("Index.md"));
        let from_root = root.join("Index.md");
        assert_eq!(
            vault.resolve("sigrid", &from_root),
            Some(root.join("people/Sigrid.md"))
        );
        // Shortest path wins when names collide...
        assert_eq!(
            vault.resolve("Plan", &from_root),
            Some(root.join("projects/Plan.md"))
        );
        // ...unless one is in the linking note's own folder.
        assert_eq!(
            vault.resolve("Plan", &root.join("projects/old/Notes.md")),
            Some(root.join("projects/old/Plan.md"))
        );
        // A path picks a specific file.
        assert_eq!(
            vault.resolve("old/Plan", &from_root),
            Some(root.join("projects/old/Plan.md"))
        );
        assert_eq!(vault.resolve("Nobody", &from_root), None);
        // Attachments: the configured folder first.
        assert_eq!(
            vault.resolve_attachment("fjord.png", &from_root),
            Some(root.join("attachments/fjord.png"))
        );
    }

    #[test]
    fn outside_a_vault_only_the_document_folder_is_indexed() {
        let dir = std::env::temp_dir().join(format!("scrald-novault-idx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("here.md"), "x").unwrap();
        std::fs::write(dir.join("sub/deeper.md"), "x").unwrap();
        let vault = VaultIndex::for_document(&dir.join("doc.md"));
        assert!(vault.resolve("here", &dir.join("doc.md")).is_some());
        assert!(vault.resolve("deeper", &dir.join("doc.md")).is_none());
    }

    #[test]
    fn labels_without_an_alias() {
        assert_eq!(display_label("Note"), "Note");
        assert_eq!(display_label("Note#Part"), "Note > Part");
        assert_eq!(display_label("#Part"), "Part");
        assert_eq!(display_label("Note#^ref"), "Note > ref");
    }

    #[test]
    fn file_urls_and_fragments() {
        assert_eq!(
            file_url(Path::new("C:/vault/My Note.md"), Some("part-two")),
            "file:///C:/vault/My%20Note.md#part-two"
        );
        assert_eq!(
            file_url(Path::new("/home/v/a#b.md"), None),
            "file:///home/v/a%23b.md"
        );
        assert_eq!(fragment_id("^abc-1"), "block-abc-1");
        assert_eq!(fragment_id("The Fjord"), "the-fjord");
    }
}
