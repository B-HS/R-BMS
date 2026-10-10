//! Unit tests for the browser's song wheel and for every graph a document draws itself, including
//! the colour parser they share.
//!
//! The wheel and the graphs are the two object kinds that read whole series rather than single
//! properties, so each test here loads a real document, hands it a frame's worth of rows or series,
//! and checks what reached the canvas -- which is the only way to tell "drew nothing because the
//! series was empty" from "drew nothing because the object never resolved".

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use rbms_skin::dst::{DrawStateSource, OffsetSource, SkinOffset};
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::property::generated::OPTION_PANEL1;
use rbms_skin::property::{SkinHost, UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerState};

use super::color::{modulate, parse_hex_color};
use super::gauge::{GAUGE_TYPES, GaugeScale};
use super::state::SelectViewState;
use super::{
    BpmTimeline, FrameData, FrameSeries, GaugeFrame, GaugeHistory, NoteDistribution, NoteField, RecentHits, SkinAssets, SkinFrame, SkinImage, SkinObjectKind,
    SkinScreen, SongBars, TimingHistogram,
};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::playfield::{LaneShade, PlayfieldView};
use crate::select::{SelectDetail, SelectRow, SelectView};
use crate::skin::Skin;
use crate::{BYTES_PER_PIXEL, Color, CpuCanvas, Renderer};

/// Width and height the fixture document is authored at, which is also the canvas every frame here
/// is drawn on, so a document rectangle and a screen rectangle differ only by the vertical flip.
const DOC_W: u32 = 320;
const DOC_H: u32 = 180;

/// Slots the fixture wheel declares.
const SLOTS: usize = 15;

/// The slot the fixture wheel calls its centre.
const CENTER: usize = 7;

/// Where the top slot sits in the document, and how the slots are spaced down from it.
const SLOT_TOP: i32 = 170;
const SLOT_PITCH: i32 = 11;
const SLOT_H: i32 = 10;

/// How wide an unfocused slot's bar is, and how much wider the focused one is.
const BAR_W: i32 = 100;
const FOCUS_W: i32 = 110;

/// Where a slot's title box starts and how wide it is, which is also where it lands on screen
/// because the fixture is drawn at the size it was authored at.
const TITLE_X: i32 = 2;
const TITLE_W: i32 = 60;

/// How bright a channel has to be to be the white of a title rather than the red of the bar under
/// it, so a scan can tell one from the other.
const INK_LEVEL: u8 = 150;

/// The rows the fixture browser is showing and which of them is focused.
const ROWS: usize = 5;
const SELECTED: usize = 2;

/// Size of the one texture the fixture's images are cut from.
const TEX_SIZE: u32 = 4;

/// A source that decodes to one opaque white square, so an image drawn through it lands on the
/// canvas as exactly the colour it was tinted with.
struct SolidAssets;

impl SkinAssets for SolidAssets {
    fn image(&mut self, _path: &Path) -> Option<SkinImage> {
        SkinImage::new(TEX_SIZE, TEX_SIZE, vec![u8::MAX; (TEX_SIZE * TEX_SIZE) as usize * BYTES_PER_PIXEL])
    }
}

/// A state source that answers nothing, because every fixture object reads its numbers from the
/// frame's screen-shaped state rather than from a property id.
struct Nothing;

impl OffsetSource for Nothing {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }
}

impl DrawStateSource for Nothing {
    fn boolean(&self, id: i32) -> Option<bool> {
        Some(if id < 0 { !UNMAPPED_BOOLEAN } else { UNMAPPED_BOOLEAN })
    }
}

impl SkinHost for Nothing {
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

/// A scratch folder holding the generated document and its source, removed when the test ends.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-skin-list-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        Scratch { root }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Where slot `index` sits in the document, from the top down.
fn slot_y(index: usize) -> i32 {
    SLOT_TOP - SLOT_PITCH * index as i32
}

/// One of the wheel's nested lists, written out slot by slot.
fn slot_list(id: &str, x: i32, w: i32, color: (u8, u8, u8)) -> String {
    let (r, g, b) = color;
    (0..SLOTS)
        .map(|index| {
            let y = slot_y(index);
            format!(r#"{{"id":"{id}","dst":[{{"x":{x},"y":{y},"w":{w},"h":{SLOT_H},"r":{r},"g":{g},"b":{b}}}]}}"#)
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// What one generated document varies from the shared fixture, so a test that needs the wheel gated
/// off, clipped, or drawing its bars from an image set writes only the part it is about.
#[derive(Default, Clone, Copy)]
struct Fixture<'a> {
    /// The members of the wheel's own destination keyframe, which every slot hangs off.
    wheel_dst: Option<&'a str>,
    /// The document's `imageset` records, as JSON members.
    imageset: &'a str,
    /// The document's one `judgegraph` record.
    judge_graph: Option<&'a str>,
}

/// A browser document carrying a fifteen-slot wheel, a plain button and one of every graph, so
/// one load exercises every object this file is about.
fn write_document(scratch: &Scratch, fixture: Fixture<'_>) -> PathBuf {
    std::fs::write(scratch.root.join("sheet.tex"), "solid").expect("the source file is writable");
    let clickable: Vec<String> = (0..SLOTS).map(|index| index.to_string()).collect();
    let body = format!(
        r#"{{
            "type": 5, "name": "wheel fixture", "w": {DOC_W}, "h": {DOC_H},
            "source": [{{ "id": "sheet", "path": "sheet.tex" }}],
            "image": [{{ "id": "bar", "src": "sheet" }}, {{ "id": "button", "src": "sheet" }}],
            "imageset": [{imageset}],
            "text": [{{ "id": "row-title", "font": "none", "size": 10, "align": 0 }}],
            "gaugegraph": [{{
                "id": "gauge-graph",
                "grooveClearAndHardBGColor": "203040",
                "grooveClearAndHardLineColor": "00FF00",
                "borderColor": "FF0000",
                "borderlineColor": "FFFF00"
            }}],
            "judgegraph": [{judge_graph}],
            "bpmgraph": [{{ "id": "bpm-graph" }}],
            "timingdistributiongraph": [{{ "id": "timing-dist", "width": 60, "devColor": "not a colour" }}],
            "timingvisualizer": [{{ "id": "ruler" }}],
            "hiterrorvisualizer": [{{ "id": "errors" }}],
            "songlist": {{
                "id": "wheel",
                "center": {CENTER},
                "clickable": [{clickable}],
                "listoff": [{listoff}],
                "liston": [{liston}],
                "text": [{text}],
                "lamp": [{lamp}]
            }},
            "destination": [
                {{ "id": "wheel", "dst": [{{ {wheel_dst} }}] }},
                {{ "id": "button", "dst": [{{ "x": 10, "y": 20, "w": 40, "h": 12 }}] }},
                {{ "id": "gauge-graph", "dst": [{{ "x": 200, "y": 100, "w": 60, "h": 40 }}] }},
                {{ "id": "judge-graph", "dst": [{{ "x": 200, "y": 60, "w": 60, "h": 30 }}] }},
                {{ "id": "bpm-graph", "dst": [{{ "x": 200, "y": 20, "w": 60, "h": 30 }}] }},
                {{ "id": "timing-dist", "dst": [{{ "x": 130, "y": 100, "w": 60, "h": 40 }}] }},
                {{ "id": "ruler", "dst": [{{ "x": 130, "y": 20, "w": 60, "h": 30 }}] }},
                {{ "id": "errors", "dst": [{{ "x": 60, "y": 20, "w": 60, "h": 30 }}] }}
            ]
        }}"#,
        clickable = clickable.join(","),
        imageset = fixture.imageset,
        judge_graph = fixture.judge_graph.unwrap_or(r#"{ "id": "judge-graph" }"#),
        wheel_dst = fixture.wheel_dst.map_or_else(|| format!(r#""x": 0, "y": 0, "w": 120, "h": {DOC_H}"#), str::to_owned),
        listoff = slot_list("bar", 0, BAR_W, (40, 60, 200)),
        liston = slot_list("bar-on", 0, FOCUS_W, (240, 60, 60)),
        text = slot_list("row-title", TITLE_X, TITLE_W, (255, 255, 255)),
        lamp = slot_list("lamp", 112, 6, (255, 255, 255)),
    );
    let path = scratch.root.join("wheel.json");
    std::fs::write(&path, body).expect("the document is writable");
    path
}

/// Loads the shared fixture, fills in the wheel's assembled slots and compiles it into a screen.
fn wheel_screen(scratch: &Scratch, canvas: &mut CpuCanvas, text: &mut TextContext) -> SkinScreen {
    wheel_screen_with(scratch, canvas, text, Fixture::default())
}

/// The same, for a document that varies from the shared fixture.
fn wheel_screen_with(scratch: &Scratch, canvas: &mut CpuCanvas, text: &mut TextContext, fixture: Fixture<'_>) -> SkinScreen {
    let document = write_document(scratch, fixture);
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&document, options).expect("the generated document loads");
    let slots = skin.nested.songlist.as_ref().expect("the loader assembled the wheel's slots");
    assert_eq!(slots.listoff.len(), SLOTS, "every slot the document declared was assembled");

    SkinScreen::build(canvas, text, &skin, &mut SolidAssets)
}

/// One browser row, distinguishable from its neighbours by its title alone.
fn row(index: usize) -> SelectRow {
    SelectRow {
        folder: false,
        title: format!("ROW {index}"),
        mode_short: "7K",
        mode_color: Color::BLUE,
        level: "12".to_owned(),
        difficulty_color: Color::RED,
        lamp: Color::GREEN,
        folder_count: None,
        dj_level: None,
        favorite: false,
    }
}

/// One frame over `data` at the start of the scene, with nothing running and no pointer.
fn frame<'a>(timers: &'a TimerState, state: &'a Nothing, data: FrameData<'a>) -> SkinFrame<'a> {
    SkinFrame { now_us: 0, timers, state, lua: None, mouse: None, data }
}

/// The gauge the reference clears at eighty of a hundred.
const GROOVE: GaugeScale = GaugeScale::new(2.0, 100.0, 80.0);

/// A gauge that clears at nothing, as the hard gauges do.
const SURVIVAL: GaugeScale = GaugeScale::new(0.0, 100.0, 0.0);

/// The reference's number for the normal gauge.
const NORMAL_GAUGE: usize = 2;

/// The reference's number for the hard gauge.
const HARD_GAUGE: usize = 3;

/// The scene time by which a gauge graph has revealed all of its line.
const GAUGE_REVEALED_MS: i64 = 1_500;

/// The limits of the nine gauges of a seven-key run: the three that clear part way up, then the six
/// that clear at nothing.
fn scales() -> [GaugeScale; GAUGE_TYPES] {
    std::array::from_fn(|gauge_type| if gauge_type < HARD_GAUGE { GROOVE } else { SURVIVAL })
}

/// A score screen showing the gauge numbered `gauge_type`, which the run left at `value`.
fn shown(gauge_type: usize, value: f32) -> GaugeFrame {
    GaugeFrame::finished(gauge_type, value, scales())
}

/// What a browser frame carries: its bars and nothing else.
fn browsing<'a>(bars: &'a SongBars<'a>) -> FrameData<'a> {
    FrameData { bars: Some(bars), ..FrameData::default() }
}

/// What a score frame carries: the four series a finished run was measured into, and the hard gauge
/// it was played on.
fn measured<'a>(gauge: &'a [f32], hist: &'a [u32], notes: NoteDistribution<'a>, tempo: &'a [(f32, f64)]) -> FrameData<'a> {
    let series = FrameSeries {
        gauge_history: Some(GaugeHistory::new(gauge)),
        timing: Some(TimingHistogram::new(hist)),
        bpm: Some(BpmTimeline::new(tempo)),
        notes: Some(notes),
        recent_hits: None,
    };
    FrameData { series, gauge: Some(shown(HARD_GAUGE, gauge.last().copied().unwrap_or_default())), ..FrameData::default() }
}

/// Draws `screen` over a cleared canvas and answers how many of its objects reached it.
fn draw(screen: &SkinScreen, text: &mut TextContext, canvas: &mut CpuCanvas, data: FrameData<'_>) -> usize {
    draw_at(screen, text, canvas, data, 0)
}

/// The same, `now_ms` milliseconds into the scene.
fn draw_at(screen: &SkinScreen, text: &mut TextContext, canvas: &mut CpuCanvas, data: FrameData<'_>, now_ms: i64) -> usize {
    let timers = TimerState::new();
    let state = Nothing;
    canvas.clear(Color::BLACK);
    let mut ctx = RenderCtx::new(crate::theme::theme(), text);
    screen.draw(&mut ctx, canvas, &SkinFrame { now_us: now_ms * MICROS_PER_MILLI, ..frame(&timers, &state, data) })
}

#[test]
fn an_opaque_colour_keeps_full_alpha() {
    assert_eq!(parse_hex_color("69F1E4"), Some(Color { r: 0x69, g: 0xF1, b: 0xE4, a: 255 }));
    assert_eq!(parse_hex_color("#69f1e4"), Some(Color { r: 0x69, g: 0xF1, b: 0xE4, a: 255 }));
}

#[test]
fn a_colour_with_its_own_alpha_keeps_it() {
    assert_eq!(parse_hex_color("00000080"), Some(Color { r: 0, g: 0, b: 0, a: 0x80 }));
}

#[test]
fn text_that_is_not_a_colour_is_refused() {
    for text in ["", "69F1E", "69F1E4F", "gggggg", "69F1E4FFFF"] {
        assert_eq!(parse_hex_color(text), None, "{text:?} was read as a colour");
    }
}

/// An unstated destination is opaque white, so modulating by it has to be the identity: every graph
/// would otherwise be drawn dimmer than the colour its own record named.
#[test]
fn an_opaque_white_destination_leaves_a_colour_alone() {
    let color = Color { r: 0x20, g: 0x40, b: 0x80, a: 0xC0 };
    assert_eq!(modulate(color, Color::rgb(255, 255, 255)), color);
    assert_eq!(modulate(color, Color { r: 255, g: 255, b: 255, a: 0 }).a, 0, "a destination faded out takes the graph with it");
    assert_eq!(modulate(color, Color::rgb(255, 0, 0)), Color { r: 0x20, g: 0, b: 0, a: 0xC0 });
}

/// A browser showing nothing, for the tests that are about the panel over it rather than the list
/// under it.
fn empty_browser() -> SelectView {
    SelectView {
        rows: Vec::new(),
        sel: 0,
        header: String::new(),
        guide: "",
        detail: SelectDetail::Empty,
        modal: None,
        score_graph: false,
        search: None,
        sort: "DEFAULT",
        filter: None,
        empty_hint: None,
    }
}

/// Whether the option panel is open reaches a document through the browser's own state source,
/// under the reference's first panel option, because that is the only thing a document can address.
#[test]
fn the_option_panel_reports_itself_open_through_the_reference_option() {
    let view = empty_browser();

    assert_eq!(SelectViewState::new(&view, 0, None, true).boolean(OPTION_PANEL1), Some(true), "an open panel does not report itself open");
    assert_eq!(SelectViewState::new(&view, 0, None, false).boolean(OPTION_PANEL1), Some(false), "a closed panel reports itself open");
}

#[test]
fn the_document_resolves_one_object_of_every_kind_this_file_draws() {
    let scratch = Scratch::new("kinds");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    for kind in [
        SkinObjectKind::SongList,
        SkinObjectKind::GaugeGraph,
        SkinObjectKind::JudgeGraph,
        SkinObjectKind::BpmGraph,
        SkinObjectKind::TimingDistribution,
        SkinObjectKind::TimingVisualizer,
        SkinObjectKind::HitError,
    ] {
        assert_eq!(screen.count_of(kind), 1, "{kind:?} did not resolve: {:?}", screen.warnings());
    }
}

/// The wheel is a ring the rows move through: the focused chart lands on the slot the document
/// called its centre, the slots either side of it hold its neighbours, and the slots past the ends
/// of the list hold nothing at all.
#[test]
fn the_focused_chart_lands_on_the_centre_slot() {
    let scratch = Scratch::new("centre");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let list = SongBars { rows: &rows, sel: SELECTED, options_open: false };
    draw(&screen, &mut text, &mut canvas, browsing(&list));

    let bar_pixel = |slot: usize| canvas.pixel_at(BAR_W as u32 - 5, DOC_H - (slot_y(slot) + SLOT_H) as u32 + 2);
    assert_eq!(bar_pixel(CENTER), Color { r: 240, g: 60, b: 60, a: 255 }, "the focused chart draws the focused bar on the centre slot");
    assert_eq!(bar_pixel(CENTER - 1), Color { r: 40, g: 60, b: 200, a: 255 }, "the slot above it holds the row before it, unfocused");
    assert_eq!(bar_pixel(CENTER + 2), Color { r: 40, g: 60, b: 200, a: 255 }, "and the last row of the list still lands two slots below");
    assert_eq!(bar_pixel(CENTER + 3), Color::BLACK, "a slot past the end of the list draws nothing");
    assert_eq!(bar_pixel(CENTER - 3), Color::BLACK, "and neither does one before its start");
}

/// The wheel needs the browser's rows, which only a select frame carries; a document that places
/// one on another screen draws no wheel rather than an empty ring of bars.
#[test]
fn a_wheel_without_the_browsers_rows_draws_nothing() {
    let scratch = Scratch::new("no-rows");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameData::default()), 1, "only the button, which reads nothing, is left");
}

/// Every graph reads a series that only one screen's state carries, so which of them draw is decided
/// by the frame rather than by the document.
#[test]
fn a_graph_draws_only_on_the_screen_whose_series_it_reads() {
    let scratch = Scratch::new("screens");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let gauge = [20.0, 40.0, 60.0, 50.0];
    let hist = [1, 4, 9, 4, 1];
    let counts = [10, 4, 2, 1, 1, 0];
    let kinds = [[0, 0, 1, 0, 0, 2, 0], [0, 0, 0, 0, 0, 3, 1]];
    let tempo = [(0.0, 150.0), (0.5, 200.0), (0.8, 120.0)];
    let notes = NoteDistribution { kinds: &kinds, ..NoteDistribution::of_judgements(&counts) };
    let series = measured(&gauge, &hist, notes, &tempo);
    assert_eq!(draw(&screen, &mut text, &mut canvas, series), 5, "the button and the four score-screen graphs");

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let list = SongBars { rows: &rows, sel: SELECTED, options_open: false };
    assert_eq!(draw(&screen, &mut text, &mut canvas, browsing(&list)), 2, "the button and the wheel, which is all a browser frame feeds");

    let field = Skin::default_for(rbms_model::Mode::BEAT_7K, DOC_W as f32, DOC_H as f32);
    let playfield = PlayfieldView { timelines: &[], microtime: 0, hispeed: 1.0, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let hits = [(-30_i64, 1_u8), (8, 0), (45, 2)];
    let play = NoteField { field: &field, playfield: &playfield, shade: LaneShade::default(), bomb: &[], keys_down: &[] };
    let playing = FrameData {
        field: Some(&play),
        gauge: Some(GaugeFrame::of_kind(0, field.gauge_clear_threshold)),
        series: FrameSeries { recent_hits: Some(RecentHits::new(&hits)), ..FrameSeries::default() },
        ..FrameData::default()
    };
    assert_eq!(draw(&screen, &mut text, &mut canvas, playing), 3, "the button, the judge ruler and the hit errors");
}

/// A run that measured nothing still has a gauge graph and a timing graph, as it has in the
/// reference: each draws its ground with nothing on it. A frame that carries no series at all leaves
/// every graph out.
#[test]
fn an_empty_run_draws_the_grounds_and_a_frame_with_no_series_draws_no_graph() {
    let scratch = Scratch::new("empty");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let series = measured(&[], &[], NoteDistribution::default(), &[]);
    assert_eq!(draw(&screen, &mut text, &mut canvas, series), 3, "the button, and the grounds of the gauge graph and the timing graph");
    assert_eq!(canvas.pixel_at(GAUGE_PANEL.0 + 30, GAUGE_PANEL.1 + 20), Color::rgb(0x20, 0x30, 0x40), "the gauge graph's ground with no line on it");
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameData::default()), 1, "only the button is left when the frame carries nothing");
}

/// Where the fixture's gauge graph lands on the canvas: its left edge and its top row. It is sixty
/// pixels by forty, so row `r` of the pixmap, counted from the foot, is canvas row `top + 39 - r`.
const GAUGE_PANEL: (u32, u32) = (200, DOC_H - 140);

/// The canvas row the gauge graph's pixmap row `row` lands on.
fn gauge_row(row: u32) -> u32 {
    GAUGE_PANEL.1 + 39 - row
}

/// A run that recorded the normal gauge at ninety and the hard gauge at fifty throughout.
fn two_gauges() -> Vec<Vec<f32>> {
    (0..GAUGE_TYPES)
        .map(|gauge_type| match gauge_type {
            NORMAL_GAUGE => vec![90.0; 8],
            HARD_GAUGE => vec![50.0; 8],
            _ => Vec::new(),
        })
        .collect()
}

/// A score frame carrying a run's gauges and the one of them that is shown.
fn gauges<'a>(history: GaugeHistory<'a>, gauge: GaugeFrame) -> FrameData<'a> {
    FrameData { series: FrameSeries { gauge_history: Some(history), ..FrameSeries::default() }, gauge: Some(gauge), ..FrameData::default() }
}

/// The gauge graph plots the history of the gauge the frame says is shown, in that gauge's colours
/// and against its clear line, and paints itself again when the shown gauge changes.
#[test]
fn the_gauge_history_is_drawn_for_the_gauge_that_is_shown_in_the_colours_the_document_named() {
    let scratch = Scratch::new("gauge");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);
    let kinds = two_gauges();
    let history = GaugeHistory::of_kinds(&kinds);
    let x = GAUGE_PANEL.0 + 30;
    let (ground, line) = (Color::rgb(0x20, 0x30, 0x40), Color::rgb(0, 255, 0));

    draw_at(&screen, &mut text, &mut canvas, gauges(history, shown(HARD_GAUGE, 50.0)), GAUGE_REVEALED_MS);
    assert_eq!(
        (canvas.pixel_at(x, gauge_row(19)), canvas.pixel_at(x, gauge_row(20))),
        (line, line),
        "fifty of a hundred is row nineteen of the thirty-eight the line climbs"
    );
    assert_eq!((canvas.pixel_at(x, gauge_row(18)), canvas.pixel_at(x, gauge_row(21))), (ground, ground), "and the line is two pixels thick");
    assert_eq!(
        (canvas.pixel_at(GAUGE_PANEL.0, gauge_row(39)), canvas.pixel_at(GAUGE_PANEL.0 + 59, gauge_row(0))),
        (ground, ground),
        "one ground for a gauge that clears at nothing"
    );
    assert_eq!(canvas.pixel_at(GAUGE_PANEL.0 + 59, gauge_row(19)), line, "the last sample's run reaches the right edge");

    draw_at(&screen, &mut text, &mut canvas, gauges(history, shown(NORMAL_GAUGE, 90.0)), GAUGE_REVEALED_MS);
    let (below, above, over) = (Color::rgb(0, 0x44, 0), Color::rgb(255, 0, 0), Color::rgb(255, 255, 0));
    assert_eq!((canvas.pixel_at(x, gauge_row(0)), canvas.pixel_at(x, gauge_row(31))), (below, below), "below the clear line at eighty");
    assert_eq!((canvas.pixel_at(x, gauge_row(32)), canvas.pixel_at(x, gauge_row(39))), (above, above), "and from it to the top");
    assert_eq!((canvas.pixel_at(x, gauge_row(34)), canvas.pixel_at(x, gauge_row(35))), (over, over), "ninety is above the line, in the colour for that side");
    assert_eq!(canvas.pixel_at(x, gauge_row(19)), below, "and the hard gauge's line is gone");

    draw_at(&screen, &mut text, &mut canvas, gauges(history, shown(HARD_GAUGE, 50.0)), GAUGE_REVEALED_MS);
    assert_eq!((canvas.pixel_at(x, gauge_row(19)), canvas.pixel_at(x, gauge_row(34))), (line, ground), "switching back paints the hard gauge again");

    draw_at(&screen, &mut text, &mut canvas, gauges(history, shown(HARD_GAUGE + 3, 50.0)), GAUGE_REVEALED_MS);
    assert_eq!(canvas.pixel_at(x, gauge_row(5)), ground, "a course gauge is drawn in the colours of the gauge it is a harder form of");
    assert_eq!(canvas.pixel_at(x, gauge_row(19)), ground, "over its own history, which this run left empty");
}

/// The ground is there from the first frame and the line is uncovered from the left over a second
/// and a half of the scene.
#[test]
fn the_gauge_line_is_revealed_from_the_left_and_the_ground_is_not() {
    let scratch = Scratch::new("gauge-reveal");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);
    let kinds = two_gauges();
    let frame = gauges(GaugeHistory::of_kinds(&kinds), shown(HARD_GAUGE, 50.0));
    let (ground, line) = (Color::rgb(0x20, 0x30, 0x40), Color::rgb(0, 255, 0));
    let line_columns = |canvas: &CpuCanvas| (0..60).filter(|column| canvas.pixel_at(GAUGE_PANEL.0 + column, gauge_row(19)) == line).count();

    for (now_ms, columns) in [(0, 0), (375, 15), (750, 30), (1_125, 45), (1_499, 59), (1_500, 60), (60_000, 60)] {
        assert_eq!(draw_at(&screen, &mut text, &mut canvas, frame, now_ms), 2, "the button and the graph");
        assert_eq!(line_columns(&canvas), columns, "{now_ms} ms into the scene");
        assert_eq!(canvas.pixel_at(GAUGE_PANEL.0 + 59, gauge_row(30)), ground, "the ground is whole at {now_ms} ms");
    }
    draw_at(&screen, &mut text, &mut canvas, frame, 750);
    assert_eq!(canvas.pixel_at(GAUGE_PANEL.0 + 29, gauge_row(19)), line, "what is uncovered is the left of the line, not the line squeezed");
    assert_eq!(canvas.pixel_at(GAUGE_PANEL.0 + 30, gauge_row(19)), ground);
}

/// A gauge graph needs both halves of what the reference reads: the history, and the gauge that
/// says which history is shown and where it clears.
#[test]
fn a_gauge_graph_with_no_gauge_to_show_or_no_history_draws_nothing() {
    let scratch = Scratch::new("gauge-missing");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);
    let kinds = two_gauges();
    let history = GaugeHistory::of_kinds(&kinds);

    let unshown = FrameData { series: FrameSeries { gauge_history: Some(history), ..FrameSeries::default() }, ..FrameData::default() };
    assert_eq!(draw_at(&screen, &mut text, &mut canvas, unshown, GAUGE_REVEALED_MS), 1);
    let unrecorded = FrameData { gauge: Some(shown(HARD_GAUGE, 50.0)), ..FrameData::default() };
    assert_eq!(draw_at(&screen, &mut text, &mut canvas, unrecorded, GAUGE_REVEALED_MS), 1);
    let short = vec![vec![50.0; 8]; HARD_GAUGE];
    assert_eq!(
        draw_at(&screen, &mut text, &mut canvas, gauges(GaugeHistory::of_kinds(&short), shown(HARD_GAUGE, 50.0)), GAUGE_REVEALED_MS),
        1,
        "no history for that gauge"
    );
    assert_eq!(draw_at(&screen, &mut text, &mut canvas, gauges(history, shown(HARD_GAUGE, 50.0)), GAUGE_REVEALED_MS), 2);
}

/// A course's graph runs through every stage, with an upright where one stage ends.
#[test]
fn the_end_of_a_courses_stage_is_marked_on_the_gauge_graph() {
    let scratch = Scratch::new("gauge-course");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);
    let kinds = two_gauges();
    let history = GaugeHistory::of_kinds(&kinds).with_sections(&[4, 8]);

    draw_at(&screen, &mut text, &mut canvas, gauges(history, shown(HARD_GAUGE, 50.0)), GAUGE_REVEALED_MS);
    let marked = GAUGE_PANEL.0 + 22;
    let mark = Color::rgb(255, 255, 255);
    assert_eq!(canvas.pixel_at(marked, gauge_row(39)), mark, "sixty pixels over eight samples puts the fourth on column twenty-two");
    assert_eq!(canvas.pixel_at(marked, gauge_row(0)), mark);
    assert_eq!(canvas.pixel_at(marked, gauge_row(19)), Color::rgb(0, 255, 0), "the line runs over the mark");
    assert_eq!(canvas.pixel_at(marked + 1, gauge_row(39)), Color::rgb(0x20, 0x30, 0x40));
}

/// Where the fixture's timing graph lands on the canvas: its left edge and its top row. Its record
/// asks for sixty columns, one to a pixel, and a run whose fullest millisecond holds ten hits is ten
/// rows tall, four pixels to a row.
const TIMING_PANEL: (u32, u32) = (130, DOC_H - 140);

/// Canvas pixels one row of that graph is tall.
const TIMING_ROW_PX: u32 = 4;

/// The colour of the timing graph at `column` and `row`, the top row being row zero.
fn timing_pixel(canvas: &CpuCanvas, column: u32, row: u32) -> Color {
    canvas.pixel_at(TIMING_PANEL.0 + column, TIMING_PANEL.1 + row * TIMING_ROW_PX + 1)
}

/// The timing graph is the reference's pixmap scaled onto the object: windows behind, a tick every
/// ten milliseconds along the top, the mean and the deviation as uprights, and a bar from the foot
/// for every millisecond, early to the right.
#[test]
fn the_timing_spread_is_drawn_as_bars_over_the_judgement_windows_with_its_mean_and_deviation() {
    let scratch = Scratch::new("timing");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let mut bins = [0_u32; 301];
    (bins[150], bins[160], bins[130]) = (10, 5, 2);
    let windows = [[-5, 5], [-15, 15], [-40, 40], [-40, 40], [-40, 40]];
    let timing = TimingHistogram::new(&bins).with_judge_area(windows);
    let data = FrameData { series: FrameSeries { timing: Some(timing), ..FrameSeries::default() }, ..FrameData::default() };
    assert_eq!(draw(&screen, &mut text, &mut canvas, data), 2, "the button and the graph");

    let (bar, mean, deviation) = (Color::rgb(0, 255, 0), Color::rgb(255, 255, 255), Color::rgb(255, 0, 0));
    let (perfect, great, good) = (Color::rgb(0, 0, 0x88), Color::rgb(0, 0x88, 0), Color::rgb(0x88, 0x88, 0));
    assert_eq!((timing_pixel(&canvas, 30, 0), timing_pixel(&canvas, 30, 9)), (bar, bar), "ten hits on time fill the middle column");
    assert_eq!((timing_pixel(&canvas, 40, 4), timing_pixel(&canvas, 40, 5)), (deviation, bar), "five hits ten milliseconds early, to the right");
    assert_eq!((timing_pixel(&canvas, 10, 7), timing_pixel(&canvas, 10, 8)), (good, bar), "two hits twenty milliseconds late, to the left");
    assert_eq!((timing_pixel(&canvas, 31, 0), timing_pixel(&canvas, 31, 9)), (mean, mean), "the mean, a millisecond early");
    assert_eq!(timing_pixel(&canvas, 22, 5), deviation, "a colour that is not one is the reference's red");
    assert_eq!((timing_pixel(&canvas, 27, 5), timing_pixel(&canvas, 18, 5), timing_pixel(&canvas, 5, 5)), (perfect, great, good));
    assert_eq!(
        (timing_pixel(&canvas, 20, 0), timing_pixel(&canvas, 20, 1), timing_pixel(&canvas, 20, 2)),
        (Color::rgb(0, 103, 0), Color::rgb(0, 103, 0), great),
        "a tick on the top two rows"
    );

    (bins[150], bins[160]) = (5, 10);
    let moved = TimingHistogram::new(&bins).with_judge_area(windows);
    draw(&screen, &mut text, &mut canvas, FrameData { series: FrameSeries { timing: Some(moved), ..FrameSeries::default() }, ..FrameData::default() });
    assert_eq!(
        (timing_pixel(&canvas, 40, 0), timing_pixel(&canvas, 30, 4), timing_pixel(&canvas, 30, 5)),
        (bar, perfect, bar),
        "other numbers are painted at once"
    );
}

/// A colour a document mistyped costs that one colour and a warning, not the graph.
#[test]
fn a_colour_that_is_not_one_is_reported_and_replaced() {
    let scratch = Scratch::new("bad-colour");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    assert!(
        screen.warnings().iter().any(|warning| warning.contains("timing-dist") && warning.contains("devColor")),
        "the mistyped colour is named: {:?}",
        screen.warnings()
    );
    assert_eq!(screen.count_of(SkinObjectKind::TimingDistribution), 1, "and the graph still resolved");
}

/// A wheel is placed by the destinations nested under it and drawn in their colours. The colour of
/// its own destination is never read, as the reference never reads it (`BarRenderer.render`), so a
/// document that wrote that destination transparent still gets its rows.
#[test]
fn a_wheel_draws_its_rows_whatever_colour_its_own_destination_names() {
    let scratch = Scratch::new("hidden");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let dst = format!(r#""x": 0, "y": 0, "w": 120, "h": {DOC_H}, "a": 0"#);
    let screen = wheel_screen_with(&scratch, &mut canvas, &mut text, Fixture { wheel_dst: Some(&dst), ..Fixture::default() });

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let list = SongBars { rows: &rows, sel: SELECTED, options_open: false };
    assert_eq!(draw(&screen, &mut text, &mut canvas, browsing(&list)), 2, "the wheel reaches the screen beside the button");
}

/// A slot is a box, not an anchor: a title longer than the slot the document drew for it is cut to
/// fit, the way the built-in row cuts one, rather than running on across whatever is beside it.
#[test]
fn a_title_longer_than_its_slot_is_cut_to_it() {
    let scratch = Scratch::new("long-title");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let mut rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    rows[SELECTED].title = "WIDE".repeat(20);
    let list = SongBars { rows: &rows, sel: SELECTED, options_open: false };
    draw(&screen, &mut text, &mut canvas, browsing(&list));

    let top = DOC_H - (slot_y(CENTER) + SLOT_H) as u32;
    let inked = |x: u32| {
        (top..top + SLOT_H as u32).any(|y| {
            let pixel = canvas.pixel_at(x, y);
            pixel.g > INK_LEVEL && pixel.b > INK_LEVEL
        })
    };
    let right = (TITLE_X + TITLE_W) as u32;
    assert!((TITLE_X as u32..right).any(inked), "the title is drawn in its slot at all");
    assert!(!(right..FOCUS_W as u32).any(inked), "and none of it lands past the slot, across the rest of the bar");
}

/// A slot's bar is cut from the image its id names, or from the first image of the set it names, and
/// an id that names neither is a document fault worth a line rather than a wheel of plain rectangles
/// nobody asked for.
#[test]
fn a_bar_can_be_cut_from_an_image_set_and_one_that_names_nothing_is_reported() {
    let scratch = Scratch::new("bar-source");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let plain = wheel_screen(&scratch, &mut canvas, &mut text);
    assert!(
        plain.warnings().iter().any(|warning| warning.contains("bar-on") && warning.contains("neither an image nor an image set")),
        "the focused bar names nothing the document declared: {:?}",
        plain.warnings()
    );

    let other = Scratch::new("bar-set");
    let fixture = Fixture { imageset: r#"{ "id": "bar-on", "images": ["bar"] }"#, ..Fixture::default() };
    let set = wheel_screen_with(&other, &mut canvas, &mut text, fixture);
    assert!(!set.warnings().iter().any(|warning| warning.contains("bar-on")), "declaring it as a set is enough to cut it from: {:?}", set.warnings());

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let list = SongBars { rows: &rows, sel: SELECTED, options_open: false };
    draw(&set, &mut text, &mut canvas, browsing(&list));
    let focused = canvas.pixel_at(BAR_W as u32 - 5, DOC_H - (slot_y(CENTER) + SLOT_H) as u32 + 2);
    assert_eq!(focused, Color { r: 240, g: 60, b: 60, a: 255 }, "and the bar it cuts lands in the colour the slot was tinted");
}

/// A wheel of no slots is no wheel. It has to resolve to nothing at all and say so, because a body
/// with nothing in it would be counted as a wheel and then draw nothing.
#[test]
fn a_wheel_with_no_slots_resolves_to_nothing_and_says_so() {
    let scratch = Scratch::new("empty-wheel");
    let body = format!(
        r#"{{
            "type": 5, "name": "empty wheel", "w": {DOC_W}, "h": {DOC_H},
            "songlist": {{ "id": "wheel", "center": {CENTER} }},
            "destination": [{{ "id": "wheel", "dst": [{{ "x": 0, "y": 0, "w": 120, "h": {DOC_H} }}] }}]
        }}"#
    );
    let path = scratch.root.join("wheel.json");
    std::fs::write(&path, body).expect("the document is writable");
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&path, options).expect("the generated document loads");

    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut SolidAssets);

    assert_eq!(screen.count_of(SkinObjectKind::SongList), 0, "a wheel with nothing in it resolved anyway: {:?}", screen.warnings());
    assert!(screen.warnings().iter().any(|warning| warning.contains("no slots")), "and the document is told why no wheel was drawn: {:?}", screen.warnings());
}

mod graph_pixels;
