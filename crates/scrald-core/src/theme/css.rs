//! Turns a theme into CSS: `--sk-*` custom properties, `@font-face` rules,
//! element styles, and the theme's own stylesheet with its `url()`s
//! confined to the theme folder (DESIGN.md §7.2, §13).

use std::fmt::Write;

use super::schema::{Appearance, BlockquoteStyle, FontSource, FontSpec, HrStyle, ThemeFile};

/// Largest `theme.css` Scrald will load, so a broken theme can't stall the UI.
pub const MAX_EXTRA_CSS: usize = 512 * 1024;

const BODY_FALLBACK: &str = "Georgia, \"Times New Roman\", serif";
const MONO_FALLBACK: &str = "\"Cascadia Code\", Consolas, \"SF Mono\", Menlo, monospace";
const UI_FALLBACK: &str = "system-ui, -apple-system, \"Segoe UI\", sans-serif";

/// The complete CSS for a theme. `asset_base` is the URL prefix for files in
/// the theme folder (ending in `/`); `extra_css` is the theme's `theme.css`.
pub fn theme_css(theme: &ThemeFile, asset_base: &str, extra_css: Option<&str>) -> String {
    let mut css = String::new();
    write_variables(&mut css, theme);
    write_font_faces(&mut css, theme, asset_base);
    write_elements(&mut css, theme, asset_base);
    if let Some(extra) = extra_css {
        css.push_str("\n/* theme.css */\n");
        css.push_str(&confine_urls(extra, asset_base));
        css.push('\n');
    }
    css
}

fn write_variables(css: &mut String, theme: &ThemeFile) {
    let c = theme.resolved_colors();
    let l = &theme.layout;
    let scheme = match theme.meta.appearance {
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    };
    // Rust note: `writeln!` into a String can't fail; `let _ =` discards the
    // `fmt::Result` it returns anyway.
    let _ = writeln!(css, ":root {{");
    let _ = writeln!(css, "  color-scheme: {scheme};");
    let colors = [
        ("background", c.background),
        ("foreground", c.foreground),
        ("muted", c.muted),
        ("accent", c.accent),
        ("link", c.link),
        ("selection", c.selection),
        ("border", c.border),
        ("code-bg", c.code_bg),
        ("quote-bar", c.quote_bar),
        ("highlight-bg", c.highlight_bg),
        ("ui-background", c.ui_background),
    ];
    for (name, color) in colors {
        let _ = writeln!(css, "  --sk-color-{name}: {color};");
    }
    for (name, value) in theme.palette.entries() {
        if let Some(color) = value.as_deref().and_then(super::color::Color::parse) {
            let _ = writeln!(css, "  --sk-palette-{name}: {color};");
        }
    }
    let body = font_stack(theme.fonts.body.as_ref(), BODY_FALLBACK);
    let heading = match &theme.fonts.heading {
        Some(spec) => font_stack(Some(spec), "var(--sk-font-body)"),
        None => "var(--sk-font-body)".to_string(),
    };
    let _ = writeln!(css, "  --sk-font-body: {body};");
    let _ = writeln!(css, "  --sk-font-heading: {heading};");
    let _ = writeln!(
        css,
        "  --sk-font-mono: {};",
        font_stack(theme.fonts.mono.as_ref(), MONO_FALLBACK)
    );
    let _ = writeln!(
        css,
        "  --sk-font-ui: {};",
        font_stack(theme.fonts.ui.as_ref(), UI_FALLBACK)
    );
    let _ = writeln!(css, "  --sk-measure: {};", l.measure);
    let _ = writeln!(css, "  --sk-font-size: {}px;", l.font_size);
    let _ = writeln!(css, "  --sk-line-height: {};", l.line_height);
    let _ = writeln!(css, "  --sk-paragraph-spacing: {}em;", l.paragraph_spacing);
    let _ = writeln!(css, "  --sk-min-font-size: {}px;", l.min_font_size);
    let _ = writeln!(css, "  --sk-max-font-size: {}px;", l.max_font_size);
    let _ = writeln!(css, "}}");
}

/// `"Family", "Fallback 1", <generic fallback>`.
fn font_stack(spec: Option<&FontSpec>, generic: &str) -> String {
    let Some(spec) = spec else {
        return generic.to_string();
    };
    let mut parts: Vec<String> = Vec::new();
    for family in std::iter::once(&spec.family).chain(spec.fallback.iter()) {
        let cleaned = clean_family(family);
        if !cleaned.is_empty() {
            parts.push(format!("\"{cleaned}\""));
        }
    }
    parts.push(generic.to_string());
    parts.join(", ")
}

/// A font family name safe to put inside a CSS double-quoted string.
fn clean_family(family: &str) -> String {
    family
        .chars()
        .filter(|c| !matches!(c, '"' | '\\' | ';' | '{' | '}' | '<' | '>') && !c.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}

fn write_font_faces(css: &mut String, theme: &ThemeFile, asset_base: &str) {
    let fonts = [
        &theme.fonts.body,
        &theme.fonts.heading,
        &theme.fonts.mono,
        &theme.fonts.ui,
    ];
    for spec in fonts.into_iter().flatten() {
        if spec.source != FontSource::Theme {
            continue;
        }
        for file in &spec.files {
            let _ = writeln!(
                css,
                "@font-face {{ font-family: \"{}\"; src: url(\"{}\"); font-weight: {}; font-style: {}; font-display: swap; }}",
                clean_family(&spec.family),
                asset_url(asset_base, &file.path),
                file.weight,
                if file.italic { "italic" } else { "normal" }
            );
        }
    }
}

fn write_elements(css: &mut String, theme: &ThemeFile, asset_base: &str) {
    let e = &theme.elements;
    let quote = match e.blockquote.style {
        BlockquoteStyle::Bar => {
            "border-left: 3px solid var(--sk-color-quote-bar); padding-left: 1em; color: var(--sk-color-muted);"
        }
        BlockquoteStyle::Indent => {
            "margin-left: 2em; margin-right: 2em; color: var(--sk-color-muted);"
        }
        BlockquoteStyle::Boxed => {
            "border: 1px solid var(--sk-color-border); background: var(--sk-color-code-bg); padding: 0.8em 1em; border-radius: 4px;"
        }
        BlockquoteStyle::PullQuote => {
            "font-size: 1.25em; text-align: center; border-top: 1px solid var(--sk-color-quote-bar); border-bottom: 1px solid var(--sk-color-quote-bar); padding: 0.6em 1.5em; margin-left: 0; margin-right: 0;"
        }
    };
    let italic = if e.blockquote.italic {
        " font-style: italic;"
    } else {
        ""
    };
    let _ = writeln!(css, ".sk-block blockquote {{ {quote}{italic} }}");

    let hr_base = ".sk-block hr { border: none; margin: 2em 0; text-align: center; }";
    match e.hr.style {
        HrStyle::Line => {
            let _ = writeln!(
                css,
                ".sk-block hr {{ border: none; border-top: 1px solid var(--sk-color-border); margin: 2em 0; }}"
            );
        }
        HrStyle::Dots => {
            let _ = writeln!(css, "{hr_base}");
            let _ = writeln!(
                css,
                ".sk-block hr::after {{ content: \"\\00b7  \\00b7  \\00b7\"; color: var(--sk-color-muted); letter-spacing: 0.5em; }}"
            );
        }
        HrStyle::Ornament => {
            let glyph = e.hr.glyph.as_deref().unwrap_or("\u{2766}");
            let _ = writeln!(css, "{hr_base}");
            let _ = writeln!(
                css,
                ".sk-block hr::after {{ content: {}; color: var(--sk-color-accent); font-size: 1.3em; }}",
                css_string(glyph)
            );
        }
        HrStyle::Image => {
            let image = e.hr.image.as_deref().unwrap_or_default();
            let _ = writeln!(
                css,
                ".sk-block hr {{ border: none; margin: 2em 0; height: 1.6em; background: url(\"{}\") center / contain no-repeat; }}",
                asset_url(asset_base, image)
            );
        }
    }

    if e.headings.h1_rule {
        let _ = writeln!(
            css,
            ".sk-block h1 {{ border-bottom: 1px solid var(--sk-color-border); padding-bottom: 0.3em; }}"
        );
    }
    if e.headings.numbering {
        // Numbers are computed by core (`data-number`), because CSS counters
        // can't cross the reader's contained sections.
        let _ = writeln!(
            css,
            ".sk-block [data-number]::before {{ content: attr(data-number) \"\\00a0\\00a0\"; color: var(--sk-color-muted); }}"
        );
    }
}

/// A CSS string literal; anything outside printable ASCII becomes `\hex `.
fn css_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == ' ' {
            out.push(c);
        } else {
            let _ = write!(out, "\\{:x} ", u32::from(c));
        }
    }
    out.push('"');
    out
}

/// URL of a file in the theme folder, with each path segment percent-encoded.
fn asset_url(asset_base: &str, relative: &str) -> String {
    let encoded: Vec<String> = relative
        .split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != ".")
        .map(percent_encode)
        .collect();
    format!("{asset_base}{}", encoded.join("/"))
}

fn percent_encode(segment: &str) -> String {
    let mut out = String::new();
    for b in segment.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Makes a theme's own stylesheet safe to load: drops `@import`, and points
/// every `url()` either into the theme folder or nowhere. Only `data:` URLs
/// and relative paths that stay inside the theme folder survive.
pub fn confine_urls(css: &str, asset_base: &str) -> String {
    let css = if css.len() > MAX_EXTRA_CSS {
        &css[..floor_char_boundary(css, MAX_EXTRA_CSS)]
    } else {
        css
    };
    let without_imports = strip_imports(css);
    let mut out = String::with_capacity(without_imports.len());
    let mut rest = without_imports.as_str();
    while let Some(start) = find_ignore_case(rest, "url(") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        let Some(end) = after.find(')') else {
            // Unclosed url(: drop the remainder rather than guess.
            rest = "";
            break;
        };
        let raw = after[..end].trim().trim_matches(['"', '\'']).trim();
        out.push_str(&format!("url(\"{}\")", confined_url(raw, asset_base)));
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn confined_url(raw: &str, asset_base: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.starts_with("data:") && !raw.contains('"') {
        return raw.to_string();
    }
    let inside_theme = !raw.is_empty()
        && !raw.contains(':')
        && !raw.starts_with('/')
        && !raw.starts_with('\\')
        && !raw.split(['/', '\\']).any(|segment| segment == "..");
    if inside_theme {
        asset_url(asset_base, raw.split(['?', '#']).next().unwrap_or_default())
    } else {
        // An empty URL loads nothing.
        String::new()
    }
}

/// Removes `@import ...;` rules, which could pull in remote stylesheets.
fn strip_imports(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = find_ignore_case(rest, "@import") {
        out.push_str(&rest[..start]);
        rest = match rest[start..].find(';') {
            Some(end) => &rest[start + end + 1..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

fn find_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "http://scrald-theme.localhost/nordic/";

    fn theme(extra: &str) -> ThemeFile {
        ThemeFile::parse(&format!(
            "[meta]\nname = \"T\"\nschema = 1\nappearance = \"dark\"\n{extra}"
        ))
        .unwrap()
    }

    #[test]
    fn variables_cover_colors_fonts_and_layout() {
        let css = theme_css(
            &theme("[colors]\nbackground = \"#2e3440\"\n[palette]\nred = \"#bf616a\"\n"),
            BASE,
            None,
        );
        assert!(css.contains("color-scheme: dark;"));
        assert!(css.contains("--sk-color-background: #2e3440;"));
        assert!(css.contains("--sk-palette-red: #bf616a;"));
        assert!(css.contains("--sk-font-heading: var(--sk-font-body);"));
        assert!(css.contains("--sk-measure: 66;"));
        assert!(css.contains("--sk-font-size: 18px;"));
    }

    #[test]
    fn font_stacks_are_quoted_and_cleaned() {
        let css = theme_css(
            &theme("[fonts]\nbody = { family = \"Lit\\\"era;ta\", fallback = [\"Georgia\"] }\n"),
            BASE,
            None,
        );
        assert!(
            css.contains(
                "--sk-font-body: \"Literata\", \"Georgia\", Georgia, \"Times New Roman\", serif;"
            ),
            "{css}"
        );
    }

    #[test]
    fn theme_fonts_get_font_face_rules() {
        let css = theme_css(
            &theme(
                "[fonts]\nbody = { family = \"Saga\", source = \"theme\", files = [{ path = \"assets/Saga Bold.woff2\", weight = 700 }] }\n",
            ),
            BASE,
            None,
        );
        assert!(
            css.contains("src: url(\"http://scrald-theme.localhost/nordic/assets/Saga%20Bold.woff2\"); font-weight: 700;"),
            "{css}"
        );
    }

    #[test]
    fn element_styles() {
        let css = theme_css(
            &theme(
                "[elements.blockquote]\nstyle = \"boxed\"\nitalic = true\n[elements.hr]\nstyle = \"ornament\"\nglyph = \"❦\"\n[elements.headings]\nh1_rule = true\nnumbering = true\n",
            ),
            BASE,
            None,
        );
        assert!(css.contains("background: var(--sk-color-code-bg); padding: 0.8em 1em; border-radius: 4px; font-style: italic;"));
        assert!(css.contains("content: \"\\2766 \";"), "{css}");
        assert!(css.contains(".sk-block h1 { border-bottom"));
        assert!(css.contains("[data-number]::before"));
    }

    #[test]
    fn extra_css_urls_are_confined_to_the_theme() {
        let extra = "@import url(https://evil.example/x.css);\n\
                     body { background: url('assets/paper.png') }\n\
                     .a { background: url(https://tracker.example/p.gif) }\n\
                     .b { background: url(../../secret.png) }\n\
                     .c { background: url(/etc/passwd) }\n\
                     .d { background: url(data:image/png;base64,AA) }\n\
                     .e { background: URL(file:///c:/x.png) }";
        let out = confine_urls(extra, BASE);
        assert!(!out.contains("@import"));
        assert!(out.contains("url(\"http://scrald-theme.localhost/nordic/assets/paper.png\")"));
        assert!(!out.contains("tracker.example"));
        assert!(!out.contains("secret"));
        assert!(!out.contains("passwd"));
        assert!(!out.contains("file:"));
        assert!(out.contains("url(\"data:image/png;base64,AA\")"));
        assert_eq!(out.matches("url(\"\")").count(), 4, "{out}");
    }

    #[test]
    fn unclosed_url_is_dropped() {
        assert_eq!(confine_urls("a { b: url(x.png", BASE), "a { b: ");
    }

    #[test]
    fn css_string_escapes() {
        assert_eq!(css_string("a\"b"), "\"a\\22 b\"");
        assert_eq!(css_string("❦"), "\"\\2766 \"");
    }
}
