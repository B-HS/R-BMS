//! The id bands this player answers beyond the reference's own, and the one question every gate
//! that checks an id against the reference asks of them.
//!
//! The reference declares the ids a skin may name, and the gates of this crate and of the renderer
//! ask [`reference_implements`] before they read one: a Lua `main_state.number` answers zero, an
//! image set's `value` falls back to its `ref`, a text's `ref` names no property, for an id the
//! reference has nothing under. This player answers a few things the reference has no id for at all,
//! and keeps them in bands of its own, well above everything the reference declares and clear of the
//! bands it leaves to a skin (options 900 to 999, events 1000 to 1999, timers 10000 to 19999):
//!
//! - events [`EXTENSION_EVENT_FIRST`] to [`EXTENSION_EVENT_LAST`];
//! - options [`EXTENSION_OPTION_FIRST`] to [`EXTENSION_OPTION_LAST`];
//! - numbers, rates, floats and texts [`EXTENSION_VALUE_FIRST`] to [`EXTENSION_VALUE_LAST`], each
//!   space on its own, as the reference keeps them. The rates and the floats are read through one
//!   lookup (a float the float space does not hold is asked of the rates), so a number is given to
//!   one of the two and never to both;
//! - reference images [`EXTENSION_IMAGE_FIRST`] to [`EXTENSION_IMAGE_LAST`], which a document places
//!   by a negative destination id, as it places the reference's own.
//!
//! An image index has no band: nothing of this player's is selected by one.
//!
//! [`implements`] is [`reference_implements`] with these bands let through, and it is what a gate
//! asks. The reference's own answer stays as it is, because what it reports is the reference.
//!
//! The names of the ids inside the bands, and which part of the game answers each, are the player's
//! (`apps/rbms-player/src/skin_host/extension.rs`). An id in a band that nothing answers reads as
//! the absent value of its kind.

use super::names::{NameSpace, reference_implements};

/// The first and last of the events this player keeps for itself.
pub const EXTENSION_EVENT_FIRST: i32 = 21_000;
pub const EXTENSION_EVENT_LAST: i32 = 21_099;

/// The first and last of the options this player keeps for itself. A negative id reads the negation
/// of its positive, as every option does.
pub const EXTENSION_OPTION_FIRST: i32 = 21_100;
pub const EXTENSION_OPTION_LAST: i32 = 21_199;

/// The first and last of the numbers, rates, floats and texts this player keeps for itself.
pub const EXTENSION_VALUE_FIRST: i32 = 21_000;
pub const EXTENSION_VALUE_LAST: i32 = 21_199;

/// The first and last of the reference images this player draws itself, as image ids: a document
/// places one by the negated id as its destination id.
pub const EXTENSION_IMAGE_FIRST: i32 = 21_000;
pub const EXTENSION_IMAGE_LAST: i32 = 21_199;

/// The NOTES RADAR polygon of the chart in hand: an opaque white hexagon on a clear square, which the
/// destination's colour and alpha tint. A document places it by the destination id `"-21020"`.
pub const RBMS_IMAGE_NOTES_RADAR: i32 = 21_020;

/// The first and last id of this player's band in `space`, or `None` for a space it keeps none in.
pub const fn band(space: NameSpace) -> Option<(i32, i32)> {
    match space {
        NameSpace::Event => Some((EXTENSION_EVENT_FIRST, EXTENSION_EVENT_LAST)),
        NameSpace::Boolean => Some((EXTENSION_OPTION_FIRST, EXTENSION_OPTION_LAST)),
        NameSpace::Integer | NameSpace::Rate | NameSpace::Float | NameSpace::Text => Some((EXTENSION_VALUE_FIRST, EXTENSION_VALUE_LAST)),
        NameSpace::ImageIndex => None,
    }
}

/// Whether `id` is in this player's band of `space`. A boolean id is asked about without its sign.
pub fn in_band(space: NameSpace, id: i32) -> bool {
    let id = if space == NameSpace::Boolean { id.saturating_abs() } else { id };
    band(space).is_some_and(|(first, last)| (first..=last).contains(&id))
}

/// Whether a property stands under `id` in `space`: one the reference has
/// ([`reference_implements`]), or one of this player's bands.
pub fn implements(space: NameSpace, id: i32) -> bool {
    reference_implements(space, id) || in_band(space, id)
}

/// Whether `image` is an image id of this player's reference image band.
pub const fn is_extension_image(image: i32) -> bool {
    EXTENSION_IMAGE_FIRST <= image && image <= EXTENSION_IMAGE_LAST
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property::generated::{ALL_IMAGE, NUMBER_PLAYLEVEL, OPTION_AUTOPLAYON, RATE_SCORE, STRING_TITLE};

    /// The spaces this player keeps a band in, the event space aside: the reference implements every
    /// event id, so its band is let through either way.
    const BANDED: [NameSpace; 5] = [NameSpace::Boolean, NameSpace::Integer, NameSpace::Rate, NameSpace::Float, NameSpace::Text];

    /// The highest timer id the reference leaves to a skin.
    const SKIN_TIMER_LAST: i32 = 19_999;

    #[test]
    fn the_reference_has_nothing_anywhere_in_a_band() {
        for space in BANDED {
            let (first, last) = band(space).expect("a banded space has a band");
            for id in first..=last {
                assert!(!reference_implements(space, id), "{space:?} {id} is the reference's");
                assert!(implements(space, id), "{space:?} {id} is let through");
            }
        }
    }

    #[test]
    fn every_band_lies_above_what_the_reference_leaves_to_a_skin() {
        const { assert!(EXTENSION_EVENT_FIRST > SKIN_TIMER_LAST && EXTENSION_VALUE_FIRST > SKIN_TIMER_LAST && EXTENSION_IMAGE_FIRST > SKIN_TIMER_LAST) };
        const { assert!(EXTENSION_EVENT_LAST < EXTENSION_OPTION_FIRST, "the event and the option bands are apart") };
        for space in NameSpace::ALL {
            if let Some((first, last)) = band(space) {
                assert!(first <= last, "{space:?} runs backwards");
            }
        }
    }

    #[test]
    fn outside_the_bands_the_answer_is_the_reference_s() {
        for (space, id) in [
            (NameSpace::Integer, NUMBER_PLAYLEVEL),
            (NameSpace::Boolean, OPTION_AUTOPLAYON),
            (NameSpace::Rate, RATE_SCORE),
            (NameSpace::Float, RATE_SCORE),
            (NameSpace::Text, STRING_TITLE),
        ] {
            assert!(implements(space, id), "{space:?} {id}");
        }
        for space in BANDED {
            let (first, last) = band(space).expect("a banded space has a band");
            assert_eq!(implements(space, first - 1), reference_implements(space, first - 1), "{space:?} below the band");
            assert!(!implements(space, last + 1), "{space:?} above the band");
        }
        assert!(!implements(NameSpace::ImageIndex, RBMS_IMAGE_NOTES_RADAR), "an image index has no band");
    }

    #[test]
    fn a_negative_option_in_the_band_reads_the_negation_of_one_that_is_there() {
        assert!(implements(NameSpace::Boolean, -EXTENSION_OPTION_FIRST));
        assert!(implements(NameSpace::Boolean, -EXTENSION_OPTION_LAST));
        assert!(!in_band(NameSpace::Boolean, i32::MIN), "the most negative id does not fold into the band");
    }

    #[test]
    fn the_notes_radar_is_an_image_of_the_band_and_none_of_the_reference_s() {
        const { assert!(is_extension_image(RBMS_IMAGE_NOTES_RADAR)) };
        assert!(ALL_IMAGE.iter().all(|(image, _)| !is_extension_image(*image)), "the reference keeps an image in the band");
        assert!(!is_extension_image(EXTENSION_IMAGE_FIRST - 1) && !is_extension_image(EXTENSION_IMAGE_LAST + 1));
    }
}
