//! The Lua environment a `.luaskin` runs in: the standard library it is given, `require`, `dofile`
//! and `loadfile` confined to the skin root, the budget that stops a runaway skin, and the record
//! of what a skin printed and what its own `pcall` swallowed.
//!
//! Every fixture under `tests/fixtures/luaenv` was written for these tests. The files whose exact
//! bytes matter -- carriage returns, a byte-order mark, bytecode, a symbolic link -- are written
//! into a scratch directory by the test that needs them, so no checkout setting can change them.

#![cfg(feature = "lua")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

use mlua::{FromLuaMulti, Function, Table, Value};
use rbms_skin::SkinError;
use rbms_skin::dst::SkinOffset;
use rbms_skin::lua::{FrameBudget, LoadBudget, LuaBudget, LuaFnKind, LuaMode, LuaPass, SkinConfigGlobal, SkinLua, SkinLuaConfig, SkinPaths, error_location};
use rbms_skin::property::DefaultState;
use rbms_skin::timer::TIMER_OFF;

/// The seed the fixtures' random draws are pinned to.
const TEST_SEED: u64 = 42;

/// Another seed, for showing that the sequence follows the seed.
const OTHER_SEED: u64 = 43;

/// How many draws a sequence comparison takes.
const DRAWS: usize = 32;

/// Instructions a budget test lets one pass execute: far more than any fixture that ends needs, and
/// few enough that a loop that does not end is cut off at once.
const SMALL_LOAD_INSTRUCTIONS: u64 = 200_000;

/// Wall clock a budget test lets one pass run for. The instruction ceiling is met long before.
const SMALL_LOAD_MICROS: u64 = 30_000_000;

/// Bytes a memory test lets the interpreter hold.
const SMALL_MEMORY_BYTES: usize = 8 * 1024 * 1024;

/// Instructions a budget test lets one call of a frame execute.
const SMALL_CALL_INSTRUCTIONS: u64 = 50_000;

/// Calls a budget test lets one frame make.
const SMALL_FRAME_CALLS: u32 = 3;

/// Wall clock a clock test lets the calls of one frame spend.
const SMALL_FRAME_MICROS: u64 = 5_000;

/// How long a clock test's host is busy between two calls: several times what the frame's calls may
/// spend.
const HOST_WORK: Duration = Duration::from_millis(25);

/// How long a test waits for work that only a broken budget would let run for ever.
const HANG_TIMEOUT: Duration = Duration::from_secs(60);

/// The environment variable that names an external skin pack for the optional tests.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// The extension of a Lua skin's entry file.
const LUA_SKIN_EXTENSION: &str = "luaskin";

/// The fixture skin's root.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("luaenv")
}

/// A scratch directory that removes itself, for the tests that need files written on the spot.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-luaenv-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Writes `bytes` to `relative` under the scratch directory, creating its parents.
    fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("a scratch file has a parent")).expect("the parent should be creatable");
        std::fs::write(&path, bytes).expect("the scratch file should be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A seeded, full-mode interpreter for the skin at `root`.
fn runtime_at(root: &Path) -> SkinLua {
    SkinLua::new(SkinLuaConfig { seed: Some(TEST_SEED), ..SkinLuaConfig::new(root) }).expect("the runtime builds")
}

/// A seeded interpreter for the fixture skin.
fn runtime() -> SkinLua {
    runtime_at(&fixture_root())
}

/// A seeded interpreter for the fixture skin, held to `budget`.
fn runtime_with(budget: LuaBudget) -> SkinLua {
    SkinLua::new(SkinLuaConfig { seed: Some(TEST_SEED), budget, ..SkinLuaConfig::new(&fixture_root()) }).expect("the runtime builds")
}

/// A budget whose load ceiling a loop that never ends meets at once.
fn small_load_budget() -> LuaBudget {
    LuaBudget { load: LoadBudget { max_instructions: SMALL_LOAD_INSTRUCTIONS, max_micros: SMALL_LOAD_MICROS }, ..LuaBudget::default() }
}

/// Runs a chunk in the interpreter and reads what it returns.
fn eval<T: FromLuaMulti>(runtime: &SkinLua, source: &str) -> T {
    runtime.lua().load(source).eval().unwrap_or_else(|error| panic!("{source} should run: {error}"))
}

/// A function value made by a chunk that returns one.
fn function(runtime: &SkinLua, source: &str) -> Function {
    eval(runtime, source)
}

/// Runs `work` on a thread of its own and fails the test, instead of hanging it, when the work has
/// not finished in [`HANG_TIMEOUT`]. For the cases whose failure is a loop no budget reaches.
fn finishes<T: Send + 'static>(what: &str, work: impl FnOnce() -> T + Send + 'static) -> T {
    let (done, finished) = channel();
    std::thread::spawn(move || {
        let _ = done.send(work());
    });
    match finished.recv_timeout(HANG_TIMEOUT) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => panic!("{what} did not finish in {HANG_TIMEOUT:?}"),
        Err(RecvTimeoutError::Disconnected) => panic!("{what} failed"),
    }
}

/// Every file under `root`, as paths relative to it.
fn files_under(root: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the directory should be listable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_path_buf());
            }
        }
    }
    found
}

#[test]
fn the_standard_library_is_the_one_the_reference_gives_a_skin() {
    let runtime = runtime();
    let present = [
        "assert",
        "collectgarbage",
        "dofile",
        "error",
        "getmetatable",
        "ipairs",
        "load",
        "loadfile",
        "next",
        "pairs",
        "pcall",
        "print",
        "rawequal",
        "rawget",
        "rawlen",
        "rawset",
        "require",
        "select",
        "setmetatable",
        "tonumber",
        "tostring",
        "type",
        "xpcall",
        "table.concat",
        "table.insert",
        "table.pack",
        "table.remove",
        "table.sort",
        "table.unpack",
        "string.format",
        "string.find",
        "string.gmatch",
        "string.gsub",
        "string.match",
        "string.rep",
        "string.sub",
        "math.floor",
        "math.modf",
        "math.pow",
        "math.atan2",
        "math.random",
        "math.randomseed",
        "bit32.band",
        "bit32.rshift",
        "coroutine.create",
        "coroutine.wrap",
        "package.loaded",
        "package.preload",
        "package.searchers",
        "package.searchpath",
    ];
    for name in present {
        assert!(eval::<bool>(&runtime, &format!("return {name} ~= nil")), "{name} should exist");
    }
    for name in ["unpack", "loadstring", "setfenv", "getfenv", "module", "string.dump", "package.loadlib", "math.log10", "debug.traceback", "debug.sethook"] {
        assert!(eval::<bool>(&runtime, &format!("return {name} == nil")), "{name} should not exist");
    }
    assert!(eval::<bool>(&runtime, "return debug.getmetatable == getmetatable and require('debug') == debug"));
    assert!(eval::<bool>(&runtime, "return require('string') == string and require('_G') == _G and package.cpath == ''"));
    assert_eq!(eval::<String>(&runtime, "return _VERSION"), "Lua 5.2");
}

#[test]
fn a_required_module_runs_once_and_is_cached() {
    let runtime = runtime();
    let same: bool =
        eval(&runtime, "local first = require('mods.counter') return first == require('mods.counter') and package.loaded['mods.counter'] == first");
    assert!(same, "the second require answers the cached table");
    assert_eq!(eval::<i64>(&runtime, "return counter_loads"), 1);
}

#[test]
fn require_reads_dots_as_directories_and_passes_the_name_and_file() {
    let runtime = runtime();
    let (name, file): (String, String) = eval(&runtime, "local leaf = require('mods.sub.leaf') return leaf.name, leaf.file");
    assert_eq!(name, "mods.sub.leaf");
    assert!(file.ends_with("/mods/sub/leaf.lua"), "got {file}");
    assert!(Path::new(&file).is_absolute(), "the file is named by its absolute path");
}

#[test]
fn a_module_that_returns_nothing_is_cached_as_true() {
    let runtime = runtime();
    assert!(eval::<bool>(&runtime, "return require('mods.silent') == true and silent_ran == true and package.loaded['mods.silent'] == true"));
}

#[test]
fn a_missing_or_broken_module_raises_a_string_a_skin_can_catch() {
    let runtime = runtime();
    let (ok, message): (bool, String) = eval(&runtime, "return pcall(require, 'mods.absent')");
    assert!(!ok);
    assert!(message.contains("module 'mods.absent' not found"), "got {message}");
    assert!(message.contains("no field package.preload['mods.absent']"), "got {message}");

    let (ok, message): (bool, String) = eval(&runtime, "return pcall(require, 'mods.broken')");
    assert!(!ok);
    assert!(message.contains("error loading module 'mods.broken'"), "got {message}");
    assert!(message.contains("mods/broken.lua:"), "got {message}");

    let (ok, message): (bool, String) = eval(&runtime, "return pcall(require, 'mods.raises')");
    assert!(!ok);
    assert!(message.starts_with("mods/raises.lua:3:"), "got {message}");
    assert!(eval::<bool>(&runtime, "return package.loaded['mods.raises'] == nil"), "a module that raised is not cached");
}

#[test]
fn the_injected_modules_are_required_without_a_file() {
    let runtime = runtime();
    assert!(eval::<bool>(
        &runtime,
        "return type(require('main_state')) == 'table' and type(require('timer_util')) == 'table' and type(require('event_util')) == 'table'"
    ));
}

#[test]
fn changing_the_search_path_does_not_widen_the_search() {
    let scratch = Scratch::new("search-path");
    scratch.write("skin/inside.lua", b"return 'inside'");
    let outside = scratch.write("outside/elsewhere.lua", b"escaped = true return 'outside'");
    let runtime = runtime_at(&scratch.path().join("skin"));
    runtime
        .lua()
        .globals()
        .set("outside_template", format!("{}/?.lua", outside.parent().expect("the file has a parent").display()))
        .expect("the global is set");

    let (ok, message): (bool, String) = eval(&runtime, "package.path = outside_template return pcall(require, 'elsewhere')");
    assert!(!ok, "got {message}");
    assert!(eval::<bool>(&runtime, "return escaped == nil and require('inside') == 'inside'"));
    assert!(eval::<bool>(
        &runtime,
        "return package.searchpath('elsewhere', outside_template) == nil and package.searchpath('inside', '?.lua') == 'inside.lua'"
    ));
}

#[test]
fn a_require_that_names_a_file_outside_the_root_is_refused() {
    let scratch = Scratch::new("require-escape");
    scratch.write("skin/inside.lua", b"return true");
    scratch.write("outside.lua", b"escaped = true return true");
    let runtime = runtime_at(&scratch.path().join("skin"));

    for name in ["..outside", "../outside", "..\\\\outside", "/outside"] {
        let (ok, message): (bool, String) = eval(&runtime, &format!("return pcall(require, '{name}')"));
        assert!(!ok, "{name} should not load");
        assert!(message.contains("not found"), "{name}: got {message}");
    }
    assert!(eval::<bool>(&runtime, "return escaped == nil"), "nothing outside the root ran");
}

#[test]
fn dofile_and_loadfile_refuse_a_path_outside_the_root() {
    let scratch = Scratch::new("dofile-escape");
    scratch.write("skin/inside.lua", b"return 'inside'");
    let outside = scratch.write("outside.lua", b"escaped = true return 'outside'");
    let runtime = runtime_at(&scratch.path().join("skin"));
    runtime.lua().globals().set("outside_path", outside.to_string_lossy().into_owned()).expect("the global is set");

    for path in ["'../outside.lua'", "'sub/../../outside.lua'", "'..\\\\outside.lua'", "outside_path"] {
        let (ok, message): (bool, String) = eval(&runtime, &format!("return pcall(dofile, {path})"));
        assert!(!ok, "dofile({path}) should fail");
        assert!(message.contains("outside the skin root"), "dofile({path}): got {message}");

        let (chunk, message): (Value, String) = eval(&runtime, &format!("return loadfile({path})"));
        assert!(chunk.is_nil(), "loadfile({path}) should answer nil");
        assert!(message.contains("outside the skin root"), "loadfile({path}): got {message}");
    }
    assert!(eval::<bool>(&runtime, "return escaped == nil and dofile('sub/../inside.lua') == 'inside'"), "a path that stays inside is fine");
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_out_of_the_root_is_refused() {
    let scratch = Scratch::new("symlink-escape");
    scratch.write("skin/inside.lua", b"return 'inside'");
    scratch.write("outside/module.lua", b"escaped = true return 'outside'");
    scratch.write("skin/real/module.lua", b"return 'linked inside'");
    std::os::unix::fs::symlink(scratch.path().join("outside"), scratch.path().join("skin").join("leak")).expect("the link is created");
    std::os::unix::fs::symlink(scratch.path().join("skin").join("real"), scratch.path().join("skin").join("alias")).expect("the link is created");
    let runtime = runtime_at(&scratch.path().join("skin"));

    let (ok, message): (bool, String) = eval(&runtime, "return pcall(require, 'leak.module')");
    assert!(!ok && message.contains("outside the skin root"), "got {message}");
    let (ok, message): (bool, String) = eval(&runtime, "return pcall(dofile, 'leak/module.lua')");
    assert!(!ok && message.contains("outside the skin root"), "got {message}");
    assert!(eval::<bool>(&runtime, "return loadfile('leak/module.lua') == nil and escaped == nil"));
    assert!(matches!(runtime.paths().script("leak/module.lua"), Err(SkinError::PathEscape(_))));

    assert_eq!(eval::<String>(&runtime, "return dofile('alias/module.lua')"), "linked inside", "a link that stays inside the root is followed");
}

#[test]
fn dofile_takes_an_absolute_path_keeps_no_cache_and_shares_the_globals() {
    let runtime = runtime();
    let absolute = format!("{}/parts/part.lua", runtime.paths().root().to_string_lossy().replace('\\', "/"));
    runtime.lua().globals().set("absolute_part", absolute).expect("the global is set");

    assert_eq!(eval::<i64>(&runtime, "return dofile(absolute_part).value"), 1);
    assert_eq!(eval::<i64>(&runtime, "return dofile('parts/part.lua').value"), 2, "the file runs again on every call");
    assert_eq!(eval::<i64>(&runtime, "return part_runs"), 2, "the chunk wrote the shared globals");
    assert_eq!(eval::<i64>(&runtime, "return dofile(absolute_part).load()"), 3);
}

#[test]
fn loadfile_compiles_without_running_and_honours_an_environment() {
    let runtime = runtime();
    assert!(eval::<bool>(&runtime, "local chunk = loadfile('parts/part.lua') return type(chunk) == 'function' and part_runs == nil"));
    assert_eq!(eval::<i64>(&runtime, "local env = {} loadfile('parts/part.lua', 't', env)() return env.part_runs"), 1);
    assert!(eval::<bool>(&runtime, "return part_runs == nil"), "the chunk wrote the environment it was given");

    let (chunk, message): (Value, String) = eval(&runtime, "return loadfile('parts/absent.lua')");
    assert!(chunk.is_nil());
    assert!(message.starts_with("cannot open parts/absent.lua"), "got {message}");

    let (chunk, message): (Value, String) = eval(&runtime, "return loadfile('mods/broken.lua')");
    assert!(chunk.is_nil());
    assert!(message.starts_with("mods/broken.lua:"), "a syntax error names the file and line: {message}");
}

#[test]
fn bytecode_is_refused_everywhere_and_cannot_be_made() {
    let scratch = Scratch::new("bytecode");
    let plain = mlua::Lua::new();
    let dumped = plain.load("leaked = true return 7").into_function().expect("the sample compiles").dump(false);
    assert!(dumped.starts_with(b"\x1bLua"), "the sample is a binary chunk");
    scratch.write("skin/binary.lua", &dumped);
    scratch.write("skin/text.lua", b"return 7");
    let runtime = runtime_at(&scratch.path().join("skin"));

    for source in
        ["load(dumped)", "load(dumped, 'chunk', 'b')", "load(dumped, 'chunk', 'bt')", "load(function() local piece = dumped dumped = nil return piece end)"]
    {
        runtime.lua().globals().set("dumped", runtime.lua().create_string(&dumped).expect("the string is created")).expect("the global is set");
        let (chunk, message): (Value, String) = eval(&runtime, &format!("return {source}"));
        assert!(chunk.is_nil(), "{source} should answer nil");
        assert!(message.contains("binary"), "{source}: got {message}");
    }
    let (chunk, message): (Value, String) = eval(&runtime, "return loadfile('binary.lua')");
    assert!(chunk.is_nil() && message.contains("binary"), "got {message}");
    let (ok, message): (bool, String) = eval(&runtime, "return pcall(dofile, 'binary.lua')");
    assert!(!ok && message.contains("binary"), "got {message}");
    let (ok, message): (bool, String) = eval(&runtime, "return pcall(require, 'binary')");
    assert!(!ok && message.contains("binary"), "got {message}");
    assert!(matches!(runtime.run_entry(Path::new("binary.lua"), LuaPass::Header, &DefaultState), Err(SkinError::LuaLoad { .. })));

    assert!(eval::<bool>(&runtime, "return leaked == nil and string.dump == nil and ('').dump == nil"));
    assert_eq!(eval::<i64>(&runtime, "return load('return 7')() + dofile('text.lua')"), 14, "text chunks still load");
    assert_eq!(eval::<i64>(&runtime, "local env = { seven = 7 } return load('return seven', 'chunk', 't', env)()"), 7);
}

#[test]
fn carriage_returns_and_a_byte_order_mark_do_not_break_a_file() {
    let scratch = Scratch::new("crlf-bom");
    scratch.write("skin/crlf.lua", b"local parts = {}\r\nparts.count = 3\r\n\r\nreturn parts\r\n");
    scratch.write("skin/crlf_fails.lua", b"local parts = nil\r\n\r\nlocal text = [[one\r\ntwo]]\r\nreturn parts.destination\r\n");
    scratch.write("skin/bom.lua", b"\xef\xbb\xbflocal label = 'marked'\nreturn { label = label }\n");
    scratch.write(
        "skin/both.luaskin",
        b"\xef\xbb\xbflocal crlf = require('crlf')\r\nlocal bom = dofile('bom.lua')\r\nreturn { count = crlf.count, label = bom.label }\r\n",
    );
    let runtime = runtime_at(&scratch.path().join("skin"));

    assert_eq!(eval::<i64>(&runtime, "return require('crlf').count"), 3);
    assert_eq!(eval::<String>(&runtime, "return require('bom').label"), "marked");
    assert_eq!(eval::<String>(&runtime, "return dofile('bom.lua').label"), "marked");

    let (ok, message): (bool, String) = eval(&runtime, "return pcall(dofile, 'crlf_fails.lua')");
    assert!(!ok);
    assert!(message.starts_with("crlf_fails.lua:5:"), "a carriage return and line feed count as one line: {message}");

    let Value::Table(skin) = runtime.run_entry(&scratch.path().join("skin").join("both.luaskin"), LuaPass::Header, &DefaultState).expect("the entry file runs")
    else {
        panic!("the entry file returns a table");
    };
    assert_eq!(skin.get::<i64>("count").expect("count is set"), 3);
    assert_eq!(skin.get::<String>("label").expect("label is set"), "marked");
}

#[test]
fn the_pattern_functions_work() {
    let runtime = runtime();
    assert_eq!(eval::<String>(&runtime, "return string.match('skin/Play/parts/frame.png', 'Play.+%.png')"), "Play/parts/frame.png");
    assert_eq!(eval::<(String, String)>(&runtime, "return string.match('key=value', '(%w+)=(%w+)')"), ("key".to_owned(), "value".to_owned()));
    assert_eq!(eval::<(String, i64)>(&runtime, "return string.gsub('a-b-c', '%-', '/')"), ("a/b/c".to_owned(), 2));
    assert_eq!(eval::<String>(&runtime, "return (string.gsub('hello world', '(%w+)', '<%1>'))"), "<hello> <world>");
    assert_eq!(
        eval::<String>(
            &runtime,
            "local words = {} for word in string.gmatch('one two  three', '%a+') do words[#words + 1] = word end return table.concat(words, ',')"
        ),
        "one,two,three"
    );
    assert_eq!(eval::<(i64, i64)>(&runtime, "return string.find('timer_util', 'util', 1, true)"), (7, 10));
    assert_eq!(eval::<String>(&runtime, "return ('%02d:%02d'):format(7, 5) .. ('x'):rep(3)"), "07:05xxx");
}

#[test]
fn a_whole_number_is_written_without_a_fraction() {
    let runtime = runtime();
    assert!(eval::<bool>(&runtime, "return 10 / 2 .. '' == '5'"));
    assert_eq!(eval::<String>(&runtime, "return tostring(1920 / 2)"), "960");
    assert_eq!(eval::<String>(&runtime, "return 2 ^ 10 .. ''"), "1024");
    assert_eq!(eval::<String>(&runtime, "return 'src' .. 27 * 12"), "src324");
    assert_eq!(eval::<String>(&runtime, "return tostring(7 / 2)"), "3.5");
    assert_eq!(eval::<String>(&runtime, "return type(1) .. ':' .. math.floor(3.7)"), "number:3");
    assert!(eval::<bool>(&runtime, "local off = -2 ^ 63 return 0 - off > 0 and off - 1 == off"), "the off value does not wrap");
    assert!(eval::<bool>(&runtime, "return 5 % 0 ~= 5 % 0"), "a modulo by zero is not a number rather than an error");
}

#[test]
fn an_error_a_skin_swallows_is_recorded_and_pcall_still_answers_as_usual() {
    let runtime = runtime();
    let (ok, message): (bool, String) = eval(&runtime, "return pcall(dofile, 'parts/fails.lua')");
    assert!(!ok);
    assert!(message.starts_with("parts/fails.lua:3: attempt to index"), "got {message}");

    let swallowed = runtime.diagnostics().swallowed;
    assert_eq!(swallowed.len(), 1);
    assert_eq!(swallowed[0].message, message);
    assert_eq!(swallowed[0].file.as_deref(), Some("parts/fails.lua"));
    assert_eq!(swallowed[0].line, Some(3));
    assert_eq!(swallowed[0].count, 1);

    let part: Table = eval(&runtime, "return dofile('parts/guarded.lua')");
    assert!(!part.get::<bool>("status").expect("status is set"), "the part's own pcall reported the failure");
    assert_eq!(part.get::<String>("part").expect("the message is kept"), message);
    assert_eq!(runtime.diagnostics().swallowed[0].count, 2, "the same message is counted, not stored again");
}

#[test]
fn pcall_and_xpcall_keep_their_usual_results() {
    let runtime = runtime();
    assert_eq!(eval::<(bool, i64, i64, i64)>(&runtime, "return pcall(function(a, b) return a, b, a + b end, 1, 2)"), (true, 1, 2, 3));
    assert_eq!(eval::<i64>(&runtime, "return select('#', pcall(function() end))"), 1, "a call that returns nothing answers only true");
    assert_eq!(eval::<i64>(&runtime, "return select('#', pcall(error, 'boom'))"), 2);
    assert!(
        eval::<bool>(&runtime, "local raised = { code = 7 } local ok, caught = pcall(error, raised) return ok == false and caught == raised"),
        "a table is handed back as it was raised"
    );
    assert!(eval::<bool>(&runtime, "local ok, caught = pcall(error) return ok == false and caught == nil"));
    assert_eq!(eval::<(bool, String)>(&runtime, "return pcall(error, 'plain', 0)"), (false, "plain".to_owned()));
    assert_eq!(
        eval::<(bool, String)>(&runtime, "return xpcall(function() error('deep', 0) end, function(message) return 'handled ' .. message end)"),
        (false, "handled deep".to_owned())
    );
    assert_eq!(eval::<(bool, i64)>(&runtime, "return xpcall(function(a, b) return a * b end, print, 6, 7)"), (true, 42));
    assert_eq!(
        eval::<String>(
            &runtime,
            "local resumed = coroutine.wrap(function() local ok, value = pcall(function() return coroutine.yield('first') end) return value end) return resumed() .. resumed('second')"
        ),
        "firstsecond",
        "a coroutine still yields across pcall"
    );

    let diagnostics = runtime.diagnostics();
    let messages: Vec<&str> = diagnostics.swallowed.iter().map(|error| error.message.as_str()).collect();
    assert_eq!(messages, ["boom", "(error object is a table value)", "(error object is a nil value)", "plain", "deep"]);
    assert!(diagnostics.swallowed.iter().all(|error| error.file.is_none() && error.line.is_none()), "none of these messages carries a position");
}

#[test]
fn a_coroutine_that_dies_is_recorded_and_resume_still_answers_as_usual() {
    let runtime = runtime();
    assert_eq!(
        eval::<(bool, i64, i64)>(
            &runtime,
            "local thread = coroutine.create(function(a) local b = coroutine.yield(a + 1) return a + b end) local _, first = coroutine.resume(thread, 1) return true, first, select(2, coroutine.resume(thread, 10))"
        ),
        (true, 2, 11)
    );
    let (ok, message): (bool, String) = eval(&runtime, "return coroutine.resume(coroutine.create(function() return dofile('parts/fails.lua') end))");
    assert!(!ok);
    assert!(message.starts_with("parts/fails.lua:3:"), "got {message}");
    assert_eq!(
        eval::<(bool, String)>(&runtime, "local thread = coroutine.create(function() end) coroutine.resume(thread) return coroutine.resume(thread)"),
        (false, "cannot resume dead coroutine".to_owned())
    );

    let swallowed = runtime.diagnostics().swallowed;
    assert_eq!(swallowed.len(), 2);
    assert_eq!((swallowed[0].file.as_deref(), swallowed[0].line), (Some("parts/fails.lua"), Some(3)));
    assert_eq!(swallowed[1].message, "cannot resume dead coroutine");
}

#[test]
fn an_error_raised_by_a_rust_function_reaches_the_skin_as_a_string() {
    let runtime = runtime();
    let (ok, kind, message): (bool, String, String) = eval(&runtime, "local ok, raised = pcall(math.random, 0) return ok, type(raised), raised");
    assert!(!ok);
    assert_eq!(kind, "string");
    assert_eq!(message, "bad argument #1 to 'random' (interval is empty)");
    let handled: String = eval(
        &runtime,
        "local _, text = xpcall(function() return math.random(5, 1) end, function(raised) return type(raised) .. ': ' .. raised end) return text",
    );
    assert_eq!(handled, "string: bad argument #2 to 'random' (interval is empty)");
    let (chunk, kind, message): (Value, String, String) =
        eval(&runtime, "local chunk, raised = load(function() return io.lines('/etc/passwd') end) return chunk, type(raised), raised");
    assert!(chunk.is_nil());
    assert_eq!(kind, "string", "a reader that fails inside a Rust function is reported as a string too");
    assert_eq!(message, "io error: Lua skin file access denied: /etc/passwd");
}

#[test]
fn setmetatable_keeps_its_usual_results() {
    let runtime = runtime();
    assert!(eval::<bool>(&runtime, "local object, metatable = {}, {} return setmetatable(object, metatable) == object and getmetatable(object) == metatable"));
    assert!(eval::<bool>(&runtime, "return getmetatable(setmetatable(setmetatable({}, {}), nil)) == nil"));
    assert!(eval::<bool>(&runtime, "return setmetatable({}, { __index = function(_, key) return key == 'x' end }).x"));
    let (ok, message, kept): (bool, String, bool) = eval(
        &runtime,
        "local locked = setmetatable({}, { __metatable = 'locked' }) local finalized = { __gc = print } \
         local ok, raised = pcall(setmetatable, locked, finalized) return ok, raised, finalized.__gc == print",
    );
    assert!(!ok);
    assert_eq!(message, "cannot change a protected metatable");
    assert!(kept, "the metatable is handed back as it was given");
    let (ok, message): (bool, String) = eval(&runtime, "return pcall(setmetatable, 1, {})");
    assert!(!ok);
    assert!(message.contains("table expected"), "got {message}");
}

#[test]
fn a_repeated_error_is_kept_once_with_its_count() {
    let runtime = runtime();
    eval::<()>(&runtime, "for _ = 1, 50 do pcall(dofile, 'parts/fails.lua') pcall(error, 'again', 0) end");
    let swallowed = runtime.diagnostics().swallowed;
    assert_eq!(swallowed.len(), 2);
    assert_eq!((swallowed[0].line, swallowed[0].count), (Some(3), 50));
    assert_eq!((swallowed[1].message.as_str(), swallowed[1].count), ("again", 50));
    assert_eq!(runtime.diagnostics().swallowed_overflow, 0);
}

#[test]
fn an_error_message_gives_up_its_file_and_line() {
    assert_eq!(error_location("Root/define.lua:12: attempt to call a nil value"), Some(("Root/define.lua", 12)));
    assert_eq!(error_location("[skin script]:1: unexpected symbol"), Some(("[skin script]", 1)));
    assert_eq!(error_location("Play/lua/a b.lua:7: boom: 3: x"), Some(("Play/lua/a b.lua", 7)));
    assert_eq!(error_location("boom"), None);
    assert_eq!(error_location("ratio 4:3 is wrong"), None);
    assert_eq!(error_location(""), None);
}

#[test]
fn print_goes_to_the_diagnostics() {
    let runtime = runtime();
    eval::<()>(&runtime, "print('loaded', 3, nil, true, 10 / 4) print() print(setmetatable({}, { __tostring = function() return 'custom' end }))");
    eval::<()>(&runtime, "for _ = 1, 20 do print('every frame') end");
    let diagnostics = runtime.diagnostics();
    let printed: Vec<(&str, u64)> = diagnostics.prints.iter().map(|line| (line.text.as_str(), line.count)).collect();
    assert_eq!(printed, [("loaded\t3\tnil\ttrue\t2.5", 1), ("", 1), ("custom", 1), ("every frame", 20)]);
    assert_eq!(diagnostics.prints_overflow, 0);
    assert!(diagnostics.swallowed.is_empty() && diagnostics.function_failures.is_empty());
}

#[test]
fn the_same_seed_draws_the_same_sequence() {
    let draw = |runtime: &SkinLua| -> Vec<i64> { (0..DRAWS).map(|_| eval(runtime, "return math.random(1000000)")).collect() };
    let seeded = |seed: u64| SkinLua::new(SkinLuaConfig { seed: Some(seed), ..SkinLuaConfig::new(&fixture_root()) }).expect("the runtime builds");

    let first = seeded(TEST_SEED);
    let second = seeded(TEST_SEED);
    let other = seeded(OTHER_SEED);
    let mut interleaved_first = Vec::new();
    let mut interleaved_second = Vec::new();
    for _ in 0..DRAWS {
        interleaved_first.push(eval::<i64>(&first, "return math.random(1000000)"));
        eval::<f64>(&other, "return math.random()");
        interleaved_second.push(eval::<i64>(&second, "return math.random(1000000)"));
    }
    assert_eq!(interleaved_first, interleaved_second, "two interpreters with one seed agree whatever else draws in between");
    assert_eq!(draw(&seeded(TEST_SEED)), interleaved_first, "a fresh interpreter repeats the sequence");
    assert_ne!(draw(&seeded(OTHER_SEED)), interleaved_first, "another seed draws another sequence");

    eval::<()>(&first, "math.randomseed(7)");
    eval::<()>(&second, "math.randomseed(7)");
    assert_eq!(draw(&first), draw(&second), "a skin that seeds itself is repeatable too");
}

#[test]
fn random_draws_stay_inside_the_bounds_they_were_given() {
    let runtime = runtime();
    assert!(eval::<bool>(&runtime, "for _ = 1, 500 do local value = math.random() if value < 0 or value >= 1 then return false end end return true"));
    assert!(eval::<bool>(
        &runtime,
        "for _ = 1, 500 do local value = math.random(6) if value < 1 or value > 6 or value % 1 ~= 0 then return false end end return true"
    ));
    assert!(eval::<bool>(
        &runtime,
        "for _ = 1, 500 do local value = math.random(-2, 2) if value < -2 or value > 2 or value % 1 ~= 0 then return false end end return true"
    ));
    assert!(eval::<bool>(
        &runtime,
        "local seen = {} for _ = 1, 500 do seen[math.random(3)] = true end return seen[1] and seen[2] and seen[3] and seen[4] == nil"
    ));
    assert_eq!(eval::<i64>(&runtime, "return math.random(1) + math.random(5, 5)"), 6);
    assert!(eval::<bool>(&runtime, "for _ = 1, 500 do if math.random(3.9) > 3 then return false end end return true"), "a fractional bound is truncated");
    assert_eq!(eval::<String>(&runtime, "return tostring(math.random(4, 4))"), "4", "a draw is a whole number");
}

#[test]
fn an_unseeded_interpreter_still_draws() {
    let runtime = SkinLua::new(SkinLuaConfig::new(&fixture_root())).expect("the runtime builds");
    assert!(eval::<bool>(&runtime, "local value = math.random(10) return value >= 1 and value <= 10"));
}

#[test]
fn the_two_passes_share_one_interpreter_and_the_second_reads_skin_config() {
    let runtime = runtime();
    let entry = fixture_root().join("entry.luaskin");

    let Value::Table(header) = runtime.run_entry(&entry, LuaPass::Header, &DefaultState).expect("the header pass runs") else {
        panic!("the header pass returns a table");
    };
    assert_eq!(header.get::<String>("name").expect("the header has a name"), "luaenv fixture");
    assert!(header.get::<Value>("selected").expect("the field is readable").is_nil(), "the header pass did not build the skin");
    assert!(eval::<bool>(&runtime, "return skin_config == nil"));

    let root = runtime.paths().root().to_path_buf();
    runtime
        .set_skin_config(SkinConfigGlobal {
            options: vec![("Backdrop".to_owned(), 901), ("Unused".to_owned(), 910)],
            file_paths: vec![("Frame".to_owned(), "plain.txt".to_owned())],
            offsets: vec![("Title".to_owned(), SkinOffset { x: 12.0, y: -4.0, w: 0.0, h: 0.0, r: 0.0, a: 0.0 })],
            get_path: Box::new(move |relative| format!("{}/{relative}", root.to_string_lossy())),
        })
        .expect("skin_config is published");

    let Value::Table(skin) = runtime.run_entry(&entry, LuaPass::Body, &DefaultState).expect("the body pass runs") else {
        panic!("the body pass returns a table");
    };
    assert_eq!(skin.get::<i64>("selected").expect("the option is read"), 901);
    assert_eq!(skin.get::<Vec<i64>>("enabled").expect("the list is read"), [901, 910]);
    assert_eq!(skin.get::<String>("frame").expect("the file choice is read"), "plain.txt");
    assert_eq!(skin.get::<String>("title_x").expect("the offset is read"), "12");
    assert_eq!(skin.get::<i64>("title_fields").expect("the offset is counted"), 6);
    assert!(skin.get::<String>("resolved").expect("the path is resolved").ends_with("/data/notes.txt"));
    assert_eq!(skin.get::<i64>("part").expect("the part ran"), 1);
    assert_eq!(skin.get::<i64>("loads").expect("the count is read"), 1, "the module the header pass loaded was not run again");
}

#[test]
fn an_entry_file_reports_the_file_and_line_it_failed_on() {
    let runtime = runtime();
    let outcome = runtime.run_entry(Path::new("parts/fails.lua"), LuaPass::Header, &DefaultState);
    let Err(SkinError::LuaLoad { path, message }) = outcome else {
        panic!("the pass should fail");
    };
    assert!(path.contains("header pass"), "got {path}");
    assert!(message.starts_with("parts/fails.lua:3: attempt to index"), "got {message}");

    let outcome = runtime.run_entry(Path::new("../secret.txt"), LuaPass::Header, &DefaultState);
    assert!(matches!(&outcome, Err(SkinError::LuaLoad { message, .. }) if message.contains("outside the skin root")), "got {outcome:?}");
}

#[test]
fn a_pass_within_its_budget_runs_and_the_next_one_starts_afresh() {
    let runtime = runtime_with(small_load_budget());
    for pass in [LuaPass::Header, LuaPass::Body, LuaPass::Body] {
        let Value::Table(result) = runtime.run_entry(Path::new("budget/modest.luaskin"), pass, &DefaultState).expect("the pass runs") else {
            panic!("the pass returns a table");
        };
        assert_eq!(result.get::<i64>("total").expect("the total is set"), 500_500);
    }
}

#[test]
fn a_loop_that_never_ends_is_cut_off() {
    let runtime = runtime_with(small_load_budget());
    let outcome = runtime.run_entry(Path::new("budget/spin.luaskin"), LuaPass::Header, &DefaultState);
    assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "got {outcome:?}");

    let outcome = runtime.run_entry(Path::new("budget/modest.luaskin"), LuaPass::Header, &DefaultState);
    assert!(outcome.is_ok(), "the interpreter is usable after an overrun: {outcome:?}");
}

#[test]
fn a_skin_cannot_catch_its_way_past_the_budget() {
    for entry in [
        "budget/guarded_spin.luaskin",
        "budget/handled_spin.luaskin",
        "budget/coroutine_spin.luaskin",
        "budget/reader_spin.luaskin",
        "budget/nested_spin.luaskin",
    ] {
        let runtime = runtime_with(small_load_budget());
        let outcome = runtime.run_entry(Path::new(entry), LuaPass::Header, &DefaultState);
        assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "{entry}: got {outcome:?}");
        assert!(runtime.diagnostics().swallowed.is_empty(), "{entry}: the budget error is not a swallowed error");
    }
}

#[test]
fn the_wall_clock_cuts_a_pass_off_too() {
    let budget = LuaBudget { load: LoadBudget { max_instructions: u64::MAX, max_micros: 20_000 }, ..LuaBudget::default() };
    let runtime = runtime_with(budget);
    let outcome = runtime.run_entry(Path::new("budget/spin.luaskin"), LuaPass::Header, &DefaultState);
    assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "got {outcome:?}");
}

#[test]
fn a_skin_that_hoards_memory_is_cut_off() {
    for entry in ["budget/hoard.luaskin", "budget/guarded_hoard.luaskin"] {
        let runtime = runtime_with(LuaBudget { max_memory_bytes: SMALL_MEMORY_BYTES, ..LuaBudget::default() });
        let outcome = runtime.run_entry(Path::new(entry), LuaPass::Header, &DefaultState);
        assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "{entry}: got {outcome:?}");
        assert!(runtime.memory_used() <= SMALL_MEMORY_BYTES, "{entry}: the interpreter never held more than its ceiling");
    }
}

#[test]
fn a_runaway_function_is_cut_off_and_the_frame_goes_on() {
    let budget = LuaBudget { frame: FrameBudget { max_call_instructions: SMALL_CALL_INSTRUCTIONS, ..FrameBudget::default() }, ..LuaBudget::default() };
    let runtime = runtime_with(budget);
    let steady = runtime.register(function(&runtime, "return function() return 7 end"), LuaFnKind::Integer);
    let runaway = runtime.register(function(&runtime, "return function() while true do end end"), LuaFnKind::Integer);
    let guarded = runtime.register(function(&runtime, "return function() while true do pcall(function() while true do end end) end end"), LuaFnKind::Boolean);
    let later = runtime.register(
        function(&runtime, "calls = 0 return function() calls = calls + 1 if calls > 1 then while true do end end return 40 + calls end"),
        LuaFnKind::Integer,
    );

    for _ in 0..2 {
        runtime
            .frame(&DefaultState, |frame| {
                assert_eq!(frame.call_integer(runaway), 0, "a function that never finished has no earlier answer");
                assert!(!frame.call_boolean(guarded));
                assert_eq!(frame.call_integer(later), 41, "the second frame repeats what the first one answered");
                assert_eq!(frame.call_integer(steady), 7, "the frame goes on after a call was cut off");
            })
            .expect("the frame binds");
    }

    let diagnostics = runtime.diagnostics();
    assert_eq!(diagnostics.frames_over_budget, 2);
    let failed: Vec<(u32, u64)> = diagnostics.function_failures.iter().map(|failure| (failure.function.0, failure.count)).collect();
    assert_eq!(failed, [(runaway.0, 2), (guarded.0, 2), (later.0, 1)]);
    assert!(diagnostics.swallowed.is_empty(), "the budget error did not stop at the function's own pcall");
}

#[test]
fn a_function_that_fails_every_frame_is_logged_once_with_where_it_failed() {
    let runtime = runtime();
    let broken = runtime.register(function(&runtime, "return dofile('parts/broken_value.lua')"), LuaFnKind::Integer);
    for _ in 0..3 {
        runtime.frame(&DefaultState, |frame| assert_eq!(frame.call_integer(broken), 0)).expect("the frame binds");
    }

    let failures = runtime.diagnostics().function_failures;
    assert_eq!(failures.len(), 1);
    assert_eq!((failures[0].function, failures[0].kind, failures[0].count), (broken, LuaFnKind::Integer, 3));
    assert_eq!(failures[0].first_message, "parts/broken_value.lua:3: attempt to index local 'missing' (a nil value)", "the message carries no call stack");
    assert_eq!(error_location(&failures[0].first_message), Some(("parts/broken_value.lua", 3)));
    assert_eq!(runtime.diagnostics().frames_over_budget, 0, "a function that raises is not a budget overrun");
}

#[test]
fn a_frame_past_its_call_ceiling_repeats_what_each_function_last_answered() {
    let budget = LuaBudget { frame: FrameBudget { max_calls: SMALL_FRAME_CALLS, ..FrameBudget::default() }, ..LuaBudget::default() };
    let runtime = runtime_with(budget);
    let counter = runtime.register(function(&runtime, "ticks = 0 return function() ticks = ticks + 1 return ticks end"), LuaFnKind::Integer);
    let label = runtime.register(function(&runtime, "return function() return 'tick ' .. ticks end"), LuaFnKind::Text);
    let timer = runtime.register(function(&runtime, "return function() return ticks * 1000 end"), LuaFnKind::Timer);
    let never = runtime.register(function(&runtime, "return function() return true end"), LuaFnKind::Boolean);
    let event = runtime.register(function(&runtime, "fired = 0 return function(argument) fired = fired + argument end"), LuaFnKind::Event);

    runtime
        .frame(&DefaultState, |frame| {
            assert_eq!(frame.call_integer(counter), 1);
            assert_eq!(frame.call_text(label), "tick 1");
            assert_eq!(frame.call_timer(timer), 1000);
            assert_eq!(frame.call_integer(counter), 1, "the fourth call is not made and the first answer is repeated");
            assert_eq!(frame.call_text(label), "tick 1");
            assert_eq!(frame.call_timer(timer), 1000);
            assert!(!frame.call_boolean(never), "a function that has never run answers its default");
            frame.call_event(event, 5);
        })
        .expect("the frame binds");
    assert_eq!(eval::<(i64, i64)>(&runtime, "return ticks, fired"), (1, 0), "nothing ran past the ceiling");
    assert_eq!(runtime.diagnostics().frames_over_budget, 1, "one frame is counted once however many calls it lost");

    runtime
        .frame(&DefaultState, |frame| {
            assert_eq!(frame.call_integer(counter), 2, "the next frame has its own allowance");
            frame.call_event(event, 5);
        })
        .expect("the frame binds");
    assert_eq!(eval::<i64>(&runtime, "return fired"), 5);
    assert_eq!(runtime.diagnostics().frames_over_budget, 1);
    assert!(runtime.diagnostics().function_failures.is_empty(), "a call that was not made did not fail");
}

#[test]
fn a_frame_out_of_wall_clock_makes_no_more_calls() {
    let budget = LuaBudget { frame: FrameBudget { max_micros: 0, ..FrameBudget::default() }, ..LuaBudget::default() };
    let runtime = runtime_with(budget);
    let timer = runtime.register(function(&runtime, "ran = false return function() ran = true return 5 end"), LuaFnKind::Timer);
    runtime.frame(&DefaultState, |frame| assert_eq!(frame.call_timer(timer), TIMER_OFF)).expect("the frame binds");
    assert!(!eval::<bool>(&runtime, "return ran"));
    assert_eq!(runtime.diagnostics().frames_over_budget, 1);
}

#[test]
fn what_the_host_does_between_two_calls_is_not_charged_to_the_skin() {
    let budget = LuaBudget { frame: FrameBudget { max_micros: SMALL_FRAME_MICROS, ..FrameBudget::default() }, ..LuaBudget::default() };
    let runtime = runtime_with(budget);
    let steady = runtime.register(function(&runtime, "return function() return 7 end"), LuaFnKind::Integer);
    runtime
        .frame(&DefaultState, |frame| {
            std::thread::sleep(HOST_WORK);
            assert_eq!(frame.call_integer(steady), 7, "a frame that began long ago has spent nothing yet");
            std::thread::sleep(HOST_WORK);
            assert_eq!(frame.call_integer(steady), 7, "the host's own work between two calls is not the skin's");
        })
        .expect("the frame binds");
    assert_eq!(runtime.diagnostics().frames_over_budget, 0);
}

#[test]
fn the_time_spent_inside_calls_adds_up_over_a_frame() {
    let budget =
        LuaBudget { frame: FrameBudget { max_micros: SMALL_FRAME_MICROS, max_call_instructions: u64::MAX, ..FrameBudget::default() }, ..LuaBudget::default() };
    let runtime = runtime_with(budget);
    let spin = runtime.register(function(&runtime, "return function() while true do end end"), LuaFnKind::Integer);
    let after = runtime.register(function(&runtime, "ran = false return function() ran = true return 7 end"), LuaFnKind::Integer);
    for _ in 0..2 {
        runtime
            .frame(&DefaultState, |frame| {
                assert_eq!(frame.call_integer(spin), 0, "the clock cuts off a call the instruction count would not");
                assert_eq!(frame.call_integer(after), 0, "the frame's clock is spent, so no later call is made");
            })
            .expect("the frame binds");
    }
    assert!(!eval::<bool>(&runtime, "return ran"));
    assert_eq!(runtime.diagnostics().frames_over_budget, 2);
}

#[test]
fn a_finalizer_never_runs_so_a_pass_that_leaves_one_behind_still_ends() {
    let (kept, returned) = finishes("a pass that collects objects with a looping __gc", || {
        let runtime = runtime_with(small_load_budget());
        let outcome = runtime.run_entry(Path::new("budget/finalizer_spin.luaskin"), LuaPass::Header, &DefaultState);
        let Ok(Value::Table(result)) = outcome else {
            panic!("the pass returns a table: {outcome:?}");
        };
        (result.get::<bool>("kept").expect("kept is set"), result.get::<bool>("returned").expect("returned is set"))
    });
    assert!(kept, "the metatable keeps its __gc field, as it does in an interpreter without finalizers");
    assert!(returned, "setmetatable still installs the metatable");
}

#[test]
fn a_finalizer_never_runs_inside_a_frame() {
    let answered = finishes("a frame that collects an object with a looping __gc", || {
        let runtime = runtime();
        let collector = runtime.register(
            function(&runtime, "return function() setmetatable({}, { __gc = function() while true do end end }) collectgarbage() return 9 end"),
            LuaFnKind::Integer,
        );
        runtime.frame(&DefaultState, |frame| frame.call_integer(collector)).expect("the frame binds")
    });
    assert_eq!(answered, 9);
}

#[test]
fn a_finalizer_never_runs_when_the_interpreter_is_closed() {
    finishes("closing an interpreter that holds an object with a looping __gc", || {
        let runtime = runtime();
        eval::<()>(&runtime, "KEEP = setmetatable({}, { __gc = function() while true do end end })");
        eval::<()>(&runtime, "local late = { __gc = 1 } LATE = setmetatable({}, late) late.__gc = function() while true do end end");
        drop(runtime);
    });
}

#[test]
fn a_pattern_that_backtracks_without_end_is_cut_off() {
    for entry in ["budget/pattern_spin.luaskin", "budget/guarded_pattern_spin.luaskin"] {
        let cut_off = finishes(entry, move || {
            let runtime = runtime_with(small_load_budget());
            let outcome = runtime.run_entry(Path::new(entry), LuaPass::Header, &DefaultState);
            assert!(runtime.diagnostics().swallowed.is_empty(), "{entry}: the budget error is not a swallowed error");
            let again = runtime.run_entry(Path::new("budget/modest.luaskin"), LuaPass::Header, &DefaultState);
            assert!(again.is_ok(), "{entry}: the interpreter is usable after an overrun: {again:?}");
            matches!(outcome, Err(SkinError::LuaBudget { .. }))
        });
        assert!(cut_off, "{entry}");
    }
}

#[test]
fn the_wall_clock_cuts_a_backtracking_pattern_off_too() {
    let cut_off = finishes("a pattern held to the wall clock alone", || {
        let budget = LuaBudget { load: LoadBudget { max_instructions: u64::MAX, max_micros: 20_000 }, ..LuaBudget::default() };
        let runtime = runtime_with(budget);
        matches!(runtime.run_entry(Path::new("budget/pattern_spin.luaskin"), LuaPass::Header, &DefaultState), Err(SkinError::LuaBudget { .. }))
    });
    assert!(cut_off);
}

#[test]
fn a_backtracking_pattern_inside_a_frame_is_cut_off_and_the_frame_goes_on() {
    let (answers, over_budget, swallowed) = finishes("a frame that calls a backtracking pattern", || {
        let budget = LuaBudget { frame: FrameBudget { max_call_instructions: SMALL_CALL_INSTRUCTIONS, ..FrameBudget::default() }, ..LuaBudget::default() };
        let runtime = runtime_with(budget);
        let backtracking =
            runtime.register(function(&runtime, "return function() return string.find(('a'):rep(40), ('a*'):rep(40) .. 'b') or 5 end"), LuaFnKind::Integer);
        let guarded = runtime.register(
            function(&runtime, "return function() while true do pcall(string.gsub, ('a'):rep(40), ('a*'):rep(40) .. 'b', '') end end"),
            LuaFnKind::Boolean,
        );
        let steady = runtime.register(function(&runtime, "return function() return 7 end"), LuaFnKind::Integer);
        let answers = runtime
            .frame(&DefaultState, |frame| (frame.call_integer(backtracking), frame.call_boolean(guarded), frame.call_integer(steady)))
            .expect("the frame binds");
        (answers, runtime.diagnostics().frames_over_budget, runtime.diagnostics().swallowed.len())
    });
    assert_eq!(answers, (0, false, 7), "the calls that ran away answer their defaults and the frame goes on");
    assert_eq!(over_budget, 1);
    assert_eq!(swallowed, 0, "the budget error did not stop at the function's own pcall");
}

#[test]
fn what_a_substitution_builds_is_held_to_the_memory_ceiling() {
    for entry in ["budget/pattern_hoard.luaskin", "budget/guarded_pattern_hoard.luaskin"] {
        let cut_off = finishes(entry, move || {
            let runtime = runtime_with(LuaBudget { max_memory_bytes: SMALL_MEMORY_BYTES, ..LuaBudget::default() });
            matches!(runtime.run_entry(Path::new(entry), LuaPass::Header, &DefaultState), Err(SkinError::LuaBudget { .. }))
        });
        assert!(cut_off, "{entry}");
    }
}

#[test]
fn a_script_field_compiles_by_the_rule_of_its_kind() {
    let runtime = runtime();
    eval::<()>(&runtime, "gauge = 80 fired = 0");
    let over = runtime.compile("gauge > 60", LuaFnKind::Boolean).expect("the condition compiles");
    let doubled = runtime.compile("gauge * 2", LuaFnKind::Integer).expect("the value compiles");
    let label = runtime.compile("'gauge ' .. gauge", LuaFnKind::Text).expect("the text compiles");
    let event = runtime.compile("fired = fired + 1", LuaFnKind::Event).expect("the statement compiles");
    let fixed = runtime.compile("gauge * 1000", LuaFnKind::Timer).expect("the timer expression compiles");
    let latched = runtime
        .compile("(function() local since = nil return function() since = since or gauge return since end end)()", LuaFnKind::Timer)
        .expect("the timer factory compiles");
    let again = runtime.compile("gauge * 2", LuaFnKind::Integer).expect("the value compiles again");
    assert_ne!(doubled, again, "every script field is its own function");
    assert_eq!(runtime.kind_of(latched), Some(LuaFnKind::Timer));

    runtime
        .frame(&DefaultState, |frame| {
            assert!(frame.call_boolean(over));
            assert_eq!(frame.call_integer(doubled), 160);
            assert_eq!(frame.call_text(label), "gauge 80");
            frame.call_event(event, 0);
            assert_eq!(frame.call_timer(fixed), 80_000);
            assert_eq!(frame.call_timer(latched), 80, "the function the script returned is the timer");
        })
        .expect("the frame binds");
    eval::<()>(&runtime, "gauge = 10");
    runtime
        .frame(&DefaultState, |frame| {
            assert_eq!(frame.call_timer(fixed), 10_000);
            assert_eq!(frame.call_timer(latched), 80, "and it keeps its own state");
        })
        .expect("the frame binds");
    assert_eq!(eval::<i64>(&runtime, "return fired"), 1);

    assert!(matches!(runtime.compile("gauge >", LuaFnKind::Boolean), Err(SkinError::Lua { .. })));
    assert!(matches!(runtime.compile("missing.field", LuaFnKind::Timer), Err(SkinError::Lua { .. })), "a timer script that raises when tried is dropped");
    assert!(matches!(runtime.compile("\u{1b}Lua", LuaFnKind::Event), Err(SkinError::Lua { .. })), "a script field is text");
}

#[test]
fn a_timer_script_that_never_ends_is_cut_off_when_it_is_tried() {
    let runtime = runtime_with(small_load_budget());
    let outcome = runtime.compile("(function() while true do end end)()", LuaFnKind::Timer);
    assert!(matches!(outcome, Err(SkinError::Lua { .. })), "got {outcome:?}");
    assert!(runtime.compile("1000", LuaFnKind::Timer).is_ok(), "the interpreter is usable afterwards");
}

#[test]
fn work_done_with_a_host_bound_is_held_to_the_load_budget() {
    let runtime = runtime_with(small_load_budget());
    let trap: Table = eval(&runtime, "return setmetatable({}, { __index = function() while true do end end })");
    let outcome = runtime.with_host(&DefaultState, || trap.get::<Value>("anything").is_err());
    assert!(matches!(outcome, Err(SkinError::LuaBudget { .. })), "got {outcome:?}");
    assert_eq!(runtime.with_host(&DefaultState, || 3).expect("plain work runs"), 3);
}

#[test]
fn the_header_only_mode_has_the_same_base_environment() {
    let runtime =
        SkinLua::new(SkinLuaConfig { mode: LuaMode::HeaderOnly, seed: Some(TEST_SEED), ..SkinLuaConfig::new(&fixture_root()) }).expect("the runtime builds");
    let Value::Table(header) = runtime.run_entry(Path::new("entry.luaskin"), LuaPass::Header, &DefaultState).expect("the header pass runs") else {
        panic!("the header pass returns a table");
    };
    assert_eq!(header.get::<i64>("type").expect("the header has a type"), 6);
    assert!(eval::<bool>(&runtime, "return next(require('main_state')) == nil"));
}

#[test]
fn a_path_is_reduced_to_its_place_under_the_root() {
    let runtime = runtime();
    let paths = runtime.paths();
    let root = paths.root().to_path_buf();
    assert!(root.is_absolute());

    assert_eq!(paths.logical("parts/part.lua").expect("a relative path resolves"), Path::new("parts/part.lua"));
    assert_eq!(paths.logical("parts\\part.lua").expect("a backslash separates too"), Path::new("parts/part.lua"));
    assert_eq!(paths.logical("./parts/../mods/counter.lua").expect("dots are applied"), Path::new("mods/counter.lua"));
    assert_eq!(paths.logical(&root.join("data").join("notes.txt").to_string_lossy()).expect("an absolute path resolves"), Path::new("data/notes.txt"));
    assert_eq!(paths.logical("not/there/yet.txt").expect("a missing file still has a place"), Path::new("not/there/yet.txt"));
    assert_eq!(paths.logical("").expect("the empty path is the root"), Path::new(""));
    assert_eq!(paths.script("parts/part.lua").expect("a script resolves"), root.join("parts").join("part.lua"));

    for outside in ["..", "../secret.txt", "parts/../../secret.txt", "/etc/hosts"] {
        assert!(matches!(paths.logical(outside), Err(SkinError::PathEscape(named)) if named == outside), "{outside} should be refused");
    }
    assert_eq!(SkinPaths::chunk_name(Path::new("Root/define.lua")), "@Root/define.lua");
}

#[test]
fn reads_prefer_the_overlay_and_writes_go_nowhere_else() {
    let scratch = Scratch::new("overlay");
    scratch.write("skin/data/kept.txt", b"from the skin");
    scratch.write("skin/data/shadowed.txt", b"from the skin");
    scratch.write("overlay/data/shadowed.txt", b"from the overlay");
    scratch.write("overlay/data/written.txt", b"from the overlay");
    let root = scratch.path().join("skin");
    let overlay = scratch.path().join("overlay");

    let paths = SkinPaths::new(&root, Some(&overlay)).expect("the paths build");
    assert_eq!(std::fs::read(paths.readable("data/kept.txt").expect("the path resolves")).expect("the file is read"), b"from the skin");
    assert_eq!(std::fs::read(paths.readable("data/shadowed.txt").expect("the path resolves")).expect("the file is read"), b"from the overlay");
    assert_eq!(std::fs::read(paths.readable("data/written.txt").expect("the path resolves")).expect("the file is read"), b"from the overlay");
    assert_eq!(paths.readable("data/absent.txt").expect("a missing file still resolves"), paths.root().join("data").join("absent.txt"));
    assert_eq!(
        paths.script("data/shadowed.txt").expect("the path resolves"),
        paths.root().join("data").join("shadowed.txt"),
        "code never comes from the overlay"
    );
    assert_eq!(paths.writable("data/new/file.txt").expect("a write resolves"), overlay.join("data").join("new").join("file.txt"));
    assert!(!overlay.join("data").join("new").exists(), "resolving a write creates nothing");
    assert!(matches!(paths.writable("../escape.txt"), Err(SkinError::PathEscape(_))));

    let listed = paths.entries("data").expect("the directory is listed");
    let prefix = format!("{}/data/", paths.root().to_string_lossy().replace('\\', "/"));
    let names: Vec<&str> = listed.iter().map(|entry| entry.strip_prefix(prefix.as_str()).expect("every entry is under the root")).collect();
    assert_eq!(names, ["kept.txt", "shadowed.txt", "written.txt"]);
    assert!(matches!(paths.entries("absent"), Err(SkinError::Read(_))));
    assert!(matches!(paths.entries(".."), Err(SkinError::PathEscape(_))));

    let read_only = SkinPaths::new(&root, None).expect("the paths build");
    assert!(matches!(read_only.writable("data/new.txt"), Err(SkinError::Read(_))), "a skin with no overlay cannot write");
    assert_eq!(read_only.entries("data").expect("the directory is listed").len(), 2);
    assert!(matches!(SkinPaths::new(&scratch.path().join("absent"), None), Err(SkinError::Read(_))), "a root that does not exist is refused");
}

#[cfg(unix)]
#[test]
fn a_root_reached_through_a_link_accepts_both_spellings() {
    let scratch = Scratch::new("root-link");
    scratch.write("real/skin/part.lua", b"return 'part'");
    std::os::unix::fs::symlink(scratch.path().join("real"), scratch.path().join("alias")).expect("the link is created");
    let runtime = runtime_at(&scratch.path().join("alias").join("skin"));
    let real = scratch.path().join("real").join("skin").join("part.lua").canonicalize().expect("the file exists");
    runtime.lua().globals().set("real_part", real.to_string_lossy().into_owned()).expect("the global is set");

    assert_eq!(eval::<String>(&runtime, "return dofile(real_part) .. dofile('part.lua')"), "partpart");
    assert_eq!(runtime.paths().logical(&real.to_string_lossy()).expect("the canonical spelling resolves"), Path::new("part.lua"));
}

#[test]
fn an_external_skin_pack_runs_its_header_pass_untouched() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    let scratch = Scratch::new("pack-overlay");
    let before = files_under(&pack);
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&pack)
        .expect("the pack should be listable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(LUA_SKIN_EXTENSION)))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "the pack holds no Lua skin");

    for entry in &entries {
        let config =
            SkinLuaConfig { overlay: Some(scratch.path().to_path_buf()), seed: Some(TEST_SEED), mode: LuaMode::HeaderOnly, ..SkinLuaConfig::new(&pack) };
        let runtime = SkinLua::new(config).expect("the runtime builds");
        let outcome = runtime.run_entry(entry, LuaPass::Header, &DefaultState);
        let Ok(Value::Table(header)) = outcome else {
            panic!("{} should return its header: {outcome:?}", entry.display());
        };
        assert!(!header.get::<Value>("type").expect("the field is readable").is_nil(), "{} declares a type", entry.display());
        assert_eq!(runtime.diagnostics().frames_over_budget, 0);
    }
    assert_eq!(files_under(&pack), before, "running the pack changed its folder");
}
