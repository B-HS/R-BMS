//! The `timingdistributiongraph` object: the histogram of a run's timing spread.

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;

use super::super::SkinFrame;
use super::super::color::modulate;
use super::super::draw::Placement;
use super::super::object::Body;
use super::{BAR_GAP, MIN_BAR_W, color_of, draw_column, line_width, plot_of};
use crate::ctx::RenderCtx;
use crate::{Color, Rect, Renderer};

/// How a run's hits were spread around the moment each note was due, which is what a timing
/// distribution graph plots.
#[derive(Debug, Clone, Copy)]
pub struct TimingHistogram<'a> {
    /// How many hits landed in each timing bucket, earliest bucket first.
    pub bins: &'a [u32],
}

impl<'a> TimingHistogram<'a> {
    /// A histogram from its buckets, earliest first.
    pub fn new(bins: &'a [u32]) -> TimingHistogram<'a> {
        TimingHistogram { bins }
    }
}

/// The histogram of a run's timing spread.
#[derive(Debug)]
pub(crate) struct TimingDistributionBody {
    bars: Color,
    average: Color,
    deviation: Color,
    width: f32,
    draw_average: bool,
    draw_deviation: bool,
}

/// The timing distribution graph behind `id`, or `None` when the document declares none by that
/// name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.timingdistributiongraph.iter().find(|graph| graph.id == id)?;
    Some(Body::TimingDistribution(TimingDistributionBody {
        bars: color_of(&graph.graph_color, "bars", id, warnings),
        average: color_of(&graph.average_color, "average", id, warnings),
        deviation: color_of(&graph.dev_color, "deviation", id, warnings),
        width: line_width(graph.line_width),
        draw_average: graph.draw_average != 0,
        draw_deviation: graph.draw_dev != 0,
    }))
}

/// Draws a histogram over `plot`, scaled to its own tallest bucket and growing from the bottom, and
/// answers whether it had anything to draw. An empty or all-zero histogram draws nothing, because a
/// flat row of nothing reads as a measurement of zero rather than as no measurement.
fn draw_histogram<R: Renderer>(r: &mut R, plot: Rect, bins: &[u32], color: Color, gap: f32) -> bool {
    let Some(peak) = bins.iter().copied().max().filter(|peak| *peak > 0) else {
        return false;
    };
    let pitch = plot.w / bins.len() as f32;
    for (at, count) in bins.iter().enumerate() {
        let height = plot.h * *count as f32 / peak as f32;
        if height <= 0.0 {
            continue;
        }
        r.fill_rect(Rect::new(plot.x + pitch * at as f32, plot.y + plot.h - height, (pitch - gap).max(MIN_BAR_W), height), color);
    }
    true
}

/// Draws the timing spread, with the run's mean and its two deviation marks over it.
pub(crate) fn draw_timing_distribution<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &TimingDistributionBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(hist) = frame.data.series.timing.map(|timing| timing.bins) else {
        return false;
    };
    let Some(plot) = plot_of(place, rect) else {
        return false;
    };
    if !draw_histogram(r, plot, hist, modulate(body.bars, place.tint), BAR_GAP) {
        return false;
    }

    let pitch = plot.w / hist.len() as f32;
    let total: f64 = hist.iter().map(|count| f64::from(*count)).sum();
    let mean = hist.iter().enumerate().map(|(at, count)| at as f64 * f64::from(*count)).sum::<f64>() / total;
    let variance = hist.iter().enumerate().map(|(at, count)| f64::from(*count) * (at as f64 - mean).powi(2)).sum::<f64>() / total;
    let center = plot.x + pitch * (mean as f32 + 0.5);
    if body.draw_average {
        draw_column(r, plot, center, body.width, modulate(body.average, place.tint));
    }
    if body.draw_deviation {
        let spread = pitch * variance.sqrt() as f32;
        let deviation = modulate(body.deviation, place.tint);
        draw_column(r, plot, center - spread, body.width, deviation);
        draw_column(r, plot, center + spread, body.width, deviation);
    }
    true
}
