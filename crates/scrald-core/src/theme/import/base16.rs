//! base16 and base24 schemes (YAML, DESIGN.md §7.3).
//!
//! A scheme lists `base00` … `base0F` (base24 adds `base10` … `base17`),
//! either at the top level with a `scheme:` name (the original format) or
//! under `palette:` with a `name:` (the current tinted-theming format).
//! Colors are hex, with or without `#`.
//!
//! base16 has no ANSI colors of its own; terminals map them the way
//! base16-shell does: background `base00`, text `base05`, selection
//! `base02`, red `base08`, yellow `base0A`, green `base0B`, cyan `base0C`,
//! blue `base0D`, magenta `base0E`, and the bright colors the same, except
//! bright black `base03` and bright white `base07`. base24 schemes have
//! their own bright colors in `base12` … `base17`.

use serde_json::Value;

use super::{SchemeParts, TerminalScheme, parse_hex};

pub const FORMAT: &str = "a base16 scheme";

/// The base16 slot for each ANSI color, in `TerminalScheme::ansi` order.
const BASE16_ANSI: [&str; 16] = [
    "base00", "base08", "base0B", "base0A", "base0D", "base0E", "base0C", "base05", "base03",
    "base08", "base0B", "base0A", "base0D", "base0E", "base0C", "base07",
];

/// base24's own bright colors: (ANSI index, slot).
const BASE24_BRIGHT: [(usize, &str); 6] = [
    (9, "base12"),
    (11, "base13"),
    (10, "base14"),
    (14, "base15"),
    (12, "base16"),
    (13, "base17"),
];

/// Reads the scheme in `text`; `name` is used if it doesn't name itself.
pub fn parse(text: &str, name: &str) -> Result<TerminalScheme, String> {
    let value: Value = serde_saphyr::from_str(text).map_err(|e| format!("not valid YAML: {e}"))?;
    // The newer format keeps the colors under `palette:`.
    let palette = value.get("palette").unwrap_or(&value);
    let slot = |key: &str| {
        // Slot names are written in either case (`base0A`, `base0a`).
        let found = palette
            .get(key)
            .or_else(|| palette.get(key.to_ascii_lowercase()));
        found.and_then(Value::as_str).and_then(parse_hex)
    };
    if slot("base00").is_none() {
        return Err("not a base16 scheme (no base00 color)".to_string());
    }
    let mut parts = SchemeParts {
        background: slot("base00"),
        foreground: slot("base05"),
        cursor: slot("base05"),
        selection: slot("base02"),
        ..SchemeParts::default()
    };
    for (i, key) in BASE16_ANSI.iter().enumerate() {
        parts.ansi[i] = slot(key);
    }
    for (i, key) in BASE24_BRIGHT {
        if let Some(color) = slot(key) {
            parts.ansi[i] = Some(color);
        }
    }
    let own_name = ["name", "scheme"]
        .iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str));
    parts.finish(own_name.unwrap_or(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::color::Color;

    const OCEAN: &str = r#"
scheme: "Ocean"
author: "Chris Kempson (http://chriskempson.com)"
base00: "2b303b"
base01: "343d46"
base02: "4f5b66"
base03: "65737e"
base04: "a7adba"
base05: "c0c5ce"
base06: "dfe1e8"
base07: "eff1f5"
base08: "bf616a"
base09: "d08770"
base0A: "ebcb8b"
base0B: "a3be8c"
base0C: "96b5b4"
base0D: "8fa1b3"
base0E: "b48ead"
base0F: "ab7967"
"#;

    #[test]
    fn reads_the_original_format() {
        let scheme = parse(OCEAN, "ocean").unwrap();
        assert_eq!(scheme.name, "Ocean");
        assert_eq!(scheme.background, Color::rgb(0x2b, 0x30, 0x3b));
        assert_eq!(scheme.foreground, Color::rgb(0xc0, 0xc5, 0xce));
        assert_eq!(scheme.selection, Some(Color::rgb(0x4f, 0x5b, 0x66)));
        // Yellow is base0A; bright black base03; bright red repeats red.
        assert_eq!(scheme.ansi[3], Color::rgb(0xeb, 0xcb, 0x8b));
        assert_eq!(scheme.ansi[8], Color::rgb(0x65, 0x73, 0x7e));
        assert_eq!(scheme.ansi[9], scheme.ansi[1]);
    }

    #[test]
    fn reads_the_palette_format_and_base24_brights() {
        let mut text =
            String::from("system: \"base24\"\nname: \"Ocean 24\"\nvariant: \"dark\"\npalette:\n");
        for line in OCEAN.lines().filter(|l| l.starts_with("base")) {
            let (key, value) = line.split_once(": ").unwrap();
            text.push_str(&format!(
                "  {}: \"#{}\"\n",
                key.to_lowercase(),
                value.trim_matches('"')
            ));
        }
        text.push_str("  base12: \"#ff0000\"\n  base16: \"#0000ff\"\n");
        let scheme = parse(&text, "file").unwrap();
        assert_eq!(scheme.name, "Ocean 24");
        assert_eq!(scheme.ansi[3], Color::rgb(0xeb, 0xcb, 0x8b));
        assert_eq!(scheme.ansi[9], Color::rgb(255, 0, 0));
        assert_eq!(scheme.ansi[12], Color::rgb(0, 0, 255));
    }

    #[test]
    fn reports_problems() {
        assert!(parse("name: x\n", "x").unwrap_err().contains("base00"));
        let missing = OCEAN.replace("base0D: \"8fa1b3\"\n", "");
        assert_eq!(
            parse(&missing, "x").unwrap_err(),
            "\"Ocean\" has no blue color"
        );
    }
}
