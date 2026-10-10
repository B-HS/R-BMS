//! The arithmetic of the hit error visualiser, checked on the strip it paints and without a
//! renderer.

use rbms_skin::model::HitErrorVisualizer;

use super::super::RecentHits;
use super::super::pixmap::blend_over;
use super::*;

const PERFECT: Rgba = [0x10, 0x20, 0xF0, 255];
const GREAT: Rgba = [0x20, 0xF0, 0x20, 255];
const GOOD: Rgba = [0xF0, 0xF0, 0x20, 255];
const BAD: Rgba = [0xF0, 0x20, 0x20, 255];
const POOR: Rgba = [0x80, 0x80, 0x80, 255];
const MIDDLE: Rgba = [255; 4];
const AVERAGE: Rgba = [255, 0, 255, 255];
const LINE: Rgba = [0x99, 0xCC, 0xFF, 0x80];

/// Windows that put the best judgement within eight milliseconds of the middle, the next within
/// twenty, the third within forty and the bad one within seventy.
const WINDOWS: [[i32; 2]; TIMING_JUDGE_AREAS] = [[-8, 8], [-20, 20], [-40, 40], [-70, 70], [-150, 150]];

/// A strip of 101 pixels over 101 milliseconds (one pixel to each), a window of ten hits, lines of
/// one pixel, and the reference's defaults otherwise.
fn body() -> HitErrorBody {
    HitErrorBody {
        width: 101,
        center: 50,
        rate: visualizer_rate(101, 50),
        line_width: 1,
        window: 10,
        ema_mode: 0,
        alpha: 0.5,
        marks: true,
        by_judgement: true,
        decay: true,
        line: LINE,
        centre: MIDDLE,
        average: AVERAGE,
        judges: [PERFECT, GREAT, GOOD, BAD, POOR],
        state: RefCell::new(Drawn { layer: Layer::new(next_serial(), "hit-error"), pixmap: Pixmap::default(), index: None, seen: 0, judge_area: None, ema: 0 }),
    }
}

/// Paints the strip for `errors`, oldest first, with a fresh average.
fn strip(body: &HitErrorBody, errors: &[i64]) -> Pixmap {
    paint(body, errors, &WINDOWS, &mut 0)
}

/// The columns of row `y` that hold something.
fn painted_in(pixmap: &Pixmap, y: usize) -> Vec<usize> {
    (0..pixmap.size().0).filter(|x| pixmap.pixel(*x, y)[3] != 0).collect()
}

#[test]
fn the_strip_is_the_record_width_by_twice_the_window() {
    let pixmap = strip(&body(), &[]);
    assert_eq!(pixmap.size(), (101, 20));
    assert_eq!(painted_in(&pixmap, 0), vec![50], "with nothing recorded only the middle line is there");
    assert_eq!(painted_in(&pixmap, 19), vec![50], "from the top of the strip to its foot");
}

#[test]
fn an_early_hit_is_to_the_left_of_the_middle_the_other_way_round_to_the_ruler() {
    let pixmap = strip(&body(), &[20]);
    let newest = painted_in(&pixmap, 0);
    assert_eq!(newest, vec![30, 50], "twenty milliseconds early is twenty pixels left of the middle line");
    let late = strip(&body(), &[-20]);
    assert_eq!(painted_in(&late, 0), vec![50, 70], "and late is to the right");
}

#[test]
fn a_hit_is_coloured_by_the_first_window_it_is_strictly_inside() {
    let at = |error: i64| strip(&body(), &[error]).pixel((50 - error.clamp(-50, 50)) as usize, 0);
    assert_eq!(at(7), PERFECT);
    assert_eq!(at(-7), PERFECT);
    assert_eq!(at(8), GREAT, "the edge of a window is outside it, so a hit exactly on it takes the next");
    assert_eq!(at(19), GREAT);
    assert_eq!(at(20), GOOD);
    assert_eq!(at(39), GOOD);
    assert_eq!(at(40), BAD);
    assert_eq!(at(69), BAD);
    assert_eq!(at(70), POOR, "a hit on the edge of the last window is the poor colour");
}

#[test]
fn a_hit_beyond_the_strip_is_drawn_on_its_edge() {
    let pixmap = strip(&body(), &[400]);
    assert!(painted_in(&pixmap, 0).contains(&0), "clamped to the milliseconds the strip reaches");
}

#[test]
fn with_decay_an_older_mark_is_shorter_and_without_it_every_mark_runs_the_whole_strip() {
    let pixmap = strip(&body(), &[30, 20, 10]);
    let newest_column = 50 - 10;
    let oldest_column = 50 - 30;
    let height_of = |column: usize| (0..20).filter(|y| pixmap.pixel(column, *y)[3] != 0).count();
    assert_eq!(height_of(newest_column), 20, "the newest mark is the whole height: it is 10 places from the oldest of a window of ten");
    assert_eq!(height_of(oldest_column), 16, "the third newest is eight places from the oldest, so two rows less to either side");
    assert_eq!(pixmap.pixel(oldest_column, 1), [0; 4], "the shorter mark starts lower");
    assert_eq!(pixmap.pixel(oldest_column, 2)[3], 255);

    let steady = HitErrorBody { decay: false, ..body() };
    let flat = strip(&steady, &[30, 20, 10]);
    assert!((0..20).all(|y| flat.pixel(oldest_column, y)[3] != 0), "without decay every mark is the strip's whole height");
}

#[test]
fn only_the_newest_hits_of_the_window_are_drawn() {
    let errors: Vec<i64> = (1..=12).collect();
    let pixmap = strip(&body(), &errors);
    let marked: Vec<usize> = painted_in(&pixmap, 9).into_iter().filter(|x| *x != 50).collect();
    assert_eq!(marked.len(), 10, "ten hits for a window of ten");
    assert!(!marked.contains(&(50 - 1)) && !marked.contains(&(50 - 2)), "the two oldest are gone");
    assert!(marked.contains(&(50 - 12)), "and the newest is there");
}

#[test]
fn the_fading_colour_spills_an_alpha_past_opaque_into_the_blue_channel() {
    assert_eq!(fading_color([0x99, 0xCC, 0xFE, 0x80], 5, 10), [0x99, 0xCC, 0xFE, 0x80], "half the window old: the record's own alpha");
    assert_eq!(fading_color([0x99, 0xCC, 0xFE, 0x80], 2, 10), [0x99, 0xCC, 0xFE, 0x33], "and a fifth of the window old: two fifths of it");
    let [_, _, blue, alpha] = fading_color([0x99, 0xCC, 0xFE, 0x80], 10, 10);
    assert_eq!((blue, alpha), (0xFF, 0x00), "twice the alpha is 256: the alpha byte wraps to zero and the carry sets the blue channel's low bit");
}

#[test]
fn the_average_follows_hits_inside_the_bad_window_by_the_records_weight_and_ignores_others() {
    let mut ema = 0;
    let lined = HitErrorBody { ema_mode: EMA_LINE, ..body() };
    paint(&lined, &[20], &WINDOWS, &mut ema);
    assert_eq!(ema, 10, "half of the way from nothing to 20");
    paint(&lined, &[20, 20], &WINDOWS, &mut ema);
    assert_eq!(ema, 15);
    paint(&lined, &[20, 20, 90], &WINDOWS, &mut ema);
    assert_eq!(ema, 15, "a hit outside the bad window is not fed to it");
    paint(&lined, &[-30], &WINDOWS, &mut ema);
    assert_eq!(ema, 15 + ((0.5_f32 * (-30.0 - 15.0)) as i64), "the weight is applied to the difference and truncated towards zero: {ema}");
    let mut fresh = 0;
    paint(&lined, &[], &WINDOWS, &mut fresh);
    assert_eq!(fresh, 0, "nothing recorded feeds nothing");
}

#[test]
fn the_average_is_a_line_a_triangle_or_both_at_the_same_column_as_a_hit_of_that_error() {
    let mut ema = 20;
    let lined = HitErrorBody { ema_mode: EMA_LINE, alpha: 0.0, marks: false, ..body() };
    let line = paint(&lined, &[], &WINDOWS, &mut ema);
    assert_eq!(painted_in(&line, 0), vec![30, 50], "20 milliseconds early, to the left of the middle line");
    assert_eq!(line.pixel(30, 19), AVERAGE, "the line is the whole height");

    let triangle = paint(&HitErrorBody { ema_mode: EMA_TRIANGLE, ..lined }, &[], &WINDOWS, &mut ema);
    assert_eq!(triangle.pixel(30, 19), [0; 4], "a triangle has no line below its point");
    assert_eq!(triangle.pixel(30, 6), AVERAGE, "its point is a third of the way down: row six of twenty");
    assert_eq!(triangle.pixel(30, 7), [0; 4]);
    assert_eq!(painted_in(&triangle, 0), vec![28, 29, 30, 31, 32, 50], "and its top is two pixels (the width, a hundredth, rounded up to even) to either side");

    let both = paint(&HitErrorBody { ema_mode: EMA_BOTH, marks: false, alpha: 0.0, ..body() }, &[], &WINDOWS, &mut ema);
    assert_eq!(both.pixel(30, 19), AVERAGE);
    assert_eq!(both.pixel(32, 0), AVERAGE);

    let none = paint(&HitErrorBody { ema_mode: 4, marks: false, alpha: 0.0, ..body() }, &[], &WINDOWS, &mut ema);
    assert_eq!(painted_in(&none, 0), vec![50], "a mode that names neither draws neither");
}

#[test]
fn a_triangle_narrows_to_its_point() {
    let mut pixmap = Pixmap::new(21, 10);
    fill_triangle(&mut pixmap, (10, 6), 4, AVERAGE);
    assert_eq!(painted_in(&pixmap, 0), (6..=14).collect::<Vec<_>>());
    assert_eq!(painted_in(&pixmap, 3), (8..=12).collect::<Vec<_>>());
    assert_eq!(painted_in(&pixmap, 6), vec![10]);
    assert!(painted_in(&pixmap, 7).is_empty());

    let mut flat = Pixmap::new(21, 4);
    fill_triangle(&mut flat, (10, 0), 3, AVERAGE);
    assert_eq!(painted_in(&flat, 0), (7..=13).collect::<Vec<_>>(), "a triangle with no height is its top row");
}

#[test]
fn translucent_marks_over_each_other_are_mixed_the_way_the_reference_mixes_them() {
    let sheer = HitErrorBody { judges: [[0, 0, 255, 0x80]; VISUALIZER_JUDGEMENTS], decay: false, ..body() };
    let pixmap = strip(&sheer, &[10, 10]);
    let once = blend_over([0, 0, 255, 0x80], [0; 4]);
    assert_eq!(pixmap.pixel(40, 0), blend_over([0, 0, 255, 0x80], once));
}

#[test]
fn the_buffer_the_visualiser_reads_is_the_last_hundred_recorded_hits() {
    let hits: Vec<(i64, u8)> = (0..150).map(|error| (error, 1)).collect();
    let recent = RecentHits::new(&hits);
    assert_eq!(recent.errors().len(), 100);
    assert_eq!(recent.recorded % RECENT_JUDGES, 50);
    assert_eq!(recent.with_recorded(300).recorded % RECENT_JUDGES, 0);
}

#[test]
fn a_record_builds_the_strip_it_names() {
    let record = HitErrorVisualizer { id: "errors".to_owned(), window_length: 500, ema_mode: 3, draw_decay: 0, color_mode: 0, ..HitErrorVisualizer::default() };
    let def = SkinDef { hiterrorvisualizer: vec![record], ..SkinDef::default() };
    let mut warnings = Vec::new();
    let Some(Body::HitError(built)) = build(&def, "errors", &mut warnings) else {
        panic!("the document declares a hit error visualiser called errors");
    };
    assert_eq!((built.window, built.ema_mode, built.decay, built.by_judgement, built.marks), (100, 3, false, false, true));
    assert_eq!(built.rate, 301.0 / 301.0);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(build(&def, "other", &mut warnings).is_none());
}
