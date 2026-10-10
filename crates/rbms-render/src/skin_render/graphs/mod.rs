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
//! what an unstated destination colour already is. The two visualisers keep the part of that which
//! the reference keeps: the ground they paint is tinted by the destination like any image, and the
//! lines the timing visualiser lays over it are not.
//!
//! The graphs the reference paints into a pixmap of their own -- the gauge history, the judgement
//! spread, the tempo timeline and the timing distribution -- are the exception: each uploads its
//! pixmap as a texture and draws it scaled onto the object's rectangle, as the reference does, and
//! the destination's colour tints that texture as it would any image. What they share for that stays
//! here: [`Layer`], one pixmap's life as a texture, and [`draw_layer`], which fits a texture to a
//! rectangle.
//!
//! What is shared between two or more kinds stays here: reading a colour, the plot rectangle, and the
//! arithmetic the two visualisers have in common.

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
use super::draw::Placement;
use super::object::Body;
use super::textures::Source;
use crate::{Rect, Renderer, TextureFilter, TextureId, UvRect};
use pixmap::{Pixmap, Rgba};

pub use bpm::BpmTimeline;
pub(crate) use bpm::{BpmGraphBody, draw_bpm_graph};
pub use gauge_graph::{GAUGE_SAMPLE_MS, GaugeHistory};
pub(crate) use gauge_graph::{GaugeGraphBody, draw_gauge_graph};
pub(crate) use hit_error::{HitErrorBody, draw_hit_error};
pub use notes_dist::{EARLY_LATE_BUCKETS, JUDGEMENTS, NOTE_KINDS, NoteDistribution, PlayCursor};
pub(crate) use notes_dist::{JudgeGraphBody, draw_judge_graph};
pub use timing_dist::{TIMING_JUDGE_AREAS, TimingHistogram};
pub(crate) use timing_dist::{TimingDistributionBody, draw_timing_distribution};
pub use timing_vis::RecentHits;
pub(crate) use timing_vis::{TimingVisualizerBody, draw_timing_visualizer};

/// Tells one graph object's textures from another's in the renderer's registry.
static NEXT_GRAPH_SERIAL: AtomicU32 = AtomicU32::new(0);

/// A number no other graph object has been given, to key its textures by.
fn next_serial() -> u32 {
    NEXT_GRAPH_SERIAL.fetch_add(1, Ordering::Relaxed)
}

/// Hex digits a colour's three channels are written in.
const COLOR_DIGITS: usize = 6;

/// Hex digits a colour that carries its own alpha is written in.
const ALPHA_DIGITS: usize = 8;

/// Hex digits one channel takes.
const CHANNEL_DIGITS: usize = 2;

/// The base a colour is written in.
const HEX_RADIX: u32 = 16;

/// The colour the reference reads from a record's text (`Color.valueOf`, in the build it ships
/// with), as the bytes a pixmap stores for it, or `None` where the reference throws.
///
/// The first six hex digits are the colour, after a `#` if the text starts with one. The alpha is
/// read from the next two only when the text is exactly eight digits long; a text of any other
/// length is opaque, whatever follows the sixth digit.
fn reference_color(text: &str) -> Option<Rgba> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    let channel = |index: usize| {
        let start = index * CHANNEL_DIGITS;
        digits.get(start..start + CHANNEL_DIGITS).and_then(|pair| u8::from_str_radix(pair, HEX_RADIX).ok())
    };
    let alpha = if digits.len() == ALPHA_DIGITS { channel(COLOR_DIGITS / CHANNEL_DIGITS)? } else { u8::MAX };
    Some([channel(0)?, channel(1)?, channel(2)?, alpha])
}

/// How many judgements the visualiser palettes name: they stop at a poor, because a miss is what a
/// note that was never hit takes and a hit error is only recorded for a note that was.
const VISUALIZER_JUDGEMENTS: usize = 5;

/// How many recent hits the reference keeps (`JudgeManager.recentJudges`), which is also how many
/// the timing visualiser draws.
const RECENT_JUDGES: usize = 100;

/// The first judgement the reference does not record a hit error for: poor (`judge < 4`).
const FIRST_UNRECORDED_JUDGE: u8 = 4;

/// The fewest and the most pixels a visualiser's lines are thick (`MathUtils.clamp(lineWidth, 1, 4)`).
const MIN_LINE_WIDTH: i32 = 1;
const MAX_LINE_WIDTH: i32 = 4;

/// What a visualiser colour that is not hex digits, or is fewer than six of them, reads as: opaque
/// red (`colorStringValidation`).
const INVALID_VISUALIZER_COLOR: Rgba = [u8::MAX, 0, 0, u8::MAX];

/// The colour of a translucent pixel that blends away: the "clear" the reference's palette puts
/// where a document asked for its poor window to be see-through.
const CLEAR: Rgba = [0; 4];

/// The colour a visualiser record wrote for `what`, validated the way the reference validates it
/// (`colorStringValidation`): anything that is not at least six hex digits and nothing else is
/// opaque red, with a warning here where the reference says nothing.
fn visualizer_color(text: &str, what: &str, id: &str, warnings: &mut Vec<String>) -> Rgba {
    let valid = text.len() >= COLOR_DIGITS && text.bytes().all(|byte| byte.is_ascii_hexdigit());
    reference_color(text).filter(|_| valid).unwrap_or_else(|| {
        warnings.push(format!("visualiser {id:?} writes its {what} as {text:?}, which is not a colour, so it is drawn in red"));
        INVALID_VISUALIZER_COLOR
    })
}

/// The five judgement colours a visualiser record names, best first, with the poor one cleared when
/// the record asks for it (`transparent == 1`).
fn visualizer_palette(colors: [&str; VISUALIZER_JUDGEMENTS], transparent: i32, id: &str, warnings: &mut Vec<String>) -> [Rgba; VISUALIZER_JUDGEMENTS] {
    const NAMES: [&str; VISUALIZER_JUDGEMENTS] = ["PGColor", "GRColor", "GDColor", "BDColor", "PRColor"];
    let poor = VISUALIZER_JUDGEMENTS - 1;
    std::array::from_fn(|index| if index == poor && transparent == 1 { CLEAR } else { visualizer_color(colors[index], NAMES[index], id, warnings) })
}

/// How thick a visualiser's lines are: the record's `lineWidth` held between one and four.
fn visualizer_line_width(declared: i32) -> i32 {
    declared.clamp(MIN_LINE_WIDTH, MAX_LINE_WIDTH)
}

/// How many pixels a millisecond of error is wide on a visualiser (`judgeWidthRate`): the record's
/// `width` over the number of milliseconds on the ruler, both sides and the middle one.
fn visualizer_rate(width: i32, center: i32) -> f32 {
    width as f32 / center.wrapping_mul(2).wrapping_add(1) as f32
}

/// The screen rectangle a graph plots inside, or `None` when its destination has no extent this
/// frame.
fn plot_of(place: &Placement<'_>, rect: SkinRect) -> Option<Rect> {
    let dst = place.viewport.place(rect);
    (dst.w > 0.0 && dst.h > 0.0).then_some(dst)
}

/// The screen rectangle a visualiser lays itself over, which keeps the sign of the destination's
/// width and height: a document mirrors a visualiser by giving it a negative extent, and the
/// reference draws it so. `None` when the destination has no extent or none that is a number.
fn signed_plot_of(place: &Placement<'_>, rect: SkinRect) -> Option<Rect> {
    let dst = place.viewport.place(rect);
    let extent = |size: f32| size.is_finite() && size != 0.0;
    (extent(dst.w) && extent(dst.h) && dst.x.is_finite() && dst.y.is_finite()).then_some(dst)
}

/// The document rectangle that lands on `placed`, a screen rectangle, once the viewport has placed
/// it: the way back from `SkinViewport::place`, for a piece the reference positions in screen
/// pixels but whose drawing is shared with every other image. `None` for a screen with no extent.
fn document_rect_of(place: &Placement<'_>, placed: Rect) -> Option<SkinRect> {
    let (scale_x, scale_y) = (place.viewport.scale_x(), place.viewport.scale_y());
    (scale_x > 0.0 && scale_y > 0.0)
        .then(|| SkinRect::new(placed.x / scale_x, place.viewport.document_y(placed.y + placed.h), placed.w / scale_x, placed.h / scale_y))
}

/// Which way up a pixmap is uploaded.
///
/// The reference draws most of its graph textures with a negative height, so the pixmap's first row
/// lands on the foot of the rectangle; the one it draws like any other image keeps its first row on
/// top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Upload {
    /// The pixmap's first row lands on the bottom of the rectangle.
    FirstRowAtFoot,
    /// The pixmap's first row lands on the top of the rectangle.
    FirstRowOnTop,
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

    /// The texture holding `pixmap` with its first row at the foot, or `None` when the renderer
    /// refused to hold it.
    fn texture<R: Renderer>(&mut self, r: &mut R, pixmap: &Pixmap) -> Option<TextureId> {
        self.texture_as(r, pixmap, Upload::FirstRowAtFoot)
    }

    /// The texture holding `pixmap` the way up `upload` names, or `None` when the renderer refused
    /// to hold it.
    fn texture_as<R: Renderer>(&mut self, r: &mut R, pixmap: &Pixmap, upload: Upload) -> Option<TextureId> {
        let (width, height) = pixmap.size();
        let size = (width as u32, height as u32);
        let known = self.handle.and_then(|handle| r.texture_size(handle));
        if self.stale || known != Some(size) {
            self.handle = Some(match upload {
                Upload::FirstRowAtFoot => r.register_texture(&self.key, &pixmap.bottom_up(), size.0, size.1),
                Upload::FirstRowOnTop => r.register_texture(&self.key, pixmap.top_down(), size.0, size.1),
            });
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
        Body::GaugeGraph(body) => body.release(r),
        Body::JudgeGraph(body) => body.release(r),
        Body::BpmGraph(body) => body.release(r),
        Body::TimingDistribution(body) => body.release(r),
        Body::TimingVisualizer(body) => body.release(r),
        Body::HitError(body) => body.release(r),
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
