//! Reading a skin into something the player can draw.
//!
//! A skin is one of two things, told apart by its file name. A `.json` or `.json5` skin is a
//! document: it is read within a size limit, parsed as strict JSON with json5 as the fallback for
//! the files that are not quite JSON, has the guarded clauses and includes that encode the player's
//! own choices resolved, and is deserialised into the mirror in [`crate::model`]. A `.luaskin` is a
//! program: [`lua_skin`] runs it twice in one interpreter and converts the table it returns into
//! the same mirror. From there the two share everything -- the files they name are resolved, their
//! destinations are assembled into interpolator tracks, and the result is one [`LoadedSkin`].
//!
//! They also share the step in between, which is the reference's header merge
//! (`JSONSkinLoader.loadJsonSkinHeader` and `SkinHeader.setSkinConfigProperty`): everything the
//! player chose -- which option each customisation row is on, which file each slot holds, how far
//! each offset is nudged -- arrives as [`SkinUserConfig`] and is applied here rather than being
//! consulted later, which is what lets a frame draw without asking any questions.
//!
//! Either kind of skin may leave Lua behind for a frame to call: a Lua skin its function values, a
//! document the scripts it wrote as strings (`script`). Both live in the interpreter the
//! [`LoadedSkin`] owns.
//!
//! What a load is held to:
//!
//! | Ceiling | Applies to |
//! | --- | --- |
//! | [`SkinLoadOptions::max_document_bytes`] | the document, each file it includes, and a Lua skin's entry file |
//! | [`MAX_INCLUDE_DEPTH`], [`MAX_INCLUDE_EXPANSIONS`] | a document's includes; a Lua skin has none |
//! | the interpreter's own per-file ceiling | every file a Lua skin requires or runs |
//! | the interpreter's budget ([`crate::lua::LuaBudget`]) | each pass of a Lua skin, each script compiled, each frame drawn |

mod branch;
#[cfg(feature = "lua")]
pub mod from_lua;
#[cfg(feature = "lua")]
pub mod lua_skin;
mod script;
mod stretch;
mod track;

pub use branch::{
    MAX_INCLUDE_DEPTH, MAX_INCLUDE_EXPANSIONS, OPTION_RANDOM_VALUE, declared_options, enabled_options, merged_options, option_holds, selected_option,
};
pub use stretch::{Filtering, StretchKind, filtering_for, stretch_rect};

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use rbms_model::Mode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::SkinError;
use crate::dst::{DestinationTrack, DrawCondition, OffsetSource, SkinOffset};
use crate::model::{Category, DEFAULT_SKIN_HEIGHT, DEFAULT_SKIN_WIDTH, Destination, Filepath, OffsetDef, PropertyDef, SKIN_TYPE_UNSET, SkinDef};
use crate::property::generated::{OFFSET_ALL, OFFSET_JUDGE_1P, OFFSET_JUDGEDETAIL_1P, OFFSET_NOTES_1P};
use crate::property::{DefaultState, NameSpace, SkinHost, id_of_name};
use crate::resolve::{CustomFile, Draw, FileResolver, build_filemap, contained, enumerate_custom_files, pattern_for};

/// Bytes a document may be before the loader refuses to parse it.
///
/// A skin is third-party data, and the lenient parser walks it character by character, so the
/// ceiling is here to stop a hostile file from turning into a long parse rather than because real
/// documents come close to it. A Lua skin's entry file is held to the same number.
pub const DEFAULT_MAX_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;

/// The extension of a skin whose entry file is a Lua program.
pub const LUA_SKIN_EXTENSION: &str = "luaskin";

/// The byte-order mark a document authored on Windows often starts with.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// The first line and column of a file, as both parsers count them once normalised.
#[cfg(feature = "json5")]
const FIRST_POSITION: usize = 1;

/// The separator every path handed to a skin is written with.
#[cfg(feature = "lua")]
const SKIN_PATH_SEPARATOR: char = '/';

/// Whether `path` names a Lua skin, by its extension and ignoring case.
pub fn is_lua_skin(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(LUA_SKIN_EXTENSION))
}

/// What read a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserKind {
    /// Strict JSON, which almost every document is.
    Json,
    /// The lenient parser, which allows comments, trailing commas, unquoted keys and single quotes.
    Json5,
    /// The Lua runtime: the document is the table a `.luaskin` program returned.
    Lua,
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

/// How one load is to be performed.
#[derive(Debug, Clone, Copy)]
pub struct SkinLoadOptions<'a> {
    /// The directory a document may read files from, and may not read outside of.
    pub root: &'a Path,
    /// The player's choices for this document.
    pub user: &'a SkinUserConfig,
    /// Pins every draw a load makes -- a random option, a random file, a wildcard, and a skin's own
    /// `math.random` -- so a test or a golden run comes out the same each time.
    pub rng_seed: Option<u64>,
    /// The mode a play document is being loaded for.
    pub mode: Mode,
    /// The document size ceiling.
    pub max_document_bytes: u64,
    /// Where a skin's file writes go. A skin never writes into its own folder: with a directory here
    /// its writes land there and are read back from there, and with `None` every write fails. Only
    /// Lua writes files, so this matters to a Lua skin and to a document whose scripts do.
    pub write_overlay: Option<&'a Path>,
}

impl<'a> SkinLoadOptions<'a> {
    /// Options with the defaults every caller but a test wants.
    pub fn new(root: &'a Path, user: &'a SkinUserConfig, mode: Mode) -> Self {
        Self { root, user, rng_seed: None, mode, max_document_bytes: DEFAULT_MAX_DOCUMENT_BYTES, write_overlay: None }
    }
}

/// The play screen for a seven-key chart, and the first of the reference's `SkinType` ids.
pub const SKIN_TYPE_PLAY_7KEYS: i32 = 0;

/// The play screen for a five-key chart.
pub const SKIN_TYPE_PLAY_5KEYS: i32 = 1;

/// The play screen for a fourteen-key chart.
pub const SKIN_TYPE_PLAY_14KEYS: i32 = 2;

/// The play screen for a ten-key chart.
pub const SKIN_TYPE_PLAY_10KEYS: i32 = 3;

/// The play screen for a nine-button chart.
pub const SKIN_TYPE_PLAY_9KEYS: i32 = 4;

/// The twenty-four key play screen.
pub const SKIN_TYPE_PLAY_24KEYS: i32 = 16;

/// The twenty-four key double play screen.
pub const SKIN_TYPE_PLAY_24KEYS_DOUBLE: i32 = 17;

/// The twenty-four key battle screen, and the last of the reference's `SkinType` ids.
pub const SKIN_TYPE_PLAY_24KEYS_BATTLE: i32 = 18;

/// The play screens, in `SkinType` order, and the mode each one draws.
const PLAY_SKIN_MODES: &[(i32, Mode)] = &[
    (SKIN_TYPE_PLAY_7KEYS, Mode::BEAT_7K),
    (SKIN_TYPE_PLAY_5KEYS, Mode::BEAT_5K),
    (SKIN_TYPE_PLAY_14KEYS, Mode::BEAT_14K),
    (SKIN_TYPE_PLAY_10KEYS, Mode::BEAT_10K),
    (SKIN_TYPE_PLAY_9KEYS, Mode::POPN_9K),
    (SKIN_TYPE_PLAY_24KEYS, Mode::KEYBOARD_24K),
];

/// The song browser, in the reference's `SkinType` numbering.
pub const SKIN_TYPE_MUSIC_SELECT: i32 = 5;

/// The screen shown while a chart is being read, which this build draws as its loading screen.
pub const SKIN_TYPE_DECIDE: i32 = 6;

/// The score screen.
pub const SKIN_TYPE_RESULT: i32 = 7;

/// The key configuration screen.
pub const SKIN_TYPE_KEY_CONFIG: i32 = 8;

/// The skin configuration screen.
pub const SKIN_TYPE_SKIN_SELECT: i32 = 9;

/// The score screen of a course.
pub const SKIN_TYPE_COURSE_RESULT: i32 = 15;

/// Every id the reference's `SkinType` has, which is every type a skin can be loaded as. The ids
/// run from the first to the last without a gap.
const KNOWN_SKIN_TYPES: RangeInclusive<i32> = SKIN_TYPE_PLAY_7KEYS..=SKIN_TYPE_PLAY_24KEYS_BATTLE;

/// The non-play screens this build draws: music select, decide, result and key config.
const SCREEN_SKIN_TYPES: &[i32] = &[SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_DECIDE, SKIN_TYPE_RESULT, SKIN_TYPE_KEY_CONFIG];

/// The play screens whose header gains the automatic offsets: every play type but the battle ones
/// (`JSONSkinLoader.loadJsonSkinHeader`).
const AUTOMATIC_OFFSET_SKIN_TYPES: &[i32] = &[
    SKIN_TYPE_PLAY_7KEYS,
    SKIN_TYPE_PLAY_5KEYS,
    SKIN_TYPE_PLAY_14KEYS,
    SKIN_TYPE_PLAY_10KEYS,
    SKIN_TYPE_PLAY_9KEYS,
    SKIN_TYPE_PLAY_24KEYS,
    SKIN_TYPE_PLAY_24KEYS_DOUBLE,
];

/// The axes an offset has: `x`, `y`, `w`, `h`, `r` and `a`.
const OFFSET_AXES: usize = 6;

/// One offset the reference adds to a play skin's header: its name, its id, and the axes the player
/// may move, in the order `x`, `y`, `w`, `h`, `r`, `a`.
type AutomaticOffset = (&'static str, i32, [bool; OFFSET_AXES]);

/// The offsets every play skin has without declaring them, after its own and in this order.
const AUTOMATIC_OFFSETS: &[AutomaticOffset] = &[
    ("All offset(%)", OFFSET_ALL, [true, true, true, true, false, false]),
    ("Notes offset", OFFSET_NOTES_1P, [false, false, false, true, false, false]),
    ("Judge offset", OFFSET_JUDGE_1P, [true, true, true, true, false, true]),
    ("Judge Detail offset", OFFSET_JUDGEDETAIL_1P, [true, true, true, true, false, true]),
];

/// The sizes a skin may be authored at (`Resolution`). A skin that names any other size is read as
/// though it had named the default one.
const SKIN_RESOLUTIONS: &[(i32, i32)] = &[
    (640, 480),
    (800, 600),
    (1024, 768),
    (DEFAULT_SKIN_WIDTH, DEFAULT_SKIN_HEIGHT),
    (1280, 960),
    (1366, 768),
    (1400, 1050),
    (1600, 900),
    (1600, 1200),
    (1680, 1050),
    (1920, 1080),
    (1920, 1200),
    (2048, 1536),
    (2560, 1440),
    (3840, 2160),
];

/// The `judgetimer` of a play skin whose note object is never placed (`PlaySkin.judgetimer`).
const DEFAULT_JUDGE_TIMER: i32 = 1;

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
/// follows [`Mode::ALL`] rather than being restated here. This is the application's question -- can
/// the skin be put on screen -- and narrower than [`is_known_skin_type`], which is the loader's.
pub fn is_supported_skin_type(skin_type: i32) -> bool {
    SCREEN_SKIN_TYPES.contains(&skin_type) || skin_type_mode(skin_type).is_some_and(|mode| Mode::ALL.contains(&mode))
}

/// Whether `skin_type` is one of the reference's `SkinType` ids, which is all a skin needs to be
/// loaded. The skin configuration screen and the course result load as well as any other, whether
/// or not this build draws them yet.
pub fn is_known_skin_type(skin_type: i32) -> bool {
    KNOWN_SKIN_TYPES.contains(&skin_type)
}

/// The offsets the reference adds to the header of a skin of this type: four for a play skin, none
/// for anything else.
pub fn automatic_offsets(skin_type: i32) -> Vec<OffsetDef> {
    if !AUTOMATIC_OFFSET_SKIN_TYPES.contains(&skin_type) {
        return Vec::new();
    }
    AUTOMATIC_OFFSETS
        .iter()
        .map(|(name, id, [x, y, w, h, r, a])| OffsetDef {
            category: String::new(),
            name: (*name).to_owned(),
            id: *id,
            x: *x,
            y: *y,
            w: *w,
            h: *h,
            r: *r,
            a: *a,
        })
        .collect()
}

/// The size a skin that declares `width` by `height` is authored at: the size itself when it is one
/// of the reference's resolutions, and 1280 by 720 otherwise (`JSONSkinLoader.loadJsonSkin`).
pub fn skin_resolution(width: i32, height: i32) -> (i32, i32) {
    if SKIN_RESOLUTIONS.contains(&(width, height)) { (width, height) } else { (DEFAULT_SKIN_WIDTH, DEFAULT_SKIN_HEIGHT) }
}

/// The timings a play screen reads from its skin's header.
///
/// The reference copies these out of the document at one particular moment: while it builds the
/// note object, which happens only when a destination places it (`JsonPlaySkinObjectLoader`). A
/// play skin that declares the fields but never places its notes therefore runs on the defaults,
/// and so does this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayTimings {
    /// Milliseconds between failing and leaving the screen.
    pub close: i32,
    /// Milliseconds loading takes at the least.
    pub loadend: i32,
    /// Milliseconds between "ready" and the first note.
    pub playstart: i32,
    /// The worst judgement that still fires a lane's bomb timer.
    pub judgetimer: i32,
    /// Milliseconds between the last note and the fade out.
    pub finishmargin: i32,
}

impl Default for PlayTimings {
    fn default() -> Self {
        Self { close: 0, loadend: 0, playstart: 0, judgetimer: DEFAULT_JUDGE_TIMER, finishmargin: 0 }
    }
}

impl PlayTimings {
    /// The timings `def` gives a play screen: its own when a destination places its note object,
    /// and the defaults when none does.
    pub fn of(def: &SkinDef) -> Self {
        let placed = def.note.as_ref().is_some_and(|note| def.destination.iter().any(|destination| destination.id == note.id));
        if !placed {
            return Self::default();
        }
        Self { close: def.close, loadend: def.loadend, playstart: def.playstart, judgetimer: def.judgetimer, finishmargin: def.finishmargin }
    }
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
    /// The headings the configuration screen groups the rows below under.
    pub categories: Vec<Category>,
    pub properties: Vec<PropertyDef>,
    /// The offsets the skin declares, followed by the ones every play skin has
    /// ([`automatic_offsets`]).
    pub offsets: Vec<OffsetDef>,
    pub custom_files: Vec<CustomFile>,
}

/// One assembled track, still carrying the object id the document attached it to.
#[derive(Debug, Clone)]
pub struct NamedTrack {
    pub id: String,
    pub track: DestinationTrack,
}

/// One judgement pop-up's assembled tracks: the images it shows and the count that rides with them.
#[derive(Debug, Default, Clone)]
pub struct JudgeTracks {
    pub images: Vec<NamedTrack>,
    /// The combo beside the pop-up, assembled with the relative flag the reference sets on it.
    pub numbers: Vec<NamedTrack>,
}

/// The song wheel's assembled tracks, one entry per slot in each list.
#[derive(Debug, Default, Clone)]
pub struct SongListTracks {
    pub listoff: Vec<NamedTrack>,
    pub liston: Vec<NamedTrack>,
    pub text: Vec<NamedTrack>,
    pub level: Vec<NamedTrack>,
    pub lamp: Vec<NamedTrack>,
    pub playerlamp: Vec<NamedTrack>,
    pub rivallamp: Vec<NamedTrack>,
    pub trophy: Vec<NamedTrack>,
    pub label: Vec<NamedTrack>,
    pub graph: Option<NamedTrack>,
}

/// The destinations a document nests inside its repeating objects, assembled once at load.
///
/// The top-level `destination` list is the only one a document draws directly; a note set, a
/// judgement pop-up and a song wheel each carry their own lists, which the reference assembles the
/// same way and at the same moment. Keeping them here rather than re-assembling them per frame is
/// what lets the renderer stay free of the interpolator's build path, and leaves every Lua
/// compilation and every warning in one place.
#[derive(Debug, Default, Clone)]
pub struct NestedTracks {
    /// Bar lines, one track per note-set `group` entry.
    pub note_group: Vec<NamedTrack>,
    pub note_bpm: Vec<NamedTrack>,
    pub note_stop: Vec<NamedTrack>,
    pub note_time: Vec<NamedTrack>,
    /// The pop-ups, keyed by the id the document gave each judge object.
    pub judge: BTreeMap<String, JudgeTracks>,
    pub songlist: Option<SongListTracks>,
}

/// A skin, read and ready to draw.
pub struct LoadedSkin {
    /// The document: what a JSON skin wrote, or what a Lua skin's body pass returned with the
    /// customisation rows of its header pass.
    pub def: SkinDef,
    pub path: PathBuf,
    pub root: PathBuf,
    pub parser: ParserKind,
    pub mode: Mode,
    /// The size the skin is authored at, as the reference reads it ([`skin_resolution`]): the size
    /// the skin declares when that is one of the reference's resolutions, and 1280 by 720 when it
    /// is not.
    ///
    /// For a Lua skin `def.w` and `def.h` hold the same two numbers, so everything that scales the
    /// skin agrees. A JSON document's `def.w` and `def.h` are left as it wrote them, and whatever
    /// still scales a document by those draws one of an unlisted size at the size it declared.
    pub resolution: (i32, i32),
    /// The timings a play screen takes from this skin.
    pub play: PlayTimings,
    /// The file slots the configuration screen offers, scanned once at load.
    pub custom_files: Vec<CustomFile>,
    /// The offsets the skin declares, followed by the ones every play skin has
    /// ([`automatic_offsets`]).
    pub offsets: Vec<OffsetDef>,
    /// The option each customisation row is on for this load, by row name and in the skin's order.
    pub selected_options: Vec<(String, i32)>,
    /// The option ids the player's choices turn on.
    pub enabled_options: BTreeSet<i32>,
    /// Every option id this document declares, on or off.
    pub declared_options: BTreeSet<i32>,
    /// Image source id to the file it resolved to.
    pub sources: BTreeMap<String, PathBuf>,
    /// Font id to the file it resolved to.
    pub fonts: BTreeMap<String, PathBuf>,
    /// The document's top-level destinations, assembled, less the ones preparing the skin removed
    /// (`Skin.prepare`): an object asking for one of the skin's own options that its customisation
    /// rows do not grant, and an object one of whose conditions is settled once on this screen and
    /// came out false. The objects that stay carry only the conditions a frame still has to ask.
    pub destinations: Vec<NamedTrack>,
    /// The destinations its repeating objects nest, assembled alongside the top-level ones.
    pub nested: NestedTracks,
    /// Everything that went wrong without costing the skin.
    pub warnings: Vec<String>,
    filemap: BTreeMap<String, String>,
    resolver: Rc<RefCell<FileResolver>>,
    option_selections: BTreeMap<i32, bool>,
    #[cfg(feature = "lua")]
    runtime: Option<crate::lua::SkinLua>,
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
        &self.filemap
    }

    /// The file a document-relative path names, checked to be inside the skin root.
    pub fn resolve(&mut self, relative: &str) -> Result<PathBuf, SkinError> {
        let directory = self.path.parent().unwrap_or(&self.root).to_path_buf();
        self.resolver.borrow_mut().resolve(&pattern_for(&directory, relative))
    }

    /// The interpreter this skin's Lua lives in: the functions a Lua skin's objects are made of, or
    /// the scripts a JSON document wrote as strings. A frame binds its host to it once and makes
    /// every call through that binding ([`SkinLua::frame`](crate::lua::SkinLua::frame)).
    ///
    /// The skin owns it: the function ids in the model are indices into this interpreter's registry
    /// and mean nothing once it is gone, so it lives exactly as long as the loaded skin does.
    #[cfg(feature = "lua")]
    pub fn runtime(&self) -> Option<&crate::lua::SkinLua> {
        self.runtime.as_ref()
    }

    /// Assembles one destination into a track, or `None` when this document's own customisation
    /// choices mean it is never drawn: it asks for one of the skin's own options, and the rows that
    /// offer those do not grant it.
    ///
    /// `relative` is the flag the play screen sets on judge-count objects and nothing else, which
    /// is where the reference sets it too (`JsonPlaySkinObjectLoader`).
    ///
    /// A destination handed in from outside may still carry a script as the string a document wrote
    /// it as; it is compiled into this skin's interpreter first, as the loader does for the skin's
    /// own. No host is bound here, so a timer script that reads game state in its one trial call
    /// costs the destination its timer, and a condition that holds still on the screen being shown
    /// is left for every frame to ask rather than settled once.
    pub fn build_track(&mut self, destination: &Destination, relative: bool) -> Result<Option<DestinationTrack>, SkinError> {
        let mut destination = destination.clone();
        self.settle(&mut destination)?;
        let built = self.assemble_track(&destination, relative)?;
        Ok(options_hold(&built.options, &self.option_selections).then_some(built.track))
    }

    /// Compiles the scripts one destination still carries as strings.
    #[cfg(feature = "lua")]
    fn settle(&mut self, destination: &mut Destination) -> Result<(), SkinError> {
        let Some(lua) = self.runtime.as_ref() else {
            return script::each_destination_slot(destination, &mut script::refuse);
        };
        let path = self.path.to_string_lossy().into_owned();
        let mut settler = script::Settler { lua, path: &path, warnings: &mut self.warnings };
        script::each_destination_slot(destination, &mut |slot| settler.settle(slot))
    }

    /// A build without Lua refuses a destination that carries a script.
    #[cfg(not(feature = "lua"))]
    fn settle(&mut self, destination: &mut Destination) -> Result<(), SkinError> {
        script::each_destination_slot(destination, &mut script::refuse)
    }

    /// Assembles a destination whose scripts are already settled.
    fn assemble_track(&mut self, destination: &Destination, relative: bool) -> Result<track::BuiltTrack, SkinError> {
        let path = self.path.to_string_lossy().into_owned();
        let mut context = track::TrackContext { path: &path, relative, warnings: &mut self.warnings };
        track::build_track(destination, &mut context)
    }
}

/// Whether the skin's own options leave an object in the draw list (`Skin.prepare`).
///
/// `options` are the ids of the object's `op` list no built-in property answers. A positive one must
/// be an id a customisation row offers and is switched to. A negative one must name an id a row
/// offers and is switched away from. An id no row offers is neither, so it removes the object
/// whichever sign the skin wrote it with.
fn options_hold(options: &[i32], selections: &BTreeMap<i32, bool>) -> bool {
    options.iter().all(|option| if *option > 0 { selections.get(option) == Some(&true) } else { selections.get(&option.wrapping_neg()) == Some(&false) })
}

/// Settles the conditions of one object that hold still on the screen being entered, and answers
/// whether the object stays in the draw list (`Skin.prepare`).
///
/// A built-in option the host calls static is asked once, here: a false answer removes the object
/// for as long as the skin is loaded, and a true one removes only the condition, so no frame asks
/// again. A property the skin named is the same property as its id and is settled the same way. A
/// function is never static. Every condition is looked at, whatever the ones before it answered.
///
/// The reference has an answer for every option it calls static. A host that has none yet for one
/// of them settles nothing: the condition stays, and each frame asks as it would have.
fn settle_static(track: &mut DestinationTrack, host: &dyn SkinHost) -> bool {
    let mut stays = true;
    track.draw_conditions.retain(|condition| {
        let id = match condition {
            DrawCondition::Option(id) => Some(*id),
            DrawCondition::Name(name) => id_of_name(NameSpace::Boolean, name),
            DrawCondition::Function(_) => None,
        };
        let Some(holds) = id.filter(|id| host.is_static(*id)).and_then(|id| host.boolean(id)) else {
            return true;
        };
        stays &= holds;
        false
    });
    stays
}

/// Removes the objects that are certain never to be drawn and strips the conditions that are
/// certain always to hold, once, after everything is built (`Skin.prepare`).
///
/// Only the top-level objects pass through here, as in the reference, whose `prepare` walks the
/// skin's own object list and none of the objects nested inside a note field, a judgement pop-up or
/// a song wheel.
fn prepare_objects(objects: Vec<(String, track::BuiltTrack)>, selections: &BTreeMap<i32, bool>, host: &dyn SkinHost) -> Vec<NamedTrack> {
    objects
        .into_iter()
        .filter_map(|(id, built)| {
            let mut track = built.track;
            let stays = options_hold(&built.options, selections) && settle_static(&mut track, host);
            stays.then_some(NamedTrack { id, track })
        })
        .collect()
}

/// Assembles one nested destination, falling back to a track that never draws.
///
/// A slot the document rules out with its own customisation choices, and one whose keyframes will
/// not assemble, both come back as an empty track rather than as nothing at all: the position of an
/// entry is the slot it belongs to -- judgement number, wheel row, bar line -- so dropping one would
/// move every slot behind it. An empty track resolves to nothing, which is what a slot that is not
/// there should look like.
///
/// A slot's conditions are never settled once, because the reference never prepares a nested object:
/// each of them is asked on every frame the slot is drawn.
fn build_slot(skin: &mut LoadedSkin, what: &str, destination: &Destination, relative: bool) -> Result<NamedTrack, SkinError> {
    let track = match skin.assemble_track(destination, relative) {
        Ok(built) if options_hold(&built.options, &skin.option_selections) => built.track,
        Ok(_) => DestinationTrack::default(),
        Err(error @ SkinError::LuaUnavailable) => return Err(error),
        Err(error) => {
            skin.warnings.push(format!("{what} {:?} was skipped: {error}", destination.id));
            DestinationTrack::default()
        }
    };
    Ok(NamedTrack { id: destination.id.clone(), track })
}

/// Assembles a whole nested list, one entry per slot the document declared.
fn build_slots(skin: &mut LoadedSkin, what: &str, list: &[Destination], relative: bool) -> Result<Vec<NamedTrack>, SkinError> {
    let mut tracks = Vec::with_capacity(list.len());
    for destination in list {
        tracks.push(build_slot(skin, what, destination, relative)?);
    }
    Ok(tracks)
}

/// Assembles the destinations the note set, the judgement pop-ups and the song wheel nest.
///
/// The reference assembles these at the same moment as the top-level list and from the same builder
/// (`JsonPlaySkinObjectLoader`, `JsonSelectSkinObjectLoader`), which is what keeps one document's
/// Lua compilation and one document's warnings in one place.
fn build_nested(skin: &mut LoadedSkin) -> Result<(), SkinError> {
    let note = skin.def.note.take();
    if let Some(note) = note.as_ref() {
        skin.nested.note_group = build_slots(skin, "note bar line", &note.group, false)?;
        skin.nested.note_bpm = build_slots(skin, "note bpm line", &note.bpm, false)?;
        skin.nested.note_stop = build_slots(skin, "note stop line", &note.stop, false)?;
        skin.nested.note_time = build_slots(skin, "note time line", &note.time, false)?;
    }
    skin.def.note = note;

    let judges = std::mem::take(&mut skin.def.judge);
    for judge in &judges {
        let images = build_slots(skin, "judge image", &judge.images, false)?;
        let numbers = build_slots(skin, "judge number", &judge.numbers, true)?;
        skin.nested.judge.insert(judge.id.clone(), JudgeTracks { images, numbers });
    }
    skin.def.judge = judges;

    let songlist = skin.def.songlist.take();
    if let Some(list) = songlist.as_ref() {
        let tracks = SongListTracks {
            listoff: build_slots(skin, "song bar", &list.listoff, false)?,
            liston: build_slots(skin, "selected song bar", &list.liston, false)?,
            text: build_slots(skin, "song bar text", &list.text, false)?,
            level: build_slots(skin, "song bar level", &list.level, false)?,
            lamp: build_slots(skin, "song bar lamp", &list.lamp, false)?,
            playerlamp: build_slots(skin, "song bar player lamp", &list.playerlamp, false)?,
            rivallamp: build_slots(skin, "song bar rival lamp", &list.rivallamp, false)?,
            trophy: build_slots(skin, "song bar trophy", &list.trophy, false)?,
            label: build_slots(skin, "song bar label", &list.label, false)?,
            graph: list.graph.as_ref().map(|graph| build_slot(skin, "song bar graph", graph, false)).transpose()?,
        };
        skin.nested.songlist = Some(tracks);
    }
    skin.def.songlist = songlist;

    Ok(())
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

/// The player's choices applied to a skin's header: what `SkinHeader.setSkinConfigProperty` leaves
/// behind, and the file map `JSONSkinLoader.load` builds from it.
pub(crate) struct MergedHeader {
    /// The option each customisation row is on, by row name and in the skin's order.
    pub options: Vec<(String, i32)>,
    /// Every option id the rows can turn on.
    pub declared: BTreeSet<i32>,
    /// The skin's own options as the reference keeps them ([`option_selections`]).
    pub selections: BTreeMap<i32, bool>,
    /// The declared offsets followed by the automatic ones.
    pub offsets: Vec<OffsetDef>,
    pub custom_files: Vec<CustomFile>,
    pub filemap: BTreeMap<String, String>,
    /// Resolves this load's patterns. Shared, because a Lua skin's `skin_config.get_path` goes on
    /// resolving through it for as long as the skin is drawn.
    pub resolver: Rc<RefCell<FileResolver>>,
}

impl MergedHeader {
    /// The option ids the choices turn on.
    pub(crate) fn enabled(&self) -> BTreeSet<i32> {
        self.options.iter().map(|(_, selected)| *selected).collect()
    }
}

/// What a skin's header declares that a player may customise.
pub(crate) struct HeaderRows<'a> {
    pub skin_type: i32,
    pub properties: &'a [PropertyDef],
    pub filepath: &'a [Filepath],
    pub offsets: &'a [OffsetDef],
}

/// The declared offsets of a skin of `skin_type` followed by its automatic ones.
fn header_offsets(skin_type: i32, declared: &[OffsetDef]) -> Vec<OffsetDef> {
    declared.iter().cloned().chain(automatic_offsets(skin_type)).collect()
}

/// The skin's own options as `Skin.option` holds them: every id a customisation row offers, and
/// whether that row is switched to it (`JSONSkinLoader.loadJsonSkin`).
///
/// `selected` is the option each row is on, row for row. The rows are entered in order, so an id two
/// rows offer reads the way the later row has it.
fn option_selections(properties: &[PropertyDef], selected: &[(String, i32)]) -> BTreeMap<i32, bool> {
    properties.iter().zip(selected).flat_map(|(row, (_, chosen))| row.item.iter().map(move |item| (item.op, item.op == *chosen))).collect()
}

/// Applies the player's choices to a header.
///
/// `directory` is the folder the skin's paths are written against and `root` the folder nothing may
/// be read outside of. The draws are made in the reference's order -- options, then file slots --
/// from the one generator the wildcards are later drawn with, so a seed pins all three.
pub(crate) fn merge_header(rows: &HeaderRows<'_>, directory: &Path, root: &Path, options: &SkinLoadOptions<'_>) -> MergedHeader {
    let mut draw = options.rng_seed.map_or_else(Draw::from_environment, Draw::from_seed);
    let selected = merged_options(rows.properties, &options.user.properties, &mut draw);
    let slots = SkinDef { filepath: rows.filepath.to_vec(), ..SkinDef::default() };
    let custom_files = enumerate_custom_files(&slots, directory, root);
    let filemap = build_filemap(&custom_files, options.user, &mut draw);
    MergedHeader {
        selections: option_selections(rows.properties, &selected),
        options: selected,
        declared: declared_options(rows.properties),
        offsets: header_offsets(rows.skin_type, rows.offsets),
        custom_files,
        resolver: Rc::new(RefCell::new(FileResolver::new(root, filemap.clone(), draw))),
        filemap,
    }
}

/// A path as `java.io.File.getPath` spells it: runs of separators collapsed and no trailing one.
#[cfg(feature = "lua")]
fn file_path_text(path: &str) -> String {
    let mut text = String::with_capacity(path.len());
    for character in path.chars() {
        if character != SKIN_PATH_SEPARATOR || !text.ends_with(SKIN_PATH_SEPARATOR) {
            text.push(character);
        }
    }
    if text.len() > SKIN_PATH_SEPARATOR.len_utf8() && text.ends_with(SKIN_PATH_SEPARATOR) {
        text.pop();
    }
    text
}

/// The `skin_config` global a skin's Lua reads (`SkinLuaAccessor.exportSkinProperty`).
///
/// `pattern_base` is the spelling of the skin's folder the file map is keyed under and `lua_root`
/// the spelling the interpreter knows it by; `get_path` resolves under the first and answers under
/// the second, so the path it hands back is one `dofile`, `io.open` and the audio calls accept.
///
/// Offsets are stored by id here, where the reference stores them by name, so the value published
/// under an offset's name is the one stored under its id.
#[cfg(feature = "lua")]
pub(crate) fn skin_config_global(merged: &MergedHeader, user: &SkinUserConfig, pattern_base: &Path, lua_root: &Path) -> crate::lua::SkinConfigGlobal {
    let resolver = Rc::clone(&merged.resolver);
    let base = pattern_for(pattern_base, "");
    let root = pattern_for(lua_root, "");
    crate::lua::SkinConfigGlobal {
        options: merged.options.clone(),
        file_paths: user.filepaths.iter().map(|(name, path)| (name.clone(), path.clone())).collect(),
        offsets: merged.offsets.iter().map(|offset| (offset.name.clone(), user.offsets.get(&offset.id).copied().unwrap_or_default())).collect(),
        get_path: Box::new(move |relative: &str| {
            let located = resolver.borrow_mut().path_text(&pattern_for(Path::new(&base), relative));
            let tail = located.strip_prefix(base.as_str()).unwrap_or(&located);
            file_path_text(&format!("{root}{tail}"))
        }),
    }
}

/// Everything the two kinds of skin hand to the part of a load they share.
pub(crate) struct Assembly {
    pub def: SkinDef,
    pub path: PathBuf,
    pub root: PathBuf,
    pub parser: ParserKind,
    pub mode: Mode,
    pub merged: MergedHeader,
    pub warnings: Vec<String>,
    #[cfg(feature = "lua")]
    pub runtime: Option<crate::lua::SkinLua>,
}

/// Resolves the files a skin names, assembles its destinations (`JSONSkinLoader.loadJsonSkin`) and
/// then prepares the result for the screen being entered (`Skin.prepare`).
///
/// `host` is that screen. It is asked once, after everything is built, for each condition it says
/// holds still there, so its state has to be settled before a skin is loaded against it.
pub(crate) fn assemble(parts: Assembly, host: &dyn SkinHost) -> Result<LoadedSkin, SkinError> {
    let mut skin = LoadedSkin {
        resolution: skin_resolution(parts.def.w, parts.def.h),
        play: PlayTimings::of(&parts.def),
        path: parts.path,
        root: parts.root,
        parser: parts.parser,
        mode: parts.mode,
        custom_files: parts.merged.custom_files,
        offsets: parts.merged.offsets,
        enabled_options: parts.merged.options.iter().map(|(_, selected)| *selected).collect(),
        selected_options: parts.merged.options,
        declared_options: parts.merged.declared,
        sources: BTreeMap::new(),
        fonts: BTreeMap::new(),
        destinations: Vec::new(),
        nested: NestedTracks::default(),
        warnings: parts.warnings,
        filemap: parts.merged.filemap,
        resolver: parts.merged.resolver,
        option_selections: parts.merged.selections,
        #[cfg(feature = "lua")]
        runtime: parts.runtime,
        def: parts.def,
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
    let mut objects = Vec::with_capacity(destinations.len());
    for destination in &destinations {
        match skin.assemble_track(destination, false) {
            Ok(built) => objects.push((destination.id.clone(), built)),
            Err(error @ SkinError::LuaUnavailable) => return Err(error),
            Err(error) => skin.warnings.push(format!("object {:?} was skipped: {error}", destination.id)),
        }
    }
    skin.def.destination = destinations;

    build_nested(&mut skin)?;

    skin.destinations = prepare_objects(objects, &skin.option_selections, host);

    Ok(skin)
}

/// Refuses a skin that declares no type, or one the reference has no screen for.
pub(crate) fn check_skin_type(skin_type: i32) -> Result<(), SkinError> {
    if skin_type == SKIN_TYPE_UNSET {
        return Err(SkinError::TypeMissing);
    }
    if !is_known_skin_type(skin_type) {
        return Err(SkinError::TypeUnsupported(skin_type));
    }
    Ok(())
}

/// What a skin says about itself, without loading it.
///
/// A `.luaskin` runs its header pass in an interpreter of its own with no game state behind it
/// ([`lua_skin::load_lua_header`]); anything else is parsed as a document.
pub fn load_header(path: &Path, options: SkinLoadOptions<'_>) -> Result<SkinHeader, SkinError> {
    if is_lua_skin(path) {
        #[cfg(feature = "lua")]
        return lua_skin::load_lua_header(path, &lua_skin::LuaSkinOptions::new(options));
        #[cfg(not(feature = "lua"))]
        return Err(SkinError::LuaUnavailable);
    }
    load_document_header(path, options)
}

/// What a document says about itself.
fn load_document_header(path: &Path, options: SkinLoadOptions<'_>) -> Result<SkinHeader, SkinError> {
    let path = contained(options.root, path)?;
    let text = read_document(&path, options.max_document_bytes)?;
    let (value, parser) = parse_value(&path, &text)?;
    let directory = path.parent().unwrap_or(options.root).to_path_buf();
    let filepath: Vec<Filepath> = header_list(&value, "filepath");
    let document = SkinDef { filepath, ..SkinDef::default() };
    let skin_type = header_number(&value, "type", SKIN_TYPE_UNSET);
    let declared: Vec<OffsetDef> = header_list(&value, "offset");

    Ok(SkinHeader {
        skin_type,
        name: header_text(&value, "name"),
        author: header_text(&value, "author"),
        width: header_number(&value, "w", DEFAULT_SKIN_WIDTH),
        height: header_number(&value, "h", DEFAULT_SKIN_HEIGHT),
        parser,
        categories: header_list(&value, "category"),
        properties: header_list(&value, "property"),
        offsets: header_offsets(skin_type, &declared),
        custom_files: enumerate_custom_files(&document, &directory, options.root),
        path,
    })
}

/// Reads a skin and everything it names, with no game state behind it.
///
/// This is [`load_skin_with_host`] against a host that knows nothing, which is all a document with no
/// scripts needs. A Lua skin reads the game state while it builds its screen, so whoever is about to
/// draw one loads it with the host of the screen being entered.
pub fn load_skin(path: &Path, options: SkinLoadOptions<'_>) -> Result<LoadedSkin, SkinError> {
    load_skin_with_host(path, options, &DefaultState)
}

/// Reads a skin and everything it names.
///
/// A `.luaskin` is run as a program ([`lua_skin::load_lua_skin`]) and anything else is parsed as a
/// document. `host` answers whatever the skin's Lua reads while it loads: the whole body pass of a
/// Lua skin, and the one trial call a timer script gets in either kind. It is also asked, once the
/// skin is assembled, for every draw condition it calls static ([`SkinHost::is_static`]): an object
/// whose static condition is false is removed for as long as the skin stays loaded, and a static
/// condition that holds is never asked again (`Skin.prepare`). The state of the screen being entered
/// therefore has to be settled before this is called.
///
/// The skin's type is checked before any of it is assembled: a skin that declares none is refused
/// rather than guessed at, and so is one that declares a type the reference does not have. Whether
/// this build has a screen to put the skin on is the caller's question ([`is_supported_skin_type`]).
pub fn load_skin_with_host(path: &Path, options: SkinLoadOptions<'_>, host: &dyn SkinHost) -> Result<LoadedSkin, SkinError> {
    if is_lua_skin(path) {
        #[cfg(feature = "lua")]
        return lua_skin::load_lua_skin(path, &lua_skin::LuaSkinOptions::new(options), host);
        #[cfg(not(feature = "lua"))]
        return Err(SkinError::LuaUnavailable);
    }
    load_document(path, options, host)
}

/// The interpreter a document's scripts are compiled into (`SkinLuaAccessor(true)`): the one a Lua
/// skin gets, rooted at the document's folder, with the game state published as globals.
#[cfg(feature = "lua")]
fn document_runtime(directory: &Path, options: &SkinLoadOptions<'_>) -> Result<crate::lua::SkinLua, SkinError> {
    let config = crate::lua::SkinLuaConfig {
        overlay: options.write_overlay.map(Path::to_path_buf),
        seed: options.rng_seed,
        ..crate::lua::SkinLuaConfig::new(directory)
    };
    let runtime = crate::lua::SkinLua::new(config)?;
    runtime.publish_globals()?;
    Ok(runtime)
}

/// Compiles every script a document wrote as a string into a fresh interpreter, and hands the
/// interpreter back for the loaded skin to keep.
#[cfg(feature = "lua")]
fn settle_scripts(
    def: &mut SkinDef,
    path: &Path,
    directory: &Path,
    merged: &MergedHeader,
    options: &SkinLoadOptions<'_>,
    host: &dyn SkinHost,
    warnings: &mut Vec<String>,
) -> Result<crate::lua::SkinLua, SkinError> {
    let runtime = document_runtime(directory, options)?;
    runtime.set_skin_config(skin_config_global(merged, options.user, directory, runtime.paths().root()))?;
    let name = path.to_string_lossy().into_owned();
    let mut settler = script::Settler { lua: &runtime, path: &name, warnings };
    runtime.with_host(host, || script::each_slot(def, &mut |slot| settler.settle(slot)))??;
    Ok(runtime)
}

/// A build without Lua has nothing to compile a script into and nothing for a host to answer, so a
/// document that carries one is refused.
#[cfg(not(feature = "lua"))]
fn refuse_scripts(def: &mut SkinDef, _host: &dyn SkinHost) -> Result<(), SkinError> {
    script::each_slot(def, &mut script::refuse)
}

/// Reads a document and everything it names.
fn load_document(path: &Path, options: SkinLoadOptions<'_>, host: &dyn SkinHost) -> Result<LoadedSkin, SkinError> {
    let path = contained(options.root, path)?;
    let text = read_document(&path, options.max_document_bytes)?;
    let (mut value, parser) = parse_value(&path, &text)?;
    let directory = path.parent().unwrap_or(options.root).to_path_buf();

    let skin_type = header_number(&value, "type", SKIN_TYPE_UNSET);
    check_skin_type(skin_type)?;

    let properties: Vec<PropertyDef> = header_list(&value, "property");
    let filepath: Vec<Filepath> = header_list(&value, "filepath");
    let offsets: Vec<OffsetDef> = header_list(&value, "offset");
    let merged = merge_header(&HeaderRows { skin_type, properties: &properties, filepath: &filepath, offsets: &offsets }, &directory, options.root, &options);
    let enabled = merged.enabled();
    let mut warnings: Vec<String> = Vec::new();

    let limit = options.max_document_bytes;
    let mut include = |target: &str| -> Result<Value, SkinError> {
        let file = merged.resolver.borrow_mut().resolve(&pattern_for(&directory, target))?;
        let text = read_document(&file, limit)?;
        parse_value(&file, &text).map(|(value, _)| value)
    };
    branch::transform(&mut value, &mut branch::BranchContext::new(&enabled, &mut include, &mut warnings))?;

    let mut def = from_value(&path, value)?;

    #[cfg(feature = "lua")]
    let runtime = Some(settle_scripts(&mut def, &path, &directory, &merged, &options, host, &mut warnings)?);
    #[cfg(not(feature = "lua"))]
    refuse_scripts(&mut def, host)?;

    assemble(
        Assembly {
            def,
            path,
            root: options.root.to_path_buf(),
            parser,
            mode: options.mode,
            merged,
            warnings,
            #[cfg(feature = "lua")]
            runtime,
        },
        host,
    )
}
