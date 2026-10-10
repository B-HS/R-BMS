//! The GPU backend's pixels, checked against the CPU reference canvas.
//!
//! Each test queues one scene on a window-less [`Gpu`] and on a [`CpuCanvas`] of the same size, reads
//! the GPU's target back and compares the two byte for byte. They are the only tests that put the
//! shaders, the blend states and the target format through an actual adapter, and what they pin is
//! the colour rule: the same draws give the same bytes on both backends.
//!
//! The distance field scenes at the end put the reference's distance field shader through the same
//! comparison: one disc-shaped field, drawn with and without an outline and a shadow, at its own
//! size, enlarged, shrunk and on fractions of a pixel. Each prints how far apart the two backends
//! came out, so a run with `--nocapture` shows the measured difference beside the allowance.
//!
//! A machine with no adapter -- a headless CI box -- cannot run them, and there they pass without
//! checking anything. Setting [`super::REQUIRE_GPU_ENV`] turns that into a failure, for a run that has to
//! prove the comparison really happened.

use rbms_render::{
    BlendMode, Color, CpuCanvas, DistanceFieldParams, DistanceFieldStyle, QuadParams, Rect, Renderer, ScaledRenderer, TextureFilter, TextureId, UvRect,
};

use super::Gpu;
use super::batch::BYTES_PER_PIXEL;
use crate::notify::{self, Level};

/// Size of the target the scenes are drawn on.
const WIDTH: u32 = 96;
const HEIGHT: u32 = 64;

/// The shape a fitted frame keeps in these tests: the scenes' own.
const SHAPE: (u32, u32) = (WIDTH, HEIGHT);

/// How far one channel may sit from the CPU canvas's value. The CPU canvas truncates each weighted
/// sum where the GPU rounds it, once for a tint and once for a blend.
const TOLERANCE: u8 = 2;

/// Edge of the texture the quad scenes sample.
const SWATCH: u32 = 8;

/// The alphas the swatch cycles through, so translucent texels are blended as well as opaque ones.
const SWATCH_ALPHAS: [u8; 4] = [255, 192, 128, 64];

/// How many times larger than the scene the enlarged target is on each axis.
const ENLARGEMENT: u32 = 2;

/// Height of the taller target the letterbox test fits the scene into.
const TALL_HEIGHT: u32 = 96;

const BACKDROP: Color = Color::rgb(18, 18, 24);
const MIDTONE: Color = Color::rgb(128, 64, 200);

/// One arrangement of draws, queued identically on both backends.
#[derive(Debug, Clone, Copy)]
enum Scene {
    Solid,
    AlphaBlend,
    Additive,
    Textured,
    MultiplyAndInvert,
    Clipped,
}

/// A texture with a different colour in every texel and a spread of alphas, so a quad sampled from
/// the wrong place or blended by the wrong rule cannot come out looking right.
fn swatch() -> Vec<u8> {
    (0..SWATCH * SWATCH).flat_map(|i| [(i * 4) as u8, (255 - i * 3) as u8, (i * 37 % 256) as u8, SWATCH_ALPHAS[i as usize % SWATCH_ALPHAS.len()]]).collect()
}

fn register_swatch<R: Renderer>(r: &mut R) -> TextureId {
    r.register_texture("pixel_tests.swatch", &swatch(), SWATCH, SWATCH)
}

/// Queue `scene` on `r`. Every rectangle sits on whole pixels, so which pixels are covered is not in
/// question and what is compared is their colour.
fn paint<R: Renderer>(scene: Scene, r: &mut R) {
    match scene {
        Scene::Solid => {
            r.clear(BACKDROP);
            r.fill_rect(Rect::new(8.0, 8.0, 40.0, 24.0), MIDTONE);
            r.fill_rect(Rect::new(50.0, 10.0, 30.0, 40.0), Color::ORANGE);
            r.fill_rect(Rect::new(4.0, 40.0, 30.0, 16.0), Color::GRAY);
        }
        Scene::AlphaBlend => {
            r.clear(Color::rgb(40, 80, 120));
            r.fill_rect(Rect::new(4.0, 4.0, 60.0, 40.0), Color { r: 200, g: 100, b: 50, a: 128 });
            r.fill_rect(Rect::new(30.0, 20.0, 50.0, 30.0), Color { r: 20, g: 220, b: 90, a: 64 });
            let tex = register_swatch(r);
            r.draw_textured_quad(tex, QuadParams { tint: Color { r: 255, g: 255, b: 255, a: 128 }, ..QuadParams::new(Rect::new(8.0, 28.0, 32.0, 32.0)) });
        }
        Scene::Additive => {
            r.clear(Color::rgb(60, 60, 60));
            r.fill_rect(Rect::new(0.0, 0.0, 48.0, 64.0), MIDTONE);
            let tex = register_swatch(r);
            r.draw_textured_quad(tex, QuadParams { blend: BlendMode::Add, ..QuadParams::new(Rect::new(8.0, 8.0, 32.0, 32.0)) });
            let faded = Color { r: 255, g: 200, b: 100, a: 128 };
            r.draw_textured_quad(tex, QuadParams { blend: BlendMode::Add, tint: faded, ..QuadParams::new(Rect::new(40.0, 24.0, 32.0, 32.0)) });
        }
        Scene::Textured => {
            r.clear(BACKDROP);
            let tex = register_swatch(r);
            r.draw_textured_quad(tex, QuadParams::new(Rect::new(4.0, 4.0, SWATCH as f32, SWATCH as f32)));
            r.draw_textured_quad(tex, QuadParams::new(Rect::new(20.0, 4.0, 32.0, 32.0)));
            let corner = UvRect::from_pixels(2, 2, 4, 4, SWATCH, SWATCH);
            r.draw_textured_quad(tex, QuadParams { src: corner, ..QuadParams::new(Rect::new(60.0, 4.0, 16.0, 16.0)) });
            let tinted = Color { r: 255, g: 128, b: 64, a: 200 };
            r.draw_textured_quad(tex, QuadParams { tint: tinted, ..QuadParams::new(Rect::new(60.0, 28.0, 24.0, 24.0)) });
            r.draw_textured_quad(tex, QuadParams { filter: TextureFilter::Linear, ..QuadParams::new(Rect::new(4.0, 40.0, 16.0, 16.0)) });
        }
        Scene::MultiplyAndInvert => {
            r.clear(Color::rgb(200, 150, 90));
            r.fill_rect(Rect::new(0.0, 32.0, 96.0, 32.0), MIDTONE);
            let tex = register_swatch(r);
            r.draw_textured_quad(tex, QuadParams { blend: BlendMode::Multiply, ..QuadParams::new(Rect::new(8.0, 16.0, 32.0, 32.0)) });
            r.draw_textured_quad(tex, QuadParams { blend: BlendMode::InvertDst, ..QuadParams::new(Rect::new(52.0, 16.0, 32.0, 32.0)) });
        }
        Scene::Clipped => {
            r.clear(BACKDROP);
            r.push_clip(Rect::new(10.0, 6.0, 40.0, 30.0));
            r.fill_rect(Rect::new(0.0, 0.0, 96.0, 64.0), MIDTONE);
            r.push_clip(Rect::new(30.0, 20.0, 60.0, 40.0));
            let tex = register_swatch(r);
            r.draw_textured_quad(tex, QuadParams::new(Rect::new(16.0, 8.0, 64.0, 48.0)));
            r.pop_clip();
            r.pop_clip();
            r.fill_rect(Rect::new(70.0, 50.0, 20.0, 10.0), Color::GREEN);
        }
    }
}

/// A window-less backend that fits the scenes' own shape, or `None` on a machine that has no adapter
/// to give one.
fn offscreen(width: u32, height: u32) -> Option<Gpu> {
    Gpu::offscreen_if_available(width, height, SHAPE)
}

/// The largest difference between any two corresponding channels, and the pixel it was found at.
fn worst_difference(gpu: &[u8], cpu: &[u8], width: u32) -> (u8, (u32, u32)) {
    assert_eq!(gpu.len(), cpu.len(), "the two frames are not the same size");
    let per_pixel = BYTES_PER_PIXEL as usize;
    gpu.iter().zip(cpu).enumerate().map(|(i, (a, b))| (a.abs_diff(*b), i / per_pixel)).max_by_key(|(difference, _)| *difference).map_or(
        (0, (0, 0)),
        |(difference, pixel)| {
            let pixel = pixel as u32;
            (difference, (pixel % width, pixel / width))
        },
    )
}

fn assert_frames_agree(gpu: &[u8], cpu: &[u8], width: u32, what: &str) {
    let (difference, (x, y)) = worst_difference(gpu, cpu, width);
    assert!(difference <= TOLERANCE, "{what}: the backends are {difference}/255 apart at ({x}, {y}), past the {TOLERANCE} allowed");
}

/// Draw `scene` on both backends at the scenes' own size and compare what comes out.
fn assert_backends_agree(scene: Scene) {
    let Some(mut gpu) = offscreen(WIDTH, HEIGHT) else {
        return;
    };
    paint(scene, &mut gpu);
    let from_gpu = gpu.capture().expect("an offscreen target reads back");

    let mut cpu = CpuCanvas::new(WIDTH, HEIGHT);
    paint(scene, &mut cpu);

    assert_eq!(gpu.size(), cpu.size(), "an unfitted target is drawn in its own pixels");
    assert_frames_agree(&from_gpu, cpu.pixels(), WIDTH, &format!("{scene:?}"));
}

#[test]
fn solid_rectangles_come_out_the_colour_they_were_given() {
    assert_backends_agree(Scene::Solid);
}

/// The test the colour decision rests on: a midtone written to the target is that midtone, not the
/// brighter value an sRGB target would have stored for it.
#[test]
fn a_midtone_reaches_the_target_as_the_byte_it_was_drawn_with() {
    let Some(mut gpu) = offscreen(WIDTH, HEIGHT) else {
        return;
    };
    gpu.clear(MIDTONE);
    let pixels = gpu.capture().expect("an offscreen target reads back");
    assert_eq!(&pixels[..BYTES_PER_PIXEL as usize], &[MIDTONE.r, MIDTONE.g, MIDTONE.b, u8::MAX]);
}

#[test]
fn translucent_draws_blend_the_way_the_cpu_canvas_does() {
    assert_backends_agree(Scene::AlphaBlend);
}

#[test]
fn additive_draws_blend_the_way_the_cpu_canvas_does() {
    assert_backends_agree(Scene::Additive);
}

#[test]
fn textured_quads_sample_and_tint_the_way_the_cpu_canvas_does() {
    assert_backends_agree(Scene::Textured);
}

#[test]
fn multiplying_and_inverting_draws_blend_the_way_the_cpu_canvas_does() {
    assert_backends_agree(Scene::MultiplyAndInvert);
}

#[test]
fn a_clip_scissors_the_same_pixels_on_both_backends() {
    assert_backends_agree(Scene::Clipped);
}

/// What the built-in screens go through on a window larger than they are laid out for: the same
/// draws, scaled out by the adapter, land on the same pixels of both backends.
#[test]
fn a_scene_scaled_out_to_a_larger_target_agrees_on_both_backends() {
    let (width, height) = (WIDTH * ENLARGEMENT, HEIGHT * ENLARGEMENT);
    let Some(mut gpu) = offscreen(width, height) else {
        return;
    };
    for scene in [Scene::Solid, Scene::Textured, Scene::Clipped] {
        paint(scene, &mut ScaledRenderer::new(&mut gpu, SHAPE));
        let from_gpu = gpu.capture().expect("an offscreen target reads back");

        let mut cpu = CpuCanvas::new(width, height);
        paint(scene, &mut ScaledRenderer::new(&mut cpu, SHAPE));

        assert_frames_agree(&from_gpu, cpu.pixels(), width, &format!("{scene:?} enlarged"));
    }
}

/// A fitted frame is drawn in the viewport's own pixels and lands inside the bars: the space it is
/// drawn in shrinks to what is left, its clips follow it there, and the bars keep the clear colour.
#[test]
fn a_fitted_frame_is_drawn_in_the_pixels_left_inside_the_bars() {
    let Some(mut gpu) = offscreen(WIDTH, TALL_HEIGHT) else {
        return;
    };
    assert_eq!(gpu.size(), (WIDTH, TALL_HEIGHT), "stretched, the whole target is the space drawn in");
    gpu.set_letterbox(true);
    assert_eq!(gpu.size(), (WIDTH, TALL_HEIGHT), "the fit does not change under a frame already being drawn");
    gpu.clear(BACKDROP);
    gpu.capture().expect("an offscreen target reads back");
    assert_eq!(gpu.size(), SHAPE, "and takes hold once that frame has been handed over");

    let bar = (TALL_HEIGHT - HEIGHT) / 2;
    assert_eq!(gpu.position_in_space(10.0, bar as f32 + 5.0, gpu.size()), (10.0, 5.0), "a position is measured from where the frame starts");

    paint(Scene::Clipped, &mut gpu);
    let from_gpu = gpu.capture().expect("an offscreen target reads back");

    let mut cpu = CpuCanvas::new(WIDTH, HEIGHT);
    paint(Scene::Clipped, &mut cpu);
    let row = (WIDTH * BYTES_PER_PIXEL) as usize;
    let bar_bytes = bar as usize * row;
    let framed = &from_gpu[bar_bytes..from_gpu.len() - bar_bytes];
    assert_frames_agree(framed, cpu.pixels(), WIDTH, "the fitted frame");

    let cleared = [BACKDROP.r, BACKDROP.g, BACKDROP.b, u8::MAX];
    let in_bars = from_gpu[..bar_bytes].chunks_exact(cleared.len()).chain(from_gpu[from_gpu.len() - bar_bytes..].chunks_exact(cleared.len()));
    for pixel in in_bars {
        assert_eq!(pixel, cleared, "a bar holds only the colour the frame was cleared to");
    }
}

/// An image past the adapter's limit is turned away with one warning, its handle draws nothing, and
/// the key is free to take an image that does fit.
#[test]
fn an_image_past_the_adapters_limit_is_refused_and_draws_nothing() {
    let Some(mut gpu) = offscreen(WIDTH, HEIGHT) else {
        return;
    };
    let limit = gpu.max_texture_size();
    assert!(limit >= SWATCH, "an adapter holds at least a swatch");

    let reported = notify::exclusive(|| {
        let refused = gpu.register_texture("pixel_tests.oversized", &[], limit + 1, 1);
        assert_eq!(gpu.texture_size(refused), None, "a refused image has no size");
        assert_eq!(gpu.register_texture("pixel_tests.oversized", &[], 1, limit + 1), refused, "and its key keeps the one handle");

        gpu.clear(BACKDROP);
        gpu.draw_textured_quad(refused, QuadParams::new(Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32)));
        assert_eq!(gpu.quad_count(), 0, "nothing is queued from a handle that names no texture");

        let accepted = gpu.register_texture("pixel_tests.oversized", &swatch(), SWATCH, SWATCH);
        assert_eq!(accepted, refused, "the same key takes an image that fits");
        assert_eq!(gpu.texture_size(accepted), Some((SWATCH, SWATCH)));

        let mut reported = Vec::new();
        notify::drain(&mut reported);
        reported
    });
    assert_eq!(reported.len(), 1, "one warning for the image, not one for every time it was offered: {reported:?}");
    assert_eq!(reported[0].0, Level::Warn);
    assert!(reported[0].1.contains("pixel_tests.oversized"), "the warning names the image: {}", reported[0].1);
}

/// A width whose rows are not on the copy alignment is read back through padded rows, and the
/// padding must not reach the caller.
#[test]
fn a_target_of_another_size_reads_back_every_row_unpadded() {
    const ODD_WIDTH: u32 = 75;
    const ODD_HEIGHT: u32 = 7;
    let Some(mut gpu) = offscreen(ODD_WIDTH, ODD_HEIGHT) else {
        return;
    };
    gpu.clear(BACKDROP);
    gpu.fill_rect(Rect::new(ODD_WIDTH as f32 - 1.0, ODD_HEIGHT as f32 - 1.0, 1.0, 1.0), MIDTONE);
    let pixels = gpu.capture().expect("an offscreen target reads back");
    assert_eq!(pixels.len(), (ODD_WIDTH * ODD_HEIGHT * BYTES_PER_PIXEL) as usize, "rows come back without the copy's padding");
    let last = pixels.len() - BYTES_PER_PIXEL as usize;
    assert_eq!(&pixels[last..], &[MIDTONE.r, MIDTONE.g, MIDTONE.b, u8::MAX], "and the last pixel is the last pixel drawn");
    assert_eq!(&pixels[..BYTES_PER_PIXEL as usize], &[BACKDROP.r, BACKDROP.g, BACKDROP.b, u8::MAX]);
}

/// Edge of the distance field the field scenes draw from, in texels.
const FIELD: u32 = 32;

/// The radius of the disc the field describes, in texels, and how far either side of its outline
/// the field runs from nothing to everything.
const DISC_RADIUS: f32 = 8.0;
const DISC_SPREAD: f32 = 4.0;

/// The distance a field holds on the outline it describes, as a share of one.
const ON_THE_OUTLINE: f32 = 0.5;

/// What the field scenes are cleared to: a colour with three different channels, none of them at
/// either end, so a wrong channel or a wrong weight shows.
const FIELD_BACKDROP: Color = Color::rgb(40, 90, 150);

/// The colour the field scenes' glyph is drawn in: translucent, so its alpha is weighed too.
const FIELD_INK: Color = Color { r: 250, g: 220, b: 40, a: 230 };

/// An outline and a shadow that are both plainly there: an outline reaching half the field's range
/// further out, and a soft shadow moved three texels right and two down.
const DRESSED: DistanceFieldStyle = DistanceFieldStyle {
    outline_distance: 0.25,
    outline_color: Color { r: 200, g: 30, b: 60, a: 255 },
    shadow_color: Color { r: 10, g: 20, b: 80, a: 200 },
    shadow_smoothing: 0.125,
    shadow_offset: (3.0 / FIELD as f32, 2.0 / FIELD as f32),
};

/// The same shadow with no smoothing, which hands the shader a ramp of no width.
const HARD_SHADOWED: DistanceFieldStyle = DistanceFieldStyle { outline_distance: 0.5, shadow_smoothing: 0.0, ..DRESSED };

/// The distance field of a disc in the middle of a [`FIELD`]-texel square: white, with the distance
/// to the disc's outline in its alpha -- half on the outline, more inside and less outside.
fn disc_field() -> Vec<u8> {
    let middle = FIELD as f32 / 2.0;
    (0..FIELD * FIELD)
        .flat_map(|i| {
            let (x, y) = ((i % FIELD) as f32 + 0.5 - middle, (i / FIELD) as f32 + 0.5 - middle);
            let held = (ON_THE_OUTLINE + (DISC_RADIUS - x.hypot(y)) / (2.0 * DISC_SPREAD)).clamp(0.0, 1.0);
            [u8::MAX, u8::MAX, u8::MAX, (held * f32::from(u8::MAX)).round() as u8]
        })
        .collect()
}

fn register_field<R: Renderer>(r: &mut R) -> TextureId {
    r.register_texture("pixel_tests.field", &disc_field(), FIELD, FIELD)
}

/// One arrangement of distance field quads, queued identically on both backends.
#[derive(Debug, Clone, Copy)]
enum FieldScene {
    /// What a text that asks for nothing is drawn with, on whole pixels and on fractions of one.
    Plain,
    /// An outline and a soft shadow.
    Dressed,
    /// The dressed glyph at twice its size, at half of it, and squeezed across only.
    Resized,
    /// A shadow with no smoothing, on whole pixels.
    HardShadowed,
    /// The dressed glyph under each blend a text can inherit, clipped, and between ordinary quads.
    Blended,
}

const FIELD_SCENES: [FieldScene; 5] = [FieldScene::Plain, FieldScene::Dressed, FieldScene::Resized, FieldScene::HardShadowed, FieldScene::Blended];

fn field_quad(dst: Rect, style: DistanceFieldStyle) -> DistanceFieldParams {
    DistanceFieldParams { dst, src: UvRect::FULL, tint: FIELD_INK, blend: BlendMode::Alpha, style }
}

fn paint_field<R: Renderer>(scene: FieldScene, r: &mut R) {
    let edge = FIELD as f32;
    r.clear(FIELD_BACKDROP);
    let tex = register_field(r);
    match scene {
        FieldScene::Plain => {
            r.draw_distance_field_quad(tex, field_quad(Rect::new(8.0, 16.0, edge, edge), DistanceFieldStyle::PLAIN));
            r.draw_distance_field_quad(tex, field_quad(Rect::new(52.5, 14.25, edge, edge), DistanceFieldStyle::PLAIN));
        }
        FieldScene::Dressed => {
            r.draw_distance_field_quad(tex, field_quad(Rect::new(8.0, 16.0, edge, edge), DRESSED));
            r.draw_distance_field_quad(tex, field_quad(Rect::new(52.5, 14.25, edge, edge), DRESSED));
        }
        FieldScene::Resized => {
            r.draw_distance_field_quad(tex, field_quad(Rect::new(0.0, 0.0, 2.0 * edge, 2.0 * edge), DRESSED));
            r.draw_distance_field_quad(tex, field_quad(Rect::new(70.0, 4.0, edge / 2.0, edge / 2.0), DRESSED));
            r.draw_distance_field_quad(tex, field_quad(Rect::new(66.3, 28.6, 0.7 * edge, edge), DRESSED));
        }
        FieldScene::HardShadowed => {
            r.draw_distance_field_quad(tex, field_quad(Rect::new(8.0, 16.0, edge, edge), HARD_SHADOWED));
            r.draw_distance_field_quad(tex, field_quad(Rect::new(52.0, 14.0, edge, edge), HARD_SHADOWED));
        }
        FieldScene::Blended => {
            r.fill_rect(Rect::new(0.0, 0.0, 48.0, 64.0), MIDTONE);
            r.draw_distance_field_quad(tex, DistanceFieldParams { blend: BlendMode::Add, ..field_quad(Rect::new(0.0, 0.0, edge, edge), DRESSED) });
            r.draw_distance_field_quad(tex, DistanceFieldParams { blend: BlendMode::Multiply, ..field_quad(Rect::new(32.0, 0.0, edge, edge), DRESSED) });
            r.draw_distance_field_quad(tex, DistanceFieldParams { blend: BlendMode::InvertDst, ..field_quad(Rect::new(64.0, 0.0, edge, edge), DRESSED) });
            r.push_clip(Rect::new(10.0, 40.0, 20.0, 24.0));
            r.draw_distance_field_quad(tex, field_quad(Rect::new(0.0, 32.0, edge, edge), DRESSED));
            r.pop_clip();
            let swatch = register_swatch(r);
            r.draw_textured_quad(swatch, QuadParams::new(Rect::new(40.0, 36.0, 24.0, 24.0)));
            r.draw_distance_field_quad(tex, field_quad(Rect::new(48.0, 32.0, edge, edge), DistanceFieldStyle::PLAIN));
            r.fill_rect(Rect::new(72.0, 44.0, 20.0, 8.0), Color { a: 128, ..Color::GREEN });
        }
    }
}

/// The reference's distance field shader comes to the same pixels on the GPU as on the CPU canvas:
/// the ramps, the outline, the shadow and the final mix, at every size and under every blend.
#[test]
fn distance_field_quads_come_out_the_way_the_cpu_canvas_draws_them() {
    let Some(mut gpu) = offscreen(WIDTH, HEIGHT) else {
        return;
    };
    for scene in FIELD_SCENES {
        paint_field(scene, &mut gpu);
        let from_gpu = gpu.capture().expect("an offscreen target reads back");

        let mut cpu = CpuCanvas::new(WIDTH, HEIGHT);
        paint_field(scene, &mut cpu);

        let (difference, (x, y)) = worst_difference(&from_gpu, cpu.pixels(), WIDTH);
        println!("distance field {scene:?}: the backends are at most {difference}/255 apart, at ({x}, {y}); {TOLERANCE} allowed");
        assert_frames_agree(&from_gpu, cpu.pixels(), WIDTH, &format!("distance field {scene:?}"));
        assert_ne!(cpu.pixels(), CpuCanvas::new(WIDTH, HEIGHT).pixels(), "the scene drew nothing to compare");
    }
}

/// What a text on a built-in sized screen goes through on a larger window: the same field quads
/// scaled out by the adapter agree as well.
#[test]
fn distance_field_quads_scaled_out_to_a_larger_target_agree_on_both_backends() {
    let (width, height) = (WIDTH * ENLARGEMENT, HEIGHT * ENLARGEMENT);
    let Some(mut gpu) = offscreen(width, height) else {
        return;
    };
    for scene in [FieldScene::Dressed, FieldScene::Resized] {
        paint_field(scene, &mut ScaledRenderer::new(&mut gpu, SHAPE));
        let from_gpu = gpu.capture().expect("an offscreen target reads back");

        let mut cpu = CpuCanvas::new(width, height);
        paint_field(scene, &mut ScaledRenderer::new(&mut cpu, SHAPE));

        let (difference, (x, y)) = worst_difference(&from_gpu, cpu.pixels(), width);
        println!("distance field {scene:?} enlarged: the backends are at most {difference}/255 apart, at ({x}, {y}); {TOLERANCE} allowed");
        assert_frames_agree(&from_gpu, cpu.pixels(), width, &format!("distance field {scene:?} enlarged"));
    }
}

/// The comparison above would pass as readily if both backends drew the field as a plain texture.
/// This holds the GPU's own pixels to what the shader has to write where that is beyond argument:
/// the glyph's colour deep inside the disc, the outline's in the band around it, the shadow's where
/// only the shadow reaches, and nothing at all past them.
#[test]
fn a_distance_field_glyph_on_the_gpu_has_its_body_its_outline_and_its_shadow() {
    const LEFT: u32 = 8;
    const TOP: u32 = 16;
    let Some(mut gpu) = offscreen(WIDTH, HEIGHT) else {
        return;
    };
    gpu.clear(Color::BLACK);
    let tex = register_field(&mut gpu);
    let opaque = Color { a: u8::MAX, ..FIELD_INK };
    let shadow = Color { a: u8::MAX, ..DRESSED.shadow_color };
    let style = DistanceFieldStyle { shadow_color: shadow, shadow_offset: (6.0 / FIELD as f32, 0.0), ..DRESSED };
    gpu.draw_distance_field_quad(
        tex,
        DistanceFieldParams { tint: opaque, ..field_quad(Rect::new(LEFT as f32, TOP as f32, FIELD as f32, FIELD as f32), style) },
    );
    let pixels = gpu.capture().expect("an offscreen target reads back");
    let at = |x: u32, y: u32| {
        let i = ((y * WIDTH + x) * BYTES_PER_PIXEL) as usize;
        Color { r: pixels[i], g: pixels[i + 1], b: pixels[i + 2], a: pixels[i + 3] }
    };
    let (middle_x, middle_y) = (LEFT + FIELD / 2, TOP + FIELD / 2);

    assert_eq!(at(middle_x, middle_y), opaque, "the middle of the disc is the glyph's own colour");
    assert_eq!(at(middle_x + 6, middle_y + 6), DRESSED.outline_color, "nine texels out along the diagonal is past the disc and inside its outline");
    assert_eq!(at(middle_x + 11, middle_y), shadow, "eleven right is past the outline and inside the shadow, which was moved six right");
    assert_eq!(at(middle_x - 12, middle_y), Color::BLACK, "twelve left is past both");
    assert_eq!(at(LEFT + FIELD + 2, middle_y), Color::BLACK, "and nothing is drawn outside the quad");
}
