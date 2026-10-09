//! Loading a `.luaskin`: two passes of one program in one interpreter.
//!
//! The reference's loader (`LuaSkinLoader.load`) is the procedure reproduced here:
//!
//! 1. Build a [`SkinLua`] rooted at the folder the entry file is in, with the write overlay and the
//!    seed from the options. That folder is the skin's whole world: `require`, `dofile` and `io`
//!    reach nothing above it.
//! 2. **Header pass.** Run the entry file with `skin_config` unset, so it returns its header table,
//!    and convert that with [`skin_def_from_lua`]. A skin with no `type` is
//!    [`SkinError::TypeMissing`] and one whose type the reference does not have is
//!    [`SkinError::TypeUnsupported`].
//! 3. **Merge the player's choices** into the header's customisation rows (`merge_header`): an
//!    option stored as the random value picks any item; a file slot with no stored choice takes the
//!    candidate its `def` names; an offset with nothing stored is zeros. A play skin gains the four
//!    automatic offsets after its own.
//! 4. Publish `skin_config`. Its `get_path` resolves through the same file map and wildcard draw the
//!    image sources use, and answers absolute paths under the skin root.
//! 5. **Body pass.** Run the entry file again on the same interpreter and convert the result. Every
//!    module the header pass required is still loaded, so a module's top level runs once, with
//!    `skin_config` unset, and the second run executes little more than the entry file's own lines.
//!    The host is bound for both passes, because a skin reads `main_state` while it builds its
//!    tables.
//! 6. Put the two results together the way the reference reads them: the type, the name, the author
//!    and the customisation rows are the header pass's, and the size, the scene timings and every
//!    object are the body pass's. Then assemble sources, fonts and destination tracks exactly as the
//!    JSON path does, and hand the interpreter to the [`LoadedSkin`], which keeps it -- with the
//!    registry of the skin's functions and the record of what went wrong -- for as long as the
//!    screen is shown.
//!
//! [`load_lua_header`] is the short form for a skin list: a separate interpreter in
//! [`LuaMode::HeaderOnly`], the header pass alone, no host.
//!
//! What a Lua skin is held to differs from a document's limits. The entry file is held to the
//! document size ceiling; every file it requires or runs to the interpreter's own per-file ceiling;
//! and each pass, each compiled script and each frame to the [`LuaBudget`]. The include limits do
//! not apply, because a Lua skin has no includes.
//!
//! Compiled only with the `lua` feature; a build without it answers [`SkinError::LuaUnavailable`] for
//! a `.luaskin`.

use std::path::{Path, PathBuf};

use mlua::Value;

use super::from_lua::{FromLua, skin_def_from_lua};
use super::{
    Assembly, HeaderRows, LoadedSkin, ParserKind, SkinHeader, SkinLoadOptions, assemble, check_skin_type, header_offsets, merge_header, skin_config_global,
    skin_resolution, skin_type_mode,
};
use crate::SkinError;
use crate::lua::main_state::is_property_name;
use crate::lua::{LuaBudget, LuaMode, LuaPass, SkinLua, SkinLuaConfig};
use crate::model::{SKIN_TYPE_UNSET, SkinDef};
use crate::property::{DefaultState, SkinHost};
use crate::resolve::{contained, enumerate_custom_files};

/// The key of the list a body pass must return for there to be anything to draw.
const DESTINATION_KEY: &str = "destination";

/// What a body pass that returned no destination list is refused with.
///
/// The reference walks the list without checking for it, so a skin that returns none fails to load
/// there too (`JSONSkinLoader.loadJsonSkin`).
const NO_DESTINATIONS: &str = "the skin returned no destination list";

/// How one Lua skin is to be loaded.
#[derive(Debug, Clone, Copy)]
pub struct LuaSkinOptions<'a> {
    /// Everything the JSON path takes too: the root, the player's choices, the seed, the mode and
    /// the write overlay.
    pub load: SkinLoadOptions<'a>,
    /// What the skin's interpreter may spend.
    pub budget: LuaBudget,
}

impl<'a> LuaSkinOptions<'a> {
    /// The given load options with the default budget.
    pub fn new(load: SkinLoadOptions<'a>) -> Self {
        Self { load, budget: LuaBudget::default() }
    }
}

/// One skin's interpreter and the entry file it runs, spelled the way the interpreter spells paths.
struct Program {
    runtime: SkinLua,
    /// The skin root, in the one spelling every path handed to the skin starts with.
    root: PathBuf,
    entry: PathBuf,
}

/// Builds the interpreter for the skin whose entry file is `path`.
///
/// The entry file has to be inside the root the caller allows and under the size ceiling before any
/// of it is run.
fn program(path: &Path, options: &LuaSkinOptions<'_>, mode: LuaMode) -> Result<Program, SkinError> {
    let load = &options.load;
    let entry = contained(load.root, path)?;
    let size = std::fs::metadata(&entry).map_err(SkinError::Read)?.len();
    if size > load.max_document_bytes {
        return Err(SkinError::TooLarge { path: entry.to_string_lossy().into_owned(), actual: size, limit: load.max_document_bytes });
    }
    let directory = entry.parent().unwrap_or(load.root);
    let runtime = SkinLua::new(SkinLuaConfig {
        root: directory.to_path_buf(),
        overlay: load.write_overlay.map(Path::to_path_buf),
        seed: load.rng_seed,
        budget: options.budget,
        mode,
    })?;
    let root = runtime.paths().root().to_path_buf();
    let entry = entry.file_name().map_or_else(|| entry.clone(), |name| root.join(name));
    Ok(Program { runtime, root, entry })
}

impl Program {
    /// Runs one pass of the entry file and converts what it returned.
    ///
    /// The conversion is made with the host still bound: a timer a skin wrote as a script is called
    /// once as it is compiled.
    fn pass(&self, pass: LuaPass, host: &dyn SkinHost, warnings: &mut Vec<String>) -> Result<(Value, SkinDef), SkinError> {
        let value = self.runtime.run_entry(&self.entry, pass, host)?;
        let def = self
            .runtime
            .with_host(host, || skin_def_from_lua(&value, &mut FromLua { lua: &self.runtime, path: &self.entry, warnings, known_name: is_property_name }))??;
        Ok((value, def))
    }
}

/// What a Lua skin says about itself, from the header pass alone.
///
/// A skin that declares no type is [`SkinError::TypeMissing`]: a program that returns no header is
/// not a skin (`LuaSkinLoader.loadHeader`).
pub fn load_lua_header(path: &Path, options: &LuaSkinOptions<'_>) -> Result<SkinHeader, SkinError> {
    let program = program(path, options, LuaMode::HeaderOnly)?;
    let mut warnings = Vec::new();
    let (_, header) = program.pass(LuaPass::Header, &DefaultState, &mut warnings)?;
    if header.skin_type == SKIN_TYPE_UNSET {
        return Err(SkinError::TypeMissing);
    }

    Ok(SkinHeader {
        skin_type: header.skin_type,
        width: header.w,
        height: header.h,
        parser: ParserKind::Lua,
        offsets: header_offsets(header.skin_type, &header.offset),
        custom_files: enumerate_custom_files(&header, &program.root, &program.root),
        name: header.name,
        author: header.author,
        categories: header.category,
        properties: header.property,
        path: program.entry,
    })
}

/// Loads a Lua skin and everything it names.
///
/// `host` answers the skin's `main_state` reads while it builds its tables, so the state of the
/// screen being entered -- the chart, the result, the connection -- must be settled before this is
/// called.
pub fn load_lua_skin(path: &Path, options: &LuaSkinOptions<'_>, host: &dyn SkinHost) -> Result<LoadedSkin, SkinError> {
    let load = &options.load;
    let program = program(path, options, LuaMode::Full)?;
    let mut warnings = Vec::new();

    let (_, header) = program.pass(LuaPass::Header, host, &mut warnings)?;
    check_skin_type(header.skin_type)?;

    let rows = HeaderRows { skin_type: header.skin_type, properties: &header.property, filepath: &header.filepath, offsets: &header.offset };
    let merged = merge_header(&rows, &program.root, &program.root, load);
    program.runtime.set_skin_config(skin_config_global(&merged, load.user, &program.root, &program.root))?;

    let (body, mut def) = program.pass(LuaPass::Body, host, &mut warnings)?;
    let placed = match &body {
        Value::Table(table) => !table.raw_get::<Value>(DESTINATION_KEY).is_ok_and(|list| list.is_nil()),
        _ => false,
    };
    if !placed {
        let name = format!("{} ({})", program.entry.to_string_lossy(), LuaPass::Body.label());
        return Err(SkinError::LuaLoad { path: name, message: NO_DESTINATIONS.to_owned() });
    }

    def.skin_type = header.skin_type;
    def.name = header.name;
    def.author = header.author;
    def.category = header.category;
    def.property = header.property;
    def.filepath = header.filepath;
    def.offset = header.offset;

    let resolution = skin_resolution(def.w, def.h);
    if resolution != (def.w, def.h) {
        warnings.push(format!(
            "{}: {}x{} is not a size a skin can be authored at, so it is read as {}x{}",
            path.display(),
            def.w,
            def.h,
            resolution.0,
            resolution.1
        ));
        (def.w, def.h) = resolution;
    }

    assemble(Assembly {
        mode: skin_type_mode(def.skin_type).unwrap_or(load.mode),
        def,
        path: program.entry,
        root: program.root,
        parser: ParserKind::Lua,
        merged,
        warnings,
        known_option: load.known_option,
        runtime: Some(program.runtime),
    })
}
