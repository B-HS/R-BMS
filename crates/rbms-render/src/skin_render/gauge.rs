//! The groove gauge a document draws for itself (`SkinGauge`), cut into the parts the document asks
//! for and coloured by the gauge being shown.
//!
//! A document names four, eight, twelve or thirty-six node images and the reference spreads them
//! over one table of thirty-six cells: six gauges, each with a lit, an unlit and a leading cell, and
//! each of those in a shade for the parts at or above the clear line and a shade for the parts below
//! it. The spreading tables below are the reference's own, so a document written for four nodes
//! lights the same cells here as there.
//!
//! Everything the bar needs beyond its own record comes with the frame ([`GaugeFrame`]): which gauge
//! is shown, how full it is, and the least, the most and the clear line of every gauge of the run.
//! Nothing is read from a built-in field. A frame that carries no gauge leaves the bar undrawn, as
//! the reference leaves it on a screen that has none.
//!
//! The arithmetic is the reference's, kept in its order and its number types: the count of lit
//! parts, the part a clear line falls in, the cell each part reads and the animation of the parts
//! behind the leading one. Two things follow from keeping it that are worth knowing. The animation
//! is a state the object carries from one frame to the next rather than a function of the clock, so
//! it steps once for every frame that finds its interval over, however long that frame took. And a
//! record whose `type` is none of the four the reference lists draws nothing at all.
//!
//! On a score screen the bar fills from its least value to the run's last over the record's
//! `starttime` and `endtime`.
//!
//! One thing is deliberately not the reference's. Its "random" animation asks the virtual machine
//! for a random number; here the numbers come from a generator the object owns, started from the
//! same seed every time, so a screen drawn twice from its first frame draws the same gauge.

use std::cell::RefCell;

use rbms_skin::dst::SkinRect;
use rbms_skin::loader::LoadedSkin;
use rbms_skin::model::GaugeDef;
use rbms_skin::property::FLOAT_ABSENT;
use rbms_skin::property::generated::FLOAT_GROOVEGAUGE_1P;
use rbms_skin::resolve::Draw;
use rbms_skin::timer::MICROS_PER_MILLI;

use super::draw::Placement;
use super::object::{Body, Sprite, image_sprite};
use super::textures::Source;
use super::{SkinAssets, SkinFrame};
use crate::ctx::RenderCtx;
use crate::{Color, Renderer, TextureFilter};

/// How many gauges a run carries side by side, in the reference's order: assisted easy, easy,
/// normal, hard, ex-hard, hazard, and the three a course is played on.
pub const GAUGE_TYPES: usize = 9;

/// The first of the three gauges a course is played on (`GrooveGauge.CLASS`). They read the cells of
/// the hard, ex-hard and hazard gauges.
const FIRST_CLASS_GAUGE: usize = 6;

/// How far the course gauges sit above the gauges whose cells they share.
const CLASS_GAUGE_SHIFT: usize = 3;

/// The least a gauge can hold, the most, and how full it has to be for the run to count as cleared,
/// all in the gauge's own units (`GaugeElementProperty`'s `min`, `max` and `border`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeScale {
    pub min: f32,
    pub max: f32,
    pub border: f32,
}

impl GaugeScale {
    /// A gauge that runs from `min` to `max` and clears at `border`.
    pub const fn new(min: f32, max: f32, border: f32) -> GaugeScale {
        GaugeScale { min, max, border }
    }
}

/// The most a gauge known only by its clear line is taken to hold: a full hundred.
const PERCENT_SCALE_MAX: f32 = 100.0;

/// The least such a gauge is taken to hold.
const PERCENT_SCALE_MIN: f32 = 0.0;

/// The gauge a screen shows, which is everything a gauge object needs beyond its own record.
///
/// Any screen that knows the gauge of a run may hand this over, so a gauge is drawn wherever it is
/// filled in: on the play screen for the run in progress, on a score screen for the one that ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeFrame {
    /// Which gauge is shown, in the reference's numbering (`GrooveGauge`'s types, zero to eight):
    /// the one the run is on while it is played, the one the player is looking at on a score screen.
    pub gauge_type: usize,
    /// How full that gauge is, in its own units. On a score screen this is the last value the run
    /// left it at. `None` reads the share of a full gauge the host reports for
    /// [`FLOAT_GROOVEGAUGE_1P`], for the screens that know their gauge no other way.
    pub value: Option<f32>,
    /// The limits of every gauge of the run, by type.
    pub scales: [GaugeScale; GAUGE_TYPES],
    /// Whether the chart is being played in another mode than it was written in. The bar then cuts
    /// itself into as many parts as put every gauge's clear line on the edge of one.
    pub mode_changed: bool,
    /// Whether this is a score screen, where the bar fills from empty as the scene opens.
    pub result: bool,
}

impl GaugeFrame {
    /// The gauge of a run in progress.
    pub fn playing(gauge_type: usize, value: f32, scales: [GaugeScale; GAUGE_TYPES]) -> GaugeFrame {
        GaugeFrame { gauge_type, value: Some(value), scales, mode_changed: false, result: false }
    }

    /// The gauge of a run that ended, as a score screen shows it: `value` is the last value the run
    /// left the gauge numbered `gauge_type` at.
    pub fn finished(gauge_type: usize, value: f32, scales: [GaugeScale; GAUGE_TYPES]) -> GaugeFrame {
        GaugeFrame { gauge_type, value: Some(value), scales, mode_changed: false, result: true }
    }

    /// A gauge known only by which of the six columns of cells it reads and by the percent of a full
    /// gauge it clears at. How full it is comes from the host ([`GaugeFrame::value`]).
    pub fn of_kind(kind: usize, clear_threshold: f32) -> GaugeFrame {
        let scale = GaugeScale::new(PERCENT_SCALE_MIN, PERCENT_SCALE_MAX, clear_threshold);
        GaugeFrame { gauge_type: kind, value: None, scales: [scale; GAUGE_TYPES], mode_changed: false, result: false }
    }

    /// The same gauge, on a chart played in another mode than it was written in or not.
    pub fn with_mode_changed(mut self, mode_changed: bool) -> GaugeFrame {
        self.mode_changed = mode_changed;
        self
    }

    /// The limits of the gauge that is shown, or `None` when the type names no gauge.
    pub fn shown_scale(&self) -> Option<GaugeScale> {
        self.scales.get(self.gauge_type).copied()
    }
}

/// Cells the reference's node table holds: six gauges of six cells each.
const GAUGE_SLOTS: usize = 36;

/// Cells one gauge takes in that table.
pub(crate) const SLOTS_PER_GAUGE: usize = 6;

/// The cell a part lit well behind the leading one reads.
pub(crate) const SLOT_LIT: usize = 0;

/// The cell an unlit part reads.
pub(crate) const SLOT_UNLIT: usize = 2;

/// The cell the leading part reads.
pub(crate) const SLOT_LEADING: usize = 4;

/// Added to a cell for a part that sits below the clear line.
pub(crate) const SLOT_BELOW_BORDER: usize = 1;

/// The cell of a node image the bar draws: always the first, because the reference builds the
/// gauge's image set with no timer and no cycle and so never steps through a node's own cells.
const NODE_CELL: u32 = 0;

/// Which node image one cell of the table reads. A document names at most thirty-six, so one byte
/// holds the answer and the whole table stays smaller than a single sprite would.
type NodeIndex = u8;

/// Which cells each of four node images fills (`JsonSkinObjectLoader`'s `case 4`).
const SPREAD_4: [&[usize]; 4] =
    [&[0, 4, 6, 10, 12, 16, 18, 22, 24, 28, 30, 34], &[1, 5, 7, 11, 13, 17, 19, 23, 25, 29, 31, 35], &[2, 8, 14, 20, 26, 32], &[3, 9, 15, 21, 27, 33]];

/// Which cells each of eight node images fills (`case 8`).
const SPREAD_8: [&[usize]; 8] = [
    &[12, 16, 18, 22],
    &[13, 17, 19, 23],
    &[14, 20],
    &[15, 21],
    &[0, 4, 6, 10, 24, 28, 30, 34],
    &[1, 5, 7, 11, 25, 29, 31, 35],
    &[2, 8, 26, 32],
    &[3, 9, 27, 33],
];

/// Which cells each of twelve node images fills (`case 12`).
const SPREAD_12: [&[usize]; 12] = [
    &[12, 18],
    &[13, 19],
    &[14, 20],
    &[15, 21],
    &[0, 6, 24, 30],
    &[1, 7, 25, 31],
    &[2, 8, 26, 32],
    &[3, 9, 27, 33],
    &[16, 22],
    &[17, 23],
    &[4, 10, 28, 34],
    &[5, 11, 29, 35],
];

/// How a gauge animates the parts around the leading one (`SkinGauge.ANIMATION_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GaugeAnimation {
    /// A stretch behind the leading part goes dark, by a length drawn afresh every cycle.
    Random,
    /// That length steps down through the range and wraps (the reference's `INCLEASE`).
    Increase,
    /// That length steps up through the range and wraps (the reference's `DECLEASE`).
    Decrease,
    /// Nothing goes dark; the leading part fades in and out instead.
    Flickering,
}

/// The document `type` that draws the dark stretch at random.
const ANIMATION_RANDOM: i32 = 0;

/// The document `type` the reference calls `INCLEASE`.
const ANIMATION_INCREASE: i32 = 1;

/// The document `type` the reference calls `DECLEASE`.
const ANIMATION_DECREASE: i32 = 2;

/// The document `type` that fades the leading part in and out.
const ANIMATION_FLICKERING: i32 = 3;

impl GaugeAnimation {
    /// The animation a document's `type` names, or `None` for a type the reference lists no
    /// animation for. Its `prepare` and its `draw` both switch on the type with no default, so such
    /// a gauge never draws.
    pub(crate) fn from_id(id: i32) -> Option<GaugeAnimation> {
        match id {
            ANIMATION_RANDOM => Some(GaugeAnimation::Random),
            ANIMATION_INCREASE => Some(GaugeAnimation::Increase),
            ANIMATION_DECREASE => Some(GaugeAnimation::Decrease),
            ANIMATION_FLICKERING => Some(GaugeAnimation::Flickering),
            _ => None,
        }
    }
}

/// What the random animation's generator is started from. Any number would do; it is fixed so that
/// two runs of the same frames draw the same gauge.
const RANDOM_SEED: u64 = 0x6761_7567_6573_6b6e;

/// How many of a generator's bits make up the fraction a random number is read as: as many as a
/// double holds exactly.
const UNIT_BITS: u32 = 53;

/// What the gauge carries from one frame to the next (`SkinGauge`'s `animation`, `atime`, `parts`
/// and `isCheckedModeChanged`).
#[derive(Debug)]
pub(crate) struct GaugeMotion {
    /// How many parts behind the leading one are dark, or how far through its cycle the flicker is.
    animation: i32,
    /// The scene time, in milliseconds, after which the animation steps again.
    next_step_ms: i64,
    /// How many parts the bar is cut into: the record's, until a chart played in another mode asks
    /// for more.
    parts: i32,
    /// Whether the parts have been checked against the mode the chart is played in.
    mode_checked: bool,
    random: Draw,
}

impl GaugeMotion {
    /// The state a gauge of `parts` parts starts in.
    pub(crate) fn new(parts: i32) -> GaugeMotion {
        GaugeMotion { animation: 0, next_step_ms: 0, parts, mode_checked: false, random: Draw::from_seed(RANDOM_SEED) }
    }

    /// A random number from zero up to but not including one, as `Math.random` answers.
    fn unit(&mut self) -> f64 {
        (self.random.next_u64() >> (u64::BITS - UNIT_BITS)) as f64 / (1_u64 << UNIT_BITS) as f64
    }

    /// Steps the animation for the frame at `time_ms` (`SkinGauge.prepare`'s switch).
    ///
    /// The three that darken a stretch step only once their interval has passed. A range the
    /// reference would divide by zero for leaves the stretch empty.
    fn step(&mut self, animation: GaugeAnimation, range: i32, cycle_ms: i64, time_ms: i64) {
        let span = range.wrapping_add(1);
        match animation {
            GaugeAnimation::Flickering => {
                self.animation = time_ms.checked_rem(cycle_ms).unwrap_or_default() as i32;
                return;
            }
            _ if self.next_step_ms >= time_ms => return,
            GaugeAnimation::Random => self.animation = (self.unit() * f64::from(span)) as i32,
            GaugeAnimation::Increase => self.animation = self.animation.wrapping_add(range).checked_rem(span).unwrap_or_default(),
            GaugeAnimation::Decrease => self.animation = self.animation.wrapping_add(1).checked_rem(span).unwrap_or_default(),
        }
        self.next_step_ms = time_ms.wrapping_add(cycle_ms);
    }

    /// Checks, once, whether the bar has to be cut into more parts than the record asked for: on a
    /// chart played in another mode, as many as put every gauge's clear line on the edge of a part.
    fn check_mode(&mut self, gauge: &GaugeFrame) {
        if self.mode_checked {
            return;
        }
        self.mode_checked = true;
        if gauge.mode_changed {
            self.parts = retuned_parts(self.parts, &gauge.scales);
        }
    }
}

/// How many parts a bar of `parts` parts is cut into on a chart played in another mode than it was
/// written in: for each gauge the first count from `parts` up to its maximum that divides its clear
/// line evenly, and the largest of those.
fn retuned_parts(parts: i32, scales: &[GaugeScale]) -> i32 {
    let mut retuned = parts;
    for scale in scales.iter().filter(|scale| scale.max.is_finite()) {
        let mut count = parts;
        while count as f32 <= scale.max {
            if scale.border % (scale.max / count as f32) == 0.0 {
                retuned = retuned.max(count);
                break;
            }
            let Some(next) = count.checked_add(1) else {
                break;
            };
            count = next;
        }
    }
    retuned
}

/// How full a score screen's gauge is drawn `time_ms` into the scene: at its least until `start_ms`,
/// then rising at the pace that would fill the whole gauge by `end_ms`, and never past `value`, the
/// value the run ended on.
fn filling_value(value: f32, scale: GaugeScale, time_ms: i64, start_ms: i64, end_ms: i64) -> f32 {
    if time_ms < start_ms {
        return scale.min;
    }
    if time_ms < end_ms {
        return value.min((scale.max * (time_ms - start_ms) as f32 / (end_ms - start_ms) as f32).max(scale.min));
    }
    value
}

/// Where a gauge type's six cells begin in the table (`exgauge`).
fn first_slot(gauge_type: usize) -> usize {
    let column = if gauge_type >= FIRST_CLASS_GAUGE { gauge_type - CLASS_GAUGE_SHIFT } else { gauge_type };
    column * SLOTS_PER_GAUGE
}

/// How many parts of a bar of `parts` a gauge at `value` of `max` lights: at least one for any
/// value above nothing.
fn lit_parts(value: f32, parts: i32, max: f32) -> i32 {
    if value > 0.0 { ((value * parts as f32 / max) as i32).max(1) } else { 0 }
}

/// Whether part `part` of `parts` sits below the clear line: the gauge value its far edge stands
/// for is under the line.
fn below_border(part: i32, parts: i32, scale: GaugeScale) -> bool {
    part as f32 * scale.max / (parts as f32) < scale.border
}

/// Which of a gauge's six cells part `part` reads, before the shade for the clear line is added,
/// when `lit` parts are lit and the animation stands at `animation`.
fn part_slot(animation_type: GaugeAnimation, part: i32, lit: i32, animation: i32) -> usize {
    match animation_type {
        GaugeAnimation::Flickering if lit >= part => SLOT_LIT,
        GaugeAnimation::Flickering => SLOT_UNLIT,
        _ if lit == part => SLOT_LEADING,
        _ if lit.wrapping_sub(animation) > part => SLOT_LIT,
        _ => SLOT_UNLIT,
    }
}

/// How many halves a flicker's cycle has: one to fade up over and one to fade down.
const FLICKER_HALVES: i64 = 2;

/// How far short of a half the last millisecond of a fade is, in milliseconds: a fade of fifty
/// milliseconds is full at its forty-ninth.
const FLICKER_LAST_STEP: i64 = 1;

/// How much of its own opacity the flickering gauge's leading part is drawn at, `animation`
/// milliseconds into a cycle of `cycle_ms`: rising over the first half and falling over the second.
///
/// The share is the reference's own quotient and is not kept between nothing and one here: with an
/// odd cycle it passes one for a millisecond, and with a cycle of two it is not a number at all.
/// [`fade`] is where it becomes an opacity.
fn flicker_share(animation: i32, cycle_ms: i64) -> f32 {
    let rise = cycle_ms as f32 / FLICKER_HALVES as f32 - FLICKER_LAST_STEP as f32;
    let elapsed = i64::from(animation);
    if elapsed < cycle_ms / FLICKER_HALVES { elapsed as f32 / rise } else { (cycle_ms - FLICKER_LAST_STEP - elapsed) as f32 / rise }
}

/// The gauge bar, resolved from the document's `gauge` object.
///
/// The table is kept as indices into the node list rather than as thirty-six sprites, because a
/// document's nodes repeat across it and a draw-list entry has to stay small enough to sit beside
/// every other kind of body.
#[derive(Debug)]
pub(crate) struct GaugeBody {
    /// The node images the document named, in the order it named them, each held on its first cell.
    pub(crate) nodes: Vec<Sprite>,
    /// Which node each cell of the reference's table reads, as far as the nodes filled it.
    pub(crate) slots: [Option<NodeIndex>; GAUGE_SLOTS],
    /// The animation the record's `type` names, or `None` for one the reference draws nothing for.
    pub(crate) animation: Option<GaugeAnimation>,
    /// How many parts behind the leading one the animation may darken.
    pub(crate) range: i32,
    /// Milliseconds one step of the animation lasts.
    pub(crate) cycle: i32,
    /// The scene time, in milliseconds, at which a score screen's gauge starts to fill.
    pub(crate) starttime: i32,
    /// The scene time at which it has filled.
    pub(crate) endtime: i32,
    pub(crate) motion: RefCell<GaugeMotion>,
}

/// The gauge behind `id`, or `None` when the document declares no `gauge` by that name.
pub(crate) fn build_gauge(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    _families: &[(String, String)],
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = skin.def.gauge.as_ref().filter(|gauge| gauge.id == id)?;
    let (nodes, slots) = node_slots(skin, def, sources, warnings);
    if slots.iter().all(Option::is_none) {
        warnings.push(format!("gauge {id:?} names no node image this build could load"));
        return None;
    }
    let animation = GaugeAnimation::from_id(def.gauge_type);
    if animation.is_none() {
        warnings.push(format!("gauge {id:?} asks for animation type {}, which the reference draws nothing for (0 to 3 are drawn)", def.gauge_type));
    }
    let body = GaugeBody {
        nodes,
        slots,
        animation,
        range: def.range,
        cycle: def.cycle,
        starttime: def.starttime,
        endtime: def.endtime,
        motion: RefCell::new(GaugeMotion::new(def.parts)),
    };
    Some(Body::Gauge(body))
}

/// The node images a document named, and the table its cells fill, spread the reference's way.
fn node_slots(skin: &LoadedSkin, def: &GaugeDef, sources: Source<'_>, warnings: &mut Vec<String>) -> (Vec<Sprite>, [Option<NodeIndex>; GAUGE_SLOTS]) {
    let mut nodes: Vec<Sprite> = Vec::with_capacity(def.nodes.len());
    let mut slots: [Option<NodeIndex>; GAUGE_SLOTS] = [None; GAUGE_SLOTS];
    let Some(spread) = spread_of(def.nodes.len()) else {
        warnings.push(format!("gauge {:?} names {} node images, which is not a shape the table is spread over", def.id, def.nodes.len()));
        return (nodes, slots);
    };
    for (node, name) in def.nodes.iter().enumerate() {
        let Some(image) = skin.def.image.iter().find(|image| &image.id == name) else {
            warnings.push(format!("gauge {:?} names node image {name:?}, which the document does not declare", def.id));
            continue;
        };
        let Some(sprite) = image_sprite(image, sources) else {
            warnings.push(format!("gauge node {name:?} has no usable source {:?}", image.src));
            continue;
        };
        let Ok(at) = NodeIndex::try_from(nodes.len()) else {
            continue;
        };
        nodes.push(Sprite { timer: None, cycle: 0, ..sprite });
        for slot in spread.cells(node) {
            slots[slot] = Some(at);
        }
    }
    (nodes, slots)
}

/// How a list of node images is spread over the table.
#[derive(Debug, Clone, Copy)]
enum Spread {
    /// Each node fills the cells one of the reference's tables lists for it.
    Table(&'static [&'static [usize]]),
    /// Thirty-six nodes fill the table one to one.
    OneToOne,
}

impl Spread {
    /// The cells the node at `node` fills.
    fn cells(self, node: usize) -> impl Iterator<Item = usize> {
        let (listed, own) = match self {
            Spread::Table(table) => (table.get(node).copied().unwrap_or_default(), None),
            Spread::OneToOne => (&[][..], Some(node)),
        };
        listed.iter().copied().chain(own)
    }
}

/// The reference's tables, each for the list of as many node images as it has rows.
const SPREAD_TABLES: [&[&[usize]]; 3] = [&SPREAD_4, &SPREAD_8, &SPREAD_12];

/// How `count` node images are spread, or `None` for a count the reference has no table for.
fn spread_of(count: usize) -> Option<Spread> {
    if count == GAUGE_SLOTS {
        return Some(Spread::OneToOne);
    }
    SPREAD_TABLES.into_iter().find(|table| table.len() == count).map(Spread::Table)
}

/// What a frame makes of a gauge: the numbers `SkinGauge.prepare` leaves behind for `draw`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GaugeReading {
    /// Where the shown gauge's six cells begin in the table.
    first_slot: usize,
    /// How many parts are lit.
    lit: i32,
    parts: i32,
    scale: GaugeScale,
    /// The animation's state for this frame.
    animation: i32,
}

/// Steps a gauge's state to the frame at `time_ms` and reads what it draws there, or `None` when
/// the frame names a gauge that is not one.
fn read_gauge(body: &GaugeBody, animation: GaugeAnimation, gauge: &GaugeFrame, reported: f32, time_ms: i64) -> Option<GaugeReading> {
    let mut motion = body.motion.borrow_mut();
    motion.step(animation, body.range, i64::from(body.cycle), time_ms);
    motion.check_mode(gauge);

    let scale = gauge.shown_scale()?;
    let held = gauge.value.unwrap_or(if reported == FLOAT_ABSENT { scale.min } else { reported * scale.max });
    let value = if gauge.result { filling_value(held, scale, time_ms, i64::from(body.starttime), i64::from(body.endtime)) } else { held };
    Some(GaugeReading {
        first_slot: first_slot(gauge.gauge_type),
        lit: lit_parts(value, motion.parts, scale.max),
        parts: motion.parts,
        scale,
        animation: motion.animation,
    })
}

/// Draws the gauge, answering whether anything reached the screen.
///
/// The bar is drawn the way `SkinGauge.draw` draws it: straight onto the batch in the destination's
/// colour and blend, part by part, with none of the fitting, filtering or turning an image goes
/// through.
pub(crate) fn draw_gauge<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &GaugeBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let (Some(gauge), Some(animation)) = (frame.data.gauge, body.animation) else {
        return false;
    };
    let time_ms = frame.now_us / MICROS_PER_MILLI;
    let reported = if gauge.value.is_none() { frame.state.float(FLOAT_GROOVEGAUGE_1P) } else { FLOAT_ABSENT };
    let Some(reading) = read_gauge(body, animation, &gauge, reported, time_ms) else {
        return false;
    };
    let GaugeReading { first_slot, lit, parts, scale, animation: state } = reading;
    let upright = Placement { object: place.object, blend: place.blend, tint: place.tint, angle_deg: 0.0, viewport: place.viewport };

    let mut drawn = false;
    for part in 1..=parts {
        let shade = usize::from(below_border(part, parts, scale)) * SLOT_BELOW_BORDER;
        let at = SkinRect::new(rect.x + rect.w * (part - 1) as f32 / parts as f32, rect.y, rect.w / parts as f32, rect.h);
        drawn |= draw_part(r, &upright, body, first_slot + part_slot(animation, part, lit, state) + shade, at);
        if animation == GaugeAnimation::Flickering && part == lit {
            let faded = fade(&upright, flicker_share(state, i64::from(body.cycle)));
            drawn |= draw_part(r, &faded, body, first_slot + SLOT_LEADING + shade, at);
        }
    }
    drawn
}

/// Draws one part of the bar from the node the table put in that cell, stretched over `at`.
fn draw_part<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &GaugeBody, slot: usize, at: SkinRect) -> bool {
    let Some(sprite) = body.slots.get(slot).copied().flatten().and_then(|node| body.nodes.get(usize::from(node))) else {
        return false;
    };
    let dst = place.viewport.place(at);
    if !(dst.w.is_finite() && dst.h.is_finite() && dst.x.is_finite() && dst.y.is_finite() && dst.w != 0.0 && dst.h != 0.0) {
        return false;
    }
    r.draw_textured_quad(sprite.tex, place.quad(dst, sprite.uv(NODE_CELL), TextureFilter::Nearest));
    true
}

/// The opacity a colour of opacity `alpha` is left with at `share` of it: the product, kept within
/// what a channel holds and truncated, and nothing at all when the share is not a number. That is
/// what the reference's colour ends up as once it has been clamped and packed for the batch.
fn faded_alpha(alpha: u8, share: f32) -> u8 {
    (f32::from(alpha) * share) as u8
}

/// The same placement at a share of its own opacity, which is how the flickering gauge fades its
/// leading part in and out over the cell already drawn there.
fn fade<'a>(place: &Placement<'a>, share: f32) -> Placement<'a> {
    let tint = Color { a: faded_alpha(place.tint.a, share), ..place.tint };
    Placement { object: place.object, blend: place.blend, tint, angle_deg: place.angle_deg, viewport: place.viewport }
}

#[cfg(test)]
mod tests;
