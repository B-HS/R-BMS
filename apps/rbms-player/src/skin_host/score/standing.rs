//! A score the way the reference measures it: the tally of a run and everything its score
//! properties derive from that tally.
//!
//! [`ScoreSheet`] is what `ScoreData` holds -- the judgements split by early and late, the combo, the
//! bad-poor count and the lamp. [`ScoreStanding`] is what `ScoreDataProperty` makes of one sheet
//! beside the best score and the target: the rates and their digits, the EX points, the rank each
//! rate has reached and the pace the best and the target would have by now. Every formula is the
//! reference's own, in the reference's own number types, so a figure that truncates there truncates
//! here (`ScoreDataProperty.java`, `ScoreData.java`).
//!
//! Both are plain values with no screen behind them. The result screen builds them once from a
//! finished run; the play screen can build them again each frame from the run so far, which is the
//! only thing that differs: `pass_notes` is every note on a result screen and the notes gone by
//! during play.

use rbms_judge::{ClearType, JudgeEngine, clear_type_id};
use rbms_model::Mode;
use rbms_play::{PlayRecord, RANK_STEP_COUNT};

use crate::skin_host::system::JUDGEMENT_KINDS;

/// How many EX points a PGREAT and a GREAT are worth (`ScoreData.getExscore`).
const EX_PER_PERFECT: u32 = 2;
const EX_PER_GREAT: u32 = 1;

/// The scale a rate is cut to a whole percent with, and the one its two digits after the dot are
/// cut from (`ScoreDataProperty.java:90-91`).
const PERCENT_SCALE: f32 = 100.0;
const AFTER_DOT_SCALE: f32 = 10_000.0;
const AFTER_DOT_MODULUS: i32 = 100;

/// The first rank step of each of the eight ranks from F up to AAA, then one past the last step
/// (`BooleanPropertyFactory.createNowRank`). A rank is the steps from its floor up to, and not
/// including, the next one's.
const RANK_BAND_FLOORS: [usize; 9] = [0, 6, 9, 12, 15, 18, 21, 24, 28];

/// How many ranks there are, F to AAA.
pub const RANK_COUNT: usize = 8;

/// The rank step each rank is secured at when asked cumulatively, AAA first
/// (`OPTION_AAA` to `OPTION_F`, `ScoreDataProperty.qualifyRank`).
pub const CUMULATIVE_RANK_STEPS: [usize; RANK_COUNT] = [24, 21, 18, 15, 12, 9, 6, 0];

/// A rank step the next-rank search stops at: every third step is a rank's floor
/// (`ScoreDataProperty.java:104`).
const RANK_FLOOR_STRIDE: usize = 3;

/// How the points of the run in progress are weighed for a kind of play: what a PGREAT, a GREAT and
/// a GOOD are worth, and whether the max combo adds a share of its own
/// (`ScoreDataProperty.java:62-83`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointFamily {
    /// Five- and ten-key beat play: 100000, 100000 and 50000 shares of the total.
    Beat5,
    /// Seven- and fourteen-key beat play: 150000, 100000 and 20000, and up to 50000 more for the
    /// max combo.
    #[default]
    Beat7,
    /// Pop'n play: 100000, 70000 and 40000.
    Popn,
    /// Every other mode, scored out of a million: 1000000, 700000 and 400000.
    Other,
}

impl PointFamily {
    /// The family a mode is scored in.
    pub fn of_mode(mode: Mode) -> PointFamily {
        if mode == Mode::BEAT_5K || mode == Mode::BEAT_10K {
            PointFamily::Beat5
        } else if mode == Mode::BEAT_7K || mode == Mode::BEAT_14K {
            PointFamily::Beat7
        } else if mode == Mode::POPN_9K {
            PointFamily::Popn
        } else {
            PointFamily::Other
        }
    }

    /// What a PGREAT, a GREAT and a GOOD are each worth, and what a full combo adds.
    fn weights(self) -> (i64, i64, i64, i64) {
        match self {
            PointFamily::Beat5 => (100_000, 100_000, 50_000, 0),
            PointFamily::Beat7 => (150_000, 100_000, 20_000, 50_000),
            PointFamily::Popn => (100_000, 70_000, 40_000, 0),
            PointFamily::Other => (1_000_000, 700_000, 400_000, 0),
        }
    }
}

/// What `ScoreData` holds of one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScoreSheet {
    /// How many times each judgement was given on the early side, in the order PGREAT, GREAT,
    /// GOOD, BAD, POOR and MISS (`epg`, `egr`, `egd`, `ebd`, `epr`, `ems`).
    pub early: [u32; JUDGEMENT_KINDS],
    /// The same on the late side (`lpg` to `lms`).
    pub late: [u32; JUDGEMENT_KINDS],
    /// How many notes the chart has (`ScoreData.notes`).
    pub notes: u32,
    /// The longest combo (`ScoreData.combo`).
    pub max_combo: u32,
    /// The bad, poor and miss count, with every note the run never reached added
    /// (`ScoreData.minbp`).
    pub min_bp: u32,
    /// The lamp the run earned, as the reference numbers lamps (`ClearType.id`).
    pub clear: u8,
    /// The kind of play, which decides how the points are weighed.
    pub family: PointFamily,
}

impl ScoreSheet {
    /// The sheet of a finished run: the judge engine's own early and late split, the lamp it was
    /// reported under and the kind of play.
    pub fn of_record(record: &PlayRecord, family: PointFamily, clear: ClearType) -> ScoreSheet {
        ScoreSheet {
            early: record.summary.early,
            late: record.summary.late,
            notes: record.summary.total_notes,
            max_combo: record.summary.max_combo,
            min_bp: record.min_bp_with_unreached(),
            clear: clear_type_id(clear),
            family,
        }
    }

    /// The sheet of a run in progress: what the judge engine has tallied so far, under the lamp the
    /// gauge stands at, for a chart played in `mode`. The bad-poor count has every note the run has
    /// not reached added, as it has when the reference closes the run (`BMSPlayer.java:909`).
    pub fn of_engine(judge: &JudgeEngine, mode: Mode) -> ScoreSheet {
        let [_, _, _, bad, poor, miss] = judge.counts;
        ScoreSheet {
            early: judge.early,
            late: judge.late,
            notes: judge.total_notes(),
            max_combo: judge.max_combo,
            min_bp: bad + poor + miss + judge.total_notes().saturating_sub(judge.total_judged()),
            clear: clear_type_id(judge.clear_lamp()),
            family: PointFamily::of_mode(mode),
        }
    }

    /// How many times one judgement was given on one side, or none for a judgement that does not
    /// exist (`ScoreData.getJudgeCount(judge, fast)`).
    pub fn count_on(&self, judge: i32, early: bool) -> u32 {
        let side = if early { &self.early } else { &self.late };
        usize::try_from(judge).ok().and_then(|judge| side.get(judge)).copied().unwrap_or_default()
    }

    /// How many times one judgement was given in all (`ScoreData.getJudgeCount(judge)`).
    pub fn count(&self, judge: i32) -> u32 {
        self.count_on(judge, true) + self.count_on(judge, false)
    }

    /// The EX score: two for each PGREAT and one for each GREAT (`ScoreData.getExscore`).
    pub fn ex_score(&self) -> u32 {
        self.count(0) * EX_PER_PERFECT + self.count(1) * EX_PER_GREAT
    }

    /// The most EX the chart can give: every note a PGREAT (`ScoreData.notes * 2`).
    pub fn max_ex_score(&self) -> u32 {
        self.notes * EX_PER_PERFECT
    }
}

/// The two scores a run is held against, and how many notes they were set over
/// (`ScoreDataProperty.setTargetScore`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TargetPace {
    /// The best EX score the player had before this run.
    pub best_score: u32,
    /// The EX score of the target.
    pub rival_score: u32,
    /// The notes of the chart those two scores are measured over.
    pub total_notes: u32,
}

/// What `ScoreDataProperty` works out from one sheet and the two scores it is held against.
///
/// The fields are the reference's own members; a figure the reference keeps as a whole number is
/// an `i32` here and one it keeps as a `float` is an `f32`.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreStanding {
    /// The points of the run (`nowpoint`).
    pub now_point: i32,
    /// The EX score so far (`nowscore`).
    pub now_ex: i32,
    /// EX over the whole chart (`rate`), its whole percent and the two digits after the dot.
    pub rate: f32,
    pub rate_int: i32,
    pub rate_after_dot: i32,
    /// EX over the notes gone by (`nowrate`), its whole percent and the two digits after the dot.
    pub now_rate: f32,
    pub now_rate_int: i32,
    pub now_rate_after_dot: i32,
    /// Whether the rate over the whole chart has reached each rank step (`rank`).
    pub rank: [bool; RANK_STEP_COUNT],
    /// Whether the rate over the notes gone by has (`nowrank`).
    pub now_rank: [bool; RANK_STEP_COUNT],
    /// Whether the best score's rate has (`bestrank`).
    pub best_rank: [bool; RANK_STEP_COUNT],
    /// How many more EX points the next rank asks for (`nextrank`).
    pub next_rank: i32,
    /// The best score, its rate, and the rate's whole percent and digits after the dot.
    pub best_score: i32,
    pub best_rate: f32,
    pub best_rate_int: i32,
    pub best_rate_after_dot: i32,
    /// The best score as it would stand by now, had it been earned evenly (`nowbestscore`), and
    /// its rate.
    pub now_best_score: i32,
    pub now_best_rate: f32,
    /// The target's score, its rate, and the rate's whole percent and digits after the dot.
    pub rival_score: i32,
    pub rival_rate: f32,
    pub rival_rate_int: i32,
    pub rival_rate_after_dot: i32,
    /// The target's score as it would stand by now, and its rate.
    pub now_rival_score: i32,
    pub now_rival_rate: f32,
}

/// A rate cut to its whole percent, and to the two digits after the dot.
fn percent_digits(rate: f32) -> (i32, i32) {
    ((rate * PERCENT_SCALE) as i32, ((rate * AFTER_DOT_SCALE) as i32) % AFTER_DOT_MODULUS)
}

/// A score over twice the notes it could have come from, which is a rate from nothing to one.
fn rate_over(score: i32, notes: i32) -> f32 {
    score as f32 / (notes * EX_PER_PERFECT as i32) as f32
}

/// Whether a rate has reached each rank step (`rate >= step / steps`).
fn steps_reached(rate: f32, gated_on_notes: bool) -> [bool; RANK_STEP_COUNT] {
    std::array::from_fn(|step| gated_on_notes && rate >= step as f32 / RANK_STEP_COUNT as f32)
}

impl ScoreStanding {
    /// The standing of `sheet` after `pass_notes` notes have gone by, against `target`.
    ///
    /// `ScoreDataProperty.setTargetScore` then `update(score, notes)`, in that order.
    pub fn of(sheet: &ScoreSheet, pass_notes: u32, target: TargetPace) -> ScoreStanding {
        let ex = sheet.ex_score() as i32;
        let total = sheet.notes as i32;
        let pass = pass_notes as i32;
        let target_total = target.total_notes as i32;
        let best_score = target.best_score as i32;
        let rival_score = target.rival_score as i32;

        let best_rate = rate_over(best_score, target_total);
        let (best_rate_int, best_rate_after_dot) = percent_digits(best_rate);
        let rival_rate = rate_over(rival_score, target_total);
        let (rival_rate_int, rival_rate_after_dot) = percent_digits(rival_rate);

        let rate = if total == 0 { 1.0 } else { rate_over(ex, total) };
        let (rate_int, rate_after_dot) = percent_digits(rate);
        let now_rate = if pass == 0 { 1.0 } else { rate_over(ex, pass) };
        let (now_rate_int, now_rate_after_dot) = percent_digits(now_rate);

        let rank = steps_reached(rate, total != 0);
        let now_rank = steps_reached(now_rate, total != 0);
        let best_rank = steps_reached(best_rate, true);

        let pace = |score: i32| if total == 0 { 0 } else { (i64::from(score) * i64::from(pass) / i64::from(total)) as i32 };
        let now_best_score = pace(best_score);
        let now_rival_score = pace(rival_score);
        let paced_rate = |score: i32| if total == 0 { 0.0 } else { rate_over(score, total) };

        ScoreStanding {
            now_point: Self::points(sheet),
            now_ex: ex,
            rate,
            rate_int,
            rate_after_dot,
            now_rate,
            now_rate_int,
            now_rate_after_dot,
            rank,
            now_rank,
            best_rank,
            next_rank: Self::next_rank(&rank, rate, ex, pass),
            best_score,
            best_rate,
            best_rate_int,
            best_rate_after_dot,
            now_best_score,
            now_best_rate: paced_rate(now_best_score),
            rival_score,
            rival_rate,
            rival_rate_int,
            rival_rate_after_dot,
            now_rival_score,
            now_rival_rate: paced_rate(now_rival_score),
        }
    }

    /// The standing of a run that has not been judged yet, against `target`.
    ///
    /// The reference calls `update(score, notes)` only after a judgement, so before the first one
    /// every figure it derives from the tally is still the zero of a fresh field: no points, no
    /// rate (not the full one a run with no notes gone by gets once it has been updated), no rank
    /// reached and no next rank. The best and the target are set at the start of the play
    /// (`setTargetScore`) and stand from the first frame.
    pub fn before_first_judgement(sheet: &ScoreSheet, target: TargetPace) -> ScoreStanding {
        ScoreStanding {
            now_point: 0,
            now_ex: 0,
            rate: 0.0,
            rate_int: 0,
            rate_after_dot: 0,
            now_rate: 0.0,
            now_rate_int: 0,
            now_rate_after_dot: 0,
            rank: [false; RANK_STEP_COUNT],
            now_rank: [false; RANK_STEP_COUNT],
            next_rank: 0,
            now_best_score: 0,
            now_best_rate: 0.0,
            now_rival_score: 0,
            now_rival_rate: 0.0,
            ..ScoreStanding::of(sheet, 0, target)
        }
    }

    /// The points of the run, weighed for its kind of play (`ScoreDataProperty.java:62-83`).
    fn points(sheet: &ScoreSheet) -> i32 {
        let total = i64::from(sheet.notes);
        if total <= 0 {
            return 0;
        }
        let (perfect, great, good, combo) = sheet.family.weights();
        let weighed = perfect * i64::from(sheet.count(0)) + great * i64::from(sheet.count(1)) + good * i64::from(sheet.count(2));
        (weighed / total) as i32 + (combo * i64::from(sheet.max_combo) / total) as i32
    }

    /// How many EX points short the run is of the next rank's floor, or of a perfect score once
    /// every rank is behind it (`ScoreDataProperty.java:100-108`).
    fn next_rank(rank: &[bool; RANK_STEP_COUNT], rate: f32, ex: i32, pass: i32) -> i32 {
        let span = pass * EX_PER_PERFECT as i32;
        (0..RANK_STEP_COUNT)
            .find(|step| step % RANK_FLOOR_STRIDE == 0 && !rank[*step])
            .map(|step| {
                let floor = f64::from(step as i32 * span) / RANK_STEP_COUNT as f64;
                (floor - f64::from(rate * span as f32)).ceil() as i32
            })
            .unwrap_or(span - ex)
    }

    /// Whether the rate over the notes gone by is in the band of one rank, F as rank 0 up to AAA as
    /// rank 7 (`BooleanPropertyFactory.createNowRank`).
    pub fn now_rank_band(&self, rank: usize) -> bool {
        Self::in_band(&self.now_rank, rank)
    }

    /// Whether the best score's rate is in the band of one rank (`createBestRank`).
    pub fn best_rank_band(&self, rank: usize) -> bool {
        Self::in_band(&self.best_rank, rank)
    }

    /// Whether the rate over the whole chart has secured a rank step (`qualifyRank`).
    pub fn secured(&self, step: usize) -> bool {
        self.rank.get(step).copied().unwrap_or_default()
    }

    fn in_band(steps: &[bool; RANK_STEP_COUNT], rank: usize) -> bool {
        let (Some(low), Some(high)) = (RANK_BAND_FLOORS.get(rank), RANK_BAND_FLOORS.get(rank + 1)) else {
            return false;
        };
        steps.get(*low).copied().unwrap_or_default() && (*high >= RANK_STEP_COUNT || !steps.get(*high).copied().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests;
