//! The `judgegraph` object, which the reference draws with its note distribution graph
//! (`SkinNoteDistributionGraph`): how a chart's notes, or a run's judgements, are spread over the
//! seconds of the chart.
//!
//! The reference paints the graph into three pixmaps of its own and draws each as an image. The
//! texture is `5 * seconds` by `5 * rows` pixels: a second is a column five pixels wide, and every
//! note of that second is a chip four pixels square stacked in it, one row of five pixels per chip,
//! the first kind of note at the foot. The pixmaps are
//!
//! - the ground, with a ruler every ten rows and every ten and sixty seconds,
//! - the chips, which are revealed from the left over the record's `delay`,
//! - and, on the play screen only, a cursor three pixels wide at the playing position (and at the
//!   practice range's ends).
//!
//! Painting happens here, into [`Pixmap`]s, with the reference's own arithmetic, and each pixmap is
//! uploaded as a texture of its own and scaled onto the object's rectangle like any other image.
//! That is what makes the chips as thick, as thin and as aliased on screen as the reference's are.
//!
//! The graph counts three different things, chosen by the record's `type`, and the numbers come
//! from the frame ([`NoteDistribution`]) rather than from the chart: a document names which of the
//! three it wants, a screen supplies the ones it knows.

use std::cell::RefCell;

use rbms_skin::dst::SkinRect;
use rbms_skin::model::SkinDef;
use rbms_skin::timer::{MICROS_PER_MILLI, timer_id};

use super::super::SkinFrame;
use super::super::draw::Placement;
use super::super::object::Body;
use super::pixmap::{Pixmap, Rgba, TRANSPARENT, hex_color, unit_color};
use super::{Layer, draw_layer, next_serial, plot_of};
use crate::ctx::RenderCtx;
use crate::{Rect, Renderer};

/// How many judgements a run is counted in, best first.
pub const JUDGEMENTS: usize = 6;

/// How many kinds of note the note-kind graph tells apart.
pub const NOTE_KINDS: usize = 7;

/// How many classes the early/late graph tells apart: unjudged, the best judgement, and the four
/// below it twice over, once for the notes hit early and once for the notes hit late.
pub const EARLY_LATE_BUCKETS: usize = 10;

/// The record's `type` for the graph that counts note kinds.
const TYPE_NOTE_KINDS: i32 = 0;

/// The record's `type` for the graph that counts the judgement each note took.
const TYPE_JUDGEMENTS: i32 = 1;

/// The record's `type` for the graph that counts the judgement each note took, early and late apart.
const TYPE_EARLY_LATE: i32 = 2;

/// Pixels a second takes along the texture, and a chip row takes up it: a chip and the gap that
/// follows it.
const PITCH: usize = 5;

/// Pixels a chip measures when the record asks for no gap on neither axis.
const CHIP: i32 = 4;

/// How many chip rows a graph is at least tall.
const MIN_ROWS: usize = 20;

/// How many chip rows a graph is at most tall.
const MAX_ROWS: usize = 100;

/// The steps the row count grows in, and the spacing of the horizontal bands behind the chips.
const ROW_STEP: usize = 10;

/// How tall a band behind the chips is, in pixels: ten chip rows.
const BAND_HEIGHT: i32 = 50;

/// How much of the ground the chips are drawn over is opaque.
const GROUND_ALPHA: f32 = 0.8;

/// How much brighter each band is than the one under it, per row it starts at.
const BAND_SHADE_PER_ROW: f32 = 0.007;

/// Seconds between two of the stronger rulers, which mark minutes.
const MINUTE: usize = 60;

/// Seconds between two of the weaker rulers.
const TEN_SECONDS: usize = 10;

/// The grey of the ruler that marks a minute.
const MINUTE_RULER: f32 = 0.25;

/// The grey of the ruler that marks ten seconds.
const TEN_SECOND_RULER: f32 = 0.125;

/// How long the chips are kept as they were before they are compared with what the screen now
/// reports, while a run is being played.
const CHIPS_REFRESH_MS: i64 = 750;

/// How long the cursor is kept where it was while a run is being played.
const CURSOR_REFRESH_MS: i64 = 50;

/// How wide the cursor is, in pixels.
const CURSOR_WIDTH: i32 = 3;

/// Milliseconds a second is counted in, which is the unit the cursor's positions are given in.
const MS_PER_SECOND: i32 = 1000;

/// The colour of the cursor that marks where a practice range starts.
const START_CURSOR: Rgba = [128, 255, 128, 255];

/// The colour of the cursor that marks where a practice range ends.
const END_CURSOR: Rgba = [255, 128, 128, 255];

/// The colour of the cursor that marks where the run is.
const NOW_CURSOR: Rgba = [255, 255, 255, 255];

/// The colours of the note-kind graph, by class: a scratch's long note end, body and plain note,
/// then the same three for a key, then a mine.
const NOTE_KIND_COLORS: [u32; NOTE_KINDS] = [0x44ff44, 0x228822, 0xff4444, 0x4444ff, 0x222288, 0xcccccc, 0x880000];

/// The colours of the judgement graph, by class: unjudged, then the six judgements from the best.
/// The reference has six classes here because its sixth judgement is the poor.
const JUDGEMENT_COLORS: [u32; JUDGEMENTS] = [0x555555, 0x0088ff, 0x00ff88, 0xffff00, 0xff8800, 0xff0000];

/// The colours of the early/late graph, by class: unjudged, the best judgement, then the four
/// below it hit early and the same four hit late.
const EARLY_LATE_COLORS: [u32; EARLY_LATE_BUCKETS] = [0x555555, 0x44ff44, 0x0088ff, 0x0066cc, 0x004488, 0x002244, 0xff8800, 0xcc6600, 0x884400, 0x442200];

/// The judgement graph's colours for a chart in the nine-key pop'n mode, whose judgements have
/// colours of their own.
const POPN_JUDGEMENT_COLORS: [u32; JUDGEMENTS] = [0x555555, 0xff5eb0, 0xffbe32, 0xdc463c, 0x6cc6ff, 0x6cc6ff];

/// The early/late graph's colours for a chart in the nine-key pop'n mode.
const POPN_EARLY_LATE_COLORS: [u32; EARLY_LATE_BUCKETS] = [0x555555, 0xff5eb0, 0x0088ff, 0x0066cc, 0x004488, 0x002244, 0xff8800, 0xcc6600, 0x884400, 0x442200];

/// Where a play screen's cursor stands.
///
/// A frame that carries one is a play screen's, which is what makes the graph draw the cursor and
/// look for the run's judgements as they change.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayCursor {
    /// Where a practice range starts, in milliseconds into the chart.
    pub start_ms: Option<i32>,
    /// Where a practice range ends, in milliseconds into the chart.
    pub end_ms: Option<i32>,
    /// The speed a practice run is played at, as a multiple of the chart's own. The playing
    /// position is scaled by it when it is above zero.
    pub speed: Option<f32>,
}

/// How a chart's notes are spread, which is what a judgement graph plots.
///
/// Any screen that knows a chart may hand this over -- the browser and the decide screen for the
/// chart under the cursor, the play and score screens for the one being played -- which is what
/// lets one graph object be drawn on all of them. Each series is one row per second of the chart,
/// and the graph reads the one its record's `type` asks for: a series the screen does not know
/// leaves the graphs that read it undrawn.
#[derive(Debug, Clone, Copy)]
pub struct NoteDistribution<'a> {
    /// How many notes of a run took each judgement, best first, over the whole run. The graph does
    /// not read these; they are what a screen that has only the totals can still say.
    pub judged: &'a [u32; JUDGEMENTS],
    /// For `type` 0: how many notes of each kind the chart has in each second. A long note counts
    /// in its body class in every second it spans.
    pub kinds: &'a [[u32; NOTE_KINDS]],
    /// For `type` 1: how many notes of the chart took each judgement in each second, the first
    /// class being the notes not judged yet.
    pub judgements: &'a [[u32; JUDGEMENTS]],
    /// For `type` 2: the same, with the notes hit early and late counted apart.
    pub early_late: &'a [[u32; EARLY_LATE_BUCKETS]],
    /// Whether the chart is in the nine-key pop'n mode, whose judgements are coloured differently.
    pub popn: bool,
    /// Present on a play screen only.
    pub playing: Option<PlayCursor>,
}

impl Default for NoteDistribution<'_> {
    fn default() -> Self {
        NoteDistribution { judged: &[0; JUDGEMENTS], kinds: &[], judgements: &[], early_late: &[], popn: false, playing: None }
    }
}

impl<'a> NoteDistribution<'a> {
    /// The distribution of a run that is known only by how many notes took each judgement, best
    /// first. It carries no seconds, so no graph is drawn from it.
    pub fn of_judgements(judged: &'a [u32; JUDGEMENTS]) -> NoteDistribution<'a> {
        NoteDistribution { judged, ..NoteDistribution::default() }
    }

    /// The series `kind` reads, flattened, and how many classes each row of it holds.
    fn rows(&self, kind: Kind) -> (&'a [u32], usize) {
        match kind {
            Kind::NoteKinds => (self.kinds.as_flattened(), NOTE_KINDS),
            Kind::Judgements => (self.judgements.as_flattened(), JUDGEMENTS),
            Kind::EarlyLate => (self.early_late.as_flattened(), EARLY_LATE_BUCKETS),
        }
    }
}

/// Which of the three things a graph counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    NoteKinds,
    Judgements,
    EarlyLate,
}

impl Kind {
    fn of(graph_type: i32) -> Option<Kind> {
        match graph_type {
            TYPE_NOTE_KINDS => Some(Kind::NoteKinds),
            TYPE_JUDGEMENTS => Some(Kind::Judgements),
            TYPE_EARLY_LATE => Some(Kind::EarlyLate),
            _ => None,
        }
    }

    /// The colour of each class. Only the kinds that count judgements have a second palette.
    fn colors(self, popn: bool) -> &'static [u32] {
        match (self, popn) {
            (Kind::NoteKinds, _) => &NOTE_KIND_COLORS,
            (Kind::Judgements, false) => &JUDGEMENT_COLORS,
            (Kind::Judgements, true) => &POPN_JUDGEMENT_COLORS,
            (Kind::EarlyLate, false) => &EARLY_LATE_COLORS,
            (Kind::EarlyLate, true) => &POPN_EARLY_LATE_COLORS,
        }
    }
}

/// How a record asks its chips to be laid out.
#[derive(Debug, Clone, Copy)]
struct ChipStyle {
    /// Whether the last class is stacked first.
    reversed: bool,
    /// Whether a chip fills its whole pitch across (`noGapX`).
    wide: bool,
    /// Whether a chip fills its whole pitch up (`noGap`).
    tall: bool,
}

/// The judgement graph a document declares.
#[derive(Debug)]
pub(crate) struct JudgeGraphBody {
    /// What it counts, or `None` for a `type` the reference has no graph for.
    kind: Option<Kind>,
    /// Whether the ground is left out (`backTexOff`).
    ground_off: bool,
    /// Milliseconds of the scene over which the chips are revealed from the left.
    delay: i32,
    style: ChipStyle,
    state: RefCell<Drawn>,
}

/// What the graph has painted, kept between frames because the reference keeps it: the textures
/// are built when the numbers change and not every frame.
#[derive(Debug)]
struct Drawn {
    ground: Layer,
    chips: Layer,
    cursor: Layer,
    /// The colour of each class. Chosen when the graph is first drawn and kept for as long as it
    /// lives, as the reference does.
    palette: Option<Vec<Rgba>>,
    /// The numbers the textures show, flattened.
    data: Vec<u32>,
    /// Chip rows the texture is tall.
    rows: usize,
    ground_pixmap: Pixmap,
    chips_pixmap: Pixmap,
    cursor_pixmap: Pixmap,
    /// The scene time the numbers were last compared with the screen's, in milliseconds.
    compared_at: Option<i64>,
    /// The scene time the cursor was last painted, in milliseconds.
    cursor_at: Option<i64>,
}

impl Drawn {
    fn new() -> Drawn {
        let serial = next_serial();
        Drawn {
            ground: Layer::new(serial, "ground"),
            chips: Layer::new(serial, "chips"),
            cursor: Layer::new(serial, "cursor"),
            palette: None,
            data: Vec::new(),
            rows: 0,
            ground_pixmap: Pixmap::default(),
            chips_pixmap: Pixmap::default(),
            cursor_pixmap: Pixmap::default(),
            compared_at: None,
            cursor_at: None,
        }
    }

    /// Forgets the numbers, so the next ones are painted from scratch.
    fn forget(&mut self) {
        self.data.clear();
        self.rows = 0;
    }
}

impl JudgeGraphBody {
    /// Hands the textures back to the renderer they were uploaded to.
    pub(crate) fn release<R: Renderer>(&self, r: &mut R) {
        let mut drawn = self.state.borrow_mut();
        drawn.ground.release(r);
        drawn.chips.release(r);
        drawn.cursor.release(r);
        drawn.forget();
    }
}

/// The judgement graph behind `id`, or `None` when the document declares none by that name.
pub(super) fn build(def: &SkinDef, id: &str, warnings: &mut Vec<String>) -> Option<Body> {
    let graph = def.judgegraph.iter().find(|graph| graph.id == id)?;
    let kind = Kind::of(graph.graph_type);
    if kind.is_none() {
        warnings.push(format!(
            "judgement graph {id:?} asks for type {}, which the reference has no graph for (0, 1 and 2 are), so it draws nothing",
            graph.graph_type
        ));
    }
    Some(Body::JudgeGraph(JudgeGraphBody {
        kind,
        ground_off: graph.back_tex_off == 1,
        delay: graph.delay,
        style: ChipStyle { reversed: graph.order_reverse == 1, wide: graph.no_gap_x == 1, tall: graph.no_gap == 1 },
        state: RefCell::new(Drawn::new()),
    }))
}

/// How many chip rows a graph is tall, given the numbers it plots.
///
/// It starts at twenty and grows by tens when a second holds more notes than there are rows, up to a
/// hundred. A row is only added when a second has *more* notes than the rows so far, so the order
/// the seconds come in matters: a second of thirty after a second of twenty-five stays at thirty
/// rows, while a second of thirty on its own asks for forty.
fn row_count(data: &[u32], classes: usize) -> usize {
    let mut rows = MIN_ROWS;
    for second in data.chunks_exact(classes) {
        let count = second.iter().map(|count| *count as usize).sum::<usize>();
        if rows < count {
            rows = (count / ROW_STEP * ROW_STEP + ROW_STEP).min(MAX_ROWS);
        }
    }
    rows
}

/// Paints the ground: the dark field, the bands every ten rows, and the rulers every ten and sixty
/// seconds.
fn paint_ground(ground: &mut Pixmap, seconds: usize, rows: usize) {
    let (width, height) = ground.size();
    ground.fill(unit_color(0.0, 0.0, 0.0, GROUND_ALPHA));
    for band in (ROW_STEP..rows).step_by(ROW_STEP) {
        let shade = BAND_SHADE_PER_ROW * band as f32;
        ground.fill_rect(0, (band * PITCH) as i32, width as i32, BAND_HEIGHT, unit_color(shade, shade, 0.0, 1.0));
    }
    for second in 0..seconds {
        let ruler = if second % MINUTE == 0 {
            MINUTE_RULER
        } else if second % TEN_SECONDS == 0 {
            TEN_SECOND_RULER
        } else {
            continue;
        };
        ground.upright_line((second * PITCH) as i32, 0, height as i32, unit_color(ruler, ruler, ruler, 1.0));
    }
}

/// Paints the chips of the seconds in `span` over a texture of `rows` rows, after clearing them.
///
/// A second's classes are stacked from the foot up, each class as many chips tall as it counts, and
/// whatever does not fit in the rows is cut. A class that counts nothing, or less, adds no chip.
fn paint_chips(chips: &mut Pixmap, data: &[u32], palette: &[Rgba], rows: usize, style: ChipStyle, span: std::ops::Range<usize>) {
    let classes = palette.len();
    let wide = CHIP + i32::from(style.wide);
    let tall = CHIP + i32::from(style.tall);
    let (_, height) = chips.size();
    chips.fill_rect((span.start * PITCH) as i32, 0, ((span.end - span.start) * PITCH) as i32, height as i32, TRANSPARENT);
    for second in span {
        let counts = &data[second * classes..(second + 1) * classes];
        let mut row = 0;
        for step in 0..classes {
            let class = if style.reversed { classes - 1 - step } else { step };
            for _ in 0..counts[class] {
                if row >= rows {
                    break;
                }
                chips.fill_rect((second * PITCH) as i32, (row * PITCH) as i32, wide, tall, palette[class]);
                row += 1;
            }
        }
    }
}

/// Paints the cursor: where a practice range starts and ends, and where the run is, each three pixels
/// wide and as tall as the texture.
///
/// The positions are scaled to the texture with the reference's own integer arithmetic.
fn paint_cursor(cursor: &mut Pixmap, seconds: usize, marks: PlayCursor, played_ms: Option<i64>) {
    cursor.clear();
    let (width, height) = cursor.size();
    let (width, height) = (width as i32, height as i32);
    let span = seconds as i32 * MS_PER_SECOND;
    let mut mark = |at: i32, color: Rgba| cursor.fill_rect(at, 0, CURSOR_WIDTH, height, color);
    if let Some(start) = marks.start_ms.filter(|start| *start >= 0) {
        mark(start.wrapping_mul(width) / span, START_CURSOR);
    }
    if let Some(end) = marks.end_ms.filter(|end| *end >= 0) {
        mark(end.wrapping_mul(width) / span, END_CURSOR);
    }
    if let Some(played) = played_ms {
        let scaled = marks.speed.filter(|speed| *speed > 0.0).map_or(played as f32, |speed| played as f32 * speed);
        mark((scaled * width as f32 / span as f32) as i32, NOW_CURSOR);
    }
}

/// How much of the chips the scene has revealed by `now_ms`, from none to all.
fn revealed(now_ms: i64, delay: i32) -> f32 {
    if now_ms >= i64::from(delay) { 1.0 } else { now_ms as f32 / delay as f32 }
}

/// The seconds the first and last differing rows of two equally long sets of numbers lie in, or
/// `None` when they are the same.
fn changed_seconds(old: &[u32], new: &[u32], classes: usize) -> Option<std::ops::Range<usize>> {
    let mut differing = old.chunks_exact(classes).zip(new.chunks_exact(classes)).enumerate().filter(|(_, (old, new))| old != new).map(|(second, _)| second);
    let first = differing.next()?;
    let last = differing.next_back().unwrap_or(first);
    Some(first..last + 1)
}

impl Drawn {
    /// Brings the pixmaps up to date with what the screen reports, as the reference does when it
    /// draws.
    ///
    /// Different numbers than the pixmaps were painted for are painted afresh, unless this is a play
    /// screen counting judgements: there the chips are only compared with the screen's numbers every
    /// 750 ms, and only the seconds that changed are painted again.
    fn refresh(&mut self, body: &JudgeGraphBody, kind: Kind, notes: &NoteDistribution<'_>, now_ms: i64) {
        let (data, classes) = notes.rows(kind);
        let seconds = data.len() / classes;
        let Drawn { ground, chips, cursor, palette, data: held, rows, ground_pixmap, chips_pixmap, cursor_pixmap, compared_at, cursor_at } = self;
        let palette: &[Rgba] = palette.get_or_insert_with(|| kind.colors(notes.popn).iter().map(|color| hex_color(*color)).collect());
        let live = notes.playing.is_some() && kind != Kind::NoteKinds;

        if held.len() != data.len() || (!live && *held != data) {
            *rows = row_count(data, classes);
            *held = data.to_vec();
            let (width, height) = (seconds * PITCH, *rows * PITCH);
            *ground_pixmap = Pixmap::new(width, height);
            *chips_pixmap = Pixmap::new(width, height);
            *cursor_pixmap = Pixmap::new(width, height);
            if !body.ground_off {
                paint_ground(ground_pixmap, seconds, *rows);
            }
            paint_chips(chips_pixmap, held, palette, *rows, body.style, 0..seconds);
            ground.mark_stale();
            chips.mark_stale();
            cursor.mark_stale();
            *cursor_at = None;
        }

        if live && compared_at.is_none_or(|at| now_ms > at + CHIPS_REFRESH_MS) {
            if let Some(span) = changed_seconds(held, data, classes) {
                held.copy_from_slice(data);
                paint_chips(chips_pixmap, held, palette, *rows, body.style, span);
                chips.mark_stale();
            }
            *compared_at = Some(now_ms);
        }
    }
}

/// Draws the distribution: the ground, the chips as far as the scene has revealed them, and on a
/// play screen the cursor.
pub(crate) fn draw_judge_graph<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &JudgeGraphBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let (Some(kind), Some(notes)) = (body.kind, frame.data.series.notes) else {
        return false;
    };
    let (data, classes) = notes.rows(kind);
    let seconds = data.len() / classes;
    let Some(plot) = plot_of(place, rect).filter(|_| seconds > 0 && (seconds * PITCH) as u64 <= u64::from(r.max_texture_size())) else {
        return false;
    };
    let now_ms = frame.now_us / MICROS_PER_MILLI;

    let mut drawn = body.state.borrow_mut();
    drawn.refresh(body, kind, &notes, now_ms);
    if let Some(marks) = notes.playing
        && drawn.cursor_at.is_none_or(|at| now_ms > at + CURSOR_REFRESH_MS)
    {
        let played = frame.timers.is_on(timer_id::PLAY).then(|| (frame.now_us - frame.timers.value_us(timer_id::PLAY)) / MICROS_PER_MILLI);
        paint_cursor(&mut drawn.cursor_pixmap, seconds, marks, played);
        drawn.cursor.mark_stale();
        drawn.cursor_at = Some(now_ms);
    }

    let render = revealed(now_ms, body.delay);
    let Drawn { ground, chips, cursor, ground_pixmap, chips_pixmap, cursor_pixmap, .. } = &mut *drawn;
    let mut reached = false;
    if let Some(tex) = ground.texture(r, ground_pixmap) {
        reached |= draw_layer(r, place, tex, texels(ground_pixmap), texels(ground_pixmap).0, plot);
    }
    if let Some(tex) = chips.texture(r, chips_pixmap) {
        let size = texels(chips_pixmap);
        let shown = ((size.0 as f32 * render) as i32).clamp(0, size.0 as i32) as u32;
        reached |= draw_layer(r, place, tex, size, shown, Rect::new(plot.x, plot.y, plot.w * render, plot.h));
    }
    if notes.playing.is_some()
        && let Some(tex) = cursor.texture(r, cursor_pixmap)
    {
        reached |= draw_layer(r, place, tex, texels(cursor_pixmap), texels(cursor_pixmap).0, plot);
    }
    reached
}

/// A pixmap's size as a texture's.
fn texels(pixmap: &Pixmap) -> (u32, u32) {
    let (width, height) = pixmap.size();
    (width as u32, height as u32)
}

#[cfg(test)]
mod tests;
