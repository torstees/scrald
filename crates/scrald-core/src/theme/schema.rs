//! The `theme.toml` format (DESIGN.md §7.2) and its validation.
//!
//! Every section except `[meta]` is optional; anything left out gets a
//! sensible default for the theme's light or dark appearance.

use serde::{Deserialize, Serialize};

use super::color::Color;

/// The schema version this build of Scrald understands.
pub const SCHEMA_VERSION: u32 = 1;

// Rust note: `#[serde(deny_unknown_fields)]` makes a misspelled key an error
// ("unknown field `backgroud`") instead of being silently ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeFile {
    pub meta: Meta,
    #[serde(default)]
    pub colors: Colors,
    #[serde(default)]
    pub palette: Palette,
    #[serde(default)]
    pub fonts: Fonts,
    #[serde(default)]
    pub layout: Layout,
    #[serde(default)]
    pub elements: Elements,
    #[serde(default)]
    pub syntax: Syntax,
    #[serde(default)]
    pub css: CssSection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub name: String,
    pub author: Option<String>,
    pub schema: u32,
    pub appearance: Appearance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
}

/// Semantic colors, as hex strings. Missing ones are derived (see `ResolvedColors`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Colors {
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub muted: Option<String>,
    pub accent: Option<String>,
    pub link: Option<String>,
    pub selection: Option<String>,
    pub border: Option<String>,
    pub code_bg: Option<String>,
    pub quote_bar: Option<String>,
    pub highlight_bg: Option<String>,
    /// Background of the sidebar and status bar.
    pub ui_background: Option<String>,
}

/// The 16 ANSI colors. Seeds syntax highlighting and callout colors (M4).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub black: Option<String>,
    pub red: Option<String>,
    pub green: Option<String>,
    pub yellow: Option<String>,
    pub blue: Option<String>,
    pub magenta: Option<String>,
    pub cyan: Option<String>,
    pub white: Option<String>,
    pub bright_black: Option<String>,
    pub bright_red: Option<String>,
    pub bright_green: Option<String>,
    pub bright_yellow: Option<String>,
    pub bright_blue: Option<String>,
    pub bright_magenta: Option<String>,
    pub bright_cyan: Option<String>,
    pub bright_white: Option<String>,
}

impl Palette {
    /// (name, value) pairs in a fixed order, for validation and CSS output.
    pub fn entries(&self) -> [(&'static str, &Option<String>); 16] {
        [
            ("black", &self.black),
            ("red", &self.red),
            ("green", &self.green),
            ("yellow", &self.yellow),
            ("blue", &self.blue),
            ("magenta", &self.magenta),
            ("cyan", &self.cyan),
            ("white", &self.white),
            ("bright-black", &self.bright_black),
            ("bright-red", &self.bright_red),
            ("bright-green", &self.bright_green),
            ("bright-yellow", &self.bright_yellow),
            ("bright-blue", &self.bright_blue),
            ("bright-magenta", &self.bright_magenta),
            ("bright-cyan", &self.bright_cyan),
            ("bright-white", &self.bright_white),
        ]
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fonts {
    pub body: Option<FontSpec>,
    pub heading: Option<FontSpec>,
    pub mono: Option<FontSpec>,
    /// Sidebar, status bar, and other interface text.
    pub ui: Option<FontSpec>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontSpec {
    pub family: String,
    #[serde(default)]
    pub source: FontSource,
    /// Weights the theme uses (for Google Fonts downloads, M8).
    #[serde(default)]
    pub weights: Vec<u16>,
    #[serde(default)]
    pub italic: bool,
    /// Families to try if this one isn't available.
    #[serde(default)]
    pub fallback: Vec<String>,
    /// Font files in the theme folder, for `source = "theme"`.
    #[serde(default)]
    pub files: Vec<FontFile>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontSource {
    /// Installed on the system.
    #[default]
    System,
    /// Downloaded once from Google Fonts and cached (M8). Until then it
    /// works if the family is installed locally.
    Google,
    /// Shipped in the theme's folder; see `FontSpec::files`.
    Theme,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontFile {
    /// Path relative to the theme folder, e.g. `assets/Literata-Regular.woff2`.
    pub path: String,
    #[serde(default = "default_weight")]
    pub weight: u16,
    #[serde(default)]
    pub italic: bool,
}

fn default_weight() -> u16 {
    400
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TextSizing {
    #[default]
    Fixed,
    Fit,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    /// Characters per line.
    pub measure: u32,
    /// Base font size in px at 100% zoom.
    pub font_size: f32,
    pub line_height: f32,
    /// Space between paragraphs, in em.
    pub paragraph_spacing: f32,
    pub text_sizing: TextSizing,
    pub min_font_size: f32,
    pub max_font_size: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            measure: 66,
            font_size: 18.0,
            line_height: 1.6,
            paragraph_spacing: 1.0,
            text_sizing: TextSizing::Fixed,
            min_font_size: 14.0,
            max_font_size: 32.0,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elements {
    #[serde(default)]
    pub blockquote: Blockquote,
    #[serde(default)]
    pub hr: Hr,
    #[serde(default)]
    pub headings: Headings,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Blockquote {
    pub style: BlockquoteStyle,
    pub italic: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockquoteStyle {
    #[default]
    Bar,
    Indent,
    Boxed,
    PullQuote,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Hr {
    pub style: HrStyle,
    /// The ornament for `style = "ornament"`, e.g. "❦".
    pub glyph: Option<String>,
    /// Image path (relative to the theme folder) for `style = "image"`.
    pub image: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HrStyle {
    #[default]
    Line,
    Dots,
    Ornament,
    Image,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Headings {
    /// Number headings 1, 1.1, 1.1.1, ... (H2 and below; H1 is the title).
    pub numbering: bool,
    /// A rule under H1.
    pub h1_rule: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Syntax {
    /// "palette" derives syntax colors from `[palette]` (M4).
    pub source: String,
}

impl Default for Syntax {
    fn default() -> Self {
        Syntax {
            source: "palette".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CssSection {
    /// Extra stylesheet in the theme folder, e.g. "theme.css".
    pub file: Option<String>,
}

/// Every color the reader uses, with gaps filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedColors {
    pub background: Color,
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
    pub link: Color,
    pub selection: Color,
    pub border: Color,
    pub code_bg: Color,
    pub quote_bar: Color,
    pub highlight_bg: Color,
    pub ui_background: Color,
}

impl ThemeFile {
    /// Parses and validates TOML text. Errors read like
    /// "line 7, column 14: invalid color "#12" for colors.background".
    pub fn parse(text: &str) -> Result<ThemeFile, Vec<String>> {
        let file: ThemeFile =
            toml::from_str(text).map_err(|e| vec![describe_toml_error(text, &e)])?;
        let problems = file.validate();
        if problems.is_empty() {
            Ok(file)
        } else {
            Err(problems)
        }
    }

    /// Checks everything TOML parsing can't: color syntax, number ranges,
    /// and file paths that must stay inside the theme folder.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.meta.schema != SCHEMA_VERSION {
            problems.push(format!(
                "meta.schema is {}, but this version of Scrald reads schema {SCHEMA_VERSION}",
                self.meta.schema
            ));
        }
        if self.meta.name.trim().is_empty() {
            problems.push("meta.name must not be empty".to_string());
        }

        let c = &self.colors;
        let colors = [
            ("background", &c.background),
            ("foreground", &c.foreground),
            ("muted", &c.muted),
            ("accent", &c.accent),
            ("link", &c.link),
            ("selection", &c.selection),
            ("border", &c.border),
            ("code_bg", &c.code_bg),
            ("quote_bar", &c.quote_bar),
            ("highlight_bg", &c.highlight_bg),
            ("ui_background", &c.ui_background),
        ];
        for (name, value) in colors {
            check_color(&mut problems, "colors", name, value);
        }
        for (name, value) in self.palette.entries() {
            check_color(&mut problems, "palette", &name.replace('-', "_"), value);
        }

        let l = &self.layout;
        check_range(
            &mut problems,
            "layout.measure",
            l.measure as f32,
            20.0,
            200.0,
        );
        check_range(&mut problems, "layout.font_size", l.font_size, 6.0, 96.0);
        check_range(&mut problems, "layout.line_height", l.line_height, 0.8, 3.0);
        check_range(
            &mut problems,
            "layout.paragraph_spacing",
            l.paragraph_spacing,
            0.0,
            5.0,
        );
        check_range(
            &mut problems,
            "layout.min_font_size",
            l.min_font_size,
            6.0,
            96.0,
        );
        check_range(
            &mut problems,
            "layout.max_font_size",
            l.max_font_size,
            6.0,
            96.0,
        );
        if l.min_font_size > l.max_font_size {
            problems.push(
                "layout.min_font_size must not be larger than layout.max_font_size".to_string(),
            );
        }

        let fonts = [
            ("body", &self.fonts.body),
            ("heading", &self.fonts.heading),
            ("mono", &self.fonts.mono),
            ("ui", &self.fonts.ui),
        ];
        for (slot, spec) in fonts {
            let Some(spec) = spec else { continue };
            if spec.family.trim().is_empty() {
                problems.push(format!("fonts.{slot}.family must not be empty"));
            }
            if spec.source == FontSource::Theme && spec.files.is_empty() {
                problems.push(format!("fonts.{slot} has source = \"theme\" but no files"));
            }
            for file in &spec.files {
                check_relative_path(&mut problems, &format!("fonts.{slot}.files"), &file.path);
                check_range(
                    &mut problems,
                    &format!("fonts.{slot}.files weight"),
                    f32::from(file.weight),
                    1.0,
                    1000.0,
                );
            }
        }

        let hr = &self.elements.hr;
        match hr.style {
            HrStyle::Ornament => match &hr.glyph {
                Some(g) if !g.trim().is_empty() && g.chars().count() <= 8 => {}
                _ => problems.push(
                    "elements.hr.glyph must be 1 to 8 characters when style = \"ornament\""
                        .to_string(),
                ),
            },
            HrStyle::Image => match &hr.image {
                Some(path) => check_relative_path(&mut problems, "elements.hr.image", path),
                None => problems
                    .push("elements.hr.image is required when style = \"image\"".to_string()),
            },
            HrStyle::Line | HrStyle::Dots => {}
        }
        if let Some(file) = &self.css.file {
            check_relative_path(&mut problems, "css.file", file);
        }
        problems
    }

    /// All semantic colors, deriving any the theme leaves out from its
    /// background, foreground, and appearance.
    pub fn resolved_colors(&self) -> ResolvedColors {
        let dark = self.meta.appearance == Appearance::Dark;
        let c = &self.colors;
        let pick = |value: &Option<String>, fallback: Color| {
            value.as_deref().and_then(Color::parse).unwrap_or(fallback)
        };
        let background = pick(
            &c.background,
            if dark {
                Color::rgb(0x22, 0x21, 0x1f)
            } else {
                Color::rgb(0xfb, 0xf8, 0xf1)
            },
        );
        let foreground = pick(
            &c.foreground,
            if dark {
                Color::rgb(0xe6, 0xe1, 0xd6)
            } else {
                Color::rgb(0x2b, 0x2a, 0x27)
            },
        );
        let accent = pick(
            &c.accent,
            pick(
                &self.palette.yellow,
                if dark {
                    Color::rgb(0xd6, 0xa4, 0x63)
                } else {
                    Color::rgb(0x8a, 0x5a, 0x2b)
                },
            ),
        );
        let link = pick(
            &c.link,
            pick(
                &self.palette.blue,
                if dark {
                    Color::rgb(0x8f, 0xb6, 0xd9)
                } else {
                    Color::rgb(0x2f, 0x5f, 0x8a)
                },
            ),
        );
        ResolvedColors {
            background,
            foreground,
            muted: pick(&c.muted, foreground.mix(background, 0.45)),
            accent,
            link,
            selection: pick(
                &c.selection,
                background.mix(accent, if dark { 0.25 } else { 0.2 }),
            ),
            border: pick(&c.border, background.mix(foreground, 0.15)),
            code_bg: pick(
                &c.code_bg,
                background.mix(foreground, if dark { 0.06 } else { 0.05 }),
            ),
            quote_bar: pick(&c.quote_bar, background.mix(accent, 0.5)),
            highlight_bg: pick(&c.highlight_bg, accent.with_alpha(0.27)),
            // Chrome sits a little darker than the page in dark themes, a
            // little toward the text color in light ones.
            ui_background: pick(
                &c.ui_background,
                if dark {
                    background.mix(Color::rgb(0, 0, 0), 0.18)
                } else {
                    background.mix(foreground, 0.03)
                },
            ),
        }
    }
}

fn check_color(problems: &mut Vec<String>, section: &str, name: &str, value: &Option<String>) {
    if let Some(v) = value
        && Color::parse(v).is_none()
    {
        problems.push(format!(
            "{section}.{name}: \"{v}\" is not a color; use #rgb, #rrggbb, or #rrggbbaa"
        ));
    }
}

fn check_range(problems: &mut Vec<String>, name: &str, value: f32, min: f32, max: f32) {
    if !(min..=max).contains(&value) {
        problems.push(format!(
            "{name} is {value}, but must be between {min} and {max}"
        ));
    }
}

/// Paths in a theme must be relative and stay inside the theme folder.
fn check_relative_path(problems: &mut Vec<String>, name: &str, path: &str) {
    let p = std::path::Path::new(path);
    let escapes = p.components().any(|c| {
        !matches!(
            c,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    });
    if path.trim().is_empty() || escapes || path.contains(':') {
        problems.push(format!(
            "{name}: \"{path}\" must be a relative path inside the theme folder (no \"..\", drive letters, or leading slash)"
        ));
    }
}

/// A TOML error as "line L, column C: message".
fn describe_toml_error(text: &str, error: &toml::de::Error) -> String {
    let message = error.message().trim().to_string();
    match error.span() {
        Some(span) => {
            let before = &text[..span.start.min(text.len())];
            let line = before.matches('\n').count() + 1;
            let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
            format!("line {line}, column {column}: {message}")
        }
        None => message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = "[meta]\nname = \"Mini\"\nschema = 1\nappearance = \"light\"\n";

    #[test]
    fn minimal_theme_gets_defaults() {
        let theme = ThemeFile::parse(MINIMAL).unwrap();
        assert_eq!(theme.layout.measure, 66);
        assert_eq!(theme.elements.blockquote.style, BlockquoteStyle::Bar);
        let colors = theme.resolved_colors();
        assert_eq!(colors.background, Color::rgb(0xfb, 0xf8, 0xf1));
    }

    #[test]
    fn design_doc_example_parses() {
        let text = r##"
[meta]
name = "Nordic Night"
author = "You"
schema = 1
appearance = "dark"

[colors]
background   = "#2e3440"
foreground   = "#d8dee9"
highlight_bg = "#ebcb8b44"

[palette]
black = "#3b4252"
red = "#bf616a"
bright_black = "#4c566a"

[fonts]
body    = { family = "Literata", source = "google", weights = [400, 700], italic = true }
mono    = { family = "JetBrains Mono", source = "system", fallback = ["Cascadia Code", "Consolas"] }

[layout]
measure = 66
text_sizing = "fit"

[elements.blockquote]
style = "pull-quote"
italic = true

[elements.hr]
style = "ornament"
glyph = "❦"

[elements.headings]
numbering = true
h1_rule = true

[syntax]
source = "palette"
"##;
        let theme = ThemeFile::parse(text).unwrap();
        assert_eq!(theme.meta.appearance, Appearance::Dark);
        assert_eq!(theme.layout.text_sizing, TextSizing::Fit);
        assert_eq!(theme.elements.blockquote.style, BlockquoteStyle::PullQuote);
        assert_eq!(
            theme.resolved_colors().background,
            Color::rgb(0x2e, 0x34, 0x40)
        );
    }

    #[test]
    fn typos_are_reported_with_location() {
        let text = "[meta]\nname = \"X\"\nschema = 1\nappearance = \"light\"\n[colors]\nbackgroud = \"#000\"\n";
        let errors = ThemeFile::parse(text).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("line 6, column 1:"), "{}", errors[0]);
        assert!(errors[0].contains("backgroud"), "{}", errors[0]);
    }

    #[test]
    fn bad_values_are_all_reported() {
        let text = format!(
            "{MINIMAL}[colors]\nbackground = \"blue\"\n[layout]\nmeasure = 5\nmin_font_size = 40\nmax_font_size = 20\n\
             [elements.hr]\nstyle = \"image\"\n[css]\nfile = \"../../secrets.css\"\n"
        );
        let errors = ThemeFile::parse(&text).unwrap_err();
        let joined = errors.join("\n");
        assert!(joined.contains("colors.background"), "{joined}");
        assert!(joined.contains("layout.measure"), "{joined}");
        assert!(
            joined.contains("min_font_size must not be larger"),
            "{joined}"
        );
        assert!(joined.contains("elements.hr.image is required"), "{joined}");
        assert!(joined.contains("css.file"), "{joined}");
    }

    #[test]
    fn schema_version_is_checked() {
        let errors = ThemeFile::parse(&MINIMAL.replace("schema = 1", "schema = 2")).unwrap_err();
        assert!(errors[0].contains("schema"), "{}", errors[0]);
    }

    #[test]
    fn paths_must_stay_inside_the_theme() {
        let mut problems = Vec::new();
        for ok in ["assets/a.woff2", "./theme.css", "x.svg"] {
            check_relative_path(&mut problems, "p", ok);
        }
        assert!(problems.is_empty(), "{problems:?}");
        for bad in ["../x.css", "/etc/x", "C:/x.css", "assets/../../x", ""] {
            let mut p = Vec::new();
            check_relative_path(&mut p, "p", bad);
            assert_eq!(p.len(), 1, "{bad}");
        }
    }
}
