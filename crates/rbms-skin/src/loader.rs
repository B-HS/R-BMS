//! Reading a skin document into something the player can draw.
//!
//! The path from file to runtime model is fixed: read within a size limit, parse as strict JSON and
//! fall back to json5 for the documents that are not quite JSON, resolve the guarded clauses and
//! includes that encode the player's own choices, then deserialise the mirror in [`crate::model`]
//! and assemble its destinations into interpolator tracks.
//!
//! Everything the player chose -- which option each customisation row is on, which file each slot
//! holds, how far each offset is nudged -- arrives as [`SkinUserConfig`] and is applied here rather
//! than being consulted later, which is what lets a frame draw without asking any questions.

mod branch;
mod stretch;
mod track;

pub use branch::{MAX_INCLUDE_DEPTH, MAX_INCLUDE_EXPANSIONS, OPTION_RANDOM_VALUE, declared_options, enabled_options, option_holds, selected_option};
pub use stretch::{Filtering, StretchKind, filtering_for, stretch_rect};

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rbms_model::Mode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::SkinError;
use crate::dst::{DestinationTrack, OffsetSource, SkinOffset};
use crate::model::{Destination, Filepath, OffsetDef, PropertyDef, SKIN_TYPE_UNSET, SkinDef};
use crate::resolve::{CustomFile, Draw, FileResolver, build_filemap, contained, enumerate_custom_files, pattern_for};

/// Bytes a document may be before the loader refuses to parse it.
///
/// A skin is third-party data, and the lenient parser walks it character by character, so the
/// ceiling is here to stop a hostile file from turning into a long parse rather than because real
/// documents come close to it.
pub const DEFAULT_MAX_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;

/// Instructions a single expression may execute before it is cut off.
const DEFAULT_MAX_INSTRUCTIONS: u32 = 200_000;

/// Bytes the Lua interpreter may allocate in total.
const DEFAULT_MAX_MEMORY_BYTES: usize = 8 * 1024 * 1024;

/// Expressions a single frame may evaluate before the rest are skipped.
const DEFAULT_MAX_CALLS_PER_FRAME: u32 = 4_096;

/// Microseconds a single expression may run for before it is cut off.
///
/// The instruction ceiling counts what the interpreter executes, which is not the same as time: a
/// standard-library call runs inside C and ticks nothing at all. This is the wall clock the hook
/// checks alongside the count.
const DEFAULT_MAX_CALL_MICROS: u64 = 20_000;

/// Microseconds one frame may spend inside Lua in total, across every expression it evaluates.
const DEFAULT_MAX_FRAME_MICROS: u64 = 4_000;

/// The byte-order mark a document authored on Windows often starts with.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// The first line and column of a file, as both parsers count them once normalised.
#[cfg(feature = "json5")]
const FIRST_POSITION: usize = 1;

/// What a single expression, and a single frame of expressions, may spend.
///
/// Kept here rather than beside the interpreter so that a build compiled without Lua still has the
/// type, and [`SkinLoadOptions`] has the same shape either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub max_instructions: u32,
    pub max_memory_bytes: usize,
    pub max_calls_per_frame: u32,
    /// Wall clock one expression may run for, which is the only ceiling a call that spends its time
    /// inside the standard library ever meets.
    pub max_call_micros: u64,
    /// Wall clock one frame may spend inside Lua altogether.
    pub max_frame_micros: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_instructions: DEFAULT_MAX_INSTRUCTIONS,
            max_memory_bytes: DEFAULT_MAX_MEMORY_BYTES,
            max_calls_per_frame: DEFAULT_MAX_CALLS_PER_FRAME,
            max_call_micros: DEFAULT_MAX_CALL_MICROS,
            max_frame_micros: DEFAULT_MAX_FRAME_MICROS,
        }
    }
}

/// Which of the two parsers read a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserKind {
    /// Strict JSON, which almost every document is.
    Json,
    /// The lenient parser, which allows comments, trailing commas, unquoted keys and single quotes.
    Json5,
}

/// The field mirror behind [`SkinOffset`]'s serde support.
///
/// [`SkinOffset`] belongs to the interpolator, which has no reason to depend on serde; persisting a
/// player's nudges is this module's concern, so the impls live here.
#[derive(Default, Serialize, Deserialize)]
#[serde(remote = "SkinOffset", default)]
struct SkinOffsetFields {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    a: f32,
}

impl Serialize for SkinOffset {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SkinOffsetFields::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for SkinOffset {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        SkinOffsetFields::deserialize(deserializer)
    }
}

/// What one player chose for one skin.
///
/// Rows are addressed by the names the document gave them, so a document that gains or loses a row
/// keeps the choices made for the rows it still has.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SkinUserConfig {
    /// The document this belongs to, as the identifier the settings file stores it under.
    pub path: String,
    /// Customisation row name to the option id it is switched to.
    pub properties: BTreeMap<String, i32>,
    /// File slot name to the file name chosen for it, or `Random`.
    pub filepaths: BTreeMap<String, String>,
    /// Offset id to the nudge applied to it.
    pub offsets: BTreeMap<i32, SkinOffset>,
}

impl OffsetSource for SkinUserConfig {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.offsets.get(&id).copied()
    }
}

/// The predicate a caller with no property registry passes.
///
/// Every id is treated as one the state source will answer for, so no draw condition is dropped;
/// once the registry exists, pass its own membership test instead.
pub fn every_option_known(_id: i32) -> bool {
    true
}

/// How one load is to be performed.
#[derive(Debug, Clone, Copy)]
pub struct SkinLoadOptions<'a> {
    /// The directory a document may read files from, and may not read outside of.
    pub root: &'a Path,
    /// The player's choices for this document.
    pub user: &'a SkinUserConfig,
    /// Pins every wildcard draw, so a test or a golden run resolves the same files each time.
    pub rng_seed: Option<u64>,
    /// What a Lua expression may spend.
    pub lua_budget: Budget,
    /// The mode a play document is being loaded for.
    pub mode: Mode,
    /// The document size ceiling.
    pub max_document_bytes: u64,
    /// Whether an option id is one this build implements.
    pub known_option: fn(i32) -> bool,
}

impl<'a> SkinLoadOptions<'a> {
    /// Options with the defaults every caller but a test wants.
    pub fn new(root: &'a Path, user: &'a SkinUserConfig, mode: Mode) -> Self {
        Self {
            root,
            user,
            rng_seed: None,
            lua_budget: Budget::default(),
            mode,
            max_document_bytes: DEFAULT_MAX_DOCUMENT_BYTES,
            known_option: every_option_known,
        }
    }
}

/// The play screen for a seven-key chart, and the first of the reference's `SkinType` ids.
pub const SKIN_TYPE_PLAY_7KEYS: i32 = 0;

/// The play screens, in `SkinType` order, and the mode each one draws.
const PLAY_SKIN_MODES: &[(i32, Mode)] =
    &[(SKIN_TYPE_PLAY_7KEYS, Mode::BEAT_7K), (1, Mode::BEAT_5K), (2, Mode::BEAT_14K), (3, Mode::BEAT_10K), (4, Mode::POPN_9K), (16, Mode::KEYBOARD_24K)];

/// The song browser, in the reference's `SkinType` numbering.
pub const SKIN_TYPE_MUSIC_SELECT: i32 = 5;

/// The screen shown while a chart is being read, which this build draws as its loading screen.
pub const SKIN_TYPE_DECIDE: i32 = 6;

/// The score screen.
pub const SKIN_TYPE_RESULT: i32 = 7;

/// The key configuration screen.
pub const SKIN_TYPE_KEY_CONFIG: i32 = 8;

/// The non-play screens this build draws: music select, decide, result and key config.
const SCREEN_SKIN_TYPES: &[i32] = &[SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_DECIDE, SKIN_TYPE_RESULT, SKIN_TYPE_KEY_CONFIG];

/// The mode a play document is authored for, or `None` when the type is not a play screen.
pub fn skin_type_mode(skin_type: i32) -> Option<Mode> {
    PLAY_SKIN_MODES.iter().find(|(id, _)| *id == skin_type).map(|(_, mode)| *mode)
}

/// The play screen a chart of `mode` is drawn by, or `None` when no document type draws that mode.
///
/// The inverse of [`skin_type_mode`] over the same table, so the two can never disagree about which
/// document a run reaches for.
pub fn mode_skin_type(mode: Mode) -> Option<i32> {
    PLAY_SKIN_MODES.iter().find(|(_, played)| *played == mode).map(|(id, _)| *id)
}

/// Whether this build has a screen for the document's type.
///
/// A play type is supported exactly when its mode is one the player actually offers, so the set
/// follows [`Mode::ALL`] rather than being restated here.
pub fn is_supported_skin_type(skin_type: i32) -> bool {
    SCREEN_SKIN_TYPES.contains(&skin_type) || skin_type_mode(skin_type).is_some_and(|mode| Mode::ALL.contains(&mode))
}

/// What a document says about itself before any of it is drawn.
///
/// The configuration screen lists documents with this, so it costs one parse and one directory scan
/// per file rather than a whole load.
#[derive(Debug, Clone)]
pub struct SkinHeader {
    pub path: PathBuf,
    pub skin_type: i32,
    pub name: String,
    pub author: String,
    pub width: i32,
    pub height: i32,
    pub parser: ParserKind,
    pub properties: Vec<PropertyDef>,
    pub offsets: Vec<OffsetDef>,
    pub custom_files: Vec<CustomFile>,
}

/// One assembled track, still carrying the object id the document attached it to.
#[derive(Debug, Clone)]
pub struct NamedTrack {
    pub id: String,
    pub track: DestinationTrack,
}

/// A document, read and ready to draw.
pub struct LoadedSkin {
    pub def: SkinDef,
    pub path: PathBuf,
    pub root: PathBuf,
    pub parser: ParserKind,
    pub mode: Mode,
    /// The file slots the configuration screen offers, scanned once at load.
    pub custom_files: Vec<CustomFile>,
    /// The option ids the player's choices turn on.
    pub enabled_options: BTreeSet<i32>,
    /// Every option id this document declares, on or off. A condition naming one of these is
    /// settled while the track is built rather than every frame, because the answer cannot change
    /// while the document is loaded: an object whose condition fails is dropped outright and one
    /// whose condition holds keeps no condition at all (`Skin.prepare`).
    pub declared_options: BTreeSet<i32>,
    /// Image source id to the file it resolved to.
    pub sources: BTreeMap<String, PathBuf>,
    /// Font id to the file it resolved to.
    pub fonts: BTreeMap<String, PathBuf>,
    /// The document's top-level destinations, assembled.
    pub destinations: Vec<NamedTrack>,
    /// Everything that went wrong without costing the skin.
    pub warnings: Vec<String>,
    resolver: FileResolver,
    known_option: fn(i32) -> bool,
    #[cfg(feature = "lua")]
    lua: Option<crate::lua::LuaSandbox>,
}

impl std::fmt::Debug for LoadedSkin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LoadedSkin")
            .field("path", &self.path)
            .field("parser", &self.parser)
            .field("destinations", &self.destinations.len())
            .field("warnings", &self.warnings.len())
            .finish()
    }
}

impl LoadedSkin {
    /// The pattern-to-file-name substitutions this load resolves paths through.
    pub fn filemap(&self) -> &BTreeMap<String, String> {
        self.resolver.filemap()
    }

    /// The file a document-relative path names, checked to be inside the skin root.
    pub fn resolve(&mut self, relative: &str) -> Result<PathBuf, SkinError> {
        let directory = self.path.parent().unwrap_or(&self.root).to_path_buf();
        self.resolver.resolve(&pattern_for(&directory, relative))
    }

    /// The sandbox this document's expressions were compiled into, if it has any.
    #[cfg(feature = "lua")]
    pub fn lua(&self) -> Option<&crate::lua::LuaSandbox> {
        self.lua.as_ref()
    }

    /// Assembles one destination into a track, or `None` when this document's own customisation
    /// choices mean it is never drawn.
    ///
    /// `relative` is the flag the play screen sets on judge-count objects and nothing else, which
    /// is where the reference sets it too (`JsonPlaySkinObjectLoader`).
    pub fn build_track(&mut self, destination: &Destination, relative: bool) -> Result<Option<DestinationTrack>, SkinError> {
        let path = self.path.to_string_lossy().into_owned();
        let mut context = track::TrackContext {
            known_option: self.known_option,
            declared_options: &self.declared_options,
            enabled_options: &self.enabled_options,
            path: &path,
            relative,
            warnings: &mut self.warnings,
        };
        #[cfg(feature = "lua")]
        let sandbox = self.lua.as_ref();
        #[cfg(not(feature = "lua"))]
        let sandbox: Option<&track::Sandbox> = None;
        track::build_track(destination, sandbox, &mut context)
    }
}

/// Reads a document as text, refusing one that is over the ceiling.
pub fn read_document(path: &Path, limit: u64) -> Result<String, SkinError> {
    let size = std::fs::metadata(path).map_err(SkinError::Read)?.len();
    if size > limit {
        return Err(SkinError::TooLarge { path: path.to_string_lossy().into_owned(), actual: size, limit });
    }
    let text = std::fs::read_to_string(path).map_err(SkinError::Read)?;
    Ok(text.trim_start_matches(BYTE_ORDER_MARK).to_owned())
}

/// Parses a document, strictly first and leniently second.
///
/// Almost every document is valid JSON, so the fast parser runs first and the lenient one only sees
/// the files that need it. When neither can read it, the lenient parser's position is reported,
/// because it is the one that got furthest.
pub fn parse_value(path: &Path, text: &str) -> Result<(Value, ParserKind), SkinError> {
    match serde_json::from_str::<Value>(text) {
        Ok(value) => Ok((value, ParserKind::Json)),
        Err(strict) => parse_lenient(path, text, &strict),
    }
}

/// The lenient second attempt.
#[cfg(feature = "json5")]
fn parse_lenient(path: &Path, text: &str, strict: &serde_json::Error) -> Result<(Value, ParserKind), SkinError> {
    match json5::from_str::<Value>(text) {
        Ok(value) => Ok((value, ParserKind::Json5)),
        Err(lenient) => {
            let position = lenient.position();
            Err(SkinError::Parse {
                path: path.to_string_lossy().into_owned(),
                line: position.map_or(strict.line(), |at| at.line + FIRST_POSITION),
                column: position.map_or(strict.column(), |at| at.column + FIRST_POSITION),
                message: lenient.to_string(),
            })
        }
    }
}

/// A build without the lenient parser reports the strict one's complaint as final.
#[cfg(not(feature = "json5"))]
fn parse_lenient(path: &Path, _text: &str, strict: &serde_json::Error) -> Result<(Value, ParserKind), SkinError> {
    Err(SkinError::Parse { path: path.to_string_lossy().into_owned(), line: strict.line(), column: strict.column(), message: strict.to_string() })
}

/// Parses a document straight into the mirror, without resolving clauses or includes.
pub fn parse_document(path: &Path, text: &str) -> Result<(SkinDef, ParserKind), SkinError> {
    let (value, parser) = parse_value(path, text)?;
    Ok((from_value(path, value)?, parser))
}

/// Deserialises a resolved tree into the mirror.
fn from_value(path: &Path, value: Value) -> Result<SkinDef, SkinError> {
    serde_json::from_value(value).map_err(|error| SkinError::Parse {
        path: path.to_string_lossy().into_owned(),
        line: error.line(),
        column: error.column(),
        message: error.to_string(),
    })
}

/// Reads one list out of a parsed tree, treating an unreadable one as absent.
///
/// The header pass runs before guarded clauses are resolved, so a list written as a clause list is
/// not readable yet; the reference's own header pass has the same blind spot.
fn header_list<T: serde::de::DeserializeOwned>(value: &Value, key: &str) -> Vec<T> {
    value.get(key).and_then(|list| serde_json::from_value(list.clone()).ok()).unwrap_or_default()
}

/// Reads one number out of a parsed tree.
fn header_number(value: &Value, key: &str, fallback: i32) -> i32 {
    value.get(key).and_then(Value::as_i64).map_or(fallback, |number| number as i32)
}

/// Reads one string out of a parsed tree.
fn header_text(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or_default().to_owned()
}

/// What a document says about itself, without loading it.
pub fn load_header(path: &Path, options: SkinLoadOptions<'_>) -> Result<SkinHeader, SkinError> {
    let path = contained(options.root, path)?;
    let text = read_document(&path, options.max_document_bytes)?;
    let (value, parser) = parse_value(&path, &text)?;
    let directory = path.parent().unwrap_or(options.root).to_path_buf();
    let filepath: Vec<Filepath> = header_list(&value, "filepath");
    let document = SkinDef { filepath, ..SkinDef::default() };

    Ok(SkinHeader {
        skin_type: header_number(&value, "type", SKIN_TYPE_UNSET),
        name: header_text(&value, "name"),
        author: header_text(&value, "author"),
        width: header_number(&value, "w", crate::model::DEFAULT_SKIN_WIDTH),
        height: header_number(&value, "h", crate::model::DEFAULT_SKIN_HEIGHT),
        parser,
        properties: header_list(&value, "property"),
        offsets: header_list(&value, "offset"),
        custom_files: enumerate_custom_files(&document, &directory, options.root),
        path,
    })
}

/// Reads a document and everything it names.
///
/// The document's type is checked before any of it is assembled: a document that declares none is
/// refused rather than guessed at, and one that declares a screen this build does not draw is
/// refused so the caller can fall back to the built-in skin and say why.
pub fn load_skin(path: &Path, options: SkinLoadOptions<'_>) -> Result<LoadedSkin, SkinError> {
    let path = contained(options.root, path)?;
    let text = read_document(&path, options.max_document_bytes)?;
    let (mut value, parser) = parse_value(&path, &text)?;
    let directory = path.parent().unwrap_or(options.root).to_path_buf();

    let skin_type = header_number(&value, "type", SKIN_TYPE_UNSET);
    if skin_type == SKIN_TYPE_UNSET {
        return Err(SkinError::TypeMissing);
    }
    if !is_supported_skin_type(skin_type) {
        return Err(SkinError::TypeUnsupported(skin_type));
    }

    let properties: Vec<PropertyDef> = header_list(&value, "property");
    let filepath: Vec<Filepath> = header_list(&value, "filepath");
    let header_document = SkinDef { filepath, ..SkinDef::default() };
    let custom_files = enumerate_custom_files(&header_document, &directory, options.root);
    let enabled = enabled_options(&properties, &options.user.properties);
    let declared = declared_options(&properties);

    let mut draw = options.rng_seed.map_or_else(Draw::from_environment, Draw::from_seed);
    let filemap = build_filemap(&custom_files, options.user, &mut draw);
    let mut resolver = FileResolver::new(options.root, filemap, draw);
    let mut warnings: Vec<String> = Vec::new();

    let limit = options.max_document_bytes;
    let include_directory = directory.clone();
    let mut include = |target: &str| -> Result<Value, SkinError> {
        let file = resolver.resolve(&pattern_for(&include_directory, target))?;
        let text = read_document(&file, limit)?;
        parse_value(&file, &text).map(|(value, _)| value)
    };
    branch::transform(&mut value, &mut branch::BranchContext::new(&enabled, &mut include, &mut warnings))?;

    let def = from_value(&path, value)?;

    let mut skin = LoadedSkin {
        path,
        root: options.root.to_path_buf(),
        parser,
        mode: options.mode,
        custom_files,
        enabled_options: enabled,
        declared_options: declared,
        sources: BTreeMap::new(),
        fonts: BTreeMap::new(),
        destinations: Vec::new(),
        warnings,
        resolver,
        known_option: options.known_option,
        #[cfg(feature = "lua")]
        lua: Some(crate::lua::LuaSandbox::new(options.root, options.lua_budget)?),
        def,
    };

    let sources: Vec<(String, String)> = skin.def.source.iter().map(|source| (source.id.clone(), source.path.clone())).collect();
    for (id, pattern) in sources {
        match skin.resolve(&pattern) {
            Ok(file) => {
                skin.sources.insert(id, file);
            }
            Err(error) => skin.warnings.push(format!("image source {id:?} was skipped: {error}")),
        }
    }

    let fonts: Vec<(String, String)> = skin.def.font.iter().map(|font| (font.id.clone(), font.path.clone())).collect();
    for (id, pattern) in fonts {
        match skin.resolve(&pattern) {
            Ok(file) => {
                skin.fonts.insert(id, file);
            }
            Err(error) => skin.warnings.push(format!("font {id:?} was skipped: {error}")),
        }
    }

    let destinations = std::mem::take(&mut skin.def.destination);
    for destination in &destinations {
        match skin.build_track(destination, false) {
            Ok(Some(track)) => skin.destinations.push(NamedTrack { id: destination.id.clone(), track }),
            Ok(None) => {}
            Err(error @ SkinError::LuaUnavailable) => return Err(error),
            Err(error) => skin.warnings.push(format!("object {:?} was skipped: {error}", destination.id)),
        }
    }
    skin.def.destination = destinations;

    Ok(skin)
}
