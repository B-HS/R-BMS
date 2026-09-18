//! The note field a play document draws for itself: the note in each lane, the long notes that run
//! between them, the mines, and the bar lines that cross the field.
//!
//! What the document owns and what the running chart owns are split the way the reference splits
//! them. The document says how wide a lane is and where it sits (`note.dst`), how tall one of its
//! notes is (`note.size`, or the height of the image's own cell), and which image each kind of note
//! draws. The chart says how far above the judgement line every note has got, and that comes from
//! [`visible_offsets`] and [`constant_offsets`] called with exactly the arguments
//! [`crate::render_playfield_on_background`] calls them with -- so a document that replaces the
//! field puts its notes on the same rows as the field it replaced.
//!
//! Two things the reference draws are not drawn here. Its expansion rate pulses with the quarter
//! note, which rbms keeps no clock for, so the rate applies as a constant to the notes themselves
//! and a document that leaves it at a hundred percent is unaffected. And its `processed` image
//! marks a note the player has
//! already been judged on, which rbms does not record per note; the chart's own hidden notes are
//! drawn from `hidden` instead, which is what that array holds in the reference.

use std::collections::HashMap;

use rbms_chart::scroll::{constant_offsets, visible_offsets};
use rbms_model::NoteKind;
use rbms_skin::dst::{DestinationTrack, DrawStateSource, LuaDrawEval, Resolved, SkinRect, WarnOnce, prepare};
use rbms_skin::loader::{Filtering, LoadedSkin, filtering_for};
use rbms_skin::model::{Animation, NoteSet};

use super::draw::Placement;
use super::object::{Body, Source, Sprite, image_sprite};
use super::{SkinAssets, SkinFrame};
use crate::ctx::RenderCtx;
use crate::playfield::PlayfieldView;
use crate::skin::Skin;
use crate::{BlendMode, Rect, Renderer, TextureFilter};

/// Percent one whole expansion rate is written as.
const FULL_EXPANSION_PERCENT: f32 = 100.0;

/// Fires once when a document declares fewer lanes than the mode being played has.
static TOO_FEW_LANES: WarnOnce = WarnOnce::new();

/// One lane of the field: where it sits, how tall its notes are, and what it draws them with.
#[derive(Debug, Default)]
pub(crate) struct NoteLane {
    /// The lane rectangle the document declared, in its own space.
    pub(crate) rect: SkinRect,
    /// How tall one note is, in the document's own pixels.
    pub(crate) height: f32,
    pub(crate) note: Option<Sprite>,
    pub(crate) ln_start: Option<Sprite>,
    pub(crate) ln_end: Option<Sprite>,
    pub(crate) ln_body: Option<Sprite>,
    pub(crate) ln_body_active: Option<Sprite>,
    pub(crate) mine: Option<Sprite>,
    pub(crate) hidden: Option<Sprite>,
}

/// One bar line, from the note set's `group` list.
///
/// The whole destination is kept rather than its first rectangle, because a line is placed, tinted,
/// faded, nudged and animated by its own keyframes exactly as any other object is; all the chart
/// adds is how far above the judgement line the section it marks has got.
#[derive(Debug)]
pub(crate) struct BarLine {
    pub(crate) track: DestinationTrack,
    pub(crate) sprite: Sprite,
}

/// The note field, resolved from the document's `note` object.
#[derive(Debug)]
pub(crate) struct NoteBody {
    pub(crate) lanes: Vec<NoteLane>,
    pub(crate) bars: Vec<BarLine>,
    /// How much wider and taller than its lane a note is drawn, as a share rather than a percent.
    pub(crate) expansion: (f32, f32),
}

/// The note field behind `id`, or `None` when the document declares no `note` by that name.
pub(crate) fn build_note(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    _families: &[(String, String)],
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = skin.def.note.as_ref().filter(|note| note.id == id)?;
    let lanes: Vec<NoteLane> = def.dst.iter().enumerate().map(|(lane, dst)| build_lane(skin, def, lane, dst, sources)).collect();
    if lanes.is_empty() {
        warnings.push(format!("note {id:?} declares no lane rectangles"));
        return None;
    }
    Some(Body::Note(NoteBody { lanes, bars: build_bars(skin, sources), expansion: expansion_of(def) }))
}

/// One lane's rectangle, note height and images.
fn build_lane(skin: &LoadedSkin, def: &NoteSet, lane: usize, dst: &Animation, sources: Source<'_>) -> NoteLane {
    let sprite = |names: &[String]| names.get(lane).and_then(|name| sprite_named(skin, name, sources));
    let note = sprite(&def.note);
    let cell_height = note.map_or(0.0, |sprite| sprite.cell_size().1);
    NoteLane {
        rect: animation_rect(dst),
        height: def.size.get(lane).copied().filter(|size| *size > 0.0).unwrap_or(cell_height),
        note,
        ln_start: sprite(&def.lnstart),
        ln_end: sprite(&def.lnend),
        ln_body: sprite(&def.lnbody),
        ln_body_active: sprite(&def.lnbody_active),
        mine: sprite(&def.mine),
        hidden: sprite(&def.hidden),
    }
}

/// The bar lines the note set's `group` list declares, each over the track the loader assembled for
/// it.
///
/// A track with no keyframes is a slot the document's own customisation ruled out, which the loader
/// keeps in place so the slots behind it do not shift; there is nothing to draw for one, so it is
/// dropped here rather than resolved to nothing every frame.
fn build_bars(skin: &LoadedSkin, sources: Source<'_>) -> Vec<BarLine> {
    skin.nested
        .note_group
        .iter()
        .filter(|named| !named.track.frames.is_empty())
        .filter_map(|named| Some(BarLine { track: named.track.clone(), sprite: sprite_named(skin, &named.id, sources)? }))
        .collect()
}

/// The sprite behind one of a note set's image ids.
fn sprite_named(skin: &LoadedSkin, name: &str, sources: Source<'_>) -> Option<Sprite> {
    let image = skin.def.image.iter().find(|image| image.id == name)?;
    image_sprite(image, sources)
}

/// The rectangle one keyframe states, with everything it left out reading as zero.
fn animation_rect(dst: &Animation) -> SkinRect {
    SkinRect::new(dst.x.unwrap_or_default() as f32, dst.y.unwrap_or_default() as f32, dst.w.unwrap_or_default() as f32, dst.h.unwrap_or_default() as f32)
}

/// The note set's expansion rate as a pair of shares.
fn expansion_of(def: &NoteSet) -> (f32, f32) {
    let share = |index: usize| def.expansionrate.get(index).map_or(1.0, |rate| *rate as f32 / FULL_EXPANSION_PERCENT);
    (share(0), share(1))
}

/// One lane as the frame draws it: which lane of the chart it is, where it sits on screen, what it
/// draws with, and whether the player is holding it.
struct Column<'a> {
    lane: usize,
    x: f32,
    w: f32,
    /// How tall one note is, in screen pixels.
    height: f32,
    images: &'a NoteLane,
    held: bool,
}

/// Where the chart has got to, which every note in the frame is placed against.
struct Scroll<'a> {
    field: &'a Skin,
    view: &'a PlayfieldView<'a>,
    /// Every timeline in the visible window, in the order the field walks them.
    visible: Vec<(usize, f32)>,
    /// The same, keyed by timeline, for the long-note ends that look their head up.
    at: HashMap<usize, f32>,
}

impl Scroll<'_> {
    /// Where a note in `column` sits on screen when its timeline is `offset` above the judgement
    /// line, before the expansion rate reshapes it.
    fn place(&self, column: &Column<'_>, offset: f32) -> Rect {
        Rect::new(column.x, self.field.judge_y - offset - column.height, column.w, column.height)
    }

    /// Whether the timeline at `index` has already reached the judgement line.
    fn passed(&self, index: usize) -> bool {
        self.view.timelines[index].time_us <= self.view.microtime
    }
}

/// Draws the field, answering whether anything reached the screen.
///
/// A frame that carries no play state leaves the field to the built-in renderer, which is what a
/// document loaded on a screen with no chart running wants.
pub(crate) fn draw_note<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &NoteBody,
    _rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(play) = frame.extra.play() else {
        return false;
    };
    let field = play.field;
    if field.lane_count() > body.lanes.len() && TOO_FEW_LANES.should_warn() {
        eprintln!("the play document declares {} lanes and the mode has {}; the rest stay empty", body.lanes.len(), field.lane_count());
    }

    let view = play.playfield;
    let height = field.lane_height();
    let visible = if view.constant {
        constant_offsets(view.timelines, view.microtime, view.hispeed, height)
    } else {
        visible_offsets(view.timelines, view.microtime, view.hispeed, height)
    };
    let at: HashMap<usize, f32> = visible.iter().copied().collect();
    let scroll = Scroll { field, view, visible, at };

    let mut drawn = false;
    for (lane, images) in body.lanes.iter().enumerate().take(field.lane_count()) {
        let placed = place.viewport.place(images.rect);
        let column = Column {
            lane,
            x: placed.x,
            w: placed.w,
            height: images.height * place.viewport.scale_y(),
            images,
            held: play.keys_down.get(lane).copied().unwrap_or_default(),
        };
        r.push_clip(Rect::new(column.x, field.top_y, column.w, height));
        drawn |= draw_long_notes(r, place, &column, &scroll, frame);
        drawn |= draw_notes(r, place, &column, body.expansion, &scroll, frame);
        r.pop_clip();
    }
    drawn |= draw_bars(r, place, body, &scroll, frame);
    drawn
}

/// Draws every long note whose body is on screen, the way the built-in field walks them: each end
/// looks its own head up, and a body whose head has run past the judgement line is anchored there.
fn draw_long_notes<R: Renderer>(r: &mut R, place: &Placement<'_>, column: &Column<'_>, scroll: &Scroll<'_>, frame: &SkinFrame<'_>) -> bool {
    if scroll.view.legacy_note {
        return false;
    }
    let field = scroll.field;
    let last_visible = scroll.visible.last().map(|(index, _)| *index).unwrap_or_default();
    let mut head: Option<usize> = None;
    let mut drawn = false;
    for (index, timeline) in scroll.view.timelines.iter().enumerate() {
        match timeline.notes.get(column.lane).and_then(Option::as_ref).map(|note| &note.kind) {
            Some(NoteKind::LongStart { .. }) => head = Some(index),
            Some(NoteKind::LongEnd { .. }) => {
                if let Some(start) = head.take()
                    && timeline.time_us >= scroll.view.microtime
                    && let Some(trunk) = long_note_trunk(column, scroll, start)
                {
                    let (sprite, start_offset) = trunk;
                    let bottom = (field.judge_y - start_offset).min(field.judge_y);
                    let top = scroll.at.get(&index).map_or(field.top_y, |offset| field.judge_y - offset).max(field.top_y);
                    if bottom > top {
                        drawn |= textured(r, place, &sprite, Rect::new(column.x, top, column.w, bottom - top), frame);
                    }
                }
            }
            _ => {}
        }
        if index >= last_visible && head.is_none() {
            break;
        }
    }
    drawn
}

/// The image one long note's body draws with and how far above the judgement line its head is, or
/// `None` when the head is too far back for the body to be on screen at all.
fn long_note_trunk(column: &Column<'_>, scroll: &Scroll<'_>, start: usize) -> Option<(Sprite, f32)> {
    let start_offset = match scroll.at.get(&start) {
        Some(offset) => *offset,
        None if scroll.passed(start) => 0.0,
        None => return None,
    };
    let active = column.held && scroll.passed(start);
    let sprite = if active { column.images.ln_body_active.or(column.images.ln_body) } else { column.images.ln_body };
    sprite.map(|sprite| (sprite, start_offset))
}

/// Draws every plain note, long-note head and tail, mine and hidden note in the visible window.
fn draw_notes<R: Renderer>(r: &mut R, place: &Placement<'_>, column: &Column<'_>, expansion: (f32, f32), scroll: &Scroll<'_>, frame: &SkinFrame<'_>) -> bool {
    let images = column.images;
    let mut drawn = false;
    for (index, offset) in &scroll.visible {
        let timeline = &scroll.view.timelines[*index];
        let at = scroll.place(column, *offset);
        let has_hidden = timeline.hidden.get(column.lane).is_some_and(Option::is_some);
        if let Some(sprite) = images.hidden.filter(|_| has_hidden) {
            drawn |= textured(r, place, &sprite, expand(at, expansion), frame);
        }
        let Some(Some(note)) = timeline.notes.get(column.lane) else {
            continue;
        };
        let sprite = match note.kind {
            NoteKind::Mine { .. } => images.mine,
            NoteKind::LongStart { .. } if !scroll.view.legacy_note => images.ln_start.or(images.note),
            NoteKind::LongEnd { .. } if scroll.view.legacy_note => None,
            NoteKind::LongEnd { .. } => images.ln_end.or(images.note),
            _ => images.note,
        };
        if let Some(sprite) = sprite {
            drawn |= textured(r, place, &sprite, expand(at, expansion), frame);
        }
    }
    drawn
}

/// Draws one bar line per section line in the visible window, each from its own destination.
///
/// The reference draws a line at the place its destination gives it and then shifts it by how far
/// the section has scrolled above the judgement line (`LaneRenderer`), so a line authored with its
/// foot on the judgement line rides the chart and one the document nudged keeps that nudge.
fn draw_bars<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &NoteBody, scroll: &Scroll<'_>, frame: &SkinFrame<'_>) -> bool {
    let field = scroll.field;
    let mut drawn = false;
    for bar in &body.bars {
        let Some(resolved) = resolve_bar(&bar.track, frame) else {
            continue;
        };
        let placed = place.viewport.place(resolved.rect);
        let line = Placement {
            object: place.object,
            blend: BlendMode::from_skin_blend(bar.track.blend),
            tint: resolved.color.into(),
            angle_deg: resolved.angle_deg,
            viewport: place.viewport,
        };
        r.push_clip(Rect::new(placed.x, field.top_y, placed.w, field.lane_height()));
        for (index, offset) in &scroll.visible {
            if !scroll.view.timelines[*index].section_line {
                continue;
            }
            drawn |= textured(r, &line, &bar.sprite, Rect::new(placed.x, placed.y - offset, placed.w, placed.h), frame);
        }
        r.pop_clip();
    }
    drawn
}

/// Where one bar line's destination puts it this frame, or `None` when the document is not drawing
/// it at all.
fn resolve_bar(track: &DestinationTrack, frame: &SkinFrame<'_>) -> Option<Resolved> {
    let state: &dyn DrawStateSource = frame.state;
    let gate: Option<&dyn LuaDrawEval> = frame.lua.map(|lua| lua as &dyn LuaDrawEval);
    prepare(track, frame.now_ms, frame.timers, state, gate, (0.0, 0.0), frame.mouse).filter(|resolved| resolved.color.a != 0)
}

/// One rectangle reshaped by the note set's expansion rate, about its own centre.
fn expand(at: Rect, expansion: (f32, f32)) -> Rect {
    let (wide, tall) = expansion;
    if wide == 1.0 && tall == 1.0 {
        return at;
    }
    let (w, h) = (at.w * wide, at.h * tall);
    Rect::new(at.x - (w - at.w) / 2.0, at.y - (h - at.h) / 2.0, w, h)
}

/// Draws one sprite cell straight onto the screen rectangle the scroll worked out.
///
/// Every other object reaches the screen through the viewport, because every other object is placed
/// in the document's own space. A note is not: its row comes from the chart in screen pixels, and
/// the viewport has already been applied to the part of it that came from the document.
fn textured<R: Renderer>(r: &mut R, place: &Placement<'_>, sprite: &Sprite, dst: Rect, frame: &SkinFrame<'_>) -> bool {
    if dst.w <= 0.0 || dst.h <= 0.0 {
        return false;
    }
    let source = sprite.cell_size();
    let filter = match filtering_for(place.object.track.filter, SkinRect::new(dst.x, dst.y, dst.w, dst.h), source) {
        Filtering::Nearest => TextureFilter::Nearest,
        Filtering::Linear => TextureFilter::Linear,
    };
    let cell = sprite.animation_index(sprite.cells(), frame.now_ms, frame.timers);
    r.draw_textured_quad(sprite.tex, place.quad(dst, sprite.uv(cell), filter));
    true
}
