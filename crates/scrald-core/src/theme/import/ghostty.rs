//! Ghostty themes (DESIGN.md §7.3).
//!
//! A theme is part of Ghostty's config format: `key = value` lines, `#`
//! comments. The colors are `background`, `foreground`, `cursor-color`,
//! `selection-background`, and `palette = N=#rrggbb` for the 16 ANSI colors.
//! Ghostty's theme files are named after the theme, with no extension, so
//! the file name is the theme's name.

use super::{SchemeParts, TerminalScheme, parse_hex};

pub const FORMAT: &str = "a Ghostty theme";

/// Reads the theme in `text`, named `name`.
pub fn parse(text: &str, name: &str) -> Result<TerminalScheme, String> {
    let mut parts = SchemeParts::default();
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        // Rust note: `split_once` returns `Some((before, after))` at the
        // first `=`, or `None` if there isn't one; `let ... else` skips
        // lines without one.
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "background" => parts.background = parse_hex(value),
            "foreground" => parts.foreground = parse_hex(value),
            "cursor-color" => parts.cursor = parse_hex(value),
            "selection-background" => parts.selection = parse_hex(value),
            "palette" => {
                if let Some((index, color)) = value.split_once('=')
                    && let Ok(i) = index.trim().parse::<usize>()
                    && i < 16
                {
                    parts.ansi[i] = parse_hex(color);
                }
            }
            _ => {}
        }
    }
    parts.finish(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::color::Color;

    const TOKYO: &str = "\
# Tokyo Night
palette = 0=#15161e
palette = 1=#f7768e
palette = 2=#9ece6a
palette = 3=#e0af68
palette = 4=#7aa2f7
palette = 5=#bb9af7
palette = 6=#7dcfff
palette = 7=#a9b1d6
palette = 8=#414868
palette = 9=#f7768e
palette = 10=#9ece6a
palette = 11=#e0af68
palette = 12=#7aa2f7
palette = 13=#bb9af7
palette = 14=#7dcfff
palette = 15=#c0caf5
background = #1a1b26
foreground = c0caf5
cursor-color = #c0caf5
selection-background = #33467c
selection-foreground = #c0caf5
";

    #[test]
    fn reads_a_theme() {
        let scheme = parse(TOKYO, "TokyoNight").unwrap();
        assert_eq!(scheme.name, "TokyoNight");
        assert_eq!(scheme.background, Color::rgb(0x1a, 0x1b, 0x26));
        // Hex without `#` works too.
        assert_eq!(scheme.foreground, Color::rgb(0xc0, 0xca, 0xf5));
        assert_eq!(scheme.selection, Some(Color::rgb(0x33, 0x46, 0x7c)));
        assert_eq!(scheme.ansi[12], Color::rgb(0x7a, 0xa2, 0xf7));
    }

    #[test]
    fn reports_a_missing_color() {
        let text = TOKYO.replace("palette = 13=#bb9af7\n", "");
        assert_eq!(
            parse(&text, "TokyoNight").unwrap_err(),
            "\"TokyoNight\" has no bright magenta color"
        );
    }
}
