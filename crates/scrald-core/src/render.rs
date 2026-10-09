//! Per-block HTML rendering and sanitizing (DESIGN.md §3, §13).

use comrak::nodes::Node;

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
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
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
            // `language-rust` on code blocks, `footnotes` sections, and so on.
            .add_generic_attributes(["class"])
            // Only checkboxes may be inputs: drop any other `type`.
            .attribute_filter(|element, attribute, value| {
                if element == "input" && attribute == "type" && value != "checkbox" {
                    None
                } else {
                    Some(value.into())
                }
            });
        Renderer { sanitizer }
    }

    /// Renders one block (and its children) to sanitized HTML.
    pub fn render(&self, node: Node<'_>, options: &comrak::Options) -> String {
        let mut html = String::new();
        // Rust note: writing into a `String` can't fail, but `format_html`
        // writes to any `fmt::Write` and so returns a Result. If it ever does
        // fail, show an error in place of the block instead of panicking.
        if let Err(e) = comrak::format_html(node, options, &mut html) {
            html = format!(
                "<p class=\"render-error\">{}</p>",
                escape_text(&e.to_string())
            );
        }
        self.sanitize(&html)
    }

    pub fn sanitize(&self, html: &str) -> String {
        self.sanitizer.clean(html).to_string()
    }
}

/// Adds `id="slug"` to a rendered heading's opening tag. Runs after
/// sanitizing; `slug` comes from `toc::slugify`, so it has no quotes or `<`.
pub fn add_heading_id(html: &str, level: u8, slug: &str) -> String {
    let open = format!("<h{level}>");
    match html.strip_prefix(&open) {
        Some(rest) => format!("<h{level} id=\"{slug}\">{rest}"),
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
        let r = Renderer::new();
        let out = r.sanitize("<p onclick=\"x()\">hi<script>alert(1)</script></p>");
        assert_eq!(out, "<p>hi</p>");
    }

    #[test]
    fn strips_javascript_urls_and_iframes() {
        let r = Renderer::new();
        assert_eq!(
            r.sanitize("<a href=\"javascript:alert(1)\">x</a>"),
            "<a rel=\"noopener noreferrer\">x</a>"
        );
        assert_eq!(r.sanitize("<iframe src=\"https://e.com\"></iframe>"), "");
    }

    #[test]
    fn keeps_checkbox_but_not_text_inputs() {
        let r = Renderer::new();
        assert_eq!(
            r.sanitize("<input type=\"checkbox\" checked=\"\" disabled=\"\" />"),
            "<input type=\"checkbox\" checked=\"\" disabled=\"\">"
        );
        assert_eq!(r.sanitize("<input type=\"text\">"), "<input>");
    }

    #[test]
    fn keeps_code_language_class() {
        let r = Renderer::new();
        let html = "<pre><code class=\"language-rust\">x</code></pre>";
        assert_eq!(r.sanitize(html), html);
    }

    #[test]
    fn keeps_math_markers() {
        let r = Renderer::new();
        let html = "<span data-math-style=\"inline\">x</span>";
        assert_eq!(r.sanitize(html), html);
    }

    #[test]
    fn heading_id_is_added() {
        assert_eq!(
            add_heading_id("<h2>Hi</h2>\n", 2, "hi"),
            "<h2 id=\"hi\">Hi</h2>\n"
        );
        assert_eq!(add_heading_id("<p>x</p>", 2, "hi"), "<p>x</p>");
    }

    #[test]
    fn escape() {
        assert_eq!(escape_text("<a & \"b\">"), "&lt;a &amp; &quot;b&quot;&gt;");
    }
}
