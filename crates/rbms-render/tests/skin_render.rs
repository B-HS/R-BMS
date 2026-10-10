//! Drawing a real JSON skin document, end to end.
//!
//! The fixture next door is a small select-screen document that uses every object kind this build
//! draws, so one load exercises the whole path: image sources become textures, destinations become
//! tracks, and a frame turns those into quads. Its images are text files naming a pattern, which the
//! decoder here generates -- no binary asset is committed for a skin that only needs to be
//! recognisable.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use rbms_chart::to_model;
use rbms_parser::parse;
use rbms_render::playfield::LaneShade;
use rbms_render::skin_render::state::{DecideChart, DecideViewState, SelectViewState};
use rbms_render::skin_render::{SkinDraw, render_decide_screen, render_keyconfig_screen, render_result_screen, render_select_screen};
use rbms_render::{
    BgaFrame, Color, CpuCanvas, FrameData, GaugeFrame, GoldenImage, GoldenOptions, NoExpressions, NoteField, PlayfieldView, PngCodec, QuadParams, Rect,
    RenderCtx, Renderer, SelectDetail, SelectRow, SelectView, Skin, SkinAssets, SkinFrame, SkinImage, SkinObjectKind, SkinScreen, SongBars, TextContext,
    TextureId, assert_golden_png, render_select_ctx,
};
use rbms_skin::dst::{DrawCondition, DrawStateSource, LuaDrawEval, LuaFnId, TimerRef};
use rbms_skin::loader::lua_skin::{LuaSkinOptions, load_lua_skin};
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::lua::{FrameBudget, LuaBudget};
use rbms_skin::model::PropertyRef;
use rbms_skin::property::generated::{OPTION_DIFFICULTY0, OPTION_DIFFICULTY5};
use rbms_skin::property::{HostCall, MapHost, SkinHost, UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerId, TimerState};

/// Width the fixture frames are drawn at, twice the document's own width so the viewport is doing
/// real work rather than mapping one to one.
const CANVAS_W: u32 = 512;

/// Height the fixture frames are drawn at.
const CANVAS_H: u32 = 288;

/// The timer the fixture animates against.
const FIXTURE_TIMER: TimerId = TimerId(1);

/// How long one pass over the fixture's animation takes.
const FIXTURE_CYCLE_MS: i64 = 800;

/// The property id the fixture's number object reads.
const FIXTURE_NUMBER_ID: i32 = 7;

/// The property id the fixture's slider and graph read.
const FIXTURE_RATE_ID: i32 = 9;

/// The property id the fixture's text object reads.
const FIXTURE_TEXT_ID: i32 = 11;

/// The option id the fixture's gated object waits on.
const FIXTURE_GATE_ID: i32 = 22;

/// PNG read and write for the golden harness, over the `png` dev-dependency.
struct Png;

impl PngCodec for Png {
    fn encode(&self, image: &GoldenImage) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&image.rgba).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn decode(&self, bytes: &[u8]) -> Result<GoldenImage, String> {
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().map_err(|e| e.to_string())?;
        let mut buffer = vec![0; reader.output_buffer_size().ok_or("golden is too large to decode")?];
        let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
        if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
            return Err(format!("golden is {:?}/{:?}, expected 8-bit RGBA", info.color_type, info.bit_depth));
        }
        buffer.truncate(info.buffer_size());
        Ok(GoldenImage::new(info.width, info.height, buffer))
    }
}

/// Generates the fixture's images from the pattern each file names.
///
/// The file is one line, `pattern width height [detail]`, which keeps the committed fixture readable
/// and its pixels reproducible on every host.
struct PatternAssets;

impl SkinAssets for PatternAssets {
    fn image(&mut self, path: &Path) -> Option<SkinImage> {
        let text = std::fs::read_to_string(path).ok()?;
        let mut words = text.split_whitespace();
        let pattern = words.next()?;
        let width: u32 = words.next()?.parse().ok()?;
        let height: u32 = words.next()?.parse().ok()?;
        let detail: u32 = words.next().and_then(|word| word.parse().ok()).unwrap_or(1);
        let extra: u32 = words.next().and_then(|word| word.parse().ok()).unwrap_or(1);
        let rgba = match pattern {
            "checker" => checker(width, height, detail.max(1)),
            "gradient" => gradient(width, height),
            "cells" => cells(width, height, detail.max(1), extra.max(1)),
            _ => return None,
        };
        SkinImage::new(width, height, rgba)
    }
}

/// A checkerboard of opaque white and half-transparent grey squares.
fn checker(width: u32, height: u32, cell: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let light = ((x / cell) + (y / cell)).is_multiple_of(2);
            rgba.extend_from_slice(if light { &[255, 255, 255, 255] } else { &[80, 80, 96, 128] });
        }
    }
    rgba
}

/// A vertical ramp from opaque green at the bottom to transparent at the top.
fn gradient(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let level = (255 * (height - 1 - y) / height.max(1)) as u8;
        for _ in 0..width {
            rgba.extend_from_slice(&[level, 220, 120, 255]);
        }
    }
    rgba
}

/// A grid whose cells each take a colour of their own, so a wrongly indexed cell is visible.
fn cells(width: u32, height: u32, columns: u32, rows: u32) -> Vec<u8> {
    let (cell_w, cell_h) = ((width / columns).max(1), (height / rows).max(1));
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let index = (y / cell_h) * columns + (x / cell_w);
            let shade = (index * 24).min(u32::from(u8::MAX)) as u8;
            rgba.extend_from_slice(&[255 - shade, shade, 200, 255]);
        }
    }
    rgba
}

/// A state source answering exactly the ids the fixture asks for, so the document's frames depend on
/// nothing but this file.
struct FixtureState {
    number: i32,
    rate: f32,
    text: String,
    gate: bool,
}

impl rbms_skin::dst::OffsetSource for FixtureState {
    fn offset(&self, _id: i32) -> Option<rbms_skin::dst::SkinOffset> {
        None
    }
}

impl rbms_skin::dst::DrawStateSource for FixtureState {
    fn boolean(&self, id: i32) -> Option<bool> {
        let answer = if id.abs() == FIXTURE_GATE_ID { self.gate } else { UNMAPPED_BOOLEAN };
        Some(if id < 0 { !answer } else { answer })
    }
}

impl SkinHost for FixtureState {
    fn integer(&self, id: i32) -> i32 {
        if id == FIXTURE_NUMBER_ID { self.number } else { UNMAPPED_INTEGER }
    }

    fn rate(&self, id: i32) -> Option<f32> {
        (id == FIXTURE_RATE_ID).then_some(self.rate)
    }

    fn float(&self, id: i32) -> f32 {
        self.rate(id).unwrap_or(UNMAPPED_FLOAT)
    }

    fn text(&self, id: i32) -> Cow<'_, str> {
        Cow::Borrowed(if id == FIXTURE_TEXT_ID { &self.text } else { UNMAPPED_STRING })
    }

    fn timer_us(&self, _id: i32) -> i64 {
        TIMER_OFF
    }

    fn now_us(&self) -> i64 {
        0
    }
}

impl Default for FixtureState {
    fn default() -> Self {
        FixtureState { number: 42, rate: 0.5, text: "RBMS".into(), gate: false }
    }
}

/// What a frame carries when all it has beyond its properties is the picture behind the field.
fn behind(backdrop: TextureId) -> FrameData<'static> {
    FrameData { bga: BgaFrame::of(Some(backdrop)), ..FrameData::default() }
}

/// Where the fixture document lives.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("skin")
}

/// Loads and compiles the fixture against a fresh canvas.
fn build(canvas: &mut CpuCanvas, text: &mut TextContext) -> (SkinScreen, Vec<String>) {
    let root = fixture_root();
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&root.join("skin.json"), options).expect("the fixture document loads");
    let load_warnings = skin.warnings.clone();
    let mut assets = PatternAssets;
    let screen = SkinScreen::build(canvas, text, &skin, &mut assets);
    (screen, load_warnings)
}

/// Draws one frame of the fixture at `now_ms`, with the fixture timer switched on at zero.
fn frame_at(now_ms: i64, state: &FixtureState) -> CpuCanvas {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let (screen, _) = build(&mut canvas, &mut text);

    let backdrop = canvas.register_texture("test.backdrop", &checker(8, 8, 2), 8, 8);
    let mut timers = TimerState::new();
    timers.set_on(FIXTURE_TIMER, 0);
    canvas.clear(Color::BLACK);

    let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
    let frame = SkinFrame { now_us: now_ms * MICROS_PER_MILLI, timers: &timers, state, lua: None, mouse: None, data: behind(backdrop) };
    screen.draw(&mut ctx, &mut canvas, &frame);
    canvas
}

#[test]
fn the_fixture_document_loads_with_every_object_kind_resolved() {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let (screen, load_warnings) = build(&mut canvas, &mut text);

    assert_eq!(load_warnings, Vec::<String>::new(), "the fixture loads without the loader dropping anything");
    assert_eq!(screen.warnings(), Vec::<String>::new(), "every object in the fixture is one this build draws");
    assert_eq!(screen.authored_size(), (256.0, 144.0), "the document's own space is what its coordinates are in");
    assert_eq!(screen.object_count(), 8, "one draw-list entry per destination");
    assert_eq!(screen.count_of(SkinObjectKind::Image), 3);
    assert_eq!(screen.count_of(SkinObjectKind::Number), 1);
    assert_eq!(screen.count_of(SkinObjectKind::Text), 1);
    assert_eq!(screen.count_of(SkinObjectKind::Slider), 1);
    assert_eq!(screen.count_of(SkinObjectKind::Graph), 1);
    assert_eq!(screen.count_of(SkinObjectKind::Background), 1);
    assert_eq!(screen.texture_count(), 3, "one texture per image source the document names");
}

#[test]
fn a_frame_is_identical_every_time_it_is_drawn() {
    let state = FixtureState::default();
    let first = frame_at(0, &state);
    let second = frame_at(0, &state);
    assert_eq!(first.pixels(), second.pixels(), "the same document, state and clock draw the same pixels");
}

#[test]
fn the_fixture_frame_matches_its_golden() {
    let canvas = frame_at(0, &FixtureState::default());
    if let Err(message) = assert_golden_png(&canvas, env!("CARGO_MANIFEST_DIR"), "skin_fixture", GoldenOptions::default(), &Png) {
        panic!("{message}");
    }
}

#[test]
fn a_timer_drives_both_the_animation_and_the_cell_it_shows() {
    let state = FixtureState::default();
    let start = frame_at(0, &state);
    let quarter = frame_at(FIXTURE_CYCLE_MS / 4, &state);
    let wrapped = frame_at(FIXTURE_CYCLE_MS * 5, &state);

    assert_ne!(start.pixels(), quarter.pixels(), "a quarter of the way through, the object has moved and its cell has changed");
    assert_eq!(start.pixels(), wrapped.pixels(), "the destination loops every 1000 ms and the cells every 800 ms, so their common period brings both back");
}

#[test]
fn an_object_whose_draw_condition_is_false_is_not_drawn() {
    let hidden = frame_at(0, &FixtureState { gate: false, ..FixtureState::default() });
    let shown = frame_at(0, &FixtureState { gate: true, ..FixtureState::default() });
    assert_ne!(hidden.pixels(), shown.pixels(), "turning the gated object's option on puts it on screen");
}

#[test]
fn a_number_object_follows_the_property_it_reads() {
    let low = frame_at(0, &FixtureState { number: 42, ..FixtureState::default() });
    let high = frame_at(0, &FixtureState { number: 907, ..FixtureState::default() });
    assert_ne!(low.pixels(), high.pixels(), "a different value picks different digit cells");
}

#[test]
fn a_slider_and_a_graph_follow_the_rate_they_read() {
    let empty = frame_at(0, &FixtureState { rate: 0.0, ..FixtureState::default() });
    let full = frame_at(0, &FixtureState { rate: 1.0, ..FixtureState::default() });
    assert_ne!(empty.pixels(), full.pixels(), "the handle travels and the bar grows with the rate");
}

#[test]
fn releasing_a_screen_hands_every_texture_back() {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let (mut screen, _) = build(&mut canvas, &mut text);

    assert_eq!(canvas.live_texture_count(), 3, "the document's three sources are registered");
    screen.release(&mut canvas);
    assert_eq!(canvas.live_texture_count(), 0, "a released screen leaves nothing behind for a reload to leak");
    assert_eq!(screen.object_count(), 0, "a released screen has nothing left to draw");
}

/// The one view the built-in browser is handed, kept as plain as the golden screens keep theirs.
fn plain_select_view() -> SelectView {
    SelectView {
        rows: Vec::new(),
        sel: 0,
        header: "ROOT".into(),
        guide: "UP DOWN",
        detail: SelectDetail::Empty,
        modal: None,
        score_graph: false,
        search: None,
        sort: "DEFAULT",
        filter: None,
        empty_hint: None,
    }
}

#[test]
fn drawing_a_document_leaves_the_built_in_screens_untouched() {
    rbms_render::font::use_embedded_fonts_only();
    let view = plain_select_view();

    let mut before = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    render_select_ctx(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut before, &view);

    let mut scratch = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let (screen, _) = build(&mut scratch, &mut text);
    let mut timers = TimerState::new();
    timers.set_on(FIXTURE_TIMER, 0);
    let state = FixtureState::default();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &state, lua: None, mouse: None, data: FrameData::default() };
    screen.draw(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut scratch, &frame);

    let mut after = CpuCanvas::new(CANVAS_W, CANVAS_H);
    render_select_ctx(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut after, &view);
    assert_eq!(before.pixels(), after.pixels(), "a document loaded and drawn does not move a pixel of the built-in screen");
}

#[test]
fn a_screen_state_answers_the_ids_its_view_knows() {
    let view = plain_select_view();
    let state = SelectViewState::new(&view, 17, None, false);
    assert_eq!(state.now_us(), 17, "the frame clock is the one the caller passed");
    assert_eq!(state.text(rbms_skin::property::generated::STRING_DIRECTORY), "ROOT", "the browser's header answers the directory id");
    assert_eq!(state.integer(rbms_skin::property::generated::NUMBER_PLAYLEVEL), UNMAPPED_INTEGER, "no chart is focused, so there is no level to report");
}

#[test]
fn a_host_without_a_sandbox_answers_a_function_value_with_the_fallback_of_its_type() {
    let evaluator = NoExpressions;
    assert!(!evaluator.call_boolean(LuaFnId(0)));
    assert_eq!(evaluator.call_integer(LuaFnId(0)), 0);
    assert_eq!(evaluator.call_float(LuaFnId(0)), 0.0);
    assert_eq!(evaluator.call_text(LuaFnId(0)), "");
    assert_eq!(evaluator.call_timer(LuaFnId(0)), TIMER_OFF);
}

/// The function a scripted fixture reads its number from.
const NUMBER_FUNCTION: LuaFnId = LuaFnId(0);

/// The function a scripted fixture reads its slider and graph from.
const RATE_FUNCTION: LuaFnId = LuaFnId(1);

/// The function a scripted fixture reads its text from.
const TEXT_FUNCTION: LuaFnId = LuaFnId(2);

/// The function a scripted fixture times its animated object by.
const TIMER_FUNCTION: LuaFnId = LuaFnId(3);

/// The function a scripted fixture gates its last object on.
const GATE_FUNCTION: LuaFnId = LuaFnId(4);

/// An evaluator that answers the scripted fixture's five functions out of a [`FixtureState`], the
/// way a loaded Lua skin's functions would read the same game state.
struct ScriptedFixture<'a> {
    state: &'a FixtureState,
    /// The microsecond the scripted timer reports having switched on.
    started_us: i64,
}

impl LuaDrawEval for ScriptedFixture<'_> {
    fn call_boolean(&self, function: LuaFnId) -> bool {
        function == GATE_FUNCTION && self.state.gate
    }

    fn call_integer(&self, function: LuaFnId) -> i32 {
        if function == NUMBER_FUNCTION { self.state.number } else { 0 }
    }

    fn call_float(&self, function: LuaFnId) -> f32 {
        if function == RATE_FUNCTION { self.state.rate } else { 0.0 }
    }

    fn call_text(&self, function: LuaFnId) -> String {
        if function == TEXT_FUNCTION { self.state.text.clone() } else { String::new() }
    }

    fn call_timer(&self, function: LuaFnId) -> i64 {
        if function == TIMER_FUNCTION { self.started_us } else { TIMER_OFF }
    }
}

/// The fixture document with every property id it reads replaced by a function handle, which is
/// what the same document is once a Lua skin has written `value = function() ... end` throughout.
fn scripted_fixture() -> LoadedSkin {
    let root = fixture_root();
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, rbms_model::Mode::BEAT_7K) };
    let mut skin = load_skin(&root.join("skin.json"), options).expect("the fixture document loads");

    skin.def.value[0].value = Some(PropertyRef::Func(NUMBER_FUNCTION));
    skin.def.text[0].value = Some(PropertyRef::Func(TEXT_FUNCTION));
    skin.def.slider[0].value = Some(PropertyRef::Func(RATE_FUNCTION));
    skin.def.graph[0].value = Some(PropertyRef::Func(RATE_FUNCTION));
    skin.def.image[0].timer = Some(PropertyRef::Func(TIMER_FUNCTION));
    for named in &mut skin.destinations {
        match named.id.as_str() {
            "frame" => named.track.timer = Some(TimerRef::Lua(TIMER_FUNCTION)),
            "gated" => named.track.draw_conditions = vec![DrawCondition::Function(GATE_FUNCTION)],
            _ => {}
        }
    }
    skin
}

/// Draws one frame of the scripted fixture at `now_ms` against `lua`, with no timer running and a
/// game state that answers nothing, so every pixel that depends on a value came through `lua`.
/// Answers the canvas and how many objects reached it.
fn scripted_frame_at(now_ms: i64, lua: &dyn LuaDrawEval) -> (CpuCanvas, usize) {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let mut assets = PatternAssets;
    let screen = SkinScreen::build(&mut canvas, &mut text, &scripted_fixture(), &mut assets);
    assert_eq!(screen.warnings(), Vec::<String>::new(), "a function value drops no object");

    let backdrop = canvas.register_texture("test.backdrop", &checker(8, 8, 2), 8, 8);
    let timers = TimerState::new();
    let silent = FixtureState { number: 0, rate: 0.0, text: String::new(), gate: false };
    canvas.clear(Color::BLACK);

    let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
    let frame = SkinFrame { now_us: now_ms * MICROS_PER_MILLI, timers: &timers, state: &silent, lua: Some(lua), mouse: None, data: behind(backdrop) };
    let drawn = screen.draw(&mut ctx, &mut canvas, &frame);
    (canvas, drawn)
}

#[test]
fn a_document_whose_values_are_functions_draws_what_the_same_ids_draw() {
    for gate in [false, true] {
        for now_ms in [0, FIXTURE_CYCLE_MS / 4] {
            let state = FixtureState { gate, ..FixtureState::default() };
            let by_id = frame_at(now_ms, &state);
            let (by_function, _) = scripted_frame_at(now_ms, &ScriptedFixture { state: &state, started_us: 0 });
            assert_eq!(
                by_id.pixels(),
                by_function.pixels(),
                "at {now_ms} ms with the gate {gate}: the number, text, slider, graph, both timers and the draw condition all came through the evaluator"
            );
        }
    }
}

#[test]
fn a_function_timer_that_is_off_hides_its_object_and_one_that_starts_later_delays_it() {
    let state = FixtureState::default();
    let (_, running) = scripted_frame_at(0, &ScriptedFixture { state: &state, started_us: 0 });
    let (_, off) = scripted_frame_at(0, &ScriptedFixture { state: &state, started_us: TIMER_OFF });
    assert_eq!(off, running - 1, "the one object timed by the function is gone and nothing else is");

    let late = FIXTURE_CYCLE_MS / 4;
    let (shifted, _) = scripted_frame_at(late * 2, &ScriptedFixture { state: &state, started_us: late * MICROS_PER_MILLI });
    assert_eq!(shifted.pixels(), frame_at(late, &state).pixels(), "a timer the function starts later shows the frame that much earlier");
}

#[test]
fn a_document_whose_values_are_functions_draws_their_fallbacks_without_an_interpreter() {
    let state = FixtureState { gate: true, ..FixtureState::default() };
    let (_, scripted) = scripted_frame_at(0, &ScriptedFixture { state: &state, started_us: 0 });
    let (_, unscripted) = scripted_frame_at(0, &NoExpressions);
    assert_eq!(
        unscripted,
        scripted - 4,
        "the gated object (false), the timed one (off), the text (empty) and the graph (zero) are not drawn; the number draws a zero and the slider rests"
    );
}

/// A run that never happened, so the fallback tests have a view to hand the result screen.
fn plain_result_view() -> rbms_render::ResultView {
    rbms_render::ResultView {
        title: "GATE".into(),
        artist: String::new(),
        mode_label: "7K",
        counts: [0; 6],
        ex_score: 0,
        max_score: 0,
        max_combo: 0,
        total_notes: 0,
        fast: [0; 2],
        slow: [0; 2],
        gauge: 0.0,
        clear_label: "FAILED",
        clear_color: Color::GRAY,
        prev_best_ex: None,
        prev_ex: None,
        show_graph: false,
        show_result_graphs: false,
        gauge_series: Vec::new(),
        timing_hist: Box::new([]),
        judge_dist: [0; 6],
    }
}

#[test]
fn every_screen_gate_reports_no_document_and_draws_nothing() {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
    canvas.clear(Color::BLACK);
    let blank = canvas.pixels().to_vec();

    let view = plain_select_view();
    let result = plain_result_view();
    let keys = [String::from("SHIFT")];
    assert!(!render_select_screen(&mut ctx, &mut canvas, None, &view));
    assert!(!render_result_screen(&mut ctx, &mut canvas, None, &result, None, false));
    let loading = DecideViewState { progress: 0.5, done: false, title: "GATE", chart: DecideChart::default(), now_us: 0, offsets: None };
    assert!(!render_decide_screen(&mut ctx, &mut canvas, None, &loading));
    assert!(!render_keyconfig_screen(&mut ctx, &mut canvas, None, &keys));
    assert_eq!(canvas.pixels(), blank.as_slice(), "a screen with no document selected leaves the frame for its own layout to fill");
}

/// The difficulty slots the decide screen is asked about below, with the slot each is answered as:
/// the five a chart can name, and on either side of them the charts that name none.
const DECIDE_DIFFICULTIES: [(i32, i32); 8] = [(-1, 0), (0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 0)];

/// A skin colours the decide screen by walking the six difficulty options and has no colour to give
/// when none of them is on, so the screen answers exactly one whatever the chart says -- and a
/// screen that is waiting for something other than a chart answers the one for no difficulty.
#[test]
fn the_decide_screen_answers_exactly_one_difficulty_option() {
    for (difficulty, slot) in DECIDE_DIFFICULTIES {
        let chart = DecideChart { difficulty, ..DecideChart::default() };
        let state = DecideViewState { progress: 0.0, done: false, title: "", chart, now_us: 0, offsets: None };
        let on: Vec<i32> = (OPTION_DIFFICULTY0..=OPTION_DIFFICULTY5).filter(|option| state.boolean(*option) == Some(true)).collect();
        assert_eq!(on, vec![OPTION_DIFFICULTY0 + slot], "difficulty {difficulty}");
        let off: Vec<i32> = (OPTION_DIFFICULTY0..=OPTION_DIFFICULTY5).filter(|option| state.boolean(-option) == Some(false)).collect();
        assert_eq!(off, on, "the negated options of difficulty {difficulty} do not mirror the plain ones");
    }
}

#[test]
fn a_screen_gate_draws_the_document_when_one_is_selected() {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let (screen, _) = build(&mut canvas, &mut text);
    let mut timers = TimerState::new();
    timers.set_on(FIXTURE_TIMER, 0);

    canvas.clear(Color::BLACK);
    let blank = canvas.pixels().to_vec();
    let document = SkinDraw { screen: &screen, timers: &timers, now_us: 0, lua: None, mouse: None, offsets: None, data: FrameData::default() };
    let view = plain_select_view();
    let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
    assert!(render_select_screen(&mut ctx, &mut canvas, Some(&document), &view), "a selected document is what the screen draws");
    assert_ne!(canvas.pixels(), blank.as_slice(), "and it reached the frame");
}

/// A scratch folder holding one generated document and its sources, removed when the test ends.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-skin-render-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        Scratch { root }
    }

    /// Writes one file under the scratch root and answers its path.
    fn write(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.join(name);
        std::fs::write(&path, body).expect("the scratch file is writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Compiles `document` against `canvas` and answers the screen it becomes.
fn compile(root: &Path, document: &Path, canvas: &mut CpuCanvas, text: &mut TextContext) -> SkinScreen {
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(document, options).expect("the generated document loads");
    let mut assets = PatternAssets;
    SkinScreen::build(canvas, text, &skin, &mut assets)
}

/// Draws `screen` over a canvas already painted `ground`, and answers the canvas.
fn over(screen: &SkinScreen, text: &mut TextContext, canvas: &mut CpuCanvas, ground: Color) {
    let timers = TimerState::new();
    let state = FixtureState::default();
    canvas.clear(ground);
    let mut ctx = RenderCtx::new(rbms_render::theme(), text);
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &state, lua: None, mouse: None, data: FrameData::default() };
    screen.draw(&mut ctx, canvas, &frame);
}

/// A one-object document over a checkerboard source, at `blend`, with the alpha the destination
/// names.
fn blended_document(scratch: &Scratch, blend: i32, alpha: i32) -> PathBuf {
    scratch.write("panel.tex", "checker 16 16 8");
    let body = format!(
        r#"{{
            "type": 5, "w": 64, "h": 32,
            "source": [{{ "id": "panel", "path": "panel.tex" }}],
            "image": [{{ "id": "sheet", "src": "panel", "x": 0, "y": 0, "w": 16, "h": 16 }}],
            "destination": [{{ "id": "sheet", "blend": {blend}, "dst": [{{ "time": 0, "x": 0, "y": 0, "w": 64, "h": 32, "a": {alpha} }}] }}]
        }}"#
    );
    scratch.write("skin.json", &body)
}

/// `SkinObject.draw` leaves before it draws anything when the resolved colour is fully transparent.
/// Under alpha and additive blending that only saves work, because both read the source alpha; under
/// multiply and invert-destination the source alpha is not in the blend equation at all, so an
/// object fading out under one of those would keep darkening the screen at `a: 0`. A fade's first
/// and last keyframe is exactly that alpha, so this is a path real documents take.
#[test]
fn a_fully_transparent_object_is_not_drawn_whatever_its_blend_mode() {
    rbms_render::font::use_embedded_fonts_only();
    let ground = Color::rgb(200, 200, 200);
    for blend in [2, 4, 9] {
        let scratch = Scratch::new(&format!("alpha-{blend}"));
        let mut text = TextContext::embedded_only();

        let faded = blended_document(&scratch, blend, 0);
        let mut canvas = CpuCanvas::new(64, 32);
        let screen = compile(&scratch.root, &faded, &mut canvas, &mut text);
        over(&screen, &mut text, &mut canvas, ground);
        assert!((0..32).flat_map(|y| (0..64).map(move |x| (x, y))).all(|(x, y)| canvas.pixel_at(x, y) == ground), "blend {blend} at a: 0 changed the screen");

        let opaque = blended_document(&scratch, blend, 255);
        let mut lit = CpuCanvas::new(64, 32);
        let screen = compile(&scratch.root, &opaque, &mut lit, &mut text);
        over(&screen, &mut text, &mut lit, ground);
        assert!(
            (0..32).flat_map(|y| (0..64).map(move |x| (x, y))).any(|(x, y)| lit.pixel_at(x, y) != ground),
            "blend {blend} at a: 255 drew nothing, so the test above proves nothing"
        );
    }
}

/// `stretch` 9 draws an image at its own pixel size, and the reference measures that against the
/// screen: `Skin.setDestination` has already multiplied the destination into screen coordinates
/// before `StretchType` reshapes it. A document authored at half the screen's size would otherwise
/// have its unresized objects come out at half size.
#[test]
fn an_unresized_object_takes_its_own_pixel_size_on_screen() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("no-resize");
    scratch.write("panel.tex", "checker 16 16 16");
    let document = scratch.write(
        "skin.json",
        r#"{
            "type": 5, "w": 64, "h": 64,
            "source": [{ "id": "panel", "path": "panel.tex" }],
            "image": [{ "id": "sheet", "src": "panel", "x": 0, "y": 0, "w": 16, "h": 16 }],
            "destination": [{ "id": "sheet", "stretch": 9, "dst": [{ "time": 0, "x": 0, "y": 0, "w": 64, "h": 64 }] }]
        }"#,
    );

    let mut text = TextContext::embedded_only();
    let mut canvas = CpuCanvas::new(128, 128);
    let screen = compile(&scratch.root, &document, &mut canvas, &mut text);
    over(&screen, &mut text, &mut canvas, Color::BLACK);

    let painted = (0..128).flat_map(|y| (0..128).map(move |x| (x, y))).filter(|(x, y)| canvas.pixel_at(*x, *y) != Color::BLACK).count();
    assert_eq!(painted, 16 * 16, "an unresized 16x16 source covers sixteen screen pixels a side, not thirty-two");
}

/// The modes that trim read less of the source instead of drawing past the destination, so what
/// reaches the screen is the middle of the image at its own scale: `stretch` 10 over a source twice
/// the destination's size on both axes shows the centre quarter, one source pixel to a screen pixel.
#[test]
fn a_trimmed_object_reads_the_middle_of_its_source_instead_of_overflowing() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("no-resize-trimmed");
    scratch.write("panel.tex", "cells 32 32 4 4");
    let document = |stretch: i32| {
        format!(
            r#"{{
                "type": 5, "w": 64, "h": 64,
                "source": [{{ "id": "panel", "path": "panel.tex" }}],
                "image": [{{ "id": "sheet", "src": "panel", "x": 0, "y": 0, "w": 32, "h": 32 }}],
                "destination": [{{ "id": "sheet", "stretch": {stretch}, "dst": [{{ "time": 0, "x": 24, "y": 24, "w": 16, "h": 16 }}] }}]
            }}"#
        )
    };
    let draw = |stretch: i32| {
        let path = scratch.write("skin.json", &document(stretch));
        let mut text = TextContext::embedded_only();
        let mut canvas = CpuCanvas::new(64, 64);
        let screen = compile(&scratch.root, &path, &mut canvas, &mut text);
        over(&screen, &mut text, &mut canvas, Color::BLACK);
        canvas
    };
    let painted = |canvas: &CpuCanvas| (0..64).flat_map(|y| (0..64).map(move |x| (x, y))).filter(|(x, y)| canvas.pixel_at(*x, *y) != Color::BLACK).count();

    let spilled = draw(9);
    assert_eq!(painted(&spilled), 32 * 32, "an unresized 32x32 source spills over its 16x16 destination");

    let trimmed = draw(10);
    assert_eq!(painted(&trimmed), 16 * 16, "the trimmed twin stays inside it");
    for (x, y) in (24..40).flat_map(|y| (24..40).map(move |x| (x, y))) {
        assert_eq!(trimmed.pixel_at(x, y), spilled.pixel_at(x, y), "and shows the same source pixel the spilled one puts at ({x}, {y})");
    }
}

/// A background is the one quad whose size the window decides, so both paths that draw one -- the
/// player's own slot and a document's `bga` object -- have to agree on how it is sampled. Point
/// sampling a shrink is what turns an animated background into a shimmer.
#[test]
fn a_resized_background_is_sampled_linearly_and_an_exact_one_is_not() {
    use rbms_render::{Rect, TextureFilter, background_filter};

    assert_eq!(background_filter(Rect::new(0.0, 0.0, 8.0, 8.0), (8, 8)), TextureFilter::Nearest, "an exact fit reads its texels straight");
    assert_eq!(background_filter(Rect::new(0.0, 0.0, 32.0, 32.0), (8, 8)), TextureFilter::Linear, "an enlargement is interpolated");
    assert_eq!(background_filter(Rect::new(0.0, 0.0, 4.0, 4.0), (8, 8)), TextureFilter::Linear, "and so is a shrink");
}

/// A document's `bga` object goes through the same rule, which is visible in the pixels: a
/// checkerboard blown up with point sampling has hard edges, and one blown up with interpolation
/// has a value between the two squares along them.
#[test]
fn a_documents_background_object_is_interpolated_when_it_is_resized() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("bga-filter");
    let document = scratch.write(
        "skin.json",
        r#"{
            "type": 5, "w": 64, "h": 64,
            "bga": { "id": "backdrop" },
            "destination": [{ "id": "backdrop", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 64, "h": 64 }] }]
        }"#,
    );

    let mut text = TextContext::embedded_only();
    let mut canvas = CpuCanvas::new(64, 64);
    let screen = compile(&scratch.root, &document, &mut canvas, &mut text);
    let backdrop = canvas.register_texture("test.bga", &checker(8, 8, 4), 8, 8);

    let timers = TimerState::new();
    let state = FixtureState::default();
    canvas.clear(Color::BLACK);
    let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &state, lua: None, mouse: None, data: behind(backdrop) };
    assert_eq!(screen.draw(&mut ctx, &mut canvas, &frame), 1, "the document's bga object drew");

    let shades: Vec<u8> = (0..64).map(|x| canvas.pixel_at(x, 32).r).collect();
    assert!(shades.iter().any(|shade| *shade > 100 && *shade < 250), "an eight-fold enlargement was point sampled: {shades:?}");
}

/// A property with nothing to report answers one of the two sentinels rather than a number, and the
/// object that reads it is not drawn at all (`SkinNumber.prepare`). Nothing in this build answers
/// one yet, so this is what keeps the first band that does -- a best score nobody has set -- from
/// drawing a ten-digit `-2147483648` across the screen.
#[test]
fn a_number_with_no_value_draws_nothing() {
    let counts = |number: i32| {
        rbms_render::font::use_embedded_fonts_only();
        let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
        let mut text = TextContext::embedded_only();
        let (screen, _) = build(&mut canvas, &mut text);
        let backdrop = canvas.register_texture("test.backdrop", &checker(8, 8, 2), 8, 8);
        let mut timers = TimerState::new();
        timers.set_on(FIXTURE_TIMER, 0);
        canvas.clear(Color::BLACK);
        let state = FixtureState { number, ..FixtureState::default() };
        let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
        let frame = SkinFrame { now_us: 0, timers: &timers, state: &state, lua: None, mouse: None, data: behind(backdrop) };
        screen.draw(&mut ctx, &mut canvas, &frame)
    };

    let ordinary = counts(42);
    assert_eq!(counts(i32::MIN), ordinary - 1, "the sentinel hides the object that reads it and nothing else");
    assert_eq!(counts(i32::MAX), ordinary - 1, "both sentinels mean the same thing");
}

/// How many objects the scale document holds. Real skins run to hundreds; the fixture next door is
/// eight, which says nothing about what a frame costs when a document is the size people publish.
const SCALE_OBJECTS: usize = 500;

/// The longest one frame of the scale document may take, generously wide because this runs in a
/// debug build on whatever machine happens to be free. It is here to catch a change that makes a
/// frame quadratic in the object count, not to measure anything.
const SCALE_FRAME_LIMIT: std::time::Duration = std::time::Duration::from_secs(2);

/// A document of `SCALE_OBJECTS` image objects tiled across the screen.
fn scale_document(scratch: &Scratch) -> PathBuf {
    scratch.write("panel.tex", "checker 8 8 4");
    let images: Vec<String> = (0..SCALE_OBJECTS).map(|i| format!(r#"{{ "id": "o{i}", "src": "panel", "x": 0, "y": 0, "w": 8, "h": 8 }}"#)).collect();
    let destinations: Vec<String> = (0..SCALE_OBJECTS)
        .map(|i| {
            let (x, y) = ((i % 32) * 8, (i / 32) * 8);
            format!(r#"{{ "id": "o{i}", "dst": [{{ "time": 0, "x": {x}, "y": {y}, "w": 8, "h": 8 }}] }}"#)
        })
        .collect();
    let body = format!(
        r#"{{ "type": 5, "w": 256, "h": 144, "source": [{{ "id": "panel", "path": "panel.tex" }}], "image": [{}], "destination": [{}] }}"#,
        images.join(","),
        destinations.join(",")
    );
    scratch.write("skin.json", &body)
}

/// A document the size people actually publish still draws every object it declares, in one frame,
/// without the cost turning superlinear.
#[test]
fn a_document_of_hundreds_of_objects_draws_them_all_in_one_frame() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("scale");
    let document = scale_document(&scratch);
    let mut text = TextContext::embedded_only();
    let mut canvas = CpuCanvas::new(256, 144);
    let screen = compile(&scratch.root, &document, &mut canvas, &mut text);
    assert_eq!(screen.object_count(), SCALE_OBJECTS);

    let timers = TimerState::new();
    let state = FixtureState::default();
    canvas.clear(Color::BLACK);
    let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &state, lua: None, mouse: None, data: FrameData::default() };
    let started = std::time::Instant::now();
    let drawn = screen.draw(&mut ctx, &mut canvas, &frame);
    let spent = started.elapsed();

    assert_eq!(drawn, SCALE_OBJECTS, "every object reached the screen");
    assert!(spent < SCALE_FRAME_LIMIT, "one frame of {SCALE_OBJECTS} objects took {spent:?}");
}

/// The option the Lua fixtures below gate their objects on, which the test switches.
const GATE_OPTION: i32 = 900;

/// What every Lua fixture starts with: `signed(id, answer)` makes a function that tells the host it
/// ran, by running event `id`, and then answers; `gated(id)` does the same and answers whether
/// [`GATE_OPTION`] is on. A host that records what it is told therefore holds the order the skin's
/// functions were called in.
const SIGNED_PRELUDE: &str = r#"
local main_state = require("main_state")
local function signed(id, answer)
    return function()
        main_state.event_exec(id)
        return answer
    end
end
local function gated(id)
    return function()
        main_state.event_exec(id)
        return main_state.option(GATE_OPTION)
    end
end
"#;

/// Three images, each gated by a function and timed by another, signed 1 to 6 in the order the
/// reference calls them.
const ORDERED_SKIN: &str = r#"
return {
    type = 5, name = "ordered", w = 256, h = 144,
    source = { { id = 0, path = "panel.tex" } },
    image = { { id = "tile", src = 0, x = 0, y = 0, w = 16, h = 16 } },
    destination = {
        { id = "tile", draw = signed(1, true), timer = signed(2, 0), dst = { { x = 0, y = 0, w = 32, h = 32 } } },
        { id = "tile", draw = signed(3, true), timer = signed(4, 0), dst = { { x = 64, y = 0, w = 32, h = 32 } } },
        { id = "tile", draw = signed(5, true), timer = signed(6, 0), dst = { { x = 128, y = 0, w = 32, h = 32 } } },
    },
}
"#;

/// How many functions [`ORDERED_SKIN`] calls on a frame.
const ORDERED_CALLS: usize = 6;

/// One object of every kind whose value is read at a moment of its own: an image set, a number, a
/// text, a slider and a graph. Each is gated by a function signed `n2`, timed by one signed `n4` and
/// reads its value from one signed `n1`; the image set's image animates on a timer signed 13.
const KINDS_SKIN: &str = r#"
local at = { { x = 0, y = 0, w = 16, h = 16 } }
return {
    type = 5, name = "kinds", w = 256, h = 144,
    source = { { id = 0, path = "panel.tex" }, { id = 1, path = "strip.tex" } },
    image = { { id = "tile", src = 0, x = 0, y = 0, w = 16, h = 16, divx = 2, timer = signed(13, 0), cycle = 100 } },
    imageset = { { id = "pick", images = { "tile" }, value = signed(11, 0) } },
    value = { { id = "count", src = 1, x = 0, y = 0, w = 40, h = 8, divx = 10, digit = 3, value = signed(21, 7) } },
    text = { { id = "label", font = 0, size = 12, value = signed(31, "x") } },
    slider = { { id = "knob", src = 0, x = 0, y = 0, w = 16, h = 16, range = 48, angle = 1, value = signed(41, 0.5) } },
    graph = { { id = "meter", src = 0, x = 0, y = 0, w = 16, h = 16, angle = 1, value = signed(51, 0.5) } },
    destination = {
        { id = "pick", draw = gated(12), timer = signed(14, 0), dst = at },
        { id = "count", draw = gated(22), timer = signed(24, 0), dst = at },
        { id = "label", draw = gated(32), timer = signed(34, 0), dst = at },
        { id = "knob", draw = gated(42), timer = signed(44, 0), dst = at },
        { id = "meter", draw = gated(52), timer = signed(54, 0), dst = at },
    },
}
"#;

/// A Lua fixture written out and loaded against `host`, held to `budget`.
fn lua_fixture(scratch: &Scratch, body: &str, host: &MapHost, budget: LuaBudget) -> LoadedSkin {
    scratch.write("panel.tex", "checker 16 16 4");
    scratch.write("strip.tex", "cells 40 8 10 1");
    let source = format!("{SIGNED_PRELUDE}{body}").replace("GATE_OPTION", &GATE_OPTION.to_string());
    let entry = scratch.write("fixture.luaskin", &source);
    let user = SkinUserConfig::default();
    let load = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    load_lua_skin(&entry, &LuaSkinOptions { load, budget }, host).expect("the Lua fixture loads")
}

/// The ids of the events a host was told to run since it was last asked, oldest first: which of a
/// fixture's signed functions ran, in the order they did.
fn signatures(host: &MapHost) -> Vec<i32> {
    host.take_calls().into_iter().filter_map(|call| if let HostCall::Event { id, .. } = call { Some(id) } else { None }).collect()
}

/// A renderer that notes, every time something is painted, how many things the host had been told
/// by then. Everything else goes straight through to the canvas underneath.
struct Watched<'a> {
    canvas: &'a mut CpuCanvas,
    host: &'a MapHost,
    told_at_paint: Vec<usize>,
}

impl Renderer for Watched<'_> {
    fn size(&self) -> (u32, u32) {
        self.canvas.size()
    }

    fn clear(&mut self, color: Color) {
        self.canvas.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.told_at_paint.push(self.host.calls().len());
        self.canvas.fill_rect(rect, color);
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        self.canvas.register_texture(key, rgba, width, height)
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.canvas.release_texture(tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.canvas.texture_size(tex)
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        self.told_at_paint.push(self.host.calls().len());
        self.canvas.draw_textured_quad(tex, params);
    }

    fn push_clip(&mut self, rect: Rect) {
        self.canvas.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.canvas.pop_clip();
    }
}

/// A frame at the start of the scene over `host`, with no interpreter in it.
fn unbound_frame<'a>(timers: &'a TimerState, host: &'a MapHost) -> SkinFrame<'a> {
    SkinFrame { now_us: 0, timers, state: host, lua: None, mouse: None, data: FrameData::default() }
}

/// `Skin.drawAllObjects` prepares every object and only then draws the first, so a skin's functions
/// have all run, in the order the document declared their objects, before anything is painted.
#[test]
fn every_object_is_prepared_before_the_first_one_is_drawn() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("ordered");
    let host = MapHost::new();
    let skin = lua_fixture(&scratch, ORDERED_SKIN, &host, LuaBudget::default());
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut PatternAssets);
    assert_eq!((screen.object_count(), screen.warnings()), (3, &[][..]));
    host.take_calls();

    let timers = TimerState::new();
    let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
    let mut watched = Watched { canvas: &mut canvas, host: &host, told_at_paint: Vec::new() };
    let runtime = skin.runtime().expect("a Lua skin has an interpreter");
    let drawn = runtime
        .frame(&host, |bound| screen.draw(&mut ctx, &mut watched, &SkinFrame { lua: Some(bound), ..unbound_frame(&timers, &host) }))
        .expect("the host binds to the interpreter");

    assert_eq!(drawn, 3, "every object reached the screen");
    assert_eq!(watched.told_at_paint, vec![ORDERED_CALLS; 3], "all six functions had run by the time the first object was painted");
    assert_eq!(signatures(&host), [1, 2, 3, 4, 5, 6], "each object's condition and then its timer, object by object in document order");
}

/// The skin's Lua is asked while a frame is prepared and never while it is drawn, so the host has to
/// be bound to the interpreter once, around the prepare stage, and the frame can be drawn after the
/// binding has ended.
#[test]
fn a_frame_is_prepared_inside_one_binding_and_drawn_after_it_has_ended() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("binding");
    let host = MapHost::new();
    let budget = LuaBudget { frame: FrameBudget { max_calls: ORDERED_CALLS as u32 - 1, ..FrameBudget::default() }, ..LuaBudget::default() };
    let skin = lua_fixture(&scratch, ORDERED_SKIN, &host, budget);
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut PatternAssets);
    let runtime = skin.runtime().expect("a Lua skin has an interpreter");
    let timers = TimerState::new();
    let frame = unbound_frame(&timers, &host);
    host.take_calls();

    let frames = 3;
    for _ in 0..frames {
        let prepared = runtime.frame(&host, |bound| screen.prepare(&SkinFrame { lua: Some(bound), ..frame })).expect("the host binds to the interpreter");
        assert_eq!((prepared.object_count(), prepared.answer_count()), (3, ORDERED_CALLS), "every object was prepared and every question kept");
        assert_eq!(signatures(&host), [1, 2, 3, 4, 5], "the one binding holds the whole prepare stage, so its budget runs out on the sixth call");
        assert_eq!(prepared.visible_count(), 2, "the timer that was never called reads as off, which hides the object it times");

        canvas.clear(Color::BLACK);
        let drawn = screen.draw_prepared(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut canvas, &frame, &prepared);
        assert_eq!(drawn, 2, "the frame draws with nothing bound to the interpreter");
        assert_eq!(signatures(&host), Vec::<i32>::new(), "and drawing it calls none of the skin's functions");
    }

    let diagnostics = runtime.diagnostics();
    assert_eq!(diagnostics.frames_over_budget, frames, "one binding a frame: every frame met the ceiling once, in its prepare stage");
    assert_eq!(diagnostics.function_failures, Vec::new(), "nothing was called outside a binding, where a skin's reads fail");

    let staged = canvas.pixels().to_vec();
    canvas.clear(Color::BLACK);
    runtime
        .frame(&host, |bound| screen.draw(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut canvas, &SkinFrame { lua: Some(bound), ..frame }))
        .expect("the host binds to the interpreter");
    assert_eq!(canvas.pixels(), staged.as_slice(), "the two stages called apart draw what one call draws");
}

/// What the kinds fixture's functions sign when every gate holds, in the reference's order: an image
/// set and a number read their value before anything else, a text reads its after it is placed, and
/// a slider and a graph read theirs last of all (`SkinImage`, `SkinNumber`, `SkinText`, `SkinSlider`
/// and `SkinGraph`, each in its own `prepare`).
const KINDS_SHOWN: [i32; 16] = [11, 12, 14, 13, 21, 22, 24, 32, 34, 31, 42, 44, 41, 52, 54, 51];

/// The same with every gate failing. No destination timer is read. The slider and the graph read
/// nothing more at all; the image set, the number and the text have their value read regardless,
/// and the image set its source's timer, because those classes read them outside the steps a failed
/// condition cuts short.
const KINDS_HIDDEN: [i32; 9] = [11, 12, 13, 21, 22, 32, 31, 42, 52];

/// A draw condition that fails stops an object's prepare where the reference stops it, which is not
/// the same place for every kind of object.
#[test]
fn an_object_whose_condition_fails_reads_only_what_the_reference_reads_of_it() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("kinds");
    let mut host = MapHost::new();
    host.booleans.insert(GATE_OPTION, true);
    let skin = lua_fixture(&scratch, KINDS_SKIN, &host, LuaBudget::default());
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut PatternAssets);
    assert_eq!((screen.object_count(), screen.warnings()), (5, &[][..]));
    let runtime = skin.runtime().expect("a Lua skin has an interpreter");
    let timers = TimerState::new();
    host.take_calls();

    let shown = runtime.frame(&host, |bound| screen.prepare(&SkinFrame { lua: Some(bound), ..unbound_frame(&timers, &host) })).expect("the host binds");
    assert_eq!(shown.visible_count(), 5);
    assert_eq!(signatures(&host), KINDS_SHOWN);

    host.booleans.insert(GATE_OPTION, false);
    let hidden = runtime.frame(&host, |bound| screen.prepare(&SkinFrame { lua: Some(bound), ..unbound_frame(&timers, &host) })).expect("the host binds");
    assert_eq!(hidden.visible_count(), 0, "a failed condition leaves the object out of the frame");
    assert_eq!(signatures(&host), KINDS_HIDDEN);

    canvas.clear(Color::BLACK);
    let blank = canvas.pixels().to_vec();
    let drawn = screen.draw_prepared(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut canvas, &unbound_frame(&timers, &host), &hidden);
    assert_eq!((drawn, canvas.pixels()), (0, blank.as_slice()), "and nothing of it is drawn");
}

/// How many slots the wheel of [`unplaced_wheel`] has, and which of them is its centre.
const WHEEL_SLOTS: usize = 3;
const WHEEL_CENTER: usize = 1;

/// A canvas pixel inside the wheel's first bar, which the document puts at (16, 100) and makes 96
/// by 24 in a space half the canvas's size, measured up from the bottom.
const WHEEL_BAR_PIXEL: (u32, u32) = (40, 60);

/// A browser document whose wheel is named by a destination with no `dst` at all, which is how a
/// published skin names it: every slot carries a destination of its own.
fn unplaced_wheel(scratch: &Scratch) -> PathBuf {
    scratch.write("panel.tex", "checker 16 16 4");
    let slots: Vec<String> =
        (0..WHEEL_SLOTS).map(|slot| format!(r#"{{ "id": "bar", "dst": [{{ "x": 16, "y": {}, "w": 96, "h": 24 }}] }}"#, 100 - 40 * slot)).collect();
    let body = format!(
        r#"{{
            "type": 5, "w": 256, "h": 144,
            "source": [{{ "id": "panel", "path": "panel.tex" }}],
            "image": [{{ "id": "bar", "src": "panel", "x": 0, "y": 0, "w": 16, "h": 16 }}],
            "songlist": {{ "id": "wheel", "center": {WHEEL_CENTER}, "listoff": [{slots}], "liston": [{slots}] }},
            "destination": [{{ "id": "wheel" }}]
        }}"#,
        slots = slots.join(",")
    );
    scratch.write("skin.json", &body)
}

/// One browser row.
fn wheel_row(index: usize) -> SelectRow {
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

/// The reference constructs a song wheel with a keyframe of its own (`SkinBar`), so the destination
/// that names one needs no `dst`: the wheel is always there, and the destinations nested under it
/// place every bar.
#[test]
fn a_song_wheel_named_by_a_destination_with_no_keyframe_is_drawn() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("unplaced-wheel");
    let document = unplaced_wheel(&scratch);
    let mut text = TextContext::embedded_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let screen = compile(&scratch.root, &document, &mut canvas, &mut text);
    assert_eq!((screen.count_of(SkinObjectKind::SongList), screen.warnings()), (1, &[][..]));

    let rows: Vec<SelectRow> = (0..WHEEL_SLOTS).map(wheel_row).collect();
    let bars = SongBars { rows: &rows, sel: WHEEL_CENTER, options_open: false };
    let timers = TimerState::new();
    let state = FixtureState::default();
    canvas.clear(Color::BLACK);
    let blank = canvas.pixels().to_vec();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &state, lua: None, mouse: None, data: FrameData { bars: Some(&bars), ..FrameData::default() } };
    let drawn = screen.draw(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut canvas, &frame);

    assert_eq!(drawn, 1, "the wheel reached the screen");
    assert_ne!(canvas.pixels(), blank.as_slice(), "and its bars with it");
    assert_ne!(
        canvas.pixel_at(WHEEL_BAR_PIXEL.0, WHEEL_BAR_PIXEL.1),
        Color::BLACK,
        "a bar is drawn in its own colour, not through the transparent keyframe the wheel starts from"
    );
}

/// A chart with one note in every lane of a seven-key field, a beat in.
const FIELD_CHART: &[u8] = b"#PLAYER 1\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00111:0001\r\n#00112:0001\r\n#00113:0001\r\n#00114:0001\r\n#00115:0001\r\n#00116:0001\r\n#00118:0001\r\n#00119:0001\r\n";

/// Lanes a seven-key field has: the turntable and seven keys.
const FIELD_LANES: usize = 8;

/// The size the play fixture is authored and drawn at. A note's row comes from the built-in field's
/// own geometry, which is laid out for a screen this tall.
const FIELD_SIZE: (u32, u32) = (1280, 720);

/// The scroll speed the play fixture is drawn at, slow enough to keep the chart's one row of notes
/// inside the field.
const FIELD_HISPEED: f64 = 0.5;

/// A play document whose note field is named by a destination that carries an offset and no `dst`,
/// as a published skin names it.
fn unplaced_field(scratch: &Scratch) -> PathBuf {
    scratch.write("panel.tex", "checker 16 16 4");
    let names = [r#""note""#; FIELD_LANES].join(",");
    let lanes: Vec<String> = (0..FIELD_LANES).map(|lane| format!(r#"{{ "x": {}, "y": 100, "w": 40, "h": 560 }}"#, 40 + 50 * lane)).collect();
    let body = format!(
        r#"{{
            "type": 0, "w": {}, "h": {},
            "source": [{{ "id": "panel", "path": "panel.tex" }}],
            "image": [{{ "id": "note", "src": "panel", "x": 0, "y": 0, "w": 16, "h": 16 }}],
            "note": {{ "id": "notes", "note": [{names}], "dst": [{lanes}] }},
            "destination": [{{ "id": "notes", "offset": 30 }}]
        }}"#,
        FIELD_SIZE.0,
        FIELD_SIZE.1,
        lanes = lanes.join(",")
    );
    scratch.write("skin.json", &body)
}

/// The same holds for a note field (`SkinNote`): its lanes are placed by the note record, and the
/// destination that names the field only says it is there.
#[test]
fn a_note_field_named_by_a_destination_with_no_keyframe_is_drawn() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("unplaced-field");
    let document = unplaced_field(&scratch);
    let mut text = TextContext::embedded_only();
    let mut canvas = CpuCanvas::new(FIELD_SIZE.0, FIELD_SIZE.1);
    let screen = compile(&scratch.root, &document, &mut canvas, &mut text);
    assert_eq!((screen.count_of(SkinObjectKind::Note), screen.warnings()), (1, &[][..]));

    let field = Skin::default_for(rbms_model::Mode::BEAT_7K, FIELD_SIZE.0 as f32, FIELD_SIZE.1 as f32);
    let chart = to_model(&parse(FIELD_CHART), rbms_model::Mode::BEAT_7K);
    let timelines = &chart.timelines;
    let playfield = PlayfieldView { timelines, microtime: 0, hispeed: FIELD_HISPEED, beam_on: &[], beam_off: &[], constant: false, legacy_note: false };
    let play = NoteField { field: &field, playfield: &playfield, shade: LaneShade::default(), bomb: &[], keys_down: &[] };
    let data = FrameData { field: Some(&play), gauge: Some(GaugeFrame::of_kind(0, field.gauge_clear_threshold)), ..FrameData::default() };
    let timers = TimerState::new();
    let state = FixtureState::default();
    canvas.clear(Color::BLACK);
    let blank = canvas.pixels().to_vec();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &state, lua: None, mouse: None, data };
    let drawn = screen.draw(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut canvas, &frame);

    assert_eq!(drawn, 1, "the note field reached the screen");
    assert_ne!(canvas.pixels(), blank.as_slice(), "and its notes with it");
}

/// Any other object needs a keyframe to be anywhere at all. The reference removes one that has none
/// before the first frame (`SkinObject.validate`), so the screen is built without it rather than
/// asking its conditions on every frame for nothing.
#[test]
fn an_ordinary_object_named_by_a_destination_with_no_keyframe_is_left_out_of_the_screen() {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new("unplaced-image");
    scratch.write("panel.tex", "checker 16 16 4");
    let document = scratch.write(
        "skin.json",
        r#"{
            "type": 5, "w": 256, "h": 144,
            "source": [{ "id": "panel", "path": "panel.tex" }],
            "image": [{ "id": "tile", "src": "panel", "x": 0, "y": 0, "w": 16, "h": 16 }],
            "destination": [{ "id": "tile" }, { "id": "tile", "dst": [{ "x": 0, "y": 0, "w": 32, "h": 32 }] }]
        }"#,
    );
    let mut text = TextContext::embedded_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let screen = compile(&scratch.root, &document, &mut canvas, &mut text);

    assert_eq!(screen.object_count(), 1, "only the destination that says where the image goes became an object");
    assert_eq!(screen.warnings().len(), 1, "and the other is reported: {:?}", screen.warnings());
    assert!(screen.warnings()[0].contains("no destination keyframe"), "{:?}", screen.warnings());
}
