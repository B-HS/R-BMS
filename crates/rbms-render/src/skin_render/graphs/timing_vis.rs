//! The `timingvisualizer` object: the judge-window ruler a play document draws its recent hit errors
//! over (`SkinTimingVisualizer`).
//!
//! The ruler is a pixmap one pixel tall and two pixels more than twice the record's
//! `judgeWidthMillis` wide, painted once per set of judge windows and stretched over the object's
//! rectangle like any image. The hit errors are separate one-colour lines laid over it, oldest
//! first, and they are not the ruler's: they take the colour the record names for them rather than
//! the destination's, are never turned with the object, and are placed in screen pixels by the
//! record's raw `width` rather than by the document's scale, so a document drawn at a size other than
//! the one it was authored at has lines that no longer sit on the ruler's marks, as in the reference.

use std::cell::RefCell;

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;

use super::super::SkinFrame;
use super::super::draw::Placement;
use super::super::object::Body;
use super::pixmap::{Pixmap, Rgba, unit_color};
use super::{
    FIRST_UNRECORDED_JUDGE, Layer, RECENT_JUDGES, TIMING_JUDGE_AREAS, Upload, VISUALIZER_JUDGEMENTS, document_rect_of, next_serial, signed_plot_of,
    visualizer_color, visualizer_line_width, visualizer_palette, visualizer_rate,
};
use crate::ctx::RenderCtx;
use crate::{Color, Rect, Renderer};

/// The windows every judgement has when a screen does not know them: none, so only the middle of
/// the ruler is painted.
const NO_WINDOWS: [[i32; 2]; TIMING_JUDGE_AREAS] = [[0; 2]; TIMING_JUDGE_AREAS];

/// The hits a run has taken most recently, which is what both visualisers plot, and the judge
/// windows they plot them against.
///
/// A hit is `(error in milliseconds, judgement)` with a hit that came early positive, as the
/// reference records it (`JudgeManager.recentJudges`, from the note's time minus the press), most
/// recent last. Judgement is the reference's number, `0` perfect great to `4` poor: a hit that took
/// a poor is not recorded by the reference and is left out here too.
#[derive(Debug, Clone, Copy)]
pub struct RecentHits<'a> {
    /// The hits the run has taken, each `(error in milliseconds, judgement)`, most recent last. The
    /// last hundred are plotted, so a run that keeps fewer shows fewer.
    pub hits: &'a [(i64, u8)],
    /// How many hits the run has recorded in all, which is what tells the hit error visualiser that
    /// a new one came in when it already shows a hundred (`JudgeManager.getRecentJudgesIndex`).
    pub recorded: usize,
    /// The window of each judgement in milliseconds, best first: how late a hit may be and still
    /// take it, which is not above zero, and how early (`JudgeProperty.getNoteJudge`). A screen that
    /// does not know the windows leaves them at zero and the ruler is bare.
    pub judge_area: [[i32; 2]; TIMING_JUDGE_AREAS],
}

impl<'a> RecentHits<'a> {
    /// The recent hits of a run, each `(error in milliseconds, judgement)`, most recent last, with
    /// every hit in the slice counted as one the run recorded and no judge windows.
    pub fn new(hits: &'a [(i64, u8)]) -> RecentHits<'a> {
        let recorded = hits.iter().filter(|(_, judgement)| *judgement < FIRST_UNRECORDED_JUDGE).count();
        RecentHits { hits, recorded, judge_area: NO_WINDOWS }
    }

    /// The same hits for a run that has recorded `recorded` of them in all, of which the slice
    /// keeps the latest.
    pub fn with_recorded(mut self, recorded: usize) -> RecentHits<'a> {
        self.recorded = recorded;
        self
    }

    /// The same hits against the judge windows of the run.
    pub fn with_judge_area(mut self, judge_area: [[i32; 2]; TIMING_JUDGE_AREAS]) -> RecentHits<'a> {
        self.judge_area = judge_area;
        self
    }

    /// The errors the reference's buffer of recent hits holds, oldest first: the last hundred hits
    /// that took a judgement better than poor.
    pub(super) fn errors(&self) -> Vec<i64> {
        let recorded: Vec<i64> = self.hits.iter().filter(|(_, judgement)| *judgement < FIRST_UNRECORDED_JUDGE).map(|(error, _)| *error).collect();
        recorded[recorded.len().saturating_sub(RECENT_JUDGES)..].to_vec()
    }
}

/// The judge-window ruler a play document draws its hit errors against.
#[derive(Debug)]
pub(crate) struct TimingVisualizerBody {
    /// How many milliseconds the ruler reaches to either side of its middle (`judgeWidthMillis`).
    center: i32,
    /// Pixels to a millisecond.
    rate: f32,
    line_width: i32,
    line: Rgba,
    centre: Rgba,
    judges: [Rgba; VISUALIZER_JUDGEMENTS],
    decay: bool,
    state: RefCell<Drawn>,
}

/// What the ruler has painted, kept between frames because the reference keeps it.
#[derive(Debug)]
struct Drawn {
    ruler: Layer,
    ruler_pixmap: Pixmap,
    painted: Option<[[i32; 2]; TIMING_JUDGE_AREAS]>,
    stroke: Layer,
    stroke_pixmap: Pixmap,
}

impl TimingVisualizerBody {
    /// Hands the textures back to the renderer they were uploaded to.
    pub(crate) fn release<R: Renderer>(&self, r: &mut R) {
        let mut drawn = self.state.borrow_mut();
        drawn.ruler.release(r);
        drawn.stroke.release(r);
        drawn.painted = None;
    }
}

/// The timing visualiser behind `id`, or `None` when the document declares none by that name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.timingvisualizer.iter().find(|graph| graph.id == id)?;
    let colors = [&graph.pgreat_color, &graph.great_color, &graph.good_color, &graph.bad_color, &graph.poor_color];
    let line_width = visualizer_line_width(graph.line_width);
    let center = graph.judge_width_millis.max(0);
    Some(Body::TimingVisualizer(TimingVisualizerBody {
        center,
        rate: visualizer_rate(graph.width, center),
        line_width,
        line: visualizer_color(&graph.line_color, "line colour", id, warnings),
        centre: visualizer_color(&graph.center_color, "centre colour", id, warnings),
        judges: visualizer_palette(colors.map(String::as_str), graph.transparent, id, warnings),
        decay: graph.draw_decay == 1,
        state: RefCell::new(Drawn {
            ruler: Layer::new(next_serial(), "ruler"),
            ruler_pixmap: Pixmap::default(),
            painted: None,
            stroke: Layer::new(next_serial(), "stroke"),
            stroke_pixmap: solid_stroke(line_width),
        }),
    }))
}

/// The white stripe every hit error is drawn from, tinted by the hit's own colour.
fn solid_stroke(line_width: i32) -> Pixmap {
    let mut pixmap = Pixmap::new(line_width as usize, 1);
    pixmap.fill([u8::MAX; 4]);
    pixmap
}

/// The opacity of a tick on the ruler, black.
const TICK_ALPHA: f32 = 0.25;

/// How many pixels lie between two ticks.
const TICK_EVERY: usize = 10;

/// The last row a tick reaches. The ruler is one row, so the second row of the stroke falls outside.
const TICK_LAST_ROW: i32 = 1;

/// Paints the ruler: the centre pixel, then each judgement's window as the band added outside the
/// one before it, then a tick every ten pixels counted from the ruler's own middle.
fn paint_ruler(body: &TimingVisualizerBody, judge_area: &[[i32; 2]; TIMING_JUDGE_AREAS]) -> Pixmap {
    let center = body.center;
    let columns = center * 2 + 1;
    let mut pixmap = Pixmap::new(columns as usize, 1);
    pixmap.blend_rect(center, 0, 1, 1, body.centre);
    let (mut left, mut right) = (center, center + 1);
    for (color, [late, early]) in body.judges.iter().zip(judge_area) {
        let from = center + (*late).clamp(-center, center);
        let until = center + (*early).clamp(-center, center) + 1;
        if left > from {
            pixmap.blend_rect(from, 0, (from - left).abs(), 1, *color);
            left = from;
        }
        if until > right {
            pixmap.blend_rect(right, 0, (until - right).abs(), 1, *color);
            right = until;
        }
    }
    let tick = unit_color(0.0, 0.0, 0.0, TICK_ALPHA);
    for column in ((center % TICK_EVERY as i32) as usize..columns as usize).step_by(TICK_EVERY) {
        pixmap.blend_upright_line(column as i32, 0, TICK_LAST_ROW, tick);
    }
    pixmap
}

/// The colour of the `age`th line, counted from the oldest of a hundred: the record's line colour
/// with an alpha that grows by a hundredth of its own with every step, so the newest is at the
/// record's alpha and the oldest at a hundredth of it.
pub(super) fn stroke_color(line: Rgba, age: usize) -> Color {
    let alpha = f32::from(line[3]) / f32::from(u8::MAX) / RECENT_JUDGES as f32 * (age + 1) as f32;
    Color { r: line[0], g: line[1], b: line[2], a: (alpha * f32::from(u8::MAX)) as u8 }
}

/// Where the line for a hit that came `error` milliseconds early is drawn, `age` places from the
/// oldest of a hundred, on a ruler drawn over `plot`; `None` for a hit beyond the ruler or a line
/// that has no height.
///
/// An early hit is to the right of the middle. With decay a line grows from the middle of the
/// ruler's height as it ages, so the oldest is a speck and the newest is the ruler's whole height.
pub(super) fn stroke_rect(body: &TimingVisualizerBody, plot: Rect, error: i64, age: usize) -> Option<Rect> {
    if !(-i64::from(body.center)..=i64::from(body.center)).contains(&error) {
        return None;
    }
    let x = plot.x + (plot.w - body.line_width as f32) / 2.0 + error as f32 * body.rate;
    let width = body.line_width as f32;
    let rect = if body.decay {
        let total = RECENT_JUDGES as f32;
        Rect::new(x, plot.y + plot.h * (RECENT_JUDGES - age) as f32 / total / 2.0, width, plot.h * age as f32 / total)
    } else {
        Rect::new(x, plot.y, width, plot.h)
    };
    (rect.h != 0.0).then_some(rect)
}

/// Draws the ruler, and over it every recent hit as a line.
///
/// A destination of negative width mirrors the ruler and keeps the lines where they were: the ruler
/// is an image like any other, and a line is placed from the destination's own corner by the signed
/// width.
pub(crate) fn draw_timing_visualizer<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &TimingVisualizerBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(recent) = frame.data.series.recent_hits else {
        return false;
    };
    let Some(plot) = signed_plot_of(place, rect) else {
        return false;
    };
    let columns = body.center as i64 * 2 + 1;
    if columns > i64::from(r.max_texture_size()) {
        return false;
    }

    let mut drawn = body.state.borrow_mut();
    if drawn.painted != Some(recent.judge_area) {
        drawn.ruler_pixmap = paint_ruler(body, &recent.judge_area);
        drawn.painted = Some(recent.judge_area);
        drawn.ruler.mark_stale();
    }
    let Drawn { ruler, ruler_pixmap, stroke, stroke_pixmap, .. } = &mut *drawn;
    let (ruler_w, ruler_h) = ruler_pixmap.size();
    let mut reached = match ruler.texture_as(r, ruler_pixmap, Upload::FirstRowOnTop) {
        Some(tex) => place.texture(r, tex, (ruler_w as u32, ruler_h as u32), SkinRect::new(0.0, 0.0, ruler_w as f32, ruler_h as f32), rect),
        None => false,
    };

    let Some(stripe) = stroke.texture_as(r, stroke_pixmap, Upload::FirstRowOnTop) else {
        return reached;
    };
    let (stripe_w, stripe_h) = stroke_pixmap.size();
    let whole = SkinRect::new(0.0, 0.0, stripe_w as f32, stripe_h as f32);
    let errors = recent.errors();
    let first_age = RECENT_JUDGES - errors.len();
    for (at, error) in errors.iter().enumerate() {
        let age = first_age + at;
        let color = stroke_color(body.line, age);
        let Some(line) = stroke_rect(body, plot, *error, age).filter(|_| color.a > 0).and_then(|line| document_rect_of(place, line)) else {
            continue;
        };
        let upright = Placement { object: place.object, blend: place.blend, tint: color, angle_deg: 0.0, viewport: place.viewport };
        reached |= upright.texture(r, stripe, (stripe_w as u32, stripe_h as u32), whole, line);
    }
    reached
}

#[cfg(test)]
mod tests;
