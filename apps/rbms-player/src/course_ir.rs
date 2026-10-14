//! Course submissions to the score server: building the submission from a finished run and folding
//! the per-stage block reasons into one course-level verdict.
//!
//! A course reaches the IR only when every stage was a real interactive play: one autoplayed or
//! watched stage keeps the whole course out (`MusicResult.java:82`, applied per stage). A stage
//! played with an assist does not: the course is sent like any other and says which assists its
//! stages had ([`CourseAssist`]), as a single chart's submission does, and the server decides what
//! that costs it (the player's decision of 2026-10-14). A course whose file marks it as not for
//! release is kept out as well (`CourseResult.java:96`).
//!
//! Nothing here implements a server trait: [`rbms_ir`] is the network branch's, and this module only
//! consumes its public types.

use rbms_course::CourseRun;
use rbms_ir::mapping::{assist_flags, ir_clear};
use rbms_ir::{API_VERSION, ChartId, JudgeBreakdown, PlayerId};
use rbms_judge::clear_type_from_id;

/// Reason a course whose file is not marked for release is never submitted
/// (`CourseResult.java:96`). The caller adds it to the per-stage reasons it hands
/// [`course_block_reason`].
pub(crate) const UNRELEASED_COURSE_REASON: &str = "unreleased course";

/// The `lntype` a course reports when its constraint set does not state one. The caller overwrites
/// it with the flavour the run actually played under.
const DEFAULT_LNTYPE: i32 = 0;

/// The assists the stages of a course were played with, taken over the whole course: it had SCRATCH
/// AUTO when any stage did, and was judged wider when any stage was.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CourseAssist {
    /// SCRATCH AUTO played some stage's turntable notes ([`crate::app_result::scratch_auto_played`]).
    pub(crate) scratch_auto: bool,
    /// Some stage was judged wider than its chart asks ([`crate::judge_setup::is_custom_judge`]).
    pub(crate) custom_judge: bool,
}

impl CourseAssist {
    /// A course none of whose stages had an assist, which is where every course starts.
    pub(crate) const NONE: CourseAssist = CourseAssist { scratch_auto: false, custom_judge: false };

    /// The course with one more stage folded in, played with these assists.
    pub(crate) fn with_stage(self, scratch_auto: bool, custom_judge: bool) -> CourseAssist {
        CourseAssist { scratch_auto: self.scratch_auto || scratch_auto, custom_judge: self.custom_judge || custom_judge }
    }

    /// Whether the course may set a best, by the single chart's rule ([`crate::updates_score`]): a
    /// SCRATCH AUTO stage costs it nothing, a stage judged wider costs it the best.
    pub(crate) fn sets_best(self) -> bool {
        !self.custom_judge
    }
}

/// Whether a finished course may set the course best: every stage was one that is sent, and none
/// was judged wider ([`CourseAssist::sets_best`]).
pub(crate) fn course_updates_score(stage_reasons: &[Option<String>], assist: CourseAssist) -> bool {
    course_block_reason(stage_reasons).is_none() && assist.sets_best()
}

/// Turn a finished course run into the submission the score server takes.
///
/// The judgment breakdown carries what a course accumulates — the six judge counts, the empty
/// poors, the fast/slow tallies and the combo breaks. The early/late split and the mean timing
/// error are per-chart analysis figures that a course does not sum, and stay at their defaults.
///
/// `lntype` is read off the course's own long-note constraint; a course that states none reports
/// [`DEFAULT_LNTYPE`] and the caller overwrites it with what the run used.
///
/// `assist` is what the stages were played with, stated the way a single chart's submission states
/// it: SCRATCH AUTO, and the assist tags ([`assist_flags`]). A course with none sends neither field.
pub(crate) fn build_course_submission(run: &CourseRun, player: &str, assist: CourseAssist) -> rbms_ir::dto::CourseSubmission {
    let totals = run.totals;
    let counts = totals.counts;
    let played_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    let mut sub = rbms_ir::dto::CourseSubmission {
        api_version: API_VERSION,
        course_hash: run.course.hash(),
        player: PlayerId { id: player.to_string() },
        clear: ir_clear(clear_type_from_id(run.clear)),
        ex_score: totals.ex,
        judge: JudgeBreakdown {
            pgreat: counts[0],
            great: counts[1],
            good: counts[2],
            bad: counts[3],
            poor: counts[4],
            miss: counts[5],
            fast: totals.fast,
            slow: totals.slow,
            combobreak: totals.combo_breaks,
            empty_poor: totals.empty_poor,
            ..Default::default()
        },
        max_combo: totals.max_combo,
        gauge_value: run.carry_gauge,
        charts: run.course.charts.iter().map(|c| ChartId { md5: c.md5.clone(), sha256: c.sha256.clone() }).collect(),
        played_at,
        lntype: course_lntype(run),
        max_ex_score: totals.max_ex,
        minbp: totals.min_bp(),
        trophy: run.trophy().map(|t| t.name.clone()),
        scratch_auto: assist.scratch_auto,
        assist: assist_flags(assist.scratch_auto, assist.custom_judge),
        extra: Default::default(),
    };
    sub.clamp_to_server_bounds();
    sub
}

/// Fold the per-stage block reasons into one course-level verdict: a course is submittable only
/// when every stage was.
///
/// The caller builds the list from the same predicate a single chart uses, one entry per stage, and
/// adds [`UNRELEASED_COURSE_REASON`] when the course file is not marked for release.
pub(crate) fn course_block_reason(stage_reasons: &[Option<String>]) -> Option<String> {
    stage_reasons.iter().flatten().next().cloned()
}

/// The long-note flavour the course itself demands (0 LN, 1 CN, 2 HCN), or [`DEFAULT_LNTYPE`] when
/// it demands none.
fn course_lntype(run: &CourseRun) -> i32 {
    use rbms_course::CourseConstraint;
    match run.course.constraint_in_group(LN_CONSTRAINT_GROUP) {
        Some(CourseConstraint::Ln) => 0,
        Some(CourseConstraint::Cn) => 1,
        Some(CourseConstraint::Hcn) => 2,
        _ => DEFAULT_LNTYPE,
    }
}

/// Exclusive group the long-note constraints sit in.
const LN_CONSTRAINT_GROUP: u8 = 4;

#[cfg(test)]
mod tests;
