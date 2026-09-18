//! The shipped bundle's single-field play documents, held to compiling clean and to standing in for
//! the field they replace.
//!
//! One document per key count rather than one for all three, because each declares its own lane
//! rectangles and the whole point of separating them is that a fault in one cannot be read off
//! another.
//!
//! Past compiling, each document has to take the screen over rather than merely sit on it: the
//! blocks it names in `replace` must all have their objects, its own lane rectangles must become the
//! field the run is drawn on, and the panels it draws in place of the built-in HUD must actually
//! reach the canvas. Those are checked against the frame the play screen really draws, because a
//! document that compiles and draws nothing is the failure this bundle has had before.

use rbms_model::Mode;
use rbms_play::{PlaySession, SessionOptions};
use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;
use rbms_store::SCORE_LN_MODE_FROM_CHART;

use crate::stage::render_tests::{play_state, play_state_with_bga};
use crate::stage::render_tests_skin::{assert_bundled_document_compiles_clean, bundled_app, render_until_screen_compiled};
use crate::stage::{HeadlessCanvas, PlayState, Stage};
use crate::{App, CH, CW, Color, Config, LaunchOptions, SkinConfig, assets};

/// A chart whose first measure is full of notes, so the head of the run already has something on
/// screen.
///
/// The harness next door starts its own chart at the second measure, which is far enough ahead that
/// nothing is in the visible window at the moment a frame is drawn; a document that draws no notes
/// would look exactly like one that draws them well. Every lane carries a note every quarter beat
/// here, so whatever the scroll speed is, the field has notes on it.
const DENSE_CHART: &str = concat!(
    "#PLAYER 1\n#TITLE snapshot\n#BPM 120\n#WAV01 a.wav\n",
    "#00011:0101010101010101\n#00012:0001000100010001\n#00013:0100010001000100\n#00014:0101010101010101\n",
    "#00015:0001000100010001\n#00018:0100010001000100\n#00019:0101010101010101\n#00016:0001000100010001\n",
);

/// Where the seven-key document puts its turntable and its first key, which is what the running
/// field has to agree with once the document is compiled.
const SCRATCH_LANE: (f32, f32) = (34.0, 64.0);
const FIRST_KEY_LANE: (f32, f32) = (98.0, 40.0);

/// The field the documents scroll notes down, as they write it.
const FIELD_TOP: f32 = 0.0;
const FIELD_JUDGE: f32 = 500.0;
const FIELD_NOTE_HEIGHT: f32 = 22.0;

/// The field above the judgement line, where the notes the document draws are and the line itself is
/// not.
const FIELD_PROBE: (u32, u32, u32, u32) = (36, 2, 316, 490);

/// The panels the document draws in place of the built-in HUD, each well inside the rectangle the
/// frame leaves clear for it.
const SCORE_PROBE: (u32, u32, u32, u32) = (36, 604, 316, 24);
const COUNTS_PROBE: (u32, u32, u32, u32) = (378, 604, 210, 80);
const GRAPH_PROBE: (u32, u32, u32, u32) = (1020, 18, 240, 120);
const KEYS_PROBE: (u32, u32, u32, u32) = (36, 510, 316, 52);

/// The colour the chart's background art is painted for the probes that look for it, and the corners
/// and centre of the rectangle the play layout hands it.
const BGA_MARK: Color = Color::rgb(220, 40, 130);
const BGA_PROBES: [(u32, u32); 3] = [(372, 116), (600, 300), (996, 582)];

/// The bundle directory the documents are installed under, which is what a bundle-scope row is
/// stored against.
const BUNDLE: &str = "steel-neon-v3";

/// The bundle rows this file switches and the options they carry, as `play-7k.json5` declares them.
const ROW_BGA_SIZE: &str = "BGA SIZE";
const ROW_PLAY_SIDE: &str = "PLAY SIDE";
const ROW_GRAPH_POSITION: &str = "GRAPH POSITION";
const BGA_OFF: i32 = 912;
const PLAY_SIDE_2P: i32 = 901;
const GRAPH_NEAR: i32 = 921;

/// Where the turntable sits once the field is on the second player's side, which is the whole point
/// of the row: past this, the lanes cannot still be on the left.
const SECOND_SIDE_SCRATCH: f32 = 1180.0;

/// The body of the score graph column in each of its two places, as `(x, y, w, h)` measured down
/// from the head of the screen: the strip beside the field and the strip at the edge of the screen.
///
/// Both are read with the chart art switched off, so what is counted in them is the column itself
/// rather than whichever of the two the art happened to cover.
const NEAR_GRAPH_COLUMN: (u32, u32, u32, u32) = (370, 150, 260, 440);
const FAR_GRAPH_COLUMN: (u32, u32, u32, u32) = (1010, 150, 260, 440);

/// How much of the column has to move for the row to have moved it, as a multiple of what is left
/// behind: the backdrop under it carries a spark here and there, so the two strips are compared
/// rather than one of them being asked to be empty.
const COLUMN_MOVED: usize = 4;

/// The lanes the field takes on the second player's side, where its notes have to be.
const SECOND_SIDE_FIELD: (u32, u32, u32, u32) = (928, 2, 316, 490);

/// How light a pixel has to be before a probe counts it as ink the document put there rather than
/// the backdrop behind it: brighter in blue than the backdrop's brightest spark and light overall.
const LIT_BLUE: u16 = 190;
const LIT_TOTAL: u16 = 420;

/// How light a pixel has to be to count as one of the key widgets, which are drawn from a grey
/// gradient rather than from ink: far above the near-black backdrop under them, far below a label.
const WIDGET_TOTAL: u16 = 200;

/// How light every channel has to be before a probe counts a pixel as one of the field's own white
/// notes, which nothing else inside the lane rectangles draws.
const NOTE_WHITE: u8 = 230;

/// The run [`DENSE_CHART`] describes, at the head of its first measure.
fn dense_play_state() -> PlayState {
    let source = rbms_parser::parse_with(DENSE_CHART.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&source, "dense.bms");
    let model = rbms_chart::to_model(&source, mode);
    PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// An app with the bundle installed and one of its bundle-scope rows switched before the documents
/// are ever read, which is what a row chosen in the SKIN tab amounts to.
///
/// The choice has to be in the configuration the app is built with rather than set on a running one:
/// a document is read once and compiled with the options that held at the time, so a row changed
/// afterwards would not reach the frame until something else made it stale.
fn bundle_with_rows(tag: &str, rows: &[(&str, i32)]) -> (App, std::path::PathBuf) {
    rbms_render::font::use_embedded_fonts_only();
    let directory = std::env::temp_dir().join(format!("rbms-bundle-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("the bundle fixture folder is writable");
    let settings = directory.join("settings.ron");
    let mut config = Config::default();
    assert!(crate::assets::install_default_skin(&settings, &mut config), "the bundled skin installs");
    crate::assets::load_theme(&settings, &config);
    for (row, option) in rows {
        config.skin.shared_customise(BUNDLE).properties.insert((*row).to_owned(), *option);
    }
    let app = App::new(String::new(), config, LaunchOptions::default(), settings.clone());
    (app, settings)
}

/// Draws the seven-key document of `app` until it has compiled, with the chart carrying background
/// art so the probes can tell where the document put it.
fn seven_key_frame(app: &mut App, settings: &std::path::Path) -> HeadlessCanvas {
    let side = 4;
    let bytes: Vec<u8> = (0..side * side).flat_map(|_| [BGA_MARK.r, BGA_MARK.g, BGA_MARK.b, u8::MAX]).collect();
    seven_key_frame_of(app, settings, move || {
        let mut frames = std::collections::HashMap::new();
        frames.insert(crate::stage::play::NO_BGA_FRAME, crate::DecodedImage::for_test(bytes.clone(), side, side));
        Stage::Play(Box::new(play_state_with_bga(frames)))
    })
}

/// Draws the seven-key document of `app` on the run `stage` builds, until the document has compiled.
fn seven_key_frame_of(app: &mut App, settings: &std::path::Path, stage: impl Fn() -> Stage) -> HeadlessCanvas {
    app.shared.mode = Mode::BEAT_7K;
    app.shared.skin_cfg =
        SkinConfig::load(assets::installed_play_skin_path(settings, &app.shared.config, Mode::BEAT_7K)).expect("the bundled play layout parses");
    app.shared.rebuild_skin();
    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(render_until_screen_compiled(app, SKIN_TYPE_PLAY_7KEYS, &mut pixels, stage), "the seven-key document never finished compiling");
    pixels
}

/// How bright a pixel's strongest channel has to be before [`lit_count`] counts it as part of a
/// panel the document drew rather than a spark in the backdrop behind it.
const COLUMN_INK: u8 = 150;

/// How many pixels of `rect` carry ink of any colour.
fn lit_count(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> usize {
    let (x, y, w, h) = rect;
    (y..y + h)
        .map(|py| {
            (x..x + w)
                .filter(|px| {
                    let pixel = pixels.pixel_at(*px, py);
                    pixel.r.max(pixel.g).max(pixel.b) >= COLUMN_INK
                })
                .count()
        })
        .sum()
}

/// Points one app at the play layout and document of `mode`, the way choosing a chart of that mode
/// would, and draws until that document has compiled.
fn playing(tag: &str, mode: Mode) -> (App, HeadlessCanvas) {
    let (mut app, settings) = bundled_app(tag);
    app.shared.mode = mode;
    app.shared.skin_cfg = SkinConfig::load(assets::installed_play_skin_path(&settings, &app.shared.config, mode)).expect("the bundled play layout parses");
    app.shared.rebuild_skin();
    let screen = rbms_skin::loader::mode_skin_type(mode).expect("the mode has a skin type");
    let mut pixels = HeadlessCanvas::new(CW, CH);
    assert!(
        render_until_screen_compiled(&mut app, screen, &mut pixels, || Stage::Play(Box::new(dense_play_state()))),
        "the play document for skin type {screen} never finished compiling"
    );
    (app, pixels)
}

/// Whether any pixel of `rect`, written as `(x, y, w, h)` measured down from the head of the screen,
/// is light enough to be ink the document put there.
///
/// A probe rather than an exact colour because every label is drawn anti-aliased over whatever panel
/// is behind it: what is asserted is that something light reached the rectangle, which is the
/// difference between a filled panel and an empty one.
fn has_lit_pixel(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> bool {
    let (x, y, w, h) = rect;
    (y..y + h).any(|py| {
        (x..x + w).any(|px| {
            let pixel = pixels.pixel_at(px, py);
            let total = u16::from(pixel.r) + u16::from(pixel.g) + u16::from(pixel.b);
            u16::from(pixel.b) >= LIT_BLUE && total >= LIT_TOTAL
        })
    })
}

/// Whether any pixel of `rect` is light enough to be one of the key widgets under the field.
fn has_widget_pixel(pixels: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> bool {
    let (x, y, w, h) = rect;
    (y..y + h).any(|py| {
        (x..x + w).any(|px| {
            let pixel = pixels.pixel_at(px, py);
            u16::from(pixel.r) + u16::from(pixel.g) + u16::from(pixel.b) >= WIDGET_TOTAL
        })
    })
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

/// Points one app at the play layout and document of `mode` and holds that document to compiling
/// with nothing to report.
fn play_document_of(tag: &str, mode: Mode) {
    let (mut app, settings) = bundled_app(tag);
    app.shared.mode = mode;
    app.shared.skin_cfg = SkinConfig::load(assets::installed_play_skin_path(&settings, &app.shared.config, mode)).expect("the bundled play layout parses");
    app.shared.rebuild_skin();
    let screen = rbms_skin::loader::mode_skin_type(mode).expect("the mode has a skin type");
    assert_bundled_document_compiles_clean(&mut app, screen, || Stage::Play(Box::new(play_state())));
}

#[test]
fn the_bundles_five_key_document_compiles_without_a_warning() {
    play_document_of("v3-play-5k", Mode::BEAT_5K);
}

#[test]
fn the_bundles_seven_key_document_compiles_without_a_warning() {
    play_document_of("v3-play-7k", Mode::BEAT_7K);
}

#[test]
fn the_bundles_nine_key_document_compiles_without_a_warning() {
    play_document_of("v3-play-9k", Mode::POPN_9K);
}

/// Every block a play document lists in `replace` has the objects that block demands, so the play
/// screen really does stand aside for all of them rather than drawing half its own HUD underneath.
#[test]
fn the_play_documents_stand_in_for_every_block_they_name() {
    for (tag, mode) in [("v3-play-blocks-5k", Mode::BEAT_5K), ("v3-play-blocks-7k", Mode::BEAT_7K), ("v3-play-blocks-9k", Mode::POPN_9K)] {
        let (app, _pixels) = playing(tag, mode);
        let screen = rbms_skin::loader::mode_skin_type(mode).expect("the mode has a skin type");
        let content = app.shared.screen_content(screen).play;
        assert!(content.field, "{tag}: the document did not take the note field over");
        assert!(content.gauge, "{tag}: the document did not take the gauge over");
        assert!(content.judge, "{tag}: the document did not take the judgement pop-up over");
        assert!(content.score, "{tag}: the document did not take the score readout over");
        assert!(content.counts, "{tag}: the document did not take the judgement counters over");
        assert!(content.graph, "{tag}: the document did not take the score graph over");
        assert!(content.cover, "{tag}: the document did not take the lane covers over");
        assert!(content.frame, "{tag}: the document did not take the frame over");
    }
}

/// The lane rectangles the seven-key document declares become the field the run is drawn on, which
/// is what keeps the notes, the covers and the key bombs on the lanes the document drew.
#[test]
fn the_play_documents_lane_rectangles_become_the_running_field() {
    let (app, _pixels) = playing("v3-play-lanes", Mode::BEAT_7K);
    let field = &app.shared.skin;
    assert_eq!(field.lane_count(), 8, "the seven-key field lost a lane");
    assert_eq!((field.x[7], field.w[7]), SCRATCH_LANE, "the turntable is not where the document put it");
    assert_eq!((field.x[0], field.w[0]), FIRST_KEY_LANE, "the first key is not where the document put it");
    assert_eq!(field.top_y, FIELD_TOP, "the field does not start where the document's lanes do");
    assert_eq!(field.judge_y, FIELD_JUDGE, "the judgement line is not where the document's lanes end");
    assert_eq!(field.note_height, FIELD_NOTE_HEIGHT, "the note height is not the one the document states");
}

/// The field the seven-key document draws reaches the canvas: its own notes are on its own lanes,
/// and the key widgets under them are lit.
#[test]
fn the_seven_key_document_draws_the_field_it_replaced() {
    let (_app, pixels) = playing("v3-play-field", Mode::BEAT_7K);
    save("v3-play-7k", &pixels);
    assert!(has_white_note(&pixels, FIELD_PROBE), "the document's own notes never reached the field it replaced");
    assert!(has_widget_pixel(&pixels, KEYS_PROBE), "the key widgets under the field drew nothing");
}

/// The chart's own background art reaches the screen through every single-field document: the
/// rectangle the play layout hands the art is one the documents leave clear, top to bottom.
#[test]
fn the_single_field_documents_leave_the_chart_art_alone() {
    let side = 4;
    let bytes: Vec<u8> = (0..side * side).flat_map(|_| [BGA_MARK.r, BGA_MARK.g, BGA_MARK.b, u8::MAX]).collect();
    for (tag, mode) in [("v3-play-bga-5k", Mode::BEAT_5K), ("v3-play-bga-7k", Mode::BEAT_7K), ("v3-play-bga-9k", Mode::POPN_9K)] {
        let (mut app, settings) = bundled_app(tag);
        app.shared.mode = mode;
        app.shared.skin_cfg = SkinConfig::load(assets::installed_play_skin_path(&settings, &app.shared.config, mode)).expect("the bundled play layout parses");
        app.shared.rebuild_skin();
        let screen = rbms_skin::loader::mode_skin_type(mode).expect("the mode has a skin type");
        let mut pixels = HeadlessCanvas::new(CW, CH);
        let frames = || {
            let mut frames = std::collections::HashMap::new();
            frames.insert(crate::stage::play::NO_BGA_FRAME, crate::DecodedImage::for_test(bytes.clone(), side, side));
            frames
        };
        assert!(
            render_until_screen_compiled(&mut app, screen, &mut pixels, || Stage::Play(Box::new(play_state_with_bga(frames())))),
            "{tag}: the play document never finished compiling"
        );
        for probe in BGA_PROBES {
            assert_eq!(pixels.pixel_at(probe.0, probe.1), BGA_MARK, "{tag}: the document covered the chart art at {probe:?}");
        }
    }
}

/// The chart's art reaches the screen through the document's own `bga` object, at the rectangle the
/// document places it, and the row that turns it off really leaves that rectangle to the backdrop.
///
/// The two halves are one test because "the art is not here" only means anything beside a frame
/// where it is: a document that stopped drawing the art altogether would pass the second half on its
/// own.
#[test]
fn the_bga_size_row_places_the_chart_art_and_can_take_it_away() {
    let (mut app, settings) = bundled_app("v3-play-bga-large");
    let large = seven_key_frame(&mut app, &settings);
    for probe in BGA_PROBES {
        assert_eq!(large.pixel_at(probe.0, probe.1), BGA_MARK, "the document did not place the chart art at {probe:?}");
    }

    let (mut app, settings) = bundle_with_rows("v3-play-bga-off", &[(ROW_BGA_SIZE, BGA_OFF)]);
    let off = seven_key_frame(&mut app, &settings);
    for probe in BGA_PROBES {
        assert_ne!(off.pixel_at(probe.0, probe.1), BGA_MARK, "the chart art was still drawn at {probe:?} with the BGA row switched off");
    }
}

/// The row that moves the field to the second player's side really moves it: the lanes the run is
/// played on are on the right of the screen and the turntable is at its outer edge.
#[test]
fn the_play_side_row_moves_the_field_to_the_second_players_side() {
    let (mut app, settings) = bundle_with_rows("v3-play-side-2p", &[(ROW_PLAY_SIDE, PLAY_SIDE_2P)]);
    let pixels = seven_key_frame_of(&mut app, &settings, || Stage::Play(Box::new(dense_play_state())));
    save("v3-play-7k-2p", &pixels);

    let field = &app.shared.skin;
    assert_eq!(field.lane_count(), 8, "the seven-key field lost a lane");
    assert!(field.x[7] >= SECOND_SIDE_SCRATCH, "the turntable is at {} rather than the outer edge of the second side", field.x[7]);
    assert!(field.x.iter().take(7).all(|x| *x >= 900.0), "a key lane is still on the first player's side");
    assert!(has_white_note(&pixels, SECOND_SIDE_FIELD), "the document drew no notes on the lanes it moved to");
}

/// The row that brings the score graph in beside the field really moves the column: the strip it
/// stood in is left to the backdrop and the strip beside the field carries it.
#[test]
fn the_graph_position_row_brings_the_score_column_beside_the_field() {
    let (mut app, settings) = bundle_with_rows("v3-play-graph-far", &[(ROW_BGA_SIZE, BGA_OFF)]);
    let far = seven_key_frame(&mut app, &settings);
    let (mut app, settings) = bundle_with_rows("v3-play-graph-near", &[(ROW_BGA_SIZE, BGA_OFF), (ROW_GRAPH_POSITION, GRAPH_NEAR)]);
    let near = seven_key_frame(&mut app, &settings);
    save("v3-play-7k-near", &near);

    let moved_in = (lit_count(&near, NEAR_GRAPH_COLUMN), lit_count(&far, NEAR_GRAPH_COLUMN));
    assert!(moved_in.0 > moved_in.1 * COLUMN_MOVED, "the strip beside the field carries {} against {} before the row moved the column", moved_in.0, moved_in.1);
    let moved_out = (lit_count(&far, FAR_GRAPH_COLUMN), lit_count(&near, FAR_GRAPH_COLUMN));
    assert!(moved_out.0 > moved_out.1 * COLUMN_MOVED, "the strip the column stood in still carries {} against {}", moved_out.1, moved_out.0);
}

/// The panels the seven-key document draws in place of the built-in HUD reach the canvas, each
/// inside the rectangle the bundle's frame leaves clear for it.
#[test]
fn the_seven_key_document_draws_the_readouts_it_replaced() {
    let (_app, pixels) = playing("v3-play-hud", Mode::BEAT_7K);
    assert!(has_lit_pixel(&pixels, SCORE_PROBE), "the score row drew nothing");
    assert!(has_lit_pixel(&pixels, COUNTS_PROBE), "the judgement counters drew nothing");
    assert!(has_lit_pixel(&pixels, GRAPH_PROBE), "the score graph column drew nothing");
}

/// The five- and nine-key documents draw their own fields too, on lanes of their own width.
#[test]
fn the_other_single_field_documents_draw_their_own_fields() {
    for (tag, name, mode, lanes) in [("v3-play-field-5k", "v3-play-5k", Mode::BEAT_5K, 6), ("v3-play-field-9k", "v3-play-9k", Mode::POPN_9K, 9)] {
        let (app, pixels) = playing(tag, mode);
        save(name, &pixels);
        assert_eq!(app.shared.skin.lane_count(), lanes, "{name}: the field lost a lane");
        assert!(has_white_note(&pixels, FIELD_PROBE), "{name}: the document's own notes never reached the field it replaced");
        assert!(has_lit_pixel(&pixels, GRAPH_PROBE), "{name}: the shared score graph column drew nothing");
    }
}
