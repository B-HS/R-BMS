//! The shipped bundle's 24-key play document, held to compiling clean and to drawing the keyboard
//! it describes.
//!
//! A file of its own rather than a case in the single-field set next door, because the keyboard is
//! the one mode no BMS chart can reach: its run is built from a bmson `mode_hint`, its field is 26
//! lanes of two widths rather than a row of equal ones, and it is drawn against a frame nothing else
//! uses. A fault in any of those would be invisible from a seven-key frame.

use rbms_model::Mode;
use rbms_play::{PlaySession, SessionOptions};
use rbms_store::SCORE_LN_MODE_FROM_CHART;

use crate::stage::render_tests_skin::{assert_bundled_document_compiles_clean, bundled_app, render_until_screen_compiled};
use crate::stage::{HeadlessCanvas, PlayState, Stage};
use crate::{App, CH, CW, SkinConfig, assets};

/// Pulses per quarter beat, which is the `resolution` [`keyboard_chart`] states.
const PULSES_PER_BEAT: i64 = 240;

/// How many quarter beats of notes the fixture chart carries, so the head of the run already has
/// something on every lane whatever the scroll speed is.
const CHART_BEATS: i64 = 8;

/// The lanes of an octave a piano draws as black keys, counted from its C.
const BLACK_KEYS_IN_AN_OCTAVE: [usize; 5] = [1, 3, 6, 8, 10];

/// Semitones in an octave, which is how far along the keyboard the key widths repeat.
const SEMITONES_IN_AN_OCTAVE: usize = 12;

/// The lane rectangles `play-24k.json5` writes, as `(x, width)`: the low scratch lane, a white key,
/// a black key and the high scratch lane.
const LOW_SCRATCH_LANE: (f32, f32) = (34.0, 42.0);
const FIRST_WHITE_LANE: (f32, f32) = (76.0, 24.0);
const FIRST_BLACK_LANE: (f32, f32) = (100.0, 19.0);
const HIGH_SCRATCH_LANE: (f32, f32) = (602.0, 42.0);

/// The field that document scrolls notes down, as it writes it.
const FIELD_X: f32 = 34.0;
const FIELD_WIDTH: f32 = 610.0;
const FIELD_TOP: f32 = 0.0;
const FIELD_JUDGE: f32 = 500.0;
const FIELD_NOTE_HEIGHT: f32 = 20.0;

/// The field above the judgement line, where the notes the document draws are and the line is not.
const FIELD_PROBE: (u32, u32, u32, u32) = (36, 2, 606, 490);

/// The bands the document draws in place of the built-in HUD, each well inside the rectangle the
/// frame leaves clear for it, measured down from the head of the screen.
const GAUGE_PROBE: (u32, u32, u32, u32) = (36, 574, 606, 18);
const KEYS_PROBE: (u32, u32, u32, u32) = (36, 514, 606, 36);
const SCORE_PROBE: (u32, u32, u32, u32) = (36, 606, 606, 20);
const COUNTS_PROBE: (u32, u32, u32, u32) = (664, 376, 274, 98);
const GRAPH_PROBE: (u32, u32, u32, u32) = (1020, 18, 240, 120);

/// How light every channel has to be to count as one of the field's own white notes, which nothing
/// else inside the lane rectangles draws.
const NOTE_WHITE: u8 = 230;

/// How bright a pixel's strongest channel has to be before a probe counts it as a panel the document
/// drew rather than a spark in the backdrop behind it.
const PANEL_INK: u8 = 120;

/// How many pixels of a band have to carry ink before it counts as drawn rather than as the odd
/// spark the backdrop paints.
const PANEL_INK_PIXELS: usize = 40;

/// A bmson chart in the keyboard mode: every one of its 26 lanes carries a note every quarter beat
/// from the head of the chart, so the first frame of the run has the whole keyboard on it.
///
/// Written out as bmson rather than BMS because no BMS channel maps to a keyboard lane: the mode is
/// reached only through `info.mode_hint`, which is exactly the route this file has to cover.
fn keyboard_chart() -> String {
    let notes: Vec<String> = (1..=Mode::KEYBOARD_24K.key as i64)
        .flat_map(|x| (0..CHART_BEATS).map(move |beat| format!(r#"{{"x":{x},"y":{},"l":0,"c":false}}"#, beat * PULSES_PER_BEAT)))
        .collect();
    format!(
        concat!(
            r#"{{"version":"1.0.0","#,
            r#""info":{{"title":"keyboard","artist":"R-BMS","mode_hint":"keyboard-24k","init_bpm":120,"resolution":{},"level":12}},"#,
            r#""sound_channels":[{{"name":"a.wav","notes":[{}]}}]}}"#
        ),
        PULSES_PER_BEAT,
        notes.join(",")
    )
}

/// The run [`keyboard_chart`] describes, at the head of its first measure.
fn keyboard_play_state() -> PlayState {
    let chart = rbms_parser::bmson::parse(keyboard_chart().as_bytes()).expect("the keyboard fixture is a bmson chart");
    assert_eq!(chart.mode(), Mode::KEYBOARD_24K, "the mode hint selects the keyboard mode");
    PlayState::new(PlaySession::new(chart.to_model(), SessionOptions::default()), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// Points one app at the keyboard play layout and document, the way choosing a 24-key chart would,
/// and draws until that document has compiled.
fn keyboard_frame(tag: &str) -> (App, HeadlessCanvas) {
    let (mut app, settings) = bundled_app(tag);
    app.shared.mode = Mode::KEYBOARD_24K;
    app.shared.skin_cfg =
        SkinConfig::load(assets::installed_play_skin_path(&settings, &app.shared.config, Mode::KEYBOARD_24K)).expect("the bundled play layout parses");
    app.shared.rebuild_skin();
    let screen = rbms_skin::loader::mode_skin_type(Mode::KEYBOARD_24K).expect("the keyboard mode has a skin type");
    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(
        render_until_screen_compiled(&mut app, screen, &mut pixels, || Stage::Play(Box::new(keyboard_play_state()))),
        "the keyboard play document never finished compiling"
    );
    save("play-24k", &pixels);
    (app, pixels)
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

/// How many pixels of `rect`, written as `(x, y, w, h)` measured down from the head of the screen,
/// carry ink the document put there rather than the backdrop behind it.
fn ink_count(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> usize {
    let (x, y, w, h) = rect;
    (y..y + h)
        .map(|py| {
            (x..x + w)
                .filter(|px| {
                    let pixel = pixels.pixel_at(*px, py);
                    pixel.r.max(pixel.g).max(pixel.b) >= PANEL_INK
                })
                .count()
        })
        .sum()
}

/// Whether any pixel of `rect` is one of the note sprite's own whites.
fn has_white_note(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> bool {
    let (x, y, w, h) = rect;
    (y..y + h).any(|py| {
        (x..x + w).any(|px| {
            let pixel = pixels.pixel_at(px, py);
            pixel.r >= NOTE_WHITE && pixel.g >= NOTE_WHITE && pixel.b >= NOTE_WHITE
        })
    })
}

#[test]
fn the_bundles_keyboard_document_compiles_without_a_warning() {
    let (mut app, settings) = bundled_app("v3-play-24k");
    app.shared.mode = Mode::KEYBOARD_24K;
    app.shared.skin_cfg =
        SkinConfig::load(assets::installed_play_skin_path(&settings, &app.shared.config, Mode::KEYBOARD_24K)).expect("the bundled play layout parses");
    app.shared.rebuild_skin();
    let screen = rbms_skin::loader::mode_skin_type(Mode::KEYBOARD_24K).expect("the keyboard mode has a skin type");
    assert_bundled_document_compiles_clean(&mut app, screen, || Stage::Play(Box::new(keyboard_play_state())));
}

/// Every block the keyboard document lists in `replace` has the objects that block demands, so the
/// play screen really does stand aside for all of them rather than drawing half its own HUD under it.
#[test]
fn the_keyboard_document_stands_in_for_every_block_it_names() {
    let (app, _pixels) = keyboard_frame("v3-play-24k-blocks");
    let screen = rbms_skin::loader::mode_skin_type(Mode::KEYBOARD_24K).expect("the keyboard mode has a skin type");
    let content = app.shared.screen_content(screen).play;
    assert!(content.field, "the document did not take the note field over");
    assert!(content.gauge, "the document did not take the gauge over");
    assert!(content.judge, "the document did not take the judgement pop-up over");
    assert!(content.score, "the document did not take the score readout over");
    assert!(content.counts, "the document did not take the judgement counters over");
    assert!(content.graph, "the document did not take the score graph over");
    assert!(content.cover, "the document did not take the lane covers over");
    assert!(content.frame, "the document did not take the frame over");
}

/// The 26 lane rectangles the document declares become the field the run is drawn on, and they are a
/// piano: the black keys are narrower than the white ones, a scratch lane closes either end, and the
/// whole keyboard is edge to edge across the field the document states.
#[test]
fn the_keyboard_documents_lane_rectangles_become_the_running_field() {
    let (app, _pixels) = keyboard_frame("v3-play-24k-lanes");
    let field = &app.shared.skin;
    assert_eq!(field.lane_count(), Mode::KEYBOARD_24K.key, "the keyboard field lost a lane");
    assert_eq!((field.x[24], field.w[24]), LOW_SCRATCH_LANE, "the low scratch lane");
    assert_eq!((field.x[0], field.w[0]), FIRST_WHITE_LANE, "the first white key");
    assert_eq!((field.x[1], field.w[1]), FIRST_BLACK_LANE, "the first black key");
    assert_eq!((field.x[25], field.w[25]), HIGH_SCRATCH_LANE, "the high scratch lane");
    assert_eq!(field.top_y, FIELD_TOP);
    assert_eq!(field.judge_y, FIELD_JUDGE);
    assert_eq!(field.note_height, FIELD_NOTE_HEIGHT);

    for lane in 0..Mode::KEYBOARD_24K.key - Mode::KEYBOARD_24K.scratch.len() {
        let black = BLACK_KEYS_IN_AN_OCTAVE.contains(&(lane % SEMITONES_IN_AN_OCTAVE));
        let want = if black { FIRST_BLACK_LANE.1 } else { FIRST_WHITE_LANE.1 };
        assert_eq!(field.w[lane], want, "lane {lane} is the wrong key width");
        assert!(!field.scratch[lane], "lane {lane} is a key rather than a scratch lane");
        if lane > 0 {
            assert_eq!(field.x[lane], field.x[lane - 1] + field.w[lane - 1], "lane {lane} does not meet the one before it");
        }
    }
    assert!(field.scratch[24] && field.scratch[25], "both scratch lanes are marked");
    assert_eq!(field.x[24], FIELD_X, "the keyboard starts at the field");
    assert_eq!(field.x[25] + field.w[25], FIELD_X + FIELD_WIDTH, "and ends where the field ends");
}

/// The document is not merely accepted: the keyboard, the gauge and the panels it draws in place of
/// the built-in HUD all reach the canvas, which is what separates a document that draws from one
/// that only compiles.
#[test]
fn the_keyboard_document_draws_its_field_and_its_hud() {
    let (_app, pixels) = keyboard_frame("v3-play-24k-pixels");
    assert!(has_white_note(&pixels, FIELD_PROBE), "no note of the keyboard field reached the canvas");
    for (name, probe) in [
        ("the gauge", GAUGE_PROBE),
        ("the key widgets", KEYS_PROBE),
        ("the score readout", SCORE_PROBE),
        ("the judgement counters", COUNTS_PROBE),
        ("the score graph", GRAPH_PROBE),
    ] {
        let lit = ink_count(&pixels, probe);
        assert!(lit >= PANEL_INK_PIXELS, "{name} drew {lit} lit pixels in {probe:?}, which is not a panel");
    }
}
