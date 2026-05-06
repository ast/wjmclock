//! Hex color parsing for TOML config values.
//!
//! Accepts the three CSS-style hex shorthands:
//!
//! - `#rgb`       — each digit doubled (`#abc` → `#aabbcc`)
//! - `#rrggbb`    — full 24-bit RGB
//! - `#rrggbbaa`  — 24-bit RGB plus alpha
//!
//! Surfaced via a custom `Deserialize` so a malformed value fails at
//! config-load with a serde error pointing at the offending TOML line —
//! no silent fallback.

use egui::Color32;
use nom::{
    IResult,
    branch::alt,
    bytes::complete::{tag, take},
    combinator::{all_consuming, map_res},
};
use serde::{Deserialize, Deserializer, de};
use std::fmt;

/// An RGBA color parsed from a CSS-style hex string. Components are stored
/// *unmultiplied* (the values from the source hex). Use `into()` to convert
/// to `egui::Color32` at the egui API boundary — that's where premultiplied
/// alpha is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Compile-time-friendly opaque constructor for default values.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 0xff }
    }

    /// Parse a CSS-style hex color. Leading/trailing whitespace is trimmed.
    pub fn parse(s: &str) -> Result<Self, ColorParseError> {
        let trimmed = s.trim();
        match all_consuming(hex_color)(trimmed) {
            Ok((_, color)) => Ok(color),
            Err(_) => Err(ColorParseError(trimmed.to_string())),
        }
    }
}

impl From<Color> for Color32 {
    fn from(c: Color) -> Self {
        Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
    }
}

#[derive(Debug)]
pub struct ColorParseError(String);

impl fmt::Display for ColorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid color {:?} (expected #rgb, #rrggbb, or #rrggbbaa)",
            self.0
        )
    }
}

impl std::error::Error for ColorParseError {}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Color::parse(&s).map_err(de::Error::custom)
    }
}

// ---- nom parser ------------------------------------------------------------

/// One hex digit expanded to a full byte (`a` → `0xaa`). Used by `#rgb`.
fn hex_nibble(input: &str) -> IResult<&str, u8> {
    map_res(take(1usize), |s: &str| {
        u8::from_str_radix(s, 16).map(|n| n * 0x11)
    })(input)
}

/// Two hex digits → one byte. Used by `#rrggbb` and `#rrggbbaa`.
fn hex_byte(input: &str) -> IResult<&str, u8> {
    map_res(take(2usize), |s: &str| u8::from_str_radix(s, 16))(input)
}

fn parse_short(input: &str) -> IResult<&str, Color> {
    let (input, r) = hex_nibble(input)?;
    let (input, g) = hex_nibble(input)?;
    let (input, b) = hex_nibble(input)?;
    Ok((input, Color { r, g, b, a: 0xff }))
}

fn parse_long(input: &str) -> IResult<&str, Color> {
    let (input, r) = hex_byte(input)?;
    let (input, g) = hex_byte(input)?;
    let (input, b) = hex_byte(input)?;
    Ok((input, Color { r, g, b, a: 0xff }))
}

fn parse_long_alpha(input: &str) -> IResult<&str, Color> {
    let (input, r) = hex_byte(input)?;
    let (input, g) = hex_byte(input)?;
    let (input, b) = hex_byte(input)?;
    let (input, a) = hex_byte(input)?;
    Ok((input, Color { r, g, b, a }))
}

fn hex_color(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag("#")(input)?;
    // Try longest first so the 3-char form doesn't shadow a longer match —
    // `all_consuming` would catch it, but ordering avoids the wasted attempt.
    alt((parse_long_alpha, parse_long, parse_short))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(s: &str) -> (u8, u8, u8, u8) {
        let c = Color::parse(s).unwrap();
        (c.r, c.g, c.b, c.a)
    }

    #[test]
    fn long_form() {
        assert_eq!(rgba("#ff8000"), (0xff, 0x80, 0x00, 0xff));
        assert_eq!(rgba("#000000"), (0, 0, 0, 0xff));
    }

    #[test]
    fn long_form_alpha() {
        assert_eq!(rgba("#ff800040"), (0xff, 0x80, 0x00, 0x40));
        assert_eq!(rgba("#00000000"), (0, 0, 0, 0));
    }

    #[test]
    fn short_form_expands_each_nibble() {
        assert_eq!(rgba("#abc"), (0xaa, 0xbb, 0xcc, 0xff));
        assert_eq!(rgba("#000"), (0, 0, 0, 0xff));
        assert_eq!(rgba("#fff"), (0xff, 0xff, 0xff, 0xff));
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(rgba("#AABBCC"), rgba("#aabbcc"));
        assert_eq!(rgba("#AbC"), rgba("#abc"));
    }

    #[test]
    fn whitespace_is_trimmed() {
        assert_eq!(rgba("  #ff8000\t"), (0xff, 0x80, 0x00, 0xff));
    }

    #[test]
    fn rejects_missing_hash() {
        assert!(Color::parse("ff8000").is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(Color::parse("#").is_err());
        assert!(Color::parse("#f").is_err());
        assert!(Color::parse("#ff").is_err());
        assert!(Color::parse("#ffff").is_err());
        assert!(Color::parse("#fffff").is_err());
        assert!(Color::parse("#fffffff").is_err()); // 7 chars
        assert!(Color::parse("#fffffffff").is_err()); // 9 chars
    }

    #[test]
    fn rejects_non_hex_chars() {
        assert!(Color::parse("#zzz").is_err());
        assert!(Color::parse("#12g456").is_err());
        assert!(Color::parse("#1c3a3").is_err()); // the example from review feedback
    }

    #[test]
    fn rejects_empty() {
        assert!(Color::parse("").is_err());
        assert!(Color::parse("   ").is_err());
    }

    #[test]
    fn deserialize_from_toml_succeeds() {
        #[derive(Deserialize)]
        struct Wrap {
            color: Color,
        }
        let w: Wrap = toml::from_str(r##"color = "#7df5d2""##).unwrap();
        assert_eq!(w.color, Color::rgb(0x7d, 0xf5, 0xd2));
    }

    #[test]
    fn deserialize_from_toml_surfaces_error() {
        #[derive(Debug, Deserialize)]
        struct Wrap {
            #[allow(dead_code)]
            color: Color,
        }
        let err = toml::from_str::<Wrap>(r##"color = "#nope""##).unwrap_err();
        assert!(err.to_string().contains("invalid color"), "got: {err}");
    }
}
