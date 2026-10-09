//! The `hiterrorvisualizer` object: the running strip of recent hit errors.
//!
//! It reads the same recent hits the timing visualiser does ([`super::RecentHits`]).

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;

use super::super::SkinFrame;
use super::super::color::modulate;
use super::super::draw::Placement;
use super::super::object::Body;
use super::{VISUALIZER_JUDGEMENTS, color_of, draw_column, error_offset, line_width, plot_of, visualizer_palette};
use crate::Color;
use crate::Renderer;
use crate::ctx::RenderCtx;

/// The faintest a decayed hit is drawn, as a share of its own alpha, so the oldest mark in the
/// window is still visible.
const OLDEST_HIT_ALPHA: f32 = 0.25;

/// The running strip of recent hit errors.
#[derive(Debug)]
pub(crate) struct HitErrorBody {
    center: Color,
    judges: [Color; VISUALIZER_JUDGEMENTS],
    average: Color,
    width: f32,
    window_ms: f32,
    /// How many of the most recent hits are kept on screen.
    window: usize,
    /// The weight one hit takes in the running average.
    smoothing: f32,
    draw_average: bool,
    /// Whether an older mark is drawn fainter than a newer one.
    decay: bool,
}

/// The hit error visualiser behind `id`, or `None` when the document declares none by that name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.hiterrorvisualizer.iter().find(|graph| graph.id == id)?;
    let colors = [&graph.pgreat_color, &graph.great_color, &graph.good_color, &graph.bad_color, &graph.poor_color];
    Some(Body::HitError(HitErrorBody {
        center: color_of(&graph.center_color, "centre", id, warnings),
        judges: visualizer_palette(colors.map(String::as_str), id, warnings),
        average: color_of(&graph.ema_color, "average", id, warnings),
        width: line_width(graph.line_width),
        window_ms: (graph.judge_width_millis.max(1)) as f32,
        window: graph.window_length.max(0) as usize,
        smoothing: graph.alpha.clamp(0.0, 1.0),
        draw_average: graph.ema_mode != 0,
        decay: graph.draw_decay != 0,
    }))
}

/// The same colour at a share of its own alpha.
fn faded(color: Color, share: f32) -> Color {
    Color { a: (f32::from(color.a) * share.clamp(0.0, 1.0)) as u8, ..color }
}

/// Draws the running hit errors, newest at full strength and the oldest of the window faintest, with
/// the smoothed average over them.
pub(crate) fn draw_hit_error<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &HitErrorBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(hits) = frame.data.series.recent_hits.map(|recent| recent.hits) else {
        return false;
    };
    let Some(plot) = plot_of(place, rect).filter(|_| !hits.is_empty()) else {
        return false;
    };

    let shown = &hits[hits.len().saturating_sub(body.window.max(1))..];
    let middle = plot.x + plot.w * 0.5;
    draw_column(r, plot, middle, body.width, modulate(body.center, place.tint));
    let mut average = 0.0;
    for (at, (error, judge)) in shown.iter().enumerate() {
        let share = if body.decay { OLDEST_HIT_ALPHA + (1.0 - OLDEST_HIT_ALPHA) * (at + 1) as f32 / shown.len() as f32 } else { 1.0 };
        let color = body.judges[usize::from(*judge).min(VISUALIZER_JUDGEMENTS - 1)];
        draw_column(r, plot, middle + error_offset(plot, *error as f32, body.window_ms), body.width, faded(modulate(color, place.tint), share));
        average += (*error as f32 - average) * body.smoothing;
    }
    if body.draw_average {
        draw_column(r, plot, middle + error_offset(plot, average, body.window_ms), body.width, modulate(body.average, place.tint));
    }
    true
}
