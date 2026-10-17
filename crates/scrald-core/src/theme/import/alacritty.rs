//! Alacritty color schemes (TOML, DESIGN.md §7.3).
//!
//! Colors sit under `[colors.primary]` (background, foreground),
//! `[colors.normal]` and `[colors.bright]` (black … white),
//! `[colors.cursor]` and `[colors.selection]`. Values are `#rrggbb` or
//! `0xrrggbb`; the cursor and selection may also be `CellForeground` or
//! `CellBackground`, which aren't colors and are skipped. The older YAML
//! format (`alacritty.yml`) isn't read. The file has no name of its own.

use toml::Value;

use super::{SchemeParts, TerminalScheme, parse_hex};

pub const FORMAT: &str = "an Alacritty color scheme";

const ANSI_KEYS: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

/// Reads the scheme in `text`, named `name`.
pub fn parse(text: &str, name: &str) -> Result<TerminalScheme, String> {
    let value: Value = toml::from_str(text).map_err(|e| format!("not valid TOML: {e}"))?;
    let colors = value
        .get("colors")
        .ok_or_else(|| "not an Alacritty color scheme (no [colors])".to_string())?;
    let color = |table: &str, key: &str| {
        colors
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(Value::as_str)
            .and_then(parse_hex)
    };
    let mut parts = SchemeParts {
        background: color("primary", "background"),
        foreground: color("primary", "foreground"),
        cursor: color("cursor", "cursor"),
        selection: color("selection", "background"),
        ..SchemeParts::default()
    };
    for (i, key) in ANSI_KEYS.iter().enumerate() {
        parts.ansi[i] = color("normal", key);
        parts.ansi[i + 8] = color("bright", key);
    }
    parts.finish(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::color::Color;

    const ONE_DARK: &str = r##"
# Colors (One Dark)
[colors.primary]
background = '#282c34'
foreground = '#abb2bf'

[colors.cursor]
text = 'CellBackground'
cursor = 'CellForeground'

[colors.selection]
text = 'CellForeground'
background = '#3e4451'

[colors.normal]
black   = '#1e2127'
red     = '#e06c75'
green   = '#98c379'
yellow  = '#d19a66'
blue    = '#61afef'
magenta = '#c678dd'
cyan    = '#56b6c2'
white   = '#abb2bf'

[colors.bright]
black   = '0x5c6370'
red     = '0xe06c75'
green   = '0x98c379'
yellow  = '0xd19a66'
blue    = '0x61afef'
magenta = '0xc678dd'
cyan    = '0x56b6c2'
white   = '0xffffff'
"##;

    #[test]
    fn reads_a_scheme() {
        let scheme = parse(ONE_DARK, "one_dark").unwrap();
        assert_eq!(scheme.name, "one_dark");
        assert_eq!(scheme.background, Color::rgb(0x28, 0x2c, 0x34));
        // `CellForeground` isn't a color.
        assert_eq!(scheme.cursor, None);
        assert_eq!(scheme.selection, Some(Color::rgb(0x3e, 0x44, 0x51)));
        assert_eq!(scheme.ansi[4], Color::rgb(0x61, 0xaf, 0xef));
        assert_eq!(scheme.ansi[8], Color::rgb(0x5c, 0x63, 0x70));
    }

    #[test]
    fn reports_problems() {
        assert!(
            parse("[window]\nopacity = 1", "x")
                .unwrap_err()
                .contains("[colors]")
        );
        let no_bright = ONE_DARK.split("[colors.bright]").next().unwrap();
        assert_eq!(
            parse(no_bright, "x").unwrap_err(),
            "\"x\" has no bright black color"
        );
        assert!(
            parse("[colors", "x")
                .unwrap_err()
                .starts_with("not valid TOML")
        );
    }
}
