//! Reference images: the pictures a document refers to rather than ships.
//!
//! A destination whose id is a negative whole number names no object of the document's own. The
//! reference turns it into an image whose source is the game's (`JSONSkinLoader`, which builds
//! `new SkinImage(-id)` over a `SkinSourceReference`): the stage file, the back bitmap and the
//! banner of the chart in hand, and one black and one white pixel. Every other negative id is an
//! image with no source at all, which is still an object -- its conditions and its timer are looked
//! at on every frame -- and is never drawn.
//!
//! The chart's three pictures are the frame's to supply ([`ReferenceImages`]), because they change
//! with the chart and not with the document. One the frame does not carry leaves its object
//! undrawn. The two pixels are this module's own: they are registered with the renderer the first
//! time an object draws one and stay registered for as long as the renderer lives.
//!
//! This player adds one picture the reference has no id for: the NOTES RADAR polygon, in the band of
//! reference image ids it keeps for itself (`rbms_skin::property::extension`). The frame carries the
//! six values of the chart in hand rather than a texture, and the object paints the polygon from
//! them ([`RadarImage`]) and draws it as it draws any other. A frame that carries no values leaves
//! it undrawn. The reference draws nothing for an id of that band, so a document written for the
//! reference is unharmed either way.
//!
//! A reference image is drawn exactly as a document's own image is (`SkinImage.draw`): its whole
//! picture is the source region, so the object's stretch mode fits it, a negative width or height
//! mirrors it, and it is sampled unfiltered unless the document asked for filtering and the picture
//! is being resized.

use std::cell::RefCell;
use std::sync::OnceLock;

use rbms_skin::dst::SkinRect;
use rbms_skin::property::extension::RBMS_IMAGE_NOTES_RADAR;
use rbms_skin::property::generated::{IMAGE_BACKBMP, IMAGE_BANNER, IMAGE_BLACK, IMAGE_STAGEFILE, IMAGE_WHITE};

use super::SkinFrame;
use super::draw::Placement;
use super::graphs::{RadarImage, RadarValues};
use super::object::Body;
use crate::{Renderer, TextureId};

/// The registry key of the black pixel. It is shared by every screen drawn on one renderer.
const BLACK_KEY: &str = "rbms.skin.reference.black";

/// The registry key of the white pixel.
const WHITE_KEY: &str = "rbms.skin.reference.white";

/// One opaque black pixel, as RGBA8.
const BLACK_PIXEL: [u8; 4] = [0, 0, 0, u8::MAX];

/// One opaque white pixel, as RGBA8.
const WHITE_PIXEL: [u8; 4] = [u8::MAX; 4];

/// How many pixels wide and tall each of the two is.
const PLAIN_SIZE: u32 = 1;

/// One of the pictures a document can refer to by a negative destination id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceImage {
    /// The chart's stage file (`IMAGE_STAGEFILE`).
    StageFile,
    /// The chart's back bitmap (`IMAGE_BACKBMP`).
    BackBmp,
    /// The chart's banner (`IMAGE_BANNER`).
    Banner,
    /// One opaque black pixel (`IMAGE_BLACK`).
    Black,
    /// One opaque white pixel (`IMAGE_WHITE`).
    White,
    /// The NOTES RADAR polygon of the chart in hand (`RBMS_IMAGE_NOTES_RADAR`), which the renderer
    /// paints from the frame's six values.
    NotesRadar,
}

impl ReferenceImage {
    /// The picture a destination id refers to, or `None` when the id is not one of the reference's
    /// five or this player's radar: the id is the negated image id, written as a whole number.
    pub fn of_destination(id: &str) -> Option<ReferenceImage> {
        let image = id.parse::<i32>().ok()?.checked_neg()?;
        Some(match image {
            IMAGE_STAGEFILE => ReferenceImage::StageFile,
            IMAGE_BACKBMP => ReferenceImage::BackBmp,
            IMAGE_BANNER => ReferenceImage::Banner,
            IMAGE_BLACK => ReferenceImage::Black,
            IMAGE_WHITE => ReferenceImage::White,
            RBMS_IMAGE_NOTES_RADAR => ReferenceImage::NotesRadar,
            _ => return None,
        })
    }

    /// The registry key and the pixel of a picture this module supplies itself, or `None` for one
    /// the frame supplies.
    fn plain(self) -> Option<(&'static str, [u8; 4])> {
        match self {
            ReferenceImage::Black => Some((BLACK_KEY, BLACK_PIXEL)),
            ReferenceImage::White => Some((WHITE_KEY, WHITE_PIXEL)),
            ReferenceImage::StageFile | ReferenceImage::BackBmp | ReferenceImage::Banner | ReferenceImage::NotesRadar => None,
        }
    }
}

/// The reference images a frame carries, each already registered with the renderer.
///
/// These are the three that belong to a chart, and the values of the NOTES RADAR polygon, which the
/// renderer paints into a texture of its own. The black and the white pixel are not here, because the
/// renderer supplies those itself. [`ReferenceImages::default`] carries none.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ReferenceImages {
    pub stagefile: Option<TextureId>,
    pub backbmp: Option<TextureId>,
    pub banner: Option<TextureId>,
    /// The six display values of the chart's NOTES RADAR, each from nothing to the largest display
    /// value (200), in the order of the axes: the first points straight up and the rest follow
    /// clockwise. A value that is not a number is read as nothing and one past either end is held at
    /// that end. A frame with no chart under the cursor, or none the radar applies to, leaves this
    /// out and the polygon is not drawn.
    pub notes_radar: Option<RadarValues>,
}

impl ReferenceImages {
    /// The texture behind one of a chart's pictures, when the frame carries it. A picture the
    /// renderer supplies or paints itself is never the frame's to carry.
    pub fn texture(&self, image: ReferenceImage) -> Option<TextureId> {
        match image {
            ReferenceImage::StageFile => self.stagefile,
            ReferenceImage::BackBmp => self.backbmp,
            ReferenceImage::Banner => self.banner,
            ReferenceImage::Black | ReferenceImage::White | ReferenceImage::NotesRadar => None,
        }
    }
}

/// An object named by a negative destination id.
#[derive(Debug)]
pub(crate) struct ReferenceBody {
    /// The picture the id refers to, or `None` for a negative id the reference has no picture
    /// under.
    image: Option<ReferenceImage>,
    /// The pixel this object draws, once it has been registered with the renderer it is drawn on.
    plain: OnceLock<TextureId>,
    /// The polygon this object paints, for the object that names the radar and no other.
    radar: Option<RefCell<RadarImage>>,
}

impl ReferenceBody {
    /// Hands back the texture this object painted for itself, if it painted one.
    pub(crate) fn release<R: Renderer>(&self, r: &mut R) {
        if let Some(radar) = &self.radar {
            radar.borrow_mut().release(r);
        }
    }

    /// The texture this object draws this frame, or `None` when there is none to draw.
    ///
    /// A pixel is registered the first time it is asked for. Its key is shared, so every object and
    /// every screen on one renderer ends up with the same texture, and asking again after the
    /// renderer has dropped it registers it afresh.
    fn texture<R: Renderer>(&self, r: &mut R, frame: &SkinFrame<'_>) -> Option<TextureId> {
        let image = self.image?;
        if image == ReferenceImage::NotesRadar {
            let values = frame.data.images.notes_radar?;
            return self.radar.as_ref()?.borrow_mut().texture(r, values);
        }
        let Some((key, pixel)) = image.plain() else {
            return frame.data.images.texture(image);
        };
        let kept = *self.plain.get_or_init(|| r.register_texture(key, &pixel, PLAIN_SIZE, PLAIN_SIZE));
        Some(if r.texture_size(kept).is_some() { kept } else { r.register_texture(key, &pixel, PLAIN_SIZE, PLAIN_SIZE) })
    }
}

/// The object behind a destination id that is a negative whole number, or `None` when the id is
/// anything else (`JSONSkinLoader`: `Integer.parseInt(dst.id)`, then `id < 0`).
///
/// Every negative id makes an object, whether or not the reference has a picture under it.
pub(crate) fn build_reference(id: &str) -> Option<Body> {
    id.parse::<i32>().ok().filter(|number| *number < 0)?;
    let image = ReferenceImage::of_destination(id);
    let radar = (image == Some(ReferenceImage::NotesRadar)).then(|| RefCell::new(RadarImage::new()));
    Some(Body::Reference(ReferenceBody { image, plain: OnceLock::new(), radar }))
}

/// Draws the picture this object refers to over its destination, answering whether anything reached
/// the screen.
pub(crate) fn draw_reference<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &ReferenceBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let Some(tex) = body.texture(r, frame) else {
        return false;
    };
    let Some(size) = r.texture_size(tex) else {
        return false;
    };
    place.texture(r, tex, size, SkinRect::new(0.0, 0.0, size.0 as f32, size.1 as f32), rect)
}
