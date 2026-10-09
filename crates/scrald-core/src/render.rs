//! Per-block HTML rendering and sanitizing (DESIGN.md §3, §13).

use comrak::nodes::Node;

/// Prefix for every HTML `id` that comes from document content, so a heading
/// called "App" (or raw HTML with `id="app"`) can't clash with the app's own
/// elements. Links keep the bare form (`#app`, `#fn-1`); the reader adds the
/// prefix when it looks an id up.
pub const ID_PREFIX: &str = "user-content-";

/// Renders single top-level blocks to sanitized HTML. Build one per document
/// (or reuse it): setting up the sanitizer allowlist isn't free.
pub struct Renderer {
    // Rust note: `ammonia::Builder<'static>` has a lifetime parameter because
    // it can borrow its allowlists; `'static` means ours are string literals
    // that live for the whole program, so the struct needs no lifetime itself.
    sanitizer: ammonia::Builder<'static>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Renderer {
    /// `allow_remote_images` lets `<img src="http(s)://...">` through; it is
    /// off unless the user allowed remote images for this document (§6.1).
    pub fn new(allow_remote_images: bool) -> Self {
        let mut sanitizer = ammonia::Builder::default();
        sanitizer
            // Task list checkboxes. They render disabled; reading never edits.
            .add_tags(["input"])
            .add_tag_attributes("input", ["type", "checked", "disabled"])
            // Footnote references and back-links point at these ids.
            .add_tag_attributes("li", ["id"])
            // Math (`$x$`, `$$x$$`, ```math) is marked for KaTeX to find later.
            .add_tag_attributes("span", ["data-math-style"])
            .add_tag_attributes("code", ["data-math-style"])
            .add_tag_attributes(
                "a",
                [
                    "id",
                    "data-footnote-ref",
                    "data-footnote-backref",
                    "data-footnote-backref-idx",
                    "aria-label",
                ],
            )
            // Images from core's asset pipeline (assets.rs).
            .add_tag_attributes("img", ["data-asset", "loading", "referrerpolicy"])
            // `data:` is allowed as a scheme only so `data:image/...` images
            // work; the filter below removes it everywhere else.
            .add_url_schemes(["data"])
            // `language-rust` on code blocks, `footnotes` sections, and so on.
            .add_generic_attributes(["class"])
            .id_prefix(Some(ID_PREFIX))
            .attribute_filter(move |element, attribute, value| {
                filter_attribute(element, attribute, value, allow_remote_images).map(Into::into)
            });
        Renderer { sanitizer }
    }

    /// Renders one block (and its children) to sanitized HTML.
    pub fn render(&self, node: Node<'_>, options: &comrak::Options) -> String {
        self.render_group(&[node], options)
    }

    /// Renders several consecutive blocks and sanitizes them as one piece of
    /// HTML, so a container opened in the first can wrap the rest.
    pub fn render_group(&self, nodes: &[Node<'_>], options: &comrak::Options) -> String {
        let mut html = String::new();
        for &node in nodes {
            self.format_into(node, options, &mut html);
        }
        self.sanitize(&html)
    }

    fn format_into(&self, node: Node<'_>, options: &comrak::Options, html: &mut String) {
        // Rust note: writing into a `String` can't fail, but `format_html`
        // writes to any `fmt::Write` and so returns a Result. If it ever does
        // fail, show an error in place of the block instead of panicking.
        let mut piece = String::new();
        match comrak::format_html(node, options, &mut piece) {
            Ok(()) => html.push_str(&piece),
            Err(e) => html.push_str(&format!(
                "<p class=\"render-error\">{}</p>",
                escape_text(&e.to_string())
            )),
        }
    }

    pub fn sanitize(&self, html: &str) -> String {
        self.sanitizer.clean(html).to_string()
    }
}

/// Extra attribute rules on top of ammonia's allowlist. Returns `None` to
/// drop the attribute.
// Rust note: the returned `Option<&str>` borrows from `value` (the only
// borrowed input that could be returned); Rust infers that link itself here.
fn filter_attribute<'v>(
    element: &str,
    attribute: &str,
    value: &'v str,
    allow_remote_images: bool,
) -> Option<&'v str> {
    let lower = value.trim_start().to_ascii_lowercase();
    match (element, attribute) {
        // Only checkboxes may be inputs.
        ("input", "type") if value != "checkbox" => None,
        // `<img src>` only for data images, and remote images when allowed.
        // Local images never use `src`: core emits `data-asset` instead, so
        // raw HTML can't point the webview at arbitrary files.
        ("img", "src") => {
            let remote = lower.starts_with("http://") || lower.starts_with("https://");
            let data_image = lower.starts_with("data:image/");
            (data_image || (remote && allow_remote_images)).then_some(value)
        }
        // `data:` URLs nowhere else (a `data:text/html` link could run script).
        _ if lower.starts_with("data:") => None,
        _ => Some(value),
    }
}

/// Adds `id="user-content-slug"` to a rendered heading's opening tag. Runs
/// after sanitizing; `slug` comes from `toc::slugify`, so it has no quotes or `<`.
pub fn add_heading_id(html: &str, level: u8, slug: &str) -> String {
    let open = format!("<h{level}>");
    match html.strip_prefix(&open) {
        Some(rest) => format!("<h{level} id=\"{ID_PREFIX}{slug}\">{rest}"),
        None => html.to_string(),
    }
}

/// Escapes text for use inside HTML.
pub fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_scripts_and_handlers() {
        let r = Renderer::default();
        let out = r.sanitize("<p onclick=\"x()\">hi<script>alert(1)</script></p>");
        assert_eq!(out, "<p>hi</p>");
    }

    #[test]
    fn strips_javascript_urls_and_iframes() {
        let r = Renderer::default();
        assert_eq!(
            r.sanitize("<a href=\"javascript:alert(1)\">x</a>"),
            "<a rel=\"noopener noreferrer\">x</a>"
        );
        assert_eq!(r.sanitize("<iframe src=\"https://e.com\"></iframe>"), "");
    }

    #[test]
    fn keeps_checkbox_but_not_text_inputs() {
        let r = Renderer::default();
        assert_eq!(
            r.sanitize("<input type=\"checkbox\" checked=\"\" disabled=\"\" />"),
            "<input type=\"checkbox\" checked=\"\" disabled=\"\">"
        );
        assert_eq!(r.sanitize("<input type=\"text\">"), "<input>");
    }

    #[test]
    fn keeps_code_language_class() {
        let r = Renderer::default();
        let html = "<pre><code class=\"language-rust\">x</code></pre>";
        assert_eq!(r.sanitize(html), html);
    }

    #[test]
    fn keeps_math_markers() {
        let r = Renderer::default();
        let html = "<span data-math-style=\"inline\">x</span>";
        assert_eq!(r.sanitize(html), html);
    }

    #[test]
    fn image_sources_are_restricted() {
        let blocked = Renderer::new(false);
        assert_eq!(
            blocked.sanitize("<img src=\"https://e.com/a.png\">"),
            "<img>"
        );
        assert_eq!(blocked.sanitize("<img src=\"secret.png\">"), "<img>");
        assert_eq!(
            blocked.sanitize("<img src=\"file:///etc/passwd\">"),
            "<img>"
        );
        assert_eq!(
            blocked.sanitize("<img src=\"data:image/png;base64,AA\">"),
            "<img src=\"data:image/png;base64,AA\">"
        );
        assert_eq!(
            blocked.sanitize("<img data-asset=\"3\">"),
            "<img data-asset=\"3\">"
        );
        let allowed = Renderer::new(true);
        assert_eq!(
            allowed.sanitize("<img src=\"https://e.com/a.png\">"),
            "<img src=\"https://e.com/a.png\">"
        );
    }

    #[test]
    fn data_urls_only_for_images() {
        let r = Renderer::default();
        assert_eq!(
            r.sanitize("<a href=\"data:text/html,<script>x</script>\">x</a>"),
            "<a rel=\"noopener noreferrer\">x</a>"
        );
    }

    #[test]
    fn content_ids_are_prefixed() {
        let r = Renderer::default();
        assert_eq!(
            r.sanitize("<a id=\"app\" href=\"#x\">x</a>"),
            "<a id=\"user-content-app\" href=\"#x\" rel=\"noopener noreferrer\">x</a>"
        );
    }

    #[test]
    fn heading_id_is_added() {
        assert_eq!(
            add_heading_id("<h2>Hi</h2>\n", 2, "hi"),
            "<h2 id=\"user-content-hi\">Hi</h2>\n"
        );
        assert_eq!(add_heading_id("<p>x</p>", 2, "hi"), "<p>x</p>");
    }

    #[test]
    fn escape() {
        assert_eq!(escape_text("<a & \"b\">"), "&lt;a &amp; &quot;b&quot;&gt;");
    }
}
