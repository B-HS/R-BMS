//! The RESULT screen: the summary of the run that just ended, plus what the score server said
//! about it.
//!
//! Drawn by a skin it is the reference's `MusicResult`, a scene with a length
//! ([`crate::stage::scene_life`]). Its clock starts on the frame the skin first draws, and on that
//! frame the three score timers go on together. The scene takes no key until the skin's `input`
//! time has passed. A confirming key then starts the fade -- unless the score server has not
//! answered yet, which holds the screen open -- and the screen leaves once the fade has run for
//! the skin's `fadeout` time. Which way it leaves is read then: for another run of the same chart
//! while one of the two run-again lane keys is still down, and for the browser otherwise.
//!
//! The lane keys are read by the reference's key index (`ResultKeyProperty`): the first four
//! confirm, the fifth runs the chart again on a new arrangement, the sixth turns the gauge the
//! graph plots to the next kind, and the seventh runs it again as it was laid out. R and N are this
//! player's own: R is the seventh key's run-again and N starts the chart after this one, both
//! decided when the key is pressed, behind the same input time and the same fade.
//!
//! The skin's files are read off the frame loop, so the screen is up for a few frames before its
//! skin can be drawn. Those frames are black and take no key but Escape, which leaves at once:
//! there is no scene yet to fade out. A skin that turns out not to load hands the screen to the
//! built-in layout.
//!
//! Without a skin the screen is the built-in one, as it always was: every key acts at once.
//!
//! A skin written for the reference offers four replay slots to save into. A run here is saved by
//! itself when it ends, so the save events do nothing, and the first slot reports what became of
//! this run's own replay ([`ReplaySlot`]).

use std::path::Path;

use rbms_judge::ClearType;
use rbms_judge::windows::JudgeWindowSet;
use rbms_model::Mode;
use rbms_play::{PlayRecord, PlaySession};
use rbms_render::result::{ResultExtras, TargetView};
use rbms_render::skin_render::frame::{GAUGE_TYPES, GaugeScale};
use rbms_render::skin_render::graphs::{EARLY_LATE_BUCKETS, JUDGEMENTS, TIMING_JUDGE_AREAS};
use rbms_render::{BpmTimeline, FrameData, FrameSeries, GaugeFrame, GaugeHistory, NoteDistribution, ReferenceImages, TimingHistogram};
use rbms_skin::property::generated::{BUTTON_FAVORITTE_CHART, BUTTON_REPLAY, BUTTON_REPLAY2, BUTTON_REPLAY3, BUTTON_REPLAY4, RATE_RANKING_POSITION};
use rbms_skin::timer::{MICROS_PER_MILLI, TimerState, timer_id};
use winit::keyboard::KeyCode;

use crate::app_result::{following_song, next_song, offers_retry, retry};
use crate::ir_ranking::RankingState;
use crate::keyconfig::KEY_INDEX_COUNT;
use crate::skin_host::options::{PlayedOptions, random_option_index};
use crate::skin_host::overview::ChartOverview;
use crate::skin_host::result::snapshot::{PreviousScore, REPLAY_SLOT_COUNT, ReplaySlot, ResultInput, ResultSnapshot};
use crate::skin_host::{Cluster, ClusterRequest, ResultScene as SceneFacts};
use crate::skin_screen::ResultDraw;
use crate::stage::scene_life::{ScenePhase, SceneTimes, advance, begin_fadeout, takes_input};
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::syssound::{SystemSound, result_sound};
use crate::{
    AppShared, CW, Color, DecodedImage, IR_RESULT_LINE_H, IR_RESULT_SCALE, IR_RESULT_X, IR_RESULT_Y, IrStatus, Rect, Renderer, ResultView, SKIN_TYPE_RESULT,
    decode_bga_image, draw_text, draw_text_right, ir_line_color, is_custom_judge, render_result_with_palette, text_width, updates_score,
};

/// What a lane key does on the result screen (`ResultKeyProperty.ResultKey`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultKey {
    /// Confirm, which closes the screen.
    Ok,
    /// Run the chart again on a new arrangement.
    ReplayDifferent,
    /// Run the chart again on the arrangement it was just played on.
    ReplaySame,
    /// Turn the gauge the graph plots to the next kind.
    ChangeGraph,
}

/// What each key index does in the modes with a turntable (`ResultKeyProperty.BEAT_7K`): the
/// turntable does nothing.
const BEAT_KEYS: [Option<ResultKey>; KEY_INDEX_COUNT] = [
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
    Some(ResultKey::ReplayDifferent),
    Some(ResultKey::ChangeGraph),
    Some(ResultKey::ReplaySame),
    None,
    None,
];

/// The same for the nine buttons (`ResultKeyProperty.POPN_9K`), whose last two confirm.
const POPN_KEYS: [Option<ResultKey>; KEY_INDEX_COUNT] = [
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
    Some(ResultKey::ReplayDifferent),
    Some(ResultKey::ChangeGraph),
    Some(ResultKey::ReplaySame),
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
];

/// The same for the keyboard layout (`ResultKeyProperty.KEYBOARD_24K`), of which only the first
/// seven keys have an index here.
const KEYBOARD_KEYS: [Option<ResultKey>; KEY_INDEX_COUNT] = [
    Some(ResultKey::Ok),
    Some(ResultKey::ReplayDifferent),
    Some(ResultKey::ChangeGraph),
    Some(ResultKey::ReplaySame),
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
    Some(ResultKey::Ok),
    None,
    None,
];

/// What each key index does in `mode` (`ResultKeyProperty.get`).
fn key_assignment(mode: Mode) -> [Option<ResultKey>; KEY_INDEX_COUNT] {
    if mode == Mode::POPN_9K {
        POPN_KEYS
    } else if mode == Mode::KEYBOARD_24K {
        KEYBOARD_KEYS
    } else {
        BEAT_KEYS
    }
}

/// The last of the gauges a single chart is played on (`GrooveGauge.HAZARD`), how many of them
/// there are, the first of the gauges a course is played on (`GrooveGauge.CLASS`) and how many of
/// those there are.
const LAST_CHART_GAUGE: usize = 5;
const CHART_GAUGES: usize = 6;
const FIRST_COURSE_GAUGE: usize = 6;
const COURSE_GAUGES: usize = 3;

/// The gauge the graph turns to from `gauge_type` (`MusicResult.input`): round the six a chart is
/// played on, or round the three a course is.
fn next_gauge_type(gauge_type: usize) -> usize {
    if gauge_type <= LAST_CHART_GAUGE { (gauge_type + 1) % CHART_GAUGES } else { (gauge_type - LAST_CHART_GAUGE) % COURSE_GAUGES + FIRST_COURSE_GAUGE }
}

/// The gauge the graph turns to on a course result (`CourseResult.input`), which has the one
/// expression `(g - 5) % 3 + 6` in the reference's signed arithmetic. It cycles the three course
/// gauges, and a gauge of a chart sends it to one of the upper ones rather than the first (3 goes to
/// 4, not 6).
pub(crate) fn next_course_gauge_type(gauge_type: usize) -> usize {
    let turned = (gauge_type as i32 - LAST_CHART_GAUGE as i32) % COURSE_GAUGES as i32 + FIRST_COURSE_GAUGE as i32;
    usize::try_from(turned).unwrap_or_default()
}

/// The gauge a screen with no record of its run is taken to show: the normal one.
const DEFAULT_GAUGE_TYPE: usize = 2;

/// Which way a scene that is over leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leave {
    /// Back to the browser.
    Browser,
    /// The same chart again, laid out as it was.
    RunAgain,
    /// The same chart again, laid out afresh.
    RunAfresh,
    /// The chart after this one in the browser's list.
    FollowOn,
}

/// What the scene reads from outside itself once a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SceneInput {
    /// Which key indices are down.
    pub(crate) held: [bool; KEY_INDEX_COUNT],
    /// The score server has been sent the run and has not answered (`STATE_IR_PROCESSING`).
    pub(crate) submitting: bool,
    /// The run can be started again.
    pub(crate) can_run_again: bool,
    /// There is a chart after this one to start.
    pub(crate) can_follow_on: bool,
}

impl SceneInput {
    /// What a frame of the single chart's result reads: the keys, the submission, and where the
    /// screen was found to lead when it was entered.
    fn read(shared: &AppShared, ways: WaysOn) -> SceneInput {
        SceneInput {
            held: held_key_indices(shared),
            submitting: matches!(shared.ir_status, IrStatus::Sending) && (shared.submit_rx.is_some() || shared.profile_submit_rx.is_some()),
            can_run_again: ways.run_again,
            can_follow_on: ways.follow_on,
        }
    }
}

/// Which key indices are down right now.
pub(crate) fn held_key_indices(shared: &AppShared) -> [bool; KEY_INDEX_COUNT] {
    std::array::from_fn(|index| shared.key_index_pressed(index))
}

/// Where a result screen can lead besides the browser, settled once as the screen is entered.
///
/// Finding the chart that was played in the library, and the chart after it in the browser's list,
/// is a walk of both. Neither moves while the result is up, so the scene does not walk them again on
/// every frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct WaysOn {
    /// The run can be started again.
    run_again: bool,
    /// There is a chart after this one to start.
    follow_on: bool,
}

impl WaysOn {
    fn of(shared: &AppShared) -> WaysOn {
        let run_again = offers_retry(shared);
        WaysOn { run_again, follow_on: run_again && following_song(shared).is_some() }
    }
}

/// Switch the three score timers on, which the reference does on every frame of the screen
/// (`MusicResult.render`): a skin read as data or as Lua states no rank time, so the score timer
/// goes on with the other two.
pub(crate) fn switch_score_timers(timers: &mut TimerState, now_us: i64) {
    for timer in [timer_id::RESULTGRAPH_BEGIN, timer_id::RESULTGRAPH_END, timer_id::RESULT_UPDATESCORE] {
        timers.switch(timer, true, now_us);
    }
}

/// The scene itself: its times, whether it has begun, the gauge on show and what the player has
/// asked of it.
///
/// It holds no clock and no timers of its own. Both are handed in, so a frame of it is the same
/// whether the application or a test is the one counting the time.
#[derive(Debug)]
pub(crate) struct Scene {
    times: SceneTimes,
    pub(crate) begun: bool,
    assign: [Option<ResultKey>; KEY_INDEX_COUNT],
    /// Which key indices were down on the last frame.
    was_down: [bool; KEY_INDEX_COUNT],
    /// Which key indices have gone down or come up since a frame last acted on them. The reference
    /// reads a lane key this way (`getKeyState` with `resetKeyChangedTime`): a key held from before
    /// the scene listens acts the moment it does, and a key that stays down acts once.
    changed: [bool; KEY_INDEX_COUNT],
    /// Enter was pressed and is still down, and nothing has acted on that press yet
    /// (`KeyBoardInputProcesseor.isKeyPressed`).
    enter: bool,
    /// The same for Escape.
    escape: bool,
    /// The same for R, this player's run-again key.
    run_again: bool,
    /// The same for N, this player's next-chart key.
    follow_on: bool,
    /// Where R or N said the scene leaves for, which the fade does not change.
    chosen: Option<Leave>,
    /// The gauge the graph and the gauge bar show (`AbstractResult.gaugeType`).
    pub(crate) gauge_type: usize,
    /// Where the key that turns the graph sends `gauge_type`, which a chart and a course word
    /// differently.
    next_gauge: fn(usize) -> usize,
}

impl Scene {
    fn new() -> Scene {
        Scene::turning_gauge_by(next_gauge_type)
    }

    /// The scene of a course result, whose graph key turns the gauge by the course's own
    /// expression.
    pub(crate) fn of_course() -> Scene {
        Scene::turning_gauge_by(next_course_gauge_type)
    }

    fn turning_gauge_by(next_gauge: fn(usize) -> usize) -> Scene {
        Scene {
            times: SceneTimes::default(),
            begun: false,
            assign: BEAT_KEYS,
            was_down: [false; KEY_INDEX_COUNT],
            changed: [false; KEY_INDEX_COUNT],
            enter: false,
            escape: false,
            run_again: false,
            follow_on: false,
            chosen: None,
            gauge_type: DEFAULT_GAUGE_TYPE,
            next_gauge,
        }
    }

    /// Begin the scene with the times its skin states and the keys of the mode that was played.
    /// `true` the one time it does: a scene that has begun is not begun again.
    pub(crate) fn begin(&mut self, times: SceneTimes, mode: Mode) -> bool {
        if self.begun {
            return false;
        }
        self.begun = true;
        self.times = times;
        self.assign = key_assignment(mode);
        true
    }

    /// Take one keyboard event.
    pub(crate) fn key(&mut self, key: &KeyInput<'_>) {
        let waiting = match key.code {
            KeyCode::Enter | KeyCode::NumpadEnter => &mut self.enter,
            KeyCode::Escape => &mut self.escape,
            KeyCode::KeyR => &mut self.run_again,
            KeyCode::KeyN => &mut self.follow_on,
            _ => return,
        };
        if key.pressed {
            *waiting = true;
        } else if key.released {
            *waiting = false;
        }
    }

    /// Note which key indices went down or came up since the last frame.
    fn note_held(&mut self, held: [bool; KEY_INDEX_COUNT]) {
        for (index, down) in held.into_iter().enumerate() {
            if down != self.was_down[index] {
                self.was_down[index] = down;
                self.changed[index] = true;
            }
        }
    }

    /// One frame of the scene at `now_us` on its clock: the timers and the times are applied first
    /// and the keys after, in the order the reference runs `render` and `input`. Answers the way
    /// out once the scene is over, and nothing before it has begun.
    pub(crate) fn step(&mut self, timers: &mut TimerState, now_us: i64, input: SceneInput) -> Option<Leave> {
        self.note_held(input.held);
        if !self.begun {
            return None;
        }
        switch_score_timers(timers, now_us);
        if advance(self.times, timers, now_us) == ScenePhase::Over {
            return Some(self.way_out(input));
        }
        if !takes_input(timers) {
            return None;
        }
        let mut confirmed = false;
        for index in 0..KEY_INDEX_COUNT {
            let Some(role) = self.assign[index] else {
                continue;
            };
            if !input.held[index] || !std::mem::take(&mut self.changed[index]) {
                continue;
            }
            match role {
                ResultKey::ChangeGraph => self.gauge_type = (self.next_gauge)(self.gauge_type),
                ResultKey::Ok | ResultKey::ReplayDifferent | ResultKey::ReplaySame => confirmed = true,
            }
        }
        let enter = std::mem::take(&mut self.enter);
        let escape = std::mem::take(&mut self.escape);
        let run_again = std::mem::take(&mut self.run_again) && input.can_run_again;
        let follow_on = std::mem::take(&mut self.follow_on) && input.can_follow_on;
        if input.submitting {
            return None;
        }
        if run_again {
            self.chosen = Some(Leave::RunAgain);
        } else if follow_on {
            self.chosen = Some(Leave::FollowOn);
        }
        if confirmed || enter || escape || self.chosen.is_some() {
            begin_fadeout(timers, now_us);
        }
        None
    }

    /// Where a scene whose fade has run out leaves for: where R or N said, or else another run
    /// while the first run-again lane key in index order is still down (`MusicResult.render`), or
    /// else the browser.
    fn way_out(&self, input: SceneInput) -> Leave {
        if let Some(chosen) = self.chosen {
            return chosen;
        }
        let asked = (0..KEY_INDEX_COUNT).filter(|index| input.held[*index]).find_map(|index| match self.assign[index] {
            Some(ResultKey::ReplayDifferent) => Some(Leave::RunAfresh),
            Some(ResultKey::ReplaySame) => Some(Leave::RunAgain),
            _ => None,
        });
        match asked {
            Some(leave) if input.can_run_again => leave,
            _ => Leave::Browser,
        }
    }
}

/// What has become of the skin the screen is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkinStatus {
    /// It is being read, or its files are.
    Reading,
    /// It is compiled and draws the screen.
    Ready,
    /// There is none: the built-in layout draws the screen.
    Gone,
}

/// What has become of the skin of the result screen `screen` (the result or the course result),
/// asked after the frame's own preparation of it.
pub(crate) fn skin_status(shared: &AppShared, screen: i32) -> SkinStatus {
    if shared.skin_screens.get(screen).is_some() && !shared.skins.is_waiting(screen) {
        return SkinStatus::Ready;
    }
    let coming = shared.has_skin_document(screen) && !shared.skin_wait_is_spent(screen);
    if coming { SkinStatus::Reading } else { SkinStatus::Gone }
}

/// Whether a skin will draw the result screen `screen` being entered: one is chosen for it, and it
/// has not already been found unreadable.
pub(crate) fn has_result_scene(shared: &AppShared, screen: i32) -> bool {
    shared.skins.document_path(&shared.config, screen).is_some()
        && (shared.skins.document(screen).is_some() || shared.skins.is_waiting(screen) || shared.skins.needs_reload_for(&shared.config, screen))
}

/// What became of a result screen's skin on one frame of drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkinFrameStart {
    pub(crate) status: SkinStatus,
    /// The scene began on this frame.
    began: bool,
}

impl SkinFrameStart {
    /// Whether the screen plays its arrival cue on this frame: the one a skin's scene begins on, or
    /// any on which no skin is coming at all. The screen itself plays it once.
    pub(crate) fn announces(self) -> bool {
        self.began || self.status == SkinStatus::Gone
    }
}

/// Get the skin of the result screen `screen` ready for a frame, and begin `scene` on the frame the
/// skin can first be drawn: the scene clock goes back to zero and the three score timers go on.
///
/// The single chart's result and the course's run this same opening (`MusicResult.render`,
/// `CourseResult.render`).
pub(crate) fn begin_scene_with_skin(shared: &mut AppShared, canvas: &mut Canvas<'_>, screen: i32, scene: &mut Scene) -> SkinFrameStart {
    canvas.clear_bga();
    shared.prepare_skin(canvas, screen);
    let status = skin_status(shared, screen);
    let began = status == SkinStatus::Ready && scene.begin(SceneTimes::of_skin(shared.skins.document(screen)), shared.mode);
    if began {
        shared.start_skin_scene_clock();
        let now_us = shared.skin_now_us();
        switch_score_timers(&mut shared.skin_timers, now_us);
    }
    SkinFrameStart { status, began }
}

/// What one frame of a result scene came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SceneFrame {
    /// The way out, once the scene is over.
    pub(crate) leave: Option<Leave>,
    /// The fade began on this frame, which is where a screen plays its closing cue.
    pub(crate) fade_began: bool,
}

/// One frame of a result scene on the application's clock and timers.
///
/// The score server's timers follow the submission once the scene has begun: one when the run is
/// sent, and one for how that ended (`MusicResult.prepare`, `CourseResult.prepare`). Which of them
/// are on is read off the same link the ranking cluster answers from
/// ([`AppShared::skin_ir_link`]), so a skin's timers and its options never disagree. Then the scene
/// steps.
pub(crate) fn run_scene_frame(shared: &mut AppShared, scene: &mut Scene, input: SceneInput) -> SceneFrame {
    let now_us = shared.skin_now_us();
    if scene.begun {
        for timer in shared.skin_ir_link().timers().on() {
            shared.skin_timers.switch(timer, true, now_us);
        }
    }
    let was_fading = shared.skin_timers.is_on(timer_id::FADEOUT);
    let leave = scene.step(&mut shared.skin_timers, now_us, input);
    SceneFrame { leave, fade_began: !was_fading && shared.skin_timers.is_on(timer_id::FADEOUT) }
}

/// What the caller knows of a finished run that neither its session nor the application says any
/// more by the time the screen is built.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunStanding {
    /// Whether the run's replay was written.
    pub(crate) replay_saved: bool,
    /// The lamp the run is reported under.
    pub(crate) lamp: ClearType,
    /// The best the player had on the chart before this run, read before the run joined the book.
    pub(crate) previous: PreviousScore,
    /// What the run was paced against, when it was paced against anything.
    pub(crate) target: Option<TargetView>,
}

/// Everything the screen keeps about the run beyond the built-in summary, in the shapes a skin's
/// frame borrows them in.
///
/// It is taken from the play session as the run ends ([`ResultRun::of`]), after the run's record
/// has been written, so a skin read against it reads a settled result.
pub(crate) struct ResultRun {
    /// The chart as it was played, lane options applied, which is the model the reference's result
    /// screen reads its graphs from.
    chart: ChartOverview,
    /// The chart's stage image, decoded only when a skin is there to show it.
    stagefile: Option<DecodedImage>,
    /// The seed the run's lanes were laid out with.
    seed: u64,
    /// Whether the run counted towards the player's records, which is what lets a run-again keep
    /// its arrangement (`PlayerResource.isUpdateScore`).
    updates_score: bool,
    /// What became of the run's own replay.
    replay: ReplaySlot,
    /// The window of each judgement in milliseconds, late bound then early, best first.
    judge_area: [[i32; 2]; TIMING_JUDGE_AREAS],
    /// The history of every gauge, by gauge type.
    gauges: Vec<Vec<f32>>,
    scales: [GaugeScale; GAUGE_TYPES],
    /// The gauge the run ended on, which the screen opens its graph on.
    finished_gauge: usize,
    timing: Vec<u32>,
    timing_average: f32,
    timing_std_dev: f32,
    by_judge: Vec<[u32; JUDGEMENTS]>,
    by_timing: Vec<[u32; EARLY_LATE_BUCKETS]>,
    /// The run as the clusters of a skin's host report it: its score beside the best it replaced
    /// and the target, the settings it was played with and whether the chart is starred
    /// (`MusicResult.updateScoreDatabase`).
    snapshot: ResultSnapshot,
}

impl ResultRun {
    /// Read the run `session` has just finished, as `standing` places it.
    ///
    /// Called once the run's record is kept in [`AppShared::run_records`]; a caller that kept none
    /// has the record taken from the session here.
    pub(crate) fn of(session: &PlaySession, shared: &AppShared, standing: RunStanding) -> ResultRun {
        let RunStanding { replay_saved, lamp, previous, target } = standing;
        let taken;
        let record = match shared.run_records.last() {
            Some(kept) => kept,
            None => {
                taken = session.record();
                &taken
            }
        };
        let model = session.model();
        let chart = ChartOverview::of_model(model);
        let skinned = shared.skins.document_path(&shared.config, SKIN_TYPE_RESULT).is_some();
        let stagefile = Path::new(&shared.chart_path)
            .parent()
            .filter(|_| skinned && !chart.stagefile.trim().is_empty())
            .and_then(|folder| decode_bga_image(folder, &chart.stagefile));
        let setup = session.judge_setup();
        let stored = replay_saved || shared.replay.is_some() || shared.scores.for_md5(&chart.md5).iter().any(|past| past.replay_file.is_some());
        let custom_judge = is_custom_judge(&setup);
        let updates_score = updates_score(shared.config.play.autoplay, shared.replay.is_some(), custom_judge, shared.config.play.scratch_auto, false);
        let snapshot = ResultSnapshot::of(ResultInput {
            previous,
            target,
            options: PlayedOptions { random: random_option_index(shared.config.play.random), custom_judge, ..PlayedOptions::default() },
            favorite_chart: Some(shared.favorites.contains(&chart.md5)),
            updates_score,
            ..ResultInput::from_record(record, shared.mode, lamp)
        });
        ResultRun {
            stagefile,
            seed: session.seed(),
            updates_score,
            replay: match (replay_saved, stored) {
                (true, _) => ReplaySlot::Saved,
                (false, true) => ReplaySlot::Exists,
                (false, false) => ReplaySlot::Missing,
            },
            judge_area: judge_area(session),
            ..ResultRun::of_record(chart, record, snapshot)
        }
    }

    /// The parts of a run that its record alone says, on `chart`, reported as `snapshot`.
    fn of_record(chart: ChartOverview, record: &PlayRecord, snapshot: ResultSnapshot) -> ResultRun {
        ResultRun {
            chart,
            snapshot,
            stagefile: None,
            seed: 0,
            updates_score: false,
            replay: ReplaySlot::Missing,
            judge_area: [[0; 2]; TIMING_JUDGE_AREAS],
            gauges: rbms_judge::gauge::GaugeIndex::ALL.map(|index| record.gauge_log.of(index).to_vec()).to_vec(),
            scales: record.gauge_bounds.map(|bounds| GaugeScale::new(bounds.min, bounds.max, bounds.border)),
            finished_gauge: record.finished_gauge.index(),
            timing: record.timing.buckets().to_vec(),
            timing_average: record.timing.average(),
            timing_std_dev: record.timing.std_dev(),
            by_judge: record.seconds.by_judge.clone(),
            by_timing: record.seconds.by_timing.clone(),
        }
    }

    /// The gauge numbered `gauge_type` as the run left it, for the gauge bar and the gauge graph.
    fn gauge(&self, gauge_type: usize) -> GaugeFrame {
        let last = self.gauges.get(gauge_type).and_then(|history| history.last()).copied().unwrap_or_default();
        GaugeFrame::finished(gauge_type, last, self.scales)
    }

    /// The series a skin's graphs plot. `judged` is how many notes took each judgement over the
    /// whole run.
    fn series<'a>(&'a self, judged: &'a [u32; JUDGEMENTS]) -> FrameSeries<'a> {
        let chart = &self.chart;
        let timing = TimingHistogram { average: self.timing_average, std_dev: self.timing_std_dev, ..TimingHistogram::new(&self.timing) };
        FrameSeries {
            gauge_history: Some(GaugeHistory::of_kinds(&self.gauges)),
            timing: Some(timing.with_judge_area(self.judge_area)),
            bpm: Some(BpmTimeline::of_chart(&chart.speeds, chart.main_bpm, chart.min_bpm, chart.max_bpm, Some(chart.length_ms))),
            notes: Some(NoteDistribution {
                judged,
                kinds: &chart.kinds,
                judgements: &self.by_judge,
                early_late: &self.by_timing,
                popn: chart.mode == Some(Mode::POPN_9K),
                playing: None,
            }),
            recent_hits: None,
        }
    }
}

/// The windows the run's key notes were judged in, in whole milliseconds, late bound then early
/// (`SkinTimingVisualizer.getJudgeArea`): the mode's note table at the chart's judge rank and the
/// player's judge widths.
fn judge_area(session: &PlaySession) -> [[i32; 2]; TIMING_JUDGE_AREAS] {
    let model = session.model();
    let setup = session.judge_setup();
    let judgerank = session.judge_window_rule().judgerank_for(model.meta.rank, model.meta.defexrank);
    let note = JudgeWindowSet::for_mode(&model.mode, judgerank, setup.judge_rate_key, setup.judge_rate_scratch).note;
    let millis = |bound_us: i64| i32::try_from(bound_us / MICROS_PER_MILLI).unwrap_or_default();
    let mut area = [[0; 2]; TIMING_JUDGE_AREAS];
    for (window, (late, early)) in area.iter_mut().zip(note.pairs()) {
        *window = [millis(late), millis(early)];
    }
    area
}

/// The result screen's own state: the summary view built when the run ended, what surrounds it,
/// and the scene a skin plays over both. The submission status under it lives in [`AppShared`],
/// because the request outlives this screen.
///
/// The view is boxed: it carries the run's whole measurement, which would otherwise set the size of
/// every other screen and make each stage change a large copy. So is everything a skin's scene
/// keeps.
pub(crate) struct ResultState {
    view: Box<ResultView>,
    extras: ResultExtras,
    /// Whether the run counted as a clear. The view carries the lamp's label and colour but not the
    /// verdict itself, and a document asks for the verdict.
    cleared: bool,
    /// The chart's tempo as `(progress through the chart, bpm)`, which a document's BPM graph plots
    /// when the screen knows nothing more of the chart.
    bpm_points: Vec<(f32, f64)>,
    /// What the run left for a skin, when the screen was built from a run.
    run: Option<Box<ResultRun>>,
    stage: Box<SceneStage>,
}

/// What the screen keeps for the scene a skin plays.
struct SceneStage {
    scene: Scene,
    skin: SkinStatus,
    /// Whether the clear or fail cue has been played.
    announced: bool,
    /// Whether the closing cue has been played.
    closed: bool,
    /// How far the wheel has turned that the ranking has not moved by yet: the part of a line a
    /// frame could not act on stays here for the turns that follow.
    scroll: f32,
    /// How far down the ranking the list is scrolled (`AbstractResult.rankingOffset`).
    ranking_offset: i32,
    /// Where the screen can lead besides the browser, as it stood when the screen was entered.
    ways: WaysOn,
}

/// How the IR status lines are drawn over a skin: in the top right corner, right-aligned, on a dim
/// strip of their own. A skin owns the rest of the screen.
const IR_OVER_SKIN_RIGHT: f32 = CW as f32 - 16.0;
const IR_OVER_SKIN_Y: f32 = 12.0;
const IR_OVER_SKIN_LINE_H: f32 = 20.0;
const IR_OVER_SKIN_SCALE: f32 = 1.2;
const IR_OVER_SKIN_PAD: f32 = 6.0;
const IR_OVER_SKIN_STRIP: Color = Color { r: 0, g: 0, b: 0, a: 150 };

impl ResultState {
    /// A screen reporting a run and nothing around it: no target, no offer to run it again, and a
    /// run that did not clear.
    pub(crate) fn new(view: ResultView) -> ResultState {
        ResultState {
            view: Box::new(view),
            extras: ResultExtras::default(),
            cleared: false,
            bpm_points: Vec::new(),
            run: None,
            stage: Box::new(SceneStage {
                scene: Scene::new(),
                skin: SkinStatus::Gone,
                announced: false,
                closed: false,
                scroll: 0.0,
                ranking_offset: 0,
                ways: WaysOn::default(),
            }),
        }
    }

    /// The same screen reporting a run that reached the end with its gauge up.
    pub(crate) fn cleared(mut self, cleared: bool) -> ResultState {
        self.cleared = cleared;
        self
    }

    /// The same screen with what surrounds the run on it: the target it was paced against, and
    /// whether the run-again keys are live.
    pub(crate) fn paced_by(mut self, extras: ResultExtras) -> ResultState {
        self.extras = extras;
        self
    }

    /// The same screen carrying the tempo of the chart the run was played on, which is what a
    /// document plots a BPM graph from.
    pub(crate) fn tempo(mut self, bpm_points: Vec<(f32, f64)>) -> ResultState {
        self.bpm_points = bpm_points;
        self
    }

    /// The same screen carrying what the run left for a skin. The graph opens on the gauge the run
    /// ended on (`MusicResult.create`).
    pub(crate) fn played(mut self, run: ResultRun) -> ResultState {
        self.stage.scene.gauge_type = run.finished_gauge;
        self.run = Some(Box::new(run));
        self
    }

    /// The series the document's own graph objects read: the run's, or for a screen built without a
    /// run the measurements the built-in panels are drawn from.
    fn series(&self) -> FrameSeries<'_> {
        match &self.run {
            Some(run) => run.series(&self.view.judge_dist),
            None => FrameSeries {
                gauge_history: Some(GaugeHistory::new(&self.view.gauge_series)),
                timing: Some(TimingHistogram::new(&self.view.timing_hist)),
                bpm: Some(BpmTimeline::new(&self.bpm_points)),
                notes: Some(NoteDistribution::of_judgements(&self.view.judge_dist)),
                recent_hits: None,
            },
        }
    }

    /// What the screen itself holds that a skin asks about: the gauge on show, the replay slots and
    /// how far the ranking is scrolled.
    pub(crate) fn scene_facts(&self, shared: &AppShared) -> SceneFacts {
        let mut replay = [ReplaySlot::Missing; REPLAY_SLOT_COUNT];
        replay[0] = self.run.as_ref().map_or(ReplaySlot::Missing, |run| run.replay);
        SceneFacts { gauge_type: self.stage.scene.gauge_type, replay, ranking_offset: self.stage.ranking_offset, ranking_total: self.ranking_total(shared) }
    }

    /// How many players the ranking of the chart holds, as far as the browser's ranking cache has
    /// been told (`RankingData.getTotalPlayer`).
    fn ranking_total(&self, shared: &AppShared) -> i32 {
        let Some(run) = &self.run else {
            return 0;
        };
        match shared.ranking_cache.peek(&run.chart.md5) {
            Some(RankingState::Ready(board)) => i32::try_from(board.rows.len()).unwrap_or(i32::MAX),
            _ => 0,
        }
    }

    /// Move the ranking by the whole lines the wheel has turned (`AbstractResult.input`). What is
    /// left of a line stays counted, so a wheel that turns less than a line a frame still moves the
    /// list once its turns add up to one.
    fn scroll_ranking(&mut self, shared: &AppShared) {
        let whole = self.stage.scroll.trunc();
        if whole == 0.0 {
            return;
        }
        self.stage.scroll -= whole;
        let last = self.ranking_total(shared).max(1) - 1;
        self.stage.ranking_offset = self.stage.ranking_offset.saturating_add(whole as i32).clamp(0, last);
    }

    /// Carry out what the skin asked of the screen during the frame that ended last.
    ///
    /// The four replay slots save nothing: the run's replay was written, or not, when the run
    /// ended. The ranking slider is routed to the browser's cluster, which is the only screen the
    /// reference lets it be dragged on besides this one.
    fn carry_out_requests(&mut self, shared: &mut AppShared) {
        for request in shared.skin_requests().take(Cluster::Result) {
            match request {
                ClusterRequest::Event { id: BUTTON_FAVORITTE_CHART, .. } => self.toggle_favourite(shared),
                ClusterRequest::Event { id: BUTTON_REPLAY | BUTTON_REPLAY2 | BUTTON_REPLAY3 | BUTTON_REPLAY4, .. } => {}
                _ => {}
            }
        }
        for request in shared.skin_requests().take(Cluster::Select) {
            if let ClusterRequest::WriteRate { id: RATE_RANKING_POSITION, value } = request
                && (0.0..1.0).contains(&value)
            {
                self.stage.ranking_offset = (self.ranking_total(shared).max(1) as f32 * value) as i32;
            }
        }
    }

    /// Star or unstar the chart that was played, write the change out at once, and show it on the
    /// button the skin drew it from.
    fn toggle_favourite(&mut self, shared: &mut AppShared) {
        let Some(run) = &mut self.run else {
            return;
        };
        run.snapshot.favorite_chart = Some(shared.favorites.toggle(&run.chart.md5));
        shared.favorites.save(&shared.favorites_path);
        shared.select_gen = shared.select_gen.wrapping_add(1);
    }

    /// Play the clear or fail cue, once (`MusicResult.prepare`).
    fn announce(&mut self, shared: &mut AppShared) {
        if !std::mem::replace(&mut self.stage.announced, true) {
            shared.play_system_sound(result_sound(self.cleared));
        }
    }

    /// Play the closing cue, once: where the fade begins on a skin's scene, and on the way out of
    /// a screen that never began one. A scene whose sound set has a closing cue silences the clear
    /// or fail cue for it (`MusicResult.render`, `MusicResult.input`).
    fn close(&mut self, shared: &mut AppShared) {
        if std::mem::replace(&mut self.stage.closed, true) {
            return;
        }
        if self.stage.scene.begun && shared.syssound.is_resolved(SystemSound::ResultClose) {
            shared.stop_system_sound(SystemSound::ResultClear);
            shared.stop_system_sound(SystemSound::ResultFail);
        }
        shared.play_system_sound(SystemSound::ResultClose);
    }

    /// Leave a scene that is over.
    ///
    /// A run laid out as it was hands its seed to the load that follows, unless it did not count
    /// towards the player's records, which the reference lays out afresh. A way out that turns out
    /// to lead nowhere leads to the browser.
    fn leave(&mut self, shared: &mut AppShared, leave: Leave) -> Transition {
        let moved = match leave {
            Leave::Browser => Transition::Stay,
            Leave::RunAfresh => retry(shared),
            Leave::RunAgain => {
                shared.retry_seed = self.run.as_ref().filter(|run| run.updates_score).map(|run| run.seed);
                retry(shared)
            }
            Leave::FollowOn => next_song(shared),
        };
        if matches!(moved, Transition::Stay) {
            shared.retry_seed = None;
            return shared.leave_play();
        }
        moved
    }

    /// The keys of the built-in screen: back to the browser, or straight into another run. The two
    /// run-again keys stand down for a run the player did not play, which has nothing to repeat.
    fn built_in_key(ctx: &mut FrameCtx<'_>, key: &KeyInput<'_>) -> Transition {
        if !key.pressed {
            return Transition::Stay;
        }
        match key.code {
            KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter => ctx.shared.leave_play(),
            KeyCode::KeyR if offers_retry(ctx.shared) => retry(ctx.shared),
            KeyCode::KeyN if offers_retry(ctx.shared) => next_song(ctx.shared),
            _ => Transition::Stay,
        }
    }

    /// Draw the skin's frame, or answer that it has none to draw yet. A Lua skin still waiting to
    /// be read is read by this call, against the run handed over here.
    fn draw_skin(&self, shared: &AppShared, canvas: &mut Canvas<'_>) -> bool {
        let stagefile = self
            .run
            .as_ref()
            .and_then(|run| run.stagefile.as_ref())
            .and_then(|image| canvas.background_texture(image.generation, &image.rgba, image.width, image.height));
        let chart = self.run.as_ref().map(|run| run.chart.meta(stagefile.is_some()));
        let data = FrameData {
            gauge: self.run.as_ref().map(|run| run.gauge(self.stage.scene.gauge_type)),
            series: self.series(),
            images: ReferenceImages { stagefile, ..ReferenceImages::default() },
            ..FrameData::default()
        };
        let scene = self.scene_facts(shared);
        let run = self.run.as_ref().map(|run| &run.snapshot);
        let draw = ResultDraw { view: &self.view, extras: &self.extras, cleared: self.cleared, chart: chart.as_ref(), scene, run, data };
        shared.draw_result_skin(canvas, SKIN_TYPE_RESULT, &draw)
    }

    /// The submission lines, drawn over a skin's frame where they cover the least of it.
    ///
    /// They are this player's own word on what the score server said, which no property of a skin
    /// carries. With no server set up there is nothing to say, and once the scene fades the skin has
    /// the screen to itself, so neither draws them.
    fn draw_ir_over_skin(shared: &AppShared, canvas: &mut Canvas<'_>) {
        if shared.ir_status == IrStatus::Off || shared.skin_timers.is_on(timer_id::FADEOUT) {
            return;
        }
        for (row, (text, kind)) in shared.ir_status.lines().iter().enumerate() {
            let y = IR_OVER_SKIN_Y + row as f32 * IR_OVER_SKIN_LINE_H;
            let width = text_width(text, IR_OVER_SKIN_SCALE);
            let strip = Rect::new(IR_OVER_SKIN_RIGHT - width - IR_OVER_SKIN_PAD, y, width + IR_OVER_SKIN_PAD + IR_OVER_SKIN_PAD, IR_OVER_SKIN_LINE_H);
            canvas.fill_rect(strip, IR_OVER_SKIN_STRIP);
            draw_text_right(canvas, IR_OVER_SKIN_RIGHT, y, IR_OVER_SKIN_SCALE, ir_line_color(*kind), text);
        }
    }

    /// The run this screen reports, for the tests that check what the run put on it.
    #[cfg(test)]
    pub(crate) fn view(&self) -> &ResultView {
        &self.view
    }

    /// Put a run's measurements on the view, for the snapshots that check they are drawn.
    #[cfg(test)]
    pub(crate) fn set_measurements(&mut self, gauge_series: Vec<f32>, timing_hist: Box<[u32]>, judge_dist: [u32; 6]) {
        self.view.gauge_series = gauge_series;
        self.view.timing_hist = timing_hist;
        self.view.judge_dist = judge_dist;
    }
}

impl StageHandler for ResultState {
    /// One frame of a skin's scene; the built-in screen has nothing to move.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let shared = &mut *ctx.shared;
        if self.stage.skin == SkinStatus::Gone {
            return Transition::Stay;
        }
        self.scroll_ranking(shared);
        self.carry_out_requests(shared);
        let input = SceneInput::read(shared, self.stage.ways);
        let frame = run_scene_frame(shared, &mut self.stage.scene, input);
        if frame.fade_began {
            self.close(shared);
        }
        match frame.leave {
            None => Transition::Stay,
            Some(leave) => self.leave(shared, leave),
        }
    }

    /// The score timers go on as the screen opens, and the clear or fail cue is played unless a
    /// skin's scene is on its way to play it.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        let shared = &mut *ctx.shared;
        let now_us = shared.skin_now_us();
        switch_score_timers(&mut shared.skin_timers, now_us);
        self.stage.ways = WaysOn::of(shared);
        self.stage.skin = if has_result_scene(shared, SKIN_TYPE_RESULT) { SkinStatus::Reading } else { SkinStatus::Gone };
        if self.stage.skin == SkinStatus::Gone {
            self.announce(shared);
        }
    }

    /// A skin's scene leaves in silence, its cues stopped where the reference stops them
    /// (`MusicResult.shutdown`). The built-in screen plays its closing cue on the way out, as it
    /// always did.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        if !self.stage.scene.begun {
            self.close(ctx.shared);
            return;
        }
        for sound in [SystemSound::ResultClear, SystemSound::ResultFail, SystemSound::ResultClose] {
            ctx.shared.stop_system_sound(sound);
        }
    }

    /// Keys on the result screen. A skin's scene keeps them until it listens; the built-in screen
    /// acts on them at once; and a screen still waiting for its skin takes Escape alone.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        match self.stage.skin {
            SkinStatus::Gone => ResultState::built_in_key(ctx, &key),
            SkinStatus::Reading if key.pressed && key.code == KeyCode::Escape => ctx.shared.leave_play(),
            SkinStatus::Reading | SkinStatus::Ready => {
                self.stage.scene.key(&key);
                Transition::Stay
            }
        }
    }

    /// The wheel moves the ranking, which only a skin shows.
    fn handle_scroll(&mut self, _ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        self.stage.scroll += lines;
        Transition::Stay
    }

    /// The skin's frame, black while one is on its way, and the built-in screen when there is
    /// none. The frame the skin first draws on is the one the scene begins on: the clock goes back
    /// to zero, the score timers go on and the clear or fail cue is played.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let shared = &mut *ctx.shared;
        let start = begin_scene_with_skin(shared, canvas, SKIN_TYPE_RESULT, &mut self.stage.scene);
        self.stage.skin = start.status;
        if start.announces() {
            self.announce(shared);
        }
        if self.draw_skin(shared, canvas) {
            ResultState::draw_ir_over_skin(shared, canvas);
            return;
        }
        if self.stage.skin == SkinStatus::Reading {
            canvas.native().clear(Color::BLACK);
            return;
        }
        render_result_with_palette(canvas, &self.view, &shared.result_palette, &self.extras);
        for (i, (text, kind)) in shared.ir_status.lines().iter().enumerate() {
            draw_text(canvas, IR_RESULT_X, IR_RESULT_Y + i as f32 * IR_RESULT_LINE_H, IR_RESULT_SCALE, ir_line_color(*kind), text);
        }
    }
}

#[cfg(test)]
mod tests;
