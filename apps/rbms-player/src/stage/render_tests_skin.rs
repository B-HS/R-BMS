//! What a screen draws once a document is selected for it, and what it draws when none is.
//!
//! The gate is one branch per screen, so both sides of it are worth pinning: with a document the
//! screen's own layout must stand aside entirely, and with none the frame must be byte for byte the
//! frame the screen drew before any of this existed.

use std::path::{Path, PathBuf};

use rbms_config::DEFAULT_SKIN_FOLDER;
use rbms_skin::loader::{SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS};

use crate::stage::render_tests::{play_state_with_bga, render, render_into, result_state};
use crate::stage::{HeadlessCanvas, SelectState, Stage};
use crate::{App, CH, CW, Color, Config, LaunchOptions};

/// Frames a test draws while a document's files are read on the worker pool.
///
/// A document is compiled off the frame loop, so the frame it is selected on still draws the
/// built-in layout and the document takes over once its images have arrived. Four seconds of frames
/// is far more than a two-pixel image needs and still fails rather than hanging if the pool never
/// finishes.
const DOCUMENT_LOAD_FRAMES: usize = 240;

/// The colour the fixture document paints its one image with, picked so no built-in screen draws it.
const MARK: Color = Color::rgb(12, 200, 90);

/// Edge of the fixture's image source, which the document stretches over the whole screen.
const SOURCE_DIM: u32 = 2;

/// A browser document with one image source stretched over everything it is given, so a frame it
/// drew is one flat colour and nothing else can be mistaken for it.
const DOCUMENT: &str = r#"{
    "type": 5,
    "name": "flat",
    "w": 128,
    "h": 72,
    "source": [{ "id": "mark", "path": "mark.png" }],
    "image": [{ "id": "sheet", "src": "mark", "x": 0, "y": 0, "w": 2, "h": 2 }],
    "destination": [{ "id": "sheet", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 128, "h": 72 }] }]
}"#;

/// Write the document and its image into a skin folder beside `settings`, and answer the document.
fn write_document(settings: &Path) -> PathBuf {
    let folder = settings.parent().unwrap_or(Path::new(".")).join(DEFAULT_SKIN_FOLDER);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the fixture folder is writable");
    let pixels = image::RgbaImage::from_pixel(SOURCE_DIM, SOURCE_DIM, image::Rgba([MARK.r, MARK.g, MARK.b, MARK.a]));
    pixels.save(folder.join("mark.png")).expect("the source image is written");
    let document = folder.join("flat.json");
    std::fs::write(&document, DOCUMENT).expect("the document is written");
    document
}

/// An app whose settings live in a folder of this test's own, so writing a skin folder beside them
/// cannot disturb the other render snapshots.
fn app_in(tag: &str, select: Option<PathBuf>) -> App {
    rbms_render::font::use_embedded_fonts_only();
    let dir = std::env::temp_dir().join(format!("rbms-skin-render-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the fixture folder is writable");
    let settings = dir.join("settings.ron");
    let mut config = Config::default();
    if let Some(document) = select {
        config.skin.select(SKIN_TYPE_MUSIC_SELECT, Some(document.to_string_lossy().into_owned()));
    }
    App::new(String::new(), config, LaunchOptions::default(), settings)
}

/// Draws the browser until its document has been compiled, answering the last frame either way.
///
/// One canvas throughout: the compiled document registers its textures with the target it was built
/// against, so a frame drawn onto a different canvas would find none of them.
fn render_until_compiled(app: &mut App) -> HeadlessCanvas {
    let mut pixels = HeadlessCanvas::new(CW, CH);
    render_until_compiled_into(app, &mut pixels);
    pixels
}

/// Draws the browser onto `pixels` until its document has been compiled, then draws one more frame
/// so the returned canvas shows the document; answers whether the document compiled in time.
fn render_until_compiled_into(app: &mut App, pixels: &mut HeadlessCanvas) -> bool {
    for _ in 0..DOCUMENT_LOAD_FRAMES {
        render_into(app, Stage::Select(Box::new(SelectState::new())), pixels);
        if app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
            render_into(app, Stage::Select(Box::new(SelectState::new())), pixels);
            return true;
        }
    }
    false
}

/// A document selected for the browser takes the whole screen: every corner is the document's own
/// colour, which the built-in list never paints.
#[test]
fn the_browser_draws_its_document_instead_of_its_own_list() {
    let settings = std::env::temp_dir().join(format!("rbms-skin-render-drawn-{}", std::process::id())).join("settings.ron");
    let document = write_document(&settings);
    let mut app = app_in("drawn", Some(document));

    let pixels = render_until_compiled(&mut app);
    for (x, y) in [(0, 0), (CW - 1, 0), (0, CH - 1), (CW - 1, CH - 1), (CW / 2, CH / 2)] {
        assert_eq!(pixels.pixel_at(x, y), MARK, "the built-in list is still showing at {x},{y}");
    }
}

/// A document's files are read on the worker pool, so a frame drawn before they arrive draws the
/// built-in layout rather than waiting. The screen still counts as having a document throughout, so
/// nothing else moves under it while it loads -- the chart's background in particular stays out of
/// the built-in slot the document is about to claim.
///
/// The fixture is two pixels, so the read can perfectly well finish inside the first frame; what is
/// asserted is the contract either way, not the timing.
#[test]
fn a_screen_keeps_drawing_while_its_document_is_read() {
    let settings = std::env::temp_dir().join(format!("rbms-skin-render-pending-{}", std::process::id())).join("settings.ron");
    let document = write_document(&settings);
    let mut app = app_in("pending", Some(document));

    let mut pixels = HeadlessCanvas::new(CW, CH);
    render_into(&mut app, Stage::Select(Box::new(SelectState::new())), &mut pixels);
    assert!(app.shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT), "the screen has a document from the frame it was chosen on");
    if !app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        assert_ne!(pixels.pixel_at(0, 0), MARK, "a frame drawn before the files arrived drew the built-in list");
    }

    assert!(render_until_compiled_into(&mut app, &mut pixels), "the document compiles within the frame budget");
    assert_eq!(pixels.pixel_at(0, 0), MARK, "once the files arrive the document takes the screen");
}

/// The same screen with nothing selected draws exactly the frame it drew before the gate existed,
/// which is what "the built-in screens keep the layout they always had" has to mean.
#[test]
fn a_screen_with_no_document_draws_the_frame_it_always_drew() {
    let mut without = app_in("fallback", None);
    let baseline = render(&mut without, Stage::Select(Box::new(SelectState::new())));

    let settings = std::env::temp_dir().join(format!("rbms-skin-render-unselected-{}", std::process::id())).join("settings.ron");
    write_document(&settings);
    let mut unselected = app_in("unselected", None);
    let drawn = render(&mut unselected, Stage::Select(Box::new(SelectState::new())));

    assert_eq!(drawn.pixel_checksum(), baseline.pixel_checksum(), "a document on disk that nothing selected changed the frame");
    assert_ne!(drawn.pixel_at(0, 0), MARK, "the built-in list drew the document's colour");
}

/// The gate is per screen: a document selected for the browser is not drawn over the score screen,
/// which would be the mistake of keying the compiled screens by anything but their type.
#[test]
fn a_document_selected_for_one_screen_is_not_drawn_over_another() {
    let settings = std::env::temp_dir().join(format!("rbms-skin-render-other-{}", std::process::id())).join("settings.ron");
    let document = write_document(&settings);
    let mut app = app_in("other", Some(document));

    let pixels = render(&mut app, Stage::Result(result_state()));
    assert_ne!(pixels.pixel_at(0, 0), MARK, "the browser's document reached the score screen");
}

/// The colour the play fixture's background image is painted, distinct from the document's own mark.
const BGA_MARK: Color = Color::rgb(220, 40, 130);

/// A play document that draws nothing but the chart's background image, so a frame it drew is that
/// image and nothing else.
const PLAY_DOCUMENT: &str = r#"{
    "type": 0,
    "name": "bga only",
    "w": 128,
    "h": 72,
    "bga": { "id": "backdrop" },
    "destination": [{ "id": "backdrop", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 128, "h": 72 }] }]
}"#;

/// A chart's background frame reaches a document's own `bga` object.
///
/// The document owns the whole screen once it is selected, so the built-in layout's background slot
/// is left empty and the image is handed to the document instead. Without that wiring the frame
/// would have no background at all: neither path would draw it.
#[test]
fn a_play_document_draws_the_charts_background_through_its_own_bga_object() {
    let dir = std::env::temp_dir().join(format!("rbms-skin-render-bga-{}", std::process::id()));
    let folder = dir.join(DEFAULT_SKIN_FOLDER);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the fixture folder is writable");
    let document = folder.join("bga.json");
    std::fs::write(&document, PLAY_DOCUMENT).expect("the document is written");

    rbms_render::font::use_embedded_fonts_only();
    let mut config = Config::default();
    config.skin.select(SKIN_TYPE_PLAY_7KEYS, Some(document.to_string_lossy().into_owned()));
    let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));

    let side = 4;
    let pixels: Vec<u8> = (0..side * side).flat_map(|_| [BGA_MARK.r, BGA_MARK.g, BGA_MARK.b, 255]).collect();
    let mut frames = std::collections::HashMap::new();
    frames.insert(crate::stage::play::NO_BGA_FRAME, crate::DecodedImage::for_test(pixels, side, side));

    let mut canvas = HeadlessCanvas::new(CW, CH);
    for _ in 0..DOCUMENT_LOAD_FRAMES {
        render_into(&mut app, Stage::Play(Box::new(play_state_with_bga(frames.clone()))), &mut canvas);
        if app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS) {
            render_into(&mut app, Stage::Play(Box::new(play_state_with_bga(frames.clone()))), &mut canvas);
            break;
        }
    }

    assert!(app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS), "the play document never finished compiling");
    assert_eq!(canvas.pixel_at(CW / 2, CH / 2), BGA_MARK, "the document's bga object drew nothing");
}
