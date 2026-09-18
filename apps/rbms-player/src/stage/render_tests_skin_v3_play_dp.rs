//! The shipped bundle's two-field play documents, held to compiling clean and to standing in for
//! what they claim.
//!
//! Ten and fourteen keys read the two-field note layout rather than the single-field one, so a
//! document that was written against the wrong one shows up here rather than on the play screens
//! beside it.
//!
//! Past compiling, three things are checked against the frame the play screen really draws. Every
//! block the document names in `replace` has to have the objects that block demands, or half the
//! screen is drawn twice. The lane rectangles the document states have to reach the resolved field,
//! because the covers, the key bombs and the HUD all read that one field. And the chart art has to
//! survive the document's own background, which is the fault this bundle has had before.

use std::collections::HashMap;

use rbms_model::Mode;

use crate::stage::render_tests::{play_state, play_state_with_bga};
use crate::stage::render_tests_skin::{assert_bundled_document_compiles_clean, bundled_app, render_until_screen_compiled};
use crate::stage::{HeadlessCanvas, Stage};
use crate::{App, CH, CW, Color, DecodedImage, SkinConfig, assets};

/// The colour the fake chart art is handed to the play screen in, picked so nothing the document
/// paints can be mistaken for it.
const ART: Color = Color::rgb(220, 40, 130);

/// Edge of that art, which the screen stretches over whatever rectangle it draws the chart art in.
const ART_SIDE: u32 = 4;

/// A pixel well inside the upper chart-art pane both documents place on the left, from the
/// specification's `(14, 168, 232, 174)`.
const ART_PROBE: (u32, u32) = (130, 255);

/// How bright a channel has to be before a probe counts the pixel as ink the document put there.
const BRIGHT: u8 = 170;

/// How light every channel has to be before a probe counts a pixel as one of the field's own white
/// notes, which nothing else inside a lane rectangle draws.
const NOTE_WHITE: u8 = 230;

/// A scroll speed slow enough to bring the fixture chart's first notes inside the field, which the
/// speed a player starts on does not: the chart's first note is a measure and a quarter in.
const SLOW_SCROLL: f64 = 0.4;

/// Where the fourteen-key document puts its lanes, as `(x, w)` in lane order: seven keys and the
/// scratch on the left, then the same on the right with the scratch outermost.
const FOURTEEN_KEY_LANES: [(f32, f32); 16] = [
    (364.0, 40.0),
    (404.0, 32.0),
    (436.0, 40.0),
    (476.0, 32.0),
    (508.0, 40.0),
    (548.0, 32.0),
    (580.0, 40.0),
    (300.0, 64.0),
    (674.0, 40.0),
    (714.0, 32.0),
    (746.0, 40.0),
    (786.0, 32.0),
    (818.0, 40.0),
    (858.0, 32.0),
    (890.0, 40.0),
    (930.0, 64.0),
];

/// The same for ten keys: five keys a side.
const TEN_KEY_LANES: [(f32, f32); 12] = [
    (400.0, 40.0),
    (440.0, 32.0),
    (472.0, 40.0),
    (512.0, 32.0),
    (544.0, 40.0),
    (336.0, 64.0),
    (710.0, 40.0),
    (750.0, 32.0),
    (782.0, 40.0),
    (822.0, 32.0),
    (854.0, 40.0),
    (894.0, 64.0),
];

/// The two note fields each document states, as `(x, w)`.
const FOURTEEN_KEY_FIELDS: [(f32, f32); 2] = [(300.0, 320.0), (674.0, 320.0)];
const TEN_KEY_FIELDS: [(f32, f32); 2] = [(336.0, 248.0), (710.0, 248.0)];

/// Where the field starts and ends on screen, and how tall one note is, from the specification.
const FIELD_TOP: f32 = 0.0;
const JUDGE_LINE: f32 = 500.0;
const NOTE_HEIGHT: f32 = 22.0;

/// The panels both documents fill in place of the built-in HUD, each well inside the rectangle the
/// bundle's frame leaves clear for it.
const INFORMATION_PROBE: (u32, u32, u32, u32) = (14, 8, 232, 150);
const COUNTS_PROBE: (u32, u32, u32, u32) = (14, 540, 232, 170);
const PACEMAKER_PROBE: (u32, u32, u32, u32) = (1008, 8, 264, 704);
const GAUGE_PROBE: (u32, u32, u32, u32) = (460, 572, 360, 22);

/// The row of readings under each field, and how far a probe looks into the field itself.
const READING_ROW: (u32, u32) = (602, 28);
const INSIDE_FIELD: u32 = 6;
const FIELD_ROW: u32 = 250;

/// How far past a field a probe looks for a note that should not be there, and how far short of the
/// judgement line a probe stops looking for one.
const OUTSIDE_FIELD: u32 = 12;
const ABOVE_JUDGEMENT: u32 = 8;

/// Points one app at the play layout and document of `mode`, the way choosing a chart of that mode
/// would. Both double modes read the two-field layout rather than the single-field one.
fn play_document_of(tag: &str, mode: Mode) {
    let (mut app, settings) = bundled_app(tag);
    app.shared.mode = mode;
    app.shared.skin_cfg = SkinConfig::load(assets::installed_play_skin_path(&settings, &app.shared.config, mode)).expect("the bundled play layout parses");
    app.shared.rebuild_skin();
    let screen = rbms_skin::loader::mode_skin_type(mode).expect("the mode has a skin type");
    assert_bundled_document_compiles_clean(&mut app, screen, || Stage::Play(Box::new(play_state())));
}

/// An app playing `mode` off the bundle, with its document compiled and chart art on screen, and
/// the last frame it drew.
fn playing(tag: &str, mode: Mode) -> (App, HeadlessCanvas) {
    playing_at(tag, mode, None)
}

/// The same, at a scroll speed of the caller's own when it needs the chart's notes on screen.
fn playing_at(tag: &str, mode: Mode, hispeed: Option<f64>) -> (App, HeadlessCanvas) {
    let (mut app, settings) = bundled_app(tag);
    app.shared.mode = mode;
    if let Some(hispeed) = hispeed {
        app.shared.config.play.hispeed = hispeed;
    }
    app.shared.skin_cfg = SkinConfig::load(assets::installed_play_skin_path(&settings, &app.shared.config, mode)).expect("the bundled play layout parses");
    app.shared.rebuild_skin();

    let rgba: Vec<u8> = (0..ART_SIDE * ART_SIDE).flat_map(|_| [ART.r, ART.g, ART.b, u8::MAX]).collect();
    let mut frames = HashMap::new();
    frames.insert(crate::stage::play::NO_BGA_FRAME, DecodedImage::for_test(rgba, ART_SIDE, ART_SIDE));

    let screen = rbms_skin::loader::mode_skin_type(mode).expect("the mode has a skin type");
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let compiled = render_until_screen_compiled(&mut app, screen, &mut pixels, || Stage::Play(Box::new(play_state_with_bga(frames.clone()))));
    assert!(compiled, "the two-field play document for {} never finished compiling", mode.name);
    (app, pixels)
}

/// Whether any pixel of `rect`, written as `(x, y, w, h)`, is bright enough to be ink the document
/// put there.
///
/// A probe rather than an exact colour because everything the document draws is tinted and blended
/// against whatever panel is behind it: what is asserted is that something light reached the
/// rectangle at all.
fn has_bright_pixel(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> bool {
    has_pixel_over(pixels, rect, BRIGHT)
}

/// Whether any pixel of `rect` is one of the note sprite's own whites.
fn has_white_note(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> bool {
    has_pixel_over(pixels, rect, NOTE_WHITE)
}

/// Whether any pixel of `rect` has every channel at or over `floor`.
fn has_pixel_over(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32), floor: u8) -> bool {
    let (x, y, w, h) = rect;
    (y..y + h).any(|row| {
        (x..x + w).any(|column| {
            let pixel = pixels.pixel_at(column, row);
            pixel.r >= floor && pixel.g >= floor && pixel.b >= floor
        })
    })
}

/// Saves the frame for the eye to check, when a capture directory was asked for.
fn save(name: &str, pixels: &HeadlessCanvas) {
    let Some(directory) = std::env::var_os("RBMS_SKIN_CAPTURE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).expect("create the capture folder");
    let image = image::RgbaImage::from_fn(CW, CH, |x, y| {
        let pixel = pixels.pixel_at(x, y);
        image::Rgba([pixel.r, pixel.g, pixel.b, pixel.a])
    });
    image.save(directory.join(format!("{name}.png"))).expect("save the capture");
}

#[test]
fn the_bundles_ten_key_document_compiles_without_a_warning() {
    play_document_of("v3-play-10k", Mode::BEAT_10K);
}

#[test]
fn the_bundles_fourteen_key_document_compiles_without_a_warning() {
    play_document_of("v3-play-14k", Mode::BEAT_14K);
}

/// Every block the two documents list in `replace` has the objects that block demands, so the play
/// screen really does stand aside for all eight rather than drawing its own field under the
/// document's.
#[test]
fn the_two_field_documents_stand_in_for_every_block_they_name() {
    for (tag, mode) in [("v3-dp-blocks-10k", Mode::BEAT_10K), ("v3-dp-blocks-14k", Mode::BEAT_14K)] {
        let (app, _pixels) = playing(tag, mode);
        let screen = rbms_skin::loader::mode_skin_type(mode).expect("the mode has a skin type");
        let content = app.shared.screen_content(screen).play;
        assert!(content.field, "{}: the document did not take the note field over", mode.name);
        assert!(content.gauge, "{}: the document did not take the gauge over", mode.name);
        assert!(content.judge, "{}: the document did not take the judgement pop-up over", mode.name);
        assert!(content.score, "{}: the document did not take the score readings over", mode.name);
        assert!(content.counts, "{}: the document did not take the judgement counts over", mode.name);
        assert!(content.graph, "{}: the document did not take the pacemaker graph over", mode.name);
        assert!(content.cover, "{}: the document did not take the lane covers over", mode.name);
        assert!(content.frame, "{}: the document did not take the frame over", mode.name);
    }
}

/// The lane rectangles the documents state are the ones the run is played on, so the covers, the key
/// bombs and the HUD all land on the notes the document draws.
#[test]
fn the_two_field_documents_state_the_field_the_run_is_played_on() {
    for (tag, mode, lanes, fields) in [
        ("v3-dp-field-10k", Mode::BEAT_10K, TEN_KEY_LANES.as_slice(), TEN_KEY_FIELDS.as_slice()),
        ("v3-dp-field-14k", Mode::BEAT_14K, FOURTEEN_KEY_LANES.as_slice(), FOURTEEN_KEY_FIELDS.as_slice()),
    ] {
        let (app, _pixels) = playing(tag, mode);
        let field = &app.shared.skin;
        let stated: Vec<(f32, f32)> = field.x.iter().copied().zip(field.w.iter().copied()).collect();
        assert_eq!(stated, lanes, "{}: the resolved lanes are not the ones the document states", mode.name);
        assert_eq!(field.fields, fields, "{}: the two note fields are not where the document puts them", mode.name);
        assert_eq!(field.top_y, FIELD_TOP, "{}: the field does not start where the document states", mode.name);
        assert_eq!(field.judge_y, JUDGE_LINE, "{}: the judgement line is not where the document states", mode.name);
        assert_eq!(field.note_height, NOTE_HEIGHT, "{}: a note is not as tall as the document states", mode.name);
    }
}

/// The frame the documents draw: chart art survives on the left, both fields are lit, and the
/// readings on either side of them reach the screen.
#[test]
fn the_two_field_documents_light_both_fields_and_leave_the_chart_art_alone() {
    for (tag, name, mode, fields) in
        [("v3-dp-frame-10k", "v3-play-10k", Mode::BEAT_10K, TEN_KEY_FIELDS), ("v3-dp-frame-14k", "v3-play-14k", Mode::BEAT_14K, FOURTEEN_KEY_FIELDS)]
    {
        let (_app, pixels) = playing(tag, mode);
        save(name, &pixels);

        assert_eq!(pixels.pixel_at(ART_PROBE.0, ART_PROBE.1), ART, "{}: the document's background covered the chart art", mode.name);

        let between = pixels.pixel_at((fields[0].0 + fields[0].1) as u32 + OUTSIDE_FIELD, FIELD_ROW);
        for (side, (x, w)) in fields.iter().enumerate() {
            let inside = pixels.pixel_at(*x as u32 + INSIDE_FIELD, FIELD_ROW);
            assert_ne!(inside, between, "{}: field {side} was not lit apart from the gap beside it", mode.name);
            let line = pixels.pixel_at(*x as u32 + INSIDE_FIELD, JUDGE_LINE as u32 - 2);
            assert_ne!(line, inside, "{}: field {side} drew no judgement line", mode.name);
            let readings = (*x as u32, READING_ROW.0, *w as u32, READING_ROW.1);
            assert!(has_bright_pixel(&pixels, readings), "{}: the readings under field {side} are missing", mode.name);
        }

        assert!(has_bright_pixel(&pixels, INFORMATION_PROBE), "{}: the chart information pane is empty", mode.name);
        assert!(has_bright_pixel(&pixels, COUNTS_PROBE), "{}: the judgement counters are empty", mode.name);
        assert!(has_bright_pixel(&pixels, PACEMAKER_PROBE), "{}: the pacemaker column is empty", mode.name);
        assert!(has_bright_pixel(&pixels, GAUGE_PROBE), "{}: the gauge is empty", mode.name);
    }
}

/// The notes the documents draw land inside the lane rectangles they state, which is the whole point
/// of replacing the built-in field: a document that names a note object but places it wrong would
/// still report the block replaced.
#[test]
fn the_two_field_documents_draw_their_notes_on_the_lanes_they_state() {
    for (tag, name, mode, field) in [
        ("v3-dp-notes-10k", "v3-play-10k-notes", Mode::BEAT_10K, TEN_KEY_FIELDS[0]),
        ("v3-dp-notes-14k", "v3-play-14k-notes", Mode::BEAT_14K, FOURTEEN_KEY_FIELDS[0]),
    ] {
        let (_app, pixels) = playing_at(tag, mode, Some(SLOW_SCROLL));
        save(name, &pixels);
        let (x, w) = (field.0 as u32, field.1 as u32);
        let height = JUDGE_LINE as u32 - ABOVE_JUDGEMENT;
        assert!(has_white_note(&pixels, (x, 0, w, height)), "{}: no note reached the field the document states", mode.name);
        assert!(!has_white_note(&pixels, (x - OUTSIDE_FIELD, 0, OUTSIDE_FIELD, height)), "{}: a note was drawn left of the field", mode.name);
        assert!(!has_white_note(&pixels, (x + w, 0, OUTSIDE_FIELD, height)), "{}: a note was drawn right of the field", mode.name);
    }
}
