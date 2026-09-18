//! The song wheel a browser document draws for itself: one slot per visible row, each carrying the
//! bar, title, level, lamps and label of the chart that landed on it.
//!
//! The wheel is a fixed ring of slots that the browser's rows move through rather than a list that
//! grows with them: slot `i` shows `rows[sel - center + i]`, which is what puts the focused chart on
//! the slot the document called its centre and leaves the slots past either end of the list empty.
//!
//! What a slot *is* is split between the document and the browser. The document owns every
//! rectangle, colour and fade, through the destinations it nests under the wheel; the browser owns
//! the content, because a row's title, level, lamp and mode are the browser's to know. A slot's bar
//! is the one piece drawn from the document's own image when its id names one, since that is the
//! piece a wheel is mostly made of; everything else is drawn in the row's own colours.
//!
//! [`hot_rects`] is the other half of the contract: a document that draws the wheel also has to say
//! where its rows ended up, because the browser answers a click by hit-testing rectangles. Only the
//! rows: the buttons a document declares in its own `hotspot` table belong to the screen rather than
//! to the wheel, because a document may declare them without drawing a wheel at all.

use rbms_skin::dst::{DestinationTrack, DrawStateSource, LuaDrawEval, Resolved, SkinRect, prepare};
use rbms_skin::loader::{LoadedSkin, NamedTrack};
use rbms_skin::model::SkinDef;

use super::color::modulate;
use super::draw::Placement;
use super::object::{Body, SkinObject, Source, Sprite, image_sprite};
use super::state::{SelectListState, SkinHotAction, SkinHotspot};
use super::{MIN_TEXT_SCALE, SkinAssets, SkinFrame, TEXT_PIXELS_PER_SCALE};
use crate::Renderer;
use crate::ctx::RenderCtx;
use crate::{Color, Rect};

/// A label's `align` that starts the line at its destination's anchor, numbered as a text object's
/// alignment is because a slot label is one.
const LABEL_ALIGN_LEFT: i32 = 0;

/// A label's `align` that centres the line on that anchor.
const LABEL_ALIGN_CENTER: i32 = 1;

/// The cell of a bar's image a slot draws.
///
/// A wheel slot changes with the row under it rather than with a timer, so a bar is a still: the
/// cell animation an ordinary image object carries is not read here.
const BAR_CELL: u32 = 0;

/// One nested destination drawn as an image, when its id named one the document declared and this
/// build could load.
#[derive(Debug)]
struct Bar {
    track: DestinationTrack,
    sprite: Option<Sprite>,
}

/// One nested destination drawn as a line of text, in the font and alignment of the text object its
/// id names.
#[derive(Debug)]
struct Label {
    track: DestinationTrack,
    align: i32,
    family: Option<String>,
}

/// Everything one slot of the wheel draws, and whether a click on it means anything.
#[derive(Debug)]
struct Slot {
    off: Option<Bar>,
    on: Option<Bar>,
    title: Option<Label>,
    level: Option<Label>,
    label: Option<Label>,
    lamp: Option<DestinationTrack>,
    player_lamp: Option<DestinationTrack>,
    clickable: bool,
}

impl Slot {
    /// The bar this slot draws, which is the focused one only on the centre slot. A wheel that
    /// declares one of the two and not the other draws that one everywhere rather than nothing.
    fn bar(&self, focused: bool) -> Option<&Bar> {
        if focused { self.on.as_ref().or(self.off.as_ref()) } else { self.off.as_ref().or(self.on.as_ref()) }
    }
}

/// The song wheel, resolved from the document's `songlist` object.
#[derive(Debug)]
pub(crate) struct SongListBody {
    center: usize,
    slots: Vec<Slot>,
}

impl SongListBody {
    /// Which row of the browser's list lands on slot `index`, or `None` when the wheel reaches past
    /// either end of it.
    fn row_at(&self, list: &SelectListState<'_>, index: usize) -> Option<usize> {
        let row = list.sel as isize + index as isize - self.center as isize;
        (row >= 0 && (row as usize) < list.rows.len()).then_some(row as usize)
    }
}

/// Adds `line` unless the same one is already there, so a wheel whose every slot names the same
/// missing bar reports it once rather than once per slot.
fn warn_once(warnings: &mut Vec<String>, line: String) {
    if !warnings.contains(&line) {
        warnings.push(line);
    }
}

/// The still image a slot's bar is cut from, when its id names an image or an image set the
/// document declared and this build could load the source of.
///
/// A bar id that names neither, or one whose source never decoded, leaves the slot to be filled in
/// its own colour rather than dropping it; that is a document fault worth a line, because a wheel of
/// plain rectangles looks like a wheel that was drawn on purpose.
fn slot_sprite(def: &SkinDef, sources: Source<'_>, id: &str, warnings: &mut Vec<String>) -> Option<Sprite> {
    let from_set =
        || def.imageset.iter().find(|set| set.id == id).and_then(|set| set.images.iter().find_map(|name| def.image.iter().find(|image| image.id == *name)));
    let Some(image) = def.image.iter().find(|image| image.id == id).or_else(from_set) else {
        warn_once(warnings, format!("song bar {id:?} names neither an image nor an image set, so its slots are filled with their own colour"));
        return None;
    };
    let sprite = image_sprite(image, sources);
    if sprite.is_none() {
        warn_once(warnings, format!("song bar {id:?} has no usable source {:?}, so its slots are filled with their own colour", image.src));
    }
    sprite
}

/// One slot's bar, from the assembled track at `index` of one of the wheel's lists.
fn bar_of(tracks: &[NamedTrack], index: usize, def: &SkinDef, sources: Source<'_>, warnings: &mut Vec<String>) -> Option<Bar> {
    let named = tracks.get(index)?;
    Some(Bar { track: named.track.clone(), sprite: slot_sprite(def, sources, &named.id, warnings) })
}

/// One slot's label, from the assembled track at `index` of one of the wheel's lists.
fn label_of(tracks: &[NamedTrack], index: usize, def: &SkinDef, families: &[(String, String)]) -> Option<Label> {
    let named = tracks.get(index)?;
    let text = def.text.iter().find(|text| text.id == named.id);
    let family = text.and_then(|text| families.iter().find(|(id, _)| *id == text.font)).map(|(_, family)| family.clone());
    Some(Label { track: named.track.clone(), align: text.map_or(LABEL_ALIGN_LEFT, |text| text.align), family })
}

/// The wheel behind `id`, or `None` when the document declares no `songlist` by that name -- or
/// declares one with no slots in it.
///
/// A wheel of no slots is refused rather than built empty, because a body of any kind is what a
/// screen counts when it decides whether the document has taken the row list over: an empty one
/// would hide the built-in browser and then draw nothing in its place.
pub(crate) fn build_songlist(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    families: &[(String, String)],
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let wheel = skin.def.songlist.as_ref().filter(|list| list.id == id)?;
    let Some(tracks) = skin.nested.songlist.as_ref() else {
        warnings.push(format!("song wheel {id:?} has no assembled slots, so its rows are left to the built-in browser"));
        return None;
    };

    let lists =
        [&tracks.listoff, &tracks.liston, &tracks.text, &tracks.level, &tracks.lamp, &tracks.playerlamp, &tracks.rivallamp, &tracks.trophy, &tracks.label];
    let count = lists.iter().map(|list| list.len()).max().unwrap_or_default();
    if count == 0 {
        warnings.push(format!("song wheel {id:?} declares no slots, so its rows are left to the built-in browser"));
        return None;
    }
    if lists.iter().any(|list| !list.is_empty() && list.len() != count) {
        warnings.push(format!("song wheel {id:?} declares lists of uneven length, so its later slots go without the pieces the shorter ones ran out of"));
    }

    let mut slots: Vec<Slot> = Vec::with_capacity(count);
    for index in 0..count {
        slots.push(Slot {
            off: bar_of(&tracks.listoff, index, &skin.def, sources, warnings),
            on: bar_of(&tracks.liston, index, &skin.def, sources, warnings),
            title: label_of(&tracks.text, index, &skin.def, families),
            level: label_of(&tracks.level, index, &skin.def, families),
            label: label_of(&tracks.label, index, &skin.def, families),
            lamp: tracks.lamp.get(index).map(|named| named.track.clone()),
            player_lamp: tracks.playerlamp.get(index).map(|named| named.track.clone()),
            clickable: wheel.clickable.contains(&(index as i32)),
        });
    }

    let center = (wheel.center.max(0) as usize).min(count - 1);
    if wheel.center < 0 || wheel.center as usize != center {
        warnings.push(format!(
            "song wheel {id:?} calls slot {} its centre, which it has no slot for, so slot {center} of its {count} is used instead",
            wheel.center
        ));
    }
    Some(Body::SongList(SongListBody { center, slots }))
}

/// Where one nested destination sits this frame, or `None` when it is not drawn at all.
fn resolve(track: &DestinationTrack, frame: &SkinFrame<'_>) -> Option<Resolved> {
    let state: &dyn DrawStateSource = frame.state;
    let gate: Option<&dyn LuaDrawEval> = frame.lua.map(|lua| lua as &dyn LuaDrawEval);
    prepare(track, frame.now_ms, frame.timers, state, gate, (0.0, 0.0), frame.mouse)
}

/// Draws one slot's bar, as the image its id names or as a plain filled rectangle when it names
/// none.
fn draw_bar<R: Renderer>(r: &mut R, place: &Placement<'_>, bar: &Bar, frame: &SkinFrame<'_>) -> bool {
    let Some(resolved) = resolve(&bar.track, frame) else {
        return false;
    };
    let tint = modulate(resolved.color.into(), place.tint);
    if tint.a == 0 {
        return false;
    }
    let Some(sprite) = &bar.sprite else {
        let dst = place.viewport.place(resolved.rect);
        if dst.w <= 0.0 || dst.h <= 0.0 {
            return false;
        }
        r.fill_rect(dst, tint);
        return true;
    };
    let slot = Placement { object: place.object, blend: place.blend, tint, angle_deg: place.angle_deg, viewport: place.viewport };
    slot.cell(r, sprite, BAR_CELL, resolved.rect)
}

/// Draws one of a slot's lines of text, in the row's own ink where the row has one.
///
/// A slot is a box rather than an anchor: a title longer than the one the document drew the wheel
/// for is cut to the destination's width with an ellipsis, the way the built-in row does, so a long
/// title stops where its slot does instead of running across the column beside it. The line is
/// measured after the slot's own font is installed, because that is the face it will be drawn in.
fn draw_label<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    label: Option<&Label>,
    frame: &SkinFrame<'_>,
    line: &str,
    ink: Option<Color>,
) -> bool {
    let Some(label) = label.filter(|_| !line.is_empty()) else {
        return false;
    };
    let Some(resolved) = resolve(&label.track, frame) else {
        return false;
    };
    let color = modulate(ink.unwrap_or(Color::WHITE), modulate(resolved.color.into(), place.tint));
    let dst = place.viewport.place(resolved.rect);
    if color.a == 0 || dst.w <= 0.0 || dst.h <= 0.0 {
        return false;
    }
    match &label.family {
        Some(family) => ctx.text.set_family(family),
        None => ctx.text.reset_family(),
    }
    let scale = (dst.h / TEXT_PIXELS_PER_SCALE).max(MIN_TEXT_SCALE);
    let fitted = ctx.fit_text(line, scale, dst.w);
    match label.align {
        LABEL_ALIGN_LEFT => ctx.draw_text(r, dst.x, dst.y, scale, color, &fitted),
        LABEL_ALIGN_CENTER => ctx.draw_text_centered(r, dst.x, dst.y, scale, color, &fitted),
        _ => ctx.draw_text_right(r, dst.x, dst.y, scale, color, &fitted),
    }
    true
}

/// Draws one of a slot's lamps, which is a plain fill in the colour the browser resolved the row's
/// clear to.
fn draw_lamp<R: Renderer>(r: &mut R, place: &Placement<'_>, track: Option<&DestinationTrack>, frame: &SkinFrame<'_>, lamp: Color) -> bool {
    let Some(resolved) = track.and_then(|track| resolve(track, frame)) else {
        return false;
    };
    let color = modulate(lamp, modulate(resolved.color.into(), place.tint));
    let dst = place.viewport.place(resolved.rect);
    if color.a == 0 || dst.w <= 0.0 || dst.h <= 0.0 {
        return false;
    }
    r.fill_rect(dst, color);
    true
}

/// Draws the wheel, answering whether anything reached the screen.
///
/// The wheel's own destination places nothing, because every slot carries one of its own; what it
/// contributes is its colour, which fades or tints the whole wheel at once.
pub(crate) fn draw_songlist<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &SongListBody,
    _rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(list) = frame.extra.select() else {
        return false;
    };
    let mut drawn = false;
    for (index, slot) in body.slots.iter().enumerate() {
        let Some(row) = body.row_at(list, index).map(|row| &list.rows[row]) else {
            continue;
        };
        if let Some(bar) = slot.bar(index == body.center) {
            drawn |= draw_bar(r, place, bar, frame);
        }
        drawn |= draw_label(ctx, r, place, slot.title.as_ref(), frame, &row.title, None);
        drawn |= draw_label(ctx, r, place, slot.level.as_ref(), frame, &row.level, Some(row.difficulty_color));
        drawn |= draw_label(ctx, r, place, slot.label.as_ref(), frame, row.mode_short, Some(row.mode_color));
        drawn |= draw_lamp(r, place, slot.lamp.as_ref(), frame, row.lamp);
        drawn |= draw_lamp(r, place, slot.player_lamp.as_ref(), frame, row.lamp);
    }
    drawn
}

/// Every row rectangle this frame's wheel offers a click on, in the document's own coordinates.
///
/// Document space rather than screen space because a draw list knows the size it was authored at
/// and not the canvas it will land on; the caller maps each rectangle through the same
/// [`super::SkinViewport`] it draws the frame with.
///
/// A row rectangle carries the row of the browser's own list that landed on that slot, not the slot
/// number, because that is what the built-in [`crate::select::SelectHot::Row`] it stands in for
/// means and what the browser sets its selection from.
pub(crate) fn hot_rects(objects: &[SkinObject], frame: &SkinFrame<'_>) -> Vec<SkinHotspot> {
    let Some((object, wheel)) = objects.iter().find_map(|object| match &object.body {
        Body::SongList(body) => Some((object, body)),
        _ => None,
    }) else {
        return Vec::new();
    };
    let (Some(list), Some(placed)) = (frame.extra.select(), drawn_wheel(object, frame)) else {
        return Vec::new();
    };

    let mut spots: Vec<SkinHotspot> = Vec::new();
    for (index, slot) in wheel.slots.iter().enumerate().filter(|(_, slot)| slot.clickable) {
        let (Some(row), Some(bar)) = (wheel.row_at(list, index), slot.bar(index == wheel.center)) else {
            continue;
        };
        let Some(rect) = resolve(&bar.track, frame).map(rect_of).and_then(|rect| clipped_to(rect, placed.clip)) else {
            continue;
        };
        spots.push(SkinHotspot { rect, action: SkinHotAction::Row(row) });
    }
    spots
}

/// Where the wheel itself landed this frame, or `None` when the document did not draw it.
///
/// A row rectangle is only a rectangle a click means anything on while the row under it is on the
/// screen, and every one of the wheel's slots hangs off the one destination this resolves: a
/// document that gated the wheel off, or faded it out, drew no rows this frame and must not answer
/// a click on them. This is the same pair of tests [`super::draw::draw_object`] leaves on.
fn drawn_wheel(object: &SkinObject, frame: &SkinFrame<'_>) -> Option<Resolved> {
    resolve(&object.track, frame).filter(|resolved| resolved.color.a != 0)
}

/// One rectangle cut down to the wheel's scissor rectangle, or `None` when the two do not overlap.
///
/// Both are in document space, where a rectangle is measured up from the bottom left, so the overlap
/// is the plain one: a slot scrolled out of the wheel's own window is drawn nowhere and is clicked
/// nowhere either.
fn clipped_to(rect: Rect, clip: Option<SkinRect>) -> Option<Rect> {
    let Some(clip) = clip else {
        return Some(rect);
    };
    let left = rect.x.max(clip.x);
    let right = (rect.x + rect.w).min(clip.x + clip.w);
    let bottom = rect.y.max(clip.y);
    let top = (rect.y + rect.h).min(clip.y + clip.h);
    (right > left && top > bottom).then(|| Rect::new(left, bottom, right - left, top - bottom))
}

/// A resolved destination as the plain rectangle a hit test takes, still in document space.
fn rect_of(resolved: Resolved) -> Rect {
    Rect { x: resolved.rect.x, y: resolved.rect.y, w: resolved.rect.w, h: resolved.rect.h }
}
