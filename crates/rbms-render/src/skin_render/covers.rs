//! The lane covers a play document draws for itself: the band the hidden modifier raises from the
//! judgement line, and the band the lift leaves below it (`SkinHidden`).
//!
//! Both are the same object in the reference, an image that is moved by offsets and cropped at a
//! line, and they are the same here. Where a cover sits is the rectangle the document gave it moved
//! by the offsets the running game publishes, and by nothing else:
//!
//! - A `hiddenCover` is moved by the lift's offset and by the hidden cover's, which the loader adds
//!   to whatever offsets the document's own destination names. The hidden cover's offset raises the
//!   image by the share of the field the modifier hides, and while the modifier is off it carries an
//!   alpha that fades the cover to nothing, which is all that keeps a cover the player has not asked
//!   for off the screen.
//! - A `liftCover` is moved by the lift's offset alone.
//!
//! [`attach_offsets`] is that loader step. The offsets then reach the cover through its destination
//! like any other object's, so nothing here reads the field a built-in screen measures.
//!
//! `disapearLine` is the line the image is cropped at: everything below it in the document's space
//! is scissored away, so the image is cut where the reference cuts it rather than resized. A cover
//! that does not reach above the line is not drawn at all, and a negative line crops nothing. With
//! `isDisapearLineLinkLift` the line rises with the lift, which is what a hidden cover wants -- it
//! grows up from the judgement line wherever that is -- and what a lift cover does not, since the
//! band it shows is exactly the one between where the judgement line was and where the lift put it.
//!
//! The cover the player pulls down from the top of the field is neither of these two records. It is
//! an ordinary slider or image the document moves with the lane cover's own value or offset.

use rbms_skin::dst::{DestinationTrack, SkinRect};
use rbms_skin::loader::LoadedSkin;
use rbms_skin::model::{HiddenCover, ImageDef, LiftCover, PropertyRef};
use rbms_skin::property::generated::{OFFSET_HIDDEN_COVER, OFFSET_LIFT};

use super::draw::Placement;
use super::object::{Body, Sprite, image_sprite};
use super::textures::Source;
use super::{SkinAssets, SkinFrame};
use crate::Renderer;
use crate::ctx::RenderCtx;

/// The offsets the loader adds to every `hiddenCover` destination, after the document's own: how
/// far the judgement line has been raised, and how far the hidden band reaches up from it together
/// with the alpha that hides the band while the modifier is off.
const HIDDEN_COVER_OFFSETS: [i32; 2] = [OFFSET_LIFT, OFFSET_HIDDEN_COVER];

/// The offsets the loader adds to every `liftCover` destination.
const LIFT_COVER_OFFSETS: [i32; 1] = [OFFSET_LIFT];

/// The lowest `disapearLine` that is a line at all; anything under it says the document wants none.
const LOWEST_DISAPPEAR_LINE: f32 = 0.0;

/// One cover, resolved from a `hiddenCover` or a `liftCover`.
///
/// Both records carry the same fields and are drawn the same way. Which of the two a cover is
/// decides only the offsets its destination is given ([`attach_offsets`]) and the default of
/// [`CoverBody::follows_lift`], which the document's own model already carries.
#[derive(Debug)]
pub(crate) struct CoverBody {
    pub(crate) sprite: Sprite,
    /// The document's own `disapearLine`, or a negative number when it declared none.
    pub(crate) disappear_line: f32,
    /// Whether that line rises with the lift (`isDisapearLineLinkLift`).
    pub(crate) follows_lift: bool,
}

impl CoverBody {
    /// The line this frame crops the cover at, or `None` when the document declared no line
    /// (`SkinHidden.prepare`'s `disapearLineAddedLift`).
    fn line(&self, frame: &SkinFrame<'_>) -> Option<f32> {
        if self.disappear_line < LOWEST_DISAPPEAR_LINE {
            return None;
        }
        let lift = if self.follows_lift { frame.state.offset(OFFSET_LIFT).map_or(0.0, |offset| offset.y) } else { 0.0 };
        Some(self.disappear_line + lift)
    }
}

/// Adds the offsets the reference's loader attaches to a cover to the destination it is drawn by
/// (`JsonPlaySkinObjectLoader`): the lift's and the hidden cover's for a `hiddenCover`, the lift's
/// for a `liftCover`. Any other object's destination is left as it is.
///
/// An offset the document's destination already names is not added again, because the reference
/// keeps an object's offsets as a set and so applies each of them once.
pub(crate) fn attach_offsets(body: &Body, track: &mut DestinationTrack) {
    let added: &[i32] = match body {
        Body::HiddenCover(_) => &HIDDEN_COVER_OFFSETS,
        Body::LiftCover(_) => &LIFT_COVER_OFFSETS,
        _ => return,
    };
    for offset in added {
        if !track.offsets.contains(offset) {
            track.offsets.push(*offset);
        }
    }
}

/// The sprite fields a cover shares with an ordinary image, as one image definition.
///
/// A cover names its source exactly the way an image does but is a record of its own, so this is
/// what lets both go through the same cutting rules rather than a second copy of them.
fn cover_image(src: &str, region: (i32, i32, i32, i32), divisions: (i32, i32), timer: Option<&PropertyRef>, cycle: i32) -> ImageDef {
    let (x, y, w, h) = region;
    ImageDef { src: src.to_owned(), x, y, w, h, divx: divisions.0, divy: divisions.1, timer: timer.cloned(), cycle, ..ImageDef::default() }
}

/// The image definition behind a `hiddenCover`.
fn hidden_image(cover: &HiddenCover) -> ImageDef {
    cover_image(&cover.src, (cover.x, cover.y, cover.w, cover.h), (cover.divx, cover.divy), cover.timer.as_ref(), cover.cycle)
}

/// The image definition behind a `liftCover`.
fn lift_image(cover: &LiftCover) -> ImageDef {
    cover_image(&cover.src, (cover.x, cover.y, cover.w, cover.h), (cover.divx, cover.divy), cover.timer.as_ref(), cover.cycle)
}

/// The cover behind `id`, or `None` when the document declares none by that name.
pub(crate) fn build_cover(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    _families: &[(String, String)],
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = &skin.def;
    if let Some(cover) = def.hidden_cover.iter().find(|cover| cover.id == id) {
        let sprite = sprite_or_warn(&hidden_image(cover), sources, id, &cover.src, warnings)?;
        return Some(Body::HiddenCover(CoverBody { sprite, disappear_line: cover.disappear_line as f32, follows_lift: cover.disappear_line_follows_lift }));
    }
    let cover = def.lift_cover.iter().find(|cover| cover.id == id)?;
    let sprite = sprite_or_warn(&lift_image(cover), sources, id, &cover.src, warnings)?;
    Some(Body::LiftCover(CoverBody { sprite, disappear_line: cover.disappear_line as f32, follows_lift: cover.disappear_line_follows_lift }))
}

/// The sprite a cover cuts out of its source, leaving a line behind when the source is not there.
fn sprite_or_warn(def: &ImageDef, sources: Source<'_>, id: &str, src: &str, warnings: &mut Vec<String>) -> Option<Sprite> {
    let sprite = image_sprite(def, sources);
    if sprite.is_none() {
        warnings.push(format!("cover {id:?} has no usable source {src:?}"));
    }
    sprite
}

/// Draws one cover where its destination and the offsets put it, answering whether anything reached
/// the screen (`SkinHidden.draw`).
///
/// The whole image is drawn into the whole rectangle. A line that crosses the rectangle scissors
/// away the part below it; one the rectangle does not reach above leaves nothing to draw. The
/// scissor is refused for a rectangle of no width, as the reference refuses it, and the cover is
/// then not drawn.
pub(crate) fn draw_cover<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &CoverBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let cell = body.sprite.animation_index(body.sprite.cells(), frame.now_us, frame.timers, frame.script());
    let Some(line) = body.line(frame) else {
        return place.cell(r, &body.sprite, cell, rect);
    };
    let top = rect.y + rect.h;
    if top <= line {
        return false;
    }
    if rect.y >= line {
        return place.cell(r, &body.sprite, cell, rect);
    }
    if rect.w <= 0.0 {
        return false;
    }
    r.push_clip(place.viewport.place(SkinRect::new(rect.x, line, rect.w, top - line)));
    let drawn = place.cell(r, &body.sprite, cell, rect);
    r.pop_clip();
    drawn
}
