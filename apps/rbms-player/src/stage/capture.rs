//! The screen capture harness: one screen, drawn at one size and one moment of its scene, kept as a
//! frame a test can assert on and a PNG a person can open.
//!
//! A [`Shot`] says what is captured -- the name it is saved under, how large the target is, how far
//! into the scene the frame is drawn, and which document has to be compiled before the frame counts
//! -- and is taken on either backend: [`Shot::take`] on a [`HeadlessCanvas`], which every machine
//! has, and [`Shot::take_on_gpu`] on a window-less [`Gpu`], which only a machine with an adapter has.
//! The helpers under them are exported for the tests that need one frame at a time.
//!
//! Nothing is written unless [`CAPTURE_DIR_ENV`] names a folder, so the harness costs a test run
//! nothing. With it set, a headless frame lands in `<folder>/<name>.png` and the same frame drawn by
//! the GPU in `<folder>/<name>.gpu.png`. Tests run side by side, so each file is written by one test
//! only: two shots that share a name and a backend belong in the same test.
//!
//! What keeps two captures of one shot alike: the embedded fonts are the only ones loaded, an app
//! built by [`app_in`] reads a settings folder its test named for it, a run is laid out with the
//! default seed, and the scene clock is put where the shot says before the frame is drawn. Two
//! things are left to the caller. A skin that draws one of its files or its options at random does
//! so from the seed its app's skin library is pinned with (`SkinLibrary::pin_seed`), which has to be
//! set for the pick to repeat. And the scene clock is a real clock: a frame reads it a moment after
//! it was set, so the time a document sees is the shot's plus however long the frame has been
//! drawing.
//!
//! [`SKIN_PACK_ENV`] points the last test here at a skin pack somebody else wrote. It is the one test
//! that draws files from outside the repository, and it passes without doing anything when the
//! variable is not set. The test names the pack the way the PACK FOLDER row does, so each screen is
//! drawn with the document of the pack that declares it -- a Lua skin as readily as a document --
//! read against the screen's own state and with a pinned seed. A screen whose skin cannot be read is
//! reported and passed over rather than failed: which screens of a pack load is what the test is run
//! to find out.
//!
//! The decide scene is captured by a test of its own, because one frame says little about it: it is
//! a scene that runs from its opening to its fade, shown for a chart. That test opens the scene for a
//! real chart and saves a frame at each of several moments, walking the scene's own times on the way
//! so the timers a skin animates on are on since when they would be.
//!
//! A bitmap font is captured twice over. A pack written here, whose one font is the renderer's
//! distance field fixture, is drawn on both backends by a test that runs on every machine and holds
//! the two frames to each other. And the decide scene of the pack [`SKIN_PACK_ENV`] names is drawn
//! once more with the row that turns the pack's bitmap fonts on switched on, on both backends, so the
//! two can be laid side by side. Either way the frame that counts is the first one whose text had
//! every page of its fonts: a page is decoded off the frame loop, after the frame that first wanted
//! a glyph on it.
//!
//! The result scene is captured the same way and for the same reason, and for a run that was really
//! played: a chart that ships with the repository is run to its end by the engine, once played for
//! it and once with nobody playing, and the screen the application makes of each is walked from its
//! first frame, through its other menu, to its fade.
//!
//! The course result scene is walked the same way, for a course of that chart played through and
//! for one that fails before its last stage.
//!
//! The browser is captured over a library that was really scanned: the sample charts that ship with
//! the repository and a handful written here, with a record on most of them, so its wheel has every
//! lamp, every difficulty and every chart label to show. It is captured on a list of charts, on the
//! root with its folder and its table, inside the table, and on the course tab; and on the list of
//! charts once more with each of the three option panels that START and SELECT call up.
//!
//! The movies of the pack are captured as they play: its decide scene and its browser with their
//! backgrounds switched to a movie, and its play scene for a chart with no pictures of its own,
//! behind which the pack plays a movie unless it is told not to. Each is drawn at several moments of
//! its scene, and a frame waits for the movie frame that is due at its moment
//! ([`MOVIE_FRAME_PATIENCE`]), so what a capture shows of a movie is decided by the scene time it
//! was asked for and not by how far a decoder thread had got. Every app made over a pack waits that
//! way, the other captures' included.
//!
//! The play scene is captured as a run that is really made: a chart is handed to the play screen the
//! way the application hands one over, and the screen is walked through the reference's states on
//! the skin's own times -- loading, ready, playing, and the fade or the failure that closes it --
//! with the engine playing the chart, or nobody playing it, in between. Seven keys are walked the
//! whole way; five, ten and fourteen are captured while they play.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use rbms_config::DEFAULT_SKIN_FOLDER;
use rbms_course::{Course, CourseChart, CourseRun, StageResult};
use rbms_library::scan::ScanRequest;
use rbms_library::{Library, SongEntry};
use rbms_play::{NullSink, PlaySession, SessionClock, SessionOptions};
use rbms_render::Renderer;
use rbms_skin::loader::{SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_RESULT};
use rbms_skin::timer::{MICROS_PER_MILLI, timer_id};
use rbms_store::SCORE_LN_MODE_FROM_CHART;
use rbms_store::scoredb::{ScoreDb, migrate_score_book};
use winit::keyboard::KeyCode;

use crate::app_play::loaded_chart_for_tests;
use crate::app_result::enter_result;
use crate::gpu::Gpu;
use crate::skin_select::SKIN_PACK_ENV;
use crate::stage::canvas::UI_SIZE;
use crate::stage::render_tests::{FRAME_DT, app, play_state, render, render_on};
use crate::stage::scene_life::SceneTimes;
use crate::stage::select::tests::record;
use crate::stage::{Canvas, CourseResultState, DecideState, FrameCtx, HeadlessCanvas, KeyConfigState, KeyInput, PlayState, SelectState, Stage, Transition};
use crate::{App, Config, LaunchOptions, SelectView, SortMode};

/// Environment variable naming the folder captures are saved in. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// Extension of a headless capture.
const CAPTURE_EXTENSION: &str = "png";

/// Extension of the same frame drawn by the GPU, so the two sit side by side under one name.
const GPU_CAPTURE_EXTENSION: &str = "gpu.png";

/// How long a test goes on drawing frames for a document that is still on its way before it gives
/// the document up.
///
/// A document is compiled off the frame loop, so the frame it is selected on still draws the
/// built-in layout and the document takes over once its images have arrived. What a test waits for
/// is that: the document compiled, or known not to be coming. This is only what keeps a worker pool
/// that never finishes from hanging the test, and is far more than a small image needs on a machine
/// busy with every other test.
const DOCUMENT_LOAD_TIMEOUT: Duration = Duration::from_secs(60);

/// How long a frame that finds work still on the worker pool stands back for it. The frames a test
/// draws are not paced, and on a window-less GPU one is over in tens of microseconds.
const WORKER_PAUSE: Duration = Duration::from_millis(1);

/// The scene time of a frame drawn the moment its scene began.
pub(super) const SCENE_START_US: i64 = 0;

/// What one capture is taken of.
pub(super) struct Shot<'a> {
    /// The file name the capture is saved under, without its extension.
    pub(super) name: &'a str,
    /// The size of the target in its own pixels.
    pub(super) size: (u32, u32),
    /// How far into its scene the frame is drawn, in microseconds.
    pub(super) scene_us: i64,
    /// The skin type of the document that has to be compiled before the frame counts, or `None`
    /// for a screen drawn by its built-in layout.
    pub(super) document: Option<i32>,
}

/// Why a shot was not taken.
#[derive(Debug)]
pub(super) enum Missed {
    /// This machine has no graphics adapter to draw the shot with.
    NoAdapter,
    /// The document the shot waits on never finished compiling.
    NotCompiled,
}

impl Shot<'_> {
    /// Draw the shot on a headless canvas, save it when a capture folder was asked for, and hand
    /// the canvas back for whatever the caller wants to hold it to.
    pub(super) fn take(&self, app: &mut App, stage: impl Fn() -> Stage) -> Result<HeadlessCanvas, Missed> {
        let mut pixels = HeadlessCanvas::new(self.size.0, self.size.1);
        self.draw(app, &mut Canvas::Headless(&mut pixels), stage)?;
        save(self.name, CAPTURE_EXTENSION, self.size, pixels.rgba());
        Ok(pixels)
    }

    /// Draw the shot with the GPU backend, save it beside the headless capture, and hand back its
    /// pixels as tightly packed RGBA8 rows.
    ///
    /// A shot that waits on a document needs an app no other target has drawn: a compiled document
    /// holds textures registered with the target it was compiled against.
    pub(super) fn take_on_gpu(&self, app: &mut App, stage: impl Fn() -> Stage) -> Result<Vec<u8>, Missed> {
        let mut gpu = Gpu::offscreen_if_available(self.size.0, self.size.1, UI_SIZE).ok_or(Missed::NoAdapter)?;
        self.draw(app, &mut Canvas::Window(&mut gpu), stage)?;
        let rgba = gpu.capture().expect("an offscreen target reads back");
        save(self.name, GPU_CAPTURE_EXTENSION, self.size, &rgba);
        Ok(rgba)
    }

    /// Draw the shot's frame on `canvas`, after the document it waits on has compiled.
    fn draw(&self, app: &mut App, canvas: &mut Canvas<'_>, stage: impl Fn() -> Stage) -> Result<(), Missed> {
        let Some(screen) = self.document else {
            draw_on(app, stage(), self.scene_us, canvas);
            return Ok(());
        };
        if draw_until_compiled_on(app, screen, self.scene_us, canvas, stage) { Ok(()) } else { Err(Missed::NotCompiled) }
    }
}

/// The settings file of a folder that belongs to one test, with an empty skin folder beside it, so
/// a document written there cannot disturb any other capture.
pub(super) fn settings_of(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("rbms-capture-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join(DEFAULT_SKIN_FOLDER)).expect("the fixture folder is writable");
    directory.join("settings.ron")
}

/// The skin folder beside one test's settings file.
pub(super) fn skin_folder_of(settings: &Path) -> PathBuf {
    settings.parent().unwrap_or(Path::new(".")).join(DEFAULT_SKIN_FOLDER)
}

/// An app reading `settings` and configured by `config`, with no library, no window and no server,
/// and with the embedded fonts as the only ones it can draw text with.
pub(super) fn app_in(settings: PathBuf, config: Config) -> App {
    rbms_render::font::use_embedded_fonts_only();
    App::new(String::new(), config, LaunchOptions::default(), settings)
}

/// Draw one stage onto a headless canvas the caller keeps, `scene_us` microseconds into its scene.
///
/// The screen is placed directly rather than transitioned into, so its `on_enter` stays out of the
/// frame, and the scene's timers are left as they are: only the clock moves.
pub(super) fn draw_at(app: &mut App, stage: Stage, scene_us: i64, pixels: &mut HeadlessCanvas) {
    draw_on(app, stage, scene_us, &mut Canvas::Headless(pixels));
}

/// Draws the stage `stage` builds onto `pixels` until the document for `screen` has compiled, then
/// draws one more frame so the canvas shows the document rather than the last built-in frame before
/// it; answers whether it compiled in time.
///
/// One canvas throughout: the compiled document registers its textures with the target it was built
/// against, so a frame drawn onto a different canvas would find none of them. A fresh stage per
/// frame, because that is the only way a screen whose own state advances -- the loading screen
/// starts its work on its second frame -- can be drawn as many times as a document takes to arrive.
pub(super) fn draw_until_compiled(app: &mut App, screen: i32, scene_us: i64, pixels: &mut HeadlessCanvas, stage: impl Fn() -> Stage) -> bool {
    draw_until_compiled_on(app, screen, scene_us, &mut Canvas::Headless(pixels), stage)
}

/// [`draw_until_compiled`] on either kind of target.
///
/// The wait is for the document and not for a number of frames: it ends on the frame that finds the
/// document compiled, or on the first that finds it neither compiled nor on its way, which is a
/// document that could not be read.
fn draw_until_compiled_on(app: &mut App, screen: i32, scene_us: i64, canvas: &mut Canvas<'_>, stage: impl Fn() -> Stage) -> bool {
    let deadline = Instant::now() + DOCUMENT_LOAD_TIMEOUT;
    loop {
        draw_on(app, stage(), scene_us, canvas);
        if app.shared.has_compiled_skin(screen) {
            draw_on(app, stage(), scene_us, canvas);
            return true;
        }
        if !app.shared.skin_is_loading(screen) || Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(WORKER_PAUSE);
    }
}

/// [`draw_at`] on either kind of target: the scene clock is put where the frame was asked for, and
/// the frame is drawn the way the render snapshot tests draw one.
fn draw_on(app: &mut App, stage: Stage, scene_us: i64, canvas: &mut Canvas<'_>) {
    let age = Duration::from_micros(u64::try_from(scene_us).expect("a scene has no time before it began"));
    app.shared.scene_started = Instant::now().checked_sub(age).expect("the process has been up for longer than the scene time asked for");
    render_on(app, stage, canvas);
}

/// Save a frame under the capture folder, when one was asked for.
fn save(name: &str, extension: &str, size: (u32, u32), rgba: &[u8]) {
    if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
        save_in(Path::new(&folder), name, extension, size, rgba);
    }
}

/// Save `size` worth of tightly packed RGBA8 rows as `<folder>/<name>.<extension>`, creating the
/// folder if it is not there, and answer the file.
fn save_in(folder: &Path, name: &str, extension: &str, size: (u32, u32), rgba: &[u8]) -> PathBuf {
    std::fs::create_dir_all(folder).expect("the capture folder can be created");
    let path = folder.join(format!("{name}.{extension}"));
    image::save_buffer(&path, rgba, size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
    path
}

/// A target half as large again as the one the built-in screens are laid out for.
const LARGE: (u32, u32) = (1920, 1080);

/// How far one block of a frame's signature may sit from the same block of the same screen drawn at
/// another size: one step of the signature's own quantisation. An edge that falls on a whole pixel
/// at one size and on a fraction of one at the other moves a block's mean by less than that.
const SIGNATURE_TOLERANCE: u8 = 1;

/// How far one channel of a GPU frame may sit from the headless canvas's value, the same allowance
/// the GPU pixel tests make for the one truncating where the other rounds.
const GPU_TOLERANCE: u8 = 2;

/// One pixel in this many may sit further apart than [`GPU_TOLERANCE`]. The two backends decide
/// differently which pixels an edge on a fraction of a pixel covers, and a screen of text is mostly
/// such edges once it is scaled; a frame drawn in the wrong place or the wrong colour is apart
/// everywhere, not along its outlines.
const GPU_EDGE_PIXELS_ONE_IN: usize = 100;

/// A scene time far enough from zero that a clock left where it was could not be mistaken for it.
const LATE_US: i64 = 30_000_000;

/// How long after a frame was drawn its scene clock may be read and still count as the time asked
/// for.
const CLOCK_SLACK_US: i64 = 5_000_000;

/// A moment a timer is switched on at, distinct from every scene time the tests draw at.
const TIMER_ON_US: i64 = 1_000_000;

/// How far into its scene a pack's screen is captured: past the opening most documents animate in
/// with, so the frame shows the layout rather than the start of a fade.
const PACK_SCENE: Duration = Duration::from_secs(5);

/// The seed every read of a pack's skin is pinned with, so a skin that picks a file or an option at
/// random is captured the same way each time.
const PACK_SEED: u64 = 1;

/// Longest one screen of a pack is waited for. A published pack names hundreds of megabytes of
/// images for one screen, and they are decoded before the screen draws at all.
const PACK_LOAD_TIMEOUT: Duration = Duration::from_secs(180);

/// How long a frame of a captured pack waits for the frame of a movie that is due at its scene time.
/// A movie is decoded off the frame loop and a capture draws one frame, so without the wait a
/// capture would show whatever had been decoded by then, or nothing.
const MOVIE_FRAME_PATIENCE: Duration = Duration::from_secs(120);

/// One screen a pack is captured on.
struct PackScreen {
    /// The skin type a document declares to be drawn on this screen.
    skin_type: i32,
    /// The name the screen's capture is saved under.
    name: &'static str,
    /// Builds the stage that draws the screen.
    stage: fn() -> Stage,
}

/// The screens a pack is captured on one frame each. The decide scene and the result scene are each
/// captured through their whole length by a test of their own.
const PACK_SCREENS: [PackScreen; 3] = [
    PackScreen { skin_type: SKIN_TYPE_MUSIC_SELECT, name: "pack-select", stage: browser },
    PackScreen { skin_type: SKIN_TYPE_KEY_CONFIG, name: "pack-keyconfig", stage: key_config },
    PackScreen { skin_type: SKIN_TYPE_PLAY_7KEYS, name: "pack-play7", stage: play },
];

fn browser() -> Stage {
    Stage::Select(Box::new(SelectState::new()))
}

fn key_config() -> Stage {
    Stage::KeyConfig(KeyConfigState::new())
}

fn play() -> Stage {
    Stage::Play(Box::new(play_state()))
}

/// The built-in browser at `size`, the moment its scene began.
fn browser_shot(name: &str, size: (u32, u32)) -> Shot<'_> {
    Shot { name, size, scene_us: SCENE_START_US, document: None }
}

/// Whether every pixel of a rectangle is the one colour, which is what a part of the target nothing
/// was drawn on looks like.
fn is_flat(pixels: &HeadlessCanvas, columns: Range<u32>, mut rows: Range<u32>) -> bool {
    let first = pixels.pixel_at(columns.start, rows.start);
    rows.all(|y| columns.clone().all(|x| pixels.pixel_at(x, y) == first))
}

/// An app with `pack` named as its skin pack and walked, and every read of a skin pinned.
fn pack_app(pack: &Path, tag: &str) -> App {
    let settings = settings_of(tag);
    let mut config = Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
    app.shared.skin_screens.wait_for_movie_frames(Some(MOVIE_FRAME_PATIENCE));
    app
}

/// The size the pack's document for `screen` says it was authored at, which is the size it is
/// captured at, or `None` when the pack has no document for that screen.
fn authored_size(app: &App, screen: i32) -> Option<(u32, u32)> {
    let header = app.shared.skins.header_of(&app.shared.config, screen)?;
    let side = |length: i32, fallback: u32| u32::try_from(length).ok().filter(|length| *length > 0).unwrap_or(fallback);
    Some((side(header.width, UI_SIZE.0), side(header.height, UI_SIZE.1)))
}

/// Draws the stage `stage` builds from the first moment of its scene until the document for
/// `screen` has been read and compiled, then once more [`PACK_SCENE`] into the scene so the
/// target shows the document past its opening; or answers the first line of why it never was.
///
/// The scene is entered the way the application enters one. Its clock is put at its beginning once
/// and then left alone: it stands still while the document is on its way and starts on the frame the
/// document first draws, which is where a skin's Lua first sees the time and where the timers it
/// keeps for itself are started from. Only then is the scene made as old as the capture asks for.
/// Putting the clock at the capture's time on every frame instead would show the skin at the moment
/// it was first run -- an opening not yet begun, not a layout that has settled.
///
/// A Lua skin is read on the screen's first frame, against the state that frame is drawn from, so a
/// skin that cannot be read is known by the second frame and nothing is waited for.
fn draw_until_settled(app: &mut App, screen: i32, canvas: &mut Canvas<'_>, stage: fn() -> Stage) -> Result<(), String> {
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    draw_on(app, stage(), SCENE_START_US, canvas);
    loop {
        if let Some(reason) = app.shared.skin_failure(screen) {
            return Err(reason.lines().next().unwrap_or_default().to_owned());
        }
        if app.shared.has_compiled_skin(screen) {
            app.shared.age_skin_scene(PACK_SCENE);
            render_on(app, stage(), canvas);
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("its files were not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs()));
        }
        render_on(app, stage(), canvas);
    }
}

/// What one file of a pack is told apart by between two listings: its length, and when it was last
/// written where the file system says. A file rewritten at the same length shows by the second.
type FileStamp = (u64, Option<SystemTime>);

/// Every file under `root`, as its path relative to it and what it is told apart by.
fn files_under(root: &Path) -> BTreeMap<PathBuf, FileStamp> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_path_buf(), (metadata.len(), metadata.modified().ok()));
            }
        }
    }
    found
}

/// The contract the harness stands on: at the size the built-in screens are laid out for, a capture
/// is byte for byte the frame the render snapshot tests draw.
#[test]
fn a_capture_at_the_ui_size_is_the_frame_the_render_tests_draw() {
    let mut app = app();
    let reference = render(&mut app, browser());
    let captured = browser_shot("select-1280x720", UI_SIZE).take(&mut app, browser).expect("a built-in screen waits on nothing");

    assert_eq!(captured.size(), UI_SIZE);
    assert_eq!(captured.pixel_checksum(), reference.pixel_checksum(), "the harness drew a different frame from the render tests");
}

/// A larger target shows the same screen, scaled to fill it: block for block the frame matches the
/// one drawn at the size the screen is laid out for, and the part of the target that size would not
/// have reached is drawn on rather than left as it was cleared.
#[test]
fn a_larger_capture_is_the_same_screen_filling_the_larger_target() {
    let mut app = app();
    let small = render(&mut app, browser());
    let large = browser_shot("select-1920x1080", LARGE).take(&mut app, browser).expect("a built-in screen waits on nothing");

    assert_eq!(large.size(), LARGE);
    let (small_blocks, large_blocks) = (small.signature(), large.signature());
    let apart = small_blocks.iter().zip(&large_blocks).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    assert!(apart <= SIGNATURE_TOLERANCE, "the larger frame is not the smaller one scaled up: a block is {apart} steps apart");

    assert!(!is_flat(&large, UI_SIZE.0..LARGE.0, 0..UI_SIZE.1), "nothing was drawn to the right of where an unscaled screen ends");
    assert!(!is_flat(&large, 0..LARGE.0, UI_SIZE.1..LARGE.1), "nothing was drawn below where an unscaled screen ends");
}

/// The same shot taken with the GPU backend is the frame the headless canvas drew, at the size the
/// screen is laid out for and at a size it is scaled out to.
#[test]
fn a_capture_taken_on_the_gpu_agrees_with_the_headless_one() {
    for (name, size) in [("select-1280x720", UI_SIZE), ("select-1920x1080", LARGE)] {
        let mut app = app();
        let shot = browser_shot(name, size);
        let from_gpu = match shot.take_on_gpu(&mut app, browser) {
            Err(Missed::NoAdapter) => return,
            taken => taken.expect("a built-in screen waits on nothing"),
        };
        let mut headless = HeadlessCanvas::new(size.0, size.1);
        draw_at(&mut app, browser(), shot.scene_us, &mut headless);

        assert_eq!(from_gpu.len(), headless.rgba().len(), "{name}: the two frames are not the same size");
        let pixels = from_gpu.chunks_exact(rbms_render::BYTES_PER_PIXEL).zip(headless.rgba().chunks_exact(rbms_render::BYTES_PER_PIXEL));
        let apart = pixels.filter(|(gpu, cpu)| gpu.iter().zip(*cpu).any(|(a, b)| a.abs_diff(*b) > GPU_TOLERANCE)).count();
        let allowed = (size.0 as usize * size.1 as usize) / GPU_EDGE_PIXELS_ONE_IN;
        assert!(apart <= allowed, "{name}: {apart} pixels differ between the backends, past the {allowed} that edges account for");
    }
}

/// A frame is drawn with the scene clock where the shot put it, earlier or later than it was, and
/// with the scene's timers as they were: moving the clock is not starting a scene.
#[test]
fn a_frame_is_drawn_at_the_scene_time_asked_for_with_the_timers_left_alone() {
    let mut app = app();
    app.shared.skin_timers.set_on(timer_id::PLAY, TIMER_ON_US);
    let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);

    draw_at(&mut app, browser(), LATE_US, &mut pixels);
    let late = app.shared.skin_now_us();
    assert!((LATE_US..LATE_US + CLOCK_SLACK_US).contains(&late), "the frame was drawn {late} us into the scene, not {LATE_US}");
    assert_eq!(app.shared.skin_timers.value_us(timer_id::PLAY), TIMER_ON_US, "drawing at a scene time moved a timer");

    draw_at(&mut app, browser(), SCENE_START_US, &mut pixels);
    let early = app.shared.skin_now_us();
    assert!((SCENE_START_US..CLOCK_SLACK_US).contains(&early), "the clock could not be put back: {early} us");
}

/// What is saved is the frame, under the name and the extension it was given, in a folder that did
/// not have to exist.
#[test]
fn a_saved_capture_reads_back_as_the_frame_it_was_taken_of() {
    const SIZE: (u32, u32) = (2, 1);
    const FRAME: [u8; 8] = [10, 20, 30, 255, 200, 100, 50, 128];
    let folder = settings_of("save").with_file_name("captures").join("nested");

    for extension in [CAPTURE_EXTENSION, GPU_CAPTURE_EXTENSION] {
        let path = save_in(&folder, "frame", extension, SIZE, &FRAME);
        assert_eq!(path, folder.join(format!("frame.{extension}")));
        let read = image::open(&path).expect("the capture is an image").into_rgba8();
        assert_eq!(read.dimensions(), SIZE);
        assert_eq!(read.as_raw().as_slice(), FRAME.as_slice(), "the saved pixels are not the frame's");
    }
}

/// Captures a pack somebody else wrote: the document it holds for each screen, on a target the size
/// the document was authored at, on both backends.
///
/// Opt-in, because the pack is outside the repository: without [`SKIN_PACK_ENV`] this passes without
/// drawing anything. With it, every screen is tried and what became of each is printed. A screen
/// whose skin could not be read is not a failure of this test -- a pack written for another program
/// asks its host for more than every screen here answers yet. What is held to is that the pack drew
/// at least one of its screens, since a run that captured nothing has shown nothing, and that
/// reading and drawing the pack left every file in its folder at the length and the time it had.
#[test]
fn a_skin_pack_named_by_the_environment_is_captured_one_document_a_screen() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let before = files_under(&pack);
    let mut captured = Vec::new();

    for PackScreen { skin_type: screen, name, stage } in PACK_SCREENS {
        let mut app = pack_app(&pack, name);
        let Some(size) = authored_size(&app, screen) else {
            println!("{name}: the pack has no document for this screen");
            continue;
        };
        let document = app.shared.skins.document_path(&app.shared.config, screen).unwrap_or_default().to_owned();

        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        match draw_until_settled(&mut app, screen, &mut Canvas::Headless(&mut pixels), stage) {
            Ok(()) => {
                save(name, CAPTURE_EXTENSION, size, pixels.rgba());
                captured.push(name);
                println!("{name}: {document} loaded and captured at {}x{}", size.0, size.1);
            }
            Err(reason) => {
                println!("{name}: {document} not drawn: {reason}");
                continue;
            }
        }

        let mut app = pack_app(&pack, &format!("{name}-gpu"));
        let Some(mut gpu) = Gpu::offscreen_if_available(size.0, size.1, UI_SIZE) else {
            println!("{name}.gpu: this machine has no graphics adapter");
            continue;
        };
        match draw_until_settled(&mut app, screen, &mut Canvas::Window(&mut gpu), stage) {
            Ok(()) => {
                let rgba = gpu.capture().expect("an offscreen target reads back");
                save(name, GPU_CAPTURE_EXTENSION, size, &rgba);
                println!("{name}.gpu: captured at {}x{}", size.0, size.1);
            }
            Err(reason) => println!("{name}.gpu: not drawn: {reason}"),
        }
    }
    assert!(!captured.is_empty(), "{} drew none of its screens", pack.display());
    assert_eq!(files_under(&pack), before, "capturing the pack changed its folder");
}

/// The chart the decide scene is captured for: a sample that ships with the repository, named from
/// this crate's folder.
const DECIDE_SAMPLE_CHART: &str = "../../samples/preview-demo/preview-demo.bms";

/// The moments of the decide scene that are captured for the sample chart, in milliseconds from its
/// beginning: its first frame, the opening, the settled layout, the end of its length, and three
/// moments of the fade that follows. A skin that fades for a second after a scene of three is half a
/// second, three quarters and all but through its fade at the last three.
const DECIDE_SHOTS_MS: [i64; 7] = [0, 500, 1500, 3000, 3500, 3750, 3950];

/// The moments captured for the chart written below, which has what the sample lacks.
const DECIDE_RICH_SHOTS_MS: [i64; 2] = [1500, 3750];

/// How many measures the written chart runs for, and where its tempo halves, quadruples and returns.
const RICH_MEASURES: u32 = 48;
const RICH_SLOW_FROM: u32 = 12;
const RICH_FAST_FROM: u32 = 24;
const RICH_HOME_FROM: u32 = 36;

/// The size of the stage image written beside that chart.
const RICH_STAGE_SIZE: (u32, u32) = (640, 480);

/// The file that image is written to, which the chart names.
const RICH_STAGE_FILE: &str = "stage.png";

/// A chart with everything a decide skin can show: a named difficulty and a level, a stage image, a
/// tempo that changes, long notes, and enough notes on the keys and the turntable to fill a graph.
fn rich_chart() -> String {
    const KEY_CHANNELS: [u32; 7] = [11, 12, 13, 14, 15, 18, 19];
    const SCRATCH_CHANNEL: u32 = 16;
    const LONG_CHANNEL: u32 = 51;
    /// The channel the plain notes of the key that long note is in are written on. They are left out
    /// of a measure that has the long note, so no note sits inside it where nobody could play it.
    const LONG_NOTE_KEY_CHANNEL: u32 = 11;
    const TEMPO_CHANNEL: u32 = 8;
    /// The most notes a key is given in one measure; the count climbs to it and starts over.
    const BUSIEST_MEASURE_NOTES: usize = 8;
    /// One key in this many rests in each measure, a different one each time.
    const RESTING_KEY_EVERY: usize = 3;
    /// One measure in this many has turntable notes and a long note.
    const TURNTABLE_EVERY: u32 = 4;
    let mut chart = String::from(concat!(
        "#PLAYER 1\n#GENRE Capture Genre\n#TITLE Capture Title\n#SUBTITLE [ANOTHER]\n#ARTIST Capture Artist\n#BPM 150\n",
        "#PLAYLEVEL 12\n#DIFFICULTY 4\n#RANK 2\n#TOTAL 300\n#LNTYPE 1\n#BPM01 75\n#BPM02 300\n#BPM03 150\n#WAV01 a.wav\n",
    ));
    chart.push_str(&format!("#STAGEFILE {RICH_STAGE_FILE}\n"));
    for measure in 0..RICH_MEASURES {
        let notes = 1 + measure as usize % BUSIEST_MEASURE_NOTES;
        let has_long_note = measure.is_multiple_of(TURNTABLE_EVERY);
        for (index, channel) in KEY_CHANNELS.into_iter().enumerate() {
            let under_the_long_note = has_long_note && channel == LONG_NOTE_KEY_CHANNEL;
            if !(measure as usize + index).is_multiple_of(RESTING_KEY_EVERY) && !under_the_long_note {
                chart.push_str(&format!("#{measure:03}{channel}:{}\n", "01".repeat(notes)));
            }
        }
        if has_long_note {
            chart.push_str(&format!("#{measure:03}{SCRATCH_CHANNEL}:0101\n#{measure:03}{LONG_CHANNEL}:01000001\n"));
        }
        let tempo = match measure {
            RICH_SLOW_FROM => Some("01"),
            RICH_FAST_FROM => Some("02"),
            RICH_HOME_FROM => Some("03"),
            _ => None,
        };
        if let Some(tempo) = tempo {
            chart.push_str(&format!("#{measure:03}{TEMPO_CHANNEL:02}:{tempo}\n"));
        }
    }
    chart
}

/// Write the chart above and its stage image into a folder of this test's own, and answer the chart.
fn write_rich_chart(tag: &str) -> PathBuf {
    let folder = settings_of(tag).with_file_name("chart");
    std::fs::create_dir_all(&folder).expect("the chart folder is writable");
    let (width, height) = RICH_STAGE_SIZE;
    let stage = image::RgbaImage::from_fn(width, height, |x, y| {
        let band = u8::try_from((x * u32::from(u8::MAX)) / width).unwrap_or(u8::MAX);
        let rise = u8::try_from((y * u32::from(u8::MAX)) / height).unwrap_or(u8::MAX);
        image::Rgba([band, rise, u8::MAX - band, u8::MAX])
    });
    stage.save(folder.join(RICH_STAGE_FILE)).expect("the stage image is written");
    let chart = folder.join("rich.bms");
    std::fs::write(&chart, rich_chart()).expect("the chart is written");
    chart
}

/// One frame of the stage that is up with the scene clock put at `scene_ms`: its update and then its
/// draw, which is the order the application runs them in. Answers what the update asked for.
pub(super) fn scene_frame_at(app: &mut App, scene_ms: i64, canvas: &mut Canvas<'_>) -> Transition {
    let age = Duration::from_millis(u64::try_from(scene_ms).expect("a scene has no time before it began"));
    app.shared.scene_started = Instant::now().checked_sub(age).expect("the process has been up for longer than the scene time asked for");
    let now = Instant::now();
    let transition = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
    if matches!(transition, Transition::Stay) {
        app.shared.hot.clear();
        app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, canvas);
    }
    transition
}

/// Open the pack's decide scene for the chart at `chart` and save a frame at each of `shots_ms` as
/// `<prefix>-<milliseconds>ms`. Answers the names saved, or why the scene never drew.
///
/// The scene is walked rather than jumped through: besides the moments that are saved, a frame is
/// run just past the scene's input time and just past its length, so `STARTINPUT` and `FADEOUT` go
/// on when the skin's own times say and a later frame shows a fade as far along as it would be.
fn capture_decide_scene(pack: &Path, tag: &str, chart: &Path, prefix: &str, shots_ms: &[i64]) -> Result<Vec<String>, String> {
    let mut app = pack_app(pack, tag);
    let size = authored_size(&app, SKIN_TYPE_DECIDE).ok_or_else(|| "the pack has no document for this screen".to_owned())?;
    let bytes = std::fs::read(chart).map_err(|error| format!("{} could not be read: {error}", chart.display()))?;
    app.shared.chart_path = chart.to_string_lossy().into_owned();
    let loaded = loaded_chart_for_tests(&bytes, &app.shared.chart_path);
    app.stage = Stage::Decide(Box::new(DecideState::loaded(&app.shared, loaded)));

    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !app.shared.has_compiled_skin(SKIN_TYPE_DECIDE) {
        scene_frame_at(&mut app, 0, &mut Canvas::Headless(&mut pixels));
        if let Some(reason) = app.shared.skin_failure(SKIN_TYPE_DECIDE) {
            return Err(reason.lines().next().unwrap_or_default().to_owned());
        }
        if Instant::now() >= deadline {
            return Err(format!("its files were not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs()));
        }
    }

    let times = SceneTimes::of_skin(app.shared.skins.document(SKIN_TYPE_DECIDE));
    let last = shots_ms.iter().copied().max().unwrap_or_default();
    let edges = [times.input_ms + 1, times.scene_ms + 1].into_iter().filter(|edge| *edge < last);
    let mut moments: Vec<i64> = shots_ms.iter().copied().chain(edges).collect();
    moments.sort_unstable();
    moments.dedup();
    let mut saved = Vec::new();
    for at_ms in moments {
        let transition = scene_frame_at(&mut app, at_ms, &mut Canvas::Headless(&mut pixels));
        if !matches!(transition, Transition::Stay) {
            return Err(format!("the scene was over by {at_ms} ms"));
        }
        if shots_ms.contains(&at_ms) {
            let name = format!("{prefix}-{at_ms:04}ms");
            save(&name, CAPTURE_EXTENSION, size, pixels.rgba());
            saved.push(name);
        }
    }
    Ok(saved)
}

/// Captures the decide scene of a pack somebody else wrote, from its first frame to its fade, for a
/// chart that ships with the repository and for one written here that has a stage image, a named
/// difficulty and a tempo that changes.
///
/// Opt-in like the capture above it: without [`SKIN_PACK_ENV`] this passes without drawing anything.
/// With it, the scene has to draw for both charts, and the pack's folder has to be left as it was.
#[test]
fn the_decide_scene_of_a_skin_pack_named_by_the_environment_is_captured_from_its_opening_to_its_fade() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let before = files_under(&pack);

    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join(DECIDE_SAMPLE_CHART);
    let saved =
        capture_decide_scene(&pack, "decide-sample", &sample, "decide", &DECIDE_SHOTS_MS).unwrap_or_else(|reason| panic!("decide: not drawn: {reason}"));
    println!("decide: captured {}", saved.join(", "));

    let rich = write_rich_chart("decide-rich-chart");
    let saved = capture_decide_scene(&pack, "decide-rich", &rich, "decide-rich", &DECIDE_RICH_SHOTS_MS)
        .unwrap_or_else(|reason| panic!("decide-rich: not drawn: {reason}"));
    println!("decide-rich: captured {}", saved.join(", "));

    assert_eq!(files_under(&pack), before, "capturing the pack's decide scene changed its folder");
}

/// The folder of the renderer's bitmap font fixtures, named from this crate's folder, and the files
/// of the distance field font among them.
const FIELD_FONT_FOLDER: &str = "../../crates/rbms-render/tests/skin/fonts";
const FIELD_FONT_FILES: [&str; 2] = ["field.fnt", "field_0.png"];

/// The size the fixture font was made at, which draws it one to one on a target of the size the
/// document below is authored at.
const FIELD_FONT_SIZE: u32 = 32;

/// Where the document's first line starts and how far below the top of the screen its top edge is,
/// and how many times the font's own size it is drawn.
const FIELD_LINE_LEFT: u32 = 100;
const FIELD_LINE_TOP: u32 = 100;
const FIELD_LINE_ENLARGED: u32 = 2;

/// How far below the top of the screen the document's second and third lines are, and the room
/// every line is given.
const FIELD_PLAIN_TOP: u32 = 300;
const FIELD_SMALL_TOP: u32 = 420;
const FIELD_LINE_ROOM: u32 = 900;

/// How far right of a line's start and below its top edge the middle of the fixture's `O` is, and
/// the radius of its disc, at the size the font was made at.
const FIELD_DISC_MIDDLE: u32 = 16;
const FIELD_DISC_RADIUS: f32 = 12.0;

/// How far past the disc, in the font's own texels, the middle of the first line's outline is: the
/// outline is a quarter of the field's range wide, which is two texels, and its inner texel is
/// wholly the outline's colour.
const FIELD_OUTLINE_MIDDLE: f32 = 1.0;

/// The colours the first line is drawn in: its glyphs, its outline and its shadow.
const FIELD_INK: crate::Color = crate::Color { r: 255, g: 255, b: 255, a: 255 };
const FIELD_OUTLINE: crate::Color = crate::Color { r: 255, g: 0, b: 0, a: 255 };

/// A pack of one test's own: a key configuration document whose only font is the fixture's
/// distance field font, showing a line with an outline and a soft shadow at twice the font's size,
/// a translucent line at the font's own size with neither, and a line at half of it.
fn field_font_pack(tag: &str) -> PathBuf {
    let pack = settings_of(tag).with_file_name("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIELD_FONT_FOLDER);
    for file in FIELD_FONT_FILES {
        std::fs::copy(fixture.join(file), pack.join(file)).expect("the fixture font is copied");
    }
    let (width, height) = UI_SIZE;
    let (large, plain, small) = (FIELD_FONT_SIZE * FIELD_LINE_ENLARGED, FIELD_FONT_SIZE, FIELD_FONT_SIZE / 2);
    let line = |id: &str, top: u32, size: u32, color: &str| {
        let bottom = height - top - size;
        format!(r#"{{ "id": "{id}", "dst": [{{ "time": 0, "x": {FIELD_LINE_LEFT}, "y": {bottom}, "w": {FIELD_LINE_ROOM}, "h": {size}{color} }}] }}"#)
    };
    let document = format!(
        r#"{{
            "type": {SKIN_TYPE_KEY_CONFIG}, "name": "Distance field font", "w": {width}, "h": {height},
            "font": [{{ "id": "0", "path": "{}", "type": 1 }}],
            "text": [
                {{ "id": "dressed", "font": "0", "size": {large}, "constantText": "OIO IO",
                   "outlineColor": "ff0000ff", "outlineWidth": 0.5,
                   "shadowColor": "2040ffc8", "shadowOffsetX": 3, "shadowOffsetY": 3, "shadowSmoothness": 0.25 }},
                {{ "id": "plain", "font": "0", "size": {plain}, "constantText": "OOII OIOI" }},
                {{ "id": "small", "font": "0", "size": {small}, "constantText": "OIOIOIOI IIOO",
                   "outlineColor": "00c060ff", "outlineWidth": 0.4 }}
            ],
            "destination": [{}, {}, {}]
        }}"#,
        FIELD_FONT_FILES[0],
        line("dressed", FIELD_LINE_TOP, large, ""),
        line("plain", FIELD_PLAIN_TOP, plain, r#", "r": 250, "g": 220, "b": 40, "a": 230"#),
        line("small", FIELD_SMALL_TOP, small, ""),
    );
    std::fs::write(pack.join("keys.json"), document).expect("the document is written");
    pack
}

/// Draws frames with `frame`, ending each the way the application ends one, until the document for
/// `screen` has compiled and a frame has been drawn whose text had every font page it asked for.
///
/// A bitmap font's pages are decoded off the frame loop and only once a glyph on one is to be
/// drawn, so the frame a line is first drawn on wants its pages and shows nothing of it, and the
/// frame its last page arrives on was drawn before that. The frame that counts is the one after: it
/// ends with no page wanted and no more pages in than the frame before it ended with.
fn draw_until_text_settles(app: &mut App, screen: i32, canvas: &mut Canvas<'_>, mut frame: impl FnMut(&mut App, &mut Canvas<'_>)) -> Result<(), String> {
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    let mut pages_in = None;
    loop {
        frame(app, canvas);
        app.shared.finish_skin_frame(canvas);
        if let Some(reason) = app.shared.skin_failure(screen) {
            return Err(reason.lines().next().unwrap_or_default().to_owned());
        }
        let settled = app.shared.skin_screens.get(screen).filter(|compiled| compiled.wanted_font_pages().is_empty()).map(|compiled| compiled.font_page_count());
        if settled.is_some() && settled == pages_in {
            return Ok(());
        }
        pages_in = settled;
        if Instant::now() >= deadline {
            return Err(format!("its text was not drawn within {} seconds", PACK_LOAD_TIMEOUT.as_secs()));
        }
        std::thread::sleep(WORKER_PAUSE);
    }
}

/// How far apart two frames of one shot are: how many pixels have a channel more than
/// [`GPU_TOLERANCE`] apart, and the widest any channel is apart.
fn frames_apart(one: &[u8], other: &[u8]) -> (usize, u8) {
    assert_eq!(one.len(), other.len(), "the two frames are not the same size");
    let pixels = one.chunks_exact(rbms_render::BYTES_PER_PIXEL).zip(other.chunks_exact(rbms_render::BYTES_PER_PIXEL));
    let widest = |(one, other): &(&[u8], &[u8])| one.iter().zip(*other).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    pixels.map(|pair| widest(&pair)).fold((0, 0), |(apart, worst), widest| (apart + usize::from(widest > GPU_TOLERANCE), worst.max(widest)))
}

/// One pixel of a frame given as tightly packed RGBA8 rows of `width` pixels.
fn pixel_of(rgba: &[u8], width: u32, x: u32, y: u32) -> crate::Color {
    let at = (y as usize * width as usize + x as usize) * rbms_render::BYTES_PER_PIXEL;
    crate::Color { r: rgba[at], g: rgba[at + 1], b: rgba[at + 2], a: rgba[at + 3] }
}

/// The name the distance field fixture's capture is saved under.
const FIELD_FONT_SHOT: &str = "field-font-1920x1080";

/// A document whose font is a distance field font is drawn by the application through the distance
/// field shader on both backends, and the two frames are the same frame. The target is half as
/// large again as the document is authored at, so every glyph is scaled by a factor that is not a
/// whole number and lands on fractions of a pixel.
///
/// The pack is written here from the renderer's own fixture, so this runs wherever the tests do; on
/// a machine with no adapter the headless half still holds the frame to what the shader has to
/// draw.
#[test]
fn a_distance_field_font_is_captured_alike_on_the_gpu_and_on_the_headless_canvas() {
    let (name, size) = (FIELD_FONT_SHOT, LARGE);
    let pack = field_font_pack("field-font-files");
    let key_config_frame = |app: &mut App, canvas: &mut Canvas<'_>| draw_on(app, key_config(), SCENE_START_US, canvas);
    let scale = size.0 as f32 / UI_SIZE.0 as f32;
    let enlarged = scale * FIELD_LINE_ENLARGED as f32;
    let middle_of = |start: u32| scale * (start + FIELD_LINE_ENLARGED * FIELD_DISC_MIDDLE) as f32;
    let middle = (middle_of(FIELD_LINE_LEFT), middle_of(FIELD_LINE_TOP));
    let along_the_diagonal = (FIELD_DISC_RADIUS + FIELD_OUTLINE_MIDDLE) * enlarged * std::f32::consts::FRAC_1_SQRT_2;
    let in_the_disc = (middle.0 as u32, middle.1 as u32);
    let in_the_outline = ((middle.0 + along_the_diagonal) as u32, (middle.1 + along_the_diagonal) as u32);

    let mut app = pack_app(&pack, name);
    let mut headless = HeadlessCanvas::new(size.0, size.1);
    draw_until_text_settles(&mut app, SKIN_TYPE_KEY_CONFIG, &mut Canvas::Headless(&mut headless), key_config_frame)
        .unwrap_or_else(|reason| panic!("{name}: not drawn: {reason}"));
    save(name, CAPTURE_EXTENSION, size, headless.rgba());
    assert!(app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG).is_empty(), "{name}: {:?}", app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG));
    assert_eq!(headless.pixel_at(in_the_disc.0, in_the_disc.1), FIELD_INK, "{name}: the middle of the first glyph is the text's colour");
    assert_eq!(headless.pixel_at(in_the_outline.0, in_the_outline.1), FIELD_OUTLINE, "{name}: and a texel past its edge is its outline's");

    let mut app = pack_app(&pack, &format!("{name}-gpu"));
    let Some(mut gpu) = Gpu::offscreen_if_available(size.0, size.1, UI_SIZE) else {
        return;
    };
    draw_until_text_settles(&mut app, SKIN_TYPE_KEY_CONFIG, &mut Canvas::Window(&mut gpu), key_config_frame)
        .unwrap_or_else(|reason| panic!("{name}.gpu: not drawn: {reason}"));
    let from_gpu = gpu.capture().expect("an offscreen target reads back");
    save(name, GPU_CAPTURE_EXTENSION, size, &from_gpu);
    assert_eq!(pixel_of(&from_gpu, size.0, in_the_disc.0, in_the_disc.1), FIELD_INK, "{name}.gpu: the middle of the first glyph");
    assert_eq!(pixel_of(&from_gpu, size.0, in_the_outline.0, in_the_outline.1), FIELD_OUTLINE, "{name}.gpu: its outline");

    let (apart, worst) = frames_apart(&from_gpu, headless.rgba());
    println!("{name}: {apart} pixels are more than {GPU_TOLERANCE}/255 apart between the backends; the widest any channel is apart is {worst}/255");
    assert_eq!(apart, 0, "{name}: the backends drew the distance field text differently, by as much as {worst}/255");
}

/// The row a pack this was written against offers its bitmap fonts under, and what the name of the
/// item that switches them on starts with.
const BITMAP_FONT_ROW: &str = "画像フォント";
const BITMAP_FONT_ON: &str = "有効";

/// How far into the decide scene the bitmap font capture is taken: the settled layout.
const DECIDE_FONT_SHOT_MS: i64 = 1500;

/// Make the choice a player makes on the SKIN tab: in the document the pack draws `screen` with,
/// switch the row called `row` to the item whose name starts with `item`. Answers whether the
/// document offers such a row and such an item.
fn choose_in_skin(app: &mut App, screen: i32, row: &str, item: &str) -> bool {
    let chosen = app.shared.skins.header_of(&app.shared.config, screen).and_then(|header| {
        let offered = header.properties.iter().find(|offered| offered.name == row)?;
        offered.item.iter().find(|offered| offered.name.starts_with(item)).map(|item| item.op)
    });
    let document = app.shared.skins.document_path(&app.shared.config, screen).map(str::to_owned);
    let (Some(option), Some(document)) = (chosen, document) else {
        return false;
    };
    app.shared.config.skin.customise(&document).properties.insert(row.to_owned(), option);
    true
}

/// Open the pack's decide scene for the chart at `chart` with the pack's bitmap fonts switched on,
/// and answer the app and the size the scene is authored at; or `None` when the pack's decide
/// document offers no such choice.
fn decide_with_bitmap_fonts(pack: &Path, tag: &str, chart: &Path) -> Option<(App, (u32, u32))> {
    decide_with_choice(pack, tag, chart, (BITMAP_FONT_ROW, BITMAP_FONT_ON))
}

/// Open the pack's decide scene for the chart at `chart` with one row of its settings switched to
/// the item `choice` names, and answer the app and the size the scene is authored at; or `None`
/// when the pack's decide document offers no such choice.
fn decide_with_choice(pack: &Path, tag: &str, chart: &Path, choice: (&str, &str)) -> Option<(App, (u32, u32))> {
    let mut app = pack_app(pack, tag);
    let size = authored_size(&app, SKIN_TYPE_DECIDE)?;
    if !choose_in_skin(&mut app, SKIN_TYPE_DECIDE, choice.0, choice.1) {
        return None;
    }
    let bytes = std::fs::read(chart).unwrap_or_else(|error| panic!("{} could not be read: {error}", chart.display()));
    app.shared.chart_path = chart.to_string_lossy().into_owned();
    let loaded = loaded_chart_for_tests(&bytes, &app.shared.chart_path);
    app.stage = Stage::Decide(Box::new(DecideState::loaded(&app.shared, loaded)));
    Some((app, size))
}

/// Walk the decide scene that is up to [`DECIDE_FONT_SHOT_MS`] on `canvas`: its first moment until
/// the document has compiled, a frame just past its input time so the timer that starts there is
/// on, and then the moment itself until its text has every font page.
fn walk_decide_to_the_font_shot(app: &mut App, canvas: &mut Canvas<'_>) -> Result<(), String> {
    let frame_at = |at_ms: i64| {
        move |app: &mut App, canvas: &mut Canvas<'_>| {
            scene_frame_at(app, at_ms, canvas);
        }
    };
    draw_until_text_settles(app, SKIN_TYPE_DECIDE, canvas, frame_at(0))?;
    let times = SceneTimes::of_skin(app.shared.skins.document(SKIN_TYPE_DECIDE));
    if times.input_ms + 1 < DECIDE_FONT_SHOT_MS {
        frame_at(times.input_ms + 1)(app, canvas);
        app.shared.finish_skin_frame(canvas);
    }
    draw_until_text_settles(app, SKIN_TYPE_DECIDE, canvas, frame_at(DECIDE_FONT_SHOT_MS))
}

/// Captures the decide scene of a pack somebody else wrote with its bitmap fonts switched on, on
/// both backends, so the text a distance field font draws can be looked at beside the same scene in
/// the pack's TrueType fonts and the two backends beside each other.
///
/// Opt-in like the captures above it, and for a pack that offers the choice; one that does not is
/// said and passed over. How far apart the two backends came out is printed rather than held to a
/// number: the scene is a real clock's, so each backend draws its own moment of the skin's
/// animations a few milliseconds from the other's.
#[test]
fn the_decide_scene_of_a_skin_pack_with_its_bitmap_fonts_on_is_captured_on_both_backends() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let before = files_under(&pack);
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join(DECIDE_SAMPLE_CHART);
    let name = format!("decide-fnt-{DECIDE_FONT_SHOT_MS:04}ms");

    let Some((mut app, size)) = decide_with_bitmap_fonts(&pack, "decide-fnt", &sample) else {
        println!("{name}: the pack's decide document offers no {BITMAP_FONT_ROW:?} row to switch on");
        return;
    };
    let mut headless = HeadlessCanvas::new(size.0, size.1);
    walk_decide_to_the_font_shot(&mut app, &mut Canvas::Headless(&mut headless)).unwrap_or_else(|reason| panic!("{name}: not drawn: {reason}"));
    save(&name, CAPTURE_EXTENSION, size, headless.rgba());
    let pages = app.shared.skin_screens.get(SKIN_TYPE_DECIDE).map_or(0, |compiled| compiled.font_page_count());
    println!("{name}: captured at {}x{} with {pages} font pages in; warnings {:?}", size.0, size.1, app.shared.skin_warnings(SKIN_TYPE_DECIDE));
    assert!(pages > 0, "{name}: the scene drew no text from a bitmap font");

    let gpu = decide_with_bitmap_fonts(&pack, "decide-fnt-gpu", &sample).zip(Gpu::offscreen_if_available(size.0, size.1, UI_SIZE));
    if let Some(((mut app, _), mut gpu)) = gpu {
        walk_decide_to_the_font_shot(&mut app, &mut Canvas::Window(&mut gpu)).unwrap_or_else(|reason| panic!("{name}.gpu: not drawn: {reason}"));
        let from_gpu = gpu.capture().expect("an offscreen target reads back");
        save(&name, GPU_CAPTURE_EXTENSION, size, &from_gpu);
        let (apart, worst) = frames_apart(&from_gpu, headless.rgba());
        let share = apart as f64 * 100.0 / (size.0 as f64 * size.1 as f64);
        println!(
            "{name}.gpu: {apart} pixels ({share:.3}%) are more than {GPU_TOLERANCE}/255 apart from the headless frame; the widest any channel is apart is {worst}/255"
        );
    } else {
        println!("{name}.gpu: this machine has no graphics adapter");
    }
    assert_eq!(files_under(&pack), before, "capturing the pack's decide scene changed its folder");
}

/// The chart the result scene is captured for, which the engine runs to its end.
const RESULT_SAMPLE_CHART: &str = DECIDE_SAMPLE_CHART;

/// The moments of the result scene that are captured before any key is pressed, in milliseconds
/// from its beginning: its first frame, its opening, and the layout once the opening has cleared.
const RESULT_SHOTS_MS: [i64; 3] = [0, 1000, 3000];

/// How long after the last of those the left arrow is held for the skin's other menu, how long the
/// menu is given to settle before it is captured, and how long after that the scene is confirmed.
const RESULT_MENU_HOLD_MS: i64 = 50;
const RESULT_MENU_SETTLE_MS: i64 = 600;
const RESULT_CONFIRM_AFTER_MS: i64 = 100;

/// How far through its fade the result scene is captured, as a share of the skin's fade time.
const RESULT_FADE_SHARE: (i64, i64) = (3, 4);

/// The step the engine is run at while it plays the chart, and how far past the chart's own playing
/// time it is run so the last gauge sample is taken.
const RESULT_RUN_FRAME_US: i64 = 10_000;
const RESULT_RUN_MARGIN_US: i64 = 1_000_000;

fn key_down(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text: None }
}

fn key_up(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: false, released: true, text: None }
}

/// Run the chart at `chart` from its first moment to past its end -- played for the player when
/// `autoplay` is set, and with nobody playing otherwise -- and answer the session the run left.
fn played_session(app: &mut App, chart: &Path, autoplay: bool) -> Result<PlaySession, String> {
    let bytes = std::fs::read(chart).map_err(|error| format!("{} could not be read: {error}", chart.display()))?;
    let name = chart.to_string_lossy().into_owned();
    let source = rbms_parser::parse_with(&bytes, Default::default());
    let model = rbms_chart::to_model(&source, rbms_chart::detect_mode(&source, &name));
    let run_us = rbms_play::play_time_ms(&model, autoplay) * MICROS_PER_MILLI + RESULT_RUN_MARGIN_US;
    app.shared.chart_path = name;
    app.shared.mode = model.mode;
    app.shared.config.play.autoplay = autoplay;
    let mut session = PlaySession::new(model, SessionOptions { autoplay, ..SessionOptions::default() });
    for frame in 0..=run_us / RESULT_RUN_FRAME_US {
        session.tick(SessionClock::at(frame * RESULT_RUN_FRAME_US), &mut NullSink);
    }
    Ok(session)
}

/// Run the chart at `chart` to its end and answer what the application makes of the run when it
/// ends: the screen it goes to, with the run's record kept and its score settled.
fn finished_run(app: &mut App, chart: &Path, autoplay: bool) -> Result<Transition, String> {
    let session = played_session(app, chart, autoplay)?;
    let mut played = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string());
    Ok(enter_result(&mut played, &mut app.shared))
}

/// Run the sample chart, open the pack's result scene on what the run left, and save a frame at
/// each of [`RESULT_SHOTS_MS`], one of the skin's other menu and one of its fade, as
/// `<prefix>-<moment>`. Answers the names saved, or why the scene never drew.
///
/// The scene is walked the way the decide scene is: a frame is run just past the skin's input time
/// so `STARTINPUT` goes on when the skin says, the other menu is asked for with the key the skin
/// polls, and the fade is begun with the key that confirms.
fn capture_result_scene(pack: &Path, tag: &str, chart: &Path, autoplay: bool, prefix: &str) -> Result<Vec<String>, String> {
    let mut app = pack_app(pack, tag);
    let size = authored_size(&app, SKIN_TYPE_RESULT).ok_or_else(|| "the pack has no document for this screen".to_owned())?;
    let entered = finished_run(&mut app, chart, autoplay)?;
    if !matches!(entered, Transition::To(Stage::Result(_))) {
        return Err("the run did not reach the result screen".to_owned());
    }
    app.switch(entered);
    walk_result_scene(&mut app, SKIN_TYPE_RESULT, size, prefix)
}

/// Walk the score scene of `screen` -- the result or the course result -- that is up in `app`, which
/// is the same scene on both, and save its frames as `<prefix>-<moment>`.
fn walk_result_scene(app: &mut App, screen: i32, size: (u32, u32), prefix: &str) -> Result<Vec<String>, String> {
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !app.shared.has_compiled_skin(screen) {
        scene_frame_at(app, 0, &mut Canvas::Headless(&mut pixels));
        if let Some(reason) = app.shared.skin_failure(screen) {
            return Err(reason.lines().next().unwrap_or_default().to_owned());
        }
        if Instant::now() >= deadline {
            return Err(format!("its files were not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs()));
        }
    }

    let times = SceneTimes::of_skin(app.shared.skins.document(screen));
    let mut saved = Vec::new();
    let mut shoot = |app: &mut App, at_ms: i64, moment: &str| -> Result<(), String> {
        if !matches!(scene_frame_at(app, at_ms, &mut Canvas::Headless(&mut pixels)), Transition::Stay) {
            return Err(format!("the scene was over by {at_ms} ms"));
        }
        if !moment.is_empty() {
            let name = format!("{prefix}-{moment}");
            save(&name, CAPTURE_EXTENSION, size, pixels.rgba());
            saved.push(name);
        }
        Ok(())
    };

    let last = RESULT_SHOTS_MS.iter().copied().max().unwrap_or_default().max(times.input_ms + 1);
    let mut moments: Vec<i64> = RESULT_SHOTS_MS.into_iter().chain([times.input_ms + 1]).collect();
    moments.sort_unstable();
    moments.dedup();
    for at_ms in moments {
        let moment = if RESULT_SHOTS_MS.contains(&at_ms) { format!("{at_ms:04}ms") } else { String::new() };
        shoot(app, at_ms, &moment)?;
    }

    app.shared.note_key(&key_down(KeyCode::ArrowLeft));
    shoot(app, last + RESULT_MENU_HOLD_MS, "")?;
    app.shared.note_key(&key_up(KeyCode::ArrowLeft));
    let menu_ms = last + RESULT_MENU_HOLD_MS + RESULT_MENU_SETTLE_MS;
    shoot(app, menu_ms, "menu2")?;

    let confirm_ms = menu_ms + RESULT_CONFIRM_AFTER_MS;
    let now = Instant::now();
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key_down(KeyCode::Enter));
    shoot(app, confirm_ms, "")?;
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key_up(KeyCode::Enter));
    if !app.shared.skin_timers.is_on(timer_id::FADEOUT) {
        return Err("the confirming key did not start the fade".to_owned());
    }
    shoot(app, confirm_ms + times.fadeout_ms * RESULT_FADE_SHARE.0 / RESULT_FADE_SHARE.1, "fade")?;
    Ok(saved)
}

/// Captures the result scene of a pack somebody else wrote for two runs of a chart that ships with
/// the repository: one the engine played to a clear, and one nobody played, which fails.
///
/// Opt-in like the captures above it: without [`SKIN_PACK_ENV`] this passes without drawing
/// anything. With it, the scene has to draw for both runs, and the pack's folder has to be left as
/// it was.
#[test]
fn the_result_scene_of_a_skin_pack_named_by_the_environment_is_captured_for_a_clear_and_a_failure() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let before = files_under(&pack);
    let chart = Path::new(env!("CARGO_MANIFEST_DIR")).join(RESULT_SAMPLE_CHART);

    for (autoplay, prefix) in [(true, "result-clear"), (false, "result-fail")] {
        let saved = capture_result_scene(&pack, prefix, &chart, autoplay, prefix).unwrap_or_else(|reason| panic!("{prefix}: not drawn: {reason}"));
        println!("{prefix}: captured {}", saved.join(", "));
    }
    assert_eq!(files_under(&pack), before, "capturing the pack's result scene changed its folder");
}

/// How many stages the courses captured below have, and the one the failed course gives up on (the
/// second, counting from zero), which leaves the third for the library to stand in for.
const COURSE_STAGES: usize = 3;
const COURSE_FAILED_STAGE: usize = 1;

/// A library entry for the chart at `path`, so a course stage that names its MD5 resolves to it.
fn library_entry(path: &Path, md5: &str, mode: rbms_model::Mode) -> SongEntry {
    SongEntry {
        path: path.to_path_buf(),
        title: String::new(),
        subtitle: String::new(),
        artist: String::new(),
        genre: String::new(),
        maker: String::new(),
        level: String::new(),
        difficulty: 0,
        init_bpm: 0.0,
        rank: 2,
        total: 0.0,
        mode,
        md5: md5.to_owned(),
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }
}

/// Play a course of `stages` stages of the chart at `chart`, each played for the player, until
/// `failing` stage, which nobody plays and which ends the course. Answers the course as the application leaves it
/// when it ends: the run alive, one record kept for each stage that was played, and the chart in
/// the library for the stages that were not.
fn finished_course(app: &mut App, chart: &Path, stages: usize, failing: Option<usize>) -> Result<CourseRun, String> {
    let mut sessions = Vec::new();
    for stage in 0..stages {
        sessions.push(played_session(app, chart, failing != Some(stage))?);
        if failing == Some(stage) {
            break;
        }
    }
    let model = sessions.first().ok_or("a course has no stages")?.model();
    let (md5, mode) = (model.md5.clone(), model.mode);
    let mut course = Course {
        name: "CAPTURE COURSE".to_owned(),
        charts: (1..=stages)
            .map(|number| CourseChart { md5: md5.clone(), sha256: model.sha256.clone(), title: format!("{} {number}", model.meta.title) })
            .collect(),
        ..Course::default()
    };
    if !course.validate() {
        return Err("the course is not valid".to_owned());
    }
    let mut run = CourseRun::new(course, 100.0);
    app.shared.run_records.clear();
    for (stage, session) in sessions.iter().enumerate() {
        let summary = session.summary();
        run.advance(&StageResult {
            ex_score: summary.ex_score,
            max_ex_score: summary.max_ex_score,
            notes: summary.total_notes,
            counts: summary.counts,
            empty_poor: summary.empty_poor,
            fast: summary.fast,
            slow: summary.slow,
            combo_breaks: crate::combo_breaks(&mode, summary.counts),
            max_combo: summary.max_combo,
            combo_at_end: session.standing_combo(),
            gauge_value: summary.gauge_value,
            clear: rbms_judge::clear_type_id(summary.clear_lamp),
            survived: failing != Some(stage) && !summary.failed,
        });
        app.shared.run_records.push(session.record());
    }
    app.shared.library = Library::from_songs(vec![library_entry(chart, &md5, mode)]);
    app.shared.course_run = Some(run.clone());
    Ok(run)
}

/// Run a course of the sample chart, open the pack's course result scene on what it left, and walk
/// it as the result scene is walked, saving its frames as `<prefix>-<moment>`.
fn capture_course_result_scene(pack: &Path, tag: &str, chart: &Path, failing: Option<usize>, prefix: &str) -> Result<Vec<String>, String> {
    let mut app = pack_app(pack, tag);
    let size = authored_size(&app, SKIN_TYPE_COURSE_RESULT).ok_or_else(|| "the pack has no document for this screen".to_owned())?;
    let run = finished_course(&mut app, chart, COURSE_STAGES, failing)?;
    let state = CourseResultState::of(&run, &app.shared);
    app.switch(Transition::To(Stage::CourseResult(Box::new(state))));
    walk_result_scene(&mut app, SKIN_TYPE_COURSE_RESULT, size, prefix)
}

/// Captures the course result scene of a pack somebody else wrote for two courses of a chart that
/// ships with the repository: one played through to a clear, and one that fails on its second stage
/// and never reaches its third.
///
/// Opt-in like the captures above it: without [`SKIN_PACK_ENV`] this passes without drawing
/// anything. With it, the scene has to draw for both courses, and the pack's folder has to be left
/// as it was.
#[test]
fn the_course_result_scene_of_a_skin_pack_named_by_the_environment_is_captured_for_a_clear_and_a_failure() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let before = files_under(&pack);
    let chart = Path::new(env!("CARGO_MANIFEST_DIR")).join(RESULT_SAMPLE_CHART);

    for (failing, prefix) in [(None, "course-result-clear"), (Some(COURSE_FAILED_STAGE), "course-result-fail")] {
        let saved = capture_course_result_scene(&pack, prefix, &chart, failing, prefix).unwrap_or_else(|reason| panic!("{prefix}: not drawn: {reason}"));
        println!("{prefix}: captured {}", saved.join(", "));
    }
    assert_eq!(files_under(&pack), before, "capturing the pack's course result scene changed its folder");
}

/// The moment of the browser's scene its cursor is moved at, in milliseconds from its beginning,
/// and the moments that are captured after it: the bars on their way in with what the move brought
/// faded in, and the settled layout.
const BROWSER_MOVE_MS: i64 = 1000;
const BROWSER_SHOTS_MS: [i64; 2] = [1300, 3000];

/// The keys START and SELECT ship on, which call up the browser's option panels.
const BROWSER_START_KEY: KeyCode = KeyCode::KeyA;
const BROWSER_SELECT_KEY: KeyCode = KeyCode::KeyW;

/// The option panels the browser is captured with, each by the name it is saved under and the keys
/// that are held for it: none, the play options, the assist options and the detail options.
const BROWSER_PANELS: [(&str, &[KeyCode]); 4] = [
    ("select-panel0", &[]),
    ("select-panel1", &[BROWSER_START_KEY]),
    ("select-panel2", &[BROWSER_SELECT_KEY]),
    ("select-panel3", &[BROWSER_START_KEY, BROWSER_SELECT_KEY]),
];

/// The moment of the browser's scene the first panel's keys go down at, how long each panel is
/// given to arrive before it is captured, which is also how long it is given to go before the next
/// one's keys go down, and the name the browser is saved under once the last panel has gone.
const BROWSER_PANELS_FROM_MS: i64 = 4000;
const BROWSER_PANEL_SETTLE_MS: i64 = 1000;
const BROWSER_PANELS_GONE: &str = "select-panels-gone";

/// The keys the default seven-key layout puts on key one and key three, which with the first panel
/// up step the random option and the gauge, and the name the panel is saved under once they have.
const BROWSER_STEP_KEYS: [KeyCode; 2] = [KeyCode::KeyZ, KeyCode::KeyX];
const BROWSER_PANEL_STEPPED: &str = "select-panel1-stepped";

/// The folder of sample charts that ships with the repository, named from this crate's folder.
const BROWSER_SAMPLE_FOLDER: &str = "../../samples/preview-demo";

/// The pictures written beside the charts below, which one of them names, and their sizes.
const BROWSER_STAGE_FILE: &str = "browse-stage.png";
const BROWSER_BANNER_FILE: &str = "browse-banner.png";
const BROWSER_STAGE_SIZE: (u32, u32) = (640, 480);
const BROWSER_BANNER_SIZE: (u32, u32) = (300, 80);

/// The title of the chart the browser's cursor is put on, which is the one that names pictures.
const BROWSER_FOCUS_TITLE: &str = "Borealis";

/// The most a record of the charts below could have scored, and when each was played.
const BROWSER_RECORD_BREAKS: u32 = 3;
const BROWSER_RECORD_PLAYED_AT: i64 = 1_000;

/// What the best judgement is worth in EX, which is what cuts a record's score into the judgements
/// it is stored as: as many of the best as go into it, and one of the next for what is left.
const EX_PER_PGREAT: u32 = 2;

/// The title of a chart nobody has played, which the browser is captured on for a score window with
/// nothing in it.
const BROWSER_UNPLAYED_TITLE: &str = "Drift";

/// The name the browser is saved under in the order a new player's list is in, which the reference's
/// order switch has no number for.
const BROWSER_DEFAULT_ORDER: &str = "select-order-default";

/// The notes of a chart with nothing special about it.
const BROWSER_PLAIN_NOTES: &str = "#00111:01010101\n#00218:0101\n";

/// The notes of a chart with a long note whose kind the player's mode decides.
const BROWSER_LONG_NOTES: &str = "#LNTYPE 1\n#00151:01000001\n#00118:0101\n";

/// The notes of a chart with a mine.
const BROWSER_MINE_NOTES: &str = "#002D1:0100\n#00118:0101\n";

/// The notes of a chart that picks some of them at random.
const BROWSER_RANDOM_NOTES: &str = "#RANDOM 2\n#IF 1\n#00111:0101\n#ENDIF\n#IF 2\n#00112:0101\n#ENDIF\n#00118:01\n";

/// One chart written for the browser to list.
struct BrowseChart {
    file: &'static str,
    title: &'static str,
    subtitle: &'static str,
    level: u32,
    /// `#DIFFICULTY`, or zero for a chart that names none.
    difficulty: u32,
    notes: &'static str,
    /// The best clear recorded on it, as the reference numbers them, and the EX of that run; `None`
    /// for a chart nobody has played.
    record: Option<(u8, u32)>,
}

/// The charts written for the browser: between them every difficulty, every clear lamp and every
/// chart label.
const BROWSE_CHARTS: [BrowseChart; 11] = [
    BrowseChart { file: "a1.bms", title: "Aurora", subtitle: "[BEGINNER]", level: 2, difficulty: 1, notes: BROWSER_PLAIN_NOTES, record: Some((8, 990)) },
    BrowseChart { file: "a2.bms", title: "Aurora", subtitle: "[HYPER]", level: 9, difficulty: 3, notes: BROWSER_LONG_NOTES, record: Some((6, 850)) },
    BrowseChart { file: "a3.bms", title: "Aurora", subtitle: "[ANOTHER]", level: 12, difficulty: 4, notes: BROWSER_MINE_NOTES, record: Some((7, 780)) },
    BrowseChart { file: "b.bms", title: BROWSER_FOCUS_TITLE, subtitle: "", level: 7, difficulty: 2, notes: BROWSER_RANDOM_NOTES, record: Some((4, 640)) },
    BrowseChart { file: "c.bms", title: "Cascade", subtitle: "[INSANE]", level: 25, difficulty: 5, notes: BROWSER_PLAIN_NOTES, record: Some((1, 210)) },
    BrowseChart { file: "d.bms", title: "Drift", subtitle: "", level: 5, difficulty: 0, notes: BROWSER_PLAIN_NOTES, record: None },
    BrowseChart { file: "e.bms", title: "Ember", subtitle: "", level: 10, difficulty: 4, notes: BROWSER_PLAIN_NOTES, record: Some((5, 700)) },
    BrowseChart { file: "f.bms", title: "Flux", subtitle: "", level: 3, difficulty: 2, notes: BROWSER_PLAIN_NOTES, record: Some((2, 420)) },
    BrowseChart { file: "g.bms", title: "Glacier", subtitle: "", level: 11, difficulty: 3, notes: BROWSER_PLAIN_NOTES, record: Some((9, 998)) },
    BrowseChart { file: "h.bms", title: "Halo", subtitle: "", level: 1, difficulty: 1, notes: BROWSER_PLAIN_NOTES, record: Some((10, 1000)) },
    BrowseChart { file: "i.bms", title: "Iris", subtitle: "", level: 6, difficulty: 2, notes: BROWSER_PLAIN_NOTES, record: Some((3, 500)) },
];

/// Write the charts above and the two pictures one of them names into a folder of this test's own,
/// and answer the folder.
fn write_browse_charts(tag: &str) -> PathBuf {
    let folder = settings_of(tag).with_file_name("charts");
    std::fs::create_dir_all(&folder).expect("the chart folder is writable");
    for (file, (width, height)) in [(BROWSER_STAGE_FILE, BROWSER_STAGE_SIZE), (BROWSER_BANNER_FILE, BROWSER_BANNER_SIZE)] {
        let picture = image::RgbaImage::from_fn(width, height, |x, y| {
            let band = u8::try_from((x * u32::from(u8::MAX)) / width).unwrap_or(u8::MAX);
            let rise = u8::try_from((y * u32::from(u8::MAX)) / height).unwrap_or(u8::MAX);
            image::Rgba([u8::MAX - band, rise, band, u8::MAX])
        });
        picture.save(folder.join(file)).expect("the picture is written");
    }
    for chart in &BROWSE_CHARTS {
        let pictures =
            if chart.title == BROWSER_FOCUS_TITLE { format!("#STAGEFILE {BROWSER_STAGE_FILE}\n#BANNER {BROWSER_BANNER_FILE}\n") } else { String::new() };
        let text = format!(
            "#PLAYER 1\n#GENRE Capture\n#TITLE {}\n#SUBTITLE {}\n#ARTIST Capture Artist\n#BPM 150\n#PLAYLEVEL {}\n#DIFFICULTY {}\n#RANK 2\n#TOTAL 300\n{pictures}{}",
            chart.title, chart.subtitle, chart.level, chart.difficulty, chart.notes
        );
        std::fs::write(folder.join(chart.file), text).expect("the chart is written");
    }
    folder
}

/// An app drawing with `pack` whose library was scanned from the sample folder and the charts
/// written above, into a song database of the test's own, with the records those charts name.
///
/// The records are in the score book the browser lists by and in a score database in memory, which
/// is what a skin's score window is read from: the judgements of each add up to the score it names,
/// so the two agree about it.
///
/// The browser's preview is switched off and the sound device is marked as one that would not
/// open, so the frames that are walked never ask the machine for one.
fn browse_app(pack: &Path, tag: &str) -> Result<App, String> {
    let mut app = pack_app(pack, tag);
    let charts = write_browse_charts(&format!("{tag}-charts"));
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join(BROWSER_SAMPLE_FOLDER);
    let database = charts.with_file_name("songs.db");
    let roots = [charts.as_path(), samples.as_path()].map(|root| root.to_string_lossy().into_owned()).to_vec();
    let report = crate::library::scan_now(&database, &ScanRequest::new(roots, true));
    if let Some(failure) = report.failure {
        return Err(format!("the library was not scanned: {failure}"));
    }
    app.shared.library = report.library;
    app.shared.song_db = crate::library::open_song_db(&database);
    let records = BROWSE_CHARTS.iter().filter_map(|chart| {
        let (clear, ex) = chart.record?;
        let entry = app.shared.library.songs().iter().find(|entry| entry.title == chart.title && entry.subtitle == chart.subtitle)?;
        Some(rbms_store::ScoreRecord {
            mode: entry.mode.name.to_owned(),
            counts: [ex / EX_PER_PGREAT, ex % EX_PER_PGREAT, 0, BROWSER_RECORD_BREAKS, 0, 0],
            ..record(&entry.md5, clear, ex, BROWSER_RECORD_BREAKS, BROWSER_RECORD_PLAYED_AT)
        })
    });
    app.shared.scores = rbms_store::ScoreBook::from_records(records.collect());
    let mut scores = ScoreDb::open_in_memory().map_err(|error| format!("the score database did not open: {error}"))?;
    scores.migrate().map_err(|error| format!("the score database was not laid out: {error}"))?;
    migrate_score_book(&mut scores, &app.shared.scores).map_err(|error| format!("the records were not stored: {error}"))?;
    app.shared.scoredb = Some(scores);
    app.shared.config.library.sort = SortMode::Title;
    app.shared.config.library.preview = false;
    app.shared.audio_failed = true;
    Ok(app)
}

/// Whether the browser that is up has come to rest: every picture it asked for is read, and its
/// wheel is not sliding.
fn browser_is_at_rest(app: &App) -> bool {
    match &app.stage {
        Stage::Select(state) => state.pictures_are_in() && state.wheel_is_at_rest(),
        _ => true,
    }
}

/// The row of the list on show the chart called `title` is on.
fn browser_row_of(app: &App, title: &str) -> Option<usize> {
    let songs = app.shared.library.songs();
    app.shared
        .select_items
        .iter()
        .position(|item| matches!(item, crate::SelectItem::Song(index) if songs.get(*index).is_some_and(|entry| entry.title == title)))
}

/// One frame of the browser at `scene_ms`, or why its skin will never draw.
fn browser_frame_at(app: &mut App, scene_ms: i64, pixels: &mut HeadlessCanvas, deadline: Instant) -> Result<(), String> {
    scene_frame_at(app, scene_ms, &mut Canvas::Headless(pixels));
    if let Some(reason) = app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT) {
        return Err(reason.lines().next().unwrap_or_default().to_owned());
    }
    if Instant::now() >= deadline {
        return Err(format!("its files were not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs()));
    }
    Ok(())
}

/// Put `state` up as the browser with its cursor one row above where it is wanted, wait for the
/// pack's document, move the cursor down onto the row with the key a player would press, wait for
/// the pictures of the chart it landed on and for the wheel to finish the slide the key started,
/// and save a frame at each of [`BROWSER_SHOTS_MS`] as `<prefix>-<milliseconds>ms`. Every frame is
/// the browser's update and then its draw, as the application runs them.
///
/// The cursor is moved rather than placed because a skin shows what belongs to the bar under the
/// cursor on the timer a move starts, so a browser nobody has moved shows none of it. The key is let
/// go again once the browser has seen it: a key left down would be held on every panel captured
/// afterwards, and the down arrow scrolls the first panel's target. The wheel slides on the wall
/// clock rather than on the scene's, so the slide is waited out rather than aged past.
fn capture_browser_view(app: &mut App, pixels: &mut HeadlessCanvas, size: (u32, u32), prefix: &str, state: SelectState) -> Result<Vec<String>, String> {
    app.stage = Stage::Select(Box::new(state));
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        browser_frame_at(app, 0, pixels, deadline)?;
    }
    browser_frame_at(app, BROWSER_MOVE_MS, pixels, deadline)?;
    let now = Instant::now();
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, key_down(KeyCode::ArrowDown));
    browser_frame_at(app, BROWSER_MOVE_MS, pixels, deadline)?;
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, key_up(KeyCode::ArrowDown));
    while !browser_is_at_rest(app) {
        browser_frame_at(app, BROWSER_MOVE_MS, pixels, deadline)?;
    }
    let mut saved = Vec::new();
    for at_ms in BROWSER_SHOTS_MS {
        browser_frame_at(app, at_ms, pixels, deadline)?;
        let name = format!("{prefix}-{at_ms:04}ms");
        save(&name, CAPTURE_EXTENSION, size, pixels.rgba());
        saved.push(name);
    }
    Ok(saved)
}

/// Capture the browser that is up with each of its option panels in turn, the first of them once
/// more after two of its keys were pressed, and the browser after the last has gone. Answers the
/// names saved, or why the browser stopped drawing.
///
/// A panel is called up the way a player calls it up: its keys go down, the browser runs a frame
/// on which it puts the panel up and starts the skin's timer for it, and the frame that is saved is
/// drawn once the skin has had time to slide the panel in. The keys are let go on a frame of their
/// own, so the panel has the same time to slide out before the next one's keys go down.
fn capture_browser_panels(app: &mut App, pixels: &mut HeadlessCanvas, size: (u32, u32)) -> Result<Vec<String>, String> {
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    let mut saved = Vec::new();
    let mut at_ms = BROWSER_PANELS_FROM_MS;
    for (name, keys) in BROWSER_PANELS {
        for key in keys {
            app.shared.note_key(&key_down(*key));
        }
        browser_frame_at(app, at_ms, pixels, deadline)?;
        at_ms += BROWSER_PANEL_SETTLE_MS;
        browser_frame_at(app, at_ms, pixels, deadline)?;
        save(name, CAPTURE_EXTENSION, size, pixels.rgba());
        saved.push(name.to_owned());
        for key in keys {
            app.shared.note_key(&key_up(*key));
        }
        browser_frame_at(app, at_ms, pixels, deadline)?;
        at_ms += BROWSER_PANEL_SETTLE_MS;
    }

    app.shared.note_key(&key_down(BROWSER_START_KEY));
    browser_frame_at(app, at_ms, pixels, deadline)?;
    for input in [key_down, key_up] {
        for key in BROWSER_STEP_KEYS {
            app.shared.note_key(&input(key));
        }
        browser_frame_at(app, at_ms, pixels, deadline)?;
    }
    at_ms += BROWSER_PANEL_SETTLE_MS;
    browser_frame_at(app, at_ms, pixels, deadline)?;
    save(BROWSER_PANEL_STEPPED, CAPTURE_EXTENSION, size, pixels.rgba());
    saved.push(BROWSER_PANEL_STEPPED.to_owned());
    app.shared.note_key(&key_up(BROWSER_START_KEY));
    browser_frame_at(app, at_ms, pixels, deadline)?;
    at_ms += BROWSER_PANEL_SETTLE_MS;

    browser_frame_at(app, at_ms, pixels, deadline)?;
    save(BROWSER_PANELS_GONE, CAPTURE_EXTENSION, size, pixels.rgba());
    saved.push(BROWSER_PANELS_GONE.to_owned());
    Ok(saved)
}

/// A course of two charts the scanned library holds, and one that names a chart it does not.
fn browse_courses(library: &Library) -> Vec<Course> {
    let held: Vec<CourseChart> =
        library.songs().iter().take(2).map(|entry| CourseChart { md5: entry.md5.clone(), sha256: String::new(), title: entry.title.clone() }).collect();
    let absent = CourseChart { md5: "0".repeat(32), sha256: String::new(), title: "absent".to_owned() };
    let mut courses = vec![
        Course { name: "CAPTURE COURSE".to_owned(), charts: held.clone(), ..Course::default() },
        Course { name: "MISSING COURSE".to_owned(), charts: held.into_iter().chain([absent]).collect(), ..Course::default() },
    ];
    courses.retain_mut(Course::validate);
    courses
}

/// Capture the pack's browser on four lists of one scanned library: its charts, its root, the
/// levels of a table, and its courses; and over its charts, with each of its option panels up, on a
/// chart with a score and on one nobody has played, and in the order a new player's list is in.
/// Answers the names saved, or why the browser never drew.
fn capture_browser(pack: &Path) -> Result<Vec<String>, String> {
    let mut app = browse_app(pack, "browser")?;
    let size = authored_size(&app, SKIN_TYPE_MUSIC_SELECT).ok_or_else(|| "the pack has no document for this screen".to_owned())?;
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let mut saved = Vec::new();

    app.shared.select_view = SelectView::AllSongs;
    app.shared.rebuild_select_items();
    let focus =
        app.shared.library.songs().iter().position(|entry| entry.title == BROWSER_FOCUS_TITLE).ok_or("the scan did not find the chart that names pictures")?;
    app.shared.sel = focus.checked_sub(1).ok_or("the chart that names pictures is the first of the list")?;
    saved.extend(capture_browser_view(&mut app, &mut pixels, size, "select-songs", SelectState::new())?);
    saved.extend(capture_browser_panels(&mut app, &mut pixels, size)?);

    let unplayed = browser_row_of(&app, BROWSER_UNPLAYED_TITLE).ok_or("the scan did not find the chart nobody has played")?;
    app.shared.sel = unplayed.checked_sub(1).ok_or("the chart nobody has played is the first of the list")?;
    saved.extend(capture_browser_view(&mut app, &mut pixels, size, "select-unplayed", SelectState::new())?);

    app.shared.config.library.sort = SortMode::Default;
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    browser_frame_at(&mut app, BROWSER_MOVE_MS, &mut pixels, deadline)?;
    while !browser_is_at_rest(&app) {
        browser_frame_at(&mut app, BROWSER_MOVE_MS, &mut pixels, deadline)?;
    }
    browser_frame_at(&mut app, OVERLAY_AT_MS, &mut pixels, deadline)?;
    save(BROWSER_DEFAULT_ORDER, CAPTURE_EXTENSION, size, pixels.rgba());
    saved.push(BROWSER_DEFAULT_ORDER.to_owned());
    app.shared.config.library.sort = SortMode::Title;

    let charts = app.shared.library.len();
    app.shared.table_names = vec!["CAPTURE TABLE".to_owned()];
    app.shared.table_levels = vec![vec![("1".to_owned(), (0..charts / 2).collect()), ("2".to_owned(), (charts / 2..charts).collect())]];
    app.shared.select_view = SelectView::Root;
    app.shared.rebuild_select_items();
    app.shared.sel = app.shared.select_items.len().saturating_sub(1);
    saved.extend(capture_browser_view(&mut app, &mut pixels, size, "select-root", SelectState::new())?);

    app.shared.select_view = SelectView::TableLevels(0);
    app.shared.rebuild_select_items();
    app.shared.sel = app.shared.select_items.len().saturating_sub(1);
    saved.extend(capture_browser_view(&mut app, &mut pixels, size, "select-table", SelectState::new())?);

    let courses = SelectState::on_courses(browse_courses(&app.shared.library), &app.shared.library);
    saved.extend(capture_browser_view(&mut app, &mut pixels, size, "select-courses", courses)?);
    Ok(saved)
}

/// Captures the browser of a pack somebody else wrote over a library that was really scanned, so the
/// bars of its wheel carry the titles, levels, lamps and labels of real charts.
///
/// Opt-in like the captures above it: without [`SKIN_PACK_ENV`] this passes without drawing
/// anything. With it, the browser has to draw every list, and neither the pack's folder nor the
/// sample folder may be changed by it.
#[test]
fn the_browser_of_a_skin_pack_named_by_the_environment_is_captured_over_a_scanned_library() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join(BROWSER_SAMPLE_FOLDER);
    let before = (files_under(&pack), files_under(&samples));

    let saved = capture_browser(&pack).unwrap_or_else(|reason| panic!("select: not drawn: {reason}"));
    println!("select: captured {}", saved.join(", "));

    assert_eq!((files_under(&pack), files_under(&samples)), before, "capturing the pack's browser changed a folder it only reads");
}

/// The moment of the browser's scene every overlay is captured at, and where each is saved under.
const OVERLAY_AT_MS: i64 = 3000;
const OVERLAY_GUIDE: &str = "overlay-guide";
const OVERLAY_FILTER: &str = "overlay-filter";
const OVERLAY_RANKING: &str = "overlay-ranking";
const OVERLAY_MODAL: &str = "overlay-modal";
const OVERLAY_SEARCH: &str = "overlay-search";
const OVERLAY_OPTIONS: &str = "overlay-options";
const OVERLAY_SYSTEM: &str = "overlay-system";
const OVERLAY_COURSES: &str = "overlay-courses";
const OVERLAY_FIRST_RUN: &str = "overlay-first-run";
const OVERLAY_NO_RESULTS: &str = "overlay-no-results";

/// What is typed into the search box of the overlay captures, a letter at a time, with the key each
/// letter is on. None of them is a key START or SELECT is on: those are held by the key set the
/// harness keeps and would call an option panel up over the shot.
const OVERLAY_QUERY: [(KeyCode, &str); 3] = [(KeyCode::KeyB, "b"), (KeyCode::KeyO, "o"), (KeyCode::KeyR, "r")];

/// What is typed into the search box of the capture that finds nothing.
const OVERLAY_NO_MATCH: [(KeyCode, &str); 3] = [(KeyCode::KeyZ, "z"), (KeyCode::KeyX, "x"), (KeyCode::KeyC, "c")];

/// What the application's own overlays are drawn with: the frame's end, which hands the scene's
/// bookkeeping on, and then the panels drawn over every screen -- the option overlay, the messages,
/// the connection dot and the debug panel.
pub(super) fn draw_app_overlays(app: &mut App, canvas: &mut Canvas<'_>) {
    let now = Instant::now();
    let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT };
    ctx.shared.finish_skin_frame(canvas);
    App::draw_overlays(&app.stage, &mut ctx, canvas);
}

/// Hand one key to the screen that is up, the way the window does.
fn press_key(app: &mut App, key: KeyInput<'_>) {
    let now = Instant::now();
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, key);
}

/// One frame of the browser with the application's overlays on top, saved as `name`.
fn overlay_shot(app: &mut App, pixels: &mut HeadlessCanvas, size: (u32, u32), name: &str, deadline: Instant) -> Result<(), String> {
    browser_frame_at(app, OVERLAY_AT_MS, pixels, deadline)?;
    draw_app_overlays(app, &mut Canvas::Headless(pixels));
    save(name, CAPTURE_EXTENSION, size, pixels.rgba());
    Ok(())
}

/// Put `open` to the browser, capture the frame it makes, and put `close` to it, so the next overlay
/// starts from the browser as it was. Every key goes up again once it has gone down, because the
/// harness keeps the keys that are held and a key left down would still be down on the next shot.
fn overlay_with(
    app: &mut App,
    pixels: &mut HeadlessCanvas,
    size: (u32, u32),
    name: &str,
    open: &[KeyInput<'_>],
    close: KeyCode,
    deadline: Instant,
) -> Result<(), String> {
    for key in open {
        press_key(app, KeyInput { code: key.code, pressed: key.pressed, released: key.released, text: key.text });
        press_key(app, key_up(key.code));
        browser_frame_at(app, OVERLAY_AT_MS, pixels, deadline)?;
    }
    overlay_shot(app, pixels, size, name, deadline)?;
    press_key(app, key_down(close));
    press_key(app, key_up(close));
    browser_frame_at(app, OVERLAY_AT_MS, pixels, deadline)
}

/// A key typing `text`, to be put to the browser's search box.
fn typed_key(code: KeyCode, text: &'static str) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text: Some(text) }
}

/// Wait for the pack's browser to compile on the stage that is up.
fn wait_for_browser(app: &mut App, pixels: &mut HeadlessCanvas) -> Result<(), String> {
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        browser_frame_at(app, 0, pixels, deadline)?;
    }
    browser_frame_at(app, OVERLAY_AT_MS, pixels, deadline)
}

/// Capture the pack's browser with each of the application's overlays up over it: the key guide,
/// the filter, the ranking, a record, the search box, the option overlay, and the messages, the
/// connection dot and the debug panel; and the course tab; and a library with nothing in it, first
/// as a new player has it and then after a search that found nothing.
fn capture_browser_overlays(pack: &Path) -> Result<Vec<String>, String> {
    let mut app = browse_app(pack, "overlays")?;
    let size = authored_size(&app, SKIN_TYPE_MUSIC_SELECT).ok_or_else(|| "the pack has no document for this screen".to_owned())?;
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    app.shared.select_view = SelectView::AllSongs;
    app.shared.rebuild_select_items();
    let focus =
        app.shared.library.songs().iter().position(|entry| entry.title == BROWSER_FOCUS_TITLE).ok_or("the scan did not find the chart that names pictures")?;
    app.shared.sel = focus.checked_sub(1).ok_or("the chart that names pictures is the first of the list")?;
    capture_browser_view(&mut app, &mut pixels, size, "overlay-base", SelectState::new())?;
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    let mut saved = vec!["overlay-base-1300ms".to_owned(), "overlay-base-3000ms".to_owned()];

    overlay_with(&mut app, &mut pixels, size, OVERLAY_GUIDE, &[key_down(KeyCode::KeyH)], KeyCode::KeyH, deadline)?;
    overlay_with(&mut app, &mut pixels, size, OVERLAY_FILTER, &[key_down(KeyCode::F2)], KeyCode::F2, deadline)?;
    overlay_with(&mut app, &mut pixels, size, OVERLAY_RANKING, &[key_down(KeyCode::KeyI)], KeyCode::KeyI, deadline)?;
    overlay_with(&mut app, &mut pixels, size, OVERLAY_MODAL, &[key_down(KeyCode::KeyR)], KeyCode::Escape, deadline)?;
    let mut search = vec![key_down(KeyCode::Slash)];
    search.extend(OVERLAY_QUERY.map(|(code, text)| typed_key(code, text)));
    overlay_with(&mut app, &mut pixels, size, OVERLAY_SEARCH, &search, KeyCode::Escape, deadline)?;
    overlay_with(&mut app, &mut pixels, size, OVERLAY_OPTIONS, &[key_down(KeyCode::F1)], KeyCode::Escape, deadline)?;
    saved.extend([OVERLAY_GUIDE, OVERLAY_FILTER, OVERLAY_RANKING, OVERLAY_MODAL, OVERLAY_SEARCH, OVERLAY_OPTIONS].map(str::to_owned));

    app.shared.config.display.debug = true;
    app.shared.config.network.server_url = Some("http://capture.invalid".to_owned());
    let now = Instant::now();
    for (level, text) in [
        (rbms_render::ToastLevel::Info, "scanned 1204 charts"),
        (rbms_render::ToastLevel::Warn, "table load failed"),
        (rbms_render::ToastLevel::Error, "replay load failed: no such file"),
    ] {
        app.shared.toasts.push(level, text, now);
    }
    overlay_shot(&mut app, &mut pixels, size, OVERLAY_SYSTEM, deadline)?;
    saved.push(OVERLAY_SYSTEM.to_owned());
    app.shared.config.display.debug = false;

    let courses = SelectState::on_courses(browse_courses(&app.shared.library), &app.shared.library);
    app.stage = Stage::Select(Box::new(courses));
    browser_frame_at(&mut app, OVERLAY_AT_MS, &mut pixels, deadline)?;
    overlay_shot(&mut app, &mut pixels, size, OVERLAY_COURSES, deadline)?;
    saved.push(OVERLAY_COURSES.to_owned());

    let mut empty = pack_app(pack, "overlays-empty");
    empty.shared.library = Library::from_songs(Vec::new());
    empty.shared.select_view = SelectView::AllSongs;
    empty.shared.rebuild_select_items();
    empty.shared.audio_failed = true;
    empty.shared.config.library.preview = false;
    empty.stage = Stage::Select(Box::new(SelectState::new()));
    wait_for_browser(&mut empty, &mut pixels)?;
    overlay_shot(&mut empty, &mut pixels, size, OVERLAY_FIRST_RUN, deadline)?;
    let nothing = [key_down(KeyCode::Slash)].into_iter().chain(OVERLAY_NO_MATCH.map(|(code, text)| typed_key(code, text)));
    for key in nothing {
        let code = key.code;
        press_key(&mut empty, key);
        press_key(&mut empty, key_up(code));
        browser_frame_at(&mut empty, OVERLAY_AT_MS, &mut pixels, deadline)?;
    }
    overlay_shot(&mut empty, &mut pixels, size, OVERLAY_NO_RESULTS, deadline)?;
    saved.extend([OVERLAY_FIRST_RUN, OVERLAY_NO_RESULTS].map(str::to_owned));
    Ok(saved)
}

/// Captures the application's own overlays over the browser of a pack somebody else wrote.
///
/// Opt-in like the captures above it: without [`SKIN_PACK_ENV`] this passes without drawing
/// anything. With it, the pack's browser has to draw with every overlay up, and neither the pack's
/// folder nor the sample folder may be changed by it.
#[test]
fn the_overlays_the_application_draws_over_the_browser_of_a_skin_pack_named_by_the_environment_are_captured() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join(BROWSER_SAMPLE_FOLDER);
    let before = (files_under(&pack), files_under(&samples));

    let saved = capture_browser_overlays(&pack).unwrap_or_else(|reason| panic!("select overlays: not drawn: {reason}"));
    println!("select overlays: captured {}", saved.join(", "));

    assert_eq!((files_under(&pack), files_under(&samples)), before, "capturing the pack's browser overlays changed a folder it only reads");
}

/// How many keysounds the chart of a captured play scene is still waiting for while it loads, and how
/// many of them have arrived, so the skin's loading bar has something between nothing and all to show.
const PLAY_LOADING_TOTAL: usize = 8;
const PLAY_LOADING_DONE: usize = 5;

/// How far a captured run is stepped at a time, which is a frame at sixty a second.
const PLAY_STEP_MS: i64 = 16;

/// How many steps a run nobody plays is given to fail in, and a run that is over to begin its fade.
const PLAY_FAIL_STEPS: usize = 4_000;
const PLAY_CLOSING_STEPS: usize = 400;

/// How long after the last note of a run the engine played a frame is taken, which is part of the way
/// through what a skin plays for a full combo.
const PLAY_FULL_COMBO_AFTER_MS: i64 = 700;

/// The moments of a failure's closing that are captured: it is cut into this many shares, and a
/// frame is taken at the end of each but the last, which is where the screen leaves.
const PLAY_CLOSING_SHARES: i64 = 4;
const PLAY_CLOSING_SHOTS: [&str; 3] = ["close-1", "close-2", "close-3"];

/// The moments of the sample chart and of the written charts that are captured while they play, in
/// milliseconds of the chart.
const PLAY_SAMPLE_SHOTS_MS: [i64; 1] = [2_600];
const PLAY_RICH_SHOTS_MS: [i64; 2] = [6_500, 21_000];
const PLAY_MODE_SHOT_MS: [i64; 1] = [9_000];

/// How many measures the charts written for five, ten and fourteen keys run for.
const MODE_CHART_MEASURES: u32 = 16;

/// The channels a chart written for one side of a mode puts its notes on: the keys in order, then the
/// turntable, and the channel one of its keys' long notes are written on.
struct SideChannels {
    notes: &'static [&'static str],
    long: &'static str,
    /// Which of `notes` is the key the long notes are in, whose plain notes are left out of a
    /// measure that has one.
    long_key: usize,
}

const FIVE_KEYS_FIRST: SideChannels = SideChannels { notes: &["11", "12", "13", "14", "15", "16"], long: "53", long_key: 2 };
const FIVE_KEYS_SECOND: SideChannels = SideChannels { notes: &["21", "22", "23", "24", "25", "26"], long: "63", long_key: 2 };
const SEVEN_KEYS_FIRST: SideChannels = SideChannels { notes: &["11", "12", "13", "14", "15", "18", "19", "16"], long: "54", long_key: 3 };
const SEVEN_KEYS_SECOND: SideChannels = SideChannels { notes: &["21", "22", "23", "24", "25", "28", "29", "26"], long: "64", long_key: 3 };

/// A chart with notes on every channel of `sides`, in a pattern that leaves each lane busy in some
/// measures and resting in others, and a long note every fourth measure.
fn mode_chart(title: &str, sides: &[SideChannels]) -> String {
    /// One lane in this many rests in each measure, a different one each time.
    const RESTING_LANE_EVERY: usize = 3;
    /// The most notes a lane is given in one measure.
    const BUSIEST_MEASURE_NOTES: usize = 4;
    /// One measure in this many has a long note.
    const LONG_NOTE_EVERY: u32 = 4;
    let mut chart = format!(
        "#PLAYER 1\n#GENRE Capture Genre\n#TITLE {title}\n#ARTIST Capture Artist\n#BPM 140\n#PLAYLEVEL 7\n#DIFFICULTY 3\n#RANK 2\n#TOTAL 300\n#LNTYPE 1\n#WAV01 a.wav\n"
    );
    for measure in 0..MODE_CHART_MEASURES {
        let has_long_note = measure.is_multiple_of(LONG_NOTE_EVERY);
        for side in sides {
            for (index, channel) in side.notes.iter().enumerate() {
                let under_the_long_note = has_long_note && index == side.long_key;
                if !(measure as usize + index).is_multiple_of(RESTING_LANE_EVERY) && !under_the_long_note {
                    let notes = 1 + (measure as usize + index) % BUSIEST_MEASURE_NOTES;
                    chart.push_str(&format!("#{measure:03}{channel}:{}\n", "01".repeat(notes)));
                }
            }
            if has_long_note {
                chart.push_str(&format!("#{measure:03}{}:01000001\n", side.long));
            }
        }
    }
    chart
}

/// The chart in `text` as the run of a mode sees it. `mode` names the mode for a chart the file's own
/// channels cannot tell from another: ten keys are written on the channels fourteen are.
fn chart_model(text: &str, name: &str, mode: Option<rbms_model::Mode>) -> rbms_model::Model {
    let source = rbms_parser::parse_with(text.as_bytes(), Default::default());
    let mode = mode.unwrap_or_else(|| rbms_chart::detect_mode(&source, name));
    rbms_chart::to_model(&source, mode)
}

/// How a captured run is played and how it is left.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayEnding {
    /// The engine plays the chart, and the run is left while it is still playing.
    StillPlaying,
    /// The engine plays the chart to its end: the fade that follows is captured.
    Fade,
    /// Nobody plays the chart, on a gauge that empties: the failure's closing is captured.
    Failure,
}

/// One run of the play scene to capture.
struct PlayRun<'a> {
    /// The settings folder of the run's own app, and the start of every name it saves.
    tag: &'a str,
    prefix: &'a str,
    model: rbms_model::Model,
    /// The file the chart is said to be, which is where its pictures would be looked for.
    chart: PathBuf,
    /// Whether the loading and the ready states are captured on the way in.
    opening: bool,
    /// The moments of the chart that are captured while it plays, in order.
    shots_ms: &'a [i64],
    ending: PlayEnding,
}

/// The play screen that is up, or why there is none.
fn play_screen(app: &App) -> Result<&PlayState, String> {
    match &app.stage {
        Stage::Play(state) => Ok(state),
        other => Err(format!("the {other:?} screen is up, not the play screen")),
    }
}

/// The two clocks of a captured run, counted by the harness rather than read off the machine.
///
/// The scene clock and the song clock are both real clocks, and a frame of a published skin takes a
/// good part of a second to draw on a headless canvas -- and saving one takes as long again. So each
/// frame of a captured run puts both clocks where this says they are before it runs: the run moves
/// by what was asked for and by nothing else, whatever machine draws it.
#[derive(Default)]
struct PlayClocks {
    /// How long the scene has run.
    scene: Duration,
    /// How long the chart has played, once it has started.
    song: Option<Duration>,
}

impl PlayClocks {
    /// Move both clocks on by `millis` and run the update of one frame of the play screen, and its
    /// draw as well when `canvas` is given.
    fn frame_after(&mut self, app: &mut App, millis: i64, canvas: Option<&mut Canvas<'_>>) -> Result<(), String> {
        let by = Duration::from_millis(u64::try_from(millis).map_err(|_| "a run only moves forward".to_owned())?);
        self.scene += by;
        self.song = self.song.map(|song| song + by);
        let now = Instant::now();
        let behind = |by: Duration| now.checked_sub(by).ok_or_else(|| "the process has not been up for as long as the run".to_owned());
        app.shared.scene_started = behind(self.scene)?;
        if let Some(song) = self.song {
            app.shared.clock = behind(song)?;
        }
        let transition = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
        if !matches!(transition, Transition::Stay) {
            return Err(format!("the play screen left at {:?}", play_screen(app).map(|play| play.phase())));
        }
        if self.song.is_none() && play_screen(app)?.phase() == Some(crate::skin_host::play::PlayPhase::Play) {
            self.song = Some(Duration::ZERO);
        }
        if let Some(canvas) = canvas {
            app.shared.hot.clear();
            app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, canvas);
        }
        Ok(())
    }

    /// Run the play screen on by `millis` in steps of [`PLAY_STEP_MS`], the way frames would carry
    /// it there, and draw the last of them. A run is not jumped: the engine plays a chart a frame at
    /// a time, and a long note it is handed whole, head and end in one frame, is not one it holds.
    fn play_on(&mut self, app: &mut App, millis: i64, canvas: &mut Canvas<'_>) -> Result<(), String> {
        let mut left = millis.max(0);
        while left > PLAY_STEP_MS {
            self.frame_after(app, PLAY_STEP_MS, None)?;
            left -= PLAY_STEP_MS;
        }
        self.frame_after(app, left, Some(canvas))
    }

    /// How long the scene has run, in whole milliseconds.
    fn scene_ms(&self) -> u128 {
        self.scene.as_millis()
    }
}

/// What a frame of a captured run shows, in words, for whoever compares the capture with it.
fn play_report(app: &App, scene_ms: u128) -> Result<String, String> {
    let play = play_screen(app)?;
    let judge = play.session.judge();
    Ok(format!(
        "{:?} at {} ms of the chart, scene {} ms: combo {} (max {}), EX {}, gauge {:.1}, PG/GR/GD/BD/PR/MS {:?}, {} of {} notes{MOVIES_REPORTED_AS}{:?}",
        play.phase(),
        play.chart_ms(),
        scene_ms,
        judge.combo,
        judge.max_combo,
        judge.ex_score,
        judge.gauge.value(),
        judge.counts,
        judge.total_judged(),
        judge.total_notes(),
        app.shared.skin_screens.movie_frames_on_show(),
    ))
}

/// What a play report says ahead of the movie frames that were on show: when each is due, in
/// microseconds since its movie was started.
const MOVIES_REPORTED_AS: &str = ", movie frames on show at ";

/// Enter the pack's play scene for one run and walk it, saving a frame at each moment the run asks
/// for as `<prefix>-<moment>`. Answers what was saved, each with what the frame shows, or why the
/// scene never drew.
fn capture_play_scene(pack: &Path, run: PlayRun<'_>) -> Result<Vec<String>, String> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use crate::skin_host::play::PlayPhase;
    use crate::stage::KeysoundLoad;
    use crate::stage::loading::ChartLoads;

    let mut app = pack_app(pack, run.tag);
    let mode = run.model.mode;
    let screen = rbms_skin::loader::mode_skin_type(mode).ok_or_else(|| "no play screen draws this mode".to_owned())?;
    let size = authored_size(&app, screen).ok_or_else(|| "the pack has no document for this screen".to_owned())?;
    let autoplay = run.ending != PlayEnding::Failure;
    let gauge = if autoplay { rbms_judge::GaugeKind::Normal } else { rbms_judge::GaugeKind::Hard };
    app.shared.chart_path = run.chart.to_string_lossy().into_owned();
    app.shared.mode = mode;
    app.shared.config.play.autoplay = autoplay;
    app.shared.config.play.gauge = gauge;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(mode);
    let play_time_ms = rbms_play::play_time_ms(&run.model, autoplay);
    let session = PlaySession::new(run.model, SessionOptions { autoplay, gauge, ..SessionOptions::default() });
    let mut play = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string());
    let arrived = Arc::new(AtomicUsize::new(PLAY_LOADING_DONE));
    let (_sender, rx) = std::sync::mpsc::channel();
    let keysounds = KeysoundLoad { rx, progress: Arc::clone(&arrived), cancel: Arc::new(AtomicBool::new(false)), total: PLAY_LOADING_TOTAL };
    play.wait_for(ChartLoads::new(None, Some(keysounds)));
    app.switch(Transition::To(Stage::Play(Box::new(play))));

    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !app.shared.has_compiled_skin(screen) {
        scene_frame_at(&mut app, 0, &mut Canvas::Headless(&mut pixels));
        if let Some(reason) = app.shared.skin_failure(screen) {
            return Err(reason.lines().next().unwrap_or_default().to_owned());
        }
        if Instant::now() >= deadline {
            return Err(format!("its files were not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs()));
        }
    }
    let mut clocks = PlayClocks::default();
    let skin = app.shared.skins.document(screen).ok_or_else(|| "the skin was let go of".to_owned())?;
    let (loadend_ms, playstart_ms, close_ms, fadeout_ms) =
        (i64::from(skin.play.loadend), i64::from(skin.play.playstart), i64::from(skin.play.close), i64::from(skin.def.fadeout));

    let mut saved = Vec::new();
    let mut shoot = |app: &mut App, pixels: &HeadlessCanvas, clocks: &PlayClocks, moment: &str| -> Result<(), String> {
        let name = format!("{}-{moment}", run.prefix);
        save(&name, CAPTURE_EXTENSION, size, pixels.rgba());
        saved.push(format!("{name}: {}", play_report(app, clocks.scene_ms())?));
        Ok(())
    };

    clocks.play_on(&mut app, loadend_ms / 2, &mut Canvas::Headless(&mut pixels))?;
    if run.opening {
        shoot(&mut app, &pixels, &clocks, "loading")?;
    }
    arrived.store(PLAY_LOADING_TOTAL, Ordering::Relaxed);
    clocks.play_on(&mut app, loadend_ms / 2 + PLAY_STEP_MS, &mut Canvas::Headless(&mut pixels))?;
    if play_screen(&app)?.phase() != Some(PlayPhase::Ready) {
        return Err(format!("the chart is not ready after its load time: {:?}", play_screen(&app)?.phase()));
    }
    clocks.play_on(&mut app, playstart_ms / 2, &mut Canvas::Headless(&mut pixels))?;
    if run.opening {
        shoot(&mut app, &pixels, &clocks, "ready")?;
    }
    clocks.play_on(&mut app, playstart_ms / 2 + PLAY_STEP_MS, &mut Canvas::Headless(&mut pixels))?;
    if play_screen(&app)?.phase() != Some(PlayPhase::Play) {
        return Err(format!("the chart is not playing after its start time: {:?}", play_screen(&app)?.phase()));
    }

    for at_ms in run.shots_ms {
        let left_ms = at_ms - play_screen(&app)?.chart_ms();
        clocks.play_on(&mut app, left_ms, &mut Canvas::Headless(&mut pixels))?;
        shoot(&mut app, &pixels, &clocks, &format!("play-{at_ms:05}ms"))?;
    }

    match run.ending {
        PlayEnding::StillPlaying => {}
        PlayEnding::Fade => {
            let to_the_last_note_ms = play_time_ms - rbms_play::PLAY_TIME_MARGIN_MS - play_screen(&app)?.chart_ms();
            clocks.play_on(&mut app, to_the_last_note_ms + PLAY_FULL_COMBO_AFTER_MS, &mut Canvas::Headless(&mut pixels))?;
            shoot(&mut app, &pixels, &clocks, "fullcombo")?;
            let left_ms = play_time_ms - play_screen(&app)?.chart_ms() + PLAY_STEP_MS;
            clocks.play_on(&mut app, left_ms, &mut Canvas::Headless(&mut pixels))?;
            if play_screen(&app)?.phase() != Some(PlayPhase::Finished) {
                return Err(format!("the run is not finished after its playing time: {:?}", play_screen(&app)?.phase()));
            }
            shoot(&mut app, &pixels, &clocks, "finished")?;
            let mut waited = 0;
            while !app.shared.skin_timers.is_on(timer_id::FADEOUT) {
                clocks.frame_after(&mut app, PLAY_STEP_MS, None)?;
                waited += 1;
                if waited > PLAY_CLOSING_STEPS {
                    return Err("the run never began to fade".to_owned());
                }
            }
            clocks.play_on(&mut app, fadeout_ms / 2, &mut Canvas::Headless(&mut pixels))?;
            shoot(&mut app, &pixels, &clocks, "fade")?;
        }
        PlayEnding::Failure => {
            let mut waited = 0;
            while play_screen(&app)?.phase() != Some(PlayPhase::Failed) {
                clocks.frame_after(&mut app, PLAY_STEP_MS, None)?;
                waited += 1;
                if waited > PLAY_FAIL_STEPS {
                    return Err("the run nobody played never failed".to_owned());
                }
            }
            for (share, moment) in PLAY_CLOSING_SHOTS.into_iter().enumerate() {
                clocks.play_on(&mut app, close_ms / PLAY_CLOSING_SHARES, &mut Canvas::Headless(&mut pixels))?;
                if share + 1 < PLAY_CLOSING_SHOTS.len() || play_screen(&app)?.phase() == Some(PlayPhase::Failed) {
                    shoot(&mut app, &pixels, &clocks, moment)?;
                }
            }
        }
    }
    Ok(saved)
}

/// Captures the play scene of a pack somebody else wrote, for runs that are really made: the sample
/// chart, which is a five-key one, walked from loading until it plays; a busier chart written here
/// on seven keys, walked from loading to the fade and, with nobody playing it, to a failure; and one
/// chart each on five, ten and fourteen keys while it plays.
///
/// Opt-in like the captures above it: without [`SKIN_PACK_ENV`] this passes without drawing
/// anything. With it, every run has to draw, and the pack's folder has to be left as it was.
#[test]
fn the_play_scene_of_a_skin_pack_named_by_the_environment_is_captured_through_its_states() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let before = files_under(&pack);

    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join(DECIDE_SAMPLE_CHART);
    let sample_text = std::fs::read_to_string(&sample).expect("the sample chart is read");
    let rich = write_rich_chart("play-rich-chart");
    let rich_text = rich_chart();
    let five = mode_chart("Five Keys", &[FIVE_KEYS_FIRST]);
    let ten = mode_chart("Ten Keys", &[FIVE_KEYS_FIRST, FIVE_KEYS_SECOND]);
    let fourteen = mode_chart("Fourteen Keys", &[SEVEN_KEYS_FIRST, SEVEN_KEYS_SECOND]);
    let written = |name: &str| rich.with_file_name(name);

    let runs = [
        PlayRun {
            tag: "play-sample",
            prefix: "play-sample",
            model: chart_model(&sample_text, "preview-demo.bms", None),
            chart: sample,
            opening: true,
            shots_ms: &PLAY_SAMPLE_SHOTS_MS,
            ending: PlayEnding::StillPlaying,
        },
        PlayRun {
            tag: "play7",
            prefix: "play7",
            model: chart_model(&rich_text, "rich.bms", None),
            chart: rich.clone(),
            opening: true,
            shots_ms: &PLAY_RICH_SHOTS_MS,
            ending: PlayEnding::Fade,
        },
        PlayRun {
            tag: "play7-fail",
            prefix: "play7-fail",
            model: chart_model(&rich_text, "rich.bms", None),
            chart: rich.clone(),
            opening: false,
            shots_ms: &[],
            ending: PlayEnding::Failure,
        },
        PlayRun {
            tag: "play5",
            prefix: "play5",
            model: chart_model(&five, "five.bms", None),
            chart: written("five.bms"),
            opening: false,
            shots_ms: &PLAY_MODE_SHOT_MS,
            ending: PlayEnding::StillPlaying,
        },
        PlayRun {
            tag: "play10",
            prefix: "play10",
            model: chart_model(&ten, "ten.bms", Some(rbms_model::Mode::BEAT_10K)),
            chart: written("ten.bms"),
            opening: false,
            shots_ms: &PLAY_MODE_SHOT_MS,
            ending: PlayEnding::StillPlaying,
        },
        PlayRun {
            tag: "play14",
            prefix: "play14",
            model: chart_model(&fourteen, "fourteen.bms", None),
            chart: written("fourteen.bms"),
            opening: false,
            shots_ms: &PLAY_MODE_SHOT_MS,
            ending: PlayEnding::StillPlaying,
        },
    ];
    for run in runs {
        let prefix = run.prefix;
        let saved = capture_play_scene(&pack, run).unwrap_or_else(|reason| panic!("{prefix}: not drawn: {reason}"));
        for line in saved {
            println!("{line}");
        }
    }
    assert_eq!(files_under(&pack), before, "capturing the pack's play scene changed its folder");
}

/// The row a pack this was written against chooses what is behind its decide scene and its browser
/// with, and what the name of the item that makes it a movie starts with.
const MOVIE_BACKGROUND_ROW: &str = "背景の種類";
const MOVIE_BACKGROUND_ON: &str = "動画";

/// The moments of the decide scene its movie is captured at.
const DECIDE_MOVIE_SHOTS_MS: [i64; 3] = [1000, 2000, 3000];

/// The moment the decide scene's clock is put back to once the last of those is captured, which
/// its movie has to follow back to the frame it showed then.
const DECIDE_MOVIE_BACK_MS: i64 = DECIDE_MOVIE_SHOTS_MS[0];

/// One more moment of the browser its movie is captured at, after the two every browser capture is
/// taken at.
const BROWSER_MOVIE_LATER_MS: i64 = 6000;

/// The moments of a run its movie is captured at.
const PLAY_MOVIE_SHOTS_MS: [i64; 2] = [3_000, 9_000];

/// Whether the movie frames that were on show at each of several captures were each a later frame
/// of a movie than the capture before had: every capture has one, and they move on.
fn movies_moved_on(on_show: &[Vec<i64>]) -> bool {
    on_show.iter().all(|frames| !frames.is_empty()) && on_show.windows(2).all(|pair| pair[0] < pair[1])
}

/// Walk the decide scene that is up through [`DECIDE_MOVIE_SHOTS_MS`] on `canvas` and then back to
/// [`DECIDE_MOVIE_BACK_MS`], running `shoot` with a name for each moment once its frame is drawn.
/// Answers when the movie frames on show at each of the moments walked through were due, or why
/// the scene never drew or its movie did not follow the clock back.
fn walk_decide_movie(app: &mut App, canvas: &mut Canvas<'_>, mut shoot: impl FnMut(&str, &mut Canvas<'_>)) -> Result<Vec<Vec<i64>>, String> {
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !app.shared.has_compiled_skin(SKIN_TYPE_DECIDE) {
        scene_frame_at(app, 0, canvas);
        if let Some(reason) = app.shared.skin_failure(SKIN_TYPE_DECIDE) {
            return Err(reason.lines().next().unwrap_or_default().to_owned());
        }
        if Instant::now() >= deadline {
            return Err(format!("its files were not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs()));
        }
    }
    scene_frame_at(app, 0, canvas);
    let mut on_show = Vec::new();
    for at_ms in DECIDE_MOVIE_SHOTS_MS {
        if !matches!(scene_frame_at(app, at_ms, canvas), Transition::Stay) {
            return Err(format!("the scene was over by {at_ms} ms"));
        }
        shoot(&format!("{at_ms:04}ms"), canvas);
        on_show.push(app.shared.skin_screens.movie_frames_on_show());
    }
    if !matches!(scene_frame_at(app, DECIDE_MOVIE_BACK_MS, canvas), Transition::Stay) {
        return Err(format!("the scene was over when its clock was put back to {DECIDE_MOVIE_BACK_MS} ms"));
    }
    shoot(&format!("back-{DECIDE_MOVIE_BACK_MS:04}ms"), canvas);
    let back = app.shared.skin_screens.movie_frames_on_show();
    if on_show.first() != Some(&back) {
        return Err(format!("the movie did not follow the scene clock back: {back:?} is on show where {:?} was", on_show.first()));
    }
    Ok(on_show)
}

/// Captures the movies of a pack somebody else wrote as they play: behind its decide scene and its
/// browser, each switched to a movie background, and behind the play scene of a chart that has no
/// pictures of its own. Each is captured at several moments of its scene, and the frame of the
/// movie that is on show has to be a later one at each. The decide scene's clock is then put back
/// to the first of its moments, where the movie has to show the frame it showed there before. The
/// decide scene is captured on the GPU as well, where a machine has one, since that is the backend
/// a movie is uploaded to frame after frame.
///
/// Opt-in like the captures above it, and for a build that decodes movies; one that does not says
/// so and passes. The pack's folder has to be left as it was.
#[test]
fn the_movies_of_a_skin_pack_named_by_the_environment_are_captured_as_they_play() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    if !rbms_video::is_enabled() {
        println!("movies: this build decodes no movies, so there is nothing to capture");
        return;
    }
    let before = files_under(&pack);
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join(DECIDE_SAMPLE_CHART);
    let choice = (MOVIE_BACKGROUND_ROW, MOVIE_BACKGROUND_ON);

    if let Some((mut app, size)) = decide_with_choice(&pack, "decide-movie", &sample, choice) {
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        let on_show = walk_decide_movie(&mut app, &mut Canvas::Headless(&mut pixels), |moment, canvas| {
            if let Canvas::Headless(pixels) = canvas {
                save(&format!("decide-movie-{moment}"), CAPTURE_EXTENSION, size, pixels.rgba());
            }
        })
        .unwrap_or_else(|reason| panic!("decide-movie: not drawn: {reason}"));
        println!("decide-movie: movie frames on show at {on_show:?}; warnings {:?}", app.shared.skin_warnings(SKIN_TYPE_DECIDE));
        assert!(movies_moved_on(&on_show), "decide-movie: the movie behind the scene did not play: {on_show:?}");

        let gpu = decide_with_choice(&pack, "decide-movie-gpu", &sample, choice).zip(Gpu::offscreen_if_available(size.0, size.1, UI_SIZE));
        if let Some(((mut app, _), mut gpu)) = gpu {
            let mut frames: Vec<Vec<u8>> = Vec::new();
            let on_show = walk_decide_movie(&mut app, &mut Canvas::Window(&mut gpu), |moment, canvas| {
                if let Canvas::Window(gpu) = canvas {
                    let rgba = gpu.capture().expect("an offscreen target reads back");
                    save(&format!("decide-movie-{moment}"), GPU_CAPTURE_EXTENSION, size, &rgba);
                    frames.push(rgba);
                }
            })
            .unwrap_or_else(|reason| panic!("decide-movie.gpu: not drawn: {reason}"));
            println!("decide-movie.gpu: movie frames on show at {on_show:?}");
            assert!(movies_moved_on(&on_show), "decide-movie.gpu: the movie behind the scene did not play: {on_show:?}");
            assert!(frames.windows(2).all(|pair| pair[0] != pair[1]), "decide-movie.gpu: two moments of the scene came out as one frame");
        } else {
            println!("decide-movie.gpu: this machine has no graphics adapter");
        }
    } else {
        println!("decide-movie: the pack's decide document offers no {MOVIE_BACKGROUND_ROW:?} row to switch to {MOVIE_BACKGROUND_ON:?}");
    }

    let mut app = browse_app(&pack, "select-movie").unwrap_or_else(|reason| panic!("select-movie: not drawn: {reason}"));
    if choose_in_skin(&mut app, SKIN_TYPE_MUSIC_SELECT, MOVIE_BACKGROUND_ROW, MOVIE_BACKGROUND_ON) {
        let size = authored_size(&app, SKIN_TYPE_MUSIC_SELECT).expect("a document that offers a choice says its size");
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        app.shared.select_view = SelectView::AllSongs;
        app.shared.rebuild_select_items();
        let saved = capture_browser_view(&mut app, &mut pixels, size, "select-movie", SelectState::new())
            .unwrap_or_else(|reason| panic!("select-movie: not drawn: {reason}"));
        let mut on_show = vec![app.shared.skin_screens.movie_frames_on_show()];
        browser_frame_at(&mut app, BROWSER_MOVIE_LATER_MS, &mut pixels, Instant::now() + PACK_LOAD_TIMEOUT)
            .unwrap_or_else(|reason| panic!("select-movie: not drawn: {reason}"));
        let later = format!("select-movie-{BROWSER_MOVIE_LATER_MS:04}ms");
        save(&later, CAPTURE_EXTENSION, size, pixels.rgba());
        on_show.push(app.shared.skin_screens.movie_frames_on_show());
        println!("select-movie: captured {}, {later}; movie frames on show at {on_show:?}", saved.join(", "));
        assert!(movies_moved_on(&on_show), "select-movie: the movie behind the browser did not play: {on_show:?}");
    } else {
        println!("select-movie: the pack's browser document offers no {MOVIE_BACKGROUND_ROW:?} row to switch to {MOVIE_BACKGROUND_ON:?}");
    }

    let rich = write_rich_chart("play-movie-chart");
    let five = mode_chart("Five Keys", &[FIVE_KEYS_FIRST]);
    let run = PlayRun {
        tag: "play-movie",
        prefix: "play-movie",
        model: chart_model(&five, "five.bms", None),
        chart: rich.with_file_name("five.bms"),
        opening: true,
        shots_ms: &PLAY_MOVIE_SHOTS_MS,
        ending: PlayEnding::StillPlaying,
    };
    let saved = capture_play_scene(&pack, run).unwrap_or_else(|reason| panic!("play-movie: not drawn: {reason}"));
    for line in &saved {
        println!("{line}");
    }
    let playing: Vec<&str> = saved.iter().filter(|line| line.contains("-play-")).filter_map(|line| line.split(MOVIES_REPORTED_AS).nth(1)).collect();
    assert_eq!(playing.len(), PLAY_MOVIE_SHOTS_MS.len(), "play-movie: a moment of the run was not captured: {saved:?}");
    assert!(playing.iter().all(|frames| *frames != "[]"), "play-movie: a chart with no pictures was played with no movie behind it: {playing:?}");
    assert!(playing.windows(2).all(|pair| pair[0] != pair[1]), "play-movie: the movie behind the run did not play: {playing:?}");

    assert_eq!(files_under(&pack), before, "capturing the pack's movies changed its folder");
}
