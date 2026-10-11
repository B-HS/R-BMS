//! The scroll speed a held note travel time comes to.
//!
//! A player who reads by the green number wants the same number on every chart: the time a note is
//! on show before it reaches the judge line, whatever the chart's tempo is and however much of the
//! lane the cover hides. With [`PlayOptions::keep_green_number`] on, that time is what the
//! configuration stores ([`PlayOptions::duration_ms`]) and the scroll speed is worked out from it,
//! which is what the reference does while its speed fix is on (`LaneRenderer.resetHispeed`,
//! `LaneRenderer.java:206-210`):
//!
//! ```text
//! hispeed = 240000 / bpm / duration * (1 - lanecover)
//! ```
//!
//! Everything here is a plain function of numbers. The play screen calls it when a chart starts and
//! whenever the cover moves, and a screen that only previews the options can call the same thing.
//!
//! Three things are settled here and nowhere else.
//!
//! - **The lift does not enter.** The note field scrolls a note over the lane the lift leaves in the
//!   same time it scrolls it over the whole lane (`LaneRenderer.java:324-326` scales the scroll by
//!   the lifted height, and `currentduration` at `:330` is `region * (1 - lanecover)` alone), so the
//!   travel time is already held when the lift moves and there is no speed to change. An arcade
//!   cabinet reaches the same reading the other way round: its lift shortens the lane without
//!   slowing the notes, and its floating speed then slows them. [`HeldTravel::lift`] is carried so a
//!   caller states the whole lane it is asking about, and it changes no answer.
//! - **The hidden cover does not enter either.** It hides notes that are about to be hit, not notes
//!   that are yet to appear, and neither the reference's `currentduration` nor an arcade cabinet's
//!   green number counts it.
//! - **A cover that is switched off counts as no cover**, as the reference reads it
//!   (`isEnablelanecover() ? getLanecover() : 0`).
//!
//! The green number a skin shows is three fifths of the travel time
//! (`IntegerPropertyFactory.duration_green`), cut to a whole number. [`green_of_duration`] and
//! [`duration_of_green`] go between the two.

use crate::schema::{DURATION_MAX_MS, DURATION_MIN_MS, HISPEED_MAX, HISPEED_MIN, PlayOptions};

/// The green number is this many fifths of the travel time (`duration * 3 / 5`).
const GREEN_NUMERATOR: i32 = 3;

/// The fifths the travel time is cut into for the green number.
const GREEN_DENOMINATOR: i32 = 5;

/// How far one press of a speed key moves a held green number.
pub const GREEN_NUMBER_STEP: i32 = 10;

/// How far one press of a speed key moves a held green number while the fine modifier is down.
pub const GREEN_NUMBER_FINE_STEP: i32 = 1;

/// The smallest green number a travel time in range reads as, which is what the GREEN NUMBER row
/// floors at.
pub const GREEN_NUMBER_MIN: i32 = 1;

/// The largest green number a travel time in range reads as.
pub const GREEN_NUMBER_MAX: i32 = DURATION_MAX_MS / GREEN_DENOMINATOR * GREEN_NUMERATOR;

/// The scroll speed a travel time is measured at before it is divided by the speed in force.
const UNIT_HISPEED: f64 = 1.0;

/// The chart scroll rate a held travel time is measured at: a chart's own `#SCROLL` is not part of
/// the tempo the whole chart is held against.
const UNIT_SCROLL: f64 = 1.0;

/// What `Math.round` adds before it takes the floor.
const ROUND_HALF: f64 = 0.5;

/// The green number a travel time reads as (`IntegerPropertyFactory.duration_green`).
pub fn green_of_duration(duration_ms: i32) -> i32 {
    (i64::from(duration_ms) * i64::from(GREEN_NUMERATOR) / i64::from(GREEN_DENOMINATOR)) as i32
}

/// The shortest travel time that reads as `green`, held to the range the row can store.
///
/// Three travel times in every five read as a green number of their own and the other two share
/// one, so every green number has a travel time and the shortest of them is the one taken.
pub fn duration_of_green(green: i32) -> i32 {
    let wanted = (i64::from(green) * i64::from(GREEN_DENOMINATOR) + i64::from(GREEN_NUMERATOR) - 1).div_euclid(i64::from(GREEN_NUMERATOR));
    wanted.clamp(i64::from(DURATION_MIN_MS), i64::from(DURATION_MAX_MS)) as i32
}

/// The travel time `by_green` green numbers away from `duration_ms`, held to its range.
pub fn duration_stepped_by_green(duration_ms: i32, by_green: i32) -> i32 {
    let green = green_of_duration(duration_ms).saturating_add(by_green).clamp(GREEN_NUMBER_MIN, GREEN_NUMBER_MAX);
    duration_of_green(green)
}

/// The travel time `by_ms` milliseconds away from `duration_ms`, held to its range
/// (`EventFactory.duration1p`).
pub fn duration_stepped(duration_ms: i32, by_ms: i32) -> i32 {
    duration_ms.saturating_add(by_ms).clamp(DURATION_MIN_MS, DURATION_MAX_MS)
}

/// How long a note is on show at `bpm` and `hispeed` under `lane_cover`, in whole milliseconds
/// (`LaneRenderer.currentduration` at a scroll rate of one), or zero where there is none: a tempo or
/// a speed of nothing, or a lane that is covered whole.
pub fn travel_duration_ms(bpm: f64, hispeed: f64, lane_cover: Option<f32>) -> i32 {
    let travel = rbms_chart::scroll::green_number(bpm, hispeed, UNIT_SCROLL, f64::from(lane_cover.unwrap_or_default()));
    (travel + ROUND_HALF).floor() as i32
}

/// The travel time to hold and the lane it is held on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldTravel {
    /// The travel time asked for, in milliseconds.
    pub duration_ms: i32,
    /// The tempo it is held at.
    pub bpm: f64,
    /// How much of the lane the cover takes, when the cover is on.
    pub lane_cover: Option<f32>,
    /// How much of the lane the lift takes, when the lift is on. It changes no answer; see the
    /// module's own notes.
    pub lift: Option<f32>,
    /// The slowest and the fastest scroll speed the answer may be.
    pub hispeed_min: f64,
    pub hispeed_max: f64,
}

impl HeldTravel {
    /// The travel time `play` holds, at `bpm`, on the lane as `play` has it: the cover and the lift
    /// count while they are switched on, and the speed may be anything the HI-SPEED row can hold.
    pub fn of(play: &PlayOptions, bpm: f64) -> HeldTravel {
        HeldTravel {
            duration_ms: play.duration_ms,
            bpm,
            lane_cover: play.enable_cover.then_some(play.cover),
            lift: play.enable_lift.then_some(play.lift),
            hispeed_min: HISPEED_MIN,
            hispeed_max: HISPEED_MAX,
        }
    }
}

/// The scroll speed a held travel time came to, and what a note's travel reads as at it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldSpeed {
    /// The scroll speed, inside the range that was asked for.
    pub hispeed: f64,
    /// The travel time a note has at that speed, in whole milliseconds. It is the one asked for
    /// unless the speed was held at an end of its range.
    pub duration_ms: i32,
    /// The green number that travel time reads as.
    pub green_number: i32,
    /// Whether the speed that would have held the travel time was outside the range, so the speed
    /// stands at the nearer end and the travel time is not the one asked for.
    pub limited: bool,
}

/// The scroll speed that holds `travel`, or `None` where no speed can: a tempo of nothing, a travel
/// time of nothing, or a lane the cover hides whole. A caller with no answer keeps the speed it has.
pub fn held_speed(travel: &HeldTravel) -> Option<HeldSpeed> {
    if travel.duration_ms <= 0 {
        return None;
    }
    let cover = f64::from(travel.lane_cover.unwrap_or_default());
    let at_unit_speed = rbms_chart::scroll::green_number(travel.bpm, UNIT_HISPEED, UNIT_SCROLL, cover);
    if !at_unit_speed.is_finite() || at_unit_speed <= 0.0 {
        return None;
    }
    let wanted = at_unit_speed / f64::from(travel.duration_ms);
    let hispeed = wanted.clamp(travel.hispeed_min, travel.hispeed_max);
    let duration_ms = travel_duration_ms(travel.bpm, hispeed, travel.lane_cover);
    Some(HeldSpeed { hispeed, duration_ms, green_number: green_of_duration(duration_ms), limited: hispeed != wanted })
}

impl PlayOptions {
    /// Put the scroll speed where the held travel time wants it at `bpm`, and answer what it came
    /// to. Nothing moves, and the answer is `None`, while the travel time is not being held or no
    /// speed can hold it.
    pub fn hold_travel_at(&mut self, bpm: f64) -> Option<HeldSpeed> {
        if !self.keep_green_number {
            return None;
        }
        let held = held_speed(&HeldTravel::of(self, bpm))?;
        self.hispeed = held.hispeed;
        Some(held)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How close two scroll speeds have to be to count as the same.
    const SPEED_TOLERANCE: f64 = 1e-9;

    fn travel(duration_ms: i32, bpm: f64, lane_cover: Option<f32>) -> HeldTravel {
        HeldTravel { duration_ms, bpm, lane_cover, lift: None, hispeed_min: HISPEED_MIN, hispeed_max: HISPEED_MAX }
    }

    #[test]
    fn the_speed_is_the_references_own_relation() {
        let held = held_speed(&travel(800, 150.0, None)).expect("a tempo and a travel time have a speed");
        assert!((held.hispeed - 2.0).abs() < SPEED_TOLERANCE, "240000 / 150 / 800 is {}", held.hispeed);
        assert_eq!((held.duration_ms, held.green_number, held.limited), (800, 480, false));
        let covered = held_speed(&travel(800, 150.0, Some(0.25))).expect("a covered lane has a speed");
        assert!((covered.hispeed - 1.5).abs() < SPEED_TOLERANCE, "the cover takes a quarter of the speed with it: {}", covered.hispeed);
        assert_eq!(covered.duration_ms, 800);
    }

    #[test]
    fn a_travel_time_reads_the_same_at_every_tempo_and_under_every_cover() {
        for bpm in [60.0, 96.0, 150.0, 177.5, 200.0, 280.0] {
            for cover in [None, Some(0.0), Some(0.137), Some(0.4), Some(0.85)] {
                let held = held_speed(&travel(500, bpm, cover)).expect("in range");
                assert_eq!(held.duration_ms, 500, "bpm {bpm} cover {cover:?} at speed {}", held.hispeed);
                assert_eq!(held.green_number, 300);
                assert_eq!(travel_duration_ms(bpm, held.hispeed, cover), 500);
            }
        }
    }

    #[test]
    fn the_lift_and_its_switch_change_no_answer() {
        let plain = travel(500, 150.0, Some(0.2));
        for lift in [None, Some(0.0), Some(0.3), Some(0.9)] {
            assert_eq!(held_speed(&HeldTravel { lift, ..plain }), held_speed(&plain), "lift {lift:?}");
        }
    }

    #[test]
    fn a_speed_outside_its_range_stops_at_the_end_and_says_what_the_travel_time_really_is() {
        let slow = held_speed(&travel(DURATION_MAX_MS, 400.0, Some(0.9))).expect("clamped, not refused");
        assert_eq!(slow.hispeed, HISPEED_MIN);
        assert!(slow.limited);
        assert_eq!(slow.duration_ms, travel_duration_ms(400.0, HISPEED_MIN, Some(0.9)));
        assert_ne!(slow.duration_ms, DURATION_MAX_MS, "the number on show is the real one, not the one asked for");
        let fast = held_speed(&travel(DURATION_MIN_MS, 60.0, None)).expect("clamped, not refused");
        assert_eq!(fast.hispeed, HISPEED_MAX);
        assert!(fast.limited);
        assert_eq!(fast.duration_ms, 200, "240000 / 60 / 20");
        assert_eq!(fast.green_number, 120);
    }

    #[test]
    fn there_is_no_speed_for_a_lane_covered_whole_or_a_tempo_of_nothing() {
        assert_eq!(held_speed(&travel(500, 150.0, Some(1.0))), None);
        assert_eq!(held_speed(&travel(500, 0.0, None)), None);
        assert_eq!(held_speed(&travel(500, -120.0, None)), None);
        assert_eq!(held_speed(&travel(0, 150.0, None)), None);
        assert_eq!(held_speed(&travel(500, f64::NAN, None)), None);
        assert_eq!(travel_duration_ms(150.0, 2.0, Some(1.0)), 0);
    }

    #[test]
    fn every_green_number_has_a_travel_time_that_reads_as_it() {
        for green in GREEN_NUMBER_MIN..=GREEN_NUMBER_MAX {
            let duration = duration_of_green(green);
            assert!((DURATION_MIN_MS..=DURATION_MAX_MS).contains(&duration));
            assert_eq!(green_of_duration(duration), green, "travel time {duration}");
            assert!(green_of_duration(duration - 1) < green, "{duration} is not the shortest travel time that reads as {green}");
        }
        assert_eq!(green_of_duration(500), 300, "500 * 3 / 5");
        assert_eq!(green_of_duration(501), 300, "cut, not rounded");
        assert_eq!(GREEN_NUMBER_MAX, green_of_duration(DURATION_MAX_MS));
    }

    #[test]
    fn a_step_moves_the_green_number_by_what_it_says_and_stops_at_the_ends() {
        assert_eq!(green_of_duration(duration_stepped_by_green(500, GREEN_NUMBER_STEP)), 310);
        assert_eq!(green_of_duration(duration_stepped_by_green(500, -GREEN_NUMBER_STEP)), 290);
        assert_eq!(green_of_duration(duration_stepped_by_green(501, GREEN_NUMBER_FINE_STEP)), 301);
        assert_eq!(duration_stepped_by_green(DURATION_MAX_MS, GREEN_NUMBER_STEP), DURATION_MAX_MS);
        assert_eq!(green_of_duration(duration_stepped_by_green(DURATION_MIN_MS, -GREEN_NUMBER_STEP)), GREEN_NUMBER_MIN);
        assert_eq!(duration_stepped_by_green(500, i32::MAX), DURATION_MAX_MS);
        assert_eq!(duration_stepped(500, 1), 501);
        assert_eq!(duration_stepped(DURATION_MIN_MS, -1), DURATION_MIN_MS);
        assert_eq!(duration_stepped(DURATION_MAX_MS, i32::MAX), DURATION_MAX_MS);
    }

    #[test]
    fn the_options_hold_their_travel_time_only_while_they_are_asked_to() {
        let mut play = PlayOptions { duration_ms: 800, cover: 0.25, enable_cover: true, ..PlayOptions::default() };
        let before = play.hispeed;
        assert_eq!(play.hold_travel_at(150.0), None, "the row is off");
        assert_eq!(play.hispeed, before);
        play.keep_green_number = true;
        let held = play.hold_travel_at(150.0).expect("held");
        assert!((play.hispeed - 1.5).abs() < SPEED_TOLERANCE);
        assert_eq!(held.duration_ms, 800);
        play.enable_cover = false;
        play.hold_travel_at(150.0);
        assert!((play.hispeed - 2.0).abs() < SPEED_TOLERANCE, "a cover that is off counts as none");
        play.cover = 1.0;
        play.enable_cover = true;
        assert_eq!(play.hold_travel_at(150.0), None);
        assert!((play.hispeed - 2.0).abs() < SPEED_TOLERANCE, "a lane covered whole keeps the speed it has");
    }
}
