//! The draw primitives the skin renderer is built on: texture registration, sampling, tinting,
//! blending, rotation and clipping, checked on the CPU reference backend.
//!
//! Three of these are golden images rather than assertions. They are small, code-generated scenes
//! (no binary assets beyond the goldens themselves), so a regression can be looked at instead of
//! only counted. Re-generate them with `RBMS_GOLDEN_UPDATE=1 cargo test -p rbms-render --test
//! primitives` after an intentional change, and read the diff picture the failure names otherwise.

use rbms_render::{
    BlendMode, Color, CpuCanvas, GlyphAtlasBinding, GoldenImage, GoldenOptions, PngCodec, QuadParams, Rect, Renderer, TextureFilter, TextureId, UvRect,
    apply_blend, apply_tint, assert_golden_png, skin_center_offset,
};

/// PNG read and write for the golden harness, over the `png` dev-dependency.
struct Png;

impl PngCodec for Png {
    fn encode(&self, image: &GoldenImage) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&image.rgba).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn decode(&self, bytes: &[u8]) -> Result<GoldenImage, String> {
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().map_err(|e| e.to_string())?;
        let mut buffer = vec![0; reader.output_buffer_size().ok_or("golden is too large to decode")?];
        let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
        if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
            return Err(format!("golden is {:?}/{:?}, expected 8-bit RGBA", info.color_type, info.bit_depth));
        }
        buffer.truncate(info.buffer_size());
        Ok(GoldenImage::new(info.width, info.height, buffer))
    }
}

fn check_golden(canvas: &CpuCanvas, name: &str) {
    if let Err(message) = assert_golden_png(canvas, env!("CARGO_MANIFEST_DIR"), name, GoldenOptions::default(), &Png) {
        panic!("{message}");
    }
}

/// A checkerboard whose light squares are opaque white and dark squares half-transparent grey, so
/// a golden shows both what a blend does to colour and what it does to coverage.
fn checker(size: u32, cell: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let light = ((x / cell) + (y / cell)).is_multiple_of(2);
            rgba.extend_from_slice(if light { &[255, 255, 255, 255] } else { &[60, 60, 60, 128] });
        }
    }
    rgba
}

/// A texture whose four quadrants are four flat colours, so a rotation is readable from which
/// corner each colour ended up in.
fn quadrants(size: u32) -> Vec<u8> {
    let colours = [[220u8, 40, 40, 255], [40, 220, 40, 255], [40, 40, 220, 255], [230, 230, 40, 255]];
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let index = usize::from(x >= size / 2) + 2 * usize::from(y >= size / 2);
            rgba.extend_from_slice(&colours[index]);
        }
    }
    rgba
}

/// A two-by-two texture, the smallest thing that tells nearest from bilinear sampling.
fn corners() -> Vec<u8> {
    vec![0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255]
}

fn solid(size: u32, colour: [u8; 4]) -> Vec<u8> {
    colour.repeat((size * size) as usize)
}

fn quad(dst: Rect, blend: BlendMode) -> QuadParams {
    let mut params = QuadParams::new(dst);
    params.blend = blend;
    params
}

/// The four modes are exactly what the reference's blend switch configures. Everything it does not
/// name — 0, 1 and the undefined integers — leaves the default alpha blend alone, and 3 restores
/// `FUNC_ADD` before it draws, so it is additive rather than subtractive.
#[test]
fn skin_blend_integers_map_onto_the_modes_the_reference_configures() {
    let expected = [
        (0, BlendMode::Alpha),
        (1, BlendMode::Alpha),
        (2, BlendMode::Add),
        (3, BlendMode::Add),
        (4, BlendMode::Multiply),
        (5, BlendMode::Alpha),
        (6, BlendMode::Alpha),
        (7, BlendMode::Alpha),
        (8, BlendMode::Alpha),
        (9, BlendMode::InvertDst),
        (10, BlendMode::Alpha),
        (-1, BlendMode::Alpha),
        (999, BlendMode::Alpha),
    ];
    for (blend, mode) in expected {
        assert_eq!(BlendMode::from_skin_blend(blend), mode, "skin blend {blend}");
    }
}

#[test]
fn each_blend_mode_combines_a_source_and_a_target_by_its_own_rule() {
    let src = Color { r: 200, g: 100, b: 50, a: 128 };
    let dst = Color { r: 60, g: 60, b: 60, a: 255 };

    let over = apply_blend(src, dst, BlendMode::Alpha);
    let expect_over = |s: u8, d: u8| ((s as u32 * 128 + d as u32 * 127) / 255) as u8;
    assert_eq!((over.r, over.g, over.b), (expect_over(200, 60), expect_over(100, 60), expect_over(50, 60)));
    assert_eq!(over.a, 255, "source-over leaves an opaque target opaque");

    let add = apply_blend(src, dst, BlendMode::Add);
    assert_eq!(add.r, ((200 * 128 + 60 * 255) / 255) as u8);
    assert_eq!(apply_blend(Color { r: 255, g: 0, b: 0, a: 255 }, Color { r: 200, g: 0, b: 0, a: 255 }, BlendMode::Add).r, 255, "adding saturates");

    let multiply = apply_blend(src, dst, BlendMode::Multiply);
    assert_eq!((multiply.r, multiply.g), ((200 * 60 / 255) as u8, (100 * 60 / 255) as u8), "the source alpha plays no part");

    let invert = apply_blend(src, dst, BlendMode::InvertDst);
    assert_eq!(invert.r, (200 * (255 - 60) / 255) as u8);
    assert_eq!(invert.b, (50 * (255 - 60) / 255) as u8);
}

/// Every mode but source-over deliberately leaves the target's alpha alone: an additive or
/// multiplicative pass over an opaque screen must not make it transparent.
#[test]
fn only_source_over_touches_the_targets_alpha() {
    let src = Color { r: 10, g: 20, b: 30, a: 40 };
    let dst = Color { r: 0, g: 0, b: 0, a: 200 };
    for mode in [BlendMode::Add, BlendMode::Multiply, BlendMode::InvertDst] {
        assert_eq!(apply_blend(src, dst, mode).a, 200, "{mode:?} moved the target alpha");
    }
    assert_eq!(apply_blend(Color { r: 0, g: 0, b: 0, a: 255 }, dst, BlendMode::Alpha).a, 255);
}

#[test]
fn a_tint_multiplies_every_channel_and_white_is_the_identity() {
    let src = Color { r: 200, g: 100, b: 50, a: 255 };
    assert_eq!(apply_tint(src, Color { r: 255, g: 255, b: 255, a: 255 }), src);
    let tinted = apply_tint(src, Color { r: 128, g: 255, b: 0, a: 64 });
    assert_eq!(tinted, Color { r: (200 * 128 / 255) as u8, g: 100, b: 0, a: (255 * 64 / 255) as u8 });
}

/// The whole point of a tinted quad, checked end to end: a white texture takes the tint's colour.
#[test]
fn a_tint_reaches_the_drawn_pixels() {
    let mut canvas = CpuCanvas::new(4, 4);
    canvas.clear(Color::BLACK);
    let tex = canvas.register_texture("white", &solid(2, [255, 255, 255, 255]), 2, 2);
    let mut params = QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0));
    params.tint = Color { r: 255, g: 0, b: 0, a: 255 };
    canvas.draw_textured_quad(tex, params);
    assert_eq!(canvas.pixel_at(1, 1), Color { r: 255, g: 0, b: 0, a: 255 });
}

/// The anchor table with the y axis flipped: the reference measures its centres from the bottom
/// because its space is y-up, and this one is y-down.
#[test]
fn the_skin_centre_anchors_map_onto_y_down_pixels() {
    let (w, h) = (10.0, 20.0);
    let expected = [
        (0, (5.0, 10.0)),
        (1, (0.0, 20.0)),
        (2, (5.0, 20.0)),
        (3, (10.0, 20.0)),
        (4, (0.0, 10.0)),
        (5, (5.0, 10.0)),
        (6, (10.0, 10.0)),
        (7, (0.0, 0.0)),
        (8, (5.0, 0.0)),
        (9, (10.0, 0.0)),
    ];
    for (center, offset) in expected {
        assert_eq!(skin_center_offset(center, w, h), offset, "centre {center}");
    }
}

/// The reference indexes its anchor tables straight from the skin file, so a value outside 0..9
/// throws there. Being more forgiving keeps one mistyped destination from killing a whole skin.
#[test]
fn a_centre_anchor_outside_the_table_falls_back_to_the_middle() {
    for center in [-1, 10, 4242] {
        assert_eq!(skin_center_offset(center, 10.0, 20.0), (5.0, 10.0), "centre {center}");
    }
}

/// The sign that is easy to get backwards. A skin's `angle` turns counter-clockwise in a y-up
/// space, so the loader negates it and a positive `angle_deg` here turns clockwise on screen: the
/// texture's top left quadrant ends up top right.
#[test]
fn a_positive_angle_turns_the_quad_clockwise_on_screen() {
    let mut canvas = CpuCanvas::new(32, 32);
    canvas.clear(Color::BLACK);
    let tex = canvas.register_texture("quadrants", &quadrants(16), 16, 16);
    let dst = Rect::new(8.0, 8.0, 16.0, 16.0);
    let mut params = QuadParams::new(dst);
    params.angle_deg = 90.0;
    params.center = skin_center_offset(0, dst.w, dst.h);
    canvas.draw_textured_quad(tex, params);

    let red = Color { r: 220, g: 40, b: 40, a: 255 };
    let green = Color { r: 40, g: 220, b: 40, a: 255 };
    let blue = Color { r: 40, g: 40, b: 220, a: 255 };
    let yellow = Color { r: 230, g: 230, b: 40, a: 255 };
    assert_eq!(canvas.pixel_at(20, 12), red, "the source top left quadrant turned to the top right");
    assert_eq!(canvas.pixel_at(20, 20), green, "the source top right turned to the bottom right");
    assert_eq!(canvas.pixel_at(12, 12), blue, "the source bottom left turned to the top left");
    assert_eq!(canvas.pixel_at(12, 20), yellow, "the source bottom right turned to the bottom left");
}

#[test]
fn a_full_turn_lands_back_where_it_started() {
    let draw = |angle: f32| {
        let mut canvas = CpuCanvas::new(32, 32);
        canvas.clear(Color::BLACK);
        let tex = canvas.register_texture("quadrants", &quadrants(16), 16, 16);
        let mut params = QuadParams::new(Rect::new(8.0, 8.0, 16.0, 16.0));
        params.angle_deg = angle;
        params.center = (8.0, 8.0);
        canvas.draw_textured_quad(tex, params);
        canvas.pixels().to_vec()
    };
    assert_eq!(draw(0.0), draw(360.0));
    assert_ne!(draw(0.0), draw(90.0), "and a quarter turn does not");
}

/// Rotating about a corner rather than the middle swings the quad rather than spinning it in
/// place, which is what a skin's `center` picks between.
#[test]
fn the_rotation_centre_decides_where_a_turned_quad_ends_up() {
    let draw = |center: (f32, f32)| {
        let mut canvas = CpuCanvas::new(64, 64);
        canvas.clear(Color::BLACK);
        let tex = canvas.register_texture("white", &solid(4, [255, 255, 255, 255]), 4, 4);
        let dst = Rect::new(24.0, 24.0, 16.0, 8.0);
        let mut params = QuadParams::new(dst);
        params.angle_deg = 90.0;
        params.center = center;
        canvas.draw_textured_quad(tex, params);
        (0..64).flat_map(|y| (0..64).map(move |x| (x, y))).filter(|(x, y)| canvas.pixel_at(*x, *y).r > 128).count()
    };
    let middle = draw(skin_center_offset(0, 16.0, 8.0));
    let corner = draw(skin_center_offset(7, 16.0, 8.0));
    assert!(middle > 0 && corner > 0, "both turns drew something");
    assert_eq!(middle, corner, "a turn about either anchor covers the same area, just elsewhere");
}

/// A one-to-one blit has to be exact: this is the path a glyph takes, and a half-texel drift would
/// blur every character on screen.
#[test]
fn a_quad_drawn_at_its_source_size_copies_the_texture_texel_for_texel() {
    let mut canvas = CpuCanvas::new(8, 8);
    canvas.clear(Color::BLACK);
    let tex = canvas.register_texture("checker", &checker(4, 1), 4, 4);
    canvas.draw_textured_quad(tex, QuadParams::new(Rect::new(2.0, 2.0, 4.0, 4.0)));
    for y in 0..4u32 {
        for x in 0..4u32 {
            let light = (x + y).is_multiple_of(2);
            let expected = if light {
                Color { r: 255, g: 255, b: 255, a: 255 }
            } else {
                apply_blend(Color { r: 60, g: 60, b: 60, a: 128 }, Color::BLACK, BlendMode::Alpha)
            };
            assert_eq!(canvas.pixel_at(2 + x, 2 + y), expected, "texel ({x}, {y})");
        }
    }
}

#[test]
fn nearest_sampling_keeps_hard_edges_and_linear_sampling_smooths_them() {
    let draw = |filter| {
        let mut canvas = CpuCanvas::new(16, 16);
        canvas.clear(Color::BLACK);
        let tex = canvas.register_texture("corners", &corners(), 2, 2);
        let mut params = QuadParams::new(Rect::new(0.0, 0.0, 16.0, 16.0));
        params.filter = filter;
        canvas.draw_textured_quad(tex, params);
        canvas
    };
    let nearest = draw(TextureFilter::Nearest);
    let shades: std::collections::BTreeSet<u8> = (0..16).flat_map(|y| (0..16).map(move |x| (x, y))).map(|(x, y)| nearest.pixel_at(x, y).r).collect();
    assert_eq!(shades.len(), 2, "nearest only ever produces the texture's own two shades");

    let linear = draw(TextureFilter::Linear);
    let shades: std::collections::BTreeSet<u8> = (0..16).flat_map(|y| (0..16).map(move |x| (x, y))).map(|(x, y)| linear.pixel_at(x, y).r).collect();
    assert!(shades.len() > 2, "linear interpolates between them, got {shades:?}");
    assert_eq!(linear.pixel_at(0, 0), nearest.pixel_at(0, 0), "the clamped edge still reads the corner texel");
}

#[test]
fn a_source_rectangle_picks_one_cell_out_of_a_sheet() {
    let mut canvas = CpuCanvas::new(4, 4);
    canvas.clear(Color::BLACK);
    let sheet = {
        let mut rgba = Vec::new();
        for y in 0..2u32 {
            for x in 0..2u32 {
                let value = ((y * 2 + x) as u8 + 1) * 50;
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        rgba
    };
    let tex = canvas.register_texture("sheet", &sheet, 2, 2);
    let mut params = QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0));
    params.src = UvRect::from_pixels(1, 1, 1, 1, 2, 2);
    canvas.draw_textured_quad(tex, params);
    assert_eq!(canvas.pixel_at(0, 0).r, 200, "the bottom right cell filled the whole quad");
    assert_eq!(canvas.pixel_at(3, 3).r, 200);
}

#[test]
fn a_texture_is_registered_once_per_key_and_re_registering_refreshes_its_pixels() {
    let mut canvas = CpuCanvas::new(4, 4);
    let first = canvas.register_texture("frame", &solid(2, [10, 10, 10, 255]), 2, 2);
    let second = canvas.register_texture("frame", &solid(2, [200, 0, 0, 255]), 2, 2);
    assert_eq!(first, second, "the same key keeps its handle");
    assert_eq!(canvas.live_texture_count(), 1, "and does not pile up textures");

    canvas.clear(Color::BLACK);
    canvas.draw_textured_quad(first, QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0)));
    assert_eq!(canvas.pixel_at(0, 0), Color { r: 200, g: 0, b: 0, a: 255 }, "the refreshed pixels are what draws");
}

#[test]
fn a_texture_reports_its_size_until_it_is_released() {
    let mut canvas = CpuCanvas::new(4, 4);
    let tex = canvas.register_texture("frame", &solid(3, [1, 2, 3, 255]), 3, 3);
    assert_eq!(canvas.texture_size(tex), Some((3, 3)));
    canvas.release_texture(tex);
    assert_eq!(canvas.texture_size(tex), None);
    assert_eq!(canvas.live_texture_count(), 0);
}

/// A stale handle has to be inert rather than fatal: a skin reload releases everything it held and
/// something drawn a frame late must not take the player down with it.
#[test]
fn drawing_or_releasing_an_unknown_handle_does_nothing() {
    let mut canvas = CpuCanvas::new(4, 4);
    canvas.clear(Color::BLACK);
    let tex = canvas.register_texture("frame", &solid(2, [255, 255, 255, 255]), 2, 2);
    canvas.release_texture(tex);
    canvas.release_texture(tex);
    canvas.release_texture(TextureId(9999));
    canvas.draw_textured_quad(tex, QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0)));
    canvas.draw_textured_quad(TextureId(9999), QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0)));
    assert_eq!(canvas.pixel_at(0, 0), Color::BLACK, "nothing was drawn");
}

/// Handles are never recycled, so a handle released by one owner cannot start naming a texture a
/// later owner registered.
#[test]
fn a_released_handle_is_not_handed_out_again() {
    let mut canvas = CpuCanvas::new(4, 4);
    let first = canvas.register_texture("a", &solid(2, [1, 1, 1, 255]), 2, 2);
    canvas.release_texture(first);
    let second = canvas.register_texture("b", &solid(2, [2, 2, 2, 255]), 2, 2);
    assert_ne!(first, second);
    assert_eq!(canvas.texture_size(first), None);
}

#[test]
fn a_zero_sized_or_empty_quad_draws_nothing() {
    let mut canvas = CpuCanvas::new(4, 4);
    canvas.clear(Color::BLACK);
    let tex = canvas.register_texture("frame", &solid(2, [255, 255, 255, 255]), 2, 2);
    for dst in [Rect::new(0.0, 0.0, 0.0, 4.0), Rect::new(0.0, 0.0, 4.0, 0.0), Rect::new(0.0, 0.0, -4.0, 4.0)] {
        canvas.draw_textured_quad(tex, QuadParams::new(dst));
    }
    let empty = canvas.register_texture("empty", &[], 0, 0);
    canvas.draw_textured_quad(empty, QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0)));
    assert_eq!(canvas.pixel_at(0, 0), Color::BLACK);
}

#[test]
fn a_clip_confines_both_fills_and_textured_quads() {
    let mut canvas = CpuCanvas::new(8, 8);
    canvas.clear(Color::BLACK);
    let tex = canvas.register_texture("white", &solid(2, [255, 255, 255, 255]), 2, 2);
    canvas.push_clip(Rect::new(0.0, 0.0, 4.0, 4.0));
    canvas.fill_rect(Rect::new(0.0, 0.0, 8.0, 2.0), Color::rgb(0, 200, 0));
    canvas.draw_textured_quad(tex, QuadParams::new(Rect::new(0.0, 2.0, 8.0, 6.0)));
    canvas.pop_clip();

    assert_eq!(canvas.pixel_at(0, 0).g, 200, "the fill drew inside the clip");
    assert_eq!(canvas.pixel_at(5, 0), Color::BLACK, "and stopped at its edge");
    assert_eq!(canvas.pixel_at(0, 3).r, 255, "the quad drew inside the clip");
    assert_eq!(canvas.pixel_at(5, 3), Color::BLACK, "and stopped at its edge");
    assert_eq!(canvas.pixel_at(0, 5), Color::BLACK, "including below it");
    assert_eq!(canvas.clip_depth(), 0, "the push and pop balanced");
}

#[test]
fn nested_clips_intersect_and_leaving_one_restores_the_other() {
    let mut canvas = CpuCanvas::new(8, 8);
    canvas.clear(Color::BLACK);
    canvas.push_clip(Rect::new(0.0, 0.0, 6.0, 6.0));
    canvas.push_clip(Rect::new(4.0, 0.0, 6.0, 8.0));
    canvas.fill_rect(Rect::new(0.0, 0.0, 8.0, 8.0), Color::rgb(200, 0, 0));
    canvas.pop_clip();
    canvas.fill_rect(Rect::new(0.0, 6.0, 8.0, 2.0), Color::rgb(0, 200, 0));
    canvas.pop_clip();

    assert_eq!(canvas.pixel_at(5, 5).r, 200, "the intersection drew");
    assert_eq!(canvas.pixel_at(3, 5), Color::BLACK, "left of the inner clip did not");
    assert_eq!(canvas.pixel_at(7, 5), Color::BLACK, "and neither did right of the outer one");
    assert_eq!(canvas.pixel_at(1, 7), Color::BLACK, "the second fill was still held by the outer clip");
}

#[test]
fn clips_that_stop_overlapping_discard_everything_until_the_matching_pop() {
    let mut canvas = CpuCanvas::new(8, 8);
    canvas.clear(Color::BLACK);
    canvas.push_clip(Rect::new(0.0, 0.0, 2.0, 2.0));
    canvas.push_clip(Rect::new(6.0, 6.0, 2.0, 2.0));
    canvas.push_clip(Rect::new(0.0, 0.0, 8.0, 8.0));
    canvas.fill_rect(Rect::new(0.0, 0.0, 8.0, 8.0), Color::rgb(200, 0, 0));
    canvas.pop_clip();
    canvas.pop_clip();
    canvas.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::rgb(0, 200, 0));
    canvas.pop_clip();

    assert!((0..8).flat_map(|y| (0..8).map(move |x| (x, y))).all(|(x, y)| canvas.pixel_at(x, y).r == 0), "nothing drew under the empty clip");
    assert_eq!(canvas.pixel_at(0, 0).g, 200, "and the outer clip still worked after the pops");
}

#[test]
fn popping_a_clip_that_was_never_pushed_is_survivable() {
    let mut canvas = CpuCanvas::new(4, 4);
    canvas.push_clip(Rect::new(0.0, 0.0, 2.0, 2.0));
    canvas.pop_clip();
    assert_eq!(canvas.clip_depth(), 0);
    canvas.clear(Color::BLACK);
    canvas.fill_rect(Rect::new(0.0, 0.0, 4.0, 4.0), Color::rgb(1, 2, 3));
    assert_eq!(canvas.pixel_at(3, 3), Color { r: 1, g: 2, b: 3, a: 255 }, "drawing is unclipped again");
}

/// A clip is not a scissor on the clear: the reference clears its whole frame too, and so does the
/// GPU backend's load operation. A clear also starts a frame, so it leaves the clip stack empty --
/// the two backends have to agree on that, or a clip left open by one frame narrows the next on one
/// of them and not the other.
#[test]
fn a_clear_ignores_the_clip_in_force_and_starts_the_frame_unclipped() {
    let mut canvas = CpuCanvas::new(4, 4);
    canvas.push_clip(Rect::new(0.0, 0.0, 1.0, 1.0));
    canvas.clear(Color::rgb(9, 9, 9));
    assert_eq!(canvas.pixel_at(3, 3), Color { r: 9, g: 9, b: 9, a: 255 }, "the clip did not scissor the clear");
    assert_eq!(canvas.clip_depth(), 0, "a frame begins unclipped whatever the last one left open");

    canvas.fill_rect(Rect::new(0.0, 0.0, 4.0, 4.0), Color::rgb(1, 2, 3));
    assert_eq!(canvas.pixel_at(3, 3), Color { r: 1, g: 2, b: 3, a: 255 }, "and drawing after it is unclipped too");
}

/// The contract says a buffer of the wrong length is padded or truncated rather than refused, so a
/// half-decoded video frame draws dark instead of killing the process. A caller that reads the
/// documentation and then checks the returned handle for failure would be checking nothing, which is
/// why the shape of this is pinned rather than left to the prose.
#[test]
fn a_texture_buffer_of_the_wrong_length_is_padded_or_truncated_rather_than_refused() {
    let mut canvas = CpuCanvas::new(4, 4);
    canvas.clear(Color::BLACK);

    let short = canvas.register_texture("short", &[255, 0, 0, 255], 2, 2);
    assert_eq!(canvas.texture_size(short), Some((2, 2)), "the texture is registered at the size it was declared at");
    canvas.draw_textured_quad(short, QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0)));
    assert_eq!(canvas.pixel_at(0, 0), Color { r: 255, g: 0, b: 0, a: 255 }, "the pixels that were supplied are there");
    assert_eq!(canvas.pixel_at(3, 3), Color::BLACK, "and the rest is transparent black, which draws as nothing");

    let long = canvas.register_texture("long", &[7; 64], 2, 2);
    assert_eq!(canvas.texture_size(long), Some((2, 2)), "an over-long buffer is truncated to the declared size");
}

/// Draw order is z-order. Two translucent quads over each other must land in the order they were
/// submitted, whichever texture each of them reads — gathering draws per texture would silently
/// swap them.
#[test]
fn overlapping_translucent_quads_land_in_submission_order() {
    let scene = |first: &str, second: &str| {
        let mut canvas = CpuCanvas::new(4, 4);
        canvas.clear(Color::BLACK);
        let red = canvas.register_texture("red", &solid(2, [255, 0, 0, 128]), 2, 2);
        let blue = canvas.register_texture("blue", &solid(2, [0, 0, 255, 128]), 2, 2);
        let handle = |name: &str| if name == "red" { red } else { blue };
        canvas.draw_textured_quad(handle(first), QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0)));
        canvas.draw_textured_quad(handle(second), QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0)));
        canvas.pixel_at(1, 1)
    };
    let red_then_blue = scene("red", "blue");
    let blue_then_red = scene("blue", "red");
    assert_ne!(red_then_blue, blue_then_red, "the order the quads were drawn in changed the result");
    assert!(red_then_blue.b > red_then_blue.r, "blue was drawn last, so blue is on top: {red_then_blue:?}");
    assert!(blue_then_red.r > blue_then_red.b, "and the other way round: {blue_then_red:?}");
}

/// Interleaving textured quads with flat fills must not reorder either kind past the other.
#[test]
fn textured_quads_and_fills_interleave_without_reordering() {
    let mut canvas = CpuCanvas::new(4, 4);
    canvas.clear(Color::BLACK);
    let tex = canvas.register_texture("white", &solid(2, [255, 255, 255, 255]), 2, 2);
    canvas.draw_textured_quad(tex, QuadParams::new(Rect::new(0.0, 0.0, 4.0, 4.0)));
    canvas.fill_rect(Rect::new(0.0, 0.0, 4.0, 4.0), Color::rgb(0, 128, 0));
    canvas.draw_textured_quad(tex, quad(Rect::new(0.0, 0.0, 2.0, 4.0), BlendMode::Multiply));
    assert_eq!(canvas.pixel_at(3, 1), Color { r: 0, g: 128, b: 0, a: 255 }, "the fill covered the first quad");
    assert_eq!(canvas.pixel_at(1, 1), Color { r: 0, g: 128, b: 0, a: 255 }, "and multiplying by white left it alone");
}

/// The atlas path exists to turn text into textured quads; it earns its place only if it draws the
/// same pixels the fill path does. Proving that here is what makes switching the default over a
/// change with no golden to regenerate.
#[test]
fn the_glyph_atlas_draws_the_same_pixels_as_the_fill_path() {
    rbms_render::font::use_embedded_fonts_only();
    let text = "Rendered 1234 :: gjpqy";
    let colour = Color { r: 235, g: 210, b: 90, a: 255 };

    let mut filled = CpuCanvas::new(256, 32);
    filled.clear(Color::BLACK);
    rbms_render::draw_text(&mut filled, 4.0, 8.0, 2.0, colour, text);

    let mut atlased = CpuCanvas::new(256, 32);
    atlased.clear(Color::BLACK);
    let mut binding = GlyphAtlasBinding::new();
    rbms_render::with_text_context(|ctx| ctx.draw_text_atlas(&mut atlased, &mut binding, (4.0, 8.0), 2.0, colour, text));

    assert!(binding.texture().is_some(), "the atlas page was uploaded");
    assert!(filled.pixels().iter().any(|b| *b != 0), "the fill path drew something to compare against");
    assert_eq!(atlased.pixels(), filled.pixels(), "the atlas path drew different pixels");
}

#[test]
fn the_glyph_atlas_packs_each_glyph_once_and_reuses_the_page() {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(128, 32);
    canvas.clear(Color::BLACK);
    let mut binding = GlyphAtlasBinding::new();
    let packed = rbms_render::with_text_context(|ctx| {
        ctx.draw_text_atlas(&mut canvas, &mut binding, (2.0, 4.0), 2.0, Color::WHITE, "abcabc");
        ctx.glyph_atlas_stats().0
    });
    assert!(packed >= 3, "the three distinct letters were packed, got {packed}");
    let (_, (width, height)) = rbms_render::with_text_context(|ctx| ctx.glyph_atlas_stats());
    assert_eq!(canvas.texture_size(binding.texture().expect("uploaded")), Some((width, height)));
    assert_eq!(canvas.live_texture_count(), 1, "one page, however many glyphs it holds");
}

#[test]
fn an_empty_string_draws_nothing_through_the_atlas() {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(32, 16);
    canvas.clear(Color::BLACK);
    let mut binding = GlyphAtlasBinding::new();
    rbms_render::with_text_context(|ctx| ctx.draw_text_atlas(&mut canvas, &mut binding, (2.0, 2.0), 2.0, Color::WHITE, ""));
    assert!((0..16).flat_map(|y| (0..32).map(move |x| (x, y))).all(|(x, y)| canvas.pixel_at(x, y) == Color::BLACK));
}

/// The four blend modes over a graded background, so what each of them does to colour and to
/// coverage is on one picture.
#[test]
fn the_blend_modes_match_their_golden() {
    let mut canvas = CpuCanvas::new(136, 48);
    canvas.clear(Color::BLACK);
    for x in 0..136 {
        let shade = (x * 255 / 135) as u8;
        canvas.fill_rect(Rect::new(x as f32, 0.0, 1.0, 48.0), Color::rgb(shade, shade / 2, 255 - shade));
    }
    let tex = canvas.register_texture("checker", &checker(16, 4), 16, 16);
    for (i, blend) in [BlendMode::Alpha, BlendMode::Add, BlendMode::Multiply, BlendMode::InvertDst].into_iter().enumerate() {
        canvas.draw_textured_quad(tex, quad(Rect::new(8.0 + i as f32 * 32.0, 8.0, 24.0, 32.0), blend));
    }
    check_golden(&canvas, "primitives_blend");
}

/// A quad turned about each of the anchors a skin can name, so a flipped sign or a swapped anchor
/// shows up as a picture rather than a number.
#[test]
fn rotation_and_centre_anchors_match_their_golden() {
    let mut canvas = CpuCanvas::new(144, 144);
    canvas.clear(Color::rgb(16, 16, 24));
    let tex = canvas.register_texture("quadrants", &quadrants(16), 16, 16);
    for (row, center) in [0, 1, 3, 7].into_iter().enumerate() {
        for (col, angle) in [0.0f32, 30.0, 90.0].into_iter().enumerate() {
            let dst = Rect::new(16.0 + col as f32 * 44.0, 16.0 + row as f32 * 32.0, 24.0, 16.0);
            let mut params = QuadParams::new(dst);
            params.angle_deg = angle;
            params.center = skin_center_offset(center, dst.w, dst.h);
            canvas.draw_textured_quad(tex, params);
        }
    }
    check_golden(&canvas, "primitives_rotation");
}

/// Nested clips against magnified sampling: the left column is clipped, the right shows nearest
/// over bilinear at the same scale.
#[test]
fn clipping_and_sampling_match_their_golden() {
    let mut canvas = CpuCanvas::new(144, 96);
    canvas.clear(Color::rgb(24, 20, 16));
    let checkerboard = canvas.register_texture("checker", &checker(16, 2), 16, 16);
    let two_by_two = canvas.register_texture("corners", &corners(), 2, 2);

    canvas.push_clip(Rect::new(8.0, 8.0, 56.0, 80.0));
    canvas.draw_textured_quad(checkerboard, QuadParams::new(Rect::new(0.0, 0.0, 144.0, 48.0)));
    canvas.push_clip(Rect::new(8.0, 48.0, 32.0, 24.0));
    canvas.fill_rect(Rect::new(0.0, 0.0, 144.0, 96.0), Color::rgb(200, 90, 40));
    canvas.pop_clip();
    canvas.pop_clip();

    for (i, filter) in [TextureFilter::Nearest, TextureFilter::Linear].into_iter().enumerate() {
        let mut params = QuadParams::new(Rect::new(76.0, 8.0 + i as f32 * 44.0, 56.0, 40.0));
        params.filter = filter;
        canvas.draw_textured_quad(two_by_two, params);
    }
    check_golden(&canvas, "primitives_clip");
}

/// A backend that records a frame and submits it at the end, which is what the GPU backend is.
///
/// Textures are registered immediately, because that is what an upload does; quads are held and
/// replayed at the end against whatever the registry holds by then. Anything that hands out
/// coordinates into a texture that later moves shows up here as a difference from the immediate
/// backend, and nowhere else -- the CPU backend rasterizes as it goes and cannot see the bug at all.
struct DeferredCanvas {
    canvas: CpuCanvas,
    recorded: Vec<Recorded>,
}

enum Recorded {
    Quad(TextureId, QuadParams),
    Fill(Rect, Color),
    PushClip(Rect),
    PopClip,
}

impl DeferredCanvas {
    fn new(width: u32, height: u32) -> DeferredCanvas {
        DeferredCanvas { canvas: CpuCanvas::new(width, height), recorded: Vec::new() }
    }

    /// Draws everything the frame recorded, and answers the finished pixels.
    fn submit(mut self) -> CpuCanvas {
        for entry in std::mem::take(&mut self.recorded) {
            match entry {
                Recorded::Quad(tex, params) => self.canvas.draw_textured_quad(tex, params),
                Recorded::Fill(rect, color) => self.canvas.fill_rect(rect, color),
                Recorded::PushClip(rect) => self.canvas.push_clip(rect),
                Recorded::PopClip => self.canvas.pop_clip(),
            }
        }
        self.canvas
    }
}

impl Renderer for DeferredCanvas {
    fn size(&self) -> (u32, u32) {
        self.canvas.size()
    }

    fn clear(&mut self, color: Color) {
        self.recorded.clear();
        self.canvas.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.recorded.push(Recorded::Fill(rect, color));
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        self.canvas.register_texture(key, rgba, width, height)
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.canvas.release_texture(tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.canvas.texture_size(tex)
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        self.recorded.push(Recorded::Quad(tex, params));
    }

    fn push_clip(&mut self, rect: Rect) {
        self.recorded.push(Recorded::PushClip(rect));
    }

    fn pop_clip(&mut self) {
        self.recorded.push(Recorded::PopClip);
    }
}

/// A frame of two lines whose sizes are far enough apart that the second makes the atlas page grow.
fn growing_text_scene<R: Renderer>(r: &mut R, binding: &mut GlyphAtlasBinding) {
    r.clear(Color::BLACK);
    rbms_render::with_text_context(|ctx| {
        ctx.reset_glyph_atlas();
        ctx.draw_text_atlas(r, binding, (2.0, 6.0), 1.0, Color::WHITE, "small line");
        ctx.draw_text_atlas(r, binding, (2.0, 60.0), 30.0, Color::WHITE, "BIG");
    });
}

/// Two lines of very different sizes, the second of which makes the atlas page grow. On an
/// immediate backend the growth cannot be noticed; on one that submits at the end, the first line's
/// coordinates would name the wrong pixels of the grown page unless the page it was measured
/// against is still there. Drawing the same scene through both backends is what catches that.
#[test]
fn a_deferred_backend_draws_the_same_text_as_an_immediate_one_across_a_page_growth() {
    rbms_render::font::use_embedded_fonts_only();
    let mut immediate = CpuCanvas::new(640, 400);
    let mut first = GlyphAtlasBinding::new();
    growing_text_scene(&mut immediate, &mut first);
    first.end_frame(&mut immediate);

    let mut deferred = DeferredCanvas::new(640, 400);
    let mut second = GlyphAtlasBinding::new();
    growing_text_scene(&mut deferred, &mut second);
    let submitted = deferred.submit();

    assert!(immediate.pixels().iter().any(|byte| *byte != 0), "the scene drew something to compare");
    assert_eq!(submitted.pixels(), immediate.pixels(), "a page that grew mid-frame moved the pixels an earlier line was measured against");
}

/// The older page a frame still needs is kept until the frame says it is done with it, and handed
/// back then -- otherwise every growth would leak a texture for the rest of the session.
#[test]
fn a_page_replaced_mid_frame_is_handed_back_when_the_frame_ends() {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(640, 400);
    let mut binding = GlyphAtlasBinding::new();
    growing_text_scene(&mut canvas, &mut binding);
    assert_eq!(binding.retired_count(), 1, "the page the first line was measured against is still uploaded");
    assert_eq!(canvas.live_texture_count(), 2, "both pages are live while the frame is unfinished");

    binding.end_frame(&mut canvas);
    assert_eq!(binding.retired_count(), 0);
    assert_eq!(canvas.live_texture_count(), 1, "the older page is handed back once the frame is submitted");
}
