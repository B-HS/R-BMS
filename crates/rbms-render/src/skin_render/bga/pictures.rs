//! Putting a chart's decoded background pictures onto the renderer the way the reference draws them.
//!
//! The reference does three things to a picture between the file and the screen, and each is a
//! property of the pixels rather than of where they land, so they are done here, once, when a
//! picture is uploaded:
//!
//! - A picture no longer than 256 pixels on its longer side is put on a 256 x 256 canvas of
//!   transparent black, centred across and flush with the top (`BGImageProcessor.convert`). Fitted
//!   into a rectangle afterwards, it keeps that square's shape and the empty space around it.
//! - A layer is drawn through a shader that turns every pixel whose red, green and blue are all
//!   zero into a transparent one (`layer.frag`). Doing that to the texels is the same thing as long
//!   as the layer is sampled unfiltered, which it is.
//! - The chart's own picture and the miss layer are drawn as they are.
//!
//! A layer and a picture of the same number are different textures for that reason, so each of the
//! three roles keeps a texture of its own and hands the renderer new pixels only when the picture it
//! is asked for changes.

use super::{BgaExpand, BgaFrame, BgaPick, BgaShow};
use crate::{Renderer, TextureId};

/// The longest side, in pixels, of a picture that is put on a canvas of its own
/// (`BGImageProcessor`'s `bgasize <= 256`), and the canvas's side.
pub const SMALL_PICTURE_EDGE: u32 = 256;

/// The registry key of the chart's own picture.
const BASE_KEY: &str = "rbms.skin.bga.base";

/// The registry key of the layer over it.
const LAYER_KEY: &str = "rbms.skin.bga.layer";

/// The registry key of the miss layer.
const MISS_KEY: &str = "rbms.skin.bga.miss";

/// Bytes in a pixel of RGBA8.
const RGBA_BYTES: usize = 4;

/// The byte of a pixel that is its alpha.
const ALPHA_INDEX: usize = 3;

/// One decoded picture as the game holds it.
#[derive(Debug, Clone, Copy)]
pub struct BgaPicture<'a> {
    /// This decode's own number: two pictures with the same one are the same pixels, which is what
    /// lets a picture that is shown for many frames be uploaded once.
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    /// `width * height` pixels of RGBA8, not premultiplied.
    pub rgba: &'a [u8],
}

impl BgaPicture<'_> {
    /// Whether the pixels are as many as the size says.
    fn is_whole(&self) -> bool {
        self.width > 0 && self.height > 0 && self.rgba.len() == self.width as usize * self.height as usize * RGBA_BYTES
    }
}

/// The picture on its 256 x 256 canvas when it is small enough to be put on one, as the pixels and
/// the canvas's size; `None` for a picture that is drawn as it is (`BGImageProcessor.convert`).
///
/// The picture lands `(256 - width) / 2` pixels across, rounded down, and at the top. Everything
/// the picture does not cover is transparent black.
pub fn on_small_canvas(picture: &BgaPicture<'_>) -> Option<(Vec<u8>, (u32, u32))> {
    if !picture.is_whole() || picture.width.max(picture.height) > SMALL_PICTURE_EDGE {
        return None;
    }
    let side = SMALL_PICTURE_EDGE as usize;
    let (width, height) = (picture.width as usize, picture.height as usize);
    let across = (side - width) / 2;
    let mut canvas = vec![0; side * side * RGBA_BYTES];
    for row in 0..height {
        let from = row * width * RGBA_BYTES;
        let to = (row * side + across) * RGBA_BYTES;
        canvas[to..to + width * RGBA_BYTES].copy_from_slice(&picture.rgba[from..from + width * RGBA_BYTES]);
    }
    Some((canvas, (SMALL_PICTURE_EDGE, SMALL_PICTURE_EDGE)))
}

/// Turns every pixel whose red, green and blue are all zero fully transparent (`layer.frag`),
/// whatever alpha it had. The colour stays zero.
pub fn key_out_black(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(RGBA_BYTES) {
        if pixel[..ALPHA_INDEX].iter().all(|channel| *channel == 0) {
            pixel[ALPHA_INDEX] = 0;
        }
    }
}

/// What a texture slot holds: whose pixels, at what size, under which handle.
#[derive(Debug, Clone, Copy)]
struct Held {
    generation: u64,
    size: (u32, u32),
    tex: TextureId,
}

/// The texture of one role, uploaded again only when the picture it shows changes.
#[derive(Debug, Default)]
struct Slot {
    held: Option<Held>,
}

impl Slot {
    /// The texture that shows `picture`, uploading it first unless it is the one already there.
    /// `None` when there is no picture or the renderer cannot hold it.
    fn show<R: Renderer>(&mut self, r: &mut R, key: &str, picture: Option<BgaPicture<'_>>, layer: bool) -> Option<TextureId> {
        let picture = picture.filter(BgaPicture::is_whole)?;
        let size = (picture.width, picture.height);
        if let Some(held) = self.held.filter(|held| held.generation == picture.generation && held.size == size && r.texture_size(held.tex).is_some()) {
            return Some(held.tex);
        }
        let (mut rgba, (width, height)) = on_small_canvas(&picture).unwrap_or_else(|| (picture.rgba.to_vec(), size));
        if layer {
            key_out_black(&mut rgba);
        }
        let tex = r.register_texture(key, &rgba, width, height);
        self.held = Some(Held { generation: picture.generation, size, tex });
        r.texture_size(tex).map(|_| tex)
    }

    fn release<R: Renderer>(&mut self, r: &mut R) {
        if let Some(held) = self.held.take() {
            r.release_texture(held.tex);
        }
    }
}

/// The three textures a chart's background draws from: its own picture, the layer over it, and the
/// miss layer.
#[derive(Debug, Default)]
pub struct BgaTextures {
    base: Slot,
    layer: Slot,
    miss: Slot,
}

impl BgaTextures {
    /// The frame that shows what `pick` says. `picture` finds the decoded picture of a picture
    /// number; a number it has none for shows as a picture the chart does not have.
    pub fn frame<'a, R: Renderer>(&mut self, r: &mut R, pick: BgaPick, expand: BgaExpand, picture: impl Fn(i32) -> Option<BgaPicture<'a>>) -> BgaFrame {
        let show = match pick {
            BgaPick::Blank => BgaShow::Blank,
            BgaPick::Miss(number) => BgaShow::Miss { image: self.miss.show(r, MISS_KEY, number.and_then(&picture), false) },
            BgaPick::Playing { base, layer } => BgaShow::Playing {
                base: self.base.show(r, BASE_KEY, base.and_then(&picture), false),
                layer: self.layer.show(r, LAYER_KEY, layer.and_then(&picture), true),
            },
        };
        BgaFrame { show, expand }
    }

    /// Hands every texture back to the renderer, for a screen that is done with its background.
    pub fn release<R: Renderer>(&mut self, r: &mut R) {
        self.base.release(r);
        self.layer.release(r);
        self.miss.release(r);
    }
}
