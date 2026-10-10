//! Tests of the distance field path: the shader's arithmetic one fragment at a time, against
//! numbers worked out by hand, and then whole quads on the canvas -- where a glyph's edge, its
//! outline and its shadow land, and how wide the edge is at each size.
//!
//! The canvas tests draw from the field of a disc: a [`FIELD`]-texel square whose alpha is one half
//! on a circle of [`DISC_RADIUS`] texels about its middle, rising to one [`DISC_SPREAD`] texels
//! inside the circle and falling to nothing as far outside it. One texel across the outline is
//! therefore an eighth of the field's range, and the shader's ramp of a sixteenth either side of
//! the outline is half a texel either side: full ink within 7.5 texels of the middle and none past
//! 8.5.

use super::*;
use crate::UNLIMITED_TEXTURE_SIZE;

/// How far a number the shader computes may sit from the one worked out by hand.
const CLOSE: f32 = 1e-6;

/// Edge of the disc's field, in texels.
const FIELD: u32 = 32;

/// The radius of the disc, in texels, and how far either side of its outline the field runs from
/// nothing to everything.
const DISC_RADIUS: f32 = 8.0;
const DISC_SPREAD: f32 = 4.0;

/// The canvas the quads are drawn on, large enough for the disc at four times its size.
const STAGE: (u32, u32) = (160, 160);

const BACKDROP: Color = Color { r: 100, g: 100, b: 100, a: u8::MAX };
const BLACK: Color = Color { r: 0, g: 0, b: 0, a: u8::MAX };
const RED: Color = Color { r: u8::MAX, g: 0, b: 0, a: u8::MAX };
const GREEN: Color = Color { r: 0, g: u8::MAX, b: 0, a: u8::MAX };
const BLUE: Color = Color { r: 0, g: 0, b: u8::MAX, a: u8::MAX };

/// Where the disc's quad is put: its top left corner, the same across and down.
const ORIGIN: u32 = 16;

/// An outline reaching a quarter of the field's range, which is two texels, further out.
const OUTLINED: DistanceFieldStyle = DistanceFieldStyle { outline_distance: 0.25, outline_color: RED, ..DistanceFieldStyle::PLAIN };

/// How far an outline of that width reaches past the disc before the ramp starts taking it away,
/// and how far before nothing of it is left, in texels.
const OUTLINE_FIRM: f32 = 1.5;
const OUTLINE_GONE: f32 = 2.5;

/// A field that rises evenly from left to right: [`SLOPE_TEXELS`] across, each texel
/// [`SLOPE_STEP`] channel values more than the one before it. A sixteenth of the field's range is
/// very nearly four of its texels.
const SLOPE_TEXELS: u32 = 64;
const SLOPE_STEP: u32 = 4;

/// How many texels of the slope a sixteenth of the field's range either side of a line comes to:
/// `2 / 16 * 255 / 4`, rounded.
const SIXTEENTH_OF_THE_SLOPE: usize = 8;

/// How many texels of the slope the glyph's edge shows over on a plain backdrop. Its ramp is those
/// same eight texels, but the alpha along it is the ramp to the fourth power, which over the ramp's
/// first three texels is too little to store as anything.
const GLYPH_EDGE_TEXELS: usize = 5;

/// How many texels left of the glyph the shadow of the slope tests is moved, which clears the
/// shadow's edge of the glyph's.
const SHADOW_LEAD: u32 = 24;

/// The row the slope is drawn on and how tall it is drawn.
const SLOPE_ROW: u32 = 8;
const SLOPE_HEIGHT: f32 = 4.0;

fn slope_field() -> Vec<u8> {
    (0..SLOPE_TEXELS).flat_map(|x| [u8::MAX, u8::MAX, u8::MAX, (x * SLOPE_STEP) as u8]).collect()
}

/// How many pixels of the slope's row are part way between two colours.
struct SlopeEdges {
    /// Neither the glyph's black nor anything the shadow or the backdrop could be.
    glyph: usize,
    /// Neither the shadow's blue nor the backdrop, left of where the glyph begins.
    shadow: usize,
}

/// Draws the slope `scale` times its size in black and counts the pixels of its edges.
fn edges_of_the_slope(scale: f32, style: DistanceFieldStyle) -> SlopeEdges {
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    canvas.clear(BACKDROP);
    let tex = canvas.register_texture("field.slope", &slope_field(), SLOPE_TEXELS, 1);
    let dst = Rect::new(ORIGIN as f32, SLOPE_ROW as f32, SLOPE_TEXELS as f32 * scale, SLOPE_HEIGHT);
    canvas.draw_distance_field_quad(tex, DistanceFieldParams { dst, src: UvRect::FULL, tint: BLACK, blend: BlendMode::Alpha, style });
    let row: Vec<Color> = (0..(SLOPE_TEXELS as f32 * scale) as u32).map(|x| canvas.pixel_at(ORIGIN + x, SLOPE_ROW)).collect();
    let first_ink = row.iter().position(|pixel| *pixel == BLACK).expect("the slope reaches full ink");
    let glyph_begins = row[..first_ink].iter().rposition(|pixel| *pixel == BACKDROP || *pixel == BLUE).map_or(0, |last| last + 1);
    let shadow = row[..glyph_begins].iter().filter(|pixel| **pixel != BLUE && **pixel != BACKDROP).count();
    SlopeEdges { glyph: first_ink - glyph_begins, shadow }
}

fn assert_written(written: [f32; 4], expected: [f32; 4], what: &str) {
    for (channel, (got, want)) in written.iter().zip(expected).enumerate() {
        assert!((got - want).abs() <= CLOSE, "{what}: channel {channel} is {got}, not {want} ({written:?})");
    }
}

/// The colour a fragment the shader wrote `written` for is stored as.
fn stored(written: [f32; 4]) -> Color {
    Color { r: channel_of(written[0]), g: channel_of(written[1]), b: channel_of(written[2]), a: channel_of(written[ALPHA]) }
}

/// The field of the disc: the distance in the alpha channel, under a colour no test draws with, so
/// a path that read the texture's colour would show.
fn disc_field() -> Vec<u8> {
    let middle = FIELD as f32 / 2.0;
    (0..FIELD * FIELD)
        .flat_map(|i| {
            let (x, y) = ((i % FIELD) as f32 + 0.5 - middle, (i / FIELD) as f32 + 0.5 - middle);
            let held = (DISTANCE_FIELD_EDGE + (DISC_RADIUS - x.hypot(y)) / (2.0 * DISC_SPREAD)).clamp(0.0, 1.0);
            [0, u8::MAX, u8::MAX, channel_of(held)]
        })
        .collect()
}

fn stage() -> (CpuCanvas, TextureId) {
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    canvas.clear(BACKDROP);
    let tex = canvas.register_texture("field.disc", &disc_field(), FIELD, FIELD);
    (canvas, tex)
}

/// The disc drawn `scale` times its size with its quad's corner on [`ORIGIN`].
fn disc(scale: f32, tint: Color, style: DistanceFieldStyle) -> DistanceFieldParams {
    let edge = FIELD as f32 * scale;
    DistanceFieldParams { dst: Rect::new(ORIGIN as f32, ORIGIN as f32, edge, edge), src: UvRect::FULL, tint, blend: BlendMode::Alpha, style }
}

/// The pixel `(right, down)` pixels from the one whose corner is the middle of a disc drawn `scale`
/// times its size. The pixel's own centre is half a pixel further along both ways.
fn from_middle(canvas: &CpuCanvas, scale: u32, right: i32, down: i32) -> Color {
    let middle = (ORIGIN + scale * FIELD / 2) as i32;
    canvas.pixel_at((middle + right) as u32, (middle + down) as u32)
}

/// How far the centre of that pixel is from the middle of the disc, in texels.
fn texels_out(scale: u32, right: i32, down: i32) -> f32 {
    (right as f32 + 0.5).hypot(down as f32 + 0.5) / scale as f32
}

/// The pixels of the row just below the middle of a disc drawn `scale` times its size, from the
/// middle rightwards to the quad's edge.
fn row_out(canvas: &CpuCanvas, scale: u32) -> Vec<Color> {
    (0..(scale * FIELD / 2) as i32).map(|right| from_middle(canvas, scale, right, 0)).collect()
}

/// Inside the glyph the shader writes the text's own colour, and outside it nothing at all.
#[test]
fn a_fragment_well_inside_is_the_tint_and_one_well_outside_is_nothing() {
    let plain = DistanceFieldStyle::PLAIN;
    let inside = DISTANCE_FIELD_EDGE + DISTANCE_FIELD_SMOOTHING;
    let outside = DISTANCE_FIELD_EDGE - DISTANCE_FIELD_SMOOTHING;
    assert_written(distance_field_fragment(inside, inside, BLACK, &plain), [0.0, 0.0, 0.0, 1.0], "the top of the ramp");
    assert_written(distance_field_fragment(1.0, 1.0, RED, &plain), [1.0, 0.0, 0.0, 1.0], "deep inside");
    assert_written(distance_field_fragment(outside, outside, BLACK, &plain), [1.0, 1.0, 1.0, 0.0], "the foot of the ramp");
    assert_written(distance_field_fragment(0.0, 0.0, RED, &plain), [1.0, 1.0, 1.0, 0.0], "far outside");
}

/// The edge is not the ramp a distance field is usually drawn with. On the outline itself the ramp
/// is at one half; the outline's transparent white is mixed half into the text's colour, the result
/// is weighed by the ramp, and the final mix weighs it by its own alpha again:
///
/// ```text
/// outlineFactor = 0.5
/// color         = mix((1, 1, 1, 0), (0, 0, 0, 1), 0.5)       = (0.5, 0.5, 0.5, 0.5)
/// mainColor     = (0.5, 0.5, 0.5, 0.5 * 0.5)                 = (0.5, 0.5, 0.5, 0.25)
/// shadow        = (1, 1, 1, 0)
/// gl_FragColor  = mix(shadow, mainColor, 0.25)               = (0.875, 0.875, 0.875, 0.0625)
/// ```
///
/// So the alpha on the outline is the ramp to the fourth power, a sixteenth, and what little is
/// drawn there of black text is nearly white.
#[test]
fn on_the_outline_the_alpha_is_the_ramp_to_the_fourth_power_and_the_colour_is_whitened() {
    let plain = DistanceFieldStyle::PLAIN;
    assert_written(distance_field_fragment(0.5, 0.5, BLACK, &plain), [0.875, 0.875, 0.875, 0.0625], "black on the outline");
    assert_written(distance_field_fragment(0.5, 0.5, RED, &plain), [1.0, 0.875, 0.875, 0.0625], "red on the outline");

    let three_quarters_up = DISTANCE_FIELD_EDGE + DISTANCE_FIELD_SMOOTHING / 2.0;
    let ramp = 0.75 * 0.75 * (3.0 - 2.0 * 0.75);
    assert!((ramp - 0.84375f32).abs() <= CLOSE);
    let glyph_alpha = ramp * ramp;
    let grey = (1.0 - glyph_alpha) + (1.0 - ramp) * glyph_alpha;
    assert_written(
        distance_field_fragment(three_quarters_up, three_quarters_up, BLACK, &plain),
        [grey, grey, grey, ramp.powi(4)],
        "three quarters of the way up the ramp",
    );
}

/// A translucent text is drawn more translucent than it asked to be: the final mix weighs the
/// glyph's alpha by itself, so half comes out a quarter, and the other three quarters of the colour
/// are the shadow's white.
#[test]
fn a_translucent_tint_comes_out_with_its_alpha_squared() {
    let half = Color { a: 128, ..BLACK };
    let alpha = 128.0 / 255.0;
    assert_written(
        distance_field_fragment(1.0, 1.0, half, &DistanceFieldStyle::PLAIN),
        [1.0 - alpha, 1.0 - alpha, 1.0 - alpha, alpha * alpha],
        "deep inside a half transparent glyph",
    );
}

/// The outline is the band between the distance the text asks for and the glyph's own edge, in the
/// outline's colour, with the same sixteenth of ramp on either side of both.
#[test]
fn an_outline_fills_the_band_between_its_distance_and_the_glyphs_edge() {
    assert_written(distance_field_fragment(0.375, 0.0, BLACK, &OUTLINED), [1.0, 0.0, 0.0, 1.0], "the middle of the band is the outline's colour");
    assert_written(distance_field_fragment(0.5, 0.0, BLACK, &OUTLINED), [0.5, 0.0, 0.0, 1.0], "on the glyph's edge it is half way to the text's");
    assert_written(distance_field_fragment(0.5625, 0.0, BLACK, &OUTLINED), [0.0, 0.0, 0.0, 1.0], "past the ramp it is the text's");
    assert_written(
        distance_field_fragment(0.25, 0.0, BLACK, &OUTLINED),
        [1.0, 0.5, 0.5, 0.25],
        "on the outline's own edge the ramp is a half: alpha a quarter, and the colour half way to the shadow's white",
    );
    assert_written(distance_field_fragment(0.1875, 0.0, BLACK, &OUTLINED), [1.0, 1.0, 1.0, 0.0], "and past its ramp there is nothing");
}

/// The shadow is the same field read somewhere else, under a ramp of its own width about the
/// outline, and it shows wherever the glyph does not cover it.
#[test]
fn a_shadow_shows_where_the_glyph_is_not_and_fades_over_its_own_ramp() {
    let soft = DistanceFieldStyle { shadow_color: BLUE, shadow_smoothing: 0.125, ..DistanceFieldStyle::PLAIN };
    assert_written(distance_field_fragment(0.0, 0.625, BLACK, &soft), [0.0, 0.0, 1.0, 1.0], "the top of the shadow's ramp");
    assert_written(distance_field_fragment(0.0, 0.5, BLACK, &soft), [0.0, 0.0, 1.0, 0.5], "half way up it");
    assert_written(distance_field_fragment(0.0, 0.5625, BLACK, &soft), [0.0, 0.0, 1.0, 0.84375], "three quarters of the way up it");
    assert_written(distance_field_fragment(0.0, 0.375, BLACK, &soft), [0.0, 0.0, 1.0, 0.0], "its foot");
    assert_written(distance_field_fragment(1.0, 1.0, BLACK, &soft), [0.0, 0.0, 0.0, 1.0], "under the glyph none of it shows");
    assert_written(
        distance_field_fragment(0.5, 1.0, BLACK, &soft),
        [0.125, 0.125, 0.875, 0.8125],
        "on the glyph's edge a quarter is the glyph's half-white and the rest the shadow",
    );

    let faint = DistanceFieldStyle { shadow_color: Color { a: 51, ..BLUE }, ..soft };
    assert_written(distance_field_fragment(0.0, 1.0, BLACK, &faint), [0.0, 0.0, 1.0, 0.2], "the shadow's own alpha scales it");
}

/// A shadow with no smoothing hands the shader a ramp with both ends alike, which the specification
/// leaves undefined. Here it is a step, off on the outline itself.
#[test]
fn a_shadow_with_no_smoothing_is_a_step_on_the_outline() {
    let hard = DistanceFieldStyle { shadow_color: BLUE, ..DistanceFieldStyle::PLAIN };
    assert_written(distance_field_fragment(0.0, 0.5, BLACK, &hard), [0.0, 0.0, 1.0, 0.0], "on the outline");
    assert_written(distance_field_fragment(0.0, 0.5001, BLACK, &hard), [0.0, 0.0, 1.0, 1.0], "just inside it");
    assert_written(distance_field_fragment(0.0, 0.4999, BLACK, &hard), [0.0, 0.0, 1.0, 0.0], "just outside it");
    assert_eq!(smoothstep(0.75, 0.25, 0.25), 1.0, "a ramp whose ends are the wrong way round runs the wrong way");
    assert_eq!(smoothstep(0.75, 0.25, 0.75), 0.0);
}

/// What a fragment exactly on the outline comes to on the canvas, worked through to the stored
/// pixel. A field two texels wide, nothing and everything, drawn half a pixel off the grid has the
/// centre of its middle pixel exactly between the two: a distance of one half.
///
/// The shader writes `(0.875, 0.875, 0.875, 0.0625)` for black there, stored as `(223, 223, 223,
/// 16)`, and over a backdrop of 100 that is `(223 * 16 + 100 * 239) / 255 = 107`. Black text
/// lightens the pixels along its edge.
#[test]
fn a_pixel_exactly_on_the_outline_is_blended_at_a_sixteenth_and_lightened() {
    const LEFT: u32 = 10;
    const ROW: u32 = 4;
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    canvas.clear(BACKDROP);
    let step = [[u8::MAX, u8::MAX, u8::MAX, 0], [u8::MAX, u8::MAX, u8::MAX, u8::MAX]].concat();
    let tex = canvas.register_texture("field.step", &step, 2, 1);
    let dst = Rect::new(LEFT as f32 + 0.5, ROW as f32, 2.0, 1.0);
    canvas
        .draw_distance_field_quad(tex, DistanceFieldParams { dst, src: UvRect::FULL, tint: BLACK, blend: BlendMode::Alpha, style: DistanceFieldStyle::PLAIN });

    assert_eq!(stored(distance_field_fragment(0.5, 0.5, BLACK, &DistanceFieldStyle::PLAIN)), Color { r: 223, g: 223, b: 223, a: 16 });
    assert_eq!(canvas.pixel_at(LEFT, ROW), BACKDROP, "the pixel whose centre is on the empty texel's edge reads nothing");
    assert_eq!(canvas.pixel_at(LEFT + 1, ROW), Color { r: 107, g: 107, b: 107, a: u8::MAX }, "the pixel between the two texels is on the outline");
    assert_eq!(canvas.pixel_at(LEFT + 2, ROW), BACKDROP, "and the quad covers no third pixel's centre");
}

/// The disc at its own size: the text's colour to within half a texel of the outline, the backdrop
/// from half a texel past it, and the texture's own colour nowhere.
#[test]
fn a_glyph_is_its_tint_inside_the_outline_and_untouched_outside() {
    let (mut canvas, tex) = stage();
    canvas.draw_distance_field_quad(tex, disc(1.0, GREEN, DistanceFieldStyle::PLAIN));

    let reach = FIELD as i32 / 2;
    for down in -reach..reach {
        for right in -reach..reach {
            let (out, pixel) = (texels_out(1, right, down), from_middle(&canvas, 1, right, down));
            if out <= DISC_RADIUS - 1.0 {
                assert_eq!(pixel, GREEN, "{out} texels out is inside the disc");
            } else if out >= DISC_RADIUS + 1.0 {
                assert_eq!(pixel, BACKDROP, "{out} texels out is outside it");
            } else {
                assert_eq!(pixel.b, pixel.r, "the edge is weighed between green and white over grey, never towards the texture's cyan");
            }
        }
    }
    assert_eq!(canvas.pixel_at(ORIGIN + FIELD, ORIGIN + FIELD / 2), BACKDROP, "nothing is drawn past the quad");
}

/// The width of the edge is a share of the field and not of a pixel, so on screen it is as many
/// times wider as the glyph is drawn larger. Measured on a field that rises evenly, the pixels that
/// are neither ink nor backdrop double when the field is drawn at twice its size and halve at half
/// of it. A shadow's edge does the same.
#[test]
fn the_edge_is_as_many_pixels_wide_as_the_glyph_is_enlarged() {
    let edge_at = |scale: f32| edges_of_the_slope(scale, DistanceFieldStyle::PLAIN).glyph;
    let (halved, plain, doubled) = (edge_at(0.5), edge_at(1.0), edge_at(2.0));
    assert_eq!(plain, GLYPH_EDGE_TEXELS, "at its own size the ramp shows over this many texels of the slope");
    assert!(doubled.abs_diff(2 * plain) <= 1, "twice the size, twice the edge: {doubled}");
    assert!(halved.abs_diff(plain / 2) <= 1, "half the size, half the edge: {halved}");

    let (mut canvas, tex) = stage();
    canvas.draw_distance_field_quad(tex, disc(0.5, BLACK, DistanceFieldStyle::PLAIN));
    let row: Vec<Color> = (0..FIELD / 4).map(|right| canvas.pixel_at(ORIGIN + FIELD / 4 + right, ORIGIN + FIELD / 4)).collect();
    let inked = row.iter().filter(|pixel| **pixel == BLACK).count();
    assert_eq!(inked, (DISC_RADIUS / 2.0) as usize, "at half its size the disc is four pixels to its edge");
    assert!(row[inked..].iter().all(|pixel| *pixel == BACKDROP), "and the pixel after the last inked one is already the backdrop: {row:?}");
}

/// An outline reaches as far past the glyph as its distance says, in its own colour, and the glyph
/// inside it is unchanged.
#[test]
fn an_outline_reaches_its_width_past_the_glyph_in_its_own_colour() {
    let (mut canvas, tex) = stage();
    canvas.draw_distance_field_quad(tex, disc(1.0, BLACK, OUTLINED));

    let reach = FIELD as i32 / 2;
    for down in -reach..reach {
        for right in -reach..reach {
            let (out, pixel) = (texels_out(1, right, down), from_middle(&canvas, 1, right, down));
            if out <= DISC_RADIUS - 1.0 {
                assert_eq!(pixel, BLACK, "{out} texels out is still the glyph");
            } else if (DISC_RADIUS + 0.75..=DISC_RADIUS + OUTLINE_FIRM - 0.25).contains(&out) {
                assert_eq!(pixel, RED, "{out} texels out is in the outline");
            } else if out >= DISC_RADIUS + OUTLINE_GONE + 0.25 {
                assert_eq!(pixel, BACKDROP, "{out} texels out is past it");
            }
        }
    }

    let wider = DistanceFieldStyle { outline_distance: 0.125, ..OUTLINED };
    let (mut canvas, tex) = stage();
    canvas.draw_distance_field_quad(tex, disc(1.0, BLACK, wider));
    let lit = |canvas: &CpuCanvas| row_out(canvas, 1).iter().filter(|pixel| **pixel != BACKDROP).count();
    assert_eq!(lit(&canvas), 11, "an outline at an eighth reaches three texels past the disc");
    let (mut plain, tex) = stage();
    plain.draw_distance_field_quad(tex, disc(1.0, BLACK, DistanceFieldStyle::PLAIN));
    assert_eq!(lit(&plain), 8, "where the bare glyph ends at its own eight");
}

/// The shadow is the glyph's own shape in the shadow's colour, moved by the offset: right and down
/// for a positive one. The offset is a share of the texture, so it is as many texels however large
/// the glyph is drawn, and the shadow never leaves the glyph's quad.
#[test]
fn a_shadow_is_the_glyph_moved_by_the_offset_in_texels_and_cut_at_the_quad() {
    const MOVED: (i32, i32) = (5, 3);
    let style =
        DistanceFieldStyle { shadow_color: BLUE, shadow_offset: (MOVED.0 as f32 / FIELD as f32, MOVED.1 as f32 / FIELD as f32), ..DistanceFieldStyle::PLAIN };
    for scale in [1u32, 2] {
        let (mut canvas, tex) = stage();
        canvas.draw_distance_field_quad(tex, disc(scale as f32, BLACK, style));
        let reach = (scale * FIELD / 2) as i32;
        let moved = (MOVED.0 * scale as i32, MOVED.1 * scale as i32);
        for down in -reach..reach {
            for right in -reach..reach {
                let glyph = texels_out(scale, right, down);
                let shadow = texels_out(scale, right - moved.0, down - moved.1);
                let pixel = from_middle(&canvas, scale, right, down);
                if glyph <= DISC_RADIUS - 1.0 {
                    assert_eq!(pixel, BLACK, "scale {scale}: the glyph covers its shadow");
                } else if glyph >= DISC_RADIUS + 1.0 && shadow <= DISC_RADIUS - 0.5 {
                    assert_eq!(pixel, BLUE, "scale {scale}: ({right}, {down}) is under the moved disc only");
                } else if glyph >= DISC_RADIUS + 1.0 && shadow >= DISC_RADIUS + 0.5 {
                    assert_eq!(pixel, BACKDROP, "scale {scale}: ({right}, {down}) is under neither");
                }
            }
        }
    }

    let far = DistanceFieldStyle { shadow_offset: (12.0 / FIELD as f32, 0.0), ..style };
    let (mut canvas, tex) = stage();
    canvas.draw_distance_field_quad(tex, disc(1.0, BLACK, far));
    assert_eq!(from_middle(&canvas, 1, 15, 0), BLUE, "a shadow moved twelve texels reaches the quad's last column");
    assert_eq!(from_middle(&canvas, 1, 16, 0), BACKDROP, "and stops there, though the moved disc goes on for four more");
}

/// A soft shadow fades over its own ramp, which is as wide as its smoothing says, twice as wide
/// for twice the smoothing, and is not the glyph's: the glyph's edge is the same whatever the
/// shadow's is.
#[test]
fn a_shadows_smoothing_widens_its_edge_and_not_the_glyphs() {
    let shadowed = |shadow_smoothing: f32| {
        let moved = (-(SHADOW_LEAD as f32) / SLOPE_TEXELS as f32, 0.0);
        edges_of_the_slope(1.0, DistanceFieldStyle { shadow_color: BLUE, shadow_offset: moved, shadow_smoothing, ..DistanceFieldStyle::PLAIN })
    };
    let (hard, soft, softer) = (shadowed(0.0), shadowed(0.0625), shadowed(0.125));
    assert_eq!(hard.shadow, 0, "with no smoothing every pixel is the shadow or is not");
    assert!(soft.shadow.abs_diff(SIXTEENTH_OF_THE_SLOPE) <= 1, "a sixteenth either side of the outline is this many texels of the slope: {}", soft.shadow);
    assert!(softer.shadow.abs_diff(2 * SIXTEENTH_OF_THE_SLOPE) <= 1, "and an eighth is twice that: {}", softer.shadow);
    assert!(hard.glyph >= GLYPH_EDGE_TEXELS, "over a shadow no less of the glyph's ramp shows than over the backdrop");
    assert_eq!((soft.glyph, softer.glyph), (hard.glyph, hard.glyph), "and it is the same ramp whatever the shadow's is");
}

/// The shadow reads the texture wherever the offset takes it, which on a page of many glyphs is
/// into the glyph next door. That is the reference's doing and is kept: a glyph with nothing to its
/// right in its own cell still gets the shadow of whatever the page holds there.
#[test]
fn a_shadow_reads_past_its_own_glyph_into_the_rest_of_the_page() {
    const CELL: u32 = 8;
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    canvas.clear(BACKDROP);
    let page: Vec<u8> = (0..2 * CELL * CELL).flat_map(|i| [u8::MAX, u8::MAX, u8::MAX, if i % (2 * CELL) < CELL { 0 } else { u8::MAX }]).collect();
    let tex = canvas.register_texture("field.page", &page, 2 * CELL, CELL);
    let style = DistanceFieldStyle { shadow_color: BLUE, shadow_offset: (-0.25, 0.0), ..DistanceFieldStyle::PLAIN };
    let dst = Rect::new(ORIGIN as f32, ORIGIN as f32, CELL as f32, CELL as f32);
    let src = UvRect::from_pixels(0, 0, CELL, CELL, 2 * CELL, CELL);
    canvas.draw_distance_field_quad(tex, DistanceFieldParams { dst, src, tint: BLACK, blend: BlendMode::Alpha, style });

    assert_eq!(canvas.pixel_at(ORIGIN + 3, ORIGIN + 4), BACKDROP, "the empty cell's left half is still empty four texels to its right");
    assert_eq!(canvas.pixel_at(ORIGIN + 5, ORIGIN + 4), BLUE, "its right half is shadowed by the cell next door");
}

/// What the shader writes goes through the blend the quad asks for, like any other quad.
#[test]
fn what_the_shader_writes_is_blended_the_way_the_quad_asks() {
    let (mut canvas, tex) = stage();
    canvas.draw_distance_field_quad(
        tex,
        DistanceFieldParams { blend: BlendMode::Add, ..disc(1.0, Color { r: 50, g: 60, b: 70, a: u8::MAX }, DistanceFieldStyle::PLAIN) },
    );
    assert_eq!(from_middle(&canvas, 1, 0, 0), Color { r: 150, g: 160, b: 170, a: u8::MAX }, "an additive glyph adds its colour to the backdrop");
    assert_eq!(from_middle(&canvas, 1, 12, 0), BACKDROP);
}

/// A clip, a texture that is gone and a quad of no size are honoured as an ordinary quad's are.
#[test]
fn a_distance_field_quad_is_clipped_and_skipped_like_any_other() {
    let (mut canvas, tex) = stage();
    let middle = ORIGIN + FIELD / 2;
    canvas.push_clip(Rect::new(middle as f32, 0.0, STAGE.0 as f32, STAGE.1 as f32));
    canvas.draw_distance_field_quad(tex, disc(1.0, BLACK, DistanceFieldStyle::PLAIN));
    canvas.pop_clip();
    assert_eq!(canvas.pixel_at(middle, middle), BLACK, "inside the clip");
    assert_eq!(canvas.pixel_at(middle - 1, middle), BACKDROP, "outside it");

    let (mut canvas, tex) = stage();
    canvas.draw_distance_field_quad(tex, DistanceFieldParams { dst: Rect::new(0.0, 0.0, 0.0, 8.0), ..disc(1.0, BLACK, DistanceFieldStyle::PLAIN) });
    canvas.release_texture(tex);
    canvas.draw_distance_field_quad(tex, disc(1.0, BLACK, DistanceFieldStyle::PLAIN));
    assert!((0..STAGE.1).all(|y| (0..STAGE.0).all(|x| canvas.pixel_at(x, y) == BACKDROP)));
    assert_eq!(canvas.max_texture_size(), UNLIMITED_TEXTURE_SIZE);
}

/// A renderer that does not draw distance fields draws the field's texels as a plain quad, and a
/// wrapper hands the call on with only the destination changed.
#[test]
fn the_default_falls_back_to_a_plain_quad_and_the_scaling_wrapper_hands_the_call_on() {
    struct Plain(CpuCanvas);
    impl Renderer for Plain {
        fn size(&self) -> (u32, u32) {
            self.0.size()
        }
        fn clear(&mut self, color: Color) {
            self.0.clear(color);
        }
        fn fill_rect(&mut self, rect: Rect, color: Color) {
            self.0.fill_rect(rect, color);
        }
        fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
            self.0.register_texture(key, rgba, width, height)
        }
        fn release_texture(&mut self, tex: TextureId) {
            self.0.release_texture(tex);
        }
        fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
            self.0.texture_size(tex)
        }
        fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
            self.0.draw_textured_quad(tex, params);
        }
        fn push_clip(&mut self, rect: Rect) {
            self.0.push_clip(rect);
        }
        fn pop_clip(&mut self) {
            self.0.pop_clip();
        }
    }

    let (canvas, tex) = stage();
    let mut plain = Plain(canvas);
    plain.draw_distance_field_quad(tex, disc(1.0, Color { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: u8::MAX }, DistanceFieldStyle::PLAIN));
    assert_eq!(from_middle(&plain.0, 1, 0, 0), Color { r: 0, g: u8::MAX, b: u8::MAX, a: u8::MAX }, "the texture's own cyan, which the shader never shows");

    let mut direct = CpuCanvas::new(STAGE.0, STAGE.1);
    direct.clear(BACKDROP);
    let tex = direct.register_texture("field.disc", &disc_field(), FIELD, FIELD);
    direct.draw_distance_field_quad(tex, disc(2.0, BLACK, OUTLINED));

    let mut target = CpuCanvas::new(STAGE.0, STAGE.1);
    target.clear(BACKDROP);
    let tex = target.register_texture("field.disc", &disc_field(), FIELD, FIELD);
    let half = (STAGE.0 / 2, STAGE.1 / 2);
    let mut scaled = crate::ScaledRenderer::new(&mut target, half);
    let quad = disc(1.0, BLACK, OUTLINED);
    scaled.draw_distance_field_quad(tex, DistanceFieldParams { dst: Rect::new(quad.dst.x / 2.0, quad.dst.y / 2.0, quad.dst.w, quad.dst.h), ..quad });
    assert_eq!(direct.pixels(), target.pixels(), "drawn through a wrapper that doubles everything, the quad is the one drawn at twice the size");
}
