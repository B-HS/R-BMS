//! The one background image a frame draws behind everything else.
//!
//! It is an ordinary registered texture drawn as an ordinary quad, and the moment it is drawn at is
//! [`Renderer::clear`]: after the frame is wiped, before the screen queues anything of its own. What
//! makes it worth a module of its own is that a screen offers the same picture on every frame while
//! the picture itself changes every few hundred milliseconds, so recognising the frame already
//! uploaded is the difference between a background that costs nothing and one that copies megabytes
//! sixty times a second.

use rbms_render::{QuadParams, Rect, Renderer, TextureFilter, TextureId};

use super::{Gpu, batch};

/// Registration key the one background image is uploaded under, so refreshing it replaces the
/// pixels already on the card rather than registering a texture per frame.
pub(crate) const BACKGROUND_TEXTURE_KEY: &str = "rbms.player.background";

/// Whether the background pixels have to go to the GPU again.
///
/// `held` is the decode and size already uploaded. Same decode at the same size means the texture
/// on the card is already the picture being offered, and the frame costs nothing; anything else is
/// a new upload. Split out so the rule can be tested without an adapter.
pub(crate) fn background_upload_needed(held: Option<(u64, (u32, u32))>, generation: u64, size: (u32, u32)) -> bool {
    held != Some((generation, size))
}

impl Gpu {
    /// The background quad this frame draws, if any: the handle and the parameters that put it
    /// where the screen asked for it, sampled the way both background paths agree on.
    pub(crate) fn background_quad(&self) -> Option<(TextureId, QuadParams)> {
        let (tex, rect) = self.background?;
        let mut params = QuadParams::new(rect);
        params.filter = self.texture_size(tex).map_or(TextureFilter::Linear, |source| rbms_render::background_filter(rect, source));
        Some((tex, params))
    }

    /// Upload `width` x `height` RGBA8 pixels as the background image and hand back its handle.
    ///
    /// The image goes through the ordinary texture registry under one stable key, so refreshing it
    /// replaces the pixels already uploaded rather than registering a texture per frame.
    /// `generation` names the decode the pixels came from: a caller that hands the same frame over
    /// again -- which is what a screen does on every one of the sixty frames a background picture
    /// lasts -- costs nothing at all, rather than copying and re-uploading megabytes. Pixels that
    /// do not fill the stated size are ignored.
    pub(crate) fn background_texture(&mut self, generation: u64, rgba: &[u8], width: u32, height: u32) -> Option<TextureId> {
        if rgba.len() != (width as usize) * (height as usize) * batch::BYTES_PER_PIXEL as usize {
            return None;
        }
        let held = self.background_texture.and_then(|tex| Some((self.background_generation?, self.texture_size(tex)?)));
        if let Some(tex) = self.background_texture.filter(|_| !background_upload_needed(held, generation, (width, height))) {
            return Some(tex);
        }
        let tex = self.register_texture(BACKGROUND_TEXTURE_KEY, rgba, width, height);
        self.background_generation = Some(generation);
        self.background_texture = Some(tex);
        Some(tex)
    }

    /// Put the background image behind this frame's quads, at `rect`.
    pub(crate) fn set_background(&mut self, generation: u64, rgba: &[u8], width: u32, height: u32, rect: Rect) {
        if let Some(tex) = self.background_texture(generation, rgba, width, height) {
            self.background = Some((tex, rect));
        }
    }

    /// Drop whatever image is behind this frame's quads.
    pub(crate) fn clear_background(&mut self) {
        self.background = None;
    }
}
