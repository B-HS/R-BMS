//! The `bpmgraph` object (`SkinBPMGraph`): the tempo timeline along a chart's length.
//!
//! The reference paints the timeline into one pixmap as large as the object's rectangle was the
//! first time it was drawn, and draws that pixmap as an image, so the lines are as thick, as
//! aliased and as far apart on screen as the pixmap's own pixels are. A tempo is a level on a
//! logarithmic scale of its ratio to the chart's main tempo, from an eighth of it to eight times it.
//! The timeline is a step line: a flat run for each tempo, joined to the next by an upright.

use std::borrow::Cow;
use std::cell::RefCell;

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;
use rbms_skin::timer::MICROS_PER_MILLI;

use super::super::SkinFrame;
use super::super::draw::Placement;
use super::super::object::Body;
use super::pixmap::{Pixmap, Rgba, hex_color};
use super::{Layer, draw_layer, next_serial, plot_of};
use crate::ctx::RenderCtx;
use crate::{Rect, Renderer};

/// Pixels thick a line is drawn when the record asks for none (`lineWidth <= 0`).
const DEFAULT_LINE_WIDTH: i32 = 2;

/// Milliseconds added after the chart's last change, so the last tempo has a little run of its own.
const TAIL_MS: i32 = 1000;

/// The lowest ratio to the main tempo that has a level of its own; anything slower sits on the floor.
const LOWEST_RATIO: f64 = 1.0 / 8.0;

/// The highest ratio to the main tempo that has a level of its own; anything faster sits on the
/// ceiling.
const HIGHEST_RATIO: f64 = 8.0;

/// What a progress through the chart is measured in when a screen only knows progress: a nominal
/// chart length. Only the proportions matter, since the timeline is stretched to the object.
const NOMINAL_CHART_MS: f64 = 1_000_000.0;

/// How many hex digits of a colour the reference reads.
const COLOR_DIGITS: usize = 6;

/// The colours the reference draws a line in when the record names none.
const DEFAULT_MAIN: u32 = 0x00ff00;
const DEFAULT_MIN: u32 = 0x0000ff;
const DEFAULT_MAX: u32 = 0xff0000;
const DEFAULT_OTHER: u32 = 0xffff00;
const DEFAULT_STOP: u32 = 0xff00ff;
const DEFAULT_TRANSITION: u32 = 0x7f7f7f;

/// Where a chart's tempo changes, which is what a tempo graph plots.
///
/// Any screen that knows a chart may hand this over: the browser and the decide screen for the
/// chart under the cursor, the play and score screens for the one being played.
///
/// There are two ways to say it. [`BpmTimeline::of_chart`] is the reference's own shape and the one
/// to use when the chart is known to the millisecond; [`BpmTimeline::new`] is for a screen that only
/// knows how far through the chart each tempo starts.
#[derive(Debug, Clone, Copy)]
pub struct BpmTimeline<'a> {
    /// Each tempo the chart takes as `(progress through the chart, bpm)`, in chart order, the first
    /// of them being the tempo the chart opens on. Read only when `changes` is empty.
    pub points: &'a [(f32, f64)],
    /// Each change of speed as `(speed, time in milliseconds)`, in chart order. A speed is the
    /// tempo times the scroll rate, and zero while the chart is stopped. The first change is the
    /// chart's opening speed at time zero and the last is the chart's end.
    pub changes: &'a [(f64, f64)],
    /// The tempo the most notes are played at.
    pub main_bpm: f64,
    /// The slowest tempo the chart plays at, which the graph colours differently.
    pub min_bpm: f64,
    /// The fastest tempo the chart plays at.
    pub max_bpm: f64,
    /// How long the song is, in milliseconds, when that is known. The timeline never extends past it.
    pub length_ms: Option<i32>,
}

impl<'a> BpmTimeline<'a> {
    /// A timeline from the tempos of a chart known only by how far through it each begins, each
    /// `(progress through the chart, bpm)`, in chart order.
    ///
    /// The first tempo is taken for the chart's main one and the extremes are found among the points.
    pub fn new(points: &'a [(f32, f64)]) -> BpmTimeline<'a> {
        let bpms = || points.iter().map(|(_, bpm)| *bpm);
        BpmTimeline {
            points,
            changes: &[],
            main_bpm: bpms().next().unwrap_or_default(),
            min_bpm: bpms().filter(|bpm| *bpm > 0.0).fold(f64::INFINITY, f64::min),
            max_bpm: bpms().fold(f64::NEG_INFINITY, f64::max),
            length_ms: None,
        }
    }

    /// A timeline in the reference's own shape: every change of speed as `(speed, time in
    /// milliseconds)`, the chart's main, slowest and fastest tempo, and the song's length.
    pub fn of_chart(changes: &'a [(f64, f64)], main_bpm: f64, min_bpm: f64, max_bpm: f64, length_ms: Option<i32>) -> BpmTimeline<'a> {
        BpmTimeline { points: &[], changes, main_bpm, min_bpm, max_bpm, length_ms }
    }

    /// The changes to draw, whichever way the timeline was given, as `(speed, time in milliseconds)`.
    fn speeds(&self) -> Cow<'a, [(f64, f64)]> {
        if !self.changes.is_empty() {
            return Cow::Borrowed(self.changes);
        }
        let mut changes: Vec<(f64, f64)> = self.points.iter().map(|(progress, bpm)| (*bpm, f64::from(*progress) * NOMINAL_CHART_MS)).collect();
        if let Some(&(speed, _)) = changes.last() {
            changes.push((speed, NOMINAL_CHART_MS));
        }
        Cow::Owned(changes)
    }

    /// How long the timeline runs, in milliseconds, when its length was not given.
    fn length(&self) -> Option<i32> {
        if self.changes.is_empty() { Some(NOMINAL_CHART_MS as i32) } else { self.length_ms }
    }
}

/// The colours a timeline is drawn in.
#[derive(Debug, Clone, Copy)]
struct Palette {
    main: Rgba,
    lowest: Rgba,
    highest: Rgba,
    other: Rgba,
    stopped: Rgba,
    transition: Rgba,
}

/// What a timeline was painted for, which is what makes it painted again when it changes.
#[derive(Debug)]
struct Painted {
    changes: Vec<(f64, f64)>,
    main_bpm: f64,
    min_bpm: f64,
    max_bpm: f64,
    length_ms: Option<i32>,
}

/// The tempo timeline along a chart's length.
#[derive(Debug)]
pub(crate) struct BpmGraphBody {
    /// Milliseconds of the scene over which the timeline is revealed from the left.
    delay: i32,
    line_width: i32,
    palette: Palette,
    state: RefCell<Drawn>,
}

/// What the graph has painted, kept between frames because the reference keeps it: the texture is
/// built when the chart changes and not every frame.
#[derive(Debug)]
struct Drawn {
    layer: Layer,
    pixmap: Pixmap,
    painted: Option<Painted>,
}

impl BpmGraphBody {
    /// Hands the texture back to the renderer it was uploaded to.
    pub(crate) fn release<R: Renderer>(&self, r: &mut R) {
        let mut drawn = self.state.borrow_mut();
        drawn.layer.release(r);
        drawn.painted = None;
    }
}

/// The colour the reference makes of what a record wrote: the hex digits it contains, the first six
/// of them. A record that wrote none gets the colour the reference falls back to.
///
/// What is written with one to five digits cannot be read as a colour (the reference stops loading
/// the skin on it), so it takes the fallback too, and says so.
fn color_of(text: &str, fallback: u32, what: &str, id: &str, warnings: &mut Vec<String>) -> Rgba {
    let digits: String = text.chars().filter(char::is_ascii_hexdigit).take(COLOR_DIGITS).collect();
    match u32::from_str_radix(&digits, 16) {
        Ok(rgb) if digits.len() == COLOR_DIGITS => hex_color(rgb),
        _ => {
            if !digits.is_empty() {
                warnings.push(format!("tempo graph {id:?} writes its {what} colour as {text:?}, which has too few digits to be one"));
            }
            hex_color(fallback)
        }
    }
}

/// The tempo graph behind `id`, or `None` when the document declares none by that name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.bpmgraph.iter().find(|graph| graph.id == id)?;
    let palette = Palette {
        main: color_of(&graph.main_bpm_color, DEFAULT_MAIN, "main tempo", id, warnings),
        lowest: color_of(&graph.min_bpm_color, DEFAULT_MIN, "lowest tempo", id, warnings),
        highest: color_of(&graph.max_bpm_color, DEFAULT_MAX, "highest tempo", id, warnings),
        other: color_of(&graph.other_bpm_color, DEFAULT_OTHER, "other tempo", id, warnings),
        stopped: color_of(&graph.stop_line_color, DEFAULT_STOP, "stop", id, warnings),
        transition: color_of(&graph.transition_line_color, DEFAULT_TRANSITION, "transition", id, warnings),
    };
    let serial = next_serial();
    Some(Body::BpmGraph(BpmGraphBody {
        delay: graph.delay.max(0),
        line_width: if graph.line_width > 0 { graph.line_width } else { DEFAULT_LINE_WIDTH },
        palette,
        state: RefCell::new(Drawn { layer: Layer::new(serial, "tempo"), pixmap: Pixmap::default(), painted: None }),
    }))
}

/// How high up a pixmap of `height` rows the line for `speed` is drawn: on a log scale of its ratio
/// to `main`, from the floor at an eighth of it to the ceiling at eight times it, leaving room for
/// the line's own thickness.
fn level(speed: f64, main: f64, height: i32, line_width: i32) -> i32 {
    let ratio = (speed / main).clamp(LOWEST_RATIO, HIGHEST_RATIO);
    let floor = LOWEST_RATIO.log10();
    ((ratio.log10() - floor) / (HIGHEST_RATIO.log10() - floor) * f64::from(height - line_width)) as i32
}

/// Paints the step line of a timeline into a transparent `width` by `height` pixmap.
///
/// Returns `None`, which draws nothing, when there are not two changes to join or the main tempo is
/// not above zero.
fn paint_timeline(width: i32, height: i32, line_width: i32, timeline: &Painted, palette: &Palette) -> Option<Pixmap> {
    let changes = &timeline.changes;
    if changes.len() < 2 || timeline.main_bpm <= 0.0 {
        return None;
    }
    let mut pixmap = Pixmap::new(width.max(0) as usize, height.max(0) as usize);
    let last = changes[changes.len() - 1];
    let mut end = last.1 as i32;
    if let Some(length) = timeline.length_ms.filter(|length| *length < end) {
        end = length;
    }
    let end = end.wrapping_add(TAIL_MS);
    let column = |time: f64| (f64::from(width) * time / f64::from(end)) as i32;
    let line_color = |speed: f64| {
        if speed == timeline.main_bpm {
            palette.main
        } else if speed == timeline.min_bpm {
            palette.lowest
        } else if speed == timeline.max_bpm {
            palette.highest
        } else if speed <= 0.0 {
            palette.stopped
        } else {
            palette.other
        }
    };
    let at = |speed: f64| level(speed, timeline.main_bpm, height, line_width);

    for pair in changes.windows(2) {
        let ((before, from), (after, until)) = (pair[0], pair[1]);
        let (y1, y2) = (at(before), at(after));
        let rise = (y2 - y1).abs() - line_width;
        if rise > 0 {
            pixmap.fill_rect(column(until), y1.min(y2) + line_width, line_width, rise, palette.transition);
        }
        let start = column(from);
        pixmap.fill_rect(start, y1, column(until) - start + line_width, line_width, line_color(before));
    }
    let (speed, time) = last;
    let start = column(time);
    pixmap.fill_rect(start, at(speed), width - start + line_width, line_width, line_color(speed));
    Some(pixmap)
}

/// Draws the timeline, revealed from the left over the record's delay.
pub(crate) fn draw_bpm_graph<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &BpmGraphBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(timeline) = frame.data.series.bpm else {
        return false;
    };
    let Some(plot) = plot_of(place, rect) else {
        return false;
    };
    let (width, height) = (plot.w.abs() as i32, plot.h.abs() as i32);
    if width <= 0 || height <= 0 || width as u64 > u64::from(r.max_texture_size()) {
        return false;
    }
    let now_ms = frame.now_us / MICROS_PER_MILLI;
    let changes = timeline.speeds();

    let mut drawn = body.state.borrow_mut();
    let unchanged = drawn.painted.as_ref().is_some_and(|painted| {
        painted.changes == *changes
            && painted.main_bpm == timeline.main_bpm
            && painted.min_bpm == timeline.min_bpm
            && painted.max_bpm == timeline.max_bpm
            && painted.length_ms == timeline.length()
    });
    if !unchanged {
        let painted = Painted {
            changes: changes.into_owned(),
            main_bpm: timeline.main_bpm,
            min_bpm: timeline.min_bpm,
            max_bpm: timeline.max_bpm,
            length_ms: timeline.length(),
        };
        drawn.pixmap = paint_timeline(width, height, body.line_width, &painted, &body.palette).unwrap_or_default();
        drawn.painted = Some(painted);
        drawn.layer.mark_stale();
    }

    let Drawn { layer, pixmap, .. } = &mut *drawn;
    let (texture_w, texture_h) = pixmap.size();
    if texture_w == 0 {
        return false;
    }
    let render = if now_ms >= i64::from(body.delay) { 1.0 } else { now_ms as f32 / body.delay as f32 };
    let Some(tex) = layer.texture(r, pixmap) else {
        return false;
    };
    let shown = ((texture_w as f32 * render) as i32).clamp(0, texture_w as i32) as u32;
    let across = (plot.w * render) as i32 as f32;
    draw_layer(r, place, tex, (texture_w as u32, texture_h as u32), shown, Rect::new(plot.x, plot.y, across, plot.h))
}

#[cfg(test)]
mod tests;
