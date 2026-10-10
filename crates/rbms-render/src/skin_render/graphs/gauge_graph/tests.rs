//! The arithmetic of the gauge graph, checked on the pixmaps it paints and without a renderer.

use super::*;

const RED: Rgba = [255, 0, 0, 255];
const GREEN: Rgba = [0, 255, 0, 255];
const DARK_RED: Rgba = [0x44, 0, 0, 255];
const DARK_GREEN: Rgba = [0, 0x44, 0, 255];
const NONE: Rgba = [0; 4];

/// A set of colours that tells the two sides of the clear line apart, line and ground.
const BAND: Band = Band { line_above: RED, ground_above: DARK_RED, line_below: GREEN, ground_below: DARK_GREEN };

/// A gauge that clears at eighty of a hundred.
const GROOVE: GaugeScale = GaugeScale::new(2.0, 100.0, 80.0);

/// A gauge that clears at nothing.
const SURVIVAL: GaugeScale = GaugeScale::new(0.0, 100.0, 0.0);

/// A rectangle whose height leaves a line two pixels thick exactly fifty rows to climb, so a gauge
/// value is twice the row its line starts on.
const SIZE: (f32, f32) = (100.0, 52.0);

/// The colours of one column of a pixmap, from row zero up to but not including `rows`.
fn column(pixmap: &Pixmap, x: usize, rows: std::ops::Range<usize>) -> Vec<Rgba> {
    rows.map(|y| pixmap.pixel(x, y)).collect()
}

#[test]
fn the_ground_is_one_colour_below_the_clear_line_and_another_from_it_to_the_top() {
    let ground = paint_ground((100.0, 50.0), GROOVE, &BAND);
    assert_eq!(ground.size(), (100, 50));
    assert_eq!(ground.pixel(0, 0), DARK_GREEN, "row zero is the foot of the graph");
    assert_eq!(ground.pixel(99, 39), DARK_GREEN, "eighty of a hundred is forty rows of fifty");
    assert_eq!(ground.pixel(0, 40), DARK_RED);
    assert_eq!(ground.pixel(99, 49), DARK_RED);

    let survival = paint_ground((100.0, 50.0), SURVIVAL, &BAND);
    assert_eq!((survival.pixel(0, 0), survival.pixel(99, 49)), (DARK_RED, DARK_RED), "a gauge that clears at nothing is all above its line");
}

#[test]
fn a_translucent_colour_above_the_line_is_mixed_into_the_one_below_rather_than_replacing_it() {
    let band = Band { ground_below: [0x44, 0, 0x44, 0x55], ground_above: [0x44, 0, 0, 0x55], ..BAND };
    let ground = paint_ground((10.0, 10.0), GROOVE, &band);
    assert_eq!(ground.pixel(5, 3), [0x44, 0, 0x44, 0x55], "the first colour is written as it is");
    assert_eq!(ground.pixel(5, 9), [0x44, 0, 46, 141], "the second is blended over it, as the reference's pixmap blends");
}

#[test]
fn the_size_of_the_pixmaps_is_the_rectangle_truncated_and_the_arithmetic_reads_it_whole() {
    let ground = paint_ground((10.9, 20.9), GROOVE, &BAND);
    assert_eq!(ground.size(), (10, 20));
    assert_eq!(ground.pixel(0, 15), DARK_GREEN, "20.9 * 80 / 100 is 16.72, so the line is on row sixteen");
    assert_eq!(ground.pixel(0, 16), DARK_RED);
    assert_eq!(paint_line((10.9, 20.9), GROOVE, &BAND, &[], &[]).size(), (10, 20));
}

#[test]
fn a_steady_gauge_is_a_flat_line_two_pixels_thick_at_its_share_of_the_height_less_the_line() {
    let line = paint_line(SIZE, GROOVE, &BAND, &[50.0; 4], &[]);
    assert_eq!(column(&line, 40, 23..29), vec![NONE, NONE, GREEN, GREEN, NONE, NONE], "fifty of a hundred over fifty rows is row twenty-five");
    assert!((0..100).all(|x| line.pixel(x, 25) == GREEN), "the last sample's run is carried to the right edge");

    let full = paint_line(SIZE, GROOVE, &BAND, &[100.0; 4], &[]);
    assert_eq!(column(&full, 40, 49..52), vec![NONE, RED, RED], "a full gauge sits on the top two rows, in the colour above the line");
    let empty = paint_line(SIZE, GROOVE, &BAND, &[0.0; 4], &[]);
    assert_eq!(column(&empty, 40, 0..3), vec![GREEN, GREEN, NONE], "and an empty one on the foot");
}

#[test]
fn each_sample_is_joined_to_the_one_before_by_an_upright_at_the_earlier_column() {
    let line = paint_line(SIZE, GROOVE, &BAND, &[20.0, 60.0], &[]);
    assert_eq!(column(&line, 0, 9..33), [vec![NONE], vec![GREEN; 22], vec![NONE]].concat(), "from row ten to row thirty and the line's own thickness");
    assert_eq!(line.pixel(1, 20), GREEN, "two pixels wide");
    assert_eq!(line.pixel(2, 20), NONE);
    assert_eq!(column(&line, 30, 29..33), vec![NONE, GREEN, GREEN, NONE], "then flat at the later sample's level");
    assert_eq!(line.pixel(30, 10), NONE, "the earlier sample has no run of its own before the first");

    let falling = paint_line(SIZE, GROOVE, &BAND, &[60.0, 20.0], &[]);
    assert_eq!(column(&falling, 0, 9..33), [vec![NONE], vec![GREEN; 22], vec![NONE]].concat(), "a fall is the same upright");
    assert_eq!(column(&falling, 30, 9..13), vec![NONE, GREEN, GREEN, NONE]);
}

#[test]
fn a_rise_through_the_clear_line_changes_colour_where_it_crosses() {
    let line = paint_line(SIZE, GROOVE, &BAND, &[0.0, 100.0], &[]);
    assert_eq!(column(&line, 0, 0..52), [vec![GREEN; 40], vec![RED; 12]].concat(), "the line is on row forty of the fifty the gauge climbs");
    assert_eq!(column(&line, 1, 38..42), vec![GREEN, GREEN, RED, RED]);
    assert_eq!(column(&line, 30, 48..52), vec![NONE, NONE, RED, RED], "the run takes the colour of the side it ends on");
    assert_eq!(line.pixel(99, 51), RED);
    assert_eq!(line.pixel(30, 0), NONE);
}

#[test]
fn a_fall_through_the_clear_line_changes_colour_where_it_crosses() {
    let line = paint_line(SIZE, GROOVE, &BAND, &[100.0, 0.0], &[]);
    assert_eq!(column(&line, 0, 0..52), [vec![GREEN; 40], vec![RED; 12]].concat());
    assert_eq!(column(&line, 30, 0..3), vec![GREEN, GREEN, NONE], "the run takes the colour of the side it ends on");
    assert_eq!(line.pixel(99, 1), GREEN);
    assert_eq!(line.pixel(30, 51), NONE);
}

#[test]
fn a_sample_on_the_clear_line_counts_as_above_it() {
    let line = paint_line(SIZE, GROOVE, &BAND, &[80.0, 80.0], &[]);
    assert_eq!(column(&line, 30, 39..43), vec![NONE, RED, RED, NONE]);
    let under = paint_line(SIZE, GROOVE, &BAND, &[79.9, 79.9], &[]);
    assert_eq!(column(&under, 30, 38..42), vec![NONE, GREEN, GREEN, NONE], "79.9 of a hundred over fifty rows is 39.95, truncated");
}

#[test]
fn the_columns_are_the_width_shared_out_over_the_samples_and_truncated() {
    let line = paint_line((10.0, 12.0), SURVIVAL, &BAND, &[0.0, 100.0, 0.0], &[]);
    assert_eq!(line.pixel(0, 5), RED, "the first join stands on column zero");
    assert_eq!(line.pixel(3, 5), RED, "and the second on ten thirds, truncated");
    assert_eq!(line.pixel(4, 5), RED);
    assert_eq!((line.pixel(2, 5), line.pixel(5, 5)), (NONE, NONE));
    assert_eq!((line.pixel(1, 10), line.pixel(2, 10)), (RED, RED), "the first run reaches the second join");
    assert_eq!(line.pixel(9, 0), RED, "and the last is carried from six to the edge");
}

#[test]
fn one_sample_or_none_draws_no_line() {
    for samples in [&[][..], &[50.0][..]] {
        let line = paint_line(SIZE, GROOVE, &BAND, samples, &[]);
        assert!((0..100).all(|x| (0..52).all(|y| line.pixel(x, y) == NONE)), "{} samples", samples.len());
    }
}

#[test]
fn the_end_of_a_courses_stage_is_a_white_upright_under_the_line() {
    let line = paint_line(SIZE, GROOVE, &BAND, &[50.0; 4], &[2, 4]);
    assert_eq!(line.pixel(25, 0), SECTION_LINE, "the stage that ends after two samples is marked at the column of the second");
    assert_eq!(line.pixel(25, 51), SECTION_LINE, "over the whole height");
    assert_eq!(line.pixel(25, 25), GREEN, "and the line is painted over it");
    assert_eq!(line.pixel(26, 0), NONE, "it is one pixel wide");
    assert_eq!(line.pixel(75, 0), NONE, "the end of the last stage is past the last sample and is not marked");

    let unmarked = paint_line(SIZE, GROOVE, &BAND, &[50.0; 4], &[0]);
    assert!((0..100).all(|x| unmarked.pixel(x, 0) == NONE), "a stage that ends before the first sample would be marked left of the graph");
}

#[test]
fn the_line_is_revealed_over_a_second_and_a_half() {
    assert_eq!(revealed(0), 0.0);
    assert_eq!(revealed(375), 0.25);
    assert_eq!(revealed(750), 0.5);
    assert_eq!(revealed(1_500), 1.0);
    assert_eq!(revealed(90_000), 1.0);
}

#[test]
fn the_course_gauges_are_drawn_in_the_colours_of_the_gauges_they_are_harder_forms_of() {
    assert_eq!(&BAND_OF_TYPE[..9], &[0, 1, 2, 3, 4, 5, 3, 4, 5]);
}

/// A colour as the six hex digits a record writes it in, opaque.
fn rgb(hex: u32) -> Rgba {
    let [_, red, green, blue] = hex.to_be_bytes();
    [red, green, blue, 255]
}

#[test]
fn the_named_colours_give_the_three_gauges_with_a_clear_line_a_pair_for_each_side_of_it() {
    let mut warnings = Vec::new();
    let bands = named_bands(&GaugeGraph::default(), &mut warnings);
    assert!(warnings.is_empty());
    let (above_line, above_ground) = (rgb(0xff0000), rgb(0x440000));
    assert_eq!(bands[0], Band { line_above: above_line, ground_above: above_ground, line_below: rgb(0xff00ff), ground_below: rgb(0x440044) });
    assert_eq!(bands[1], Band { line_above: above_line, ground_above: above_ground, line_below: rgb(0x00ffff), ground_below: rgb(0x004444) });
    assert_eq!(bands[2], Band { line_above: above_line, ground_above: above_ground, line_below: rgb(0x00ff00), ground_below: rgb(0x004400) });
}

#[test]
fn the_named_colours_give_the_three_gauges_that_clear_at_nothing_one_pair_each() {
    let mut warnings = Vec::new();
    let bands = named_bands(&GaugeGraph::default(), &mut warnings);
    for (band, line, ground) in [(3, 0xff0000, 0x440000), (4, 0xffff00, 0x444400), (5, 0xcccccc, 0x444444)] {
        let (line, ground) = (rgb(line), rgb(ground));
        assert_eq!(bands[band], Band { line_above: line, ground_above: ground, line_below: line, ground_below: ground }, "band {band}");
    }
}

#[test]
fn a_colour_list_is_read_four_to_a_gauge_and_what_it_leaves_out_is_black() {
    let texts = ["112233", "445566", "778899", "aabbcc", "ddeeff"];
    let graph = GaugeGraph { color: texts.map(str::to_owned).to_vec(), ..GaugeGraph::default() };
    let mut warnings = Vec::new();
    let bands = listed_bands(&graph, &mut warnings);
    assert!(warnings.is_empty());
    assert_eq!(bands[0], Band { line_above: rgb(0x112233), ground_above: rgb(0x445566), line_below: rgb(0x778899), ground_below: rgb(0xaabbcc) });
    assert_eq!(bands[1], Band { line_above: rgb(0xddeeff), ground_above: MISSING_COLOR, line_below: MISSING_COLOR, ground_below: MISSING_COLOR });
    assert!(
        bands[2..]
            .iter()
            .all(|band| *band == Band { line_above: MISSING_COLOR, ground_above: MISSING_COLOR, line_below: MISSING_COLOR, ground_below: MISSING_COLOR })
    );

    let long = GaugeGraph { color: vec!["ffffff".to_owned(); 30], ..GaugeGraph::default() };
    assert_eq!(listed_bands(&long, &mut warnings)[5].ground_below, rgb(0xffffff), "the twenty-fourth is the last one read");
}

#[test]
fn a_colour_of_eight_digits_carries_its_alpha_and_one_that_is_no_colour_is_black_with_a_warning() {
    let graph =
        GaugeGraph { id: "trend".to_owned(), assist_clear_bg_color: "44004455".to_owned(), hazard_line_color: "nothing".to_owned(), ..GaugeGraph::default() };
    let mut warnings = Vec::new();
    let bands = named_bands(&graph, &mut warnings);
    assert_eq!(bands[0].ground_below, [0x44, 0, 0x44, 0x55]);
    assert_eq!(bands[5].line_above, MISSING_COLOR);
    assert!(warnings.iter().any(|warning| warning.contains("trend") && warning.contains("hazardLineColor")), "{warnings:?}");
}

#[test]
fn a_history_answers_for_the_gauge_that_is_shown() {
    let kinds = vec![vec![1.0], vec![2.0, 3.0], vec![]];
    let every = GaugeHistory::of_kinds(&kinds);
    assert_eq!(every.of(1), Some(&[2.0, 3.0][..]));
    assert_eq!(every.of(2), Some(&[][..]));
    assert_eq!(every.of(3), None, "a gauge the run recorded nothing for");

    let only = [5.0, 6.0];
    let one = GaugeHistory::new(&only);
    assert_eq!((one.of(0), one.of(8)), (Some(&only[..]), Some(&only[..])), "a single history is plotted whichever gauge is shown");
    assert!(one.sections.is_empty());
    assert_eq!(one.with_sections(&[2]).sections, &[2]);
}
