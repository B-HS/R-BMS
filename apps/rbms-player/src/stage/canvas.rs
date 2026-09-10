//! What a screen draws onto: the window, or a headless canvas the render snapshot tests use.
//!
//! The renderer's drawing helpers are generic over [`Renderer`], which rules out a trait object
//! here, so the two targets are an enum instead. Beyond the geometry a screen also needs the
//! single background-image slot and the quad counter the debug overlay reports.

use rbms_render::{Color, QuadParams, Rect, Renderer, TextureId};
#[cfg(test)]
use rbms_render::{CpuCanvas, TextureFilter};

use crate::gpu::Gpu;

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
/// the window target also provides. Built only by the render snapshot tests.
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

    /// Where the screen put its background image this frame, if it put one.
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

    /// The colour one pixel of the drawn frame ended up.
    pub(crate) fn pixel_at(&self, x: u32, y: u32) -> Color {
        self.pixels.pixel_at(x, y)
    }

    /// How many non-transparent pixels the screen painted, so an empty frame is distinguishable
    /// from a drawn one.
    pub(crate) fn painted_pixels(&self) -> usize {
        self.pixels.pixels().chunks_exact(4).filter(|p| p[3] != 0).count()
    }

    /// An FNV-1a checksum over every rendered byte. Finer than [`HeadlessCanvas::signature`], which
    /// averages the frame into blocks: two frames that differ only in a digit are told apart here.
    pub(crate) fn pixel_checksum(&self) -> u64 {
        self.pixels.pixels().iter().fold(FNV_OFFSET_BASIS, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME))
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

    fn push_clip(&mut self, rect: Rect) {
        self.pixels.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.pixels.pop_clip();
    }
}

/// The draw target handed to a screen.
pub(crate) enum Canvas<'a> {
    Window(&'a mut Gpu),
    #[cfg(test)]
    Headless(&'a mut HeadlessCanvas),
}

impl Canvas<'_> {
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

    /// Put `width` x `height` RGBA8 pixels behind the screen's quads, at `rect`.
    ///
    /// The image is an ordinary registered texture drawn as an ordinary quad: the target holds it
    /// and redraws it from [`Renderer::clear`], which is the one moment that sits after the screen
    /// wipes the frame and before it queues anything of its own.
    pub(crate) fn set_background(&mut self, generation: u64, rgba: &[u8], width: u32, height: u32, rect: Rect) {
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

impl Renderer for Canvas<'_> {
    fn size(&self) -> (u32, u32) {
        match self {
            Canvas::Window(gpu) => gpu.size(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.size(),
        }
    }

    fn clear(&mut self, color: Color) {
        match self {
            Canvas::Window(gpu) => gpu.clear(color),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.clear(color),
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        match self {
            Canvas::Window(gpu) => gpu.fill_rect(rect, color),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.fill_rect(rect, color),
        }
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        match self {
            Canvas::Window(gpu) => gpu.register_texture(key, rgba, width, height),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.register_texture(key, rgba, width, height),
        }
    }

    fn release_texture(&mut self, tex: TextureId) {
        match self {
            Canvas::Window(gpu) => gpu.release_texture(tex),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.release_texture(tex),
        }
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        match self {
            Canvas::Window(gpu) => gpu.texture_size(tex),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.texture_size(tex),
        }
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        match self {
            Canvas::Window(gpu) => gpu.draw_textured_quad(tex, params),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.draw_textured_quad(tex, params),
        }
    }

    fn push_clip(&mut self, rect: Rect) {
        match self {
            Canvas::Window(gpu) => gpu.push_clip(rect),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.push_clip(rect),
        }
    }

    fn pop_clip(&mut self) {
        match self {
            Canvas::Window(gpu) => gpu.pop_clip(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.pop_clip(),
        }
    }
}
