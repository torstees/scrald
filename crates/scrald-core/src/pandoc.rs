//! Pandoc syntax (DESIGN.md §5.3–5.4, §6.3): `{#id .class key=val}`
//! attributes, figures, fenced divs, bracketed spans, and table captions.
//!
//! comrak parses attributes on headings, fenced code, links, images, and
//! inline code (into each node's `attrs`), but its HTML output ignores them;
//! the passes here put them into the markup.

use comrak::nodes::{Attributes, Node, NodeValue};

use crate::render::{self, ID_PREFIX, escape_text};

// --- Attribute blocks ---------------------------------------------------------

/// Parses an attribute block at the start of `text`: `{#id .class key=val
/// key="quoted value"}`. Returns the attributes and the bytes consumed
/// (through the closing `}`), or `None` if `text` doesn't start with a
/// well-formed, non-empty block. A lone `-` means `.unnumbered`, as in Pandoc.
pub fn parse_attributes(text: &str) -> Option<(Attributes, usize)> {
    if !text.starts_with('{') {
        return None;
    }
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut end = None;
    for (i, c) in text.char_indices().skip(1) {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) if c == '\n' => return None,
            Some(_) => current.push(c),
            None => match c {
                '"' | '\'' => quote = Some(c),
                '}' => {
                    end = Some(i + 1);
                    break;
                }
                '{' => return None,
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        // Rust note: `mem::take` moves the String out and
                        // leaves an empty one behind, avoiding a copy.
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                c => current.push(c),
            },
        }
    }
    let end = end?;
    if !current.is_empty() {
        tokens.push(current);
    }
    if tokens.is_empty() {
        return None;
    }

    let mut attrs = Attributes::default();
    for token in tokens {
        if let Some(id) = token.strip_prefix('#') {
            if id.is_empty() {
                return None;
            }
            attrs.id = Some(id.to_string());
        } else if let Some(class) = token.strip_prefix('.') {
            if class.is_empty() {
                return None;
            }
            attrs.classes.push(class.to_string());
        } else if token == "-" {
            attrs.classes.push("unnumbered".to_string());
        } else if let Some((key, value)) = token.split_once('=') {
            let key_ok = !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'));
            if !key_ok {
                return None;
            }
            attrs.pairs.push((key.to_string(), value.to_string()));
        } else {
            return None;
        }
    }
    Some((attrs, end))
}

/// ` id="…" class="…"` for an opening tag, escaped. `id_prefix` is
/// `ID_PREFIX` for HTML added after sanitizing (which would otherwise add
/// it), and "" before. Pairs are left to the caller: only a few are safe.
pub fn attribute_html(attrs: &Attributes, extra_class: Option<&str>, id_prefix: &str) -> String {
    let mut out = String::new();
    if let Some(id) = &attrs.id {
        out.push_str(&format!(" id=\"{id_prefix}{}\"", escape_text(id)));
    }
    let mut classes: Vec<&str> = extra_class.into_iter().collect();
    classes.extend(attrs.classes.iter().map(String::as_str));
    if !classes.is_empty() {
        out.push_str(&format!(" class=\"{}\"", escape_text(&classes.join(" "))));
    }
    out
}

/// Adds an id and classes to the first tag of already-sanitized HTML (a
/// heading's `<h2>`, a table's `<table>`). The tag must not already have
/// those attributes.
pub fn add_to_first_tag(html: &str, attrs: &Attributes) -> String {
    let extra = attribute_html(attrs, None, ID_PREFIX);
    if extra.is_empty() {
        return html.to_string();
    }
    let Some(open) = html.find('<') else {
        return html.to_string();
    };
    let name_end = html[open + 1..]
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .map_or(html.len(), |e| open + 1 + e);
    format!("{}{extra}{}", &html[..name_end], &html[name_end..])
}

/// A node's attributes, if it has any.
pub fn attributes_of(node: Node<'_>) -> Option<Attributes> {
    node.data()
        .attrs
        .as_deref()
        .filter(|a| a.id.is_some() || !a.classes.is_empty() || !a.pairs.is_empty())
        .cloned()
}

/// Reads a trailing attribute block from a heading's text when comrak
/// didn't (it rejects Pandoc's `{-}` shorthand), stores it as the heading's
/// attributes, and removes it from the text.
pub fn take_heading_attributes(heading: Node<'_>) {
    if attributes_of(heading).is_some() {
        return;
    }
    let Some(last) = heading.last_child() else {
        return;
    };
    let Some(text) = text_of(last) else {
        return;
    };
    let trimmed = text.trim_end();
    if let Some(open) = trimmed.rfind('{')
        && let Some((attrs, consumed)) = parse_attributes(&trimmed[open..])
        && open + consumed == trimmed.len()
    {
        set_text(last, trimmed[..open].trim_end());
        heading.data_mut().attrs = Some(Box::new(attrs));
    }
}

// --- Links and inline code ------------------------------------------------------

/// Applies attributes to links and inline code, and turns bracketed spans
/// (`[text]{.smallcaps}`) into `<span>`s. Images are handled by the image
/// collector (`assets.rs`), since it builds their markup anyway.
pub fn transform_inlines<'a>(arena: &'a comrak::Arena<'a>, node: Node<'a>) {
    let targets: Vec<Node<'a>> = node
        .descendants()
        .filter(|n| matches!(n.data().value, NodeValue::Link(_) | NodeValue::Code(_)))
        .collect();
    for target in targets {
        let Some(attrs) = attributes_of(target) else {
            continue;
        };
        let extra = attribute_html(&attrs, None, "");
        // Rust note: this `match` hands back a value (an Option) from each
        // arm; the `data()` borrow ends before the nodes are rearranged.
        let replacement = match &target.data().value {
            NodeValue::Link(link) => {
                let title = if link.title.is_empty() {
                    String::new()
                } else {
                    format!(" title=\"{}\"", escape_text(&link.title))
                };
                Some((
                    format!("<a href=\"{}\"{title}{extra}>", escape_text(&link.url)),
                    Some("</a>"),
                ))
            }
            NodeValue::Code(code) => Some((
                format!("<code{extra}>{}</code>", escape_text(&code.literal)),
                None,
            )),
            _ => None,
        };
        let Some((open, close)) = replacement else {
            continue;
        };
        target.insert_before(html_inline(arena, open));
        let children: Vec<Node<'a>> = target.children().collect();
        for child in children {
            target.insert_before(child);
        }
        if let Some(close) = close {
            target.insert_before(html_inline(arena, close.to_string()));
        }
        target.detach();
    }
    transform_spans(arena, node);
}

fn html_inline<'a>(arena: &'a comrak::Arena<'a>, html: String) -> Node<'a> {
    arena.alloc(NodeValue::HtmlInline(html).into())
}

fn text_node<'a>(arena: &'a comrak::Arena<'a>, text: &str) -> Node<'a> {
    arena.alloc(NodeValue::Text(text.to_string().into()).into())
}

fn text_of(node: Node<'_>) -> Option<String> {
    match &node.data().value {
        NodeValue::Text(t) => Some(t.to_string()),
        _ => None,
    }
}

/// Bracketed spans: `[` in one text node (or the same one), `]{attrs}` in a
/// later sibling. The brackets and the attribute block are replaced by an
/// opening and closing `<span>`; whatever sits between keeps its formatting.
fn transform_spans<'a>(arena: &'a comrak::Arena<'a>, node: Node<'a>) {
    let closers: Vec<Node<'a>> = node
        .descendants()
        .filter(|n| text_of(n).is_some_and(|t| t.contains("]{")))
        .collect();
    for closer in closers {
        // A closer may hold several spans; handle them one at a time.
        let mut current = closer;
        while let Some(text) = text_of(current) {
            let Some((close_at, attrs, consumed)) = find_span_close(&text) else {
                break;
            };
            // The matching `[`: earlier in this node, or in an earlier sibling.
            let opener = text[..close_at]
                .rfind('[')
                .map(|at| (current, at))
                .or_else(|| find_opener_before(current));
            let Some((open_node, open_at)) = opener else {
                break;
            };
            let span = format!("<span{}>", attribute_html(&attrs, None, ""));

            // Split the closer: before `]`, the `</span>`, and after `}`.
            let before = &text[..close_at];
            let after = &text[close_at + 1 + consumed..];
            let rest = text_node(arena, after);
            current.insert_after(rest);
            current.insert_after(html_inline(arena, "</span>".to_string()));
            set_text(current, before);

            // Split the opener around `[`.
            let open_text = text_of(open_node).unwrap_or_default();
            let (left, right) = (&open_text[..open_at], &open_text[open_at + 1..]);
            let right_node = text_node(arena, right);
            open_node.insert_after(right_node);
            open_node.insert_after(html_inline(arena, span));
            set_text(open_node, left);
            current = rest;
        }
    }
}

/// `"text]{.x} more"` -> (index of `]`, the attributes, bytes of `{…}`).
fn find_span_close(text: &str) -> Option<(usize, Attributes, usize)> {
    let mut from = 0;
    while let Some(i) = text[from..].find("]{") {
        let at = from + i;
        if let Some((attrs, consumed)) = parse_attributes(&text[at + 1..]) {
            return Some((at, attrs, consumed));
        }
        from = at + 2;
    }
    None
}

/// The last `[` in a text node before `node`, among its earlier siblings.
fn find_opener_before(node: Node<'_>) -> Option<(Node<'_>, usize)> {
    let mut sibling = node.previous_sibling();
    while let Some(s) = sibling {
        if let Some(text) = text_of(s)
            && let Some(at) = text.rfind('[')
        {
            return Some((s, at));
        }
        sibling = s.previous_sibling();
    }
    None
}

fn set_text(node: Node<'_>, text: &str) {
    node.data_mut().value = NodeValue::Text(text.to_string().into());
}

// --- Figures --------------------------------------------------------------------

/// A paragraph holding just one image, to become a `<figure>` once images
/// are resolved.
pub struct FigurePlan<'a> {
    paragraph: Node<'a>,
    caption: String,
    attrs: Attributes,
    /// A percentage width (`width=40%`), applied to the whole figure: on
    /// the image it would be a percentage of the shrink-wrapped figure.
    width_percent: Option<u32>,
}

/// `"40%"` -> 40, for percentages from 1 to 100.
pub fn percentage(value: &str) -> Option<u32> {
    let n: u32 = value.trim().strip_suffix('%')?.parse().ok()?;
    (1..=100).contains(&n).then_some(n)
}

/// Finds images that become figures: alone in a paragraph with alt text
/// (Pandoc's implicit figures), or marked `.figure`. Layout classes and the
/// id move from the image to the figure, so `.right` floats the whole
/// figure; `width`/`height` stay on the image. Run before image resolution
/// (which replaces the alt text), then call `finish_figures` after it.
pub fn plan_figures(node: Node<'_>) -> Vec<FigurePlan<'_>> {
    let mut plans = Vec::new();
    let paragraphs: Vec<Node<'_>> = node
        .descendants()
        .filter(|n| matches!(n.data().value, NodeValue::Paragraph))
        .collect();
    for paragraph in paragraphs {
        let content: Vec<Node<'_>> = paragraph
            .children()
            .filter(|c| !text_of(c).is_some_and(|t| t.trim().is_empty()))
            .filter(|c| !matches!(c.data().value, NodeValue::SoftBreak | NodeValue::LineBreak))
            .collect();
        let [image] = content.as_slice() else {
            continue;
        };
        if !matches!(image.data().value, NodeValue::Image(_)) {
            continue;
        }
        let caption = crate::document::plain_text(image);
        let mut attrs = attributes_of(image).unwrap_or_default();
        let marked = attrs.classes.iter().any(|c| c == "figure");
        if caption.trim().is_empty() && !marked {
            continue;
        }
        attrs.classes.retain(|c| c != "figure");
        // The image keeps only its size, unless that's a percentage.
        let mut pairs = std::mem::take(&mut attrs.pairs);
        let width_percent = pairs
            .iter()
            .find(|(k, _)| k == "width")
            .and_then(|(_, v)| percentage(v));
        if width_percent.is_some() {
            pairs.retain(|(k, _)| k != "width" && k != "height");
        }
        let size = Attributes {
            pairs,
            ..Attributes::default()
        };
        image.data_mut().attrs = Some(Box::new(size));
        plans.push(FigurePlan {
            paragraph,
            caption: caption.trim().to_string(),
            attrs,
            width_percent,
        });
    }
    plans
}

/// Turns each planned paragraph into `<figure>` with a `<figcaption>`.
pub fn finish_figures(plans: Vec<FigurePlan<'_>>, options: &comrak::Options) {
    for plan in plans {
        // The sanitizer allows exactly `width: N%` as a figure's style.
        let style = plan
            .width_percent
            .map(|n| format!(" style=\"width: {n}%\""))
            .unwrap_or_default();
        let open = format!(
            "<figure{}{style}>",
            attribute_html(&plan.attrs, Some("sk-figure"), "")
        );
        let close = if plan.caption.is_empty() {
            "</figure>\n".to_string()
        } else {
            format!(
                "<figcaption>{}</figcaption></figure>\n",
                escape_text(&plan.caption)
            )
        };
        render::wrap_children_as_html(plan.paragraph, &open, &close, options);
    }
}

// --- Fenced divs ----------------------------------------------------------------

/// A `:::` fenced div found by `extract_divs`. Offsets are bytes into the
/// text that was scanned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivSpan {
    /// Start of the opening fence line.
    pub start: usize,
    /// End of the closing fence line (before its line break).
    pub end: usize,
    pub attrs: Attributes,
}

/// Finds Pandoc fenced divs and blanks their fence lines to spaces (line
/// breaks and byte offsets unchanged), so the parser only sees the content.
///
/// An opening fence is three or more colons followed by attributes
/// (`::: warning`, `::: {#id .class}`, optionally ending in more colons); a
/// line of colons alone closes the innermost open div, whatever its length,
/// so divs nest as in Pandoc. comrak's own `:::` directives can't do that: a
/// closing fence closes every open directive at once. Fences inside fenced
/// code are ignored, and an opener that is never closed stays as text.
/// Returns the blanked text and the divs, outermost first.
pub fn extract_divs(text: &str) -> (String, Vec<DivSpan>) {
    if !text.contains(":::") {
        return (text.to_string(), Vec::new());
    }
    let mut spans = Vec::new();
    // Open divs: (start of the fence line, end of its content, attributes).
    let mut open: Vec<(usize, usize, Attributes)> = Vec::new();
    let mut blank: Vec<(usize, usize)> = Vec::new();
    let mut code_fence: Option<&str> = None;
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let content_end = line_start + content.len();
        let trimmed = content.trim_start();
        let indent = content.len() - trimmed.len();
        let marker = ["```", "~~~"].into_iter().find(|m| trimmed.starts_with(m));
        match (code_fence, marker) {
            (None, Some(m)) => code_fence = Some(m),
            (Some(opened), Some(m)) if opened == m => code_fence = None,
            _ => {}
        }
        if code_fence.is_none() && marker.is_none() && indent < 4 {
            match div_fence(trimmed) {
                Some(None) => {
                    if let Some((start, open_end, attrs)) = open.pop() {
                        blank.push((start, open_end));
                        blank.push((line_start, content_end));
                        spans.push(DivSpan {
                            start,
                            end: content_end,
                            attrs,
                        });
                    }
                }
                Some(Some(attrs)) => open.push((line_start, content_end, attrs)),
                None => {}
            }
        }
        line_start += line.len();
    }

    let mut out = text.to_string();
    for (start, end) in blank {
        // Fence lines are ASCII colons, spaces, and attribute text; replacing
        // the whole range keeps the length whatever characters it holds.
        let spaces = " ".repeat(end - start);
        out.replace_range(start..end, &spaces);
    }
    spans.sort_by_key(|s| s.start);
    (out, spans)
}

/// `Some(None)` for a closing fence (colons only), `Some(Some(attrs))` for
/// an opening fence, `None` for any other line.
fn div_fence(line: &str) -> Option<Option<Attributes>> {
    let colons = line.chars().take_while(|&c| c == ':').count();
    if colons < 3 {
        return None;
    }
    let rest = line[colons..].trim();
    if rest.is_empty() {
        return Some(None);
    }
    let attrs = div_attributes(rest);
    let any = attrs.id.is_some() || !attrs.classes.is_empty() || !attrs.pairs.is_empty();
    any.then_some(Some(attrs))
}

/// The attributes of a fenced div's info string: `warning` (a class) or
/// `{#id .a .b}`, optionally followed by more colons.
pub fn div_attributes(info: &str) -> Attributes {
    let info = info.trim().trim_end_matches(':').trim_end();
    if info.starts_with('{') {
        return parse_attributes(info).map(|(a, _)| a).unwrap_or_default();
    }
    Attributes {
        classes: info
            .split_whitespace()
            .take(1)
            .map(str::to_string)
            .collect(),
        ..Attributes::default()
    }
}

/// The `<div>` tag that opens a fenced div.
pub fn div_open_tag(span: &DivSpan) -> String {
    format!("<div{}>\n", attribute_html(&span.attrs, Some("sk-div"), ""))
}

// --- Table captions -------------------------------------------------------------

/// Whether `node` is a table caption paragraph (`Table: The caption`, or
/// `: The caption`), which Pandoc lets follow a table.
pub fn is_table_caption(node: Node<'_>) -> bool {
    if !matches!(node.data().value, NodeValue::Paragraph) {
        return false;
    }
    node.first_child()
        .and_then(text_of)
        .is_some_and(|t| caption_prefix_len(&t).is_some())
}

fn caption_prefix_len(text: &str) -> Option<usize> {
    if text.starts_with("Table:") {
        Some("Table:".len())
    } else if text.starts_with(": ") || text == ":" {
        Some(1)
    } else {
        None
    }
}

/// Strips the `Table:` prefix and a trailing `{attrs}` from a caption
/// paragraph, returning the attributes (for the table).
pub fn take_table_caption(paragraph: Node<'_>) -> Attributes {
    if let Some(first) = paragraph.first_child()
        && let Some(text) = text_of(first)
        && let Some(len) = caption_prefix_len(&text)
    {
        set_text(first, text[len..].trim_start());
    }
    let mut attrs = Attributes::default();
    if let Some(last) = paragraph.last_child()
        && let Some(text) = text_of(last)
        && let Some(open) = text.rfind('{')
        && let Some((found, consumed)) = parse_attributes(&text[open..])
        && open + consumed == text.trim_end().len()
    {
        attrs = found;
        set_text(last, text[..open].trim_end());
    }
    attrs
}

/// Moves a rendered caption paragraph (`<p>…</p>` after `</table>`) into the
/// table as `<caption>`. Works on sanitized HTML, moving sanitized content.
pub fn move_caption_into_table(html: &str) -> String {
    let Some(table_end) = html.rfind("</table>") else {
        return html.to_string();
    };
    let (table, after) = html.split_at(table_end + "</table>".len());
    if after.trim().is_empty() {
        return html.to_string();
    }
    let caption = after
        .trim()
        .strip_prefix("<p>")
        .and_then(|s| s.strip_suffix("</p>"))
        .unwrap_or("")
        .trim();
    if caption.is_empty() {
        return table.to_string();
    }
    match table
        .find("<table")
        .and_then(|start| table[start..].find('>').map(|e| start + e + 1))
    {
        Some(insert_at) => format!(
            "{}<caption>{caption}</caption>{}",
            &table[..insert_at],
            &table[insert_at..]
        ),
        None => table.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_blocks() {
        let (attrs, used) =
            parse_attributes("{#fjord .right .wide width=40% alt=\"a b\"} rest").unwrap();
        assert_eq!(attrs.id.as_deref(), Some("fjord"));
        assert_eq!(attrs.classes, ["right", "wide"]);
        assert_eq!(
            attrs.pairs,
            [
                ("width".to_string(), "40%".to_string()),
                ("alt".to_string(), "a b".to_string())
            ]
        );
        assert_eq!(used, "{#fjord .right .wide width=40% alt=\"a b\"}".len());
        assert_eq!(parse_attributes("{-}").unwrap().0.classes, ["unnumbered"]);
    }

    #[test]
    fn not_attribute_blocks() {
        for text in [
            "{}",
            "{ }",
            "no brace",
            "{.a",
            "{just words}",
            "{#}",
            "{.}",
            "{a{b}",
        ] {
            assert!(parse_attributes(text).is_none(), "{text}");
        }
    }

    #[test]
    fn attribute_html_escapes() {
        let (attrs, _) = parse_attributes("{#\"a<b\" .x}").unwrap();
        assert_eq!(
            attribute_html(&attrs, Some("sk-div"), ""),
            " id=\"a&lt;b\" class=\"sk-div x\""
        );
        assert_eq!(
            add_to_first_tag("<h2>Title</h2>", &attrs),
            "<h2 id=\"user-content-a&lt;b\" class=\"x\">Title</h2>"
        );
    }

    #[test]
    fn divs_nest_with_any_fence_length() {
        let text = "::: {#outer .sidebar} :::\r\nA\n\n::: inner\nB\n:::\n:::\n\nAfter.\n";
        let (blanked, spans) = extract_divs(text);
        assert_eq!(blanked.len(), text.len());
        assert_eq!(blanked.matches(':').count(), 0);
        assert!(blanked.contains("\r\nA\n") && blanked.ends_with("\n\nAfter.\n"));
        assert_eq!(spans.len(), 2);
        assert_eq!(
            (spans[0].start, spans[0].attrs.id.as_deref()),
            (0, Some("outer"))
        );
        assert_eq!(spans[0].end, text.find("\n\nAfter").unwrap());
        assert_eq!(spans[1].attrs.classes, ["inner"]);
    }

    #[test]
    fn unclosed_divs_and_code_stay_text() {
        let text = "```\n::: a\n:::\n```\n::: never closed\ntext\n";
        let (blanked, spans) = extract_divs(text);
        assert_eq!(blanked, text);
        assert!(spans.is_empty());
    }

    #[test]
    fn div_info_strings() {
        assert_eq!(div_attributes("warning").classes, ["warning"]);
        assert_eq!(
            div_attributes("{#side .sidebar} ::::").id.as_deref(),
            Some("side")
        );
        assert_eq!(div_attributes("").classes, Vec::<String>::new());
    }

    #[test]
    fn captions_move_into_tables() {
        let html = "<table>\n<tr><td>1</td></tr>\n</table>\n<p>The <em>numbers</em></p>\n";
        assert_eq!(
            move_caption_into_table(html),
            "<table><caption>The <em>numbers</em></caption>\n<tr><td>1</td></tr>\n</table>"
        );
        assert_eq!(
            move_caption_into_table("<table></table>\n<p></p>"),
            "<table></table>"
        );
    }
}
