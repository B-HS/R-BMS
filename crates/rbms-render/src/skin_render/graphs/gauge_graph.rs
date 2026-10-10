//! The `gaugegraph` object (`SkinGaugeGraphObject`): how a run's gauge moved, sample by sample.
//!
//! The reference paints the graph into two pixmaps as large as the object's rectangle is on screen
//! and draws each as an image, upside down so that an empty gauge sits on the foot of the rectangle:
//!
//! - the ground, in the colour of the gauge that is shown, with the stretch from the clear line up
//!   to a full gauge painted over it in a second colour,
//! - and the line, a step line through the samples that changes colour where it crosses the clear
//!   line, revealed from the left over a second and a half of the scene.
//!
//! A run carries the history of every gauge it could have been played on, and the graph plots the
//! one the frame says is shown ([`GaugeFrame::gauge_type`](super::super::gauge::GaugeFrame)): the
//! gauge the run is on while it is played, the one the player has switched to on a score screen.
//! The clear line and the top of the scale are that gauge's own, read from the same frame.
//!
//! Painting happens here, into [`Pixmap`]s, with the reference's own arithmetic and its own pixmap
//! blending, so a record that names translucent colours comes out the way it does there: the
//! stretch above the clear line is the second colour mixed into the first rather than replacing
//! it. The reference paints once and again only when the shown gauge changes or the rectangle does;
//! here the pixmaps are painted again whenever anything they were painted from differs, which is
//! the same on a score screen and keeps a history that is still growing up to date.

use std::cell::RefCell;

use rbms_skin::dst::SkinRect;
use rbms_skin::model::{GaugeGraph, SkinDef};
use rbms_skin::timer::MICROS_PER_MILLI;

use super::super::SkinFrame;
use super::super::draw::Placement;
use super::super::gauge::GaugeScale;
use super::super::object::Body;
use super::pixmap::{Pixmap, Rgba};
use super::{Layer, draw_layer, next_serial, plot_of, reference_color};
use crate::ctx::RenderCtx;
use crate::{Rect, Renderer};

/// Milliseconds between two samples of a gauge history, which is what the reference's play screen
/// records one at.
pub const GAUGE_SAMPLE_MS: i64 = 500;

/// Milliseconds of the scene over which the line is revealed from the left (`delay`). A record has
/// no field for it.
const REVEAL_MS: i64 = 1500;

/// Pixels thick the line is (`lineWidth`). A record has no field for it either.
const LINE_WIDTH: i32 = 2;

/// How many sets of colours a graph carries: one for each of the six gauges a chart is played on.
const BANDS: usize = 6;

/// How many colours one such set holds.
const COLORS_PER_BAND: usize = 4;

/// Which set of colours each gauge type is drawn in (`typetable`): the three gauges a course is
/// played on share the sets of hard, ex-hard and hazard.
const BAND_OF_TYPE: [usize; 10] = [0, 1, 2, 3, 4, 5, 3, 4, 5, 3];

/// The first of the gauges that have no ground or line below the clear line of their own, because
/// they clear at nothing: the named colours give them one pair for the whole graph.
const FIRST_SURVIVAL_BAND: usize = 3;

/// The colour a set is given for a colour its record left out of the `color` list, and the colour a
/// text that is not a colour is read as.
const MISSING_COLOR: Rgba = [0, 0, 0, u8::MAX];

/// The colour of the upright line that marks where one stage of a course ends and the next begins.
const SECTION_LINE: Rgba = [u8::MAX; 4];

/// How a run's gauge moved, which is what a gauge graph plots.
///
/// Any screen that knows a run's gauge may hand this over: the score screen for the run that just
/// ended, the play screen for the one in progress. The samples are [`GAUGE_SAMPLE_MS`] apart and in
/// the gauge's own units.
#[derive(Debug, Clone, Copy)]
pub struct GaugeHistory<'a> {
    /// The samples of one gauge, oldest first, for a screen that knows the history of the gauge the
    /// run was played on and of no other. Read only when [`Self::kinds`] is empty, and then plotted
    /// whichever gauge the frame says is shown.
    pub samples: &'a [f32],
    /// The samples of every gauge the run could have been played on, by gauge type: the reference
    /// records all of them side by side, which is what lets a score screen switch between them.
    pub kinds: &'a [Vec<f32>],
    /// For a course: how many samples there are up to the end of each stage, counted from the
    /// start. An upright line is drawn where each stage ends.
    pub sections: &'a [usize],
}

impl<'a> GaugeHistory<'a> {
    /// A history known for one gauge only, oldest sample first.
    pub fn new(samples: &'a [f32]) -> GaugeHistory<'a> {
        GaugeHistory { samples, kinds: &[], sections: &[] }
    }

    /// A history known for every gauge type, each oldest sample first.
    pub fn of_kinds(kinds: &'a [Vec<f32>]) -> GaugeHistory<'a> {
        GaugeHistory { samples: &[], kinds, sections: &[] }
    }

    /// The same history with the ends of a course's stages marked.
    pub fn with_sections(mut self, sections: &'a [usize]) -> GaugeHistory<'a> {
        self.sections = sections;
        self
    }

    /// The samples of the gauge numbered `gauge_type`, or `None` when the history has every gauge's
    /// samples and none for that one.
    pub fn of(&self, gauge_type: usize) -> Option<&'a [f32]> {
        if self.kinds.is_empty() { Some(self.samples) } else { self.kinds.get(gauge_type).map(Vec::as_slice) }
    }
}

/// The four colours one gauge's graph is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Band {
    /// The line where the gauge is at or above its clear line (`borderline`).
    line_above: Rgba,
    /// The ground from the clear line up (`bordercolor`).
    ground_above: Rgba,
    /// The line where the gauge is below its clear line (`graphline`).
    line_below: Rgba,
    /// The ground below the clear line, and under the rest before that is painted over it
    /// (`graphcolor`).
    ground_below: Rgba,
}

/// The gauge history a document declares.
#[derive(Debug)]
pub(crate) struct GaugeGraphBody {
    bands: [Band; BANDS],
    state: RefCell<Drawn>,
}

/// What the graph has painted, kept between frames because the reference keeps it.
#[derive(Debug)]
struct Drawn {
    ground: Layer,
    line: Layer,
    ground_pixmap: Pixmap,
    line_pixmap: Pixmap,
    painted: Option<Painted>,
}

/// Everything the pixmaps were painted from.
#[derive(Debug)]
struct Painted {
    gauge_type: usize,
    scale: GaugeScale,
    samples: Vec<f32>,
    sections: Vec<usize>,
    /// The rectangle's size on screen, as the bit patterns of the two numbers the arithmetic reads.
    size: (u32, u32),
}

impl GaugeGraphBody {
    /// Hands the textures back to the renderer they were uploaded to.
    pub(crate) fn release<R: Renderer>(&self, r: &mut R) {
        let mut drawn = self.state.borrow_mut();
        drawn.ground.release(r);
        drawn.line.release(r);
        drawn.painted = None;
    }
}

/// The colour a record wrote for `what`, read the way the reference reads it, or opaque black with a
/// warning when the text is not a colour.
fn color_of(text: &str, what: &str, id: &str, warnings: &mut Vec<String>) -> Rgba {
    reference_color(text).unwrap_or_else(|| {
        warnings.push(format!("gauge graph {id:?} writes its {what} as {text:?}, which is not a colour"));
        MISSING_COLOR
    })
}

/// The six sets of colours a record's `color` list spells out: four to a gauge in the order line
/// above, ground above, line below, ground below, with opaque black for whatever the list leaves
/// out and nothing read past the twenty-fourth.
fn listed_bands(graph: &GaugeGraph, warnings: &mut Vec<String>) -> [Band; BANDS] {
    let mut colors = [[MISSING_COLOR; COLORS_PER_BAND]; BANDS];
    for (index, text) in graph.color.iter().take(BANDS * COLORS_PER_BAND).enumerate() {
        colors[index / COLORS_PER_BAND][index % COLORS_PER_BAND] = color_of(text, "listed colour", &graph.id, warnings);
    }
    colors.map(|[line_above, ground_above, line_below, ground_below]| Band { line_above, ground_above, line_below, ground_below })
}

/// The six sets of colours a record's named fields spell out.
///
/// The three gauges that clear part way up have a ground and a line of their own below the clear
/// line and share one pair above it. The three that clear at nothing have one pair each, used for
/// the whole graph.
fn named_bands(graph: &GaugeGraph, warnings: &mut Vec<String>) -> [Band; BANDS] {
    let mut read = |text: &str, what: &str| color_of(text, what, &graph.id, warnings);
    let grounds = [
        read(&graph.assist_clear_bg_color, "assistClearBGColor"),
        read(&graph.assist_and_easy_fail_bg_color, "assistAndEasyFailBGColor"),
        read(&graph.groove_fail_bg_color, "grooveFailBGColor"),
        read(&graph.groove_clear_and_hard_bg_color, "grooveClearAndHardBGColor"),
        read(&graph.ex_hard_bg_color, "exHardBGColor"),
        read(&graph.hazard_bg_color, "hazardBGColor"),
    ];
    let lines = [
        read(&graph.assist_clear_line_color, "assistClearLineColor"),
        read(&graph.assist_and_easy_fail_line_color, "assistAndEasyFailLineColor"),
        read(&graph.groove_fail_line_color, "grooveFailLineColor"),
        read(&graph.groove_clear_and_hard_line_color, "grooveClearAndHardLineColor"),
        read(&graph.ex_hard_line_color, "exHardLineColor"),
        read(&graph.hazard_line_color, "hazardLineColor"),
    ];
    let (line_above, ground_above) = (read(&graph.borderline_color, "borderlineColor"), read(&graph.border_color, "borderColor"));
    std::array::from_fn(|band| {
        if band < FIRST_SURVIVAL_BAND {
            Band { line_above, ground_above, line_below: lines[band], ground_below: grounds[band] }
        } else {
            Band { line_above: lines[band], ground_above: grounds[band], line_below: lines[band], ground_below: grounds[band] }
        }
    })
}

/// The gauge graph behind `id`, or `None` when the document declares none by that name.
///
/// A record with a `color` list is drawn in the colours of that list and its named colours are not
/// read, as in the reference's loader. A list written out empty cannot be told from one that was
/// never written, so it reads as the named colours here where the reference would draw in black.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.gaugegraph.iter().find(|graph| graph.id == id)?;
    let bands = if graph.color.is_empty() { named_bands(graph, warnings) } else { listed_bands(graph, warnings) };
    let serial = next_serial();
    let state = Drawn {
        ground: Layer::new(serial, "ground"),
        line: Layer::new(serial, "line"),
        ground_pixmap: Pixmap::default(),
        line_pixmap: Pixmap::default(),
        painted: None,
    };
    Some(Body::GaugeGraph(GaugeGraphBody { bands, state: RefCell::new(state) }))
}

/// Paints the ground of a rectangle `size` pixels on screen: the colour below the clear line over
/// all of it, then the colour above mixed in from the clear line to the top of the scale.
///
/// Row zero is the foot of the graph, as it is for the line.
fn paint_ground(size: (f32, f32), scale: GaugeScale, band: &Band) -> Pixmap {
    let (width, height) = size;
    let mut ground = Pixmap::new(width as i32 as usize, height as i32 as usize);
    ground.fill(band.ground_below);
    ground.blend_rect(0, (height * scale.border / scale.max) as i32, width as i32, (height * (scale.max - scale.border) / scale.max) as i32, band.ground_above);
    ground
}

/// Paints the step line of `samples` into a transparent pixmap the size of the rectangle on screen.
///
/// Each sample is joined to the one before it by an upright at the earlier sample's column and a
/// flat run from there to its own. The upright is split where it crosses the clear line, and each
/// piece takes the colour of the side it is on; the flat run takes the colour of the side the later
/// sample is on. The last sample's run is carried on to the right edge. Where a stage of a course
/// ends, a white upright is drawn first.
fn paint_line(size: (f32, f32), scale: GaugeScale, band: &Band, samples: &[f32], sections: &[usize]) -> Pixmap {
    let (width, height) = size;
    let mut line = Pixmap::new(width as i32 as usize, height as i32 as usize);
    let count = samples.len() as i32;
    let GaugeScale { max, border, .. } = scale;
    let column = |index: i32| (width * index as f32 / count as f32) as i32;
    let level = |value: f32| ((value / max) * (height - LINE_WIDTH as f32)) as i32;
    let color_of = |value: f32| if value < border { band.line_below } else { band.line_above };

    let mut last: Option<(f32, i32, i32)> = None;
    for (index, after) in (0..count).zip(samples.iter().copied()) {
        if sections.contains(&(index as usize)) {
            line.blend_upright_line(column(index - 1), 0, height as i32, SECTION_LINE);
        }
        let Some(before) = (index as usize).checked_sub(1).map(|earlier| samples[earlier]) else {
            continue;
        };
        let (x1, x2) = (column(index - 1), column(index));
        let (y1, y2, yb) = (level(before), level(after), level(border));
        last = Some((after, x2, y2));
        match (before < border, after < border) {
            (true, true) | (false, false) => {
                line.blend_rect(x1, y1.min(y2), LINE_WIDTH, (y2 - y1).abs() + LINE_WIDTH, color_of(before));
            }
            (true, false) => {
                line.blend_rect(x1, y1, LINE_WIDTH, yb - y1, band.line_below);
                line.blend_rect(x1, yb, LINE_WIDTH, y2 - yb + LINE_WIDTH, band.line_above);
            }
            (false, true) => {
                line.blend_rect(x1, yb, LINE_WIDTH, y1 - yb + LINE_WIDTH, band.line_above);
                line.blend_rect(x1, y2, LINE_WIDTH, yb - y2, band.line_below);
            }
        }
        line.blend_rect(x1, y2, x2 - x1, LINE_WIDTH, color_of(after));
    }
    if let Some((value, x, y)) = last {
        line.blend_rect(x, y, (width - x as f32) as i32, LINE_WIDTH, color_of(value));
    }
    line
}

/// How much of the line the scene has revealed by `now_ms`, from none to all.
fn revealed(now_ms: i64) -> f32 {
    if now_ms >= REVEAL_MS { 1.0 } else { now_ms as f32 / REVEAL_MS as f32 }
}

/// Draws the gauge history: the ground, and the line as far as the scene has revealed it.
///
/// A frame that carries no history, or no gauge to say which one is shown and where it clears,
/// draws nothing.
pub(crate) fn draw_gauge_graph<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &GaugeGraphBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let (Some(history), Some(gauge)) = (frame.data.series.gauge_history, frame.data.gauge) else {
        return false;
    };
    let (Some(samples), Some(scale), Some(band)) = (history.of(gauge.gauge_type), gauge.shown_scale(), BAND_OF_TYPE.get(gauge.gauge_type)) else {
        return false;
    };
    let Some(plot) = plot_of(place, rect) else {
        return false;
    };
    let (width, height) = (plot.w as i32, plot.h as i32);
    let largest = i64::from(r.max_texture_size());
    if width <= 0 || height <= 0 || i64::from(width) > largest || i64::from(height) > largest {
        return false;
    }

    let mut drawn = body.state.borrow_mut();
    let size = (plot.w.to_bits(), plot.h.to_bits());
    let current = drawn.painted.as_ref().is_some_and(|painted| {
        painted.gauge_type == gauge.gauge_type
            && painted.scale == scale
            && painted.size == size
            && painted.samples == samples
            && painted.sections == history.sections
    });
    if !current {
        drawn.ground_pixmap = paint_ground((plot.w, plot.h), scale, &body.bands[*band]);
        drawn.line_pixmap = paint_line((plot.w, plot.h), scale, &body.bands[*band], samples, history.sections);
        drawn.painted = Some(Painted { gauge_type: gauge.gauge_type, scale, samples: samples.to_vec(), sections: history.sections.to_vec(), size });
        drawn.ground.mark_stale();
        drawn.line.mark_stale();
    }

    let render = revealed(frame.now_us / MICROS_PER_MILLI);
    let Drawn { ground, line, ground_pixmap, line_pixmap, .. } = &mut *drawn;
    let texels = (width as u32, height as u32);
    let mut reached = false;
    if let Some(tex) = ground.texture(r, ground_pixmap) {
        reached |= draw_layer(r, place, tex, texels, texels.0, plot);
    }
    if let Some(tex) = line.texture(r, line_pixmap) {
        let across = (plot.w * render) as i32;
        reached |= draw_layer(r, place, tex, texels, across.clamp(0, width) as u32, Rect::new(plot.x, plot.y, across as f32, plot.h));
    }
    reached
}

#[cfg(test)]
mod tests;
