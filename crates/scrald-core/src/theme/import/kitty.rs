//! kitty themes (`.conf`, DESIGN.md §7.3).
//!
//! `key value` lines with `#` comments: `background`, `foreground`,
//! `cursor`, `selection_background`, and `color0` … `color15`. Themes from
//! the kitty-themes collection name themselves in a `## name: …` comment;
//! otherwise the file name is used.

use super::{SchemeParts, TerminalScheme, parse_hex};

pub const FORMAT: &str = "a kitty theme";

/// Reads the theme in `text`; `name` is used if the file doesn't name itself.
pub fn parse(text: &str, name: &str) -> Result<TerminalScheme, String> {
    let mut parts = SchemeParts::default();
    let mut own_name = None;
    for line in text.lines().map(str::trim) {
        if let Some(comment) = line.strip_prefix('#') {
            if let Some(n) = comment.trim_start_matches('#').trim().strip_prefix("name:") {
                own_name = Some(n.trim().to_string());
            }
            continue;
        }
        let mut words = line.split_whitespace();
        let (Some(key), Some(value)) = (words.next(), words.next()) else {
            continue;
        };
        match key {
            "background" => parts.background = parse_hex(value),
            "foreground" => parts.foreground = parse_hex(value),
            "cursor" => parts.cursor = parse_hex(value),
            "selection_background" => parts.selection = parse_hex(value),
            _ => {
                if let Some(i) = key
                    .strip_prefix("color")
                    .and_then(|n| n.parse::<usize>().ok())
                    .filter(|&i| i < 16)
                {
                    parts.ansi[i] = parse_hex(value);
                }
            }
        }
    }
    parts.finish(own_name.as_deref().unwrap_or(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::color::Color;

    const GRUVBOX: &str = "\
# vim:ft=kitty

## name: Gruvbox Dark
## author: Pavel Pertsev

background  #282828
foreground  #ebdbb2
cursor      #928374
selection_foreground #928374
selection_background #3c3836

# black
color0  #282828
color8  #928374
color1  #cc241d
color9  #fb4934
color2  #98971a
color10 #b8bb26
color3  #d79921
color11 #fabd2d
color4  #458588
color12 #83a598
color5  #b16286
color13 #d3869b
color6  #689d6a
color14 #8ec07c
color7  #a89984
color15 #ebdbb2
";

    #[test]
    fn reads_a_theme_with_its_own_name() {
        let scheme = parse(GRUVBOX, "gruvbox_dark").unwrap();
        assert_eq!(scheme.name, "Gruvbox Dark");
        assert_eq!(scheme.background, Color::rgb(0x28, 0x28, 0x28));
        assert_eq!(scheme.cursor, Some(Color::rgb(0x92, 0x83, 0x74)));
        assert_eq!(scheme.ansi[11], Color::rgb(0xfa, 0xbd, 0x2d));
    }

    #[test]
    fn falls_back_to_the_file_name() {
        let text = GRUVBOX.replace("## name: Gruvbox Dark\n", "");
        assert_eq!(parse(&text, "gruvbox_dark").unwrap().name, "gruvbox_dark");
        let missing = GRUVBOX.replace("color4  #458588\n", "");
        assert_eq!(
            parse(&missing, "x").unwrap_err(),
            "\"Gruvbox Dark\" has no blue color"
        );
    }
}
