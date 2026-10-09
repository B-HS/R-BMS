//! What a screen draws once a document is selected for it, and what it draws when none is.
//!
//! The gate is one branch per screen, so both sides of it are worth pinning: with a document the
//! screen's own layout must stand aside entirely, and with none the frame must be byte for byte the
//! frame the built-in screen draws.

use std::path::{Path, PathBuf};

use rbms_config::DEFAULT_SKIN_FOLDER;
use rbms_skin::loader::{SKIN_TYPE_DECIDE, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_RESULT};

use crate::stage::loading::LoadingState;
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

/// The file the fixture document is written to inside the skin folder.
const DOCUMENT_FILE: &str = "flat.json";

/// A document for one screen with one image source stretched over everything it is given, so a
/// frame it drew is one flat colour and nothing else can be mistaken for it.
fn flat_document(screen: i32) -> String {
    format!(
        r#"{{
            "type": {screen},
            "name": "flat",
            "w": 128,
            "h": 72,
            "source": [{{ "id": "mark", "path": "mark.png" }}],
            "image": [{{ "id": "sheet", "src": "mark", "x": 0, "y": 0, "w": {SOURCE_DIM}, "h": {SOURCE_DIM} }}],
            "destination": [{{ "id": "sheet", "dst": [{{ "time": 0, "x": 0, "y": 0, "w": 128, "h": 72 }}] }}]
        }}"#
    )
}

/// The settings file of a folder that belongs to one test, so writing a skin folder beside it
/// cannot disturb the other render snapshots.
fn settings_of(tag: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("rbms-document-render-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join(DEFAULT_SKIN_FOLDER)).expect("the fixture folder is writable");
    directory.join("settings.ron")
}

/// The skin folder beside one test's settings file.
fn skin_folder_of(settings: &Path) -> PathBuf {
    settings.parent().unwrap_or(Path::new(".")).join(DEFAULT_SKIN_FOLDER)
}

/// Write the flat document for `screen` and its image into the skin folder beside `settings`, and
/// answer the document.
fn write_document(settings: &Path, screen: i32) -> PathBuf {
    let folder = skin_folder_of(settings);
    let pixels = image::RgbaImage::from_pixel(SOURCE_DIM, SOURCE_DIM, image::Rgba([MARK.r, MARK.g, MARK.b, MARK.a]));
    pixels.save(folder.join("mark.png")).expect("the source image is written");
    let document = folder.join(DOCUMENT_FILE);
    std::fs::write(&document, flat_document(screen)).expect("the document is written");
    document
}

/// An app reading `settings`, with `selected` chosen as the document one screen is drawn with.
fn app_with(settings: PathBuf, selected: Option<(i32, &Path)>) -> App {
    rbms_render::font::use_embedded_fonts_only();
    let mut config = Config::default();
    if let Some((screen, document)) = selected {
        config.skin.select(screen, Some(document.to_string_lossy().into_owned()));
    }
    App::new(String::new(), config, LaunchOptions::default(), settings)
}

/// An app in a folder of this test's own whose `screen` is drawn with the flat document.
fn app_drawing(tag: &str, screen: i32) -> App {
    let settings = settings_of(tag);
    let document = write_document(&settings, screen);
    app_with(settings, Some((screen, &document)))
}

/// Draws the stage `stage` builds onto `pixels` until the document for `screen` has compiled, then
/// draws one more frame so the canvas shows the document rather than the last built-in frame before
/// it; answers whether it compiled in time.
///
/// One canvas throughout: the compiled document registers its textures with the target it was built
/// against, so a frame drawn onto a different canvas would find none of them. A fresh stage per
/// frame, because that is the only way a screen whose own state advances -- the loading screen
/// starts its work on its second frame -- can be drawn as many times as a document takes to arrive.
fn render_until_compiled(app: &mut App, screen: i32, pixels: &mut HeadlessCanvas, stage: impl Fn() -> Stage) -> bool {
    for _ in 0..DOCUMENT_LOAD_FRAMES {
        render_into(app, stage(), pixels);
        if app.shared.has_compiled_skin(screen) {
            render_into(app, stage(), pixels);
            return true;
        }
    }
    false
}

/// A fresh song browser.
fn browser() -> Stage {
    Stage::Select(Box::new(SelectState::new()))
}

/// Holds a frame to being the flat document and nothing else: every corner and the centre are the
/// document's own colour, which no built-in screen paints.
fn assert_drawn_by_the_document_alone(pixels: &HeadlessCanvas, screen: &str) {
    for (x, y) in [(0, 0), (CW - 1, 0), (0, CH - 1), (CW - 1, CH - 1), (CW / 2, CH / 2)] {
        assert_eq!(pixels.pixel_at(x, y), MARK, "the built-in {screen} screen is still showing at {x},{y}");
    }
}

/// A document selected for the browser takes the whole screen.
#[test]
fn the_browser_draws_its_document_instead_of_its_own_list() {
    let mut app = app_drawing("drawn", SKIN_TYPE_MUSIC_SELECT);
    let mut pixels = HeadlessCanvas::new(CW, CH);

    assert!(render_until_compiled(&mut app, SKIN_TYPE_MUSIC_SELECT, &mut pixels, browser), "the document compiles within the frame budget");
    assert_drawn_by_the_document_alone(&pixels, "browser");
}

/// The score screen and the loading screen hand their whole frame to a document the same way the
/// browser does: nothing of the built-in layout is left under or over it.
#[test]
fn the_score_and_loading_screens_are_drawn_by_their_documents_alone() {
    let mut score = app_drawing("score", SKIN_TYPE_RESULT);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(render_until_compiled(&mut score, SKIN_TYPE_RESULT, &mut pixels, || Stage::Result(result_state())), "the score document compiles in time");
    assert_drawn_by_the_document_alone(&pixels, "score");

    let mut loading = app_drawing("loading", SKIN_TYPE_DECIDE);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let stage = || Stage::Loading(LoadingState::song(0));
    assert!(render_until_compiled(&mut loading, SKIN_TYPE_DECIDE, &mut pixels, stage), "the loading document compiles in time");
    assert_drawn_by_the_document_alone(&pixels, "loading");
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
    let mut app = app_drawing("pending", SKIN_TYPE_MUSIC_SELECT);

    let mut pixels = HeadlessCanvas::new(CW, CH);
    render_into(&mut app, browser(), &mut pixels);
    assert!(app.shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT), "the screen has a document from the frame it was chosen on");
    if !app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        assert_ne!(pixels.pixel_at(0, 0), MARK, "a frame drawn before the files arrived drew the built-in list");
    }

    assert!(render_until_compiled(&mut app, SKIN_TYPE_MUSIC_SELECT, &mut pixels, browser), "the document compiles within the frame budget");
    assert_eq!(pixels.pixel_at(0, 0), MARK, "once the files arrive the document takes the screen");
}

/// The same screen with nothing selected draws exactly the frame the built-in screen draws, whether
/// or not a document lies in the skin folder.
#[test]
fn a_screen_with_no_document_draws_the_frame_it_always_drew() {
    let mut without = app_with(settings_of("fallback"), None);
    let baseline = render(&mut without, browser());

    let settings = settings_of("unselected");
    write_document(&settings, SKIN_TYPE_MUSIC_SELECT);
    let mut unselected = app_with(settings, None);
    let drawn = render(&mut unselected, browser());

    assert_eq!(drawn.pixel_checksum(), baseline.pixel_checksum(), "a document on disk that nothing selected changed the frame");
    assert_ne!(drawn.pixel_at(0, 0), MARK, "the built-in list drew the document's colour");
}

/// The gate is per screen: a document selected for the browser is not drawn over the score screen,
/// which would be the mistake of keying the compiled screens by anything but their type.
#[test]
fn a_document_selected_for_one_screen_is_not_drawn_over_another() {
    let mut app = app_drawing("other", SKIN_TYPE_MUSIC_SELECT);
    let mut baseline_app = app_with(settings_of("other-baseline"), None);

    let pixels = render(&mut app, Stage::Result(result_state()));
    let baseline = render(&mut baseline_app, Stage::Result(result_state()));
    assert_ne!(pixels.pixel_at(0, 0), MARK, "the browser's document reached the score screen");
    assert_eq!(pixels.pixel_checksum(), baseline.pixel_checksum(), "a document selected for the browser changed the score screen");
}

/// The colour the play fixture's background image is painted, distinct from the document's own mark.
const BGA_MARK: Color = Color::rgb(220, 40, 130);

/// Edge of the background image the play fixture hands the screen.
const BGA_DIM: u32 = 4;

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
    let settings = settings_of("bga");
    let document = skin_folder_of(&settings).join("bga.json");
    std::fs::write(&document, PLAY_DOCUMENT).expect("the document is written");
    let mut app = app_with(settings, Some((SKIN_TYPE_PLAY_7KEYS, &document)));

    let pixels: Vec<u8> = (0..BGA_DIM * BGA_DIM).flat_map(|_| [BGA_MARK.r, BGA_MARK.g, BGA_MARK.b, BGA_MARK.a]).collect();
    let mut frames = std::collections::HashMap::new();
    frames.insert(crate::stage::play::NO_BGA_FRAME, crate::DecodedImage::for_test(pixels, BGA_DIM, BGA_DIM));

    let mut canvas = HeadlessCanvas::new(CW, CH);
    let stage = || Stage::Play(Box::new(play_state_with_bga(frames.clone())));
    assert!(render_until_compiled(&mut app, SKIN_TYPE_PLAY_7KEYS, &mut canvas, stage), "the play document never finished compiling");
    assert_eq!(canvas.pixel_at(CW / 2, CH / 2), BGA_MARK, "the document's bga object drew nothing");
}
