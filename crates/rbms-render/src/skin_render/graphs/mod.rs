//! The graphs and visualisers a document draws instead of the built-in panels: the gauge history,
//! the judgement spread, the tempo timeline, the timing distribution and the running hit error.
//!
//! Each kind lives in a file of its own, together with the series it reads: a property id answers
//! one value and none of these is one value, so each graph names the shape of what it plots and a
//! frame carries that beside its scalar source ([`super::frame::FrameSeries`]). Its colours come from
//! the record the document declared it with. Most draw with `fill_rect` rather than a line
//! primitive, because the renderer has no line primitive: a line here is a rectangle one `lineWidth`
//! thick.
//!
//! A destination's own colour modulates whatever the record named, so a document fades a whole graph
//! out through the destination it placed it at and leaves it alone by drawing it white, which is
//! what an unstated destination colour already is.
//!
//! The two graphs the reference paints into a pixmap of their own, the judgement spread and the
//! tempo timeline, are the exception: each uploads its pixmap as a texture and draws it scaled onto
//! the object's rectangle, as the reference does, and the destination's colour tints that texture as
//! it would any image. What they share for that stays here: [`Layer`], one pixmap's life as a
//! texture, and [`draw_layer`], which fits a texture to a rectangle.
//!
//! What is shared between two or more kinds stays here: reading a colour, the plot rectangle, an
//! upright line, and the ruler arithmetic the two visualisers have in common.

mod bpm;
mod gauge_graph;
mod hit_error;
mod notes_dist;
mod pixmap;
mod timing_dist;
mod timing_vis;

use std::sync::atomic::{AtomicU32, Ordering};

use rbms_skin::dst::SkinRect;
use rbms_skin::loader::{Filtering, LoadedSkin, filtering_for, stretch_rect};

use super::SkinAssets;
use super::color::parse_hex_color;
use super::draw::Placement;
use super::object::Body;
use super::textures::Source;
use crate::{Color, Rect, Renderer, TextureFilter, TextureId, UvRect};
use pixmap::Pixmap;

pub use bpm::BpmTimeline;
pub(crate) use bpm::{BpmGraphBody, draw_bpm_graph};
pub use gauge_graph::GaugeHistory;
pub(crate) use gauge_graph::{GaugeGraphBody, draw_gauge_graph};
pub(crate) use hit_error::{HitErrorBody, draw_hit_error};
pub use notes_dist::{EARLY_LATE_BUCKETS, JUDGEMENTS, NOTE_KINDS, NoteDistribution, PlayCursor};
pub(crate) use notes_dist::{JudgeGraphBody, draw_judge_graph};
pub use timing_dist::TimingHistogram;
pub(crate) use timing_dist::{TimingDistributionBody, draw_timing_distribution};
pub use timing_vis::RecentHits;
pub(crate) use timing_vis::{TimingVisualizerBody, draw_timing_visualizer};

/// How many judgements the visualiser palettes name: they stop at a poor, because a miss is what a
/// note that was never hit takes and a hit error is only recorded for a note that was.
const VISUALIZER_JUDGEMENTS: usize = 5;

/// Tells one graph object's textures from another's in the renderer's registry.
static NEXT_GRAPH_SERIAL: AtomicU32 = AtomicU32::new(0);

/// A number no other graph object has been given, to key its textures by.
fn next_serial() -> u32 {
    NEXT_GRAPH_SERIAL.fetch_add(1, Ordering::Relaxed)
}

/// The colour a graph falls back to when the document wrote something that is not one.
const FALLBACK_COLOR: Color = Color::WHITE;

/// Thinnest a line is drawn, so a document that asked for none still leaves a mark.
const MIN_LINE_W: f32 = 1.0;

/// Narrowest a histogram bar is drawn, so a full histogram still shows every bucket.
const MIN_BAR_W: f32 = 1.0;

/// Gap between two histogram bars, taken off the bar rather than added to the pitch.
const BAR_GAP: f32 = 1.0;

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

/// The screen rectangle a graph plots inside, or `None` when its destination has no extent this
/// frame.
fn plot_of(place: &Placement<'_>, rect: SkinRect) -> Option<Rect> {
    let dst = place.viewport.place(rect);
    (dst.w > 0.0 && dst.h > 0.0).then_some(dst)
}

/// Draws one upright line across a plot, centred on `x`.
fn draw_column<R: Renderer>(r: &mut R, plot: Rect, x: f32, width: f32, color: Color) {
    r.fill_rect(Rect::new(x - width * 0.5, plot.y, width, plot.h), color);
}

/// How far from the middle of a ruler one timing error lands, in pixels.
fn error_offset(plot: Rect, error_ms: f32, window_ms: f32) -> f32 {
    plot.w * 0.5 * (error_ms / window_ms).clamp(-1.0, 1.0)
}

/// One pixmap of a graph, kept uploaded as a texture of its own.
///
/// The pixmap itself belongs to the graph, which paints it when its numbers change. A layer only
/// knows whether what the renderer holds is still that pixmap: it uploads again when told the pixmap
/// changed, and when the renderer does not know its texture (a graph drawn onto another renderer than
/// the one it was uploaded to).
#[derive(Debug)]
struct Layer {
    key: String,
    handle: Option<TextureId>,
    stale: bool,
}

impl Layer {
    /// A layer called `name` among the textures of the graph numbered `serial`.
    fn new(serial: u32, name: &str) -> Layer {
        Layer { key: format!("rbms.skin.graph.{serial}.{name}"), handle: None, stale: true }
    }

    /// Records that the pixmap was painted again, so the next [`Layer::texture`] uploads it.
    fn mark_stale(&mut self) {
        self.stale = true;
    }

    /// The texture holding `pixmap`, or `None` when the renderer refused to hold it.
    fn texture<R: Renderer>(&mut self, r: &mut R, pixmap: &Pixmap) -> Option<TextureId> {
        let (width, height) = pixmap.size();
        let size = (width as u32, height as u32);
        let known = self.handle.and_then(|handle| r.texture_size(handle));
        if self.stale || known != Some(size) {
            self.handle = Some(r.register_texture(&self.key, &pixmap.bottom_up(), size.0, size.1));
            self.stale = false;
        }
        self.handle.filter(|handle| r.texture_size(*handle).is_some())
    }

    /// Hands the texture back to the renderer.
    fn release<R: Renderer>(&mut self, r: &mut R) {
        if let Some(handle) = self.handle.take() {
            r.release_texture(handle);
        }
        self.stale = true;
    }
}

/// Hands back whatever textures the graph in `body` holds. Every other kind of object holds none of
/// its own.
pub(crate) fn release<R: Renderer>(body: &Body, r: &mut R) {
    match body {
        Body::JudgeGraph(body) => body.release(r),
        Body::BpmGraph(body) => body.release(r),
        _ => {}
    }
}

/// Draws the left `shown` columns of a texture of `size` pixels onto `dst`, a screen rectangle, the
/// way an image of that size is drawn: fitted by the object's stretch mode, sampled with the filter
/// its destination chose, tinted, blended and turned with the object. Answers whether anything was
/// drawn.
fn draw_layer<R: Renderer>(r: &mut R, place: &Placement<'_>, tex: TextureId, size: (u32, u32), shown: u32, dst: Rect) -> bool {
    if shown == 0 || !(dst.w > 0.0 && dst.h > 0.0) {
        return false;
    }
    let region = SkinRect::new(0.0, 0.0, shown as f32, size.1 as f32);
    let (fitted, source) = stretch_rect(place.object.stretch, SkinRect::new(dst.x, dst.y, dst.w, dst.h), region);
    if fitted.w <= 0.0 || fitted.h <= 0.0 {
        return false;
    }
    let filter = match filtering_for(place.object.track.filter, fitted, (source.w, source.h)) {
        Filtering::Nearest => TextureFilter::Nearest,
        Filtering::Linear => TextureFilter::Linear,
    };
    let (texture_w, texture_h) = (size.0.max(1) as f32, size.1.max(1) as f32);
    let uv = UvRect::new(source.x / texture_w, source.y / texture_h, (source.x + source.w) / texture_w, (source.y + source.h) / texture_h);
    r.draw_textured_quad(tex, place.quad(Rect::new(fitted.x, fitted.y, fitted.w, fitted.h), uv, filter));
    true
}

/// The graph behind `id`, or `None` when the document declares none by that name.
///
/// The kinds are asked in the order the reference's loader walks them, so an id two records share
/// resolves to the same one it always did. The judgement graph is the one kind that gives way: the
/// loader only breaks out of its search once it has one, so a later record of the same id, a tempo
/// graph or a visualiser, is the one that is drawn.
pub(crate) fn build_graph(
    skin: &LoadedSkin,
    id: &str,
    _sources: Source<'_>,
    _families: &[(String, String)],
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = &skin.def;
    if let Some(body) = gauge_graph::build(def, id, warnings) {
        return Some(body);
    }
    let distribution = notes_dist::build(def, id, warnings);
    bpm::build(def, id, warnings)
        .or_else(|| hit_error::build(def, id, warnings))
        .or_else(|| timing_vis::build(def, id, warnings))
        .or_else(|| timing_dist::build(def, id, warnings))
        .or(distribution)
}
