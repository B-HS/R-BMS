//! Drawing a whole screen into one rectangle of a canvas.
//!
//! A built-in screen is laid out in the fixed [`UI_SIZE`] space and says where things go in it. A
//! skin is drawn on the target's own pixels and lays itself out for as many of them as it is handed
//! ([`Canvas::native`]). [`with_region`] is where the two meet: the rectangle is given in the fixed
//! space, and what is drawn in it is drawn on the pixels that rectangle really covers, as a target of
//! its own ([`RegionRenderer`]) -- its own size, its own origin, and a clip that keeps everything
//! inside.
//!
//! Both targets a canvas can be are drawn on alike. The move is arithmetic and the clip is the one
//! every backend has, so nothing here knows whether the frame ends up in a window or in memory.

use rbms_render::{Rect, RegionRenderer, Renderer, scale_between, scale_rect};

use crate::stage::Canvas;
use crate::stage::canvas::{NativeCanvas, UI_SIZE};

/// Run `draw` on the part of `canvas` that `rect` covers, as a renderer of its own.
///
/// `rect` is in [`UI_SIZE`] coordinates. The renderer `draw` is handed reports the size of the
/// rectangle in the target's own pixels, measures every coordinate from the rectangle's top left
/// corner, and draws nothing outside it. Its `clear` paints the rectangle and leaves the rest of the
/// frame alone, so a screen that begins by clearing can be drawn over a frame another screen has
/// already begun.
///
/// The rectangle is put on whole pixels of the target, so two frames drawn into the same rectangle
/// land on the same pixels, and what `draw` is handed may be a pixel larger or smaller than `rect`
/// scaled out exactly.
pub(crate) fn with_region<'a, T>(canvas: &mut Canvas<'a>, rect: Rect, draw: impl FnOnce(&mut RegionRenderer<'_, NativeCanvas<'_, 'a>>) -> T) -> T {
    let mut native = canvas.native();
    let placed = scale_rect(rect, scale_between(UI_SIZE, native.size()));
    draw(&mut RegionRenderer::new(&mut native, placed))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rbms_render::{Color, QuadParams, TextureFilter, draw_text, draw_text_centered, theme};

    use super::*;
    use crate::gpu::Gpu;
    use crate::stage::HeadlessCanvas;

    /// Environment variable naming the folder captures are saved in, the one the screen capture
    /// harness reads. Nothing is saved without it.
    const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

    /// The name the test pattern is saved under.
    const PATTERN_SHOT: &str = "region-pattern";

    /// A target half as large again as the one the built-in screens are laid out for, and how many
    /// times larger than it that is.
    const LARGE: (u32, u32) = (1920, 1080);
    const LARGE_ENLARGEMENT: f32 = 1.5;

    /// A target of the size the built-in screens are laid out for is no larger than it.
    const SAME_SIZE: f32 = 1.0;

    /// The two targets every test here draws on, each with how much larger than [`UI_SIZE`] it is.
    const TARGETS: [((u32, u32), f32); 2] = [(UI_SIZE, SAME_SIZE), (LARGE, LARGE_ENLARGEMENT)];

    /// Where the screen round the region writes its own line, in [`UI_SIZE`] coordinates.
    const SCREEN_TEXT_AT: (f32, f32) = (24.0, 24.0);

    /// The rectangle the screen draws after the region is done, in [`UI_SIZE`] coordinates: across
    /// the screen, below the region.
    const SCREEN_BAR: Rect = Rect { x: 24.0, y: 600.0, w: 1232.0, h: 8.0 };

    /// How opaque the band that reaches past both sides of the pattern is.
    const BAND_ALPHA: u8 = 96;

    /// How much of the pattern's height its colour bars and its squares take.
    const BARS_SHARE: f32 = 0.25;
    const SQUARE_SHARE: f32 = 0.3;

    /// How many widths of the pattern the band that only a clip keeps in is across.
    const BAND_WIDTHS: f32 = 3.0;

    /// The rectangle the pattern is drawn into, in [`UI_SIZE`] coordinates: the shape of a screen,
    /// away from every edge of the canvas.
    const REGION: Rect = Rect { x: 424.0, y: 84.0, w: 832.0, h: 468.0 };

    /// How far one channel of a GPU frame may sit from the headless canvas's value, the allowance the
    /// GPU pixel tests make for the one truncating where the other rounds.
    const GPU_TOLERANCE: u8 = 2;

    /// How many pixels of a GPU frame may sit further off than that: the ones an edge of a quad
    /// falls on, where the two backends round a coverage differently.
    const GPU_EDGE_PIXELS_ONE_IN: usize = 100;

    /// Edge of the checker texture the pattern samples.
    const CHECKER: u32 = 8;

    /// The colours the pattern is drawn in.
    const WASH: Color = Color::rgb(16, 18, 28);
    const FRAME: Color = Color::rgb(230, 210, 70);
    const BARS: [Color; 6] = [
        Color::rgb(230, 70, 70),
        Color::rgb(230, 150, 60),
        Color::rgb(230, 210, 70),
        Color::rgb(70, 220, 120),
        Color::rgb(70, 130, 230),
        Color::rgb(160, 90, 220),
    ];

    /// How thick the frame round the pattern is, and how far the bars and the checker sit from it,
    /// in the region's own pixels.
    const FRAME_THICKNESS: f32 = 4.0;
    const INSET: f32 = 24.0;

    /// The text sizes of the pattern.
    const TITLE_SCALE: f32 = 2.0;
    const NOTE_SCALE: f32 = 1.2;

    /// A checker of two shades, so a quad drawn from it shows where it was stretched to.
    fn checker() -> Vec<u8> {
        (0..CHECKER * CHECKER)
            .flat_map(|index| if (index % CHECKER + index / CHECKER).is_multiple_of(2) { [235, 235, 235, 255] } else { [60, 60, 70, 255] })
            .collect()
    }

    /// A screen drawn the way a skin draws one: it asks how large it is, clears, and fills what it
    /// was handed from edge to edge -- with one rectangle that reaches far past every edge, which
    /// only a clip can keep in.
    fn paint_pattern<R: Renderer>(r: &mut R) {
        let (width, height) = r.size();
        let (width, height) = (width as f32, height as f32);
        r.clear(WASH);
        r.fill_rect(Rect::new(-width, height * 0.5 - FRAME_THICKNESS, width * BAND_WIDTHS, FRAME_THICKNESS * 2.0), Color { a: BAND_ALPHA, ..Color::WHITE });
        r.fill_rect(Rect::new(0.0, 0.0, width, FRAME_THICKNESS), FRAME);
        r.fill_rect(Rect::new(0.0, height - FRAME_THICKNESS, width, FRAME_THICKNESS), FRAME);
        r.fill_rect(Rect::new(0.0, 0.0, FRAME_THICKNESS, height), FRAME);
        r.fill_rect(Rect::new(width - FRAME_THICKNESS, 0.0, FRAME_THICKNESS, height), FRAME);

        let bar_width = (width - INSET * 2.0) / BARS.len() as f32;
        let bar_height = height * BARS_SHARE;
        for (index, color) in BARS.iter().enumerate() {
            r.fill_rect(Rect::new(INSET + bar_width * index as f32, INSET, bar_width, bar_height), *color);
        }

        let side = height * SQUARE_SHARE;
        let tex = r.register_texture("skin_preview.region.checker", &checker(), CHECKER, CHECKER);
        r.draw_textured_quad(tex, QuadParams::new(Rect::new(INSET, height - INSET - side, side, side)));
        r.draw_textured_quad(
            tex,
            QuadParams { filter: TextureFilter::Linear, ..QuadParams::new(Rect::new(INSET * 2.0 + side, height - INSET - side, side, side)) },
        );
        r.push_clip(Rect::new(width - INSET - side, height - INSET - side, side, side));
        r.fill_rect(Rect::new(0.0, 0.0, width, height), BARS[4]);
        r.pop_clip();

        draw_text_centered(r, width * 0.5, height * 0.5 - INSET * 2.0, TITLE_SCALE, Color::WHITE, &format!("REGION {width:.0} X {height:.0}"));
        draw_text(r, INSET, height * 0.5 + INSET * 0.5, NOTE_SCALE, FRAME, "DRAWN IN ITS OWN PIXELS - CLIPPED TO ITS OWN EDGES");
    }

    /// A frame of a built-in screen with the pattern in one rectangle of it: the screen's own
    /// background and a line of its own text, then the region, then one more rectangle of the
    /// screen's, which the region must have left unclipped.
    fn paint_frame(canvas: &mut Canvas<'_>) {
        let colors = theme();
        canvas.clear(colors.bg);
        draw_text(canvas, SCREEN_TEXT_AT.0, SCREEN_TEXT_AT.1, TITLE_SCALE, colors.text, "REGION TEST PATTERN");
        with_region(canvas, REGION, |region| paint_pattern(region));
        canvas.fill_rect(SCREEN_BAR, colors.accent);
    }

    /// Save a frame as `<capture folder>/<name>.<extension>`, when a capture folder was named.
    fn save(name: &str, extension: &str, size: (u32, u32), rgba: &[u8]) -> Option<PathBuf> {
        let folder = PathBuf::from(std::env::var_os(CAPTURE_DIR_ENV)?);
        std::fs::create_dir_all(&folder).expect("the capture folder can be created");
        let path = folder.join(format!("{name}.{extension}"));
        image::save_buffer(&path, rgba, size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
        Some(path)
    }

    fn headless_frame(size: (u32, u32)) -> HeadlessCanvas {
        rbms_render::font::use_embedded_fonts_only();
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        paint_frame(&mut Canvas::Headless(&mut pixels));
        pixels
    }

    /// The rectangle the region covers on a target `enlargement` times [`UI_SIZE`], as whole pixels:
    /// `(left, top, right, bottom)` with the far edges exclusive.
    fn region_pixels(enlargement: f32) -> (u32, u32, u32, u32) {
        let edge = |value: f32| (value * enlargement).round() as u32;
        (edge(REGION.x), edge(REGION.y), edge(REGION.x + REGION.w), edge(REGION.y + REGION.h))
    }

    #[test]
    fn a_region_is_handed_the_pixels_its_rectangle_really_covers() {
        for (size, enlargement) in TARGETS {
            let mut pixels = HeadlessCanvas::new(size.0, size.1);
            let mut canvas = Canvas::Headless(&mut pixels);
            let (left, top, right, bottom) = region_pixels(enlargement);
            let (seen, placed) = with_region(&mut canvas, REGION, |region| (region.size(), region.rect()));
            assert_eq!(seen, (right - left, bottom - top), "a target of {size:?}");
            assert_eq!((placed.x, placed.y), (left as f32, top as f32), "a target of {size:?}");
        }
    }

    /// The pattern fills its rectangle to the last pixel and reaches no further, on a target of the
    /// size the screens are laid out for and on a larger one, and the screen round it is drawn as it
    /// would have been without it.
    #[test]
    fn a_screen_drawn_in_a_region_fills_the_rectangle_and_nothing_else() {
        for (size, enlargement) in TARGETS {
            let pixels = headless_frame(size);
            let (left, top, right, bottom) = region_pixels(enlargement);
            let background = theme().bg;
            for (x, y) in [(left, top), (right - 1, top), (left, bottom - 1), (right - 1, bottom - 1)] {
                assert_eq!(pixels.pixel_at(x, y), FRAME, "the corner ({x}, {y}) of the region on a target of {size:?}");
            }
            for (x, y) in
                [(left - 1, top), (left, top - 1), (right, bottom - 1), (right - 1, bottom), (left - 1, (top + bottom) / 2), (right, (top + bottom) / 2)]
            {
                assert_eq!(pixels.pixel_at(x, y), background, "the pixel ({x}, {y}) outside the region on a target of {size:?}");
            }
            let bar = (((SCREEN_BAR.x + SCREEN_BAR.w * 0.5) * enlargement) as u32, ((SCREEN_BAR.y + SCREEN_BAR.h * 0.5) * enlargement) as u32);
            assert_eq!(pixels.pixel_at(bar.0, bar.1), theme().accent, "the screen drew unclipped after the region on a target of {size:?}");
            save(&format!("{PATTERN_SHOT}-{}x{}", size.0, size.1), "png", size, pixels.rgba());
        }
    }

    /// The GPU draws a region as the headless canvas does: the same frame, to within what the two
    /// backends round differently. A machine with no adapter has nothing to compare and passes.
    #[test]
    fn a_region_is_drawn_alike_on_the_gpu_and_on_the_headless_canvas() {
        for size in [UI_SIZE, LARGE] {
            let headless = headless_frame(size);
            let Some(mut gpu) = Gpu::offscreen_if_available(size.0, size.1, UI_SIZE) else {
                return;
            };
            paint_frame(&mut Canvas::Window(&mut gpu));
            let drawn = gpu.capture().expect("an offscreen target reads back");
            save(&format!("{PATTERN_SHOT}-{}x{}", size.0, size.1), "gpu.png", size, &drawn);

            assert_eq!(drawn.len(), headless.rgba().len());
            let apart = drawn.iter().zip(headless.rgba()).filter(|(one, other)| one.abs_diff(**other) > GPU_TOLERANCE).count();
            let allowed = drawn.len() / GPU_EDGE_PIXELS_ONE_IN;
            assert!(apart <= allowed, "{apart} channels of a {size:?} frame differ by more than {GPU_TOLERANCE}, over the {allowed} allowed");
        }
    }
}
