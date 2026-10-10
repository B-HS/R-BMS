//! What the course result screen keeps of the course for a skin to draw: the course taken as one
//! run, in the shapes a skin's frame borrows.
//!
//! It is made once when the course ends, from the stages' records the application kept as they
//! ended ([`AppShared::run_records`]) and from the charts of the stages the course never reached,
//! which it reads off the library so their notes and their length are charged to the course the way
//! the reference charges them (`MusicResult.render`, `CourseResult.create`).

use std::path::Path;

use rbms_course::{CourseChart, CourseRun};
use rbms_judge::{ClearType, clear_type_from_id};
use rbms_model::Mode;
use rbms_play::last_note_time_ms;
use rbms_render::result::{ResultExtras, ResultView};
use rbms_render::skin_render::frame::{GAUGE_TYPES, GaugeScale};
use rbms_render::{FrameSeries, GaugeFrame, GaugeHistory};

use crate::AppShared;
use crate::app_play::chart_model;
use crate::course_ui::library_index;
use crate::format::clear_label_color;
use crate::keyconfig::mode_config_key;
use crate::skin_host::chart::ChartMeta;
use crate::skin_host::options::{PlayedOptions, random_option_index};
use crate::skin_host::result::course::{CourseTally, UnplayedStage};
use crate::skin_host::result::snapshot::ResultSnapshot;

/// The course taken as one run.
pub(super) struct CourseSkinRun {
    /// What the clusters report from.
    pub(super) snapshot: ResultSnapshot,
    /// The summary the screen was drawn from before the clusters existed, which still answers the
    /// ids no cluster knows.
    pub(super) view: ResultView,
    pub(super) extras: ResultExtras,
    gauges: Vec<Vec<f32>>,
    sections: Vec<usize>,
    scales: [GaugeScale; GAUGE_TYPES],
    finished_gauge: usize,
    title: String,
    notes: i32,
    mode: Mode,
}

impl CourseSkinRun {
    /// Read the course `run` has just finished, against the records its stages left in `shared`.
    pub(super) fn of(run: &CourseRun, shared: &AppShared) -> CourseSkinRun {
        let records = &shared.run_records;
        let unplayed: Vec<UnplayedStage> = run.course.charts.iter().skip(records.len()).map(|chart| unplayed_stage(shared, chart)).collect();
        let mut tally = CourseTally::of(run, records, &unplayed, shared.mode);
        tally.input.updates_score = shared.course_stage_reasons.iter().all(Option::is_none);
        tally.input.options = PlayedOptions { random: random_option_index(shared.config.play.random), ..PlayedOptions::default() };
        let lamp = clear_type_from_id(tally.input.sheet.clear);
        let view = summary_view(run, &tally, lamp, shared.mode);
        CourseSkinRun {
            snapshot: ResultSnapshot::of(tally.input),
            view,
            extras: ResultExtras::default(),
            gauges: tally.gauges,
            sections: tally.sections,
            scales: tally.scales,
            finished_gauge: tally.finished_gauge,
            title: run.course.name.clone(),
            notes: i32::try_from(tally.notes).unwrap_or(i32::MAX),
            mode: shared.mode,
        }
    }

    /// The gauge the graph opens on.
    pub(super) fn finished_gauge(&self) -> usize {
        self.finished_gauge
    }

    /// The gauge numbered `gauge_type` as the course left it, for the gauge bar: the last value of
    /// the course's last stage, which for a stage the course never reached is the floor it was
    /// filled with (`SkinGauge.prepare` reads the end of `getCourseGauge().get(size - 1)`). The gauge
    /// number beside the bar is not this: it reads where the last stage played ended.
    pub(super) fn gauge(&self, gauge_type: usize) -> GaugeFrame {
        let end = self.gauges.get(gauge_type).and_then(|history| history.last()).copied().unwrap_or_default();
        GaugeFrame::finished(gauge_type, end, self.scales)
    }

    /// The series the graphs plot: the stages' gauge histories end to end, with a line where each
    /// stage ends. The timing graph is the single result's alone (`SkinTimingDistributionGraph`
    /// draws on `MusicResult` only), so it is left empty here.
    pub(super) fn series(&self) -> FrameSeries<'_> {
        FrameSeries { gauge_history: Some(GaugeHistory::of_kinds(&self.gauges).with_sections(&self.sections)), ..FrameSeries::default() }
    }

    /// The course as the chart cluster reads a chart: it is named by the course, and has the notes
    /// of every stage.
    pub(super) fn chart(&self) -> ChartMeta<'_> {
        ChartMeta { title: &self.title, heading: Some(&self.title), mode: Some(self.mode), notes: Some(self.notes), ..ChartMeta::default() }
    }
}

/// The summary the screen carried before the clusters existed, taken from the course's totals.
fn summary_view(run: &CourseRun, tally: &CourseTally, lamp: ClearType, mode: Mode) -> ResultView {
    let totals = &run.totals;
    let (clear_label, clear_color) = clear_label_color(lamp);
    ResultView {
        title: run.course.name.clone(),
        artist: String::new(),
        mode_label: mode_config_key(mode),
        counts: totals.counts,
        ex_score: totals.ex,
        max_score: totals.max_ex,
        max_combo: totals.max_combo,
        total_notes: tally.notes,
        fast: [totals.fast, 0],
        slow: [totals.slow, 0],
        gauge: run.carry_gauge,
        clear_label,
        clear_color,
        prev_best_ex: None,
        prev_ex: None,
        show_graph: false,
        show_result_graphs: false,
        gauge_series: Vec::new(),
        timing_hist: Box::default(),
        judge_dist: totals.counts,
    }
}

/// What the library's file for a stage of the course says of its chart: how many notes it has and
/// when the last of them falls. A stage whose chart cannot be found or read counts for nothing.
fn unplayed_stage(shared: &AppShared, chart: &CourseChart) -> UnplayedStage {
    let model = library_index(&shared.library, chart).and_then(|index| shared.library.songs().get(index)).and_then(|entry| chart_model_of(&entry.path));
    let Some(model) = model else {
        return UnplayedStage::default();
    };
    UnplayedStage { notes: u32::try_from(rbms_chart::count_playable_notes(&model)).unwrap_or(u32::MAX), last_note_ms: last_note_time_ms(&model) }
}

/// The play model of the chart in the file at `path`.
fn chart_model_of(path: &Path) -> Option<rbms_model::Model> {
    let bytes = std::fs::read(path).ok()?;
    chart_model(&bytes, &path.to_string_lossy())
}
