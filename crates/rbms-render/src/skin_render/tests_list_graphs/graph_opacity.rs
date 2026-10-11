//! How every graph takes the alpha a player's offset leaves it: the gauge graph, the judgement
//! graph, the tempo graph, the timing distribution, the judge ruler and the hit errors.
//!
//! The shared fixture is loaded with each graph's destination naming one offset and drawn three
//! times against a host that answers it: with the offset at nothing, with it taking half of the
//! alpha away, and with it taking all of it. Each graph is drawn over black, so how much light its
//! rectangle holds is how much of the graph reached the canvas.

use std::borrow::Cow;

use rbms_skin::dst::{DrawStateSource, OffsetSource, SkinOffset};
use rbms_skin::property::{SkinHost, UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerState};

use super::{DOC_H, DOC_W, Fixture, GAUGE_REVEALED_MS, PLAYED_GAUGE_BORDER, Scratch, measured, wheel_screen_with};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::skin_render::{FrameData, FrameSeries, GaugeFrame, NoteDistribution, RecentHits, SkinFrame, SkinScreen};
use crate::{Color, CpuCanvas, Renderer};

/// The offset every graph of the fixture names.
const FADE_OFFSET: i32 = 77;

/// The members that name it, as a destination writes them.
const NAMING_THE_OFFSET: &str = r#""offset": 77,"#;

/// The shift that takes half of an opaque graph's alpha away, and the one that takes all of it.
const HALF_SHIFT: f32 = -128.0;
const WHOLE_SHIFT: f32 = -255.0;

/// How much of its opaque light a graph may hold at half its alpha, in hundredths: half, give or
/// take what the rounding of each pixel's blend and a mark's own alpha come to.
const HALF_LIGHT_PERCENT: std::ops::RangeInclusive<u64> = 35..=55;

/// A rectangle of the document, as `(x, y, w, h)` with `y` measured up from the document's foot.
type DocumentRect = (u32, u32, u32, u32);

/// One of the fixture's graphs: what a failure calls it, and the rectangle it is drawn over.
type NamedGraph = (&'static str, DocumentRect);

/// The fixture's graphs, by the frame that feeds them.
const SCORE_GRAPHS: [NamedGraph; 4] =
    [("gauge graph", (200, 100, 60, 40)), ("judge graph", (200, 60, 60, 30)), ("bpm graph", (200, 20, 60, 30)), ("timing distribution", (130, 100, 60, 40))];
const PLAY_GRAPHS: [NamedGraph; 2] = [("judge ruler", (130, 20, 60, 30)), ("hit errors", (60, 20, 60, 30))];

/// A game that answers nothing but one alpha shift, under the offset the graphs name.
struct Fading(f32);

impl OffsetSource for Fading {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        (id == FADE_OFFSET).then_some(SkinOffset { a: self.0, ..SkinOffset::default() })
    }
}

impl DrawStateSource for Fading {
    fn boolean(&self, id: i32) -> Option<bool> {
        Some(if id < 0 { !UNMAPPED_BOOLEAN } else { UNMAPPED_BOOLEAN })
    }
}

impl SkinHost for Fading {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, _id: i32) -> f32 {
        UNMAPPED_FLOAT
    }

    fn text(&self, _id: i32) -> Cow<'_, str> {
        Cow::Borrowed(UNMAPPED_STRING)
    }

    fn timer_us(&self, _id: i32) -> i64 {
        TIMER_OFF
    }

    fn now_us(&self) -> i64 {
        0
    }
}

/// Draws `screen` over black against a host shifting the graphs' alpha by `shift`.
fn draw_shifted(screen: &SkinScreen, text: &mut TextContext, canvas: &mut CpuCanvas, data: FrameData<'_>, shift: f32) -> usize {
    let timers = TimerState::new();
    let state = Fading(shift);
    canvas.clear(Color::BLACK);
    let mut ctx = RenderCtx::new(crate::theme::theme(), text);
    let frame = SkinFrame { now_us: GAUGE_REVEALED_MS * MICROS_PER_MILLI, timers: &timers, state: &state, lua: None, mouse: None, data };
    screen.draw(&mut ctx, canvas, &frame)
}

/// The light a rectangle of the document holds on the canvas: every channel of every pixel of it,
/// added up.
fn light(canvas: &CpuCanvas, rect: DocumentRect) -> u64 {
    let (x, y, w, h) = rect;
    let top = DOC_H - y - h;
    (top..top + h)
        .flat_map(|row| (x..x + w).map(move |column| (column, row)))
        .map(|(column, row)| {
            let pixel = canvas.pixel_at(column, row);
            u64::from(pixel.r) + u64::from(pixel.g) + u64::from(pixel.b)
        })
        .sum()
}

/// Holds each of `graphs` against what the three shifts should leave of it, drawing `data` afresh
/// for each.
fn assert_graphs_fade<'a>(screen: &SkinScreen, text: &mut TextContext, canvas: &mut CpuCanvas, data: impl Fn() -> FrameData<'a>, graphs: &[NamedGraph]) {
    let opaque_count = draw_shifted(screen, text, canvas, data(), 0.0);
    let opaque: Vec<u64> = graphs.iter().map(|(_, rect)| light(canvas, *rect)).collect();
    for ((name, _), lit) in graphs.iter().zip(&opaque) {
        assert!(*lit > 0, "the {name} is not on the canvas with the offset at nothing");
    }

    assert_eq!(draw_shifted(screen, text, canvas, data(), HALF_SHIFT), opaque_count, "a graph at half its alpha is still drawn");
    for ((name, rect), opaque) in graphs.iter().zip(&opaque) {
        let percent = light(canvas, *rect) * 100 / opaque;
        assert!(HALF_LIGHT_PERCENT.contains(&percent), "the {name} holds {percent}% of its light at half its alpha");
    }

    assert_eq!(draw_shifted(screen, text, canvas, data(), WHOLE_SHIFT), opaque_count - graphs.len(), "a graph with no alpha left is drawn");
    for (name, rect) in graphs {
        assert_eq!(light(canvas, *rect), 0, "something of the {name} reached the canvas with no alpha left");
    }
}

/// The four graphs of a score screen fade with the offset their destinations name.
#[test]
fn the_graphs_of_a_finished_run_fade_with_the_offset_their_destinations_name() {
    let scratch = Scratch::new("graph-opacity-score");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen_with(&scratch, &mut canvas, &mut text, Fixture { graph_members: Some(NAMING_THE_OFFSET), ..Fixture::default() });

    let gauge = [20.0, 40.0, 60.0, 50.0];
    let hist = [1, 4, 9, 4, 1];
    let counts = [10, 4, 2, 1, 1, 0];
    let kinds = [[0, 0, 1, 0, 0, 2, 0], [0, 0, 0, 0, 0, 3, 1]];
    let tempo = [(0.0, 150.0), (0.5, 200.0), (0.8, 120.0)];
    let data = || measured(&gauge, &hist, NoteDistribution { kinds: &kinds, ..NoteDistribution::of_judgements(&counts) }, &tempo);
    assert_graphs_fade(&screen, &mut text, &mut canvas, data, &SCORE_GRAPHS);
}

/// The two graphs of a play screen fade with the offset their destinations name, the marks each
/// draws over its ground with the ground.
#[test]
fn the_graphs_of_a_run_in_progress_fade_with_the_offset_their_destinations_name() {
    let scratch = Scratch::new("graph-opacity-play");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen_with(&scratch, &mut canvas, &mut text, Fixture { graph_members: Some(NAMING_THE_OFFSET), ..Fixture::default() });

    let hits = [(-30_i64, 1_u8), (8, 0), (45, 2), (-12, 0), (20, 1), (3, 0)];
    let data = || FrameData {
        gauge: Some(GaugeFrame::of_kind(0, PLAYED_GAUGE_BORDER)),
        series: FrameSeries { recent_hits: Some(RecentHits::new(&hits)), ..FrameSeries::default() },
        ..FrameData::default()
    };
    assert_graphs_fade(&screen, &mut text, &mut canvas, data, &PLAY_GRAPHS);
}
