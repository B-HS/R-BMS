//! What a screen draws once a document is selected for it, and what it draws when none is.
//!
//! The gate is one branch per screen, so both sides of it are worth pinning: with a document the
//! screen's own layout must stand aside entirely, and with none the frame must be byte for byte the
//! frame the built-in screen draws.

use std::path::{Path, PathBuf};

use rbms_skin::loader::{SKIN_TYPE_DECIDE, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_RESULT};

use crate::stage::capture::{Missed, SCENE_START_US, Shot, app_in, draw_at, draw_until_compiled, settings_of, skin_folder_of};
use crate::stage::render_tests::{play_state_with_bga, render, result_state};
use crate::stage::{DecideState, HeadlessCanvas, LoadingState, SelectState, Stage};
use crate::{App, CH, CW, Color, Config};

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
    let mut config = Config::default();
    if let Some((screen, document)) = selected {
        config.skin.select(screen, Some(document.to_string_lossy().into_owned()));
    }
    app_in(settings, config)
}

/// An app in a folder of this test's own whose `screen` is drawn with the flat document.
fn app_drawing(tag: &str, screen: i32) -> App {
    let settings = settings_of(tag);
    let document = write_document(&settings, screen);
    app_with(settings, Some((screen, &document)))
}

/// Draws the stage `stage` builds onto `pixels` until the document for `screen` has compiled;
/// answers whether it compiled in time. None of these documents animate, so every frame is drawn at
/// the moment the scene began.
fn render_until_compiled(app: &mut App, screen: i32, pixels: &mut HeadlessCanvas, stage: impl Fn() -> Stage) -> bool {
    draw_until_compiled(app, screen, SCENE_START_US, pixels, stage)
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

/// The score screen and the decide screen hand their whole frame to a document the same way the
/// browser does: nothing of a built-in layout is left under or over it.
#[test]
fn the_score_and_decide_screens_are_drawn_by_their_documents_alone() {
    let mut score = app_drawing("score", SKIN_TYPE_RESULT);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(render_until_compiled(&mut score, SKIN_TYPE_RESULT, &mut pixels, || Stage::Result(result_state())), "the score document compiles in time");
    assert_drawn_by_the_document_alone(&pixels, "score");

    let mut decide = app_drawing("decide", SKIN_TYPE_DECIDE);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let stage = || Stage::Decide(Box::new(DecideState::song(0)));
    assert!(render_until_compiled(&mut decide, SKIN_TYPE_DECIDE, &mut pixels, stage), "the decide document compiles in time");
    assert_drawn_by_the_document_alone(&pixels, "decide");
}

/// Frames the loading screen is drawn for below: more than the two a document takes to be asked for
/// and read.
const LOADING_FRAMES: usize = 4;

/// The LOADING screen is the built-in one whatever documents are chosen: a decide document draws
/// the decide scene, and what the loading screen waits for after it -- or instead of it, for a scan
/// or a table -- is not that scene.
#[test]
fn the_loading_screen_is_never_drawn_by_the_decide_document() {
    let stage = || Stage::Loading(LoadingState::song(0));
    let built_in = render(&mut crate::stage::render_tests::app(), stage());

    let mut app = app_drawing("loading", SKIN_TYPE_DECIDE);
    let mut pixels = HeadlessCanvas::new(CW, CH);
    for _ in 0..LOADING_FRAMES {
        draw_at(&mut app, stage(), SCENE_START_US, &mut pixels);
    }
    assert!(!app.shared.has_skin_document(SKIN_TYPE_DECIDE), "the loading screen asked for the decide document");
    assert_eq!(pixels.pixel_checksum(), built_in.pixel_checksum(), "the loading screen is not the built-in one");
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
    draw_at(&mut app, browser(), SCENE_START_US, &mut pixels);
    assert!(app.shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT), "the screen has a document from the frame it was chosen on");
    if !app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        assert_ne!(pixels.pixel_at(0, 0), MARK, "a frame drawn before the files arrived drew the built-in list");
    }

    assert!(render_until_compiled(&mut app, SKIN_TYPE_MUSIC_SELECT, &mut pixels, browser), "the document compiles within the frame budget");
    assert_eq!(pixels.pixel_at(0, 0), MARK, "once the files arrive the document takes the screen");
}

/// A document screen is captured on either backend, each from an app of its own: a compiled document
/// holds textures registered with the target it was compiled against, so the app one target drew
/// with has nothing the other can draw from.
#[test]
fn a_document_screen_is_captured_alike_on_both_backends() {
    let shot = Shot { name: "document-select", size: (CW, CH), scene_us: SCENE_START_US, document: Some(SKIN_TYPE_MUSIC_SELECT) };

    let mut headless_app = app_drawing("shot-headless", SKIN_TYPE_MUSIC_SELECT);
    let pixels = shot.take(&mut headless_app, browser).expect("the document compiles within the frame budget");
    assert_drawn_by_the_document_alone(&pixels, "browser");

    let mut gpu_app = app_drawing("shot-gpu", SKIN_TYPE_MUSIC_SELECT);
    let from_gpu = match shot.take_on_gpu(&mut gpu_app, browser) {
        Err(Missed::NoAdapter) => return,
        taken => taken.expect("the document compiles within the frame budget"),
    };
    let mark = [MARK.r, MARK.g, MARK.b, MARK.a];
    assert!(from_gpu.chunks_exact(mark.len()).all(|pixel| pixel == mark), "the GPU drew something other than the document over the whole target");
}

/// A target half as large again as the size the unresized fixture is authored at.
const LARGE: (u32, u32) = (1920, 1080);

/// Size of the image the unresized fixture draws, in the image's own pixels.
const STAMP: (u32, u32) = (40, 20);

/// Edge of the square the unresized fixture gives that image, centred on the document: roomier than
/// the image on both axes, at the authored size and on the larger target alike.
const STAMP_BOX: u32 = 80;

/// The `stretch` value that draws an image at its own pixel size, centred in its destination.
const STRETCH_NO_RESIZE: i32 = 9;

/// A browser document authored at the size the built-in screens are laid out for, drawing one image
/// that asks not to be resized.
fn unresized_document() -> String {
    let (stamp_w, stamp_h) = STAMP;
    let (x, y) = ((CW - STAMP_BOX) / 2, (CH - STAMP_BOX) / 2);
    format!(
        r#"{{
            "type": {SKIN_TYPE_MUSIC_SELECT},
            "name": "unresized",
            "w": {CW},
            "h": {CH},
            "source": [{{ "id": "stamp", "path": "stamp.png" }}],
            "image": [{{ "id": "stamp", "src": "stamp", "x": 0, "y": 0, "w": {stamp_w}, "h": {stamp_h} }}],
            "destination": [
                {{ "id": "stamp", "stretch": {STRETCH_NO_RESIZE}, "dst": [{{ "time": 0, "x": {x}, "y": {y}, "w": {STAMP_BOX}, "h": {STAMP_BOX} }}] }}
            ]
        }}"#
    )
}

/// A document is drawn on the target's own pixels, not laid out for the built-in screens' size and
/// then enlarged with them: on a target half as large again as the document, an image that asks not
/// to be resized still covers exactly as many pixels as it has. Enlarged after the fact it would
/// cover half as many again on each axis.
#[test]
fn a_document_is_laid_out_in_the_targets_own_pixels() {
    let settings = settings_of("unresized");
    let folder = skin_folder_of(&settings);
    image::RgbaImage::from_pixel(STAMP.0, STAMP.1, image::Rgba([MARK.r, MARK.g, MARK.b, MARK.a])).save(folder.join("stamp.png")).expect("the image is written");
    let document = folder.join("unresized.json");
    std::fs::write(&document, unresized_document()).expect("the document is written");
    let mut app = app_with(settings, Some((SKIN_TYPE_MUSIC_SELECT, &document)));

    let shot = Shot { name: "document-unresized-1920x1080", size: LARGE, scene_us: SCENE_START_US, document: Some(SKIN_TYPE_MUSIC_SELECT) };
    let pixels = shot.take(&mut app, browser).expect("the document compiles within the frame budget");

    let (left, top) = ((LARGE.0 - STAMP.0) / 2, (LARGE.1 - STAMP.1) / 2);
    let (right, bottom) = (left + STAMP.0, top + STAMP.1);
    assert_eq!(pixels.pixel_at(left, top), MARK, "the image does not start where its own size, centred, puts it");
    assert_eq!(pixels.pixel_at(right - 1, bottom - 1), MARK, "the image does not reach as far as its own size");
    for (x, y) in [(left - 1, top), (left, top - 1), (right, bottom - 1), (right - 1, bottom)] {
        assert_ne!(pixels.pixel_at(x, y), MARK, "the image was drawn larger than its own pixels: it covers {x},{y}");
    }
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
