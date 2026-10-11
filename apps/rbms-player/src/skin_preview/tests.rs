//! The preview from the menu's side of its boundary: what it says of a screen, what it draws and
//! where, that what it draws moves, and what it leaves behind.
//!
//! Every test goes through the calls the menu makes -- `update`, `draw`, `settings_changed` and
//! `release` -- on an application with no window, no sound device, no library and a settings folder
//! of its own. A frame is made the way the menu makes one: the preview is brought up to date, the
//! frame is cleared, the preview is drawn into its rectangle, and the frame is ended.
//!
//! Time is the preview's own, counted from what `update` is handed, so a test moves a scene by
//! handing it seconds and never by waiting. The one thing that is waited for is a document's files,
//! which are decoded on other threads: the wait ends on the first frame that does not say the
//! document is on its way, whatever that frame says instead.
//!
//! With [`CAPTURE_DIR_ENV`] naming a folder, the frames are also saved there as
//! `preview-<screen>-<milliseconds>.png`. With [`SKIN_PACK_ENV`] naming a pack as well, that pack's
//! screens are previewed and saved as `preview-pack-<screen>-<milliseconds>.png`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rbms_config::{DEFAULT_SKIN_FOLDER, SettingId};
use rbms_render::{Color, FrameData, Rect, Renderer};
use rbms_skin::loader::{
    SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS,
    SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_RESULT, SKIN_TYPE_SKIN_SELECT,
};
use rbms_skin::resolve::plain_path;

use super::*;
use crate::gpu::Gpu;
use crate::skin_host::chart::ChartMeta;
use crate::skin_host::play::PlayPhase;
use crate::skin_screen::DecideDraw;
use crate::skin_select::{OffsetAxis, SKIN_PACK_ENV, SkinRow, pack_from_environment};
use crate::stage::HeadlessCanvas;
use crate::stage::canvas::UI_SIZE;
use crate::{App, Config, LaunchOptions};

/// Environment variable naming the folder captures are saved in, the one the screen capture harness
/// reads. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The skin pack this repository ships, relative to the player's manifest.
const DEFAULT_PACK: &str = "../../assets/skins/rbms-default";

/// Where a test asks for the preview, in the space the menu is laid out in.
const RECT: Rect = Rect { x: 424.0, y: 84.0, w: 832.0, h: 468.0 };

/// A target half as large again as the one the built-in screens are laid out for, and how many
/// times larger than it that is.
const LARGE: (u32, u32) = (1920, 1080);
const LARGE_ENLARGEMENT: f32 = 1.5;
const SAME_SIZE: f32 = 1.0;

/// What the menu is taken to have cleared its frame to: a colour no skin fills a screen with, so a
/// pixel a preview drew on is told from one it left alone.
const BACKDROP: Color = Color::rgb(200, 0, 200);

/// How far a scene is moved at a time when it is moved without being drawn, in seconds: short
/// enough that a sample run is handed its chart a note at a time.
const STEP_SECONDS: f32 = 0.05;

/// A frame that moves its scene by nothing.
const NO_TIME: f32 = 0.0;

/// How long a test pauses between two frames while a document's files are being decoded.
const WORKER_PAUSE: Duration = Duration::from_millis(1);

/// What every read of a document is pinned with, so a random option falls the same way each time.
const SEED: u64 = 1;

const MICROS_PER_MILLI: i64 = 1_000;

/// How far one channel of a GPU frame may sit from the menu's clear colour and still be a pixel
/// nothing drew on.
const GPU_TOLERANCE: u8 = 2;

/// The screens the shipped pack is previewed on, each with the name its captures are saved under and
/// the moments of its scene that are drawn, in milliseconds.
const SHIPPED_SCREENS: [(i32, &str, &[i64]); 9] = [
    (SKIN_TYPE_MUSIC_SELECT, "select", &[0, 2_000, 5_000]),
    (SKIN_TYPE_DECIDE, "decide", &[500, 2_000, 3_600]),
    (SKIN_TYPE_PLAY_5KEYS, "play5", &[1_000, 7_000, 14_000]),
    (SKIN_TYPE_PLAY_7KEYS, "play7", &[1_000, 7_000, 14_000, 30_000]),
    (SKIN_TYPE_PLAY_9KEYS, "play9", &[1_000, 7_000, 14_000]),
    (SKIN_TYPE_PLAY_10KEYS, "play10", &[1_000, 7_000, 14_000]),
    (SKIN_TYPE_PLAY_14KEYS, "play14", &[1_000, 7_000, 14_000]),
    (SKIN_TYPE_RESULT, "result", &[300, 3_000, 15_300]),
    (SKIN_TYPE_COURSE_RESULT, "courseresult", &[300, 3_000]),
];

/// The screens a pack named by the environment is previewed on: the shipped ones and the screens a
/// preview has nothing to draw on, each drawn at one moment of its scene.
const PACK_SCREENS: [(i32, &str, i64); 11] = [
    (SKIN_TYPE_MUSIC_SELECT, "select", 3_000),
    (SKIN_TYPE_DECIDE, "decide", 1_500),
    (SKIN_TYPE_PLAY_5KEYS, "play5", 9_000),
    (SKIN_TYPE_PLAY_7KEYS, "play7", 9_000),
    (SKIN_TYPE_PLAY_9KEYS, "play9", 9_000),
    (SKIN_TYPE_PLAY_10KEYS, "play10", 9_000),
    (SKIN_TYPE_PLAY_14KEYS, "play14", 9_000),
    (SKIN_TYPE_RESULT, "result", 3_000),
    (SKIN_TYPE_COURSE_RESULT, "courseresult", 3_000),
    (SKIN_TYPE_KEY_CONFIG, "keyconfig", 0),
    (SKIN_TYPE_PLAY_24KEYS, "play24", 0),
];

/// A settings file in a folder of one test's own, with an empty skin folder beside it.
fn settings_of(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("rbms-skin-preview-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join(DEFAULT_SKIN_FOLDER)).expect("the fixture folder is writable");
    directory.join("settings.ron")
}

/// An app with no library, no window and no server, drawing its screens with `pack` when one is
/// named and with no skin at all otherwise.
fn app_with(tag: &str, pack: Option<&Path>) -> App {
    rbms_render::font::use_embedded_fonts_only();
    let mut app = App::new(String::new(), Config::default(), LaunchOptions::default(), settings_of(tag));
    app.shared.config.skin = rbms_config::SkinOptions::default();
    app.shared.skins = crate::skin_select::SkinLibrary::new(&app.shared.settings_path, &app.shared.config);
    app.shared.skins.pin_seed(Some(SEED));
    if let Some(pack) = pack {
        app.shared.set_skin_pack(Some(pack.to_string_lossy().into_owned()));
    }
    app
}

/// The shipped pack, by the path the application would hold it under: resolved, and spelled without
/// the prefix Windows resolves a path with.
fn shipped_pack() -> PathBuf {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK);
    plain_path(&pack.canonicalize().unwrap_or_else(|error| panic!("the shipped skin pack is not at {}: {error}", pack.display())))
}

/// A pack folder of one test's own holding `files`, each a name and what is written under it.
fn pack_of(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let pack = std::env::temp_dir().join(format!("rbms-skin-preview-pack-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&pack);
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    for (name, source) in files {
        std::fs::write(pack.join(name), source).expect("the skin is written");
    }
    pack
}

/// Save a frame as `<capture folder>/<name>.<extension>`, when a capture folder was named.
fn save(name: &str, extension: &str, size: (u32, u32), rgba: &[u8]) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV).map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&folder).expect("the capture folder can be created");
    image::save_buffer(folder.join(format!("{name}.{extension}")), rgba, size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
}

/// The rectangle [`RECT`] covers on a target `enlargement` times [`UI_SIZE`], as whole pixels:
/// `(left, top, right, bottom)` with the far edges exclusive.
fn rect_pixels(enlargement: f32) -> (u32, u32, u32, u32) {
    let edge = |value: f32| (value * enlargement).round() as u32;
    (edge(RECT.x), edge(RECT.y), edge(RECT.x + RECT.w), edge(RECT.y + RECT.h))
}

/// How many pixels of `rgba` outside [`RECT`] are further than `tolerance` from the menu's clear
/// colour, and how many inside it are.
fn drawn_on(rgba: &[u8], size: (u32, u32), enlargement: f32, tolerance: u8) -> (usize, usize) {
    let (left, top, right, bottom) = rect_pixels(enlargement);
    let backdrop = [BACKDROP.r, BACKDROP.g, BACKDROP.b];
    let (mut outside, mut inside) = (0, 0);
    for (index, pixel) in rgba.chunks_exact(4).enumerate() {
        let (x, y) = (index as u32 % size.0, index as u32 / size.0);
        if pixel.iter().zip(backdrop).all(|(channel, clear)| channel.abs_diff(clear) <= tolerance) {
            continue;
        }
        if (left..right).contains(&x) && (top..bottom).contains(&y) {
            inside += 1;
        } else {
            outside += 1;
        }
    }
    (outside, inside)
}

/// A menu's worth of what a preview needs: the application, the preview, and the one canvas every
/// frame of it is drawn on -- a compiled document's textures belong to the target it was compiled
/// against.
struct Bench {
    app: App,
    preview: SkinPreview,
    pixels: HeadlessCanvas,
    size: (u32, u32),
}

impl Bench {
    fn new(tag: &str, pack: Option<&Path>) -> Bench {
        Bench::sized(tag, pack, UI_SIZE)
    }

    fn sized(tag: &str, pack: Option<&Path>, size: (u32, u32)) -> Bench {
        Bench { app: app_with(tag, pack), preview: SkinPreview::new(), pixels: HeadlessCanvas::new(size.0, size.1), size }
    }

    /// Bring the preview up to date `seconds` on, without drawing.
    fn update(&mut self, seconds: f32) {
        self.preview.update(&mut FrameCtx { shared: &mut self.app.shared, now: Instant::now(), dt: seconds });
    }

    /// One frame as the menu makes it, `seconds` after the one before: what the preview said of it.
    fn frame(&mut self, seconds: f32) -> PreviewStatus {
        self.update(seconds);
        let mut canvas = Canvas::Headless(&mut self.pixels);
        canvas.clear(BACKDROP);
        let mut ctx = FrameCtx { shared: &mut self.app.shared, now: Instant::now(), dt: seconds };
        let status = self.preview.draw(&mut ctx, &mut canvas, RECT).clone();
        self.app.shared.finish_skin_frame(&mut canvas);
        status
    }

    /// Frames that move the scene by nothing until one says something other than that the document
    /// is on its way.
    fn settle(&mut self) -> PreviewStatus {
        loop {
            let status = self.frame(NO_TIME);
            if status != PreviewStatus::Loading {
                return status;
            }
            std::thread::sleep(WORKER_PAUSE);
        }
    }

    /// Put the preview on `screen` and draw it until it has said what is there.
    fn show(&mut self, screen: i32) -> PreviewStatus {
        self.app.shared.config.skin.screen = screen;
        self.app.shared.reload_skin();
        self.preview.settings_changed(&mut self.app.shared);
        self.settle()
    }

    /// Move the scene on to `millis` of its own time without drawing, then draw it there. A scene
    /// that stops being drawn on the way -- it ended and began again -- is drawn where it stands.
    fn frame_at(&mut self, millis: i64) -> PreviewStatus {
        let target_us = millis * MICROS_PER_MILLI;
        while self.preview.scene_us() < target_us && self.preview.status().is_drawn() {
            let left = (target_us - self.preview.scene_us()) as f32 / MICROS_PER_SECOND as f32;
            let before = self.preview.scene_us();
            self.update(left.min(STEP_SECONDS));
            if self.preview.scene_us() <= before {
                break;
            }
        }
        self.frame(NO_TIME)
    }

    fn save(&self, name: &str) {
        save(name, "png", self.size, self.pixels.rgba());
    }
}

#[test]
fn a_screen_with_no_document_says_so_and_draws_nothing() {
    let mut bench = Bench::new("no-document", None);
    for screen in [SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_SKIN_SELECT] {
        bench.app.shared.config.skin.screen = screen;
        let status = bench.frame(NO_TIME);
        assert_eq!(status, PreviewStatus::Absent(PreviewAbsence::NoDocument), "screen {screen}");
        assert_eq!(bench.pixels.quad_count(), 0, "a preview that says nothing is there drew on screen {screen}");
    }
}

/// The shipped pack has no document for the key configuration, so that screen reads as one with no
/// document rather than as one with no preview.
#[test]
fn a_screen_the_pack_has_no_document_for_reads_as_having_none() {
    let mut bench = Bench::new("pack-gap", Some(&shipped_pack()));
    assert_eq!(bench.show(SKIN_TYPE_KEY_CONFIG), PreviewStatus::Absent(PreviewAbsence::NoDocument));
    assert_eq!(bench.pixels.quad_count(), 0);
}

#[test]
fn every_status_but_a_drawn_frame_has_a_line_to_show() {
    let statuses = [
        PreviewStatus::Loading,
        PreviewStatus::Absent(PreviewAbsence::NoDocument),
        PreviewStatus::Absent(PreviewAbsence::UnsupportedScreen),
        PreviewStatus::Absent(PreviewAbsence::NoPreview),
        PreviewStatus::Failed("the document names a file that is not there".to_owned()),
        PreviewStatus::PackTrouble("a skin of the pack did not read".to_owned()),
    ];
    for status in &statuses {
        assert!(!status.is_drawn());
        assert!(status.message().is_some_and(|message| !message.is_empty()), "{status:?}");
    }
    assert_eq!(statuses[4].detail(), Some("the document names a file that is not there"), "a failure's reason is what is written under it");
    assert_eq!(statuses[5].detail(), Some("a skin of the pack did not read"), "what is wrong with the pack is what is written under it");
    assert!(PreviewStatus::Drawn.is_drawn());
    assert_eq!(PreviewStatus::Drawn.message(), None);
    assert_eq!(PreviewStatus::Drawn.detail(), None);
}

#[test]
fn a_released_preview_begins_again_when_it_is_next_brought_up_to_date() {
    let mut bench = Bench::new("released", None);
    bench.app.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    bench.frame(NO_TIME);
    bench.preview.release(&mut bench.app.shared);
    bench.preview.release(&mut bench.app.shared);
    assert_eq!(*bench.preview.status(), PreviewStatus::Loading);
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Absent(PreviewAbsence::NoDocument));
}

/// A preview released before it was ever brought up to date has nothing to let go of.
#[test]
fn a_preview_that_never_drew_is_released_without_a_word() {
    let mut bench = Bench::new("never-drew", Some(&shipped_pack()));
    bench.preview.release(&mut bench.app.shared);
    assert_eq!(*bench.preview.status(), PreviewStatus::Loading);
    assert_eq!(bench.app.shared.skin_screens.texture_stats().count, 0);
}

/// A Lua skin whose program does not parse, so its header never answers and no screen is drawn with
/// it.
const UNPARSED: &str = "local skin = { type = 6, name = 'Unparsed' w = 1280 \nreturn skin +";

/// A skin of the pack that cannot be read far enough to say which screen it draws is named where the
/// preview would be, with why: nothing else on the menu would say it is there and broken. Once the
/// file reads and the folder is walked again, the screen it declares is drawn with it.
#[test]
fn a_skin_of_the_pack_that_does_not_parse_is_named_in_the_previews_place() {
    let pack = decide_pack("unparsed", UNPARSED);
    let mut bench = Bench::new("unparsed", Some(&pack));
    let status = bench.show(SKIN_TYPE_DECIDE);
    assert!(!status.is_drawn(), "{status:?}");
    let detail = status.detail().unwrap_or_default().to_owned();
    assert!(detail.contains("decide.luaskin"), "the file that did not read is not named: {status:?}");
    assert!(!detail.chars().any(char::is_control), "{detail:?}");
    assert_eq!(bench.pixels.quad_count(), 0);
    let loaded = bench.app.shared.setting_line(SettingId::SkinInfo).1;
    assert!(loaded.contains("decide.luaskin"), "the LOADED row says nothing of the skin that did not read: {loaded}");
    assert_eq!(bench.frame(STEP_SECONDS), status, "what is said of the pack moved with nothing changed");

    std::fs::write(pack.join("decide.luaskin"), PLAIN_DECIDE).expect("the skin is written");
    bench.app.shared.rescan_skins_afresh();
    bench.app.shared.reload_skin();
    bench.preview.settings_changed(&mut bench.app.shared);
    assert_eq!(bench.settle(), PreviewStatus::Drawn, "{:?}", bench.app.shared.skin_failure(SKIN_TYPE_DECIDE));
}

/// A pack folder that is not on the disk is said to be missing, in the preview's place and on the
/// row that names it.
#[test]
fn a_pack_folder_that_is_not_there_is_said_to_be_missing() {
    let missing = std::env::temp_dir().join(format!("rbms-skin-preview-no-such-pack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&missing);
    let mut bench = Bench::new("no-pack", None);
    bench.app.shared.set_skin_pack(Some(missing.to_string_lossy().into_owned()));
    let status = bench.show(SKIN_TYPE_DECIDE);
    assert!(status.detail().is_some_and(|detail| detail.contains("PACK FOLDER")), "{status:?}");
    assert_eq!(bench.pixels.quad_count(), 0);
    let row = bench.app.shared.setting_line(SettingId::SkinPack).1;
    assert!(row.starts_with("(MISSING)"), "the PACK FOLDER row does not say its folder is missing: {row}");
    let loaded = bench.app.shared.setting_line(SettingId::SkinInfo).1;
    assert!(loaded.contains("PACK FOLDER"), "the LOADED row: {loaded}");

    std::fs::create_dir_all(&missing).expect("the folder can be made");
    bench.app.shared.rescan_skins_afresh();
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Absent(PreviewAbsence::NoDocument), "a folder that is there and empty");
    assert!(!bench.app.shared.setting_line(SettingId::SkinPack).1.starts_with("(MISSING)"));
}

/// Draw the decide screen the way its own screen does until its document is compiled, and answer
/// which read of the document that was.
fn decide_screen_drawn_for_real(bench: &mut Bench) -> Option<u64> {
    let chart = ChartMeta::default();
    loop {
        let mut canvas = Canvas::Headless(&mut bench.pixels);
        bench.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_DECIDE);
        let drawn = bench.app.shared.draw_decide_skin(&mut canvas, &DecideDraw { chart: &chart, progress: 0.0, data: FrameData::default() });
        bench.app.shared.finish_skin_frame(&mut canvas);
        if drawn && bench.app.shared.has_compiled_skin(SKIN_TYPE_DECIDE) {
            return bench.app.shared.skins.build_of(SKIN_TYPE_DECIDE);
        }
        std::thread::sleep(WORKER_PAUSE);
    }
}

/// The screen the menu was opened over keeps its compiled document and the read it was compiled
/// from while another screen is previewed, so going back to it draws it on the first frame -- as
/// going back from the settings screen does.
#[test]
fn the_screen_under_the_menu_is_kept_while_another_is_previewed() {
    let mut bench = Bench::new("parked", Some(&shipped_pack()));
    let build = decide_screen_drawn_for_real(&mut bench);
    let underneath = bench.app.shared.suspend_skin_scene();

    assert_eq!(bench.show(SKIN_TYPE_PLAY_7KEYS), PreviewStatus::Drawn);
    assert_eq!(bench.frame(STEP_SECONDS), PreviewStatus::Drawn);
    assert!(bench.app.shared.has_compiled_skin(SKIN_TYPE_DECIDE), "the screen under the menu was let go of");
    assert!(
        !bench.app.shared.skins.needs_reload_for(&bench.app.shared.config, SKIN_TYPE_DECIDE),
        "the skin of the screen under the menu is due to be read again"
    );

    bench.preview.release(&mut bench.app.shared);
    bench.app.shared.resume_skin_scene(underneath);
    let chart = ChartMeta::default();
    let mut canvas = Canvas::Headless(&mut bench.pixels);
    bench.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_DECIDE);
    assert_eq!(bench.app.shared.skins.build_of(SKIN_TYPE_DECIDE), build, "the screen gone back to was read again");
    assert!(
        bench.app.shared.draw_decide_skin(&mut canvas, &DecideDraw { chart: &chart, progress: 0.0, data: FrameData::default() }),
        "the first frame back drew no skin"
    );
    bench.app.shared.finish_skin_frame(&mut canvas);
    assert!(!bench.app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS), "the previewed screen outlived the menu");
    assert!(
        bench.app.shared.skins.needs_reload_for(&bench.app.shared.config, SKIN_TYPE_PLAY_7KEYS),
        "the skin read for the preview was left for the play screen"
    );
}

/// A screen under the menu that was itself previewed was read against sample state: it is read
/// again by its own screen, whichever screen the preview was on when the menu was left.
#[test]
fn the_screen_under_the_menu_is_read_again_once_it_was_previewed_itself() {
    let mut bench = Bench::new("parked-previewed", Some(&shipped_pack()));
    let build = decide_screen_drawn_for_real(&mut bench);
    let underneath = bench.app.shared.suspend_skin_scene();

    assert_eq!(bench.show(SKIN_TYPE_DECIDE), PreviewStatus::Drawn);
    assert_ne!(bench.app.shared.skins.build_of(SKIN_TYPE_DECIDE), build, "the preview drew the read made for the screen under the menu");
    assert_eq!(bench.show(SKIN_TYPE_PLAY_7KEYS), PreviewStatus::Drawn);

    bench.preview.release(&mut bench.app.shared);
    bench.app.shared.resume_skin_scene(underneath);
    assert!(bench.app.shared.skins.needs_reload_for(&bench.app.shared.config, SKIN_TYPE_DECIDE), "a skin read for the preview was left for the decide screen");
}

/// The first frame of a document says it is on its way and reads nothing: the frame that reads a
/// Lua skin runs the whole skin, and the menu has to have said so before it does.
#[test]
fn the_first_frame_of_a_document_reads_nothing() {
    let mut bench = Bench::new("announced", Some(&shipped_pack()));
    bench.app.shared.config.skin.screen = SKIN_TYPE_DECIDE;
    bench.app.shared.reload_skin();
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Loading);
    assert_eq!(bench.pixels.quad_count(), 0, "a frame that said the document is on its way drew");
    assert!(bench.app.shared.skins.is_waiting(SKIN_TYPE_DECIDE), "the skin was read on the frame that only says it is coming");
    assert!(bench.app.shared.skins.document(SKIN_TYPE_DECIDE).is_none());
    assert_eq!(bench.settle(), PreviewStatus::Drawn);
}

/// Every screen the shipped pack draws is previewed: a frame lands in the rectangle and nowhere
/// else, and the scene is not standing still.
#[test]
fn the_shipped_pack_is_previewed_on_every_screen_it_draws() {
    let pack = shipped_pack();
    for (screen, name, moments) in SHIPPED_SCREENS {
        let mut bench = Bench::new(&format!("shipped-{name}"), Some(&pack));
        let failure = |bench: &Bench| bench.app.shared.skin_failure(screen).map(str::to_owned);
        assert_eq!(bench.show(screen), PreviewStatus::Drawn, "{name} did not draw: {:?}", failure(&bench));
        assert_eq!(bench.app.shared.skin_warnings(screen), &[] as &[String], "{name} did not compile whole");
        let mut frames = Vec::new();
        for millis in moments {
            assert_eq!(bench.frame_at(*millis), PreviewStatus::Drawn, "{name} stopped drawing at {millis} ms");
            assert!(bench.pixels.quad_count() > 0, "{name} drew nothing at {millis} ms");
            let (outside, inside) = drawn_on(bench.pixels.rgba(), bench.size, SAME_SIZE, 0);
            assert_eq!(outside, 0, "{name} drew on {outside} pixels outside its rectangle at {millis} ms");
            let (left, top, right, bottom) = rect_pixels(SAME_SIZE);
            assert_eq!(inside, ((right - left) * (bottom - top)) as usize, "{name} left part of its rectangle as the menu cleared it at {millis} ms");
            bench.save(&format!("preview-{name}-{millis}"));
            frames.push(bench.pixels.pixel_checksum());
        }
        assert!(frames.windows(2).all(|pair| pair[0] != pair[1]), "{name} drew the same frame at two moments of its scene: {frames:?}");
    }
}

/// On a larger target the frame is laid out for the pixels its rectangle really covers there, and
/// still reaches no further.
#[test]
fn a_preview_fills_its_rectangle_and_nothing_else_on_a_larger_target() {
    let pack = shipped_pack();
    for (screen, name, millis) in [(SKIN_TYPE_MUSIC_SELECT, "select", 2_000), (SKIN_TYPE_PLAY_7KEYS, "play7", 9_000)] {
        let mut bench = Bench::sized(&format!("large-{name}"), Some(&pack), LARGE);
        assert_eq!(bench.show(screen), PreviewStatus::Drawn, "{name}");
        assert_eq!(bench.frame_at(millis), PreviewStatus::Drawn, "{name}");
        let (outside, inside) = drawn_on(bench.pixels.rgba(), bench.size, LARGE_ENLARGEMENT, 0);
        let (left, top, right, bottom) = rect_pixels(LARGE_ENLARGEMENT);
        assert_eq!(outside, 0, "{name} drew outside its rectangle on a {LARGE:?} target");
        assert_eq!(inside, ((right - left) * (bottom - top)) as usize, "{name} did not fill its rectangle on a {LARGE:?} target");
        bench.save(&format!("preview-{name}-{millis}-{}x{}", LARGE.0, LARGE.1));
    }
}

/// The GPU draws a preview where the headless canvas does. A machine with no adapter has nothing to
/// draw on and passes.
#[test]
fn a_preview_drawn_on_the_gpu_stays_inside_its_rectangle() {
    let pack = shipped_pack();
    for (screen, name, millis) in [(SKIN_TYPE_MUSIC_SELECT, "select", 2_000), (SKIN_TYPE_PLAY_7KEYS, "play7", 9_000)] {
        let Some(mut gpu) = Gpu::offscreen_if_available(UI_SIZE.0, UI_SIZE.1, UI_SIZE) else {
            return;
        };
        let mut app = app_with(&format!("gpu-{name}"), Some(&pack));
        let mut preview = SkinPreview::new();
        app.shared.config.skin.screen = screen;
        app.shared.reload_skin();
        let mut frame = |app: &mut App, preview: &mut SkinPreview, seconds: f32, draws: bool| {
            let mut ctx = FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: seconds };
            preview.update(&mut ctx);
            if !draws {
                return preview.status().clone();
            }
            let mut canvas = Canvas::Window(&mut gpu);
            canvas.clear(BACKDROP);
            let status = preview.draw(&mut ctx, &mut canvas, RECT).clone();
            app.shared.finish_skin_frame(&mut canvas);
            status
        };
        while frame(&mut app, &mut preview, NO_TIME, true) == PreviewStatus::Loading {
            std::thread::sleep(WORKER_PAUSE);
        }
        while preview.scene_us() < millis * MICROS_PER_MILLI && preview.status().is_drawn() {
            frame(&mut app, &mut preview, STEP_SECONDS, false);
        }
        assert_eq!(frame(&mut app, &mut preview, NO_TIME, true), PreviewStatus::Drawn, "{name}");
        let drawn = gpu.capture().expect("an offscreen target reads back");
        save(&format!("preview-{name}-{millis}"), "gpu.png", UI_SIZE, &drawn);
        let (outside, inside) = drawn_on(&drawn, UI_SIZE, SAME_SIZE, GPU_TOLERANCE);
        assert_eq!(outside, 0, "{name} drew outside its rectangle on the GPU");
        assert!(inside > 0, "{name} drew nothing on the GPU");
    }
}

/// A sample run loads, stands ready, plays its chart with its figures moving, closes, and begins
/// again.
#[test]
fn a_sample_run_plays_its_chart_and_begins_again_when_it_ends() {
    const WHILE_LOADING_MS: i64 = 500;
    const EARLY_IN_THE_CHART_MS: i64 = 6_000;
    const LATER_IN_THE_CHART_MS: i64 = 16_000;
    const LONGER_THAN_THE_SCENE_MS: i64 = 60_000;
    let mut bench = Bench::new("run", Some(&shipped_pack()));
    assert_eq!(bench.show(SKIN_TYPE_PLAY_7KEYS), PreviewStatus::Drawn);
    let standing = |bench: &Bench| bench.preview.play_sample().expect("a play screen is previewed from a sample run").standing();

    assert_eq!(bench.frame_at(WHILE_LOADING_MS), PreviewStatus::Drawn);
    assert_eq!(standing(&bench).0, PlayPhase::Preload);

    assert_eq!(bench.frame_at(EARLY_IN_THE_CHART_MS), PreviewStatus::Drawn);
    let (phase, early_ms, early_combo, early_score, early_gauge) = standing(&bench);
    assert_eq!(phase, PlayPhase::Play);
    assert!(early_ms > 0 && early_combo > 0 && early_score > 0, "nothing was played {EARLY_IN_THE_CHART_MS} ms in: {:?}", standing(&bench));

    assert_eq!(bench.frame_at(LATER_IN_THE_CHART_MS), PreviewStatus::Drawn);
    let (phase, later_ms, later_combo, later_score, later_gauge) = standing(&bench);
    assert_eq!(phase, PlayPhase::Play);
    assert!(later_ms > early_ms && later_combo > early_combo && later_score > early_score, "the run stood still: {:?}", standing(&bench));
    assert!(later_gauge != early_gauge, "the gauge stood at {early_gauge} from {early_ms} ms to {later_ms} ms");

    bench.frame_at(LONGER_THAN_THE_SCENE_MS);
    assert!(bench.preview.scene_us() < LONGER_THAN_THE_SCENE_MS * MICROS_PER_MILLI, "the scene ran on past its end");
    assert_eq!(bench.settle(), PreviewStatus::Drawn, "the scene did not come back after it began again");
    let (phase, chart_ms, combo, score, _) = standing(&bench);
    assert_eq!((phase, chart_ms, combo, score), (PlayPhase::Preload, 0, 0, 0), "the run that began again is not a new one");
    assert!(bench.app.shared.audio.is_none(), "a sample run opened the output stream");
}

/// The cursor of the sample list moves down a bar at a time and the frame moves with it.
#[test]
fn the_sample_list_turns_under_its_cursor() {
    const BEFORE_THE_FIRST_STEP_MS: i64 = 1_000;
    const AFTER_THE_FIRST_STEP_MS: i64 = 2_500;
    let mut bench = Bench::new("list", Some(&shipped_pack()));
    assert_eq!(bench.show(SKIN_TYPE_MUSIC_SELECT), PreviewStatus::Drawn);
    let cursor = |bench: &Bench| bench.preview.select_sample().expect("the browser is previewed from a sample list").cursor();
    assert_eq!(bench.frame_at(BEFORE_THE_FIRST_STEP_MS), PreviewStatus::Drawn);
    let (first, first_frame) = (cursor(&bench), bench.pixels.pixel_checksum());
    assert_eq!(bench.frame_at(AFTER_THE_FIRST_STEP_MS), PreviewStatus::Drawn);
    assert_eq!(cursor(&bench), first + 1);
    assert_ne!(bench.pixels.pixel_checksum(), first_frame, "the frame did not move with the cursor");
}

/// A scene waiting for its document does not move, however many seconds go by.
#[test]
fn a_scene_stands_still_until_its_document_is_drawn() {
    const A_LONG_FRAME: f32 = 5.0;
    let mut bench = Bench::new("held", Some(&shipped_pack()));
    bench.app.shared.config.skin.screen = SKIN_TYPE_RESULT;
    bench.app.shared.reload_skin();
    while bench.frame(A_LONG_FRAME) == PreviewStatus::Loading {
        assert_eq!(bench.preview.scene_us(), 0, "the scene moved while its document was on its way");
        std::thread::sleep(WORKER_PAUSE);
    }
    assert_eq!(*bench.preview.status(), PreviewStatus::Drawn);
    assert_eq!(bench.preview.scene_us(), 0, "the first frame of the document is not the scene's first moment");
    bench.frame(A_LONG_FRAME);
    let moved = bench.preview.scene_us();
    assert!(moved > 0 && moved <= (LONGEST_STEP_SECONDS * MICROS_PER_SECOND as f32) as i64, "one frame moved the scene by {moved} us");
}

/// A skin that tells the game things on every frame, and while it is read: an event of the skin
/// menu's own, and a sound.
const TALKATIVE_DECIDE: &str = r#"
local main_state = require("main_state")
local skin = { type = 6, name = "Talkative", w = 1280, h = 720, scene = 3000, fadeout = 500 }
if skin_config then
    main_state.event_exec(14)
    main_state.audio_play("tick.wav", 1)
    skin.source = { { id = 0, path = "panel.png" } }
    skin.image = { { id = "panel", src = 0, x = 0, y = 0, w = 8, h = 8 } }
    skin.destination = {
        { id = "panel", dst = { { x = 0, y = 0, w = 1280, h = 720 } }, draw = function()
            main_state.event_exec(14)
            main_state.audio_play("tick.wav", 1)
            return true
        end },
    }
end
return skin
"#;

/// The edge of the one image the fixture skins draw from.
const PANEL_EDGE: u32 = 8;

/// A pack of one decide skin with the image it draws from.
fn decide_pack(tag: &str, source: &str) -> PathBuf {
    let pack = pack_of(tag, &[("decide.luaskin", source)]);
    image::RgbaImage::from_pixel(PANEL_EDGE, PANEL_EDGE, image::Rgba([u8::MAX; 4])).save(pack.join("panel.png")).expect("the sheet is written");
    pack
}

/// What a previewed skin tells the game goes nowhere: nothing waits for the end of the frame, no
/// request reaches a screen and no sound is asked for. The same skin drawn as the decide screen
/// draws it does leave its calls, which is what shows the fixture talks at all.
#[test]
fn a_previewed_skin_is_not_heard_and_asks_nothing_of_the_game() {
    let pack = decide_pack("talkative", TALKATIVE_DECIDE);
    let mut bench = Bench::new("talkative", Some(&pack));
    bench.app.shared.config.skin.screen = SKIN_TYPE_DECIDE;
    bench.app.shared.reload_skin();
    loop {
        bench.update(NO_TIME);
        let mut canvas = Canvas::Headless(&mut bench.pixels);
        canvas.clear(BACKDROP);
        let mut ctx = FrameCtx { shared: &mut bench.app.shared, now: Instant::now(), dt: NO_TIME };
        let status = bench.preview.draw(&mut ctx, &mut canvas, RECT).clone();
        assert_eq!(bench.app.shared.skin_calls_waiting(), 0, "a previewed skin's calls were kept for the end of the frame");
        bench.app.shared.finish_skin_frame(&mut canvas);
        assert!(bench.app.shared.skin_requests().is_empty(), "a previewed skin's request reached a screen");
        if status != PreviewStatus::Loading {
            assert_eq!(status, PreviewStatus::Drawn, "{:?}", bench.app.shared.skin_failure(SKIN_TYPE_DECIDE));
            break;
        }
        std::thread::sleep(WORKER_PAUSE);
    }
    assert_eq!(bench.frame(STEP_SECONDS), PreviewStatus::Drawn);
    assert!(bench.app.shared.skin_requests().is_empty());

    let mut canvas = Canvas::Headless(&mut bench.pixels);
    bench.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_DECIDE);
    let chart = ChartMeta::default();
    assert!(bench.app.shared.draw_decide_skin(&mut canvas, &DecideDraw { chart: &chart, progress: 0.0, data: FrameData::default() }));
    assert!(bench.app.shared.skin_calls_waiting() > 0, "the fixture skin told the game nothing when the decide screen drew it");
}

/// A Lua skin whose header answers and whose body gives up, and what it says as it does.
const GIVES_UP: &str =
    "local skin = { type = 6, name = 'Gives up', w = 1280, h = 720 }\nif skin_config then\n    error('the sample skin gives up')\nend\nreturn skin\n";
const GIVES_UP_REASON: &str = "the sample skin gives up";

/// A Lua skin whose header answers and whose body never ends, and what the loader says of a skin it
/// cut off.
const ENDLESS: &str = "if skin_config then\n    while true do end\nend\nreturn { type = 6, name = 'Endless', w = 1280, h = 720 }\n";
const BUDGET_REASON: &str = "exceeded its evaluation budget";

/// A decide skin that draws one image over the whole screen.
const PLAIN_DECIDE: &str = r#"
local skin = { type = 6, name = "Plain", w = 1280, h = 720, scene = 3000, fadeout = 500 }
if skin_config then
    skin.source = { { id = 0, path = "panel.png" } }
    skin.image = { { id = "panel", src = 0, x = 0, y = 0, w = 8, h = 8 } }
    skin.destination = { { id = "panel", dst = { { x = 0, y = 0, w = 1280, h = 720 } } } }
end
return skin
"#;

/// How many frames a failure is held to after it was first said.
const FRAMES_AFTER_A_FAILURE: usize = 5;

/// A skin that cannot be read says why in the preview's place, draws nothing, and is not read again
/// on the frames that follow -- not even once its file is one that would read. Asking for it again
/// reads it.
#[test]
fn a_skin_that_cannot_be_read_says_why_once_and_is_read_again_only_when_asked() {
    let pack = decide_pack("broken", GIVES_UP);
    let mut bench = Bench::new("broken", Some(&pack));
    let failed = bench.show(SKIN_TYPE_DECIDE);
    let PreviewStatus::Failed(reason) = &failed else {
        panic!("a skin that gives up was answered with {failed:?}");
    };
    assert!(reason.contains(GIVES_UP_REASON), "{reason}");
    assert!(reason.starts_with("decide.luaskin"), "the reason does not begin with the file that failed: {reason}");
    assert!(!reason.contains(&*pack.to_string_lossy()), "the reason spends its line on the folder the pack is in: {reason}");
    assert!(!reason.chars().any(char::is_control), "the reason carries its traceback: {reason:?}");
    assert_eq!(failed.detail(), Some(reason.as_str()));
    let loaded = bench.app.shared.setting_line(SettingId::SkinInfo).1;
    assert!(loaded.contains(GIVES_UP_REASON) && !loaded.chars().any(char::is_control), "the LOADED row: {loaded:?}");
    assert_eq!(bench.pixels.quad_count(), 0, "a preview that failed drew");

    std::fs::write(pack.join("decide.luaskin"), PLAIN_DECIDE).expect("the skin is written");
    for _ in 0..FRAMES_AFTER_A_FAILURE {
        assert_eq!(bench.frame(STEP_SECONDS), failed, "a skin that failed was read again without being asked for");
        assert_eq!(bench.pixels.quad_count(), 0);
    }

    bench.app.shared.rescan_skins_afresh();
    bench.app.shared.reload_skin();
    bench.preview.settings_changed(&mut bench.app.shared);
    assert_eq!(*bench.preview.status(), PreviewStatus::Loading);
    assert_eq!(bench.settle(), PreviewStatus::Drawn, "{:?}", bench.app.shared.skin_failure(SKIN_TYPE_DECIDE));
}

/// What is said once about a skin the preview could not read does not speak of a built-in screen
/// drawn in its place: in a preview nothing is, and the rectangle says so itself.
#[test]
fn a_failure_in_the_preview_is_not_said_to_leave_the_built_in_screen_drawing() {
    crate::notify::exclusive(|| {
        let pack = decide_pack("said", GIVES_UP);
        let mut bench = Bench::new("said", Some(&pack));
        assert!(matches!(bench.show(SKIN_TYPE_DECIDE), PreviewStatus::Failed(_)));
        bench.frame(STEP_SECONDS);
        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        let about: Vec<String> = said.into_iter().map(|(_, message)| message).filter(|message| message.contains("decide.luaskin")).collect();
        assert_eq!(about.len(), 1, "the failure was not said exactly once: {about:?}");
        assert!(about[0].starts_with("skin decide.luaskin did not load: ") && about[0].contains(GIVES_UP_REASON), "{:?}", about[0]);
        assert!(!about[0].contains("built-in") && !about[0].contains('\n'), "{:?}", about[0]);
    });
}

/// A skin that never finishes loading is cut off by the loader's budget, and the preview says so
/// rather than standing still with it.
#[test]
fn a_skin_that_overruns_its_budget_says_so() {
    let pack = decide_pack("endless", ENDLESS);
    let mut bench = Bench::new("endless", Some(&pack));
    let failed = bench.show(SKIN_TYPE_DECIDE);
    assert!(matches!(&failed, PreviewStatus::Failed(reason) if reason.contains(BUDGET_REASON)), "{failed:?}");
    assert_eq!(bench.frame(STEP_SECONDS), failed);
}

/// Moving to another screen lets go of the one before it: its compiled document and the textures
/// only it drew from.
#[test]
fn changing_the_screen_lets_go_of_the_one_before() {
    let mut bench = Bench::new("changed", Some(&shipped_pack()));
    assert_eq!(bench.show(SKIN_TYPE_MUSIC_SELECT), PreviewStatus::Drawn);
    assert!(bench.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT));

    assert_eq!(bench.show(SKIN_TYPE_DECIDE), PreviewStatus::Drawn);
    assert!(!bench.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT), "the browser's document stayed compiled under the decide screen's");
    assert!(bench.app.shared.has_compiled_skin(SKIN_TYPE_DECIDE));
    assert_eq!(bench.preview.scene_us(), 0, "the scene of the screen arrived at did not begin at its first moment");

    assert_eq!(bench.show(SKIN_TYPE_KEY_CONFIG), PreviewStatus::Absent(PreviewAbsence::NoDocument));
    bench.frame(NO_TIME);
    assert!(!bench.app.shared.has_compiled_skin(SKIN_TYPE_DECIDE), "a screen with nothing to draw kept the one before it compiled");
    assert_eq!(bench.app.shared.skin_screens.texture_stats().count, 0, "textures stayed uploaded with no screen to draw them");
}

/// Leaving the menu leaves nothing of the preview uploaded or playing by the end of the frame that
/// follows, and the Lua skin it read is due to be read again by the screen it really draws.
#[test]
fn a_released_preview_leaves_nothing_behind() {
    let mut bench = Bench::new("left", Some(&shipped_pack()));
    let underneath = bench.app.shared.suspend_skin_scene();
    assert_eq!(bench.show(SKIN_TYPE_MUSIC_SELECT), PreviewStatus::Drawn);
    assert!(bench.app.shared.skin_screens.texture_stats().count > 0, "the preview drew from no texture");
    assert!(!bench.app.shared.skins.needs_reload_for(&bench.app.shared.config, SKIN_TYPE_MUSIC_SELECT));

    bench.preview.release(&mut bench.app.shared);
    bench.app.shared.resume_skin_scene(underneath);
    assert!(bench.preview.select_sample().is_none(), "the sample state outlived the preview");
    assert!(
        bench.app.shared.skins.needs_reload_for(&bench.app.shared.config, SKIN_TYPE_MUSIC_SELECT),
        "the skin read for the preview was left for the browser"
    );

    bench.app.shared.finish_skin_frame(&mut Canvas::Headless(&mut bench.pixels));
    assert!(!bench.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT));
    assert_eq!(bench.app.shared.skin_screens.texture_stats().count, 0, "the preview's textures stayed uploaded");
    assert_eq!(bench.app.shared.skin_screens.movies_playing(), 0);
}

/// The name the player goes by, and the title of a chart in the player's library.
const PLAYERS_NAME: &str = "SOMEBODY REAL";
const PLAYERS_CHART: &str = "A CHART OF THE PLAYER";

/// A library entry for a chart of the player's.
fn players_chart() -> rbms_library::SongEntry {
    rbms_library::SongEntry {
        path: PathBuf::from("players-chart.bms"),
        title: PLAYERS_CHART.to_owned(),
        subtitle: String::new(),
        artist: PLAYERS_NAME.to_owned(),
        genre: String::new(),
        maker: String::new(),
        level: "12".to_owned(),
        difficulty: 4,
        init_bpm: 190.0,
        rank: 2,
        total: 0.0,
        mode: rbms_model::Mode::BEAT_7K,
        md5: "0123456789abcdef0123456789abcdef".to_owned(),
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }
}

/// The frame of the browser's preview at its first moment, drawn by an application that `prepare`
/// has set up.
fn first_browser_frame(tag: &str, prepare: impl FnOnce(&mut App)) -> u64 {
    let mut bench = Bench::new(tag, Some(&shipped_pack()));
    prepare(&mut bench.app);
    assert_eq!(bench.show(SKIN_TYPE_MUSIC_SELECT), PreviewStatus::Drawn);
    bench.pixels.pixel_checksum()
}

/// A preview is the same frame whatever the player's library holds and whatever the player is
/// called. The clock a skin may show is the machine's, so the frame of the library's application is
/// held to the frame an empty one drew before it or the one it drew after.
#[test]
fn the_players_library_and_name_are_no_part_of_a_preview() {
    let empty_before = first_browser_frame("nobody-before", |_| {});
    let with_a_library = first_browser_frame("somebody", |app| {
        app.shared.library = rbms_library::Library::from_songs(vec![players_chart()]);
        app.shared.config.network.player_id = PLAYERS_NAME.to_owned();
        app.shared.rebuild_select_items();
    });
    let empty_after = first_browser_frame("nobody-after", |_| {});
    assert!(with_a_library == empty_before || with_a_library == empty_after, "the preview drew something of the player's own");
}

/// The first row of the document being configured that moves an offset `wanted` picks across or
/// down.
fn offset_row(app: &App, screen: i32, wanted: impl Fn(i32) -> bool) -> Option<SkinRow> {
    let header = app.shared.skins.header_of(&app.shared.config, screen)?;
    let moves = |row: &SkinRow| match row {
        SkinRow::Offset(at, OffsetAxis::X | OffsetAxis::Y) => header.offsets.get(*at).is_some_and(|offset| wanted(offset.id)),
        _ => false,
    };
    app.shared.skins.rows(&app.shared.config).into_iter().find(moves)
}

/// How many steps an offset is nudged by, which is far enough to see.
const NUDGES: i32 = 40;

/// Nudge `row` and tell the preview, as the menu does.
fn nudge(bench: &mut Bench, row: SkinRow) {
    let nudged = (0..NUDGES).filter(|_| bench.app.shared.step_skin_row(row, 1)).count();
    assert!(nudged > 0, "the offset did not move");
    bench.preview.settings_changed(&mut bench.app.shared);
}

/// An offset is drawn with on the very next frame with nothing read again. The offset of the whole
/// screen, which a compiled screen settles once, has the screen compiled again -- and the document
/// is still not read. An option has the document read again. Through all of it the scene carries on
/// from where it stood.
#[test]
fn an_offset_is_drawn_with_at_once_and_an_option_has_the_document_read_again() {
    const INTO_THE_CHART_MS: i64 = 8_000;
    let screen = SKIN_TYPE_PLAY_7KEYS;
    let mut bench = Bench::new("settings", Some(&shipped_pack()));
    assert_eq!(bench.show(screen), PreviewStatus::Drawn);
    assert_eq!(bench.frame_at(INTO_THE_CHART_MS), PreviewStatus::Drawn);
    let (build, scene_us, before) = (bench.app.shared.skins.build_of(screen), bench.preview.scene_us(), bench.pixels.pixel_checksum());
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn);
    assert_eq!(bench.pixels.pixel_checksum(), before, "two frames of one moment of the scene differ, so no frame can be held against another");

    let part = offset_row(&bench.app, screen, |id| id != OFFSET_ALL).expect("a play skin lets a part of it be nudged");
    nudge(&mut bench, part);
    assert_eq!(*bench.preview.status(), PreviewStatus::Drawn, "a nudge of an offset stopped the preview");
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn);
    assert_eq!(bench.app.shared.skins.build_of(screen), build, "a nudge of an offset had the document read again");

    let whole = offset_row(&bench.app, screen, |id| id == OFFSET_ALL).expect("a play skin lets the whole of it be nudged");
    let unmoved = bench.pixels.pixel_checksum();
    nudge(&mut bench, whole);
    assert_eq!(*bench.preview.status(), PreviewStatus::Drawn, "a nudge of the whole screen stopped the preview");
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn, "the frame after a nudge of the whole screen was not drawn");
    assert_eq!(bench.app.shared.skins.build_of(screen), build, "the offset of the whole screen had the document read again");
    assert_ne!(bench.pixels.pixel_checksum(), unmoved, "the whole screen did not move on the frame after it was nudged");
    bench.save("preview-play7-8000-moved");
    for _ in 0..NUDGES {
        assert!(bench.app.shared.step_skin_row(whole, -1));
        bench.preview.settings_changed(&mut bench.app.shared);
        assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn, "a step of the whole screen cost a frame");
    }
    assert_eq!(bench.pixels.pixel_checksum(), unmoved, "the whole screen did not come back to where it was");

    let rows = bench.app.shared.skins.rows(&bench.app.shared.config);
    let option = rows.into_iter().find(|row| matches!(row, SkinRow::Property(_))).expect("the shipped play skin has an option");
    assert!(bench.app.shared.step_skin_row(option, 1));
    bench.app.shared.reload_skin();
    bench.preview.settings_changed(&mut bench.app.shared);
    assert_eq!(*bench.preview.status(), PreviewStatus::Loading);
    assert_eq!(bench.settle(), PreviewStatus::Drawn, "{:?}", bench.app.shared.skin_failure(screen));
    assert_ne!(bench.app.shared.skins.build_of(screen), build, "an option did not have the document read again");
    assert_eq!(bench.preview.scene_us(), scene_us, "the scene did not carry on from where it stood");
    assert_eq!(bench.preview.play_sample().map(|sample| sample.standing().0), Some(PlayPhase::Play));
}

/// Being told that a setting moved when none did asks nothing of the preview: a document that is on
/// its way still arrives when the preview is told so before every frame, and a frame that is drawn
/// stays drawn.
#[test]
fn a_preview_told_of_nothing_new_goes_on_as_it_was() {
    let mut bench = Bench::new("told-again", Some(&shipped_pack()));
    bench.app.shared.config.skin.screen = SKIN_TYPE_DECIDE;
    bench.app.shared.reload_skin();
    loop {
        bench.preview.settings_changed(&mut bench.app.shared);
        if bench.frame(NO_TIME) != PreviewStatus::Loading {
            break;
        }
        std::thread::sleep(WORKER_PAUSE);
    }
    assert_eq!(*bench.preview.status(), PreviewStatus::Drawn, "{:?}", bench.app.shared.skin_failure(SKIN_TYPE_DECIDE));
    bench.preview.settings_changed(&mut bench.app.shared);
    assert_eq!(*bench.preview.status(), PreviewStatus::Drawn, "a preview told of nothing new stopped drawing");
    assert_eq!(bench.frame(STEP_SECONDS), PreviewStatus::Drawn);
}

/// With a pack named by the environment, every screen of it is previewed and what the preview says
/// of each is printed: a frame, or why there is none. Without one there is nothing to preview.
#[test]
fn a_skin_pack_named_by_the_environment_is_previewed_one_screen_at_a_time() {
    let Some(pack) = pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    for (screen, name, millis) in PACK_SCREENS {
        let mut bench = Bench::new(&format!("pack-{name}"), Some(&pack));
        let first = bench.show(screen);
        let status = if first.is_drawn() { bench.frame_at(millis) } else { first };
        let (outside, _) = drawn_on(bench.pixels.rgba(), bench.size, SAME_SIZE, 0);
        assert_eq!(outside, 0, "{name} drew outside its rectangle");
        match &status {
            PreviewStatus::Drawn => bench.save(&format!("preview-pack-{name}-{millis}")),
            said => assert_eq!(bench.pixels.quad_count(), 0, "{name} drew and said {said:?}"),
        }
        assert_ne!(status, PreviewStatus::Loading);
        println!("{name}: {status:?} {:?} {:?}", status.message(), status.detail());
    }
}
