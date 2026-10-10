//! The `hiterrorvisualizer` object: the running strip of recent hit errors with a smoothed average
//! over it (`SkinHitErrorVisualizer`).
//!
//! The strip is a pixmap of the record's `width` by twice its `windowLength`, repainted whenever the
//! run has recorded another hit and drawn over the object's rectangle like any image. It reads the
//! same recent hits the timing visualiser does ([`super::RecentHits`]) but plots them the other way
//! round: a hit that came early is to the left of the middle line here and to the right on the
//! ruler. The smoothed average is kept between repaints, so a run that records two hits between two
//! frames feeds it only the later one, as the reference does.

use std::cell::RefCell;

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;

use super::super::SkinFrame;
use super::super::draw::Placement;
use super::super::object::Body;
use super::pixmap::{Pixmap, Rgba};
use super::{
    Layer, RECENT_JUDGES, TIMING_JUDGE_AREAS, Upload, VISUALIZER_JUDGEMENTS, next_serial, signed_plot_of, visualizer_color, visualizer_line_width,
    visualizer_palette, visualizer_rate,
};
use crate::Renderer;
use crate::ctx::RenderCtx;

/// The record value that switches a mode on.
const SWITCH_ON: i32 = 1;

/// The fewest and the most hits a strip keeps on show (`MathUtils.clamp(windowLength, 1, 100)`).
const MIN_WINDOW: i32 = 1;

/// How many pixel rows a strip has to each hit of its window.
const ROWS_PER_HIT: i32 = 2;

/// The `emaMode` values that draw the line, the triangle, or both.
const EMA_LINE: i32 = 1;
const EMA_TRIANGLE: i32 = 2;
const EMA_BOTH: i32 = 3;

/// How wide the average's triangle is, as a share of the strip's width.
const TRIANGLE_WIDTH_SHARE: f64 = 0.01;

/// How tall the average's triangle is, as the share of the strip's height it reaches down.
const TRIANGLE_HEIGHT_DIVISOR: i32 = 3;

/// The window of the best judgement a hit has to be inside of for the average to count it: the
/// bad window, the widest a recorded hit can be in.
const AVERAGE_WINDOW: usize = VISUALIZER_JUDGEMENTS - 2;

/// How the strip's hit marks are coloured when the record does not colour them by judgement: its
/// line colour, with an alpha that grows with the mark's age (`lineColor.a * i / (windowLength / 2)`).
const FADE_HALVES: f32 = 2.0;

/// The running strip of recent hit errors.
#[derive(Debug)]
pub(crate) struct HitErrorBody {
    width: i32,
    center: i32,
    rate: f32,
    line_width: i32,
    /// How many of the most recent hits are kept on screen.
    window: i32,
    /// The record's `emaMode`: 0 draws no average, 1 a line, 2 a triangle, 3 both.
    ema_mode: i32,
    /// The weight one hit takes in the running average.
    alpha: f32,
    /// Whether marks are drawn at all (`hiterrorMode == 1`).
    marks: bool,
    /// Whether marks are coloured by the window they landed in (`colorMode == 1`) or all in the
    /// line colour.
    by_judgement: bool,
    /// Whether an older mark is shorter than a newer one.
    decay: bool,
    line: Rgba,
    centre: Rgba,
    average: Rgba,
    judges: [Rgba; VISUALIZER_JUDGEMENTS],
    state: RefCell<Drawn>,
}

/// What the strip has painted, kept between frames because the reference keeps it.
#[derive(Debug)]
struct Drawn {
    layer: Layer,
    pixmap: Pixmap,
    /// The slot of the recent-hit buffer the strip was last painted at.
    index: Option<usize>,
    /// How many hits the run had recorded then.
    seen: usize,
    /// The judge windows the strip was painted against.
    judge_area: Option<[[i32; 2]; TIMING_JUDGE_AREAS]>,
    /// The smoothed average of the errors the strip has been fed, in milliseconds.
    ema: i64,
}

impl HitErrorBody {
    /// Hands the texture back to the renderer it was uploaded to.
    pub(crate) fn release<R: Renderer>(&self, r: &mut R) {
        let mut drawn = self.state.borrow_mut();
        drawn.layer.release(r);
        drawn.index = None;
    }
}

/// The hit error visualiser behind `id`, or `None` when the document declares none by that name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.hiterrorvisualizer.iter().find(|graph| graph.id == id)?;
    let colors = [&graph.pgreat_color, &graph.great_color, &graph.good_color, &graph.bad_color, &graph.poor_color];
    let center = graph.judge_width_millis.max(0);
    if graph.width <= 0 {
        warnings.push(format!("hit error visualiser {id:?} asks for a width of {}, which leaves it no strip, so it draws nothing", graph.width));
    }
    Some(Body::HitError(HitErrorBody {
        width: graph.width,
        center,
        rate: visualizer_rate(graph.width, center),
        line_width: visualizer_line_width(graph.line_width),
        window: graph.window_length.clamp(MIN_WINDOW, RECENT_JUDGES as i32),
        ema_mode: graph.ema_mode,
        alpha: graph.alpha,
        marks: graph.hiterror_mode == SWITCH_ON,
        by_judgement: graph.color_mode == SWITCH_ON,
        decay: graph.draw_decay == SWITCH_ON,
        line: visualizer_color(&graph.line_color, "line colour", id, warnings),
        centre: visualizer_color(&graph.center_color, "centre colour", id, warnings),
        average: visualizer_color(&graph.ema_color, "average colour", id, warnings),
        judges: visualizer_palette(colors.map(String::as_str), graph.transparent, id, warnings),
        state: RefCell::new(Drawn { layer: Layer::new(next_serial(), "hit-error"), pixmap: Pixmap::default(), index: None, seen: 0, judge_area: None, ema: 0 }),
    }))
}

/// Whether `error` lies strictly inside the window of the judgement numbered `judgement`.
fn within(error: i64, judge_area: &[[i32; 2]; TIMING_JUDGE_AREAS], judgement: usize) -> bool {
    let [late, early] = judge_area[judgement];
    error > i64::from(late) && error < i64::from(early)
}

/// The colour a mark is painted in when the record colours by judgement: that of the first window
/// the error is strictly inside, and the poor colour for an error inside none of them.
fn judgement_color(body: &HitErrorBody, judge_area: &[[i32; 2]; TIMING_JUDGE_AREAS], error: i64) -> Rgba {
    let judgement = (0..=AVERAGE_WINDOW).find(|judgement| within(error, judge_area, *judgement)).unwrap_or(VISUALIZER_JUDGEMENTS - 1);
    body.judges[judgement]
}

/// The colour of the mark `age` places from the oldest of a window of `window`, when the record
/// does not colour by judgement.
///
/// The alpha is the line colour's times `age` over half the window, so the newest marks reach past
/// full opacity. The reference packs the colour into one integer and the part of the alpha above 255
/// spills into the lowest bit of the blue channel, which this keeps.
fn fading_color(line: Rgba, age: i32, window: i32) -> Rgba {
    let alpha = f32::from(line[3]) / f32::from(u8::MAX) * age as f32 / (window as f32 / FADE_HALVES);
    let scaled = (alpha * f32::from(u8::MAX)) as i32 as u32;
    let packed = u32::from(line[0]) << 24 | u32::from(line[1]) << 16 | u32::from(line[2]) << 8 | scaled;
    let [red, green, blue, alpha] = packed.to_be_bytes();
    [red, green, blue, alpha]
}

/// Where a mark of `error` milliseconds early is, as the pixmap column its line starts at: to the
/// left of the middle for an early hit, truncated towards the middle.
fn column_of(body: &HitErrorBody, error: i64) -> i32 {
    let clamped = error.clamp(-i64::from(body.center), i64::from(body.center));
    (body.width - body.line_width) / 2 + (clamped as f32 * -body.rate) as i32
}

/// Fills the triangle with corners `apex` and the two ends of the top row `y = 0`, one pixel row at
/// a time and blended in.
fn fill_triangle(pixmap: &mut Pixmap, apex: (i32, i32), half_width: i32, color: Rgba) {
    let (apex_x, apex_y) = apex;
    if apex_y <= 0 {
        pixmap.blend_rect(apex_x - half_width, 0, half_width * 2 + 1, 1, color);
        return;
    }
    for y in 0..=apex_y {
        let inset = (half_width * y + apex_y - 1) / apex_y;
        let left = apex_x - half_width + inset;
        let right = apex_x + half_width - inset;
        if right >= left {
            pixmap.blend_rect(left, y, right - left + 1, 1, color);
        }
    }
}

/// Paints the whole strip for `errors`, oldest first, and moves the average on by the newest.
fn paint(body: &HitErrorBody, errors: &[i64], judge_area: &[[i32; 2]; TIMING_JUDGE_AREAS], ema: &mut i64) -> Pixmap {
    let window = body.window;
    let height = window * ROWS_PER_HIT;
    let mut pixmap = Pixmap::new(body.width as usize, height as usize);
    let newest = |back: usize| errors.len().checked_sub(back + 1).map(|at| errors[at]);

    if body.marks {
        for age in (1..=window).rev() {
            let Some(error) = newest((window - age) as usize) else {
                continue;
            };
            let color = if body.by_judgement { judgement_color(body, judge_area, error) } else { fading_color(body.line, age, window) };
            let column = column_of(body, error);
            if body.decay {
                pixmap.blend_rect(column, window - age, body.line_width, age * ROWS_PER_HIT, color);
            } else {
                pixmap.blend_rect(column, 0, body.line_width, RECENT_JUDGES as i32 * ROWS_PER_HIT, color);
            }
        }
    }

    pixmap.blend_rect((body.width - body.line_width) / 2, 0, body.line_width, height, body.centre);

    if body.ema_mode != 0 {
        if let Some(last) = newest(0).filter(|last| within(*last, judge_area, AVERAGE_WINDOW)) {
            *ema += (body.alpha * (last - *ema) as f32) as i64;
        }
        let column = column_of(body, i64::from(*ema as i32));
        if body.ema_mode == EMA_LINE || body.ema_mode == EMA_BOTH {
            pixmap.blend_rect(column, 0, body.line_width, height, body.average);
        }
        if body.ema_mode == EMA_TRIANGLE || body.ema_mode == EMA_BOTH {
            let mut half_width = (f64::from(body.width) * TRIANGLE_WIDTH_SHARE) as i32;
            half_width += half_width % 2;
            fill_triangle(&mut pixmap, (column + body.line_width / 2, height / TRIANGLE_HEIGHT_DIVISOR), half_width, body.average);
        }
    }
    pixmap
}

/// Draws the running hit errors over the object's rectangle.
pub(crate) fn draw_hit_error<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &HitErrorBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(recent) = frame.data.series.recent_hits else {
        return false;
    };
    if signed_plot_of(place, rect).is_none() || body.width <= 0 {
        return false;
    }
    let largest = i64::from(r.max_texture_size());
    if i64::from(body.width) > largest || i64::from(body.window * ROWS_PER_HIT) > largest {
        return false;
    }

    let mut drawn = body.state.borrow_mut();
    if drawn.judge_area != Some(recent.judge_area) || recent.recorded < drawn.seen {
        drawn.judge_area = Some(recent.judge_area);
        drawn.index = None;
        drawn.ema = 0;
    }
    let index = recent.recorded % RECENT_JUDGES;
    if drawn.index != Some(index) {
        let mut ema = drawn.ema;
        drawn.pixmap = paint(body, &recent.errors(), &recent.judge_area, &mut ema);
        drawn.ema = ema;
        drawn.index = Some(index);
        drawn.seen = recent.recorded;
        drawn.layer.mark_stale();
    }

    let Drawn { layer, pixmap, .. } = &mut *drawn;
    let (width, height) = pixmap.size();
    let Some(tex) = layer.texture_as(r, pixmap, Upload::FirstRowOnTop) else {
        return false;
    };
    place.texture(r, tex, (width as u32, height as u32), SkinRect::new(0.0, 0.0, width as f32, height as f32), rect)
}

#[cfg(test)]
mod tests;
