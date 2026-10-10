//! The arithmetic of the timing distribution graph, checked on the pixmap it paints and without a
//! renderer.

use super::*;

const PERFECT: Rgba = [0, 0, 0x88, 255];
const GREAT: Rgba = [0, 0x88, 0, 255];
const GOOD: Rgba = [0x88, 0x88, 0, 255];
const BAD: Rgba = [0x88, 0, 0, 255];
const POOR: Rgba = [0, 0, 0, 255];
const WHITE: Rgba = [255; 4];
const GREY: Rgba = [0xAA, 0xAA, 0xAA, 255];
const SOLID_BAR: Rgba = [0, 255, 0, 255];

/// The bar colour of the skin this was written against: green, not quite opaque.
const SHEER_BAR: Rgba = [0, 255, 0, 0xEE];

/// How many entries the test histograms hold: thirty milliseconds to either side of on time.
const BINS: usize = 61;

/// The entry of a test histogram that counts the hits exactly on time.
const ON_TIME: usize = BINS / 2;

/// Windows that put the best judgement within five milliseconds, the next within fifteen and the
/// third well past what a graph of sixty-one columns shows.
const WINDOWS: [[i32; 2]; TIMING_JUDGE_AREAS] = [[-5, 5], [-15, 15], [-40, 40], [-40, 40], [-40, 40]];

/// A graph of `columns` columns in the reference's default colours, with `bar` for its bars.
fn body(columns: i32, bar: Rgba) -> TimingDistributionBody {
    TimingDistributionBody {
        columns,
        graph: bar,
        average: WHITE,
        deviation: GREY,
        judges: [PERFECT, GREAT, GOOD, BAD, POOR],
        draw_average: true,
        draw_deviation: true,
        state: RefCell::new(Drawn { layer: Layer::new(next_serial(), "timing"), pixmap: Pixmap::default(), painted: None }),
    }
}

/// A run that hit ten notes on time, five ten milliseconds early and two twenty late.
fn run() -> [u32; BINS] {
    let mut bins = [0; BINS];
    bins[ON_TIME] = 10;
    bins[ON_TIME + 10] = 5;
    bins[ON_TIME - 20] = 2;
    bins
}

/// One column of a pixmap, top row first.
fn column(pixmap: &Pixmap, x: usize) -> Vec<Rgba> {
    (0..pixmap.size().1).map(|y| pixmap.pixel(x, y)).collect()
}

#[test]
fn the_mean_and_the_deviation_are_those_of_the_offsets_from_the_middle_entry() {
    let mut bins = [0_u32; 301];
    assert_eq!(statistics(&bins), (f32::MAX, -1.0), "a run that hit nothing has neither");

    bins[150 + 10] = 4;
    assert_eq!(statistics(&bins), (10.0, 0.0), "early is the positive side");

    bins[150 + 10] = 0;
    bins[150 - 10] = 1;
    bins[150 + 30] = 3;
    assert_eq!(statistics(&bins), (20.0, 300.0_f64.sqrt() as f32));

    let (average, deviation) = statistics(&run());
    assert_eq!(average, 10.0 / 17.0, "a whole-number sum over a whole-number count");
    assert!((deviation - 8.7249).abs() < 1e-3, "the root of 1294.12 over seventeen: {deviation}");
    assert_eq!(statistics(&[]), (f32::MAX, -1.0));
}

#[test]
fn a_histogram_made_from_its_counts_carries_their_statistics_and_no_windows() {
    let bins = run();
    let histogram = TimingHistogram::new(&bins);
    assert_eq!((histogram.average, histogram.std_dev), statistics(&bins));
    assert_eq!(histogram.judge_area, [[0; 2]; TIMING_JUDGE_AREAS]);
    assert_eq!(histogram.center(), 30);
    assert_eq!(histogram.with_judge_area(WINDOWS).judge_area, WINDOWS);
    assert_eq!(TimingHistogram::new(&[0; 301]).center(), 150, "the reference's own range");
}

#[test]
fn a_graph_is_ten_rows_tall_or_its_fullest_millisecond_rounded_up_to_the_next_ten() {
    for (fullest, rows) in [(0, 10), (9, 10), (10, 10), (11, 20), (19, 20), (20, 30), (25, 30), (100, 110)] {
        assert_eq!(row_count(&[1, fullest, 3]), rows, "a fullest millisecond of {fullest} hits");
    }
    assert_eq!(row_count(&[]), 10);
}

#[test]
fn the_columns_are_the_width_over_the_line_width_held_between_one_and_the_width() {
    assert_eq!(column_count(301, 1), Some(301));
    assert_eq!(column_count(450, 2), Some(225));
    assert_eq!(column_count(300, 7), Some(42));
    assert_eq!(column_count(301, 0), Some(301), "no line width is a line width of one");
    assert_eq!(column_count(301, -4), Some(301));
    assert_eq!(column_count(301, 500), Some(1), "a line wider than the graph is the graph's width");
    assert_eq!(column_count(1, 1), Some(1));
    assert_eq!(column_count(0, 1), None, "the reference divides by a width of nothing here");
    assert_eq!(column_count(-3, 1), Some(0), "and by a negative width, which leaves no column");
}

#[test]
fn a_number_is_rounded_half_up_the_way_the_reference_rounds_it() {
    assert_eq!([0.4, 0.5, 1.5, 2.5, -0.4, -0.5, -0.6, -1.5].map(java_round), [0, 1, 2, 3, 0, 0, -1, -1]);
    assert_eq!(java_round(f32::NAN), 0);
    assert_eq!(java_round(f32::MAX), i32::MAX);
}

#[test]
fn a_colour_is_six_hex_digits_or_more_and_anything_else_is_red() {
    let mut warnings = Vec::new();
    let mut read = |text: &str| validated_color(text, "graphColor", "spread", &mut warnings);
    assert_eq!(read("00FF00EE"), [0, 255, 0, 0xEE]);
    assert_eq!(read("000088"), [0, 0, 0x88, 255]);
    assert_eq!(read("00ff001"), [0, 255, 0, 255], "seven digits are a colour and no alpha");
    assert_eq!(read("0011223344"), [0, 0x11, 0x22, 255], "and so are ten");
    assert!(warnings.is_empty());

    for text in ["12345", "GG0000", "#00FF00", "00 FF 00", ""] {
        assert_eq!(validated_color(text, "graphColor", "spread", &mut warnings), INVALID_COLOR, "{text:?}");
    }
    assert_eq!(warnings.len(), 5);
    assert!(warnings.iter().all(|warning| warning.contains("spread") && warning.contains("graphColor")), "{warnings:?}");
}

#[test]
fn the_ground_is_the_judgement_windows_as_bands_around_the_on_time_column() {
    let bins = [0; BINS];
    let pixmap = paint(&body(BINS as i32, SOLID_BAR), &TimingHistogram::new(&bins).with_judge_area(WINDOWS));
    assert_eq!(pixmap.size(), (BINS, 10));
    let row: Vec<Rgba> = (0..BINS).map(|x| pixmap.pixel(x, 5)).collect();
    let expected = [vec![GOOD; 15], vec![GREAT; 10], vec![PERFECT; 11], vec![GREAT; 10], vec![GOOD; 15]].concat();
    assert_eq!(row, expected, "five either side of the middle, then out to fifteen, then to the edge the third window is cut at");
    assert_eq!(pixmap.pixel(31, 9), PERFECT, "a run that hit nothing has no mean to draw");
}

#[test]
fn windows_nobody_supplied_leave_only_the_on_time_column() {
    let bins = [0; BINS];
    let pixmap = paint(&body(BINS as i32, SOLID_BAR), &TimingHistogram::new(&bins));
    assert_eq!((pixmap.pixel(29, 5), pixmap.pixel(30, 5), pixmap.pixel(31, 5)), ([0; 4], PERFECT, [0; 4]));
}

#[test]
fn a_window_is_painted_only_where_it_reaches_past_the_ones_before_it() {
    let bins = [0; BINS];
    let lopsided = [[-2, 10], [-20, 4], [-20, 12], [0, 0], [-30, 30]];
    let pixmap = paint(&body(BINS as i32, SOLID_BAR), &TimingHistogram::new(&bins).with_judge_area(lopsided));
    let row: Vec<Rgba> = (0..BINS).map(|x| pixmap.pixel(x, 5)).collect();
    let expected = [vec![POOR; 10], vec![GREAT; 18], vec![PERFECT; 13], vec![GOOD; 2], vec![POOR; 18]].concat();
    assert_eq!(row, expected, "the second window adds nothing on the early side, and the fourth nothing at all");
}

#[test]
fn every_tenth_column_from_the_middle_is_ticked_on_the_top_two_rows() {
    let bins = [0; BINS];
    let pixmap = paint(&body(BINS as i32, SOLID_BAR), &TimingHistogram::new(&bins).with_judge_area(WINDOWS));
    assert_eq!(column(&pixmap, 20)[..3], [[0, 103, 0, 255], [0, 103, 0, 255], GREAT], "a quarter black mixed into the window under it");
    assert_eq!(column(&pixmap, 50)[..3], [[103, 103, 0, 255], [103, 103, 0, 255], GOOD]);
    assert_eq!(column(&pixmap, 21)[..2], [GREAT, GREAT]);
    assert_eq!((0..BINS).filter(|x| pixmap.pixel(*x, 0) != pixmap.pixel(*x, 5)).collect::<Vec<_>>(), vec![0, 10, 20, 30, 40, 50, 60]);

    let narrow = paint(&body(27, SOLID_BAR), &TimingHistogram::new(&bins).with_judge_area(WINDOWS));
    assert_eq!(
        (0..27).filter(|x| narrow.pixel(*x, 0) != narrow.pixel(*x, 5)).collect::<Vec<_>>(),
        vec![3, 13, 23],
        "the ticks are counted out from the middle column"
    );
}

#[test]
fn the_bars_hang_from_the_foot_one_row_to_a_hit_over_the_lines_and_the_ground() {
    let bins = run();
    let pixmap = paint(&body(BINS as i32, SOLID_BAR), &TimingHistogram::new(&bins).with_judge_area(WINDOWS));
    assert_eq!(pixmap.size(), (BINS, 10));
    assert_eq!(column(&pixmap, 30), vec![SOLID_BAR; 10], "ten hits on time fill the ten rows");
    assert_eq!(column(&pixmap, 40), [vec![GREY; 5], vec![SOLID_BAR; 5]].concat(), "five hits ten milliseconds early, over the deviation line");
    assert_eq!(column(&pixmap, 10)[7..], [GOOD, SOLID_BAR, SOLID_BAR], "two hits twenty milliseconds late");
    assert_eq!(column(&pixmap, 31), vec![WHITE; 10], "the mean of ten seventeenths of a millisecond rounds to one");
    assert_eq!(column(&pixmap, 22), vec![GREY; 10], "and the deviation of 8.7 to nine either side of that");
}

#[test]
fn a_translucent_bar_is_mixed_into_what_is_behind_it() {
    let bins = run();
    let pixmap = paint(&body(BINS as i32, SHEER_BAR), &TimingHistogram::new(&bins).with_judge_area(WINDOWS));
    assert_eq!(pixmap.pixel(30, 5), [0, 238, 10, 255], "over the best window");
    assert_eq!(pixmap.pixel(30, 0), [0, 238, 7, 255], "over the tick on it");
    assert_eq!(pixmap.pixel(40, 9), [12, 249, 12, 255], "over the deviation line");
    assert_eq!(pixmap.pixel(10, 9), [10, 247, 0, 255], "over the third window");

    let bare = paint(&body(BINS as i32, SHEER_BAR), &TimingHistogram::new(&bins));
    assert_eq!(bare.pixel(10, 9), [0, 238, 0, 0xEE], "and over nothing it is darkened by its own alpha");
}

#[test]
fn a_record_can_leave_the_mean_and_the_deviation_out() {
    let bins = run();
    let histogram = TimingHistogram::new(&bins).with_judge_area(WINDOWS);
    let mut plain = body(BINS as i32, SOLID_BAR);
    plain.draw_average = false;
    let pixmap = paint(&plain, &histogram);
    assert_eq!((pixmap.pixel(31, 5), pixmap.pixel(22, 5)), (PERFECT, GREY));

    plain.draw_deviation = false;
    let pixmap = paint(&plain, &histogram);
    assert_eq!((pixmap.pixel(31, 5), pixmap.pixel(22, 5), pixmap.pixel(40, 2)), (PERFECT, GREAT, GREAT));
}

#[test]
fn statistics_that_fall_off_the_graph_draw_no_line_and_the_frames_own_are_the_ones_drawn() {
    let bins = run();
    let mut histogram = TimingHistogram::new(&bins).with_judge_area(WINDOWS);
    (histogram.average, histogram.std_dev) = (-12.4, 3.5);
    let pixmap = paint(&body(BINS as i32, SOLID_BAR), &histogram);
    assert_eq!(column(&pixmap, 18)[2..], vec![WHITE; 8], "twelve milliseconds late");
    assert_eq!((pixmap.pixel(14, 5), pixmap.pixel(22, 5)), (GREY, GREY), "and four either side, 3.5 rounding up");

    (histogram.average, histogram.std_dev) = (-500.0, 1_000.0);
    let pixmap = paint(&body(BINS as i32, SOLID_BAR), &histogram);
    assert!((0..BINS).all(|x| pixmap.pixel(x, 5) != WHITE && pixmap.pixel(x, 5) != GREY));
}

#[test]
fn the_two_ends_of_the_histogram_are_never_drawn() {
    let mut bins = [0; BINS];
    (bins[0], bins[BINS - 1], bins[1], bins[BINS - 2]) = (4, 4, 3, 3);
    let mut plain = body(BINS as i32, SOLID_BAR);
    (plain.draw_average, plain.draw_deviation) = (false, false);
    let pixmap = paint(&plain, &TimingHistogram::new(&bins));
    assert_eq!((pixmap.pixel(0, 9), pixmap.pixel(BINS - 1, 9)), ([0; 4], [0; 4]), "the reference leaves the entries at either end out");
    assert_eq!((pixmap.pixel(1, 9), pixmap.pixel(BINS - 2, 9)), (SOLID_BAR, SOLID_BAR));
}

#[test]
fn a_graph_narrower_or_wider_than_the_histogram_keeps_on_time_in_its_middle_column() {
    let bins = run();
    let mut narrow = body(21, SOLID_BAR);
    (narrow.draw_average, narrow.draw_deviation) = (false, false);
    let pixmap = paint(&narrow, &TimingHistogram::new(&bins));
    assert_eq!(pixmap.size(), (21, 10));
    assert_eq!(pixmap.pixel(10, 0), SOLID_BAR, "on time is the middle of twenty-one columns");
    assert_eq!(pixmap.pixel(20, 9), SOLID_BAR, "ten early is its last column");
    assert_eq!(pixmap.pixel(0, 9), [0; 4], "and twenty late is off its left edge");

    let mut wide = body(101, SOLID_BAR);
    (wide.draw_average, wide.draw_deviation) = (false, false);
    let pixmap = paint(&wide, &TimingHistogram::new(&bins));
    assert_eq!((pixmap.pixel(50, 0), pixmap.pixel(60, 9), pixmap.pixel(30, 9)), (SOLID_BAR, SOLID_BAR, SOLID_BAR));
    assert_eq!((pixmap.pixel(20, 9), pixmap.pixel(80, 9)), ([0; 4], [0; 4]), "past the histogram's ends there is nothing to draw");
}

#[test]
fn a_pixmap_is_painted_again_only_for_other_numbers() {
    let bins = run();
    let histogram = TimingHistogram::new(&bins).with_judge_area(WINDOWS);
    let painted = Painted { bins: bins.to_vec(), average: histogram.average.to_bits(), std_dev: histogram.std_dev.to_bits(), judge_area: WINDOWS };
    assert!(painted.shows(&histogram));
    assert!(!painted.shows(&TimingHistogram::new(&bins)), "other windows");
    assert!(!painted.shows(&TimingHistogram { average: 2.0, ..histogram }), "another mean");
    let mut other = bins;
    other[3] = 1;
    assert!(!painted.shows(&TimingHistogram { bins: &other, ..histogram }), "other counts");

    let unmeasured = TimingHistogram { average: f32::NAN, ..histogram };
    let painted = Painted { average: f32::NAN.to_bits(), ..painted };
    assert!(painted.shows(&unmeasured), "a mean that is not a number is still the same mean");
}
