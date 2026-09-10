//! Course progress: what one stage produced, what the whole run has accumulated, and the decision
//! taken at each stage boundary.
//!
//! A course is one continuous play. The gauge and the combo carry from stage to stage rather than
//! restarting, the judgment tallies accumulate, and the run ends the moment a stage does not
//! survive — the stages after a failure are never played.
//!
//! Nothing here knows how a chart is judged: a finished stage arrives as a [`StageResult`] of plain
//! numbers, which is what keeps this crate independent of the play and judge engines.

use crate::model::{Course, CourseChart, TrophyRule};

/// Clear-lamp id of a course that has not been played (`ClearType.java:42-49`, id 0).
pub const CLEAR_NO_PLAY: u8 = 0;

/// Clear-lamp id of a course that ran out of gauge (`ClearType.java:42-49`, id 1).
pub const CLEAR_FAILED: u8 = 1;

/// Index of the first judgment in [`StageResult::counts`] that counts towards the bad-poor total.
const FIRST_BP_JUDGE: usize = 3;

/// Percent scale of the rates a course is graded on.
const PERCENT: f64 = 100.0;

/// Theoretical EX score per note (`GradeBar.java:88`, `notes * 2`).
const EX_PER_NOTE: u32 = 2;

/// What one finished stage of a course contributed.
///
/// The fields mirror what a finished run reports, so the integration layer fills this in from a
/// play summary without reinterpreting anything. `counts` is in judge order (perfect great, great,
/// good, bad, poor, miss).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StageResult {
    pub ex_score: u32,
    pub max_ex_score: u32,
    /// Notes the chart holds, which is the denominator both course rates are taken over.
    pub notes: u32,
    pub counts: [u32; 6],
    pub empty_poor: u32,
    pub fast: u32,
    pub slow: u32,
    /// Judgments that actually broke the combo, which depends on the chart's mode and so is decided
    /// by the caller rather than derived from `counts` here.
    pub combo_breaks: u32,
    /// Longest combo within this stage, already counting a streak carried in from the stage before.
    pub max_combo: u32,
    /// Combo still standing when the stage ended, which the next stage starts from.
    pub combo_at_end: u32,
    /// Gauge left when the stage ended, which the next stage starts from.
    pub gauge_value: f32,
    /// Clear-lamp id this stage reached (`ClearType.java:42-49`).
    pub clear: u8,
    /// Whether the gauge held out. A stage that did not survive ends the course.
    pub survived: bool,
}

/// Everything the stages so far have added up to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CourseTotals {
    pub ex: u32,
    pub max_ex: u32,
    pub notes: u32,
    pub counts: [u32; 6],
    pub empty_poor: u32,
    pub fast: u32,
    pub slow: u32,
    pub combo_breaks: u32,
    /// Longest combo anywhere in the course, including one spanning a stage boundary.
    pub max_combo: u32,
    /// Combo standing at the last stage boundary, which the next stage is seeded with.
    pub combo_carry: u32,
}

impl CourseTotals {
    /// Bad plus poor plus miss, the count a trophy's miss rate is taken over
    /// (`PlaySummary::min_bp`, `GradeBar.java:87`).
    pub fn min_bp(&self) -> u32 {
        self.counts[FIRST_BP_JUDGE..].iter().sum()
    }

    /// Miss rate over the course, in percent (`GradeBar.java:87`). Zero notes reads as zero.
    pub fn miss_rate(&self) -> f64 {
        if self.notes == 0 {
            return 0.0;
        }
        f64::from(self.min_bp()) * PERCENT / f64::from(self.notes)
    }

    /// Score rate over the course, in percent (`GradeBar.java:88`). Zero notes reads as zero.
    pub fn score_rate(&self) -> f64 {
        if self.notes == 0 {
            return 0.0;
        }
        f64::from(self.ex) * PERCENT / f64::from(self.notes * EX_PER_NOTE)
    }

    /// Fold one finished stage in.
    fn add(&mut self, stage: &StageResult) {
        self.ex += stage.ex_score;
        self.max_ex += stage.max_ex_score;
        self.notes += stage.notes;
        for (total, count) in self.counts.iter_mut().zip(stage.counts) {
            *total += count;
        }
        self.empty_poor += stage.empty_poor;
        self.fast += stage.fast;
        self.slow += stage.slow;
        self.combo_breaks += stage.combo_breaks;
        self.max_combo = self.max_combo.max(stage.max_combo);
        self.combo_carry = stage.combo_at_end;
    }
}

/// What a stage boundary decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CourseStep {
    /// Load the next stage.
    Next,
    /// Every stage was played and survived.
    Cleared,
    /// This stage ran out of gauge; the stages after it are not played.
    Failed,
}

/// A course being played: which stage is next, what carries into it, and what the run has scored.
#[derive(Clone, Debug)]
pub struct CourseRun {
    pub course: Course,
    /// Stage about to be played, or the stage count once the course is over.
    pub index: usize,
    /// Gauge the next stage starts from.
    pub carry_gauge: f32,
    pub totals: CourseTotals,
    /// Stage the run failed on, if it did.
    pub failed_at: Option<usize>,
    /// Clear-lamp id the course stands at (`ClearType.java:42-49`). A course is judged on one
    /// continuous gauge, so this is the lamp the last stage played reached, and [`CLEAR_FAILED`]
    /// once a stage has failed.
    pub clear: u8,
}

impl CourseRun {
    /// Start `course` with the gauge its first stage begins on.
    pub fn new(course: Course, initial_gauge: f32) -> CourseRun {
        CourseRun { course, index: 0, carry_gauge: initial_gauge, totals: CourseTotals::default(), failed_at: None, clear: CLEAR_NO_PLAY }
    }

    /// The chart the next stage plays, or `None` once the course is over.
    pub fn current_chart(&self) -> Option<&CourseChart> {
        self.course.charts.get(self.index)
    }

    /// Whether the run has nothing left to play, either because it failed or because every stage is
    /// behind it.
    pub fn is_finished(&self) -> bool {
        self.failed_at.is_some() || self.index >= self.course.stage_count()
    }

    /// One-based number of the stage about to be played, for the progress line.
    pub fn stage_number(&self) -> usize {
        (self.index + 1).min(self.course.stage_count().max(1))
    }

    /// Take one finished stage and say what happens next.
    ///
    /// The stage's tallies are folded in first, so a failed stage still counts towards the totals
    /// the result screen shows. A run that is already over is left untouched and reports the state
    /// it is already in.
    pub fn advance(&mut self, stage: &StageResult) -> CourseStep {
        if self.is_finished() {
            return self.finished_step();
        }
        self.totals.add(stage);
        self.carry_gauge = stage.gauge_value;
        self.clear = stage.clear;
        if !stage.survived {
            self.failed_at = Some(self.index);
            self.clear = CLEAR_FAILED;
            return CourseStep::Failed;
        }
        self.index += 1;
        self.finished_step_or_next()
    }

    /// The highest trophy this run qualifies for (`GradeBar.java:70-88`).
    ///
    /// Trophies are declared easiest first, so the search runs from the last one back and takes the
    /// first that both rates satisfy.
    pub fn trophy(&self) -> Option<&TrophyRule> {
        let notes = self.totals.notes;
        let min_bp = self.totals.min_bp();
        let ex = self.totals.ex;
        self.course.trophies.iter().rev().find(|rule| rule.qualifies(notes, min_bp, ex))
    }

    /// The step a run that is over reports.
    fn finished_step(&self) -> CourseStep {
        if self.failed_at.is_some() { CourseStep::Failed } else { CourseStep::Cleared }
    }

    /// Whether another stage remains after the index has moved on.
    fn finished_step_or_next(&self) -> CourseStep {
        if self.index >= self.course.stage_count() { CourseStep::Cleared } else { CourseStep::Next }
    }
}
