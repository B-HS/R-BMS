//! The `main_state` module a Lua skin reads the game through, its `timer_util` and `event_util`
//! companions, the binding that points all three at a host, and the frame that calls a skin's
//! function values.
//!
//! Every test drives the modules through [`MapHost`], so each asserts what a skin is told for a
//! state written out in full: the value, its unit and what "nothing" reads as.

#![cfg(feature = "lua")]

use std::path::{Path, PathBuf};

use mlua::{FromLuaMulti, Function, Table, Value};
use rbms_model::Mode;
use rbms_skin::dst::{LuaDrawEval, SkinOffset, TimerRef, prepare};
use rbms_skin::loader::lua_skin::{LuaSkinOptions, load_lua_skin};
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig};
use rbms_skin::lua::main_state::{custom_timer_us, named_id};
use rbms_skin::lua::{FrameBudget, LuaBudget, LuaFnKind, LuaMode, SkinLua, SkinLuaConfig, error_message};
use rbms_skin::model::PropertyRef;
use rbms_skin::property::{FLOAT_ABSENT, HostCall, INTEGER_ABSENT, MapHost, NameSpace, ScoreSnapshot, SkinHost, VolumeBus};
use rbms_skin::timer::{TIMER_OFF, TimerState};

/// The seed every interpreter of these tests starts from.
const TEST_SEED: u64 = 7;

/// The chunk that puts the three modules where the test chunks reach them.
const PREAMBLE: &str = "ms = require('main_state') tu = require('timer_util') eu = require('event_util')";

/// Every function `main_state` offers.
const MAIN_STATE_FUNCTIONS: [&str; 49] = [
    "option",
    "number",
    "numbers",
    "float_number",
    "text",
    "offset",
    "timer",
    "timer_is_on",
    "timer_is_off",
    "timer_elapsed",
    "timer_elapsed_ms",
    "timer_elapsed_seconds",
    "time",
    "set_timer",
    "event_exec",
    "event_index",
    "key_pressed",
    "screen_width",
    "screen_height",
    "rate",
    "exscore",
    "rate_best",
    "exscore_best",
    "rate_rival",
    "exscore_rival",
    "volume_sys",
    "volume_key",
    "volume_bg",
    "set_volume_sys",
    "set_volume_key",
    "set_volume_bg",
    "gauge",
    "gauge_type",
    "judge",
    "audio_play",
    "audio_loop",
    "audio_preload",
    "audio_stop",
    "audio_dispose",
    "file_exists",
    "file_mkdir",
    "file_list",
    "file_read_lines",
    "file_write",
    "file_append",
    "file_clear",
    "file_count_lines",
    "http_get_lines",
    "http_get",
];

/// The functions of `main_state` that read or command the host, each as a call that is valid while a
/// host is bound.
const HOST_CALLS: [&str; 38] = [
    "ms.option(1)",
    "ms.number(1)",
    "ms.numbers(1, 2)",
    "ms.float_number(1)",
    "ms.text(1)",
    "ms.offset(1)",
    "ms.timer(1)",
    "ms.timer_is_on(1)",
    "ms.timer_is_off(1)",
    "ms.timer_elapsed(1)",
    "ms.timer_elapsed_ms(1)",
    "ms.timer_elapsed_seconds(1)",
    "ms.time()",
    "ms.event_exec(1)",
    "ms.event_index(308)",
    "ms.key_pressed(1)",
    "ms.screen_width()",
    "ms.screen_height()",
    "ms.rate()",
    "ms.exscore()",
    "ms.rate_best()",
    "ms.exscore_best()",
    "ms.rate_rival()",
    "ms.exscore_rival()",
    "ms.volume_sys()",
    "ms.volume_key()",
    "ms.volume_bg()",
    "ms.set_volume_sys(1)",
    "ms.set_volume_key(1)",
    "ms.set_volume_bg(1)",
    "ms.gauge()",
    "ms.gauge_type()",
    "ms.judge(0)",
    "ms.audio_play('a.ogg')",
    "ms.audio_loop('a.ogg')",
    "ms.audio_preload('a.ogg')",
    "ms.audio_stop('a.ogg')",
    "ms.audio_dispose('a.ogg')",
];

/// A custom timer id, inside the band a skin may write.
const CUSTOM_TIMER: i32 = 10_001;

/// The scene clock most tests read, in microseconds.
const NOW_US: i64 = 5_250_000;

/// The microsecond the sample timer switched on.
const TIMER_ON_US: i64 = 2_000_500;

/// The id of the sample timer.
const SAMPLE_TIMER: i32 = 41;

/// Bytes of the largest file `file_read_lines` reads.
const LINES_FILE_LIMIT: u64 = 64 * 1024 * 1024;

/// Calls a frame of the refusal test may make before the meter refuses the next.
const FEW_FRAME_CALLS: u32 = 3;

/// Instructions one call of the cut-off test may execute: far more than a function that ends needs,
/// and few enough that one that does not end is cut off at once.
const FEW_CALL_INSTRUCTIONS: u64 = 50_000;

/// How often the reference reads the timer of two objects in one frame: `isOff` and then `get` for
/// each (`SkinObject.prepareRegion`).
const READS_OF_TWO_OBJECTS: usize = 4;

/// The option the shared-timer skin gates its last object on.
const GATE_OPTION: i32 = 900;

/// The scene clock the shared-timer skin is prepared against, in microseconds.
const SHARED_NOW_US: i64 = 2_000_000;

/// How long before the frame the shared timer of that skin reports it switched on, in microseconds.
const SHARED_ELAPSED_US: i64 = 250_000;

/// Where a quarter of the way through its one-second slide puts an object of that skin.
const SHARED_X: f32 = 25.0;

/// A skin of this file's own: two objects and the image they show all follow one timer function,
/// and a third object is gated by one function and timed by another. Every function counts its own
/// calls in the global `called`.
const SHARED_TIMER_SKIN: &str = r#"
local main_state = require("main_state")
called = { shared = 0, gate = 0, behind = 0 }
local function shared()
    called.shared = called.shared + 1
    return main_state.time() - SHARED_ELAPSED_US
end
local function gate()
    called.gate = called.gate + 1
    return main_state.option(GATE_OPTION)
end
local function behind()
    called.behind = called.behind + 1
    return main_state.time()
end
local slide = { { time = 0, x = 0, y = 0, w = 16, h = 16 }, { time = 1000, x = 100 } }
return {
    type = 5, name = "shared timer", w = 1280, h = 720,
    source = { { id = 0, path = "panel.png" } },
    image = { { id = "tile", src = 0, x = 0, y = 0, w = 32, h = 16, divx = 2, timer = shared, cycle = 1000 } },
    destination = {
        { id = "tile", timer = shared, dst = slide },
        { id = "tile", timer = shared, dst = slide },
        { id = "tile", draw = gate, timer = behind, dst = slide },
    },
}
"#;

/// What one returned value reads as under each kind: as a condition, a whole number, a number, a
/// text and a timer.
type Readings = (bool, i32, f32, &'static str, i64);

/// A function body and what each kind reads of what it returns (`LuaValue.toboolean`, `toint`,
/// `tofloat`, `tojstring` and `tolong`). Nothing is not an error under any kind: it is false, zero,
/// the word `nil` and a timer on since the scene began.
const RETURNED: [(&str, Readings); 16] = [
    ("", (false, 0, 0.0, "nil", 0)),
    ("return nil", (false, 0, 0.0, "nil", 0)),
    ("return false", (false, 0, 0.0, "false", 0)),
    ("return true", (true, 0, 0.0, "true", 0)),
    ("return 0", (true, 0, 0.0, "0", 0)),
    ("return 7.9", (true, 7, 7.9, "7.9", 7)),
    ("return -7.9", (true, -7, -7.9, "-7.9", -7)),
    ("return 10 / 4", (true, 2, 2.5, "2.5", 2)),
    ("return 10 / 2", (true, 5, 5.0, "5", 5)),
    ("return '12'", (true, 12, 12.0, "12", 12)),
    ("return '1.5'", (true, 1, 1.5, "1.5", 1)),
    ("return 'twelve'", (true, 0, 0.0, "twelve", 0)),
    ("return {}", (true, 0, 0.0, "table", 0)),
    ("return 1e10", (true, 1_410_065_408, 1.0e10, "10000000000", 10_000_000_000)),
    ("return 5, 9", (true, 5, 5.0, "5", 5)),
    ("return ms.timer_off_value", (true, 0, i64::MIN as f32, "-9223372036854775808", TIMER_OFF)),
];

/// Bytes `file_count_lines` reads at a time, which is where a character can be split in two.
const COUNT_BLOCK: usize = 64 * 1024;

/// The fixture skin's root, which these tests only ever read.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("luaenv")
}

/// A scratch directory that removes itself, for the tests that write files.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-main-state-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A seeded interpreter for the skin at `root`, with the three modules in globals.
fn runtime_with(config: SkinLuaConfig) -> SkinLua {
    let runtime = SkinLua::new(SkinLuaConfig { seed: Some(TEST_SEED), ..config }).expect("the runtime builds");
    runtime.lua().load(PREAMBLE).exec().expect("the modules are published");
    runtime
}

/// A full-mode interpreter over the read-only fixture skin.
fn runtime() -> SkinLua {
    runtime_with(SkinLuaConfig::new(&fixture_root()))
}

/// Runs `source` with `host` bound and answers what it returns.
fn eval<R: FromLuaMulti>(runtime: &SkinLua, host: &dyn SkinHost, source: &str) -> R {
    runtime.with_host(host, || runtime.lua().load(source).eval::<R>()).expect("the host binds").unwrap_or_else(|error| panic!("{source}: {error}"))
}

/// Runs `source` with `host` bound and answers the message it raised.
fn eval_error(runtime: &SkinLua, host: &dyn SkinHost, source: &str) -> String {
    match runtime.with_host(host, || runtime.lua().load(source).eval::<Value>()).expect("the host binds") {
        Ok(value) => panic!("{source} should have raised, but returned {value:?}"),
        Err(error) => error_message(&error),
    }
}

/// Registers the function `source` evaluates to.
fn function(runtime: &SkinLua, source: &str, kind: LuaFnKind) -> rbms_skin::dst::LuaFnId {
    runtime.register(runtime.lua().load(source).eval::<Function>().expect("the function is created"), kind)
}

/// A host with one of everything a read can ask for.
fn sample_host() -> MapHost {
    let mut host = MapHost::new();
    host.booleans.insert(33, true);
    host.booleans.insert(42, false);
    host.integers.insert(96, 12);
    host.integers.insert(71, 3_456);
    host.floats.insert(1102, 0.875);
    host.rates.insert(1, 0.25);
    host.texts.insert(10, "FREEDOM DiVE".to_owned());
    host.image_indices.insert(308, 2);
    host.offsets.insert(3, SkinOffset { x: 1.5, y: -2.0, w: 3.0, h: 4.0, r: 90.0, a: 128.0 });
    host.timers.insert(SAMPLE_TIMER, TIMER_ON_US);
    host.now_us = NOW_US;
    host.pressed_keys.insert(21);
    host.screen = Some((1920, 1080));
    host.gauge = 82.5;
    host.gauge_type = 3;
    host.judges.insert(0, 1_000);
    host.judges.insert(1, 250);
    host.score = ScoreSnapshot { rate: 0.5, exscore: 2_250 };
    host.score_best = ScoreSnapshot { rate: 0.75, exscore: 3_000 };
    host.score_rival = ScoreSnapshot { rate: 0.625, exscore: 2_800 };
    host.volume_system.set(0.5);
    host.volume_key.set(0.25);
    host.volume_background.set(0.125);
    host
}

#[test]
fn the_module_offers_the_whole_api_and_only_through_require() {
    let runtime = runtime();
    for name in MAIN_STATE_FUNCTIONS {
        let kind: String = runtime.lua().load(format!("return type(ms.{name})")).eval().expect("the check runs");
        assert_eq!(kind, "function", "main_state.{name}");
    }
    let count: usize = runtime.lua().load("local count = 0 for _ in pairs(ms) do count = count + 1 end return count").eval().expect("the count runs");
    assert_eq!(count, MAIN_STATE_FUNCTIONS.len() + 1, "the functions above and timer_off_value are all there is");

    let fresh = SkinLua::new(SkinLuaConfig::new(&fixture_root())).expect("the runtime builds");
    let global: Value = fresh.lua().globals().get("main_state").expect("the global reads");
    assert!(global.is_nil(), "a Lua skin reaches the module through require alone");
    let same: bool = fresh.lua().load("return require('main_state') == require('main_state')").eval().expect("the check runs");
    assert!(same);
}

#[test]
fn an_option_reads_false_for_what_nothing_implements_whatever_its_sign() {
    let (runtime, host) = (runtime(), sample_host());
    assert!(eval::<bool>(&runtime, &host, "return ms.option(33)"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.option(-33)"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.option(42)"));
    assert!(eval::<bool>(&runtime, &host, "return ms.option(-42)"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.option(999)"), "an option nobody implements is false");
    assert!(!eval::<bool>(&runtime, &host, "return ms.option(-999)"), "and so is its negation");
    assert!(eval::<bool>(&runtime, &host, "return ms.option('33')"), "a string that spells a number is an id");
    assert!(eval::<bool>(&runtime, &host, "return ms.option(33.9)"), "an id is cut towards zero");
    assert!(eval::<bool>(&runtime, &host, "return ms.option('autoplay_on')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.option('!autoplay_on')"));
    assert!(eval::<bool>(&runtime, &host, "return ms.option('!gauge_groove')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.option('no_such_option')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.option('!no_such_option')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.option()"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.option({})"));
}

#[test]
fn a_number_tells_no_value_from_no_property() {
    let (runtime, host) = (runtime(), sample_host());
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.number(71)"), 3_456);
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.number('playlevel')"), 12);
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.number(72)"), i64::from(INTEGER_ABSENT), "a property with nothing to report is the sentinel");
    assert!(eval::<bool>(&runtime, &host, "return ms.number(72) == -2147483648"), "which a skin compares against as a literal");
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.number(104)"), 0, "an id the reference has no property under is zero");
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.number(-1)"), 0);
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.number(70000)"), 0);
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.number('no_such_number')"), 0);
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.number(nil)"), 0);
    assert_eq!(eval::<(i64, i64, i64)>(&runtime, &host, "return ms.numbers(71, 'playlevel', 104)"), (3_456, 12, 0));
    assert_eq!(eval::<i64>(&runtime, &host, "return select('#', ms.numbers())"), 0);
}

#[test]
fn a_float_falls_back_to_the_rates_and_a_text_is_empty_when_absent() {
    let (runtime, host) = (runtime(), sample_host());
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.float_number(1102)"), 0.875);
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.float_number('score_rate')"), 0.875);
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.float_number(1)"), 0.25, "a rate answers a float read");
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.float_number('musicselect_position')"), 0.25);
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.float_number(9999)"), 0.0);
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.float_number('no_such_float')"), 0.0);
    assert_eq!(
        eval::<f64>(&runtime, &host, "return ms.float_number(360)"),
        f64::from(FLOAT_ABSENT),
        "a float the reference has and the host carries nothing under is the reference's own 'no value', which is not zero"
    );

    assert_eq!(eval::<String>(&runtime, &host, "return ms.text(10)"), "FREEDOM DiVE");
    assert_eq!(eval::<String>(&runtime, &host, "return ms.text('title')"), "FREEDOM DiVE");
    assert_eq!(eval::<String>(&runtime, &host, "return ms.text(11)"), "");
    assert_eq!(eval::<String>(&runtime, &host, "return ms.text('no_such_text')"), "");
    assert_eq!(eval::<String>(&runtime, &host, "return ms.text()"), "");
}

#[test]
fn an_offset_is_a_fresh_table_of_six_numbers() {
    let (runtime, host) = (runtime(), sample_host());
    let values: (f64, f64, f64, f64, f64, f64) = eval(&runtime, &host, "local o = ms.offset(3) return o.x, o.y, o.w, o.h, o.r, o.a");
    assert_eq!(values, (1.5, -2.0, 3.0, 4.0, 90.0, 128.0));
    let zeros: (f64, f64, f64, f64, f64, f64) = eval(&runtime, &host, "local o = ms.offset(4) return o.x, o.y, o.w, o.h, o.r, o.a");
    assert_eq!(zeros, (0.0, 0.0, 0.0, 0.0, 0.0, 0.0), "an offset the host does not set is all zeros");
    assert!(!eval::<bool>(&runtime, &host, "return ms.offset(3) == ms.offset(3)"), "each call builds its own table");
    assert!(eval_error(&runtime, &host, "return ms.offset(200)").contains("offset id out of range: 200"));
    assert!(eval_error(&runtime, &host, "return ms.offset(-1)").contains("out of range"));
}

#[test]
fn timers_and_the_clock_are_microseconds_as_doubles() {
    let (runtime, host) = (runtime(), sample_host());
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.time()"), NOW_US as f64);
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.timer(41)"), TIMER_ON_US as f64);
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.timer(40)"), TIMER_OFF as f64, "an off timer is the sentinel");
    assert!(eval::<bool>(&runtime, &host, "return ms.timer(40) == ms.timer_off_value and ms.timer_off_value == -2^63"));
    assert!(eval::<bool>(&runtime, &host, "return math.type == nil and type(ms.timer(41)) == 'number'"), "this is a one-number-type interpreter");

    assert_eq!(eval::<(bool, bool)>(&runtime, &host, "return ms.timer_is_on(41), ms.timer_is_off(41)"), (true, false));
    assert_eq!(eval::<(bool, bool)>(&runtime, &host, "return ms.timer_is_on(40), ms.timer_is_off(40)"), (false, true));

    assert_eq!(eval::<f64>(&runtime, &host, "return ms.timer_elapsed(41)"), (NOW_US - TIMER_ON_US) as f64);
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.timer_elapsed_ms(41)"), 3_249.0, "milliseconds are whole, cut and not rounded");
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.timer_elapsed_seconds(41)"), 3.2495);
    assert_eq!(
        eval::<(f64, f64, f64)>(&runtime, &host, "return ms.timer_elapsed(40), ms.timer_elapsed_ms(40), ms.timer_elapsed_seconds(40)"),
        (-1.0, -1.0, -1.0)
    );
}

#[test]
fn a_skin_may_write_a_custom_timer_and_no_other() {
    let (runtime, mut host) = (runtime(), sample_host());
    assert_eq!(custom_timer_us(runtime.lua(), CUSTOM_TIMER), None);
    assert!(eval::<bool>(&runtime, &host, "return ms.timer_is_off(10001)"));

    assert!(eval::<bool>(&runtime, &host, "return ms.set_timer(10001, ms.time())"));
    assert_eq!(custom_timer_us(runtime.lua(), CUSTOM_TIMER), Some(NOW_US));
    assert!(host.calls().is_empty(), "the host is not told");

    host.now_us = NOW_US + 1_000_000;
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.timer(10001)"), NOW_US as f64, "the value outlives the binding it was written in");
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.timer_elapsed_ms(10001)"), 1_000.0);

    assert!(eval::<bool>(&runtime, &host, "return ms.set_timer(10001, ms.timer_off_value)"));
    assert_eq!(custom_timer_us(runtime.lua(), CUSTOM_TIMER), Some(TIMER_OFF));
    assert!(eval::<bool>(&runtime, &host, "return ms.timer_is_off(10001)"));

    assert!(eval::<bool>(&runtime, &host, "return ms.set_timer(19999, 1)"));
    for refused in ["ms.set_timer(41, 0)", "ms.set_timer(9999, 0)", "ms.set_timer(20000, 0)", "ms.set_timer()"] {
        assert!(eval_error(&runtime, &host, refused).contains("cannot be changed by a skin"), "{refused}");
    }
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.timer(41)"), TIMER_ON_US as f64, "a refused write changes nothing");
}

#[test]
fn an_event_runs_with_its_missing_arguments_as_zero() {
    let (runtime, host) = (runtime(), sample_host());
    assert!(eval::<bool>(&runtime, &host, "return ms.event_exec(13)"));
    assert!(eval::<bool>(&runtime, &host, "return ms.event_exec(42, -1)"));
    assert!(eval::<bool>(&runtime, &host, "return ms.event_exec('1000', 2.9, 3)"));
    assert_eq!(
        host.take_calls(),
        vec![HostCall::Event { id: 13, arg1: 0, arg2: 0 }, HostCall::Event { id: 42, arg1: -1, arg2: 0 }, HostCall::Event { id: 1000, arg1: 2, arg2: 3 }]
    );
    assert!(eval_error(&runtime, &host, "return ms.event_exec()").contains("event_exec takes an event id"));
    assert!(eval_error(&runtime, &host, "return ms.event_exec(1, 2, 3, 4)").contains("at most two arguments"));
    assert!(host.calls().is_empty());
}

#[test]
fn an_image_index_exists_only_where_the_reference_has_one() {
    let (runtime, host) = (runtime(), sample_host());
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.event_index(308)"), 2);
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.event_index(42)"), -1, "an index with nothing selected draws nothing");
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.event_index(505)"), -1, "a per-key judgement is answered from code");
    assert!(eval_error(&runtime, &host, "return ms.event_index(71)").contains("no image index property has the id 71"));
}

#[test]
fn a_key_is_a_code_or_a_display_name() {
    let (runtime, host) = (runtime(), sample_host());
    assert!(eval::<bool>(&runtime, &host, "return ms.key_pressed(21)"));
    assert!(eval::<bool>(&runtime, &host, "return ms.key_pressed('Left')"));
    assert!(eval::<bool>(&runtime, &host, "return ms.key_pressed('21')"), "a string that spells a number is a code");
    assert!(!eval::<bool>(&runtime, &host, "return ms.key_pressed(22)"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.key_pressed('Right')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.key_pressed(-1)"), "a negative code is never pressed");
    assert!(!eval::<bool>(&runtime, &host, "return ms.key_pressed('LEFT')"), "a constant's name is not a display name");
    assert!(!eval::<bool>(&runtime, &host, "return ms.key_pressed()"));
}

#[test]
fn the_screen_the_scores_the_gauge_and_the_volumes_read_through() {
    let (runtime, host) = (runtime(), sample_host());
    assert_eq!(eval::<(i64, i64)>(&runtime, &host, "return ms.screen_width(), ms.screen_height()"), (1920, 1080));
    assert_eq!(eval::<(f64, f64)>(&runtime, &host, "return ms.rate(), ms.exscore()"), (0.5, 2_250.0));
    assert_eq!(eval::<(f64, f64)>(&runtime, &host, "return ms.rate_best(), ms.exscore_best()"), (0.75, 3_000.0));
    assert_eq!(eval::<(f64, f64)>(&runtime, &host, "return ms.rate_rival(), ms.exscore_rival()"), (0.625, 2_800.0));
    assert_eq!(eval::<(f64, i64)>(&runtime, &host, "return ms.gauge(), ms.gauge_type()"), (82.5, 3));
    assert_eq!(eval::<(i64, i64, i64)>(&runtime, &host, "return ms.judge(0), ms.judge(1), ms.judge(5)"), (1_000, 250, 0));
    assert_eq!(eval::<(f64, f64, f64)>(&runtime, &host, "return ms.volume_sys(), ms.volume_key(), ms.volume_bg()"), (0.5, 0.25, 0.125));

    let empty = MapHost::new();
    assert_eq!(eval::<(i64, i64)>(&runtime, &empty, "return ms.screen_width(), ms.screen_height()"), (1280, 720));
    assert_eq!(eval::<(f64, i64, f64)>(&runtime, &empty, "return ms.gauge(), ms.gauge_type(), ms.rate()"), (0.0, 0, 0.0));
}

#[test]
fn setting_a_volume_reaches_the_host_unchecked() {
    let (runtime, host) = (runtime(), sample_host());
    assert!(eval::<bool>(&runtime, &host, "return ms.set_volume_sys(0.75)"));
    assert!(eval::<bool>(&runtime, &host, "return ms.set_volume_key('0.5')"));
    assert!(eval::<bool>(&runtime, &host, "return ms.set_volume_bg(3)"));
    assert_eq!(
        host.take_calls(),
        vec![
            HostCall::SetVolume { bus: VolumeBus::System, value: 0.75 },
            HostCall::SetVolume { bus: VolumeBus::Key, value: 0.5 },
            HostCall::SetVolume { bus: VolumeBus::Background, value: 3.0 },
        ]
    );
    assert_eq!(eval::<f64>(&runtime, &host, "return ms.volume_bg()"), 3.0, "the next read sees what was set");
}

#[test]
fn a_sound_is_named_by_a_path_inside_the_skin_and_a_volume_within_bounds() {
    let (runtime, host) = (runtime(), sample_host());
    let sound = runtime.paths().root().join("sound").join("decide.ogg");
    assert!(eval::<bool>(&runtime, &host, "return ms.audio_play('sound/decide.ogg')"));
    assert!(eval::<bool>(&runtime, &host, "return ms.audio_play('sound/decide.ogg', 0.5)"));
    assert!(eval::<bool>(&runtime, &host, "return ms.audio_loop('sound\\\\decide.ogg', 9)"));
    assert!(eval::<bool>(&runtime, &host, "return ms.audio_play('sound/decide.ogg', -1)"));
    assert!(eval::<bool>(&runtime, &host, "return ms.audio_preload('sound/decide.ogg')"));
    assert!(eval::<bool>(&runtime, &host, "return ms.audio_stop('sound/decide.ogg')"));
    assert!(eval::<bool>(&runtime, &host, "return ms.audio_dispose('sound/decide.ogg')"));
    assert_eq!(
        host.take_calls(),
        vec![
            HostCall::AudioPlay { path: sound.clone(), volume: 1.0, looped: false },
            HostCall::AudioPlay { path: sound.clone(), volume: 0.5, looped: false },
            HostCall::AudioPlay { path: sound.clone(), volume: 2.0, looped: true },
            HostCall::AudioPlay { path: sound.clone(), volume: 0.0, looped: false },
            HostCall::AudioPreload { path: sound.clone() },
            HostCall::AudioStop { path: sound.clone() },
            HostCall::AudioDispose { path: sound },
        ]
    );

    for escaping in [
        "ms.audio_play('../outside.ogg')",
        "ms.audio_loop('/etc/passwd')",
        "ms.audio_preload('a/../../b.ogg')",
        "ms.audio_stop('..')",
        "ms.audio_dispose('../x')",
    ] {
        let message = eval_error(&runtime, &host, &format!("return {escaping}"));
        assert!(message.contains("skin file access denied: "), "{escaping}: {message}");
    }
    assert!(host.calls().is_empty(), "a refused path reaches nobody");
}

#[test]
fn files_are_read_from_the_merged_tree_and_written_to_the_overlay_alone() {
    let scratch = Scratch::new("files");
    let (root, overlay) = (scratch.path().join("skin"), scratch.path().join("overlay"));
    std::fs::create_dir_all(root.join("data")).expect("the skin root is created");
    std::fs::write(root.join("data").join("history.txt"), "one\r\ntwo\n").expect("the fixture is written");
    std::fs::write(root.join("data").join("binary.dat"), [0xff, 0xfe, 0x00]).expect("the fixture is written");
    let runtime = runtime_with(SkinLuaConfig { overlay: Some(overlay.clone()), ..SkinLuaConfig::new(&root) });
    let host = MapHost::new();

    assert!(eval::<bool>(&runtime, &host, "return ms.file_exists('data/history.txt')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.file_exists('data/missing.txt')"));
    assert!(eval_error(&runtime, &host, "return ms.file_exists('../outside.txt')").contains("skin file access denied: ../outside.txt"));

    assert_eq!(
        eval::<(i64, String, String)>(&runtime, &host, "local l = ms.file_read_lines('data/history.txt') return #l, l[1], l[2]"),
        (2, "one".to_owned(), "two".to_owned())
    );
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.file_count_lines('data/history.txt')"), 2);
    assert_eq!(eval::<i64>(&runtime, &host, "return #ms.file_read_lines('data/missing.txt')"), 0);
    assert_eq!(eval::<i64>(&runtime, &host, "return #ms.file_read_lines('data/binary.dat')"), 0, "a file that is not text has no lines");
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.file_count_lines('../outside.txt')"), 0);

    assert!(eval::<bool>(&runtime, &host, "return ms.file_append('data/history.txt', 'three\\n')"));
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.file_count_lines('data/history.txt')"), 3, "an append starts from what the skin already holds");
    assert_eq!(std::fs::read_to_string(root.join("data").join("history.txt")).expect("the original reads"), "one\r\ntwo\n", "the skin folder is never written");
    assert_eq!(std::fs::read_to_string(overlay.join("data").join("history.txt")).expect("the copy reads"), "one\r\ntwo\nthree\n");

    assert!(eval::<bool>(&runtime, &host, "return ms.file_write('new/deep/note.txt', 'hello')"));
    assert_eq!(std::fs::read_to_string(overlay.join("new").join("deep").join("note.txt")).expect("the note reads"), "hello");
    assert!(!root.join("new").exists());
    assert!(eval::<bool>(&runtime, &host, "return ms.file_exists('new/deep/note.txt')"));
    assert!(eval::<bool>(&runtime, &host, "return ms.file_write('new/deep/note.txt', 12)"), "a number is written as its text");
    assert_eq!(eval::<String>(&runtime, &host, "return ms.file_read_lines('new/deep/note.txt')[1]"), "12");
    assert!(eval::<bool>(&runtime, &host, "return ms.file_clear('new/deep/note.txt')"));
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.file_count_lines('new/deep/note.txt')"), 0);
    assert!(!eval::<bool>(&runtime, &host, "return ms.file_write('../outside.txt', 'x')"));
    assert!(!scratch.path().join("outside.txt").exists());

    assert!(eval::<bool>(&runtime, &host, "return ms.file_mkdir('made/here')"));
    assert!(overlay.join("made").join("here").is_dir());
    assert!(!eval::<bool>(&runtime, &host, "return ms.file_mkdir('../made')"));

    let (listing, count): (String, i64) = eval(&runtime, &host, "return ms.file_list('data')");
    let prefix = runtime.paths().root().to_string_lossy().replace('\\', "/");
    assert_eq!(listing, format!("{prefix}/data/binary.dat\n{prefix}/data/history.txt\n"));
    assert_eq!(count, 2);
    assert_eq!(eval::<(String, i64)>(&runtime, &host, "return ms.file_list('data', '[^/]+%.txt$')"), ("history.txt\n".to_owned(), 1));
    assert_eq!(eval::<(String, i64)>(&runtime, &host, "return ms.file_list('data', '')"), (listing, 2), "an empty pattern is no pattern");
    assert_eq!(eval::<(String, i64)>(&runtime, &host, "return ms.file_list('data', '%')"), (String::new(), 0), "a malformed pattern lists nothing");
    assert_eq!(eval::<(String, i64)>(&runtime, &host, "return ms.file_list('nowhere')"), (String::new(), 0));
    assert_eq!(eval::<(String, i64)>(&runtime, &host, "return ms.file_list('../..')"), (String::new(), 0));
}

#[test]
fn a_file_too_large_to_hold_is_counted_but_not_read() {
    let scratch = Scratch::new("large");
    let (root, overlay) = (scratch.path().join("skin"), scratch.path().join("overlay"));
    std::fs::create_dir_all(&root).expect("the skin root is created");
    for (name, size) in [("at_limit.dat", LINES_FILE_LIMIT), ("over_limit.dat", LINES_FILE_LIMIT + 1)] {
        std::fs::File::create(root.join(name)).and_then(|file| file.set_len(size)).expect("the sparse fixture is made");
    }
    let runtime = runtime_with(SkinLuaConfig { overlay: Some(overlay), ..SkinLuaConfig::new(&root) });
    let host = MapHost::new();

    assert_eq!(eval::<i64>(&runtime, &host, "return #ms.file_read_lines('over_limit.dat')"), 0, "its lines would be held outside the memory ceiling");
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.file_count_lines('over_limit.dat')"), 1, "counting never holds the file");
    assert_eq!(eval::<i64>(&runtime, &host, "return #ms.file_read_lines('at_limit.dat')"), 1);
}

#[test]
fn counting_lines_reads_text_split_anywhere_and_refuses_what_is_not_text() {
    let scratch = Scratch::new("count");
    let root = scratch.path().join("skin");
    std::fs::create_dir_all(&root).expect("the skin root is created");
    for shift in 0..4 {
        let mut text = "a".repeat(COUNT_BLOCK - 2 + shift);
        text.push_str("\u{d55c}\r\n\u{1f3b5}\n");
        text.push_str(&"\u{e9}\r".repeat(COUNT_BLOCK));
        text.push_str("last");
        std::fs::write(root.join(format!("split{shift}.txt")), &text).expect("the fixture is written");
    }
    let mut broken = "line\n".repeat(COUNT_BLOCK).into_bytes();
    broken.extend_from_slice(&[0xe3, 0x81]);
    std::fs::write(root.join("cut_short.txt"), &broken).expect("the fixture is written");
    broken.extend_from_slice(b"\nmore\n");
    std::fs::write(root.join("broken.txt"), &broken).expect("the fixture is written");
    let runtime = runtime_with(SkinLuaConfig::new(&root));
    let host = MapHost::new();

    for shift in 0..4 {
        let (counted, read): (i64, i64) =
            eval(&runtime, &host, &format!("return ms.file_count_lines('split{shift}.txt'), #ms.file_read_lines('split{shift}.txt')"));
        assert_eq!(counted, COUNT_BLOCK as i64 + 3, "split{shift}.txt");
        assert_eq!(counted, read, "split{shift}.txt counts the lines it reads");
    }
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.file_count_lines('cut_short.txt')"), 0, "a character the file ends inside is not text");
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.file_count_lines('broken.txt')"), 0);
    assert_eq!(eval::<i64>(&runtime, &host, "return ms.file_count_lines('missing.txt')"), 0);
}

#[cfg(unix)]
#[test]
fn appending_to_a_read_only_file_of_the_skin_makes_a_writable_copy() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new("read-only-append");
    let (root, overlay) = (scratch.path().join("skin"), scratch.path().join("overlay"));
    std::fs::create_dir_all(&root).expect("the skin root is created");
    let original = root.join("history.txt");
    std::fs::write(&original, "one\n").expect("the fixture is written");
    std::fs::set_permissions(&original, std::fs::Permissions::from_mode(0o444)).expect("the fixture is made read-only");
    let runtime = runtime_with(SkinLuaConfig { overlay: Some(overlay.clone()), ..SkinLuaConfig::new(&root) });
    let host = MapHost::new();

    assert!(eval::<bool>(&runtime, &host, "return ms.file_append('history.txt', 'two\\n')"));
    assert!(eval::<bool>(&runtime, &host, "return ms.file_append('history.txt', 'three\\n')"), "the copy can be written again");
    let copy = overlay.join("history.txt");
    assert_eq!(std::fs::read_to_string(&copy).expect("the copy reads"), "one\ntwo\nthree\n");
    assert!(!std::fs::metadata(&copy).expect("the copy exists").permissions().readonly(), "the copy does not inherit the original's permissions");
    assert!(eval::<bool>(&runtime, &host, "return ms.file_clear('history.txt')"));
    assert_eq!(std::fs::read_to_string(&original).expect("the original reads"), "one\n", "the skin folder is never written");
}

#[test]
fn a_skin_without_an_overlay_cannot_write_at_all() {
    let scratch = Scratch::new("readonly");
    std::fs::write(scratch.path().join("note.txt"), "kept").expect("the fixture is written");
    let runtime = runtime_with(SkinLuaConfig::new(scratch.path()));
    let host = MapHost::new();
    assert!(!eval::<bool>(&runtime, &host, "return ms.file_write('note.txt', 'gone')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.file_append('note.txt', 'more')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.file_clear('note.txt')"));
    assert!(!eval::<bool>(&runtime, &host, "return ms.file_mkdir('made')"));
    assert_eq!(std::fs::read_to_string(scratch.path().join("note.txt")).expect("the note reads"), "kept");
    assert_eq!(std::fs::read_dir(scratch.path()).expect("the root lists").count(), 1, "nothing was created beside it");
}

#[test]
fn the_network_functions_always_refuse() {
    let (runtime, host) = (runtime(), sample_host());
    for call in ["ms.http_get('https://example.invalid/')", "ms.http_get_lines('https://example.invalid/', 100)", "ms.http_get()"] {
        let (body, message): (Value, String) = eval(&runtime, &host, &format!("return {call}"));
        assert!(body.is_nil(), "{call}");
        assert!(message.contains("network access"), "{call}: {message}");
    }
    assert!(host.calls().is_empty());
}

#[test]
fn a_call_with_no_host_bound_fails_like_a_call_into_an_empty_module() {
    let (runtime, host) = (runtime(), sample_host());
    let unbound = |source: &str| error_message(&runtime.lua().load(source).eval::<Value>().expect_err("nothing is bound"));

    for call in HOST_CALLS {
        let message = unbound(&format!("return {call}"));
        assert!(message.contains("attempt to call") && message.contains("nil value"), "{call} before any binding: {message}");
    }
    for call in HOST_CALLS {
        eval::<Value>(&runtime, &host, &format!("return {call}"));
    }
    for call in HOST_CALLS {
        let message = unbound(&format!("return {call}"));
        assert!(message.contains("attempt to call") && message.contains("nil value"), "{call} after a binding ended: {message}");
    }

    assert!(runtime.lua().load("return ms.set_timer(10001, 5)").eval::<bool>().expect("a custom timer needs no host"));
    assert!(runtime.lua().load("return ms.timer_off_value == -2^63").eval::<bool>().expect("the constant needs no host"));
    let caught: (bool, String) = runtime.lua().load("return pcall(ms.option, 33)").eval().expect("a skin may catch the failure");
    assert!(!caught.0 && caught.1.contains("nil value"), "a skin's own pcall sees a string: {caught:?}");
}

#[test]
fn the_functions_are_the_same_objects_under_every_binding() {
    let runtime = runtime();
    runtime.lua().load("kept_option, kept_time = ms.option, ms.time").exec().expect("a skin caches the functions while it loads");
    let mut host = sample_host();
    assert!(eval::<bool>(&runtime, &host, "return kept_option == ms.option and kept_option(33)"));
    host.booleans.insert(33, false);
    host.now_us = 9;
    assert_eq!(eval::<(bool, f64)>(&runtime, &host, "return kept_option(33), kept_time()"), (false, 9.0), "a cached function reads whatever is bound now");

    let other = MapHost::new();
    assert_eq!(eval::<(bool, f64)>(&runtime, &other, "return kept_option(33), kept_time()"), (false, 0.0));
}

#[test]
fn a_header_only_interpreter_publishes_three_empty_modules() {
    let runtime =
        SkinLua::new(SkinLuaConfig { mode: LuaMode::HeaderOnly, seed: Some(TEST_SEED), ..SkinLuaConfig::new(&fixture_root()) }).expect("the runtime builds");
    for module in ["main_state", "timer_util", "event_util"] {
        let empty: bool = runtime.lua().load(format!("return next(require('{module}')) == nil")).eval().expect("the check runs");
        assert!(empty, "{module} is an empty table while a header is read");
    }
    let host = sample_host();
    let outcome = runtime.with_host(&host, || runtime.lua().load("return require('main_state').option(33)").eval::<Value>()).expect("binding does nothing");
    let message = error_message(&outcome.expect_err("there is no such function"));
    assert!(message.contains("attempt to call") && message.contains("nil value"), "{message}");
    assert_eq!(named_id(runtime.lua(), NameSpace::Boolean, "autoplay_on"), Some(33), "a name still resolves without the module");
}

#[test]
fn an_observed_boolean_latches_the_moment_it_turned_true_and_lets_go_when_it_turns_false() {
    let runtime = runtime();
    runtime.lua().load("flag = false").exec().expect("the flag is set");
    let watched = function(&runtime, "return tu.timer_observe_boolean(function() return flag end)", LuaFnKind::Timer);
    let constant = function(&runtime, "return tu.timer_observe_boolean(function() return 0 end)", LuaFnKind::Timer);
    let second = function(&runtime, "return tu.timer_observe_boolean(function() return flag end)", LuaFnKind::Timer);
    let mut host = MapHost::new();
    let mut step = |now_us: i64, flag: bool| {
        host.now_us = now_us;
        runtime.lua().globals().set("flag", flag).expect("the flag is set");
        runtime.frame(&host, |frame| (frame.call_timer(watched), frame.call_timer(watched), frame.call_timer(constant))).expect("the frame binds")
    };

    assert_eq!(step(1_000, false), (TIMER_OFF, TIMER_OFF, 1_000), "zero is true, so the constant one switches on at once");
    assert_eq!(step(2_000, true), (2_000, 2_000, 1_000), "two calls in one frame agree");
    assert_eq!(step(3_000, true), (2_000, 2_000, 1_000), "the moment is latched, not refreshed");
    assert_eq!(step(4_000, false), (TIMER_OFF, TIMER_OFF, 1_000));
    assert_eq!(step(5_000, true), (5_000, 5_000, 1_000), "turning true again latches the new moment");

    let late = runtime.frame(&host, |frame| frame.call_timer(second)).expect("the frame binds");
    assert_eq!(late, 5_000, "each returned function keeps its own state: this one first saw the flag at 5000");
    host.now_us = 6_000;
    let (first, other) = runtime.frame(&host, |frame| (frame.call_timer(watched), frame.call_timer(second))).expect("the frame binds");
    assert_eq!((first, other), (5_000, 5_000));
}

#[test]
fn an_observer_fails_as_a_whole_when_what_it_observes_fails_and_recovers_with_it() {
    let runtime = runtime();
    runtime.lua().load("broken = true").exec().expect("the flag is set");
    let observer = function(&runtime, "return tu.timer_observe_boolean(function() if broken then error('not yet') end return true end)", LuaFnKind::Timer);
    let mut host = MapHost::new();
    host.now_us = 700;
    assert_eq!(runtime.frame(&host, |frame| frame.call_timer(observer)).expect("the frame binds"), TIMER_OFF);
    runtime.lua().load("broken = false").exec().expect("the flag is cleared");
    assert_eq!(runtime.frame(&host, |frame| frame.call_timer(observer)).expect("the frame binds"), 700);

    let failures = runtime.diagnostics().function_failures;
    assert_eq!(failures.len(), 1);
    assert_eq!((failures[0].function, failures[0].count), (observer, 1));
    assert!(failures[0].first_message.contains("not yet"));

    let refused = error_message(&runtime.lua().load("return tu.timer_observe_boolean(42)").eval::<Value>().expect_err("a number is not a function"));
    assert!(refused.contains("function expected, got number"), "{refused}");
}

#[test]
fn the_other_timer_helpers_read_values_not_ids() {
    let (runtime, mut host) = (runtime(), sample_host());
    assert_eq!(eval::<f64>(&runtime, &host, "return tu.now_timer(ms.timer(41))"), (NOW_US - TIMER_ON_US) as f64);
    assert_eq!(eval::<f64>(&runtime, &host, "return tu.now_timer(ms.timer(40))"), 0.0, "an off timer has run for no time");
    assert_eq!(eval::<f64>(&runtime, &host, "return tu.now_timer(nil)"), NOW_US as f64, "what is not a number reads as zero, a timer on since the start");
    assert_eq!(eval::<(bool, bool)>(&runtime, &host, "return tu.is_timer_on(ms.timer(41)), tu.is_timer_off(ms.timer(41))"), (true, false));
    assert_eq!(eval::<(bool, bool)>(&runtime, &host, "return tu.is_timer_on(ms.timer_off_value), tu.is_timer_off(ms.timer_off_value)"), (false, true));
    assert_eq!(eval::<(bool, bool)>(&runtime, &host, "return tu.is_timer_on(false), tu.is_timer_on('x')"), (true, true));

    runtime.lua().load("by_id = tu.timer_function(41) passive = tu.new_passive_timer()").exec().expect("the helpers are created without a host");
    assert_eq!(eval::<f64>(&runtime, &host, "return by_id()"), TIMER_ON_US as f64);
    assert_eq!(eval::<f64>(&runtime, &host, "return passive.timer()"), TIMER_OFF as f64);
    assert_eq!(eval::<(bool, f64)>(&runtime, &host, "return passive.turn_on(), passive.timer()"), (true, NOW_US as f64));
    host.now_us = NOW_US + 10;
    assert_eq!(
        eval::<(bool, f64)>(&runtime, &host, "return passive.turn_on(), passive.timer()"),
        (true, NOW_US as f64),
        "turning on what is on changes nothing"
    );
    assert_eq!(eval::<(bool, f64)>(&runtime, &host, "return passive.turn_on_reset(), passive.timer()"), (true, (NOW_US + 10) as f64));
    assert_eq!(eval::<(bool, f64)>(&runtime, &host, "return passive.turn_off(), passive.timer()"), (true, TIMER_OFF as f64));
    host.timers.remove(&SAMPLE_TIMER);
    assert_eq!(eval::<f64>(&runtime, &host, "return by_id()"), TIMER_OFF as f64, "the function reads the timer each time it is called");
}

#[test]
fn the_event_helpers_fire_on_the_edge_they_watch() {
    let runtime = runtime();
    runtime
        .lua()
        .load(
            r#"
            flag, value, fired = false, ms.timer_off_value, {}
            local function count(name) return function() fired[name] = (fired[name] or 0) + 1 end end
            local function read() return value end
            turn_true = eu.event_observe_turn_true(function() return flag end, count('turn_true'))
            any_timer = eu.event_observe_timer(read, count('timer'))
            timer_on = eu.event_observe_timer_on(read, count('timer_on'))
            timer_off = eu.event_observe_timer_off(read, count('timer_off'))
            spaced = eu.event_min_interval(100, count('spaced'))
            function tick() return turn_true(), any_timer(), timer_on(), timer_off(), spaced() end
            "#,
        )
        .exec()
        .expect("the helpers are created without a host");
    let mut host = MapHost::new();
    let mut step = |now_us: i64, flag: bool, value: i64| {
        host.now_us = now_us;
        let ticked: (bool, bool, bool, bool, bool) = eval(&runtime, &host, &format!("flag, value = {flag}, {value} return tick()"));
        assert_eq!(ticked, (true, true, true, true, true), "every event function answers true");
        let fired: Table = runtime.lua().globals().get("fired").expect("the counts read");
        let count = |name: &str| fired.get::<Option<i64>>(name).expect("a count reads").unwrap_or(0);
        (count("turn_true"), count("timer"), count("timer_on"), count("timer_off"), count("spaced"))
    };

    assert_eq!(step(0, false, i64::MIN), (0, 0, 0, 1, 1), "a timer that starts off counts as having just switched off");
    assert_eq!(step(50_000, true, 40_000), (1, 1, 1, 1, 1), "50 ms is under the interval");
    assert_eq!(step(99_999, true, 40_000), (1, 1, 1, 1, 1), "nothing changed and 99 ms is still under it");
    assert_eq!(step(100_000, true, 60_000), (1, 2, 1, 1, 2), "a timer set again fires the plain observer only");
    assert_eq!(step(150_000, false, i64::MIN), (1, 2, 1, 2, 2));
    assert_eq!(step(200_000, true, 190_000), (2, 3, 2, 2, 3));

    for refused in [
        "eu.event_observe_turn_true(1, print)",
        "eu.event_observe_timer(print)",
        "eu.event_observe_timer_on(nil, print)",
        "eu.event_observe_timer_off(print, 'x')",
        "eu.event_min_interval(10)",
    ] {
        let message = error_message(&runtime.lua().load(format!("return {refused}")).eval::<Value>().expect_err("an argument is not a function"));
        assert!(message.contains("function expected"), "{refused}: {message}");
    }
}

#[test]
fn a_frame_answers_each_kind_with_its_default_when_the_function_fails_and_calls_it_again() {
    let (runtime, host) = (runtime(), sample_host());
    runtime.lua().load("attempts = 0").exec().expect("the counter is set");
    let flaky = "function() attempts = attempts + 1 if attempts % 2 == 1 then error('odd attempt') end return VALUE end";
    let make = |value: &str, kind: LuaFnKind| function(&runtime, &format!("return {}", flaky.replace("VALUE", value)), kind);
    let (boolean, integer, float, text, timer) = (
        make("true", LuaFnKind::Boolean),
        make("ms.number(71)", LuaFnKind::Integer),
        make("ms.float_number(1102)", LuaFnKind::Float),
        make("ms.text(10)", LuaFnKind::Text),
        make("ms.timer(41)", LuaFnKind::Timer),
    );

    runtime
        .frame(&host, |frame| {
            assert!(!frame.call_boolean(boolean));
            assert!(frame.call_boolean(boolean), "the function is not disabled: the next call runs it again");
            assert_eq!((frame.call_integer(integer), frame.call_integer(integer)), (0, 3_456));
            assert_eq!((frame.call_float(float), frame.call_float(float)), (0.0, 0.875));
            assert_eq!((frame.call_text(text), frame.call_text(text)), (String::new(), "FREEDOM DiVE".to_owned()));
            assert_eq!((frame.call_timer(timer), frame.call_timer(timer)), (TIMER_OFF, TIMER_ON_US));
        })
        .expect("the frame binds");
    runtime.frame(&host, |frame| assert!(!frame.call_boolean(boolean), "and it may fail again on a later frame")).expect("the frame binds");

    let failures = runtime.diagnostics().function_failures;
    assert_eq!(
        failures.iter().map(|failure| (failure.function, failure.kind, failure.count)).collect::<Vec<_>>(),
        [
            (boolean, LuaFnKind::Boolean, 2),
            (integer, LuaFnKind::Integer, 1),
            (float, LuaFnKind::Float, 1),
            (text, LuaFnKind::Text, 1),
            (timer, LuaFnKind::Timer, 1),
        ]
    );
    assert!(failures.iter().all(|failure| failure.first_message.contains("odd attempt")), "the first message is kept: {failures:?}");
}

#[test]
fn a_timer_function_that_answers_nothing_is_on_since_zero() {
    let (runtime, host) = (runtime(), sample_host());
    let silent = function(&runtime, "return function() end", LuaFnKind::Timer);
    let truthy = function(&runtime, "return function() return true end", LuaFnKind::Timer);
    let off = function(&runtime, "return function() return ms.timer_off_value end", LuaFnKind::Timer);
    let cut = function(&runtime, "return function() return 1500.9 end", LuaFnKind::Timer);
    let values = runtime.frame(&host, |frame| [silent, truthy, off, cut].map(|timer| frame.call_timer(timer))).expect("the frame binds");
    assert_eq!(values, [0, 0, TIMER_OFF, 1_500]);
    assert!(runtime.diagnostics().function_failures.is_empty());
}

#[test]
fn an_event_function_is_called_with_its_one_argument_and_a_failure_is_only_recorded() {
    let (runtime, host) = (runtime(), sample_host());
    let forward = function(&runtime, "return function(...) seen = { select('#', ...), ... }; ms.event_exec(13, (...)) end", LuaFnKind::Event);
    let broken = function(&runtime, "return function() error('no such panel') end", LuaFnKind::Event);
    let observed = function(&runtime, "return eu.event_observe_turn_true(function() return true end, function() ms.set_volume_key(0.5) end)", LuaFnKind::Event);
    runtime
        .frame(&host, |frame| {
            frame.call_event(forward, -1);
            frame.call_event(broken, 0);
            frame.call_event(observed, 0);
            frame.call_event(observed, 0);
        })
        .expect("the frame binds");

    assert_eq!(runtime.lua().load("return seen[1], seen[2]").eval::<(i64, i64)>().expect("the arguments read"), (1, -1));
    assert_eq!(host.take_calls(), vec![HostCall::Event { id: 13, arg1: -1, arg2: 0 }, HostCall::SetVolume { bus: VolumeBus::Key, value: 0.5 }]);
    let failures = runtime.diagnostics().function_failures;
    assert_eq!(failures.len(), 1);
    assert_eq!((failures[0].function, failures[0].kind, failures[0].count), (broken, LuaFnKind::Event, 1));
}

#[test]
fn a_frame_is_the_evaluator_for_functions_and_for_names() {
    let (runtime, host) = (runtime(), sample_host());
    let shown = function(&runtime, "return function() return ms.option(33) end", LuaFnKind::Boolean);
    let level = function(&runtime, "return function() return ms.number(96) * 2 end", LuaFnKind::Integer);
    runtime
        .frame(&host, |frame| {
            let evaluator: &dyn LuaDrawEval = frame;
            assert!(evaluator.call_boolean(shown));
            assert_eq!(evaluator.call_integer(level), 24);
            assert_eq!(evaluator.call_timer(shown), 0, "a function read as another kind follows that kind's rule");

            assert!(evaluator.named_boolean("autoplay_on"));
            assert!(!evaluator.named_boolean("!autoplay_on"));
            assert!(evaluator.named_boolean("!gauge_groove"));
            assert!(!evaluator.named_boolean("no_such_option"));
            assert_eq!(evaluator.named_integer("playlevel"), 12);
            assert_eq!(evaluator.named_integer("maxbpm"), INTEGER_ABSENT, "a named property with nothing to report is the sentinel");
            assert_eq!(evaluator.named_integer("no_such_number"), 0);
            assert_eq!(evaluator.named_float("musicselect_position"), 0.25);
            assert_eq!(evaluator.named_float("score_rate"), 0.0, "a slider or graph names a rate, never a float");
            assert_eq!(evaluator.named_text("title"), "FREEDOM DiVE");
            assert_eq!(evaluator.named_text("no_such_text"), "");
            assert_eq!(frame.host().now_us(), NOW_US);
        })
        .expect("the frame binds");
}

#[test]
fn a_scenario_file_drives_the_module_end_to_end() {
    let host: MapHost = serde_json::from_str(
        r#"{
            "booleans": { "151": true },
            "integers": { "96": 11 },
            "texts": { "10": "Scenario" },
            "timers": { "1": 500000 },
            "now_us": 2000000,
            "judges": { "0": 12 },
            "gauge": 20.0,
            "volume_system": 0.5
        }"#,
    )
    .expect("the scenario parses");
    let runtime = runtime();
    let read: (bool, i64, String, f64, i64, f64, f64) = eval(
        &runtime,
        &host,
        "return ms.option('chart_difficulty_1'), ms.number(96), ms.text(10), ms.timer_elapsed_seconds(1), ms.judge(0), ms.gauge(), ms.volume_sys()",
    );
    assert_eq!(read, (true, 11, "Scenario".to_owned(), 1.5, 12, 20.0, 0.5));
}

#[test]
fn every_kind_reads_what_a_function_returned_by_its_own_rule() {
    let (runtime, host) = (runtime(), sample_host());
    for (body, expected) in RETURNED {
        let returned = function(&runtime, &format!("return function() {body} end"), LuaFnKind::Boolean);
        let read = runtime
            .frame(&host, |frame| {
                (frame.call_boolean(returned), frame.call_integer(returned), frame.call_float(returned), frame.call_text(returned), frame.call_timer(returned))
            })
            .expect("the frame binds");
        let (boolean, integer, float, text, timer) = read;
        assert_eq!((boolean, integer, float, text.as_str(), timer), expected, "{body:?}");
    }
    assert!(runtime.diagnostics().function_failures.is_empty(), "no value of any type is an error: {:?}", runtime.diagnostics().function_failures);
}

/// The reference calls a timer function on every read, and it reads a timer twice for each object
/// that follows one. A frame drawn through the evaluator calls it once and reuses the answer.
#[test]
fn a_timer_read_through_the_evaluator_is_called_once_a_frame() {
    let (runtime, host) = (runtime(), sample_host());
    let counted = function(&runtime, "reads = 0 return function() reads = reads + 1 return reads * 1000 end", LuaFnKind::Timer);
    let other = function(&runtime, "others = 0 return function() others = others + 1 return 7 end", LuaFnKind::Timer);
    assert_eq!(runtime.frame_cost().calls, 0, "nothing has been called before the first frame");

    let first = runtime
        .frame(&host, |frame| {
            let evaluator: &dyn LuaDrawEval = frame;
            [evaluator.call_timer(counted), evaluator.call_timer(other), evaluator.call_timer(counted), frame.timer(counted)]
        })
        .expect("the frame binds");
    assert_eq!(first, [1_000, 7, 1_000, 1_000], "every read of the frame is given the one call's answer");
    assert_eq!(eval::<(i64, i64)>(&runtime, &host, "return reads, others"), (1, 1));
    let cost = runtime.frame_cost();
    assert_eq!((cost.calls, cost.reused), (2, 2), "two functions were called and two reads were answered without a call");

    let second = runtime.frame(&host, |frame| [frame.timer(counted), frame.timer(counted)]).expect("the frame binds");
    assert_eq!(second, [2_000, 2_000], "the next frame calls the function again");
    assert_eq!((runtime.frame_cost().calls, runtime.frame_cost().reused), (1, 1), "and counts its own calls from nothing");

    let plain = runtime.frame(&host, |frame| [frame.call_timer(counted), frame.call_timer(counted)]).expect("the frame binds");
    assert_eq!(plain, [3_000, 4_000], "a plain call is a call every time it is made");
    assert_eq!((runtime.frame_cost().calls, runtime.frame_cost().reused), (2, 0));
}

/// What makes one call a frame safe for the timers a skin builds with `timer_observe_boolean`: the
/// observer latches against the frame clock, so the reference's second, third and fourth call of a
/// frame change nothing the first did not. Read the reference's way and read once, the two observers
/// answer the same on every frame.
#[test]
fn an_observed_boolean_answers_the_same_called_once_a_frame_as_called_on_every_read() {
    let runtime = runtime();
    runtime.lua().load("flag = false asked = { every = 0, once = 0 }").exec().expect("the globals are set");
    let every = function(&runtime, "return tu.timer_observe_boolean(function() asked.every = asked.every + 1 return flag end)", LuaFnKind::Timer);
    let once = function(&runtime, "return tu.timer_observe_boolean(function() asked.once = asked.once + 1 return flag end)", LuaFnKind::Timer);
    let mut host = MapHost::new();
    let steps = [(1_000, false), (2_000, true), (3_000, true), (4_000, false), (5_000, false), (6_000, true)];

    let mut answers = Vec::new();
    for (now_us, flag) in steps {
        host.now_us = now_us;
        runtime.lua().globals().set("flag", flag).expect("the flag is set");
        let (reference, reused) = runtime
            .frame(&host, |frame| ([(); READS_OF_TWO_OBJECTS].map(|()| frame.call_timer(every)), [(); READS_OF_TWO_OBJECTS].map(|()| frame.timer(once))))
            .expect("the frame binds");
        assert_eq!(reused, reference, "at {now_us} with the flag {flag}");
        answers.push(reused[0]);
    }
    assert_eq!(answers, [TIMER_OFF, 2_000, 2_000, TIMER_OFF, TIMER_OFF, 6_000]);
    let frames = steps.len() as i64;
    assert_eq!(eval::<(i64, i64)>(&runtime, &host, "return asked.every, asked.once"), (frames * READS_OF_TWO_OBJECTS as i64, frames));
}

#[test]
fn a_timer_that_fails_is_off_for_its_frame_is_logged_once_for_it_and_is_called_again_on_the_next() {
    let (runtime, host) = (runtime(), sample_host());
    runtime.lua().load("attempts = 0").exec().expect("the counter is set");
    let flaky = function(
        &runtime,
        "return function() attempts = attempts + 1 if attempts % 2 == 1 then error('odd attempt') end return ms.timer(41) end",
        LuaFnKind::Timer,
    );

    let failed = runtime.frame(&host, |frame| [frame.timer(flaky), frame.timer(flaky)]).expect("the frame binds");
    assert_eq!(failed, [TIMER_OFF, TIMER_OFF], "the one failed call answers every read of its frame");
    let recovered = runtime.frame(&host, |frame| [frame.timer(flaky), frame.timer(flaky)]).expect("the frame binds");
    assert_eq!(recovered, [TIMER_ON_US, TIMER_ON_US], "the function is not disabled: the next frame calls it again");

    assert_eq!(eval::<i64>(&runtime, &host, "return attempts"), 2, "one call a frame");
    let failures = runtime.diagnostics().function_failures;
    assert_eq!(failures.iter().map(|failure| (failure.function, failure.kind, failure.count)).collect::<Vec<_>>(), [(flaky, LuaFnKind::Timer, 1)]);
    assert!(failures[0].first_message.contains("odd attempt"), "{failures:?}");
}

#[test]
fn a_function_the_budget_refuses_answers_what_it_answered_on_the_frame_before() {
    let budget = LuaBudget { frame: FrameBudget { max_calls: FEW_FRAME_CALLS, ..FrameBudget::default() }, ..LuaBudget::default() };
    let runtime = runtime_with(SkinLuaConfig { budget, ..SkinLuaConfig::new(&fixture_root()) });
    runtime.lua().load("ticks = 0").exec().expect("the counter is set");
    let count = function(&runtime, "return function() ticks = ticks + 1 return ticks end", LuaFnKind::Integer);
    let label = function(&runtime, "return function() return 'tick ' .. ticks end", LuaFnKind::Text);
    let since = function(&runtime, "return function() return ticks * 1000 end", LuaFnKind::Timer);
    let share = function(&runtime, "return function() return ticks / 4 end", LuaFnKind::Float);
    let shown = function(&runtime, "return function() return ticks > 0 end", LuaFnKind::Boolean);
    let host = MapHost::new();

    let first = runtime.frame(&host, |frame| (frame.call_integer(count), frame.call_text(label), frame.timer(since))).expect("the frame binds");
    assert_eq!(first, (1, "tick 1".to_owned(), 1_000));
    assert_eq!(runtime.diagnostics().frames_over_budget, 0);

    let starved = runtime
        .frame(&host, |frame| {
            let spent = [frame.call_integer(count), frame.call_integer(count), frame.call_integer(count)];
            (spent, frame.call_text(label), frame.timer(since), frame.timer(since), frame.call_float(share), frame.call_boolean(shown))
        })
        .expect("the frame binds");
    assert_eq!(
        starved,
        ([2, 3, 4], "tick 1".to_owned(), 1_000, 1_000, 0.0, false),
        "past the ceiling a text and a timer hold what they answered a frame ago, and a function that has never run answers its default"
    );
    assert_eq!((runtime.frame_cost().calls, runtime.frame_cost().reused), (FEW_FRAME_CALLS, 1), "the refused timer was asked for once and reused once");
    assert_eq!(runtime.diagnostics().frames_over_budget, 1);

    let after = runtime.frame(&host, |frame| (frame.call_text(label), frame.timer(since), frame.call_integer(count))).expect("the frame binds");
    assert_eq!(after, ("tick 4".to_owned(), 4_000, 5), "the next frame has its own allowance and reads afresh");
    assert_eq!(runtime.diagnostics().frames_over_budget, 1);
    assert!(runtime.diagnostics().function_failures.is_empty(), "a call that was not made did not fail");
}

#[test]
fn a_call_cut_off_while_it_runs_answers_what_the_function_answered_on_the_frame_before() {
    let budget = LuaBudget { frame: FrameBudget { max_call_instructions: FEW_CALL_INSTRUCTIONS, ..FrameBudget::default() }, ..LuaBudget::default() };
    let runtime = runtime_with(SkinLuaConfig { budget, ..SkinLuaConfig::new(&fixture_root()) });
    runtime.lua().load("spin = false level = 3").exec().expect("the globals are set");
    let value = function(&runtime, "return function() while spin do end return level end", LuaFnKind::Integer);
    let since = function(&runtime, "return function() while spin do end return level * 1000 end", LuaFnKind::Timer);
    let host = MapHost::new();
    let read = || runtime.frame(&host, |frame| (frame.call_integer(value), frame.timer(since), frame.timer(since))).expect("the frame binds");

    assert_eq!(read(), (3, 3_000, 3_000));
    runtime.lua().load("spin = true level = 9").exec().expect("the globals are set");
    assert_eq!(read(), (3, 3_000, 3_000), "a function that ran away is cut off and reads as it did a frame ago");
    assert_eq!(runtime.frame_cost().calls, 2, "the timer that was cut off is not called a second time in the frame");
    assert_eq!(runtime.diagnostics().frames_over_budget, 1);

    runtime.lua().load("spin = false").exec().expect("the global is set");
    assert_eq!(read(), (9, 9_000, 9_000), "and is called again on the next frame");
    assert_eq!(runtime.diagnostics().frames_over_budget, 1);
}

/// A timer function is one entry however many objects and images a skin hangs it on, so one call a
/// frame serves them all, and a function behind a condition that fails is not called at all.
#[test]
fn a_loaded_skin_calls_a_timer_its_objects_share_once_a_frame_and_none_behind_a_failed_condition() {
    let scratch = Scratch::new("shared-timer");
    std::fs::write(scratch.path().join("panel.png"), []).expect("the source file is written");
    let entry = scratch.path().join("shared.luaskin");
    let source = SHARED_TIMER_SKIN.replace("SHARED_ELAPSED_US", &SHARED_ELAPSED_US.to_string()).replace("GATE_OPTION", &GATE_OPTION.to_string());
    std::fs::write(&entry, source).expect("the skin is written");
    let mut host = MapHost::new();
    host.now_us = SHARED_NOW_US;
    let user = SkinUserConfig::default();
    let load = SkinLoadOptions { rng_seed: Some(TEST_SEED), ..SkinLoadOptions::new(scratch.path(), &user, Mode::BEAT_7K) };
    let skin = load_lua_skin(&entry, &LuaSkinOptions::new(load), &host).expect("the skin loads");
    let runtime = skin.runtime().expect("a Lua skin has an interpreter");
    let animated = skin.def.image[0].timer.as_ref().and_then(PropertyRef::timer).expect("the image animates on a timer");
    assert_eq!(skin.destinations[0].track.timer, Some(animated), "the objects and their image name one function, which is one entry");
    assert_eq!(skin.destinations[1].track.timer, Some(animated));
    assert!(matches!(animated, TimerRef::Lua(_)));

    let timers = TimerState::new();
    let called = |runtime: &SkinLua, host: &MapHost| eval::<(i64, i64, i64)>(runtime, host, "return called.shared, called.gate, called.behind");
    let frame_of = |host: &MapHost| {
        runtime
            .frame(host, |frame| {
                let placed: Vec<Option<f32>> = skin
                    .destinations
                    .iter()
                    .map(|named| {
                        let placed = prepare(&named.track, host.now_us, &timers, host, Some(frame), (0.0, 0.0), None);
                        animated.value_us(&timers, Some(frame));
                        placed.map(|resolved| resolved.rect.x)
                    })
                    .collect();
                placed
            })
            .expect("the frame binds")
    };

    assert_eq!(called(runtime, &host), (0, 0, 0), "nothing is called before the first frame");
    assert_eq!(frame_of(&host), [Some(SHARED_X), Some(SHARED_X), None], "both objects are placed by the one answer, and the gated one is left out");
    assert_eq!(called(runtime, &host), (1, 1, 0), "five reads of the shared timer made one call, and the timer behind the failed condition none");
    assert_eq!((runtime.frame_cost().calls, runtime.frame_cost().reused), (2, 4));

    host.booleans.insert(GATE_OPTION, true);
    assert_eq!(frame_of(&host), [Some(SHARED_X), Some(SHARED_X), Some(0.0)]);
    assert_eq!(called(runtime, &host), (2, 2, 1), "each function is called once more on the next frame, the gated timer now among them");
    assert!(runtime.diagnostics().function_failures.is_empty(), "{:?}", runtime.diagnostics().function_failures);
}
