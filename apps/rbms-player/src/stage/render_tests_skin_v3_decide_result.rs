//! The shipped bundle's loading and score documents: what they compile to and what they draw.
//!
//! The loading screen stands in for the reference's decide screen, and its own state advances on the
//! second frame it is drawn; a fresh stage each frame is what lets it be drawn for as long as a
//! document takes to arrive without the chart load behind it ever starting.
//!
//! The score screen is the one screen of the bundle that hands every block of its native output to
//! the document, so both halves of that bargain are checked here: the document compiles the objects
//! each block asks for, and what it draws in their place is its own.

use rbms_render::result::{ResultContent, ResultView};
use rbms_skin::loader::{SKIN_TYPE_DECIDE, SKIN_TYPE_RESULT};

use crate::stage::HeadlessCanvas;
use crate::stage::Stage;
use crate::stage::loading::LoadingState;
use crate::stage::render_tests::result_state;
use crate::stage::render_tests_skin::{assert_bundled_document_compiles_clean, bundled_app, render_until_screen_compiled};
use crate::stage::result::ResultState;
use crate::{CH, CW, Color};

/// The channel level a pixel has to reach to count as one of the document's own marks.
///
/// Every backdrop this bundle ships covers the whole screen, so "was anything drawn here" cannot be
/// asked as "is this pixel opaque". It is asked as "is this pixel light", which every glyph, rule
/// and bar in these two documents is and the dark gradients behind them are not.
const LIT_CHANNEL: u8 = 140;

/// How far a channel may drift from a reference colour and still count as the same paint, which
/// covers the tint a panel is drawn through.
const CHANNEL_TOLERANCE: i32 = 12;

/// Whether two colours are the same paint within [`CHANNEL_TOLERANCE`].
fn close_to(left: Color, right: Color) -> bool {
    let apart = |a: u8, b: u8| (i32::from(a) - i32::from(b)).abs() <= CHANNEL_TOLERANCE;
    apart(left.r, right.r) && apart(left.g, right.g) && apart(left.b, right.b)
}

/// How many pixels of a rectangle the document lit, by [`LIT_CHANNEL`].
fn lit_in(canvas: &HeadlessCanvas, rect: (u32, u32, u32, u32)) -> usize {
    let (x, y, w, h) = rect;
    let mut count = 0;
    for py in y..(y + h).min(CH) {
        for px in x..(x + w).min(CW) {
            let pixel = canvas.pixel_at(px, py);
            if pixel.r.max(pixel.g).max(pixel.b) >= LIT_CHANNEL {
                count += 1;
            }
        }
    }
    count
}

/// Saves a capture of one screen when the run asked for them, the way the bundle's own snapshot test
/// does, so these two screens can be looked at beside the others.
fn save_capture(name: &str, canvas: &HeadlessCanvas) {
    let Some(directory) = std::env::var_os("RBMS_SKIN_CAPTURE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).expect("create the capture folder");
    let image = image::RgbaImage::from_fn(CW, CH, |x, y| {
        let pixel = canvas.pixel_at(x, y);
        image::Rgba([pixel.r, pixel.g, pixel.b, pixel.a])
    });
    image.save(directory.join(format!("{name}.png"))).expect("save the capture");
}

/// A run that scored `ex` of `max`, measured all the way through.
///
/// The measurements matter as much as the score: the trend, judgement and timing panels have nothing
/// to draw for a run nothing was recorded for, so a frame made from one would show those three
/// panels empty whether the document declared them or not.
fn run_scoring(ex: u32, max: u32) -> ResultState {
    ResultState::new(ResultView {
        title: "band".into(),
        artist: String::new(),
        mode_label: "7K",
        counts: [3, 2, 1, 0, 0, 0],
        ex_score: ex,
        max_score: max,
        max_combo: 5,
        total_notes: max / 2,
        fast: [1, 0],
        slow: [1, 0],
        gauge: 80.0,
        clear_label: "CLEAR",
        clear_color: Color::GREEN,
        prev_best_ex: Some(6),
        prev_ex: Some(4),
        show_graph: true,
        show_result_graphs: true,
        gauge_series: vec![20.0, 45.0, 62.0, 58.0, 80.0],
        timing_hist: Box::new([1, 2, 5, 3, 1]),
        judge_dist: [3, 2, 1, 0, 0, 0],
    })
}

/// Draws one score screen until its document has compiled, and answers the frame.
fn score_frame(tag: &str, run: impl Fn() -> ResultState) -> HeadlessCanvas {
    let (mut app, _settings) = bundled_app(tag);
    let mut canvas = HeadlessCanvas::new(CW, CH);
    assert!(render_until_screen_compiled(&mut app, SKIN_TYPE_RESULT, &mut canvas, || Stage::Result(run())), "the score document never finished compiling");
    canvas
}

#[test]
fn the_bundles_loading_document_compiles_without_a_warning() {
    let (mut app, _settings) = bundled_app("v3-decide");
    assert_bundled_document_compiles_clean(&mut app, SKIN_TYPE_DECIDE, || Stage::Loading(LoadingState::song(0)));
}

#[test]
fn the_bundles_score_document_compiles_without_a_warning() {
    let (mut app, _settings) = bundled_app("v3-result");
    assert_bundled_document_compiles_clean(&mut app, SKIN_TYPE_RESULT, || Stage::Result(result_state()));
}

/// The loading screen paints its own band and names the chart it is about to start, which is what
/// "the document owns the whole screen" has to look like rather than a black frame.
#[test]
fn the_loading_document_names_the_chart_it_is_starting() {
    let (mut app, _settings) = bundled_app("v3-decide-drawn");
    app.shared.library = rbms_library::Library::from_songs(vec![crate::stage::select::tests::entry("First light", "Artist A", "11")]);
    let mut canvas = HeadlessCanvas::new(CW, CH);
    assert!(
        render_until_screen_compiled(&mut app, SKIN_TYPE_DECIDE, &mut canvas, || Stage::Loading(LoadingState::song(0))),
        "the loading document never finished compiling"
    );
    save_capture("decide", &canvas);

    assert!(
        !close_to(canvas.pixel_at(CW / 2, 500), canvas.pixel_at(CW / 2, 300)),
        "the band across the loading screen is the same colour as the backdrop above it, so it never drew"
    );
    assert!(lit_in(&canvas, (100, 326, 700, 52)) > 0, "the chart the load is starting was not named");
    assert!(lit_in(&canvas, (980, 494, 200, 22)) > 0, "the loading state on the band did not draw");
    assert!(lit_in(&canvas, ARTIST_ROW) > 0, "the artist of the chart the load is starting was not drawn");
}

/// Where the loading document puts the artist of the chart it is starting, measured down from the
/// head of the screen.
///
/// Everything under the title comes from the library rather than from the load itself, so it is the
/// row that shows whether the screen is being drawn from the state the stage built or from one made
/// up on the way past.
const ARTIST_ROW: (u32, u32, u32, u32) = (100, 408, 560, 20);

/// The score document names every block of native output and compiles the objects each of them asks
/// for, which is the only state in which the built-in screen stands aside.
#[test]
fn the_score_document_takes_over_every_block_of_the_native_screen() {
    let (mut app, _settings) = bundled_app("v3-result-content");
    let mut canvas = HeadlessCanvas::new(CW, CH);
    assert!(
        render_until_screen_compiled(&mut app, SKIN_TYPE_RESULT, &mut canvas, || Stage::Result(result_state())),
        "the score document never finished compiling"
    );

    assert_eq!(
        app.shared.screen_content(SKIN_TYPE_RESULT).result,
        ResultContent { score: true, clear: true, judgment: true, target: true, grade: true, graphs: true, title: true, hint: true, ir: true },
        "a block the document named did not compile everything it needs"
    );
}

/// What the document draws where the built-in screen used to: its own steel top bar rather than the
/// theme's, and its own columns of readings, trends and bars.
#[test]
fn the_score_document_paints_its_own_top_bar_and_columns() {
    let canvas = score_frame("v3-result-drawn", || run_scoring(8, 12));
    save_capture("result", &canvas);

    assert!(!close_to(canvas.pixel_at(CW / 2, 20), rbms_render::theme().topbar), "the built-in top bar is still the one on screen");
    assert!(lit_in(&canvas, (520, 10, 240, 30)) > 0, "the screen is not titled");
    assert!(lit_in(&canvas, (24, 56, 368, 112)) > 0, "the gauge trend panel is empty");
    assert!(lit_in(&canvas, (32, 280, 352, 190)) > 0, "the score table is empty");
    assert!(lit_in(&canvas, (32, 530, 352, 130)) > 0, "the judgement rows are empty");
    assert!(lit_in(&canvas, (440, 84, 380, 150)) > 0, "the judgement panel is empty");
    assert!(lit_in(&canvas, (1016, 12, 250, 120)) > 0, "the score graph column is empty");
    assert!(lit_in(&canvas, (1100, 300, 50, 280)) > 0, "the EX bar did not reach its share of the column");
}

/// A run's rank letter comes from the one band it landed in: a perfect run puts three glyphs on the
/// plate where a two-thirds run puts one, which no ungated set of eight texts could do.
#[test]
fn the_score_document_shows_the_letter_of_the_band_the_run_reached() {
    let best = score_frame("v3-result-rank-best", || run_scoring(12, 12));
    let middling = score_frame("v3-result-rank-mid", || run_scoring(8, 12));

    let plate = (120, 186, 160, 48);
    let (best_lit, middling_lit) = (lit_in(&best, plate), lit_in(&middling, plate));
    assert!(middling_lit > 0, "no rank letter reached the plate");
    assert!(
        best_lit > middling_lit,
        "a perfect run and a two-thirds run put the same amount of letter on the plate ({best_lit} against {middling_lit}), so the band gate is not choosing between them"
    );
}

/// A whole number leaves its leading places blank rather than filling them.
///
/// The digit sheets this bundle ships carry ten glyphs and a blank eleventh cell, and a strip cut to
/// eleven or twelve cells makes the renderer read that eleventh cell as the alternate zero and paint
/// it into every leading place. The best-score reading is a single digit wide, so four blank places
/// sit in front of it and are the cheapest place to notice that.
#[test]
fn a_numbers_leading_places_stay_blank() {
    let canvas = score_frame("v3-result-digits", result_state);

    assert!(lit_in(&canvas, (364, 347, 20, 28)) > 0, "the best score did not draw at all");
    assert_eq!(lit_in(&canvas, (276, 347, 86, 28)), 0, "a leading place of the best score was filled in");
}
