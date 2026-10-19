//! Contrast checks (DESIGN.md §7.2): every pair of colors the reader and its
//! chrome draw text or controls in, measured against WCAG 2 AA. Text needs
//! 4.5:1; the accent, which also draws focus rings and the progress line,
//! needs 3:1 as a non-text color (and 4.5:1 where it's text, as in tags).

use super::color::Color;
use super::schema::ThemeFile;

/// Text and the background it's drawn on.
pub const TEXT: f32 = 4.5;
/// Focus rings, borders of controls, and other non-text marks.
pub const NON_TEXT: f32 = 3.0;

/// One pair of colors that doesn't reach its minimum contrast.
#[derive(Debug, Clone, PartialEq)]
pub struct ContrastProblem {
    /// What is drawn on what, e.g. "muted text on the sidebar".
    pub what: String,
    pub ratio: f32,
    pub minimum: f32,
}

impl std::fmt::Display for ContrastProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {:.2}:1, needs {}:1",
            self.what, self.ratio, self.minimum
        )
    }
}

/// Every pair in the theme below its minimum, worst first.
pub fn contrast_problems(theme: &ThemeFile) -> Vec<ContrastProblem> {
    let c = theme.resolved_colors();
    let page = c.background;
    // Highlights and tag chips are translucent; measure what's on screen.
    let highlight = c.highlight_bg.over(page);
    let chip = c.accent.with_alpha(0.12).over(page);

    let mut pairs: Vec<(String, Color, Color, f32)> = vec![
        ("text on the page".into(), c.foreground, page, TEXT),
        (
            "text on the sidebar".into(),
            c.foreground,
            c.ui_background,
            TEXT,
        ),
        ("text on code".into(), c.foreground, c.code_bg, TEXT),
        (
            "text on a selection".into(),
            c.foreground,
            c.selection,
            TEXT,
        ),
        ("text on a highlight".into(), c.foreground, highlight, TEXT),
        ("muted text on the page".into(), c.muted, page, TEXT),
        (
            "muted text on the sidebar".into(),
            c.muted,
            c.ui_background,
            TEXT,
        ),
        ("comments in code".into(), c.muted, c.code_bg, TEXT),
        ("links on the page".into(), c.link, page, TEXT),
        ("links on the sidebar".into(), c.link, c.ui_background, TEXT),
        ("accent text in a tag".into(), c.accent, chip, TEXT),
        ("accent on the page".into(), c.accent, page, NON_TEXT),
        (
            "accent on the sidebar".into(),
            c.accent,
            c.ui_background,
            NON_TEXT,
        ),
    ];

    // Syntax colors on code, and the callout titles drawn in them on a 9%
    // tint (ui/src/app.css). Without a palette color, the UI falls back to
    // the accent or the link color.
    let p = &theme.palette;
    let palette = [
        ("green", &p.green, c.accent),
        ("cyan", &p.cyan, c.link),
        ("magenta", &p.magenta, c.accent),
        ("blue", &p.blue, c.link),
        ("yellow", &p.yellow, c.accent),
        ("red", &p.red, c.accent),
    ];
    for (name, value, fallback) in palette {
        let color = value.as_deref().and_then(Color::parse).unwrap_or(fallback);
        pairs.push((format!("{name} code on code"), color, c.code_bg, TEXT));
        let tint = color.with_alpha(0.09).over(page);
        pairs.push((format!("{name} callout title"), color, tint, TEXT));
    }

    let mut problems: Vec<ContrastProblem> = pairs
        .into_iter()
        .filter_map(|(what, fg, bg, minimum)| {
            let ratio = fg.contrast(bg);
            // Rust note: `cond.then_some(x)` is `Some(x)` if `cond` holds,
            // else `None`, which is what `filter_map` keeps or drops.
            (ratio < minimum).then_some(ContrastProblem {
                what,
                ratio,
                minimum,
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
            "muted text on the page: 1.61:1, needs 4.5:1"
        );
        assert!(problems.iter().any(|p| p.what == "text on the page"));
        let shortfalls: Vec<f32> = problems.iter().map(|p| p.ratio / p.minimum).collect();
        assert!(shortfalls.windows(2).all(|w| w[0] <= w[1]), "{problems:?}");
    }
}
