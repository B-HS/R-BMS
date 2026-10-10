//! The decide scene, first as the bare state machine with a clock the test counts, then as the
//! screen: drawn by a small skin of its own, entered from the browser, and left for the run, the
//! LOADING screen or the browser. Last come the ways in that go wrong before there is a scene at
//! all: a chart that cannot be read, and a skin whose files never arrive.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rbms_course::{Course, CourseChart, CourseRun};
use rbms_library::{Library, SongEntry};
use rbms_skin::timer::{MICROS_PER_MILLI, timer_id};

use super::*;
use crate::app_play::loaded_chart_for_tests;
use crate::notify::Level;
use crate::skin_screen::texture_tests::HandDecode;
use crate::stage::capture::{app_in, settings_of};
use crate::stage::render_tests::FRAME_DT;
use crate::stage::select::tests::{entry, press, release};
use crate::stage::{HeadlessCanvas, KeysoundLoad, SelectState, StageId};
use crate::{App, CH, CW, Config};

/// The times the scene tests run on, which are the fixture skin's own.
const TIMES: SceneTimes = SceneTimes { input_ms: 2000, scene_ms: 10_000, fadeout_ms: 4000 };

/// Nothing held down.
const IDLE: HeldInput = HeldInput { skip: false, start_and_select: false };

/// A key that ends the scene early held down.
const SKIPPING: HeldInput = HeldInput { skip: true, start_and_select: false };

/// START and SELECT held down together.
const CANCELLING: HeldInput = HeldInput { skip: false, start_and_select: true };

fn at_ms(millis: i64) -> i64 {
    millis * MICROS_PER_MILLI
}

/// The first moment the scene listens, and a moment while it does.
const LISTENING_MS: i64 = TIMES.input_ms + 1;
const WHILE_LISTENING_MS: i64 = TIMES.input_ms + 500;

/// A scene that has begun on [`TIMES`], with its timers.
fn begun() -> (DecideScene, TimerState) {
    let mut scene = DecideScene::default();
    assert!(scene.begin(TIMES));
    (scene, TimerState::new())
}

/// A scene is begun once, by the frame its skin first draws on, and keeps the times it began with:
/// that frame is the only one that plays the decide cue.
#[test]
fn a_scene_begins_once_however_many_frames_its_skin_draws() {
    let mut scene = DecideScene::default();
    let begins = (0..5).filter(|_| scene.begin(TIMES)).count();
    assert_eq!(begins, 1, "the scene began, and so played its cue, more than once");
    assert!(!scene.begin(SceneTimes::default()));
    assert_eq!(scene.times, TIMES, "beginning again replaced the scene's times");
}

/// Until its skin has drawn, a scene has no clock to run on: nothing times out and no key acts.
#[test]
fn a_scene_that_has_not_begun_neither_runs_nor_listens() {
    let mut scene = DecideScene::default();
    let mut timers = TimerState::new();
    scene.key(&press(KeyCode::Enter));
    assert_eq!(scene.step(&mut timers, at_ms(TIMES.scene_ms + TIMES.fadeout_ms + 10_000), SKIPPING), None);
    assert!(!timers.is_on(timer_id::STARTINPUT) && !timers.is_on(timer_id::FADEOUT));
}

/// A key held from before the scene listens does nothing until it does, and then ends the scene at
/// once: the reference reads the key's state, not its press.
#[test]
fn a_skip_key_held_before_the_scene_listens_acts_the_moment_it_does() {
    let (mut scene, mut timers) = begun();
    assert_eq!(scene.step(&mut timers, at_ms(TIMES.input_ms), SKIPPING), None);
    assert!(!timers.is_on(timer_id::FADEOUT), "a key acted before the scene took input");

    assert_eq!(scene.step(&mut timers, at_ms(LISTENING_MS), SKIPPING), None);
    assert_eq!(timers.value_us(timer_id::FADEOUT), at_ms(LISTENING_MS), "the held key did not start the fade when input opened");
}

/// Enter and Escape pressed and let go again before the scene listens are forgotten; one that is
/// still down when it starts listening is acted on then.
#[test]
fn a_key_tapped_before_the_scene_listens_is_ignored_and_one_still_down_is_not() {
    let (mut tapped, mut timers) = begun();
    for code in [KeyCode::Enter, KeyCode::Escape] {
        tapped.key(&press(code));
        tapped.step(&mut timers, at_ms(TIMES.input_ms / 2), IDLE);
        tapped.key(&release(code));
    }
    assert_eq!(tapped.step(&mut timers, at_ms(WHILE_LISTENING_MS), IDLE), None);
    assert!(!timers.is_on(timer_id::FADEOUT) && !tapped.cancelled, "a tap made before the scene took input was acted on");

    let (mut held, mut timers) = begun();
    held.key(&press(KeyCode::Enter));
    held.step(&mut timers, at_ms(TIMES.input_ms / 2), IDLE);
    assert!(!timers.is_on(timer_id::FADEOUT));
    held.step(&mut timers, at_ms(LISTENING_MS), IDLE);
    assert!(timers.is_on(timer_id::FADEOUT), "Enter still down when input opened did not skip");
}

/// Enter, or either Enter key, starts the fade without waiting for the scene's own length, and the
/// scene leaves for the run once the fade has run its time.
#[test]
fn enter_skips_to_the_fade_and_the_scene_leaves_for_the_run_when_it_is_over() {
    for code in [KeyCode::Enter, KeyCode::NumpadEnter] {
        let (mut scene, mut timers) = begun();
        scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), IDLE);
        scene.key(&press(code));
        assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), IDLE), None);
        assert_eq!(timers.value_us(timer_id::FADEOUT), at_ms(WHILE_LISTENING_MS), "{code:?} did not start the fade");

        assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS + TIMES.fadeout_ms), IDLE), None, "the scene left before its fade was over");
        assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS + TIMES.fadeout_ms + 1), IDLE), Some(Leave::Onward));
    }
}

/// Left alone, the scene fades when its length has passed and leaves for the run after the fade.
#[test]
fn a_scene_left_alone_runs_its_length_and_leaves_for_the_run() {
    let (mut scene, mut timers) = begun();
    assert_eq!(scene.step(&mut timers, at_ms(TIMES.scene_ms), IDLE), None);
    assert!(!timers.is_on(timer_id::FADEOUT));
    assert_eq!(scene.step(&mut timers, at_ms(TIMES.scene_ms + 1), IDLE), None);
    assert!(timers.is_on(timer_id::FADEOUT));
    assert_eq!(scene.step(&mut timers, at_ms(TIMES.scene_ms + 1 + TIMES.fadeout_ms + 1), IDLE), Some(Leave::Onward));
}

/// Escape, or START and SELECT together, starts the same fade and turns the way out around. The
/// scene still waits the fade out before it leaves.
#[test]
fn escape_or_start_with_select_fades_out_to_the_browser() {
    let (mut by_key, mut timers) = begun();
    by_key.step(&mut timers, at_ms(WHILE_LISTENING_MS), IDLE);
    by_key.key(&press(KeyCode::Escape));
    assert_eq!(by_key.step(&mut timers, at_ms(WHILE_LISTENING_MS), IDLE), None);
    assert_eq!(by_key.step(&mut timers, at_ms(WHILE_LISTENING_MS + TIMES.fadeout_ms), IDLE), None, "a cancelled scene left before its fade was over");
    assert_eq!(by_key.step(&mut timers, at_ms(WHILE_LISTENING_MS + TIMES.fadeout_ms + 1), IDLE), Some(Leave::Cancelled));

    let (mut by_buttons, mut timers) = begun();
    by_buttons.step(&mut timers, at_ms(WHILE_LISTENING_MS), CANCELLING);
    assert_eq!(by_buttons.step(&mut timers, at_ms(WHILE_LISTENING_MS + TIMES.fadeout_ms + 1), IDLE), Some(Leave::Cancelled));
}

/// Once the fade has begun the keys that end the scene do nothing more: a skip is not turned into
/// a cancel, and the fade is not started over.
#[test]
fn a_fading_scene_takes_no_more_keys() {
    let (mut scene, mut timers) = begun();
    scene.step(&mut timers, at_ms(WHILE_LISTENING_MS), SKIPPING);
    scene.key(&press(KeyCode::Escape));
    assert_eq!(scene.step(&mut timers, at_ms(WHILE_LISTENING_MS + 100), CANCELLING), None);
    assert_eq!(timers.value_us(timer_id::FADEOUT), at_ms(WHILE_LISTENING_MS), "a key pressed during the fade started it again");
    assert_eq!(
        scene.step(&mut timers, at_ms(WHILE_LISTENING_MS + TIMES.fadeout_ms + 1), IDLE),
        Some(Leave::Onward),
        "a key pressed during the fade cancelled the scene"
    );
}

/// What the scene reads from the keys: one of the keys 1, 3, 5 and 7 down is a skip and none of the
/// others is, and START with SELECT counts only while both are down. The shipped seven-key layout
/// plays keys 1 to 7 on Z S X D C F V, and START and SELECT are on A and W.
#[test]
fn the_scene_reads_the_odd_keys_as_a_skip_and_start_with_select_as_a_cancel() {
    const SKIP_KEYS: [KeyCode; 4] = [KeyCode::KeyZ, KeyCode::KeyX, KeyCode::KeyC, KeyCode::KeyV];
    const OTHER_KEYS: [KeyCode; 4] = [KeyCode::KeyS, KeyCode::KeyD, KeyCode::KeyF, KeyCode::ShiftLeft];
    const START_KEY: KeyCode = KeyCode::KeyA;
    const SELECT_KEY: KeyCode = KeyCode::KeyW;
    let mut app = crate::stage::render_tests::app();
    app.shared.mode = rbms_model::Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(app.shared.mode);
    assert_eq!(HeldInput::read(&app.shared), IDLE);

    for code in SKIP_KEYS {
        app.shared.note_key(&press(code));
        assert_eq!(HeldInput::read(&app.shared), SKIPPING, "{code:?} is not read as a skip");
        app.shared.note_key(&release(code));
    }
    for code in OTHER_KEYS {
        app.shared.note_key(&press(code));
        assert_eq!(HeldInput::read(&app.shared), IDLE, "{code:?} is read as a skip");
        app.shared.note_key(&release(code));
    }

    app.shared.note_key(&press(START_KEY));
    assert_eq!(HeldInput::read(&app.shared), IDLE, "START alone cancels");
    app.shared.note_key(&press(SELECT_KEY));
    assert_eq!(HeldInput::read(&app.shared), CANCELLING);
    app.shared.note_key(&release(START_KEY));
    assert_eq!(HeldInput::read(&app.shared), IDLE, "SELECT alone cancels");
}

/// The fixture skin: a decide skin stating the [`TIMES`] above, which paints the screen one colour,
/// a green square in the top left corner while `STARTINPUT` is on and a red one in the top right
/// while `FADEOUT` is.
const SKIN: &str = include_str!("fixture/decide.luaskin");

/// A decide skin whose body raises, so it can never draw.
const BROKEN_SKIN: &str = r#"
if skin_config then
    error("no such asset")
end
return { type = 6, name = "Broken", w = 1280, h = 720 }
"#;

/// What the fixture paints the screen with, and its two markers.
const GROUND: Color = Color::rgb(20, 40, 80);
const LISTENING_MARK: Color = Color::rgb(0, 255, 0);
const FADING_MARK: Color = Color::rgb(255, 0, 0);

/// A pixel inside each marker, and one in the middle of the screen.
const LISTENING_MARK_AT: (u32, u32) = (10, 10);
const FADING_MARK_AT: (u32, u32) = (CW - 10, 10);
const MIDDLE: (u32, u32) = (CW / 2, CH / 2);

/// Frames a skin with no files is given to be read and compiled.
const SKIN_FRAMES: usize = 240;

/// The chart the scene is shown for: four notes in its first measure and two in its third.
const CHART: &[u8] =
    b"#PLAYER 1\n#TITLE Decided\n#ARTIST Composer\n#GENRE Fixture\n#PLAYLEVEL 7\n#DIFFICULTY 3\n#BPM 120\n#WAV01 a.wav\n#00111:01010101\n#00211:0101\n";
const CHART_NAME: &str = "decided.bms";

/// How many keysounds the chart of a load that is still running is waiting for.
const PENDING_KEYSOUNDS: usize = 4;

/// The file a Lua decide skin is written to in a pack.
const LUA_SKIN_FILE: &str = "decide.luaskin";

/// An app whose skin pack is one folder holding `skin` as its decide skin, a Lua one.
fn app_with_decide_skin(tag: &str, skin: &str) -> App {
    app_with_decide_document(tag, LUA_SKIN_FILE, skin)
}

/// An app whose skin pack is one folder holding `body`, under the name `file`, as its decide skin.
fn app_with_decide_document(tag: &str, file: &str, body: &str) -> App {
    let settings = settings_of(&format!("decide-{tag}"));
    let pack: PathBuf = settings.with_file_name("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    std::fs::write(pack.join(file), body).expect("the skin is written");
    let mut config = Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    app_in(settings, config)
}

/// The fixture chart with nothing left to decode.
fn chart() -> LoadedChart {
    loaded_chart_for_tests(CHART, CHART_NAME)
}

/// The fixture chart with its keysounds still decoding, and the flag that stops them.
fn chart_still_loading() -> (LoadedChart, Arc<AtomicBool>) {
    let cancel = Arc::new(AtomicBool::new(false));
    let (_sender, rx) = std::sync::mpsc::channel();
    let keysounds = KeysoundLoad { rx, progress: Arc::new(AtomicUsize::new(0)), cancel: Arc::clone(&cancel), total: PENDING_KEYSOUNDS };
    (LoadedChart { keysounds: Some(keysounds), ..chart() }, cancel)
}

/// Open the decide screen over the browser for a chart that came parsed.
fn open(app: &mut App, loaded: LoadedChart) {
    let stage = Stage::Decide(Box::new(DecideState::loaded(&app.shared, loaded)));
    app.switch(Transition::Open(stage));
    assert_eq!(app.stage.id(), StageId::Decide);
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

/// The decide screen that is up.
fn decide(app: &App) -> &DecideState {
    let Stage::Decide(state) = &app.stage else {
        panic!("the {:?} screen is up, not the decide screen", app.stage);
    };
    state
}

/// Run frames until the skin has drawn and the scene has begun.
fn run_until_begun(app: &mut App, pixels: &mut HeadlessCanvas) {
    for _ in 0..SKIN_FRAMES {
        assert!(matches!(frame(app, pixels), Transition::Stay), "the screen left before its scene began");
        if decide(app).scene.begun {
            return;
        }
    }
    panic!("the decide skin never drew: {:?}", app.shared.skin_failure(SKIN_TYPE_DECIDE));
}

/// Move the scene clock on by `millis` and run one frame.
fn frame_after_ms(app: &mut App, pixels: &mut HeadlessCanvas, millis: i64) -> Transition {
    app.shared.age_skin_scene(Duration::from_millis(u64::try_from(millis).expect("a scene only moves forward")));
    frame(app, pixels)
}

/// Send one key to the screen that is up.
fn key(app: &mut App, key: KeyInput<'_>) {
    let now = Instant::now();
    let transition = app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key);
    assert!(matches!(transition, Transition::Stay), "a key left the decide screen without its fade");
}

/// Run the scene from its beginning to past the end of its fade, a step at a time, and answer the
/// transition it leaves with.
fn run_to_the_end(app: &mut App, pixels: &mut HeadlessCanvas) -> Transition {
    assert!(matches!(frame_after_ms(app, pixels, TIMES.scene_ms + 500), Transition::Stay), "the scene left without fading");
    frame_after_ms(app, pixels, TIMES.fadeout_ms + 500)
}

/// A browser on a library of one chart, with the pack's decide skin if there is one.
fn browsing(mut app: App) -> (App, SelectState) {
    app.shared.library = Library::from_songs(vec![entry("alpha", "a", "5")]);
    app.shared.select_view = SelectView::AllSongs;
    app.shared.rebuild_select_items();
    (app, SelectState::new())
}

/// Send one key to the browser and answer where it goes.
fn browser_key(app: &mut App, state: &mut SelectState, code: KeyCode) -> Transition {
    let now = Instant::now();
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(code))
}

/// Without a decide skin a picked chart goes where it always went: the LOADING screen. That is
/// every player who has not chosen a skin pack.
#[test]
fn without_a_decide_skin_a_picked_chart_goes_straight_to_the_loading_screen() {
    let (mut app, mut state) = browsing(crate::stage::render_tests::app());
    assert!(!app.shared.has_decide_scene());
    assert!(matches!(browser_key(&mut app, &mut state, KeyCode::Enter), Transition::Open(Stage::Loading(_))));
    assert!(matches!(app.shared.decided_chart_stage(chart()), Stage::Play(_)), "a parsed chart with nothing to decode did not start its run");
}

/// With one, the same key opens the decide scene over the browser, for a run and for the practice
/// panel alike, as the reference sends every way of starting a chart through its decide screen.
#[test]
fn with_a_decide_skin_a_picked_chart_opens_the_decide_scene() {
    let (mut app, mut state) = browsing(app_with_decide_skin("picked", SKIN));
    assert!(app.shared.has_decide_scene());
    assert!(matches!(browser_key(&mut app, &mut state, KeyCode::Enter), Transition::Open(Stage::Decide(_))));

    assert!(matches!(browser_key(&mut app, &mut state, KeyCode::F4), Transition::Open(Stage::Decide(_))), "the practice key skipped the decide scene");
    assert!(app.shared.has_practice_request());
    assert!(matches!(app.shared.decided_chart_stage(chart()), Stage::Decide(_)), "a chart parsed for a replay skipped the decide scene");
}

/// The name of the course the tests below run.
const COURSE_NAME: &str = "Fixture Course";

/// Begin a course of two stages, both the one chart in the library, the way the browser begins one:
/// the player's own play settings are put aside and the ones in effect are changed.
fn begin_course(app: &mut App) -> rbms_config::PlayOptions {
    let entry = &app.shared.library.songs()[0];
    let stage = CourseChart { md5: entry.md5.clone(), sha256: String::new(), title: entry.title.clone() };
    let course = Course { name: COURSE_NAME.into(), charts: vec![stage.clone(), stage], ..Course::default() };
    let own = app.shared.config.play.clone();
    app.shared.course_settings_backup = Some(own.clone());
    app.shared.config.play.constant_speed = !own.constant_speed;
    app.shared.course_run = Some(CourseRun::new(course, 0.0));
    own
}

/// A course goes through the decide scene once, before its first stage. The stages after it follow
/// on from a result screen and are loaded as they always were.
#[test]
fn a_course_opens_with_the_decide_scene_and_loads_its_later_stages_directly() {
    let (mut app, _) = browsing(app_with_decide_skin("course", SKIN));
    begin_course(&mut app);

    assert!(matches!(crate::load_course_stage(&mut app.shared), Transition::To(Stage::Decide(_))), "the first stage skipped the decide scene");
    app.shared.course_run.as_mut().expect("the course is running").index = 1;
    assert!(matches!(crate::load_course_stage(&mut app.shared), Transition::To(Stage::Loading(_))), "a later stage went back through the decide scene");
}

/// The scene's clock starts on the frame its skin first draws, whatever was on the clock and the
/// timers while the skin was being read, and the screen is the skin's alone from then on.
#[test]
fn the_scene_begins_on_the_frame_its_skin_first_draws_with_its_timers_off() {
    const WHILE_READING: Duration = Duration::from_secs(30);
    let mut app = app_with_decide_skin("begins", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    app.shared.skin_timers.set_on(timer_id::FADEOUT, 0);
    open(&mut app, chart());
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "the browser's timers followed the player into the decide scene");

    app.shared.age_skin_scene(WHILE_READING);
    app.shared.skin_timers.set_on(timer_id::STARTINPUT, 0);
    run_until_begun(&mut app, &mut pixels);

    assert!(app.shared.skin_now_us() < at_ms(TIMES.input_ms), "the scene did not begin when its skin first drew");
    assert!(!app.shared.skin_timers.is_on(timer_id::STARTINPUT), "a timer was on before the scene began");
    assert_eq!(decide(&app).scene.times, TIMES, "the scene does not run on its skin's own times");
    assert_eq!(pixels.pixel_at(MIDDLE.0, MIDDLE.1), GROUND, "the skin is not what drew the screen");
    assert_eq!(pixels.pixel_at(LISTENING_MARK_AT.0, LISTENING_MARK_AT.1), GROUND, "the scene listened from its first frame");
}

/// The whole scene, left alone: it listens once its input time has passed, fades once its length
/// has, and goes to the run when the fade is over -- a chart with nothing left to decode starts
/// playing. The run begins a scene of its own, with every timer off again.
#[test]
fn a_scene_left_alone_listens_fades_and_starts_the_run() {
    let mut app = app_with_decide_skin("whole", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    open(&mut app, chart());
    run_until_begun(&mut app, &mut pixels);

    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.input_ms + 500), Transition::Stay));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert_eq!(pixels.pixel_at(LISTENING_MARK_AT.0, LISTENING_MARK_AT.1), LISTENING_MARK, "STARTINPUT did not go on after the input time");
    assert_eq!(pixels.pixel_at(FADING_MARK_AT.0, FADING_MARK_AT.1), GROUND, "the scene faded before its length had passed");

    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.scene_ms), Transition::Stay));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert_eq!(pixels.pixel_at(FADING_MARK_AT.0, FADING_MARK_AT.1), FADING_MARK, "FADEOUT did not go on after the scene's length");

    let leaving = frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms + 500);
    assert!(matches!(leaving, Transition::To(Stage::Play(_))), "a loaded chart did not start its run when the scene ended");
    app.switch(leaving);
    assert_eq!(app.stage.id(), StageId::Play);
    assert!(!app.shared.skin_timers.is_on(timer_id::STARTINPUT) && !app.shared.skin_timers.is_on(timer_id::FADEOUT), "the scene's timers outlived it");
    assert_eq!(app.suspended.len(), 1, "the browser is no longer suspended under the run");
}

/// A scene that ends while the chart's files are still decoding hands the load on to the LOADING
/// screen, still running, and that screen does not play the decide cue a second time.
#[test]
fn a_scene_that_ends_before_the_chart_is_in_hands_the_load_to_the_loading_screen() {
    let mut app = app_with_decide_skin("unfinished", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let (loaded, cancel) = chart_still_loading();
    open(&mut app, loaded);
    run_until_begun(&mut app, &mut pixels);

    let leaving = run_to_the_end(&mut app, &mut pixels);
    let Transition::To(Stage::Loading(loading)) = &leaving else {
        panic!("a chart that was still loading did not go to the loading screen");
    };
    assert!(loading.keysounds_pending(), "the load did not travel with the screen change");
    assert!(!loading.plays_the_decide_cue(), "the decide cue would be heard a second time");
    assert!(!cancel.load(Ordering::Relaxed), "going on to the run stopped the chart's decodes");
}

/// Enter after the scene has started listening skips the rest of its length: the fade starts at
/// once, and the scene leaves after it. The same key before then does nothing.
#[test]
fn enter_skips_the_scene_only_once_it_listens() {
    let mut app = app_with_decide_skin("skipped", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    open(&mut app, chart());
    run_until_begun(&mut app, &mut pixels);

    key(&mut app, press(KeyCode::Enter));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    key(&mut app, release(KeyCode::Enter));
    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.input_ms + 500), Transition::Stay));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert_eq!(pixels.pixel_at(FADING_MARK_AT.0, FADING_MARK_AT.1), GROUND, "a key pressed before the scene took input skipped it");

    key(&mut app, press(KeyCode::Enter));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));
    assert_eq!(pixels.pixel_at(FADING_MARK_AT.0, FADING_MARK_AT.1), FADING_MARK, "Enter did not start the fade");
    assert!(
        matches!(frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms + 500), Transition::To(Stage::Play(_))),
        "a skipped scene did not go on to the run"
    );
}

/// Escape fades the scene out to the browser: the chart's decodes are stopped, what the chart was
/// picked for is forgotten, and the browser comes back as it was left, timers and all.
#[test]
fn escape_stops_the_load_and_returns_to_the_browser_it_was_opened_from() {
    const BROWSER_TIMER_ON_US: i64 = 1234;
    let mut app = app_with_decide_skin("cancelled", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    app.shared.skin_timers.set_on(timer_id::SONGBAR_CHANGE, BROWSER_TIMER_ON_US);
    app.shared.practice_requested = true;
    let (loaded, cancel) = chart_still_loading();
    open(&mut app, loaded);
    run_until_begun(&mut app, &mut pixels);

    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.input_ms + 500), Transition::Stay));
    key(&mut app, press(KeyCode::Escape));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay), "a cancelled scene left without its fade");
    assert!(!cancel.load(Ordering::Relaxed), "the load was stopped before the scene had faded out");

    let leaving = frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms + 500);
    assert!(matches!(leaving, Transition::Back), "a cancelled scene did not go back");
    assert!(cancel.load(Ordering::Relaxed), "the chart kept decoding after the player went back");
    assert!(!app.shared.has_practice_request(), "the next chart picked would still open the practice panel");

    app.switch(leaving);
    assert_eq!(app.stage.id(), StageId::Select);
    assert!(app.suspended.is_empty());
    assert_eq!(app.shared.skin_timers.value_us(timer_id::SONGBAR_CHANGE), BROWSER_TIMER_ON_US, "the browser's scene was not put back as it was left");
    assert!(!app.shared.skin_timers.is_on(timer_id::FADEOUT), "the decide scene's fade followed the player back to the browser");
}

/// A decide skin that turns out not to load ends the scene before it begins: the chart goes on to
/// the LOADING screen, which plays the decide cue as it does with no decide scene, and the next
/// chart picked does not try the skin again.
#[test]
fn a_decide_skin_that_cannot_be_read_falls_back_to_the_loading_screen() {
    crate::notify::exclusive(|| {
        let mut app = app_with_decide_skin("broken", BROKEN_SKIN);
        let mut pixels = HeadlessCanvas::new(CW, CH);
        assert!(app.shared.has_decide_scene(), "a skin nobody has tried to read yet is taken at its word");
        let (loaded, cancel) = chart_still_loading();
        open(&mut app, loaded);

        let leaving = (0..SKIN_FRAMES).map(|_| frame(&mut app, &mut pixels)).find(|transition| !matches!(transition, Transition::Stay));
        let Some(Transition::To(Stage::Loading(loading))) = &leaving else {
            panic!("a scene with no skin to draw it did not go on to the loading screen");
        };
        assert!(loading.keysounds_pending() && loading.plays_the_decide_cue(), "the fallback is not the loading screen a skinless chart gets");
        assert!(!cancel.load(Ordering::Relaxed), "falling back stopped the chart's decodes");
        assert!(!app.shared.has_decide_scene(), "a skin that failed to read would be tried again for the next chart");
        assert!(matches!(app.shared.decided_song_stage(0), Stage::Loading(_)));

        let mut said = Vec::new();
        crate::notify::drain(&mut said);
    });
}

/// Every visit to the decide scene reads its skin again, even one that follows a cancel: a Lua skin
/// builds its screen from the chart it is read against.
#[test]
fn each_visit_reads_the_decide_skin_again() {
    let mut app = app_with_decide_skin("revisit", SKIN);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    open(&mut app, chart());
    run_until_begun(&mut app, &mut pixels);
    let first = app.shared.skins.build_of(SKIN_TYPE_DECIDE).expect("the skin was read for the first visit");

    frame_after_ms(&mut app, &mut pixels, TIMES.input_ms + 500);
    key(&mut app, press(KeyCode::Escape));
    frame(&mut app, &mut pixels);
    let leaving = frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms + 500);
    assert!(matches!(leaving, Transition::Back));
    app.switch(leaving);

    open(&mut app, chart());
    run_until_begun(&mut app, &mut pixels);
    let second = app.shared.skins.build_of(SKIN_TYPE_DECIDE).expect("the skin was read for the second visit");
    assert_ne!(first, second, "the second visit drew the skin that was read for the first chart");
}

/// A folder of this test's own for the charts it picks, beside the settings folder `tag` names.
fn chart_folder(tag: &str) -> PathBuf {
    let folder = settings_of(&format!("decide-{tag}-charts")).with_file_name("charts");
    std::fs::create_dir_all(&folder).expect("the chart folder is writable");
    folder
}

/// Point the one chart in the library at `path`.
fn library_chart_at(app: &mut App, path: PathBuf) {
    app.shared.library = Library::from_songs(vec![SongEntry { path, ..entry("alpha", "a", "5") }]);
    app.shared.rebuild_select_items();
}

/// Everything said so far that names `path`.
fn said_about(path: &std::path::Path) -> Vec<(Level, String)> {
    let mut heard = Vec::new();
    crate::notify::drain(&mut heard);
    let named = path.to_string_lossy();
    heard.into_iter().filter(|(_, message)| message.contains(named.as_ref())).collect()
}

/// A chart picked in the browser is parsed as the decide screen is entered. One whose file is gone,
/// or is not a chart, never becomes a scene: nothing is decoded, the reason is said once, and the
/// first frame goes back to the browser the screen was opened over.
#[test]
fn a_picked_chart_that_cannot_be_read_goes_straight_back_to_the_browser() {
    const NOT_A_CHART: &[u8] = b"not a chart";
    crate::notify::exclusive(|| {
        let folder = chart_folder("unreadable");
        let (missing, broken) = (folder.join("missing.bms"), folder.join("broken.bmson"));
        std::fs::write(&broken, NOT_A_CHART).expect("the broken chart is written");

        for (tag, path, reason) in [("missing", missing, "chart not found"), ("broken", broken, "chart could not be read")] {
            let (mut app, _) = browsing(app_with_decide_skin(tag, SKIN));
            let mut pixels = HeadlessCanvas::new(CW, CH);
            library_chart_at(&mut app, path.clone());
            app.shared.practice_requested = true;

            let picked = app.shared.decided_song_stage(0);
            assert!(matches!(picked, Stage::Decide(_)), "{tag}: a picked chart skipped the decide scene");
            app.switch(Transition::Open(picked));
            assert!(decide(&app).failed, "{tag}: a chart that cannot be read was taken in");
            assert!(decide(&app).assets.is_none(), "{tag}: something of an unreadable chart is being decoded");

            let leaving = frame(&mut app, &mut pixels);
            assert!(matches!(leaving, Transition::Back), "{tag}: the screen did not go back on its first frame");
            assert!(!app.shared.has_practice_request(), "{tag}: the next chart picked would still open the practice panel");
            let said = said_about(&path);
            assert_eq!(said.len(), 1, "{tag}: the failure was not said exactly once: {said:?}");
            assert_eq!(said[0].0, Level::Error);
            assert!(said[0].1.starts_with(reason), "{tag}: {:?} does not say {reason:?}", said[0].1);

            app.switch(leaving);
            assert_eq!(app.stage.id(), StageId::Select);
            assert!(app.suspended.is_empty(), "{tag}: the browser was not what the screen went back to");
        }
    });
}

/// A course whose first chart cannot be read is over before it began: the screen goes back, the run
/// is forgotten, and the player's own play settings are the ones in effect again.
#[test]
fn a_course_whose_first_chart_cannot_be_read_ends_before_it_begins() {
    crate::notify::exclusive(|| {
        let (mut app, _) = browsing(app_with_decide_skin("course-unreadable", SKIN));
        let mut pixels = HeadlessCanvas::new(CW, CH);
        library_chart_at(&mut app, chart_folder("course-unreadable").join("missing.bms"));
        let own = begin_course(&mut app);

        let opening = crate::load_course_stage(&mut app.shared);
        assert!(matches!(opening, Transition::To(Stage::Decide(_))), "the first stage skipped the decide scene");
        app.switch(opening);
        let leaving = frame(&mut app, &mut pixels);
        assert!(matches!(leaving, Transition::Back));
        assert!(app.shared.course_run.is_none(), "a course that never started is still running");
        assert_eq!(app.shared.config.play, own, "the course's play settings outlived it");
        assert!(app.shared.course_settings_backup.is_none());

        app.switch(leaving);
        assert_eq!(app.stage.id(), StageId::Select);
    });
}

/// Cancelling the decide scene a course opens with abandons the course, not only the chart: its
/// decodes are stopped, the run is forgotten, and the play settings the course had changed are put
/// back.
#[test]
fn cancelling_the_scene_a_course_opens_with_ends_the_course() {
    let (mut app, _) = browsing(app_with_decide_skin("course-cancelled", SKIN));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let own = begin_course(&mut app);
    let (loaded, cancel) = chart_still_loading();
    let opening = Stage::Decide(Box::new(DecideState::loaded(&app.shared, loaded)));
    app.switch(Transition::To(opening));
    assert_eq!(decide(&app).course.as_deref(), Some(COURSE_NAME), "the scene does not show the course it opens");
    run_until_begun(&mut app, &mut pixels);

    assert!(matches!(frame_after_ms(&mut app, &mut pixels, TIMES.input_ms + 500), Transition::Stay));
    key(&mut app, press(KeyCode::Escape));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay), "a cancelled scene left without its fade");
    assert!(app.shared.course_run.is_some(), "the course ended before the scene had faded out");

    let leaving = frame_after_ms(&mut app, &mut pixels, TIMES.fadeout_ms + 500);
    assert!(matches!(leaving, Transition::Back), "a cancelled course did not go back");
    assert!(cancel.load(Ordering::Relaxed), "the stage kept decoding after the course was abandoned");
    assert!(app.shared.course_run.is_none(), "the course is still running");
    assert_eq!(app.shared.config.play, own, "the course's play settings outlived it");
    assert!(app.shared.course_settings_backup.is_none());

    app.switch(leaving);
    assert_eq!(app.stage.id(), StageId::Select);
}

/// A decide skin that is only data and draws nothing. The tests below leave its files undecoded, so
/// it never gets as far as drawing.
const PLAIN_SKIN: &str = r#"{ "type": 6, "name": "Plain", "w": 1280, "h": 720, "source": [], "image": [], "destination": [] }"#;
const PLAIN_SKIN_FILE: &str = "decide.json";

/// Frames a screen is watched for while nothing is expected to change.
const WAITING_FRAMES: usize = 5;

/// The decide screen opened for a chart that is still loading, with its skin's files stuck in a
/// decode that reports nothing for as long as the handle is kept, and the flag that stops the
/// chart's own decodes.
fn opened_with_its_skin_stuck(tag: &str) -> (App, HandDecode, Arc<AtomicBool>) {
    let mut app = app_with_decide_document(tag, PLAIN_SKIN_FILE, PLAIN_SKIN);
    let (loaded, cancel) = chart_still_loading();
    open(&mut app, loaded);
    let decode = HandDecode::begin(&mut app.shared, SKIN_TYPE_DECIDE);
    (app, decode, cancel)
}

/// A decide skin whose files never arrive is not waited for without end. The screen is black while
/// it waits, and once the wait has outlasted what a scene is held for the chart goes on to the
/// LOADING screen, as it does when the skin cannot be read: its load still running, and the decide
/// cue still to be played.
#[test]
fn a_decide_skin_waited_for_past_the_limit_is_given_up_on_for_the_loading_screen() {
    let (mut app, _decode, cancel) = opened_with_its_skin_stuck("stuck");
    let mut pixels = HeadlessCanvas::new(CW, CH);
    for _ in 0..WAITING_FRAMES {
        assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay), "the screen left while its skin was still within its wait");
    }
    assert!(!decide(&app).scene.begun, "a scene began without a skin to draw it");
    assert_eq!(pixels.pixel_at(MIDDLE.0, MIDDLE.1), Color::BLACK, "something was drawn before the skin was in");

    app.shared.outlast_skin_wait();
    let leaving = (0..WAITING_FRAMES).map(|_| frame(&mut app, &mut pixels)).find(|transition| !matches!(transition, Transition::Stay));
    let Some(Transition::To(Stage::Loading(loading))) = &leaving else {
        panic!("a scene whose skin never arrived did not go on to the loading screen");
    };
    assert!(loading.keysounds_pending() && loading.plays_the_decide_cue(), "the fallback is not the loading screen a skinless chart gets");
    assert!(!cancel.load(Ordering::Relaxed), "giving up on the skin stopped the chart's decodes");
}

/// While the skin is still being read there is no scene to fade out, so Escape goes back to the
/// browser on the next frame and stops the chart's decodes. Enter has nothing to skip yet.
#[test]
fn escape_leaves_for_the_browser_at_once_while_the_skin_is_still_being_read() {
    let (mut app, _decode, cancel) = opened_with_its_skin_stuck("stuck-escape");
    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay));

    key(&mut app, press(KeyCode::Enter));
    assert!(matches!(frame(&mut app, &mut pixels), Transition::Stay), "Enter left a screen that has no scene yet");
    key(&mut app, release(KeyCode::Enter));
    assert!(!cancel.load(Ordering::Relaxed));

    key(&mut app, press(KeyCode::Escape));
    let leaving = frame(&mut app, &mut pixels);
    assert!(matches!(leaving, Transition::Back), "Escape did not leave a screen still waiting for its skin");
    assert!(cancel.load(Ordering::Relaxed), "the chart kept decoding after the player went back");

    app.switch(leaving);
    assert_eq!(app.stage.id(), StageId::Select);
    assert!(app.suspended.is_empty());
}
