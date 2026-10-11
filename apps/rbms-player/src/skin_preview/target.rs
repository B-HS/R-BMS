//! What a preview is asked to show, and what the scroll of a previewed play screen comes to.
//!
//! A preview follows one of two things. The skin menu's follows the screen its SCREEN row names,
//! which is stored. The play menu's is handed a play screen and a tempo by the menu itself
//! ([`PlayTarget`]): neither is a setting, both only choose the sample the play options are shown
//! on.
//!
//! [`SpeedReadout`] is the one place the two halves of the play menu get the figures of a target
//! from. The menu writes them under the preview and on its rows; the preview's sample run is drawn
//! at the same speed, because both come to it through `rbms_config::held_speed` and
//! `rbms_config::travel_duration_ms` at the tempo [`PlayTarget::scroll_bpm`] names.

use rbms_config::{HeldTravel, PlayOptions, green_of_duration, held_speed, travel_duration_ms};

use crate::stage::play::skin_preview::steady_scroll_bpm;

/// What the lane cover's share of the lane is multiplied by to read it as the white number
/// (skin number 14).
const WHITE_NUMBER_SCALE: f32 = 1000.0;

/// A play screen previewed on a sample chart of one tempo.
///
/// The chart holds `bpm` from its first measure to its last, so the tempo it opens at, its slowest,
/// its fastest and the one most of its notes are played at are all `bpm`, and whichever of them the
/// HI-SPEED FIX row names is the same number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayTarget {
    /// The play screen, as a skin type id (`rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS` and the ones
    /// beside it).
    pub(crate) screen: i32,
    /// The tempo of the sample chart, in beats a minute.
    pub(crate) bpm: u32,
}

/// What a preview shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewTarget {
    /// The screen the skin menu is configuring: the stored `config.skin.screen`, read every frame.
    Configured,
    /// The play screen and the tempo its caller names, whatever `config.skin.screen` holds.
    Play(PlayTarget),
}

/// What the scroll of a previewed play screen comes to under the play options as they stand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SpeedReadout {
    /// The tempo the travel time is measured at: the target's own, or the field's while CONSTANT
    /// is on.
    pub(crate) bpm: f64,
    /// The scroll speed the sample is drawn at: the stored one, or the one that holds the green
    /// number at `bpm` while KEEP GREEN NUMBER is on.
    pub(crate) hispeed: f64,
    /// How long a note is on show at that speed under the lane cover as it stands, in whole
    /// milliseconds.
    pub(crate) duration_ms: i32,
    /// The green number that travel time reads as.
    pub(crate) green_number: i32,
    /// The white number: the lane cover's share of the lane to the nearest thousandth, whether or
    /// not the cover is switched on.
    pub(crate) white_number: i32,
    /// Whether the lane cover is switched on, which is when the white number is in force.
    pub(crate) cover_on: bool,
    /// Whether the speed was worked out from a held green number rather than read from HI-SPEED.
    pub(crate) held: bool,
    /// Whether the speed that would have held the green number is outside what HI-SPEED can be, so
    /// the speed stands at the nearer end and the green number on show is not the one asked for.
    pub(crate) limited: bool,
}

impl PlayTarget {
    /// The tempo the scroll of this target is worked out at under `play`: the chart's own, and the
    /// one tempo the field scrolls at while CONSTANT is on.
    pub(crate) fn scroll_bpm(self, play: &PlayOptions) -> f64 {
        steady_scroll_bpm(play, self.bpm)
    }

    /// What the scroll of this target comes to under `play`.
    ///
    /// While KEEP GREEN NUMBER is on and a speed can hold the travel time asked for, the speed is
    /// the held one (`rbms_config::held_speed`, which is what a run of the chart starts at).
    /// Otherwise the speed is the stored HI-SPEED and the travel time is what a note has at it.
    pub(crate) fn readout(self, play: &PlayOptions) -> SpeedReadout {
        let bpm = self.scroll_bpm(play);
        let lane_cover = play.enable_cover.then_some(play.cover);
        let held = play.keep_green_number.then(|| held_speed(&HeldTravel::of(play, bpm))).flatten();
        let (hispeed, duration_ms, limited) = match held {
            Some(held) => (held.hispeed, held.duration_ms, held.limited),
            None => (play.hispeed, travel_duration_ms(bpm, play.hispeed, lane_cover), false),
        };
        SpeedReadout {
            bpm,
            hispeed,
            duration_ms,
            green_number: green_of_duration(duration_ms),
            white_number: (play.cover * WHITE_NUMBER_SCALE).round() as i32,
            cover_on: play.enable_cover,
            held: held.is_some(),
            limited,
        }
    }
}

#[cfg(test)]
mod tests {
    use rbms_config::{DEFAULT_DURATION_MS, PlayOptions};
    use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;

    use super::*;

    const SLOW: PlayTarget = PlayTarget { screen: SKIN_TYPE_PLAY_7KEYS, bpm: 100 };
    const FAST: PlayTarget = PlayTarget { screen: SKIN_TYPE_PLAY_7KEYS, bpm: 200 };

    /// A held green number reads the same at any tempo, and the speed that holds it does not.
    #[test]
    fn a_held_green_number_is_the_same_at_two_tempos_and_the_speed_is_not() {
        let play = PlayOptions { keep_green_number: true, duration_ms: DEFAULT_DURATION_MS, ..PlayOptions::default() };
        let (slow, fast) = (SLOW.readout(&play), FAST.readout(&play));
        assert!(slow.held && fast.held);
        assert_eq!(slow.green_number, fast.green_number, "the green number moved with the tempo");
        assert_eq!(slow.duration_ms, DEFAULT_DURATION_MS);
        assert!((slow.hispeed - fast.hispeed * 2.0).abs() < 1e-9, "half the tempo did not double the speed: {} and {}", slow.hispeed, fast.hispeed);
    }

    /// With nothing held the speed is the stored one at any tempo, and the travel time follows the
    /// tempo.
    #[test]
    fn a_stored_speed_is_the_same_at_two_tempos_and_the_travel_time_is_not() {
        let play = PlayOptions { keep_green_number: false, ..PlayOptions::default() };
        let (slow, fast) = (SLOW.readout(&play), FAST.readout(&play));
        assert!(!slow.held && !fast.held && !slow.limited);
        assert!((slow.hispeed - play.hispeed).abs() < f64::EPSILON && (fast.hispeed - play.hispeed).abs() < f64::EPSILON);
        assert_eq!(slow.duration_ms, fast.duration_ms * 2, "half the tempo did not double the travel time");
    }

    /// The lane cover shortens the travel time of a stored speed, speeds a held one up, and is the
    /// white number either way; switched off it does neither and the number stays what it was.
    #[test]
    fn the_lane_cover_enters_the_travel_time_only_while_it_is_on() {
        let open = PlayOptions { enable_cover: true, cover: 0.0, ..PlayOptions::default() };
        let covered = PlayOptions { cover: 0.5, ..open.clone() };
        let off = PlayOptions { enable_cover: false, ..covered.clone() };
        assert_eq!(SLOW.readout(&covered).duration_ms * 2, SLOW.readout(&open).duration_ms);
        assert_eq!(SLOW.readout(&off).duration_ms, SLOW.readout(&open).duration_ms);
        assert_eq!((SLOW.readout(&covered).white_number, SLOW.readout(&covered).cover_on), (500, true));
        assert_eq!((SLOW.readout(&off).white_number, SLOW.readout(&off).cover_on), (500, false));

        let held = PlayOptions { keep_green_number: true, ..covered };
        let held_open = PlayOptions { cover: 0.0, ..held.clone() };
        assert_eq!(SLOW.readout(&held).green_number, SLOW.readout(&held_open).green_number);
        assert!(SLOW.readout(&held).hispeed < SLOW.readout(&held_open).hispeed, "a cover did not slow the speed that holds the number");
    }

    /// A green number no speed in range can hold is said to be limited, and the number on show is
    /// the one the nearest speed gives.
    #[test]
    fn a_green_number_out_of_reach_is_limited() {
        let play = PlayOptions { keep_green_number: true, duration_ms: 1, ..PlayOptions::default() };
        let readout = SLOW.readout(&play);
        assert!(readout.held && readout.limited);
        assert!((readout.hispeed - rbms_config::HISPEED_MAX).abs() < f64::EPSILON);
        assert!(readout.duration_ms > play.duration_ms);
    }

    /// Under CONSTANT the field scrolls at one tempo of its own, so the target's tempo changes
    /// nothing.
    #[test]
    fn under_constant_the_tempo_of_the_target_changes_nothing() {
        for keep_green_number in [false, true] {
            let play = PlayOptions { constant_speed: true, keep_green_number, ..PlayOptions::default() };
            assert_eq!(SLOW.readout(&play), FAST.readout(&play), "held {keep_green_number}");
            assert!((SLOW.scroll_bpm(&play) - FAST.scroll_bpm(&play)).abs() < f64::EPSILON);
        }
    }
}
