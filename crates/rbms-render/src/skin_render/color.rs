//! The one colour parser every document-declared graph shares, and the one way they mix a colour
//! with the destination that placed them.
//!
//! A document writes its palette as hex strings and the mirror in [`rbms_skin::model`] keeps them
//! exactly as written, because validating a third-party document is not its job. Turning one of
//! those strings into a colour is, and doing it in one place keeps the graphs from each inventing
//! their own tolerance for what counts as a colour.

use crate::{CHANNEL_MAX, Color};

/// Hex digits in an opaque `RRGGBB` colour.
const OPAQUE_DIGITS: usize = 6;

/// Hex digits in an `RRGGBBAA` colour.
const ALPHA_DIGITS: usize = 8;

/// Hex digits one channel takes.
const DIGITS_PER_CHANNEL: usize = 2;

/// The base a colour is written in.
const HEX_RADIX: u32 = 16;

/// The colour a document wrote, or `None` when the text is not one.
///
/// `RRGGBB` is opaque and `RRGGBBAA` carries its own alpha, which is the pair the reference's own
/// graph records are written in. A leading `#` is accepted because documents written by hand carry
/// one often enough that refusing it would only ever lose a colour.
pub fn parse_hex_color(text: &str) -> Option<Color> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != OPAQUE_DIGITS && digits.len() != ALPHA_DIGITS {
        return None;
    }
    let mut channels = [u8::MAX; 4];
    for (index, channel) in digits.as_bytes().chunks(DIGITS_PER_CHANNEL).enumerate() {
        let text = std::str::from_utf8(channel).ok()?;
        channels[index] = u8::from_str_radix(text, HEX_RADIX).ok()?;
    }
    Some(Color { r: channels[0], g: channels[1], b: channels[2], a: channels[3] })
}

/// One colour scaled by another, channel by channel.
///
/// The objects that draw a wheel or a graph carry colours of their own rather than sampling a
/// texture, so this is what a destination's colour means to them: an unstated destination is opaque
/// white and leaves the object exactly as its record named it, while a document that dims or fades
/// one gets the same fade an ordinary image tint would have given it.
pub(crate) fn modulate(color: Color, tint: Color) -> Color {
    let channel = |left: u8, right: u8| ((u32::from(left) * u32::from(right)) / CHANNEL_MAX) as u8;
    Color { r: channel(color.r, tint.r), g: channel(color.g, tint.g), b: channel(color.b, tint.b), a: channel(color.a, tint.a) }
}
