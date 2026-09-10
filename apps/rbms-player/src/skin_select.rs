//! The SKIN tab: the documents found on disk, the one each screen is drawn with, and the
//! customisation rows the chosen document declares.
//!
//! A document declares its own rows — a list of named options, a file slot filled from a wildcard,
//! a nudge applied to one destination — so the SKIN tab cannot be a fixed table the way every other
//! tab is. The five rows that are always there (the screen, the document, what loaded, reload and
//! reset) stay in `rbms_config::SETTINGS` with the rest of the screen; the rows below them are
//! built here from the chosen document's header and are addressed by [`SkinRow`].
//!
//! Only the header is read to build the rows, which is one parse and one directory scan per
//! document rather than a whole load; the whole document is read when the player asks for it, or
//! when a choice that changes what is drawn moves.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rbms_config::{Config, DEFAULT_SKIN_FOLDER, DEFAULT_VALUE, NONE_VALUE, skin_document_label};
use rbms_skin::dst::{DrawCondition, SkinOffset};
use rbms_skin::loader::{
    LoadedSkin, ParserKind, SkinHeader, SkinLoadOptions, SkinUserConfig, is_supported_skin_type, load_header, load_skin, selected_option, skin_type_mode,
};
use rbms_skin::model::{OffsetDef, PropertyDef};
use rbms_skin::resolve::{CustomFile, RANDOM_SELECTION};

/// How deep under the skin folder documents are looked for: the folder itself, one directory per
/// skin, and one more for the skins that group their variants in a subdirectory.
const SCAN_MAX_DEPTH: usize = 3;

/// How many documents one walk of the skin folder will collect. A folder with more than this is
/// almost certainly the wrong folder, and the row could not be cycled through them anyway.
const SCAN_MAX_DOCUMENTS: usize = 512;

/// The extensions a skin document is written with, strict parser first.
const DOCUMENT_EXTENSIONS: [&str; 2] = ["json", "json5"];

/// One left/right step on an offset row.
const OFFSET_STEP: f32 = 1.0;

/// Furthest an offset row moves or resizes a destination along one axis, in the document's own
/// pixels. Wide enough to push an object right off a screen of any size the loader accepts.
const OFFSET_POSITION_LIMIT: f32 = 2000.0;

/// Furthest an offset row turns a destination, in degrees.
const OFFSET_ANGLE_LIMIT: f32 = 360.0;

/// Furthest an offset row shifts a destination's alpha, in the same 0..=255 units the document
/// writes colours in.
const OFFSET_ALPHA_LIMIT: f32 = 255.0;

/// The value an offset row is put back to.
const OFFSET_RESET: f32 = 0.0;

/// Separator between a customisation row's category and its own name.
const CATEGORY_SEPARATOR: &str = " > ";

/// Shown on the LOADED row while the built-in screen is being drawn.
const BUILT_IN_INFO: &str = "BUILT-IN SCREEN";

/// Shown on the LOADED row for a document that has been chosen but not read yet.
const NOT_READ_INFO: &str = "NOT READ YET";

/// Shown on the LOADED row when the chosen document declares a screen this build does not draw.
const UNSUPPORTED_INFO: &str = "SCREEN NOT SUPPORTED - DRAWING THE BUILT-IN ONE";

/// Shown on the LOADED row for a document that decides what to draw with Lua expressions.
const LUA_INFO: &str = "LUA";

/// One axis of a skin offset, which a document says one by one whether the player may nudge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OffsetAxis {
    X,
    Y,
    W,
    H,
    R,
    A,
}

impl OffsetAxis {
    /// Every axis, in the order the rows are listed.
    pub(crate) const ALL: [OffsetAxis; 6] = [OffsetAxis::X, OffsetAxis::Y, OffsetAxis::W, OffsetAxis::H, OffsetAxis::R, OffsetAxis::A];

    /// What the row calls this axis.
    fn label(self) -> &'static str {
        match self {
            OffsetAxis::X => "X",
            OffsetAxis::Y => "Y",
            OffsetAxis::W => "W",
            OffsetAxis::H => "H",
            OffsetAxis::R => "ANGLE",
            OffsetAxis::A => "ALPHA",
        }
    }

    /// Whether the document lets the player nudge this axis.
    fn allowed(self, def: &OffsetDef) -> bool {
        match self {
            OffsetAxis::X => def.x,
            OffsetAxis::Y => def.y,
            OffsetAxis::W => def.w,
            OffsetAxis::H => def.h,
            OffsetAxis::R => def.r,
            OffsetAxis::A => def.a,
        }
    }

    /// The nudge currently applied along this axis.
    fn get(self, offset: SkinOffset) -> f32 {
        match self {
            OffsetAxis::X => offset.x,
            OffsetAxis::Y => offset.y,
            OffsetAxis::W => offset.w,
            OffsetAxis::H => offset.h,
            OffsetAxis::R => offset.r,
            OffsetAxis::A => offset.a,
        }
    }

    fn set(self, offset: &mut SkinOffset, value: f32) {
        let slot = match self {
            OffsetAxis::X => &mut offset.x,
            OffsetAxis::Y => &mut offset.y,
            OffsetAxis::W => &mut offset.w,
            OffsetAxis::H => &mut offset.h,
            OffsetAxis::R => &mut offset.r,
            OffsetAxis::A => &mut offset.a,
        };
        *slot = value;
    }

    /// How far either way this axis may be nudged.
    fn limit(self) -> f32 {
        match self {
            OffsetAxis::X | OffsetAxis::Y | OffsetAxis::W | OffsetAxis::H => OFFSET_POSITION_LIMIT,
            OffsetAxis::R => OFFSET_ANGLE_LIMIT,
            OffsetAxis::A => OFFSET_ALPHA_LIMIT,
        }
    }
}

/// One customisation row the chosen document declares, addressed by its position in the document's
/// own list so a document that is edited between two runs keeps the rows it still has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SkinRow {
    /// The `property[]` entry at this index: a list of named options, one of which is on.
    Property(usize),
    /// The `filepath[]` entry at this index: a slot filled by one of the files the pattern matches.
    File(usize),
    /// One axis of the `offset[]` entry at this index.
    Offset(usize, OffsetAxis),
}

/// The documents on disk, and the one being drawn for the screen the SKIN tab is configuring.
///
/// The folder is walked once when the settings screen opens rather than every frame, so a folder on
/// a slow disk costs one pause rather than a stutter per row.
pub(crate) struct SkinLibrary {
    /// The directory documents are looked for in, which is also the only directory a document may
    /// read files from.
    root: PathBuf,
    /// Every document found under the root, by screen and then by name.
    documents: Vec<SkinHeader>,
    /// The document each screen is drawn with, once it has been read whole.
    ///
    /// One entry per screen rather than one for the whole library, because the screens outlive the
    /// tab that chose them: the browser is drawn with its own document while a run is being
    /// configured on another.
    loaded: BTreeMap<i32, LoadedDocument>,
    /// Why one screen's chosen document is not being drawn, when it is not.
    errors: BTreeMap<i32, String>,
    /// The screens whose choices have moved since their document was last read.
    stale: BTreeSet<i32>,
    /// Handed out to each read so a renderer that compiled a document notices it has been read
    /// again. Never reused, so a screen read twice is never mistaken for the same build.
    next_build: u64,
}

/// One screen's document, and what it was read from.
struct LoadedDocument {
    skin: Box<LoadedSkin>,
    /// The path it was read from, so a selection that moves is noticed.
    path: String,
    /// Which read produced it.
    build: u64,
}

impl SkinLibrary {
    /// A library rooted where the configuration says, with nothing scanned yet.
    pub(crate) fn new(settings_path: &Path, config: &Config) -> SkinLibrary {
        SkinLibrary {
            root: skin_root(settings_path, config),
            documents: Vec::new(),
            loaded: BTreeMap::new(),
            errors: BTreeMap::new(),
            stale: BTreeSet::new(),
            next_build: 0,
        }
    }

    /// The document one screen is drawn with, or `None` while the built-in screen is drawing.
    pub(crate) fn document(&self, screen: i32) -> Option<&LoadedSkin> {
        self.loaded.get(&screen).map(|entry| entry.skin.as_ref())
    }

    /// Which read produced the document one screen is drawn with. A renderer that holds a screen
    /// built from an earlier read compares this and rebuilds when it has moved.
    pub(crate) fn build_of(&self, screen: i32) -> Option<u64> {
        self.loaded.get(&screen).map(|entry| entry.build)
    }

    /// Walk the skin folder again and read the header of every document under it.
    ///
    /// Called when the settings screen opens and when the player asks for a reload, so a skin
    /// dropped into the folder while the game is running is found without a restart.
    pub(crate) fn rescan(&mut self, settings_path: &Path, config: &Config) {
        self.root = skin_root(settings_path, config);
        self.documents = scan(&self.root);
        self.documents.sort_by_cached_key(|header| (header.skin_type, header.name.to_ascii_uppercase(), header.path.clone()));
    }

    /// The documents that declare the screen the tab is configuring, in the order the row cycles.
    fn candidates(&self, config: &Config) -> Vec<&SkinHeader> {
        self.documents.iter().filter(|header| header.skin_type == config.skin.screen).collect()
    }

    /// The header of the document chosen for the screen being configured.
    fn chosen(&self, config: &Config) -> Option<&SkinHeader> {
        let path = config.skin.document(config.skin.screen)?;
        self.documents.iter().find(|header| header.path.to_string_lossy() == path)
    }

    /// What the SKIN row shows: the document's own name, or the built-in screen.
    pub(crate) fn document_value(&self, config: &Config) -> String {
        let Some(path) = config.skin.document(config.skin.screen) else {
            return DEFAULT_VALUE.to_string();
        };
        match self.chosen(config) {
            Some(header) if !header.name.trim().is_empty() => header.name.clone(),
            _ => skin_document_label(path),
        }
    }

    /// What the LOADED row shows: the size, parser and warning count of the document being drawn,
    /// or why the built-in screen is being drawn instead.
    pub(crate) fn info(&self, config: &Config) -> String {
        let screen = config.skin.screen;
        if let Some(error) = self.errors.get(&screen) {
            return error.clone();
        }
        let Some(path) = config.skin.document(screen) else {
            return BUILT_IN_INFO.to_string();
        };
        if !is_supported_skin_type(screen) {
            return UNSUPPORTED_INFO.to_string();
        }
        let Some(skin) = self.loaded.get(&screen).filter(|entry| entry.path == path && !self.stale.contains(&screen)).map(|entry| &entry.skin) else {
            return NOT_READ_INFO.to_string();
        };
        let parser = match skin.parser {
            ParserKind::Json => "JSON",
            ParserKind::Json5 => "JSON5",
        };
        let lua = if uses_lua(skin) { format!(" - {LUA_INFO}") } else { String::new() };
        let author = if skin.def.author.trim().is_empty() { String::new() } else { format!(" - {}", skin.def.author) };
        format!("{}X{} - {}{} - {} WARNINGS{}", skin.def.w, skin.def.h, parser, lua, skin.warnings.len(), author)
    }

    /// The customisation rows the chosen document declares, top to bottom.
    pub(crate) fn rows(&self, config: &Config) -> Vec<SkinRow> {
        let Some(header) = self.chosen(config) else {
            return Vec::new();
        };
        let properties = (0..header.properties.len()).map(SkinRow::Property);
        let files = (0..header.custom_files.len()).map(SkinRow::File);
        let offsets = header
            .offsets
            .iter()
            .enumerate()
            .flat_map(|(at, def)| OffsetAxis::ALL.into_iter().filter(move |axis| axis.allowed(def)).map(move |axis| SkinRow::Offset(at, axis)));
        properties.chain(files).chain(offsets).collect()
    }

    /// The label and value of one customisation row.
    pub(crate) fn line(&self, config: &Config, row: SkinRow) -> (String, String) {
        let Some(header) = self.chosen(config) else {
            return (NONE_VALUE.to_string(), NONE_VALUE.to_string());
        };
        let stored = config.skin.document(config.skin.screen).and_then(|path| config.skin.customisation(path));
        match row {
            SkinRow::Property(at) => match header.properties.get(at) {
                Some(property) => (
                    labelled(&property.category, &property.name),
                    property_value(property, stored.and_then(|entry| entry.properties.get(&property.name).copied())),
                ),
                None => (NONE_VALUE.to_string(), NONE_VALUE.to_string()),
            },
            SkinRow::File(at) => match header.custom_files.get(at) {
                Some(file) => {
                    (labelled(&file.category, &file.name), file_value(file, stored.and_then(|entry| entry.filepaths.get(&file.name)).map(String::as_str)))
                }
                None => (NONE_VALUE.to_string(), NONE_VALUE.to_string()),
            },
            SkinRow::Offset(at, axis) => match header.offsets.get(at) {
                Some(def) => {
                    let nudge = stored.and_then(|entry| entry.offsets.get(&def.id).copied()).unwrap_or_default();
                    (format!("{} {}", labelled(&def.category, &def.name), axis.label()), format!("{:+}", axis.get(nudge) as i32))
                }
                None => (NONE_VALUE.to_string(), NONE_VALUE.to_string()),
            },
        }
    }

    /// Step one customisation row, and report whether anything moved.
    ///
    /// A property or a file changes what is drawn and which files are read, so the document has to
    /// be read again; an offset is read live out of the stored choices, so it does not.
    pub(crate) fn step(&mut self, config: &mut Config, row: SkinRow, delta: i32) -> bool {
        let Some(path) = config.skin.document(config.skin.screen).map(str::to_owned) else {
            return false;
        };
        let Some(header) = self.chosen(config) else {
            return false;
        };
        match row {
            SkinRow::Property(at) => {
                let Some(property) = header.properties.get(at).cloned() else {
                    return false;
                };
                let current = config.skin.customisation(&path).and_then(|entry| entry.properties.get(&property.name).copied());
                let at = stepped(property.item.iter().position(|item| item.op == selected_option(&property, current)), property.item.len(), delta);
                let Some(item) = at.and_then(|at| property.item.get(at)) else {
                    return false;
                };
                config.skin.customise(&path).properties.insert(property.name.clone(), item.op);
                self.stale.insert(config.skin.screen);
                true
            }
            SkinRow::File(at) => {
                let Some(file) = header.custom_files.get(at).cloned() else {
                    return false;
                };
                let current = config.skin.customisation(&path).and_then(|entry| entry.filepaths.get(&file.name)).cloned();
                let chosen = current.unwrap_or_else(|| file_value(&file, None));
                let at = stepped(file.candidates.iter().position(|name| *name == chosen), file.candidates.len(), delta);
                let Some(name) = at.and_then(|at| file.candidates.get(at)) else {
                    return false;
                };
                config.skin.customise(&path).filepaths.insert(file.name.clone(), name.clone());
                self.stale.insert(config.skin.screen);
                true
            }
            SkinRow::Offset(at, axis) => {
                let Some(id) = header.offsets.get(at).map(|def| def.id) else {
                    return false;
                };
                let stored = config.skin.customise(&path).offsets.entry(id).or_default();
                let next = (axis.get(*stored) + delta as f32 * OFFSET_STEP).clamp(-axis.limit(), axis.limit());
                if (next - axis.get(*stored)).abs() < f32::EPSILON {
                    return false;
                }
                axis.set(stored, next);
                true
            }
        }
    }

    /// Put one customisation row back to what the document's author chose, and report whether it
    /// moved. Only an offset row has a value the player types a key to zero; the rest are cycled.
    pub(crate) fn reset_row(&mut self, config: &mut Config, row: SkinRow) -> bool {
        let SkinRow::Offset(at, axis) = row else {
            return false;
        };
        let Some(path) = config.skin.document(config.skin.screen).map(str::to_owned) else {
            return false;
        };
        let Some(id) = self.chosen(config).and_then(|header| header.offsets.get(at)).map(|def| def.id) else {
            return false;
        };
        let nudged = config.skin.customisation(&path).and_then(|entry| entry.offsets.get(&id).copied()).unwrap_or_default();
        if (axis.get(nudged) - OFFSET_RESET).abs() < f32::EPSILON {
            return false;
        }
        let stored = config.skin.customise(&path).offsets.entry(id).or_default();
        axis.set(stored, OFFSET_RESET);
        true
    }

    /// Step the SKIN row through the built-in screen and every document that declares this screen.
    pub(crate) fn cycle_document(&mut self, config: &mut Config, delta: i32) -> bool {
        let candidates: Vec<String> = self.candidates(config).iter().map(|header| header.path.to_string_lossy().into_owned()).collect();
        let current = config.skin.document(config.skin.screen).map(str::to_owned);
        let at = current.as_deref().and_then(|path| candidates.iter().position(|entry| entry == path));
        let Some(next) = stepped(Some(at.map_or(0, |at| at + 1)), candidates.len() + 1, delta) else {
            return false;
        };
        let chosen = if next == 0 { None } else { candidates.get(next - 1).cloned() };
        if chosen == current {
            return false;
        }
        config.skin.select(config.skin.screen, chosen);
        self.stale.insert(config.skin.screen);
        true
    }

    /// Drop every choice made in the chosen document, and report whether there was one to drop.
    pub(crate) fn forget(&mut self, config: &mut Config) -> bool {
        let Some(path) = config.skin.document(config.skin.screen).map(str::to_owned) else {
            return false;
        };
        if config.skin.customisation(&path).is_none() {
            return false;
        }
        config.skin.forget(&path);
        self.stale.insert(config.skin.screen);
        true
    }

    /// Whether the document chosen for the open screen is not the one that has been read: nothing
    /// has been read yet, the selection points somewhere else, or a choice has moved since.
    ///
    /// A document that failed to read is not retried on its own — the reason stays on the LOADED
    /// row until the player moves a choice or asks for a reload — so a broken document costs one
    /// parse rather than one per entry to the screen.
    pub(crate) fn needs_reload(&self, config: &Config) -> bool {
        self.needs_reload_for(config, config.skin.screen)
    }

    /// The same question asked about a screen other than the one the tab is configuring, which is
    /// how a screen the player walks into gets its document read without the tab being opened.
    pub(crate) fn needs_reload_for(&self, config: &Config, screen: i32) -> bool {
        let Some(path) = config.skin.document(screen) else {
            return self.loaded.contains_key(&screen);
        };
        if self.stale.contains(&screen) {
            return true;
        }
        !self.errors.contains_key(&screen) && self.loaded.get(&screen).is_none_or(|entry| entry.path != path)
    }

    /// Read the chosen document whole with the choices made for it, so what is on screen is what
    /// the rows say. A document that cannot be read leaves the built-in screen drawing and the
    /// reason on the LOADED row.
    pub(crate) fn reload(&mut self, config: &Config) {
        self.reload_for(config, config.skin.screen);
    }

    /// Read one screen's chosen document, whichever screen the tab happens to be configuring.
    pub(crate) fn reload_for(&mut self, config: &Config, screen: i32) {
        self.stale.remove(&screen);
        self.errors.remove(&screen);
        self.loaded.remove(&screen);
        let Some(path) = config.skin.document(screen) else {
            return;
        };
        if !is_supported_skin_type(screen) {
            return;
        }
        let user = config.skin.user_config(path);
        let mode = skin_type_mode(screen).unwrap_or(crate::MODE);
        let root = self.root.clone();
        match load_skin(Path::new(path), SkinLoadOptions::new(&root, &user, mode)) {
            Ok(skin) => {
                let build = self.next_build;
                self.next_build += 1;
                self.loaded.insert(screen, LoadedDocument { skin: Box::new(skin), path: path.to_owned(), build });
            }
            Err(error) => {
                self.errors.insert(screen, error.to_string());
            }
        }
    }
}

/// Whether the document leaves any of its draw decisions to a Lua expression, which is worth saying
/// on the LOADED row because it is the one part of a document that runs rather than being read.
fn uses_lua(skin: &LoadedSkin) -> bool {
    skin.destinations.iter().any(|entry| entry.track.draw_conditions.iter().any(|condition| matches!(condition, DrawCondition::Lua(_))))
}

/// The directory documents are looked for in: the one the row names, or [`DEFAULT_SKIN_FOLDER`]
/// beside the settings file.
fn skin_root(settings_path: &Path, config: &Config) -> PathBuf {
    match config.skin.folder.as_deref() {
        Some(folder) => PathBuf::from(folder),
        None => settings_path.parent().unwrap_or(Path::new(".")).join(DEFAULT_SKIN_FOLDER),
    }
}

/// The header of every document under `root` that declares a screen, up to [`SCAN_MAX_DEPTH`] deep.
///
/// A file that is not a skin, or is a skin this build cannot read, is passed over silently: the
/// folder is the player's own and may hold anything.
fn scan(root: &Path) -> Vec<SkinHeader> {
    let user = SkinUserConfig::default();
    let mut found = Vec::new();
    let mut directories = vec![(root.to_path_buf(), 0usize)];
    while let Some((directory, depth)) = directories.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if found.len() >= SCAN_MAX_DOCUMENTS {
                return found;
            }
            let path = entry.path();
            if path.is_dir() {
                if depth + 1 < SCAN_MAX_DEPTH {
                    directories.push((path, depth + 1));
                }
                continue;
            }
            if !is_document(&path) {
                continue;
            }
            if let Ok(header) = load_header(&path, SkinLoadOptions::new(root, &user, crate::MODE)) {
                found.push(header);
            }
        }
    }
    found
}

/// Whether a file name is one a skin document is written under.
fn is_document(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| DOCUMENT_EXTENSIONS.iter().any(|known| extension.eq_ignore_ascii_case(known)))
}

/// One row's label: its category and its own name, or its own name when the document gave it no
/// category.
fn labelled(category: &str, name: &str) -> String {
    let name = if name.trim().is_empty() { NONE_VALUE } else { name };
    if category.trim().is_empty() {
        name.to_ascii_uppercase()
    } else {
        format!("{}{CATEGORY_SEPARATOR}{}", category.to_ascii_uppercase(), name.to_ascii_uppercase())
    }
}

/// The name of the option a customisation row is switched to.
fn property_value(property: &PropertyDef, chosen: Option<i32>) -> String {
    let op = selected_option(property, chosen);
    property.item.iter().find(|item| item.op == op).map_or_else(|| NONE_VALUE.to_string(), |item| item.name.to_ascii_uppercase())
}

/// The file a slot is filled with: the player's choice, the document's own suggestion, or the first
/// candidate the pattern matched.
fn file_value(file: &CustomFile, chosen: Option<&str>) -> String {
    if let Some(chosen) = chosen.filter(|name| file.candidates.iter().any(|candidate| candidate == name)) {
        return chosen.to_owned();
    }
    if let Some(default) = file.default.as_deref().filter(|name| file.candidates.iter().any(|candidate| candidate == name)) {
        return default.to_owned();
    }
    file.candidates.first().cloned().unwrap_or_else(|| RANDOM_SELECTION.to_owned())
}

/// One step around a list, or `None` when the list is empty or nothing is on it yet.
fn stepped(at: Option<usize>, len: usize, delta: i32) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let at = at.unwrap_or_default();
    Some((at as i32 + delta).rem_euclid(len as i32) as usize)
}

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod tests;
