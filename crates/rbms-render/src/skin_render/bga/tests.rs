//! Tests of the `bga` object: what it draws from a frame, how a decoded picture becomes a texture,
//! and which picture the playhead says is showing.
//!
//! The object is drawn onto a [`CpuCanvas`] through the dispatch a real frame goes through, from
//! pictures of flat colours, so what is asserted is the pixel a screen would have shown. A canvas is
//! the size of the rectangle the object is placed in, so a document pixel is a screen pixel, and the
//! ground is a grey nothing here draws in, so a pixel the object left alone is plain to see.

use rbms_model::{Mode, TimeLine};
use rbms_skin::dst::{DestinationTrack, Keyframe, SkinColor, SkinRect};
use rbms_skin::loader::StretchKind;
use rbms_skin::property::DefaultState;
use rbms_skin::timer::TimerState;

use super::super::draw::draw_object;
use super::super::object::{Body, SkinObject};
use super::super::{FrameData, SkinFrame, SkinViewport};
use super::{
    BgaBody, BgaEvent, BgaExpand, BgaFrame, BgaPick, BgaPicture, BgaPlayhead, BgaTextures, DEFAULT_MISS_LAYER_DURATION_MS, MISS_LAYER_NONE, SMALL_PICTURE_EDGE,
    key_out_black, on_small_canvas,
};
use crate::ctx::with_render_ctx;
use crate::{Color, CpuCanvas, QuadParams, Rect, Renderer, TextureId};

/// The side of the square canvas and rectangle most tests draw on.
const SIDE: u32 = 300;

/// The side of the larger square the fitting tests draw on, so a picture is both smaller than it
/// and past the 256 pixels that would put it on a canvas of its own.
const WIDE_SIDE: u32 = 600;

/// The size of the picture the fitting tests draw: twice as wide as it is tall, past 256 pixels so no
/// canvas of its own is made for it, and half the side of the larger square so every fit lands on
/// whole pixels.
const WIDE_PICTURE: (u32, u32) = (300, 150);

/// What nothing the object draws can be mistaken for.
const GROUND: Color = Color { r: 200, g: 200, b: 200, a: 255 };

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// `width` x `height` pixels of one colour.
fn flat(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    (0..width * height).flat_map(|_| color).collect()
}

/// A picture whose left half is `left` and whose right half is `right`.
fn halves(width: u32, height: u32, left: [u8; 4], right: [u8; 4]) -> Vec<u8> {
    (0..height).flat_map(|_| (0..width).flat_map(move |x| if x < width / 2 { left } else { right })).collect()
}

/// A decode of `rgba`, numbered `generation`.
fn picture(rgba: &[u8], width: u32, height: u32, generation: u64) -> BgaPicture<'_> {
    BgaPicture { generation, width, height, rgba }
}

/// The destination of a `bga` object: one rectangle held still, in white of the given opacity, with
/// the document's stretch.
fn track(rect: SkinRect, alpha: u8, stretch: i32, angle_deg: f32) -> DestinationTrack {
    DestinationTrack {
        frames: vec![Keyframe { time_ms: 0, rect, clip: None, color: SkinColor::rgba(255, 255, 255, alpha), angle_deg }],
        stretch,
        ..DestinationTrack::default()
    }
}

/// A `bga` object over the whole of a `side` x `side` canvas.
fn placed(side: u32, alpha: u8, stretch: i32) -> SkinObject {
    object(SkinRect::new(0.0, 0.0, side as f32, side as f32), alpha, stretch, 0.0)
}

fn object(rect: SkinRect, alpha: u8, stretch: i32, angle_deg: f32) -> SkinObject {
    SkinObject { track: track(rect, alpha, stretch, angle_deg), stretch: StretchKind::from_id(stretch), body: Body::Bga(BgaBody) }
}

/// Draws `object` over a `side` x `side` ground with `bga` as the frame's background, and answers
/// the canvas and whether anything was drawn.
fn draw(side: u32, object: &SkinObject, bga: BgaFrame) -> (CpuCanvas, bool) {
    let mut canvas = CpuCanvas::new(side, side);
    canvas.clear(GROUND);
    let drawn = draw_on(&mut canvas, side, object, bga);
    (canvas, drawn)
}

/// The same, onto a canvas that already holds the textures `bga` names.
fn draw_on(canvas: &mut CpuCanvas, side: u32, object: &SkinObject, bga: BgaFrame) -> bool {
    let timers = TimerState::new();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &DefaultState, lua: None, mouse: None, data: FrameData { bga, ..FrameData::default() } };
    let viewport = SkinViewport::new((side as f32, side as f32), (side as f32, side as f32));
    with_render_ctx(|ctx| draw_object(ctx, canvas, object, &viewport, &frame))
}

/// A texture of `rgba` registered on the canvas as it is.
fn texture(canvas: &mut CpuCanvas, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
    canvas.register_texture(key, rgba, width, height)
}

fn pixel(canvas: &CpuCanvas, x: u32, y: u32) -> [u8; 4] {
    let color = canvas.pixel_at(x, y);
    [color.r, color.g, color.b, color.a]
}

fn ground() -> [u8; 4] {
    [GROUND.r, GROUND.g, GROUND.b, GROUND.a]
}

#[test]
fn a_frame_that_shows_nothing_leaves_the_object_undrawn() {
    let (canvas, drawn) = draw(SIDE, &placed(SIDE, 255, -1), BgaFrame::default());
    assert!(!drawn, "the default frame drew something");
    assert_eq!(pixel(&canvas, 10, 10), ground());

    let (_, drawn) = draw(SIDE, &placed(SIDE, 255, -1), BgaFrame::of(None));
    assert!(!drawn, "a frame with no picture drew something");
}

#[test]
fn the_layer_lies_over_the_picture_and_its_pure_black_lets_the_picture_through() {
    let mut canvas = CpuCanvas::new(SIDE, SIDE);
    let mut textures = BgaTextures::default();
    let base = flat(SIDE, SIDE, RED);
    let layer = halves(SIDE, SIDE, BLACK, GREEN);
    let pictures = [base, layer];
    let frame = textures.frame(&mut canvas, BgaPick::Playing { base: Some(0), layer: Some(1) }, BgaExpand::Full, |number| {
        let index = usize::try_from(number).ok()?;
        pictures.get(index).map(|rgba| picture(rgba, SIDE, SIDE, 10 + u64::from(number.unsigned_abs())))
    });
    canvas.clear(GROUND);
    assert!(draw_on(&mut canvas, SIDE, &placed(SIDE, 255, -1), frame));

    assert_eq!(pixel(&canvas, 20, 150), RED, "the layer's black pixels did not let the picture show");
    assert_eq!(pixel(&canvas, SIDE - 20, 150), GREEN, "the layer's coloured pixels did not cover the picture");
}

#[test]
fn a_pixel_that_is_almost_black_is_not_keyed_out() {
    let mut rgba = [0, 0, 0, 255, 0, 0, 1, 255, 0, 0, 0, 7, 9, 0, 0, 255];
    key_out_black(&mut rgba);
    assert_eq!(rgba[..4], [0, 0, 0, 0], "pure black kept its alpha");
    assert_eq!(rgba[4..8], [0, 0, 1, 255], "a dark blue was keyed out");
    assert_eq!(rgba[8..12], [0, 0, 0, 0], "a black pixel with its own alpha was kept");
    assert_eq!(rgba[12..16], [9, 0, 0, 255]);
}

#[test]
fn a_chart_with_no_picture_is_black_under_its_layer() {
    let mut canvas = CpuCanvas::new(SIDE, SIDE);
    let mut textures = BgaTextures::default();
    let layer = halves(SIDE, SIDE, BLACK, BLUE);
    let frame = textures.frame(&mut canvas, BgaPick::Playing { base: None, layer: Some(0) }, BgaExpand::Full, |_| Some(picture(&layer, SIDE, SIDE, 1)));
    canvas.clear(GROUND);
    assert!(draw_on(&mut canvas, SIDE, &placed(SIDE, 255, -1), frame));

    assert_eq!(pixel(&canvas, 20, 150), BLACK, "the rectangle was not black where the layer is keyed out");
    assert_eq!(pixel(&canvas, SIDE - 20, 150), BLUE);
}

#[test]
fn before_play_the_whole_rectangle_is_black_in_the_colour_the_destination_names() {
    let (canvas, drawn) = draw(SIDE, &placed(SIDE, 255, -1), BgaFrame::blank(BgaExpand::KeepAspectRatio));
    assert!(drawn);
    assert_eq!(pixel(&canvas, 0, 0), BLACK);
    assert_eq!(pixel(&canvas, SIDE - 1, SIDE - 1), BLACK, "the black did not reach the far corner");

    let (canvas, _) = draw(SIDE, &placed(SIDE, 128, -1), BgaFrame::blank(BgaExpand::Full));
    let shade = pixel(&canvas, 100, 100)[0];
    assert!((95..=105).contains(&shade), "half-opaque black over {} came to {shade}", GROUND.r);
}

#[test]
fn no_picture_at_all_while_playing_is_a_black_rectangle_as_well() {
    let (canvas, drawn) = draw(SIDE, &placed(SIDE, 255, -1), BgaFrame::playing(None, None, BgaExpand::KeepAspectRatio));
    assert!(drawn);
    assert_eq!(pixel(&canvas, 150, 150), BLACK);
}

#[test]
fn the_miss_layer_shows_alone_and_leaves_the_rest_of_the_rectangle_as_it_was() {
    let mut canvas = CpuCanvas::new(WIDE_SIDE, WIDE_SIDE);
    let wide = flat(WIDE_PICTURE.0, WIDE_PICTURE.1, BLUE);
    let image = texture(&mut canvas, "miss", &wide, WIDE_PICTURE.0, WIDE_PICTURE.1);
    canvas.clear(GROUND);
    assert!(draw_on(&mut canvas, WIDE_SIDE, &placed(WIDE_SIDE, 255, -1), BgaFrame::miss(Some(image), BgaExpand::KeepAspectRatio)));

    assert_eq!(pixel(&canvas, 300, 300), BLUE, "the miss layer's picture is not there");
    assert_eq!(pixel(&canvas, 300, 50), ground(), "the miss layer drew black or its picture outside its own shape");

    let (_, drawn) = draw(WIDE_SIDE, &placed(WIDE_SIDE, 255, -1), BgaFrame::miss(None, BgaExpand::Full));
    assert!(!drawn, "a miss step that shows nothing drew something");
}

#[test]
fn the_expand_setting_fits_a_picture_that_is_not_the_rectangles_shape() {
    let wide = flat(WIDE_PICTURE.0, WIDE_PICTURE.1, GREEN);
    let rows = |canvas: &CpuCanvas, x: u32| (0..WIDE_SIDE).filter(|y| pixel(canvas, x, *y) == GREEN).collect::<Vec<_>>();
    let columns = |canvas: &CpuCanvas, y: u32| (0..WIDE_SIDE).filter(|x| pixel(canvas, *x, y) == GREEN).collect::<Vec<_>>();
    let shown = |expand: BgaExpand, stretch: i32| {
        let mut canvas = CpuCanvas::new(WIDE_SIDE, WIDE_SIDE);
        let image = texture(&mut canvas, "wide", &wide, WIDE_PICTURE.0, WIDE_PICTURE.1);
        canvas.clear(GROUND);
        draw_on(&mut canvas, WIDE_SIDE, &placed(WIDE_SIDE, 255, stretch), BgaFrame::playing(Some(image), None, expand));
        canvas
    };

    let full = shown(BgaExpand::Full, -1);
    assert_eq!((rows(&full, 300).len(), columns(&full, 300).len()), (WIDE_SIDE as usize, WIDE_SIDE as usize), "FULL did not fill the rectangle");

    let kept = shown(BgaExpand::KeepAspectRatio, -1);
    let kept_rows = rows(&kept, 300);
    assert_eq!((kept_rows.first(), kept_rows.last()), (Some(&150), Some(&449)), "KEEP_ASPECT_RATIO did not fit the width and centre the height");
    assert_eq!(columns(&kept, 300).len(), WIDE_SIDE as usize);

    let off = shown(BgaExpand::Off, -1);
    let (off_rows, off_columns) = (rows(&off, 300), columns(&off, 300));
    assert_eq!((off_columns.first(), off_columns.last(), off_rows.first(), off_rows.last()), (Some(&150), Some(&449), Some(&225), Some(&374)), "OFF enlarged");
}

#[test]
fn a_destination_that_names_a_stretch_is_fitted_by_it_and_not_by_the_expand_setting() {
    let wide = flat(WIDE_PICTURE.0, WIDE_PICTURE.1, GREEN);
    let mut canvas = CpuCanvas::new(WIDE_SIDE, WIDE_SIDE);
    let image = texture(&mut canvas, "wide", &wide, WIDE_PICTURE.0, WIDE_PICTURE.1);
    canvas.clear(GROUND);
    let no_expanding = 8;
    assert!(draw_on(&mut canvas, WIDE_SIDE, &placed(WIDE_SIDE, 255, no_expanding), BgaFrame::playing(Some(image), None, BgaExpand::Full)));

    assert_eq!(pixel(&canvas, 300, 300), GREEN);
    assert_eq!(pixel(&canvas, 100, 300), ground(), "FULL filled a rectangle whose destination asked not to be enlarged");
}

#[test]
fn the_object_is_never_turned() {
    let image_pixels = halves(SIDE, SIDE, RED, GREEN);
    let drawn = |angle_deg: f32| {
        let mut canvas = CpuCanvas::new(SIDE, SIDE);
        let image = texture(&mut canvas, "halves", &image_pixels, SIDE, SIDE);
        canvas.clear(GROUND);
        let turned = object(SkinRect::new(0.0, 0.0, SIDE as f32, SIDE as f32), 255, -1, angle_deg);
        draw_on(&mut canvas, SIDE, &turned, BgaFrame::playing(Some(image), None, BgaExpand::Full));
        canvas.pixels().to_vec()
    };
    assert_eq!(drawn(0.0), drawn(90.0), "the destination's angle turned the background");
}

#[test]
fn a_document_can_place_the_background_more_than_once_and_each_placement_draws_the_frame() {
    let mut canvas = CpuCanvas::new(WIDE_SIDE, WIDE_SIDE);
    let red = flat(WIDE_SIDE, WIDE_SIDE, RED);
    let image = texture(&mut canvas, "red", &red, WIDE_SIDE, WIDE_SIDE);
    canvas.clear(GROUND);
    let frame = BgaFrame::playing(Some(image), None, BgaExpand::Full);

    let upper = object(SkinRect::new(0.0, 400.0, 200.0, 200.0), 255, -1, 0.0);
    let lower = object(SkinRect::new(300.0, 0.0, 200.0, 200.0), 255, -1, 0.0);
    assert!(draw_on(&mut canvas, WIDE_SIDE, &upper, frame));
    assert!(draw_on(&mut canvas, WIDE_SIDE, &lower, frame));

    assert_eq!(pixel(&canvas, 100, 100), RED, "the first placement is missing");
    assert_eq!(pixel(&canvas, 400, 500), RED, "the second placement is missing");
    assert_eq!(pixel(&canvas, 250, 300), ground(), "something was drawn between the two placements");
}

#[test]
fn a_small_picture_sits_on_a_square_canvas_centred_across_and_flush_with_the_top() {
    let small = flat(100, 50, RED);
    let (canvas, size) = on_small_canvas(&picture(&small, 100, 50, 1)).expect("a picture of 100 x 50 goes on a canvas");
    assert_eq!(size, (SMALL_PICTURE_EDGE, SMALL_PICTURE_EDGE));
    let at = |x: usize, y: usize| {
        let start = (y * SMALL_PICTURE_EDGE as usize + x) * 4;
        [canvas[start], canvas[start + 1], canvas[start + 2], canvas[start + 3]]
    };
    assert_eq!(at(77, 0), [0, 0, 0, 0], "the column before the picture is not empty");
    assert_eq!(at(78, 0), RED, "the picture does not start (256 - 100) / 2 columns in, at the top");
    assert_eq!(at(177, 49), RED, "the picture does not end where its width says");
    assert_eq!(at(178, 0), [0, 0, 0, 0]);
    assert_eq!(at(100, 50), [0, 0, 0, 0], "the rows below the picture are not empty");

    let odd = flat(101, 10, RED);
    let (canvas, _) = on_small_canvas(&picture(&odd, 101, 10, 2)).expect("a picture of 101 x 10 goes on a canvas");
    assert_eq!(canvas[76 * 4 + 3], 0);
    assert_eq!(canvas[77 * 4 + 3], 255, "an odd leftover was not rounded down: the picture starts at (256 - 101) / 2 = 77");
}

#[test]
fn only_a_picture_of_256_or_less_on_its_longer_side_goes_on_a_canvas() {
    let edge = flat(SMALL_PICTURE_EDGE, 1, RED);
    let (canvas, _) = on_small_canvas(&picture(&edge, SMALL_PICTURE_EDGE, 1, 1)).expect("256 is small enough");
    assert_eq!(canvas[3], 255, "a picture as wide as the canvas starts at its left edge");

    let long = flat(SMALL_PICTURE_EDGE + 1, 1, RED);
    assert!(on_small_canvas(&picture(&long, SMALL_PICTURE_EDGE + 1, 1, 2)).is_none(), "257 is drawn as it is");
    assert!(on_small_canvas(&picture(&edge[..8], SMALL_PICTURE_EDGE, 1, 3)).is_none(), "a picture whose pixels are short was put on a canvas");
}

/// A canvas that counts the pictures it is handed, which is how a test tells a picture that was
/// uploaded again from one that was not.
struct Counting {
    canvas: CpuCanvas,
    registered: usize,
}

impl Renderer for Counting {
    fn size(&self) -> (u32, u32) {
        self.canvas.size()
    }

    fn clear(&mut self, color: Color) {
        self.canvas.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.canvas.fill_rect(rect, color);
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        self.registered += 1;
        self.canvas.register_texture(key, rgba, width, height)
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.canvas.release_texture(tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.canvas.texture_size(tex)
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        self.canvas.draw_textured_quad(tex, params);
    }

    fn push_clip(&mut self, rect: Rect) {
        self.canvas.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.canvas.pop_clip();
    }
}

#[test]
fn a_picture_shown_for_many_frames_is_uploaded_once() {
    let mut target = Counting { canvas: CpuCanvas::new(8, 8), registered: 0 };
    let mut textures = BgaTextures::default();
    let (first, second) = (flat(300, 300, RED), flat(300, 300, GREEN));
    let pictures = [picture(&first, 300, 300, 1), picture(&second, 300, 300, 2)];
    let shown = |textures: &mut BgaTextures, target: &mut Counting, number: i32| {
        textures
            .frame(target, BgaPick::Playing { base: Some(number), layer: None }, BgaExpand::Full, |wanted| pictures.get(usize::try_from(wanted).ok()?).copied())
    };

    let first_frame = shown(&mut textures, &mut target, 0);
    let again = shown(&mut textures, &mut target, 0);
    assert_eq!(first_frame, again);
    assert_eq!(target.registered, 1, "the same picture was uploaded again");

    shown(&mut textures, &mut target, 1);
    assert_eq!(target.registered, 2, "a new picture was not uploaded");
    shown(&mut textures, &mut target, 1);
    assert_eq!(target.registered, 2);

    textures.release(&mut target);
    let after = shown(&mut textures, &mut target, 1);
    assert_eq!(target.registered, 3, "a picture the screen let go of was not uploaded again");
    assert!(matches!(after.show, super::BgaShow::Playing { base: Some(_), .. }));
}

#[test]
fn a_number_the_chart_has_no_picture_for_shows_as_no_picture() {
    let mut target = Counting { canvas: CpuCanvas::new(8, 8), registered: 0 };
    let mut textures = BgaTextures::default();
    let frame = textures.frame(&mut target, BgaPick::Playing { base: Some(9), layer: Some(9) }, BgaExpand::Full, |_| None);
    assert_eq!(frame.show, super::BgaShow::Playing { base: None, layer: None });
    assert_eq!(target.registered, 0);
    let miss = textures.frame(&mut target, BgaPick::Miss(Some(9)), BgaExpand::Full, |_| None);
    assert_eq!(miss.show, super::BgaShow::Miss { image: None });
}

/// An event at `time_ms` that shows picture `base` and layer `layer`.
fn event(time_ms: i64, base: i32, layer: i32) -> BgaEvent {
    BgaEvent { time_ms, base, layer, miss: None }
}

/// An event that installs a miss layer of `steps`.
fn miss_event(time_ms: i64, steps: &[i32]) -> BgaEvent {
    BgaEvent { time_ms, base: -1, layer: -1, miss: Some(steps.to_vec()) }
}

fn playing(base: Option<i32>, layer: Option<i32>) -> BgaPick {
    BgaPick::Playing { base, layer }
}

#[test]
fn the_playhead_shows_black_until_play_starts_and_passes_nothing_meanwhile() {
    let mut head = BgaPlayhead::new(vec![event(0, 4, 5), event(100, 6, -1)]);
    head.prepare(-1);
    assert_eq!(head.pick(), BgaPick::Blank);

    head.prepare(0);
    assert_eq!(head.pick(), playing(Some(4), Some(5)), "an event at the very start of play was not passed");
}

#[test]
fn an_event_is_passed_when_the_clock_reaches_it_and_holds_until_the_next() {
    let mut head = BgaPlayhead::new(vec![event(100, 4, -1), event(300, -1, 7), event(500, 8, -2), event(700, -2, -1)]);
    head.prepare(-1);

    head.prepare(99);
    assert_eq!(head.pick(), playing(None, None));
    head.prepare(100);
    assert_eq!(head.pick(), playing(Some(4), None), "the event at 100 was not passed at 100");
    head.prepare(299);
    assert_eq!(head.pick(), playing(Some(4), None));
    head.prepare(300);
    assert_eq!(head.pick(), playing(Some(4), Some(7)), "-1 for the picture did not leave it alone");
    head.prepare(500);
    assert_eq!(head.pick(), playing(Some(8), None), "-2 for the layer did not take it away");
    head.prepare(5_000);
    assert_eq!(head.pick(), playing(None, None), "-2 for the picture did not take it away");
}

#[test]
fn a_clock_that_jumps_over_several_events_passes_them_in_order() {
    let mut head = BgaPlayhead::new(vec![event(100, 1, 1), event(200, 2, -1), event(300, 3, -1)]);
    head.prepare(-1);
    head.prepare(250);
    assert_eq!(head.pick(), playing(Some(2), Some(1)));
}

#[test]
fn an_event_is_passed_once_however_often_the_clock_is_read() {
    let mut head = BgaPlayhead::new(vec![event(100, 1, -1), event(200, 2, -1)]);
    head.prepare(-1);
    head.prepare(150);
    head.prepare(150);
    head.prepare(150);
    assert_eq!(head.pick(), playing(Some(1), None));
    head.prepare(250);
    head.prepare(250);
    assert_eq!(head.pick(), playing(Some(2), None));
}

#[test]
fn resetting_the_playhead_forgets_what_it_showed_and_starts_the_chart_over() {
    let mut head = BgaPlayhead::new(vec![event(100, 1, 2)]);
    head.prepare(-1);
    head.prepare(500);
    assert_eq!(head.pick(), playing(Some(1), Some(2)));

    head.reset();
    assert_eq!(head.pick(), playing(None, None), "a reset playhead still shows the old picture");
    head.prepare(-1);
    head.prepare(500);
    assert_eq!(head.pick(), playing(Some(1), Some(2)), "the chart did not play again from its start");
}

#[test]
fn an_event_at_the_instant_of_a_reset_waits_for_the_clock_to_go_negative_first() {
    let mut head = BgaPlayhead::new(vec![event(0, 1, -1)]);
    head.reset();
    head.prepare(0);
    assert_eq!(head.pick(), playing(None, None), "an event at 0 was passed by a clock that never was before play");
}

#[test]
fn the_miss_layer_shows_for_its_duration_from_the_moment_of_the_miss() {
    let mut head = BgaPlayhead::new(vec![event(0, 1, -1), miss_event(0, &[9])]);
    head.prepare(-1);
    head.prepare(1_000);
    head.start_miss(1_000, DEFAULT_MISS_LAYER_DURATION_MS);

    assert_eq!(head.pick(), BgaPick::Miss(Some(9)), "the miss layer is not showing at the moment of the miss");
    head.prepare(1_499);
    assert_eq!(head.pick(), BgaPick::Miss(Some(9)), "the miss layer ended early");
    head.prepare(1_500);
    assert_eq!(head.pick(), playing(Some(1), None), "the miss layer outstayed its duration");
    head.prepare(999);
    assert_eq!(head.pick(), playing(Some(1), None), "the miss layer showed before the miss");
}

#[test]
fn the_miss_layer_steps_through_its_pictures_over_its_duration() {
    let mut head = BgaPlayhead::new(vec![miss_event(0, &[10, 11, 12, MISS_LAYER_NONE])]);
    head.prepare(-1);
    head.prepare(2_000);
    head.start_miss(2_000, 900);
    let at = |head: &mut BgaPlayhead, ms: i64| {
        head.prepare(ms);
        head.pick()
    };

    assert_eq!(at(&mut head, 2_000), BgaPick::Miss(Some(10)));
    assert_eq!(at(&mut head, 2_299), BgaPick::Miss(Some(10)), "3 * 299 / 900 is 0");
    assert_eq!(at(&mut head, 2_300), BgaPick::Miss(Some(11)), "3 * 300 / 900 is 1");
    assert_eq!(at(&mut head, 2_600), BgaPick::Miss(Some(12)));
    assert_eq!(at(&mut head, 2_899), BgaPick::Miss(Some(12)), "the last step is never reached, as the reference's is not");
    assert_eq!(at(&mut head, 2_900), playing(None, None));
}

#[test]
fn a_step_that_shows_nothing_shows_neither_black_nor_the_picture() {
    let mut head = BgaPlayhead::new(vec![event(0, 1, -1), miss_event(0, &[MISS_LAYER_NONE, MISS_LAYER_NONE])]);
    head.prepare(-1);
    head.prepare(100);
    head.start_miss(100, 500);
    assert_eq!(head.pick(), BgaPick::Miss(None));
}

#[test]
fn a_miss_at_the_start_of_the_clock_or_before_the_layer_is_installed_shows_nothing() {
    let mut head = BgaPlayhead::new(vec![event(0, 1, -1), miss_event(500, &[9])]);
    head.prepare(-1);
    head.prepare(100);
    head.start_miss(100, 500);
    assert_eq!(head.pick(), playing(Some(1), None), "a miss before the chart installed a miss layer showed one");

    head.prepare(600);
    head.start_miss(0, 500);
    assert_eq!(head.pick(), playing(Some(1), None), "a miss at time zero showed the layer");
}

#[test]
fn a_miss_layer_with_a_duration_of_nothing_never_shows() {
    let mut head = BgaPlayhead::new(vec![miss_event(0, &[9])]);
    head.prepare(-1);
    head.prepare(100);
    head.start_miss(100, 0);
    assert_eq!(head.pick(), playing(None, None));
}

#[test]
fn a_new_miss_restarts_the_layer() {
    let mut head = BgaPlayhead::new(vec![miss_event(0, &[9])]);
    head.prepare(-1);
    head.prepare(100);
    head.start_miss(100, 500);
    head.prepare(550);
    assert_eq!(head.pick(), BgaPick::Miss(Some(9)));
    head.start_miss(550, 500);
    head.prepare(1_000);
    assert_eq!(head.pick(), BgaPick::Miss(Some(9)), "the second miss did not restart the layer");
    head.prepare(1_050);
    assert_eq!(head.pick(), playing(None, None));
}

#[test]
fn a_playhead_over_a_model_reads_each_timelines_picture_and_layer_in_milliseconds() {
    let lanes = Mode::BEAT_7K.key;
    let mut plain = TimeLine::empty(lanes, 500_000, 0.5, 120.0);
    let mut shown = TimeLine::empty(lanes, 1_999_999, 1.0, 120.0);
    shown.bga = 3;
    shown.layer = 4;
    plain.section_line = true;
    let mut head = BgaPlayhead::of_chart(&[plain, shown]);

    head.prepare(-1);
    head.prepare(1_998);
    assert_eq!(head.pick(), playing(None, None), "a timeline with no picture installed one");
    head.prepare(1_999);
    assert_eq!(head.pick(), playing(Some(3), Some(4)), "1_999_999 microseconds is 1_999 whole milliseconds");
}
