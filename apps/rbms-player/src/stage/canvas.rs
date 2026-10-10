//! What a screen draws onto: the window, or a headless canvas the render snapshot tests use.
//!
//! The renderer's drawing helpers are generic over [`Renderer`], which rules out a trait object
//! here, so the two targets are an enum instead. Beyond the geometry a screen also needs the
//! single background-image slot and the quad counter the debug overlay reports.
//!
//! A target is as large as the window is, and there are two ways to address it. [`Canvas`] itself is
//! the fixed [`UI_SIZE`] screen the built-in screens and the system overlays are laid out for: what
//! is drawn on it is scaled out to the target, so those screens keep their place and their size
//! whatever the window does. [`Canvas::native`] is the same target in its own pixels, which is what
//! a skin screen draws on. Both feed one frame, in the order they are drawn.

use std::collections::HashMap;

use rbms_render::skin_render::frame::{BgaExpand, BgaFrame, BgaPick, BgaTextures};
use rbms_render::{Color, DistanceFieldParams, QuadParams, Rect, Renderer, ScaledRenderer, TextureId, scale_between, scale_rect};
#[cfg(test)]
use rbms_render::{CpuCanvas, TextureFilter};

use crate::assets::bga_picture;
use crate::gpu::Gpu;
use crate::{CH, CW, DecodedImage};

/// The size the built-in screens and the system overlays are laid out for, whatever size the target
/// they end up on is.
pub(crate) const UI_SIZE: (u32, u32) = (CW, CH);

/// Registration key the headless target holds its background image under, matching the window
/// target's own key so both refresh one texture rather than accumulating one per frame.
#[cfg(test)]
const BACKGROUND_TEXTURE_KEY: &str = "rbms.player.background";

/// Bytes per pixel in the RGBA8 buffers a background image arrives as.
#[cfg(test)]
const BYTES_PER_PIXEL: usize = rbms_render::BYTES_PER_PIXEL;

/// Grid the headless signature averages the frame over. Coarse enough that antialiasing noise does
/// not move it, fine enough that a screen drawing the wrong thing does.
#[cfg(test)]
const SIGNATURE_COLS: u32 = 16;
#[cfg(test)]
const SIGNATURE_ROWS: u32 = 9;

/// FNV-1a 64-bit offset basis, for the exact per-pixel frame checksum.
#[cfg(test)]
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
#[cfg(test)]
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// A headless draw target: the renderer's CPU canvas plus the background-image slot and quad count
/// the window target also provides. Built only by the render snapshot tests and the capture
/// harness.
#[cfg(test)]
pub(crate) struct HeadlessCanvas {
    pixels: CpuCanvas,
    /// The background texture and where it goes, redrawn by every [`Renderer::clear`] exactly as
    /// the window target redraws it.
    background: Option<(TextureId, Rect)>,
    quads: usize,
}

#[cfg(test)]
impl HeadlessCanvas {
    pub(crate) fn new(w: u32, h: u32) -> HeadlessCanvas {
        HeadlessCanvas { pixels: CpuCanvas::new(w, h), background: None, quads: 0 }
    }

    /// Where the screen put its background image this frame, if it put one, in this canvas's own
    /// pixels.
    pub(crate) fn background(&self) -> Option<Rect> {
        self.background.map(|(_, rect)| rect)
    }

    /// A coarse signature of the rendered pixels, for snapshot comparisons: one averaged colour
    /// per cell of a [`SIGNATURE_COLS`] x [`SIGNATURE_ROWS`] grid.
    pub(crate) fn signature(&self) -> Vec<u8> {
        self.pixels.block_signature(SIGNATURE_COLS, SIGNATURE_ROWS)
    }

    /// How many quads the screen drew, matching what the debug overlay reports for the window.
    pub(crate) fn quad_count(&self) -> usize {
        self.quads
    }

    /// The drawn frame as tightly packed RGBA8 rows, top row first.
    pub(crate) fn rgba(&self) -> &[u8] {
        self.pixels.pixels()
    }

    /// The colour one pixel of the drawn frame ended up.
    pub(crate) fn pixel_at(&self, x: u32, y: u32) -> Color {
        self.pixels.pixel_at(x, y)
    }

    /// How many non-transparent pixels the screen painted, so an empty frame is distinguishable
    /// from a drawn one.
    pub(crate) fn painted_pixels(&self) -> usize {
        self.rgba().chunks_exact(4).filter(|p| p[3] != 0).count()
    }

    /// An FNV-1a checksum over every rendered byte. Finer than [`HeadlessCanvas::signature`], which
    /// averages the frame into blocks: two frames that differ only in a digit are told apart here.
    pub(crate) fn pixel_checksum(&self) -> u64 {
        self.rgba().iter().fold(FNV_OFFSET_BASIS, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME))
    }
}

#[cfg(test)]
impl Renderer for HeadlessCanvas {
    fn size(&self) -> (u32, u32) {
        self.pixels.size()
    }

    fn clear(&mut self, color: Color) {
        self.quads = 0;
        self.pixels.clear(color);
        if let Some((tex, rect)) = self.background {
            let mut params = QuadParams::new(rect);
            params.filter = self.texture_size(tex).map_or(TextureFilter::Linear, |source| rbms_render::background_filter(rect, source));
            self.draw_textured_quad(tex, params);
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.quads += 1;
        self.pixels.fill_rect(rect, color);
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        self.pixels.register_texture(key, rgba, width, height)
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.pixels.release_texture(tex);
        if self.background.is_some_and(|(id, _)| id == tex) {
            self.background = None;
        }
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.pixels.texture_size(tex)
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        self.quads += 1;
        self.pixels.draw_textured_quad(tex, params);
    }

    fn draw_distance_field_quad(&mut self, tex: TextureId, params: DistanceFieldParams) {
        self.quads += 1;
        self.pixels.draw_distance_field_quad(tex, params);
    }

    fn push_clip(&mut self, rect: Rect) {
        self.pixels.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.pixels.pop_clip();
    }
}

/// The draw target handed to a screen, addressed as the fixed [`UI_SIZE`] screen.
pub(crate) enum Canvas<'a> {
    Window(&'a mut Gpu),
    #[cfg(test)]
    Headless(&'a mut HeadlessCanvas),
}

/// A [`Canvas`]'s target addressed in its own pixels, with nothing scaled on the way.
///
/// Its size is the viewport a frame lands in -- the window, less any letterbox bars -- so a skin
/// laid out for that size puts each coordinate on the pixel it names.
pub(crate) struct NativeCanvas<'c, 'a>(&'c mut Canvas<'a>);

impl<'a> Canvas<'a> {
    /// The same target in its own pixels. See [`NativeCanvas`].
    pub(crate) fn native(&mut self) -> NativeCanvas<'_, 'a> {
        NativeCanvas(self)
    }

    /// The target's size in its own pixels.
    fn native_size(&self) -> (u32, u32) {
        match self {
            Canvas::Window(gpu) => gpu.size(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.size(),
        }
    }

    /// Where a rectangle given in [`UI_SIZE`] coordinates lands on the target.
    fn to_native(&self, rect: Rect) -> Rect {
        scale_rect(rect, scale_between(UI_SIZE, self.native_size()))
    }

    /// Run `draw` on this target scaled out from [`UI_SIZE`].
    fn scaled<T>(&mut self, draw: impl FnOnce(&mut ScaledRenderer<'_, NativeCanvas<'_, 'a>>) -> T) -> T {
        draw(&mut ScaledRenderer::new(&mut self.native(), UI_SIZE))
    }

    /// Upload `width` x `height` RGBA8 pixels as the background image and hand back its handle.
    ///
    /// `generation` names the decode they came from, so a screen that hands the same picture over
    /// on every frame pays for one upload rather than sixty a second. The handle is what a skin
    /// document's own `bga` object is drawn from; the built-in layout goes through
    /// [`Canvas::set_background`] instead, which also records where the target should put it.
    pub(crate) fn background_texture(&mut self, generation: u64, rgba: &[u8], width: u32, height: u32) -> Option<TextureId> {
        match self {
            Canvas::Window(gpu) => gpu.background_texture(generation, rgba, width, height),
            #[cfg(test)]
            Canvas::Headless(canvas) => {
                if rgba.len() != (width as usize) * (height as usize) * BYTES_PER_PIXEL {
                    return None;
                }
                Some(canvas.register_texture(BACKGROUND_TEXTURE_KEY, rgba, width, height))
            }
        }
    }

    /// Put `width` x `height` RGBA8 pixels behind the screen's quads, at `rect` in [`UI_SIZE`]
    /// coordinates.
    ///
    /// The image is an ordinary registered texture drawn as an ordinary quad: the target holds it
    /// and redraws it from [`Renderer::clear`], which is the one moment that sits after the screen
    /// wipes the frame and before it queues anything of its own. The target keeps the rectangle in
    /// its own pixels, so whether the image lands at exactly its own size -- and so whether it is
    /// point sampled -- is decided where it is actually drawn.
    pub(crate) fn set_background(&mut self, generation: u64, rgba: &[u8], width: u32, height: u32, rect: Rect) {
        let rect = self.to_native(rect);
        match self {
            Canvas::Window(gpu) => gpu.set_background(generation, rgba, width, height, rect),
            #[cfg(test)]
            Canvas::Headless(canvas) => {
                if rgba.len() == (width as usize) * (height as usize) * BYTES_PER_PIXEL {
                    let tex = canvas.register_texture(BACKGROUND_TEXTURE_KEY, rgba, width, height);
                    canvas.background = Some((tex, rect));
                }
            }
        }
    }

    /// The frame a skin document's `bga` object draws from: the pictures `pick` names, uploaded
    /// through `textures` only when they are not the ones it already holds.
    ///
    /// Unlike [`Canvas::background_texture`] this keeps the chart's picture, the layer over it and
    /// the miss layer apart, each in a texture of its own, and shapes the pixels the way the
    /// reference does ([`BgaTextures`]). A picture number the chart has no image for shows as none.
    pub(crate) fn bga_frame(&mut self, textures: &mut BgaTextures, pick: BgaPick, expand: BgaExpand, pictures: &HashMap<i32, DecodedImage>) -> BgaFrame {
        textures.frame(&mut self.native(), pick, expand, |number| pictures.get(&number).map(bga_picture))
    }

    /// Hands the textures [`Canvas::bga_frame`] made back to the target, for a screen that is done
    /// with its background.
    pub(crate) fn release_bga_textures(&mut self, textures: &mut BgaTextures) {
        textures.release(&mut self.native());
    }

    /// Drop whatever image is behind the screen's quads.
    pub(crate) fn clear_bga(&mut self) {
        match self {
            Canvas::Window(gpu) => gpu.clear_background(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.background = None,
        }
    }

    /// How many quads the screen has drawn, as reported by the debug overlay.
    pub(crate) fn quad_count(&self) -> usize {
        match self {
            Canvas::Window(gpu) => gpu.quad_count(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.quads,
        }
    }
}

impl Renderer for NativeCanvas<'_, '_> {
    fn size(&self) -> (u32, u32) {
        self.0.native_size()
    }

    fn clear(&mut self, color: Color) {
        match &mut *self.0 {
            Canvas::Window(gpu) => gpu.clear(color),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.clear(color),
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        match &mut *self.0 {
            Canvas::Window(gpu) => gpu.fill_rect(rect, color),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.fill_rect(rect, color),
        }
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        match &mut *self.0 {
            Canvas::Window(gpu) => gpu.register_texture(key, rgba, width, height),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.register_texture(key, rgba, width, height),
        }
    }

    fn release_texture(&mut self, tex: TextureId) {
        match &mut *self.0 {
            Canvas::Window(gpu) => gpu.release_texture(tex),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.release_texture(tex),
        }
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        match &*self.0 {
            Canvas::Window(gpu) => gpu.texture_size(tex),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.texture_size(tex),
        }
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        match &mut *self.0 {
            Canvas::Window(gpu) => gpu.draw_textured_quad(tex, params),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.draw_textured_quad(tex, params),
        }
    }

    fn draw_distance_field_quad(&mut self, tex: TextureId, params: DistanceFieldParams) {
        match &mut *self.0 {
            Canvas::Window(gpu) => gpu.draw_distance_field_quad(tex, params),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.draw_distance_field_quad(tex, params),
        }
    }

    fn push_clip(&mut self, rect: Rect) {
        match &mut *self.0 {
            Canvas::Window(gpu) => gpu.push_clip(rect),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.push_clip(rect),
        }
    }

    fn pop_clip(&mut self) {
        match &mut *self.0 {
            Canvas::Window(gpu) => gpu.pop_clip(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.pop_clip(),
        }
    }

    fn max_texture_size(&self) -> u32 {
        match &*self.0 {
            Canvas::Window(gpu) => gpu.max_texture_size(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.max_texture_size(),
        }
    }
}

impl Renderer for Canvas<'_> {
    fn size(&self) -> (u32, u32) {
        UI_SIZE
    }

    fn clear(&mut self, color: Color) {
        self.native().clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.scaled(|ui| ui.fill_rect(rect, color));
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        self.native().register_texture(key, rgba, width, height)
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.native().release_texture(tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        match self {
            Canvas::Window(gpu) => gpu.texture_size(tex),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.texture_size(tex),
        }
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        self.scaled(|ui| ui.draw_textured_quad(tex, params));
    }

    fn draw_distance_field_quad(&mut self, tex: TextureId, params: DistanceFieldParams) {
        self.scaled(|ui| ui.draw_distance_field_quad(tex, params));
    }

    fn push_clip(&mut self, rect: Rect) {
        self.scaled(|ui| ui.push_clip(rect));
    }

    fn pop_clip(&mut self) {
        self.native().pop_clip();
    }

    fn max_texture_size(&self) -> u32 {
        match self {
            Canvas::Window(gpu) => gpu.max_texture_size(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.max_texture_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rbms_render::skin_render::frame::BgaShow;

    use super::*;

    /// How many times larger than [`UI_SIZE`] the enlarged target is on each axis.
    const ENLARGEMENT: u32 = 2;

    const INK: Color = Color::rgb(200, 100, 50);

    /// A rectangle in [`UI_SIZE`] coordinates, well inside the screen.
    const PATCH: Rect = Rect { x: 100.0, y: 50.0, w: 40.0, h: 20.0 };

    /// The one opaque pixel the background tests upload.
    const DOT: [u8; BYTES_PER_PIXEL] = [9, 8, 7, 255];

    /// The side of the square pictures the chart's-pictures test hands over, past the 256 pixels that
    /// would put one on a canvas of its own.
    const BGA_SIDE: u32 = 300;

    fn enlarged() -> HeadlessCanvas {
        HeadlessCanvas::new(UI_SIZE.0 * ENLARGEMENT, UI_SIZE.1 * ENLARGEMENT)
    }

    /// The contract every built-in screen leans on: at the size they are laid out for, drawing
    /// through the canvas is drawing on the target, to the last bit.
    #[test]
    fn a_target_of_the_ui_size_is_drawn_on_unscaled() {
        let mut through_canvas = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
        let mut canvas = Canvas::Headless(&mut through_canvas);
        canvas.clear(Color::BLACK);
        canvas.fill_rect(Rect::new(100.5, 50.25, 40.75, 20.5), INK);
        canvas.push_clip(Rect::new(10.0, 10.0, 300.5, 300.0));
        canvas.fill_rect(Rect::new(0.0, 0.0, 1280.0, 720.0), Color { a: 77, ..Color::GREEN });
        canvas.pop_clip();

        let mut direct = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
        direct.clear(Color::BLACK);
        direct.fill_rect(Rect::new(100.5, 50.25, 40.75, 20.5), INK);
        direct.push_clip(Rect::new(10.0, 10.0, 300.5, 300.0));
        direct.fill_rect(Rect::new(0.0, 0.0, 1280.0, 720.0), Color { a: 77, ..Color::GREEN });
        direct.pop_clip();

        assert_eq!(through_canvas.pixel_checksum(), direct.pixel_checksum());
    }

    #[test]
    fn a_larger_target_still_reads_as_the_ui_size_and_is_filled_to_scale() {
        let mut pixels = enlarged();
        let mut canvas = Canvas::Headless(&mut pixels);
        assert_eq!(canvas.size(), UI_SIZE, "a built-in screen is laid out for the same screen whatever the window is");
        canvas.clear(Color::BLACK);
        canvas.fill_rect(PATCH, INK);

        let (x0, y0) = (PATCH.x as u32 * ENLARGEMENT, PATCH.y as u32 * ENLARGEMENT);
        let (x1, y1) = ((PATCH.x + PATCH.w) as u32 * ENLARGEMENT, (PATCH.y + PATCH.h) as u32 * ENLARGEMENT);
        assert_eq!(pixels.pixel_at(x0, y0), INK, "the patch starts twice as far in");
        assert_eq!(pixels.pixel_at(x1 - 1, y1 - 1), INK, "and reaches twice as far");
        assert_eq!(pixels.pixel_at(x1, y1 - 1), Color::BLACK);
        assert_eq!(pixels.pixel_at(x0 - 1, y0), Color::BLACK);
    }

    /// What a skin screen draws on: the same target, with a coordinate naming the pixel it lands on.
    #[test]
    fn the_native_view_is_the_target_in_its_own_pixels() {
        let mut pixels = enlarged();
        let mut canvas = Canvas::Headless(&mut pixels);
        canvas.clear(Color::BLACK);
        let mut native = canvas.native();
        assert_eq!(native.size(), (UI_SIZE.0 * ENLARGEMENT, UI_SIZE.1 * ENLARGEMENT));
        native.fill_rect(PATCH, INK);
        canvas.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::GREEN);

        assert_eq!(pixels.pixel_at(PATCH.x as u32, PATCH.y as u32), INK, "nothing was scaled on the way");
        assert_eq!(pixels.pixel_at((PATCH.x + PATCH.w) as u32, PATCH.y as u32), Color::BLACK);
        assert_eq!(pixels.pixel_at(ENLARGEMENT - 1, ENLARGEMENT - 1), Color::GREEN, "and a scaled draw shares the frame with it");
        assert_eq!(pixels.quad_count(), 2, "both ways of drawing count toward the one frame");
    }

    /// The background slot is handed a rectangle in the screen's coordinates and keeps it in the
    /// target's, so it is drawn where the screen asked however large the target is.
    #[test]
    fn the_background_slot_is_placed_in_the_targets_own_pixels() {
        let mut pixels = enlarged();
        let mut canvas = Canvas::Headless(&mut pixels);
        canvas.set_background(0, &DOT, 1, 1, PATCH);
        canvas.clear(Color::BLACK);

        let scale = ENLARGEMENT as f32;
        assert_eq!(pixels.background(), Some(Rect::new(PATCH.x * scale, PATCH.y * scale, PATCH.w * scale, PATCH.h * scale)));
        let inside = pixels.pixel_at((PATCH.x + PATCH.w) as u32 * ENLARGEMENT - 1, (PATCH.y + PATCH.h) as u32 * ENLARGEMENT - 1);
        assert_eq!(inside, Color { r: DOT[0], g: DOT[1], b: DOT[2], a: DOT[3] });
    }

    /// A chart's pictures reach the target through three textures of their own, a picture that stays
    /// is uploaded once however many frames carry it, a number the chart has no image for is no
    /// picture, and a screen that is done hands the textures back.
    #[test]
    fn a_charts_pictures_reach_the_target_one_texture_each_and_are_uploaded_once() {
        let picture = |shade: u8| DecodedImage::for_test((0..BGA_SIDE * BGA_SIDE).flat_map(|_| [shade, 0, 0, 255]).collect(), BGA_SIDE, BGA_SIDE);
        let pictures = HashMap::from([(0, picture(10)), (1, picture(20))]);
        let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
        let mut canvas = Canvas::Headless(&mut pixels);
        let mut textures = BgaTextures::default();
        let playing = |base, layer| BgaPick::Playing { base, layer };

        let first = canvas.bga_frame(&mut textures, playing(Some(0), Some(1)), BgaExpand::default(), &pictures);
        let again = canvas.bga_frame(&mut textures, playing(Some(0), Some(1)), BgaExpand::default(), &pictures);
        assert_eq!(first, again, "the same two pictures asked for twice came back as different textures");
        let BgaShow::Playing { base: Some(base), layer: Some(layer) } = first.show else {
            panic!("a chart with both pictures showed {:?}", first.show);
        };
        assert_ne!(base, layer, "the picture and the layer share a texture");
        assert_eq!(canvas.native().texture_size(base), Some((BGA_SIDE, BGA_SIDE)));

        let lost = canvas.bga_frame(&mut textures, playing(Some(9), None), BgaExpand::default(), &pictures);
        assert_eq!(lost.show, BgaShow::Playing { base: None, layer: None }, "a number with no image showed a picture");

        canvas.release_bga_textures(&mut textures);
        assert_eq!(canvas.native().texture_size(base), None, "the textures were not handed back");
    }

    #[test]
    fn the_headless_target_has_no_texture_ceiling() {
        let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
        let mut canvas = Canvas::Headless(&mut pixels);
        assert_eq!(canvas.max_texture_size(), rbms_render::UNLIMITED_TEXTURE_SIZE);
        assert_eq!(canvas.native().max_texture_size(), rbms_render::UNLIMITED_TEXTURE_SIZE);
    }
}
