//! Editing a theme's `theme.toml` from the theme editor (DESIGN.md §7.2).
//!
//! Each change sets or removes one key, such as `colors.link` or
//! `fonts.heading`. The file is edited in place with `toml_edit`, so
//! everything else in it (comments, key order, spacing, a value's trailing
//! comment) stays as the user wrote it. The result is checked with the same
//! parser that loads themes, so the editor can't save a theme that wouldn't
//! load.

use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, InlineTable, Item, Table, TableLike, Value};

use super::schema::{Appearance, BlockquoteStyle, FontSource, HrStyle, TextSizing, ThemeFile};

/// One edit: set `key` (a dotted path) to `value`, or remove it if `value`
/// is `None`, so a color goes back to being derived.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeChange {
    pub key: String,
    pub value: Option<serde_json::Value>,
}

/// The keys the editor may change. Anything else is refused, so the editor
/// can't be used to write arbitrary TOML.
fn editable(path: &[&str]) -> bool {
    const COLORS: [&str; 11] = [
        "background",
        "foreground",
        "muted",
        "accent",
        "link",
        "selection",
        "border",
        "code_bg",
        "quote_bar",
        "highlight_bg",
        "ui_background",
    ];
    const FONT_SLOTS: [&str; 4] = ["body", "heading", "mono", "ui"];
    const FONT_KEYS: [&str; 5] = ["family", "source", "weights", "italic", "fallback"];
    const LAYOUT: [&str; 7] = [
        "measure",
        "font_size",
        "line_height",
        "paragraph_spacing",
        "text_sizing",
        "min_font_size",
        "max_font_size",
    ];
    match path {
        ["meta", "name" | "appearance"] => true,
        ["colors", key] => COLORS.contains(key),
        ["fonts", slot] => FONT_SLOTS.contains(slot),
        ["fonts", slot, key] => FONT_SLOTS.contains(slot) && FONT_KEYS.contains(key),
        ["layout", key] => LAYOUT.contains(key),
        ["elements", "blockquote", "style" | "italic"] => true,
        ["elements", "hr", "style" | "glyph"] => true,
        ["elements", "headings", "numbering" | "h1_rule"] => true,
        ["elements", "front", "cover" | "title"] => true,
        _ => false,
    }
}

/// `text` (a `theme.toml`) with `changes` applied, or what's wrong: a key
/// the editor can't change, or the problems the edited theme would have.
pub fn apply_changes(text: &str, changes: &[ThemeChange]) -> Result<String, Vec<String>> {
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| vec![format!("theme.toml can't be edited: {e}")])?;
    for change in changes {
        let path: Vec<&str> = change.key.split('.').collect();
        if !editable(&path) {
            return Err(vec![format!(
                "{} can't be changed in the editor",
                change.key
            )]);
        }
        let (last, parents) = path.split_last().expect("split always yields a segment");
        match &change.value {
            None => {
                if let Some(table) = existing_table(&mut doc, parents) {
                    table.remove(last);
                }
            }
            Some(json) => {
                let mut value = toml_value(json)
                    .ok_or_else(|| vec![format!("{}: unsupported value {json}", change.key)])?;
                let table = table_at(&mut doc, parents)
                    .map_err(|e| vec![format!("{}: {e}", change.key)])?;
                // Keep the comment after the old value, if there was one.
                if let Some(old) = table.get(last).and_then(Item::as_value) {
                    *value.decor_mut() = old.decor().clone();
                }
                table.insert(last, Item::Value(value));
            }
        }
    }
    let edited = doc.to_string();
    ThemeFile::parse(&edited)?;
    Ok(edited)
}

/// The table at `path`, if it exists (a `[table]` or an inline `{ }` one).
fn existing_table<'a>(doc: &'a mut DocumentMut, path: &[&str]) -> Option<&'a mut dyn TableLike> {
    let mut current: &mut dyn TableLike = doc.as_table_mut();
    for segment in path {
        current = current.get_mut(segment)?.as_table_like_mut()?;
    }
    Some(current)
}

/// The table at `path`, created if it's missing. Font slots are created as
/// inline tables (`body = { family = "…" }`), as the bundled themes write
/// them; other sections as `[section]` tables.
fn table_at<'a>(doc: &'a mut DocumentMut, path: &[&str]) -> Result<&'a mut dyn TableLike, String> {
    // Rust note: `&mut dyn TableLike` is a mutable reference to "some type
    // with the TableLike methods" (a `[table]` or an inline one); walking
    // down reassigns it to each child in turn.
    let mut current: &mut dyn TableLike = doc.as_table_mut();
    for (depth, segment) in path.iter().enumerate() {
        if current.get(segment).is_none() {
            let item = if depth == 1 && path[0] == "fonts" {
                Item::Value(Value::InlineTable(InlineTable::new()))
            } else {
                let mut table = Table::new();
                // `[elements.hr]` alone, without an empty `[elements]` above.
                table.set_implicit(true);
                Item::Table(table)
            };
            current.insert(segment, item);
        }
        current = current
            .get_mut(segment)
            .and_then(Item::as_table_like_mut)
            .ok_or_else(|| format!("{segment} isn't a table"))?;
    }
    Ok(current)
}

/// A TOML value for the JSON the editor sends: text, numbers, true/false,
/// or a list of those.
fn toml_value(json: &serde_json::Value) -> Option<Value> {
    Some(match json {
        serde_json::Value::String(s) => Value::from(s.as_str()),
        serde_json::Value::Bool(b) => Value::from(*b),
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(i) => Value::from(i),
            None => Value::from(n.as_f64()?),
        },
        serde_json::Value::Array(items) => {
            let mut array = toml_edit::Array::new();
            for item in items {
                array.push(toml_value(item)?);
            }
            Value::Array(array)
        }
        _ => return None,
    })
}

/// A color as the editor shows it: what the file sets (if anything) and
/// what the theme actually uses, derived if the file leaves it out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ColorValue {
    pub key: String,
    pub value: Option<String>,
    pub resolved: String,
}

/// A font slot as the editor shows it; `family` is `None` if the theme
/// leaves the slot to the default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FontValue {
    pub slot: String,
    pub family: Option<String>,
    pub source: FontSource,
    pub weights: Vec<u16>,
    pub italic: bool,
    pub fallback: Vec<String>,
}

/// Everything the theme editor starts from.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorValues {
    pub name: String,
    pub appearance: Appearance,
    pub colors: Vec<ColorValue>,
    pub fonts: Vec<FontValue>,
    pub measure: u32,
    pub font_size: f32,
    pub line_height: f32,
    pub paragraph_spacing: f32,
    pub text_sizing: TextSizing,
    pub blockquote_style: BlockquoteStyle,
    pub blockquote_italic: bool,
    pub hr_style: HrStyle,
    pub hr_glyph: Option<String>,
    pub numbering: bool,
    pub h1_rule: bool,
    pub front_cover: bool,
    pub front_title: bool,
}

impl EditorValues {
    pub fn from_theme(file: &ThemeFile) -> Self {
        let r = file.resolved_colors();
        let c = &file.colors;
        let colors = [
            ("background", &c.background, r.background),
            ("foreground", &c.foreground, r.foreground),
            ("muted", &c.muted, r.muted),
            ("accent", &c.accent, r.accent),
            ("link", &c.link, r.link),
            ("selection", &c.selection, r.selection),
            ("border", &c.border, r.border),
            ("code_bg", &c.code_bg, r.code_bg),
            ("quote_bar", &c.quote_bar, r.quote_bar),
            ("highlight_bg", &c.highlight_bg, r.highlight_bg),
            ("ui_background", &c.ui_background, r.ui_background),
        ]
        .into_iter()
        .map(|(key, value, resolved)| ColorValue {
            key: key.to_string(),
            value: value.clone(),
            resolved: resolved.to_string(),
        })
        .collect();
        let f = &file.fonts;
        let fonts = [
            ("body", &f.body),
            ("heading", &f.heading),
            ("mono", &f.mono),
            ("ui", &f.ui),
        ]
        .into_iter()
        .map(|(slot, spec)| FontValue {
            slot: slot.to_string(),
            family: spec.as_ref().map(|s| s.family.clone()),
            source: spec.as_ref().map(|s| s.source).unwrap_or_default(),
            weights: spec.as_ref().map(|s| s.weights.clone()).unwrap_or_default(),
            italic: spec.as_ref().is_some_and(|s| s.italic),
            fallback: spec
                .as_ref()
                .map(|s| s.fallback.clone())
                .unwrap_or_default(),
        })
        .collect();
        let l = &file.layout;
        let e = &file.elements;
        EditorValues {
            name: file.meta.name.clone(),
            appearance: file.meta.appearance,
            colors,
            fonts,
            measure: l.measure,
            font_size: l.font_size,
            line_height: l.line_height,
            paragraph_spacing: l.paragraph_spacing,
            text_sizing: l.text_sizing,
            blockquote_style: e.blockquote.style,
            blockquote_italic: e.blockquote.italic,
            hr_style: e.hr.style,
            hr_glyph: e.hr.glyph.clone(),
            numbering: e.headings.numbering,
            h1_rule: e.headings.h1_rule,
            front_cover: e.front.cover,
            front_title: e.front.title,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const THEME: &str = r##"# My theme: keep this comment.

[meta]
name = "Mine"
schema = 1
appearance = "dark"

[colors]
background = "#222222"   # the page
foreground = "#dddddd"
link       = "#88aaff"

[fonts]
body = { family = "Literata", source = "google", weights = [400, 700] }
"##;

    fn set(key: &str, value: serde_json::Value) -> ThemeChange {
        ThemeChange {
            key: key.to_string(),
            value: Some(value),
        }
    }

    fn remove(key: &str) -> ThemeChange {
        ThemeChange {
            key: key.to_string(),
            value: None,
        }
    }

    #[test]
    fn a_change_touches_only_its_key() {
        let edited = apply_changes(THEME, &[set("colors.background", json!("#101010"))]).unwrap();
        assert_eq!(
            edited,
            THEME.replace("background = \"#222222\"", "background = \"#101010\"")
        );
    }

    #[test]
    fn new_keys_and_sections_are_added() {
        let edited = apply_changes(
            THEME,
            &[
                set("colors.accent", json!("#ffcc00")),
                set("layout.measure", json!(72)),
                set("layout.line_height", json!(1.7)),
                set("elements.hr.style", json!("ornament")),
                set("elements.hr.glyph", json!("❦")),
                set("fonts.heading.family", json!("Cormorant Garamond")),
                set("fonts.heading.weights", json!([600])),
            ],
        )
        .unwrap();
        let theme = ThemeFile::parse(&edited).unwrap();
        assert_eq!(theme.colors.accent.as_deref(), Some("#ffcc00"));
        assert_eq!(theme.layout.measure, 72);
        assert!((theme.layout.line_height - 1.7).abs() < 1e-6);
        assert_eq!(theme.elements.hr.style, HrStyle::Ornament);
        assert_eq!(theme.fonts.heading.as_ref().unwrap().weights, [600]);
        // Font slots are inline tables; other sections get headers, with no
        // empty [elements] above [elements.hr].
        assert!(edited.contains("heading = { family = \"Cormorant Garamond\", weights = [600] }"));
        assert!(edited.contains("[elements.hr]"));
        assert!(!edited.contains("[elements]\n"));
        assert!(edited.starts_with("# My theme: keep this comment."));
    }

    #[test]
    fn removing_a_color_lets_it_be_derived() {
        let edited =
            apply_changes(THEME, &[remove("colors.link"), remove("colors.muted")]).unwrap();
        assert!(!edited.contains("link"));
        assert!(ThemeFile::parse(&edited).unwrap().colors.link.is_none());
        // A whole font slot can go too.
        let edited = apply_changes(THEME, &[remove("fonts.body")]).unwrap();
        assert!(ThemeFile::parse(&edited).unwrap().fonts.body.is_none());
    }

    #[test]
    fn a_values_trailing_comment_stays() {
        let edited = apply_changes(THEME, &[set("colors.background", json!("#000000"))]).unwrap();
        assert!(edited.contains("background = \"#000000\"   # the page"));
    }

    #[test]
    fn invalid_results_and_other_keys_are_refused() {
        let bad_color = apply_changes(THEME, &[set("colors.link", json!("blue"))]).unwrap_err();
        assert!(bad_color[0].contains("colors.link"), "{bad_color:?}");
        let other = apply_changes(THEME, &[set("css.file", json!("../../x.css"))]).unwrap_err();
        assert_eq!(other, ["css.file can't be changed in the editor"]);
        let nested = apply_changes(THEME, &[set("colors.link.x", json!("1"))]).unwrap_err();
        assert!(nested[0].contains("can't be changed"));
        let object = apply_changes(THEME, &[set("colors.link", json!({"a": 1}))]).unwrap_err();
        assert!(object[0].contains("unsupported value"));
    }

    #[test]
    fn editor_values_show_set_and_derived_colors() {
        let values = EditorValues::from_theme(&ThemeFile::parse(THEME).unwrap());
        let link = values.colors.iter().find(|c| c.key == "link").unwrap();
        assert_eq!(link.value.as_deref(), Some("#88aaff"));
        let muted = values.colors.iter().find(|c| c.key == "muted").unwrap();
        assert_eq!(muted.value, None);
        assert!(muted.resolved.starts_with('#'));
        let body = &values.fonts[0];
        assert_eq!(body.family.as_deref(), Some("Literata"));
        assert_eq!(body.source, FontSource::Google);
        assert_eq!(values.fonts[1].family, None);
        assert_eq!(values.measure, 66);
        // The cover and missing title are on unless a theme says otherwise.
        assert!(values.front_cover && values.front_title);
    }

    #[test]
    fn the_cover_and_title_can_be_turned_off() {
        let edited = apply_changes(THEME, &[set("elements.front.title", json!(false))]).unwrap();
        assert!(edited.contains(
            "[elements.front]
title = false"
        ));
        let theme = ThemeFile::parse(&edited).unwrap();
        assert!(theme.elements.front.cover);
        assert!(!theme.elements.front.title);
    }
}
