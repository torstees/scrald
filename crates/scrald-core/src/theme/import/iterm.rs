//! iTerm2 color presets (`.itermcolors`, DESIGN.md §7.3).
//!
//! The file is an XML property list: a dictionary from names such as
//! `Ansi 4 Color` or `Background Color` to dictionaries of `Red Component`,
//! `Green Component`, and `Blue Component`, each a number from 0 to 1. The
//! nesting is always that shallow, so a small scanner reads it without a
//! plist library. The file has no name of its own; the file name is used.

use super::{SchemeParts, TerminalScheme};
use crate::theme::color::Color;

pub const FORMAT: &str = "an iTerm2 color preset";

/// Reads the preset in `text`, named `name`.
pub fn parse(text: &str, name: &str) -> Result<TerminalScheme, String> {
    if !text.contains("<plist") {
        return Err("not an iTerm2 color preset (no <plist>)".to_string());
    }
    let mut parts = SchemeParts::default();
    for (key, body) in color_entries(text) {
        let Some(color) = color_from_dict(body) else {
            continue;
        };
        match key {
            "Background Color" => parts.background = Some(color),
            "Foreground Color" => parts.foreground = Some(color),
            "Cursor Color" => parts.cursor = Some(color),
            "Selection Color" => parts.selection = Some(color),
            _ => {
                let index = key
                    .strip_prefix("Ansi ")
                    .and_then(|rest| rest.strip_suffix(" Color"))
                    .and_then(|n| n.parse::<usize>().ok());
                if let Some(i) = index.filter(|&i| i < 16) {
                    parts.ansi[i] = Some(color);
                }
            }
        }
    }
    parts.finish(name)
}

/// Every `<key>NAME</key>` followed by a `<dict>…</dict>`, as (NAME, the
/// dictionary's inside). Color dictionaries don't nest, so the first
/// `</dict>` closes each.
fn color_entries(text: &str) -> Vec<(&str, &str)> {
    let mut entries = Vec::new();
    let mut rest = text;
    while let Some((key, after_key)) = next_element(rest, "key") {
        rest = after_key;
        let trimmed = after_key.trim_start();
        if let Some(inside) = trimmed.strip_prefix("<dict>")
            && let Some(end) = inside.find("</dict>")
        {
            entries.push((key.trim(), &inside[..end]));
            rest = &inside[end + "</dict>".len()..];
        }
    }
    entries
}

/// The text of the next `<tag>…</tag>` in `text`, and what follows it.
// Rust note: `'a` names a lifetime: both returned slices borrow from `text`
// (not from `tag`). With two reference inputs the compiler can't guess which
// one the output points into, so it has to be written out.
fn next_element<'a>(text: &'a str, tag: &str) -> Option<(&'a str, &'a str)> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = start + text[start..].find(&close)?;
    Some((&text[start..end], &text[end + close.len()..]))
}

/// A color from a dictionary's `Red Component` etc. (0 to 1). Alpha, if
/// present, is ignored; color spaces are all treated as sRGB.
fn color_from_dict(body: &str) -> Option<Color> {
    let mut red = None;
    let mut green = None;
    let mut blue = None;
    let mut rest = body;
    while let Some((key, after)) = next_element(rest, "key") {
        // The value is the next element: <real>, or <integer> for 0 and 1.
        let value = next_element(after, "real")
            .or_else(|| next_element(after, "integer"))
            .filter(|(_, tail)| after.len() - tail.len() < 200);
        let component = value.and_then(|(v, _)| v.trim().parse::<f32>().ok());
        match key.trim() {
            "Red Component" => red = component,
            "Green Component" => green = component,
            "Blue Component" => blue = component,
            _ => {}
        }
        rest = after;
    }
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(Color::rgb(byte(red?), byte(green?), byte(blue?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One color entry, as iTerm2 writes it.
    fn entry(name: &str, r: f32, g: f32, b: f32) -> String {
        format!(
            "\t<key>{name}</key>\n\t<dict>\n\t\t<key>Alpha Component</key>\n\t\t<real>1</real>\n\
             \t\t<key>Blue Component</key>\n\t\t<real>{b}</real>\n\
             \t\t<key>Color Space</key>\n\t\t<string>sRGB</string>\n\
             \t\t<key>Green Component</key>\n\t\t<real>{g}</real>\n\
             \t\t<key>Red Component</key>\n\t\t<real>{r}</real>\n\t</dict>\n"
        )
    }

    fn preset(skip: Option<&str>) -> String {
        let mut body = String::new();
        for i in 0..16 {
            let name = format!("Ansi {i} Color");
            if Some(name.as_str()) != skip {
                body.push_str(&entry(&name, i as f32 / 15.0, 0.5, 0.0));
            }
        }
        body.push_str(&entry("Background Color", 0.156, 0.165, 0.211));
        body.push_str(&entry("Foreground Color", 0.972, 0.972, 0.949));
        body.push_str(&entry("Selection Color", 0.266, 0.278, 0.352));
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n{body}</dict>\n</plist>\n"
        )
    }

    #[test]
    fn reads_a_preset() {
        let scheme = parse(&preset(None), "Dracula").unwrap();
        assert_eq!(scheme.name, "Dracula");
        assert_eq!(scheme.background, Color::rgb(40, 42, 54));
        assert_eq!(scheme.foreground, Color::rgb(248, 248, 242));
        assert_eq!(scheme.selection, Some(Color::rgb(68, 71, 90)));
        assert_eq!(scheme.cursor, None);
        assert_eq!(scheme.ansi[0], Color::rgb(0, 128, 0));
        assert_eq!(scheme.ansi[15], Color::rgb(255, 128, 0));
    }

    #[test]
    fn integers_and_exponents_are_numbers() {
        let text = preset(None)
            .replace("<real>0.5</real>", "<real>5e-1</real>")
            .replace("<real>1</real>", "<integer>1</integer>");
        let scheme = parse(&text, "X").unwrap();
        assert_eq!(scheme.ansi[3].g, 128);
        assert_eq!(scheme.ansi[15].r, 255);
    }

    #[test]
    fn reports_a_missing_color() {
        let err = parse(&preset(Some("Ansi 9 Color")), "Dracula").unwrap_err();
        assert_eq!(err, "\"Dracula\" has no bright red color");
        assert!(parse("{}", "X").unwrap_err().contains("<plist>"));
    }
}
