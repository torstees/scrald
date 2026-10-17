//! Windows Terminal color schemes (DESIGN.md §7.3).
//!
//! Accepts a whole `settings.json` (its `schemes` list), a single scheme
//! object (what the Terminal's settings and sites like
//! windowsterminalthemes.dev copy), or a list of schemes. `settings.json`
//! allows comments and trailing commas, which are removed before parsing.

use serde_json::{Map, Value};

use super::TerminalScheme;
use crate::theme::color::Color;

/// What `theme_toml`'s comment calls this format.
pub const FORMAT: &str = "a Windows Terminal color scheme";

/// The scheme keys for the 16 ANSI colors, in `TerminalScheme::ansi` order.
/// Windows Terminal calls magenta "purple".
const ANSI_KEYS: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "purple",
    "cyan",
    "white",
    "brightBlack",
    "brightRed",
    "brightGreen",
    "brightYellow",
    "brightBlue",
    "brightPurple",
    "brightCyan",
    "brightWhite",
];

/// Reads every color scheme in `text`. Fails if the text isn't JSON, holds
/// no schemes, or a scheme is missing a color or has an invalid one.
pub fn parse(text: &str) -> Result<Vec<TerminalScheme>, String> {
    let json = strip_jsonc(text.trim_start_matches('\u{feff}'));
    let value: Value = serde_json::from_str(&json).map_err(|e| format!("not valid JSON: {e}"))?;
    let objects: Vec<&Map<String, Value>> = match &value {
        Value::Object(map) => match map.get("schemes") {
            Some(Value::Array(list)) => list.iter().filter_map(Value::as_object).collect(),
            // A single scheme, if it looks like one (any other JSON file
            // would only produce "has no background color").
            _ if map.contains_key("background") => vec![map],
            _ => Vec::new(),
        },
        Value::Array(list) => list.iter().filter_map(Value::as_object).collect(),
        _ => Vec::new(),
    };
    if objects.is_empty() {
        return Err("no Windows Terminal color schemes found".to_string());
    }
    objects
        .into_iter()
        .enumerate()
        .map(|(i, object)| scheme(object).map_err(|e| format!("scheme {}: {e}", i + 1)))
        .collect()
}

fn scheme(object: &Map<String, Value>) -> Result<TerminalScheme, String> {
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("Imported theme")
        .to_string();
    let color = |key: &str| -> Result<Color, String> {
        match object.get(key) {
            Some(Value::String(text)) => Color::parse(text)
                .ok_or_else(|| format!("\"{name}\": {key} \"{text}\" is not a color")),
            _ => Err(format!("\"{name}\" has no {key} color")),
        }
    };
    let optional = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_str)
            .and_then(Color::parse)
    };
    let mut ansi = [Color::rgb(0, 0, 0); 16];
    // Rust note: `iter_mut().zip(...)` walks two sequences side by side,
    // with a mutable reference into the array, like Python's `zip`.
    for (slot, key) in ansi.iter_mut().zip(ANSI_KEYS) {
        *slot = color(key)?;
    }
    Ok(TerminalScheme {
        background: color("background")?,
        foreground: color("foreground")?,
        cursor: optional("cursorColor"),
        selection: optional("selectionBackground"),
        ansi,
        name,
    })
}

/// Removes `//` and `/* */` comments and trailing commas (before `}` or
/// `]`), leaving strings alone, so JSON with comments parses as JSON.
fn strip_jsonc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // Rust note: `peekable()` wraps an iterator so `peek()` can look at the
    // next item without taking it; `by_ref()` below lets an inner `for` loop
    // consume from the same iterator and hand it back when it breaks.
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                    }
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                // Up to the end of the line; the line break stays.
                while chars.peek().is_some_and(|&n| n != '\n') {
                    chars.next();
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = ' ';
                for n in chars.by_ref() {
                    if previous == '*' && n == '/' {
                        break;
                    }
                    previous = n;
                }
                out.push(' ');
            }
            '}' | ']' => {
                // Drop a comma that only whitespace separates from here.
                let kept = out.trim_end().len();
                if out[..kept].ends_with(',') {
                    out.truncate(kept - 1);
                }
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAMPBELL: &str = r##"{
        "name": "Campbell",
        "background": "#0C0C0C",
        "foreground": "#CCCCCC",
        "cursorColor": "#FFFFFF",
        "selectionBackground": "#FFFFFF",
        "black": "#0C0C0C", "red": "#C50F1F", "green": "#13A10E", "yellow": "#C19C00",
        "blue": "#0037DA", "purple": "#881798", "cyan": "#3A96DD", "white": "#CCCCCC",
        "brightBlack": "#767676", "brightRed": "#E74856", "brightGreen": "#16C60C",
        "brightYellow": "#F9F1A5", "brightBlue": "#3B78FF", "brightPurple": "#B4009E",
        "brightCyan": "#61D6D6", "brightWhite": "#F2F2F2"
    }"##;

    #[test]
    fn reads_a_single_scheme() {
        let schemes = parse(CAMPBELL).expect("parses");
        assert_eq!(schemes.len(), 1);
        let s = &schemes[0];
        assert_eq!(s.name, "Campbell");
        assert_eq!(s.background, Color::rgb(0x0c, 0x0c, 0x0c));
        assert_eq!(s.cursor, Some(Color::rgb(255, 255, 255)));
        // "purple" is magenta, at index 5; bright white is last.
        assert_eq!(s.ansi[5], Color::rgb(0x88, 0x17, 0x98));
        assert_eq!(s.ansi[15], Color::rgb(0xf2, 0xf2, 0xf2));
    }

    #[test]
    fn reads_the_schemes_in_a_settings_file_with_comments() {
        let settings = format!(
            r#"// This file was initially generated by Windows Terminal
            {{
                "$schema": "https://aka.ms/terminal-profiles-schema",
                "defaultProfile": "{{61c54bbd}}", // a comment after a value
                /* a block
                   comment */
                "profiles": {{ "list": [ {{ "name": "PowerShell", }}, ] }},
                "schemes": [
                    {CAMPBELL},
                    {renamed},
                ],
                "url": "https://example.com/a//b"
            }}"#,
            renamed = CAMPBELL.replace("Campbell", "Campbell Copy"),
        );
        let schemes = parse(&settings).expect("parses");
        let names: Vec<&str> = schemes.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Campbell", "Campbell Copy"]);
    }

    #[test]
    fn reads_a_list_of_schemes_with_a_bom() {
        let list = format!("\u{feff}[{CAMPBELL}]");
        assert_eq!(parse(&list).expect("parses").len(), 1);
    }

    #[test]
    fn reports_what_is_wrong() {
        let missing = CAMPBELL.replace(r##""brightCyan": "#61D6D6","##, "");
        assert_eq!(
            parse(&missing).unwrap_err(),
            "scheme 1: \"Campbell\" has no brightCyan color"
        );
        let bad = CAMPBELL.replace("#3A96DD", "teal");
        assert_eq!(
            parse(&bad).unwrap_err(),
            "scheme 1: \"Campbell\": cyan \"teal\" is not a color"
        );
        assert!(parse("not json").unwrap_err().starts_with("not valid JSON"));
        for empty in [
            r#"{"schemes": []}"#,
            r#"{"name": "x", "version": 1}"#,
            "[]",
            "3",
        ] {
            assert_eq!(
                parse(empty).unwrap_err(),
                "no Windows Terminal color schemes found",
                "{empty}"
            );
        }
    }

    #[test]
    fn reads_the_fixture_files() {
        // The files the PR's manual tests import.
        let settings =
            include_str!("../../../tests/fixtures/terminal/windows-terminal-settings.json");
        let names: Vec<String> = parse(settings)
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, ["Campbell", "Solarized Light", "Tango Dark"]);
        let single = include_str!("../../../tests/fixtures/terminal/tango-dark.json");
        assert_eq!(parse(single).unwrap()[0].name, "Tango Dark");
    }

    #[test]
    fn comment_markers_inside_strings_are_kept() {
        assert_eq!(
            strip_jsonc(r#"{"a": "x // y /* z */", "b": [1, 2,],}"#),
            r#"{"a": "x // y /* z */", "b": [1, 2]}"#
        );
        assert_eq!(
            strip_jsonc(r#"{"q": "say \"hi\" // ok"}"#),
            r#"{"q": "say \"hi\" // ok"}"#
        );
    }
}
