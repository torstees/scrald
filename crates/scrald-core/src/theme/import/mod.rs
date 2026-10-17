//! Creating themes from terminal color schemes (DESIGN.md §7.3).
//!
//! Each supported format is read into a `TerminalScheme`; `theme_toml` then
//! turns a scheme into an ordinary `theme.toml`, so an imported theme needs no
//! special handling afterwards. The semantic colors a terminal doesn't have
//! (muted, accent, link, ...) are derived and written out, so the user can
//! adjust them.

pub mod windows_terminal;

use super::color::Color;
use super::schema::{
    Appearance, Colors, CssSection, Elements, Fonts, Layout, Meta, Palette, SCHEMA_VERSION, Syntax,
    ThemeFile,
};

/// Reads the color schemes in a terminal theme file. Returns them with a
/// description of the format, for the new theme's comment. (Windows Terminal
/// only, so far; DESIGN.md §7.3 lists the formats to come.)
pub fn read_schemes(text: &str) -> Result<(Vec<TerminalScheme>, &'static str), String> {
    windows_terminal::parse(text).map(|schemes| (schemes, windows_terminal::FORMAT))
}

/// A terminal color scheme, as read from any supported format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalScheme {
    pub name: String,
    pub background: Color,
    pub foreground: Color,
    pub cursor: Option<Color>,
    pub selection: Option<Color>,
    /// The 16 ANSI colors in the usual order: black, red, green, yellow,
    /// blue, magenta, cyan, white, then the bright variants.
    pub ansi: [Color; 16],
}

const BLACK: usize = 0;
const YELLOW: usize = 3;
const BLUE: usize = 4;
const BRIGHT: usize = 8;

/// Contrast with the background that links need: body text's 4.5:1.
const LINK_CONTRAST: f32 = 4.5;
/// The accent colors quote bars, rules, and other details: 3:1, the WCAG
/// minimum for large text and graphics.
const ACCENT_CONTRAST: f32 = 3.0;
/// Muted text (captions, the status bar) still has to be read: 4.5:1, or
/// three quarters of the body text's own contrast if that is lower.
const MUTED_CONTRAST: f32 = 4.5;
/// Text on a selection: 4.5:1, or 80% of the text's own contrast.
const SELECTION_CONTRAST: f32 = 4.5;
/// How different a selection must be from the page to be seen at all.
const SELECTION_VISIBLE: f32 = 1.15;

impl TerminalScheme {
    /// Light or dark, from which of black or white text reads better on the
    /// background.
    pub fn appearance(&self) -> Appearance {
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);
        if self.background.contrast(white) >= self.background.contrast(black) {
            Appearance::Dark
        } else {
            Appearance::Light
        }
    }

    /// The scheme as a theme, with every semantic color filled in.
    pub fn to_theme(&self) -> ThemeFile {
        let bg = self.background;
        let fg = self.foreground;
        let text_contrast = fg.contrast(bg);
        // Both the normal and the bright variant, each made readable; then
        // the more colorful of the two. (Some schemes, like Solarized, put
        // grays in the bright slots.)
        let pick = |index: usize, minimum: f32| {
            let normal = readable(self.ansi[index], bg, minimum);
            let bright = readable(self.ansi[index + BRIGHT], bg, minimum);
            if colorfulness(bright) > colorfulness(normal) {
                bright
            } else {
                normal
            }
        };
        let accent = pick(YELLOW, ACCENT_CONTRAST);
        let link = pick(BLUE, LINK_CONTRAST);
        let muted = muted(fg, bg, MUTED_CONTRAST.min(text_contrast * 0.75));
        // Terminals draw selections translucently over the text, so the
        // scheme's color (often plain white) may not work as a solid one.
        let text_on_selection = SELECTION_CONTRAST.min(text_contrast * 0.8);
        let candidates = [
            self.selection.map(|c| Color { a: 255, ..c }),
            Some(bg.mix(accent, 0.3)),
            Some(bg.mix(fg, 0.2)),
        ];
        let selection = candidates
            .into_iter()
            .flatten()
            .find_map(|c| solid_selection(c, bg, fg, text_on_selection))
            .unwrap_or_else(|| bg.mix(fg, 0.15));
        let hex = |c: Color| Some(c.to_string());
        let mut theme = ThemeFile {
            meta: Meta {
                name: self.name.clone(),
                author: None,
                schema: SCHEMA_VERSION,
                appearance: self.appearance(),
            },
            colors: Colors {
                background: hex(bg),
                foreground: hex(fg),
                muted: hex(muted),
                accent: hex(accent),
                link: hex(link),
                selection: hex(selection),
                ..Colors::default()
            },
            palette: palette(&self.ansi),
            fonts: Fonts::default(),
            layout: Layout::default(),
            elements: Elements::default(),
            syntax: Syntax::default(),
            css: CssSection::default(),
        };
        // The rest (border, code background, ...) are derived from these the
        // same way as for any theme that leaves them out, then written down.
        let resolved = theme.resolved_colors();
        let c = &mut theme.colors;
        c.border = hex(resolved.border);
        c.code_bg = hex(resolved.code_bg);
        c.quote_bar = hex(resolved.quote_bar);
        c.highlight_bg = hex(resolved.highlight_bg);
        c.ui_background = hex(resolved.ui_background);
        theme
    }
}

/// Darkens `color` (on a light background) or lightens it (on a dark one) in
/// small steps until it has at least `minimum` contrast with `bg`. Black or
/// white has at least 4.58:1 with any color, so this ends by then.
fn readable(color: Color, bg: Color, minimum: f32) -> Color {
    let black = Color::rgb(0, 0, 0);
    let white = Color::rgb(255, 255, 255);
    let toward = if bg.contrast(white) >= bg.contrast(black) {
        white
    } else {
        black
    };
    // Rust note: `(0..=20).map(...)` is a lazy range of steps; `find` stops at
    // the first that passes, like a `for` loop with an early `return`.
    (0..=20)
        .map(|step| color.mix(toward, step as f32 / 20.0))
        .find(|c| c.contrast(bg) >= minimum)
        .unwrap_or(toward)
}

/// Text color between `fg` and `bg`: as far toward the background as it can
/// go (up to 45%) while keeping `minimum` contrast.
fn muted(fg: Color, bg: Color, minimum: f32) -> Color {
    (0..=9)
        .rev()
        .map(|step| fg.mix(bg, step as f32 * 0.05))
        .find(|c| c.contrast(bg) >= minimum)
        .unwrap_or(fg)
}

/// `color` moved toward the page background until the text on it has
/// `text_minimum` contrast, as long as it still stands out from the page.
fn solid_selection(color: Color, bg: Color, fg: Color, text_minimum: f32) -> Option<Color> {
    (0..=16)
        .map(|step| color.mix(bg, step as f32 / 20.0))
        .take_while(|c| c.contrast(bg) >= SELECTION_VISIBLE)
        .find(|c| fg.contrast(*c) >= text_minimum)
}

/// How far a color is from gray: its largest channel minus its smallest.
fn colorfulness(c: Color) -> u8 {
    c.r.max(c.g).max(c.b) - c.r.min(c.g).min(c.b)
}

fn palette(ansi: &[Color; 16]) -> Palette {
    let hex = |i: usize| Some(ansi[i].to_string());
    Palette {
        black: hex(BLACK),
        red: hex(1),
        green: hex(2),
        yellow: hex(YELLOW),
        blue: hex(BLUE),
        magenta: hex(5),
        cyan: hex(6),
        white: hex(7),
        bright_black: hex(BRIGHT),
        bright_red: hex(9),
        bright_green: hex(10),
        bright_yellow: hex(11),
        bright_blue: hex(12),
        bright_magenta: hex(13),
        bright_cyan: hex(14),
        bright_white: hex(15),
    }
}

/// The text of a `theme.toml` for `scheme`. `format` names where it came
/// from ("a Windows Terminal color scheme"), for the comment at the top.
pub fn theme_toml(scheme: &TerminalScheme, format: &str) -> String {
    let theme = scheme.to_theme();
    let appearance = match theme.meta.appearance {
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    };
    let mut out = String::new();
    out.push_str(&format!(
        "# Imported by Scrald from {format}.\n\
         # The colors under [colors] other than background and foreground were\n\
         # derived from the palette for readable text; adjust them freely.\n\n"
    ));
    out.push_str("[meta]\n");
    out.push_str(&format!("name = {}\n", toml_string(&theme.meta.name)));
    out.push_str(&format!("schema = {SCHEMA_VERSION}\n"));
    out.push_str(&format!("appearance = \"{appearance}\"\n\n"));

    out.push_str("[colors]\n");
    let c = &theme.colors;
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
    push_table(&mut out, &colors);

    out.push_str("\n[palette]\n");
    // `entries` names them with hyphens, for CSS; TOML keys use underscores.
    let entries = theme.palette.entries();
    let palette: Vec<(String, &Option<String>)> = entries
        .iter()
        .map(|(name, value)| (name.replace('-', "_"), *value))
        .collect();
    let palette: Vec<(&str, &Option<String>)> =
        palette.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    push_table(&mut out, &palette);
    out
}

/// Writes `key = "value"` lines with the values lined up.
fn push_table(out: &mut String, rows: &[(&str, &Option<String>)]) {
    let width = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (key, value) in rows {
        if let Some(value) = value {
            out.push_str(&format!("{key:width$} = \"{value}\"\n"));
        }
    }
}

/// A TOML basic string, quoted and escaped.
fn toml_string(text: &str) -> String {
    toml::Value::String(text.to_string()).to_string()
}

/// A folder name for a theme called `name`: lowercase letters, digits, and
/// hyphens ("One Half Dark" becomes "one-half-dark").
pub fn theme_id(name: &str) -> String {
    let mut id = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
        } else if !id.ends_with('-') && !id.is_empty() {
            id.push('-');
        }
    }
    let id = id.trim_end_matches('-').to_string();
    if id.is_empty() {
        "imported".to_string()
    } else {
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(hex: &str) -> Color {
        Color::parse(hex).expect("test color")
    }

    /// Windows Terminal's default "Campbell" scheme.
    fn campbell() -> TerminalScheme {
        TerminalScheme {
            name: "Campbell".to_string(),
            background: c("#0C0C0C"),
            foreground: c("#CCCCCC"),
            cursor: Some(c("#FFFFFF")),
            selection: Some(c("#FFFFFF")),
            ansi: [
                "#0C0C0C", "#C50F1F", "#13A10E", "#C19C00", "#0037DA", "#881798", "#3A96DD",
                "#CCCCCC", "#767676", "#E74856", "#16C60C", "#F9F1A5", "#3B78FF", "#B4009E",
                "#61D6D6", "#F2F2F2",
            ]
            .map(c),
        }
    }

    /// "One Half Light", a light scheme with a pale yellow.
    fn one_half_light() -> TerminalScheme {
        TerminalScheme {
            name: "One Half Light".to_string(),
            background: c("#FAFAFA"),
            foreground: c("#383A42"),
            cursor: Some(c("#4F525D")),
            selection: Some(c("#FFFFFF")),
            ansi: [
                "#383A42", "#E45649", "#50A14F", "#C18301", "#0184BC", "#A626A4", "#0997B3",
                "#FAFAFA", "#4F525D", "#DF6C75", "#98C379", "#E4C07A", "#61AFEF", "#C577DD",
                "#56B5C1", "#FFFFFF",
            ]
            .map(c),
        }
    }

    #[test]
    fn appearance_comes_from_the_background() {
        assert_eq!(campbell().appearance(), Appearance::Dark);
        assert_eq!(one_half_light().appearance(), Appearance::Light);
    }

    /// Solarized Light: body text below 4.5:1, and grays in the bright slots.
    fn solarized_light() -> TerminalScheme {
        TerminalScheme {
            name: "Solarized Light".to_string(),
            background: c("#FDF6E3"),
            foreground: c("#657B83"),
            cursor: None,
            selection: Some(c("#FFFFFF")),
            ansi: [
                "#002B36", "#DC322F", "#859900", "#B58900", "#268BD2", "#D33682", "#2AA198",
                "#EEE8D5", "#073642", "#CB4B16", "#586E75", "#657B83", "#839496", "#6C71C4",
                "#93A1A1", "#FDF6E3",
            ]
            .map(c),
        }
    }

    #[test]
    fn derived_colors_are_readable() {
        for scheme in [campbell(), one_half_light(), solarized_light()] {
            let colors = scheme.to_theme().resolved_colors();
            let (bg, fg) = (colors.background, colors.foreground);
            let text = fg.contrast(bg);
            let name = &scheme.name;
            assert!(colors.link.contrast(bg) >= LINK_CONTRAST, "{name} link");
            assert!(
                colors.accent.contrast(bg) >= ACCENT_CONTRAST,
                "{name} accent"
            );
            assert!(
                colors.muted.contrast(bg) >= MUTED_CONTRAST.min(text * 0.75),
                "{name} muted"
            );
            assert!(
                colors.muted.contrast(bg) <= text + 0.01,
                "{name} muted is quieter"
            );
            assert!(
                fg.contrast(colors.selection) >= SELECTION_CONTRAST.min(text * 0.8),
                "{name} text on selection"
            );
            assert!(
                colors.selection.contrast(bg) >= SELECTION_VISIBLE,
                "{name} selection shows"
            );
            // Links and accents keep a color of their own.
            assert!(colorfulness(colors.link) > 60, "{name} link is colored");
            assert!(colorfulness(colors.accent) > 60, "{name} accent is colored");
        }
    }

    #[test]
    fn picks_the_colorful_variant_not_a_gray() {
        // Solarized's bright yellow is its body-text gray and its bright
        // blue a gray too; accent and link come from the real colors.
        let colors = solarized_light().to_theme().resolved_colors();
        assert_ne!(colors.accent, c("#657B83"));
        assert_ne!(colors.link, c("#839496"));
    }

    #[test]
    fn link_uses_the_bright_blue_when_the_normal_one_is_too_dark() {
        // Campbell's blue (#0037DA) is unreadable on near-black; lightening
        // it washes it out, so its bright blue (#3B78FF) wins as it is.
        let link = campbell().to_theme().resolved_colors().link;
        assert_eq!(link, c("#3B78FF"));
        assert!(link.contrast(c("#0C0C0C")) >= LINK_CONTRAST);
        // One Half Light's blue passes nearly as is.
        let light = one_half_light().to_theme().resolved_colors().link;
        assert!(light.contrast(c("#FAFAFA")) >= LINK_CONTRAST);
    }

    #[test]
    fn written_theme_loads_back_with_the_same_colors() {
        for scheme in [campbell(), one_half_light()] {
            let text = theme_toml(&scheme, "a test");
            let loaded = ThemeFile::parse(&text).unwrap_or_else(|e| panic!("{e:?}\n{text}"));
            assert_eq!(loaded.meta.name, scheme.name);
            assert_eq!(loaded.meta.appearance, scheme.appearance());
            assert_eq!(
                loaded.resolved_colors(),
                scheme.to_theme().resolved_colors()
            );
            assert_eq!(
                loaded.palette.bright_cyan,
                Some(scheme.ansi[14].to_string())
            );
            assert!(text.starts_with("# Imported by Scrald from a test."));
        }
    }

    #[test]
    fn names_with_quotes_are_escaped() {
        let mut scheme = campbell();
        scheme.name = r#"Bob's "Best" \ Theme"#.to_string();
        let loaded = ThemeFile::parse(&theme_toml(&scheme, "x")).expect("valid TOML");
        assert_eq!(loaded.meta.name, scheme.name);
    }

    #[test]
    fn theme_ids_are_folder_safe() {
        assert_eq!(theme_id("One Half Dark"), "one-half-dark");
        assert_eq!(theme_id("  Tango (Dark)! "), "tango-dark");
        assert_eq!(theme_id("Sólarized"), "s-larized");
        assert_eq!(theme_id("???"), "imported");
    }
}
