//! The arithmetic of the tempo graph, checked on the pixmap it paints and without a renderer.

use super::*;

const MAIN: Rgba = [0, 255, 0, 255];
const LOWEST: Rgba = [0, 0, 255, 255];
const HIGHEST: Rgba = [255, 0, 0, 255];
const OTHER: Rgba = [255, 255, 0, 255];
const STOPPED: Rgba = [255, 0, 255, 255];
const TRANSITION: Rgba = [127, 127, 127, 255];
const NONE: Rgba = [0; 4];

const LINE: i32 = 2;
const WIDTH: i32 = 61;
const HEIGHT: i32 = 30;

fn palette() -> Palette {
    Palette { main: MAIN, lowest: LOWEST, highest: HIGHEST, other: OTHER, stopped: STOPPED, transition: TRANSITION }
}

fn timeline(changes: &[(f64, f64)], main: f64, min: f64, max: f64, length: Option<i32>) -> Painted {
    Painted { changes: changes.to_vec(), main_bpm: main, min_bpm: min, max_bpm: max, length_ms: length }
}

fn painted(changes: &[(f64, f64)], main: f64, min: f64, max: f64, length: Option<i32>) -> Pixmap {
    paint_timeline(WIDTH, HEIGHT, LINE, &timeline(changes, main, min, max, length), &palette()).expect("two changes make a timeline")
}

#[test]
fn a_tempo_sits_on_a_log_scale_of_its_ratio_to_the_main_tempo() {
    let at = |ratio: f64| level(150.0 * ratio, 150.0, HEIGHT, LINE);
    assert_eq!(at(1.0), 14, "the main tempo is half way up the room the line leaves");
    assert_eq!(at(2.0), 18);
    assert_eq!(at(0.5), 9);
    assert_eq!(at(8.0), 28, "an eighth fold tempo is at the ceiling, a line's thickness below the top");
    assert_eq!(at(100.0), 28, "and anything faster stays there");
    assert_eq!(at(0.125), 0);
    assert_eq!(at(0.001), 0, "while anything slower, a stop included, sits on the floor");
    assert_eq!(level(0.0, 150.0, HEIGHT, LINE), 0);
}

#[test]
fn a_step_line_is_a_flat_run_per_tempo_joined_by_an_upright() {
    let changes = [(150.0, 0.0), (300.0, 30_000.0), (150.0, 60_000.0)];
    let pixmap = painted(&changes, 150.0, 150.0, 300.0, Some(60_000));

    assert_eq!(pixmap.size(), (WIDTH as usize, HEIGHT as usize));
    assert_eq!(pixmap.pixel(10, 14), MAIN, "the opening tempo runs along the main level");
    assert_eq!(pixmap.pixel(10, 15), MAIN, "two pixels thick");
    assert_eq!(pixmap.pixel(10, 13), NONE);
    assert_eq!(pixmap.pixel(10, 16), NONE);
    assert_eq!(pixmap.pixel(45, 18), HIGHEST, "the faster half runs higher, in the colour of the chart's fastest tempo");
    assert_eq!(pixmap.pixel(45, 19), HIGHEST);
    assert_eq!(pixmap.pixel(30, 16), TRANSITION, "where the tempo changes an upright joins the two runs");
    assert_eq!(pixmap.pixel(31, 17), TRANSITION);
    assert_eq!(pixmap.pixel(60, 14), MAIN, "and the last tempo runs to the right edge");
    assert_eq!(pixmap.pixel(60, 16), TRANSITION, "after the second upright, the run back down");
}

#[test]
fn the_chart_is_stretched_over_the_width_with_a_second_to_spare() {
    let changes = [(150.0, 0.0), (150.0, 30_000.0)];
    let pixmap = painted(&changes, 150.0, 150.0, 150.0, Some(60_000));
    assert_eq!(pixmap.pixel(29, 14), MAIN);
    assert_eq!(pixmap.pixel(WIDTH as usize - 1, 14), MAIN);

    let steps = [(150.0, 0.0), (300.0, 30_000.0)];
    let at_end = painted(&steps, 150.0, 150.0, 300.0, None);
    assert_eq!(at_end.pixel(59, 16), TRANSITION, "with no length the last change is the end plus a second: 30000 of 31000 across 61 pixels is column 59");
    assert_eq!(at_end.pixel(60, 18), HIGHEST, "and the last run is what is left of the width");

    let shorter = painted(&steps, 150.0, 150.0, 300.0, Some(10_000));
    assert_eq!(shorter.pixel(WIDTH as usize - 1, 14), MAIN, "a song shorter than the chart pulls the end in, so the first run covers the width");
    assert_eq!(shorter.pixel(WIDTH as usize - 1, 18), NONE, "and the last change lands past the right edge, taking its run with it");
}

#[test]
fn each_tempo_is_coloured_by_what_it_is_to_the_chart() {
    let changes = [(150.0, 0.0), (100.0, 10_000.0), (300.0, 20_000.0), (0.0, 30_000.0), (120.0, 40_000.0), (150.0, 50_000.0)];
    let pixmap = painted(&changes, 150.0, 100.0, 300.0, Some(50_000));
    let column = |time: f64| (61.0 * time / 51_000.0) as usize + 2;
    let level_of = |speed: f64| level(speed, 150.0, HEIGHT, LINE) as usize;

    assert_eq!(pixmap.pixel(column(0.0), level_of(150.0)), MAIN);
    assert_eq!(pixmap.pixel(column(10_000.0), level_of(100.0)), LOWEST);
    assert_eq!(pixmap.pixel(column(20_000.0), level_of(300.0)), HIGHEST);
    assert_eq!(pixmap.pixel(column(30_000.0), level_of(0.0)), STOPPED);
    assert_eq!(pixmap.pixel(column(40_000.0), level_of(120.0)), OTHER);
}

#[test]
fn an_upright_no_taller_than_the_line_is_not_drawn() {
    let changes = [(150.0, 0.0), (160.0, 30_000.0), (150.0, 60_000.0)];
    let pixmap = painted(&changes, 150.0, 150.0, 160.0, Some(60_000));
    let level_a = level(150.0, 150.0, HEIGHT, LINE);
    let level_b = level(160.0, 150.0, HEIGHT, LINE);
    assert!((level_b - level_a).abs() <= LINE, "the fixture's two tempos are close enough that no upright is wanted");
    assert!((0..HEIGHT as usize).all(|y| (0..WIDTH as usize).all(|x| pixmap.pixel(x, y) != TRANSITION)));
}

#[test]
fn a_timeline_with_nothing_to_join_or_no_main_tempo_paints_nothing() {
    let one = timeline(&[(150.0, 0.0)], 150.0, 150.0, 150.0, None);
    assert!(paint_timeline(WIDTH, HEIGHT, LINE, &one, &palette()).is_none());
    let none = timeline(&[], 150.0, 150.0, 150.0, None);
    assert!(paint_timeline(WIDTH, HEIGHT, LINE, &none, &palette()).is_none());
    let zero = timeline(&[(150.0, 0.0), (160.0, 1_000.0)], 0.0, 0.0, 0.0, None);
    assert!(paint_timeline(WIDTH, HEIGHT, LINE, &zero, &palette()).is_none(), "a main tempo of zero has no scale to put the others on");
}

#[test]
fn a_colour_is_the_first_six_hex_digits_a_record_wrote() {
    let mut warnings = Vec::new();
    let read = |text: &str, warnings: &mut Vec<String>| color_of(text, 0x123456, "main tempo", "tempo", warnings);

    assert_eq!(read("ff8000", &mut warnings), [255, 128, 0, 255]);
    assert_eq!(read("#FF8000", &mut warnings), [255, 128, 0, 255], "a hash is not a digit");
    assert_eq!(read("ff800080", &mut warnings), [255, 128, 0, 255], "and an alpha past the sixth digit is dropped");
    assert_eq!(read("zz", &mut warnings), [0x12, 0x34, 0x56, 255], "no digits at all is the record's fallback");
    assert_eq!(read("", &mut warnings), [0x12, 0x34, 0x56, 255]);
    assert!(warnings.is_empty());

    assert_eq!(read("abc", &mut warnings), [0x12, 0x34, 0x56, 255], "too few digits cannot be a colour");
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("\"abc\"") && warnings[0].contains("main tempo"), "{warnings:?}");
}

#[test]
fn progress_points_are_laid_over_a_nominal_chart_with_an_end_to_run_to() {
    let points = [(0.0, 150.0), (0.5, 300.0)];
    let timeline = BpmTimeline::new(&points);
    assert_eq!((timeline.main_bpm, timeline.min_bpm, timeline.max_bpm), (150.0, 150.0, 300.0));
    assert_eq!(timeline.speeds().into_owned(), vec![(150.0, 0.0), (300.0, 500_000.0), (300.0, 1_000_000.0)]);
    assert_eq!(timeline.length(), Some(1_000_000));

    assert!(BpmTimeline::new(&[]).speeds().is_empty());
    assert_eq!(BpmTimeline::new(&[(0.0, 0.0), (0.5, 90.0)]).min_bpm, 90.0, "a stop is not the slowest tempo");
}

#[test]
fn a_chart_in_the_references_own_shape_is_used_as_given() {
    let changes = [(150.0, 0.0), (300.0, 5.0)];
    let timeline = BpmTimeline::of_chart(&changes, 150.0, 150.0, 300.0, Some(7));
    assert_eq!(timeline.speeds().into_owned(), changes.to_vec());
    assert_eq!(timeline.length(), Some(7));
    assert_eq!(BpmTimeline::of_chart(&changes, 150.0, 150.0, 300.0, None).length(), None);
}
