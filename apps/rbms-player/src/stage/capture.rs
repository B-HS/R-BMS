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

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use rbms_config::DEFAULT_SKIN_FOLDER;
use rbms_render::Renderer;
use rbms_skin::loader::{SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_RESULT};
use rbms_skin::timer::timer_id;

use crate::app_play::loaded_chart_for_tests;
use crate::gpu::Gpu;
use crate::skin_select::SKIN_PACK_ENV;
use crate::stage::canvas::UI_SIZE;
use crate::stage::render_tests::{FRAME_DT, app, play_state, render, render_on, result_state};
use crate::stage::scene_life::SceneTimes;
use crate::stage::{Canvas, DecideState, FrameCtx, HeadlessCanvas, KeyConfigState, SelectState, Stage, Transition};
use crate::{App, Config, LaunchOptions};

/// Environment variable naming the folder captures are saved in. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

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
const PACK_SCENE: Duration = Duration::from_secs(5);

/// The seed every read of a pack's skin is pinned with, so a skin that picks a file or an option at
/// random is captured the same way each time.
const PACK_SEED: u64 = 1;

/// Longest one screen of a pack is waited for. A published pack names hundreds of megabytes of
/// images for one screen, and they are decoded before the screen draws at all.
const PACK_LOAD_TIMEOUT: Duration = Duration::from_secs(180);

/// One screen a pack is captured on.
struct PackScreen {
    /// The skin type a document declares to be drawn on this screen.
    skin_type: i32,
    /// The name the screen's capture is saved under.
    name: &'static str,
    /// Builds the stage that draws the screen.
    stage: fn() -> Stage,
}

/// The screens a pack is captured on one frame each. The decide scene is captured through its whole
/// length by a test of its own.
const PACK_SCREENS: [PackScreen; 4] = [
    PackScreen { skin_type: SKIN_TYPE_MUSIC_SELECT, name: "pack-select", stage: browser },
    PackScreen { skin_type: SKIN_TYPE_RESULT, name: "pack-result", stage: result },
    PackScreen { skin_type: SKIN_TYPE_KEY_CONFIG, name: "pack-keyconfig", stage: key_config },
    PackScreen { skin_type: SKIN_TYPE_PLAY_7KEYS, name: "pack-play7", stage: play },
];

fn browser() -> Stage {
    Stage::Select(Box::new(SelectState::new()))
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

/// An app with `pack` named as its skin pack and walked, and every read of a skin pinned.
fn pack_app(pack: &Path, tag: &str) -> App {
    let settings = settings_of(tag);
    let mut config = Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
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
        for (index, channel) in KEY_CHANNELS.into_iter().enumerate() {
            if !(measure as usize + index).is_multiple_of(RESTING_KEY_EVERY) {
                chart.push_str(&format!("#{measure:03}{channel}:{}\n", "01".repeat(notes)));
            }
        }
        if measure.is_multiple_of(TURNTABLE_EVERY) {
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
fn scene_frame_at(app: &mut App, scene_ms: i64, canvas: &mut Canvas<'_>) -> Transition {
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
