//! The `timingdistributiongraph` object (`SkinTimingDistributionGraph`): how far from each note's
//! moment a run's hits landed, a millisecond to a column.
//!
//! The reference paints the graph into one pixmap and draws it as an image, the right way up. The
//! pixmap is as many columns wide as the record's `width` divided by its `lineWidth`, with the
//! middle column standing for a hit exactly on time, and as many rows tall as the fullest
//! millisecond rounded up to the next ten. Into it go, in this order,
//!
//! - the ground: the judgement windows as upright bands, the best in the middle and each wider one
//!   showing where it reaches past the one before,
//! - a faint tick on the top two rows every ten columns,
//! - the run's mean as an upright line, and a line a standard deviation to either side of it,
//! - and the distribution itself, a bar from the foot for every millisecond.
//!
//! Painting happens here, into a [`Pixmap`], with the reference's own arithmetic and its own pixmap
//! blending, and the pixmap is scaled onto the object's rectangle like any other image. The
//! reference paints it once for as long as the object lives; here it is painted again whenever the
//! numbers it was painted from differ.
//!
//! The reference draws this on the score screen of a single chart and nowhere else. Here the frame
//! decides: the graph is drawn wherever a frame carries a [`TimingHistogram`].

use std::cell::RefCell;

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;

use super::super::SkinFrame;
use super::super::draw::Placement;
use super::super::object::Body;
use super::pixmap::{Pixmap, Rgba, unit_color};
use super::{Layer, Upload, draw_layer, next_serial, plot_of, reference_color};
use crate::ctx::RenderCtx;
use crate::{Rect, Renderer};

/// How many judgements have a window on the graph's ground: the five a hit can take, best first.
pub const TIMING_JUDGE_AREAS: usize = 5;

/// What a histogram's mean reads when no hit was counted (`Float.MAX_VALUE`), which leaves the mean
/// undrawn.
const NO_AVERAGE: f32 = f32::MAX;

/// What a histogram's standard deviation reads when no hit was counted, which leaves the deviation
/// lines undrawn.
const NO_DEVIATION: f32 = -1.0;

/// How a run's hits were spread around the moment each note was due, which is what a timing
/// distribution graph plots.
#[derive(Debug, Clone, Copy)]
pub struct TimingHistogram<'a> {
    /// How many notes were hit at each whole millisecond from their moment. The middle entry counts
    /// the hits exactly on time, the entries after it the hits that came early by one millisecond,
    /// two and so on, and the entries before it the late ones. The reference counts a hundred and
    /// fifty milliseconds to either side, three hundred and one entries.
    pub bins: &'a [u32],
    /// The mean of those offsets in milliseconds, early being positive, or `f32::MAX` when no hit
    /// was counted.
    pub average: f32,
    /// Their standard deviation in milliseconds, or `-1.0` when no hit was counted.
    pub std_dev: f32,
    /// The window of each judgement in milliseconds, best first: how late a hit may be and still
    /// take it, which is not above zero, and how early. A screen that does not know the windows
    /// leaves them at zero, and only the on-time column of the ground is painted.
    pub judge_area: [[i32; 2]; TIMING_JUDGE_AREAS],
}

impl<'a> TimingHistogram<'a> {
    /// A histogram from its counts, with the mean and the deviation worked out from them the way
    /// the reference's score screen works them out.
    pub fn new(bins: &'a [u32]) -> TimingHistogram<'a> {
        let (average, std_dev) = statistics(bins);
        TimingHistogram { bins, average, std_dev, judge_area: [[0; 2]; TIMING_JUDGE_AREAS] }
    }

    /// The same histogram over the judgement windows of the run.
    pub fn with_judge_area(mut self, judge_area: [[i32; 2]; TIMING_JUDGE_AREAS]) -> TimingHistogram<'a> {
        self.judge_area = judge_area;
        self
    }

    /// The entry that counts the hits exactly on time (`getArrayCenter`).
    fn center(&self) -> i32 {
        middle_of(self.bins.len())
    }
}

/// How many sides of on time a histogram and a graph have: late and early, with on time between
/// them.
const SIDES: usize = 2;

/// Which of `count` entries, late to early, counts the hits exactly on time: the middle one.
fn middle_of(count: usize) -> i32 {
    (count / SIDES) as i32
}

/// The mean and the standard deviation of a histogram's offsets (`TimingDistribution.statisticValueCalcuate`):
/// whole-number sums for the mean, single precision for the squares.
fn statistics(bins: &[u32]) -> (f32, f32) {
    let center = middle_of(bins.len());
    let offsets = || bins.iter().zip(0_i32..).map(move |(count, index)| (*count as i32, index - center));
    let count = offsets().fold(0_i32, |count, (hits, _)| count.wrapping_add(hits));
    if count == 0 {
        return (NO_AVERAGE, NO_DEVIATION);
    }
    let sum = offsets().fold(0_i32, |sum, (hits, offset)| sum.wrapping_add(hits.wrapping_mul(offset)));
    let average = sum as f32 / count as f32;
    let squares = offsets().fold(0.0_f32, |squares, (hits, offset)| squares + hits as f32 * (offset as f32 - average) * (offset as f32 - average));
    (average, f64::from(squares / count as f32).sqrt() as f32)
}

/// How many rows a graph is at least tall.
const MIN_ROWS: i32 = 10;

/// The steps the row count grows in.
const ROW_STEP: i32 = 10;

/// How many columns lie between two ticks on the top of the ground.
const TICK_EVERY: usize = 10;

/// The last row a tick reaches, counted from the top row, which is row zero.
const TICK_LAST_ROW: i32 = 1;

/// How opaque the black of a tick is.
const TICK_ALPHA: f32 = 0.25;

/// The colour a record's colour is read as when the text has anything but hex digits in it or
/// fewer than six of them (`SkinTimingVisualizer.colorStringValidation`).
const INVALID_COLOR: Rgba = [u8::MAX, 0, 0, u8::MAX];

/// The fewest hex digits a colour is written in.
const COLOR_DIGITS: usize = 6;

/// The record value that switches the mean or the deviation lines on. Any other value leaves them
/// off.
const SWITCH_ON: i32 = 1;

/// The colour a record wrote for `what`, validated the way the reference validates the colours of
/// this graph: anything that is not at least six hex digits and nothing else is opaque red. The
/// reference says nothing when it does that; here the record is named in a warning.
fn validated_color(text: &str, what: &str, id: &str, warnings: &mut Vec<String>) -> Rgba {
    let valid = text.len() >= COLOR_DIGITS && text.bytes().all(|byte| byte.is_ascii_hexdigit());
    reference_color(text).filter(|_| valid).unwrap_or_else(|| {
        warnings.push(format!("timing distribution graph {id:?} writes its {what} as {text:?}, which is not a colour, so it is drawn in red"));
        INVALID_COLOR
    })
}

/// The fraction from which a number is rounded up.
const ROUND_UP_FROM: f64 = 0.5;

/// A whole number rounded from a single-precision one the way `Math.round` rounds it: half up,
/// held within what a whole number holds, and zero for something that is not a number.
fn java_round(value: f32) -> i32 {
    (f64::from(value) + ROUND_UP_FROM).floor() as i32
}

/// The histogram of a run's timing spread.
#[derive(Debug)]
pub(crate) struct TimingDistributionBody {
    /// How many columns the pixmap is wide (`gx`): the record's `width` over its `lineWidth`.
    columns: i32,
    graph: Rgba,
    average: Rgba,
    deviation: Rgba,
    /// The colour of each judgement's window, best first.
    judges: [Rgba; TIMING_JUDGE_AREAS],
    draw_average: bool,
    draw_deviation: bool,
    state: RefCell<Drawn>,
}

/// What the graph has painted, kept between frames because the reference keeps it.
#[derive(Debug)]
struct Drawn {
    layer: Layer,
    pixmap: Pixmap,
    painted: Option<Painted>,
}

/// Everything the pixmap was painted from. The two statistics are kept as their bit patterns, so
/// that one which is not a number still compares equal to itself.
#[derive(Debug)]
struct Painted {
    bins: Vec<u32>,
    average: u32,
    std_dev: u32,
    judge_area: [[i32; 2]; TIMING_JUDGE_AREAS],
}

impl Painted {
    fn shows(&self, histogram: &TimingHistogram<'_>) -> bool {
        self.bins == histogram.bins
            && self.average == histogram.average.to_bits()
            && self.std_dev == histogram.std_dev.to_bits()
            && self.judge_area == histogram.judge_area
    }
}

impl TimingDistributionBody {
    /// Hands the texture back to the renderer it was uploaded to.
    pub(crate) fn release<R: Renderer>(&self, r: &mut R) {
        let mut drawn = self.state.borrow_mut();
        drawn.layer.release(r);
        drawn.painted = None;
    }
}

/// How many columns a record of `width` and `line_width` asks for, or `None` where the reference's
/// own arithmetic divides by zero: a width of nothing.
///
/// The line width is held between one and the width as it was written, not the width after it has
/// been raised to one, so a record with a width below one divides one by that width.
fn column_count(width: i32, line_width: i32) -> Option<i32> {
    let across = width.max(1);
    let per_column = if line_width < 1 {
        1
    } else if line_width > width {
        width
    } else {
        line_width
    };
    across.checked_div(per_column)
}

/// The timing distribution graph behind `id`, or `None` when the document declares none by that
/// name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.timingdistributiongraph.iter().find(|graph| graph.id == id)?;
    let columns = column_count(graph.width, graph.line_width).filter(|columns| *columns > 0);
    if columns.is_none() {
        warnings.push(format!(
            "timing distribution graph {id:?} asks for a width of {} in lines of {}, which leaves it no columns, so it draws nothing",
            graph.width, graph.line_width
        ));
    }
    let mut read = |text: &str, what: &str| validated_color(text, what, id, warnings);
    let judges = [
        read(&graph.pgreat_color, "PGColor"),
        read(&graph.great_color, "GRColor"),
        read(&graph.good_color, "GDColor"),
        read(&graph.bad_color, "BDColor"),
        read(&graph.poor_color, "PRColor"),
    ];
    Some(Body::TimingDistribution(TimingDistributionBody {
        columns: columns.unwrap_or_default(),
        graph: read(&graph.graph_color, "graphColor"),
        average: read(&graph.average_color, "averageColor"),
        deviation: read(&graph.dev_color, "devColor"),
        judges,
        draw_average: graph.draw_average == SWITCH_ON,
        draw_deviation: graph.draw_dev == SWITCH_ON,
        state: RefCell::new(Drawn { layer: Layer::new(next_serial(), "timing"), pixmap: Pixmap::default(), painted: None }),
    }))
}

/// How many rows a graph is tall: ten, or the fullest millisecond rounded up to the ten above it.
fn row_count(bins: &[u32]) -> i32 {
    bins.iter().fold(MIN_ROWS, |rows, count| {
        let count = *count as i32;
        if rows < count { (count / ROW_STEP).wrapping_mul(ROW_STEP).wrapping_add(ROW_STEP) } else { rows }
    })
}

/// Paints the whole graph into a pixmap of the body's columns by the histogram's rows.
///
/// Row zero is the top of the graph. Every colour is mixed in rather than written, as the
/// reference's pixmap mixes it, so a tick darkens the window under it and a translucent bar lets
/// the lines behind it through.
fn paint(body: &TimingDistributionBody, histogram: &TimingHistogram<'_>) -> Pixmap {
    let columns = body.columns;
    let middle = columns / SIDES as i32;
    let rows = row_count(histogram.bins);
    let mut pixmap = Pixmap::new(columns as usize, rows as usize);

    pixmap.blend_rect(middle, 0, 1, rows, body.judges[0]);
    let (mut left, mut right) = (middle, middle + 1);
    for (color, [late, early]) in body.judges.iter().zip(histogram.judge_area) {
        let from = middle + late.clamp(-middle, middle);
        let until = middle + early.clamp(-middle, middle) + 1;
        if left > from {
            pixmap.blend_rect(from, 0, (from - left).abs(), rows, *color);
            left = from;
        }
        if until > right {
            pixmap.blend_rect(right, 0, (until - right).abs(), rows, *color);
            right = until;
        }
    }

    let tick = unit_color(0.0, 0.0, 0.0, TICK_ALPHA);
    for column in (middle % TICK_EVERY as i32..middle * 2 + 1).step_by(TICK_EVERY) {
        pixmap.blend_upright_line(column, 0, TICK_LAST_ROW, tick);
    }

    let mean = java_round(histogram.average);
    if body.draw_average && histogram.average != NO_AVERAGE {
        pixmap.blend_upright_line(middle.wrapping_add(mean), 0, rows, body.average);
    }
    if body.draw_deviation && histogram.std_dev != NO_DEVIATION {
        let spread = java_round(histogram.std_dev);
        pixmap.blend_upright_line(middle.wrapping_add(mean).wrapping_add(spread), 0, rows, body.deviation);
        pixmap.blend_upright_line(middle.wrapping_add(mean).wrapping_sub(spread), 0, rows, body.deviation);
    }

    let center = histogram.center();
    for offset in -middle..columns - middle {
        if -center < offset && offset < center {
            let count = histogram.bins[(center + offset) as usize] as i32;
            pixmap.blend_rect(middle + offset, rows - count, 1, count, body.graph);
        }
    }
    pixmap
}

/// Draws the timing spread over the object's rectangle.
pub(crate) fn draw_timing_distribution<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &TimingDistributionBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(histogram) = frame.data.series.timing else {
        return false;
    };
    let Some(plot) = plot_of(place, rect).filter(|_| body.columns > 0) else {
        return false;
    };
    let largest = i64::from(r.max_texture_size());
    if i64::from(body.columns) > largest || i64::from(row_count(histogram.bins)) > largest {
        return false;
    }

    let mut drawn = body.state.borrow_mut();
    if !drawn.painted.as_ref().is_some_and(|painted| painted.shows(&histogram)) {
        drawn.pixmap = paint(body, &histogram);
        drawn.painted = Some(Painted {
            bins: histogram.bins.to_vec(),
            average: histogram.average.to_bits(),
            std_dev: histogram.std_dev.to_bits(),
            judge_area: histogram.judge_area,
        });
        drawn.layer.mark_stale();
    }

    let Drawn { layer, pixmap, .. } = &mut *drawn;
    let (width, height) = pixmap.size();
    let Some(tex) = layer.texture_as(r, pixmap, Upload::FirstRowOnTop) else {
        return false;
    };
    draw_layer(r, place, tex, (width as u32, height as u32), width as u32, Rect::new(plot.x, plot.y, plot.w, plot.h))
}

#[cfg(test)]
mod tests;
