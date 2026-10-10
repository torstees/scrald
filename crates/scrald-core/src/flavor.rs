//! Markdown flavors (DESIGN.md §5): GFM, Obsidian, and Pandoc as profiles
//! over one parser, plus detection and the resolution order.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// A Markdown dialect. Each is a set of comrak options (`comrak_options`);
/// they share most syntax and differ in a few conflicting constructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flavor {
    Gfm,
    Obsidian,
    Pandoc,
}

impl Flavor {
    pub const ALL: [Flavor; 3] = [Flavor::Gfm, Flavor::Obsidian, Flavor::Pandoc];

    /// Parses `gfm`, `obsidian`, or `pandoc` (any case, surrounding spaces ignored).
    pub fn from_name(name: &str) -> Option<Flavor> {
        match name.trim().to_ascii_lowercase().as_str() {
            "gfm" | "github" => Some(Flavor::Gfm),
            "obsidian" => Some(Flavor::Obsidian),
            "pandoc" => Some(Flavor::Pandoc),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Flavor::Gfm => "gfm",
            Flavor::Obsidian => "obsidian",
            Flavor::Pandoc => "pandoc",
        }
    }
}

/// Where a document's flavor came from (DESIGN.md §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FlavorSource {
    /// The user chose it for this document in Scrald.
    Document,
    /// `scrald-flavor` in front matter.
    FrontMatter,
    /// `flavor` in the nearest `.scrald.toml`.
    Folder,
    /// Detected from the document's content (and an Obsidian vault folder).
    Detected,
    /// Nothing pointed anywhere: GFM.
    Default,
}

/// The comrak options for a flavor. All three share GFM's core (tables,
/// task lists, footnotes, autolinks, math); the differences are the
/// conflicting constructs from DESIGN.md §5.1:
///
/// - `~x~` is strikethrough in GFM and Obsidian, but subscript in Pandoc.
/// - `[[wikilinks]]` and `==highlight==` are Obsidian syntax.
/// - `^[inline footnotes]`, `^superscript^`, definition lists, and smart
///   punctuation are Pandoc defaults.
/// - `> [!NOTE]` alerts: comrak's in GFM; in Obsidian, Scrald's callout
///   transform (`obsidian.rs`), which adds Obsidian's types, folding, titles.
pub fn comrak_options(flavor: Flavor) -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    let ext = &mut options.extension;
    ext.strikethrough = true;
    ext.table = true;
    ext.tasklist = true;
    ext.footnotes = true;
    ext.math_dollars = true;
    ext.math_code = true;
    match flavor {
        Flavor::Gfm => {
            ext.autolink = true;
            ext.alerts = true;
        }
        Flavor::Obsidian => {
            ext.autolink = true;
            // No comrak alerts: Scrald's callout transform handles `[!NOTE]`
            // and every Obsidian type, with folding and custom titles.
            // [[Note|shown text]]: Obsidian puts the target before the pipe.
            ext.wikilinks_title_after_pipe = true;
            ext.highlight = true;
        }
        Flavor::Pandoc => {
            // Overrides single-tilde strikethrough; `~~x~~` still strikes through.
            ext.subscript = true;
            ext.superscript = true;
            ext.inline_footnotes = true;
            ext.description_lists = true;
            // `{#id .class key=val}` after headings, fenced code, links,
            // images, and inline code; comrak parses them, `pandoc.rs`
            // renders them. `:::` fenced divs are found by `pandoc.rs` too.
            ext.header_attributes = true;
            ext.fenced_code_attributes = true;
            ext.link_attributes = true;
            ext.inline_code_attributes = true;
            options.parse.smart = true;
        }
    }
    // Raw HTML is passed through to ammonia, which removes anything unsafe.
    // Without this, comrak would replace raw HTML with a comment instead.
    options.render.r#unsafe = true;
    options
}

/// Callout types that only Obsidian uses. GitHub's alert types (note, tip,
/// important, warning, caution) say nothing about the flavor.
const OBSIDIAN_CALLOUTS: &[&str] = &[
    "abstract", "summary", "tldr", "info", "todo", "hint", "success", "check", "done", "question",
    "help", "faq", "failure", "fail", "missing", "danger", "error", "bug", "example", "quote",
    "cite",
];

/// Front matter keys that suggest a Pandoc document.
const PANDOC_KEYS: &[&str] = &[
    "bibliography",
    "csl",
    "link-citations",
    "reference-section-title",
];

/// Signal counts behind a detection, for tests and for explaining the result.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Signals {
    pub obsidian: u32,
    pub pandoc: u32,
}

/// Weight of being inside an Obsidian vault: more than any amount of syntax
/// that also happens to appear in other dialects.
const VAULT_WEIGHT: u32 = 10;

/// Guesses the flavor from a document's text (front matter included) and
/// whether it lives in an Obsidian vault. A fast line scan that ignores
/// fenced code, so it stays instant even at 500K words (DESIGN.md §5.2).
/// No signals means GFM.
pub fn detect(text: &str, in_vault: bool) -> (Flavor, Signals) {
    let mut signals = scan(text);
    if in_vault {
        signals.obsidian += VAULT_WEIGHT;
    }
    let flavor = if signals.obsidian == 0 && signals.pandoc == 0 {
        Flavor::Gfm
    } else if signals.obsidian >= signals.pandoc {
        Flavor::Obsidian
    } else {
        Flavor::Pandoc
    };
    (flavor, signals)
}

fn scan(text: &str) -> Signals {
    let mut s = Signals::default();
    let mut in_fence: Option<&str> = None;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_start();
        // Skip fenced code: its contents are not Markdown.
        let fence = if line.starts_with("```") {
            Some("```")
        } else if line.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        match (in_fence, fence) {
            (None, Some(f)) => {
                in_fence = Some(f);
                continue;
            }
            (Some(open), Some(f)) if open == f => {
                in_fence = None;
                continue;
            }
            (Some(_), _) => continue,
            (None, None) => {}
        }

        // Obsidian.
        if line.contains("[[") && line.contains("]]") {
            s.obsidian += 1;
        }
        if line.contains("%%") {
            s.obsidian += 1;
        }
        if has_highlight(line) {
            s.obsidian += 1;
        }
        if let Some(kind) = callout_type(line)
            && OBSIDIAN_CALLOUTS.contains(&kind.as_str())
        {
            s.obsidian += 2;
        }
        if line.starts_with('>')
            && line.contains("[!")
            && (line.contains("]+") || line.contains("]-"))
        {
            // Foldable callout: `> [!note]-`.
            s.obsidian += 2;
        }

        // Pandoc.
        if line.starts_with(":::") {
            s.pandoc += 2;
        }
        if line.contains("^[") {
            s.pandoc += 1;
        }
        if line.contains("{.") || line.contains("{#") {
            s.pandoc += 1;
        }
        if index == 0 && line.starts_with("% ") {
            s.pandoc += 2;
        }
        if PANDOC_KEYS
            .iter()
            .any(|key| line.starts_with(key) && line[key.len()..].trim_start().starts_with(':'))
        {
            s.pandoc += 2;
        }
    }
    s
}

/// `==text==` with something between the markers.
fn has_highlight(line: &str) -> bool {
    let mut rest = line;
    while let Some(start) = rest.find("==") {
        let after = &rest[start + 2..];
        match after.find("==") {
            Some(end) if end > 0 && !after[..end].starts_with(' ') => return true,
            Some(end) => rest = &after[end + 2..],
            None => return false,
        }
    }
    false
}

/// The type in a callout or alert line like `> [!Warning]+ Title`, lowercased.
fn callout_type(line: &str) -> Option<String> {
    let rest = line.strip_prefix('>')?.trim_start().strip_prefix("[!")?;
    let end = rest.find(']')?;
    Some(rest[..end].trim().to_ascii_lowercase())
}

/// Whether `doc_path` is inside an Obsidian vault: some ancestor folder
/// contains a `.obsidian` directory.
pub fn in_obsidian_vault(doc_path: &Path) -> bool {
    doc_path
        .parent()
        .is_some_and(|dir| dir.ancestors().any(|a| a.join(".obsidian").is_dir()))
}

/// The flavor a document uses: the per-document choice, then front matter,
/// then the folder config, then detection, then GFM (DESIGN.md §5.2).
/// `detected` is only consulted when nothing explicit applies.
pub fn resolve(
    document: Option<Flavor>,
    front_matter: Option<&str>,
    folder: Option<&str>,
    detected: impl FnOnce() -> Flavor,
) -> (Flavor, FlavorSource) {
    if let Some(flavor) = document {
        return (flavor, FlavorSource::Document);
    }
    if let Some(flavor) = front_matter.and_then(Flavor::from_name) {
        return (flavor, FlavorSource::FrontMatter);
    }
    if let Some(flavor) = folder.and_then(Flavor::from_name) {
        return (flavor, FlavorSource::Folder);
    }
    match detected() {
        Flavor::Gfm => (Flavor::Gfm, FlavorSource::Default),
        other => (other, FlavorSource::Detected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(flavor: Flavor, text: &str) -> String {
        comrak::markdown_to_html(text, &comrak_options(flavor))
    }

    #[test]
    fn names() {
        for flavor in Flavor::ALL {
            assert_eq!(Flavor::from_name(flavor.name()), Some(flavor));
        }
        assert_eq!(Flavor::from_name(" Obsidian "), Some(Flavor::Obsidian));
        assert_eq!(Flavor::from_name("markdown"), None);
    }

    #[test]
    fn tilde_conflict_is_resolved_per_flavor() {
        assert!(html(Flavor::Gfm, "H~2~O").contains("<del>2</del>"));
        assert!(html(Flavor::Pandoc, "H~2~O").contains("<sub>2</sub>"));
        // Double tildes strike through everywhere.
        assert!(html(Flavor::Pandoc, "~~gone~~").contains("<del>gone</del>"));
    }

    #[test]
    fn obsidian_syntax_only_in_obsidian() {
        let text = "See [[Other note|the note]] and ==this==.";
        let obsidian = html(Flavor::Obsidian, text);
        assert!(obsidian.contains("<mark>this</mark>"), "{obsidian}");
        assert!(obsidian.contains("data-wikilink"), "{obsidian}");
        let gfm = html(Flavor::Gfm, text);
        assert!(gfm.contains("[[Other note|the note]]"), "{gfm}");
        assert!(!gfm.contains("<mark>"));
    }

    #[test]
    fn pandoc_syntax_only_in_pandoc() {
        let text = "E = mc^2^ and a note.^[Inline footnote.]\n\nTerm\n: Definition\n";
        let pandoc = html(Flavor::Pandoc, text);
        assert!(pandoc.contains("<sup>2</sup>"), "{pandoc}");
        assert!(pandoc.contains("footnote-ref"), "{pandoc}");
        assert!(pandoc.contains("<dl>"), "{pandoc}");
        let gfm = html(Flavor::Gfm, text);
        assert!(
            !gfm.contains("<dl>") && !gfm.contains("<sup>2</sup>"),
            "{gfm}"
        );
    }

    #[test]
    fn plain_markdown_is_gfm() {
        let (flavor, signals) = detect("# Title\n\nJust *text* and a [link](x.md).\n", false);
        assert_eq!(flavor, Flavor::Gfm);
        assert_eq!(signals, Signals::default());
    }

    #[test]
    fn detects_obsidian() {
        let text = "# Daily\n\nMet [[Sigrid]] today. ==Important==.\n\n> [!todo]- Tasks\n> - call\n%% private %%\n";
        assert_eq!(detect(text, false).0, Flavor::Obsidian);
    }

    #[test]
    fn github_alerts_alone_are_not_obsidian() {
        let (flavor, _) = detect(
            "> [!NOTE]\n> GitHub alert\n\n> [!WARNING]\n> Careful\n",
            false,
        );
        assert_eq!(flavor, Flavor::Gfm);
    }

    #[test]
    fn detects_pandoc() {
        let text = "% The Title\n\n::: {.warning}\nCareful.^[Really.]\n:::\n\n# Heading {#intro}\n";
        assert_eq!(detect(text, false).0, Flavor::Pandoc);
        let with_keys = "---\nbibliography: refs.bib\ncsl: chicago.csl\n---\n# Paper\n";
        assert_eq!(detect(with_keys, false).0, Flavor::Pandoc);
    }

    #[test]
    fn code_is_ignored() {
        let text =
            "# Snippets\n\n```\n[[not a wikilink]] ::: ==x== ^[y]\n```\n\n~~~\n%% nope %%\n~~~\n";
        assert_eq!(detect(text, false).0, Flavor::Gfm);
    }

    #[test]
    fn vault_outweighs_stray_syntax() {
        let text = "# Note\n\n::: aside\nA Pandoc-looking div.\n:::\n";
        assert_eq!(detect(text, false).0, Flavor::Pandoc);
        assert_eq!(detect(text, true).0, Flavor::Obsidian);
    }

    #[test]
    fn vault_detection() {
        let root = std::env::temp_dir().join(format!("scrald-vault-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".obsidian")).unwrap();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        assert!(in_obsidian_vault(&root.join("notes").join("a.md")));
        assert!(in_obsidian_vault(&root.join("b.md")));
        let outside = std::env::temp_dir().join(format!("scrald-novault-{}", std::process::id()));
        std::fs::create_dir_all(&outside).unwrap();
        assert!(!in_obsidian_vault(&outside.join("c.md")));
    }

    #[test]
    fn resolution_order() {
        let never = || -> Flavor { panic!("detection should not run") };
        assert_eq!(
            resolve(Some(Flavor::Pandoc), Some("obsidian"), None, never),
            (Flavor::Pandoc, FlavorSource::Document)
        );
        assert_eq!(
            resolve(None, Some("obsidian"), Some("pandoc"), never),
            (Flavor::Obsidian, FlavorSource::FrontMatter)
        );
        assert_eq!(
            resolve(None, Some("bogus"), Some("pandoc"), never),
            (Flavor::Pandoc, FlavorSource::Folder)
        );
        assert_eq!(
            resolve(None, None, None, || Flavor::Obsidian),
            (Flavor::Obsidian, FlavorSource::Detected)
        );
        assert_eq!(
            resolve(None, None, None, || Flavor::Gfm),
            (Flavor::Gfm, FlavorSource::Default)
        );
    }

    #[test]
    fn detection_is_fast_on_large_documents() {
        let text = crate::generate::generate_document(500_000, 1);
        let started = std::time::Instant::now();
        let _ = detect(&text, false);
        // Generous bound for debug builds on CI; release is a few ms.
        assert!(
            started.elapsed() < std::time::Duration::from_millis(500),
            "{:?}",
            started.elapsed()
        );
    }
}
