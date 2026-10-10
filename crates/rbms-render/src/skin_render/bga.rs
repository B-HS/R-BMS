//! The `bga` object: the picture a chart plays behind its field (`play/SkinBGA`,
//! `play/bga/BGAProcessor.drawBGA`).
//!
//! The document only says where the picture goes. What is shown there is the frame's to supply
//! ([`BgaFrame`]), because it changes with the chart's own timeline and not with anything a document
//! declares. Which picture that is, at a given moment, is [`BgaPlayhead`]'s to say, and getting a
//! decoded picture onto the renderer in the shape the reference draws it is [`BgaTextures`]'.
//!
//! What the object draws, in the order the reference draws it:
//!
//! 1. Before the chart plays, and with no timeline to play, one black pixel over the whole
//!    rectangle, with nothing fitted.
//! 2. While the miss layer is showing, the miss layer's picture alone: neither the black nor the
//!    chart's own picture.
//! 3. Otherwise the chart's picture, or the black pixel over the whole rectangle when there is
//!    none, and the layer picture over it with every pure black pixel turned transparent.
//!
//! Every picture is fitted by the destination's `stretch` when it names one, and by the player's
//! `bgaExpand` when it does not. The object is never turned: the reference draws it without the
//! destination's angle. A document may place the same `bga` as many times as it likes, each with
//! its own rectangle, colour, blend and stretch, and every placement draws what the frame carries.
//!
//! Dropped by this build: the video a chart's `#BGA` may name (a movie file is skipped like a
//! missing picture), the practice screen's stand-in drawn in the rectangle in practice mode (the
//! frame leaves the object undrawn there), and the reference's texture-filter stickiness (a layer is
//! sampled unfiltered, as a texture nobody has drawn linearly is).

mod pictures;
mod playhead;
#[cfg(test)]
mod tests;

use rbms_skin::dst::SkinRect;
use rbms_skin::loader::{StretchKind, stretch_rect};

use super::SkinFrame;
use super::draw::Placement;
use crate::{QuadParams, Rect, Renderer, TextureFilter, TextureId, UvRect};

pub use pictures::{BgaPicture, BgaTextures, SMALL_PICTURE_EDGE, key_out_black, on_small_canvas};
pub use playhead::{BgaEvent, BgaPick, BgaPlayhead, DEFAULT_MISS_LAYER_DURATION_MS, MISS_LAYER_NONE};

/// The registry key of the black pixel the object fills with.
const BLACK_KEY: &str = "rbms.skin.bga.black";

/// One opaque black pixel, as RGBA8.
const BLACK_PIXEL: [u8; 4] = [0, 0, 0, u8::MAX];

/// How many pixels wide and tall the black one is.
const BLACK_SIZE: u32 = 1;

/// How a picture that does not fit its rectangle is fitted when the destination names no stretch of
/// its own (`Config.bgaExpand`, which `SkinBGA` turns into a stretch type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BgaExpand {
    /// `BGAEXPAND_FULL`: fill the rectangle whatever shape the picture is (`STRETCH`).
    Full,
    /// `BGAEXPAND_KEEP_ASPECT_RATIO`, the reference's default: keep the picture's shape and fit it
    /// inside the rectangle, centred (`KEEP_ASPECT_RATIO_FIT_INNER`).
    #[default]
    KeepAspectRatio,
    /// `BGAEXPAND_OFF`: keep the shape, never enlarge, and centre (`KEEP_ASPECT_RATIO_NO_EXPANDING`).
    Off,
}

impl BgaExpand {
    /// The mode `Config.bgaExpand` names. The reference clamps the setting to 0..=2 on load and
    /// `SkinBGA` stretches for anything else, so a value outside them fills the rectangle.
    pub const fn from_config(value: i32) -> BgaExpand {
        match value {
            1 => BgaExpand::KeepAspectRatio,
            2 => BgaExpand::Off,
            _ => BgaExpand::Full,
        }
    }

    /// The stretch a destination that names none fits its picture with.
    pub const fn stretch(self) -> StretchKind {
        match self {
            BgaExpand::Full => StretchKind::Stretch,
            BgaExpand::KeepAspectRatio => StretchKind::FitInner,
            BgaExpand::Off => StretchKind::NoExpanding,
        }
    }
}

/// What a frame has the `bga` object show.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BgaShow {
    /// Nothing: the object is not drawn. What a frame that has no chart behind it carries, and what
    /// a practice screen carries.
    #[default]
    Nothing,
    /// The chart has not started playing: the rectangle is filled black.
    Blank,
    /// The chart is playing: its own picture, or black where it has none, and the layer over it.
    Playing {
        /// The chart's own picture, already registered with the renderer.
        base: Option<TextureId>,
        /// The layer over it, already registered with its black keyed out ([`key_out_black`]).
        layer: Option<TextureId>,
    },
    /// The miss layer is showing: its picture on its own, or nothing at all when it names none.
    Miss {
        /// The miss layer's picture, already registered with the renderer.
        image: Option<TextureId>,
    },
}

/// What the `bga` object shows this frame, and how it fits a picture to a rectangle that names no
/// stretch of its own.
///
/// [`BgaFrame::default`] shows nothing, which leaves the object undrawn.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BgaFrame {
    pub show: BgaShow,
    pub expand: BgaExpand,
}

impl BgaFrame {
    /// A frame that shows one picture and no layer, filling the rectangle: the shape every play
    /// screen supplied before the layers had a place of their own. A frame with no picture shows
    /// nothing.
    pub fn of(base: Option<TextureId>) -> BgaFrame {
        match base {
            Some(base) => BgaFrame { show: BgaShow::Playing { base: Some(base), layer: None }, expand: BgaExpand::Full },
            None => BgaFrame::default(),
        }
    }

    /// The frame of a chart that is playing.
    pub fn playing(base: Option<TextureId>, layer: Option<TextureId>, expand: BgaExpand) -> BgaFrame {
        BgaFrame { show: BgaShow::Playing { base, layer }, expand }
    }

    /// The frame of a chart that has not started: black.
    pub fn blank(expand: BgaExpand) -> BgaFrame {
        BgaFrame { show: BgaShow::Blank, expand }
    }

    /// The frame of a miss layer that is showing.
    pub fn miss(image: Option<TextureId>, expand: BgaExpand) -> BgaFrame {
        BgaFrame { show: BgaShow::Miss { image }, expand }
    }
}

/// The `bga` object, which carries nothing of its own: the document's record names an id and no
/// more.
#[derive(Debug)]
pub(crate) struct BgaBody;

/// Draws what the frame supplied, at the place the document put its `bga` object, and answers
/// whether anything reached the screen.
pub(crate) fn draw_bga<R: Renderer>(r: &mut R, place: &Placement<'_>, _body: &BgaBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let bga = frame.data.bga;
    let area = place.viewport.place(rect);
    let stretch = if place.object.track.stretch >= 0 { place.object.stretch } else { bga.expand.stretch() };
    match bga.show {
        BgaShow::Nothing => false,
        BgaShow::Blank => fill_black(r, place, area),
        BgaShow::Miss { image } => image.is_some_and(|tex| draw_picture(r, place, tex, area, stretch, TextureFilter::Linear)),
        BgaShow::Playing { base, layer } => {
            let filled = match base {
                Some(tex) => draw_picture(r, place, tex, area, stretch, TextureFilter::Linear),
                None => fill_black(r, place, area),
            };
            let covered = layer.is_some_and(|tex| draw_picture(r, place, tex, area, stretch, TextureFilter::Nearest));
            filled | covered
        }
    }
}

/// Whether a size is one a quad can be drawn at: a number, and not nothing. A negative one is drawn
/// mirrored.
fn has_extent(size: f32) -> bool {
    size.is_finite() && size != 0.0
}

/// The object's quad over `dst`, with the destination's angle dropped: the reference's `SkinBGA`
/// draws through the renderer's plain draw, which has no rotation.
fn upright(place: &Placement<'_>, dst: Rect, src: UvRect, filter: TextureFilter) -> QuadParams {
    QuadParams { angle_deg: 0.0, ..place.quad(dst, src, filter) }
}

/// The whole rectangle in black, tinted and blended as the destination says (`blanktex`).
fn fill_black<R: Renderer>(r: &mut R, place: &Placement<'_>, area: Rect) -> bool {
    if !(has_extent(area.w) && has_extent(area.h) && area.x.is_finite() && area.y.is_finite()) {
        return false;
    }
    let tex = r.register_texture(BLACK_KEY, &BLACK_PIXEL, BLACK_SIZE, BLACK_SIZE);
    if r.texture_size(tex).is_none() {
        return false;
    }
    r.draw_textured_quad(tex, upright(place, area, UvRect::FULL, TextureFilter::Nearest));
    true
}

/// One picture of `tex` fitted into `area` by `stretch` (`drawBGAFixRatio`), the whole picture being
/// the region the stretch measures against.
fn draw_picture<R: Renderer>(r: &mut R, place: &Placement<'_>, tex: TextureId, area: Rect, stretch: StretchKind, filter: TextureFilter) -> bool {
    let Some((width, height)) = r.texture_size(tex) else {
        return false;
    };
    let (width, height) = (width.max(1) as f32, height.max(1) as f32);
    let whole = SkinRect::new(0.0, 0.0, width, height);
    let (fitted, source) = stretch_rect(stretch, SkinRect::new(area.x, area.y, area.w, area.h), whole);
    if !(has_extent(fitted.w) && has_extent(fitted.h) && fitted.x.is_finite() && fitted.y.is_finite()) {
        return false;
    }
    let region = UvRect::new(source.x / width, source.y / height, (source.x + source.w) / width, (source.y + source.h) / height);
    r.draw_textured_quad(tex, upright(place, Rect { x: fitted.x, y: fitted.y, w: fitted.w, h: fitted.h }, region, filter));
    true
}
