//! The NOTES RADAR reference image drawn through the dispatch a real frame goes through: painted from
//! the six values the frame carries, tinted by the destination, and left out when the frame has none.
//!
//! The canvas is as large as the picture and the document is authored at that size, so a pixel of the
//! picture is a pixel of the canvas. A full radar's polygon lies between rows 4 and 252.

use rbms_chart::RADAR_AXIS_COUNT;

use super::*;

/// The destination id a document places the polygon by.
const RADAR_ID: &str = "-21020";

/// How many pixels each edge of the picture and of the canvas is.
const SIDE: u32 = 256;

/// A pixel inside the triangle of the middle and the first two corners, and one at the same height to
/// the left of the middle, which a full radar covers and that triangle does not.
const IN_TRIANGLE: (u32, u32) = (150, 70);
const OUTSIDE_TRIANGLE: (u32, u32) = (100, 70);

const FULL: [f32; RADAR_AXIS_COUNT] = [200.0; RADAR_AXIS_COUNT];
const FIRST_TWO: [f32; RADAR_AXIS_COUNT] = [200.0, 200.0, 0.0, 0.0, 0.0, 0.0];

fn canvas() -> CpuCanvas {
    let mut canvas = CpuCanvas::new(SIDE, SIDE);
    wipe(&mut canvas);
    canvas
}

/// The radar object over the whole canvas, tinted yellow and opaque.
fn radar() -> SkinObject {
    let rect = SkinRect::new(0.0, 0.0, SIDE as f32, SIDE as f32);
    SkinObject { track: held(rect, SkinColor::rgba(u8::MAX, u8::MAX, 0, u8::MAX)), ..reference_at(rect, RADAR_ID) }
}

/// Draws `object` on `canvas` against a frame that carries the radar `values`, answering whether
/// anything reached the screen.
fn paint_with(canvas: &mut CpuCanvas, object: &SkinObject, values: Option<[f32; RADAR_AXIS_COUNT]>) -> bool {
    let host = MapHost::new();
    let timers = TimerState::new();
    let data = FrameData { images: ReferenceImages { notes_radar: values, ..ReferenceImages::default() }, ..FrameData::default() };
    let frame = SkinFrame { now_us: host.now_us, timers: &timers, state: &host, lua: None, mouse: None, data };
    let viewport = SkinViewport::new((SIDE as f32, SIDE as f32), (SIDE as f32, SIDE as f32));
    with_render_ctx(|ctx| draw_object(ctx, canvas, object, &viewport, &frame))
}

#[test]
fn the_polygon_is_painted_from_the_frames_values_and_tinted_by_the_destination() {
    let mut canvas = canvas();
    assert!(paint_with(&mut canvas, &radar(), Some(FULL)));
    assert_eq!(rgb(&canvas, 128, 128), YELLOW, "the white polygon takes the destination's colour");
    assert_eq!(rgb(&canvas, 128, 3), BACKDROP, "and nothing is drawn above the first corner");
    assert_eq!(rgb(&canvas, 4, 4), BACKDROP, "or in the corners of the picture");

    let (red, green, blue) = rgb(&canvas, 235, 128);
    assert!(
        red > BACKDROP.0 && red < u8::MAX && green > BACKDROP.1 && green < u8::MAX && blue < BACKDROP.2,
        "a pixel an edge cuts is blended with what is behind"
    );
}

#[test]
fn the_first_axis_points_up_and_the_rest_follow_clockwise() {
    let mut canvas = canvas();
    assert!(paint_with(&mut canvas, &radar(), Some(FIRST_TWO)));
    assert_eq!(rgb(&canvas, IN_TRIANGLE.0, IN_TRIANGLE.1), YELLOW, "the triangle lies up and to the right of the middle");
    assert_eq!(rgb(&canvas, OUTSIDE_TRIANGLE.0, OUTSIDE_TRIANGLE.1), BACKDROP, "and nothing is left of the middle column");
    assert_eq!(rgb(&canvas, 150, 190), BACKDROP, "or below the middle");
}

#[test]
fn a_frame_that_carries_no_values_leaves_the_object_undrawn() {
    let mut canvas = canvas();
    let object = radar();
    assert!(!paint_with(&mut canvas, &object, None));
    assert_eq!(rgb(&canvas, 128, 128), BACKDROP);
    assert_eq!(canvas.live_texture_count(), 0, "and no texture is made for it");
    assert!(prepared(&object, &MapHost::new()), "though it is an object all the same, and is prepared like one");
}

#[test]
fn values_that_are_nothing_draw_nothing_that_can_be_seen() {
    let mut canvas = canvas();
    for values in [[0.0; RADAR_AXIS_COUNT], [f32::NAN; RADAR_AXIS_COUNT]] {
        paint_with(&mut canvas, &radar(), Some(values));
        assert_eq!(rgb(&canvas, 128, 128), BACKDROP, "{values:?} leave no polygon");
        assert_eq!(rgb(&canvas, 140, 128), BACKDROP);
    }
}

#[test]
fn new_values_replace_the_polygon_the_next_frame() {
    let mut canvas = canvas();
    let object = radar();
    assert!(paint_with(&mut canvas, &object, Some(FULL)));
    assert_eq!(rgb(&canvas, 128, 200), YELLOW);
    assert_eq!(rgb(&canvas, OUTSIDE_TRIANGLE.0, OUTSIDE_TRIANGLE.1), YELLOW);

    wipe(&mut canvas);
    assert!(paint_with(&mut canvas, &object, Some(FIRST_TWO)));
    assert_eq!(rgb(&canvas, 128, 200), BACKDROP, "the part the new values give up is gone");
    assert_eq!(rgb(&canvas, OUTSIDE_TRIANGLE.0, OUTSIDE_TRIANGLE.1), BACKDROP);
    assert_eq!(rgb(&canvas, IN_TRIANGLE.0, IN_TRIANGLE.1), YELLOW);
    assert_eq!(canvas.live_texture_count(), 1, "and it is the same texture painted again");
}

#[test]
fn the_texture_is_handed_back_with_the_object() {
    let mut canvas = canvas();
    let object = radar();
    assert!(paint_with(&mut canvas, &object, Some(FULL)));
    assert_eq!(canvas.live_texture_count(), 1);
    object.release(&mut canvas);
    assert_eq!(canvas.live_texture_count(), 0);

    assert!(paint_with(&mut canvas, &object, Some(FULL)), "an object that was released draws again");
    assert_eq!(rgb(&canvas, 128, 128), YELLOW);
}

#[test]
fn only_the_radars_own_id_is_the_polygon() {
    assert_eq!(ReferenceImage::of_destination(RADAR_ID), Some(ReferenceImage::NotesRadar));
    assert_eq!(ReferenceImage::of_destination("21020"), None, "the id is the negated image id, not the image id");

    let mut canvas = canvas();
    for id in ["-21000", "-21019", "-21021", "-21199", "-21200"] {
        let rect = SkinRect::new(0.0, 0.0, SIDE as f32, SIDE as f32);
        let object = reference_at(rect, id);
        assert!(prepared(&object, &MapHost::new()), "{id} is an object, as every negative id is");
        assert!(!paint_with(&mut canvas, &object, Some(FULL)), "{id} names no picture, so it is never drawn");
    }
    assert_eq!(rgb(&canvas, 128, 128), BACKDROP);
}
