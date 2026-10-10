//! The headless render harness, the invariants that span every screen, and the screen history.
//!
//! Every stage is drawn onto a [`HeadlessCanvas`] with the embedded fonts pinned, so the frames are
//! deterministic without a window or a GPU. What is here is what only makes sense across all of
//! them: that each paints something, that drawing one twice gives the same signature, and that no
//! two come out looking alike — which is what a stage wired to the wrong state would do. The
//! transition tests at the bottom pin the screen history: what a screen opened over another
//! resumes, and what it leaves behind.
//!
//! Each screen's own snapshots live in a file of its own next door, so two people working on two
//! screens never edit the same one. They build their screens through the helpers here.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use rbms_play::{PlaySession, SessionOptions};
use rbms_render::ResultView;
use rbms_skin::timer::timer_id;
use rbms_store::SCORE_LN_MODE_FROM_CHART;

use crate::settings_view::visible_rows;
use crate::stage::{
    Canvas, FoldersState, FrameCtx, HeadlessCanvas, KeyConfigState, LoadingState, PlayState, ResultState, SelectState, SettingsState, Stage, TablesState,
};
use crate::stage::{KeyInput, StageId, Transition};
use crate::{App, CH, CW, Color, Config, KeyCode, LaunchOptions};
use rbms_config::{AdjustOutcome, SettingTab, adjust, tab_rows};

/// A one-measure 7-key chart: every lane plus the scratch, so the model has the same lane count as
/// the skin the app builds for [`crate::MODE`].
const CHART: &str = concat!(
    "#PLAYER 1\n#TITLE snapshot\n#BPM 120\n#WAV01 a.wav\n",
    "#00111:0101\n#00112:0100\n#00113:0001\n#00114:0101\n#00115:0100\n#00118:0001\n#00119:0101\n#00116:0100\n",
);

/// An app with no library, no window and no server: everything the screens read out of
/// [`crate::AppShared`] is at its default.
///
/// Its settings folder belongs to the test that asked for it. Tests run side by side, and two apps
/// opening one score database at the same moment leave one of them reporting a failure, which lands
/// on the message bus every other test reads.
pub(super) fn app() -> App {
    rbms_render::font::use_embedded_fonts_only();
    let dir = std::env::temp_dir().join(format!("rbms-render-tests-{}-{:?}", std::process::id(), std::thread::current().id()));
    App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron"))
}

pub(super) fn play_state() -> PlayState {
    play_state_with_bga(std::collections::HashMap::new())
}

/// The same play screen with a background image map of the caller's own, for the tests that need one
/// on screen.
pub(super) fn play_state_with_bga(bga: std::collections::HashMap<i32, crate::DecodedImage>) -> PlayState {
    let src = rbms_parser::parse_with(CHART.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&src, "snapshot.bms");
    let model = rbms_chart::to_model(&src, mode);
    PlayState::new(PlaySession::new(model, SessionOptions::default()), bga, 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

pub(super) fn result_state() -> ResultState {
    ResultState::new(ResultView {
        title: "snapshot".into(),
        artist: String::new(),
        mode_label: "7K",
        counts: [3, 2, 1, 0, 0, 0],
        ex_score: 8,
        max_score: 12,
        max_combo: 5,
        total_notes: 6,
        fast: [1, 0],
        slow: [1, 0],
        gauge: 80.0,
        clear_label: "CLEAR",
        clear_color: Color::GREEN,
        prev_best_ex: Some(6),
        prev_ex: Some(4),
        show_graph: true,
        show_result_graphs: true,
        gauge_series: Vec::new(),
        timing_hist: Box::new([]),
        judge_dist: [0; 6],
    })
}

/// The frame length the snapshots are drawn at, so a screen that animates draws one frame's worth.
pub(super) const FRAME_DT: f32 = 1.0 / 60.0;

/// Draw one stage onto a fresh headless canvas and return it. The screen is placed directly rather
/// than transitioned into, so a screen's `on_enter` (which can enumerate audio devices) stays out
/// of the render snapshots.
pub(super) fn render(app: &mut App, stage: Stage) -> HeadlessCanvas {
    let mut pixels = HeadlessCanvas::new(CW, CH);
    render_into(app, stage, &mut pixels);
    pixels
}

/// Draw one stage onto a canvas the caller keeps.
///
/// A test that needs several frames of the same screen has to reuse one canvas: a compiled skin
/// document holds textures registered with the target it was built against, so a fresh canvas per
/// frame would leave the document pointing at handles that target never heard of.
pub(super) fn render_into(app: &mut App, stage: Stage, pixels: &mut HeadlessCanvas) {
    render_on(app, stage, &mut Canvas::Headless(pixels));
}

/// [`render_into`] on either kind of target, for the capture harness, which also draws on a GPU.
pub(super) fn render_on(app: &mut App, stage: Stage, canvas: &mut Canvas<'_>) {
    app.stage = stage;
    let now = std::time::Instant::now();
    app.shared.hot.clear();
    let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT };
    app.stage.draw(&mut ctx, canvas);
}

/// Every screen, freshly constructed, in the order they appear in [`Stage`].
pub(super) fn every_stage() -> Vec<(&'static str, Stage)> {
    vec![
        ("Select", Stage::Select(Box::new(SelectState::new()))),
        ("Settings", Stage::Settings(SettingsState::new())),
        ("KeyConfig", Stage::KeyConfig(KeyConfigState::new())),
        ("Tables", Stage::Tables(TablesState::new())),
        ("Folders", Stage::Folders(FoldersState::new())),
        ("Loading", Stage::Loading(LoadingState::song(0))),
        ("Play", Stage::Play(Box::new(play_state()))),
        ("Result", Stage::Result(result_state())),
    ]
}

#[test]
fn every_stage_paints_a_frame() {
    let mut app = app();
    for (name, stage) in every_stage() {
        let pixels = render(&mut app, stage);
        assert!(pixels.painted_pixels() > 0, "{name} drew nothing");
        assert!(pixels.quad_count() > 0, "{name} drew no quads");
    }
}

#[test]
fn drawing_a_stage_twice_gives_the_same_frame() {
    let mut app = app();
    for (name, stage) in every_stage() {
        let first = render(&mut app, stage).signature();
        let stage = every_stage().into_iter().find(|(n, _)| *n == name).map(|(_, s)| s).expect("stage rebuilt");
        let second = render(&mut app, stage).signature();
        assert_eq!(first, second, "{name} is not deterministic");
    }
}

#[test]
fn no_two_stages_render_the_same_frame() {
    let mut app = app();
    let mut seen: HashSet<Vec<u8>> = HashSet::new();
    for (name, stage) in every_stage() {
        let signature = render(&mut app, stage).signature();
        assert!(seen.insert(signature), "{name} renders the same frame as another stage");
    }
}

#[test]
fn only_the_screens_with_clickable_rows_record_hot_regions() {
    let mut app = app();
    for (name, stage) in every_stage() {
        render(&mut app, stage);
        let clickable = matches!(name, "Select" | "Settings");
        assert_eq!(!app.shared.hot.is_empty(), clickable, "{name} hot regions");
    }
}

#[test]
fn the_background_image_slot_is_cleared_by_the_screens_that_do_not_use_it() {
    let mut app = app();
    for (name, stage) in every_stage() {
        let pixels = render(&mut app, stage);
        assert!(pixels.background().is_none(), "{name} left a background image behind");
    }
}

/// The background image is an ordinary texture drawn as an ordinary quad, and the one moment it is
/// drawn at is the screen's own clear: after the frame is wiped, before the screen queues anything.
/// Setting it before drawing is therefore enough to put it behind everything, with no special path
/// in either backend.
#[test]
fn a_background_image_is_drawn_by_the_clear_that_follows_it() {
    const SIDE: u32 = 8;
    let marker = Color::rgb(9, 200, 40);
    let pixels: Vec<u8> = (0..SIDE * SIDE).flat_map(|_| [marker.r, marker.g, marker.b, 255]).collect();

    let mut canvas = HeadlessCanvas::new(CW, CH);
    let rect = rbms_render::Rect::new(0.0, 0.0, CW as f32, CH as f32);
    Canvas::Headless(&mut canvas).set_background(1, &pixels, SIDE, SIDE, rect);
    let held = canvas.background().expect("the slot holds where the image goes");
    assert_eq!((held.x, held.y, held.w, held.h), (rect.x, rect.y, rect.w, rect.h));

    rbms_render::Renderer::clear(&mut canvas, Color::BLACK);
    assert_eq!(canvas.pixel_at(1, 1), marker, "the clear paints the background over the colour it cleared to");
    assert_eq!(canvas.quad_count(), 1, "the background is one quad, counted like every other");

    Canvas::Headless(&mut canvas).clear_bga();
    rbms_render::Renderer::clear(&mut canvas, Color::BLACK);
    assert_eq!(canvas.pixel_at(1, 1), Color::BLACK, "a cleared slot leaves the frame as the screen wiped it");
    assert_eq!(canvas.quad_count(), 0);
}

/// A background is stretched over whatever rectangle the layout gives it, and the chart's own frames
/// are routinely larger than that. Point sampling a shrink like that turns an animated background
/// into a shimmer, so the target that draws it uses the same rule a document's own `bga` object
/// does: interpolate unless the image lands at exactly its own size.
#[test]
fn a_resized_background_image_is_interpolated_by_the_clear_that_draws_it() {
    const SIDE: u32 = 8;
    let checker: Vec<u8> = (0..SIDE * SIDE)
        .flat_map(|index| {
            let light = ((index % SIDE) / 4 + (index / SIDE) / 4).is_multiple_of(2);
            if light { [255, 255, 255, 255] } else { [0, 0, 0, 255] }
        })
        .collect();

    let mut canvas = HeadlessCanvas::new(CW, CH);
    let rect = rbms_render::Rect::new(0.0, 0.0, CW as f32, CH as f32);
    Canvas::Headless(&mut canvas).set_background(1, &checker, SIDE, SIDE, rect);
    rbms_render::Renderer::clear(&mut canvas, Color::BLACK);

    let row: Vec<u8> = (0..CW).map(|x| canvas.pixel_at(x, CH / 4).r).collect();
    assert!(row.iter().any(|shade| *shade > 20 && *shade < 235), "the enlargement was point sampled rather than interpolated");
}

/// Handing the same decode over again costs nothing: a background picture lasts hundreds of
/// milliseconds while the screen offers it on every one of the frames in between, and copying and
/// re-uploading megabytes sixty times a second for one picture is the difference between a
/// background that is free and one that is not.
#[test]
fn a_background_frame_handed_over_twice_is_uploaded_once() {
    const SIDE: u32 = 8;
    let pixels: Vec<u8> = (0..SIDE * SIDE).flat_map(|_| [9u8, 200, 40, 255]).collect();

    let mut canvas = HeadlessCanvas::new(CW, CH);
    let rect = rbms_render::Rect::new(0.0, 0.0, CW as f32, CH as f32);
    let first = Canvas::Headless(&mut canvas).background_texture(7, &pixels, SIDE, SIDE);
    let again = Canvas::Headless(&mut canvas).background_texture(7, &pixels, SIDE, SIDE);
    assert_eq!(first, again, "one decode is one texture however many frames offer it");
    Canvas::Headless(&mut canvas).set_background(7, &pixels, SIDE, SIDE, rect);

    assert!(crate::gpu::background_upload_needed(None, 7, (SIDE, SIDE)), "nothing uploaded yet");
    assert!(!crate::gpu::background_upload_needed(Some((7, (SIDE, SIDE))), 7, (SIDE, SIDE)), "the same decode at the same size is already there");
    assert!(crate::gpu::background_upload_needed(Some((7, (SIDE, SIDE))), 8, (SIDE, SIDE)), "the next frame of the animation is a new decode");
    assert!(crate::gpu::background_upload_needed(Some((7, (SIDE, SIDE))), 7, (SIDE, 16)), "a decode that changed size needs a new texture");
}

/// Pixels that do not fill the size they are handed over with are refused rather than read past.
#[test]
fn a_background_image_whose_pixels_do_not_match_its_size_is_refused() {
    let mut canvas = HeadlessCanvas::new(CW, CH);
    Canvas::Headless(&mut canvas).set_background(1, &[0, 0, 0, 255], 4, 4, rbms_render::Rect::new(0.0, 0.0, 4.0, 4.0));
    assert!(canvas.background().is_none(), "one pixel is not a four by four image");
}

/// The JUDGE tab draws every row it declares, and draws a different frame from the tab next to it —
/// which is what a tab wired to the wrong row list would not do.
#[test]
fn the_judge_tab_paints_its_own_rows() {
    let mut app = app();
    let rows = tab_rows(SettingTab::Judge, &app.shared.config);
    let visible = visible_rows();
    assert!(rows.len() > 1, "the JUDGE tab has rows to draw");

    let judge = render(&mut app, Stage::Settings(SettingsState::on_tab(SettingTab::Judge)));
    assert!(judge.painted_pixels() > 0, "the JUDGE tab drew nothing");
    assert_eq!(app.shared.hot.len(), rows.len().min(visible) + SettingTab::ALL.len(), "every visible JUDGE row and every tab is clickable");

    let play = render(&mut app, Stage::Settings(SettingsState::on_tab(SettingTab::Play)));
    assert_ne!(judge.signature(), play.signature(), "the JUDGE tab renders the same frame as the PLAY tab");
    let judge_again = render(&mut app, Stage::Settings(SettingsState::on_tab(SettingTab::Judge)));
    assert_eq!(judge.signature(), judge_again.signature(), "the JUDGE tab is not deterministic");
}

/// Draw the JUDGE tab with the cursor `down` rows from the top, which is also how far the row list
/// has scrolled. Driven through the screen's own key handler rather than its private state, so the
/// scroll under test is the one a user gets.
fn render_judge_tab(app: &mut App, down: usize) -> HeadlessCanvas {
    app.stage = Stage::Settings(SettingsState::on_tab(SettingTab::Judge));
    let now = std::time::Instant::now();
    for _ in 0..down {
        let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT };
        app.stage.handle_key(&mut ctx, KeyInput { code: KeyCode::ArrowDown, pressed: true, released: false, text: None });
    }
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let mut canvas = Canvas::Headless(&mut pixels);
    app.shared.hot.clear();
    let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT };
    app.stage.draw(&mut ctx, &mut canvas);
    pixels
}

/// Every value on the JUDGE tab reaches the frame: stepping one row changes what is painted, so a
/// row whose descriptor is wired to nothing cannot pass unnoticed. The cursor is walked down to the
/// row under test first, because the tab is longer than the list can show at once.
#[test]
fn every_judge_row_changes_the_frame_when_it_is_stepped() {
    let mut app = app();
    for (at, id) in tab_rows(SettingTab::Judge, &Config::default()).into_iter().enumerate() {
        app.shared.config = Config::default();
        let before = render_judge_tab(&mut app, at).pixel_checksum();
        assert_eq!(adjust(&mut app.shared.config, id, 1), AdjustOutcome::Changed, "{id:?} does not step");
        let after = render_judge_tab(&mut app, at).pixel_checksum();
        assert_ne!(before, after, "{id:?} steps without changing the frame");
    }
}

#[test]
fn a_screen_opened_over_another_resumes_it_exactly_where_it_was() {
    let mut app = app();
    app.switch(Transition::Open(Stage::Settings(SettingsState::new())));
    app.switch(Transition::Open(Stage::KeyConfig(KeyConfigState::new())));
    assert_eq!(app.stage.id(), StageId::KeyConfig);
    app.switch(Transition::Back);
    assert_eq!(app.stage.id(), StageId::Settings, "the key config editor returns to the settings screen it was opened from");
    app.switch(Transition::Back);
    assert_eq!(app.stage.id(), StageId::Select, "the settings screen returns to the browser");
    assert!(app.suspended.is_empty(), "nothing is left suspended");
}

#[test]
fn a_run_replaces_the_loading_screen_and_still_returns_to_the_browser() {
    let mut app = app();
    app.switch(Transition::Open(Stage::Loading(LoadingState::song(0))));
    app.switch(Transition::To(Stage::Play(Box::new(play_state()))));
    app.switch(Transition::To(Stage::Result(result_state())));
    assert_eq!(app.suspended.len(), 1, "only the browser is suspended under a run");
    app.switch(Transition::Back);
    assert_eq!(app.stage.id(), StageId::Select);
}

#[test]
fn going_back_with_nothing_suspended_lands_on_a_fresh_browser() {
    let mut app = app();
    app.switch(Transition::To(Stage::Play(Box::new(play_state()))));
    assert!(app.suspended.is_empty(), "replacing a screen suspends nothing");
    app.switch(Transition::Back);
    assert_eq!(app.stage.id(), StageId::Select);
}

#[test]
fn staying_leaves_the_screen_and_its_history_alone() {
    let mut app = app();
    app.switch(Transition::Open(Stage::Folders(FoldersState::new())));
    app.switch(Transition::Stay);
    assert_eq!(app.stage.id(), StageId::Folders);
    assert_eq!(app.suspended.len(), 1);
}

/// How old a scene is made before a transition is judged on what it does to the clock.
const SCENE_AGE: Duration = Duration::from_secs(10);

/// A visit to another screen far longer than the scene it interrupted had been running.
const LONG_VISIT: Duration = Duration::from_secs(100);

const SECOND_US: i64 = 1_000_000;

fn aged_app() -> App {
    let mut app = app();
    app.shared.age_skin_scene(SCENE_AGE);
    app
}

fn scene_age_us(app: &App) -> i64 {
    app.shared.skin_now_us()
}

#[test]
fn replacing_a_screen_switches_every_skin_timer_off_and_restarts_the_scene_clock() {
    let mut app = aged_app();
    app.shared.skin_timers.set_on(timer_id::PLAY, SECOND_US);
    app.shared.skin_timers.set_on(timer_id::PANEL1_ON, SECOND_US);
    assert!(scene_age_us(&app) >= 10 * SECOND_US, "the scene was not aged");

    app.switch(Transition::To(Stage::Folders(FoldersState::new())));
    assert!(!app.shared.skin_timers.is_on(timer_id::PLAY), "a timer outlived the scene it belonged to");
    assert!(!app.shared.skin_timers.is_on(timer_id::PANEL1_ON), "a timer outlived the scene it belonged to");
    assert!(scene_age_us(&app) < 5 * SECOND_US, "the clock carried on from the previous scene: {} us", scene_age_us(&app));
}

#[test]
fn the_timers_the_arriving_screen_switches_on_start_from_a_zero_clock() {
    let mut app = aged_app();
    app.switch(Transition::To(Stage::Result(result_state())));
    let began = app.shared.skin_timers.value_us(timer_id::RESULTGRAPH_BEGIN);
    assert!(app.shared.skin_timers.is_on(timer_id::RESULTGRAPH_BEGIN), "the result screen did not start its trend");
    assert!(began < 5 * SECOND_US, "the result screen timed its opening against the old clock: {began} us");
}

#[test]
fn going_back_with_nothing_suspended_starts_a_new_scene() {
    let mut app = aged_app();
    app.shared.skin_timers.set_on(timer_id::PLAY, SECOND_US);
    app.switch(Transition::Back);
    assert!(!app.shared.skin_timers.is_on(timer_id::PLAY));
    assert!(scene_age_us(&app) < 5 * SECOND_US);
}

#[test]
fn a_screen_opened_over_another_gets_a_scene_of_its_own() {
    let mut app = aged_app();
    app.shared.skin_timers.set_on(timer_id::PANEL1_ON, SECOND_US);
    app.switch(Transition::Open(Stage::Folders(FoldersState::new())));
    assert!(!app.shared.skin_timers.is_on(timer_id::PANEL1_ON), "the screen underneath leaked a timer into the one opened over it");
    assert!(scene_age_us(&app) < 5 * SECOND_US, "the opened screen inherited the clock of the one underneath");
}

/// The scene's clock is held against the time the test itself took rather than against a round
/// number of seconds: a machine busy enough to spend ten seconds between two lines of a test is
/// then no different from one that spends none, and the hundred seconds of the visit still cannot
/// hide in what is allowed.
#[test]
fn coming_back_resumes_the_scene_with_its_timers_and_a_clock_that_stood_still() {
    let mut app = aged_app();
    app.shared.skin_timers.set_on(timer_id::PANEL1_ON, SECOND_US);
    let began = Instant::now();
    let left_at = scene_age_us(&app);
    assert!(left_at >= 10 * SECOND_US, "the scene was not aged");
    app.switch(Transition::Open(Stage::Folders(FoldersState::new())));
    app.shared.age_skin_scene(LONG_VISIT);

    app.switch(Transition::Back);
    assert_eq!(app.stage.id(), StageId::Select);
    assert_eq!(app.shared.skin_timers.value_us(timer_id::PANEL1_ON), SECOND_US, "the timer was not put back as it was");
    let age = scene_age_us(&app);
    let spent_us = i64::try_from(began.elapsed().as_micros()).expect("a test does not run for centuries");
    assert!(age >= left_at, "the clock went back past where the scene was left: {age} us, left at {left_at} us");
    assert!(age <= left_at + spent_us, "the clock ran on while another screen was up: {age} us, left at {left_at} us with {spent_us} us spent since");
}

#[test]
fn each_screen_in_a_stack_gets_back_the_scene_it_was_parked_with() {
    let mut app = aged_app();
    app.shared.skin_timers.set_on(timer_id::PLAY, SECOND_US);
    app.switch(Transition::Open(Stage::Settings(SettingsState::new())));
    app.shared.skin_timers.set_on(timer_id::READY, 2 * SECOND_US);
    app.switch(Transition::Open(Stage::KeyConfig(KeyConfigState::new())));
    assert!(!app.shared.skin_timers.is_on(timer_id::READY), "the screen underneath leaked a timer into the one opened over it");

    app.switch(Transition::Back);
    assert_eq!(app.stage.id(), StageId::Settings);
    assert_eq!(app.shared.skin_timers.value_us(timer_id::READY), 2 * SECOND_US);
    assert!(!app.shared.skin_timers.is_on(timer_id::PLAY));

    app.switch(Transition::Back);
    assert_eq!(app.stage.id(), StageId::Select);
    assert_eq!(app.shared.skin_timers.value_us(timer_id::PLAY), SECOND_US);
    assert!(!app.shared.skin_timers.is_on(timer_id::READY));
}

/// The song clock and the skin clock are different clocks: a run starting resets the one the audio
/// is anchored to, and a document's opening must not jump when it does.
#[test]
fn starting_a_run_does_not_move_the_scene_clock() {
    let mut app = aged_app();
    app.shared.start_play();
    assert!(scene_age_us(&app) >= 10 * SECOND_US, "starting a run restarted the skin clock");
}
