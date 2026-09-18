//! The groove gauge a play document draws for itself, cut into the parts the document asks for and
//! coloured by the gauge the run is on.
//!
//! A document names between four and thirty-six node images and the reference spreads them over one
//! table of thirty-six cells: six gauges, each with a lit, an unlit and a leading cell, and each of
//! those in a shade for the parts below the clear line and a shade for the parts above it. The
//! spreading tables below are the reference's own, so a document written for four nodes lights the
//! same cells here as there.
//!
//! One thing is deliberately not the reference's. Its "random" animation draws a fresh random number
//! every cycle, which no headless frame could reproduce; here every animation is a function of the
//! frame clock alone, so two runs of the same frame draw the same gauge.

use rbms_skin::dst::SkinRect;
use rbms_skin::loader::LoadedSkin;
use rbms_skin::model::GaugeDef;
use rbms_skin::property::generated::FLOAT_GROOVEGAUGE_1P;

use super::draw::Placement;
use super::object::{Body, Source, Sprite, image_sprite};
use super::{SkinAssets, SkinFrame};
use crate::Renderer;
use crate::ctx::RenderCtx;

/// Cells the reference's node table holds: six gauges of six cells each.
const GAUGE_SLOTS: usize = 36;

/// Cells one gauge takes in that table.
pub(crate) const SLOTS_PER_GAUGE: usize = 6;

/// The cell a part lit well behind the leading one reads.
const SLOT_LIT: usize = 0;

/// The cell an unlit part reads.
pub(crate) const SLOT_UNLIT: usize = 2;

/// The cell the leading part reads.
pub(crate) const SLOT_LEADING: usize = 4;

/// Added to a cell for a part that sits below the clear line.
pub(crate) const SLOT_BELOW_BORDER: usize = 1;

/// Percent of a full gauge, which is the unit the resolved field carries its clear line in.
const GAUGE_FULL_PERCENT: f32 = 100.0;

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

/// How a gauge animates the parts around the leading one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum GaugeAnimation {
    /// A stretch behind the leading part goes dark, by a length that changes every cycle.
    #[default]
    Scatter,
    /// That length grows towards the range and wraps.
    Grow,
    /// That length shrinks towards zero and wraps.
    Shrink,
    /// Nothing goes dark; the leading part pulses instead.
    Pulse,
}

/// The document `type` that grows the dark stretch.
const ANIMATION_GROW: i32 = 1;

/// The document `type` that shrinks it.
const ANIMATION_SHRINK: i32 = 2;

/// The document `type` that pulses the leading part instead.
const ANIMATION_PULSE: i32 = 3;

impl GaugeAnimation {
    /// The animation a document's `type` names. Anything the reference does not list scatters, which
    /// is the type it numbers zero.
    fn from_id(id: i32) -> GaugeAnimation {
        match id {
            ANIMATION_GROW => GaugeAnimation::Grow,
            ANIMATION_SHRINK => GaugeAnimation::Shrink,
            ANIMATION_PULSE => GaugeAnimation::Pulse,
            _ => GaugeAnimation::Scatter,
        }
    }
}

/// Mixed into the cycle count to scatter the dark stretch without a random number generator.
///
/// Any odd multiplier coprime with the ranges a document asks for would do; this one is the 32-bit
/// constant Knuth's multiplicative hash uses, chosen because it spreads small counters well.
const SCATTER_MULTIPLIER: i64 = 2_654_435_761;

/// The gauge bar, resolved from the document's `gauge` object.
///
/// The table is kept as indices into the node list rather than as thirty-six sprites, because a
/// document's nodes repeat across it and a draw-list entry has to stay small enough to sit beside
/// every other kind of body.
#[derive(Debug)]
pub(crate) struct GaugeBody {
    /// The node images the document named, in the order it named them.
    pub(crate) nodes: Vec<Sprite>,
    /// Which node each cell of the reference's table reads, as far as the nodes filled it.
    pub(crate) slots: [Option<NodeIndex>; GAUGE_SLOTS],
    /// How many parts the bar is cut into.
    pub(crate) parts: i32,
    pub(crate) animation: GaugeAnimation,
    /// How many parts behind the leading one the animation may darken.
    pub(crate) range: i32,
    /// Milliseconds one step of the animation lasts.
    pub(crate) cycle: i32,
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
    let body =
        GaugeBody { nodes, slots, parts: def.parts.max(1), animation: GaugeAnimation::from_id(def.gauge_type), range: def.range.max(0), cycle: def.cycle };
    Some(Body::Gauge(body))
}

/// The node images a document named, and the table its cells fill, spread the reference's way.
fn node_slots(skin: &LoadedSkin, def: &GaugeDef, sources: Source<'_>, warnings: &mut Vec<String>) -> (Vec<Sprite>, [Option<NodeIndex>; GAUGE_SLOTS]) {
    let mut nodes: Vec<Sprite> = Vec::with_capacity(def.nodes.len());
    let mut slots: [Option<NodeIndex>; GAUGE_SLOTS] = [None; GAUGE_SLOTS];
    let spread: Option<&[&[usize]]> = match def.nodes.len() {
        4 => Some(&SPREAD_4),
        8 => Some(&SPREAD_8),
        12 => Some(&SPREAD_12),
        GAUGE_SLOTS => None,
        other => {
            warnings.push(format!("gauge {:?} names {other} node images, which is not a shape the table is spread over", def.id));
            return (nodes, slots);
        }
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
        nodes.push(sprite);
        match spread {
            Some(table) => {
                for slot in table.get(node).copied().unwrap_or(&[]) {
                    slots[*slot] = Some(at);
                }
            }
            None => slots[node] = Some(at),
        }
    }
    (nodes, slots)
}

/// How many parts behind the leading one are dark this frame.
///
/// Every animation is a whole number of cycles into the frame clock, so nothing here depends on how
/// often the screen was drawn or on a random number.
fn dark_run(animation: GaugeAnimation, range: i32, cycle: i32, now_ms: i64) -> i32 {
    if range <= 0 || cycle <= 0 || animation == GaugeAnimation::Pulse {
        return 0;
    }
    let steps = (now_ms / i64::from(cycle)).max(0);
    let span = i64::from(range) + 1;
    let run = match animation {
        GaugeAnimation::Grow => steps.saturating_mul(i64::from(range)) % span,
        GaugeAnimation::Shrink => steps % span,
        _ => (steps.wrapping_mul(SCATTER_MULTIPLIER) >> 16).rem_euclid(span),
    };
    run as i32
}

/// How far through its cycle the pulsing gauge's leading part is, from nothing to full and back.
fn pulse_alpha(cycle: i32, now_ms: i64) -> f32 {
    if cycle <= 1 {
        return 1.0;
    }
    let span = cycle as f32;
    let half = span / 2.0;
    let at = now_ms.rem_euclid(i64::from(cycle)) as f32;
    let share = if at < half { at / half } else { (span - at) / half };
    share.clamp(0.0, 1.0)
}

/// Draws the gauge, answering whether anything reached the screen.
///
/// The clear line and the gauge in play both come from the frame's play state, so a screen that
/// carries none -- the score screen, whose own gauge growth the reference drives from `starttime`
/// and `endtime` -- leaves its gauge to the built-in renderer for now.
pub(crate) fn draw_gauge<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &GaugeBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(play) = frame.extra.play() else {
        return false;
    };
    let parts = body.parts.max(1);
    let filled = frame.state.float(FLOAT_GROOVEGAUGE_1P).clamp(0.0, 1.0);
    let lit = if filled > 0.0 { ((filled * parts as f32) as i32).max(1) } else { 0 };
    let border = (play.field.gauge_clear_threshold / GAUGE_FULL_PERCENT).clamp(0.0, 1.0);
    let column = play.gauge_kind.saturating_mul(SLOTS_PER_GAUGE);
    let dark = dark_run(body.animation, body.range, body.cycle, frame.now_ms);
    let step = rect.w / parts as f32;

    let mut drawn = false;
    for part in 1..=parts {
        let below = (part as f32 / parts as f32) < border;
        let shade = usize::from(below) * SLOT_BELOW_BORDER;
        let at = SkinRect::new(rect.x + step * (part - 1) as f32, rect.y, step, rect.h);
        let state = match body.animation {
            GaugeAnimation::Pulse if lit >= part => SLOT_LIT,
            GaugeAnimation::Pulse => SLOT_UNLIT,
            _ if lit == part => SLOT_LEADING,
            _ if lit - dark > part => SLOT_LIT,
            _ => SLOT_UNLIT,
        };
        drawn |= draw_part(r, place, body, column + state + shade, at, frame);
        if body.animation == GaugeAnimation::Pulse && lit == part {
            let faded = fade(place, pulse_alpha(body.cycle, frame.now_ms));
            drawn |= draw_part(r, &faded, body, column + SLOT_LEADING + shade, at, frame);
        }
    }
    drawn
}

/// Draws one part of the bar from the node the table put in that cell.
fn draw_part<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &GaugeBody, slot: usize, at: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let Some(sprite) = body.slots.get(slot).copied().flatten().and_then(|node| body.nodes.get(usize::from(node)).copied()) else {
        return false;
    };
    let cell = sprite.animation_index(sprite.cells(), frame.now_ms, frame.timers);
    place.cell(r, &sprite, cell, at)
}

/// The same placement at a share of its own opacity, which is how the pulsing gauge fades its
/// leading part in and out over the cell already drawn there.
fn fade<'a>(place: &Placement<'a>, share: f32) -> Placement<'a> {
    let mut tint = place.tint;
    tint.a = (f32::from(tint.a) * share).round().clamp(0.0, f32::from(u8::MAX)) as u8;
    Placement { object: place.object, blend: place.blend, tint, angle_deg: place.angle_deg, viewport: place.viewport }
}
