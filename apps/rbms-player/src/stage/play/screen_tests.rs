//! The play screen as a skin draws it: entered the way the application enters it, drawn by a small
//! skin of its own, and walked through the reference's states on that skin's times.
//!
//! The scene clock and the song clock are both real clocks, so a test counts them itself
//! ([`Clocks`]): it moves them on by hand ([`pass`]) rather than waiting, and every frame is run
//! with both clocks put where the count says they are, before it and again after it ([`pin`]). The
//! time a frame takes, and the time the machine leaves the test waiting between two frames, are
//! then no part of the run, however busy the machine is: what is asserted is which side of a time
//! a frame fell on.

use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rbms_judge::GaugeKind;
use rbms_library::Library;
use rbms_play::SessionOptions;
use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;

use super::scene::{PlayTimes, elapsed_ms};
use super::*;
use crate::app_play::{LoadedChart, loaded_chart_for_tests};
use crate::stage::capture::{app_in, settings_of};
use crate::stage::render_tests::FRAME_DT;
use crate::stage::select::tests::{entry, press, release};
use crate::stage::{DecideState, HeadlessCanvas, KeysoundLoad, LoadingState, Stage, StageId};
use crate::{App, CH, CW, Config};

mod run_tests;

/// The fixture skin: a seven-key play skin with a note field, which paints the screen one colour
/// and puts up a square for each timer the scene switches, and one while the screen is loading.
const SKIN: &str = include_str!("fixture/play7.luaskin");

/// The fixture decide skin of the decide scene's own tests, for the way from that scene to this.
const DECIDE_SKIN: &str = include_str!("../decide/fixture/decide.luaskin");

/// The times the fixture states.
const TIMES: PlayTimes = PlayTimes { input_ms: 500, loadend_ms: 2000, playstart_ms: 1000, close_ms: 1500, finishmargin_ms: 800, fadeout_ms: 600 };

/// A seven-key chart with two notes in its first key, at two and three seconds, and one in its
/// sixth at two.
const CHART: &[u8] = b"#PLAYER 1\n#TITLE Scene\n#ARTIST Fixture\n#BPM 120\n#WAV01 a.wav\n#00111:0101\n#00118:01\n";
const CHART_NAME: &str = "scene.bms";

/// When the chart's first and last notes are reached, and how long the reference keeps it playing.
const FIRST_NOTE_MS: i64 = 2_000;
const LAST_NOTE_MS: i64 = 3_000;
const PLAY_TIME_MS: i64 = LAST_NOTE_MS + scene::TIME_MARGIN_MS;

/// How far either side of a time a test steps.
const SLACK_MS: i64 = 250;

/// How long past a note the engine counts it as missed, with room to spare.
const MISS_AFTER_MS: i64 = 1_000;

/// What the fixture paints the screen with, and its marks.
const GROUND: Color = Color::rgb(20, 40, 80);
const LOADING_MARK: Color = Color::rgb(255, 255, 255);
const READY_MARK: Color = Color::rgb(0, 255, 0);
const PLAY_MARK: Color = Color::rgb(0, 0, 255);
const PREVIEW_MARK: Color = Color::rgb(0, 255, 255);
const FADE_MARK: Color = Color::rgb(255, 255, 0);
const MUSIC_END_MARK: Color = Color::rgb(255, 0, 255);
const FAILED_MARK: Color = Color::rgb(255, 0, 0);

/// The colour of the fixture's notes, and the edge of the image they are drawn from.
const NOTE: Color = Color::rgb(240, 240, 120);
const NOTE_CELL_EDGE: u32 = 2;

/// How many times smaller than the size the fixture is authored at the tests draw it, which is
/// what keeps a frame cheap.
const SHRINK: u32 = 4;

/// A pixel inside each mark, at the size the fixture is authored at.
const LOADING_AT: (u32, u32) = (CW / 2, CH / 2);
const READY_AT: (u32, u32) = (10, 10);
const PLAY_AT: (u32, u32) = (74, 10);
const PREVIEW_AT: (u32, u32) = (138, 10);
const FADE_AT: (u32, u32) = (CW - 182, 10);
const MUSIC_END_AT: (u32, u32) = (CW - 118, 10);
const FAILED_AT: (u32, u32) = (CW - 10, 10);

/// The middle of the fixture's first lane, and the rows its note field covers.
const FIRST_LANE_X: u32 = 60;
const FIELD_TOP: u32 = 100;
const FIELD_BOTTOM: u32 = 500;

/// Frames a skin with no files is given to be read and compiled.
const SKIN_FRAMES: usize = 240;

/// The keys START and SELECT ship on, and the key the first lane ships on.
const START_KEY: KeyCode = KeyCode::KeyA;
const SELECT_KEY: KeyCode = KeyCode::KeyW;

/// How many keysounds the chart of a load that is still running is waiting for.
const PENDING_KEYSOUNDS: usize = 4;

/// The two clocks of the run under test, counted by the test rather than read off the machine.
#[derive(Clone, Copy)]
struct Clocks {
    /// How long the scene has run.
    scene: Duration,
    /// How long the chart has played, once it has started.
    song: Option<Duration>,
}

impl Clocks {
    /// A scene that has just begun, with its chart yet to start.
    const UNSTARTED: Clocks = Clocks { scene: Duration::ZERO, song: None };
}

thread_local! {
    /// The clocks of the test that is running on this thread, begun afresh with every app a test
    /// makes ([`app_with_skins`]).
    static CLOCKS: Cell<Clocks> = const { Cell::new(Clocks::UNSTARTED) };
}

/// Put the scene clock and the song clock of `app` where the test's count says they are.
fn pin(app: &mut App) {
    let clocks = CLOCKS.get();
    let now = Instant::now();
    let behind = |by: Duration| now.checked_sub(by).expect("the process has been up for longer than the time asked for");
    app.shared.scene_started = behind(clocks.scene);
    if let Some(song) = clocks.song {
        app.shared.clock = behind(song);
    }
}

/// An app whose skin pack is one folder holding the given skins, each under its own file name.
fn app_with_skins(tag: &str, skins: &[(&str, &str)]) -> App {
    CLOCKS.set(Clocks::UNSTARTED);
    let settings = settings_of(&format!("play-{tag}"));
    let pack: PathBuf = settings.with_file_name("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    for (file, body) in skins {
        std::fs::write(pack.join(file), body).expect("the skin is written");
    }
    let cell = image::RgbaImage::from_pixel(NOTE_CELL_EDGE, NOTE_CELL_EDGE, image::Rgba([NOTE.r, NOTE.g, NOTE.b, NOTE.a]));
    cell.save(pack.join("cell.png")).expect("the note image is written");
    let mut config = Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    config.play.autoplay = false;
    let mut app = app_in(settings, config);
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(app.shared.mode);
    app.shared.replay = None;
    app
}

/// An app whose pack holds the fixture play skin and nothing else.
fn app_with_play_skin(tag: &str) -> App {
    app_with_skins(tag, &[("play7.luaskin", SKIN)])
}

/// The fixture chart as the run sees it.
fn model() -> rbms_model::Model {
    let src = rbms_parser::parse_with(CHART, Default::default());
    rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, CHART_NAME))
}

/// A run of the fixture chart on `options`, with nothing left to decode.
fn run(options: SessionOptions) -> PlayState {
    PlayState::new(PlaySession::new(model(), options), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// A run the game plays by itself, which clears.
fn autoplayed(app: &mut App) -> PlayState {
    app.shared.config.play.autoplay = true;
    run(SessionOptions { autoplay: true, ..SessionOptions::default() })
}

/// A run nobody plays on a gauge that empties at the first note missed.
fn unplayed() -> PlayState {
    run(SessionOptions { gauge: GaugeKind::Hazard, ..SessionOptions::default() })
}

/// The fixture chart parsed, with its keysounds still decoding, and what stops and finishes them.
fn chart_still_loading() -> (LoadedChart, Arc<AtomicBool>, Arc<AtomicUsize>) {
    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicUsize::new(0));
    let (_sender, rx) = std::sync::mpsc::channel();
    let keysounds = KeysoundLoad { rx, progress: Arc::clone(&progress), cancel: Arc::clone(&cancel), total: PENDING_KEYSOUNDS };
    (LoadedChart { keysounds: Some(keysounds), ..loaded_chart_for_tests(CHART, CHART_NAME) }, cancel, progress)
}

/// Enter the play screen the way the application does.
fn enter(app: &mut App, play: PlayState) {
    app.switch(Transition::To(Stage::Play(Box::new(play))));
    assert_eq!(app.stage.id(), StageId::Play);
}

/// The play screen that is up.
fn play(app: &App) -> &PlayState {
    let Stage::Play(state) = &app.stage else {
        panic!("the {:?} screen is up, not the play screen", app.stage);
    };
    state
}

fn phase(app: &App) -> Option<PlayPhase> {
    play(app).phase()
}

/// One frame of whatever screen is up, as the application runs one: its update, and its draw
/// unless the update asked to leave.
///
/// The frame is run with both clocks where the test's count has them, and they are put back there
/// once it is over, so the frame costs the run no time however long the machine takes over it. The
/// chart's own clock is counted from the frame that leaves the chart playing.
fn frame(app: &mut App, pixels: &mut HeadlessCanvas) -> Transition {
    pin(app);
    let now = Instant::now();
    let transition = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
    if matches!(transition, Transition::Stay) {
        app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, &mut Canvas::Headless(pixels));
    }
    let clocks = CLOCKS.get();
    if clocks.song.is_none() && matches!(&app.stage, Stage::Play(state) if state.phase() == Some(PlayPhase::Play)) {
        CLOCKS.set(Clocks { song: Some(Duration::ZERO), ..clocks });
    }
    pin(app);
    transition
}

/// The update of a frame alone, with the clocks left to run as they do, for the test that reads the
/// scene clock the moment a frame set it.
fn update(app: &mut App) {
    let now = Instant::now();
    let transition = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
    assert!(matches!(transition, Transition::Stay), "the play screen left in the {:?} state", phase(app));
}

/// One frame that has to leave the screen where it is.
fn stay(app: &mut App, pixels: &mut HeadlessCanvas) {
    assert!(matches!(frame(app, pixels), Transition::Stay), "the play screen left in the {:?} state", phase(app));
}

/// Run frames until the fixture skin has been read and has drawn.
fn run_until_drawn(app: &mut App, pixels: &mut HeadlessCanvas) {
    for _ in 0..SKIN_FRAMES {
        stay(app, pixels);
        if app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS) {
            stay(app, pixels);
            return;
        }
    }
    panic!("the play skin never drew: {:?}", app.shared.skin_failure(SKIN_TYPE_PLAY_7KEYS));
}

/// Move the scene clock on by `millis`, and the song clock with it once the chart is playing.
fn pass(app: &mut App, millis: i64) {
    let by = Duration::from_millis(u64::try_from(millis).expect("time only moves forward"));
    let clocks = CLOCKS.get();
    CLOCKS.set(Clocks { scene: clocks.scene + by, song: clocks.song.map(|song| song + by) });
    pin(app);
}

/// Move both clocks on by `millis` and run one frame that stays.
fn stay_after(app: &mut App, pixels: &mut HeadlessCanvas, millis: i64) {
    pass(app, millis);
    stay(app, pixels);
}

/// Enter `play` and walk its scene to the frame the chart starts playing on.
fn started(app: &mut App, pixels: &mut HeadlessCanvas, play: PlayState) {
    enter(app, play);
    run_until_drawn(app, pixels);
    stay_after(app, pixels, TIMES.loadend_ms + SLACK_MS);
    assert_eq!(phase(app), Some(PlayPhase::Ready));
    stay_after(app, pixels, TIMES.playstart_ms + SLACK_MS);
    assert_eq!(phase(app), Some(PlayPhase::Play));
}

/// Send one key to the screen that is up, as the application does: the held-key set first.
fn key(app: &mut App, key: KeyInput<'_>) -> Transition {
    let now = Instant::now();
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key)
}

/// The canvas the fixture is drawn on.
fn canvas() -> HeadlessCanvas {
    HeadlessCanvas::new(CW / SHRINK, CH / SHRINK)
}

/// The pixel of a frame of the fixture that shows the point `at` of the size it is authored at.
fn pixel(pixels: &HeadlessCanvas, at: (u32, u32)) -> Color {
    pixels.pixel_at(at.0 / SHRINK, at.1 / SHRINK)
}

/// Without a play skin nothing of this is in the way: the chart waits on the LOADING screen, the
/// run starts the moment its screen is up, and no scene is run over it.
#[test]
fn without_a_play_skin_a_chart_waits_on_the_loading_screen_and_starts_at_once() {
    let mut app = crate::stage::render_tests::app();
    assert!(!app.shared.has_play_scene());
    let (loading, _, _) = chart_still_loading();
    assert!(matches!(app.shared.enter_loaded_chart(loading), Stage::Loading(_)), "a chart with files to decode skipped the LOADING screen");

    let stage = app.shared.enter_loaded_chart(loaded_chart_for_tests(CHART, CHART_NAME));
    assert!(matches!(stage, Stage::Play(_)));
    app.switch(Transition::To(stage));
    assert_eq!(phase(&app), None, "a run no skin draws was given a scene");
    assert!(!app.shared.skin_timers.is_on(timer_id::READY));
}

/// With one, a chart goes to the play screen as soon as it is parsed, files still decoding or not,
/// and waits there: loading does not end until the files are in, however long the skin's own load
/// time has been over.
#[test]
fn with_a_play_skin_a_chart_loads_on_the_play_screen_until_its_files_are_in() {
    let mut app = app_with_play_skin("loads");
    let mut pixels = canvas();
    assert!(app.shared.has_play_scene());
    let (loading, _, progress) = chart_still_loading();
    let stage = app.shared.enter_loaded_chart(loading);
    assert!(matches!(stage, Stage::Play(_)), "a chart whose play screen a skin draws went to the LOADING screen");
    app.switch(Transition::To(stage));
    assert_eq!(phase(&app), Some(PlayPhase::Preload));

    run_until_drawn(&mut app, &mut pixels);
    assert_eq!(pixel(&pixels, LOADING_AT), LOADING_MARK, "the skin is not told the screen is loading");
    stay_after(&mut app, &mut pixels, TIMES.loadend_ms * 3);
    assert_eq!(phase(&app), Some(PlayPhase::Preload), "a chart became ready with its keysounds still decoding");
    assert_eq!(pixel(&pixels, READY_AT), GROUND);

    progress.store(PENDING_KEYSOUNDS, Ordering::Relaxed);
    stay(&mut app, &mut pixels);
    assert_eq!(phase(&app), Some(PlayPhase::Ready), "the chart did not become ready once its files were in");
    assert_eq!(pixel(&pixels, LOADING_AT), GROUND, "the skin still reads the screen as loading");
}

/// The whole way in: loading for as long as the skin says, ready for as long as it says, then
/// playing. Each state switches the timer the skin animates it on, and none of them early.
#[test]
fn a_chart_loads_readies_and_plays_on_the_skins_own_times() {
    let mut app = app_with_play_skin("times");
    let mut pixels = canvas();
    let run = autoplayed(&mut app);
    enter(&mut app, run);
    assert_eq!(phase(&app), Some(PlayPhase::Preload));
    run_until_drawn(&mut app, &mut pixels);
    assert_eq!(pixel(&pixels, MUSIC_END_AT), GROUND, "the skin is not what drew the screen");
    assert_eq!(pixel(&pixels, LOADING_AT), LOADING_MARK);

    stay_after(&mut app, &mut pixels, TIMES.loadend_ms - SLACK_MS * 2);
    assert_eq!(phase(&app), Some(PlayPhase::Preload), "the chart was ready before the skin's load time was over");
    stay_after(&mut app, &mut pixels, SLACK_MS * 3);
    assert_eq!(phase(&app), Some(PlayPhase::Ready));
    assert_eq!(pixel(&pixels, READY_AT), READY_MARK, "the ready timer is not on");
    assert_eq!(pixel(&pixels, PLAY_AT), GROUND, "the play timer is on before the chart plays");

    stay_after(&mut app, &mut pixels, TIMES.playstart_ms - SLACK_MS * 2);
    assert_eq!(phase(&app), Some(PlayPhase::Ready), "the chart played before the skin's start time was over");
    stay_after(&mut app, &mut pixels, SLACK_MS * 3);
    assert_eq!(phase(&app), Some(PlayPhase::Play));
    assert_eq!(pixel(&pixels, PLAY_AT), PLAY_MARK, "the play timer is not on");
    assert_eq!(pixel(&pixels, READY_AT), READY_MARK, "the ready timer went off when the chart started");
}

/// The song clock starts when the chart starts playing. However long the screen loaded and stood
/// ready, the chart is at its beginning on the frame it starts and nothing of it has gone by.
#[test]
fn the_song_clock_starts_when_the_chart_starts_playing() {
    const LONG_WAIT_MS: i64 = 20_000;
    let mut app = app_with_play_skin("clock");
    let mut pixels = canvas();
    let run = autoplayed(&mut app);
    enter(&mut app, run);
    run_until_drawn(&mut app, &mut pixels);

    app.shared.note_key(&press(START_KEY));
    stay_after(&mut app, &mut pixels, LONG_WAIT_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Preload), "a chart became ready with START held");
    assert_eq!((play(&app).song_us, play(&app).session.judge().total_judged()), (0, 0), "the chart moved while its screen was loading");
    app.shared.note_key(&release(START_KEY));
    stay(&mut app, &mut pixels);
    stay_after(&mut app, &mut pixels, 1_000 + SLACK_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Ready));
    assert_eq!(play(&app).session.judge().total_judged(), 0);

    stay_after(&mut app, &mut pixels, TIMES.playstart_ms + SLACK_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Play));
    stay(&mut app, &mut pixels);
    let song_ms = play(&app).song_us / MICROS_PER_MILLI;
    assert!((0..FIRST_NOTE_MS).contains(&song_ms), "the chart did not start from its beginning: it is at {song_ms} ms");
    assert_eq!(play(&app).session.judge().total_judged(), 0, "notes went by before the chart started");
    let played_ms = elapsed_ms(&app.shared.skin_timers, timer_id::PLAY, app.shared.skin_now_us());
    assert!((song_ms - played_ms).abs() < SLACK_MS, "the play timer reads {played_ms} ms with the chart at {song_ms} ms");
}

/// A run whose gauge empties fails where it is: the failure's timer goes on, the screen stays up
/// for the skin's close time, and only then is the run left for the result.
#[test]
fn a_run_that_fails_closes_on_the_skins_time_and_then_reaches_the_result() {
    let mut app = app_with_play_skin("fails");
    let mut pixels = canvas();
    started(&mut app, &mut pixels, unplayed());

    stay_after(&mut app, &mut pixels, FIRST_NOTE_MS + MISS_AFTER_MS);
    assert!(play(&app).session.is_failed(), "the fixture's gauge is empty after its first missed note");
    assert_eq!(phase(&app), Some(PlayPhase::Failed));
    assert_eq!(pixel(&pixels, FAILED_AT), FAILED_MARK, "the failure's timer is not on");
    assert_eq!(pixel(&pixels, FADE_AT), GROUND, "a failure does not fade: its skin closes it");

    let failed_at_song = play(&app).song_us;
    stay_after(&mut app, &mut pixels, TIMES.close_ms - SLACK_MS * 2);
    assert_eq!(phase(&app), Some(PlayPhase::Failed), "the failure closed before the skin's close time");
    assert_eq!(play(&app).song_us, failed_at_song, "a failed run went on being played");
    pass(&mut app, SLACK_MS * 3);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::To(Stage::Result(_))), "a failure that has closed did not reach the result");
}

/// A run that is played to its end waits out the reference's five seconds after its last note,
/// then the skin's margin, then fades for the skin's fade time, and then reaches the result.
#[test]
fn a_run_played_to_its_end_waits_its_margin_fades_and_reaches_the_result() {
    let mut app = app_with_play_skin("finishes");
    let mut pixels = canvas();
    let run = autoplayed(&mut app);
    started(&mut app, &mut pixels, run);

    stay_after(&mut app, &mut pixels, PLAY_TIME_MS - SLACK_MS * 2);
    assert_eq!(phase(&app), Some(PlayPhase::Play), "the run ended before its playing time was over");
    assert!(play(&app).session.all_notes_resolved());
    assert!(app.shared.skin_timers.is_on(timer_id::ENDOFNOTE_1P), "the last note went by without its timer");

    stay_after(&mut app, &mut pixels, SLACK_MS * 3);
    assert_eq!(phase(&app), Some(PlayPhase::Finished));
    assert_eq!(pixel(&pixels, MUSIC_END_AT), MUSIC_END_MARK, "the music-end timer is not on");
    assert_eq!(pixel(&pixels, FADE_AT), GROUND, "the fade began without the skin's margin");

    stay_after(&mut app, &mut pixels, TIMES.finishmargin_ms + SLACK_MS);
    assert_eq!(pixel(&pixels, FADE_AT), FADE_MARK, "the fade did not begin once the margin was over");
    stay_after(&mut app, &mut pixels, TIMES.fadeout_ms - SLACK_MS * 2);
    assert_eq!(phase(&app), Some(PlayPhase::Finished), "the run left before its fade was over");
    pass(&mut app, SLACK_MS * 3);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::To(Stage::Result(_))));
}

/// A practice slice has nothing to load, so it begins ready; its play timer starts as far into the
/// chart as the slice does, and it ends where the slice ends whatever its gauge holds.
#[test]
fn a_practice_slice_begins_ready_and_plays_from_where_the_slice_starts() {
    const SLICE_START_MS: i32 = 1_000;
    const SLICE_END_MS: i32 = 2_500;
    let mut app = app_with_play_skin("practice");
    let mut pixels = canvas();
    let mut panel = crate::practice::PracticePanel::new("md5".to_string(), Mode::BEAT_7K, 60_000, None, 300.0);
    panel.property.start_ms = SLICE_START_MS;
    panel.property.end_ms = SLICE_END_MS;
    let mut slice = unplayed();
    slice.set_practice(panel.start());
    enter(&mut app, slice);
    assert_eq!(phase(&app), Some(PlayPhase::Ready), "a practice slice went back to loading");

    run_until_drawn(&mut app, &mut pixels);
    assert_eq!(pixel(&pixels, LOADING_AT), GROUND, "a slice that is ready was drawn as loading");
    assert_eq!(pixel(&pixels, READY_AT), READY_MARK);
    stay_after(&mut app, &mut pixels, TIMES.playstart_ms + SLACK_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Play));
    stay(&mut app, &mut pixels);
    let played_ms = elapsed_ms(&app.shared.skin_timers, timer_id::PLAY, app.shared.skin_now_us());
    let slice_start = i64::from(SLICE_START_MS);
    assert!((played_ms - slice_start).abs() < SLACK_MS, "the play timer reads {played_ms} ms at the start of a slice from {slice_start} ms");

    stay_after(&mut app, &mut pixels, i64::from(SLICE_END_MS - SLICE_START_MS) + SLACK_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Finished), "a slice did not end where it ends, or its locked gauge failed it");
    assert_eq!(pixel(&pixels, FAILED_AT), GROUND);
}

/// Holding START while the chart loads previews it on timer 141, a second ahead of the first note,
/// and letting go ends the preview.
#[test]
fn holding_start_while_the_chart_loads_previews_it() {
    const PREVIEWED_FOR_MS: i64 = 600;
    let mut app = app_with_play_skin("preview");
    let mut pixels = canvas();
    let run = autoplayed(&mut app);
    enter(&mut app, run);
    run_until_drawn(&mut app, &mut pixels);
    assert_eq!(pixel(&pixels, PREVIEW_AT), GROUND);

    app.shared.note_key(&press(START_KEY));
    stay(&mut app, &mut pixels);
    stay(&mut app, &mut pixels);
    assert_eq!(pixel(&pixels, PREVIEW_AT), PREVIEW_MARK, "the preview's timer is not on");
    let ahead_ms = FIRST_NOTE_MS - elapsed_ms(&app.shared.skin_timers, CHART_PREVIEW, app.shared.skin_now_us());
    assert!((ahead_ms - 1_000).abs() < SLACK_MS, "the preview is {ahead_ms} ms ahead of the first note, not a second");
    stay_after(&mut app, &mut pixels, PREVIEWED_FOR_MS);
    let previewed_ms = play(&app).drawn_chart_us(&app.shared) / MICROS_PER_MILLI;
    let expected_ms = FIRST_NOTE_MS - 1_000 + PREVIEWED_FOR_MS;
    assert!((previewed_ms - expected_ms).abs() < SLACK_MS, "the note field is drawn at {previewed_ms} ms of the chart");
    assert!((FIELD_TOP..FIELD_BOTTOM).any(|y| pixel(&pixels, (FIRST_LANE_X, y)) == NOTE), "the previewed chart's notes are not in the skin's note field");

    app.shared.note_key(&release(START_KEY));
    stay(&mut app, &mut pixels);
    assert!(!app.shared.skin_timers.is_on(CHART_PREVIEW));
    assert_eq!(play(&app).drawn_chart_us(&app.shared), 0, "the note field did not go back to the chart's beginning");
}

/// A library of the one chart the run is of, so the run can be started again.
fn with_the_chart_in_the_library(app: &mut App) {
    let mut song = entry("Scene", "Fixture", "1");
    song.path = PathBuf::from("/fixture/scene.bms");
    song.mode = Mode::BEAT_7K;
    app.shared.chart_path = song.path.to_string_lossy().into_owned();
    app.shared.library = Library::from_songs(vec![song]);
}

/// A failed run played by hand starts again at once on START, laid out afresh, or on SELECT, laid
/// out as it was -- without waiting for the failure to close.
#[test]
fn start_or_select_starts_a_failed_run_again_at_once() {
    for (retry_key, keeps_layout) in [(START_KEY, false), (SELECT_KEY, true)] {
        let mut app = app_with_play_skin(if keeps_layout { "retry-same" } else { "retry-afresh" });
        let mut pixels = canvas();
        with_the_chart_in_the_library(&mut app);
        started(&mut app, &mut pixels, unplayed());
        stay_after(&mut app, &mut pixels, FIRST_NOTE_MS + MISS_AFTER_MS);
        assert_eq!(phase(&app), Some(PlayPhase::Failed));
        let seed = play(&app).session.seed();

        app.shared.note_key(&press(retry_key));
        assert!(matches!(frame(&mut app, &mut pixels), Transition::To(Stage::Loading(_))), "{retry_key:?} did not start the chart again");
        assert_eq!(app.shared.retry_seed, keeps_layout.then_some(seed), "{retry_key:?} carried the wrong layout over");
    }
}

/// A run that is not played by hand is not started again: START on a failed replay changes
/// nothing, and the failure closes as it would have.
#[test]
fn a_failed_replay_is_not_started_again() {
    let mut app = app_with_play_skin("retry-replay");
    let mut pixels = canvas();
    with_the_chart_in_the_library(&mut app);
    let replay = rbms_store::Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: String::new(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    };
    app.shared.replay = Some(replay.clone());
    started(&mut app, &mut pixels, run(SessionOptions { gauge: GaugeKind::Hazard, replay: Some(replay), ..SessionOptions::default() }));
    stay_after(&mut app, &mut pixels, FIRST_NOTE_MS + MISS_AFTER_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Failed));

    app.shared.note_key(&press(START_KEY));
    stay(&mut app, &mut pixels);
    assert_eq!(phase(&app), Some(PlayPhase::Failed));
    pass(&mut app, TIMES.close_ms + SLACK_MS);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::To(Stage::Result(_))));
}

/// Escape while a run is closing skips what is left of the closing: the result comes at once.
#[test]
fn escape_skips_the_rest_of_a_closing_run() {
    let mut app = app_with_play_skin("skip");
    let mut pixels = canvas();
    app.shared.config.play.play_escape = PlayEscape::Hold;
    started(&mut app, &mut pixels, unplayed());
    stay_after(&mut app, &mut pixels, FIRST_NOTE_MS + MISS_AFTER_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Failed));

    assert!(matches!(key(&mut app, release(KeyCode::Escape)), Transition::Stay), "a key coming up skipped the closing");
    assert!(matches!(key(&mut app, press(KeyCode::Escape)), Transition::To(Stage::Result(_))), "Escape did not skip the closing");
}

/// Escape before the run is closing leaves the chart on the terms of the ESCAPE setting, as it
/// always did, and a chart left while it loads has its decodes stopped.
#[test]
fn escape_leaves_a_chart_that_is_loading_and_stops_its_decodes() {
    let mut app = app_with_play_skin("leave");
    let mut pixels = canvas();
    with_the_chart_in_the_library(&mut app);
    let (loading, cancel, _) = chart_still_loading();
    let stage = app.shared.enter_loaded_chart(loading);
    app.switch(Transition::To(stage));
    run_until_drawn(&mut app, &mut pixels);

    let left = key(&mut app, press(KeyCode::Escape));
    assert!(!matches!(left, Transition::Stay), "Escape did not leave a chart that is loading");
    assert!(!cancel.load(Ordering::Relaxed));
    app.switch(left);
    assert!(cancel.load(Ordering::Relaxed), "the chart was left with its keysounds still decoding");
}

/// Escape on a chart with nothing left to hit finishes the run rather than abandoning it: it fades
/// out as a run the player ended, with no margin, and reaches the result.
#[test]
fn escape_on_a_chart_with_nothing_left_to_hit_fades_out_to_the_result() {
    let mut app = app_with_play_skin("ends");
    let mut pixels = canvas();
    let run = autoplayed(&mut app);
    started(&mut app, &mut pixels, run);
    stay_after(&mut app, &mut pixels, LAST_NOTE_MS + MISS_AFTER_MS);
    assert!(play(&app).session.all_notes_resolved());
    assert_eq!(phase(&app), Some(PlayPhase::Play));

    assert!(matches!(key(&mut app, press(KeyCode::Escape)), Transition::Stay), "a finished run was abandoned");
    assert_eq!(phase(&app), Some(PlayPhase::Finished));
    stay(&mut app, &mut pixels);
    assert_eq!(pixel(&pixels, FADE_AT), FADE_MARK, "the fade did not begin at once");
    assert_eq!(pixel(&pixels, MUSIC_END_AT), GROUND);
    pass(&mut app, TIMES.fadeout_ms + SLACK_MS);
    assert!(matches!(frame(&mut app, &mut pixels), Transition::To(Stage::Result(_))));
}

/// A key pressed before the chart plays lights the skin's key beam and is never handed to the run,
/// so the replay holds neither it nor its release. Once the chart plays, a press is the run's.
#[test]
fn a_key_pressed_before_the_chart_plays_is_not_part_of_the_run() {
    let mut app = app_with_play_skin("keys");
    let mut pixels = canvas();
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == 0).map(|(code, _)| *code).expect("the first lane has a key");
    enter(&mut app, unplayed());
    run_until_drawn(&mut app, &mut pixels);

    key(&mut app, press(lane_key));
    stay(&mut app, &mut pixels);
    assert!(app.shared.skin_timers.is_on(timer_id::KEYON_1P_KEY1), "a key that is down did not light its beam while the chart loads");
    assert!(play(&app).session.recorded_events().is_empty(), "a key pressed while the chart loads was recorded");
    stay_after(&mut app, &mut pixels, TIMES.loadend_ms + SLACK_MS);
    stay_after(&mut app, &mut pixels, TIMES.playstart_ms + SLACK_MS);
    assert_eq!(phase(&app), Some(PlayPhase::Play));
    key(&mut app, release(lane_key));
    assert!(play(&app).session.recorded_events().is_empty(), "a release with no press behind it was recorded");
    stay(&mut app, &mut pixels);
    assert!(app.shared.skin_timers.is_on(timer_id::KEYOFF_1P_KEY1));

    key(&mut app, press(lane_key));
    key(&mut app, release(lane_key));
    let recorded: Vec<bool> = play(&app).session.recorded_events().iter().map(|event| event.press).collect();
    assert_eq!(recorded, [true, false], "a press made while the chart plays was not the run's");
}

/// A judgement the engine gives by itself -- a lane it plays for the player -- reaches the skin: the
/// lane's bomb timer goes on, the note reads as judged to the note field, and the long way round
/// through the trace is what carries it.
#[test]
fn a_judgement_the_engine_gives_by_itself_reaches_the_skins_timers() {
    let mut app = app_with_play_skin("judged");
    let mut pixels = canvas();
    let run = autoplayed(&mut app);
    started(&mut app, &mut pixels, run);
    assert!(!app.shared.skin_timers.is_on(timer_id::BOMB_1P_KEY1));

    stay_after(&mut app, &mut pixels, FIRST_NOTE_MS + SLACK_MS);
    assert!(play(&app).session.judge().total_judged() > 0, "the engine has played the first notes");
    assert!(app.shared.skin_timers.is_on(timer_id::BOMB_1P_KEY1), "a note the engine played lit no bomb");
    assert!(app.shared.skin_timers.is_on(timer_id::BOMB_1P_KEY6), "the bomb is not lit in the lane the note was in");
    assert!(!app.shared.skin_timers.is_on(timer_id::BOMB_1P_KEY2));
}

/// A replay under analysis that is paused carries the scene clock with it: the scene's time is the
/// play timer's start plus the place in the chart, so the skin stands still with the notes. A jump
/// moves the scene with the chart and switches every lane's timers off.
#[test]
fn a_replay_under_analysis_carries_the_scene_clock_with_it() {
    const BETWEEN_FRAMES_US: i64 = 120_000;
    const BETWEEN_FRAMES: Duration = Duration::from_micros(BETWEEN_FRAMES_US.unsigned_abs());
    let mut app = app_with_play_skin("analysis");
    let mut pixels = canvas();
    let replay = rbms_store::Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: String::new(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    };
    app.shared.replay = Some(replay.clone());
    started(&mut app, &mut pixels, run(SessionOptions { analysis: true, replay: Some(replay), ..SessionOptions::default() }));
    stay_after(&mut app, &mut pixels, 500);

    assert!(matches!(key(&mut app, press(KeyCode::Space)), Transition::Stay));
    assert!(play(&app).session.analysis_paused());
    update(&mut app);
    let paused_at = play(&app).session.analysis_position_us();
    let scene_at = |app: &App| app.shared.skin_now_us() - app.shared.skin_timers.value_us(timer_id::PLAY);
    std::thread::sleep(BETWEEN_FRAMES);
    update(&mut app);
    assert_eq!(play(&app).session.analysis_position_us(), paused_at);
    let drift_us = scene_at(&app) - paused_at;
    assert!((0..BETWEEN_FRAMES_US).contains(&drift_us), "the scene ran on by {drift_us} us while the replay was paused");

    app.shared.skin_timers.set_on(timer_id::KEYON_1P_KEY1, 0);
    app.shared.skin_timers.set_on(timer_id::BOMB_1P_SCRATCH, 0);
    assert!(matches!(key(&mut app, press(KeyCode::PageUp)), Transition::Stay));
    assert!(
        !app.shared.skin_timers.is_on(timer_id::KEYON_1P_KEY1) && !app.shared.skin_timers.is_on(timer_id::BOMB_1P_SCRATCH),
        "a jump left a lane's timers on"
    );
    let jumped_to = play(&app).session.analysis_position_us();
    assert_eq!(jumped_to, paused_at + ANALYSIS_SEEK_STEP_US);
    let off_us = scene_at(&app) - jumped_to;
    assert!((0..BETWEEN_FRAMES_US).contains(&off_us), "the scene did not jump with the chart: it is {off_us} us off");
    stay(&mut app, &mut pixels);
    assert_eq!(phase(&app), Some(PlayPhase::Play), "a replay under analysis ended by itself");
}

/// A course's later stages are loaded like any other chart, so with a play skin they too go
/// straight to the play screen and load there; the one frame the LOADING screen is up for before
/// the chart is parsed is left black rather than drawn as the built-in screen.
#[test]
fn a_later_stage_of_a_course_loads_on_the_play_screen_too() {
    let mut app = app_with_play_skin("course");
    with_the_chart_in_the_library(&mut app);
    let stage = rbms_course::CourseChart { md5: app.shared.library.songs()[0].md5.clone(), sha256: String::new(), title: "Scene".into() };
    let course = rbms_course::Course { name: "Fixture Course".into(), charts: vec![stage.clone(), stage], ..rbms_course::Course::default() };
    app.shared.course_run = Some(rbms_course::CourseRun::new(course, 0.0));
    app.shared.course_run.as_mut().expect("the course is running").index = 1;

    assert!(matches!(crate::load_course_stage(&mut app.shared), Transition::To(Stage::Loading(_))));
    let waiting = crate::stage::render_tests::render(&mut app, Stage::Loading(LoadingState::song(0)));
    assert_eq!(pixel(&waiting, LOADING_AT), Color::BLACK, "the built-in LOADING screen was drawn on the way to a skin's play screen");
    let (loading, _, _) = chart_still_loading();
    let entered = app.shared.enter_loaded_chart(loading);
    assert!(matches!(entered, Stage::Play(_)), "a later stage of a course went to the LOADING screen");
    app.switch(Transition::To(entered));
    assert_eq!(phase(&app), Some(PlayPhase::Preload));

    let mut plain = crate::stage::render_tests::app();
    with_the_chart_in_the_library(&mut plain);
    let built_in = crate::stage::render_tests::render(&mut plain, Stage::Loading(LoadingState::song(0)));
    assert_ne!(pixel(&built_in, (CW / 2, CH / 2 - 30)), Color::BLACK, "without a play skin the LOADING screen is the built-in one");
}

/// A chart asked for practice still waits where it always did, because the practice panel is built
/// from the decoded chart.
#[test]
fn a_chart_asked_for_practice_does_not_load_on_the_play_screen() {
    let mut app = app_with_play_skin("practice-request");
    app.shared.practice_requested = true;
    let (loading, _, _) = chart_still_loading();
    assert!(matches!(app.shared.enter_loaded_chart(loading), Stage::Loading(_)));
}

/// The decide scene hands a chart whose play screen a skin draws straight to that screen, files
/// still decoding or not: the LOADING screen is never up between the two.
#[test]
fn the_decide_scene_hands_a_chart_straight_to_a_skins_play_screen() {
    const STEP_MS: i64 = 1_000;
    const STEPS: usize = 40;
    let mut app = app_with_skins("decided", &[("play7.luaskin", SKIN), ("decide.luaskin", DECIDE_SKIN)]);
    let mut pixels = canvas();
    let (loading, _, _) = chart_still_loading();
    let decide = DecideState::loaded(&app.shared, loading);
    app.switch(Transition::Open(Stage::Decide(Box::new(decide))));

    let mut seen = vec![app.stage.id()];
    for _ in 0..SKIN_FRAMES + STEPS {
        let transition = frame(&mut app, &mut pixels);
        if !matches!(transition, Transition::Stay) {
            app.switch(transition);
            seen.push(app.stage.id());
        }
        if app.stage.id() != StageId::Decide {
            break;
        }
        if app.shared.has_compiled_skin(rbms_skin::loader::SKIN_TYPE_DECIDE) {
            pass(&mut app, STEP_MS);
        }
    }
    assert_eq!(seen, [StageId::Decide, StageId::Play], "the chart did not go from the decide scene to the play screen");
    assert_eq!(phase(&app), Some(PlayPhase::Preload), "a chart handed over with files to decode is not loading");
}
