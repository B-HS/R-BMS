//! What a document's own timers, events and clickable objects do over a frame.
//!
//! Every test here loads a real document and compiles it the way a screen does, because the three
//! features only exist once the loader has parsed them and the sandbox has compiled what they
//! carry. The timer test goes all the way to pixels: what a custom timer is for is making an
//! animation run, so the thing worth asserting is that the object was drawn.

use std::path::{Path, PathBuf};

use rbms_skin::dst::{DrawStateSource, LuaDrawEval, LuaExprId, OffsetSource, SkinOffset};
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::lua::{LuaFrame, LuaSandbox};
use rbms_skin::property::{SkinStateSource, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{TimerId, TimerRequest, TimerState};

use super::{DocumentEvents, SkinEventClick, SkinEventFrame, SkinEventRequest, SkinRef, is_builtin_event};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::skin_render::state::{FrameExtra, SkinHotAction, SkinHotspot};
use crate::skin_render::{SkinAssets, SkinExprEval, SkinFrame, SkinImage, SkinScreen};
use crate::{BYTES_PER_PIXEL, Color, CpuCanvas, Rect, Renderer};

/// Width and height every fixture is authored at, which is also the canvas it is drawn on, so a
/// document rectangle and a screen rectangle differ only by the vertical flip.
const DOC_W: u32 = 64;
const DOC_H: u32 = 32;

/// Size of the one texture a fixture's images are cut from.
const TEX_SIZE: u32 = 4;

/// What that texture decodes to, which is also what an object drawn from it with no tint of its own
/// lands on the canvas as.
const SOLID: Color = Color { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: u8::MAX };

/// The option id a fixture's conditions read, which the test state answers.
const OPTION_READY: i32 = 900;

/// A second option id, for the test that needs two conditions.
const OPTION_OTHER: i32 = 901;

/// The document's own timer the fixtures animate against.
const TIMER_FLASH: i32 = 10_001;

/// A second document timer, which only an action ever moves.
const TIMER_PASSIVE: i32 = 10_002;

/// The moment a fixture's timer expression answers with, in milliseconds.
const TIMER_MOMENT: i64 = 1_000;

/// The built-in event a fixture's clickable objects name: the browser's sort order.
const EVENT_SORT: i32 = 12;

/// An event number nothing built in uses, which a document's own `customEvents` may take.
const EVENT_CUSTOM: i32 = 5_000;

/// How long a fixture's conditional event waits between firings.
const MIN_INTERVAL_MS: i32 = 500;

/// A scratch folder holding the generated document and its source, removed when the test ends.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-skin-events-{tag}-{}", std::process::id()));
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

/// A host that decodes one opaque white square and compiles expressions into the document's own
/// sandbox, which is what the player does.
struct DocumentAssets<'a> {
    sandbox: Option<&'a LuaSandbox>,
}

impl SkinAssets for DocumentAssets<'_> {
    fn image(&mut self, _path: &Path) -> Option<SkinImage> {
        SkinImage::new(TEX_SIZE, TEX_SIZE, vec![u8::MAX; (TEX_SIZE * TEX_SIZE) as usize * BYTES_PER_PIXEL])
    }

    fn expression(&mut self, source: &str) -> Option<LuaExprId> {
        self.sandbox?.compile(source).ok()
    }
}

/// The game state the fixtures read: two options the test switches, and the clock.
struct TestState {
    ready: bool,
    other: bool,
    now_ms: i64,
}

impl TestState {
    fn new() -> TestState {
        TestState { ready: false, other: false, now_ms: 0 }
    }
}

impl OffsetSource for TestState {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }
}

impl DrawStateSource for TestState {
    fn boolean(&self, id: i32) -> bool {
        match id {
            OPTION_READY => self.ready,
            OPTION_OTHER => self.other,
            _ => false,
        }
    }
}

impl SkinStateSource for TestState {
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
        self.now_ms
    }
}

/// One frame's evaluator, the same wrapper around [`LuaFrame`] the player builds.
struct Evaluator<'a> {
    frame: LuaFrame<'a>,
}

impl LuaDrawEval for Evaluator<'_> {
    fn eval_draw(&self, expr: LuaExprId) -> Option<bool> {
        self.frame.eval_draw(expr)
    }
}

impl SkinExprEval for Evaluator<'_> {
    fn eval_integer(&self, expr: LuaExprId) -> Option<i32> {
        self.frame.eval_int(expr)
    }

    fn eval_float(&self, expr: LuaExprId) -> Option<f32> {
        self.frame.eval_float(expr)
    }

    fn eval_text(&self, expr: LuaExprId) -> Option<String> {
        self.frame.eval_string(expr)
    }

    fn eval_timer(&self, expr: LuaExprId) -> Option<i64> {
        self.frame.eval_timer(expr)
    }

    fn run_action(&self, expr: LuaExprId) -> Vec<TimerRequest> {
        self.frame.run_action(expr)
    }
}

/// A document carrying one solid image, whatever the test declares beyond it, and the destinations
/// it hangs off.
fn write_document(scratch: &Scratch, members: &str, destination: &str) -> PathBuf {
    std::fs::write(scratch.root.join("sheet.tex"), "solid").expect("the source file is writable");
    let body = format!(
        r#"{{
            "type": 5, "name": "events fixture", "w": {DOC_W}, "h": {DOC_H},
            "source": [{{ "id": "sheet", "path": "sheet.tex" }}],
            {members}
            "destination": [{destination}]
        }}"#
    );
    let path = scratch.root.join("events.json");
    std::fs::write(&path, body).expect("the document is writable");
    path
}

/// One compiled document: the loaded skin, its screen and its events.
struct Compiled {
    skin: rbms_skin::loader::LoadedSkin,
    screen: SkinScreen,
    events: DocumentEvents,
    warnings: Vec<String>,
}

/// Loads and compiles one fixture exactly as a screen does, sandbox and all.
fn compile(scratch: &Scratch, canvas: &mut CpuCanvas, text: &mut TextContext, members: &str, destination: &str) -> Compiled {
    let document = write_document(scratch, members, destination);
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&document, options).expect("the generated document loads");
    let mut assets = DocumentAssets { sandbox: skin.lua() };
    let screen = SkinScreen::build(canvas, text, &skin, &mut assets);
    let mut warnings = Vec::new();
    let events = DocumentEvents::build(&skin.def, &mut assets, &mut warnings);
    Compiled { skin, screen, events, warnings }
}

impl Compiled {
    /// This document's sandbox bound to `state`, which is what a frame evaluates through.
    fn evaluator<'a>(&'a self, state: &'a TestState) -> Option<Evaluator<'a>> {
        self.skin.lua().map(|sandbox| Evaluator { frame: sandbox.frame(state) })
    }
}

/// One frame of the document's own timers and events.
fn event_frame<'a>(state: &'a TestState, lua: Option<&'a Evaluator<'a>>) -> SkinEventFrame<'a> {
    SkinEventFrame { now_ms: state.now_ms, state, lua: lua.map(|frame| frame as &dyn SkinExprEval) }
}

/// One drawn frame, with the pointer where the test put it.
fn draw_frame<'a>(state: &'a TestState, timers: &'a TimerState, lua: Option<&'a Evaluator<'a>>, mouse: Option<(f32, f32)>) -> SkinFrame<'a> {
    SkinFrame { now_ms: state.now_ms, timers, state, lua: lua.map(|frame| frame as &dyn SkinExprEval), mouse, background: None, extra: FrameExtra::None }
}

/// Draws one frame over a cleared canvas and answers how many objects reached it.
fn drawn(compiled: &Compiled, state: &TestState, timers: &TimerState, canvas: &mut CpuCanvas, text: &mut TextContext) -> usize {
    canvas.clear(Color::BLACK);
    let evaluator = compiled.evaluator(state);
    let frame = draw_frame(state, timers, evaluator.as_ref(), None);
    let mut ctx = RenderCtx::new(crate::theme::theme(), text);
    compiled.screen.draw(&mut ctx, canvas, &frame)
}

#[test]
fn a_custom_timer_expression_puts_its_object_on_the_screen() {
    let scratch = Scratch::new("timer");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = compile(
        &scratch,
        &mut canvas,
        &mut text,
        &format!(
            r#""image": [{{ "id": "flash", "src": "sheet" }}],
            "customTimers": [{{ "id": {TIMER_FLASH}, "timer": "skin.boolean({OPTION_READY}) and {TIMER_MOMENT} or nil" }}],"#
        ),
        &format!(r#"{{ "id": "flash", "timer": {TIMER_FLASH}, "dst": [{{ "time": 0, "x": 0, "y": 0, "w": {DOC_W}, "h": {DOC_H} }}] }}"#),
    );
    assert!(compiled.warnings.is_empty(), "the document's timer compiled: {:?}", compiled.warnings);

    let mut timers = TimerState::new();
    let mut state = TestState::new();
    state.now_ms = TIMER_MOMENT;

    {
        let evaluator = compiled.evaluator(&state);
        assert!(compiled.events.update(&mut timers, &event_frame(&state, evaluator.as_ref())).is_empty(), "a timer asks nothing of the player");
    }
    assert!(!timers.is_on(TimerId(TIMER_FLASH)), "the expression answered no moment, so the timer is off");
    assert_eq!(drawn(&compiled, &state, &timers, &mut canvas, &mut text), 0, "nothing hangs off a timer that is off");
    assert_eq!(canvas.pixel_at(DOC_W / 2, DOC_H / 2), Color::BLACK, "and the canvas is as clear as it was left");

    state.ready = true;
    {
        let evaluator = compiled.evaluator(&state);
        compiled.events.update(&mut timers, &event_frame(&state, evaluator.as_ref()));
    }
    assert_eq!(timers.get(TimerId(TIMER_FLASH)), Some(TIMER_MOMENT), "the timer switched on at the moment the expression named");
    assert_eq!(drawn(&compiled, &state, &timers, &mut canvas, &mut text), 1, "the object now animates");
    assert_eq!(canvas.pixel_at(DOC_W / 2, DOC_H / 2), SOLID, "and its first keyframe reached the pixels");
}

#[test]
fn a_custom_timer_numbered_as_a_built_in_one_is_refused() {
    let scratch = Scratch::new("shadow-timer");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = compile(
        &scratch,
        &mut canvas,
        &mut text,
        r#""image": [{ "id": "flash", "src": "sheet" }],
        "customTimers": [{ "id": 41, "timer": "1000" }],"#,
        r#"{ "id": "flash", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 8, "h": 8 }] }"#,
    );

    assert_eq!(compiled.warnings.len(), 1, "the timer was dropped with a word about why: {:?}", compiled.warnings);
    assert!(compiled.warnings[0].contains("41"), "the warning names the timer: {:?}", compiled.warnings[0]);

    let mut timers = TimerState::new();
    let state = TestState::new();
    let evaluator = compiled.evaluator(&state);
    compiled.events.update(&mut timers, &event_frame(&state, evaluator.as_ref()));
    assert!(!timers.is_on(TimerId(41)), "and the built-in timer of that number is left to the player");
}

#[test]
fn a_conditional_event_fires_once_and_again_only_after_its_interval() {
    let scratch = Scratch::new("interval");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = compile(
        &scratch,
        &mut canvas,
        &mut text,
        &format!(
            r#""image": [{{ "id": "flash", "src": "sheet" }}],
            "customEvents": [{{ "id": {EVENT_CUSTOM}, "action": {EVENT_SORT}, "condition": {OPTION_READY}, "minInterval": {MIN_INTERVAL_MS} }}],"#
        ),
        r#"{ "id": "flash", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 8, "h": 8 }] }"#,
    );
    assert!(compiled.warnings.is_empty(), "the event compiled: {:?}", compiled.warnings);

    let mut timers = TimerState::new();
    let mut state = TestState::new();
    let fire = |state: &TestState, timers: &mut TimerState| {
        let evaluator = compiled.evaluator(state);
        compiled.events.update(timers, &event_frame(state, evaluator.as_ref()))
    };

    assert!(fire(&state, &mut timers).is_empty(), "a condition that does not hold fires nothing");

    state.ready = true;
    let sort = vec![SkinEventRequest { id: EVENT_SORT, step: 0 }];
    assert_eq!(fire(&state, &mut timers), sort, "the first frame the condition holds fires, whatever the interval says");

    state.now_ms = i64::from(MIN_INTERVAL_MS) - 1;
    assert!(fire(&state, &mut timers).is_empty(), "and it stays quiet for the interval it named");

    state.now_ms = i64::from(MIN_INTERVAL_MS);
    assert_eq!(fire(&state, &mut timers), sort, "the interval measured from the last firing is what lets it fire again");

    state.now_ms += 1;
    assert!(fire(&state, &mut timers).is_empty(), "which restarts the wait");
}

#[test]
fn an_event_with_no_condition_waits_to_be_named() {
    let scratch = Scratch::new("named");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = compile(
        &scratch,
        &mut canvas,
        &mut text,
        &format!(
            r#""image": [{{ "id": "flash", "src": "sheet", "act": {EVENT_CUSTOM}, "click": 0 }}],
            "customEvents": [{{ "id": {EVENT_CUSTOM}, "action": "skin.set_timer({TIMER_PASSIVE})" }}],"#
        ),
        r#"{ "id": "flash", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 8, "h": 8 }] }"#,
    );
    assert!(compiled.warnings.is_empty(), "the event and the click compiled: {:?}", compiled.warnings);

    let mut timers = TimerState::new();
    let mut state = TestState::new();
    state.now_ms = TIMER_MOMENT;
    let evaluator = compiled.evaluator(&state);
    let frame = event_frame(&state, evaluator.as_ref());

    assert!(compiled.events.update(&mut timers, &frame).is_empty(), "nothing fires it by itself");
    assert!(!timers.is_on(TimerId(TIMER_PASSIVE)), "so its action never ran");

    let click = SkinEventClick { act: SkinRef::Id(EVENT_CUSTOM), step: 1 };
    assert!(compiled.events.click(click, &mut timers, &frame).is_empty(), "its action asks nothing of the player");
    assert_eq!(timers.get(TimerId(TIMER_PASSIVE)), Some(TIMER_MOMENT), "but it switched the document's own passive timer on, as of now");
}

#[test]
fn a_built_in_number_shadows_a_custom_event_of_the_same_number() {
    let scratch = Scratch::new("shadow-event");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = compile(
        &scratch,
        &mut canvas,
        &mut text,
        &format!(
            r#""image": [{{ "id": "flash", "src": "sheet", "act": {EVENT_SORT}, "click": 0 }}],
            "customEvents": [{{ "id": {EVENT_SORT}, "action": "skin.set_timer({TIMER_PASSIVE})", "condition": {OPTION_READY} }}],"#
        ),
        r#"{ "id": "flash", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 8, "h": 8 }] }"#,
    );
    assert_eq!(compiled.warnings.len(), 1, "the document was told its event is unreachable: {:?}", compiled.warnings);

    let mut timers = TimerState::new();
    let mut state = TestState::new();
    state.ready = true;
    let evaluator = compiled.evaluator(&state);
    let frame = event_frame(&state, evaluator.as_ref());

    assert!(compiled.events.update(&mut timers, &frame).is_empty(), "the shadowed event is not even updated");
    let click = SkinEventClick { act: SkinRef::Id(EVENT_SORT), step: 1 };
    assert_eq!(
        compiled.events.click(click, &mut timers, &frame),
        vec![SkinEventRequest { id: EVENT_SORT, step: 1 }],
        "the click reaches the player's own event"
    );
    assert!(!timers.is_on(TimerId(TIMER_PASSIVE)), "and the document's action never ran");
}

#[test]
fn the_reference_numbering_decides_which_events_are_the_players() {
    assert!(is_builtin_event(EVENT_SORT), "the browser's sort order is one of the reference's own");
    assert!(is_builtin_event(101) && is_builtin_event(164), "so is every key assignment, across both of its bands");
    assert!(is_builtin_event(370) && is_builtin_event(385), "and every practice row");
    assert!(!is_builtin_event(140), "the gap between the two key bands is not");
    assert!(!is_builtin_event(EVENT_CUSTOM), "and a number outside the table is the document's to define");
}

/// The fixture every click test shares: two overlapping objects, the second drawn on top of the
/// first, each with an `act` of its own.
fn click_fixture(scratch: &Scratch, canvas: &mut CpuCanvas, text: &mut TextContext, click: i32) -> Compiled {
    compile(
        scratch,
        canvas,
        text,
        &format!(
            r#""image": [
                {{ "id": "under", "src": "sheet", "act": {EVENT_SORT}, "click": {click} }},
                {{ "id": "over", "src": "sheet", "act": {EVENT_CUSTOM}, "click": {click} }},
                {{ "id": "hidden", "src": "sheet", "act": {EVENT_SORT}, "click": {click} }}
            ],
            "customEvents": [{{ "id": {EVENT_CUSTOM}, "action": "skin.set_timer({TIMER_PASSIVE})" }}],"#
        ),
        &format!(
            r#"{{ "id": "hidden", "op": [{OPTION_OTHER}], "dst": [{{ "time": 0, "x": 0, "y": 0, "w": {DOC_W}, "h": {DOC_H} }}] }},
            {{ "id": "under", "dst": [{{ "time": 0, "x": 0, "y": 0, "w": {DOC_W}, "h": {DOC_H} }}] }},
            {{ "id": "over", "dst": [{{ "time": 0, "x": 0, "y": 0, "w": 32, "h": {DOC_H} }}] }}"#
        ),
    )
}

/// The event rectangles one frame offers, placed on a canvas the size the document was authored at.
fn spots(compiled: &Compiled, state: &TestState, timers: &TimerState) -> Vec<SkinHotspot> {
    let evaluator = compiled.evaluator(state);
    let frame = draw_frame(state, timers, evaluator.as_ref(), None);
    compiled.screen.event_hotspots_on_screen(&frame, (DOC_W, DOC_H), &compiled.events)
}

/// The act one rectangle carries.
fn act_of(spot: &SkinHotspot) -> SkinEventClick {
    match spot.action {
        SkinHotAction::Event(click) => click,
        other => panic!("an `act` object offered {other:?} rather than an event"),
    }
}

#[test]
fn a_click_rectangle_is_offered_only_where_its_object_was_drawn() {
    let scratch = Scratch::new("click");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = click_fixture(&scratch, &mut canvas, &mut text, 0);
    assert_eq!(compiled.events.click_count(), 3, "every object with an `act` offers one");

    let timers = TimerState::new();
    let state = TestState::new();
    let offered = spots(&compiled, &state, &timers);

    assert_eq!(offered.len(), 2, "the object whose draw condition fails is not clickable: {offered:?}");
    assert_eq!(act_of(&offered[0]).act, SkinRef::Id(EVENT_SORT), "the lower object is offered first");
    assert_eq!(
        act_of(&offered[1]).act,
        SkinRef::Id(EVENT_CUSTOM),
        "and the one drawn over it last, so a caller testing its last match first tests the topmost"
    );
    assert_eq!(offered[0].rect, Rect::new(0.0, 0.0, DOC_W as f32, DOC_H as f32), "a rectangle is placed on the canvas the frame was drawn on");
    assert_eq!(offered[1].rect, Rect::new(0.0, 0.0, 32.0, DOC_H as f32));
    assert_eq!(act_of(&offered[0]).step, 1, "click 0 steps forward wherever it is clicked");
}

#[test]
fn a_click_of_one_turns_the_step_around() {
    let scratch = Scratch::new("click-back");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = click_fixture(&scratch, &mut canvas, &mut text, 1);

    let offered = spots(&compiled, &TestState::new(), &TimerState::new());
    assert_eq!(offered.len(), 2, "one rectangle each, as click 0 gives");
    assert_eq!(act_of(&offered[0]).step, -1, "and the whole of it steps back");
}

#[test]
fn a_click_of_two_splits_the_rectangle_left_and_right() {
    let scratch = Scratch::new("click-halves");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = click_fixture(&scratch, &mut canvas, &mut text, 2);

    let offered = spots(&compiled, &TestState::new(), &TimerState::new());
    assert_eq!(offered.len(), 4, "each drawn object offers a half each");
    let half = DOC_W as f32 / 2.0;
    assert_eq!(offered[0].rect, Rect::new(0.0, 0.0, half, DOC_H as f32), "the left half comes first");
    assert_eq!(act_of(&offered[0]).step, -1, "and steps back");
    assert_eq!(offered[1].rect, Rect::new(half, 0.0, half, DOC_H as f32), "the right half is the rest of it");
    assert_eq!(act_of(&offered[1]).step, 1, "and steps forward");
}

#[test]
fn a_click_of_three_splits_the_rectangle_bottom_and_top() {
    let scratch = Scratch::new("click-rows");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = click_fixture(&scratch, &mut canvas, &mut text, 3);

    let offered = spots(&compiled, &TestState::new(), &TimerState::new());
    assert_eq!(offered.len(), 4);
    let half = DOC_H as f32 / 2.0;
    assert_eq!(act_of(&offered[0]).step, -1, "the document's lower half steps back");
    assert_eq!(offered[0].rect, Rect::new(0.0, half, DOC_W as f32, half), "and lands on the lower half of the canvas, because the axis is flipped");
    assert_eq!(act_of(&offered[1]).step, 1, "its upper half steps forward");
    assert_eq!(offered[1].rect, Rect::new(0.0, 0.0, DOC_W as f32, half));
}

#[test]
fn a_click_value_this_build_has_no_meaning_for_offers_nothing() {
    let scratch = Scratch::new("click-unknown");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = click_fixture(&scratch, &mut canvas, &mut text, 9);

    assert_eq!(compiled.events.click_count(), 0, "no object was given a rectangle");
    assert_eq!(compiled.warnings.len(), 3, "and each was warned about: {:?}", compiled.warnings);
    assert!(spots(&compiled, &TestState::new(), &TimerState::new()).is_empty());
}

#[test]
fn an_object_gated_on_the_pointer_is_clickable_only_under_it() {
    let scratch = Scratch::new("mouse-rect");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = compile(
        &scratch,
        &mut canvas,
        &mut text,
        &format!(r#""image": [{{ "id": "flash", "src": "sheet", "act": {EVENT_SORT}, "click": 0 }}],"#),
        r#"{ "id": "flash", "mouseRect": { "x": 0, "y": 0, "w": 8, "h": 8 },
           "dst": [{ "time": 0, "x": 10, "y": 10, "w": 20, "h": 10 }] }"#,
    );

    let timers = TimerState::new();
    let state = TestState::new();
    let offered = |mouse: Option<(f32, f32)>| {
        let evaluator = compiled.evaluator(&state);
        let frame = draw_frame(&state, &timers, evaluator.as_ref(), mouse);
        compiled.screen.event_hotspots_on_screen(&frame, (DOC_W, DOC_H), &compiled.events)
    };

    assert!(offered(None).is_empty(), "a pointer rectangle with no pointer draws nothing, so there is nothing to click");
    assert!(offered(Some((30.0, 30.0))).is_empty(), "nor while the pointer is outside it");
    assert_eq!(offered(Some((14.0, 14.0))).len(), 1, "and the object is clickable exactly while it is drawn");
}

#[test]
fn a_document_that_declares_none_of_the_three_has_nothing_to_run() {
    let scratch = Scratch::new("bare");
    let mut canvas = CpuCanvas::new(DOC_W, DOC_H);
    let mut text = TextContext::embedded_only();
    let compiled = compile(
        &scratch,
        &mut canvas,
        &mut text,
        r#""image": [{ "id": "flash", "src": "sheet" }],"#,
        r#"{ "id": "flash", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 8, "h": 8 }] }"#,
    );

    assert!(compiled.events.is_empty(), "nothing was compiled from a document that declared nothing");
    assert!(spots(&compiled, &TestState::new(), &TimerState::new()).is_empty(), "and no rectangle is offered, whatever it drew");
}
