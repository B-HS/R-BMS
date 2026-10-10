//! Property cluster G: the result of a finished run.
//!
//! Where the values come from: the score a run ended on beside the one it replaced: clear or
//! failed, what was updated, who won, the miss count, the timing average, the gauge the graph is
//! on, the replay slots and the course the run belongs to. It is read from the run's
//! [`ResultSnapshot`](snapshot::ResultSnapshot), made once when the screen is entered, and from what
//! the screen holds itself: the gauge the graph is on and the replay slots.
//!
//! What this cluster answers: numbers 76, 173, 175-178 and 370-377; floats 372, 374 and 376;
//! options 42-43 and 1046 (which kind of gauge the graph is on), 90-91, 196-198, 330-336,
//! 352-354, 1196-1204 and 1330-1336; image indices 370-371; strings 150-159. The run's score
//! itself -- the EX score, the rates, the best it replaced (170-172) -- is cluster B's, read from
//! the same snapshot.
//!
//! A value is read as `IntegerPropertyFactory`, `BooleanPropertyFactory`, `FloatPropertyFactory`
//! and `StringPropertyFactory` read it from `AbstractResult`, including the odd ones: the timing
//! average in milliseconds (float 374) is the reach of the distribution divided by a thousand, the
//! reference reading the array centre where it means the average. An update is a comparison with
//! `oldscore`, which for a chart never played is a score of nothing with the worst possible
//! bad-poor count, so a first run updates every figure it improves on that.
//!
//! A cluster that has no run answers nothing, so the ids fall through to whatever else can.
//!
//! A course result reads the same cluster: [`course::CourseTally`] adds a course's stages into the one
//! run its snapshot is made from.

use std::borrow::Cow;
use std::cmp::Ordering;

use rbms_skin::property::INTEGER_ABSENT;
use rbms_skin::property::generated::*;

use super::score::whole;
use super::{ClusterState, INDEX_CLEAR, INDEX_TARGET_CLEAR};
use snapshot::{FinishedRun, REPLAY_SLOT_COUNT, ReplaySlot};

pub mod course;
pub mod snapshot;

/// The reference's `Integer.MAX_VALUE`, which is the bad-poor count of a chart never played.
const MISS_COUNT_NEVER_PLAYED: i64 = i32::MAX as i64;

/// The gauges that count up to a clear, and the ones that count down from a full gauge: the type
/// numbers below and from this one up (`BooleanPropertyFactory.gauge_groove` and `gauge_hard`).
const FIRST_HARD_GAUGE: i32 = 3;

/// The gauges drained at the EX rate (`BooleanPropertyFactory.gauge_ex`).
const EX_RATE_GAUGES: [i32; 6] = [0, 1, 4, 5, 7, 8];

/// The thousand microseconds in a millisecond, and the ten that make the digits after the dot of
/// the average duration (`IntegerPropertyFactory.duration_average`).
const MICROS_PER_MILLI: i64 = 1_000;
const MICROS_PER_HUNDREDTH_MILLI: i64 = 10;
const AFTER_DOT_MODULUS: i64 = 100;

/// The scale and modulus of the digits after the dot of a timing figure.
const TIMING_DIGIT_SCALE: f32 = 100.0;
const TIMING_DIGIT_MODULUS: i32 = 100;

/// A millisecond count as seconds' thousandths: the unit the reference divides the reach of the
/// timing distribution by for its float figure.
const MILLIS_PER_UNIT: f32 = 1_000.0;

/// The option ids of each replay slot's three states, in the order none, stored, saved just now
/// (`OPTION_NO_REPLAYDATA` and its neighbours).
const REPLAY_OPTIONS: [[i32; 3]; REPLAY_SLOT_COUNT] = [
    [OPTION_NO_REPLAYDATA, OPTION_REPLAYDATA, OPTION_REPLAYDATA_SAVED],
    [OPTION_NO_REPLAYDATA2, OPTION_REPLAYDATA2, OPTION_REPLAYDATA2_SAVED],
    [OPTION_NO_REPLAYDATA3, OPTION_REPLAYDATA3, OPTION_REPLAYDATA3_SAVED],
    [OPTION_NO_REPLAYDATA4, OPTION_REPLAYDATA4, OPTION_REPLAYDATA4_SAVED],
];

/// The state a replay slot is in for each of the three ids of [`REPLAY_OPTIONS`].
const REPLAY_STATES: [ReplaySlot; 3] = [ReplaySlot::Missing, ReplaySlot::Exists, ReplaySlot::Saved];

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct ResultState<'a> {
    /// The finished run, when there is one on the screen.
    pub run: Option<FinishedRun<'a>>,
}

impl<'a> ResultState<'a> {
    /// The finished run on a result screen.
    pub fn of(finished: FinishedRun<'a>) -> ResultState<'a> {
        ResultState { run: Some(finished) }
    }
}

/// The two digits after the dot of a timing in milliseconds, which for a negative timing are
/// negative too (`IntegerPropertyFactory.timing_average_afterdot`).
fn timing_digits(timing: f32) -> i32 {
    if timing >= 0.0 {
        (timing * TIMING_DIGIT_SCALE) as i32 % TIMING_DIGIT_MODULUS
    } else {
        (-((timing.abs() * TIMING_DIGIT_SCALE) % TIMING_DIGIT_SCALE)) as i32
    }
}

impl ClusterState for ResultState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let finished = self.run?;
        let run = finished.snapshot;
        let standing = &run.score.standing;
        let sheet = &run.score.sheet;
        let ex = i64::from(standing.now_ex);
        let old_ex = i64::from(run.previous.ex_score);
        let rival = i64::from(standing.rival_score);
        let combo = sheet.max_combo.cmp(&run.previous.max_combo);
        let miss = i64::from(sheet.min_bp).cmp(&run.previous.min_bp.map_or(MISS_COUNT_NEVER_PLAYED, i64::from));
        let rank_rate = standing.now_rate.partial_cmp(&standing.best_rate);
        let gauge = finished.scene.gauge_type as i32;
        if let Some((slot, state)) = REPLAY_OPTIONS.iter().enumerate().find_map(|(slot, ids)| Some((slot, ids.iter().position(|option| *option == id)?))) {
            return Some(finished.scene.replay[slot] == REPLAY_STATES[state]);
        }
        Some(match id {
            OPTION_RESULT_CLEAR => run.cleared(),
            OPTION_RESULT_FAIL => !run.cleared(),
            OPTION_UPDATE_SCORE => ex.cmp(&old_ex).is_gt(),
            OPTION_DRAW_SCORE => ex.cmp(&old_ex).is_eq(),
            OPTION_UPDATE_MAXCOMBO => combo.is_gt(),
            OPTION_DRAW_MAXCOMBO => combo.is_eq(),
            OPTION_UPDATE_MISSCOUNT => miss.is_lt(),
            OPTION_DRAW_MISSCOUNT => miss.is_eq(),
            OPTION_UPDATE_SCORERANK => rank_rate == Some(Ordering::Greater),
            OPTION_DRAW_SCORERANK => rank_rate == Some(Ordering::Equal),
            OPTION_UPDATE_TARGET => ex.cmp(&rival).is_gt(),
            OPTION_DRAW_TARGET => ex.cmp(&rival).is_eq(),
            OPTION_1PWIN => ex.cmp(&rival).is_gt(),
            OPTION_2PWIN => ex.cmp(&rival).is_lt(),
            OPTION_DRAW => ex.cmp(&rival).is_eq(),
            OPTION_GAUGE_GROOVE => gauge < FIRST_HARD_GAUGE,
            OPTION_GAUGE_HARD => gauge >= FIRST_HARD_GAUGE,
            OPTION_GAUGE_EX => EX_RATE_GAUGES.contains(&gauge),
            _ => return None,
        })
    }

    fn integer(&self, id: i32) -> Option<i32> {
        let run = self.run?.snapshot;
        let new = &run.score.sheet;
        let old = &run.previous;
        let new_miss = whole(new.min_bp);
        let timing = &run.timing;
        let duration = timing.avg_duration_us;
        Some(match id {
            NUMBER_MISSCOUNT | NUMBER_MISSCOUNT2 => new_miss,
            NUMBER_TARGET_MAXCOMBO if old.max_combo > 0 => whole(old.max_combo),
            NUMBER_DIFF_MAXCOMBO if old.max_combo > 0 => whole(new.max_combo) - whole(old.max_combo),
            NUMBER_TARGET_MAXCOMBO | NUMBER_DIFF_MAXCOMBO => INTEGER_ABSENT,
            NUMBER_TARGET_MISSCOUNT => old.min_bp.map_or(INTEGER_ABSENT, whole),
            NUMBER_DIFF_MISSCOUNT => old.min_bp.map_or(INTEGER_ABSENT, |miss| new_miss - whole(miss)),
            NUMBER_CLEAR => i32::from(new.clear),
            NUMBER_TARGET_CLEAR => i32::from(old.clear),
            NUMBER_AVERAGE_DURATION => (duration / MICROS_PER_MILLI) as i32,
            NUMBER_AVERAGE_DURATION_AFTERDOT => ((duration / MICROS_PER_HUNDREDTH_MILLI) % AFTER_DOT_MODULUS) as i32,
            NUMBER_AVERAGE_TIMING => timing.average_ms as i32,
            NUMBER_AVERAGE_TIMING_AFTERDOT => timing_digits(timing.average_ms),
            NUMBER_STDDEV_TIMING => timing.std_dev_ms as i32,
            NUMBER_STDDEV_TIMING_AFTERDOT => (timing.std_dev_ms * TIMING_DIGIT_SCALE) as i32 % TIMING_DIGIT_MODULUS,
            _ => return None,
        })
    }

    fn image_index(&self, id: i32) -> Option<i32> {
        let run = self.run?.snapshot;
        match id {
            INDEX_CLEAR => Some(i32::from(run.score.sheet.clear)),
            INDEX_TARGET_CLEAR => Some(i32::from(run.previous.clear)),
            _ => None,
        }
    }

    fn float(&self, id: i32) -> Option<f32> {
        let timing = &self.run?.snapshot.timing;
        match id {
            FLOAT_DURATION_AVERAGE => Some(timing.avg_duration_us as f32 / MILLIS_PER_UNIT),
            FLOAT_TIMING_AVERAGE => Some(timing.center_ms as f32 / MILLIS_PER_UNIT),
            FLOAT_TIMIGN_STDDEV => Some(timing.std_dev_ms),
            _ => None,
        }
    }

    fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        let run = self.run?.snapshot;
        if !(STRING_COURSE1_TITLE..=STRING_COURSE10_TITLE).contains(&id) {
            return None;
        }
        let title = usize::try_from(id - STRING_COURSE1_TITLE).ok().and_then(|index| run.course_titles.get(index));
        Some(Cow::Borrowed(title.map_or("", String::as_str)))
    }
}

#[cfg(test)]
mod external_tests;
#[cfg(test)]
mod tests;
