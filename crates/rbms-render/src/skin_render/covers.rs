//! The lane covers a play document draws for itself: the band the hidden modifier raises from the
//! judgement line, and the band the lift leaves below it.
//!
//! The reference keeps both bands in one object class and moves each with an offset of its own: a
//! `hiddenCover` carries the hidden band's offset as well as the lift's, and a `liftCover` carries
//! only the lift's (`JsonPlaySkinObjectLoader`). rbms measures the hidden band as a share of the
//! field in [`LaneShade`] and folds the lift into the judgement line itself, so each cover here takes
//! the part of the rectangle the document gave it that its own band covers: `hiddenCover` takes the
//! share the hidden modifier hides, measured up from the foot of that rectangle, and `liftCover`
//! takes whatever of that rectangle the lift has left below the judgement line.
//!
//! The cover the player pulls down from the top of the field is neither of these two records, here
//! or in the reference, where it is an ordinary image the document moves with an offset of its own.
//! rbms publishes no such offset yet, so that band stays with [`crate::render_lane_cover`] and a
//! document cannot draw it.
//!
//! `disapearLine` keeps its own meaning: the part of a cover below that line in the document's space
//! is scissored away rather than resized, so the image is cropped exactly where the reference crops
//! it, and the line follows the player's lift when the document asks it to.

use rbms_skin::dst::SkinRect;
use rbms_skin::loader::LoadedSkin;
use rbms_skin::model::{HiddenCover, ImageDef, LiftCover};

use super::draw::Placement;
use super::object::{Body, Source, Sprite, image_sprite};
use super::{SkinAssets, SkinFrame};
use crate::Renderer;
use crate::ctx::RenderCtx;

/// The offset the player's lift is published under (`OFFSET_LIFT`), which the disappearing line
/// follows when the document links the two.
const OFFSET_LIFT: i32 = 3;

/// The lowest `disapearLine` that is a line at all; anything under it says the document wants none.
const LOWEST_DISAPPEAR_LINE: f32 = 0.0;

/// Which band of the field a cover hides. Both grow up from the foot of the cover's own rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoverBand {
    /// The share the hidden modifier takes off the field from the judgement line up.
    FromJudgement,
    /// Whatever of the rectangle the lift has left below the judgement line, which grows as the
    /// player lifts because the judgement line itself is what moves.
    BelowJudgement,
}

/// One cover, resolved from a `hiddenCover` or a `liftCover`.
///
/// Both records carry the same fields, so both resolve to this one body and [`CoverBand`] says
/// which way it grows.
#[derive(Debug)]
pub(crate) struct CoverBody {
    pub(crate) sprite: Sprite,
    pub(crate) band: CoverBand,
    /// The document's own `disapearLine`, or a negative number when it declared none.
    pub(crate) disappear_line: f32,
    pub(crate) follows_lift: bool,
}

/// The sprite fields a cover shares with an ordinary image, as one image definition.
///
/// A cover names its source exactly the way an image does but is a record of its own, so this is
/// what lets both go through the same cutting rules rather than a second copy of them.
fn cover_image(src: &str, region: (i32, i32, i32, i32), divisions: (i32, i32), timer: Option<&rbms_skin::model::PropertyRef>, cycle: i32) -> ImageDef {
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
        let body =
            CoverBody { sprite, band: CoverBand::FromJudgement, disappear_line: cover.disappear_line as f32, follows_lift: cover.disappear_line_follows_lift };
        return Some(Body::HiddenCover(body));
    }
    let cover = def.lift_cover.iter().find(|cover| cover.id == id)?;
    let sprite = sprite_or_warn(&lift_image(cover), sources, id, &cover.src, warnings)?;
    let body =
        CoverBody { sprite, band: CoverBand::BelowJudgement, disappear_line: cover.disappear_line as f32, follows_lift: cover.disappear_line_follows_lift };
    Some(Body::LiftCover(body))
}

/// The sprite a cover cuts out of its source, leaving a line behind when the source is not there.
fn sprite_or_warn(def: &ImageDef, sources: Source<'_>, id: &str, src: &str, warnings: &mut Vec<String>) -> Option<Sprite> {
    let sprite = image_sprite(def, sources);
    if sprite.is_none() {
        warnings.push(format!("cover {id:?} has no usable source {src:?}"));
    }
    sprite
}

/// Draws one cover, answering whether anything reached the screen.
///
/// A frame that carries no play state leaves the covers to the built-in renderer, which is what a
/// document loaded on a screen that measures no lane shade wants.
pub(crate) fn draw_cover<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &CoverBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(play) = frame.extra.play() else {
        return false;
    };
    if rect.h <= 0.0 {
        return false;
    }
    let covered = match body.band {
        CoverBand::FromJudgement => rect.h * play.shade.hidden.clamp(0.0, 1.0),
        CoverBand::BelowJudgement => (place.viewport.document_y(play.field.judge_y) - rect.y).clamp(0.0, rect.h),
    };
    if covered <= 0.0 {
        return false;
    }

    let band = SkinRect::new(rect.x, rect.y, rect.w, covered);
    let Some(visible) = visible_band(body, band, frame) else {
        return false;
    };
    let cell = body.sprite.animation_index(body.sprite.cells(), frame.now_ms, frame.timers);
    let clipped = visible != band;
    if clipped {
        r.push_clip(place.viewport.place(visible));
    }
    let drawn = place.cell(r, &body.sprite, cell, band);
    if clipped {
        r.pop_clip();
    }
    drawn
}

/// The part of `band` that survives the disappearing line, or `None` when the line has taken all of
/// it.
///
/// The line is a document-space height, so everything at or above it stays and everything below is
/// cropped. A band that does not reach the line at all is gone entirely, which is the reference's
/// own first test.
fn visible_band(body: &CoverBody, band: SkinRect, frame: &SkinFrame<'_>) -> Option<SkinRect> {
    if body.disappear_line < LOWEST_DISAPPEAR_LINE {
        return Some(band);
    }
    let lift = if body.follows_lift { frame.state.offset(OFFSET_LIFT).map_or(0.0, |offset| offset.y) } else { 0.0 };
    let line = body.disappear_line + lift;
    if band.y + band.h <= line {
        return None;
    }
    if band.y >= line {
        return Some(band);
    }
    Some(SkinRect::new(band.x, line, band.w, band.y + band.h - line))
}
