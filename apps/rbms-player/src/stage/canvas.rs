//! What a screen draws onto: the window, or a headless canvas the render snapshot tests use.
//!
//! The renderer's drawing helpers are generic over [`Renderer`], which rules out a trait object
//! here, so the two targets are an enum instead. Beyond the geometry a screen also needs the
//! single background-image slot and the quad counter the debug overlay reports.

#[cfg(test)]
use rbms_render::CpuCanvas;
use rbms_render::{Color, Rect, Renderer};

use crate::gpu::Gpu;

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
    /// Where the last background image was placed, and how many bytes it had.
    bga: Option<(Rect, usize)>,
    quads: usize,
}

#[cfg(test)]
impl HeadlessCanvas {
    pub(crate) fn new(w: u32, h: u32) -> HeadlessCanvas {
        HeadlessCanvas { pixels: CpuCanvas::new(w, h), bga: None, quads: 0 }
    }

    /// Where the screen put its background image this frame, if it put one.
    pub(crate) fn bga(&self) -> Option<(Rect, usize)> {
        self.bga
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
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.quads += 1;
        self.pixels.fill_rect(rect, color);
    }
}

/// The draw target handed to a screen.
pub(crate) enum Canvas<'a> {
    Window(&'a mut Gpu),
    #[cfg(test)]
    Headless(&'a mut HeadlessCanvas),
}

impl Canvas<'_> {
    /// Put an image behind the screen's quads, at `rect`.
    pub(crate) fn set_bga(&mut self, rgba: &[u8], rect: Rect) {
        match self {
            Canvas::Window(gpu) => gpu.set_bga(rgba, rect),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.bga = Some((rect, rgba.len())),
        }
    }

    /// Drop whatever image is behind the screen's quads.
    pub(crate) fn clear_bga(&mut self) {
        match self {
            Canvas::Window(gpu) => gpu.clear_bga(),
            #[cfg(test)]
            Canvas::Headless(canvas) => canvas.bga = None,
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
}
