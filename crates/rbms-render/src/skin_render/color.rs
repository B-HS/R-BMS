//! The one colour parser every document-declared graph shares.
//!
//! A document writes its palette as hex strings and the mirror in [`rbms_skin::model`] keeps them
//! exactly as written, because validating a third-party document is not its job. Turning one of
//! those strings into a colour is, and doing it in one place keeps the graphs from each inventing
//! their own tolerance for what counts as a colour.

use crate::Color;

/// Hex digits in an opaque `RRGGBB` colour.
const OPAQUE_DIGITS: usize = 6;

/// Hex digits in an `RRGGBBAA` colour.
const ALPHA_DIGITS: usize = 8;

/// Hex digits one channel takes.
pub(crate) const DIGITS_PER_CHANNEL: usize = 2;

/// The base a colour is written in.
pub(crate) const HEX_RADIX: u32 = 16;

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
