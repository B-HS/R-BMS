//! The interpreter itself and the base environment a skin runs in.
//!
//! The reference runs skins with LuaJ's standard globals, minus the handful of functions that reach
//! the machine (`SkinLuaAccessor.createStandardGlobals`). This module builds the same environment
//! out of parts this crate controls:
//!
//! - [`new_state`] opens only the libraries that cannot reach outside the interpreter: base, table,
//!   string, math, bit32 and coroutine. `io`, `os` and `package` are never opened from the C
//!   library; their modules install tables written in Rust instead. The C `debug` library stays
//!   closed, and `debug` is the one-function table the reference leaves in its place.
//! - [`install`] finishes the base library. `print` goes to [`LuaLog::printed`](super::LuaLog).
//!   `load` accepts text chunks only and `string.dump` is removed, so bytecode never enters.
//!   `setmetatable` never marks an object for finalization: the interpreter runs a `__gc`
//!   metamethod with its hook switched off, so one that loops would hang the process where no
//!   budget can reach it -- while loading, in a frame, or when the interpreter is closed. The
//!   reference's interpreter has no finalizers at all, so nothing a skin can observe is lost: the
//!   metatable keeps its `__gc` field and it is simply never called. The four pattern functions
//!   of `string` are replaced by the metered ones of [`pattern`](super::pattern).
//!   `dofile` and `loadfile` take an absolute path or one relative to the skin root, resolve it
//!   through [`SkinPaths`], keep no cache and run in the shared globals. `pcall`, `xpcall` and
//!   `coroutine.resume` behave as usual but report every error they swallow to
//!   [`LuaLog::swallowed`](super::LuaLog), because a skin wraps each part it loads in `pcall` and
//!   a missing API would otherwise show up only as a part that is silently absent. `math.random`
//!   draws from a generator this interpreter owns, seeded from the configuration or, without one,
//!   from the machine.
//! - [`run_file`] reads a file inside the skin root, strips a byte-order mark, compiles it as a
//!   text chunk named `@<path from the root>` and calls it, returning its first result.
//! - [`set_skin_config`] publishes the `skin_config` global the body pass reads.
//!
//! Two rules shape how the functions here are built.
//!
//! **Errors stay Lua values.** An error raised from a Rust function reaches a skin as an opaque
//! object, not the string every Lua program expects from `pcall`. So the Rust halves below never
//! raise: they answer `nil, message`, and a few lines of Lua raise the message as a string.
//! `pcall` and `xpcall` also turn an error object raised by another library's Rust function into its
//! message before handing it to the skin.
//!
//! **The budget is not catchable.** Once the meter has cut the interpreter off, everything that
//! turns an error into a return value -- `pcall`, `xpcall`, `coroutine.resume` and `load` -- passes
//! the error on instead, so a skin that wraps everything in `pcall` cannot outlive its budget, and
//! one that nests catchers cannot multiply what is left of it. Running out of memory counts as
//! running out of budget.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use mlua::chunk::ChunkMode;
use mlua::{Lua, LuaOptions, StdLib, Table, Value, Variadic};

use super::{LuaShared, SkinPaths, coerce, error_message, package, pattern};
use crate::dst::SkinOffset;
use crate::resolve::Draw;

/// The global the body pass finds its configuration under. It is `nil` during the header pass, which
/// is how an entry file tells the two apart.
pub const SKIN_CONFIG_GLOBAL: &str = "skin_config";

/// The field of `skin_config` that maps a file slot's name to the stored choice.
const CONFIG_FILE_PATH: &str = "file_path";

/// The field of `skin_config` that resolves a skin-relative path.
const CONFIG_GET_PATH: &str = "get_path";

/// The field of `skin_config` that maps a customisation row's name to the selected `op`.
const CONFIG_OPTION: &str = "option";

/// The field of `skin_config` that lists the selected `op` of every row.
const CONFIG_ENABLED_OPTIONS: &str = "enabled_options";

/// The field of `skin_config` that maps an offset's name to its six values.
const CONFIG_OFFSET: &str = "offset";

/// The name the base glue carries in the interpreter's own messages.
const BASE_GLUE_CHUNK_NAME: &str = "=[skin base]";

/// The global table of the mathematical functions.
const MATH_GLOBAL: &str = "math";

/// The module name the reference's stand-in for the debug library is published under.
const DEBUG_MODULE: &str = "debug";

/// The bytes a UTF-8 file may open with, which the interpreter would read as a syntax error.
const UTF8_BYTE_ORDER_MARK: [u8; 3] = [0xef, 0xbb, 0xbf];

/// Bytes of one Lua source file this build will read. The largest file of the skin this was
/// measured against is well under a megabyte; the ceiling only keeps a mistaken path to some huge
/// file from being read into memory whole.
const MAX_SCRIPT_BYTES: u64 = 64 * 1024 * 1024;

/// The message the interpreter raises when the allocator refuses it memory. It is a fixed string of
/// the C implementation and the only sign of the condition a `pcall` inside Lua is given.
const MEMORY_ERROR_MESSAGE: &str = "not enough memory";

/// Bits of a double's mantissa, which is how many random bits make one uniform fraction.
const FRACTION_BITS: u32 = 53;

/// The base library's functions that are written in Lua.
///
/// The chunk receives the three Rust halves: `print_line(text)` records a line, `caught(error,
/// record)` answers whether a caught error must be passed on and the value to hand the skin, and
/// `read_script(path)` answers a file's source and chunk name or `nil` and a message.
///
/// `pcall` and `xpcall` call the interpreter's own, so a coroutine may still yield across them.
/// `load` and `loadfile` pass an environment on only when one was given, because the interpreter
/// tells an absent fourth argument from a `nil` one. `coroutine.resume` and `load` are settled the
/// same way as `pcall`, because each of them also turns an error into a return value: a coroutine
/// that dies, and a chunk whose reader function raises.
///
/// `setmetatable` hides a metatable's `__gc` field for the one moment the interpreter looks for
/// it, which is when it decides whether the object is to be finalized, and puts it back. The
/// field is read and written raw, so no metamethod of the skin's runs in between.
const BASE_GLUE: &str = r##"
local print_line, caught, read_script = ...
local raw_load, raw_pcall, raw_xpcall, raw_error, raw_resume = load, pcall, xpcall, error, coroutine.resume
local raw_setmetatable, rawget, rawset, type = setmetatable, rawget, rawset, type
local select, tostring, concat, getmetatable = select, tostring, table.concat, getmetatable

function print(...)
    local parts = {}
    for index = 1, select("#", ...) do
        parts[index] = tostring((select(index, ...)))
    end
    print_line(concat(parts, "\t"))
end

local function settle_load(chunk, message)
    if chunk ~= nil then
        return chunk, message
    end
    local pass_on, value = caught(message, false)
    if pass_on then
        raw_error(message, 0)
    end
    return nil, value
end

function load(chunk, chunkname, mode, ...)
    if select("#", ...) > 0 then
        return settle_load(raw_load(chunk, chunkname, "t", (...)))
    end
    return settle_load(raw_load(chunk, chunkname, "t"))
end

local function load_path(path, ...)
    local source, name = read_script(path)
    if not source then
        return nil, name
    end
    if select("#", ...) > 0 then
        return settle_load(raw_load(source, name, "t", (...)))
    end
    return settle_load(raw_load(source, name, "t"))
end

function loadfile(path, mode, ...)
    return load_path(path, ...)
end

function dofile(path)
    local chunk, message = load_path(path)
    if not chunk then
        raw_error(message, 0)
    end
    return chunk()
end

local function settle(ok, ...)
    if ok then
        return ok, ...
    end
    local pass_on, value = caught((...), true)
    if pass_on then
        raw_error((...), 0)
    end
    return false, value
end

function pcall(callee, ...)
    return settle(raw_pcall(callee, ...))
end

local function settle_handled(ok, ...)
    if not ok and caught((...), false) then
        raw_error((...), 0)
    end
    return ok, ...
end

function xpcall(callee, handler, ...)
    local function relay(raised)
        local pass_on, value = caught(raised, true)
        if pass_on then
            return raised
        end
        return handler(value)
    end
    return settle_handled(raw_xpcall(callee, relay, ...))
end

function coroutine.resume(thread, ...)
    return settle(raw_resume(thread, ...))
end

function setmetatable(object, metatable)
    if type(object) ~= "table" or type(metatable) ~= "table" then
        return raw_setmetatable(object, metatable)
    end
    local finalizer = rawget(metatable, "__gc")
    if finalizer == nil then
        return raw_setmetatable(object, metatable)
    end
    rawset(metatable, "__gc", nil)
    local ok, result = raw_pcall(raw_setmetatable, object, metatable)
    rawset(metatable, "__gc", finalizer)
    if not ok then
        raw_error(result, 0)
    end
    return result
end

string.dump = nil
debug = { getmetatable = getmetatable }
return debug
"##;

/// The contents of the `skin_config` global (`SkinLuaAccessor.exportSkinProperty`).
///
/// Built by the loader between the two passes, from the header the first pass produced and the
/// player's stored choices.
pub struct SkinConfigGlobal {
    /// `skin_config.option`: customisation row name to the selected item's `op`, in header order.
    /// The same values in the same order become the `skin_config.enabled_options` array.
    pub options: Vec<(String, i32)>,
    /// `skin_config.file_path`: file slot name to the stored choice. A slot the player never chose
    /// has no entry.
    pub file_paths: Vec<(String, String)>,
    /// `skin_config.offset`: offset name to its stored value, zeros when nothing is stored. Published
    /// as a table of the six fields `x`, `y`, `w`, `h`, `r`, `a`.
    pub offsets: Vec<(String, SkinOffset)>,
    /// `skin_config.get_path`: a skin-relative path, wildcards included, to the absolute path it
    /// resolves to. It answers for a path that does not exist as well, and it is called during
    /// frames, long after loading, so it owns whatever it needs.
    pub get_path: Box<dyn Fn(&str) -> String>,
}

impl std::fmt::Debug for SkinConfigGlobal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SkinConfigGlobal")
            .field("options", &self.options)
            .field("file_paths", &self.file_paths)
            .field("offsets", &self.offsets)
            .finish_non_exhaustive()
    }
}

/// A fresh interpreter with the libraries a skin may have and none of the ones it may not.
pub(crate) fn new_state() -> mlua::Result<Lua> {
    Lua::new_with(StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::BIT | StdLib::MATH, LuaOptions::default())
}

/// Completes the base library as the module documentation describes.
pub(crate) fn install(lua: &Lua, shared: &Rc<LuaShared>) -> mlua::Result<()> {
    let log = Rc::clone(shared);
    let print_line = lua.create_function(move |_, line: mlua::LuaString| {
        log.log.printed(&line.to_string_lossy());
        Ok(())
    })?;

    let judge = Rc::clone(shared);
    let caught = lua.create_function(move |lua, (raised, record): (Value, bool)| {
        let message = describe(&raised);
        if message == MEMORY_ERROR_MESSAGE {
            judge.meter.cut_off();
        }
        let pass_on = judge.meter.exhausted();
        if record && !pass_on {
            judge.log.swallowed(&message);
        }
        let value = match raised {
            Value::Error(_) => Value::String(lua.create_string(&message)?),
            other => other,
        };
        Ok((pass_on, value))
    })?;

    let reader = Rc::clone(shared);
    let read = lua.create_function(move |lua, path: Value| {
        let named = match &path {
            Value::String(_) | Value::Integer(_) | Value::Number(_) => coerce::to_jstring(&path),
            other => return Ok((Value::Nil, format!("cannot open a file named by a {} value", other.type_name()))),
        };
        match read_script(&reader.paths, &named) {
            Ok((source, chunk_name)) => Ok((Value::String(lua.create_string(&source)?), chunk_name)),
            Err(message) => Ok((Value::Nil, message)),
        }
    })?;

    let debug = lua.load(BASE_GLUE).set_name(BASE_GLUE_CHUNK_NAME).set_mode(ChunkMode::Text).call::<Value>((print_line, caught, read))?;
    package::preload(lua, DEBUG_MODULE, debug)?;
    pattern::install(lua, shared)?;
    install_random(lua, shared.seed)
}

/// What a caught error says, whatever kind of value was raised.
fn describe(raised: &Value) -> String {
    match raised {
        Value::String(message) => message.to_string_lossy(),
        Value::Error(error) => error_message(error),
        other => format!("(error object is a {} value)", other.type_name()),
    }
}

/// Replaces `math.random` and `math.randomseed` with a generator this interpreter owns.
///
/// The C library's generator is one per process, so two skins loaded side by side -- or a skin list
/// reading headers while a screen is up -- would draw from each other's sequence and no seed could
/// make a capture repeat. The arguments are read as LuaJ reads them: truncated to integers, with
/// `random(m)` answering `1..=m` and `random(m, n)` answering `m..=n`.
fn install_random(lua: &Lua, seed: Option<u64>) -> mlua::Result<()> {
    let draw = Rc::new(RefCell::new(seed.map_or_else(Draw::from_environment, Draw::from_seed)));
    let math: Table = lua.globals().get(MATH_GLOBAL)?;

    let source = Rc::clone(&draw);
    let random = lua.create_function(move |_, bounds: Variadic<Value>| {
        let mut draw = source.borrow_mut();
        let (low, high, position) = match bounds.as_slice() {
            [] => return Ok((draw.next_u64() >> (u64::BITS - FRACTION_BITS)) as f64 / (1u64 << FRACTION_BITS) as f64),
            [high] => (1, random_bound(high, 1)?, 1),
            [low, high, ..] => (random_bound(low, 1)?, random_bound(high, 2)?, 2),
        };
        if high < low {
            return Err(mlua::Error::runtime(format!("bad argument #{position} to 'random' (interval is empty)")));
        }
        let span = (high as i128 - low as i128 + 1) as u128;
        Ok((low as i128 + (u128::from(draw.next_u64()) % span) as i128) as f64)
    })?;
    math.set("random", random)?;

    let randomseed = lua.create_function(move |_, seed: Value| {
        *draw.borrow_mut() = Draw::from_seed(coerce::to_long(&seed) as u64);
        Ok(())
    })?;
    math.set("randomseed", randomseed)
}

/// One bound of `math.random`, truncated toward zero.
fn random_bound(bound: &Value, position: usize) -> mlua::Result<i64> {
    match coerce::to_number(bound) {
        Some(_) => Ok(coerce::to_long(bound)),
        None => Err(mlua::Error::runtime(format!("bad argument #{position} to 'random' (number expected, got {})", bound.type_name()))),
    }
}

/// The source of one Lua file inside the skin root and the name its chunk is to carry, or the
/// message `loadfile` answers when it cannot be had.
///
/// The file is always the skin root's own, never the overlay's. A byte-order mark is dropped;
/// carriage returns are left alone, because the interpreter already reads `\r\n` as one line end.
pub(crate) fn read_script(paths: &SkinPaths, named: &str) -> Result<(Vec<u8>, String), String> {
    let cannot = |reason: &dyn std::fmt::Display| format!("cannot open {named}: {reason}");
    let logical = paths.logical(named).map_err(|error| cannot(&error))?;
    let file = paths.root().join(&logical);
    let size = std::fs::metadata(&file).map_err(|error| cannot(&error))?.len();
    if size > MAX_SCRIPT_BYTES {
        return Err(cannot(&format_args!("{size} bytes is over the {MAX_SCRIPT_BYTES} byte limit")));
    }
    let mut source = std::fs::read(&file).map_err(|error| cannot(&error))?;
    if source.starts_with(&UTF8_BYTE_ORDER_MARK) {
        source.drain(..UTF8_BYTE_ORDER_MARK.len());
    }
    Ok((source, SkinPaths::chunk_name(&logical)))
}

/// Runs one file of the skin and returns the value it returns.
pub(crate) fn run_file(lua: &Lua, shared: &LuaShared, path: &Path) -> mlua::Result<Value> {
    let (source, chunk_name) = read_script(&shared.paths, &path.to_string_lossy()).map_err(mlua::Error::runtime)?;
    lua.load(source).set_name(chunk_name).set_mode(ChunkMode::Text).into_function()?.call::<Value>(())
}

/// Publishes the `skin_config` global (`SkinLuaAccessor.exportSkinProperty`).
pub(crate) fn set_skin_config(lua: &Lua, config: SkinConfigGlobal) -> mlua::Result<()> {
    let table = lua.create_table()?;

    let file_path = lua.create_table()?;
    for (name, path) in config.file_paths {
        file_path.set(name, path)?;
    }
    table.set(CONFIG_FILE_PATH, file_path)?;

    let get_path = config.get_path;
    table.set(CONFIG_GET_PATH, lua.create_function(move |_, relative: Value| Ok(get_path(&coerce::to_jstring(&relative))))?)?;

    let option = lua.create_table()?;
    let enabled_options = lua.create_table()?;
    for (name, selected) in config.options {
        option.set(name, selected)?;
        enabled_options.push(selected)?;
    }
    table.set(CONFIG_OPTION, option)?;
    table.set(CONFIG_ENABLED_OPTIONS, enabled_options)?;

    let offsets = lua.create_table()?;
    for (name, offset) in config.offsets {
        let values = lua.create_table()?;
        for (field, value) in [("x", offset.x), ("y", offset.y), ("w", offset.w), ("h", offset.h), ("r", offset.r), ("a", offset.a)] {
            values.set(field, value)?;
        }
        offsets.set(name, values)?;
    }
    table.set(CONFIG_OFFSET, offsets)?;

    lua.globals().set(SKIN_CONFIG_GLOBAL, table)
}
