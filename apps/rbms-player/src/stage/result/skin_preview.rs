//! The finished run the skin preview shows on a result screen and on a course result screen.
//!
//! It lives under the result screen for what only a module here can reach: the gauge and the series
//! a result screen hands a skin beside the run's own figures, and the scene that switches a result
//! skin's timers. The run is a sample one. Nothing is read from the score book, and nothing is
//! saved or sent.
//!
//! Both screens show the sample chart of the play preview played from its first note to its last by
//! the same written-down hand ([`played_sample`]), so the figures are what the engine made of that
//! run and not numbers typed in here: a run that cleared, with nothing recorded before it, no target
//! and no score server. The course is four stages of that chart, each begun on the gauge and the
//! combo the one before it ended with.
//!
//! A result scene runs for as long as its skin says, which for a published skin is until a key is
//! pressed. A preview has no keys, so the scene is given a length of its own ([`SHOWN_MS`]) after
//! which it fades the way a confirmed one does, and begins again.

use rbms_config::Config;
use rbms_course::{Course, CourseChart, CourseRun, StageResult};
use rbms_model::Mode;
use rbms_play::PlaySession;
use rbms_render::skin_render::frame::{GAUGE_TYPES, GaugeScale};
use rbms_render::skin_render::graphs::JUDGEMENTS;
use rbms_render::{FrameData, FrameSeries, GaugeFrame, GaugeHistory};
use rbms_skin::loader::{SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_RESULT};

use super::{ResultRun, Scene, SceneInput, judge_area, switch_score_timers};
use crate::AppShared;
use crate::skin_host::ResultScene;
use crate::skin_host::chart::ChartMeta;
use crate::skin_host::options::{PlayedOptions, random_option_index};
use crate::skin_host::overview::ChartOverview;
use crate::skin_host::result::course::CourseTally;
use crate::skin_host::result::snapshot::{ResultInput, ResultSnapshot};
use crate::skin_host::system::CourseStage;
use crate::skin_preview::SampleStep;
use crate::skin_screen::ResultDraw;
use crate::skin_screen::preview::{PreviewDrawn, PreviewFrame, PreviewScene};
use crate::stage::Canvas;
use crate::stage::play::skin_preview::{Carried, played_sample};
use crate::stage::scene_life::SceneTimes;

/// The mode of the run both result screens show.
const SAMPLE_MODE: Mode = Mode::BEAT_7K;

/// How long a result scene is on show before it fades, in milliseconds.
const SHOWN_MS: i64 = 15_000;

/// What the sample course is called and what its stages are, and the gauge its first stage begins
/// on.
const COURSE_NAME: &str = "SAMPLE COURSE";
const COURSE_STAGE_TITLE: &str = "SAMPLE STAGE";
const COURSE_STAGES: usize = 4;
const COURSE_OPENING_GAUGE: f32 = 100.0;

/// The times a result scene of the preview runs on: the skin's own, with the preview's length in
/// place of the skin's.
fn scene_times(shared: &AppShared, screen: i32) -> SceneTimes {
    SceneTimes { scene_ms: SHOWN_MS, ..SceneTimes::of_skin(shared.skins.document(screen)) }
}

/// One frame of a result scene on the preview's own time: begun on the first frame its skin can be
/// drawn on, with the three score timers switched on as the screen switches them, and over once its
/// fade has run.
fn step_scene(scene: &mut Scene, shared: &mut AppShared, screen: i32, now_us: i64, ready: bool) -> SampleStep {
    if !ready {
        return SampleStep::Running;
    }
    if scene.begin(scene_times(shared, screen), SAMPLE_MODE) {
        switch_score_timers(&mut shared.skin_timers, now_us);
    }
    match scene.step(&mut shared.skin_timers, now_us, SceneInput::default()) {
        Some(_) => SampleStep::Over,
        None => SampleStep::Running,
    }
}

/// The settings a sample run is reported as played with: the player's own random option and nothing
/// else.
fn played_options(config: &Config) -> PlayedOptions {
    PlayedOptions { random: random_option_index(config.play.random), ..PlayedOptions::default() }
}

/// A sample run on the result screen.
pub(crate) struct ResultSample {
    run: Box<ResultRun>,
    /// How many notes took each judgement over the whole run.
    judged: [u32; JUDGEMENTS],
    scene: Scene,
}

impl ResultSample {
    /// The sample chart played through, as the result screen keeps a run. `None` only if the sample
    /// chart cannot be written for the mode, which it can.
    pub(crate) fn new(config: &Config) -> Option<ResultSample> {
        let session = played_sample(SAMPLE_MODE, config, None)?;
        let record = session.record();
        let chart = ChartOverview::of_model(session.model());
        let snapshot = ResultSnapshot::of(ResultInput {
            options: played_options(config),
            favorite_chart: Some(false),
            ..ResultInput::from_record(&record, SAMPLE_MODE, record.summary.clear_lamp)
        });
        let run = ResultRun { judge_area: judge_area(&session), updates_score: true, ..ResultRun::of_record(chart, &record, snapshot) };
        let mut scene = Scene::new();
        scene.gauge_type = run.finished_gauge;
        Some(ResultSample { run: Box::new(run), judged: record.summary.counts, scene })
    }

    /// Move the scene on to `now_us`. `ready` is whether the skin that draws it can be drawn.
    pub(crate) fn step(&mut self, shared: &mut AppShared, now_us: i64, ready: bool) -> SampleStep {
        step_scene(&mut self.scene, shared, SKIN_TYPE_RESULT, now_us, ready)
    }

    /// Draw one frame of the result into `at.rect`, from what the result screen hands its own skin:
    /// the chart, the gauge the graph is on, the run's series and the run itself.
    pub(crate) fn draw(&self, shared: &AppShared, canvas: &mut Canvas<'_>, at: PreviewFrame) -> PreviewDrawn {
        let chart = self.run.chart.meta(false);
        let gauge_type = self.scene.gauge_type;
        let data = FrameData { gauge: Some(self.run.gauge(gauge_type)), series: self.run.series(&self.judged), ..FrameData::default() };
        let scene = ResultScene { gauge_type, ..ResultScene::default() };
        let draw = ResultDraw { chart: Some(&chart), scene, run: Some(&self.run.snapshot), data };
        shared.draw_skin_preview(canvas, at, PreviewScene::Result { draw, course: None })
    }
}

/// What one stage of the sample course came to, as a course is told of it.
fn stage_result(session: &PlaySession) -> StageResult {
    let summary = session.summary();
    StageResult {
        ex_score: summary.ex_score,
        max_ex_score: summary.max_ex_score,
        notes: summary.total_notes,
        counts: summary.counts,
        empty_poor: summary.empty_poor,
        fast: summary.fast,
        slow: summary.slow,
        combo_breaks: crate::combo_breaks(&session.model().mode, summary.counts),
        max_combo: summary.max_combo,
        combo_at_end: session.standing_combo(),
        gauge_value: summary.gauge_value,
        clear: rbms_judge::clear_type_id(summary.clear_lamp),
        survived: !summary.failed,
    }
}

/// A sample course on the course result screen: the course taken as one run.
pub(crate) struct CourseSample {
    snapshot: ResultSnapshot,
    gauges: Vec<Vec<f32>>,
    sections: Vec<usize>,
    scales: [GaugeScale; GAUGE_TYPES],
    /// The notes of every stage together.
    notes: i32,
    scene: Scene,
}

impl CourseSample {
    /// A course of [`COURSE_STAGES`] stages of the sample chart, each played through on what the one
    /// before it left, and added up the way the course result adds a course up.
    pub(crate) fn new(config: &Config) -> Option<CourseSample> {
        let mut records = Vec::with_capacity(COURSE_STAGES);
        let mut run: Option<CourseRun> = None;
        for _ in 0..COURSE_STAGES {
            let carried =
                run.as_ref().map_or(Carried { gauge: COURSE_OPENING_GAUGE, combo: 0 }, |run| Carried { gauge: run.carry_gauge, combo: run.totals.combo_carry });
            let session = played_sample(SAMPLE_MODE, config, Some(carried))?;
            let course = run.get_or_insert_with(|| CourseRun::new(sample_course(&session), COURSE_OPENING_GAUGE));
            course.advance(&stage_result(&session));
            records.push(session.record());
        }
        let run = run?;
        let mut tally = CourseTally::of(&run, &records, &[], SAMPLE_MODE);
        tally.input.options = played_options(config);
        let mut scene = Scene::of_course();
        scene.gauge_type = tally.finished_gauge;
        Some(CourseSample {
            snapshot: ResultSnapshot::of(tally.input),
            gauges: tally.gauges,
            sections: tally.sections,
            scales: tally.scales,
            notes: i32::try_from(tally.notes).unwrap_or(i32::MAX),
            scene,
        })
    }

    /// Move the scene on to `now_us`. `ready` is whether the skin that draws it can be drawn.
    pub(crate) fn step(&mut self, shared: &mut AppShared, now_us: i64, ready: bool) -> SampleStep {
        step_scene(&mut self.scene, shared, SKIN_TYPE_COURSE_RESULT, now_us, ready)
    }

    /// Draw one frame of the course result into `at.rect`, from what the course result screen hands
    /// its own skin: the course named as a chart, the gauge its last stage ended on, the stages'
    /// gauge histories end to end, and the course as one run. The skin is told the course's last
    /// stage is the one in hand, as it is on the screen itself.
    pub(crate) fn draw(&self, shared: &AppShared, canvas: &mut Canvas<'_>, at: PreviewFrame) -> PreviewDrawn {
        let chart = ChartMeta { title: COURSE_NAME, heading: Some(COURSE_NAME), mode: Some(SAMPLE_MODE), notes: Some(self.notes), ..ChartMeta::default() };
        let gauge_type = self.scene.gauge_type;
        let end = self.gauges.get(gauge_type).and_then(|history| history.last()).copied().unwrap_or_default();
        let data = FrameData {
            gauge: Some(GaugeFrame::finished(gauge_type, end, self.scales)),
            series: FrameSeries { gauge_history: Some(GaugeHistory::of_kinds(&self.gauges).with_sections(&self.sections)), ..FrameSeries::default() },
            ..FrameData::default()
        };
        let scene = ResultScene { gauge_type, ..ResultScene::default() };
        let draw = ResultDraw { chart: Some(&chart), scene, run: Some(&self.snapshot), data };
        let stage = CourseStage { index: COURSE_STAGES - 1, count: COURSE_STAGES };
        shared.draw_skin_preview(canvas, at, PreviewScene::Result { draw, course: Some(stage) })
    }
}

/// The sample course: [`COURSE_STAGES`] stages of the chart `session` played, each under a title of
/// its own.
fn sample_course(session: &PlaySession) -> Course {
    let model = session.model();
    let charts = (1..=COURSE_STAGES)
        .map(|number| CourseChart { md5: model.md5.clone(), sha256: model.sha256.clone(), title: format!("{COURSE_STAGE_TITLE} {number}") })
        .collect();
    Course { name: COURSE_NAME.to_owned(), charts, ..Course::default() }
}
