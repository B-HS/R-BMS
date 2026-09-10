//! The Lua sandbox: that the whitelisted API is all a document can reach, that a runaway expression
//! is cut off rather than hanging the frame, and that a broken one hides its object instead of
//! failing the skin.

#![cfg(feature = "lua")]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rbms_skin::SkinError;
use rbms_skin::dst::{DrawStateSource, LuaDrawEval, OffsetSource, SkinOffset};
use rbms_skin::loader::Budget;
use rbms_skin::lua::{LuaSandbox, SkinStateSource};

/// Every global the sandbox removes before a document is compiled.
const FORBIDDEN: &[&str] = &[
    "dofile",
    "loadfile",
    "load",
    "loadstring",
    "require",
    "collectgarbage",
    "rawset",
    "rawget",
    "rawequal",
    "rawlen",
    "setmetatable",
    "getmetatable",
    "newproxy",
    "print",
    "io",
    "os",
    "package",
    "debug",
];

/// A skin root for the tests that do not touch the filesystem.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("minimal")
}

/// A sandbox with the shipping budget.
fn sandbox() -> LuaSandbox {
    LuaSandbox::new(&root(), Budget::default()).expect("the sandbox should build")
}

/// A hand-built game state, standing in for the property registry.
#[derive(Debug, Default)]
struct FakeState {
    booleans: BTreeSet<i32>,
    integers: BTreeMap<i32, i32>,
    floats: BTreeMap<i32, f32>,
    strings: BTreeMap<i32, String>,
    timers: BTreeMap<i32, i64>,
    offsets: BTreeMap<i32, SkinOffset>,
    now: i64,
}

impl OffsetSource for FakeState {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.offsets.get(&id).copied()
    }
}

impl DrawStateSource for FakeState {
    fn boolean(&self, id: i32) -> bool {
        if id < 0 { !self.booleans.contains(&-id) } else { self.booleans.contains(&id) }
    }
}

impl SkinStateSource for FakeState {
    fn integer(&self, id: i32) -> i32 {
        self.integers.get(&id).copied().unwrap_or_default()
    }

    fn float(&self, id: i32) -> f32 {
        self.floats.get(&id).copied().unwrap_or_default()
    }

    fn string(&self, id: i32) -> &str {
        self.strings.get(&id).map(String::as_str).unwrap_or_default()
    }

    fn timer(&self, id: i32) -> Option<i64> {
        self.timers.get(&id).copied()
    }

    fn now_ms(&self) -> i64 {
        self.now
    }
}

#[test]
fn every_forbidden_global_is_gone() {
    let sandbox = sandbox();
    let state = FakeState::default();
    for name in FORBIDDEN {
        let present = sandbox.eval_bool(&format!("{name} ~= nil"), &state).expect("the check itself should run");
        assert!(!present, "{name} must not be reachable from a skin expression");
    }
}

#[test]
fn the_libraries_a_skin_may_use_are_still_there() {
    let sandbox = sandbox();
    let state = FakeState::default();
    for name in ["math", "string", "table"] {
        assert!(sandbox.eval_bool(&format!("{name} ~= nil"), &state).expect("the check should run"), "{name} should be available");
    }
}

#[test]
fn a_skin_cannot_open_a_file() {
    let sandbox = sandbox();
    let state = FakeState::default();
    let outcome = sandbox.eval_bool(r#"io.open("/etc/passwd") ~= nil"#, &state);
    assert!(matches!(outcome, Err(SkinError::Lua { .. })), "reaching for a file must fail, got {outcome:?}");
}

#[test]
fn a_skin_cannot_load_more_code_at_runtime() {
    let sandbox = sandbox();
    let state = FakeState::default();
    assert!(matches!(sandbox.eval_bool(r#"load("return 1")()"#, &state), Err(SkinError::Lua { .. })));
}

#[test]
fn an_endless_loop_is_cut_off_by_the_instruction_budget() {
    let sandbox = LuaSandbox::new(&root(), Budget { max_instructions: 20_000, ..Budget::default() }).expect("the sandbox should build");
    let state = FakeState::default();
    let outcome = sandbox.eval_bool("(function() while true do end end)()", &state);
    assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "got {outcome:?}");
}

#[test]
fn the_instruction_budget_is_restored_between_expressions() {
    let sandbox = LuaSandbox::new(&root(), Budget { max_instructions: 60_000, ..Budget::default() }).expect("the sandbox should build");
    let state = FakeState::default();
    for _ in 0..4 {
        let value = sandbox.eval_int("(function() local total = 0 for i = 1, 500 do total = total + i end return total end)()", &state);
        assert_eq!(value.expect("a short loop fits the budget"), 125_250);
    }
}

#[test]
fn a_runaway_allocation_is_cut_off_by_the_memory_budget() {
    let sandbox =
        LuaSandbox::new(&root(), Budget { max_instructions: u32::MAX, max_memory_bytes: 512 * 1024, ..Budget::default() }).expect("the sandbox should build");
    let state = FakeState::default();
    let outcome = sandbox.eval_bool("(function() local t = {} local i = 1 while true do t[i] = string.rep('x', 1024) i = i + 1 end end)()", &state);
    assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "got {outcome:?}");
}

#[test]
fn randomness_is_pinned_so_two_sandboxes_agree() {
    let state = FakeState::default();
    let first = sandbox().eval_float("math.random()", &state).expect("random should run");
    let second = sandbox().eval_float("math.random()", &state).expect("random should run");
    assert_eq!(first, second, "a document that reaches for randomness must still draw the same picture");
}

#[test]
fn the_whitelisted_api_reads_the_state_it_is_given() {
    let sandbox = sandbox();
    let mut state = FakeState { now: 4_200, ..FakeState::default() };
    state.booleans.insert(901);
    state.integers.insert(10, 37);
    state.floats.insert(110, 0.25);
    state.strings.insert(300, "title".to_owned());
    state.timers.insert(41, 1_500);

    assert!(sandbox.eval_bool("skin.boolean(901)", &state).expect("boolean should run"));
    assert_eq!(sandbox.eval_int("skin.number(10)", &state).expect("number should run"), 37);
    assert_eq!(sandbox.eval_float("skin.float(110)", &state).expect("float should run"), 0.25);
    assert_eq!(sandbox.eval_string("skin.text(300)", &state).expect("text should run"), "title");
    assert_eq!(sandbox.eval_int("skin.timer(41)", &state).expect("timer should run"), 1_500);
    assert_eq!(sandbox.eval_int("skin.time()", &state).expect("time should run"), 4_200);
}

#[test]
fn an_unset_timer_reads_as_nil_rather_than_zero() {
    let sandbox = sandbox();
    let state = FakeState::default();
    assert!(sandbox.eval_bool("skin.timer(41) == nil", &state).expect("the check should run"));
}

#[test]
fn a_negative_option_id_is_negated_by_the_state_source() {
    let sandbox = sandbox();
    let mut state = FakeState::default();
    state.booleans.insert(901);
    assert!(!sandbox.eval_bool("skin.boolean(-901)", &state).expect("the check should run"));
    assert!(sandbox.eval_bool("skin.boolean(-902)", &state).expect("the check should run"));
}

#[test]
fn an_unimplemented_id_reads_as_a_default_rather_than_failing() {
    let sandbox = sandbox();
    let state = FakeState::default();
    assert!(!sandbox.eval_bool("skin.boolean(12345)", &state).expect("the check should run"));
    assert_eq!(sandbox.eval_int("skin.number(12345)", &state).expect("the check should run"), 0);
    assert_eq!(sandbox.eval_string("skin.text(12345)", &state).expect("the check should run"), "");
}

#[test]
fn truth_follows_lua_rather_than_rust() {
    let sandbox = sandbox();
    let state = FakeState::default();
    assert!(sandbox.eval_bool("0", &state).expect("zero should run"), "zero is true in Lua");
    assert!(!sandbox.eval_bool("nil", &state).expect("nil should run"));
    assert!(!sandbox.eval_bool("false", &state).expect("false should run"));
    assert!(sandbox.eval_bool(r#""""#, &state).expect("the empty string should run"), "the empty string is true in Lua");
}

#[test]
fn the_same_source_compiles_once() {
    let sandbox = sandbox();
    let first = sandbox.compile("skin.number(10) > 0").expect("compiles");
    let second = sandbox.compile("skin.number(10) > 0").expect("compiles");
    assert_eq!(first, second);
    assert_eq!(sandbox.compiled_count(), 1);
}

#[test]
fn a_source_that_will_not_compile_is_reported_with_its_text() {
    let sandbox = sandbox();
    let outcome = sandbox.compile("this is not lua at all ===");
    match outcome {
        Err(SkinError::Lua { expr, .. }) => assert_eq!(expr, "this is not lua at all ==="),
        other => panic!("a broken expression should report itself, got {other:?}"),
    }
}

#[test]
fn a_whole_chunk_compiles_when_the_expression_form_does_not() {
    let sandbox = sandbox();
    let state = FakeState::default();
    assert_eq!(sandbox.eval_int("local total = 1 + 2 return total", &state).expect("a chunk should run"), 3);
}

#[test]
fn a_result_of_the_wrong_shape_is_reported() {
    let sandbox = sandbox();
    let state = FakeState::default();
    assert!(matches!(sandbox.eval_int("{}", &state), Err(SkinError::Lua { .. })));
}

#[test]
fn a_frame_answers_draw_conditions_and_counts_them() {
    let sandbox = sandbox();
    let mut state = FakeState::default();
    state.integers.insert(10, 1);
    let expr = sandbox.compile("skin.number(10) > 0").expect("compiles");

    let frame = sandbox.frame(&state);
    assert_eq!(frame.eval_draw(expr), Some(true));
    assert_eq!(frame.eval_draw(expr), Some(true));
    assert_eq!(frame.calls(), 2);
}

#[test]
fn a_failing_expression_hides_its_object_instead_of_failing_the_frame() {
    let sandbox = sandbox();
    let state = FakeState::default();
    let expr = sandbox.compile("error('boom')").expect("compiles");
    assert_eq!(sandbox.frame(&state).eval_draw(expr), None);
}

#[test]
fn a_frame_stops_evaluating_once_it_passes_its_call_budget() {
    let sandbox = LuaSandbox::new(&root(), Budget { max_calls_per_frame: 2, ..Budget::default() }).expect("the sandbox should build");
    let state = FakeState::default();
    let expr = sandbox.compile("true").expect("compiles");
    let frame = sandbox.frame(&state);

    assert_eq!(frame.eval_draw(expr), Some(true));
    assert_eq!(frame.eval_draw(expr), Some(true));
    assert_eq!(frame.eval_draw(expr), None, "the third evaluation is past the frame budget");
    assert_eq!(frame.calls(), 2);
}

#[test]
fn a_new_frame_starts_its_call_budget_over() {
    let sandbox = LuaSandbox::new(&root(), Budget { max_calls_per_frame: 1, ..Budget::default() }).expect("the sandbox should build");
    let state = FakeState::default();
    let expr = sandbox.compile("true").expect("compiles");

    assert_eq!(sandbox.frame(&state).eval_draw(expr), Some(true));
    assert_eq!(sandbox.frame(&state).eval_draw(expr), Some(true));
}

#[test]
fn the_sandbox_remembers_the_root_it_was_built_for() {
    let sandbox = sandbox();
    assert_eq!(sandbox.root(), root());
    assert_eq!(sandbox.budget(), Budget::default());
}

/// The `string` functions the sandbox removes, and why.
const FORBIDDEN_STRING: &[&str] = &["dump", "find", "gmatch", "gsub", "match"];

/// `string.dump` hands out bytecode, and Lua's own unloader does next to no validation of a chunk it
/// is given back, so the two belong together: no way to produce bytecode, and no way to load it.
#[test]
fn a_skin_cannot_produce_or_load_bytecode() {
    let sandbox = sandbox();
    let state = FakeState::default();
    for name in FORBIDDEN_STRING {
        let source = format!("string.{name} == nil");
        assert_eq!(sandbox.eval_bool(&source, &state).ok(), Some(true), "string.{name} is still reachable");
    }

    let binary = "\u{1b}Lua\u{51}\u{0}";
    let Err(SkinError::Lua { message, .. }) = sandbox.compile(binary) else {
        panic!("a chunk that starts with the bytecode signature must be refused");
    };
    assert!(message.contains("binary chunk"), "it must be refused for being bytecode, not for happening to be malformed: {message}");
}

/// Pattern matching runs entirely inside C, where the instruction hook never fires and the allocator
/// is never asked for anything, so a pattern with several `.-` captures over a long subject runs for
/// as long as it likes with neither budget noticing. The expression below took seconds before those
/// functions were taken out of the sandbox, on a frame loop that has sixteen milliseconds.
#[test]
fn a_backtracking_pattern_cannot_run_at_all_let_alone_forever() {
    let sandbox = sandbox();
    let state = FakeState::default();
    let started = std::time::Instant::now();
    let outcome = sandbox.eval_bool(r#"string.find(string.rep("a", 120), "^(.-)(.-)(.-)(.-)(.-)b$") ~= nil"#, &state);
    let spent = started.elapsed();

    assert!(matches!(outcome, Err(SkinError::Lua { .. })), "got {outcome:?}");
    assert!(spent < std::time::Duration::from_millis(200), "the expression ran for {spent:?}");
}

/// The formatting and slicing a document actually reaches for is still there: taking the pattern
/// functions out is not taking the library out.
#[test]
fn the_string_functions_a_document_needs_are_still_there() {
    let sandbox = sandbox();
    let state = FakeState::default();
    assert_eq!(sandbox.eval_string(r#"string.format("%02d", 7)"#, &state).ok(), Some("07".to_owned()));
    assert_eq!(sandbox.eval_string(r#"("rbms"):sub(1, 2):upper()"#, &state).ok(), Some("RB".to_owned()));
}

/// The frame budget is what stops a document's expressions from running the frame loop off the road,
/// so it has to cover every read a document makes -- not only the draw conditions. A `value`, a
/// `floatvalue` and a `text` field each take an expression too, and an object is far more likely to
/// have one of those than a gate.
#[test]
fn the_frame_budget_covers_value_reads_as_well_as_draw_conditions() {
    let sandbox = LuaSandbox::new(&root(), Budget { max_calls_per_frame: 3, ..Budget::default() }).expect("the sandbox should build");
    let state = FakeState::default();
    let number = sandbox.compile("7").expect("compiles");
    let text = sandbox.compile("'x'").expect("compiles");
    let frame = sandbox.frame(&state);

    assert_eq!(frame.eval_int(number), Some(7));
    assert_eq!(frame.eval_float(number), Some(7.0));
    assert_eq!(frame.eval_string(text), Some("x".to_owned()));
    assert_eq!(frame.calls(), 3, "each read spends one of the frame's evaluations");

    assert_eq!(frame.eval_int(number), None, "past the budget a value read falls back to its default");
    assert_eq!(frame.eval_draw(number), None, "and so does a draw condition, out of the same allowance");
}

/// Time is the other half of the same budget: a frame that has spent its whole Lua allowance stops
/// evaluating, however few evaluations that took.
#[test]
fn a_frame_that_spends_its_whole_time_allowance_stops_evaluating() {
    let sandbox = LuaSandbox::new(&root(), Budget { max_frame_micros: 0, ..Budget::default() }).expect("the sandbox should build");
    let state = FakeState::default();
    let expr = sandbox.compile("true").expect("compiles");
    let frame = sandbox.frame(&state);

    assert_eq!(frame.eval_draw(expr), None, "a frame with no time left evaluates nothing");
    assert_eq!(frame.calls(), 0);
}

/// A wall clock catches what an instruction count cannot. The count is restored per call and the
/// clock is too, so an expression that fits comfortably still runs on every frame.
#[test]
fn an_expression_is_held_to_a_wall_clock_as_well_as_an_instruction_count() {
    let sandbox = LuaSandbox::new(&root(), Budget { max_instructions: u32::MAX, max_call_micros: 1, ..Budget::default() }).expect("the sandbox builds");
    let state = FakeState::default();
    let outcome = sandbox.eval_int("(function() local total = 0 for i = 1, 20000000 do total = total + i end return total end)()", &state);
    assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "got {outcome:?}");

    let quick = sandbox.eval_int("1 + 1", &state);
    assert_eq!(quick.ok(), Some(2), "the clock starts over for the next expression");
}
