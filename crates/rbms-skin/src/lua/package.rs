//! `require` and the `package` table, searching the skin root and nothing else.
//!
//! The reference appends `<skin folder>/?.lua` to LuaJ's search path and lets the stock searcher do
//! the rest (`SkinLuaAccessor.setDirectory`). Here the `package` table is built by hand, and the
//! searcher that looks for files does not read `package.path` at all, so a skin cannot widen where
//! its modules come from:
//!
//! - [`install`] creates `package` with `loaded`, `preload`, `path`, `cpath`, `config`,
//!   `searchers` and `searchpath`, and the global `require`. There is no `package.loadlib`: there
//!   is no way to load native code. `package.path` is set to what the reference would show, for a
//!   skin that reads it, and changing it changes nothing.
//! - `require(name)` answers `package.loaded[name]` when it is set, then tries `package.preload`,
//!   then turns every `.` in `name` into a path separator and loads `<root>/<name>.lua` through
//!   [`SkinPaths`](super::SkinPaths). A module's chunk runs once per interpreter and its result --
//!   or `true` when it returns nothing -- is cached in `package.loaded`. That cache is what makes
//!   the body pass reuse the modules the header pass loaded, and it survives between the two.
//! - A module that cannot be found raises the usual `module '<name>' not found` error, which a skin
//!   is free to catch. A name that would resolve outside the skin root is not found either, and the
//!   message says why.
//! - [`preload`] is how the other libraries publish a module that has no file: `main_state`,
//!   `timer_util`, `event_util`, `luajava`, `io` and `os`. Despite its name it writes
//!   `package.loaded`, as the reference does, so `require` answers the module without running
//!   anything.
//!
//! `require` itself is written in Lua, so that an error a module raises while it loads reaches the
//! skin as the string it was raised as.

use std::rc::Rc;

use mlua::chunk::ChunkMode;
use mlua::{Lua, Table, Value};

use super::LuaShared;
use super::env::read_script;
use crate::resolve;

/// The global the module table is published under.
const PACKAGE_GLOBAL: &str = "package";

/// The field of `package` that caches loaded modules.
const LOADED_FIELD: &str = "loaded";

/// The name the global table is cached under, as the stock library caches it.
const GLOBALS_MODULE: &str = "_G";

/// The standard libraries opened from the C implementation, which `require` must answer too.
const STANDARD_MODULES: [&str; 5] = ["string", "table", "math", "coroutine", "bit32"];

/// What a module name's separator becomes in a file name.
const MODULE_SEPARATOR: char = '.';

/// The separator of the directories in a module's file name.
const DIRECTORY_SEPARATOR: &str = "/";

/// The extension of a module's file.
const MODULE_EXTENSION: &str = ".lua";

/// The mark a search path template stands the module name on.
const TEMPLATE_MARK: char = '?';

/// The separator of the templates in a search path.
const TEMPLATE_SEPARATOR: char = ';';

/// `package.config` as the stock library publishes it: the directory separator, the template
/// separator, the name mark, the executable-directory mark and the ignore mark, one per line.
const PACKAGE_CONFIG: &str = "/\n;\n?\n!\n-\n";

/// The name the module glue carries in the interpreter's own messages.
const PACKAGE_GLUE_CHUNK_NAME: &str = "=[skin package]";

/// `require` and the two searchers behind it (`loadlib.c`, `ll_require`).
///
/// The chunk receives the `package` table, its `loaded` table, and the two Rust halves:
/// `find_module(name)` answers a module's source, chunk name and file name, or `nil` and the line
/// to add to the not-found message; `search_path(name, path, sep, rep)` is `package.searchpath`
/// confined to the skin root.
const PACKAGE_GLUE: &str = r##"
local package, loaded, find_module, search_path = ...
local load, error, type, concat = load, error, type, table.concat

local function search_preload(name)
    local loader = package.preload[name]
    if loader == nil then
        return "\n\tno field package.preload['" .. name .. "']"
    end
    return loader
end

local function search_root(name)
    local source, chunk, file = find_module(name)
    if not source then
        return chunk
    end
    local loader, message = load(source, chunk)
    if not loader then
        error("error loading module '" .. name .. "' from file '" .. file .. "':\n\t" .. message, 0)
    end
    return loader, file
end

package.searchers = { search_preload, search_root }
package.searchpath = search_path

function require(name)
    local module = loaded[name]
    if module then
        return module
    end
    local searchers = package.searchers
    if type(searchers) ~= "table" then
        error("'package.searchers' must be a table", 2)
    end
    local missing = {}
    for index = 1, #searchers do
        local loader, extra = searchers[index](name)
        if type(loader) == "function" then
            local result = loader(name, extra)
            if result ~= nil then
                loaded[name] = result
            end
            if loaded[name] == nil then
                loaded[name] = true
            end
            return loaded[name]
        elseif type(loader) == "string" then
            missing[#missing + 1] = loader
        end
    end
    error("module '" .. name .. "' not found:" .. concat(missing), 2)
end
"##;

/// The `package` table, created on first use.
fn package_table(lua: &Lua) -> mlua::Result<Table> {
    let globals = lua.globals();
    match globals.get::<Value>(PACKAGE_GLOBAL)? {
        Value::Table(package) => Ok(package),
        _ => {
            let package = lua.create_table()?;
            globals.set(PACKAGE_GLOBAL, &package)?;
            Ok(package)
        }
    }
}

/// The `package.loaded` table, created on first use.
fn loaded(lua: &Lua) -> mlua::Result<Table> {
    let package = package_table(lua)?;
    match package.get::<Value>(LOADED_FIELD)? {
        Value::Table(loaded) => Ok(loaded),
        _ => {
            let loaded = lua.create_table()?;
            package.set(LOADED_FIELD, &loaded)?;
            Ok(loaded)
        }
    }
}

/// The file a module name stands for, as a path from the skin root.
fn module_file(name: &str) -> String {
    format!("{}{MODULE_EXTENSION}", name.replace(MODULE_SEPARATOR, DIRECTORY_SEPARATOR))
}

/// Builds the `package` table and the global `require`.
pub(crate) fn install(lua: &Lua, shared: &Rc<LuaShared>) -> mlua::Result<()> {
    let globals = lua.globals();
    let package = package_table(lua)?;
    let loaded = loaded(lua)?;
    loaded.set(GLOBALS_MODULE, &globals)?;
    loaded.set(PACKAGE_GLOBAL, &package)?;
    for name in STANDARD_MODULES {
        loaded.set(name, globals.get::<Value>(name)?)?;
    }

    let root = resolve::pattern_for(shared.paths.root(), "");
    package.set("preload", lua.create_table()?)?;
    package.set("path", format!("{TEMPLATE_MARK}{MODULE_EXTENSION}{TEMPLATE_SEPARATOR}{root}{DIRECTORY_SEPARATOR}{TEMPLATE_MARK}{MODULE_EXTENSION}"))?;
    package.set("cpath", "")?;
    package.set("config", PACKAGE_CONFIG)?;

    let finder = Rc::clone(shared);
    let find_module = lua.create_function(move |lua, name: mlua::LuaString| {
        let file = module_file(&name.to_string_lossy());
        match read_script(&finder.paths, &file) {
            Ok((source, chunk_name)) => Ok((Value::String(lua.create_string(&source)?), chunk_name, resolve::pattern_for(finder.paths.root(), &file))),
            Err(message) => Ok((Value::Nil, format!("\n\t{message}"), String::new())),
        }
    })?;

    let searcher = Rc::clone(shared);
    let search_path = lua.create_function(move |_, (name, path, separator, replacement): (String, String, Option<String>, Option<String>)| {
        let separator = separator.unwrap_or_else(|| MODULE_SEPARATOR.to_string());
        let replacement = replacement.unwrap_or_else(|| DIRECTORY_SEPARATOR.to_owned());
        let name = if separator.is_empty() { name } else { name.replace(&separator, &replacement) };
        let mut missing = String::new();
        for template in path.split(TEMPLATE_SEPARATOR) {
            let file = template.replace(TEMPLATE_MARK, &name);
            if searcher.paths.script(&file).is_ok_and(|found| found.is_file()) {
                return Ok((Some(file), None));
            }
            missing.push_str(&format!("\n\tno file '{file}'"));
        }
        Ok((None, Some(missing)))
    })?;

    lua.load(PACKAGE_GLUE).set_name(PACKAGE_GLUE_CHUNK_NAME).set_mode(ChunkMode::Text).call::<()>((package, loaded, find_module, search_path))
}

/// Publishes `module` as `require(name)`'s answer without a file behind it.
pub(crate) fn preload(lua: &Lua, name: &str, module: Value) -> mlua::Result<()> {
    loaded(lua)?.set(name, module)
}
