//! Hex colors: parsing, blending, and formatting. Used to validate theme
//! colors and to derive the colors a theme leaves out.

use std::fmt;

/// An RGBA color, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    /// Parses `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa`.
    pub fn parse(text: &str) -> Option<Color> {
        let hex = text.trim().strip_prefix('#')?;
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let digit = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).ok();
        let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        match hex.len() {
            3 | 4 => Some(Color {
                r: digit(0)? * 17,
                g: digit(1)? * 17,
                b: digit(2)? * 17,
                a: if hex.len() == 4 { digit(3)? * 17 } else { 255 },
            }),
            6 | 8 => Some(Color {
                r: pair(0)?,
                g: pair(2)?,
                b: pair(4)?,
                a: if hex.len() == 8 { pair(6)? } else { 255 },
            }),
            _ => None,
        }
    }

    /// Mixes `self` toward `other`: `amount` 0.0 gives `self`, 1.0 gives `other`.
    pub fn mix(self, other: Color, amount: f32) -> Color {
        let t = amount.clamp(0.0, 1.0);
        let channel =
            |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
        Color {
            r: channel(self.r, other.r),
            g: channel(self.g, other.g),
            b: channel(self.b, other.b),
            a: channel(self.a, other.a),
        }
    }

    /// Relative luminance as WCAG defines it: 0.0 for black, 1.0 for white.
    /// Alpha is ignored.
    pub fn luminance(self) -> f32 {
        let linear = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }

    /// WCAG contrast ratio between two colors, from 1.0 (the same) to 21.0
    /// (black on white). Body text wants 4.5 or more.
    pub fn contrast(self, other: Color) -> f32 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// The opaque color seen when `self` (possibly translucent) is drawn over
    /// `background`, as a highlight is drawn over the page.
    pub fn over(self, background: Color) -> Color {
        Color {
            a: 255,
            ..background.mix(self, f32::from(self.a) / 255.0)
        }
    }

    /// The same color with a different alpha (0.0..=1.0).
    pub fn with_alpha(self, alpha: f32) -> Color {
        Color {
            a: (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
            ..self
        }
    }
}

// Rust note: implementing `Display` gives `to_string()` and lets the type be
// used in `format!("{}")`, like overloading `operator<<` in C++.
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)?;
        if self.a != 255 {
            write!(f, "{:02x}", self.a)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_hex_forms() {
        assert_eq!(Color::parse("#abc"), Some(Color::rgb(0xaa, 0xbb, 0xcc)));
        assert_eq!(Color::parse("#2e3440"), Some(Color::rgb(0x2e, 0x34, 0x40)));
        assert_eq!(Color::parse(" #ebcb8b44 ").map(|c| c.a), Some(0x44));
        assert_eq!(Color::parse("#abcd").map(|c| c.a), Some(0xdd));
    }

    #[test]
    fn rejects_non_hex() {
        for bad in ["2e3440", "#12", "#12345", "#gggggg", "red", "", "#é12"] {
            assert_eq!(Color::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn mixes_and_formats() {
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);
        assert_eq!(black.mix(white, 0.5).to_string(), "#808080");
        assert_eq!(black.mix(white, 2.0), white);
        assert_eq!(white.with_alpha(0.5).to_string(), "#ffffff80");
        assert_eq!(white.with_alpha(0.5).over(black).to_string(), "#808080");
        assert_eq!(white.over(black), white);
    }

    #[test]
    fn contrast_follows_wcag() {
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);
        assert!((black.contrast(white) - 21.0).abs() < 0.01);
        assert!((white.contrast(white) - 1.0).abs() < 0.01);
        // #767676 is the classic lightest gray that passes 4.5:1 on white.
        let gray = Color::rgb(0x76, 0x76, 0x76);
        assert!((gray.contrast(white) - 4.54).abs() < 0.02);
    }
}
