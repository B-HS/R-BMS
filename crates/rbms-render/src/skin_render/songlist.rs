//! The song wheel a browser document draws for itself, as the reference draws one (`SkinBar` and
//! the `BarRenderer` behind it).
//!
//! The wheel is a fixed column of slots that the browser's bars turn through. Slot `i` shows the bar
//! `i - center` places from the cursor, going round the ends of the list, so a list shorter than the
//! wheel repeats and only an empty one leaves it bare. The slot the document calls its centre draws
//! the bar under the cursor with the `liston` destination; every other slot draws with `listoff`.
//!
//! What a slot *is* is split between the document and the browser. The document owns every
//! rectangle, colour and animation, through the destinations it nests under the wheel: one bar per
//! slot, and one text per kind of title, one number per difficulty, one image per lamp, trophy and
//! label, each placed against a bar's own corner. The browser owns the content ([`SongBars`]).
//!
//! Three things about how the reference draws a wheel are kept as they are, because a skin is
//! authored against them:
//!
//! - The wheel is drawn pass by pass rather than bar by bar: every bar's image, then every folder's
//!   graph, then every title, trophy, lamp, level and label. Where bars overlap, a later bar's image
//!   lies under an earlier bar's title.
//! - A title, a trophy, a lamp and a label are drawn where their destination last put them, whether
//!   or not it would draw them now: the reference prepares each of those once a frame and then draws
//!   it on every bar without asking again, so one hidden by a condition or a timer keeps the place
//!   and colour it last had, and one that was never placed draws nothing. A bar's own image, its
//!   level and a folder's graph are asked each time and do go away.
//! - A folder's graph sets no colour, blend or filter of its own and is drawn with whatever the bar
//!   image drawn before it left behind.
//!
//! A frame is settled while it is prepared. The reference asks a slot's conditions twice in a
//! frame, once as it prepares the wheel and once as it draws the bar, and asks a level's again for
//! every bar; here all of that happens in [`prepare_songlist`], in the same order, and what it
//! comes to is kept on the wheel for [`draw_songlist`] and for the pointer ([`bar_targets`]) to read.
//! Drawing asks the skin's Lua nothing.

mod bars;
mod build;
#[cfg(test)]
mod tests;

use std::cell::{Cell, RefCell};

use rbms_skin::dst::{DestinationTrack, DrawStateSource, Resolved, SkinColor, SkinRect, prepare};
use rbms_skin::loader::{Filtering, filtering_for, stretch_rect};
use rbms_skin::property::INTEGER_ABSENT;

use super::draw::Placement;
use super::object::{Body, MAX_PLACES, SkinObject, Sprite};
use super::text::{self, TextBody};
use super::{SkinFrame, SkinViewport};
use crate::ctx::RenderCtx;
use crate::{BlendMode, Color, QuadParams, Renderer, TextureFilter};

pub use bars::{
    BarDistribution, BarHold, BarKind, BarScroll, BarScroller, BarTrophy, LAMP_KINDS, RANK_KINDS, SCROLL_DURATION_HIGH_MS, SCROLL_DURATION_LOW_MS, SongBar,
    SongBars,
};
pub(crate) use build::build_songlist;

use bars::{LABEL_LONG_NOTE, LABEL_MINE, LABEL_RANDOM};

/// No offset: a part prepared where its own destination puts it.
const ORIGIN: (f32, f32) = (0.0, 0.0);

/// The whole numbers a property answers with when it has nothing to report, which draw no digits at
/// all (`SkinNumber.prepare`).
const INTEGER_NO_VALUE: [i32; 2] = [i32::MIN, i32::MAX];

/// A number's `align` that leaves its places where they fall, which is flush right.
const NUMBER_ALIGN_RIGHT: i32 = 0;

/// A number's `align` that pulls its places over the blanks to their left. Any other value pulls
/// them half as far and centres them.
const NUMBER_ALIGN_LEFT: i32 = 1;

/// The base a level is written out in.
const DECIMAL: u32 = 10;

/// The glyph a digit strip keeps its alternate zero in, one past the ten digits.
const GLYPH_ALTERNATE_ZERO: u32 = DECIMAL;

/// The `zeropadding` that fills a number's leading places with the alternate zero, and the one that
/// fills them with the ordinary zero.
const PADDING_ALTERNATE_ZERO: i32 = 2;
const PADDING_ZERO: i32 = 1;

/// Where a part's destination last put it, and in what colour (`SkinObject.region`, `color` and
/// `angle`, which outlive the frame they were worked out on).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Kept {
    rect: SkinRect,
    color: SkinColor,
    angle_deg: f32,
}

impl Kept {
    /// A part that has never been placed: no extent and no colour, so it draws nothing.
    const NEVER: Kept = Kept { rect: SkinRect::new(0.0, 0.0, 0.0, 0.0), color: SkinColor { r: 0, g: 0, b: 0, a: 0 }, angle_deg: 0.0 };

    /// The rectangle moved by `by`, which is how a part lands on a bar.
    fn moved(self, by: (f32, f32)) -> SkinRect {
        SkinRect::new(self.rect.x + by.0, self.rect.y + by.1, self.rect.w, self.rect.h)
    }
}

impl From<Resolved> for Kept {
    fn from(resolved: Resolved) -> Kept {
        Kept { rect: resolved.rect, color: resolved.color, angle_deg: resolved.angle_deg }
    }
}

/// Where one nested destination sits this frame, moved by `offset`, or `None` when its conditions,
/// its timer or the pointer leave it out (`SkinObject.prepare`).
fn place(track: &DestinationTrack, frame: &SkinFrame<'_>, offset: (f32, f32)) -> Option<Resolved> {
    let state: &dyn DrawStateSource = frame.state;
    prepare(track, frame.now_us, frame.timers, state, frame.script(), offset, frame.mouse)
}

/// The entry of a list the document may have left a hole in.
fn part<T>(list: &[Option<T>], index: usize) -> Option<&T> {
    list.get(index)?.as_ref()
}

/// The entry of such a list a bar names with a number of its own, which may be no entry at all.
fn numbered<T>(list: &[Option<T>], index: i32) -> Option<&T> {
    part(list, usize::try_from(index).ok()?)
}

/// One slot's bar in one of its two states: the image set it is cut from and the destination that
/// places it.
#[derive(Debug)]
struct BarPart {
    /// The destination as an object, which is what a placement measures its filter, its anchor and
    /// its stretch against.
    object: SkinObject,
    /// One image per kind of bar, with a hole where the document's set names an image this build
    /// could not load. All of them animate on one timer and one cycle.
    variants: Vec<Option<Sprite>>,
    /// Where the destination put the bar when the wheel was prepared, or `None` when it left the
    /// bar out (`SkinImage.draw` and `region` as `SkinBar.prepare` leaves them).
    shown: Cell<Option<SkinRect>>,
}

impl BarPart {
    /// Prepares the bar as the wheel prepares every one of its slots, whichever bar lands on it:
    /// the destination, then the first image of the set, which has to be there for the slot to be
    /// drawn at all (`SkinImage.prepare` with no value).
    fn prepare(&self, frame: &SkinFrame<'_>) {
        let placed = place(&self.object.track, frame, ORIGIN);
        let first = self.variants.first().copied().flatten();
        if let Some(sprite) = &first {
            sprite.prepare(frame);
        }
        self.shown.set(first.and(placed).map(|placed| placed.rect));
    }

    /// The image a bar of kind `image` is cut from. A kind past the end of the set is cut from the
    /// first image, and a hole in the set is no image (`SkinImage.prepare`).
    fn variant(&self, image: usize) -> Option<Sprite> {
        let image = if image >= self.variants.len() { 0 } else { image };
        self.variants.get(image).copied().flatten()
    }
}

/// One slot of the wheel: the bar it draws under the cursor and the one it draws everywhere else.
#[derive(Debug, Default)]
struct Slot {
    on: Option<BarPart>,
    off: Option<BarPart>,
}

/// One image a bar carries: a lamp, a trophy or a label.
#[derive(Debug)]
struct ImagePart {
    object: SkinObject,
    sprite: Sprite,
    /// Where the destination last put the image.
    kept: Cell<Kept>,
    /// The cell of its animation this frame is on.
    cell: Cell<u32>,
}

impl ImagePart {
    /// Prepares the image once for every bar it is drawn on (`SkinImage.prepare`): the place is
    /// taken when the destination gives one, and the animation moves on either way.
    fn prepare(&self, frame: &SkinFrame<'_>) {
        if let Some(placed) = place(&self.object.track, frame, ORIGIN) {
            self.kept.set(placed.into());
        }
        self.cell.set(self.sprite.animation_index(self.sprite.cells(), frame.now_us, frame.timers, frame.script()));
    }
}

/// One of the wheel's texts: the destination that places a kind of title, and a line of text for
/// each bar it is naming at the moment.
#[derive(Debug)]
struct TextPart {
    /// The text as the document declared it. Its own string is read while the wheel is prepared
    /// and never drawn.
    object: SkinObject,
    /// As many lines as the wheel has slots, each holding the title it composed last.
    lines: Vec<TextBody>,
    /// Which bar each line is holding the title of.
    owners: Vec<Cell<Option<usize>>>,
    kept: Cell<Kept>,
}

impl TextPart {
    /// Prepares the text once for every bar it names (`SkinText.prepare`): the place, and then the
    /// string of its own it would show anywhere else.
    fn prepare(&self, frame: &SkinFrame<'_>) {
        if let Some(placed) = place(&self.object.track, frame, ORIGIN) {
            self.kept.set(placed.into());
        }
        if let Body::Text(body) = &self.object.body
            && body.value.is_named()
        {
            body.value.text(frame.state, frame.lua);
        }
    }

    /// Hands a line to every bar this text names on this frame and takes the line back from every
    /// bar it no longer names, so a bar keeps its composed title for as long as it stays on the
    /// wheel and turning the wheel by one bar composes one title.
    fn settle<R: Renderer>(&self, wanted: impl Fn(usize) -> bool, bars: impl Iterator<Item = usize>, r: &mut R) {
        for (line, owner) in self.lines.iter().zip(&self.owners) {
            if owner.get().is_some_and(|bar| !wanted(bar)) {
                line.release(r);
                owner.set(None);
            }
        }
        for bar in bars {
            if self.owners.iter().any(|owner| owner.get() == Some(bar)) {
                continue;
            }
            if let Some(free) = self.owners.iter().find(|owner| owner.get().is_none()) {
                free.set(Some(bar));
            }
        }
    }

    /// The line holding the title of `bar`.
    fn line_of(&self, bar: usize) -> Option<&TextBody> {
        self.owners.iter().position(|owner| owner.get() == Some(bar)).and_then(|line| self.lines.get(line))
    }

    /// Hands every composed title back to the renderer.
    fn release<R: Renderer>(&self, r: &mut R) {
        for (line, owner) in self.lines.iter().zip(&self.owners) {
            line.release(r);
            owner.set(None);
        }
    }
}

/// One of the wheel's level numbers: the digit strip of one difficulty and the destination that
/// places it.
#[derive(Debug)]
struct LevelPart {
    object: SkinObject,
    sprite: Sprite,
    /// Glyphs one set of the strip holds: the ten digits, and the alternate zero when there is one.
    per_set: u32,
    /// Sets the strip animates through.
    sets: u32,
    /// How many places the number is drawn in.
    digits: usize,
    /// What fills the places a level does not reach ([`PADDING_ALTERNATE_ZERO`] or nothing).
    zero_padding: i32,
    /// Gap between places, in document pixels.
    space: f32,
    align: i32,
    /// The number the document's own `ref` names, which the reference reads once a frame and then
    /// draws nothing from: every bar's level is handed to the number instead.
    own_value: Option<i32>,
}

impl LevelPart {
    /// Prepares the number as the wheel prepares it, with the value of its own it will not draw
    /// (`SkinNumber.prepare`): a value that says nothing ends it there, and otherwise the
    /// destination is asked and then the strip's timer.
    fn prepare(&self, frame: &SkinFrame<'_>) {
        let own = self.own_value.map_or(INTEGER_ABSENT, |id| frame.state.integer(id));
        if !INTEGER_NO_VALUE.contains(&own) && place(&self.object.track, frame, ORIGIN).is_some() {
            self.sprite.prepare(frame);
        }
    }

    /// The glyph each place of `value` shows, most significant first, and how many places there are
    /// (`SkinNumber.prepare`, the branch for a strip with no negative half).
    fn glyphs(&self, value: i32) -> ([Option<u32>; MAX_PLACES], usize) {
        let mut places = [None; MAX_PLACES];
        let count = self.digits.min(MAX_PLACES);
        let mut remaining = value.unsigned_abs();
        for place in (0..count).rev() {
            places[place] = if remaining > 0 || place + 1 == count {
                Some(remaining % DECIMAL)
            } else {
                match self.zero_padding {
                    PADDING_ALTERNATE_ZERO => Some(GLYPH_ALTERNATE_ZERO),
                    PADDING_ZERO => Some(0),
                    _ => None,
                }
            };
            remaining /= DECIMAL;
        }
        (places, count)
    }
}

/// The graph a folder's bar carries: how the charts under it are spread over the clear lamps or the
/// score ranks (`SkinDistributionGraph`).
#[derive(Debug)]
struct GraphPart {
    track: DestinationTrack,
    /// The sheet the segments are cut from: one column per lamp or rank, one row per frame of the
    /// animation.
    sprite: Sprite,
    /// How many lamps or ranks the sheet has a column for.
    kinds: u32,
    /// Frames each column animates through.
    frames: u32,
    /// Whether the charts are counted by rank rather than by lamp.
    ranks: bool,
    /// Where the graph sits on a bar this frame and the frame of its animation, or `None` when it
    /// is not drawn.
    shown: Cell<Option<(SkinRect, u32)>>,
}

impl GraphPart {
    /// Prepares the graph (`SkinDistributionGraph.prepare`): with folders left uncounted it is not
    /// drawn and its destination is not asked.
    fn prepare(&self, frame: &SkinFrame<'_>, counted: bool) {
        if !counted {
            self.shown.set(None);
            return;
        }
        let placed = place(&self.track, frame, ORIGIN);
        let step = self.sprite.animation_index(self.frames, frame.now_us, frame.timers, frame.script());
        self.shown.set(placed.map(|placed| (placed.rect, step)));
    }
}

/// A bar's image as the frame's second look at its slot left it: the image of the bar's own kind,
/// and the colour and size the slot's destination gave it.
#[derive(Debug, Clone, Copy)]
struct DrawnImage {
    sprite: Sprite,
    cell: u32,
    placed: Kept,
}

/// A bar's level as the frame left it: which number draws it, where, and from which set.
#[derive(Debug, Clone, Copy)]
struct DrawnLevel {
    part: usize,
    placed: Kept,
    set: u32,
    value: i32,
}

/// What one slot shows on one frame (`BarRenderer.BarArea`).
#[derive(Debug, Clone, Copy)]
struct BarArea {
    /// Which bar of the list landed on the slot.
    bar: usize,
    /// Where the bar's corner is, slide included and before it is cut to a whole pixel.
    at: (f32, f32),
    /// Which image of the bar set the bar is cut from, or `None` when the slot draws nothing.
    image: Option<usize>,
    /// Which of the wheel's texts names the bar.
    text: usize,
    drawn: Option<DrawnImage>,
    level: Option<DrawnLevel>,
}

impl BarArea {
    /// A slot that draws nothing.
    const EMPTY: BarArea = BarArea { bar: 0, at: ORIGIN, image: None, text: 0, drawn: None, level: None };
}

/// The song wheel, resolved from the document's `songlist` object.
#[derive(Debug)]
pub(crate) struct SongListBody {
    /// The slot the cursor's bar lands on. A document may name one the wheel has no slot for, and
    /// then no slot is the cursor's.
    center: i32,
    /// The slots whose bars take a press, in the order the document listed them.
    clickable: Vec<usize>,
    slots: Vec<Slot>,
    text: Vec<Option<TextPart>>,
    level: Vec<Option<LevelPart>>,
    lamp: Vec<Option<ImagePart>>,
    player_lamp: Vec<Option<ImagePart>>,
    rival_lamp: Vec<Option<ImagePart>>,
    trophy: Vec<Option<ImagePart>>,
    label: Vec<Option<ImagePart>>,
    graph: Option<GraphPart>,
    /// What each slot shows, as the frame prepared last left it.
    areas: RefCell<Vec<BarArea>>,
}

impl SongListBody {
    /// The bar slot `slot` draws: the cursor's on the centre slot and the other one everywhere
    /// else (`SkinBar.getBarImages`).
    fn bar_part(&self, slot: usize) -> Option<&BarPart> {
        let on = i32::try_from(slot).is_ok_and(|slot| slot == self.center);
        let slot = self.slots.get(slot)?;
        if on { slot.on.as_ref() } else { slot.off.as_ref() }
    }

    /// Every image the wheel prepares once a frame, in the order the reference prepares them.
    fn prepare_parts(&self, frame: &SkinFrame<'_>, counted: bool) {
        self.slots.iter().filter_map(|slot| slot.on.as_ref()).for_each(|bar| bar.prepare(frame));
        self.slots.iter().filter_map(|slot| slot.off.as_ref()).for_each(|bar| bar.prepare(frame));
        self.trophy.iter().flatten().for_each(|image| image.prepare(frame));
        self.text.iter().flatten().for_each(|text| text.prepare(frame));
        self.level.iter().flatten().for_each(|level| level.prepare(frame));
        for images in [&self.label, &self.lamp, &self.player_lamp, &self.rival_lamp] {
            images.iter().flatten().for_each(|image| image.prepare(frame));
        }
        if let Some(graph) = &self.graph {
            graph.prepare(frame, counted);
        }
    }

    /// What slot `slot` shows: which bar landed on it, where the slide has carried it, and what it
    /// is cut from and named with (`BarRenderer.prepare`).
    ///
    /// A bar slides in from the slot beside it along a straight line. Across the screen the slide is
    /// held to one slot's distance, and up and down it is not, so with a second notch queued a bar
    /// starts two slots away vertically and one horizontally.
    fn lay_out(&self, slot: usize, list: &SongBars<'_>) -> BarArea {
        let Some(region) = self.bar_part(slot).and_then(|bar| bar.shown.get()) else {
            return BarArea::EMPTY;
        };
        let Some(bar) = list.bar_on(slot, self.center) else {
            return BarArea::EMPTY;
        };
        let mut at = (region.x, region.y);
        if let Some(remaining) = list.scroll.remaining()
            && let Some(beside) = list.scroll.neighbour(slot).and_then(|next| self.bar_part(next)).and_then(|bar| bar.shown.get())
        {
            at.0 += (beside.x - region.x) * remaining.clamp(-1.0, 1.0);
            at.1 += (beside.y - region.y) * remaining;
        }
        let shown = &list.bars[bar];
        let image = shown.kind.image_index();
        let text = image.map_or(0, |image| shown.text_index(image, |index| part(&self.text, index).is_some()));
        BarArea { bar, at, image, text, drawn: None, level: None }
    }

    /// The second look the reference takes at a slot as it draws the bar on it
    /// (`SkinImage.draw` with a value): the slot's destination is asked again, moved to where the
    /// bar is, and the image is the one of the bar's own kind.
    fn look_again(&self, slot: usize, area: &BarArea, frame: &SkinFrame<'_>) -> Option<DrawnImage> {
        let image = area.image?;
        let bar = self.bar_part(slot)?;
        let region = bar.shown.get()?;
        let placed = place(&bar.object.track, frame, (area.at.0 - region.x, area.at.1 - region.y));
        let sprite = bar.variant(image)?;
        let cell = sprite.animation_index(sprite.cells(), frame.now_us, frame.timers, frame.script());
        Some(DrawnImage { sprite, cell, placed: placed?.into() })
    }

    /// The level a chart's bar draws (`SkinNumber.draw` with the level for its value): the number
    /// of the chart's difficulty, asked afresh for this bar and moved onto it.
    fn level_of(&self, area: &BarArea, shown: &SongBar, frame: &SkinFrame<'_>) -> Option<DrawnLevel> {
        let index = shown.level_index()?;
        let level = part(&self.level, index)?;
        if INTEGER_NO_VALUE.contains(&shown.level) {
            return None;
        }
        let placed = place(&level.object.track, frame, area.at)?;
        let set = level.sprite.animation_index(level.sets, frame.now_us, frame.timers, frame.script());
        Some(DrawnLevel { part: index, placed: placed.into(), set, value: shown.level })
    }
}

/// Prepares the wheel for one frame: every part it nests, then what each slot shows, then the
/// questions the reference leaves until it draws (`SkinBar.prepare`, `BarRenderer.prepare` and the
/// prepares inside `BarRenderer.render`).
///
/// A frame with no bars, or with none in its list, leaves every slot empty.
pub(crate) fn prepare_songlist(body: &SongListBody, frame: &SkinFrame<'_>) {
    let list = frame.data.bars;
    body.prepare_parts(frame, list.is_some_and(|list| list.folder_lamps));

    let mut areas = body.areas.borrow_mut();
    areas.clear();
    let Some(list) = list.filter(|list| !list.bars.is_empty()) else {
        areas.resize(body.slots.len(), BarArea::EMPTY);
        return;
    };
    areas.extend((0..body.slots.len()).map(|slot| body.lay_out(slot, list)));
    for (slot, area) in areas.iter_mut().enumerate() {
        area.drawn = body.look_again(slot, area, frame);
    }
    for area in areas.iter_mut().filter(|area| area.image.is_some()) {
        area.level = body.level_of(area, &list.bars[area.bar], frame);
    }
}

/// What the object drawn last left on the reference's batch: the colour, the blend and the filter
/// the next thing drawn without setting its own is drawn with.
#[derive(Debug, Clone, Copy)]
struct Batch {
    tint: Color,
    blend: BlendMode,
    filter: TextureFilter,
}

/// A coordinate cut to a whole screen pixel and read back into the document's space.
///
/// The reference lays a skin out in screen pixels and holds a bar's corner as a whole number of
/// them (`ba.x = (int) ...`), so the cut is made after the document has been scaled to the screen.
fn whole_pixel(value: f32, scale: f32) -> f32 {
    if scale > 0.0 { (value * scale) as i32 as f32 / scale } else { value as i32 as f32 }
}

/// Everything drawing one wheel is done with.
struct Painter<'a, 'c, R: Renderer> {
    ctx: &'a mut RenderCtx<'c>,
    r: &'a mut R,
    viewport: &'a SkinViewport,
    batch: Batch,
    drawn: bool,
}

impl<R: Renderer> Painter<'_, '_, R> {
    /// Where a bar's corner is once it has been cut to a whole pixel.
    fn corner(&self, at: (f32, f32)) -> (f32, f32) {
        (whole_pixel(at.0, self.viewport.scale_x()), whole_pixel(at.1, self.viewport.scale_y()))
    }

    /// Draws one cell of one part at `rect`, in the part's own colour, blend, filter and stretch
    /// (`SkinObject.draw`). A part with no colour draws nothing and leaves the batch as it was.
    fn image(&mut self, object: &SkinObject, sprite: &Sprite, cell: u32, rect: SkinRect, kept: Kept) {
        if kept.color.a == 0 {
            return;
        }
        let blend = BlendMode::from_skin_blend(object.track.blend);
        let tint: Color = kept.color.into();
        let screen = self.viewport.place(rect);
        let (fitted, source) = stretch_rect(object.stretch, SkinRect::new(screen.x, screen.y, screen.w, screen.h), sprite.region(cell));
        let filter = match filtering_for(object.track.filter, fitted, (source.w, source.h)) {
            Filtering::Nearest => TextureFilter::Nearest,
            Filtering::Linear => TextureFilter::Linear,
        };
        self.batch = Batch { tint, blend, filter };
        self.ctx.text.leave_blend(blend);
        let place = Placement { object, blend, tint, angle_deg: kept.angle_deg, viewport: self.viewport };
        self.drawn |= place.cell(self.r, sprite, cell, rect);
    }

    /// Draws an image where its destination last put it, moved onto the bar at `corner`.
    fn kept(&mut self, image: &ImagePart, corner: (f32, f32)) {
        let kept = image.kept.get();
        self.image(&image.object, &image.sprite, image.cell.get(), kept.moved(corner), kept);
    }

    /// Draws a folder's graph on the bar at `corner` (`SkinDistributionGraph.draw`): one segment
    /// per lamp or rank, best first from the left, each as wide as its share of the charts counted
    /// by lamp. The segments are drawn with whatever the batch was left with.
    fn graph(&mut self, graph: &GraphPart, counts: &BarDistribution, corner: (f32, f32)) {
        let Some((region, step)) = graph.shown.get() else {
            return;
        };
        let total: u32 = counts.lamps.iter().sum();
        if total == 0 {
            return;
        }
        let shares: &[u32] = if graph.ranks { &counts.ranks } else { &counts.lamps };
        let mut before = 0_u32;
        for (kind, share) in shares.iter().enumerate().rev() {
            let x = region.x + before as f32 * region.w / total as f32 + corner.0;
            let width = *share as f32 * region.w / total as f32;
            before += share;
            let dst = self.viewport.place(SkinRect::new(x, region.y + corner.1, width, region.h));
            if dst.w <= 0.0 || dst.h <= 0.0 {
                continue;
            }
            let cell = step * graph.kinds + kind as u32;
            let quad =
                QuadParams { src: graph.sprite.uv(cell), tint: self.batch.tint, blend: self.batch.blend, filter: self.batch.filter, ..QuadParams::new(dst) };
            self.r.draw_textured_quad(graph.sprite.tex, quad);
            self.drawn = true;
        }
    }

    /// Draws a bar's title where its text's destination last put it, moved onto the bar at `corner`
    /// (`SkinBar.drawText`). A text sets no blend, so the title is blended as the last image was.
    fn title(&mut self, text: &TextPart, bar: usize, title: &str, corner: (f32, f32)) {
        let kept = text.kept.get();
        let Some(line) = text.line_of(bar).filter(|_| kept.color.a != 0) else {
            return;
        };
        let place = Placement {
            object: &text.object,
            blend: self.ctx.text.inherited_blend(),
            tint: kept.color.into(),
            angle_deg: kept.angle_deg,
            viewport: self.viewport,
        };
        self.drawn |= text::draw_line(self.ctx, self.r, &place, line, kept.moved(corner), title);
    }

    /// Draws a chart's level in the number of its difficulty (`SkinNumber.draw`): the places a level
    /// does not reach are left out and the rest are moved over them as the number's `align` says.
    fn level(&mut self, level: &LevelPart, drawn: &DrawnLevel, corner: (f32, f32), slid: (f32, f32)) {
        let rect = drawn.placed.moved((corner.0 - slid.0, corner.1 - slid.1));
        let (places, count) = level.glyphs(drawn.value);
        let step = rect.w + level.space;
        let blanks = places[..count].iter().filter(|glyph| glyph.is_none()).count() as f32;
        let shift = match level.align {
            NUMBER_ALIGN_RIGHT => 0.0,
            NUMBER_ALIGN_LEFT => step * blanks,
            _ => step * 0.5 * blanks,
        };
        for (place, glyph) in places[..count].iter().enumerate() {
            let Some(glyph) = glyph else {
                continue;
            };
            let at = SkinRect::new(rect.x + step * place as f32 - shift, rect.y, rect.w, rect.h);
            self.image(&level.object, &level.sprite, drawn.set * level.per_set + glyph, at, drawn.placed);
        }
    }
}

/// Draws the wheel as the frame prepared last left it, answering whether anything reached the
/// screen (`BarRenderer.render`).
///
/// The wheel's own destination places nothing and tints nothing: every part carries a destination
/// of its own. The passes run in the reference's order -- bars, graphs, titles, trophies, lamps,
/// levels, labels -- and each goes over every slot before the next begins.
pub(crate) fn draw_songlist<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &SongListBody,
    _rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(list) = frame.data.bars else {
        return false;
    };
    let areas = body.areas.borrow();
    let batch = Batch { tint: Color::WHITE, blend: ctx.text.inherited_blend(), filter: TextureFilter::Nearest };
    let mut painter = Painter { ctx, r, viewport: place.viewport, batch, drawn: false };
    let on_show = |area: &&BarArea| area.image.is_some() && area.bar < list.bars.len();

    for (slot, area) in areas.iter().enumerate() {
        if let (Some(drawn), Some(bar)) = (area.drawn, body.bar_part(slot)) {
            let corner = painter.corner(area.at);
            let rect = SkinRect::new(corner.0, corner.1, drawn.placed.rect.w, drawn.placed.rect.h);
            painter.image(&bar.object, &drawn.sprite, drawn.cell, rect, drawn.placed);
        }
    }

    if let Some(graph) = &body.graph {
        for area in areas.iter().filter(on_show) {
            let shown = &list.bars[area.bar];
            if let Some(counts) = shown.distribution.as_deref().filter(|_| shown.kind.is_directory()) {
                let corner = painter.corner(area.at);
                painter.graph(graph, counts, corner);
            }
        }
    }

    for (index, text) in body.text.iter().enumerate() {
        if let Some(text) = text {
            let named = || areas.iter().filter(on_show).filter(|area| area.text == index).map(|area| area.bar);
            text.settle(|bar| named().any(|named| named == bar), named(), painter.r);
        }
    }
    for area in areas.iter().filter(on_show) {
        if let Some(text) = part(&body.text, area.text) {
            let corner = painter.corner(area.at);
            painter.title(text, area.bar, &list.bars[area.bar].title, corner);
        }
    }

    for area in areas.iter().filter(on_show) {
        if let Some(trophy) = list.bars[area.bar].trophy_index().and_then(|index| part(&body.trophy, index)) {
            let corner = painter.corner(area.at);
            painter.kept(trophy, corner);
        }
    }

    for area in areas.iter().filter(on_show) {
        let shown = &list.bars[area.bar];
        let corner = painter.corner(area.at);
        let lamps = if list.rival {
            [numbered(&body.player_lamp, shown.lamp), numbered(&body.rival_lamp, shown.rival_lamp)]
        } else {
            [numbered(&body.lamp, shown.lamp), None]
        };
        for lamp in lamps.into_iter().flatten() {
            painter.kept(lamp, corner);
        }
    }

    for area in areas.iter().filter(on_show) {
        if let Some(drawn) = &area.level
            && let Some(level) = part(&body.level, drawn.part)
        {
            let corner = painter.corner(area.at);
            painter.level(level, drawn, corner, area.at);
        }
    }

    for area in areas.iter().filter(on_show) {
        let shown = &list.bars[area.bar];
        let corner = painter.corner(area.at);
        let long_note = shown.long_note_label(list.ln_mode).and_then(|label| part(&body.label, label).or_else(|| part(&body.label, LABEL_LONG_NOTE)));
        let mine = shown.has_mines().then(|| part(&body.label, LABEL_MINE)).flatten();
        let random = shown.has_random().then(|| part(&body.label, LABEL_RANDOM)).flatten();
        for label in [long_note, mine, random].into_iter().flatten() {
            painter.kept(label, corner);
        }
    }
    painter.drawn
}

/// One bar of a wheel that takes a press, as the frame prepared last left it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BarTarget {
    /// The slot the bar is drawn on.
    pub(crate) slot: usize,
    /// How many bars below the cursor's the bar is, negative above it.
    pub(crate) offset: i32,
    /// Which bar of the frame's list it is.
    pub(crate) bar: usize,
    /// Where the slot's destination put the bar, before any slide: a bar is pressed where it
    /// belongs, not where it is passing through (`BarRenderer.mousePressed`).
    pub(crate) region: SkinRect,
}

/// The bars of the wheel a press may land on, in the order the document listed its `clickable`
/// slots, which is the order a press is offered to them in.
///
/// Only a bar whose image the frame left standing is here: a slot whose destination hid it, or
/// whose bar has no image of its kind, takes nothing.
pub(crate) fn bar_targets(body: &SongListBody) -> Vec<BarTarget> {
    let areas = body.areas.borrow();
    body.clickable
        .iter()
        .filter_map(|slot| {
            let area = areas.get(*slot).filter(|area| area.drawn.is_some())?;
            let region = body.bar_part(*slot)?.shown.get()?;
            let offset = i32::try_from(*slot).ok()?.checked_sub(body.center)?;
            Some(BarTarget { slot: *slot, offset, bar: area.bar, region })
        })
        .collect()
}

/// Hands back the textures the wheel in `body` composed its titles into. Every other kind of object
/// holds none of this kind.
pub(crate) fn release<R: Renderer>(body: &Body, r: &mut R) {
    if let Body::SongList(body) = body {
        body.text.iter().flatten().for_each(|text| text.release(r));
    }
}
