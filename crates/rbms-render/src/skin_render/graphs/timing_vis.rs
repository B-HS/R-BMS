//! The `timingvisualizer` object: the judge-window ruler a play document draws its hit errors
//! against.

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;

use super::super::SkinFrame;
use super::super::color::modulate;
use super::super::draw::Placement;
use super::super::object::Body;
use super::{VISUALIZER_JUDGEMENTS, color_of, draw_column, error_offset, line_width, plot_of, visualizer_palette};
use crate::ctx::RenderCtx;
use crate::{Color, Rect, Renderer};

/// The hits a run has taken most recently, which is what both visualisers plot.
#[derive(Debug, Clone, Copy)]
pub struct RecentHits<'a> {
    /// Each hit as `(error in milliseconds, judgement)`, most recent last.
    pub hits: &'a [(i64, u8)],
}

impl<'a> RecentHits<'a> {
    /// The recent hits of a run, each `(error in milliseconds, judgement)`, most recent last.
    pub fn new(hits: &'a [(i64, u8)]) -> RecentHits<'a> {
        RecentHits { hits }
    }
}

/// The judge-window ruler a play document draws its hit errors against.
///
/// rbms measures no judge window a document could be handed, so the bands are the widest error each
/// judgement has actually taken in the run so far: the window as the run itself has shown it.
#[derive(Debug)]
pub(crate) struct TimingVisualizerBody {
    line: Color,
    center: Color,
    judges: [Color; VISUALIZER_JUDGEMENTS],
    width: f32,
    window_ms: f32,
}

/// The timing visualiser behind `id`, or `None` when the document declares none by that name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.timingvisualizer.iter().find(|graph| graph.id == id)?;
    let colors = [&graph.pgreat_color, &graph.great_color, &graph.good_color, &graph.bad_color, &graph.poor_color];
    Some(Body::TimingVisualizer(TimingVisualizerBody {
        line: color_of(&graph.line_color, "ruler", id, warnings),
        center: color_of(&graph.center_color, "centre", id, warnings),
        judges: visualizer_palette(colors.map(String::as_str), id, warnings),
        width: line_width(graph.line_width),
        window_ms: (graph.judge_width_millis.max(1)) as f32,
    }))
}

/// Draws the judge-window ruler: the widest error each judgement has taken, widest band first so the
/// tighter ones stay on top, then the ruler itself and the mark a note landed exactly on.
pub(crate) fn draw_timing_visualizer<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &TimingVisualizerBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(hits) = frame.data.series.recent_hits.map(|recent| recent.hits) else {
        return false;
    };
    let Some(plot) = plot_of(place, rect) else {
        return false;
    };

    let widest = |judgement: usize| {
        hits.iter()
            .filter(|(_, judge)| usize::from(*judge).min(VISUALIZER_JUDGEMENTS - 1) == judgement)
            .map(|(error, _)| error.unsigned_abs() as f32)
            .fold(0.0_f32, f32::max)
    };
    for judgement in (0..VISUALIZER_JUDGEMENTS).rev() {
        let half = error_offset(plot, widest(judgement), body.window_ms);
        if half <= 0.0 {
            continue;
        }
        r.fill_rect(Rect::new(plot.x + plot.w * 0.5 - half, plot.y, half * 2.0, plot.h), modulate(body.judges[judgement], place.tint));
    }
    r.fill_rect(Rect::new(plot.x, plot.y + (plot.h - body.width) * 0.5, plot.w, body.width), modulate(body.line, place.tint));
    draw_column(r, plot, plot.x + plot.w * 0.5, body.width, modulate(body.center, place.tint));
    true
}
