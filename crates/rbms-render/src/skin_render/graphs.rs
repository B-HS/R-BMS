//! The graphs and visualisers a document draws instead of the built-in panels: the gauge history,
//! the judgement spread, the tempo timeline, the timing distribution, the running hit error and the
//! note density of the focused chart.
//!
//! Each takes its numbers from the screen-shaped state beside the frame's scalar source -- a
//! property id answers one value and none of these is one value -- and its colours from the record
//! the document declared it with. Every one draws with `fill_rect` rather than a line primitive,
//! because the renderer has no line primitive: a line here is a rectangle one `lineWidth` thick.
//!
//! A destination's own colour modulates whatever the record named, so a document fades a whole graph
//! out through the destination it placed it at and leaves it alone by drawing it white, which is
//! what an unstated destination colour already is.

use rbms_skin::dst::SkinRect;
use rbms_skin::loader::LoadedSkin;

use super::color::{modulate, parse_hex_color};
use super::draw::Placement;
use super::object::{Body, Source};
use super::{SkinAssets, SkinFrame};
use crate::ctx::RenderCtx;
use crate::result::ResultPalette;
use crate::select::{DensityView, SelectDetail};
use crate::{Color, Rect, Renderer};

/// How many judgements a run is counted in, best first.
const JUDGEMENTS: usize = 6;

/// How many judgements the visualiser palettes name: they stop at a poor, because a miss is what a
/// note that was never hit takes and a hit error is only recorded for a note that was.
const VISUALIZER_JUDGEMENTS: usize = 5;

/// The colour a graph falls back to when the document wrote something that is not one.
const FALLBACK_COLOR: Color = Color::WHITE;

/// Thinnest a line is drawn, so a document that asked for none still leaves a mark.
const MIN_LINE_W: f32 = 1.0;

/// Thickness of a panel's border.
const BORDER_W: f32 = 1.0;

/// Percent of a full gauge, which is the scale a gauge history is plotted against.
const GAUGE_FULL: f32 = 100.0;

/// Width of one plotted sample of the gauge history.
const SAMPLE_W: f32 = 1.0;

/// Height of one plotted sample of the gauge history, so a flat run still reads as a line.
const SAMPLE_H: f32 = 2.0;

/// Narrowest a histogram bar is drawn, so a full histogram still shows every bucket.
const MIN_BAR_W: f32 = 1.0;

/// Gap between two histogram bars, taken off the bar rather than added to the pitch.
const BAR_GAP: f32 = 1.0;

/// The judgement graph the reference counts note kinds into, one column per second of the chart.
///
/// rbms keeps no per-second note breakdown of a finished run, so the spread it does keep is drawn
/// instead: one bar per judgement, side by side. This is the shape a document gets by default and
/// the one anything unrecognised falls back to.
const JUDGE_GRAPH_BY_NOTE: i32 = 0;

/// The judgement graph the reference counts each note's judgement into, stacking the six of them
/// into one column. rbms measures exactly that spread, over the run rather than over each second,
/// so it is drawn as the one column those six share.
const JUDGE_GRAPH_BY_JUDGEMENT: i32 = 1;

/// Tempo difference below which two points count as the same tempo, which is what decides whether a
/// segment is the chart's main tempo or one of its extremes.
const BPM_EPSILON: f64 = 0.001;

/// The faintest a decayed hit is drawn, as a share of its own alpha, so the oldest mark in the
/// window is still visible.
const OLDEST_HIT_ALPHA: f32 = 0.25;

/// The colour the document wrote for `what`, or a plain white one with a warning when the text is
/// not a colour at all.
fn color_of(text: &str, what: &str, id: &str, warnings: &mut Vec<String>) -> Color {
    match parse_hex_color(text) {
        Some(color) => color,
        None => {
            warnings.push(format!("graph {id:?} writes its {what} as {text:?}, which is not a colour"));
            FALLBACK_COLOR
        }
    }
}

/// The five judgement colours a visualiser palette names, best first.
fn visualizer_palette(colors: [&str; VISUALIZER_JUDGEMENTS], id: &str, warnings: &mut Vec<String>) -> [Color; VISUALIZER_JUDGEMENTS] {
    const NAMES: [&str; VISUALIZER_JUDGEMENTS] = ["PGColor", "GRColor", "GDColor", "BDColor", "PRColor"];
    std::array::from_fn(|index| color_of(colors[index], NAMES[index], id, warnings))
}

/// How thick a document asked one of a graph's lines to be.
fn line_width(declared: i32) -> f32 {
    (declared as f32).max(MIN_LINE_W)
}

/// The same colour at a share of its own alpha.
fn faded(color: Color, share: f32) -> Color {
    Color { a: (f32::from(color.a) * share.clamp(0.0, 1.0)) as u8, ..color }
}

/// The screen rectangle a graph plots inside, or `None` when its destination has no extent this
/// frame.
fn plot_of(place: &Placement<'_>, rect: SkinRect) -> Option<Rect> {
    let dst = place.viewport.place(rect);
    (dst.w > 0.0 && dst.h > 0.0).then_some(dst)
}

/// Draws the four edges of a panel, each inside it.
fn draw_border<R: Renderer>(r: &mut R, plot: Rect, color: Color) {
    r.fill_rect(Rect::new(plot.x, plot.y, plot.w, BORDER_W), color);
    r.fill_rect(Rect::new(plot.x, plot.y + plot.h - BORDER_W, plot.w, BORDER_W), color);
    r.fill_rect(Rect::new(plot.x, plot.y, BORDER_W, plot.h), color);
    r.fill_rect(Rect::new(plot.x + plot.w - BORDER_W, plot.y, BORDER_W, plot.h), color);
}

/// Draws one upright line across a plot, centred on `x`.
fn draw_column<R: Renderer>(r: &mut R, plot: Rect, x: f32, width: f32, color: Color) {
    r.fill_rect(Rect::new(x - width * 0.5, plot.y, width, plot.h), color);
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

/// The gauge history of a finished run.
///
/// The record carries a background and a line colour for each of the six clear bands, and the score
/// screen's series carries no gauge kind to pick one with, so the groove pair is what a history is
/// plotted in. Picking the band the run was actually played on is a follow-up on the state rather
/// than on this drawing.
#[derive(Debug)]
pub(crate) struct GaugeGraphBody {
    background: Color,
    line: Color,
    border: Color,
}

/// How a run's judgements were spread over its notes.
///
/// The reference draws these bars from a texture and its record names no palette at all, so they are
/// drawn in the built-in judgement colours, which a document tints through its destination.
#[derive(Debug)]
pub(crate) struct JudgeGraphBody {
    colors: [Color; JUDGEMENTS],
    /// Whether the worst judgement is drawn first.
    reversed: bool,
    gap: f32,
    /// Whether the six judgements share one column instead of taking a bar each.
    stacked: bool,
}

/// The tempo timeline along a chart's length.
#[derive(Debug)]
pub(crate) struct BpmGraphBody {
    main: Color,
    lowest: Color,
    highest: Color,
    other: Color,
    transition: Color,
    width: f32,
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

/// The per-second note density of the focused chart.
#[derive(Debug)]
pub(crate) struct DensityBody {
    bar: Color,
    peak: Color,
    width: f32,
}

/// The graph behind `id`, or `None` when the document declares none by that name.
pub(crate) fn build_graph(
    skin: &LoadedSkin,
    id: &str,
    _sources: Source<'_>,
    _families: &[(String, String)],
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = &skin.def;
    if let Some(graph) = def.gaugegraph.iter().find(|graph| graph.id == id) {
        return Some(Body::GaugeGraph(GaugeGraphBody {
            background: color_of(&graph.groove_clear_and_hard_bg_color, "background", id, warnings),
            line: color_of(&graph.groove_clear_and_hard_line_color, "line", id, warnings),
            border: color_of(&graph.border_color, "border", id, warnings),
        }));
    }
    if let Some(graph) = def.judgegraph.iter().find(|graph| graph.id == id) {
        let gap = if graph.no_gap == 0 { BAR_GAP } else { 0.0 };
        let kind = match graph.graph_type {
            JUDGE_GRAPH_BY_NOTE | JUDGE_GRAPH_BY_JUDGEMENT => graph.graph_type,
            other => {
                warnings.push(format!(
                    "judgement graph {id:?} asks for type {other}, which rbms records nothing for, so it is drawn as type {JUDGE_GRAPH_BY_NOTE}"
                ));
                JUDGE_GRAPH_BY_NOTE
            }
        };
        return Some(Body::JudgeGraph(JudgeGraphBody {
            colors: ResultPalette::default().judge_colors,
            reversed: graph.order_reverse != 0,
            gap,
            stacked: kind == JUDGE_GRAPH_BY_JUDGEMENT,
        }));
    }
    if let Some(graph) = def.bpmgraph.iter().find(|graph| graph.id == id) {
        return Some(Body::BpmGraph(BpmGraphBody {
            main: color_of(&graph.main_bpm_color, "main tempo", id, warnings),
            lowest: color_of(&graph.min_bpm_color, "lowest tempo", id, warnings),
            highest: color_of(&graph.max_bpm_color, "highest tempo", id, warnings),
            other: color_of(&graph.other_bpm_color, "other tempo", id, warnings),
            transition: color_of(&graph.transition_line_color, "transition", id, warnings),
            width: line_width(graph.line_width),
        }));
    }
    if let Some(graph) = def.timingdistributiongraph.iter().find(|graph| graph.id == id) {
        return Some(Body::TimingDistribution(TimingDistributionBody {
            bars: color_of(&graph.graph_color, "bars", id, warnings),
            average: color_of(&graph.average_color, "average", id, warnings),
            deviation: color_of(&graph.dev_color, "deviation", id, warnings),
            width: line_width(graph.line_width),
            draw_average: graph.draw_average != 0,
            draw_deviation: graph.draw_dev != 0,
        }));
    }
    if let Some(graph) = def.timingvisualizer.iter().find(|graph| graph.id == id) {
        let colors = [&graph.pgreat_color, &graph.great_color, &graph.good_color, &graph.bad_color, &graph.poor_color];
        return Some(Body::TimingVisualizer(TimingVisualizerBody {
            line: color_of(&graph.line_color, "ruler", id, warnings),
            center: color_of(&graph.center_color, "centre", id, warnings),
            judges: visualizer_palette(colors.map(String::as_str), id, warnings),
            width: line_width(graph.line_width),
            window_ms: (graph.judge_width_millis.max(1)) as f32,
        }));
    }
    if let Some(graph) = def.hiterrorvisualizer.iter().find(|graph| graph.id == id) {
        let colors = [&graph.pgreat_color, &graph.great_color, &graph.good_color, &graph.bad_color, &graph.poor_color];
        return Some(Body::HitError(HitErrorBody {
            center: color_of(&graph.center_color, "centre", id, warnings),
            judges: visualizer_palette(colors.map(String::as_str), id, warnings),
            average: color_of(&graph.ema_color, "average", id, warnings),
            width: line_width(graph.line_width),
            window_ms: (graph.judge_width_millis.max(1)) as f32,
            window: graph.window_length.max(0) as usize,
            smoothing: graph.alpha.clamp(0.0, 1.0),
            draw_average: graph.ema_mode != 0,
            decay: graph.draw_decay != 0,
        }));
    }
    let graph = def.densitygraph.iter().find(|graph| graph.id == id)?;
    Some(Body::Density(DensityBody {
        bar: color_of(&graph.bar_color, "bars", id, warnings),
        peak: color_of(&graph.peak_color, "peak", id, warnings),
        width: line_width(graph.line_width),
    }))
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
    let Some(series) = frame.extra.result().map(|state| state.gauge_series) else {
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

/// Draws the judgement spread, in whichever of the two shapes the document's record asked for.
pub(crate) fn draw_judge_graph<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &JudgeGraphBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(counts) = frame.extra.result().map(|state| state.judge_dist) else {
        return false;
    };
    let Some(plot) = plot_of(place, rect) else {
        return false;
    };
    if body.stacked { draw_judge_stack(r, place, body, plot, counts) } else { draw_judge_bars(r, place, body, plot, counts) }
}

/// One bar per judgement, side by side, each scaled to the commonest of them.
fn draw_judge_bars<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &JudgeGraphBody, plot: Rect, counts: &[u32; JUDGEMENTS]) -> bool {
    let Some(peak) = counts.iter().copied().max().filter(|peak| *peak > 0) else {
        return false;
    };
    let pitch = plot.w / JUDGEMENTS as f32;
    for (judgement, count) in counts.iter().enumerate() {
        let height = plot.h * *count as f32 / peak as f32;
        if height <= 0.0 {
            continue;
        }
        let at = if body.reversed { JUDGEMENTS - 1 - judgement } else { judgement };
        let x = plot.x + pitch * at as f32;
        r.fill_rect(Rect::new(x, plot.y + plot.h - height, (pitch - body.gap).max(MIN_BAR_W), height), modulate(body.colors[judgement], place.tint));
    }
    true
}

/// One column the six judgements share, each taking the share of the run it was given, stacked from
/// the bottom up with the best judgement at the foot of it unless the record reversed the order.
fn draw_judge_stack<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &JudgeGraphBody, plot: Rect, counts: &[u32; JUDGEMENTS]) -> bool {
    let judged: u32 = counts.iter().sum();
    if judged == 0 {
        return false;
    }
    let width = (plot.w - body.gap).max(MIN_BAR_W);
    let mut foot = plot.y + plot.h;
    for step in 0..JUDGEMENTS {
        let judgement = if body.reversed { JUDGEMENTS - 1 - step } else { step };
        let height = plot.h * counts[judgement] as f32 / judged as f32;
        if height <= 0.0 {
            continue;
        }
        foot -= height;
        r.fill_rect(Rect::new(plot.x, foot, width, height), modulate(body.colors[judgement], place.tint));
    }
    true
}

/// Which of the tempo colours one point takes: the chart's own tempo, one of its two extremes, or
/// anything else it passes through.
fn bpm_color(body: &BpmGraphBody, bpm: f64, main: f64, lowest: f64, highest: f64) -> Color {
    let same = |left: f64, right: f64| (left - right).abs() < BPM_EPSILON;
    if same(bpm, main) {
        return body.main;
    }
    if same(bpm, highest) {
        return body.highest;
    }
    if same(bpm, lowest) {
        return body.lowest;
    }
    body.other
}

/// Draws the tempo timeline as a step line: one flat run per tempo, joined by an upright at each
/// change, which is what makes a tempo change read as a step rather than a slope.
pub(crate) fn draw_bpm_graph<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &BpmGraphBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(points) = frame.extra.result().map(|state| state.bpm_points) else {
        return false;
    };
    let Some(plot) = plot_of(place, rect).filter(|_| !points.is_empty()) else {
        return false;
    };

    let main = points[0].1;
    let lowest = points.iter().map(|(_, bpm)| *bpm).fold(f64::INFINITY, f64::min);
    let highest = points.iter().map(|(_, bpm)| *bpm).fold(f64::NEG_INFINITY, f64::max);
    let span = (highest - lowest).max(BPM_EPSILON);
    let travel = (plot.h - body.width).max(0.0);
    let level_of = |bpm: f64| plot.y + plot.h - body.width - travel * ((bpm - lowest) / span) as f32;
    let x_of = |progress: f32| plot.x + plot.w * progress.clamp(0.0, 1.0);

    for (at, (progress, bpm)) in points.iter().enumerate() {
        let start = x_of(*progress);
        let end = points.get(at + 1).map_or(plot.x + plot.w, |(next, _)| x_of(*next));
        let level = level_of(*bpm);
        r.fill_rect(Rect::new(start, level, (end - start).max(body.width), body.width), modulate(bpm_color(body, *bpm, main, lowest, highest), place.tint));
        if let Some((_, next)) = points.get(at + 1) {
            let (top, bottom) = (level.min(level_of(*next)), level.max(level_of(*next)));
            r.fill_rect(Rect::new(end, top, body.width, bottom - top + body.width), modulate(body.transition, place.tint));
        }
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
    let Some(hist) = frame.extra.result().map(|state| state.timing_hist) else {
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

/// How far from the middle of a ruler one timing error lands, in pixels.
fn error_offset(plot: Rect, error_ms: f32, window_ms: f32) -> f32 {
    plot.w * 0.5 * (error_ms / window_ms).clamp(-1.0, 1.0)
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
    let Some(hits) = frame.extra.play().map(|state| state.recent_hits) else {
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
    let Some(hits) = frame.extra.play().map(|state| state.recent_hits) else {
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

/// The focused chart's density, when a chart rather than a folder is focused and it was measured.
fn density_of<'a>(frame: &'a SkinFrame<'_>) -> Option<&'a DensityView> {
    match frame.extra.select()?.detail {
        SelectDetail::Song(detail) => detail.density.as_ref(),
        _ => None,
    }
}

/// Draws the note-density histogram, with the tallest second of the chart marked across it.
pub(crate) fn draw_density<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &DensityBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let (Some(density), Some(plot)) = (density_of(frame), plot_of(place, rect)) else {
        return false;
    };
    if !draw_histogram(r, plot, &density.bins, modulate(body.bar, place.tint), BAR_GAP) {
        return false;
    }
    r.fill_rect(Rect::new(plot.x, plot.y, plot.w, body.width), modulate(body.peak, place.tint));
    true
}
