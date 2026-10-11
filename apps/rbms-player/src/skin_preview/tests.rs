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
use rbms_render::skin_render::frame::first_lane_rect;
use rbms_render::{Color, FrameData, Rect, Renderer};
use rbms_skin::loader::{
    SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS,
    SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_RESULT, SKIN_TYPE_SKIN_SELECT,
};
use rbms_skin::resolve::plain_path;
use rbms_skin::timer::timer_id;

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

/// A preview made for a play screen shows it whatever screen the skin menu is configuring, takes
/// another tempo up under the scene on show with the run as far through its chart as it was, takes
/// another mode as a scene of its own, and is still of what it was told after it was released.
#[test]
fn a_preview_of_a_play_target_follows_what_it_is_told_and_not_the_settings() {
    const SLOW: PlayTarget = PlayTarget { screen: SKIN_TYPE_PLAY_7KEYS, bpm: 100 };
    const FAST: PlayTarget = PlayTarget { screen: SKIN_TYPE_PLAY_7KEYS, bpm: 200 };
    const WELL_INTO_THE_CHART_MS: i64 = 12_000;
    let mut bench = Bench::new("play-target", Some(&shipped_pack()));
    bench.app.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    let settings = bench.app.shared.config.clone();
    bench.preview = SkinPreview::of_play(SLOW);
    assert_eq!(bench.preview.target(), PreviewTarget::Play(SLOW));
    assert_eq!(bench.settle(), PreviewStatus::Drawn);
    assert!(bench.preview.play_sample().is_some() && bench.preview.select_sample().is_none(), "the preview followed the screen being configured");
    assert_eq!(bench.preview.tempo, SampleTempo::Steady(SLOW.bpm));
    assert_eq!(bench.frame_at(WELL_INTO_THE_CHART_MS), PreviewStatus::Drawn);
    assert!(bench.preview.scene_us() > 0);

    bench.preview.show_play(SLOW);
    bench.update(NO_TIME);
    assert!(bench.preview.scene_us() > 0, "naming the target on show began its scene again");

    let (scene_us, (_, slow_ms, slow_combo, ..)) = (bench.preview.scene_us(), run_of(&bench).standing());
    assert!(slow_combo > 0, "nothing was played {WELL_INTO_THE_CHART_MS} ms in");
    bench.preview.show_play(FAST);
    bench.update(NO_TIME);
    assert_eq!((bench.preview.scene_us(), bench.preview.tempo), (scene_us, SampleTempo::Steady(FAST.bpm)), "another tempo began the scene again");
    assert_eq!(*bench.preview.status(), PreviewStatus::Drawn, "another tempo took the frame away");
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn, "the frame after another tempo was named is not a frame of the skin");
    let (phase, fast_ms, fast_combo, ..) = run_of(&bench).standing();
    assert!((run_of(&bench).session().model().init_bpm - f64::from(FAST.bpm)).abs() < f64::EPSILON, "the run is not on a chart at the tempo named");
    assert_eq!(phase, PlayPhase::Play);
    let twice = FAST.bpm / SLOW.bpm;
    assert!(
        (fast_ms * i64::from(twice)).abs_diff(slow_ms) <= u64::from(twice),
        "the run at twice the tempo is not half as far in: {slow_ms} ms then {fast_ms} ms"
    );
    assert!(fast_combo.abs_diff(slow_combo) <= 1, "the run taken up had a combo of {fast_combo} where the one before had {slow_combo}");

    let five_keys = PlayTarget { screen: SKIN_TYPE_PLAY_5KEYS, bpm: FAST.bpm };
    bench.preview.show_play(five_keys);
    assert_eq!(bench.settle(), PreviewStatus::Drawn);
    assert_eq!(bench.preview.screen, Some(SKIN_TYPE_PLAY_5KEYS));

    bench.preview.release(&mut bench.app.shared);
    assert_eq!(bench.preview.target(), PreviewTarget::Play(five_keys), "a released preview forgot what it shows");
    assert_eq!(bench.settle(), PreviewStatus::Drawn, "a released preview did not begin again on its target");
    assert_eq!(bench.app.shared.config, settings, "a preview of a play target changed a setting");
}

/// A preview asked for a play screen no sample chart is written for, or one the pack has no
/// document for, says so in the words a preview of the skin menu says it in.
#[test]
fn a_play_target_with_nothing_to_draw_says_so() {
    let mut bench = Bench::new("play-target-absent", Some(&shipped_pack()));
    bench.preview = SkinPreview::of_play(PlayTarget { screen: SKIN_TYPE_PLAY_24KEYS, bpm: 150 });
    assert_eq!(bench.settle(), PreviewStatus::Absent(PreviewAbsence::NoDocument));

    let mut bare = Bench::new("play-target-bare", None);
    bare.preview = SkinPreview::of_play(PlayTarget { screen: SKIN_TYPE_PLAY_7KEYS, bpm: 150 });
    assert_eq!(bare.settle(), PreviewStatus::Absent(PreviewAbsence::NoDocument));
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

/// The play screens the play menu previews, each with the name its captures are saved under.
const PLAY_MODES: [(i32, &str); 5] =
    [(SKIN_TYPE_PLAY_7KEYS, "7k"), (SKIN_TYPE_PLAY_5KEYS, "5k"), (SKIN_TYPE_PLAY_14KEYS, "14k"), (SKIN_TYPE_PLAY_10KEYS, "10k"), (SKIN_TYPE_PLAY_9KEYS, "9k")];

/// The target the play menu opens on.
const SEVEN_KEYS: PlayTarget = PlayTarget { screen: SKIN_TYPE_PLAY_7KEYS, bpm: OPENING_BPM };

/// The seven key screen on a sample chart at `bpm`.
fn seven_keys_at(bpm: u32) -> PlayTarget {
    PlayTarget { bpm, ..SEVEN_KEYS }
}

/// The tempos the sample chart is written at where a test compares one with another: a slow one,
/// the one the menu opens on and one twice the slow one.
const SLOW_BPM: u32 = 100;
const OPENING_BPM: u32 = 150;
const FAST_BPM: u32 = 200;
const FAST_OVER_SLOW: f64 = 2.0;

/// The lane cover, the lift and the hidden cover a test switches on, as shares of the lane.
const COVER: f32 = 0.3;
const LIFT: f32 = 0.15;
const HIDDEN: f32 = 0.1;

/// How far a place read off a document may be from where it is expected, in pixels.
const PLACE_TOLERANCE: f32 = 1.0;

const MILLIS_PER_SECOND: f32 = 1_000.0;

/// How many steps of a scene a sample run is given to leave its loading and its ready states.
const STEPS_TO_START: usize = 400;

/// A moment of the sample chart with notes on their way down and a run behind it, in milliseconds
/// of the chart.
const MID_CHART_MS: i64 = 7_000;

/// How far two measured times may be apart and still be the same time, as a share of the larger:
/// a note's place is read to the pixel, of a lane a few hundred high.
const MEASURE_TOLERANCE: f64 = 0.04;

/// How far a row read off a frame may be from where it is worked out to be, in pixels, and how far
/// the first row a lift changes may be: the judgement line is drawn over the top of the lift, a few
/// rows thick.
const ROW_TOLERANCE: f32 = 3.0;
const LIFTED_LINE_TOLERANCE: f32 = 6.0;

/// How bright every channel of a pixel of a white note is at the least, and how many rows the
/// thinnest note covers.
const NOTE_BRIGHTNESS: u8 = 200;
const THINNEST_NOTE_ROWS: u32 = 3;

/// The part of the lane a note is looked for in, as shares of the lane above its judgement line:
/// under a lane cover of three tenths, and over where a skin writes its judgements.
const WATCHED_FROM: f32 = 0.68;
const WATCHED_TO: f32 = 0.38;

/// Where in the lane a note is read the first time and the second, as shares of the lane above its
/// judgement line, and how much of the lane the note before it has to be behind.
const READ_HIGH: f64 = 0.62;
const READ_LOW: f64 = 0.44;
const NOTE_APART: f64 = 0.3;

/// Where the shipped play skin draws its gauge, as a share of the screen from its top.
const GAUGE_ROW: f32 = 940.0 / 1080.0;

/// A bench whose preview is of `target`, drawn until a frame of the shipped pack is on show.
fn play_bench(tag: &str, target: PlayTarget) -> Bench {
    let mut bench = Bench::new(tag, Some(&shipped_pack()));
    bench.preview = SkinPreview::of_play(target);
    assert_eq!(bench.settle(), PreviewStatus::Drawn, "{:?}", bench.app.shared.skin_failure(target.screen));
    bench
}

/// The sample run on show.
fn run_of(bench: &Bench) -> &PlaySample {
    bench.preview.play_sample().expect("a play screen is previewed from a sample run")
}

/// Where the sample chart stands, in milliseconds.
fn chart_ms(bench: &Bench) -> i64 {
    run_of(bench).standing().1
}

/// Tell the preview that a setting moved, as the menu does after it wrote one.
fn told(bench: &mut Bench) {
    bench.preview.settings_changed(&mut bench.app.shared);
}

/// Move the scene on without drawing until the sample chart is playing.
fn start_playing(bench: &mut Bench) {
    for _ in 0..STEPS_TO_START {
        if run_of(bench).standing().0 == PlayPhase::Play {
            return;
        }
        bench.update(STEP_SECONDS);
    }
    panic!("the sample run never began to play: {:?}", run_of(bench).standing());
}

/// Move the scene `millis` on without drawing and draw it there.
fn frame_after(bench: &mut Bench, millis: i64) -> PreviewStatus {
    bench.frame_at(bench.preview.scene_us() / MICROS_PER_MILLI + millis)
}

/// Draw the frame of the pass of the chart that is playing at `millis` of the chart.
fn chart_frame(bench: &mut Bench, millis: i64) -> PreviewStatus {
    start_playing(bench);
    let ahead = millis - chart_ms(bench);
    assert!(ahead >= 0, "the chart is already {} ms in", chart_ms(bench));
    frame_after(bench, ahead)
}

/// One pixel of the last frame.
fn pixel(bench: &Bench, x: u32, y: u32) -> [u8; 3] {
    let at = ((y * bench.size.0 + x) * 4) as usize;
    [bench.pixels.rgba()[at], bench.pixels.rgba()[at + 1], bench.pixels.rgba()[at + 2]]
}

/// Lane zero of a previewed play screen as it lands in the test's rectangle: the column down its
/// middle, how wide it is, and the rows of its top and of its judgement line with no lift under it.
struct LaneSeen {
    column: u32,
    left: f32,
    width: f32,
    ceiling: f32,
    line: f32,
}

impl LaneSeen {
    fn of(bench: &Bench, screen: i32) -> LaneSeen {
        let skin = bench.app.shared.skins.document(screen).expect("the play screen's document is read");
        let lane = first_lane_rect(skin).expect("a play skin places its note field");
        let (across, down) = (RECT.w / skin.def.w as f32, RECT.h / skin.def.h as f32);
        LaneSeen {
            column: (RECT.x + (lane.x + lane.w / 2.0) * across) as u32,
            left: RECT.x + lane.x * across,
            width: lane.w * across,
            ceiling: RECT.y + (skin.def.h as f32 - lane.y - lane.h) * down,
            line: RECT.y + (skin.def.h as f32 - lane.y) * down,
        }
    }

    fn height(&self) -> f32 {
        self.line - self.ceiling
    }

    /// The row `share` of the lane above its judgement line.
    fn row(&self, share: f32) -> f32 {
        self.line - self.height() * share
    }

    /// The colours down the middle of the lane, from its top to its judgement line.
    fn column_of(&self, bench: &Bench) -> Vec<[u8; 3]> {
        (self.ceiling as u32..self.line as u32).map(|row| pixel(bench, self.column, row)).collect()
    }

    /// The row under the lowest note in the watched part of the lane, or `None` with no note there.
    fn note_foot(&self, bench: &Bench) -> Option<u32> {
        let bright = |row: u32| pixel(bench, self.column, row).iter().all(|channel| *channel >= NOTE_BRIGHTNESS);
        let foot = (self.row(WATCHED_FROM) as u32..self.row(WATCHED_TO) as u32).rev().find(|row| bright(*row))?;
        (foot + 1 - THINNEST_NOTE_ROWS..=foot).all(bright).then_some(foot + 1)
    }
}

/// The rows of the lane, counted from its top, that are not the colour they are in `before`: the
/// first and the last of them, or `None` when the lane is as it was.
fn changed_rows(before: &[[u8; 3]], now: &[[u8; 3]]) -> Option<(usize, usize)> {
    let changed = |row: &usize| before[*row] != now[*row];
    Some(((0..before.len()).find(changed)?, (0..before.len()).rev().find(changed)?))
}

/// When each plain note of lane zero of the chart on show is played, in milliseconds.
fn lane_zero_notes_ms(bench: &Bench) -> Vec<f64> {
    let lines = &run_of(bench).session().model().timelines;
    lines.iter().filter(|line| line.notes.first().is_some_and(Option::is_some)).map(|line| line.time_us as f64 / MICROS_PER_MILLI as f64).collect()
}

/// How long a note takes to come down the whole of lane zero as the preview draws it, in
/// milliseconds, read off two frames: one note is found in the lane at two moments of the chart,
/// and the lane's height is divided by how fast it fell between them.
///
/// The note is one with nothing close under it, and the two moments are where the figures of
/// `target` put it at two heights of the lane: a run drawn at another speed has no note there, or
/// has one that fell another distance.
fn measured_fall_ms(bench: &mut Bench, target: PlayTarget) -> f64 {
    start_playing(bench);
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn);
    let play = bench.app.shared.config.play.clone();
    let readout = target.readout(&play);
    let open = 1.0 - if readout.cover_on { f64::from(play.cover) } else { 0.0 };
    let fall_ms = f64::from(readout.duration_ms) / open;
    let lane = LaneSeen::of(bench, target.screen);
    let notes = lane_zero_notes_ms(bench);
    let mut chosen = None;
    for _ in 0..STEPS_TO_START * 2 {
        let now = chart_ms(bench) as f64;
        chosen = (0..notes.len()).find(|at| notes[*at] - READ_HIGH * fall_ms > now && (*at == 0 || notes[*at] - notes[*at - 1] >= NOTE_APART * fall_ms));
        if chosen.is_some() {
            break;
        }
        bench.update(STEP_SECONDS);
    }
    let note_ms = notes[chosen.expect("no note of the sample chart can be told from the one before it")];
    let mut read = |share: f64| {
        let ahead = (note_ms - share * fall_ms) as i64 - chart_ms(bench);
        assert_eq!(frame_after(bench, ahead), PreviewStatus::Drawn);
        let foot = lane.note_foot(bench).unwrap_or_else(|| panic!("no note is {share} of the way up the lane {} ms before it is played", share * fall_ms));
        (chart_ms(bench) as f64, f64::from(foot))
    };
    let (high_ms, high_row) = read(READ_HIGH);
    let (low_ms, low_row) = read(READ_LOW);
    f64::from(lane.height()) * (low_ms - high_ms) / (low_row - high_row)
}

/// Whether two times in milliseconds are the same as far as a frame can tell.
fn same_time(one: f64, other: f64) -> bool {
    (one - other).abs() <= one.max(other) * MEASURE_TOLERANCE
}

/// Every mode the play menu offers is previewed: a frame of its play screen lands in the rectangle
/// and nowhere else, with its chart playing.
#[test]
fn the_play_menus_preview_draws_every_mode_it_offers() {
    for (screen, name) in PLAY_MODES {
        let mut bench = play_bench(&format!("mode-{name}"), PlayTarget { screen, ..SEVEN_KEYS });
        assert_eq!(chart_frame(&mut bench, MID_CHART_MS), PreviewStatus::Drawn, "{name}");
        assert_eq!(bench.app.shared.skin_warnings(screen), &[] as &[String], "{name} did not compile whole");
        let (outside, inside) = drawn_on(bench.pixels.rgba(), bench.size, SAME_SIZE, 0);
        let (left, top, right, bottom) = rect_pixels(SAME_SIZE);
        assert_eq!(outside, 0, "{name} drew outside its rectangle");
        assert_eq!(inside, ((right - left) * (bottom - top)) as usize, "{name} left part of its rectangle as the menu cleared it");
        let (phase, _, combo, ..) = run_of(&bench).standing();
        assert!(phase == PlayPhase::Play && combo > 0, "{name}: {:?}", run_of(&bench).standing());
        bench.save(&format!("play-preview-{name}-default"));
    }
}

/// With a green number held, a note comes down the lane in the same time at every tempo the menu
/// names, which is the time the menu writes beside it, and the speed that holds it differs.
#[test]
fn a_held_green_number_brings_a_note_down_in_the_same_time_at_every_tempo() {
    let mut bench = play_bench("held", seven_keys_at(SLOW_BPM));
    let play = &mut bench.app.shared.config.play;
    (play.keep_green_number, play.enable_cover) = (true, false);
    told(&mut bench);
    let mut speeds = Vec::new();
    for bpm in [SLOW_BPM, OPENING_BPM, FAST_BPM] {
        let target = seven_keys_at(bpm);
        bench.preview.show_play(target);
        let fell = measured_fall_ms(&mut bench, target);
        let readout = target.readout(&bench.app.shared.config.play);
        assert!(readout.held && readout.duration_ms == bench.app.shared.config.play.duration_ms, "{readout:?}");
        assert!(same_time(fell, f64::from(readout.duration_ms)), "at {bpm} a note fell in {fell} ms and the number held is {} ms", readout.duration_ms);
        speeds.push(readout.hispeed);
        bench.save(&format!("play-preview-7k-held-{bpm}"));
    }
    assert!(speeds.windows(2).all(|pair| pair[0] > pair[1]), "the speed that holds the number did not fall as the tempo rose: {speeds:?}");
}

/// With nothing held the speed is the stored one, so a note comes down in half the time at twice
/// the tempo, and in the time the menu writes beside it at each.
#[test]
fn a_stored_speed_brings_a_note_down_faster_at_a_faster_tempo() {
    const STORED_SPEED: f64 = 3.0;
    let mut bench = play_bench("stored", seven_keys_at(SLOW_BPM));
    let play = &mut bench.app.shared.config.play;
    (play.keep_green_number, play.enable_cover, play.hispeed) = (false, false, STORED_SPEED);
    told(&mut bench);
    let mut fell = Vec::new();
    for bpm in [SLOW_BPM, FAST_BPM] {
        let target = seven_keys_at(bpm);
        bench.preview.show_play(target);
        let measured = measured_fall_ms(&mut bench, target);
        let readout = target.readout(&bench.app.shared.config.play);
        assert!(!readout.held && (readout.hispeed - STORED_SPEED).abs() < f64::EPSILON, "{readout:?}");
        assert!(same_time(measured, f64::from(readout.duration_ms)), "at {bpm} a note fell in {measured} ms and its target reads {} ms", readout.duration_ms);
        fell.push(measured);
        bench.save(&format!("play-preview-7k-stored-{bpm}"));
    }
    assert!(same_time(fell[0], fell[1] * FAST_OVER_SLOW), "twice the tempo did not halve the fall: {fell:?}");
}

/// Under CONSTANT the field scrolls at one tempo of its own, so a note comes down in the same time
/// whatever the chart's tempo is, held number or stored speed.
#[test]
fn under_constant_a_note_comes_down_in_the_same_time_at_every_tempo() {
    let mut bench = play_bench("constant", seven_keys_at(SLOW_BPM));
    let play = &mut bench.app.shared.config.play;
    (play.constant_speed, play.enable_cover) = (true, false);
    for keep_green_number in [false, true] {
        bench.app.shared.config.play.keep_green_number = keep_green_number;
        told(&mut bench);
        let mut fell = Vec::new();
        for bpm in [SLOW_BPM, FAST_BPM] {
            let target = seven_keys_at(bpm);
            bench.preview.show_play(target);
            let measured = measured_fall_ms(&mut bench, target);
            let readout = target.readout(&bench.app.shared.config.play);
            assert!(same_time(measured, f64::from(readout.duration_ms)), "held {keep_green_number} at {bpm}: {measured} ms against {}", readout.duration_ms);
            fell.push(measured);
        }
        assert!(same_time(fell[0], fell[1]), "held {keep_green_number}: the tempo moved the fall under CONSTANT: {fell:?}");
    }
    bench.save("play-preview-7k-constant");
}

/// A lane cover leaves a held green number what it was: the note falls slower, and what the cover
/// leaves of the lane is crossed in the time that is held. Under a stored speed the note falls as
/// fast as it did, and is on show for less.
#[test]
fn a_lane_cover_slows_a_held_note_and_leaves_a_stored_speed_alone() {
    let target = SEVEN_KEYS;
    let mut bench = play_bench("covered", target);
    for keep_green_number in [true, false] {
        let play = &mut bench.app.shared.config.play;
        (play.keep_green_number, play.enable_cover, play.cover) = (keep_green_number, false, COVER);
        told(&mut bench);
        let open = measured_fall_ms(&mut bench, target);
        bench.app.shared.config.play.enable_cover = true;
        told(&mut bench);
        let covered = measured_fall_ms(&mut bench, target);
        let readout = target.readout(&bench.app.shared.config.play);
        let on_show = covered * f64::from(1.0 - COVER);
        assert!(
            same_time(on_show, f64::from(readout.duration_ms)),
            "held {keep_green_number}: a note is on show for {on_show} ms against {}",
            readout.duration_ms
        );
        if keep_green_number {
            assert!(same_time(on_show, open), "the cover moved the time that is held: {open} ms open, {on_show} ms under it");
        } else {
            assert!(same_time(covered, open), "the cover moved a stored speed: {open} ms open, {covered} ms under it");
        }
    }
}

/// The lane cover, the lift and the hidden cover are drawn where the settings put them on the frame
/// after they move, each only while it is switched on, and the lane is as it was once they are off.
/// The lane is read in the chart's first measure, which is empty, so nothing else is in it.
#[test]
fn the_covers_and_the_lift_are_drawn_where_the_settings_put_them() {
    const BEFORE_THE_FIRST_NOTE_MS: i64 = 500;
    let mut bench = play_bench("covers", SEVEN_KEYS);
    let play = &mut bench.app.shared.config.play;
    (play.enable_cover, play.enable_lift, play.enable_hidden) = (false, false, false);
    (play.cover, play.lift, play.hidden) = (COVER, LIFT, HIDDEN);
    told(&mut bench);
    assert_eq!(chart_frame(&mut bench, BEFORE_THE_FIRST_NOTE_MS), PreviewStatus::Drawn);
    let lane = LaneSeen::of(&bench, SEVEN_KEYS.screen);
    let (bare, bare_frame) = (lane.column_of(&bench), bench.pixels.pixel_checksum());
    let rows = |share: f32| lane.height() * share;
    let drawn = |bench: &mut Bench, set: fn(&mut rbms_config::PlayOptions, bool)| {
        set(&mut bench.app.shared.config.play, true);
        told(bench);
        assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn, "a cover cost a frame");
        let changed = changed_rows(&bare, &lane.column_of(bench)).expect("a cover that is switched on changed nothing of the lane");
        set(&mut bench.app.shared.config.play, false);
        told(bench);
        assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn);
        assert_eq!(bench.pixels.pixel_checksum(), bare_frame, "a cover that is switched off left something behind");
        (changed.0 as f32, changed.1 as f32)
    };

    let (cover_top, cover_foot) = drawn(&mut bench, |play, on| play.enable_cover = on);
    assert!(cover_top <= ROW_TOLERANCE && (cover_foot - rows(COVER)).abs() <= ROW_TOLERANCE, "the lane cover is over rows {cover_top} to {cover_foot}");
    let (hidden_top, hidden_foot) = drawn(&mut bench, |play, on| play.enable_hidden = on);
    assert!(
        (hidden_top - rows(1.0 - HIDDEN)).abs() <= ROW_TOLERANCE && (hidden_foot - rows(1.0)).abs() <= ROW_TOLERANCE,
        "the hidden cover is over rows {hidden_top} to {hidden_foot}"
    );
    let (lift_top, _) = drawn(&mut bench, |play, on| play.enable_lift = on);
    assert!((lift_top - rows(1.0 - LIFT)).abs() <= LIFTED_LINE_TOLERANCE, "the lift begins at row {lift_top} of {}", lane.height());

    let play = &mut bench.app.shared.config.play;
    (play.enable_cover, play.enable_lift, play.enable_hidden) = (true, true, true);
    told(&mut bench);
    assert_eq!(chart_frame(&mut bench, MID_CHART_MS), PreviewStatus::Drawn);
    bench.save("play-preview-7k-cover30-lift15-hidden10");
}

/// The run goes on for as long as the menu is up: past the end of its chart the scene neither ends
/// nor is read again, a run of the chart is begun afresh with nothing of the last one left on, and
/// the combo and the judgements move again. Nothing is sounded and no setting is touched, the judge
/// offset least of all, though it is set to move by itself.
#[test]
fn the_play_menus_preview_goes_on_past_the_end_of_its_chart() {
    const PASSES: usize = 2;
    const DRAWN_EVERY: usize = 40;
    const PLAYERS_OFFSET_MS: i32 = 7;
    let mut bench = play_bench("endless", seven_keys_at(FAST_BPM));
    bench.app.shared.config.judge.auto_offset = true;
    bench.app.shared.config.judge.offset_ms = PLAYERS_OFFSET_MS;
    told(&mut bench);
    let settings = bench.app.shared.config.clone();
    start_playing(&mut bench);
    let notes = run_of(&bench).session().summary().total_notes;
    let play_time_ms = run_of(&bench).session().play_time_ms();
    let steps = (play_time_ms as f32 / (STEP_SECONDS * MILLIS_PER_SECOND)) as usize * (PASSES + 1);
    let (mut passes, mut last_ms, mut last_combo) = (0, 0, 0);
    for step in 0..steps {
        let scene_us = bench.preview.scene_us();
        let full_combo_was_on = bench.app.shared.skin_timers.is_on(timer_id::FULLCOMBO_1P);
        bench.update(STEP_SECONDS);
        assert_eq!(*bench.preview.status(), PreviewStatus::Drawn, "the scene was begun again {step} steps in");
        assert!(bench.preview.scene_us() > scene_us, "the scene stood still or went back {step} steps in");
        let (phase, now_ms, combo, ..) = run_of(&bench).standing();
        assert_eq!(phase, PlayPhase::Play, "the scene left its playing state {step} steps in");
        if now_ms < last_ms {
            passes += 1;
            assert_eq!(last_combo, notes, "the run before was begun again before its last note");
            assert!(full_combo_was_on, "the run before never showed its full combo");
            assert_eq!(combo, 0, "the run that began again is not a new one");
            let timers = &bench.app.shared.skin_timers;
            assert!(!timers.is_on(timer_id::FULLCOMBO_1P) && !timers.is_on(timer_id::ENDOFNOTE_1P), "what the run before came to is still on show");
            assert!(timers.is_on(timer_id::PLAY) && bench.preview.scene_us() - timers.value_us(timer_id::PLAY) < MICROS_PER_SECOND as i64);
        }
        (last_ms, last_combo) = (now_ms, combo);
        if step % DRAWN_EVERY == 0 {
            assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn, "{step} steps in");
        }
    }
    assert!(passes >= PASSES, "the chart came round {passes} times in {steps} steps");
    assert!(last_combo > 0 && bench.app.shared.skin_timers.is_on(timer_id::JUDGE_1P), "nothing is judged in the run that began again");
    assert_eq!(bench.app.shared.config, settings, "the preview changed a setting");
    assert!(bench.app.shared.audio.is_none(), "a sample run opened the output stream");
}

/// What a run of the sample chart at `offset_ms` of judge offset, self adjusting or not, has
/// tallied a set way into its chart: its judgements, and its early and late hits.
fn tally_under(tag: &str, offset_ms: i32, auto_offset: bool) -> ([u32; 6], u32, u32) {
    const TALLIED_AT_MS: i64 = 12_000;
    let mut bench = play_bench(tag, seven_keys_at(FAST_BPM));
    bench.app.shared.config.judge.offset_ms = offset_ms;
    bench.app.shared.config.judge.auto_offset = auto_offset;
    told(&mut bench);
    start_playing(&mut bench);
    while chart_ms(&bench) < TALLIED_AT_MS {
        bench.update(STEP_SECONDS);
    }
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn);
    assert_eq!(
        (bench.app.shared.config.judge.offset_ms, bench.app.shared.config.judge.auto_offset),
        (offset_ms, auto_offset),
        "the run moved the judge offset"
    );
    let summary = run_of(&bench).session().summary();
    (summary.counts, summary.fast, summary.slow)
}

/// The sample hand is judged the same whatever the player's judge offset is and whether or not it
/// adjusts itself: the offset is the player's own, and a sample run neither reads nor moves it.
#[test]
fn the_sample_run_is_judged_the_same_under_any_judge_offset() {
    const OFFSET_MS: i32 = 50;
    let early = tally_under("offset-early", OFFSET_MS, true);
    let late = tally_under("offset-late", -OFFSET_MS, false);
    assert!(early.0.iter().sum::<u32>() > 0 && early.1 > 0 && early.2 > 0, "nothing was tallied: {early:?}");
    assert_eq!(early, late, "the judge offset reached the sample run");
}

/// Another gauge is shown from the frame after it is named: the run is on it, as far through its
/// chart and on the combo it had, and the scene goes on.
#[test]
fn another_gauge_is_taken_up_where_the_run_stands() {
    use rbms_judge::GaugeKind;
    use rbms_judge::gauge::GaugeIndex;
    let mut bench = play_bench("gauge", SEVEN_KEYS);
    assert_eq!(chart_frame(&mut bench, MID_CHART_MS), PreviewStatus::Drawn);
    let gauge_row = |bench: &Bench| {
        let (left, _, right, _) = rect_pixels(SAME_SIZE);
        (left..right).map(|x| pixel(bench, x, (RECT.y + RECT.h * GAUGE_ROW) as u32)).collect::<Vec<_>>()
    };
    let on_gauge = |bench: &Bench| run_of(bench).session().judge().gauge.selected_index();
    let (scene_us, (_, before_ms, combo, score, _), normal_row) = (bench.preview.scene_us(), run_of(&bench).standing(), gauge_row(&bench));
    assert_eq!(on_gauge(&bench), GaugeIndex::Normal);
    bench.save("play-preview-7k-gauge-normal");

    bench.app.shared.config.play.gauge = GaugeKind::Hard;
    told(&mut bench);
    assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn, "another gauge cost a frame");
    assert_eq!(on_gauge(&bench), GaugeIndex::Hard, "the run is not on the gauge that was named");
    let (phase, now_ms, now_combo, now_score, _) = run_of(&bench).standing();
    assert_eq!((phase, now_ms, now_combo, now_score), (PlayPhase::Play, before_ms, combo, score), "the run on the new gauge is not where the one before was");
    assert_eq!(bench.preview.scene_us(), scene_us, "another gauge moved the scene");
    assert_ne!(gauge_row(&bench), normal_row, "the gauge is drawn as it was");
    bench.save("play-preview-7k-gauge-hard");
}

/// Step the option called `name` of the document `screen` is drawn with and tell the preview, the
/// way the play menu does for a row of the previewed skin: the stored choice, the read the library
/// is asked for, then the word that something moved.
fn step_play_skin_option(bench: &mut Bench, screen: i32, name: &str) {
    let shared = &mut bench.app.shared;
    let header = shared.skins.header_of(&shared.config, screen).expect("the play screen has a document");
    let at = header.properties.iter().position(|property| property.name == name).unwrap_or_else(|| panic!("the play skin has no option called {name}"));
    let configured = std::mem::replace(&mut shared.config.skin.screen, screen);
    assert!(shared.step_skin_row(SkinRow::Property(at), 1), "{name} did not move");
    shared.config.skin.screen = configured;
    if shared.skins.needs_reload_for(&shared.config, screen) {
        shared.skins.request_for(&shared.config, screen);
    }
    told(bench);
}

/// An option of the previewed skin has its document read again under the scene as it stands: the
/// lanes are drawn as wide as the option says, the run goes on from where it was and is judged as
/// before, and a run begun afresh afterwards is fed the document that was read again.
#[test]
fn an_option_of_the_play_skin_is_shown_without_the_run_beginning_again() {
    const WIDE: f32 = 1.5;
    const A_FEW_NOTES_ON_MS: i64 = 2_000;
    let screen = SEVEN_KEYS.screen;
    let mut bench = play_bench("skin-option", SEVEN_KEYS);
    assert_eq!(chart_frame(&mut bench, MID_CHART_MS), PreviewStatus::Drawn);
    let (narrow, build, scene_us, before_ms) =
        (LaneSeen::of(&bench, screen), bench.app.shared.skins.build_of(screen), bench.preview.scene_us(), chart_ms(&bench));

    step_play_skin_option(&mut bench, screen, "Lane Width");
    assert_eq!(*bench.preview.status(), PreviewStatus::Loading);
    assert!(bench.preview.waits_in_the_dark(), "a document read again for an option is said to be loading over the frame it had");
    assert_eq!(bench.settle(), PreviewStatus::Drawn, "{:?}", bench.app.shared.skin_failure(screen));
    assert_ne!(bench.app.shared.skins.build_of(screen), build, "the option did not have the document read again");
    assert_eq!((bench.preview.scene_us(), chart_ms(&bench)), (scene_us, before_ms), "the run did not carry on from where it stood");
    let wide = LaneSeen::of(&bench, screen);
    assert!((wide.width - narrow.width * WIDE).abs() <= PLACE_TOLERANCE, "the lane is {} wide where it was {}", wide.width, narrow.width);
    bench.save("play-preview-7k-wide");

    bench.app.shared.config.play.gauge = rbms_judge::GaugeKind::Hard;
    told(&mut bench);
    let swapped_us = bench.preview.scene_us();
    assert_eq!(frame_after(&mut bench, A_FEW_NOTES_ON_MS), PreviewStatus::Drawn);
    let timers = &bench.app.shared.skin_timers;
    assert!(timers.is_on(timer_id::JUDGE_1P) && timers.value_us(timer_id::JUDGE_1P) > swapped_us, "the run begun afresh is not judged on the skin's timers");

    step_play_skin_option(&mut bench, screen, "Scratch Side");
    assert_eq!(bench.settle(), PreviewStatus::Drawn, "{:?}", bench.app.shared.skin_failure(screen));
    let moved = LaneSeen::of(&bench, screen);
    assert!((moved.left - wide.left).abs() > PLACE_TOLERANCE, "the turntable changed sides and the first key stayed where it was");
    assert_eq!(run_of(&bench).standing().0, PlayPhase::Play);
    bench.save("play-preview-7k-wide-scratch-right");
}

/// Leaving the play menu leaves the screen it was opened over as it was -- the read it was compiled
/// from, its textures, a frame on the first frame back -- and nothing of the previewed play screen.
#[test]
fn leaving_the_play_menu_leaves_only_the_screen_under_it() {
    let mut bench = Bench::new("play-left", Some(&shipped_pack()));
    let build = decide_screen_drawn_for_real(&mut bench);
    let textures = bench.app.shared.skin_screens.texture_stats().count;
    let underneath = bench.app.shared.suspend_skin_scene();

    bench.preview = SkinPreview::of_play(SEVEN_KEYS);
    assert_eq!(bench.settle(), PreviewStatus::Drawn);
    assert_eq!(chart_frame(&mut bench, MID_CHART_MS), PreviewStatus::Drawn);
    assert!(bench.app.shared.skin_screens.texture_stats().count > textures, "the preview drew from no texture of its own");

    bench.preview.release(&mut bench.app.shared);
    bench.app.shared.resume_skin_scene(underneath);
    assert!(bench.preview.play_sample().is_none(), "the sample run outlived the preview");
    let chart = ChartMeta::default();
    let mut canvas = Canvas::Headless(&mut bench.pixels);
    bench.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_DECIDE);
    assert_eq!(bench.app.shared.skins.build_of(SKIN_TYPE_DECIDE), build, "the screen gone back to was read again");
    assert!(
        bench.app.shared.draw_decide_skin(&mut canvas, &DecideDraw { chart: &chart, progress: 0.0, data: FrameData::default() }),
        "the first frame back drew no skin"
    );
    bench.app.shared.finish_skin_frame(&mut canvas);
    assert!(!bench.app.shared.has_compiled_skin(SEVEN_KEYS.screen), "the previewed play screen outlived the menu");
    assert!(bench.app.shared.skins.needs_reload_for(&bench.app.shared.config, SEVEN_KEYS.screen), "the skin read for the preview was left for the play screen");
    assert_eq!(bench.app.shared.skin_screens.texture_stats().count, textures, "the preview's textures stayed uploaded");
    assert_eq!(bench.app.shared.skin_screens.movies_playing(), 0);
}

/// With a pack named by the environment, its seven key play screen is previewed the way the play
/// menu previews it and taken through the settings the menu moves, a capture at each: a held number
/// at two tempos, a stored speed, the covers, CONSTANT and another gauge. A pack with no document
/// for the screen has nothing to show.
#[test]
fn a_skin_pack_named_by_the_environment_is_previewed_as_the_play_menu_shows_it() {
    const SETTLED_AFTER_MS: i64 = 1_500;
    let Some(pack) = pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    let mut bench = Bench::new("pack-play-menu", Some(&pack));
    bench.preview = SkinPreview::of_play(seven_keys_at(SLOW_BPM));
    let first = bench.settle();
    println!("play menu preview of the pack: {first:?} {:?}", first.detail());
    if !first.is_drawn() {
        return;
    }
    assert_eq!(chart_frame(&mut bench, MID_CHART_MS), PreviewStatus::Drawn);
    let scene_us = bench.preview.scene_us();
    let shown = |bench: &mut Bench, name: &str, set: &dyn Fn(&mut Config)| {
        set(&mut bench.app.shared.config);
        told(bench);
        assert_eq!(bench.frame(NO_TIME), PreviewStatus::Drawn, "{name} cost a frame");
        assert_eq!(frame_after(bench, SETTLED_AFTER_MS), PreviewStatus::Drawn, "{name}");
        assert_eq!(drawn_on(bench.pixels.rgba(), bench.size, SAME_SIZE, 0).0, 0, "{name} drew outside its rectangle");
        bench.save(&format!("play-preview-pack-7k-{name}"));
    };
    shown(&mut bench, "held-100", &|config| config.play.keep_green_number = true);
    bench.preview.show_play(seven_keys_at(FAST_BPM));
    shown(&mut bench, "held-200", &|_| {});
    shown(&mut bench, "stored-200", &|config| config.play.keep_green_number = false);
    shown(&mut bench, "covers", &|config| {
        let play = &mut config.play;
        (play.enable_cover, play.enable_lift, play.enable_hidden) = (true, true, true);
        (play.cover, play.lift, play.hidden) = (COVER, LIFT, HIDDEN);
    });
    shown(&mut bench, "constant", &|config| config.play.constant_speed = true);
    shown(&mut bench, "gauge-hard", &|config| config.play.gauge = rbms_judge::GaugeKind::Hard);
    assert!(bench.preview.scene_us() > scene_us && run_of(&bench).standing().0 == PlayPhase::Play, "the scene began again on the way");
}
