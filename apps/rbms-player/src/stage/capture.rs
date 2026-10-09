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
//! things are left to the caller. A document that draws one of its files at random does so from
//! `rbms_skin::resolve::SEED_ENV`, which has to be set for the pick to repeat. And the scene clock
//! is a real clock: a frame reads it a moment after it was set, so the time a document sees is the
//! shot's plus however long the frame has been drawing.
//!
//! [`SKIN_PACK_ENV`] points the last test here at a folder of documents somebody else wrote. It is
//! the one test that draws files from outside the repository, and it passes without doing anything
//! when the variable is not set.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rbms_config::DEFAULT_SKIN_FOLDER;
use rbms_render::Renderer;
use rbms_skin::loader::{SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_RESULT};
use rbms_skin::timer::timer_id;

use crate::gpu::Gpu;
use crate::stage::canvas::UI_SIZE;
use crate::stage::render_tests::{app, play_state, render, render_on, result_state};
use crate::stage::{Canvas, HeadlessCanvas, KeyConfigState, LoadingState, SelectState, Stage};
use crate::{App, Config, LaunchOptions};

/// Environment variable naming the folder captures are saved in. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// Environment variable naming a folder of skin documents to capture, one per screen.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// Extension of a headless capture.
const CAPTURE_EXTENSION: &str = "png";

/// Extension of the same frame drawn by the GPU, so the two sit side by side under one name.
const GPU_CAPTURE_EXTENSION: &str = "gpu.png";

/// Frames a test draws while a document's files are read on the worker pool.
///
/// A document is compiled off the frame loop, so the frame it is selected on still draws the
/// built-in layout and the document takes over once its images have arrived. Four seconds of frames
/// is far more than a small image needs and still fails rather than hanging if the pool never
/// finishes.
const DOCUMENT_LOAD_FRAMES: usize = 240;

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
fn draw_until_compiled_on(app: &mut App, screen: i32, scene_us: i64, canvas: &mut Canvas<'_>, stage: impl Fn() -> Stage) -> bool {
    for _ in 0..DOCUMENT_LOAD_FRAMES {
        draw_on(app, stage(), scene_us, canvas);
        if app.shared.has_compiled_skin(screen) {
            draw_on(app, stage(), scene_us, canvas);
            return true;
        }
    }
    false
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
const PACK_SCENE_US: i64 = 5_000_000;

/// One screen a pack is captured on.
struct PackScreen {
    /// The skin type a document declares to be drawn on this screen.
    skin_type: i32,
    /// The name the screen's capture is saved under.
    name: &'static str,
    /// Builds the stage that draws the screen.
    stage: fn() -> Stage,
}

/// The screens a pack is captured on.
const PACK_SCREENS: [PackScreen; 5] = [
    PackScreen { skin_type: SKIN_TYPE_MUSIC_SELECT, name: "pack-select", stage: browser },
    PackScreen { skin_type: SKIN_TYPE_DECIDE, name: "pack-decide", stage: loading },
    PackScreen { skin_type: SKIN_TYPE_RESULT, name: "pack-result", stage: result },
    PackScreen { skin_type: SKIN_TYPE_KEY_CONFIG, name: "pack-keyconfig", stage: key_config },
    PackScreen { skin_type: SKIN_TYPE_PLAY_7KEYS, name: "pack-play7", stage: play },
];

fn browser() -> Stage {
    Stage::Select(Box::new(SelectState::new()))
}

fn loading() -> Stage {
    Stage::Loading(LoadingState::song(0))
}

fn result() -> Stage {
    Stage::Result(result_state())
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

/// An app with the first document `pack` holds for `screen` chosen and read the way the SKIN tab
/// chooses and reads one, or `None` when the pack has no document for that screen.
fn pack_app(pack: &Path, tag: &str, screen: i32) -> Option<App> {
    let settings = settings_of(tag);
    let mut config = Config::default();
    config.skin.folder = Some(pack.to_string_lossy().into_owned());
    config.skin.screen = screen;
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.rescan(&settings, &app.shared.config);
    if !app.shared.skins.cycle_document(&mut app.shared.config, 1) {
        return None;
    }
    app.shared.skins.reload_for(&app.shared.config, screen);
    Some(app)
}

/// The size the document chosen for `screen` was authored at, which is the size it is captured at,
/// or why the library would not read it.
fn authored_size(app: &App, screen: i32) -> Result<(u32, u32), String> {
    let Some(skin) = app.shared.skins.document(screen) else {
        return Err(app.shared.skins.info(&app.shared.config));
    };
    let side = |length: i32, fallback: u32| u32::try_from(length).ok().filter(|length| *length > 0).unwrap_or(fallback);
    Ok((side(skin.def.w, UI_SIZE.0), side(skin.def.h, UI_SIZE.1)))
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

/// Captures a pack somebody else wrote: the first document it holds for each screen, on a target
/// the size the document was authored at, on both backends.
///
/// Opt-in, because the pack is outside the repository: without [`SKIN_PACK_ENV`] this passes without
/// drawing anything. With it, every screen the pack has a document for is captured before anything
/// is asserted, so one document that cannot be drawn does not cost the captures of the others.
#[test]
fn a_skin_pack_named_by_the_environment_is_captured_one_document_a_screen() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());

    let mut undrawn = Vec::new();
    for PackScreen { skin_type: screen, name, stage } in PACK_SCREENS {
        let Some(mut app) = pack_app(&pack, name, screen) else {
            continue;
        };
        let size = match authored_size(&app, screen) {
            Ok(size) => size,
            Err(reason) => {
                undrawn.push(format!("{name}: {reason}"));
                continue;
            }
        };
        let shot = Shot { name, size, scene_us: PACK_SCENE_US, document: Some(screen) };
        if let Err(missed) = shot.take(&mut app, stage) {
            undrawn.push(format!("{name}: {missed:?}"));
            continue;
        }
        let on_gpu = pack_app(&pack, &format!("{name}-gpu"), screen).map(|mut app| shot.take_on_gpu(&mut app, stage));
        if let Some(Err(Missed::NotCompiled)) = on_gpu {
            undrawn.push(format!("{name}.gpu: {:?}", Missed::NotCompiled));
        }
    }
    assert!(undrawn.is_empty(), "the pack has documents that were not drawn: {undrawn:#?}");
}
