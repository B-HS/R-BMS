//! The scripts a JSON document writes as strings.
//!
//! A document may put Lua source wherever it may put a property id. The loader compiles each one
//! into the interpreter the loaded skin owns -- the same runtime a Lua skin is loaded into, with the
//! game state published as globals the way the reference publishes it for a JSON skin -- and a
//! frame calls them through its binding. These tests cover that path end to end: what a script can
//! see, how its result is read, and that the runtime's containment holds for a script as it does
//! for a Lua skin (`skin_lua_env.rs` covers the runtime itself).

#![cfg(feature = "lua")]

use std::path::{Path, PathBuf};

use rbms_model::Mode;
use rbms_skin::dst::LuaFnId;
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, load_skin_with_host};
use rbms_skin::lua::{LuaFnKind, SkinLua, SkinLuaConfig};
use rbms_skin::model::{EventRef, PropertyRef};
use rbms_skin::property::{DefaultState, INTEGER_ABSENT, MapHost, SkinHost};
use rbms_skin::timer::TIMER_OFF;

/// A seed every load in this file pins.
const TEST_SEED: u64 = 7;

/// An integer id the host answers for.
const SAMPLE_NUMBER: i32 = 71;

/// The value stored under [`SAMPLE_NUMBER`].
const SAMPLE_NUMBER_VALUE: i32 = 1_234;

/// A text id the host answers for.
const SAMPLE_TEXT: i32 = 10;

/// A timer id the host reports as running.
const RUNNING_TIMER: i32 = 41;

/// The microsecond [`RUNNING_TIMER`] switched on.
const RUNNING_SINCE_US: i64 = 5_000_000;

/// The scene clock of the host.
const NOW_US: i64 = 9_000_000;

/// The option the customisation row of [`CONFIGURED_DOCUMENT`] is on by default.
const DEFAULT_OPTION: i32 = 901;

/// The offset id [`CONFIGURED_DOCUMENT`] declares.
const DECLARED_OFFSET: i32 = 40;

/// A scratch directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-script-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Writes `text` to `relative` under the scratch directory, creating its parents.
    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("a scratch file has a parent")).expect("the parent should be creatable");
        std::fs::write(&path, text).expect("the scratch file should be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A host that answers the sample ids.
fn host() -> MapHost {
    let mut host = MapHost::new();
    host.integers.insert(SAMPLE_NUMBER, SAMPLE_NUMBER_VALUE);
    host.texts.insert(SAMPLE_TEXT, "ALBIDA".to_owned());
    host.timers.insert(RUNNING_TIMER, RUNNING_SINCE_US);
    host.now_us = NOW_US;
    host
}

/// Loads `document` from a scratch folder of its own, with `host` behind it.
fn load(scratch: &Scratch, document: &str, host: &dyn SkinHost) -> LoadedSkin {
    let path = scratch.write("skin.json", document);
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(TEST_SEED), ..SkinLoadOptions::new(scratch.path(), &user, Mode::BEAT_7K) };
    load_skin_with_host(&path, options, host).unwrap_or_else(|error| panic!("the document should load: {error}"))
}

/// A music select document with nothing in it, for the tests that compile their own scripts.
fn empty(scratch: &Scratch) -> LoadedSkin {
    load(scratch, r#"{ "type": 5 }"#, &DefaultState)
}

/// The interpreter a loaded document owns.
fn runtime(skin: &LoadedSkin) -> &SkinLua {
    skin.runtime().expect("a build with Lua gives a document an interpreter")
}

/// The function a property field was compiled into.
fn function(property: &Option<PropertyRef>) -> LuaFnId {
    property.as_ref().and_then(PropertyRef::function).unwrap_or_else(|| panic!("the field should hold a compiled script, got {property:?}"))
}

/// Compiles `source` as a field of `kind` in `skin`'s interpreter.
fn compile(skin: &LoadedSkin, source: &str, kind: LuaFnKind) -> LuaFnId {
    runtime(skin).compile(source, kind).unwrap_or_else(|error| panic!("{source} should compile: {error}"))
}

#[test]
fn a_document_finds_the_game_state_in_its_globals() {
    let scratch = Scratch::new("globals");
    let skin = empty(&scratch);
    let lua = runtime(&skin).lua();

    for name in ["option", "number", "float_number", "text", "timer", "time", "event_exec", "is_timer_on", "timer_observe_boolean", "event_observe_turn_true"] {
        let kind: String = lua.load(format!("return type({name})")).eval().expect("the check runs");
        assert_eq!(kind, "function", "{name} is a global of a document's interpreter");
    }
    let same: bool =
        lua.load("return option == require('main_state').option and is_timer_on == require('timer_util').is_timer_on").eval().expect("the check runs");
    assert!(same, "the globals are the modules' own members, so the modules are still there to be required");
}

#[test]
fn a_lua_skin_is_not_given_the_globals() {
    let scratch = Scratch::new("no-globals");
    let runtime = SkinLua::new(SkinLuaConfig { seed: Some(TEST_SEED), ..SkinLuaConfig::new(scratch.path()) }).expect("the runtime builds");
    let absent: bool = runtime.lua().load("return option == nil and number == nil and is_timer_on == nil").eval().expect("the check runs");
    assert!(absent, "a Lua skin requires the modules; nothing is exported for it");
}

#[test]
fn a_script_reads_the_host_the_frame_is_bound_to() {
    let scratch = Scratch::new("reads");
    let host = host();
    let document = format!(
        r#"{{ "type": 5,
            "value": [{{ "id": "count", "src": "0", "value": "number({SAMPLE_NUMBER}) + 1" }}],
            "text": [{{ "id": "title", "font": "0", "value": "text({SAMPLE_TEXT}) .. '!'" }}],
            "destination": [{{ "id": "count", "draw": "is_timer_on(timer({RUNNING_TIMER})) and time() > timer({RUNNING_TIMER})", "dst": [{{ "time": 0 }}] }}] }}"#
    );
    let skin = load(&scratch, &document, &host);
    assert_eq!(skin.warnings, Vec::<String>::new());
    let count = function(&skin.def.value[0].value);
    let title = function(&skin.def.text[0].value);
    let gate = function(&skin.def.destination[0].draw);

    let (number, text, drawn) =
        runtime(&skin).frame(&host, |frame| (frame.call_integer(count), frame.call_text(title), frame.call_boolean(gate))).expect("the frame binds");
    assert_eq!(number, SAMPLE_NUMBER_VALUE + 1);
    assert_eq!(text, "ALBIDA!");
    assert!(drawn, "the timer is on and the clock is past the moment it switched on");

    let silent =
        runtime(&skin).frame(&DefaultState, |frame| (frame.call_integer(count), frame.call_text(title), frame.call_boolean(gate))).expect("the frame binds");
    assert_eq!(silent, (INTEGER_ABSENT + 1, "!".to_owned(), false), "the same scripts against a host that knows nothing");
}

#[test]
fn a_script_result_is_read_by_the_rule_of_its_field() {
    let scratch = Scratch::new("coercion");
    let skin = empty(&scratch);
    let zero = compile(&skin, "0", LuaFnKind::Boolean);
    let nothing = compile(&skin, "nil", LuaFnKind::Boolean);
    let quotient = compile(&skin, "7 / 2", LuaFnKind::Integer);
    let whole = compile(&skin, "10 / 2", LuaFnKind::Text);
    let share = compile(&skin, "'0.25'", LuaFnKind::Float);

    runtime(&skin)
        .frame(&DefaultState, |frame| {
            assert!(frame.call_boolean(zero), "truth is Lua's: zero is true");
            assert!(!frame.call_boolean(nothing));
            assert_eq!(frame.call_integer(quotient), 3, "an integer field truncates towards zero");
            assert_eq!(frame.call_text(whole), "5", "a whole quotient is written without a fraction");
            assert_eq!(frame.call_float(share), 0.25, "a string that spells a number is that number");
        })
        .expect("the frame binds");
}

#[test]
fn a_timer_script_is_tried_once_and_may_hand_over_a_function() {
    let scratch = Scratch::new("timers");
    let host = host();
    let document = format!(
        r#"{{ "type": 5, "image": [
            {{ "id": "plain", "src": "0", "timer": "timer({RUNNING_TIMER})" }},
            {{ "id": "made", "src": "0", "timer": "timer_function({RUNNING_TIMER})" }},
            {{ "id": "counted", "src": "0", "timer": "(function() tried = (tried or 0) + 1 return 7 end)()" }}
        ] }}"#
    );
    let skin = load(&scratch, &document, &host);
    assert_eq!(skin.warnings, Vec::<String>::new());
    let plain = function(&skin.def.image[0].timer);
    let made = function(&skin.def.image[1].timer);
    let counted = function(&skin.def.image[2].timer);

    let tried: i32 = runtime(&skin).lua().load("return tried").eval().expect("the count reads");
    assert_eq!(tried, 1, "the chunk was called exactly once while the document loaded");

    let (plain, made, counted) =
        runtime(&skin).frame(&host, |frame| (frame.call_timer(plain), frame.call_timer(made), frame.call_timer(counted))).expect("the frame binds");
    assert_eq!(plain, RUNNING_SINCE_US, "a chunk that yields a number is itself the timer");
    assert_eq!(made, RUNNING_SINCE_US, "a chunk that yields a function hands that function over");
    assert_eq!(counted, 7);

    let off = runtime(&skin).frame(&DefaultState, |frame| frame.call_timer(plain_of(&skin))).expect("the frame binds");
    assert_eq!(off, TIMER_OFF, "against a host with no such timer the same script reports it off");
}

/// The first image's timer function, for the one test that reads it twice.
fn plain_of(skin: &LoadedSkin) -> LuaFnId {
    function(&skin.def.image[0].timer)
}

#[test]
fn an_event_script_is_compiled_as_it_stands_and_reaches_the_host() {
    let scratch = Scratch::new("events");
    let host = host();
    let document = r#"{ "type": 5, "image": [
        { "id": "counter", "src": "0", "act": "pressed = (pressed or 0) + 1" },
        { "id": "button", "src": "0", "act": "event_exec(13)" }
    ] }"#;
    let skin = load(&scratch, document, &host);
    assert_eq!(skin.warnings, Vec::<String>::new(), "a statement is a whole chunk: no `return` is put in front of an event");
    let (Some(EventRef::Lua(counter)), Some(EventRef::Lua(button))) = (skin.def.image[0].act.clone(), skin.def.image[1].act.clone()) else {
        panic!("both events should be compiled: {:?}", skin.def.image);
    };

    runtime(&skin)
        .frame(&host, |frame| {
            frame.call_event(counter, 0);
            frame.call_event(counter, 0);
            frame.call_event(button, 0);
        })
        .expect("the frame binds");
    let pressed: i32 = runtime(&skin).lua().load("return pressed").eval().expect("the count reads");
    assert_eq!(pressed, 2);
    assert_eq!(host.calls().len(), 1, "the host was asked to run one event: {:?}", host.calls());
}

/// A document with one customisation row, one file slot and one offset of its own.
const CONFIGURED_DOCUMENT: &str = r#"{
    "type": 5,
    "property": [{ "name": "Panel", "item": [{ "name": "on", "op": 901 }, { "name": "off", "op": 902 }], "def": "on" }],
    "filepath": [{ "name": "Gauge", "path": "gauge/*.png", "def": "hard" }],
    "offset": [{ "name": "Shift", "id": 40, "x": true }]
}"#;

#[test]
fn a_document_is_given_skin_config_too() {
    let scratch = Scratch::new("config");
    scratch.write("gauge/groove.png", "");
    scratch.write("gauge/hard.png", "");
    let path = scratch.write("skin.json", CONFIGURED_DOCUMENT);
    let mut user = SkinUserConfig::default();
    user.offsets.insert(DECLARED_OFFSET, rbms_skin::dst::SkinOffset { x: 12.0, ..rbms_skin::dst::SkinOffset::default() });
    let options = SkinLoadOptions { rng_seed: Some(TEST_SEED), ..SkinLoadOptions::new(scratch.path(), &user, Mode::BEAT_7K) };
    let skin = load_skin_with_host(&path, options, &DefaultState).expect("the document should load");

    let option = compile(&skin, "skin_config.option['Panel']", LuaFnKind::Integer);
    let listed = compile(&skin, "#skin_config.enabled_options", LuaFnKind::Integer);
    let shift = compile(&skin, "skin_config.offset['Shift'].x", LuaFnKind::Integer);
    let gauge = compile(&skin, "skin_config.get_path('gauge/*.png')", LuaFnKind::Text);
    let reads = compile(&skin, "io.open(skin_config.get_path('gauge/*.png'), 'r') ~= nil", LuaFnKind::Boolean);

    runtime(&skin)
        .frame(&DefaultState, |frame| {
            assert_eq!(frame.call_integer(option), DEFAULT_OPTION);
            assert_eq!(frame.call_integer(listed), 1);
            assert_eq!(frame.call_integer(shift), 12, "an offset is published under its name with the value stored under its id");
            let path = frame.call_text(gauge);
            assert!(path.ends_with("/gauge/hard.png"), "the slot's `def` named the file by its stem: {path}");
            assert!(Path::new(&path).is_absolute(), "the path is one the interpreter's own file functions accept: {path}");
            assert!(frame.call_boolean(reads), "and `io.open` opens it");
        })
        .expect("the frame binds");
}

#[test]
fn a_script_may_require_a_module_beside_its_document() {
    let scratch = Scratch::new("require");
    scratch.write("parts/helper.lua", "local helper = {}\nfunction helper.double(value) return value * 2 end\nreturn helper\n");
    let host = host();
    let document =
        format!(r#"{{ "type": 5, "value": [{{ "id": "count", "src": "0", "value": "require('parts.helper').double(number({SAMPLE_NUMBER}))" }}] }}"#);
    let skin = load(&scratch, &document, &host);
    let count = function(&skin.def.value[0].value);

    let doubled = runtime(&skin).frame(&host, |frame| frame.call_integer(count)).expect("the frame binds");
    assert_eq!(doubled, SAMPLE_NUMBER_VALUE * 2);
}

#[test]
fn a_script_that_never_ends_is_cut_off_and_the_frame_goes_on() {
    let scratch = Scratch::new("runaway");
    let skin = empty(&scratch);
    let runaway = compile(&skin, "(function() while true do end end)()", LuaFnKind::Integer);
    let fine = compile(&skin, "42", LuaFnKind::Integer);

    let (stuck, after) = runtime(&skin).frame(&DefaultState, |frame| (frame.call_integer(runaway), frame.call_integer(fine))).expect("the frame binds");
    assert_eq!(stuck, 0, "a script cut off before it ever answered reads as its field's default");
    assert_eq!(after, 42, "and the next script of the same frame still runs");

    let diagnostics = runtime(&skin).diagnostics();
    assert_eq!(diagnostics.frames_over_budget, 1);
    assert_eq!(diagnostics.function_failures.len(), 1, "the failure is recorded once: {:?}", diagnostics.function_failures);
}

#[test]
fn a_script_that_raises_reads_as_its_default_and_is_recorded_once() {
    let scratch = Scratch::new("raises");
    let skin = empty(&scratch);
    let broken = compile(&skin, "no_such_function()", LuaFnKind::Boolean);

    let drawn = runtime(&skin).frame(&DefaultState, |frame| (frame.call_boolean(broken), frame.call_boolean(broken))).expect("the frame binds");
    assert_eq!(drawn, (false, false), "a condition that raises hides its object rather than failing the frame");
    let failures = runtime(&skin).diagnostics().function_failures;
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].count, 2, "it is called again every time, as the reference does");
    assert!(failures[0].first_message.contains("no_such_function"), "{}", failures[0].first_message);
}

#[test]
fn a_script_cannot_make_or_load_bytecode_or_leave_the_root() {
    let scratch = Scratch::new("contained");
    scratch.write("skin/inside.lua", "return 1");
    scratch.write("secret.txt", "not for a skin");
    let path = scratch.write("skin/skin.json", r#"{ "type": 5 }"#);
    let root = scratch.path().join("skin");
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(TEST_SEED), ..SkinLoadOptions::new(&root, &user, Mode::BEAT_7K) };
    let skin = load_skin_with_host(&path, options, &DefaultState).expect("the document should load");

    let checks = [
        "string.dump == nil",
        "load(string.char(27) .. 'Lua') == nil",
        "dofile('inside.lua') == 1",
        "not pcall(dofile, '../secret.txt')",
        "io.open('../secret.txt') == nil",
        "io.open('written.txt', 'w') == nil",
        "os.execute == nil and os.remove == nil and os.getenv == nil",
    ];
    for check in checks {
        let script = compile(&skin, check, LuaFnKind::Boolean);
        let held = runtime(&skin).frame(&DefaultState, |frame| frame.call_boolean(script)).expect("the frame binds");
        assert!(held, "{check} must hold for a document's script");
    }
    assert!(!root.join("written.txt").exists(), "a document loaded with no write overlay writes nowhere");
}
