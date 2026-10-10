//! The result screen, first as the bare scene with a clock the test counts, then as the screen:
//! drawn by a small skin of its own and left for the browser or another run, and last as the
//! built-in screen it is without a skin.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rbms_library::{Library, SongEntry};
use rbms_play::{NullSink, SessionClock, SessionOptions};
use rbms_render::result::TargetView;
use rbms_store::{Replay, SCORE_LN_MODE_FROM_CHART};

use super::*;
use crate::app_result::enter_result;
use crate::ir_ext::MultiIr;
use crate::ir_outcome::format_submit_outcome;
use crate::ir_ranking::{RankingBoard, RankingRow};
use crate::stage::capture::{app_in, settings_of};
use crate::stage::render_tests::FRAME_DT;
use crate::stage::select::tests::{press, release};
use crate::stage::{HeadlessCanvas, PlayState, Stage, StageId};
use crate::{App, CH, Config, LaunchOptions, SelectItem};

/// The times the scene tests run on, which are the fixture skin's own.
const TIMES: SceneTimes = SceneTimes { input_ms: 2000, scene_ms: 3_600_000, fadeout_ms: 4000 };

/// The first moment the scene listens, and a moment while it does.
const LISTENING_MS: i64 = TIMES.input_ms + 1;
const WHILE_LISTENING_MS: i64 = TIMES.input_ms + 500;

/// The key indices of the seven-key layout by what they do.
const OK_INDICES: [usize; 4] = [0, 1, 2, 3];
const AFRESH_INDEX: usize = 4;
const GRAPH_INDEX: usize = 5;
const AGAIN_INDEX: usize = 6;

/// The keys the shipped seven-key layout plays those indices on.
const OK_KEY: KeyCode = KeyCode::KeyZ;
const AFRESH_KEY: KeyCode = KeyCode::KeyC;
const GRAPH_KEY: KeyCode = KeyCode::KeyF;
const AGAIN_KEY: KeyCode = KeyCode::KeyV;

fn at_ms(millis: i64) -> i64 {
    millis * MICROS_PER_MILLI
}

/// Nothing held, on a run that can be started again and followed on.
const IDLE: SceneInput = SceneInput { held: [false; KEY_INDEX_COUNT], submitting: false, can_run_again: true, can_follow_on: true };

/// The same with the key of one index down.
fn holding(index: usize) -> SceneInput {
    let mut input = IDLE;
    input.held[index] = true;
    input
}

/// A scene that has begun on [`TIMES`] in the seven-key layout, with its timers.
fn begun() -> (Scene, TimerState) {
    let mut scene = Scene::new();
    assert!(scene.begin(TIMES, Mode::BEAT_7K));
    (scene, TimerState::new())
}

/// A scene that has begun and reached the moment it listens with nothing held.
fn listening() -> (Scene, TimerState) {
    let (mut scene, mut timers) = begun();
    assert_eq!(scene.step(&mut timers, at_ms(LISTENING_MS), IDLE), None);
    assert!(takes_input(&timers));
    (scene, timers)
}

/// Run a fading scene to just past the end of its fade and answer the way it leaves.
fn fade_out(scene: &mut Scene, timers: &mut TimerState, input: SceneInput) -> Option<Leave> {
    let began = timers.value_us(timer_id::FADEOUT);
    assert_eq!(scene.step(timers, began + at_ms(TIMES.fadeout_ms), input), None, "the scene left before its fade was over");
    scene.step(timers, began + at_ms(TIMES.fadeout_ms + 1), input)
}

/// The three score timers go on together on the scene's first frame, as the reference switches
/// them on every frame of the screen, and stay as they were switched.
#[test]
fn the_score_timers_go_on_together_on_the_scenes_first_frame() {
    let (mut scene, mut timers) = begun();
    scene.step(&mut timers, 0, IDLE);
    for timer in [timer_id::RESULTGRAPH_BEGIN, timer_id::RESULTGRAPH_END, timer_id::RESULT_UPDATESCORE] {
        assert_eq!(timers.value_us(timer), 0, "{timer:?} did not go on with the scene's first frame");
    }
    scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), IDLE);
    assert_eq!(timers.value_us(timer_id::RESULTGRAPH_END), 0, "a score timer was started again");
    assert!(!timers.is_on(timer_id::FADEOUT), "a scene with an hour to run faded on its own");
}

/// Until its skin has drawn, a scene has no clock to run on: nothing is switched and no key acts.
#[test]
fn a_scene_that_has_not_begun_neither_runs_nor_listens() {
    let mut scene = Scene::new();
    let mut timers = TimerState::new();
    scene.key(&press(KeyCode::Enter));
    assert_eq!(scene.step(&mut timers, at_ms(TIMES.input_ms + 10_000), holding(OK_INDICES[0])), None);
    assert!(!timers.is_on(timer_id::RESULTGRAPH_BEGIN) && !timers.is_on(timer_id::STARTINPUT) && !timers.is_on(timer_id::FADEOUT));
}

/// A key tapped while the scene does not listen is forgotten, and one still down when it starts
/// listening acts then: the reference reads a key's state with whether it changed, not its press.
#[test]
fn a_key_acts_only_once_the_scene_listens() {
    let (mut tapped, mut timers) = begun();
    for code in [KeyCode::Enter, KeyCode::Escape, KeyCode::KeyR, KeyCode::KeyN] {
        tapped.key(&press(code));
        tapped.step(&mut timers, at_ms(TIMES.input_ms / 2), holding(OK_INDICES[0]));
        tapped.key(&release(code));
    }
    assert_eq!(tapped.step(&mut timers, at_ms(TIMES.input_ms), IDLE), None);
    assert_eq!(tapped.step(&mut timers, at_ms(WHILE_LISTENING_MS), IDLE), None);
    assert!(!timers.is_on(timer_id::FADEOUT), "a key tapped before the scene took input was acted on");

    let (mut held, mut timers) = begun();
    assert_eq!(held.step(&mut timers, at_ms(TIMES.input_ms), holding(OK_INDICES[0])), None);
    assert!(!timers.is_on(timer_id::FADEOUT), "a key acted before the scene took input");
    held.step(&mut timers, at_ms(LISTENING_MS), holding(OK_INDICES[0]));
    assert_eq!(timers.value_us(timer_id::FADEOUT), at_ms(LISTENING_MS), "a key held through the wait did not act when input opened");
}

/// Each confirming key starts the fade, and the scene leaves for the browser once the fade has run
/// for longer than the skin's fade time.
#[test]
fn a_confirming_key_fades_the_scene_out_to_the_browser() {
    for index in OK_INDICES {
        let (mut scene, mut timers) = listening();
        assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), holding(index)), None);
        assert_eq!(timers.value_us(timer_id::FADEOUT), at_ms(WHILE_LISTENING_MS), "key index {index} did not start the fade");
        assert_eq!(fade_out(&mut scene, &mut timers, IDLE), Some(Leave::Browser));
    }
    for code in [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Escape] {
        let (mut scene, mut timers) = listening();
        scene.key(&press(code));
        assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), IDLE), None);
        assert!(timers.is_on(timer_id::FADEOUT), "{code:?} did not start the fade");
        assert_eq!(fade_out(&mut scene, &mut timers, IDLE), Some(Leave::Browser));
    }
}

/// Once the fade has begun no key starts it over or turns the gauge.
#[test]
fn a_fading_scene_takes_no_more_keys() {
    let (mut scene, mut timers) = listening();
    scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), holding(OK_INDICES[0]));
    let shown = scene.gauge_type;
    scene.key(&press(KeyCode::Enter));
    assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS + 100), holding(GRAPH_INDEX)), None);
    assert_eq!(timers.value_us(timer_id::FADEOUT), at_ms(WHILE_LISTENING_MS), "a key pressed during the fade started it again");
    assert_eq!(scene.gauge_type, shown, "the gauge was turned during the fade");
}

/// The sixth key turns the gauge the graph plots, once a press, round the six gauges a chart is
/// played on or round the three a course is, and does not close the screen.
#[test]
fn the_graph_key_turns_the_gauge_to_the_next_kind() {
    let (mut scene, mut timers) = listening();
    scene.gauge_type = DEFAULT_GAUGE_TYPE;
    let mut seen = Vec::new();
    for press in 0..CHART_GAUGES as i64 {
        let now = at_ms(WHILE_LISTENING_MS + press * 10);
        scene.step(&mut timers, now, holding(GRAPH_INDEX));
        seen.push(scene.gauge_type);
        scene.step(&mut timers, now + 1, holding(GRAPH_INDEX));
        assert_eq!(seen.last(), Some(&scene.gauge_type), "a key that stayed down turned the gauge twice");
        scene.step(&mut timers, now + 2, IDLE);
    }
    assert_eq!(seen, [3, 4, 5, 0, 1, 2]);
    assert!(!timers.is_on(timer_id::FADEOUT), "turning the gauge closed the screen");

    assert_eq!([6, 7, 8].map(next_gauge_type), [7, 8, 6], "a course's gauges turn among themselves");
    assert_eq!(next_gauge_type(LAST_CHART_GAUGE), 0);
}

/// The two run-again lane keys confirm like any other, and which way the scene leaves is read when
/// the fade ends: for another run while one of them is still down, the fifth key before the
/// seventh, and for the browser when neither is or the run cannot be started again.
#[test]
fn a_run_again_key_still_down_when_the_fade_ends_starts_another_run() {
    let faded_by = |index: usize| {
        let (mut scene, mut timers) = listening();
        scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), holding(index));
        assert!(timers.is_on(timer_id::FADEOUT), "key index {index} did not start the fade");
        (scene, timers)
    };
    let (mut scene, mut timers) = faded_by(AGAIN_INDEX);
    assert_eq!(fade_out(&mut scene, &mut timers, holding(AGAIN_INDEX)), Some(Leave::RunAgain));
    let (mut scene, mut timers) = faded_by(AFRESH_INDEX);
    assert_eq!(fade_out(&mut scene, &mut timers, holding(AFRESH_INDEX)), Some(Leave::RunAfresh));

    let (mut scene, mut timers) = faded_by(AGAIN_INDEX);
    let mut both = holding(AGAIN_INDEX);
    both.held[AFRESH_INDEX] = true;
    assert_eq!(fade_out(&mut scene, &mut timers, both), Some(Leave::RunAfresh), "the fifth key is read before the seventh");

    let (mut scene, mut timers) = faded_by(AGAIN_INDEX);
    assert_eq!(fade_out(&mut scene, &mut timers, IDLE), Some(Leave::Browser), "a key let go during the fade still started a run");

    let (mut scene, mut timers) = faded_by(AGAIN_INDEX);
    let watched = SceneInput { can_run_again: false, can_follow_on: false, ..holding(AGAIN_INDEX) };
    assert_eq!(fade_out(&mut scene, &mut timers, watched), Some(Leave::Browser), "a run nobody played was started again");
}

/// R and N decide the way out when they are pressed, behind the same wait and the same fade, and
/// do nothing at all for a run that cannot be repeated or followed.
#[test]
fn the_players_own_run_again_keys_wait_for_the_scene_and_ride_its_fade() {
    for (code, leave) in [(KeyCode::KeyR, Leave::RunAgain), (KeyCode::KeyN, Leave::FollowOn)] {
        let (mut scene, mut timers) = begun();
        scene.key(&press(code));
        assert_eq!(scene.step(&mut timers, at_ms(TIMES.input_ms), IDLE), None);
        assert!(!timers.is_on(timer_id::FADEOUT), "{code:?} acted before the scene took input");
        assert_eq!(scene.step(&mut timers, at_ms(LISTENING_MS), IDLE), None);
        assert!(timers.is_on(timer_id::FADEOUT), "{code:?} still down when input opened did not act");
        scene.key(&release(code));
        assert_eq!(fade_out(&mut scene, &mut timers, IDLE), Some(leave), "{code:?} did not decide the way out");

        let (mut scene, mut timers) = listening();
        scene.key(&press(code));
        let unrepeatable = SceneInput { can_run_again: false, can_follow_on: false, ..IDLE };
        assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), unrepeatable), None);
        assert!(!timers.is_on(timer_id::FADEOUT), "{code:?} closed a screen it has nothing to start from");
    }

    let (mut scene, mut timers) = listening();
    scene.key(&press(KeyCode::KeyN));
    scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), SceneInput { can_follow_on: false, ..IDLE });
    assert!(!timers.is_on(timer_id::FADEOUT), "N closed the screen at the end of the list");
}

/// While the score server has not answered, a confirming key is taken and closes nothing; the
/// player presses again once it has.
#[test]
fn the_screen_stays_open_while_the_score_is_being_submitted() {
    let (mut scene, mut timers) = listening();
    let submitting = SceneInput { submitting: true, ..holding(OK_INDICES[0]) };
    scene.key(&press(KeyCode::Enter));
    scene.key(&press(KeyCode::KeyR));
    assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), submitting), None);
    assert!(!timers.is_on(timer_id::FADEOUT), "the screen closed while the score was being submitted");

    assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS + 100), holding(OK_INDICES[0])), None);
    assert!(!timers.is_on(timer_id::FADEOUT), "a press made during the submission closed the screen after it");

    scene.step(&mut timers, at_ms(WHILE_LISTENING_MS + 200), IDLE);
    scene.step(&mut timers, at_ms(WHILE_LISTENING_MS + 300), holding(OK_INDICES[0]));
    assert!(timers.is_on(timer_id::FADEOUT), "a new press after the submission did not close the screen");
    assert_eq!(fade_out(&mut scene, &mut timers, IDLE), Some(Leave::Browser), "a run-again asked for during the submission was kept");
}

/// A scene left alone for its whole length fades on its own and leaves for the browser.
#[test]
fn a_scene_left_alone_runs_its_length_and_leaves_for_the_browser() {
    let (mut scene, mut timers) = begun();
    assert_eq!(scene.step(&mut timers, at_ms(TIMES.scene_ms), IDLE), None);
    assert!(!timers.is_on(timer_id::FADEOUT));
    assert_eq!(scene.step(&mut timers, at_ms(TIMES.scene_ms + 1), IDLE), None);
    assert!(timers.is_on(timer_id::FADEOUT));
    assert_eq!(fade_out(&mut scene, &mut timers, IDLE), Some(Leave::Browser));
}

/// The keys are assigned by mode as the reference assigns them: the turntable does nothing, the
/// nine buttons confirm with their last two, and the keyboard layout has its own order.
#[test]
fn each_mode_assigns_its_keys_as_the_reference_does() {
    for mode in [Mode::BEAT_5K, Mode::BEAT_7K, Mode::BEAT_10K, Mode::BEAT_14K] {
        assert_eq!(key_assignment(mode), BEAT_KEYS, "{}", mode.name);
    }
    assert_eq!(BEAT_KEYS[AFRESH_INDEX], Some(ResultKey::ReplayDifferent));
    assert_eq!(BEAT_KEYS[GRAPH_INDEX], Some(ResultKey::ChangeGraph));
    assert_eq!(BEAT_KEYS[AGAIN_INDEX], Some(ResultKey::ReplaySame));
    assert_eq!(BEAT_KEYS[7..], [None, None], "the turntable closes the screen");
    assert_eq!(key_assignment(Mode::POPN_9K)[7..], [Some(ResultKey::Ok), Some(ResultKey::Ok)]);
    assert_eq!(
        key_assignment(Mode::KEYBOARD_24K)[..4],
        [Some(ResultKey::Ok), Some(ResultKey::ReplayDifferent), Some(ResultKey::ChangeGraph), Some(ResultKey::ReplaySame)]
    );
}

/// The fixture skin: a result skin stating the [`TIMES`] above, which paints the screen one colour
/// and a square for each timer the scene switches.
const SKIN: &str = include_str!("fixture/result.luaskin");

/// A result skin whose body raises, so it can never draw.
const BROKEN_SKIN: &str = r#"
if skin_config then
    error("no such asset")
end
return { type = 7, name = "Broken", w = 1280, h = 720 }
"#;

/// What the fixture paints the screen with, and its markers: listening and fading in the top
/// corners, the three score timers along the bottom.
const GROUND: Color = Color::rgb(20, 40, 80);
const LISTENING_MARK: Color = Color::rgb(0, 255, 0);
const FADING_MARK: Color = Color::rgb(255, 0, 0);
const SCORE_MARKS: [(u32, Color); 3] = [(10, Color::rgb(255, 255, 0)), (110, Color::rgb(255, 0, 255)), (210, Color::rgb(0, 255, 255))];

/// The squares the fixture paints along the bottom for what it is told of the run, each drawn only
/// while the skin reads the stated values back from its host: every note of the fixture chart a
/// PGREAT (EX 8, combo 4, a rate of 100); a clear with a lamp above FAILED and a new best; a failure
/// under the FAILED lamp; nothing scored with all four notes missed late; no score server; one set
/// up; and the timer of a submission under way.
const PERFECT_SCORE_MARK: (u32, Color) = (310, Color::rgb(255, 128, 0));
const CLEARED_MARK: (u32, Color) = (410, Color::rgb(0, 128, 255));
const FAILED_MARK: (u32, Color) = (510, Color::rgb(255, 0, 128));
const ALL_MISSED_LATE_MARK: (u32, Color) = (610, Color::rgb(128, 255, 0));
const OFFLINE_MARK: (u32, Color) = (710, Color::rgb(128, 0, 255));
const ONLINE_MARK: (u32, Color) = (810, Color::rgb(255, 255, 255));
const SENDING_MARK: (u32, Color) = (910, Color::rgb(0, 0, 255));

/// A pixel the submission lines cover when they are drawn over a skin: inside the strip of the
/// first line, which is also inside the fixture's fading square.
const IR_LINE_AT: (u32, u32) = (CW - 30, 20);

/// How far the fixture run is driven to play the whole chart out.
const WHOLE_RUN_US: i64 = 9_000_000;

/// How many notes the fixture chart has.
const RECORD_NOTES: u32 = 4;

/// The judgement a note that went by unplayed takes, as a score numbers its judgements.
const POOR: i32 = 4;

/// A pixel inside each top marker, one on bare ground, and the row the bottom markers are on.
const LISTENING_MARK_AT: (u32, u32) = (10, 10);
const FADING_MARK_AT: (u32, u32) = (CW - 10, 10);
const GROUND_AT: (u32, u32) = (CW / 2, 100);
const SCORE_MARK_ROW: u32 = CH - 10;

/// Frames a skin with no files is given to be read and compiled.
const SKIN_FRAMES: usize = 240;

/// Where the chart the fixture run was played on sits, and the chart after it.
const PLAYED_PATH: &str = "/songs/played.bms";
const AFTER_PATH: &str = "/songs/after.bms";

/// Four notes on one key lane, which is all a run needs to leave a record behind.
const RECORD_CHART: &[u8] = b"#PLAYER 1\n#TITLE Recorded\n#BPM 120\n#WAV01 a.wav\n#00111:01010101\n";

/// How far the fixture run is driven, and at what step: past the first two notes.
const RECORD_RUN_US: i64 = 3_000_000;
const RECORD_FRAME_US: i64 = 10_000;

fn song(path: &str, md5: &str) -> SongEntry {
    SongEntry {
        path: PathBuf::from(path),
        title: md5.to_string(),
        subtitle: String::new(),
        artist: String::new(),
        genre: String::new(),
        maker: String::new(),
        level: String::new(),
        difficulty: 0,
        init_bpm: 120.0,
        rank: 2,
        total: 100.0,
        mode: Mode::BEAT_7K,
        md5: md5.to_string(),
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }
}

/// Put the played chart and a second one after it in `app`'s library, played by the player on the
/// shipped seven-key layout, which is what puts every run-again key within reach.
fn with_library(mut app: App) -> App {
    app.shared.config.play.autoplay = false;
    app.shared.replay = None;
    app.shared.library = Library::from_songs(vec![song(PLAYED_PATH, "played"), song(AFTER_PATH, "after")]);
    app.shared.select_items = vec![SelectItem::Song(0), SelectItem::Song(1)];
    app.shared.chart_path = PLAYED_PATH.to_string();
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(app.shared.mode);
    app
}

/// An app with no skin at all, on a settings folder of this test's own.
fn plain_app() -> App {
    let dir = std::env::temp_dir().join(format!("rbms-result-stage-tests-{}-{:?}", std::process::id(), std::thread::current().id()));
    App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron"))
}

/// An app whose skin pack is one folder holding `skin` as its result skin, with the library above.
fn app_with_result_skin(tag: &str, skin: &str) -> App {
    let settings = settings_of(&format!("result-{tag}"));
    let pack: PathBuf = settings.with_file_name("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    std::fs::write(pack.join("result.luaskin"), skin).expect("the skin is written");
    let mut config = Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    with_library(app_in(settings, config))
}

fn view() -> ResultView {
    ResultView {
        title: "run".into(),
        artist: String::new(),
        mode_label: "7K",
        counts: [1, 0, 0, 0, 0, 0],
        ex_score: 2,
        max_score: 2,
        max_combo: 1,
        total_notes: 1,
        fast: [0, 0],
        slow: [0, 0],
        gauge: 100.0,
        clear_label: "CLEAR",
        clear_color: Color::GREEN,
        prev_best_ex: None,
        prev_ex: None,
        show_graph: true,
        show_result_graphs: true,
        gauge_series: Vec::new(),
        timing_hist: Box::new([]),
        judge_dist: [0; 6],
    }
}

fn state() -> ResultState {
    ResultState::new(view())
}

/// A run on [`RECORD_CHART`] that has been ticked from its start, so its gauges have a history.
fn finished_session() -> PlaySession {
    let src = rbms_parser::parse_with(RECORD_CHART, Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "record.bms"));
    let mut session = PlaySession::new(model, SessionOptions::default());
    for frame in 0..=RECORD_RUN_US / RECORD_FRAME_US {
        session.tick(SessionClock::at(frame * RECORD_FRAME_US), &mut NullSink);
    }
    session
}

/// The play screen at the end of a run of the fixture chart played out whole: by the autoplayer,
/// every note a PGREAT, or by nobody, every note missed.
fn played_out(autoplay: bool) -> PlayState {
    let src = rbms_parser::parse_with(RECORD_CHART, Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "record.bms"));
    let mut session = PlaySession::new(model, SessionOptions { autoplay, ..SessionOptions::default() });
    for frame in 0..=WHOLE_RUN_US / RECORD_FRAME_US {
        session.tick(SessionClock::at(frame * RECORD_FRAME_US), &mut NullSink);
    }
    PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// End a run of the fixture chart the way the application does and bring its result screen up,
/// drawn by the fixture skin, as far as the frame its scene begins on.
fn result_of_a_run(tag: &str, autoplay: bool) -> (App, HeadlessCanvas) {
    let mut app = app_with_result_skin(tag, SKIN);
    app.shared.config.play.autoplay = autoplay;
    let mut play = played_out(autoplay);
    let entered = enter_result(&mut play, &mut app.shared);
    assert!(matches!(entered, Transition::To(Stage::Result(_))), "the run did not end on its result screen");
    app.switch(entered);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    run_until_begun(&mut app, &mut pixels);
    (app, pixels)
}

/// The run the result screen that is up reports to a skin.
fn reported(app: &App) -> &ResultSnapshot {
    &result(app).run.as_ref().expect("the screen was built from a run").snapshot
}

fn mark_at(pixels: &HeadlessCanvas, (x, color): (u32, Color)) -> bool {
    pixels.pixel_at(x, SCORE_MARK_ROW) == color
}

/// How a fixture run stands when the test has nothing to say of it: reported under the normal
/// lamp, on a chart never played, against no target.
fn standing(replay_saved: bool) -> RunStanding {
    RunStanding { replay_saved, lamp: ClearType::Normal, previous: PreviousScore::default(), target: None }
}

/// A replay of the played chart, as the one a run was started from.
fn watched_replay() -> Replay {
    Replay {
        chart_path: PLAYED_PATH.to_string(),
        md5: String::new(),
        mode: String::new(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    }
}

/// Enter the result screen the way the application does.
fn enter(app: &mut App, state: ResultState) {
    app.switch(Transition::To(Stage::Result(state)));
    assert_eq!(app.stage.id(), StageId::Result);
}

/// One frame of whatever screen is up, as the application runs one: its update, and its draw
/// unless the update asked to leave.
fn frame(app: &mut App, pixels: &mut HeadlessCanvas) -> Transition {
    let now = Instant::now();
    let transition = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
    if matches!(transition, Transition::Stay) {
        app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, &mut Canvas::Headless(pixels));
    }
    transition
}

/// The result screen that is up.
fn result(app: &App) -> &ResultState {
    let Stage::Result(state) = &app.stage else {
        panic!("the {:?} screen is up, not the result screen", app.stage);
    };
    state
}

/// Run frames until the skin has drawn and the scene has begun.
fn run_until_begun(app: &mut App, pixels: &mut HeadlessCanvas) {
    for _ in 0..SKIN_FRAMES {
        assert!(matches!(frame(app, pixels), Transition::Stay), "the screen left before its scene began");
        if result(app).stage.scene.begun {
            return;
        }
    }
    panic!("the result skin never drew: {:?}", app.shared.skin_failure(SKIN_TYPE_RESULT));
}

/// Move the scene clock on by `millis` and run one frame.
fn frame_after_ms(app: &mut App, pixels: &mut HeadlessCanvas, millis: i64) -> Transition {
    app.shared.age_skin_scene(Duration::from_millis(u64::try_from(millis).expect("a scene only moves forward")));
    frame(app, pixels)
}

/// Send one key to the screen that is up, through the path the window's keys take.
fn key(app: &mut App, key: KeyInput<'_>) -> Transition {
    let now = Instant::now();
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key)
}

/// A result screen drawn by the fixture skin whose scene has begun and is listening.
fn listening_screen(tag: &str, state: ResultState) -> (App, HeadlessCanvas) {
    let mut app = app_with_result_skin(tag, SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, state);
    run_until_begun(&mut app, &mut pixels);
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, LISTENING_MS), Transition::Stay));
    assert!(takes_input(&app.shared.skin_timers), "the scene is not listening past its input time");
    (app, pixels)
}

/// The frame the skin first draws on is the scene's first moment: the clock is back at zero, the
/// three score timers are on from it, and the skin shows them while neither listening nor fading.
#[test]
fn the_scene_begins_on_the_frame_its_skin_first_draws() {
    const WAITED: Duration = Duration::from_secs(30);
    let mut app = app_with_result_skin("begins", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, state());
    app.shared.age_skin_scene(WAITED);
    run_until_begun(&mut app, &mut pixels);

    let now_us = app.shared.skin_now_us();
    assert!(now_us < WAITED.as_micros() as i64 / 2, "the scene clock was not started with the skin: {now_us} us");
    for timer in [timer_id::RESULTGRAPH_BEGIN, timer_id::RESULTGRAPH_END, timer_id::RESULT_UPDATESCORE] {
        assert!(app.shared.skin_timers.is_on(timer), "{timer:?} is not on");
        assert!(app.shared.skin_timers.value_us(timer) <= now_us, "{timer:?} was switched on against the old clock");
    }
    assert_eq!(pixels.pixel_at(GROUND_AT.0, GROUND_AT.1), GROUND, "the skin did not draw the screen");
    for (x, mark) in SCORE_MARKS {
        assert_eq!(pixels.pixel_at(x, SCORE_MARK_ROW), mark, "a score timer's mark is missing at {x}");
    }
    assert_ne!(pixels.pixel_at(LISTENING_MARK_AT.0, LISTENING_MARK_AT.1), LISTENING_MARK, "the scene listens from its first frame");
    assert_ne!(pixels.pixel_at(FADING_MARK_AT.0, FADING_MARK_AT.1), FADING_MARK);
}

/// A key pressed while the scene waits out its input time does nothing; the same key once it
/// listens starts the fade, and the screen leaves for the browser when the fade has run its time.
#[test]
fn a_confirming_key_is_ignored_until_the_scene_listens_and_then_fades_it_out() {
    let mut app = app_with_result_skin("confirm", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, state());
    run_until_begun(&mut app, &mut pixels);

    for code in [KeyCode::Enter, KeyCode::Escape] {
        assert!(matches!(key(&mut app, press(code)), Transition::Stay), "{code:?} left the screen at once");
        assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
        key(&mut app, release(code));
    }
    app.shared.note_key(&press(OK_KEY));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    app.shared.note_key(&release(OK_KEY));
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "a key acted while the scene was not listening");

    assert!(matches!(frame_after_ms(&mut app, &mut pixels, LISTENING_MS), Transition::Stay));
    assert_eq!(pixels.pixel_at(LISTENING_MARK_AT.0, LISTENING_MARK_AT.1), LISTENING_MARK);
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "a key tapped during the wait was acted on afterwards");

    key(&mut app, press(KeyCode::Enter));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay), "the screen left without fading");
    assert_eq!(pixels.pixel_at(FADING_MARK_AT.0, FADING_MARK_AT.1), FADING_MARK, "Enter did not start the fade");
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms / 2), Transition::Stay), "the screen left before its fade was over");
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms), Transition::Back), "the faded screen did not go back to the browser");
}

/// The lane keys are read by key index from whatever is held: the first key confirms, and the
/// sixth turns the gauge the screen shows without closing it.
#[test]
fn the_lane_keys_confirm_and_turn_the_gauge() {
    let (mut app, mut pixels) = listening_screen("lanes", state());
    let shown = result(&app).stage.scene.gauge_type;
    app.shared.note_key(&press(GRAPH_KEY));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    app.shared.note_key(&release(GRAPH_KEY));
    assert_eq!(result(&app).stage.scene.gauge_type, next_gauge_type(shown), "the sixth key did not turn the gauge");
    assert_eq!(result(&app).scene_facts(&app.shared).gauge_type, next_gauge_type(shown), "the gauge a skin is told is not the one on show");
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT));

    app.shared.note_key(&press(OK_KEY));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert!(app.shared.skin_timers.is_on(timer_id::FADEOUT), "the first key did not confirm");
}

/// R runs the chart again as it was laid out and N starts the chart after it, each once the fade
/// they start has run out. The run laid out as it was hands its seed to the load that follows.
#[test]
fn the_players_own_keys_start_another_run_when_the_fade_ends() {
    for (code, carries_seed) in [(KeyCode::KeyR, true), (KeyCode::KeyN, false)] {
        let session = finished_session();
        let mut app = app_with_result_skin(&format!("own-{code:?}"), SKIN);
        let run = ResultRun::of(&session, &app.shared, standing(false));
        assert!(run.updates_score, "a run the player played counts towards the records");
        let mut pixels = HeadlessCanvas::new(CW, CH);
        enter(&mut app, state().played(run));
        run_until_begun(&mut app, &mut pixels);

        assert!(matches!(key(&mut app, press(code)), Transition::Stay), "{code:?} started a run without waiting for the scene");
        assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
        key(&mut app, release(code));
        assert!(matches!(frame_after_ms(&mut app, &mut pixels, LISTENING_MS), Transition::Stay));
        assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "{code:?} tapped during the wait was acted on");

        key(&mut app, press(code));
        assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay), "{code:?} started a run without fading");
        key(&mut app, release(code));
        assert!(app.shared.skin_timers.is_on(timer_id::FADEOUT), "{code:?} did not start the fade");
        let moved = frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms + 1);
        assert!(matches!(moved, Transition::To(Stage::Loading(_))), "{code:?} did not start a run when the fade ended");
        assert_eq!(app.shared.retry_seed, carries_seed.then(|| session.seed()), "{code:?}: the arrangement handed on is wrong");
    }
}

/// The seventh key held through the fade runs the chart again as it was, and the fifth runs it on
/// a new arrangement: only the first hands the run's seed on.
#[test]
fn a_run_again_lane_key_held_through_the_fade_starts_the_run_it_names() {
    for (code, carries_seed) in [(AGAIN_KEY, true), (AFRESH_KEY, false)] {
        let session = finished_session();
        let mut app = app_with_result_skin(&format!("lane-{code:?}"), SKIN);
        let run = ResultRun::of(&session, &app.shared, standing(false));
        let mut pixels = HeadlessCanvas::new(CW, CH);
        enter(&mut app, state().played(run));
        run_until_begun(&mut app, &mut pixels);
        assert!(matches!(frame_after_ms(&mut app, &mut pixels, LISTENING_MS), Transition::Stay));

        app.shared.note_key(&press(code));
        assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
        assert!(app.shared.skin_timers.is_on(timer_id::FADEOUT), "{code:?} did not start the fade");
        let moved = frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms + 1);
        assert!(matches!(moved, Transition::To(Stage::Loading(_))), "{code:?} held through the fade did not start a run");
        assert_eq!(app.shared.retry_seed, carries_seed.then(|| session.seed()));
    }
}

/// The result of a replay that was watched has nothing to run again: R and N do nothing, the
/// seventh key only closes the screen, and the first replay slot reports the replay that exists.
#[test]
fn the_result_of_a_watched_replay_closes_without_offering_another_run() {
    let session = finished_session();
    let mut app = app_with_result_skin("watched", SKIN);
    app.shared.replay = Some(watched_replay());
    let run = ResultRun::of(&session, &app.shared, standing(false));
    assert_eq!(run.replay, ReplaySlot::Exists, "the replay being watched is not reported as stored");
    assert!(!run.updates_score);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, state().played(run));
    run_until_begun(&mut app, &mut pixels);
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, LISTENING_MS), Transition::Stay));
    assert_eq!(result(&app).scene_facts(&app.shared).replay, [ReplaySlot::Exists, ReplaySlot::Missing, ReplaySlot::Missing, ReplaySlot::Missing]);

    for code in [KeyCode::KeyR, KeyCode::KeyN] {
        key(&mut app, press(code));
        assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
        key(&mut app, release(code));
        assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "{code:?} closed the result of a replay");
    }
    app.shared.note_key(&press(AGAIN_KEY));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert!(app.shared.skin_timers.is_on(timer_id::FADEOUT), "the seventh key did not confirm");
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms + 1), Transition::Back), "a watched replay was run again");
    assert_eq!(app.shared.retry_seed, None);
}

/// A run that ended is handed to the skin's host: an autoplayed run of four PGREATs reads as EX 8,
/// a combo of 4, a rate of 100, a clear and a new best, and nothing of a failure.
#[test]
fn a_run_that_cleared_reaches_its_skin_as_the_score_it_made() {
    let (app, pixels) = result_of_a_run("cleared-run", true);
    let run = reported(&app);
    assert_eq!((run.score.sheet.ex_score(), run.score.sheet.max_combo, run.score.sheet.notes), (RECORD_NOTES * 2, RECORD_NOTES, RECORD_NOTES));
    assert!(run.cleared() && run.score.sheet.clear > rbms_judge::clear_type_id(ClearType::Failed), "the lamp is {}", run.score.sheet.clear);
    assert!(!run.updates_score, "an autoplayed run counts towards no record");

    assert!(mark_at(&pixels, PERFECT_SCORE_MARK), "the skin did not read EX 8, combo 4 and a rate of 100 back");
    assert!(mark_at(&pixels, CLEARED_MARK), "the skin did not read a clear, its lamp and a new best back");
    assert!(!mark_at(&pixels, FAILED_MARK) && !mark_at(&pixels, ALL_MISSED_LATE_MARK), "a cleared run reads as a failure");
}

/// A run nobody played fails with every note missed late: the skin reads the FAILED lamp, four POOR
/// and all four of them on the late side. The best it is held against is the one that stood before
/// the run joined the book, which for a chart never played is none.
#[test]
fn a_run_that_failed_reaches_its_skin_with_its_misses_on_the_late_side() {
    let (app, pixels) = result_of_a_run("failed-run", false);
    let run = reported(&app);
    assert_eq!((run.score.sheet.ex_score(), run.score.sheet.max_combo), (0, 0));
    assert_eq!((run.score.sheet.count(POOR), run.score.sheet.count_on(POOR, false)), (RECORD_NOTES, RECORD_NOTES), "the misses are not late ones");
    assert_eq!(run.score.sheet.clear, rbms_judge::clear_type_id(ClearType::Failed));
    assert!(!run.cleared() && run.updates_score);
    assert_eq!(run.previous, PreviousScore::default(), "the run was held against a book it had already joined");
    assert_eq!(app.shared.scores.for_md5(&result(&app).run.as_ref().expect("a run").chart.md5).len(), 1, "the run did not join the book");

    assert!(mark_at(&pixels, FAILED_MARK), "the skin did not read the failure and its lamp back");
    assert!(mark_at(&pixels, ALL_MISSED_LATE_MARK), "the skin did not read four late POOR back");
    assert!(!mark_at(&pixels, CLEARED_MARK) && !mark_at(&pixels, PERFECT_SCORE_MARK), "a failed run reads as a clear");
}

/// A screen built without a run tells the result clusters nothing, so a skin reads none of the
/// run's marks from it.
#[test]
fn a_screen_built_without_a_run_reports_no_score_to_its_skin() {
    let (_, pixels) = listening_screen("no-run", state());
    for mark in [PERFECT_SCORE_MARK, CLEARED_MARK, ALL_MISSED_LATE_MARK] {
        assert!(!mark_at(&pixels, mark), "a mark of a run is drawn at {} with no run on the screen", mark.0);
    }
}

/// The ranking cluster is told whether a score server is set up, on every frame: a skin reads
/// offline without one and online with one.
#[test]
fn a_skin_reads_whether_a_score_server_is_set_up() {
    let (mut app, mut pixels) = listening_screen("ir-link", state());
    assert!(!app.shared.has_primary_ir_server(), "the fixture app has a score server");
    assert!(mark_at(&pixels, OFFLINE_MARK) && !mark_at(&pixels, ONLINE_MARK), "a player with no score server does not read as offline");

    let network = rbms_config::NetworkOptions { server_url: Some("http://127.0.0.1:9".to_string()), ..app.shared.config.network.clone() };
    app.shared.multi_ir = MultiIr::from_network(&network);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert!(mark_at(&pixels, ONLINE_MARK) && !mark_at(&pixels, OFFLINE_MARK), "a player with a score server does not read as online");
}

/// The submission lines are this player's own, drawn over the skin: not at all when no server is
/// set up, while a submission has something to say, and no longer once the scene fades.
#[test]
fn the_submission_lines_are_drawn_over_a_skin_only_while_they_say_something() {
    let (mut app, mut pixels) = listening_screen("ir-lines", state());
    assert_eq!(app.shared.ir_status, IrStatus::Off);
    assert_eq!(pixels.pixel_at(IR_LINE_AT.0, IR_LINE_AT.1), GROUND, "a line was drawn with no score server set up");

    app.shared.ir_status = IrStatus::Skipped("autoplay");
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert_ne!(pixels.pixel_at(IR_LINE_AT.0, IR_LINE_AT.1), GROUND, "a run that was not sent does not say why");

    key(&mut app, press(KeyCode::Enter));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert!(app.shared.skin_timers.is_on(timer_id::FADEOUT));
    assert_eq!(pixels.pixel_at(IR_LINE_AT.0, IR_LINE_AT.1), FADING_MARK, "a line is still drawn over the fading scene");
}

/// Until the skin can be drawn the screen is black and takes no key but Escape, which leaves at
/// once: there is no scene yet to fade out.
#[test]
fn a_screen_waiting_for_its_skin_is_black_and_takes_escape_alone() {
    let mut app = app_with_result_skin("reading", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, state());
    assert_eq!(result(&app).stage.skin, SkinStatus::Reading);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    if result(&app).stage.skin == SkinStatus::Reading {
        assert_eq!(pixels.pixel_at(GROUND_AT.0, GROUND_AT.1), Color::BLACK, "something was drawn before the skin was");
    }
    assert!(matches!(key(&mut app, press(KeyCode::Enter)), Transition::Stay), "Enter left a screen whose scene had not begun");
    assert!(matches!(key(&mut app, press(KeyCode::KeyR)), Transition::Stay));

    let mut waiting = app_with_result_skin("reading-escape", SKIN);
    enter(&mut waiting, state());
    assert!(matches!(key(&mut waiting, press(KeyCode::Escape)), Transition::Back), "Escape did not leave a screen still waiting for its skin");
}

/// A skin that cannot be read hands the screen to the built-in layout, whose keys act at once.
#[test]
fn a_skin_that_cannot_be_read_leaves_the_built_in_screen_and_its_keys() {
    let mut app = app_with_result_skin("broken", BROKEN_SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, state());
    for _ in 0..SKIN_FRAMES {
        assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
        if result(&app).stage.skin == SkinStatus::Gone {
            break;
        }
    }
    assert_eq!(result(&app).stage.skin, SkinStatus::Gone, "a skin that raised was still waited for");
    assert!(pixels.painted_pixels() > 0, "the built-in screen did not take over");
    assert!(matches!(key(&mut app, press(KeyCode::KeyR)), Transition::To(Stage::Loading(_))), "the built-in run-again key did not act at once");
    assert_eq!(app.shared.retry_seed, None, "the built-in run-again key handed an arrangement on");
}

/// What the run left reaches a skin in the shapes its objects read: every gauge's history, the
/// gauge on show with its last value, the timing spread a millisecond to a count with the judge
/// windows, and the chart's notes and tempo.
#[test]
fn the_run_reaches_a_skin_as_the_series_its_graphs_plot() {
    let session = finished_session();
    let app = with_library(plain_app());
    let run = ResultRun::of(&session, &app.shared, standing(true));
    let record = session.record();
    assert_eq!(run.replay, ReplaySlot::Saved);
    assert_eq!(run.gauges.len(), GAUGE_TYPES);
    assert!(run.gauges.iter().all(|history| history.len() == record.gauge_log.len()) && !record.gauge_log.is_empty());
    assert!(run.judge_area[0][0] < 0 && run.judge_area[0][1] > 0, "the best judgement has no window: {:?}", run.judge_area);
    assert!(run.judge_area.windows(2).all(|pair| pair[1][0] <= pair[0][0] && pair[1][1] >= pair[0][1]), "the windows do not widen: {:?}", run.judge_area);

    let state = ResultState::new(ResultView { judge_dist: [1, 2, 3, 4, 5, 6], ..view() }).played(run);
    assert_eq!(state.stage.scene.gauge_type, record.finished_gauge.index(), "the graph does not open on the gauge the run ended on");
    let run = state.run.as_ref().expect("the screen keeps its run");
    let series = state.series();
    let history = series.gauge_history.expect("a gauge history");
    assert_eq!(history.of(record.finished_gauge.index()), Some(record.gauge_log.of(record.finished_gauge)));
    let timing = series.timing.expect("a timing spread");
    assert_eq!(timing.bins, record.timing.buckets());
    assert_eq!((timing.average, timing.std_dev, timing.judge_area), (record.timing.average(), record.timing.std_dev(), run.judge_area));
    let notes = series.notes.expect("a note distribution");
    assert_eq!(*notes.judged, [1, 2, 3, 4, 5, 6]);
    assert_eq!((notes.judgements, notes.early_late), (record.seconds.by_judge.as_slice(), record.seconds.by_timing.as_slice()));
    assert!(!notes.kinds.is_empty(), "the chart's notes by kind are missing");
    assert!(!series.bpm.expect("a tempo graph").changes.is_empty());

    let gauge = run.gauge(record.finished_gauge.index());
    assert_eq!(gauge.value, record.gauge_log.last(record.finished_gauge));
    assert!(gauge.result, "the gauge is not a finished run's");
}

/// The wheel moves the ranking between its first row and its last, and stays put when there is no
/// ranking to move through.
#[test]
fn the_wheel_scrolls_the_ranking_within_its_rows() {
    const ROWS: usize = 5;
    let session = finished_session();
    let mut app = app_with_result_skin("wheel", SKIN);
    let run = ResultRun::of(&session, &app.shared, standing(false));
    let md5 = run.chart.md5.clone();
    enter(&mut app, state().played(run));
    let scroll = |app: &mut App, lines: f32| {
        let now = Instant::now();
        app.stage.handle_pointer(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, (0.0, 0.0), crate::pointer::PointerInput::Scroll { lines });
        app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
        result(app).scene_facts(&app.shared).ranking_offset
    };
    assert_eq!(scroll(&mut app, 3.0), 0, "a ranking with nobody in it was scrolled");

    let row = RankingRow { rank: None, player: "p".into(), ex_score: 1, lamp: "CLEAR", lamp_color: Color::GREEN, fast_slow: None, replay_id: None };
    let board = RankingBoard { rows: vec![row; ROWS], you: None, rivals: Vec::new() };
    app.shared.ranking_cache.insert(md5, RankingState::Ready(Box::new(board)));
    assert_eq!(result(&app).scene_facts(&app.shared).ranking_total, ROWS as i32);
    assert_eq!(scroll(&mut app, 3.0), 3);
    assert_eq!(scroll(&mut app, 10.0), ROWS as i32 - 1, "the ranking was scrolled past its last row");
    assert_eq!(scroll(&mut app, -20.0), 0, "the ranking was scrolled above its first row");

    const PART_OF_A_LINE: f32 = 0.4;
    assert_eq!(scroll(&mut app, PART_OF_A_LINE), 0, "less than a line moved the ranking");
    assert_eq!(scroll(&mut app, PART_OF_A_LINE), 0);
    assert_eq!(scroll(&mut app, PART_OF_A_LINE), 1, "turns of less than a line never added up to one");
    assert_eq!(scroll(&mut app, -PART_OF_A_LINE), 1, "the part of a line left over was thrown away");
    assert_eq!(scroll(&mut app, -PART_OF_A_LINE), 1);
    assert_eq!(scroll(&mut app, -PART_OF_A_LINE), 0);
}

/// What the skin asks of the screen is carried out on the next frame: the favourite button stars
/// the chart that was played, and the replay slots save nothing, the run's replay being saved by
/// itself.
#[test]
fn the_skins_events_are_carried_out_by_the_screen() {
    let session = finished_session();
    let (mut app, mut pixels) = listening_screen("events", state());
    let run = ResultRun::of(&session, &app.shared, standing(false));
    let md5 = run.chart.md5.clone();
    let Stage::Result(up) = &mut app.stage else {
        panic!("the result screen is not up");
    };
    up.run = Some(Box::new(run));

    for id in [BUTTON_REPLAY, BUTTON_REPLAY2, BUTTON_REPLAY3, BUTTON_REPLAY4, BUTTON_FAVORITTE_CHART] {
        app.shared.skin_requests().push(Cluster::Result, ClusterRequest::Event { id, arg1: 1, arg2: 0 });
    }
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay), "an event of the skin's left the screen");
    assert!(app.shared.favorites.contains(&md5), "the favourite button did not star the chart");
    assert_eq!(reported(&app).favorite_chart, Some(true), "the skin's favourite button is not told the chart is starred now");
    assert_eq!(result(&app).scene_facts(&app.shared).replay[0], ReplaySlot::Missing, "a save event changed a replay slot");
    assert!(app.shared.skin_requests().take(Cluster::Result).is_empty(), "the screen left its own requests waiting");
}

/// The score server's timers follow the submission: one when the run is sent, and one for how it
/// ended.
#[test]
fn the_submission_switches_the_score_servers_timers() {
    let (mut app, mut pixels) = listening_screen("submission", state());
    assert!(!app.shared.skin_timers.is_on(timer_id::IR_CONNECT_BEGIN), "a run that was never sent started the connection timer");
    assert!(!mark_at(&pixels, SENDING_MARK));

    app.shared.ir_status = IrStatus::Sending;
    frame(&mut app, &mut pixels);
    assert!(app.shared.skin_timers.is_on(timer_id::IR_CONNECT_BEGIN));
    assert!(mark_at(&pixels, SENDING_MARK), "the skin does not show the submission under way");
    assert!(!app.shared.skin_timers.is_on(timer_id::IR_CONNECT_SUCCESS) && !app.shared.skin_timers.is_on(timer_id::IR_CONNECT_FAIL));

    let unanswered = rbms_ir::SubmitOutcome { submit: Err(rbms_ir::IrError::Network("no answer".into())), replay: None };
    app.shared.ir_status = IrStatus::Reported(format_submit_outcome(&unanswered));
    frame(&mut app, &mut pixels);
    assert!(app.shared.skin_timers.is_on(timer_id::IR_CONNECT_FAIL), "a failed submission did not start its timer");
    assert!(!app.shared.skin_timers.is_on(timer_id::IR_CONNECT_SUCCESS));
}

#[test]
fn without_a_skin_the_leaving_keys_go_back_to_the_browser_at_once() {
    for code in [KeyCode::Escape, KeyCode::Enter, KeyCode::NumpadEnter] {
        let mut app = plain_app();
        app.shared.config.play.autoplay = false;
        enter(&mut app, state());
        assert_eq!(result(&app).stage.skin, SkinStatus::Gone);
        assert!(!matches!(key(&mut app, press(code)), Transition::Stay), "{code:?} did not leave the result screen");
    }
}

/// Without a skin both run-again keys start a chart at once, so the LOADING screen replaces the
/// result and the browser stays suspended underneath for the next run to return to. Neither hands
/// an arrangement on.
#[test]
fn without_a_skin_the_run_again_keys_start_another_run_at_once() {
    for code in [KeyCode::KeyR, KeyCode::KeyN] {
        let mut app = with_library(plain_app());
        enter(&mut app, state());
        assert!(matches!(key(&mut app, press(code)), Transition::To(Stage::Loading(_))), "{code:?} did not start a run");
        assert_eq!(app.shared.retry_seed, None);
    }
}

/// A run the player did not play has nothing to repeat, and a chart that is not in the library
/// has nothing to start again from, so neither key may start a run in either case.
#[test]
fn without_a_skin_the_run_again_keys_stand_down_for_a_run_that_cannot_be_repeated() {
    let mut demonstration = with_library(plain_app());
    demonstration.shared.config.play.autoplay = true;
    let mut shown = state();
    let tap = |app: &mut App, state: &mut ResultState, code: KeyCode| {
        let now = Instant::now();
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(code))
    };
    assert!(matches!(tap(&mut demonstration, &mut shown, KeyCode::KeyR), Transition::Stay));
    assert!(matches!(tap(&mut demonstration, &mut shown, KeyCode::KeyN), Transition::Stay));

    let mut loose = plain_app();
    loose.shared.config.play.autoplay = false;
    let mut played = state();
    assert!(matches!(tap(&mut loose, &mut played, KeyCode::KeyR), Transition::Stay), "there is no library to start from");
    assert!(matches!(tap(&mut loose, &mut played, KeyCode::KeyN), Transition::Stay));
}

/// A key going back up is not a press, or every leaving key would fire twice.
#[test]
fn without_a_skin_a_released_key_does_nothing() {
    let mut app = plain_app();
    let mut state = state();
    let now = Instant::now();
    let moved = state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, release(KeyCode::Escape));
    assert!(matches!(moved, Transition::Stay));
}

/// Without a skin nothing about the screen waits: its update asks for nothing, however old the
/// scene is, and the score timers are on from the moment it opened.
#[test]
fn without_a_skin_the_screen_runs_no_scene() {
    let mut app = with_library(plain_app());
    let mut pixels = HeadlessCanvas::new(CW, CH);
    enter(&mut app, state());
    for timer in [timer_id::RESULTGRAPH_BEGIN, timer_id::RESULTGRAPH_END, timer_id::RESULT_UPDATESCORE] {
        assert!(app.shared.skin_timers.is_on(timer), "{timer:?} is not on as the screen opens");
    }
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.scene_ms), Transition::Stay), "the built-in screen left on its own");
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT) && !app.shared.skin_timers.is_on(timer_id::STARTINPUT));
    assert!(pixels.painted_pixels() > 0);
}

/// The chart's tempo reaches the screen as one point per tempo it holds, each placed by how far
/// through the chart that tempo started.
#[test]
fn the_charts_tempo_changes_become_the_points_a_document_plots() {
    use crate::app_result::tempo_points;
    use rbms_model::TimeLine;

    const LANES: usize = 8;
    let lines = vec![
        TimeLine::empty(LANES, 0, 0.0, 120.0),
        TimeLine::empty(LANES, 1_000_000, 1.0, 120.0),
        TimeLine::empty(LANES, 2_000_000, 2.0, 180.0),
        TimeLine::empty(LANES, 4_000_000, 4.0, 180.0),
    ];
    assert_eq!(tempo_points(&lines), vec![(0.0, 120.0), (0.5, 180.0)], "a tempo is carried once, at the progress it started from");
    assert!(tempo_points(&[]).is_empty(), "a chart with no lines has no tempo to plot");
    assert!(tempo_points(&lines[..1]).is_empty(), "a chart with no length has nothing to place a point on");
}

/// A screen built without a run still hands a document the measurements the built-in panels are
/// drawn from, and the tempo it was given.
#[test]
fn a_screen_built_without_a_run_hands_over_the_views_measurements() {
    let state = ResultState::new(ResultView { gauge_series: vec![50.0, 60.0], judge_dist: [1, 2, 3, 4, 5, 6], ..view() }).tempo(vec![(0.0, 150.0)]);
    let series = state.series();
    assert_eq!(series.gauge_history.map(|history| history.samples), Some(&[50.0, 60.0][..]), "the gauge the run held is not the one the document reads");
    assert_eq!(series.notes.map(|notes| *notes.judged), Some([1, 2, 3, 4, 5, 6]), "the judgements the run took are not the ones the document reads");
    assert_eq!(series.bpm.map(|timeline| timeline.points), Some(&[(0.0, 150.0)][..]), "the chart's tempo is not the one the document reads");
}

/// What surrounds the run reaches the built-in screen: a screen built with a target and the
/// run-again offer cannot paint the same frame as the bare one.
#[test]
fn the_target_and_the_run_again_offer_are_carried_onto_the_screen() {
    let mut app = with_library(plain_app());
    let paint = |app: &mut App, state: &mut ResultState| {
        rbms_render::font::use_embedded_fonts_only();
        let mut pixels = HeadlessCanvas::new(CW, CH);
        let mut canvas = Canvas::Headless(&mut pixels);
        let now = Instant::now();
        app.shared.hot.clear();
        state.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, &mut canvas);
        pixels.signature()
    };
    let bare = paint(&mut app, &mut state());
    let extras = ResultExtras { target: Some(TargetView { name: "RANK AAA".into(), ex: 2 }), run_again: true };
    let paced = paint(&mut app, &mut ResultState::new(view()).paced_by(extras));
    assert_ne!(bare, paced, "the target and the run-again hint are not drawn");
}
