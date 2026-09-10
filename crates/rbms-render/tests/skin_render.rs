//! Drawing a real JSON skin document, end to end.
//!
//! The fixture next door is a small select-screen document that uses every object kind this build
//! draws, so one load exercises the whole path: image sources become textures, destinations become
//! tracks, and a frame turns those into quads. Its images are text files naming a pattern, which the
//! decoder here generates -- no binary asset is committed for a skin that only needs to be
//! recognisable.

use std::path::{Path, PathBuf};

use rbms_render::skin_render::state::SelectViewState;
use rbms_render::skin_render::{SkinDraw, render_decide_screen, render_keyconfig_screen, render_result_screen, render_select_screen};
use rbms_render::{
    Color, CpuCanvas, GoldenImage, GoldenOptions, NoExpressions, PngCodec, RenderCtx, Renderer, SelectDetail, SelectView, SkinAssets, SkinFrame, SkinImage,
    SkinObjectKind, SkinScreen, TextContext, assert_golden_png, render_select_ctx,
};
use rbms_skin::dst::LuaExprId;
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::property::{SkinStateSource, UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{TimerId, TimerState};

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
struct PatternAssets {
    /// How many expressions were asked for, so a test can prove the document carried none.
    compiled: usize,
}

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

    fn expression(&mut self, _source: &str) -> Option<LuaExprId> {
        self.compiled += 1;
        None
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
    fn boolean(&self, id: i32) -> bool {
        let answer = if id.abs() == FIXTURE_GATE_ID { self.gate } else { UNMAPPED_BOOLEAN };
        if id < 0 { !answer } else { answer }
    }
}

impl SkinStateSource for FixtureState {
    fn integer(&self, id: i32) -> i32 {
        if id == FIXTURE_NUMBER_ID { self.number } else { UNMAPPED_INTEGER }
    }

    fn float(&self, id: i32) -> f32 {
        if id == FIXTURE_RATE_ID { self.rate } else { UNMAPPED_FLOAT }
    }

    fn string(&self, id: i32) -> &str {
        if id == FIXTURE_TEXT_ID { &self.text } else { UNMAPPED_STRING }
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        0
    }
}

impl Default for FixtureState {
    fn default() -> Self {
        FixtureState { number: 42, rate: 0.5, text: "RBMS".into(), gate: false }
    }
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
    let mut assets = PatternAssets { compiled: 0 };
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
    let frame = SkinFrame { now_ms, timers: &timers, state, lua: None, mouse: None, background: Some(backdrop) };
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
    let frame = SkinFrame { now_ms: 0, timers: &timers, state: &state, lua: None, mouse: None, background: None };
    screen.draw(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut scratch, &frame);

    let mut after = CpuCanvas::new(CANVAS_W, CANVAS_H);
    render_select_ctx(&mut RenderCtx::new(rbms_render::theme(), &mut text), &mut after, &view);
    assert_eq!(before.pixels(), after.pixels(), "a document loaded and drawn does not move a pixel of the built-in screen");
}

#[test]
fn a_screen_state_answers_the_ids_its_view_knows() {
    let view = plain_select_view();
    let state = SelectViewState { view: &view, now_ms: 17, offsets: None };
    assert_eq!(state.now_ms(), 17, "the frame clock is the one the caller passed");
    assert_eq!(state.string(rbms_skin::property::generated::STRING_DIRECTORY), "ROOT", "the browser's header answers the directory id");
    assert_eq!(state.integer(rbms_skin::property::generated::NUMBER_PLAYLEVEL), UNMAPPED_INTEGER, "no chart is focused, so there is no level to report");
}

#[test]
fn a_document_with_no_expressions_never_asks_the_host_to_compile_one() {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let root = fixture_root();
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&root.join("skin.json"), options).expect("the fixture document loads");
    let mut assets = PatternAssets { compiled: 0 };
    let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut assets);

    assert_eq!(assets.compiled, 0, "every value in the fixture is a plain property id");
    assert!(screen.families().is_empty(), "the fixture names no font file, so nothing was registered with the text engine");
}

#[test]
fn a_host_without_a_sandbox_answers_nothing_rather_than_guessing() {
    let evaluator = NoExpressions;
    assert_eq!(rbms_render::SkinExprEval::eval_integer(&evaluator, LuaExprId(0)), None);
    assert_eq!(rbms_render::SkinExprEval::eval_float(&evaluator, LuaExprId(0)), None);
    assert_eq!(rbms_render::SkinExprEval::eval_text(&evaluator, LuaExprId(0)), None);
    assert_eq!(rbms_skin::dst::LuaDrawEval::eval_draw(&evaluator, LuaExprId(0)), None);
}

/// A run that never happened, so the fallback tests have a view to hand the result screen.
fn plain_result_view() -> rbms_render::ResultView {
    rbms_render::ResultView {
        title: "GATE".into(),
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
    assert!(!render_decide_screen(&mut ctx, &mut canvas, None, 0.5, false, "GATE"));
    assert!(!render_keyconfig_screen(&mut ctx, &mut canvas, None, &keys));
    assert_eq!(canvas.pixels(), blank.as_slice(), "a screen with no document selected leaves the frame for its own layout to fill");
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
    let document = SkinDraw { screen: &screen, timers: &timers, now_ms: 0, lua: None, mouse: None, background: None, offsets: None };
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
    let mut assets = PatternAssets { compiled: 0 };
    SkinScreen::build(canvas, text, &skin, &mut assets)
}

/// Draws `screen` over a canvas already painted `ground`, and answers the canvas.
fn over(screen: &SkinScreen, text: &mut TextContext, canvas: &mut CpuCanvas, ground: Color) {
    let timers = TimerState::new();
    let state = FixtureState::default();
    canvas.clear(ground);
    let mut ctx = RenderCtx::new(rbms_render::theme(), text);
    let frame = SkinFrame { now_ms: 0, timers: &timers, state: &state, lua: None, mouse: None, background: None };
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
    let frame = SkinFrame { now_ms: 0, timers: &timers, state: &state, lua: None, mouse: None, background: Some(backdrop) };
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
        let frame = SkinFrame { now_ms: 0, timers: &timers, state: &state, lua: None, mouse: None, background: Some(backdrop) };
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
    let frame = SkinFrame { now_ms: 0, timers: &timers, state: &state, lua: None, mouse: None, background: None };
    let started = std::time::Instant::now();
    let drawn = screen.draw(&mut ctx, &mut canvas, &frame);
    let spent = started.elapsed();

    assert_eq!(drawn, SCALE_OBJECTS, "every object reached the screen");
    assert!(spent < SCALE_FRAME_LIMIT, "one frame of {SCALE_OBJECTS} objects took {spent:?}");
}
