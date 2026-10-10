//! The arithmetic of the timing visualiser, checked on the ruler it paints and on where it puts each
//! line, without a renderer.

use rbms_skin::model::TimingVisualizer;

use super::super::pixmap::blend_over;
use super::*;

const PERFECT: Rgba = [0, 0, 0x88, 255];
const GREAT: Rgba = [0, 0x88, 0, 255];
const GOOD: Rgba = [0x88, 0x88, 0, 255];
const BAD: Rgba = [0x88, 0, 0, 255];
const POOR: Rgba = [0x22, 0x22, 0x22, 255];
const MIDDLE: Rgba = [255; 4];
const LINE: Rgba = [0, 255, 0, 255];

/// Windows that put the best judgement within five milliseconds of the middle, the next within
/// fifteen, the third twenty to either side and the bad one twenty-five early and thirty late.
const WINDOWS: [[i32; 2]; TIMING_JUDGE_AREAS] = [[-5, 5], [-15, 15], [-20, 20], [-30, 25], [-40, 40]];

/// The reference's defaults: a ruler of 301 pixels over 301 milliseconds, so one pixel to each.
fn body(center: i32, width: i32, decay: bool) -> TimingVisualizerBody {
    TimingVisualizerBody {
        center,
        rate: visualizer_rate(width, center),
        line_width: 1,
        line: LINE,
        centre: MIDDLE,
        judges: [PERFECT, GREAT, GOOD, BAD, POOR],
        decay,
        state: RefCell::new(Drawn {
            ruler: Layer::new(next_serial(), "ruler"),
            ruler_pixmap: Pixmap::default(),
            painted: None,
            stroke: Layer::new(next_serial(), "stroke"),
            stroke_pixmap: solid_stroke(1),
        }),
    }
}

fn plot() -> Rect {
    Rect::new(100.0, 40.0, 301.0, 20.0)
}

/// What a tick leaves of a colour it is painted over: a quarter black mixed in.
fn ticked(color: Rgba) -> Rgba {
    blend_over(unit_color(0.0, 0.0, 0.0, TICK_ALPHA), color)
}

const CLEAR_PIXEL: Rgba = [0; 4];

#[test]
fn the_ruler_is_the_middle_pixel_with_each_window_added_outside_the_one_before() {
    let ruler = paint_ruler(&body(47, 95, true), &WINDOWS);
    assert_eq!(ruler.size(), (95, 1), "two pixels more than twice the milliseconds it reaches");
    let at = |offset: i32| ruler.pixel((47 + offset) as usize, 0);
    assert_eq!(at(0), ticked(MIDDLE), "the middle pixel is its own colour, over the best window, and the tick row passes through it");
    assert_eq!((at(-5), at(5)), (PERFECT, PERFECT), "the best window reaches five to either side, both ends included");
    assert_eq!((at(-6), at(6), at(-15), at(15)), (GREAT, GREAT, GREAT, GREAT));
    assert_eq!((at(-16), at(19), at(-19)), (GOOD, GOOD, GOOD));
    assert_eq!((at(-29), at(25), at(-21), at(21)), (BAD, BAD, BAD, BAD), "the bad window is wider late than early");
    assert_eq!((at(-31), at(26), at(-39), at(39)), (POOR, POOR, POOR, POOR), "and the poor window is what is left out to its edge");
    assert_eq!((at(-41), at(41)), (CLEAR_PIXEL, CLEAR_PIXEL), "beyond the poor window the ruler is bare");
    assert_eq!(at(-30), ticked(BAD), "a tick falls on every tenth pixel from the middle");
}

#[test]
fn a_window_past_the_ruler_is_cut_to_it() {
    let ruler = paint_ruler(&body(47, 95, true), &[[-500, 500]; TIMING_JUDGE_AREAS]);
    assert_eq!((ruler.pixel(1, 0), ruler.pixel(93, 0)), (PERFECT, PERFECT), "the best window fills the ruler to both ends");
    assert_eq!((ruler.pixel(0, 0), ruler.pixel(94, 0)), (PERFECT, PERFECT), "the first pixel and the last");
}

#[test]
fn a_window_inside_the_one_before_adds_nothing() {
    let narrowing = [[-10, 10], [-4, 4], [-8, 8], [-2, 2], [-1, 1]];
    let ruler = paint_ruler(&body(20, 41, true), &narrowing);
    let at = |offset: i32| ruler.pixel((20 + offset) as usize, 0);
    assert_eq!((at(-9), at(9), at(-3), at(3)), (PERFECT, PERFECT, PERFECT, PERFECT), "the best window already covers everything the others name");
    assert_eq!(at(11), CLEAR_PIXEL, "and nothing is painted beyond it");
}

#[test]
fn a_tick_darkens_every_tenth_pixel_from_the_middle_of_the_ruler_mod_ten() {
    let ruler = paint_ruler(&body(25, 51, true), &[[0; 2]; TIMING_JUDGE_AREAS]);
    let painted: Vec<usize> = (0..51).filter(|x| ruler.pixel(*x, 0)[3] != 0).collect();
    assert_eq!(painted, vec![5, 15, 25, 35, 45], "the ticks start at 25 % 10 and the middle pixel is on one");
    assert_eq!(ruler.pixel(5, 0), [0, 0, 0, 62], "a quarter black over nothing, its alpha truncated twice as the reference's pixmap truncates it");
    assert_eq!(ruler.pixel(25, 0), [192, 192, 192, 255], "over the opaque middle it only mixes");
}

#[test]
fn a_cleared_poor_window_leaves_the_ruler_as_it_was() {
    let mut cleared = body(40, 81, true);
    cleared.judges[4] = CLEAR_PIXEL;
    let ruler = paint_ruler(&cleared, &WINDOWS);
    assert_eq!(ruler.pixel(40 - 35, 0), CLEAR_PIXEL, "nothing is painted where the poor window would have been");
}

#[test]
fn an_early_hit_is_to_the_right_of_the_middle_of_the_ruler() {
    let lines = body(150, 301, false);
    let middle = stroke_rect(&lines, plot(), 0, 99).expect("a hit on time is drawn");
    assert_eq!(middle.x, 100.0 + 150.0, "the middle of 301 pixels, a one pixel line");
    let early = stroke_rect(&lines, plot(), 20, 99).expect("a hit within the ruler is drawn");
    assert_eq!(early.x, middle.x + 20.0, "twenty milliseconds early is twenty pixels to the right");
    let late = stroke_rect(&lines, plot(), -20, 99).expect("a hit within the ruler is drawn");
    assert_eq!(late.x, middle.x - 20.0, "and late is to the left");
}

#[test]
fn a_destination_of_negative_width_puts_the_lines_on_the_middle_of_the_same_span() {
    let lines = body(150, 301, false);
    let mirrored = Rect::new(401.0, 40.0, -301.0, 20.0);
    let line = stroke_rect(&lines, mirrored, 20, 99).expect("drawn");
    let upright = stroke_rect(&lines, plot(), 20, 99).expect("drawn");
    assert_eq!(line.x, upright.x, "a span written from its right edge has the same middle");
}

#[test]
fn a_hit_past_the_ruler_is_not_drawn_and_its_edge_is() {
    let lines = body(150, 301, false);
    assert!(stroke_rect(&lines, plot(), 151, 99).is_none());
    assert!(stroke_rect(&lines, plot(), -151, 99).is_none());
    assert!(stroke_rect(&lines, plot(), 150, 99).is_some());
    assert!(stroke_rect(&lines, plot(), -150, 99).is_some());
}

#[test]
fn the_width_of_the_record_sets_the_scale_whatever_the_rectangle_is() {
    let wide = body(150, 602, false);
    let narrow = body(150, 301, false);
    let big = Rect::new(0.0, 0.0, 1000.0, 20.0);
    let early_wide = stroke_rect(&wide, big, 10, 99).expect("drawn");
    let early_narrow = stroke_rect(&narrow, big, 10, 99).expect("drawn");
    let middle = (1000.0 - 1.0) / 2.0;
    assert_eq!(early_narrow.x, middle + 10.0, "ten milliseconds is ten pixels of a 301 pixel record on a rectangle of any size");
    assert_eq!(early_wide.x, middle + 10.0 * 602.0 / 301.0, "and twice that of a 602 pixel record");
    assert_eq!(early_narrow.w, 1.0, "the line is the record's line width, not scaled with the rectangle either");
}

#[test]
fn with_decay_a_line_grows_from_the_middle_of_the_height_as_it_ages_and_without_it_fills_the_height() {
    let decaying = body(150, 301, true);
    let newest = stroke_rect(&decaying, plot(), 0, 99).expect("drawn");
    assert_eq!((newest.y, newest.h), (40.0 + 20.0 * 1.0 / 100.0 / 2.0, 20.0 * 99.0 / 100.0));
    let half = stroke_rect(&decaying, plot(), 0, 50).expect("drawn");
    assert_eq!((half.y, half.h), (40.0 + 5.0, 10.0), "the line of age fifty is half the height, centred");
    assert!(stroke_rect(&decaying, plot(), 0, 0).is_none(), "the oldest place has no height");

    let steady = body(150, 301, false);
    let oldest = stroke_rect(&steady, plot(), 0, 0).expect("drawn");
    assert_eq!((oldest.y, oldest.h), (40.0, 20.0), "without decay every line is the whole height");
}

#[test]
fn a_line_is_fainter_the_older_it_is() {
    let faint = [0, 255, 0, 200];
    assert_eq!(stroke_color(faint, 99), Color { r: 0, g: 255, b: 0, a: 200 }, "the newest is at the record's alpha");
    assert_eq!(stroke_color(faint, 49).a, 100, "the fiftieth place has half of it");
    assert_eq!(stroke_color(faint, 0).a, 2, "and the oldest a hundredth of it, truncated");
    assert_eq!(stroke_color([0, 255, 0, 255], 99), Color { r: 0, g: 255, b: 0, a: 255 });
}

#[test]
fn the_buffer_keeps_the_last_hundred_hits_better_than_poor_oldest_first() {
    let hits: Vec<(i64, u8)> = (0..130).map(|error| (error, if error % 10 == 9 { 4 } else { 0 })).collect();
    let errors = RecentHits::new(&hits).errors();
    assert_eq!(errors.len(), 100);
    assert_eq!(errors.last(), Some(&128), "the newest recorded hit last: 129 took a poor and is not recorded");
    assert!(errors.iter().all(|error| error % 10 != 9));
    assert_eq!(RecentHits::new(&hits).recorded, 117, "a hit is counted when it is recorded");
    assert_eq!(RecentHits::new(&hits).with_recorded(500).recorded, 500);
    assert!(RecentHits::new(&[]).errors().is_empty());
}

#[test]
fn a_record_builds_the_ruler_it_names_and_warns_of_a_colour_that_is_not_one() {
    let record = TimingVisualizer {
        id: "ruler".to_owned(),
        width: 200,
        judge_width_millis: 100,
        line_width: 9,
        pgreat_color: "red".to_owned(),
        transparent: 1,
        ..TimingVisualizer::default()
    };
    let def = SkinDef { timingvisualizer: vec![record], ..SkinDef::default() };
    let mut warnings = Vec::new();
    let Some(Body::TimingVisualizer(built)) = build(&def, "ruler", &mut warnings) else {
        panic!("the document declares a timing visualiser called ruler");
    };
    assert_eq!((built.center, built.line_width, built.decay), (100, 4, true), "the line width is held to four");
    assert_eq!(built.rate, 200.0 / 201.0);
    assert_eq!(built.judges[0], [255, 0, 0, 255], "a colour that is not hex is opaque red");
    assert_eq!(built.judges[4], CLEAR_PIXEL, "a record that asks for it clears its poor window");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(build(&def, "other", &mut warnings).is_none());
}
