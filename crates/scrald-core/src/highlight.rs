//! Syntax highlighting for fenced code (DESIGN.md §2, §4): syntect emits
//! CSS classes (`sk-hl-keyword`, `sk-hl-string`, ...) instead of colors, so
//! the theme's palette decides how code looks.

use std::sync::OnceLock;

use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Prefix on every highlighting class, so they can't clash with others.
pub const CLASS_PREFIX: &str = "sk-hl-";

/// Code blocks larger than this stay plain: highlighting is roughly linear
/// but not free, and huge blocks are usually data, not code to read.
pub const MAX_HIGHLIGHT_BYTES: usize = 64 * 1024;

/// Fence languages rendered by the frontend (math, diagrams, music), never
/// highlighted as code.
const RENDERED_LANGUAGES: &[&str] = &["math", "mermaid", "abc"];

/// The syntax definitions, loaded once on first use (a few ms).
fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Highlighted HTML for `code` in `language` (the fence's first word, e.g.
/// `rust`, `py`, `bash`): the inside of a `<code>` element, as spans with
/// `sk-hl-*` classes. `None` when the language is unknown, rendered by the
/// frontend, or the code is too large; the caller then leaves it plain.
pub fn highlight(code: &str, language: &str) -> Option<String> {
    let language = language.trim();
    if language.is_empty()
        || code.len() > MAX_HIGHLIGHT_BYTES
        || RENDERED_LANGUAGES
            .iter()
            .any(|l| l.eq_ignore_ascii_case(language))
    {
        return None;
    }
    let set = syntaxes();
    let syntax = set
        .find_syntax_by_token(language)
        .or_else(|| set.find_syntax_by_token(&language.to_ascii_lowercase()))?;
    let mut generator = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        set,
        ClassStyle::SpacedPrefixed {
            prefix: CLASS_PREFIX,
        },
    );
    for line in LinesWithEndings::from(code) {
        generator
            .parse_html_for_line_which_includes_newline(line)
            .ok()?;
    }
    Some(generator.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_known_languages_with_prefixed_classes() {
        let html = highlight("fn main() {\n    let x = \"skål\";\n}\n", "rust").unwrap();
        assert!(html.contains("sk-hl-keyword"), "{html}");
        assert!(html.contains("sk-hl-string"), "{html}");
        assert!(html.contains("skål"));
        // Classes only, never inline colors.
        assert!(!html.contains("style="), "{html}");
    }

    #[test]
    fn language_tokens_and_case() {
        assert!(highlight("x = 1\n", "py").is_some());
        assert!(highlight("x = 1\n", "Python").is_some());
        assert!(highlight("echo hi\n", "bash").is_some());
    }

    #[test]
    fn plain_when_unknown_rendered_or_huge() {
        assert_eq!(highlight("x", "no-such-language"), None);
        assert_eq!(highlight("x", ""), None);
        assert_eq!(highlight("a^2", "math"), None);
        assert_eq!(highlight("graph TD", "mermaid"), None);
        assert_eq!(highlight("X:1\nK:C\nCDE|", "abc"), None);
        assert_eq!(highlight(&"x\n".repeat(MAX_HIGHLIGHT_BYTES), "rust"), None);
    }

    #[test]
    fn html_in_code_is_escaped() {
        let html = highlight("<script>alert(1)</script>\n", "rust").unwrap();
        assert!(!html.contains("<script>"), "{html}");
        // Escaped, though split across spans: `&lt;` + `script` + `&gt;`.
        assert!(html.contains("&lt;") && html.contains("&gt;"), "{html}");
    }
}
