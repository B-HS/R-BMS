//! A whole course taken as one run, for the course result screen of a skin.
//!
//! The reference keeps one score for a course, `resource.getCourseScoreData()`, which every stage's
//! score is added to as the stage ends (`MusicResult.updateScoreDatabase`), and one gauge history
//! per stage that the course result joins end to end (`SkinGaugeGraphObject.prepare`). This is that
//! arithmetic over the records a course's stages left, as a pure function: the screen hands in the
//! course and the records and gets back what the clusters read ([`ResultInput`]) and what the gauge
//! graph plots.
//!
//! A stage the course never reached still counts. Its notes are part of the course's note count, a
//! failed course charges them all as bad-poor, and its gauge history is a flat run of zeroes as
//! long as the stage would have been (`CourseResult.create`), so the graph shows the course falling
//! to the floor and staying there.
//!
//! The reference charges those notes twice. The stage that failed adds every later stage's notes
//! to the course's bad-poor count as its own score is added (`MusicResult.updateScoreDatabase`), and
//! adds the notes of every stage with no gauge history again as its result screen closes
//! (`MusicResult.render`). Both name the same stages, so the miss count a failed course shows is the
//! stages played plus twice the notes never reached. That is kept as it is.

use rbms_course::CourseRun;
use rbms_judge::ClearType;
use rbms_judge::gauge::{GaugeIndex, lamp_for_gauge};
use rbms_judge::{clear_type_from_id, clear_type_id};
use rbms_model::Mode;
use rbms_play::{GAUGE_LOG_INTERVAL_MS, PlayRecord};
use rbms_render::skin_render::frame::{GAUGE_TYPES, GaugeScale};

use super::snapshot::{GaugeEnds, ResultInput, TimingFigures};
use crate::skin_host::score::standing::{PointFamily, ScoreSheet};
use crate::skin_host::system::JUDGEMENT_KINDS;

/// The judge counts the course gives no Good to, and no Great to: with a combo that never broke, the
/// two together decide between a full combo, a perfect and a max (`MusicResult.updateScoreDatabase`).
const GOOD: usize = 2;
const GREAT: usize = 1;

/// How many times a failed course is charged the notes of the stages it never reached: once as the
/// failing stage's score joins the course's, and once more as that stage's result screen closes.
const UNREACHED_STAGE_CHARGES: u32 = 2;

/// The scale of a gauge a course has no record of: a full hundred, cleared at nothing.
const UNKNOWN_SCALE: GaugeScale = GaugeScale::new(0.0, 100.0, 0.0);

/// A stage the course never reached: what is known of its chart without having played it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnplayedStage {
    /// How many notes the chart has (`BMSModel.getTotalNotes`).
    pub notes: u32,
    /// When its last note falls, in milliseconds (`BMSModel.getLastNoteTime`).
    pub last_note_ms: i64,
}

impl UnplayedStage {
    /// How many samples the zeroes standing in for its gauge history run to: one for every
    /// half second up to the last note, and one more (`(lastNoteTime + 500) / 500`).
    pub fn gauge_samples(&self) -> usize {
        usize::try_from((self.last_note_ms + GAUGE_LOG_INTERVAL_MS) / GAUGE_LOG_INTERVAL_MS).unwrap_or_default()
    }
}

/// A course as one run: what the clusters report from, and what the gauge graph plots.
#[derive(Debug, Clone, PartialEq)]
pub struct CourseTally {
    /// The course's score, gauge, and titles. The best it replaced, the target and the settings it
    /// was played with are left as a run with none would have them; the caller sets the ones it
    /// knows.
    pub input: ResultInput,
    /// Every gauge's history with the stages joined end to end, by gauge type.
    pub gauges: Vec<Vec<f32>>,
    /// How many samples there are up to the end of each stage, counted from the start
    /// (`SkinGaugeGraphObject.section`).
    pub sections: Vec<usize>,
    /// The bounds of each gauge as the last stage left them (`resource.getGrooveGauge`).
    pub scales: [GaugeScale; GAUGE_TYPES],
    /// The gauge the last stage ended on, which the screen opens its graph on
    /// (`gaugeType = resource.getGrooveGauge().getType()`).
    pub finished_gauge: usize,
    /// The notes of the whole course, the ones never reached included.
    pub notes: u32,
}

impl CourseTally {
    /// The course `run` as the stages in `records` played it, in order, with `unplayed` standing for
    /// the stages after them.
    pub fn of(run: &CourseRun, records: &[PlayRecord], unplayed: &[UnplayedStage], mode: Mode) -> CourseTally {
        let last = records.last();
        let (gauges, sections) = joined_gauges(records, unplayed);
        let notes = records.iter().map(|record| record.summary.total_notes).sum::<u32>() + unplayed.iter().map(|stage| stage.notes).sum::<u32>();
        let clear = course_lamp(run, last);
        let sheet = course_sheet(run, records, unplayed, mode, notes, clear);
        let input = ResultInput {
            sheet,
            gauge: last.map_or_else(GaugeEnds::default, |record| GaugeEnds::of_log(&record.gauge_log)),
            timing: TimingFigures::default(),
            course_titles: run.course.charts.iter().map(|chart| chart.title.clone()).collect(),
            course_clear: Some(clear_type_id(clear)),
            ..ResultInput::default()
        };
        CourseTally {
            input,
            gauges,
            sections,
            scales: last
                .map_or([UNKNOWN_SCALE; GAUGE_TYPES], |record| record.gauge_bounds.map(|bounds| GaugeScale::new(bounds.min, bounds.max, bounds.border))),
            finished_gauge: last.map_or(0, |record| record.finished_gauge.index()),
            notes,
        }
    }
}

/// The course's score: the judgements of every stage added up, with the stages never reached
/// charged, twice over, to a course that failed.
fn course_sheet(run: &CourseRun, records: &[PlayRecord], unplayed: &[UnplayedStage], mode: Mode, notes: u32, clear: ClearType) -> ScoreSheet {
    let mut early = [0; JUDGEMENT_KINDS];
    let mut late = [0; JUDGEMENT_KINDS];
    for record in records {
        for (total, count) in early.iter_mut().zip(record.summary.early) {
            *total += count;
        }
        for (total, count) in late.iter_mut().zip(record.summary.late) {
            *total += count;
        }
    }
    let reached: u32 = records.iter().map(PlayRecord::min_bp_with_unreached).sum();
    let skipped: u32 = if run.failed_at.is_some() { unplayed.iter().map(|stage| stage.notes).sum() } else { 0 };
    ScoreSheet {
        early,
        late,
        notes,
        max_combo: run.totals.max_combo,
        min_bp: reached + skipped * UNREACHED_STAGE_CHARGES,
        clear: clear_type_id(clear),
        family: PointFamily::of_mode(mode),
    }
}

/// Every gauge's history over the whole course, and where each stage's share of it ends.
fn joined_gauges(records: &[PlayRecord], unplayed: &[UnplayedStage]) -> (Vec<Vec<f32>>, Vec<usize>) {
    let mut gauges = vec![Vec::new(); GAUGE_TYPES];
    let mut sections = Vec::new();
    let mut length = 0;
    for record in records {
        for index in GaugeIndex::ALL {
            gauges[index.index()].extend_from_slice(record.gauge_log.of(index));
        }
        length += record.gauge_log.len();
        sections.push(length);
    }
    for stage in unplayed {
        for history in &mut gauges {
            history.resize(length + stage.gauge_samples(), 0.0);
        }
        length += stage.gauge_samples();
        sections.push(length);
    }
    (gauges, sections)
}

/// The lamp the course earned (`MusicResult.updateScoreDatabase`): failed once a stage failed; the
/// assisted lamp when the last stage was played with an assist; a full combo, a perfect or a max
/// when the combo ran unbroken through every note of the course; and otherwise the lamp of the
/// gauge the last stage ended on.
fn course_lamp(run: &CourseRun, last: Option<&PlayRecord>) -> ClearType {
    if run.failed_at.is_some() {
        return ClearType::Failed;
    }
    let standing = clear_type_from_id(run.clear);
    if matches!(standing, ClearType::AssistEasy | ClearType::LightAssistEasy) {
        return standing;
    }
    let totals = &run.totals;
    if totals.notes > 0 && totals.max_combo == totals.notes {
        return match (totals.counts[GOOD], totals.counts[GREAT]) {
            (0, 0) => ClearType::Max,
            (0, _) => ClearType::Perfect,
            _ => ClearType::FullCombo,
        };
    }
    last.map_or(standing, |record| lamp_for_gauge(record.finished_gauge))
}

#[cfg(test)]
mod tests;
