//! Property cluster B: the score of the run, in progress or finished.
//!
//! Where the values come from: the tally of the run and the scores it is held against: EX score and
//! rates, judgement counts, the best and the target, and the gauge `main_state` reports.
//! [`standing`] holds the reference's `ScoreData` and `ScoreDataProperty` as plain values, and a
//! [`RunScore`] is one of each. A finished run is borrowed from its
//! [`ResultSnapshot`](super::result::snapshot::ResultSnapshot) ([`ScoreState::of_result`]); the play
//! screen will lend the run so far the same way.
//!
//! What this cluster answers: numbers 71-72, 75, 80-89, 100-103, 105, 107-108, 110-116, 121-123,
//! 128, 135-136, 150-158, 170-172, 174, 183-184, 271, 407 and 410-427; floats 85-89, 122, 135, 155,
//! 157, 183, 1102, 1107 and 1115; rates 110-115, 140-145 and 147; options 200-207, 220-227,
//! 230-240, 300-307, 320-327, 340-347 and 2241-2246; and what `main_state` asks about the run
//! itself: the gauge, its type, the judgement counts and the three scores. The total note count
//! (74, 106) is the chart's and stays with cluster A. The figures that compare the run with the
//! best it replaced (173, 175-178) are the result screen's, cluster G.
//!
//! A value is read as `IntegerPropertyFactory`, `BooleanPropertyFactory`, `FloatPropertyFactory`
//! and `MainStatePropertyLuaApiExporter` read it. That includes what looks wrong there and is kept:
//! the run's maximum score reads zero rather than absent when there is no score, the rate ids
//! 140-147 are the browser's and read zero anywhere else, and `main_state.gauge` is zero outside
//! play.
//!
//! A cluster that has no run answers nothing, so the ids fall through to whatever else can.

use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, INTEGER_ABSENT, ScoreSlot, ScoreSnapshot};

use super::ClusterState;
use super::result::snapshot::FinishedRun;
use standing::{CUMULATIVE_RANK_STEPS, RANK_COUNT, ScoreSheet, ScoreStanding, TargetPace};

pub mod standing;

/// How many judgement kinds there are, PGREAT to MISS.
pub(super) const JUDGE_KINDS: i32 = 6;

/// The judgement codes the combined counts add up, in the reference's order
/// (`IntegerPropertyFactory.java:441-476`).
pub(super) const GREAT: i32 = 1;
pub(super) const BAD: i32 = 3;
pub(super) const POOR: i32 = 4;
pub(super) const MISS: i32 = 5;

/// The share of the gauge each of the gauge-range options (0-9, 10-19 ... 90-99 and 100) covers
/// (`BooleanPropertyFactory.GaugeDrawCondition`).
const GAUGE_RANGE_SHARE: f32 = 0.1;

/// Early and late: the two sides each judgement's count is split on, in the order the numbers 410-419
/// pair them.
pub(super) const SIDES_PER_JUDGEMENT: i32 = 2;

/// A share of a hundred, which is what the judgement-rate numbers are cut to.
const PERCENT: i32 = 100;

/// The scale and the modulus of the digit after the dot of a gauge number
/// (`IntegerPropertyFactory.createGrooveGaugeAfterDotProperty`).
const TENTHS_PER_UNIT: f32 = 10.0;
const DIGIT_MODULUS: i32 = 10;

/// A count as the whole number a skin shows.
pub fn whole(count: u32) -> i32 {
    i32::try_from(count).unwrap_or(i32::MAX)
}

/// The digit after the dot of a gauge value. A gauge above nothing and below a tenth shows as a
/// tenth, so it never reads as empty while it is not.
fn tenths_digit(value: f32) -> i32 {
    let tenths = value * TENTHS_PER_UNIT;
    (if tenths > 0.0 && tenths < 1.0 { 1.0 } else { tenths }) as i32 % DIGIT_MODULUS
}

/// One run's score and where it stands: the tally and what the reference makes of it.
#[derive(Debug, Clone, PartialEq)]
pub struct RunScore {
    pub sheet: ScoreSheet,
    pub standing: ScoreStanding,
}

impl RunScore {
    /// A run that has ended: every note has gone by.
    pub fn finished(sheet: ScoreSheet, target: TargetPace) -> RunScore {
        RunScore::in_progress(sheet, sheet.notes, target)
    }

    /// A run `pass_notes` notes into its chart.
    pub fn in_progress(sheet: ScoreSheet, pass_notes: u32, target: TargetPace) -> RunScore {
        RunScore { standing: ScoreStanding::of(&sheet, pass_notes, target), sheet }
    }
}

/// A gauge as the gauge numbers read it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeReading {
    /// Where the gauge stands, from nothing to its maximum.
    pub value: f32,
    /// The most the gauge holds, which the gauge-range options divide.
    pub max: f32,
    /// Which gauge it is, as the reference numbers gauges.
    pub kind: i32,
    /// Whether the gauge is the one of a run in progress. The ranges and `main_state.gauge` answer
    /// only for that: a finished run's gauge is a number on the screen and nothing more.
    pub live: bool,
}

impl GaugeReading {
    /// What the gauge holds when it is full, for a gauge that records a bare percentage.
    pub const FULL: f32 = 100.0;
}

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScoreState<'a> {
    /// The run, when there is one on the screen.
    pub run: Option<&'a RunScore>,
    /// The gauge the gauge numbers read.
    pub gauge: Option<GaugeReading>,
}

impl<'a> ScoreState<'a> {
    /// The finished run on a result screen, with the gauge the screen has the graph on.
    pub fn of_result(finished: FinishedRun<'a>) -> ScoreState<'a> {
        ScoreState { run: Some(&finished.snapshot.score), gauge: finished.gauge_reading() }
    }

    /// Which rank an option id of a band that starts at `first` stands for, AAA first, or `None`
    /// for an id outside the band.
    fn rank_in_band(id: i32, first: i32) -> Option<usize> {
        let step = usize::try_from(id - first).ok().filter(|step| *step < RANK_COUNT)?;
        Some(RANK_COUNT - 1 - step)
    }

    /// The share of the gauge an option of the range band stands for, as a low and a high end of
    /// the gauge's maximum.
    fn gauge_range(id: i32) -> Option<(f32, f32)> {
        let range = id.checked_sub(OPTION_1P_0_9).filter(|range| (0..=OPTION_1P_100 - OPTION_1P_0_9).contains(range))?;
        Some((range as f32 * GAUGE_RANGE_SHARE, (range + 1) as f32 * GAUGE_RANGE_SHARE))
    }

    /// The gauge number of a finished run's selected gauge, absent when it has none.
    fn gauge_number(&self, read: impl Fn(f32) -> i32) -> i32 {
        self.gauge.map_or(INTEGER_ABSENT, |gauge| read(gauge.value))
    }
}

impl ClusterState for ScoreState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let run = self.run?;
        let standing = &run.standing;
        if let Some(rank) =
            Self::rank_in_band(id, OPTION_1P_AAA).or_else(|| Self::rank_in_band(id, OPTION_RESULT_AAA_1P)).or_else(|| Self::rank_in_band(id, OPTION_NOW_AAA_1P))
        {
            return Some(standing.now_rank_band(rank));
        }
        if let Some(rank) = Self::rank_in_band(id, OPTION_BEST_AAA_1P) {
            return Some(standing.best_rank_band(rank));
        }
        if let Some(rank) = Self::rank_in_band(id, OPTION_AAA) {
            return Some(standing.secured(CUMULATIVE_RANK_STEPS[RANK_COUNT - 1 - rank]));
        }
        if let Some((low, high)) = Self::gauge_range(id) {
            return Some(self.gauge.filter(|gauge| gauge.live).is_some_and(|gauge| gauge.value >= low * gauge.max && gauge.value < high * gauge.max));
        }
        match id {
            OPTION_PERFECT_EXIST..=OPTION_MISS_EXIST => Some(run.sheet.count(id - OPTION_PERFECT_EXIST) > 0),
            _ => None,
        }
    }

    fn integer(&self, id: i32) -> Option<i32> {
        let run = self.run?;
        let sheet = &run.sheet;
        let standing = &run.standing;
        let count = |judge: i32| whole(sheet.count(judge));
        let count_on = |judge: i32, early: bool| whole(sheet.count_on(judge, early));
        Some(match id {
            NUMBER_SCORE | NUMBER_SCORE2 | NUMBER_SCORE3 => standing.now_ex,
            NUMBER_MAXSCORE => whole(sheet.max_ex_score()),
            NUMBER_MAXCOMBO | NUMBER_MAXCOMBO2 | NUMBER_MAXCOMBO3 => whole(sheet.max_combo),
            NUMBER_PERFECT2..=NUMBER_POOR2 => count(id - NUMBER_PERFECT2),
            NUMBER_PERFECT_RATE..=NUMBER_POOR_RATE if sheet.notes > 0 => count(id - NUMBER_PERFECT_RATE) * PERCENT / whole(sheet.notes),
            NUMBER_PERFECT_RATE..=NUMBER_POOR_RATE => INTEGER_ABSENT,
            NUMBER_POINT => standing.now_point,
            NUMBER_SCORE_RATE => standing.now_rate_int,
            NUMBER_SCORE_RATE_AFTERDOT => standing.now_rate_after_dot,
            NUMBER_GROOVEGAUGE => self.gauge_number(|value| value as i32),
            NUMBER_GROOVEGAUGE_AFTERDOT => self.gauge_number(tenths_digit),
            NUMBER_DIFF_EXSCORE | NUMBER_DIFF_EXSCORE2 | NUMBER_DIFF_TARGETSCORE => standing.now_ex - standing.now_rival_score,
            NUMBER_PERFECT..=NUMBER_POOR => count(id - NUMBER_PERFECT),
            NUMBER_EARLY_PERFECT..=NUMBER_LATE_POOR => {
                let offset = id - NUMBER_EARLY_PERFECT;
                count_on(offset / SIDES_PER_JUDGEMENT, offset % SIDES_PER_JUDGEMENT == 0)
            }
            NUMBER_MISS => count(MISS),
            NUMBER_EARLY_MISS => count_on(MISS, true),
            NUMBER_LATE_MISS => count_on(MISS, false),
            NUMBER_TOTALEARLY => (GREAT..JUDGE_KINDS).map(|judge| count_on(judge, true)).sum(),
            NUMBER_TOTALLATE => (GREAT..JUDGE_KINDS).map(|judge| count_on(judge, false)).sum(),
            NUMBER_COMBOBREAK => count(BAD) + count(POOR),
            NUMBER_POOR_PLUS_MISS => count(POOR) + count(MISS),
            NUMBER_BAD_PLUS_POOR_PLUS_MISS => count(BAD) + count(POOR) + count(MISS),
            NUMBER_TOTAL_RATE | NUMBER_SCORE_RATE2 => standing.rate_int,
            NUMBER_TOTAL_RATE_AFTERDOT | NUMBER_SCORE_RATE_AFTERDOT2 => standing.rate_after_dot,
            NUMBER_TARGET_SCORE | NUMBER_TARGET_SCORE2 | NUMBER_RIVAL_SCORE => standing.rival_score,
            NUMBER_TARGET_SCORE_RATE | NUMBER_TARGET_TOTAL_RATE | NUMBER_TARGET_SCORE_RATE2 => standing.rival_rate_int,
            NUMBER_TARGET_SCORE_RATE_AFTERDOT | NUMBER_TARGET_TOTAL_RATE_AFTERDOT | NUMBER_TARGET_SCORE_RATE_AFTERDOT2 => standing.rival_rate_after_dot,
            NUMBER_HIGHSCORE | NUMBER_HIGHSCORE2 => standing.best_score,
            NUMBER_DIFF_HIGHSCORE | NUMBER_DIFF_HIGHSCORE2 => standing.now_ex - standing.now_best_score,
            NUMBER_DIFF_NEXTRANK => standing.next_rank,
            NUMBER_BEST_RATE => standing.best_rate_int,
            NUMBER_BEST_RATE_AFTERDOT => standing.best_rate_after_dot,
            _ => return None,
        })
    }

    fn rate(&self, id: i32) -> Option<f32> {
        let run = self.run?;
        let standing = &run.standing;
        Some(match id {
            RATE_SCORE => standing.rate,
            RATE_SCORE_FINAL => standing.now_rate,
            RATE_BESTSCORE_NOW => standing.now_best_rate,
            RATE_BESTSCORE => standing.best_rate,
            RATE_TARGETSCORE_NOW => standing.now_rival_rate,
            RATE_TARGETSCORE => standing.rival_rate,
            RATE_PGREAT..=RATE_MAXCOMBO | RATE_EXSCORE => 0.0,
            _ => return None,
        })
    }

    fn float(&self, id: i32) -> Option<f32> {
        let run = self.run?;
        let sheet = &run.sheet;
        let standing = &run.standing;
        Some(match id {
            FLOAT_PERFECT_RATE..=FLOAT_POOR_RATE if sheet.notes > 0 => sheet.count(id - FLOAT_PERFECT_RATE) as f32 / sheet.notes as f32,
            FLOAT_PERFECT_RATE..=FLOAT_POOR_RATE => FLOAT_ABSENT,
            FLOAT_SCORE_RATE => standing.now_rate,
            FLOAT_TOTAL_RATE | FLOAT_SCORE_RATE2 => standing.rate,
            FLOAT_BEST_RATE => standing.best_rate,
            FLOAT_RIVAL_RATE | FLOAT_TARGET_RATE | FLOAT_TARGET_RATE2 => standing.rival_rate,
            FLOAT_GROOVEGAUGE_1P => self.gauge.map_or(FLOAT_ABSENT, |gauge| gauge.value),
            _ => return None,
        })
    }

    fn gauge(&self) -> Option<f32> {
        self.run.map(|_| self.gauge.filter(|gauge| gauge.live).map_or(0.0, |gauge| gauge.value))
    }

    fn gauge_type(&self) -> Option<i32> {
        self.run.map(|_| self.gauge.filter(|gauge| gauge.live).map_or(0, |gauge| gauge.kind))
    }

    fn judge(&self, judge: i32) -> Option<i32> {
        self.run.map(|run| whole(run.sheet.count(judge)))
    }

    fn score(&self, slot: ScoreSlot) -> Option<ScoreSnapshot> {
        let standing = &self.run?.standing;
        Some(match slot {
            ScoreSlot::Current => ScoreSnapshot { rate: standing.now_rate, exscore: standing.now_ex },
            ScoreSlot::Best => ScoreSnapshot { rate: standing.now_best_rate, exscore: standing.best_score },
            ScoreSlot::Rival => ScoreSnapshot { rate: standing.rival_rate, exscore: standing.rival_score },
        })
    }
}

#[cfg(test)]
mod tests;
