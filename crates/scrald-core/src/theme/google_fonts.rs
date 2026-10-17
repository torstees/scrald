//! Google Fonts for themes (DESIGN.md §7.5): the pure half. Building the
//! request for a theme's family, reading the `@font-face` rules Google sends
//! back, and writing rules that point at the downloaded copies. The app does
//! the downloading and caching.
//!
//! Everything Google's CSS puts into Scrald's CSS is checked first: only
//! `https://fonts.gstatic.com/` woff2 files are fetched, and styles, weights,
//! and unicode ranges must look like what they are.

use serde::{Deserialize, Serialize};

use super::schema::FontSpec;

/// Weights fetched when a theme doesn't list any: regular and bold.
const DEFAULT_WEIGHTS: [u16; 2] = [400, 700];

/// Where font files may come from.
const FONT_HOST: &str = "https://fonts.gstatic.com/";

/// Whether `family` can be asked for: letters, digits, and spaces (every
/// Google Fonts family name), so it can't change the request.
pub fn valid_family(family: &str) -> bool {
    let family = family.trim();
    !family.is_empty()
        && family.len() <= 64
        && family
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ')
}

/// The part of the request after the family name that picks its styles,
/// e.g. `ital,wght@0,400;0,700;1,400;1,700`. Also the cache's key for which
/// styles were downloaded.
pub fn variant(spec: &FontSpec) -> String {
    let mut weights: Vec<u16> = spec
        .weights
        .iter()
        .copied()
        .filter(|w| (1..=1000).contains(w))
        .collect();
    if weights.is_empty() {
        weights = DEFAULT_WEIGHTS.to_vec();
    }
    weights.sort_unstable();
    weights.dedup();
    if spec.italic {
        let mut axes: Vec<String> = weights.iter().map(|w| format!("0,{w}")).collect();
        axes.extend(weights.iter().map(|w| format!("1,{w}")));
        format!("ital,wght@{}", axes.join(";"))
    } else {
        let list: Vec<String> = weights.iter().map(u16::to_string).collect();
        format!("wght@{}", list.join(";"))
    }
}

/// The Google Fonts CSS request for `family` with `variant` (from `variant`),
/// or for the family's default style when `variant` is `None`. Google refuses
/// the whole request if any listed weight doesn't exist, so the app falls
/// back to the default style then.
pub fn css_url(family: &str, variant: Option<&str>) -> Option<String> {
    if !valid_family(family) {
        return None;
    }
    let name = family.trim().replace(' ', "+");
    Some(match variant {
        Some(v) => format!("https://fonts.googleapis.com/css2?family={name}:{v}&display=swap"),
        None => format!("https://fonts.googleapis.com/css2?family={name}&display=swap"),
    })
}

/// One `@font-face` rule from Google's CSS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFace {
    pub style: String,
    /// A weight (`400`) or a range for variable fonts (`100 900`).
    pub weight: String,
    pub unicode_range: Option<String>,
    pub url: String,
}

/// The `@font-face` rules in Google's CSS that are safe to use: a woff2 file
/// on fonts.gstatic.com and well-formed values. Others are skipped.
pub fn parse_css(css: &str) -> Vec<RemoteFace> {
    let mut faces = Vec::new();
    for block in css.split("@font-face").skip(1) {
        let Some(body) = block
            .split_once('{')
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(body, _)| body)
        else {
            continue;
        };
        let mut style = None;
        let mut weight = None;
        let mut unicode_range = None;
        let mut url = None;
        for declaration in body.split(';') {
            let Some((property, value)) = declaration.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match property.trim() {
                "font-style" => style = Some(value.to_string()),
                "font-weight" => weight = Some(value.to_string()),
                "unicode-range" => unicode_range = Some(value.to_string()),
                "src" => url = woff2_url(value),
                _ => {}
            }
        }
        let style = style.unwrap_or_else(|| "normal".to_string());
        let weight = weight.unwrap_or_else(|| "400".to_string());
        let (Some(url), true, true, true) = (
            url,
            matches!(style.as_str(), "normal" | "italic"),
            valid_weight(&weight),
            unicode_range.as_deref().is_none_or(valid_unicode_range),
        ) else {
            continue;
        };
        faces.push(RemoteFace {
            style,
            weight,
            unicode_range,
            url,
        });
    }
    faces
}

/// The URL in `url(…) format('woff2')`, if it's a font file on Google's
/// font host with nothing in it that could break out of a CSS string.
fn woff2_url(src: &str) -> Option<String> {
    if !src.contains("woff2") {
        return None;
    }
    let inside = src.strip_prefix("url(")?.split_once(')')?.0;
    let url = inside.trim().trim_matches(|c| c == '"' || c == '\'');
    let safe = url.starts_with(FONT_HOST)
        && url
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/:._-~%".contains(c));
    safe.then(|| url.to_string())
}

/// `400`, or `100 900` for a variable font.
fn valid_weight(weight: &str) -> bool {
    let parts: Vec<&str> = weight.split_whitespace().collect();
    (1..=2).contains(&parts.len())
        && parts
            .iter()
            .all(|p| p.parse::<u16>().is_ok_and(|w| (1..=1000).contains(&w)))
}

/// `U+0000-00FF, U+0131, U+02??` and the like.
fn valid_unicode_range(range: &str) -> bool {
    range.split(',').all(|part| {
        part.trim().strip_prefix("U+").is_some_and(|rest| {
            !rest.is_empty()
                && rest
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() || c == '-' || c == '?')
        })
    })
}

/// A downloaded face, as the app's cache records it: `file` is the name of
/// the woff2 file in the family's cache folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedFace {
    pub style: String,
    pub weight: String,
    pub unicode_range: Option<String>,
    pub file: String,
}

/// `@font-face` rules for `family` using downloaded files, which are served
/// at `base_url` + file name (`base_url` ends in `/`). Values are checked
/// again, since they come back from the cache.
pub fn font_face_css(family: &str, faces: &[CachedFace], base_url: &str) -> String {
    if !valid_family(family) {
        return String::new();
    }
    let mut css = String::new();
    for face in faces {
        let file_ok = face
            .file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        if !file_ok
            || !matches!(face.style.as_str(), "normal" | "italic")
            || !valid_weight(&face.weight)
            || !face
                .unicode_range
                .as_deref()
                .is_none_or(valid_unicode_range)
        {
            continue;
        }
        css.push_str(&format!(
            "@font-face {{ font-family: \"{}\"; font-style: {}; font-weight: {}; font-display: swap; src: url(\"{base_url}{}\") format(\"woff2\");",
            family.trim(),
            face.style,
            face.weight,
            face.file
        ));
        if let Some(range) = &face.unicode_range {
            css.push_str(&format!(" unicode-range: {range};"));
        }
        css.push_str(" }\n");
    }
    css
}

/// The cache folder name for `family` and `variant`: the family in lowercase
/// with hyphens, plus a short hash of the variant, so different weight sets
/// don't share a folder.
pub fn cache_folder(family: &str, variant: &str) -> String {
    let slug: String = family
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let hash = crate::hash::fnv1a(variant.as_bytes()) as u32;
    format!("{slug}-{hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::schema::FontSource;

    fn spec(family: &str, weights: &[u16], italic: bool) -> FontSpec {
        FontSpec {
            family: family.to_string(),
            source: FontSource::Google,
            weights: weights.to_vec(),
            italic,
            fallback: Vec::new(),
            files: Vec::new(),
        }
    }

    /// The shape of what fonts.googleapis.com returns to a current browser.
    const LITERATA: &str = "/* cyrillic-ext */
@font-face {
  font-family: 'Literata';
  font-style: italic;
  font-weight: 400;
  font-display: swap;
  src: url(https://fonts.gstatic.com/s/literata/v35/or38Q6P12-iJxAIgLYT1PLs1a-t7PU0AbeE9KJ5T7ihaO_CS.woff2) format('woff2');
  unicode-range: U+0460-052F, U+1C80-1C8A, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F;
}
/* latin */
@font-face {
  font-family: 'Literata';
  font-style: normal;
  font-weight: 200 900;
  font-display: swap;
  src: url(https://fonts.gstatic.com/s/literata/v35/or3hQ6P12-iJxAIgLa78DkrbXsDgk0oVDaDPYLanFLHpPf2TbBG_F_bcTWCWp8g.woff2) format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}
";

    #[test]
    fn variants_list_weights_and_italics() {
        assert_eq!(
            variant(&spec("Literata", &[700, 400, 400], false)),
            "wght@400;700"
        );
        assert_eq!(
            variant(&spec("Literata", &[400, 700], true)),
            "ital,wght@0,400;0,700;1,400;1,700"
        );
        // No weights listed: regular and bold. Nonsense weights are dropped.
        assert_eq!(
            variant(&spec("Literata", &[0, 5000], false)),
            "wght@400;700"
        );
    }

    #[test]
    fn request_urls() {
        assert_eq!(
            css_url("Cormorant Garamond", Some("wght@600")).as_deref(),
            Some(
                "https://fonts.googleapis.com/css2?family=Cormorant+Garamond:wght@600&display=swap"
            )
        );
        assert_eq!(
            css_url("Literata", None).as_deref(),
            Some("https://fonts.googleapis.com/css2?family=Literata&display=swap")
        );
        for bad in ["", "A&b=c", "Literata:ital", "../x", "Ünicode"] {
            assert_eq!(css_url(bad, None), None, "{bad}");
        }
    }

    #[test]
    fn reads_google_css() {
        let faces = parse_css(LITERATA);
        assert_eq!(faces.len(), 2);
        assert_eq!(faces[0].style, "italic");
        assert_eq!(faces[0].weight, "400");
        assert!(faces[0].url.ends_with("ihaO_CS.woff2"));
        assert_eq!(faces[1].weight, "200 900");
        assert!(
            faces[1]
                .unicode_range
                .as_deref()
                .unwrap()
                .starts_with("U+0000-00FF, U+0131")
        );
    }

    #[test]
    fn unsafe_faces_are_skipped() {
        let other_host = LITERATA.replace("https://fonts.gstatic.com/", "https://evil.example/");
        assert!(parse_css(&other_host).is_empty());
        let quote = LITERATA.replace("ihaO_CS.woff2", "x\");}body{color:red}.woff2");
        assert_eq!(parse_css(&quote).len(), 1);
        let range = LITERATA.replace("U+0460-052F", "U+0460-052F, red");
        assert_eq!(parse_css(&range).len(), 1);
        // A brace ends the rule early: what's kept is only the valid part.
        let brace = LITERATA.replace("U+0460-052F,", "U+0460-052F; } body { color: red;");
        assert!(
            parse_css(&brace).iter().all(|f| !f
                .unicode_range
                .as_deref()
                .unwrap_or("")
                .contains("color"))
        );
        let style = LITERATA.replace("font-style: italic", "font-style: oblique 10deg");
        assert_eq!(parse_css(&style).len(), 1);
        let ttf = LITERATA.replace(".woff2) format('woff2')", ".ttf) format('truetype')");
        assert!(parse_css(&ttf).is_empty());
    }

    #[test]
    fn local_rules_point_at_the_cache() {
        let faces = vec![
            CachedFace {
                style: "normal".to_string(),
                weight: "200 900".to_string(),
                unicode_range: Some("U+0000-00FF".to_string()),
                file: "0.woff2".to_string(),
            },
            CachedFace {
                style: "normal".to_string(),
                weight: "400".to_string(),
                unicode_range: None,
                file: "../../secret".to_string(),
            },
        ];
        let css = font_face_css(
            "Literata",
            &faces,
            "http://scrald-theme.localhost/_google-fonts/literata-1/",
        );
        assert_eq!(
            css,
            "@font-face { font-family: \"Literata\"; font-style: normal; font-weight: 200 900; \
             font-display: swap; src: url(\"http://scrald-theme.localhost/_google-fonts/literata-1/0.woff2\") \
             format(\"woff2\"); unicode-range: U+0000-00FF; }\n"
        );
        assert_eq!(font_face_css("x\"y", &faces, "/"), "");
    }

    #[test]
    fn cache_folders_differ_by_variant() {
        let a = cache_folder("Cormorant Garamond", "wght@600");
        assert!(a.starts_with("cormorant-garamond-"));
        assert_ne!(a, cache_folder("Cormorant Garamond", "wght@400;600"));
    }
}
