//! How a control that is held down repeats: once on the press, then, after a short wait, at a
//! steady pace that quickens the longer the hold lasts.
//!
//! The count a hold has earned is worked out from how long it has lasted and from nothing else, so
//! it is the same at any frame rate: a frame that comes late takes every step the time since the
//! last one earned.

use std::time::{Duration, Instant};

/// How long a control is held before it starts to repeat.
pub(super) const REPEAT_DELAY: Duration = Duration::from_millis(300);

/// How often a held control repeats, which is the reference's own pace for a held turntable
/// (`ControlInputProcessor.java:261`).
pub(super) const REPEAT_EVERY: Duration = Duration::from_millis(50);

/// From how long into a hold each repeat counts for more than one step, and for how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Quickening {
    pub(super) after: Duration,
    pub(super) steps: u32,
}

/// How a held control repeats: the steps of each repeat, stage by stage. Every pace waits
/// [`REPEAT_DELAY`] and then repeats every [`REPEAT_EVERY`], one step a repeat until a stage says
/// otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Pace {
    pub(super) quickenings: &'static [Quickening],
}

/// The pace of a shade moved with the turntable: a step a repeat to begin with, so a short hold
/// lands on the step the player wants, and several once the hold is plainly a long way to go. The
/// thousand steps from one end of a lane to the other take a little over seven seconds.
pub(super) const SHADE_PACE: Pace =
    Pace { quickenings: &[Quickening { after: Duration::from_millis(1_500), steps: 4 }, Quickening { after: Duration::from_millis(3_000), steps: 10 }] };

/// The pace of a value whose whole range is a few hundred steps: it never quickens.
pub(super) const STEADY_PACE: Pace = Pace { quickenings: &[] };

/// One step, which is what a press is and what a repeat is until a stage quickens it.
const SINGLE_STEP: u32 = 1;

impl Pace {
    /// How many steps a hold that has lasted `held` has earned, the press itself among them.
    pub(super) fn earned(&self, held: Duration) -> u32 {
        if held < REPEAT_DELAY {
            return SINGLE_STEP;
        }
        let mut earned = SINGLE_STEP;
        let mut from = REPEAT_DELAY;
        let mut steps = SINGLE_STEP;
        for stage in self.quickenings.iter().filter(|stage| stage.after > REPEAT_DELAY) {
            if held < stage.after {
                break;
            }
            earned = earned.saturating_add(repeats_between(from, stage.after).saturating_mul(steps));
            from = stage.after;
            steps = stage.steps;
        }
        earned.saturating_add((repeats_between(from, held).saturating_add(SINGLE_STEP)).saturating_mul(steps))
    }
}

/// How many whole repeats fit between two points of a hold.
fn repeats_between(from: Duration, to: Duration) -> u32 {
    u32::try_from(to.saturating_sub(from).as_nanos() / REPEAT_EVERY.as_nanos()).unwrap_or(u32::MAX)
}

/// One control as it is held: since when, and how many of the steps the hold has earned were
/// already handed out.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct HeldRepeat {
    since: Option<Instant>,
    taken: u32,
}

impl HeldRepeat {
    /// The steps due at `now` for a control read as held or not: one the moment it is first seen
    /// down, and then the repeats.
    pub(super) fn steps(&mut self, now: Instant, down: bool, pace: &Pace) -> u32 {
        let pressed = if down && self.since.is_none() { SINGLE_STEP } else { 0 };
        pressed + self.repeats(now, down, pace)
    }

    /// The repeats due at `now`, for a control whose press was acted on where it arrived: nothing
    /// the moment it is first seen down, and then what the hold earns after it.
    pub(super) fn repeats(&mut self, now: Instant, down: bool, pace: &Pace) -> u32 {
        if !down {
            *self = HeldRepeat::default();
            return 0;
        }
        let Some(since) = self.since else {
            *self = HeldRepeat { since: Some(now), taken: SINGLE_STEP };
            return 0;
        };
        let earned = pace.earned(now.saturating_duration_since(since));
        let due = earned.saturating_sub(self.taken);
        self.taken = self.taken.max(earned);
        due
    }

    /// Forget the hold, so a control still down counts as pressed afresh.
    pub(super) fn reset(&mut self) {
        *self = HeldRepeat::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame of sixty to the second, near enough.
    const FRAME: Duration = Duration::from_millis(16);

    /// Everything a hold of `length` hands out when it is read once every `frame`.
    fn handed_out(length: Duration, frame: Duration, pace: &Pace) -> u32 {
        let start = Instant::now();
        let mut held = HeldRepeat::default();
        let mut total = 0;
        let mut at = Duration::ZERO;
        while at <= length {
            total += held.steps(start + at, true, pace);
            at += frame;
        }
        total
    }

    /// A press is one step however long the key stays down short of the wait, and the first repeat
    /// comes the moment the wait is over.
    #[test]
    fn a_press_is_one_step_until_the_wait_is_over() {
        for millis in [0, 16, 80, 120, 299] {
            assert_eq!(SHADE_PACE.earned(Duration::from_millis(millis)), 1, "held {millis} ms");
        }
        assert_eq!(SHADE_PACE.earned(REPEAT_DELAY), 2);
        assert_eq!(SHADE_PACE.earned(REPEAT_DELAY + REPEAT_EVERY - Duration::from_millis(1)), 2);
        assert_eq!(SHADE_PACE.earned(REPEAT_DELAY + REPEAT_EVERY), 3);
    }

    /// The pace quickens stage by stage, and the count never falls back as a stage begins.
    #[test]
    fn a_long_hold_quickens_and_never_loses_a_step() {
        assert_eq!(SHADE_PACE.earned(Duration::from_millis(1_499)), 1 + 24, "a step every fifty milliseconds from 300 to 1499");
        assert_eq!(SHADE_PACE.earned(Duration::from_millis(1_500)), 1 + 24 + 4, "the first repeat of the second stage");
        assert_eq!(SHADE_PACE.earned(Duration::from_millis(2_999)), 1 + 24 + 30 * 4);
        assert_eq!(SHADE_PACE.earned(Duration::from_millis(3_000)), 1 + 24 + 30 * 4 + 10);
        let mut last = 0;
        for millis in 0..8_000 {
            let earned = SHADE_PACE.earned(Duration::from_millis(millis));
            assert!(earned >= last, "the count fell from {last} to {earned} at {millis} ms");
            last = earned;
        }
        assert!(last >= 1_000, "eight seconds are not enough to cross a lane: {last}");
        assert_eq!(STEADY_PACE.earned(Duration::from_millis(3_000)), 1 + 55, "a steady pace never quickens");
    }

    /// What a hold hands out is what its length earned, whatever the frames it was read on.
    #[test]
    fn what_a_hold_hands_out_does_not_depend_on_the_frame_rate() {
        for length in [100, 700, 2_000, 4_000].map(Duration::from_millis) {
            let earned = SHADE_PACE.earned(length);
            for frame in [Duration::from_millis(1), Duration::from_millis(4), FRAME, Duration::from_millis(100)] {
                let read = handed_out(length, frame, &SHADE_PACE);
                let last_read = Duration::from_nanos((length.as_nanos() / frame.as_nanos() * frame.as_nanos()) as u64);
                assert_eq!(read, SHADE_PACE.earned(last_read), "a hold of {length:?} read every {frame:?}");
                assert!(read <= earned);
            }
        }
    }

    /// A control let go starts over, and one whose press was acted on elsewhere hands out its
    /// repeats alone.
    #[test]
    fn letting_go_starts_the_hold_over_and_a_press_taken_elsewhere_is_not_counted_twice() {
        let start = Instant::now();
        let mut held = HeldRepeat::default();
        assert_eq!(held.steps(start, true, &STEADY_PACE), 1);
        assert_eq!(held.steps(start + FRAME, true, &STEADY_PACE), 0);
        assert_eq!(held.steps(start + FRAME * 2, false, &STEADY_PACE), 0);
        assert_eq!(held.steps(start + FRAME * 3, true, &STEADY_PACE), 1, "a second press is a step of its own");

        let mut repeating = HeldRepeat::default();
        assert_eq!(repeating.repeats(start, true, &STEADY_PACE), 0, "the press was acted on where it arrived");
        assert_eq!(repeating.repeats(start + REPEAT_DELAY - FRAME, true, &STEADY_PACE), 0);
        assert_eq!(repeating.repeats(start + REPEAT_DELAY, true, &STEADY_PACE), 1);
        assert_eq!(repeating.repeats(start + REPEAT_DELAY + REPEAT_EVERY * 3, true, &STEADY_PACE), 3);
        repeating.reset();
        assert_eq!(repeating.repeats(start + Duration::from_secs(2), true, &STEADY_PACE), 0, "a hold that was forgotten starts over");
    }
}
