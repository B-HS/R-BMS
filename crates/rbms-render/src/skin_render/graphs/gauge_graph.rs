//! The `gaugegraph` object: the gauge history of a run.

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;

use super::super::SkinFrame;
use super::super::color::modulate;
use super::super::draw::Placement;
use super::super::object::Body;
use super::{color_of, plot_of};
use crate::ctx::RenderCtx;
use crate::{Color, Rect, Renderer};

/// Thickness of a panel's border.
const BORDER_W: f32 = 1.0;

/// Percent of a full gauge, which is the scale a gauge history is plotted against.
const GAUGE_FULL: f32 = 100.0;

/// Width of one plotted sample of the gauge history.
const SAMPLE_W: f32 = 1.0;

/// Height of one plotted sample of the gauge history, so a flat run still reads as a line.
const SAMPLE_H: f32 = 2.0;

/// The gauge of a run at each sample taken of it, which is what a gauge graph plots.
///
/// Any screen that knows a run's gauge may hand this over: the score screen for the run that just
/// ended, the play screen for the one in progress.
#[derive(Debug, Clone, Copy)]
pub struct GaugeHistory<'a> {
    /// The gauge at each sample of the run, oldest first, in the percent the views carry it as.
    pub samples: &'a [f32],
}

impl<'a> GaugeHistory<'a> {
    /// A history from its samples, oldest first, in percent.
    pub fn new(samples: &'a [f32]) -> GaugeHistory<'a> {
        GaugeHistory { samples }
    }
}

/// The gauge history of a finished run.
///
/// The record carries a background and a line colour for each of the six clear bands, and the
/// history carries no gauge kind to pick one with, so the groove pair is what a history is plotted
/// in. Picking the band the run was actually played on is a follow-up on the state rather than on
/// this drawing.
#[derive(Debug)]
pub(crate) struct GaugeGraphBody {
    background: Color,
    line: Color,
    border: Color,
}

/// The gauge graph behind `id`, or `None` when the document declares none by that name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.gaugegraph.iter().find(|graph| graph.id == id)?;
    Some(Body::GaugeGraph(GaugeGraphBody {
        background: color_of(&graph.groove_clear_and_hard_bg_color, "background", id, warnings),
        line: color_of(&graph.groove_clear_and_hard_line_color, "line", id, warnings),
        border: color_of(&graph.border_color, "border", id, warnings),
    }))
}

/// Draws the four edges of a panel, each inside it.
fn draw_border<R: Renderer>(r: &mut R, plot: Rect, color: Color) {
    r.fill_rect(Rect::new(plot.x, plot.y, plot.w, BORDER_W), color);
    r.fill_rect(Rect::new(plot.x, plot.y + plot.h - BORDER_W, plot.w, BORDER_W), color);
    r.fill_rect(Rect::new(plot.x, plot.y, BORDER_W, plot.h), color);
    r.fill_rect(Rect::new(plot.x + plot.w - BORDER_W, plot.y, BORDER_W, plot.h), color);
}

/// Draws the gauge history, answering whether anything reached the screen.
///
/// One column of the plot is one sample of the series picked by position, so a long run and a short
/// one both fill the panel; the same shape the built-in trend draws.
pub(crate) fn draw_gauge_graph<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &GaugeGraphBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(series) = frame.data.series.gauge_history.map(|history| history.samples) else {
        return false;
    };
    let (Some(plot), Some(last)) = (plot_of(place, rect), series.len().checked_sub(1)) else {
        return false;
    };

    r.fill_rect(plot, modulate(body.background, place.tint));
    let columns = (plot.w as usize).max(1);
    let span = columns.saturating_sub(1).max(1);
    let line = modulate(body.line, place.tint);
    for column in 0..columns {
        let level = (series[column * last / span] / GAUGE_FULL).clamp(0.0, 1.0);
        let top = plot.y + (plot.h - SAMPLE_H) * (1.0 - level);
        r.fill_rect(Rect::new(plot.x + column as f32, top, SAMPLE_W, SAMPLE_H), line);
    }
    draw_border(r, plot, modulate(body.border, place.tint));
    true
}
