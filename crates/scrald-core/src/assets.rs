//! Image path resolution (DESIGN.md §6.1) and the markup that replaces
//! Markdown images in rendered blocks.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use comrak::nodes::{Node, NodeValue};
use serde::Serialize;

use crate::obsidian::VaultIndex;
use crate::render::escape_text;

/// A local file an image in the document resolved to. The app serves only
/// these files, by `id`, through the `scrald-asset` protocol (§6.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAsset {
    pub id: u32,
    pub path: PathBuf,
    /// Pixel size read from the file header, when the format is recognized.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Where relative image paths are looked up, in priority order.
#[derive(Debug, Clone, Default)]
pub struct AssetContext {
    /// Directories to try, in order (front matter `assets:`, then the
    /// document's own directory).
    pub search_dirs: Vec<PathBuf>,
    /// Whether `http(s)` images may load (off by default, for privacy).
    pub allow_remote: bool,
    /// For Obsidian documents: the vault, searched by file name when the
    /// directories above don't have the image (§6.1).
    // Rust note: `Arc` (atomically reference-counted pointer) shares one
    // index between clones without copying it, like `std::shared_ptr`.
    pub vault: Option<Arc<VaultIndex>>,
    /// The document's path, for vault lookups that prefer nearby files.
    pub doc_path: PathBuf,
}

impl AssetContext {
    /// The standard lookup order for a document at `doc_path` whose front
    /// matter names `assets_dir` (relative to the document, or absolute).
    pub fn for_document(doc_path: &Path, assets_dir: Option<&str>, allow_remote: bool) -> Self {
        let doc_dir = doc_path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut search_dirs = Vec::new();
        if let Some(dir) = assets_dir.map(str::trim).filter(|d| !d.is_empty()) {
            search_dirs.push(doc_dir.join(dir));
        }
        search_dirs.push(doc_dir);
        AssetContext {
            search_dirs,
            allow_remote,
            vault: None,
            doc_path: doc_path.to_path_buf(),
        }
    }
}

/// What an image source turned into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// A local file that exists.
    Local(PathBuf),
    /// A local path that couldn't be found; lists every path tried.
    Missing { attempted: Vec<PathBuf> },
    /// An `http(s)` URL.
    Remote(String),
    /// A `data:` URL, used as-is.
    Data(String),
    /// Some other scheme, which is never loaded.
    Unsupported(String),
}

/// Resolves an image `src` as written in Markdown.
pub fn resolve(src: &str, ctx: &AssetContext) -> Resolved {
    let src = src.trim();
    let lower = src.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Resolved::Remote(src.to_string());
    }
    if lower.starts_with("data:") {
        return Resolved::Data(src.to_string());
    }
    if let Some(rest) = strip_prefix_ignore_case(src, "file://") {
        return check_exists(file_url_path(rest));
    }
    if has_scheme(src) {
        return Resolved::Unsupported(src.to_string());
    }

    // Drop any `#fragment` or `?query`, then undo URL percent-encoding
    // (comrak encodes spaces in link destinations as %20).
    let path_part = src.split(['#', '?']).next().unwrap_or_default();
    let relative = PathBuf::from(percent_decode(path_part));
    if relative.is_absolute() {
        return check_exists(relative);
    }

    let mut attempted = Vec::new();
    for dir in &ctx.search_dirs {
        let candidate = dir.join(&relative);
        if candidate.is_file() {
            return Resolved::Local(candidate);
        }
        attempted.push(candidate);
    }
    if let Some(vault) = &ctx.vault {
        let name = relative.to_string_lossy();
        if let Some(found) = vault.resolve_attachment(&name, &ctx.doc_path) {
            return Resolved::Local(found);
        }
        attempted.push(vault.root.join(format!("(anywhere in the vault)/{name}")));
    }
    Resolved::Missing { attempted }
}

fn check_exists(path: PathBuf) -> Resolved {
    if path.is_file() {
        Resolved::Local(path)
    } else {
        Resolved::Missing {
            attempted: vec![path],
        }
    }
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    // Rust note: this function returns a slice of its input, so the return
    // type must say which input it borrows from. The `'a` lifetime ties the
    // result to `text` (not `prefix`), like documenting "points into text".
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// `scheme:` at the start, like `mailto:` or `javascript:`. A Windows drive
/// letter (`C:`) is a single character, so it doesn't count.
fn has_scheme(src: &str) -> bool {
    match src.find(':') {
        Some(i) if i > 1 => src[..i]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')),
        _ => false,
    }
}

/// Path from the part of a `file://` URL after the scheme. `/C:/x` (Windows
/// drive paths) drops the leading slash.
fn file_url_path(rest: &str) -> PathBuf {
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let decoded = percent_decode(rest);
    let bytes = decoded.as_bytes();
    let is_drive_path =
        bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':';
    if is_drive_path {
        PathBuf::from(&decoded[1..])
    } else {
        PathBuf::from(decoded)
    }
}

/// Decodes `%XX` escapes. Invalid escapes are kept literally, and the result
/// is converted to UTF-8 lossily (file names are text in Markdown anyway).
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // Rust note: a "let chain" (edition 2024): the `if` runs only when the
        // condition holds AND the pattern matches, binding `hi` and `lo`.
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
        {
            out.push(hi * 16 + lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Reads an image's pixel size from its header, without decoding it.
pub fn image_size(path: &Path) -> Option<(u32, u32)> {
    let size = imagesize::size(path).ok()?;
    Some((
        u32::try_from(size.width).ok()?,
        u32::try_from(size.height).ok()?,
    ))
}

/// The display name of a path or URL: its last segment.
fn file_name(src: &str) -> &str {
    src.rsplit(['/', '\\']).next().unwrap_or(src)
}

/// HTML for a local image that the frontend points at the asset protocol.
pub fn local_image_html(asset: &ImageAsset, alt: &str, title: Option<&str>) -> String {
    let mut html = format!(
        "<img data-asset=\"{}\" alt=\"{}\" loading=\"lazy\"",
        asset.id,
        escape_text(alt)
    );
    if let (Some(w), Some(h)) = (asset.width, asset.height) {
        // With width/height attributes (and CSS `height: auto`), the browser
        // reserves the right space before the image loads: no scroll jumps.
        html.push_str(&format!(" width=\"{w}\" height=\"{h}\""));
    }
    push_title(&mut html, title);
    html.push('>');
    html
}

/// HTML for a local image at a requested size (Obsidian `![[pic.png|300]]`
/// or `|300x200`). A width alone keeps the natural aspect ratio.
pub fn local_image_html_sized(
    asset: &ImageAsset,
    alt: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> String {
    let (w, h) = match (width, height, asset.width, asset.height) {
        (Some(w), Some(h), _, _) => (Some(w), Some(h)),
        (Some(w), None, Some(nw), Some(nh)) if nw > 0 => (
            Some(w),
            Some((u64::from(w) * u64::from(nh) / u64::from(nw)) as u32),
        ),
        (Some(w), None, _, _) => (Some(w), None),
        (None, _, nw, nh) => (nw, nh),
    };
    let mut html = format!(
        "<img data-asset=\"{}\" alt=\"{}\" loading=\"lazy\"",
        asset.id,
        escape_text(alt)
    );
    if let Some(w) = w {
        html.push_str(&format!(" width=\"{w}\""));
    }
    if let Some(h) = h {
        html.push_str(&format!(" height=\"{h}\""));
    }
    html.push('>');
    html
}

/// HTML for a remote image the user allowed for this document.
pub fn remote_image_html(url: &str, alt: &str, title: Option<&str>) -> String {
    let mut html = format!(
        "<img src=\"{}\" alt=\"{}\" loading=\"lazy\" referrerpolicy=\"no-referrer\"",
        escape_text(url),
        escape_text(alt)
    );
    push_title(&mut html, title);
    html.push('>');
    html
}

/// Placeholder for an image whose file wasn't found (§6.1): the file name,
/// with the paths that were tried in a tooltip.
pub fn missing_image_html(src: &str, alt: &str, attempted: &[PathBuf]) -> String {
    let tried: Vec<String> = attempted.iter().map(|p| p.display().to_string()).collect();
    let tooltip = format!("Image not found: {src}\nTried:\n{}", tried.join("\n"));
    placeholder_html("sk-image-missing", file_name(src), alt, &tooltip)
}

/// Placeholder for a blocked remote image.
pub fn blocked_image_html(url: &str, alt: &str) -> String {
    let tooltip = format!("Remote image not loaded: {url}");
    placeholder_html("sk-image-remote", file_name(url), alt, &tooltip)
}

/// Placeholder for an image with an unsupported scheme.
pub fn unsupported_image_html(src: &str, alt: &str) -> String {
    let tooltip = format!("Unsupported image source: {src}");
    placeholder_html("sk-image-missing", file_name(src), alt, &tooltip)
}

fn placeholder_html(class: &str, name: &str, alt: &str, tooltip: &str) -> String {
    let label = if alt.trim().is_empty() { name } else { alt };
    format!(
        "<span class=\"sk-image-placeholder {class}\" title=\"{}\">{}</span>",
        escape_text(tooltip),
        escape_text(label)
    )
}

fn push_title(html: &mut String, title: Option<&str>) {
    if let Some(t) = title.filter(|t| !t.is_empty()) {
        html.push_str(&format!(" title=\"{}\"", escape_text(t)));
    }
}

/// `target` relative to the folder `from`, with `/` separators so the front
/// matter reads the same on every platform: `../images`, `assets`. Returns
/// `None` when they share no root (another drive on Windows); then the
/// absolute path is the only option. Both paths should be absolute.
pub fn relative_path(from: &Path, target: &Path) -> Option<String> {
    use std::path::Component;
    let from: Vec<Component<'_>> = from.components().collect();
    let to: Vec<Component<'_>> = target.components().collect();
    // Rust note: `zip` pairs up two iterators; `take_while` stops at the
    // first pair that differs, so this counts the shared leading parts.
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    if common == 0
        || !matches!(
            from.first(),
            Some(Component::Prefix(_) | Component::RootDir)
        )
    {
        return None;
    }
    let mut parts: Vec<String> = vec!["..".to_string(); from.len() - common];
    parts.extend(
        to[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

/// Replaces images in a document with Scrald's own markup (DESIGN.md §6),
/// and collects the local files they resolve to: Markdown images, raw HTML
/// `<img>` tags, and Obsidian image embeds.
#[derive(Debug, Default)]
pub struct ImageCollector {
    pub assets: Vec<ImageAsset>,
    pub remote_count: usize,
}

impl ImageCollector {
    /// Rewrites every image under `node`: Markdown images become inline HTML
    /// (an asset reference, a remote image if allowed, or a placeholder), and
    /// `<img>` tags in raw HTML get the same resolution.
    pub fn rewrite(&mut self, node: Node<'_>, ctx: &AssetContext) {
        // Collect first: changing nodes while walking the tree would confuse
        // the iterator.
        let targets: Vec<Node<'_>> = node
            .descendants()
            .filter(|n| {
                matches!(
                    n.data().value,
                    NodeValue::Image(_) | NodeValue::HtmlBlock(_) | NodeValue::HtmlInline(_)
                )
            })
            .collect();

        for target in targets {
            // Rust note: this block ends the `data()` borrow before
            // `data_mut()` below; a RefCell panics if both are held at once.
            let image = match &target.data().value {
                NodeValue::Image(link) => Some((link.url.clone(), link.title.clone())),
                _ => None,
            };
            if let Some((url, title)) = image {
                let alt = crate::document::plain_text(target);
                let mut html = self.image_html(&url, &alt, Some(title.as_str()), ctx);
                if let Some(attrs) = crate::pandoc::attributes_of(target) {
                    html = apply_image_attributes(&html, &attrs);
                }
                // The alt text now lives in the HTML; drop the child nodes.
                let children: Vec<Node<'_>> = target.children().collect();
                for child in children {
                    child.detach();
                }
                target.data_mut().value = NodeValue::HtmlInline(html);
                continue;
            }
            // Raw HTML: rewrite any <img> tags in place.
            let mut data = target.data_mut();
            match &mut data.value {
                NodeValue::HtmlBlock(block) if contains_img(&block.literal) => {
                    block.literal = self.rewrite_raw_images(&block.literal, ctx);
                }
                NodeValue::HtmlInline(literal) if contains_img(literal) => {
                    *literal = self.rewrite_raw_images(literal, ctx);
                }
                _ => {}
            }
        }
    }

    /// The markup for one Markdown image.
    fn image_html(
        &mut self,
        url: &str,
        alt: &str,
        title: Option<&str>,
        ctx: &AssetContext,
    ) -> String {
        match resolve(url, ctx) {
            Resolved::Local(file) => {
                let asset = self.asset_for(file);
                local_image_html(&asset, alt, title)
            }
            Resolved::Missing { attempted } => missing_image_html(url, alt, &attempted),
            Resolved::Remote(remote) => {
                self.remote_count += 1;
                if ctx.allow_remote {
                    remote_image_html(&remote, alt, title)
                } else {
                    blocked_image_html(&remote, alt)
                }
            }
            Resolved::Data(data) => remote_image_html(&data, alt, title),
            Resolved::Unsupported(src) => unsupported_image_html(&src, alt),
        }
    }

    /// The markup for an Obsidian image embed that resolved to `file`.
    pub fn embed_html(
        &mut self,
        file: PathBuf,
        alt: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> String {
        let asset = self.asset_for(file);
        local_image_html_sized(&asset, alt, width, height)
    }

    /// The asset for `file`, reusing the id if the document uses it twice.
    pub fn asset_for(&mut self, file: PathBuf) -> ImageAsset {
        if let Some(existing) = self.assets.iter().find(|a| a.path == file) {
            return existing.clone();
        }
        let size = image_size(&file);
        let asset = ImageAsset {
            id: self.assets.len() as u32,
            path: file,
            width: size.map(|(w, _)| w),
            height: size.map(|(_, h)| h),
        };
        self.assets.push(asset.clone());
        asset
    }

    /// Resolves the `src` of each `<img>` tag in raw HTML (#125): a local
    /// file becomes `data-asset`, like Markdown images; a missing one loses
    /// its `src` and gets a tooltip; remote ones follow the remote-images
    /// setting. Other attributes (`width`, `alt`) are kept.
    fn rewrite_raw_images(&mut self, html: &str, ctx: &AssetContext) -> String {
        let mut out = String::with_capacity(html.len());
        let mut rest = html;
        while let Some(start) = find_img_tag(rest) {
            out.push_str(&rest[..start]);
            let tag_and_after = &rest[start..];
            let end = tag_and_after
                .find('>')
                .map_or(tag_and_after.len(), |e| e + 1);
            out.push_str(&self.rewrite_img_tag(&tag_and_after[..end], ctx));
            rest = &tag_and_after[end..];
        }
        out.push_str(rest);
        out
    }

    fn rewrite_img_tag(&mut self, tag: &str, ctx: &AssetContext) -> String {
        let Some((src_start, src_end, src)) = find_attribute(tag, "src") else {
            return tag.to_string();
        };
        let replacement = match resolve(&src, ctx) {
            Resolved::Local(file) => {
                let asset = self.asset_for(file);
                format!("data-asset=\"{}\"", asset.id)
            }
            Resolved::Missing { .. } | Resolved::Unsupported(_) => {
                format!(
                    "title=\"{}\"",
                    escape_text(&format!("Image not found: {src}"))
                )
            }
            Resolved::Remote(_) => {
                self.remote_count += 1;
                if ctx.allow_remote {
                    tag[src_start..src_end].to_string()
                } else {
                    String::new()
                }
            }
            Resolved::Data(_) => tag[src_start..src_end].to_string(),
        };
        format!("{}{}{}", &tag[..src_start], replacement, &tag[src_end..])
    }
}

/// Applies Pandoc attributes (`{#id .right width=40%}`) to an image's
/// markup: the id and classes, and `width`/`height` as pixels (`300`,
/// `300px`) or a percentage, which replace the size read from the file.
/// Other units and other keys are ignored: no inline styles reach the page.
pub fn apply_image_attributes(html: &str, attrs: &comrak::nodes::Attributes) -> String {
    let size = |key: &str| {
        attrs
            .pairs
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| {
                let v = v.trim().trim_end_matches("px");
                let digits = v.strip_suffix('%').unwrap_or(v);
                (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
                    .then(|| v.to_string())
            })
    };
    let (width, height) = (size("width"), size("height"));
    let mut html = html.to_string();
    if width.is_some() || height.is_some() {
        // The file's natural size no longer applies.
        for name in ["width", "height"] {
            if let Some((start, end, _)) = find_attribute(&html, name) {
                html.replace_range(start - 1..end, "");
            }
        }
    }
    let mut extra = crate::pandoc::attribute_html(attrs, None, "");
    if let Some(w) = width {
        extra.push_str(&format!(" width=\"{w}\""));
    }
    if let Some(h) = height {
        extra.push_str(&format!(" height=\"{h}\""));
    }
    // After the tag name: `<img` or `<span`.
    let name_end = html[1..]
        .find(|c: char| c.is_whitespace() || c == '>')
        .map_or(html.len(), |e| e + 1);
    html.insert_str(name_end, &extra);
    html
}

fn contains_img(html: &str) -> bool {
    html.to_ascii_lowercase().contains("<img")
}

/// Byte offset of the next `<img` tag (followed by whitespace, `/`, or `>`).
fn find_img_tag(html: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find("<img") {
        let at = from + i;
        match lower[at + 4..].chars().next() {
            Some(c) if c.is_whitespace() || c == '/' || c == '>' => return Some(at),
            None => return None,
            _ => from = at + 4,
        }
    }
    None
}

/// The span of `name=value` inside a tag, and the unquoted value.
fn find_attribute(tag: &str, name: &str) -> Option<(usize, usize, String)> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let preceded = lower[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
        let after = lower[at + name.len()..].trim_start();
        if !preceded || !after.starts_with('=') {
            continue;
        }
        let value_start = tag.len() - after.len() + 1;
        let value_part = &tag[value_start..];
        let trimmed = value_part.trim_start();
        let lead = value_part.len() - trimmed.len();
        let (value, consumed) = match trimmed.chars().next() {
            Some(q @ ('"' | '\'')) => {
                let close = trimmed[1..].find(q).map_or(trimmed.len() - 1, |c| c + 1);
                (
                    trimmed[1..close].to_string(),
                    (close + 1).min(trimmed.len()),
                )
            }
            _ => {
                let len = trimmed
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .unwrap_or(trimmed.len());
                (trimmed[..len].to_string(), len)
            }
        };
        return Some((at, value_start + lead + consumed, value));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_use_forward_slashes() {
        let root = std::env::temp_dir();
        let doc_dir = root.join("notes").join("novel");
        assert_eq!(
            relative_path(&doc_dir, &doc_dir.join("images")),
            Some("images".to_string())
        );
        assert_eq!(
            relative_path(&doc_dir, &root.join("notes").join("shared").join("pics")),
            Some("../shared/pics".to_string())
        );
        assert_eq!(relative_path(&doc_dir, &doc_dir), Some(".".to_string()));
        assert_eq!(
            relative_path(Path::new("relative"), Path::new("relative/x")),
            None
        );
    }

    #[test]
    fn image_attributes_replace_the_natural_size() {
        let (attrs, _) =
            crate::pandoc::parse_attributes("{#map .right width=40% height=2em}").unwrap();
        let html =
            "<img data-asset=\"0\" alt=\"Map\" loading=\"lazy\" width=\"800\" height=\"600\">";
        assert_eq!(
            apply_image_attributes(html, &attrs),
            "<img id=\"map\" class=\"right\" width=\"40%\" data-asset=\"0\" alt=\"Map\" loading=\"lazy\">"
        );
        let (classes_only, _) = crate::pandoc::parse_attributes("{.center}").unwrap();
        assert_eq!(
            apply_image_attributes(html, &classes_only),
            html.replacen("<img", "<img class=\"center\"", 1)
        );
    }

    /// A fresh, empty temp directory for one test.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrald-assets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A minimal valid 3x2 PNG (header and IHDR are all imagesize reads).
    fn write_png(path: &Path) {
        let mut png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
        png.extend_from_slice(&3u32.to_be_bytes());
        png.extend_from_slice(&2u32.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        std::fs::write(path, png).unwrap();
    }

    #[test]
    fn resolves_in_assets_dir_before_document_dir() {
        let dir = temp_dir("order");
        std::fs::create_dir_all(dir.join("img")).unwrap();
        write_png(&dir.join("img").join("a.png"));
        write_png(&dir.join("a.png"));
        let ctx = AssetContext::for_document(&dir.join("doc.md"), Some("img"), false);
        assert_eq!(
            resolve("a.png", &ctx),
            Resolved::Local(dir.join("img").join("a.png"))
        );
    }

    #[test]
    fn falls_back_to_document_dir_and_decodes_spaces() {
        let dir = temp_dir("fallback");
        std::fs::create_dir_all(dir.join("sub dir")).unwrap();
        write_png(&dir.join("sub dir").join("my pic.png"));
        let ctx = AssetContext::for_document(&dir.join("doc.md"), Some("missing-assets"), false);
        assert_eq!(
            resolve("sub%20dir/my%20pic.png", &ctx),
            Resolved::Local(dir.join("sub dir").join("my pic.png"))
        );
    }

    #[test]
    fn missing_lists_attempted_paths_in_order() {
        let dir = temp_dir("missing");
        let ctx = AssetContext::for_document(&dir.join("doc.md"), Some("img"), false);
        assert_eq!(
            resolve("nope.png", &ctx),
            Resolved::Missing {
                attempted: vec![dir.join("img").join("nope.png"), dir.join("nope.png")]
            }
        );
    }

    #[test]
    fn absolute_paths_and_file_urls() {
        let dir = temp_dir("absolute");
        let png = dir.join("abs.png");
        write_png(&png);
        let ctx = AssetContext::default();
        assert_eq!(
            resolve(&png.display().to_string(), &ctx),
            Resolved::Local(png.clone())
        );
        // Build a file URL the way a user would write it on this platform.
        let url_path = png.display().to_string().replace('\\', "/");
        let url = if url_path.starts_with('/') {
            format!("file://{url_path}")
        } else {
            format!("file:///{url_path}")
        };
        assert_eq!(resolve(&url, &ctx), Resolved::Local(png));
    }

    #[test]
    fn remote_data_and_other_schemes() {
        let ctx = AssetContext::default();
        assert_eq!(
            resolve("https://e.com/a.png", &ctx),
            Resolved::Remote("https://e.com/a.png".into())
        );
        assert_eq!(
            resolve("HTTP://e.com/a.png", &ctx),
            Resolved::Remote("HTTP://e.com/a.png".into())
        );
        assert!(matches!(
            resolve("data:image/png;base64,xx", &ctx),
            Resolved::Data(_)
        ));
        assert!(matches!(
            resolve("javascript:alert(1)", &ctx),
            Resolved::Unsupported(_)
        ));
    }

    #[test]
    fn drive_letters_are_not_schemes() {
        assert!(!has_scheme("C:/pics/a.png"));
        assert!(has_scheme("mailto:x@y"));
        assert_eq!(
            file_url_path("/C:/pics/a%20b.png"),
            PathBuf::from("C:/pics/a b.png")
        );
        assert_eq!(
            file_url_path("localhost/home/a.png"),
            PathBuf::from("/home/a.png")
        );
    }

    #[test]
    fn percent_decoding() {
        assert_eq!(percent_decode("a%20b%C3%A5"), "a bå");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz%4"), "%zz%4");
    }

    #[test]
    fn reads_png_size() {
        let dir = temp_dir("size");
        let png = dir.join("s.png");
        write_png(&png);
        assert_eq!(image_size(&png), Some((3, 2)));
        assert_eq!(image_size(&dir.join("none.png")), None);
    }

    #[test]
    fn markup_is_escaped() {
        let asset = ImageAsset {
            id: 4,
            path: PathBuf::from("x.png"),
            width: Some(3),
            height: Some(2),
        };
        assert_eq!(
            local_image_html(&asset, "a \"q\" <b>", Some("T")),
            "<img data-asset=\"4\" alt=\"a &quot;q&quot; &lt;b&gt;\" loading=\"lazy\" width=\"3\" height=\"2\" title=\"T\">"
        );
        let html = missing_image_html("img/fjord.png", "", &[PathBuf::from("a/fjord.png")]);
        assert!(html.contains(">fjord.png</span>"));
        assert!(html.contains("Tried:\na/fjord.png"));
    }
}
