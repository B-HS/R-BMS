//! The default skin pack held to drawing every screen it has a document for, on every machine.
//!
//! The captures in the parent module draw a pack somebody names with [`SKIN_PACK_ENV`], so they only
//! run where a person asks them to. The default pack is in the repository (`assets/skins/rbms-default`)
//! and is what a first run draws, so these tests name it themselves and run wherever the tests do:
//! one for the browser, the decide scene, the result, the course result, and the play scene of each of
//! the five key modes the pack has a document for.
//!
//! Each enters the screen the way the captures do -- through the same setup, with the same sample
//! chart or a chart written for the mode -- waits for the pack's document to compile, draws a frame
//! of its settled layout, and holds what came of it to what a skin's frame is made of rather than to
//! how it looks: the document compiled and nothing was said against it, the compiled screen has
//! objects and the kinds of object the screen is for, the app drew quads, the corners of the frame
//! are the pack's own background colour -- which the built-in screens are not drawn on -- and the
//! frame is not one flat colour. No picture of a whole screen is kept: the pack's looks are changed
//! on purpose and often, and a picture of it would only be re-recorded.
//!
//! The frames are drawn small ([`GATE_SIZE`], a third of the size the pack is authored at). A skin
//! scales to whatever target it is drawn on, and the cost of a frame on the processor's canvas is
//! what a run of all nine screens spends its time on.

use rbms_render::SkinObjectKind;
use rbms_skin::loader::SKIN_TYPE_PLAY_5KEYS;
use rbms_skin::loader::{SKIN_TYPE_PLAY_9KEYS, SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS};
use rbms_skin::resolve::plain_path;

use super::*;
use crate::stage::select::tests::entry;

/// The default pack's folder in the repository, from this package's own.
const DEFAULT_PACK_FOLDER: &str = "../../assets/skins/rbms-default";

/// The file of the pack that says which colours it is drawn in.
const THEME_FILE: &str = "lua/theme.lua";

/// What the line that gives the pack's background colour starts with. The colour follows as three
/// numbers in braces.
const BACKGROUND_LINE_START: &str = "bg = {";

/// The size every frame of the gate is drawn at, 16 to 9 like the pack and a third of its size.
const GATE_SIZE: (u32, u32) = (640, 360);

/// The moment of a scene the browser, decide and result screens are drawn at: past the fade the
/// pack opens them with and well inside the decide scene's length.
const GATE_SCENE_MS: i64 = 1_500;

/// How far into its chart a play scene is drawn, in milliseconds: enough for notes to be on the field.
const GATE_CHART_MS: i64 = 3_000;

/// The sample chart the screens that need a run are given, from this package's own.
const GATE_CHART: &str = DECIDE_SAMPLE_CHART;

/// How many songs the browser is given to list.
const GATE_SONGS: [&str; 5] = ["Aurora", "Borealis", "Cascade", "Drift", "Ember"];

/// The default pack, spelled the way the application spells a folder it was told about.
fn default_pack() -> PathBuf {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK_FOLDER);
    plain_path(&folder.canonicalize().unwrap_or_else(|error| panic!("{} cannot be resolved: {error}", folder.display())))
}

/// The pack's background colour, read out of the file that defines the pack's colours so the gate
/// follows the pack when its palette is changed.
fn background_of(pack: &Path) -> crate::Color {
    let theme = std::fs::read_to_string(pack.join(THEME_FILE)).unwrap_or_else(|error| panic!("the pack's {THEME_FILE} cannot be read: {error}"));
    let line = theme.lines().map(str::trim_start).find(|line| line.starts_with(BACKGROUND_LINE_START)).expect("the pack's colours name a background");
    let channels: Vec<u8> = line[BACKGROUND_LINE_START.len()..]
        .split('}')
        .next()
        .unwrap_or_default()
        .split(',')
        .map(|channel| channel.trim().parse().unwrap_or_else(|error| panic!("{channel:?} is not a colour channel: {error}")))
        .collect();
    let [r, g, b] = channels[..] else {
        panic!("the background is {channels:?}, not three channels");
    };
    crate::Color { r, g, b, a: u8::MAX }
}

/// A canvas the size of the gate's frames.
fn gate_canvas() -> HeadlessCanvas {
    HeadlessCanvas::new(GATE_SIZE.0, GATE_SIZE.1)
}

/// Hold the frame in `pixels`, which `app` has just drawn for `screen`, to being the pack's: the
/// document compiled and nothing was said against it, the compiled screen has a draw list holding
/// every kind of object in `kinds`, the app drew quads, and the frame is the pack's colour in its
/// corners and not one colour throughout.
fn assert_drawn_by_the_pack(label: &str, app: &App, screen: i32, kinds: &[SkinObjectKind], pixels: &HeadlessCanvas, background: crate::Color) {
    assert_eq!(app.shared.skin_failure(screen), None, "{label}: the pack's document failed");
    assert_eq!(app.shared.skin_warnings(screen), &[] as &[String], "{label}: the pack's document compiled with warnings");
    let compiled = app.shared.skin_screens.get(screen).unwrap_or_else(|| panic!("{label}: the application did not build the pack's screen"));
    assert!(compiled.object_count() > 0, "{label}: the pack's draw list is empty");
    for kind in kinds {
        assert!(compiled.count_of(*kind) > 0, "{label}: the pack's draw list has no {kind:?}");
    }
    assert!(pixels.quad_count() > 0, "{label}: the application drew nothing");
    save(&format!("gate-{}", label.replace(' ', "-")), CAPTURE_EXTENSION, GATE_SIZE, pixels.rgba());
    let (last_x, last_y) = (GATE_SIZE.0 - 1, GATE_SIZE.1 - 1);
    for (x, y) in [(0, 0), (last_x, 0), (0, last_y), (last_x, last_y)] {
        assert_eq!(pixels.pixel_at(x, y), background, "{label}: ({x}, {y}) is not the pack's background, so the pack did not draw the frame");
    }
    assert!(!is_flat(pixels, 0..GATE_SIZE.0, 0..GATE_SIZE.1), "{label}: the frame is one colour");
}

/// Run the scene that is up until the pack's document for `screen` has compiled, draw it at
/// [`GATE_SCENE_MS`] into its scene, and hold the frame to being the pack's.
fn gate_scene(label: &str, mut app: App, screen: i32, kinds: &[SkinObjectKind]) {
    let background = background_of(&default_pack());
    let mut pixels = gate_canvas();
    wait_for_scene_document(&mut app, screen, &mut Canvas::Headless(&mut pixels)).unwrap_or_else(|reason| panic!("{label}: not drawn: {reason}"));
    scene_frame_at(&mut app, GATE_SCENE_MS, &mut Canvas::Headless(&mut pixels));
    assert_drawn_by_the_pack(label, &app, screen, kinds, &pixels, background);
}

/// The sample chart, from this package's own folder.
fn gate_chart() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(GATE_CHART)
}

/// The browser lists the charts of a small library, and its draw list holds the song wheel.
#[test]
fn the_default_pack_draws_the_browser() {
    let mut app = pack_app(&default_pack(), "gate-select");
    app.shared.library = Library::from_songs(GATE_SONGS.iter().map(|title| entry(title, "artist", "7")).collect());
    app.shared.select_view = SelectView::AllSongs;
    app.shared.config.library.sort = SortMode::Title;
    app.shared.config.library.preview = false;
    app.shared.audio_failed = true;
    app.shared.rebuild_select_items();
    app.stage = browser();
    gate_scene("select", app, SKIN_TYPE_MUSIC_SELECT, &[SkinObjectKind::SongList, SkinObjectKind::Text, SkinObjectKind::Image]);
}

/// The decide scene is up for the sample chart.
#[test]
fn the_default_pack_draws_the_decide_scene() {
    let (app, _) = decide_scene_app(&default_pack(), "gate-decide", &gate_chart()).unwrap_or_else(|reason| panic!("decide: not entered: {reason}"));
    gate_scene("decide", app, SKIN_TYPE_DECIDE, &[SkinObjectKind::Text, SkinObjectKind::Reference]);
}

/// The result scene is up for a run the engine played to a clear.
#[test]
fn the_default_pack_draws_the_result_scene() {
    let (app, _) = result_scene_app(&default_pack(), "gate-result", &gate_chart(), true).unwrap_or_else(|reason| panic!("result: not entered: {reason}"));
    gate_scene("result", app, SKIN_TYPE_RESULT, &[SkinObjectKind::Text, SkinObjectKind::Number, SkinObjectKind::GaugeGraph]);
}

/// The course result scene is up for a course played through.
#[test]
fn the_default_pack_draws_the_course_result_scene() {
    let (app, _) = course_result_scene_app(&default_pack(), "gate-course-result", &gate_chart(), None)
        .unwrap_or_else(|reason| panic!("course result: not entered: {reason}"));
    gate_scene("course result", app, SKIN_TYPE_COURSE_RESULT, &[SkinObjectKind::Text, SkinObjectKind::Number, SkinObjectKind::GaugeGraph]);
}

/// A play scene for a chart written for the mode `sides` are the channels of, walked from its
/// loading until the chart plays and drawn [`GATE_CHART_MS`] into it. The play screen's draw list
/// holds the note field, the gauge and the judgement pop-ups.
fn gate_play(label: &str, screen: i32, sides: &[SideChannels], mode: Option<rbms_model::Mode>) {
    let pack = default_pack();
    let background = background_of(&pack);
    let name = format!("{label}.bms");
    let model = chart_model(&mode_chart(label, sides), &name, mode);
    let chart = settings_of(&format!("gate-play-{label}")).with_file_name(&name);
    let EnteredPlay { mut app, screen: entered, size: _, play_time_ms: _, arrived, _keysounds } =
        enter_play_scene(&pack, &format!("gate-{label}"), &chart, model, PlayEnding::StillPlaying)
            .unwrap_or_else(|reason| panic!("{label}: not entered: {reason}"));
    assert_eq!(entered, screen, "{label}: the mode is drawn by another screen");

    let mut pixels = gate_canvas();
    wait_for_scene_document(&mut app, screen, &mut Canvas::Headless(&mut pixels)).unwrap_or_else(|reason| panic!("{label}: not drawn: {reason}"));
    let skin = app.shared.skins.document(screen).expect("a compiled screen has its document");
    let timings = (i64::from(skin.play.loadend), i64::from(skin.play.playstart));
    let mut clocks = PlayClocks::default();
    clocks.open(&mut app, &mut pixels, &arrived, timings, |_, _, _, _| Ok(())).unwrap_or_else(|reason| panic!("{label}: not opened: {reason}"));
    let left_ms = GATE_CHART_MS - play_screen(&app).unwrap_or_else(|reason| panic!("{label}: {reason}")).chart_ms();
    clocks.play_on(&mut app, left_ms, &mut Canvas::Headless(&mut pixels)).unwrap_or_else(|reason| panic!("{label}: not played: {reason}"));
    assert_drawn_by_the_pack(label, &app, screen, &[SkinObjectKind::Note, SkinObjectKind::Gauge, SkinObjectKind::Judge], &pixels, background);
}

#[test]
fn the_default_pack_draws_the_play_scene_of_five_keys() {
    gate_play("five-keys", SKIN_TYPE_PLAY_5KEYS, &[FIVE_KEYS_FIRST], None);
}

#[test]
fn the_default_pack_draws_the_play_scene_of_seven_keys() {
    gate_play("seven-keys", SKIN_TYPE_PLAY_7KEYS, &[SEVEN_KEYS_FIRST], None);
}

#[test]
fn the_default_pack_draws_the_play_scene_of_nine_keys() {
    gate_play("nine-keys", SKIN_TYPE_PLAY_9KEYS, &[NINE_KEYS], Some(rbms_model::Mode::POPN_9K));
}

#[test]
fn the_default_pack_draws_the_play_scene_of_ten_keys() {
    gate_play("ten-keys", SKIN_TYPE_PLAY_10KEYS, &[FIVE_KEYS_FIRST, FIVE_KEYS_SECOND], Some(rbms_model::Mode::BEAT_10K));
}

#[test]
fn the_default_pack_draws_the_play_scene_of_fourteen_keys() {
    gate_play("fourteen-keys", SKIN_TYPE_PLAY_14KEYS, &[SEVEN_KEYS_FIRST, SEVEN_KEYS_SECOND], None);
}
