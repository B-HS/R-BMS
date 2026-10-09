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
use rbms_skin::timer::{TIMER_OFF, TimerState};

use super::color::{modulate, parse_hex_color};
use super::state::{FrameExtra, PlayObjectState, ResultSeriesState, SelectListState, SelectViewState};
use super::{SkinAssets, SkinFrame, SkinImage, SkinObjectKind, SkinScreen};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::playfield::{LaneShade, PlayfieldView};
use crate::result::ResultPalette;
use crate::select::{SelectDetail, SelectRow, SelectView};
use crate::skin::Skin;
use crate::{BYTES_PER_PIXEL, Color, CpuCanvas, Rect, Renderer};

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
                "borderColor": "FF0000"
            }}],
            "judgegraph": [{judge_graph}],
            "bpmgraph": [{{ "id": "bpm-graph" }}],
            "timingdistributiongraph": [{{ "id": "timing-dist", "devColor": "not a colour" }}],
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

/// One frame over `extra`, with nothing running and no pointer.
fn frame<'a>(timers: &'a TimerState, state: &'a Nothing, extra: FrameExtra<'a>) -> SkinFrame<'a> {
    SkinFrame { now_us: 0, timers, state, lua: None, mouse: None, background: None, extra }
}

/// Draws `screen` over a cleared canvas and answers how many of its objects reached it.
fn draw(screen: &SkinScreen, text: &mut TextContext, canvas: &mut CpuCanvas, extra: FrameExtra<'_>) -> usize {
    let timers = TimerState::new();
    let state = Nothing;
    canvas.clear(Color::BLACK);
    let mut ctx = RenderCtx::new(crate::theme::theme(), text);
    screen.draw(&mut ctx, canvas, &frame(&timers, &state, extra))
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
    let list = SelectListState { rows: &rows, sel: SELECTED, options_open: false };
    draw(&screen, &mut text, &mut canvas, FrameExtra::Select(&list));

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

    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::None), 1, "only the button, which reads nothing, is left");
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
    let tempo = [(0.0, 150.0), (0.5, 200.0), (0.8, 120.0)];
    let series = ResultSeriesState { gauge_series: &gauge, timing_hist: &hist, judge_dist: &counts, bpm_points: &tempo };
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::Result(&series)), 5, "the button and the four score-screen graphs");

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let list = SelectListState { rows: &rows, sel: SELECTED, options_open: false };
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::Select(&list)), 2, "the button and the wheel, which is all a browser frame feeds");

    let field = Skin::default_for(rbms_model::Mode::BEAT_7K, DOC_W as f32, DOC_H as f32);
    let playfield = PlayfieldView { timelines: &[], microtime: 0, hispeed: 1.0, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let hits = [(-30_i64, 1_u8), (8, 0), (45, 2)];
    let play =
        PlayObjectState { field: &field, playfield: &playfield, shade: LaneShade::default(), gauge_kind: 0, bomb: &[], keys_down: &[], recent_hits: &hits };
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::Play(&play)), 3, "the button, the judge ruler and the hit errors");
}

/// A measurement of nothing is not a measurement of zero: an empty series leaves its panel to the
/// built-in screen rather than drawing an empty frame over it.
#[test]
fn an_empty_series_draws_no_graph_at_all() {
    let scratch = Scratch::new("empty");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let series = ResultSeriesState { gauge_series: &[], timing_hist: &[], judge_dist: &[0; 6], bpm_points: &[] };
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::Result(&series)), 1, "only the button is left when the run measured nothing");
}

/// The gauge history is drawn in the colours its own record named: its ground, the line the samples
/// trace across it, and the border over both.
#[test]
fn the_gauge_history_is_drawn_in_the_colours_the_document_named() {
    let scratch = Scratch::new("gauge");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let gauge = [50.0_f32; 8];
    let series = ResultSeriesState { gauge_series: &gauge, timing_hist: &[], judge_dist: &[0; 6], bpm_points: &[] };
    draw(&screen, &mut text, &mut canvas, FrameExtra::Result(&series));

    let panel = Rect::new(200.0, (DOC_H - 140) as f32, 60.0, 40.0);
    assert_eq!(canvas.pixel_at(230, panel.y as u32 + 5), Color::rgb(0x20, 0x30, 0x40), "the panel's ground");
    let half_way = panel.y + (panel.h - 2.0) * 0.5;
    assert_eq!(canvas.pixel_at(230, half_way as u32), Color::rgb(0, 255, 0), "a run held at half gauge traces its line half way up");
    assert_eq!(canvas.pixel_at(230, panel.y as u32), Color::rgb(255, 0, 0), "and the border is drawn over both");
}

/// A colour a document mistyped costs that one colour and a warning, not the graph.
#[test]
fn a_colour_that_is_not_one_is_reported_and_replaced() {
    let scratch = Scratch::new("bad-colour");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    assert!(
        screen.warnings().iter().any(|warning| warning.contains("timing-dist") && warning.contains("deviation")),
        "the mistyped colour is named: {:?}",
        screen.warnings()
    );
    assert_eq!(screen.count_of(SkinObjectKind::TimingDistribution), 1, "and the graph still resolved");
}

/// Every one of the wheel's slots hangs off the wheel's own destination, so a document that faded
/// that out drew no rows this frame.
#[test]
fn a_wheel_the_document_faded_out_draws_none_of_its_rows() {
    let scratch = Scratch::new("hidden");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let dst = format!(r#""x": 0, "y": 0, "w": 120, "h": {DOC_H}, "a": 0"#);
    let screen = wheel_screen_with(&scratch, &mut canvas, &mut text, Fixture { wheel_dst: Some(&dst), ..Fixture::default() });

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let list = SelectListState { rows: &rows, sel: SELECTED, options_open: false };
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::Select(&list)), 1, "the faded wheel reaches the screen nowhere, leaving only the button");
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
    let list = SelectListState { rows: &rows, sel: SELECTED, options_open: false };
    draw(&screen, &mut text, &mut canvas, FrameExtra::Select(&list));

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
    let list = SelectListState { rows: &rows, sel: SELECTED, options_open: false };
    draw(&set, &mut text, &mut canvas, FrameExtra::Select(&list));
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

/// The judgement graph the document asked for, rather than the one shape for every record: a spread
/// across six bars, or the one column those six share.
#[test]
fn a_judgement_graph_takes_the_shape_its_record_asked_for() {
    let counts = [10, 4, 2, 1, 1, 0];
    let series = ResultSeriesState { gauge_series: &[], timing_hist: &[], judge_dist: &counts, bpm_points: &[] };
    let palette = ResultPalette::default().judge_colors;

    let bars = Scratch::new("judge-bars");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&bars, &mut canvas, &mut text);
    draw(&screen, &mut text, &mut canvas, FrameExtra::Result(&series));
    assert_eq!(canvas.pixel_at(210, 110), palette[1], "by default the second judgement takes a bar of its own, scaled to the commonest");
    assert_eq!(canvas.pixel_at(210, 100), Color::BLACK, "and nothing of it reaches above its own count");

    let stacked = Scratch::new("judge-stack");
    let fixture = Fixture { judge_graph: Some(r#"{ "id": "judge-graph", "type": 1 }"#), ..Fixture::default() };
    let screen = wheel_screen_with(&stacked, &mut canvas, &mut text, fixture);
    draw(&screen, &mut text, &mut canvas, FrameExtra::Result(&series));
    assert_eq!(canvas.pixel_at(210, 110), palette[0], "asked to stack them, the best judgement takes the foot of the one column");
    assert_eq!(canvas.pixel_at(210, 100), palette[1], "the next one sits on top of it");
    assert_eq!(canvas.pixel_at(210, 95), palette[2], "and so on up, each taking its share of the run");
}

/// A judgement graph counted in something rbms never recorded is drawn as the one it does record,
/// and says so, rather than silently handing the document a different graph.
#[test]
fn a_judgement_graph_rbms_records_nothing_for_falls_back_and_says_so() {
    let scratch = Scratch::new("judge-unknown");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let fixture = Fixture { judge_graph: Some(r#"{ "id": "judge-graph", "type": 2 }"#), ..Fixture::default() };
    let screen = wheel_screen_with(&scratch, &mut canvas, &mut text, fixture);

    assert!(
        screen.warnings().iter().any(|warning| warning.contains("judge-graph") && warning.contains("type 2")),
        "the fallback is named: {:?}",
        screen.warnings()
    );

    let counts = [10, 4, 2, 1, 1, 0];
    let series = ResultSeriesState { gauge_series: &[], timing_hist: &[], judge_dist: &counts, bpm_points: &[] };
    draw(&screen, &mut text, &mut canvas, FrameExtra::Result(&series));
    let palette = ResultPalette::default().judge_colors;
    assert_eq!(canvas.pixel_at(210, 110), palette[1], "and what it draws is the shape a record with no type at all gets");
    assert_eq!(canvas.pixel_at(210, 100), Color::BLACK);
}
