//! The `luajava` facade: the few Java classes old skins reach for, imitated in plain tables.
//!
//! Skins written for the reference's earlier releases call straight into Java. The reference no
//! longer allows that and offers a narrow imitation in its place (`LegacySkinLuaApi`); this is the
//! same imitation, with its file access routed through the overlay and its network access switched
//! off. [`install`] publishes one table as both the global `luajava` and `package.loaded.luajava`,
//! so `require("luajava")` answers the table a skin that never required it already sees.
//!
//! **The table.** `luajava.bindClass(name)`, `luajava.new(class, ...)` and
//! `luajava.newInstance(name, ...)`, and nothing else.
//!
//! - `bindClass` accepts `java.io.File`, `com.badlogic.gdx.Gdx`, `com.badlogic.gdx.Input`,
//!   `com.badlogic.gdx.controllers.Controllers` and `com.badlogic.gdx.controllers.Controller`, and
//!   raises `Legacy Lua skin class access denied: <name>` for any other name. The two classes that
//!   are only ever constructed or named, `java.io.File` and `Controller`, answer a table carrying the
//!   name under `__legacy_class`.
//! - `new` accepts only the `java.io.File` marker and a path. A path that leaves the skin root is
//!   refused when the object is made (`skin file access denied: <path>`), as the reference refuses
//!   it. The object has `mkdir` and `listFiles` and nothing else; both are called with a colon, so
//!   the receiver is ignored.
//! - `newInstance` accepts `java.net.URL` with a string, `java.io.InputStreamReader`, which hands its
//!   second argument back, and `java.io.BufferedReader`, which hands back its second argument when
//!   that is a table and refuses anything else. Every other name is refused.
//!
//! **`java.io.File`.** The path is read the way every other library reads one: an absolute path
//! under the root or a path relative to the root, through [`SkinPaths`], and never relative to the
//! working directory.
//!
//! - `mkdir` is `Files.createDirectory` on the tree the skin sees, which is the overlay laid over
//!   the skin root. It makes exactly one directory and answers whether it did. It answers `false`,
//!   and creates nothing, when the directory is already there in either tree, when its parent is in
//!   neither, and when the skin has no overlay. The directory is made in the overlay, with the
//!   overlay's copies of any parents that so far exist only in the skin root. The skin root is never
//!   written.
//! - `listFiles` answers an array, indexed from 1, of the entries of the directory as the skin sees
//!   it: the overlay's and the skin root's merged, each name once, sorted, written as absolute
//!   paths that start with the root's spelling and are separated by `/` ([`SkinPaths::entries`]).
//!   It answers `nil` when the path is not a directory in either tree, and an empty array for an
//!   empty one.
//!
//! **`com.badlogic.gdx.Gdx`.** `Gdx.graphics:getWidth()` and `getHeight()` answer the window size and
//! `Gdx.input:isKeyPressed(code)` answers whether the key is held. Methods are called with a colon
//! and ignore their receiver; `isKeyPressed` reads its last argument, which must be a number.
//! This module holds no host of its own. It reaches the one bound for the current frame through
//! `main_state.screen_width`, `screen_height` and `key_pressed`, looked up in
//! `package.loaded.main_state` when the facade is called rather than when it is built, so a facade
//! a skin kept from its load pass answers for the frame that is running. Where `main_state` offers
//! no such function -- the header-only interpreter leaves it an empty table -- the answers are the
//! ones the reference gives while no window exists: zero, and no key held.
//!
//! **`com.badlogic.gdx.Input`.** `Input.Keys.<NAME>` answers the libGDX key code of the constant of
//! that name ([`INPUT_KEYS`], read out of the reference's `gdx.jar`): `UP` 19, `DOWN` 20, `LEFT` 21,
//! `RIGHT` 22, `A` 29, `ENTER` 66, `ESCAPE` 131, `F12` 255 and so on. The reference's facade asks
//! `Input.Keys.valueOf`, which looks a name up among the keys' *display names* and so answers `-1`
//! for `RIGHT`, where only `Right` works; this build answers what the skin meant. A display name is
//! still understood after the constants, so a skin written against the reference's own behaviour
//! keeps working. A name that is neither answers `-1`, as `valueOf` does, and a key that is not a
//! string or a number is refused. `Gdx.input:isKeyPressed(-1)`, the libGDX "any key", answers `false`:
//! `main_state.key_pressed` refuses a negative code and the host has no way to be asked whether any
//! key is held.
//!
//! **`com.badlogic.gdx.controllers.Controllers`.** `Controllers.getControllers()` answers a list
//! with `size` zero and a `first()` that answers `nil`: this build reports no controller to a skin.
//!
//! **`java.net.URL`.** The object, its `openConnection()` and the connection's `setRequestMethod` and
//! `setConnectTimeout` work, with the reference's checks on their arguments: only `GET` is a method.
//! `connect()`, `getResponseCode()` and `getInputStream()` always raise
//! `Legacy Lua skin HTTP connection failed: network access is not available to skins`. No socket is
//! opened and no name is looked up, so a skin that wraps the call in `pcall` takes its failure
//! branch and one that does not stops where the reference would stop on an unreachable host.
//!
//! The Rust half is four small functions that never raise, in the manner of the other libraries: they
//! answer a value, or `nil` and a message, and a few lines of Lua raise the message as a string
//! with the position of the skin's own call.

use std::path::Path;
use std::rc::Rc;

use mlua::chunk::ChunkMode;
use mlua::{Lua, LuaString, Table, Value};

use super::{LuaShared, SkinPaths, main_state, package};
use crate::SkinError;

/// The global, and the module name, the facade is published under.
const LUAJAVA_MODULE: &str = "luajava";

/// The name the facade's glue carries in the interpreter's own messages.
const FACADE_CHUNK_NAME: &str = "=[skin luajava]";

/// What a path outside the skin root is refused with (`SkinLuaPathResolver.resolve`).
const ACCESS_DENIED: &str = "skin file access denied: ";

/// Why a connection is refused.
const NETWORK_REFUSED: &str = "network access is not available to skins";

/// The facade proper, written in Lua so that every error a skin sees is a string raised at the line
/// of the skin's own call.
///
/// The chunk receives the table of Rust functions and returns the `luajava` table. It reads
/// `package.loaded` once, here, and reads `main_state` out of it each time a window question is
/// asked.
const FACADE: &str = r##"
local core = ...
local loaded = package.loaded
local error, type, tonumber, tostring, select, setmetatable = error, type, tonumber, tostring, select, setmetatable

local CLASS_FIELD = "__legacy_class"
local FILE_CLASS = "java.io.File"
local CONTROLLER_CLASS = "com.badlogic.gdx.controllers.Controller"

local function refuse_argument(value, position, name, expected)
    local place = position and (" #" .. position) or ""
    error("bad argument" .. place .. " to '" .. name .. "' (" .. expected .. " expected, got " .. type(value) .. ")", 4)
end

local function text_argument(value, position, name)
    local kind = type(value)
    if kind == "string" or kind == "number" then
        return tostring(value)
    end
    refuse_argument(value, position, name, "string")
end

local function number_argument(value, position, name)
    local number = tonumber(value)
    if number == nil then
        refuse_argument(value, position, name, "number")
    end
    return number
end

local function main_state_function(name)
    local module = loaded.main_state
    if type(module) ~= "table" then
        return nil
    end
    local found = module[name]
    if type(found) ~= "function" then
        return nil
    end
    return found
end

local function last_argument(...)
    local count = select("#", ...)
    if count == 0 then
        return nil
    end
    return (select(count, ...))
end

local function is_key_pressed(...)
    local code = number_argument(last_argument(...), nil, "isKeyPressed")
    local press = main_state_function("key_pressed")
    if press == nil then
        return false
    end
    return press(code) == true
end

local function screen_dimension(name)
    return function()
        local read = main_state_function(name)
        if read == nil then
            return 0
        end
        return read()
    end
end

local get_width = screen_dimension("screen_width")
local get_height = screen_dimension("screen_height")

local function gdx_facade()
    return {
        graphics = { getWidth = get_width, getHeight = get_height },
        input = { isKeyPressed = is_key_pressed },
    }
end

local keys_metatable = {
    __index = function(_, key)
        return core.key_code(text_argument(key, 2, "__index"))
    end,
}

local function input_facade()
    return { Keys = setmetatable({}, keys_metatable) }
end

local function first_controller()
    return nil
end

local function controllers_facade()
    return {
        getControllers = function()
            return { size = 0, first = first_controller }
        end,
    }
end

local function marker_facade(name)
    return function()
        return { [CLASS_FIELD] = name }
    end
end

local class_facades = {
    ["com.badlogic.gdx.Gdx"] = gdx_facade,
    ["com.badlogic.gdx.Input"] = input_facade,
    ["com.badlogic.gdx.controllers.Controllers"] = controllers_facade,
    [CONTROLLER_CLASS] = marker_facade(CONTROLLER_CLASS),
    [FILE_CLASS] = marker_facade(FILE_CLASS),
}

local function file_facade(path)
    return {
        mkdir = function()
            return core.file_mkdir(path)
        end,
        listFiles = function()
            return core.file_list(path)
        end,
    }
end

local function refuse_connection()
    error("Legacy Lua skin HTTP connection failed: " .. core.network_refused, 2)
end

local function connection_facade()
    return {
        setRequestMethod = function(_, method)
            local verb = text_argument(method, 2, "setRequestMethod")
            if verb ~= "GET" then
                error("Legacy Lua skin HTTP method denied: " .. verb, 2)
            end
        end,
        setConnectTimeout = function(_, timeout)
            number_argument(timeout, 2, "setConnectTimeout")
        end,
        connect = refuse_connection,
        getResponseCode = refuse_connection,
        getInputStream = refuse_connection,
    }
end

local function url_facade()
    return {
        openConnection = function()
            return connection_facade()
        end,
    }
end

local luajava = {}

function luajava.bindClass(name)
    local class_name = text_argument(name, 1, "bindClass")
    local build = class_facades[class_name]
    if build == nil then
        error("Legacy Lua skin class access denied: " .. class_name, 2)
    end
    local facade = build()
    return facade
end

function luajava.new(class, ...)
    if type(class) ~= "table" then
        error("Legacy Lua skin constructor access denied", 2)
    end
    local name = class[CLASS_FIELD]
    if name ~= FILE_CLASS then
        error("Legacy Lua skin constructor access denied: " .. (name == nil and "null" or tostring(name)), 2)
    end
    local path = text_argument((...), 2, "new")
    local denied = core.file_denied(path)
    if denied then
        error(denied, 2)
    end
    local facade = file_facade(path)
    return facade
end

function luajava.newInstance(name, ...)
    local class_name = text_argument(name, 1, "newInstance")
    local argument = ...
    if class_name == "java.net.URL" then
        text_argument(argument, 2, "newInstance")
        local facade = url_facade()
        return facade
    elseif class_name == "java.io.InputStreamReader" then
        return argument
    elseif class_name == "java.io.BufferedReader" then
        if type(argument) ~= "table" then
            error("Legacy Lua skin reader access denied", 2)
        end
        return argument
    end
    error("Legacy Lua skin constructor access denied: " .. class_name, 2)
end

return luajava
"##;

/// Every constant of libGDX 1.9.9's `Input.Keys` that holds an `int`, with its value, read out of
/// the reference's `gdx.jar`. The names are the Java field names, which is what a skin writes
/// after `Input.Keys.`.
const INPUT_KEYS: &[(&str, i32)] = &[
    ("ANY_KEY", -1),
    ("NUM_0", 7),
    ("NUM_1", 8),
    ("NUM_2", 9),
    ("NUM_3", 10),
    ("NUM_4", 11),
    ("NUM_5", 12),
    ("NUM_6", 13),
    ("NUM_7", 14),
    ("NUM_8", 15),
    ("NUM_9", 16),
    ("A", 29),
    ("ALT_LEFT", 57),
    ("ALT_RIGHT", 58),
    ("APOSTROPHE", 75),
    ("AT", 77),
    ("B", 30),
    ("BACK", 4),
    ("BACKSLASH", 73),
    ("C", 31),
    ("CALL", 5),
    ("CAMERA", 27),
    ("CLEAR", 28),
    ("COMMA", 55),
    ("D", 32),
    ("DEL", 67),
    ("BACKSPACE", 67),
    ("FORWARD_DEL", 112),
    ("DPAD_CENTER", 23),
    ("DPAD_DOWN", 20),
    ("DPAD_LEFT", 21),
    ("DPAD_RIGHT", 22),
    ("DPAD_UP", 19),
    ("CENTER", 23),
    ("DOWN", 20),
    ("LEFT", 21),
    ("RIGHT", 22),
    ("UP", 19),
    ("E", 33),
    ("ENDCALL", 6),
    ("ENTER", 66),
    ("ENVELOPE", 65),
    ("EQUALS", 70),
    ("EXPLORER", 64),
    ("F", 34),
    ("FOCUS", 80),
    ("G", 35),
    ("GRAVE", 68),
    ("H", 36),
    ("HEADSETHOOK", 79),
    ("HOME", 3),
    ("I", 37),
    ("J", 38),
    ("K", 39),
    ("L", 40),
    ("LEFT_BRACKET", 71),
    ("M", 41),
    ("MEDIA_FAST_FORWARD", 90),
    ("MEDIA_NEXT", 87),
    ("MEDIA_PLAY_PAUSE", 85),
    ("MEDIA_PREVIOUS", 88),
    ("MEDIA_REWIND", 89),
    ("MEDIA_STOP", 86),
    ("MENU", 82),
    ("MINUS", 69),
    ("MUTE", 91),
    ("N", 42),
    ("NOTIFICATION", 83),
    ("NUM", 78),
    ("O", 43),
    ("P", 44),
    ("PERIOD", 56),
    ("PLUS", 81),
    ("POUND", 18),
    ("POWER", 26),
    ("Q", 45),
    ("R", 46),
    ("RIGHT_BRACKET", 72),
    ("S", 47),
    ("SEARCH", 84),
    ("SEMICOLON", 74),
    ("SHIFT_LEFT", 59),
    ("SHIFT_RIGHT", 60),
    ("SLASH", 76),
    ("SOFT_LEFT", 1),
    ("SOFT_RIGHT", 2),
    ("SPACE", 62),
    ("STAR", 17),
    ("SYM", 63),
    ("T", 48),
    ("TAB", 61),
    ("U", 49),
    ("UNKNOWN", 0),
    ("V", 50),
    ("VOLUME_DOWN", 25),
    ("VOLUME_UP", 24),
    ("W", 51),
    ("X", 52),
    ("Y", 53),
    ("Z", 54),
    ("META_ALT_LEFT_ON", 16),
    ("META_ALT_ON", 2),
    ("META_ALT_RIGHT_ON", 32),
    ("META_SHIFT_LEFT_ON", 64),
    ("META_SHIFT_ON", 1),
    ("META_SHIFT_RIGHT_ON", 128),
    ("META_SYM_ON", 4),
    ("CONTROL_LEFT", 129),
    ("CONTROL_RIGHT", 130),
    ("ESCAPE", 131),
    ("END", 132),
    ("INSERT", 133),
    ("PAGE_UP", 92),
    ("PAGE_DOWN", 93),
    ("PICTSYMBOLS", 94),
    ("SWITCH_CHARSET", 95),
    ("BUTTON_CIRCLE", 255),
    ("BUTTON_A", 96),
    ("BUTTON_B", 97),
    ("BUTTON_C", 98),
    ("BUTTON_X", 99),
    ("BUTTON_Y", 100),
    ("BUTTON_Z", 101),
    ("BUTTON_L1", 102),
    ("BUTTON_R1", 103),
    ("BUTTON_L2", 104),
    ("BUTTON_R2", 105),
    ("BUTTON_THUMBL", 106),
    ("BUTTON_THUMBR", 107),
    ("BUTTON_START", 108),
    ("BUTTON_SELECT", 109),
    ("BUTTON_MODE", 110),
    ("NUMPAD_0", 144),
    ("NUMPAD_1", 145),
    ("NUMPAD_2", 146),
    ("NUMPAD_3", 147),
    ("NUMPAD_4", 148),
    ("NUMPAD_5", 149),
    ("NUMPAD_6", 150),
    ("NUMPAD_7", 151),
    ("NUMPAD_8", 152),
    ("NUMPAD_9", 153),
    ("COLON", 243),
    ("F1", 244),
    ("F2", 245),
    ("F3", 246),
    ("F4", 247),
    ("F5", 248),
    ("F6", 249),
    ("F7", 250),
    ("F8", 251),
    ("F9", 252),
    ("F10", 253),
    ("F11", 254),
    ("F12", 255),
];

/// The code `Input.Keys.<name>` answers: the constant of that name, then the key of that display
/// name (`Input.Keys.valueOf`, which is all the reference looks at), then [`UNKNOWN_KEY`].
fn input_key(name: &str) -> i32 {
    INPUT_KEYS.iter().find(|(constant, _)| *constant == name).map_or_else(|| main_state::key_code(name), |(_, code)| *code)
}

/// The refusal for a name the skin may not use, or `None` when the skin may name it.
fn denial(paths: &SkinPaths, named: &str) -> Option<String> {
    match paths.logical(named) {
        Ok(_) => None,
        Err(SkinError::PathEscape(_)) => Some(format!("{ACCESS_DENIED}{named}")),
        Err(other) => Some(other.to_string()),
    }
}

/// `File.mkdir` (`Files.createDirectory`) on the tree the skin sees.
fn make_directory(shared: &LuaShared, named: &str) -> bool {
    let paths = &shared.paths;
    let Ok(logical) = paths.logical(named) else {
        return false;
    };
    let Some(parent) = logical.parent() else {
        return false;
    };
    let exists = |relative: &Path| paths.readable(&SkinPaths::display(relative)).is_ok_and(|found| found.exists());
    if exists(&logical) || !exists(parent) {
        return false;
    }
    let Ok(target) = paths.writable(named) else {
        return false;
    };
    let Some(directory) = target.parent() else {
        return false;
    };
    let quota = &shared.quota;
    quota.admit_entries(quota.missing_directories(directory) + 1).is_ok()
        && quota.create_directories(directory).is_ok()
        && quota.create_directory(&target).is_ok()
}

/// Publishes the `luajava` facade.
pub(crate) fn install(lua: &Lua, shared: &Rc<LuaShared>) -> mlua::Result<()> {
    let core = lua.create_table()?;
    core.set("network_refused", NETWORK_REFUSED)?;

    let files = Rc::clone(shared);
    core.set("file_denied", lua.create_function(move |_, named: LuaString| Ok(denial(&files.paths, &named.to_string_lossy())))?)?;

    let files = Rc::clone(shared);
    core.set("file_mkdir", lua.create_function(move |_, named: LuaString| Ok(make_directory(&files, &named.to_string_lossy())))?)?;

    let files = Rc::clone(shared);
    core.set(
        "file_list",
        lua.create_function(move |lua, named: LuaString| match files.paths.entries(&named.to_string_lossy()) {
            Ok(entries) => lua.create_sequence_from(entries).map(Some),
            Err(_) => Ok(None),
        })?,
    )?;

    core.set("key_code", lua.create_function(|_, name: LuaString| Ok(input_key(&name.to_string_lossy())))?)?;

    let luajava: Table = lua.load(FACADE).set_name(FACADE_CHUNK_NAME).set_mode(ChunkMode::Text).call(core)?;
    lua.globals().set(LUAJAVA_MODULE, &luajava)?;
    package::preload(lua, LUAJAVA_MODULE, Value::Table(luajava))
}

#[cfg(test)]
mod tests {
    use super::{INPUT_KEYS, input_key};
    use crate::lua::main_state;

    const UNKNOWN_KEY: i32 = -1;

    #[test]
    fn no_constant_is_listed_twice() {
        let mut names: Vec<_> = INPUT_KEYS.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "a constant appears twice in the table");
    }

    #[test]
    fn the_arrow_keys_answer_what_the_skin_meant() {
        assert_eq!([input_key("UP"), input_key("DOWN"), input_key("LEFT"), input_key("RIGHT")], [19, 20, 21, 22]);
    }

    #[test]
    fn a_name_that_is_both_a_constant_and_a_display_name_means_one_key() {
        for (name, code) in INPUT_KEYS {
            let displayed = main_state::key_code(name);
            assert!(displayed == UNKNOWN_KEY || displayed == *code, "{name} is {code} as a constant and {displayed} as a display name");
        }
    }

    #[test]
    fn a_display_name_is_understood_after_the_constants_and_anything_else_is_unknown() {
        assert_eq!(input_key("Right"), 22);
        assert_eq!(input_key("L-Ctrl"), 129);
        assert_eq!([input_key("right"), input_key("NOPE"), input_key("")], [UNKNOWN_KEY; 3]);
    }
}
