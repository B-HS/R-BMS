//! The polygon the NOTES RADAR picture holds, checked on its pixels and without a renderer.
//!
//! The picture is 256 pixels square with its middle on the corner four pixels meet at, and the
//! largest value puts a corner 124 pixels from it, so the first corner of a full radar is at
//! `(128, 4)` and the one opposite it at `(128, 252)`, and the corners to either side of the
//! vertical edges are at `x = 128 +- 124 * sin(60 degrees) = 128 +- 107.39`.

use super::*;

const CLEAR: [u8; 4] = [0; 4];
const OPAQUE: [u8; 4] = [255; 4];
const MARK: [u8; 4] = [1, 2, 3, 4];

const FULL: RadarValues = [200.0; RADAR_AXIS_COUNT];

/// The middle of the picture as a pixel index: the first pixel past it.
const MIDDLE: usize = IMAGE_SIZE / 2;

/// The picture `values` are painted as.
fn painted_for(values: RadarValues) -> Pixmap {
    let mut image = RadarImage::new();
    image.refresh(values);
    image.pixmap
}

fn alpha(pixmap: &Pixmap, x: usize, y: usize) -> u8 {
    pixmap.pixel(x, y)[3]
}

/// How much of the picture is covered, in pixels.
fn area(pixmap: &Pixmap) -> f64 {
    (0..IMAGE_SIZE).flat_map(|y| (0..IMAGE_SIZE).map(move |x| (x, y))).map(|(x, y)| f64::from(alpha(pixmap, x, y)) / FULL_ALPHA).sum()
}

#[test]
fn the_picture_is_a_square_of_the_documented_size() {
    assert_eq!(painted_for(FULL).size(), (256, 256));
}

#[test]
fn a_full_radar_is_an_upright_hexagon_of_the_outer_radius() {
    let pixmap = painted_for(FULL);
    assert_eq!(pixmap.pixel(MIDDLE, MIDDLE), OPAQUE, "the middle is inside it");
    assert_eq!(pixmap.pixel(MIDDLE - 1, MIDDLE - 1), OPAQUE);

    assert_eq!(alpha(&pixmap, MIDDLE, 3), 0, "nothing is above the first corner, which is at row 4");
    assert_eq!(alpha(&pixmap, MIDDLE, 10), 255, "and the polygon is wide enough by then to cover the middle column");
    assert_eq!(alpha(&pixmap, MIDDLE, 253), 0, "nothing is below the opposite corner, at row 252");
    assert_eq!(alpha(&pixmap, MIDDLE, 245), 255);

    assert_eq!(alpha(&pixmap, 234, MIDDLE), 255, "the right edge is vertical at x = 235.39");
    assert!((98..=100).contains(&alpha(&pixmap, 235, MIDDLE)), "so the pixel it cuts is covered by 39 per cent of its width");
    assert_eq!(alpha(&pixmap, 236, MIDDLE), 0);
    assert_eq!(alpha(&pixmap, 21, MIDDLE), 255, "and the left edge is vertical at x = 20.61");
    assert!((98..=100).contains(&alpha(&pixmap, 20, MIDDLE)), "which covers 39 per cent of the pixel it cuts as well");
    assert_eq!(alpha(&pixmap, 19, MIDDLE), 0);

    assert_eq!(
        (alpha(&pixmap, 4, 4), alpha(&pixmap, 250, 4), alpha(&pixmap, 4, 250), alpha(&pixmap, 250, 250)),
        (0, 0, 0, 0),
        "the corners of the square are clear"
    );
}

#[test]
fn a_full_radar_is_the_same_mirrored_either_way() {
    let pixmap = painted_for(FULL);
    for y in 0..IMAGE_SIZE {
        for x in 0..IMAGE_SIZE {
            assert_eq!(pixmap.pixel(x, y), pixmap.pixel(IMAGE_SIZE - 1 - x, y), "({x}, {y}) differs from its mirror left to right");
            assert_eq!(pixmap.pixel(x, y), pixmap.pixel(x, IMAGE_SIZE - 1 - y), "({x}, {y}) differs from its mirror top to bottom");
        }
    }
}

#[test]
fn a_full_radar_covers_the_area_of_the_hexagon() {
    let hexagon = 1.5 * 3.0_f64.sqrt() * OUTER_RADIUS * OUTER_RADIUS;
    let covered = area(&painted_for(FULL));
    assert!((covered - hexagon).abs() < hexagon * 0.001, "{covered} pixels covered, where a hexagon of that radius holds {hexagon}");
}

#[test]
fn a_slanted_edge_is_smoothed_by_partial_alpha() {
    let pixmap = painted_for(FULL);
    let partial = |columns: std::ops::Range<usize>| columns.filter(|x| (1..255).contains(&alpha(&pixmap, *x, 30))).count();
    assert!((1..=3).contains(&partial(0..MIDDLE)), "the left edge slopes across less than two pixels of the row, so it cuts a few");
    assert!((1..=3).contains(&partial(MIDDLE..IMAGE_SIZE)), "and so does the right one");
    assert!((0..IMAGE_SIZE).all(|x| pixmap.pixel(x, 30)[..3] == OPAQUE[..3] || alpha(&pixmap, x, 30) == 0), "whatever its alpha, the polygon is white");
}

#[test]
fn a_value_moves_its_corner_in_proportion() {
    let half = painted_for([100.0; RADAR_AXIS_COUNT]);
    assert_eq!(alpha(&half, MIDDLE, 64), 0, "the first corner is 62 pixels up, at row 66");
    assert_eq!(alpha(&half, MIDDLE, 75), 255);
    assert_eq!(alpha(&half, 180, MIDDLE), 255, "the vertical edges are at 128 +- 53.69");
    assert_eq!(alpha(&half, 183, MIDDLE), 0);
    assert_eq!(alpha(&half, 75, MIDDLE), 255, "and the left one is at 74.31");
    assert_eq!(alpha(&half, 73, MIDDLE), 0);
}

#[test]
fn the_axes_run_clockwise_from_the_top() {
    let pixmap = painted_for([200.0, 200.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(alpha(&pixmap, 150, 70), 255, "the triangle of the middle and the first two corners lies up and to the right of the middle");
    assert_eq!(alpha(&pixmap, 170, 100), 255);
    assert_eq!(alpha(&pixmap, 190, 100), 0, "and its third edge slopes in from the second corner to the middle");
    assert_eq!(alpha(&pixmap, 100, 70), 0, "nothing is left of the middle column");
    assert_eq!(alpha(&pixmap, 150, 190), 0, "or below the middle");
}

#[test]
fn a_dip_between_two_spikes_is_not_filled_in() {
    let pixmap = painted_for([200.0, 20.0, 200.0, 20.0, 200.0, 20.0]);
    assert_eq!(alpha(&pixmap, MIDDLE, 48), 255, "a spike is there along an axis that has a large value");
    assert_eq!(
        alpha(&pixmap, 171, 103),
        0,
        "but between two of them, past the corner that a small value draws in, it is not, as the hull of the spikes would have it"
    );
    assert!(area(&pixmap) < area(&painted_for(FULL)) / 2.0);
}

#[test]
fn values_with_no_area_paint_nothing() {
    let nothing = Pixmap::new(IMAGE_SIZE, IMAGE_SIZE);
    assert_eq!(painted_for([0.0; RADAR_AXIS_COUNT]), nothing, "no value draws no polygon");
    assert_eq!(painted_for([200.0, 0.0, 0.0, 0.0, 0.0, 0.0]), nothing, "one corner out and the rest in the middle is a line");
    assert_eq!(painted_for([200.0, 0.0, 0.0, 200.0, 0.0, 0.0]), nothing, "and so are two opposite ones");
}

#[test]
fn a_value_that_is_not_a_number_is_nothing_and_one_past_the_ends_is_held() {
    assert_eq!(painted_for([f32::NAN; RADAR_AXIS_COUNT]), painted_for([0.0; RADAR_AXIS_COUNT]));
    let junk = [f32::NAN, -3.0, 250.0, f32::INFINITY, f32::NEG_INFINITY, 100.0];
    assert_eq!(painted_for(junk), painted_for([0.0, 0.0, 200.0, 200.0, 0.0, 100.0]));
    assert_eq!(painted_for([250.0; RADAR_AXIS_COUNT]), painted_for(FULL), "a value past the largest reaches no further than the largest");
}

#[test]
fn the_picture_is_painted_again_only_when_the_values_change() {
    let mut image = RadarImage::new();
    image.refresh([200.0, 100.0, 50.0, 0.0, 0.0, 0.0]);
    assert_eq!(image.pixmap.pixel(150, 100), OPAQUE, "a first look paints it");

    image.pixmap.fill_rect(0, 0, 1, 1, MARK);
    image.refresh([200.0, 100.0, 50.0, 0.0, 0.0, 0.0]);
    assert_eq!(image.pixmap.pixel(0, 0), MARK, "the same values leave the pixels alone");

    image.refresh([200.0, 100.0, 50.0, f32::NAN, -1.0, 0.0]);
    assert_eq!(image.pixmap.pixel(0, 0), MARK, "so do values that are held to the same ones");

    image.refresh([200.0, 100.0, 51.0, 0.0, 0.0, 0.0]);
    assert_eq!(image.pixmap.pixel(0, 0), CLEAR, "a changed value paints the picture afresh");
}
