//! Which files a screen's document has decoded, how long the textures stay uploaded, and what the
//! scene clock does while they are on their way.
//!
//! Every document here is a small data-only one in a pack of the test's own, and every sheet is a
//! flat colour written as a bitmap -- a format whose files are the same length whatever their
//! colour, which is what lets a test save over a sheet without the file looking any different from
//! outside. A frame is a screen's `prepare_skin`, its draw where the test looks at pixels, and the
//! frame end the frame loop runs after every draw.
//!
//! One test walks the screens themselves -- the browser, the decide scene opened over it, the run
//! and the result -- because how long a screen's textures stay uploaded is decided by the way each
//! screen is left, and that is the application's to get right, not this module's.

use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::channel;

use rbms_render::skin_render::textures::rgba_bytes;
use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;

use super::*;
use crate::app_play::loaded_chart_for_tests;
use crate::stage::{DecideState, FrameCtx, HeadlessCanvas, ResultState, SelectState, Stage, StageId, Transition};

/// How long a document with a sheet or two is given to compile, and how long a frame that finds it
/// still decoding stands back for the worker pool. The decode is off the frame loop, so the number
/// of frames drawn says nothing about how long a worker has had.
const DECODE_WAIT: Duration = Duration::from_secs(20);
const DECODE_FRAME_PAUSE: Duration = Duration::from_millis(1);

/// Edge of every sheet, in pixels.
const SHEET_EDGE: u32 = 8;

/// The colours the sheets are filled with, none of which a built-in screen paints.
const COMMON_INK: crate::Color = crate::Color::rgb(12, 200, 90);
const OWN_INK: crate::Color = crate::Color::rgb(200, 12, 90);
const EDITED_INK: crate::Color = crate::Color::rgb(30, 60, 220);

/// The option the documents' customisation row grants, and the one it does not.
const GRANTED_OPTION: i32 = 901;
const WITHHELD_OPTION: i32 = 902;

/// How long the tests that watch the scene clock leave it to stand.
const CLOCK_WAIT: Duration = Duration::from_millis(40);

/// How far a file's modification time is moved on when a test saves over it, well past the
/// coarsest time a file system keeps.
const EDIT_STEP: Duration = Duration::from_secs(5);

/// An app drawing with a pack of the test's own, and the canvas its screens are compiled against.
struct Pack {
    app: crate::App,
    folder: PathBuf,
    pixels: HeadlessCanvas,
}

impl Pack {
    /// A pack holding `documents`, each written under its own file name, and one sheet per colour.
    fn new(tag: &str, documents: &[(&str, String)], sheets: &[(&str, crate::Color)]) -> Pack {
        rbms_render::font::use_embedded_fonts_only();
        let home = std::env::temp_dir().join(format!("rbms-skin-textures-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let folder = home.join("pack");
        std::fs::create_dir_all(&folder).expect("the pack folder is writable");
        for (name, body) in documents {
            std::fs::write(folder.join(name), body).expect("the document is written");
        }
        for (name, ink) in sheets {
            write_sheet(&folder.join(name), *ink);
        }
        let mut config = crate::Config::default();
        config.skin.pack = Some(folder.to_string_lossy().into_owned());
        let app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron"));
        Pack { app, folder, pixels: HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1) }
    }

    /// One frame of `screen` with nothing drawn: its document is asked for, and the frame ends.
    fn frame(&mut self, screen: i32) {
        let mut canvas = Canvas::Headless(&mut self.pixels);
        self.app.shared.prepare_skin(&mut canvas, screen);
        self.app.shared.finish_skin_frame(&mut canvas);
    }

    /// One frame of the key configuration screen, drawn; answers whether its document drew it.
    fn key_config_frame(&mut self) -> bool {
        let mut canvas = Canvas::Headless(&mut self.pixels);
        self.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_KEY_CONFIG);
        let drew = self.app.shared.draw_keyconfig_skin(&mut canvas, &[]);
        self.app.shared.finish_skin_frame(&mut canvas);
        drew
    }

    /// Frames of `screen` until its document has compiled.
    fn compile(&mut self, screen: i32) {
        let began = Instant::now();
        loop {
            self.frame(screen);
            if self.app.shared.has_compiled_skin(screen) {
                return;
            }
            assert!(began.elapsed() <= DECODE_WAIT, "the document of screen {screen} never compiled: {:?}", self.app.shared.skin_failure(screen));
            std::thread::sleep(DECODE_FRAME_PAUSE);
        }
    }

    /// Whether the texture pool holds the sheet called `name`.
    fn uploaded(&self, name: &str) -> bool {
        let pool = &self.app.shared.skin_screens.textures;
        self.app
            .shared
            .skins
            .document(SKIN_TYPE_KEY_CONFIG)
            .into_iter()
            .chain(self.app.shared.skins.document(SKIN_TYPE_RESULT))
            .any(|document| document.sources.values().any(|path| path.file_name().is_some_and(|file| file == name) && pool.contains(path)))
    }

    /// How many textures are uploaded.
    fn textures(&self) -> usize {
        self.app.shared.skin_screens.texture_stats().count
    }

    /// The colour at the middle of the left or the right half of the last frame drawn.
    fn ink(&self, right: bool) -> crate::Color {
        let x = if right { UI_SIZE.0 * 3 / 4 } else { UI_SIZE.0 / 4 };
        self.pixels.pixel_at(x, UI_SIZE.1 / 2)
    }
}

/// Writes a flat sheet of `ink`.
fn write_sheet(path: &Path, ink: crate::Color) {
    image::RgbaImage::from_pixel(SHEET_EDGE, SHEET_EDGE, image::Rgba([ink.r, ink.g, ink.b, u8::MAX])).save(path).expect("the sheet is written");
}

/// Saves a sheet over with another colour and leaves the file looking as it did from outside: the
/// same length, and the modification time it had.
fn repaint_unnoticed(path: &Path, ink: crate::Color) {
    let written = std::fs::metadata(path).and_then(|file| file.modified()).expect("the sheet has a modification time");
    write_sheet(path, ink);
    std::fs::File::options().write(true).open(path).and_then(|file| file.set_modified(written)).expect("the modification time is put back");
}

/// Saves a sheet over with another colour, the way its author would: the file is newer afterwards.
fn repaint(path: &Path, ink: crate::Color) {
    let written = std::fs::metadata(path).and_then(|file| file.modified()).expect("the sheet has a modification time");
    write_sheet(path, ink);
    std::fs::File::options().write(true).open(path).and_then(|file| file.set_modified(written + EDIT_STEP)).expect("the modification time is moved on");
}

/// One source declaration.
fn source(id: &str, file: &str) -> String {
    format!(r#"{{ "id": "{id}", "path": "{file}" }}"#)
}

/// One image cut from the whole of a sheet, named for its source.
fn image(id: &str) -> String {
    format!(r#"{{ "id": "{id}", "src": "{id}", "x": 0, "y": 0, "w": {SHEET_EDGE}, "h": {SHEET_EDGE} }}"#)
}

/// One destination over the left or the right half of the screen, gated on `op` when there is one.
fn half(id: &str, right: bool, op: Option<i32>) -> String {
    let x = if right { UI_SIZE.0 / 2 } else { 0 };
    let gate = op.map_or(String::new(), |op| format!(r#""op": [{op}], "#));
    format!(r#"{{ "id": "{id}", {gate}"dst": [{{ "time": 0, "x": {x}, "y": 0, "w": {}, "h": {} }}] }}"#, UI_SIZE.0 / 2, UI_SIZE.1)
}

/// A document for the screen of type `skin_type`, with one customisation row granting
/// [`GRANTED_OPTION`].
fn document(skin_type: i32, sources: &[String], images: &[String], destinations: &[String]) -> String {
    format!(
        r#"{{
            "type": {skin_type}, "name": "Textures", "w": {}, "h": {},
            "property": [{{ "name": "Row", "item": [{{ "name": "on", "op": {GRANTED_OPTION} }}, {{ "name": "off", "op": {WITHHELD_OPTION} }}], "def": "on" }}],
            "source": [{}],
            "image": [{}],
            "destination": [{}]
        }}"#,
        UI_SIZE.0,
        UI_SIZE.1,
        sources.join(", "),
        images.join(", "),
        destinations.join(", ")
    )
}

/// A document that draws the shared sheet over the left half and a sheet of its own over the right.
fn two_sheet_document(skin_type: i32, own: &str) -> String {
    document(
        skin_type,
        &[source("common", "common.bmp"), source("own", own)],
        &[image("common"), image("own")],
        &[half("common", false, None), half("own", true, None)],
    )
}

/// A pack whose key configuration and result screens share one sheet and have one each of their own.
fn sharing_pack(tag: &str) -> Pack {
    Pack::new(
        tag,
        &[("keys.json", two_sheet_document(SKIN_TYPE_KEY_CONFIG, "keys.bmp")), ("result.json", two_sheet_document(SKIN_TYPE_RESULT, "result.bmp"))],
        &[("common.bmp", COMMON_INK), ("keys.bmp", OWN_INK), ("result.bmp", OWN_INK)],
    )
}

/// A document has only the files its drawn objects read decoded. A sheet it declares for an object
/// nobody placed, and one for an object its customisation row rules out, are never opened -- both
/// are files that would not decode, and nothing is said about either.
#[test]
fn only_the_sheets_a_document_draws_from_are_decoded() {
    let body = document(
        SKIN_TYPE_KEY_CONFIG,
        &[source("shown", "shown.bmp"), source("spare", "spare.bmp"), source("ruled-out", "ruled-out.bmp")],
        &[image("shown"), image("spare"), image("ruled-out")],
        &[half("shown", false, Some(GRANTED_OPTION)), half("ruled-out", true, Some(WITHHELD_OPTION))],
    );
    let mut pack = Pack::new("referenced", &[("keys.json", body)], &[("shown.bmp", COMMON_INK)]);
    for broken in ["spare.bmp", "ruled-out.bmp"] {
        std::fs::write(pack.folder.join(broken), b"not a bitmap").expect("the broken sheet is written");
    }

    pack.compile(SKIN_TYPE_KEY_CONFIG);
    let warnings = pack.app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG);
    assert!(warnings.is_empty(), "a sheet the document does not draw from was read: {warnings:?}");
    assert_eq!(pack.textures(), 1);
    assert_eq!(pack.app.shared.skin_screens.texture_stats().bytes, rgba_bytes((SHEET_EDGE, SHEET_EDGE)));
    assert_eq!(pack.app.shared.debug_skin_texture_line(), "SKIN TEX 1  0.0 MB");

    assert!(pack.key_config_frame(), "a compiled document did not draw its screen");
    assert_eq!(pack.ink(false), COMMON_INK);
    assert_ne!(pack.ink(true), COMMON_INK, "the object the customisation ruled out was drawn");
}

/// Leaving a screen gives its textures up, and the screen that follows takes over the file the two
/// share without decoding it again: the shared sheet was saved over in between, unnoticeably, and
/// the next screen still shows what was uploaded.
#[test]
fn a_screen_that_is_left_gives_up_its_textures_and_the_next_reuses_the_file_they_share() {
    let mut pack = sharing_pack("leave");
    pack.app.shared.begin_skin_scene();
    pack.compile(SKIN_TYPE_RESULT);
    assert_eq!(pack.textures(), 2);
    assert!(pack.uploaded("common.bmp") && pack.uploaded("result.bmp"));

    repaint_unnoticed(&pack.folder.join("common.bmp"), EDITED_INK);
    pack.app.shared.begin_skin_scene();
    pack.compile(SKIN_TYPE_KEY_CONFIG);

    assert!(!pack.app.shared.has_compiled_skin(SKIN_TYPE_RESULT), "the screen that was left is still compiled");
    assert!(!pack.uploaded("result.bmp"), "the sheet only the screen that was left drew from is still uploaded");
    assert!(pack.uploaded("common.bmp") && pack.uploaded("keys.bmp"));
    assert_eq!(pack.textures(), 2);

    assert!(pack.key_config_frame());
    assert_eq!(pack.ink(false), COMMON_INK, "the shared sheet was decoded a second time");
    assert_eq!(pack.ink(true), OWN_INK);
}

/// A screen with another opened over it keeps its textures for as long as it is parked, and the
/// screen that was opened over it gives its own up once it is closed again.
#[test]
fn a_parked_screen_keeps_its_textures_and_the_one_opened_over_it_gives_its_own_up() {
    let mut pack = sharing_pack("parked");
    pack.app.shared.begin_skin_scene();
    pack.compile(SKIN_TYPE_RESULT);

    let parked = pack.app.shared.suspend_skin_scene();
    pack.compile(SKIN_TYPE_KEY_CONFIG);
    pack.frame(SKIN_TYPE_KEY_CONFIG);
    assert!(pack.app.shared.has_compiled_skin(SKIN_TYPE_RESULT), "a parked screen was let go of");
    assert!(pack.uploaded("result.bmp"));
    assert_eq!(pack.textures(), 3, "the shared sheet is uploaded once, and each screen's own beside it");

    pack.app.shared.resume_skin_scene(parked);
    pack.frame(SKIN_TYPE_RESULT);
    assert!(pack.app.shared.has_compiled_skin(SKIN_TYPE_RESULT));
    assert!(!pack.app.shared.has_compiled_skin(SKIN_TYPE_KEY_CONFIG), "the screen that was closed is still compiled");
    assert!(!pack.uploaded("keys.bmp") && pack.uploaded("common.bmp") && pack.uploaded("result.bmp"));
    assert_eq!(pack.textures(), 2);
}

/// A parked scene that is dropped rather than put back stops holding its screens: the next scene
/// change lets go of them like any other screen nobody draws.
#[test]
fn a_parked_scene_that_is_dropped_stops_holding_its_screen() {
    let mut pack = sharing_pack("dropped");
    pack.app.shared.begin_skin_scene();
    pack.compile(SKIN_TYPE_RESULT);

    drop(pack.app.shared.suspend_skin_scene());
    pack.app.shared.begin_skin_scene();
    pack.compile(SKIN_TYPE_KEY_CONFIG);
    assert!(!pack.app.shared.has_compiled_skin(SKIN_TYPE_RESULT));
    assert_eq!(pack.textures(), 2);
}

/// A scene begun over a parked screen is a change of screen, not a screen opened to be closed again:
/// the parked screen is let go of with the one that was opened over it, nothing of either stays
/// uploaded, and the parked scene has its document compiled again when it is put back.
#[test]
fn a_scene_begun_over_a_parked_screen_lets_go_of_it_until_it_is_put_back() {
    let mut pack = sharing_pack("begun-over");
    pack.app.shared.begin_skin_scene();
    pack.compile(SKIN_TYPE_RESULT);
    let parked = pack.app.shared.suspend_skin_scene();
    pack.compile(SKIN_TYPE_KEY_CONFIG);
    assert_eq!(pack.textures(), 3);

    pack.app.shared.begin_skin_scene();
    pack.frame(SKIN_TYPE_MUSIC_SELECT);
    assert!(!pack.app.shared.has_compiled_skin(SKIN_TYPE_RESULT), "a parked screen stayed compiled under a scene begun over it");
    assert!(!pack.app.shared.has_compiled_skin(SKIN_TYPE_KEY_CONFIG));
    assert_eq!(pack.textures(), 0, "a screen nobody draws still has textures uploaded");

    pack.app.shared.resume_skin_scene(parked);
    pack.compile(SKIN_TYPE_RESULT);
    assert!(pack.uploaded("common.bmp") && pack.uploaded("result.bmp"));
    assert_eq!(pack.textures(), 2);
}

/// A sheet its author saved over is decoded again the next time the skin is read, into the texture
/// the file already had.
#[test]
fn a_sheet_saved_over_is_decoded_again_the_next_time_its_skin_is_read() {
    let mut pack = sharing_pack("edited");
    pack.app.shared.begin_skin_scene();
    pack.compile(SKIN_TYPE_KEY_CONFIG);
    assert!(pack.key_config_frame());
    assert_eq!(pack.ink(true), OWN_INK);

    repaint(&pack.folder.join("keys.bmp"), EDITED_INK);
    let config = pack.app.shared.config.clone();
    pack.app.shared.skins.request_for(&config, SKIN_TYPE_KEY_CONFIG);
    pack.frame(SKIN_TYPE_KEY_CONFIG);
    pack.compile(SKIN_TYPE_KEY_CONFIG);

    assert!(pack.key_config_frame());
    assert_eq!(pack.ink(true), EDITED_INK, "the sheet that was saved over still shows what it used to");
    assert_eq!(pack.ink(false), COMMON_INK);
    assert_eq!(pack.textures(), 2, "the edited sheet was uploaded beside the old one");
}

/// A document whose sheets come to more than the budget goes without the one that does not fit,
/// and says so.
#[test]
fn a_sheet_that_does_not_fit_the_budget_is_left_out_with_a_warning() {
    let mut pack = sharing_pack("budget");
    pack.app.shared.skin_screens.textures = SkinTexturePool::with_budget(rgba_bytes((SHEET_EDGE, SHEET_EDGE)));
    pack.compile(SKIN_TYPE_KEY_CONFIG);

    let warnings = pack.app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG);
    assert!(warnings.iter().any(|line| line.contains("would take this skin's textures past") && line.contains("keys.bmp")), "{warnings:?}");
    assert_eq!(pack.textures(), 1);
    assert!(pack.uploaded("common.bmp") && !pack.uploaded("keys.bmp"));
}

/// A key configuration document with one sheet over the left half, and a pack holding it.
fn one_sheet_pack(tag: &str) -> Pack {
    let body = document(SKIN_TYPE_KEY_CONFIG, &[source("common", "common.bmp")], &[image("common")], &[half("common", false, None)]);
    Pack::new(tag, &[("keys.json", body)], &[("common.bmp", COMMON_INK)])
}

/// What a test holds of a decode it runs by hand: where the worker's result goes, and the count the
/// screen reads to know the worker is done. The decode is waited for for as long as this is held and
/// has not finished.
pub(crate) struct HandDecode {
    results: std::sync::mpsc::Sender<(SkinAssetJob, SkinAssetRead)>,
    done: Arc<AtomicUsize>,
}

impl HandDecode {
    /// Reads the document of `screen`, which has to be one that is only data, and leaves it waiting
    /// on one decode that finishes when the test says so, in place of the worker pool.
    pub(crate) fn begin(shared: &mut AppShared, screen: i32) -> HandDecode {
        let config = shared.config.clone();
        shared.skins.request_for(&config, screen);
        let build = shared.skins.build_of(screen).expect("a document that is only data is read when it is asked for");
        let (results, received) = channel();
        let done = Arc::new(AtomicUsize::new(0));
        let pending = PendingScreen {
            build,
            pool: (received, Arc::clone(&done), 1),
            ready: BTreeMap::new(),
            stamps: BTreeMap::new(),
            pins: Vec::new(),
            cancel: Arc::new(AtomicBool::new(false)),
        };
        shared.skin_screens.pending.insert(screen, pending);
        HandDecode { results, done }
    }

    /// Hands the one sheet the document of `screen` names over decoded, as the worker would have.
    fn finish(&self, shared: &AppShared, screen: i32) {
        let document = shared.skins.document(screen).expect("the document is read");
        let sheet = document.sources.values().next().expect("the document names its sheet");
        let decoded = image::open(sheet).expect("the sheet decodes").to_rgba8();
        let image = SkinImage::new(SHEET_EDGE, SHEET_EDGE, decoded.into_raw()).expect("the sheet is the size it was written at");
        let read = SkinAssetRead { asset: SkinAsset::Image(image), stamp: None };
        self.results.send(((SkinAssetKind::Image, sheet.clone()), read)).expect("the screen is still waiting");
        self.done.store(1, Ordering::Relaxed);
    }
}

/// Until a document's sheet has been decoded nothing of the document is drawn, and the scene does
/// not begin: its clock stands where it was when the wait started, and moves again on the frame the
/// document is compiled -- which is the first frame the object is on screen.
#[test]
fn a_scene_stands_still_and_draws_nothing_of_its_document_until_the_sheets_are_in() {
    let mut pack = one_sheet_pack("held");
    pack.app.shared.begin_skin_scene();
    let began = Instant::now();
    let decode = HandDecode::begin(&mut pack.app.shared, SKIN_TYPE_KEY_CONFIG);

    assert!(!pack.key_config_frame(), "a document whose sheet is not decoded drew its screen");
    assert_ne!(pack.ink(false), COMMON_INK, "an object was drawn before its sheet was decoded");
    assert!(pack.app.shared.skin_is_loading(SKIN_TYPE_KEY_CONFIG));
    assert!(pack.app.shared.has_skin_document(SKIN_TYPE_KEY_CONFIG), "a document on its way does not count as the screen's");

    let stood_at = pack.app.shared.skin_now_us();
    std::thread::sleep(CLOCK_WAIT);
    assert!(!pack.key_config_frame());
    assert_eq!(pack.app.shared.skin_now_us(), stood_at, "the scene clock ran while the document was on its way");

    decode.finish(&pack.app.shared, SKIN_TYPE_KEY_CONFIG);
    assert!(pack.key_config_frame(), "the frame the sheet arrived on was not drawn by the document");
    let now = Duration::from_micros(u64::try_from(pack.app.shared.skin_now_us()).expect("a scene clock does not run backwards"));
    let wall = began.elapsed();
    assert_eq!(pack.ink(false), COMMON_INK);
    assert!(!pack.app.shared.skin_is_loading(SKIN_TYPE_KEY_CONFIG));
    assert!(wall.saturating_sub(now) >= CLOCK_WAIT, "the wait was counted as scene time: {now:?} of {wall:?}");

    std::thread::sleep(CLOCK_WAIT);
    assert!(Duration::from_micros(u64::try_from(pack.app.shared.skin_now_us()).unwrap_or_default()) >= now + CLOCK_WAIT, "the clock did not start again");
}

/// A wait that has outlasted its limit is given up on: the scene runs on without its document
/// rather than standing still for a read that is not finishing, and is not held for it again.
#[test]
fn a_wait_past_its_limit_lets_the_scene_run_without_its_document() {
    let mut pack = one_sheet_pack("limit");
    pack.app.shared.begin_skin_scene();
    let _decode = HandDecode::begin(&mut pack.app.shared, SKIN_TYPE_KEY_CONFIG);
    assert!(!pack.key_config_frame());
    assert!(!pack.app.shared.skin_wait_is_spent(SKIN_TYPE_KEY_CONFIG), "a wait that has only begun counts as given up on");
    pack.app.shared.outlast_skin_wait();

    assert!(!pack.key_config_frame());
    assert!(pack.app.shared.skin_is_loading(SKIN_TYPE_KEY_CONFIG), "the document stopped being waited for");
    assert!(pack.app.shared.skin_wait_is_spent(SKIN_TYPE_KEY_CONFIG), "nothing says the wait was given up on");
    let released_at = pack.app.shared.skin_now_us();
    std::thread::sleep(CLOCK_WAIT);
    assert!(!pack.key_config_frame());
    assert!(pack.app.shared.skin_now_us() > released_at, "the scene is still standing after its limit");
    assert!(pack.app.shared.skin_screens.hold.is_none(), "the scene was held again for the same document");
}

/// A scene held for a screen that the frame no longer asks for is set going again when the frame
/// ends, so a clock is never left standing for a document nobody is waiting on.
#[test]
fn a_hold_ends_with_the_frame_that_stops_asking_for_its_screen() {
    let mut pack = one_sheet_pack("unasked");
    pack.app.shared.begin_skin_scene();
    let _decode = HandDecode::begin(&mut pack.app.shared, SKIN_TYPE_KEY_CONFIG);
    assert!(!pack.key_config_frame());
    assert!(pack.app.shared.skin_screens.hold.is_some());

    pack.frame(SKIN_TYPE_RESULT);
    assert!(pack.app.shared.skin_screens.hold.is_none(), "the scene stayed held for a screen the frame did not ask for");
}

/// A decode whose workers have all gone without accounting for their files -- one that died on a
/// file never counts it -- is not waited for: the document is compiled without what never came, says
/// so, and the scene goes on.
#[test]
fn a_document_whose_workers_went_without_reporting_is_compiled_without_their_files() {
    let mut pack = one_sheet_pack("deserted");
    pack.app.shared.begin_skin_scene();
    let decode = HandDecode::begin(&mut pack.app.shared, SKIN_TYPE_KEY_CONFIG);
    assert!(!pack.key_config_frame());
    assert!(pack.app.shared.skin_is_loading(SKIN_TYPE_KEY_CONFIG));

    drop(decode);
    assert!(pack.key_config_frame(), "a document whose workers are gone is still being waited for");
    assert!(!pack.app.shared.skin_is_loading(SKIN_TYPE_KEY_CONFIG));
    assert!(pack.app.shared.skin_screens.hold.is_none(), "the scene is still held for a decode that is over");
    let warnings = pack.app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG);
    assert!(warnings.iter().any(|line| line.contains("common.bmp")), "nothing was said about the sheet that never came: {warnings:?}");
    assert_eq!(pack.textures(), 0);
    assert_ne!(pack.ink(false), COMMON_INK);
}

/// The chart the walk below decides on and plays: seven keys, and nothing to decode.
const FLOW_CHART: &[u8] = b"#PLAYER 1\n#TITLE Flow\n#ARTIST Fixture\n#BPM 120\n#00111:01010101\n";
const FLOW_CHART_NAME: &str = "flow.bms";

/// The screens the walk passes through, each with the sheet only it draws from.
const FLOW_SCREENS: [(i32, &str, &str); 4] = [
    (SKIN_TYPE_MUSIC_SELECT, "select.json", "select.bmp"),
    (SKIN_TYPE_DECIDE, "decide.json", "decide.bmp"),
    (SKIN_TYPE_PLAY_7KEYS, "play.json", "play.bmp"),
    (SKIN_TYPE_RESULT, "result.json", "result.bmp"),
];

/// How many textures one of those screens has uploaded when it is the only one compiled: the sheet
/// they all share, and its own.
const FLOW_SCREEN_TEXTURES: usize = 2;

/// A pack with a document for each of [`FLOW_SCREENS`], all of which draw one shared sheet as well.
fn flow_pack(tag: &str) -> Pack {
    let documents: Vec<(&str, String)> = FLOW_SCREENS.iter().map(|(skin_type, name, own)| (*name, two_sheet_document(*skin_type, own))).collect();
    let sheets: Vec<(&str, crate::Color)> = std::iter::once(("common.bmp", COMMON_INK)).chain(FLOW_SCREENS.iter().map(|(_, _, own)| (*own, OWN_INK))).collect();
    let mut pack = Pack::new(tag, &documents, &sheets);
    pack.app.shared.mode = rbms_model::Mode::BEAT_7K;
    pack
}

/// One frame of whichever stage is up, the way the frame loop makes it: the stage draws, and the
/// frame ends.
fn stage_frame(app: &mut crate::App, pixels: &mut HeadlessCanvas) {
    let mut canvas = Canvas::Headless(pixels);
    let mut ctx = FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: PACK_FRAME_DT };
    app.stage.draw(&mut ctx, &mut canvas);
    app.shared.finish_skin_frame(&mut canvas);
}

impl Pack {
    /// Frames of the stage that is up until the document of `screen` has compiled, and one more, so
    /// what is uploaded is what a screen that has settled in holds.
    fn settle_on(&mut self, screen: i32) {
        let began = Instant::now();
        while !self.app.shared.has_compiled_skin(screen) {
            stage_frame(&mut self.app, &mut self.pixels);
            assert!(began.elapsed() <= DECODE_WAIT, "the document of screen {screen} never compiled: {:?}", self.app.shared.skin_failure(screen));
            std::thread::sleep(DECODE_FRAME_PAUSE);
        }
        stage_frame(&mut self.app, &mut self.pixels);
    }

    /// Asserts that `screen` is the only one of [`FLOW_SCREENS`] compiled and that nothing but what
    /// it draws from is uploaded.
    fn assert_only(&self, screen: i32, when: &str) {
        for (other, name, _) in FLOW_SCREENS {
            assert_eq!(self.app.shared.has_compiled_skin(other), other == screen, "{when}: {name} is compiled or not against what is on screen");
        }
        let own = self.app.shared.skin_screens.get(screen).map(SkinScreen::texture_stats).unwrap_or_default();
        assert_eq!(own.count, FLOW_SCREEN_TEXTURES, "{when}: the screen did not upload its two sheets");
        assert_eq!(self.app.shared.skin_screens.texture_stats(), own, "{when}: something the screen does not draw from is still uploaded");
        assert_eq!(self.app.shared.skin_screens.textures.unheld(), 0, "{when}: a texture nobody holds was not freed");
    }

    /// Runs the decide scene that is up, a frame at a time, until it asks to leave, and answers what
    /// it asked for. `drawn` is called after every frame of it.
    fn run_decide_scene(&mut self, mut drawn: impl FnMut(&Pack)) -> Transition {
        let began = Instant::now();
        loop {
            let now = Instant::now();
            let transition = self.app.stage.update(&mut FrameCtx { shared: &mut self.app.shared, now, dt: PACK_FRAME_DT });
            if !matches!(transition, Transition::Stay) {
                return transition;
            }
            stage_frame(&mut self.app, &mut self.pixels);
            drawn(self);
            assert!(began.elapsed() <= DECODE_WAIT, "the decide scene never ended: {:?}", self.app.shared.skin_failure(SKIN_TYPE_DECIDE));
            std::thread::sleep(DECODE_FRAME_PAUSE);
        }
    }
}

/// The way a chart is really played: picked in the browser, which opens the decide scene over it,
/// then the run, the result, and back. The browser stays compiled under the decide scene, which may
/// yet be cancelled. Once the run begins it is a screen the player has left: through the run and the
/// result only the screen being drawn is compiled and only its textures are uploaded, and the
/// browser reads its own again when the player comes back to it.
#[test]
fn a_chart_played_from_the_browser_leaves_only_the_screen_being_drawn_uploaded() {
    let mut pack = flow_pack("flow");
    pack.app.shared.begin_skin_scene();
    pack.settle_on(SKIN_TYPE_MUSIC_SELECT);
    pack.assert_only(SKIN_TYPE_MUSIC_SELECT, "in the browser");

    let decide = Stage::Decide(Box::new(DecideState::loaded(&pack.app.shared, loaded_chart_for_tests(FLOW_CHART, FLOW_CHART_NAME))));
    pack.app.switch(Transition::Open(decide));
    let mut seen_with_the_browser_under_it = false;
    let leaving = pack.run_decide_scene(|pack| {
        if pack.app.shared.has_compiled_skin(SKIN_TYPE_DECIDE) {
            assert!(pack.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT), "the browser was let go of under a decide scene that could still be cancelled");
            assert_eq!(pack.textures(), FLOW_SCREEN_TEXTURES + 1, "the shared sheet is uploaded once, and each screen's own beside it");
            seen_with_the_browser_under_it = true;
        }
    });
    assert!(seen_with_the_browser_under_it, "the decide scene never drew with its document");
    assert!(matches!(leaving, Transition::To(Stage::Play(_))), "a decided chart with nothing to decode did not start its run");

    pack.app.switch(leaving);
    pack.settle_on(SKIN_TYPE_PLAY_7KEYS);
    pack.assert_only(SKIN_TYPE_PLAY_7KEYS, "during the run");

    pack.app.switch(Transition::To(Stage::Result(ResultState::new(pack_result_view()))));
    pack.settle_on(SKIN_TYPE_RESULT);
    pack.assert_only(SKIN_TYPE_RESULT, "on the result screen");

    pack.app.switch(Transition::Back);
    assert_eq!(pack.app.stage.id(), StageId::Select);
    assert!(pack.app.suspended.is_empty());
    pack.settle_on(SKIN_TYPE_MUSIC_SELECT);
    pack.assert_only(SKIN_TYPE_MUSIC_SELECT, "back in the browser");
}

/// The environment variable that names an external skin pack.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// The environment variable that names the folder the external pack's frames are written to.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The size the external pack's screens are drawn at, which is the size the pack this was written
/// against is authored at.
const PACK_CANVAS: (u32, u32) = (1920, 1080);

/// The seed every read of the external pack's skins is pinned to.
const PACK_SEED: u64 = 1;

/// How far into its scene each of the external pack's screens is captured, past its opening.
const PACK_SETTLED: Duration = Duration::from_secs(3);

/// The frame length the external pack's screens are drawn at.
const PACK_FRAME_DT: f32 = 1.0 / 60.0;

/// A finished run for the external pack's result screen to report.
fn pack_result_view() -> ResultView {
    ResultView {
        title: "textures".into(),
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
        prev_best_ex: None,
        prev_ex: None,
        show_graph: true,
        show_result_graphs: true,
        gauge_series: Vec::new(),
        timing_hist: Box::new([]),
        judge_dist: [0; 6],
    }
}

/// Which of the external pack's screens a scene is drawn on.
#[derive(Debug, Clone, Copy)]
enum PackScreen {
    Select,
    Result,
}

impl PackScreen {
    fn skin_type(self) -> i32 {
        match self {
            PackScreen::Select => SKIN_TYPE_MUSIC_SELECT,
            PackScreen::Result => SKIN_TYPE_RESULT,
        }
    }
}

/// One frame of `screen` the way the frame loop makes it: the screen is drawn, and the frame ends.
fn pack_frame(app: &mut crate::App, pixels: &mut HeadlessCanvas, screen: PackScreen) {
    match screen {
        PackScreen::Select => stage_frame(app, pixels),
        PackScreen::Result => {
            let mut canvas = Canvas::Headless(pixels);
            app.shared.prepare_skin(&mut canvas, SKIN_TYPE_RESULT);
            app.shared.draw_result_skin(&mut canvas, &pack_result_view(), &ResultExtras::default(), true, FrameData::default());
            app.shared.finish_skin_frame(&mut canvas);
        }
    }
}

/// Begins a scene on `screen` and draws it until its skin has compiled, then once more a few
/// seconds into the scene so the canvas shows the screen past its opening; says how long the skin
/// took and what is uploaded once it is in.
fn enter_pack_screen(app: &mut crate::App, pixels: &mut HeadlessCanvas, screen: PackScreen, label: &str) {
    app.shared.begin_skin_scene();
    app.stage = Stage::Select(Box::new(SelectState::new()));
    let began = Instant::now();
    let mut frames = 0_u32;
    let first_drawn = loop {
        let frame_began = Instant::now();
        pack_frame(app, pixels, screen);
        frames += 1;
        if app.shared.has_compiled_skin(screen.skin_type()) {
            break frame_began;
        }
        if let Some(reason) = app.shared.skin_failure(screen.skin_type()) {
            println!("{label}: not read - {}", reason.lines().next().unwrap_or_default());
            return;
        }
        assert!(began.elapsed() <= DECODE_WAIT, "{label}: the skin's files were not read in time");
        std::thread::sleep(DECODE_FRAME_PAUSE);
    };
    let waited = first_drawn.duration_since(began);
    let scene_at_first_frame = Duration::from_micros(u64::try_from(app.shared.skin_now_us()).unwrap_or_default()).saturating_sub(first_drawn.elapsed());
    let own = app.shared.skin_screens.get(screen.skin_type()).map(SkinScreen::texture_stats).unwrap_or_default();
    let uploaded = app.shared.skin_screens.texture_stats();
    println!(
        "{label}: first drawn {} ms and {frames} frames after it was entered, {} ms into its scene | its textures {} ({} RGBA bytes) | uploaded in all {} ({} RGBA bytes), {} waiting to be freed | {}",
        waited.as_millis(),
        scene_at_first_frame.as_millis(),
        own.count,
        own.bytes,
        uploaded.count,
        uploaded.bytes,
        app.shared.skin_screens.textures.unheld(),
        app.shared.debug_skin_texture_line(),
    );
    app.shared.age_skin_scene(PACK_SETTLED);
    pack_frame(app, pixels, screen);
}

/// Writes the frame on `pixels` under the capture folder, when one was asked for.
fn save_pack_frame(pixels: &HeadlessCanvas, name: &str) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV).map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&folder).expect("the capture folder is creatable");
    let frame = image::RgbaImage::from_raw(PACK_CANVAS.0, PACK_CANVAS.1, pixels.rgba().to_vec()).expect("the canvas holds a whole frame");
    frame.save(folder.join(name)).expect("the capture is written");
}

/// With `RBMS_SKIN_PACK` naming a skin pack, its browser and result screens are entered one after
/// the other and what each has uploaded is printed. Nothing of the pack is committed and nothing
/// here needs it to pass: with the variable unset the test returns at once. With
/// `RBMS_SKIN_CAPTURE_DIR` set as well, the browser's frame and the result screen's -- that one
/// under the debug panel, which says what is uploaded -- are written there.
///
/// What is checked is the lifetime the fixtures above pin down, on a skin of real size: a screen
/// that was left is no longer compiled, and nothing it alone drew from is still uploaded.
#[test]
fn the_textures_of_a_skin_pack_named_by_the_environment_follow_the_screen_being_drawn() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    rbms_render::font::use_embedded_fonts_only();
    let home = std::env::temp_dir().join(format!("rbms-skin-textures-external-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let mut config = crate::Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron"));
    app.shared.skins.pin_seed(Some(PACK_SEED));
    let mut pixels = HeadlessCanvas::new(PACK_CANVAS.0, PACK_CANVAS.1);

    enter_pack_screen(&mut app, &mut pixels, PackScreen::Select, "select");
    enter_pack_screen(&mut app, &mut pixels, PackScreen::Select, "select, entered again");
    save_pack_frame(&pixels, "textures-select.png");
    let browser_alone = app.shared.skin_screens.texture_stats();
    enter_pack_screen(&mut app, &mut pixels, PackScreen::Result, "result");
    app.shared.config.display.debug = true;
    let mut ctx = FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: PACK_FRAME_DT };
    crate::App::draw_overlays(&app.stage, &mut ctx, &mut Canvas::Headless(&mut pixels));
    save_pack_frame(&pixels, "textures-result.png");

    if app.shared.has_compiled_skin(SKIN_TYPE_RESULT) {
        assert!(!app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT), "the browser's skin is still compiled behind the result screen");
        let own = app.shared.skin_screens.get(SKIN_TYPE_RESULT).map(SkinScreen::texture_stats).unwrap_or_default();
        assert_eq!(app.shared.skin_screens.texture_stats(), own, "something the result screen does not draw from is still uploaded");
        assert_eq!(app.shared.skin_screens.textures.unheld(), 0);
    }
    println!("select alone had {} textures ({} RGBA bytes) uploaded", browser_alone.count, browser_alone.bytes);
    let _ = std::fs::remove_dir_all(&home);
}
