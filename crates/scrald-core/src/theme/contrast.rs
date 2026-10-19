//! Contrast checks (DESIGN.md §7.2): every pair of colors the reader and its
//! chrome draw text or controls in, measured against WCAG 2 AA. Text needs
//! 4.5:1; the accent, which also draws focus rings and the progress line,
//! needs 3:1 as a non-text color (and 4.5:1 where it's text, as in tags).

use serde::Serialize;

use super::color::Color;
use super::schema::ThemeFile;

/// Text and the background it's drawn on.
pub const TEXT: f32 = 4.5;
/// Focus rings, borders of controls, and other non-text marks.
pub const NON_TEXT: f32 = 3.0;

/// One pair of colors that doesn't reach its minimum contrast.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContrastProblem {
    /// What is drawn on what, e.g. "muted text on the sidebar".
    pub what: String,
    pub ratio: f32,
    pub minimum: f32,
    /// The theme colors involved, as the editor names them: `colors` keys
    /// such as `"muted"` and `"ui_background"`, or `"palette.green"`.
    pub colors: Vec<String>,
    /// The two colors as drawn, opaque, for a sample of the problem.
    pub fg: String,
    pub bg: String,
}

impl std::fmt::Display for ContrastProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Cut to two decimals rather than rounded, so 4.496 never reads as
        // the 4.50 it fails to reach.
        let ratio = (self.ratio * 100.0).floor() / 100.0;
        write!(f, "{}: {ratio:.2}:1, needs {}:1", self.what, self.minimum)
    }
}

/// One pair to measure: a description, the color drawn and the color it's
/// drawn on (each with the theme key it comes from), and the minimum.
struct Pair {
    what: String,
    fg: (Color, String),
    bg: (Color, String),
    minimum: f32,
}

fn pair(what: &str, fg: (Color, &str), bg: (Color, &str), minimum: f32) -> Pair {
    Pair {
        what: what.to_string(),
        fg: (fg.0, fg.1.to_string()),
        bg: (bg.0, bg.1.to_string()),
        minimum,
    }
}

/// Every pair in the theme below its minimum, worst first.
pub fn contrast_problems(theme: &ThemeFile) -> Vec<ContrastProblem> {
    let c = theme.resolved_colors();
    let text = (c.foreground, "foreground");
    let muted = (c.muted, "muted");
    let link = (c.link, "link");
    let accent = (c.accent, "accent");
    let page = (c.background, "background");
    let sidebar = (c.ui_background, "ui_background");
    let code = (c.code_bg, "code_bg");
    // Highlights and tag chips are translucent; measure what's on screen.
    let highlight = (c.highlight_bg.over(c.background), "highlight_bg");
    let chip = (c.accent.with_alpha(0.12).over(c.background), "background");

    let mut pairs = vec![
        pair("text on the page", text, page, TEXT),
        pair("text on the sidebar", text, sidebar, TEXT),
        pair("text on code", text, code, TEXT),
        pair(
            "text on a selection",
            text,
            (c.selection, "selection"),
            TEXT,
        ),
        pair("text on a highlight", text, highlight, TEXT),
        pair("muted text on the page", muted, page, TEXT),
        pair("muted text on the sidebar", muted, sidebar, TEXT),
        pair("comments in code", muted, code, TEXT),
        pair("links on the page", link, page, TEXT),
        pair("links on the sidebar", link, sidebar, TEXT),
        pair("accent text in a tag", accent, chip, TEXT),
        pair("accent on the page", accent, page, NON_TEXT),
        pair("accent on the sidebar", accent, sidebar, NON_TEXT),
    ];

    // Syntax colors on code, and the callout titles drawn in them on a 9%
    // tint (ui/src/app.css). Without a palette color, the UI falls back to
    // the accent or the link color.
    let p = &theme.palette;
    let palette = [
        ("green", &p.green, accent),
        ("cyan", &p.cyan, link),
        ("magenta", &p.magenta, accent),
        ("blue", &p.blue, link),
        ("yellow", &p.yellow, accent),
        ("red", &p.red, accent),
    ];
    for (name, value, fallback) in palette {
        let key = format!("palette.{name}");
        let color = match value.as_deref().and_then(Color::parse) {
            Some(color) => (color, key.as_str()),
            None => fallback,
        };
        pairs.push(pair(&format!("{name} code on code"), color, code, TEXT));
        let tint = (color.0.with_alpha(0.09).over(c.background), "background");
        pairs.push(pair(&format!("{name} callout title"), color, tint, TEXT));
    }

    let mut problems: Vec<ContrastProblem> = pairs
        .into_iter()
        .filter_map(|pair| {
            let ratio = pair.fg.0.contrast(pair.bg.0);
            // Rust note: `cond.then_some(x)` is `Some(x)` if `cond` holds,
            // else `None`, which is what `filter_map` keeps or drops.
            (ratio < pair.minimum).then_some(ContrastProblem {
                what: pair.what,
                ratio,
                minimum: pair.minimum,
                colors: vec![pair.fg.1, pair.bg.1],
                fg: pair.fg.0.to_string(),
                bg: pair.bg.0.to_string(),
            })
        })
        .collect();
    // Rust note: `f32` has no total order (NaN), so `sort_by` with
    // `total_cmp` is used instead of `sort`.
    problems.sort_by(|a, b| (a.ratio / a.minimum).total_cmp(&(b.ratio / b.minimum)));
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_themes_meet_wcag_aa() {
        let mut failures = Vec::new();
        for (id, text) in super::super::BUNDLED {
            let theme = ThemeFile::parse(text).expect("bundled themes parse");
            for problem in contrast_problems(&theme) {
                failures.push(format!("{id}: {problem}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    #[test]
    fn low_contrast_is_reported_worst_first() {
        let theme = ThemeFile::parse(
            "[meta]\nname = \"Gray\"\nschema = 1\nappearance = \"light\"\n\
             [colors]\nbackground = \"#ffffff\"\nforeground = \"#777777\"\nmuted = \"#cccccc\"\n",
        )
        .unwrap();
        let problems = contrast_problems(&theme);
        let muted = problems
            .iter()
            .find(|p| p.what == "muted text on the page")
            .unwrap();
        assert_eq!(
            muted.to_string(),
            "muted text on the page: 1.60:1, needs 4.5:1"
        );
        assert_eq!(muted.colors, ["muted", "background"]);
        assert_eq!(
            (muted.fg.as_str(), muted.bg.as_str()),
            ("#cccccc", "#ffffff")
        );
        assert!(problems.iter().any(|p| p.what == "text on the page"));
        let shortfalls: Vec<f32> = problems.iter().map(|p| p.ratio / p.minimum).collect();
        assert!(shortfalls.windows(2).all(|w| w[0] <= w[1]), "{problems:?}");
    }

    #[test]
    fn ratios_just_short_of_the_minimum_never_print_as_passing() {
        let problem = ContrastProblem {
            what: "text on the page".into(),
            ratio: 4.497,
            minimum: TEXT,
            colors: vec![],
            fg: String::new(),
            bg: String::new(),
        };
        assert_eq!(problem.to_string(), "text on the page: 4.49:1, needs 4.5:1");
    }

    #[test]
    fn palette_colors_name_their_key_or_the_fallback() {
        let theme = ThemeFile::parse(
            "[meta]
name = \"Pale\"
schema = 1
appearance = \"light\"
             [colors]
background = \"#ffffff\"
accent = \"#dddddd\"
             [palette]
yellow = \"#ffff00\"
",
        )
        .unwrap();
        let problems = contrast_problems(&theme);
        let find = |what: &str| problems.iter().find(|p| p.what == what).unwrap();
        assert_eq!(
            find("yellow code on code").colors,
            ["palette.yellow", "code_bg"]
        );
        // No green in the palette: green code is drawn in the accent.
        assert_eq!(find("green code on code").colors, ["accent", "code_bg"]);
        assert_eq!(
            find("accent text in a tag").colors,
            ["accent", "background"]
        );
    }
}
