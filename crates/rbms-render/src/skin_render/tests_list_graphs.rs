//! Unit tests for the browser's song wheel and for every graph a document draws itself, including
//! the colour parser they share.
//!
//! The wheel and the graphs are the two object kinds that read whole series rather than single
//! properties, so each test here loads a real document, hands it a frame's worth of rows or series,
//! and checks what reached the canvas -- which is the only way to tell "drew nothing because the
//! series was empty" from "drew nothing because the object never resolved".

use std::path::{Path, PathBuf};

use rbms_skin::dst::{DrawStateSource, LuaExprId, OffsetSource, SkinOffset};
use rbms_skin::loader::{HOTSPOT_ACTIONS, SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::property::generated::{NUMBER_BAD, NUMBER_GOOD, NUMBER_GREAT, NUMBER_MAXCOMBO, NUMBER_MISS, NUMBER_PERFECT, NUMBER_POOR, OPTION_PANEL1};
use rbms_skin::property::{SkinStateSource, UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::TimerState;

use super::color::{modulate, parse_hex_color};
use super::state::{
    FrameExtra, OPTION_ROW_FOCUSED_FIRST, OPTION_ROW_LABEL_FIRST, OPTION_ROW_VALUE_FIRST, OptionsRows, PlayObjectState, ResultSeriesState, SELECT_STAT_COUNT,
    SELECT_STAT_FIRST, SelectListState, SelectViewState, SkinHotAction, SkinHotspot,
};
use super::{SkinAssets, SkinFrame, SkinImage, SkinObjectKind, SkinScreen};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::playfield::{LaneShade, PlayfieldView};
use crate::result::ResultPalette;
use crate::select::{CoverState, DensityView, DetailView, RecordRowView, RecordsView, SelectDetail, SelectRow, SelectView};
use crate::skin::Skin;
use crate::theme::OPTIONS_ROW_COUNT;
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

    fn expression(&mut self, _source: &str) -> Option<LuaExprId> {
        None
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
    fn boolean(&self, id: i32) -> bool {
        if id < 0 { !UNMAPPED_BOOLEAN } else { UNMAPPED_BOOLEAN }
    }
}

impl SkinStateSource for Nothing {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, _id: i32) -> f32 {
        UNMAPPED_FLOAT
    }

    fn string(&self, _id: i32) -> &str {
        UNMAPPED_STRING
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
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

/// A browser document carrying a fifteen-slot wheel, a clickable button and one of every graph, so
/// one load exercises every object this file is about.
fn write_document(scratch: &Scratch, fixture: Fixture<'_>) -> PathBuf {
    std::fs::write(scratch.root.join("sheet.tex"), "solid").expect("the source file is writable");
    let clickable: Vec<String> = (0..SLOTS).map(|index| index.to_string()).collect();
    let mut hotspots: Vec<String> = HOTSPOT_ACTIONS.iter().map(|action| format!(r#"{{"id":"button","action":"{action}"}}"#)).collect();
    hotspots.push(r#"{"id":"button","action":"teleport"}"#.to_owned());
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
            "densitygraph": [{{ "id": "density", "barColor": "00FFFF", "peakColor": "FFD300" }}],
            "timingvisualizer": [{{ "id": "ruler" }}],
            "hiterrorvisualizer": [{{ "id": "errors" }}],
            "hotspot": [{hotspots}],
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
                {{ "id": "density", "dst": [{{ "x": 130, "y": 60, "w": 60, "h": 30 }}] }},
                {{ "id": "ruler", "dst": [{{ "x": 130, "y": 20, "w": 60, "h": 30 }}] }},
                {{ "id": "errors", "dst": [{{ "x": 60, "y": 20, "w": 60, "h": 30 }}] }}
            ]
        }}"#,
        clickable = clickable.join(","),
        hotspots = hotspots.join(","),
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

/// A focused chart whose per-second density is `bins`.
fn detail_with_density(bins: Vec<u32>) -> SelectDetail {
    SelectDetail::Song(Box::new(DetailView {
        accent: Color::WHITE,
        title: "TITLE".to_owned(),
        subtitle: String::new(),
        artist: String::new(),
        genre_maker: String::new(),
        mode_short: "7K",
        mode_color: Color::BLUE,
        level: "12".to_owned(),
        difficulty_color: Color::RED,
        difficulty_name: "ANOTHER",
        cover: CoverState::None,
        stats: Vec::new(),
        density: Some(DensityView { bins, peak: 4.0, avg: 2.0, end: 0.0 }),
        records: RecordsView { plays: 0, clears: 0, best: None, rank_bar: None, recent: Vec::new() },
    }))
}

/// One frame over `extra`, with nothing running and no pointer.
fn frame<'a>(timers: &'a TimerState, state: &'a Nothing, extra: FrameExtra<'a>) -> SkinFrame<'a> {
    SkinFrame { now_ms: 0, timers, state, lua: None, mouse: None, background: None, extra }
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

#[test]
fn no_two_private_id_bands_overlap() {
    let rows = OPTIONS_ROW_COUNT as i32;
    let mut ids: Vec<i32> = Vec::new();
    for first in [OPTION_ROW_LABEL_FIRST, OPTION_ROW_VALUE_FIRST, OPTION_ROW_FOCUSED_FIRST] {
        ids.extend(first..first + rows);
    }
    ids.extend(SELECT_STAT_FIRST..SELECT_STAT_FIRST + SELECT_STAT_COUNT as i32);

    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "two of the private id bands share a number");
}

/// The labels a fixture panel carries, one per row, so a row that answered its neighbour's label
/// fails rather than passing on a shared string.
const PANEL_LABELS: [&str; OPTIONS_ROW_COUNT] =
    ["RANDOM", "GAUGE", "HI-SPEED", "FIX HI-SPEED", "LANE COVER", "LIFT", "HIDDEN", "SCRATCH SIDE", "SCRATCH AUTO", "AUTOPLAY", "TARGET"];

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

/// The option panel's rows reach a document through the browser's own state source, because that is
/// the only thing a document can address: it asks for a private string id and has to be answered
/// with what the configuration holds rather than with an empty string.
#[test]
fn the_option_panels_rows_answer_the_private_ids_a_document_asks_them_by() {
    let view = empty_browser();
    let values: [String; OPTIONS_ROW_COUNT] = std::array::from_fn(|row| format!("VALUE {row}"));
    let rows = OptionsRows { labels: PANEL_LABELS, values, focused: 3, open: true };
    let state = SelectViewState::new(&view, 0, None, Some(&rows));

    for (row, label) in PANEL_LABELS.iter().enumerate() {
        let at = row as i32;
        assert_eq!(state.string(OPTION_ROW_LABEL_FIRST + at), *label, "row {row} answered the wrong label");
        assert_eq!(state.string(OPTION_ROW_VALUE_FIRST + at), format!("VALUE {row}"), "row {row} answered the wrong value");
        assert_eq!(state.boolean(OPTION_ROW_FOCUSED_FIRST + at), row == 3, "row {row} disagreed about being focused");
    }
    assert!(state.boolean(OPTION_PANEL1), "an open panel does not report itself open");
}

/// The judgement counts and the longest combo of the focused chart's best run answer the same whole
/// number ids the score screen answers, so a browser document can show a chart's best run broken down
/// rather than only its total.
///
/// A chart nothing has been played on answers every one of them zero: that is a real reading -- no
/// run, no notes judged -- and not an absent one, so a document draws six zeroes rather than the
/// counts of whichever chart was focused last.
#[test]
fn the_focused_charts_best_run_answers_the_judgement_count_ids() {
    /// The ids a document reads one run's judgement counts through, best judgement first.
    const JUDGE_COUNT_IDS: [i32; 6] = [NUMBER_PERFECT, NUMBER_GREAT, NUMBER_GOOD, NUMBER_BAD, NUMBER_POOR, NUMBER_MISS];
    /// The best run the fixture chart carries, in that same order.
    const BEST_COUNTS: [u32; 6] = [812, 134, 27, 4, 9, 3];
    /// The longest unbroken run of that same play.
    const BEST_MAX_COMBO: u32 = 604;

    let best = RecordRowView {
        when: "2026-09-18 12:00".to_owned(),
        lamp: Color::GREEN,
        lamp_label: "HARD",
        ex: 1758,
        max_ex: 1978,
        bp: BEST_COUNTS[3] + BEST_COUNTS[4] + BEST_COUNTS[5],
        counts: BEST_COUNTS,
        max_combo: BEST_MAX_COMBO,
        trend: None,
    };
    let mut played = empty_browser();
    played.detail = detail_with_record(Some(best));
    let state = SelectViewState::new(&played, 0, None, None);
    for (judgement, id) in JUDGE_COUNT_IDS.into_iter().enumerate() {
        assert_eq!(state.integer(id), BEST_COUNTS[judgement] as i32, "judgement {judgement} answered the wrong count");
    }
    assert_eq!(state.integer(NUMBER_MAXCOMBO), BEST_MAX_COMBO as i32, "the longest combo of the best run");

    let mut unplayed = empty_browser();
    unplayed.detail = detail_with_record(None);
    let state = SelectViewState::new(&unplayed, 0, None, None);
    for (judgement, id) in JUDGE_COUNT_IDS.into_iter().enumerate() {
        assert_eq!(state.integer(id), 0, "judgement {judgement} of a chart with no record");
    }
    assert_eq!(state.integer(NUMBER_MAXCOMBO), 0, "and the longest combo of a chart with no record");

    let nothing_focused = empty_browser();
    let state = SelectViewState::new(&nothing_focused, 0, None, None);
    assert_eq!(state.integer(JUDGE_COUNT_IDS[0]), 0, "a browser with no chart focused counts nothing either");
    assert_eq!(state.integer(NUMBER_MAXCOMBO), 0);
}

/// A focused chart whose only local record is `best`.
fn detail_with_record(best: Option<RecordRowView>) -> SelectDetail {
    let plays = usize::from(best.is_some());
    let mut detail = match detail_with_density(Vec::new()) {
        SelectDetail::Song(detail) => detail,
        _ => unreachable!("the density fixture focuses a chart"),
    };
    detail.records = RecordsView { plays, clears: plays, best, rank_bar: None, recent: Vec::new() };
    SelectDetail::Song(detail)
}

/// A frame the browser carries no panel on answers every one of those ids as an unmapped one, so a
/// document that draws a panel over a closed browser draws an empty one rather than the last rows it
/// happened to see.
#[test]
fn a_browser_with_no_option_panel_answers_its_rows_as_unmapped() {
    let view = empty_browser();
    let state = SelectViewState::new(&view, 0, None, None);

    assert_eq!(state.string(OPTION_ROW_LABEL_FIRST), UNMAPPED_STRING);
    assert_eq!(state.string(OPTION_ROW_VALUE_FIRST), UNMAPPED_STRING);
    assert_eq!(state.boolean(OPTION_ROW_FOCUSED_FIRST), UNMAPPED_BOOLEAN);
    assert_eq!(state.boolean(OPTION_PANEL1), UNMAPPED_BOOLEAN);
}

/// A hit rectangle is hit-tested against the pointer on the canvas, not against the document, so the
/// same flip and scale the frame was drawn through has to be applied to it. The fixture is authored
/// at the canvas size, so a rectangle low in the document lands high on a screen of that size and
/// twice as far down one of twice the height.
#[test]
fn a_hotspot_is_placed_on_the_canvas_the_frame_was_drawn_on() {
    let scratch = Scratch::new("placed");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let timers = TimerState::new();
    let state = Nothing;
    let frame = frame(&timers, &state, FrameExtra::None);
    let button = |spots: Vec<SkinHotspot>| spots.into_iter().find(|spot| spot.action == SkinHotAction::Search).map(|spot| spot.rect);

    assert_eq!(button(screen.hotspots(&frame)), Some(Rect::new(10.0, 20.0, 40.0, 12.0)), "the document's own answer is in its own coordinates");
    let same_size = button(screen.hotspots_on_screen(&frame, (DOC_W, DOC_H)));
    assert_eq!(same_size, Some(Rect::new(10.0, (DOC_H as f32) - 32.0, 40.0, 12.0)), "the vertical axis is flipped onto the canvas");
    let twice_as_tall = button(screen.hotspots_on_screen(&frame, (DOC_W * 2, DOC_H * 2)));
    assert_eq!(twice_as_tall, Some(Rect::new(20.0, ((DOC_H as f32) - 32.0) * 2.0, 80.0, 24.0)), "and both axes scale with the canvas");
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
        SkinObjectKind::Density,
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
    let detail = SelectDetail::Empty;
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
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

/// The hit rectangles a click is answered from: one per clickable slot that a row actually landed
/// on, carrying that row rather than the slot, plus whatever the document's hotspot table names.
#[test]
fn the_wheel_reports_a_hit_rectangle_for_every_row_it_drew() {
    let scratch = Scratch::new("hot");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let detail = SelectDetail::Empty;
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
    let timers = TimerState::new();
    let state = Nothing;
    let spots = screen.hotspots(&frame(&timers, &state, FrameExtra::Select(&list)));

    let rows_reported: Vec<SkinHotAction> = spots.iter().map(|spot| spot.action).filter(|action| matches!(action, SkinHotAction::Row(_))).collect();
    let expected: Vec<SkinHotAction> = (0..ROWS).map(SkinHotAction::Row).collect();
    assert_eq!(rows_reported, expected, "every row on screen answers a click, numbered as the browser numbers its own rows");

    let rect_of = |action: SkinHotAction| spots.iter().find(|spot| spot.action == action).map(|spot| spot.rect);
    let focused_slot = Rect::new(0.0, slot_y(CENTER) as f32, FOCUS_W as f32, SLOT_H as f32);
    assert_eq!(rect_of(SkinHotAction::Row(SELECTED)), Some(focused_slot), "the focused row's rectangle is the focused bar it was drawn as");
    let first_slot = Rect::new(0.0, slot_y(CENTER - SELECTED) as f32, BAR_W as f32, SLOT_H as f32);
    assert_eq!(rect_of(SkinHotAction::Row(0)), Some(first_slot), "and an unfocused row's is its own slot, in document coordinates");
    assert_eq!(rect_of(SkinHotAction::Search), Some(Rect::new(10.0, 20.0, 40.0, 12.0)), "the hotspot table answers for the button it named");
}

/// A hotspot is still answered on a frame that carries no rows, because the buttons a document draws
/// do not depend on the wheel having anything in it.
///
/// Every action the loader keeps has to be one the wheel turns into a rectangle, or a document would
/// declare a hotspot that loads clean and then does nothing; the one the loader already dropped
/// never reaches here at all.
#[test]
fn every_action_the_loader_keeps_becomes_a_rectangle_the_browser_can_act_on() {
    let scratch = Scratch::new("actions");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let timers = TimerState::new();
    let state = Nothing;
    let spots: Vec<SkinHotspot> = screen.hotspots(&frame(&timers, &state, FrameExtra::None));
    let actions: Vec<SkinHotAction> = spots.iter().map(|spot| spot.action).collect();
    let expected = [
        SkinHotAction::Folders,
        SkinHotAction::ModalClose,
        SkinHotAction::ModalReplay,
        SkinHotAction::Records,
        SkinHotAction::Search,
        SkinHotAction::Settings,
        SkinHotAction::Sort,
        SkinHotAction::Tables,
    ];
    assert_eq!(expected.len(), HOTSPOT_ACTIONS.len(), "the loader accepts an action this test does not name");
    assert_eq!(actions, expected, "the wheel reports no rows on this frame, so the hotspot table is all that is left");
    for spot in &spots {
        assert_eq!(spot.rect, Rect::new(10.0, 20.0, 40.0, 12.0), "each one is the rectangle of the object it named");
    }
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
    let detail = detail_with_density(vec![0, 4, 2]);
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::Select(&list)), 3, "the button, the wheel and the density of the focused chart");

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

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let detail = detail_with_density(Vec::new());
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::Select(&list)), 2, "a chart with no measured density draws no histogram");
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

/// The density histogram is scaled to the busiest second of the chart, which is also where its peak
/// marker lands.
#[test]
fn the_density_histogram_is_scaled_to_the_busiest_second() {
    let scratch = Scratch::new("density");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let screen = wheel_screen(&scratch, &mut canvas, &mut text);

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let detail = detail_with_density(vec![0, 4, 2]);
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
    draw(&screen, &mut text, &mut canvas, FrameExtra::Select(&list));

    let top = DOC_H - 90;
    assert_eq!(canvas.pixel_at(155, top + 15), Color::rgb(0, 255, 255), "the busiest second fills its whole column");
    assert_eq!(canvas.pixel_at(135, top + 15), Color::BLACK, "a second with no notes draws no bar");
    assert_eq!(canvas.pixel_at(175, top + 5), Color::BLACK, "a quieter second only fills its share of the column");
    assert_eq!(canvas.pixel_at(175, top + 20), Color::rgb(0, 255, 255));
    assert_eq!(canvas.pixel_at(135, top), Color::rgb(0xFF, 0xD3, 0x00), "and the peak is marked across the top of the panel");
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

/// A row rectangle is only worth answering a click on while its row is on the screen, and every one
/// of the wheel's slots hangs off the wheel's own destination: a document that faded that out drew
/// no rows this frame, so a click has to fall through to whatever is behind them.
#[test]
fn a_wheel_the_document_faded_out_answers_no_click_on_its_rows() {
    let scratch = Scratch::new("hidden");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let dst = format!(r#""x": 0, "y": 0, "w": 120, "h": {DOC_H}, "a": 0"#);
    let screen = wheel_screen_with(&scratch, &mut canvas, &mut text, Fixture { wheel_dst: Some(&dst), ..Fixture::default() });

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let detail = SelectDetail::Empty;
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
    assert_eq!(draw(&screen, &mut text, &mut canvas, FrameExtra::Select(&list)), 1, "the faded wheel reaches the screen nowhere, leaving only the button");

    let timers = TimerState::new();
    let state = Nothing;
    let spots = screen.hotspots(&frame(&timers, &state, FrameExtra::Select(&list)));
    assert!(!spots.iter().any(|spot| matches!(spot.action, SkinHotAction::Row(_))), "no row of a wheel nobody can see is clicked: {spots:?}");
    assert!(spots.iter().any(|spot| spot.action == SkinHotAction::Search), "the buttons beside it are their own objects and still answer");
}

/// A wheel with a window of its own shows only the slots inside it, so only those are clicked: a
/// slot scrolled past the edge of the window is drawn nowhere and hit nowhere.
#[test]
fn a_row_scrolled_out_of_the_wheels_window_is_clicked_nowhere() {
    let scratch = Scratch::new("clipped");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let dst = format!(r#""x": 0, "y": 0, "w": 120, "h": {DOC_H}, "clip_x": 0, "clip_y": 100, "clip_w": 120, "clip_h": 80"#);
    let screen = wheel_screen_with(&scratch, &mut canvas, &mut text, Fixture { wheel_dst: Some(&dst), ..Fixture::default() });

    let rows: Vec<SelectRow> = (0..ROWS).map(row).collect();
    let detail = SelectDetail::Empty;
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
    let timers = TimerState::new();
    let state = Nothing;
    let spots = screen.hotspots(&frame(&timers, &state, FrameExtra::Select(&list)));

    let reported: Vec<SkinHotAction> = spots.iter().map(|spot| spot.action).filter(|action| matches!(action, SkinHotAction::Row(_))).collect();
    let expected: Vec<SkinHotAction> = (0..3).map(SkinHotAction::Row).collect();
    assert_eq!(reported, expected, "the two rows below the window are gone, and the three that reach into it are not");

    let focused = spots.iter().find(|spot| spot.action == SkinHotAction::Row(SELECTED)).map(|spot| spot.rect);
    let overlap = (slot_y(CENTER) + SLOT_H - 100) as f32;
    assert_eq!(focused, Some(Rect::new(0.0, 100.0, FOCUS_W as f32, overlap)), "and the row straddling the edge is clickable only where it was drawn");
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
    let detail = SelectDetail::Empty;
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
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
    let detail = SelectDetail::Empty;
    let list = SelectListState { rows: &rows, sel: SELECTED, detail: &detail, options: None };
    draw(&set, &mut text, &mut canvas, FrameExtra::Select(&list));
    let focused = canvas.pixel_at(BAR_W as u32 - 5, DOC_H - (slot_y(CENTER) + SLOT_H) as u32 + 2);
    assert_eq!(focused, Color { r: 240, g: 60, b: 60, a: 255 }, "and the bar it cuts lands in the colour the slot was tinted");
}

/// A wheel of no slots is no wheel. It has to resolve to nothing at all, because a body of any kind
/// is what a screen counts when it decides the document has taken the row list over: an empty one
/// would hide the built-in browser and then draw nothing in its place.
#[test]
fn a_wheel_with_no_slots_leaves_the_rows_to_the_built_in_browser() {
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
    assert!(
        screen.warnings().iter().any(|warning| warning.contains("no slots")),
        "and the document is told why its rows stayed with the browser: {:?}",
        screen.warnings()
    );
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
