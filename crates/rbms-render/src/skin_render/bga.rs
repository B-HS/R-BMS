//! The `bga` object: the picture a chart plays behind its field.
//!
//! The document only says where the picture goes. What is shown there is the frame's to supply
//! ([`BgaFrame`]), because it changes with the chart's own timeline and not with anything a document
//! declares.

use rbms_skin::dst::SkinRect;

use super::SkinFrame;
use super::draw::Placement;
use crate::{Renderer, TextureFilter, TextureId, UvRect};

/// What the `bga` object shows this frame.
///
/// [`BgaFrame::default`] shows nothing, which leaves the object undrawn.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BgaFrame {
    /// The picture under everything else, already registered with the renderer.
    pub base: Option<TextureId>,
}

impl BgaFrame {
    /// A frame that shows one picture, when there is one.
    pub fn of(base: Option<TextureId>) -> BgaFrame {
        BgaFrame { base }
    }
}

/// The `bga` object, which carries nothing of its own: the document's record names an id and no
/// more.
#[derive(Debug)]
pub(crate) struct BgaBody;

/// Draws the picture the frame supplied, at the place the document put its `bga` object.
pub(crate) fn draw_bga<R: Renderer>(r: &mut R, place: &Placement<'_>, _body: &BgaBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let Some(tex) = frame.data.bga.base else {
        return false;
    };
    let dst = place.viewport.place(rect);
    if dst.w <= 0.0 || dst.h <= 0.0 {
        return false;
    }
    let filter = r.texture_size(tex).map_or(TextureFilter::Linear, |source| crate::background_filter(dst, source));
    r.draw_textured_quad(tex, place.quad(dst, UvRect::FULL, filter));
    true
}
