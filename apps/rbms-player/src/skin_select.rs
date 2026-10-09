//! The SKIN tab: the documents found on disk, the one each screen is drawn with, and the
//! customisation rows the chosen document declares.
//!
//! A document declares its own rows — a list of named options, a file slot filled from a wildcard,
//! a nudge applied to one destination — so the SKIN tab cannot be a fixed table the way every other
//! tab is. The six rows that are always there (the pack folder, the screen, the document, what
//! loaded, reload and reset) stay in `rbms_config::SETTINGS` with the rest of the screen; the rows
//! below them are built here from the chosen document's header and are addressed by [`SkinRow`].
//!
//! Only the header is read to build the rows, which is one parse and one directory scan per
//! document rather than a whole load; the whole document is read when the player asks for it, or
//! when a choice that changes what is drawn moves. A Lua skin's header is a program run in an
//! interpreter of its own with no game behind it (`load_lua_header`), so what it answered is kept
//! against the file's modification time and the folder can be walked again without running it.
//!
//! Documents come from two places. The skin folder is walked a few directories deep and offers
//! everything in it to the SKIN row. A skin pack is one folder holding a document for each screen:
//! naming one -- on the PACK FOLDER row, or for one run with [`SKIN_PACK_ENV`] -- gives every screen
//! the player has not chosen a document for by hand the pack's document that declares it.
//!
//! A document is read whole in one of two ways. One that is only data is read the moment it is
//! asked for. A Lua skin builds its screen by reading the game while it runs (`load_lua_skin`), so
//! it waits until the screen it draws is on its first frame and is read against that screen's own
//! state; the library holds the request until then.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rbms_config::{Config, DEFAULT_SKIN_FOLDER, DEFAULT_VALUE, NONE_VALUE, skin_document_label};
use rbms_skin::dst::SkinOffset;
use rbms_skin::loader::{
    LoadedSkin, ParserKind, SkinHeader, SkinLoadOptions, SkinUserConfig, is_lua_skin, is_supported_skin_type, load_header, load_skin_with_host,
    selected_option, skin_type_mode,
};
use rbms_skin::model::{OffsetDef, PropertyDef};
use rbms_skin::property::{DefaultState, SkinHost};
use rbms_skin::resolve::{CustomFile, RANDOM_SELECTION};

use crate::assets::skin_overlay_folder;

/// Environment variable naming a skin pack folder for this run only. It outranks the pack the
/// settings file names and is never written back to it.
pub(crate) const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// How deep under the skin folder documents are looked for: the folder itself, one directory per
/// skin, and one more for the skins that group their variants in a subdirectory.
const SCAN_MAX_DEPTH: usize = 3;

/// How many documents one walk of the skin folder will collect. A folder with more than this is
/// almost certainly the wrong folder, and the row could not be cycled through them anyway.
const SCAN_MAX_DOCUMENTS: usize = 512;

/// How deep a skin pack is looked into: the pack is one folder, and the documents are in it.
const PACK_SCAN_DEPTH: usize = 1;

/// The extensions a skin document is written with: a Lua skin's entry file, then the two dialects
/// of a document, strict parser first.
const DOCUMENT_EXTENSIONS: [&str; 3] = ["luaskin", "json", "json5"];

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

/// Shown on the LOADED row for a Lua skin that is read when the screen it draws next opens.
const WAITING_INFO: &str = "READ WHEN ITS SCREEN OPENS";

/// What the SKIN row puts before the name of a document the pack chose rather than the player.
const PACK_DOCUMENT_PREFIX: &str = "PACK: ";

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

/// What a read of a whole document is made with, beyond the choices stored for it.
#[derive(Clone, Copy)]
pub(crate) struct SkinRead<'a> {
    /// Answers whatever the skin's Lua reads while it loads, which for a Lua skin is the state of
    /// the screen it is about to draw.
    pub(crate) host: &'a dyn SkinHost,
    /// Pins every draw the read makes -- a random option, a random file, a skin's own
    /// `math.random` -- or leaves them to chance with `None`.
    pub(crate) seed: Option<u64>,
}

impl SkinRead<'static> {
    /// A read with no game behind it, which is all a document with no scripts needs.
    pub(crate) fn unhosted(seed: Option<u64>) -> SkinRead<'static> {
        SkinRead { host: &DefaultState, seed }
    }
}

/// The documents on disk, and the one being drawn for the screen the SKIN tab is configuring.
///
/// The folder is walked once when the settings screen opens rather than every frame, so a folder on
/// a slow disk costs one pause rather than a stutter per row.
pub(crate) struct SkinLibrary {
    /// The settings file, beside which the skins' own writes are kept.
    settings_path: PathBuf,
    /// The directory documents are looked for in, which is also the only directory a document
    /// outside the pack may read files from.
    root: PathBuf,
    /// The pack this run was started with, which outranks the configured one.
    forced_pack: Option<PathBuf>,
    /// The pack in effect when the folders were last walked.
    pack: Option<PathBuf>,
    /// Every document found under the root and in the pack, by screen and then by name.
    documents: Vec<SkinHeader>,
    /// The pack's document for each screen it has one for.
    pack_documents: BTreeMap<i32, String>,
    /// The documents that were found in the pack, which are the ones read with the pack as their
    /// root.
    pack_paths: BTreeSet<PathBuf>,
    /// What every header read so far answered.
    headers: HeaderCache,
    /// The document each screen is drawn with, once it has been read whole.
    ///
    /// One entry per screen rather than one for the whole library, because the screens outlive the
    /// tab that chose them: the browser is drawn with its own document while a run is being
    /// configured on another.
    loaded: BTreeMap<i32, LoadedDocument>,
    /// Why one screen's chosen document is not being drawn, when it is not, with the document that
    /// failed: a reason belongs to the file it was raised by, not to the screen.
    errors: BTreeMap<i32, ReadFailure>,
    /// The screens whose choices have moved since their document was last read.
    stale: BTreeSet<i32>,
    /// The Lua skins waiting for their screen's first frame, by screen, with the path asked for.
    waiting: BTreeMap<i32, String>,
    /// The waiting skins that have been read and not yet taken in.
    ///
    /// A frame is drawn through a shared borrow of the application, and the frame is the first place
    /// a screen's state exists to read a Lua skin against. So the read lands here, and the next
    /// frame's preparation, which holds the application exclusively, moves it into `loaded`.
    arrived: RefCell<BTreeMap<i32, Arrival>>,
    /// The failures nobody has been told about yet, by screen.
    unannounced: BTreeMap<i32, String>,
    /// What every read is pinned with, or `None` to leave the draws to chance.
    seed: Option<u64>,
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

/// A read of one screen's document that failed.
struct ReadFailure {
    /// The path that was read, so a screen that is since drawn with another document is not held to
    /// this one's failure.
    path: String,
    /// What the read said, whole.
    reason: String,
}

/// A waiting skin's read, on its way into the library.
struct Arrival {
    /// The path that was read, so a selection that moved in the meantime is noticed.
    path: String,
    outcome: Result<Box<LoadedSkin>, String>,
}

/// What a document's header answered, and the file it was answered from.
struct CachedHeader {
    /// The file's modification time and length when it was read. A file that cannot be stat'ed has
    /// neither and is never answered from memory.
    stamp: Option<(SystemTime, u64)>,
    /// The header, or `None` for a file that is not a skin this build can read.
    header: Option<SkinHeader>,
}

/// Every header read so far, by the path it was found under.
///
/// Reading a Lua skin's header runs its program, and a pack is a dozen of them; the folder is
/// walked again every time the settings screen opens. A header is therefore read once per version
/// of its file. What the header says about the files beside it -- the candidates of a file slot --
/// is as old as the read, which is why asking for a reload starts from nothing.
#[derive(Default)]
struct HeaderCache {
    entries: BTreeMap<PathBuf, CachedHeader>,
    /// How many headers were read from disk rather than answered from memory.
    reads: u64,
}

impl HeaderCache {
    /// The header of the document at `path`, read with `root` as the folder it may reach into.
    fn header(&mut self, path: &Path, root: &Path) -> Option<SkinHeader> {
        let stamp = std::fs::metadata(path).ok().and_then(|file| Some((file.modified().ok()?, file.len())));
        if let Some(known) = self.entries.get(path).filter(|known| stamp.is_some() && known.stamp == stamp) {
            return known.header.clone();
        }
        self.reads += 1;
        let user = SkinUserConfig::default();
        let header = load_header(path, SkinLoadOptions::new(root, &user, crate::MODE)).ok();
        self.entries.insert(path.to_path_buf(), CachedHeader { stamp, header: header.clone() });
        header
    }

    /// The header of every document under `root` that declares a screen, up to `max_depth` deep.
    ///
    /// A file that is not a skin, or is a skin this build cannot read, is passed over silently: the
    /// folder is the player's own and may hold anything.
    fn scan(&mut self, root: &Path, max_depth: usize) -> Vec<SkinHeader> {
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
                    if depth + 1 < max_depth {
                        directories.push((path, depth + 1));
                    }
                    continue;
                }
                if !is_document(&path) {
                    continue;
                }
                if let Some(header) = self.header(&path, root) {
                    found.push(header);
                }
            }
        }
        found
    }
}

impl SkinLibrary {
    /// A library rooted where the configuration says, with the skin folder not walked yet. The pack
    /// the configuration names is read at once, because the screens it draws do not wait for the
    /// settings screen to be opened.
    pub(crate) fn new(settings_path: &Path, config: &Config) -> SkinLibrary {
        let mut library = SkinLibrary {
            settings_path: settings_path.to_path_buf(),
            root: skin_root(settings_path, config),
            forced_pack: None,
            pack: None,
            documents: Vec::new(),
            pack_documents: BTreeMap::new(),
            pack_paths: BTreeSet::new(),
            headers: HeaderCache::default(),
            loaded: BTreeMap::new(),
            errors: BTreeMap::new(),
            stale: BTreeSet::new(),
            waiting: BTreeMap::new(),
            arrived: RefCell::new(BTreeMap::new()),
            unannounced: BTreeMap::new(),
            seed: None,
            next_build: 0,
        };
        library.read_pack(config);
        library
    }

    /// The same library with `pack` drawn for this run whatever the configuration names, which is
    /// what [`SKIN_PACK_ENV`] asks for. `None` leaves the configuration in charge.
    pub(crate) fn forcing_pack(mut self, pack: Option<PathBuf>, config: &Config) -> SkinLibrary {
        self.forced_pack = pack;
        self.read_pack(config);
        self
    }

    /// Pin every read this library makes, for a test or a capture that has to come out the same each
    /// time. `None` leaves a random option and a random file to chance, which is what a player gets.
    #[cfg(test)]
    pub(crate) fn pin_seed(&mut self, seed: Option<u64>) {
        self.seed = seed;
    }

    /// What the screens' own reads are pinned with.
    pub(crate) fn seed(&self) -> Option<u64> {
        self.seed
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

    /// The path of the document one screen is to be drawn with: the one the player chose for it by
    /// hand, and otherwise the pack's. `None` leaves the screen to its built-in layout.
    pub(crate) fn document_path<'a>(&'a self, config: &'a Config, screen: i32) -> Option<&'a str> {
        config.skin.document(screen).or_else(|| self.pack_documents.get(&screen).map(String::as_str))
    }

    /// Whether the pack that should be in effect is not the one the folders were last walked for:
    /// the PACK FOLDER row moved, or a settings file arrived from somewhere else.
    pub(crate) fn pack_moved(&self, config: &Config) -> bool {
        wanted_pack(self.forced_pack.as_deref(), config) != self.pack.as_deref()
    }

    /// What the PACK FOLDER row shows: the pack in effect, and where it came from when it is not the
    /// one the row stores.
    pub(crate) fn pack_value(&self, config: &Config) -> String {
        match (self.forced_pack.as_deref(), config.skin.pack_folder()) {
            (Some(forced), _) => format!("{} ({SKIN_PACK_ENV})", forced.display()),
            (None, Some(pack)) => pack.to_owned(),
            (None, None) => NONE_VALUE.to_string(),
        }
    }

    /// Walk the skin folder and the pack again and read the header of every document in them.
    ///
    /// Called when the settings screen opens, so a skin dropped into the folder while the game is
    /// running is found without a restart. A header whose file has not changed is not read again.
    pub(crate) fn rescan(&mut self, settings_path: &Path, config: &Config) {
        self.settings_path = settings_path.to_path_buf();
        self.root = skin_root(settings_path, config);
        let mut documents = self.headers.scan(&self.root, SCAN_MAX_DEPTH);
        for header in self.read_pack(config) {
            if !documents.iter().any(|known| known.path == header.path) {
                documents.push(header);
            }
        }
        documents.sort_by_cached_key(|header| (header.skin_type, header.name.to_ascii_uppercase(), header.path.clone()));
        self.documents = documents;
    }

    /// [`SkinLibrary::rescan`] with every header read from its file again, which is what the RELOAD
    /// row asks for: a file added beside a document changes what its header offers without touching
    /// the document itself.
    pub(crate) fn rescan_afresh(&mut self, settings_path: &Path, config: &Config) {
        self.headers.entries.clear();
        self.rescan(settings_path, config);
    }

    /// Read the headers of the pack in effect and settle which of its documents draws each screen:
    /// the one that declares the screen, and the first by file name when several do. Answers the
    /// headers it read.
    fn read_pack(&mut self, config: &Config) -> Vec<SkinHeader> {
        self.pack = wanted_pack(self.forced_pack.as_deref(), config).map(Path::to_path_buf);
        let mut found = match self.pack.clone() {
            Some(pack) => self.headers.scan(&pack, PACK_SCAN_DEPTH),
            None => Vec::new(),
        };
        found.sort_by_cached_key(|header| (file_name_key(&header.path), header.path.clone()));
        self.pack_paths = found.iter().map(|header| header.path.clone()).collect();
        self.pack_documents.clear();
        for header in &found {
            self.pack_documents.entry(header.skin_type).or_insert_with(|| header.path.to_string_lossy().into_owned());
        }
        self.drop_moved(config);
        found
    }

    /// Forget what was read for every screen that is no longer drawn with the document that read
    /// was of: the pack was changed or taken away, or a settings file arrived that chooses
    /// differently. This reaches the screens nobody is looking at, so a document the player has
    /// left behind does not keep its interpreter and its failure until its screen is next opened.
    pub(crate) fn drop_moved(&mut self, config: &Config) {
        let read = self.loaded.iter().map(|(screen, entry)| (*screen, entry.path.as_str()));
        let waiting = self.waiting.iter().map(|(screen, path)| (*screen, path.as_str()));
        let failed = self.errors.iter().map(|(screen, failure)| (*screen, failure.path.as_str()));
        let moved: BTreeSet<i32> =
            read.chain(waiting).chain(failed).filter(|(screen, path)| self.document_path(config, *screen) != Some(*path)).map(|(screen, _)| screen).collect();
        for screen in moved {
            self.forget_read(screen);
        }
    }

    /// The documents that declare the screen the tab is configuring, in the order the row cycles.
    fn candidates(&self, config: &Config) -> Vec<&SkinHeader> {
        self.documents.iter().filter(|header| header.skin_type == config.skin.screen).collect()
    }

    /// The header of the document one screen is to be drawn with.
    pub(crate) fn header_of(&self, config: &Config, screen: i32) -> Option<&SkinHeader> {
        let path = self.document_path(config, screen)?;
        self.documents.iter().find(|header| header.path == Path::new(path))
    }

    /// The header of the document the screen being configured is to be drawn with.
    fn chosen(&self, config: &Config) -> Option<&SkinHeader> {
        self.header_of(config, config.skin.screen)
    }

    /// What the SKIN row shows: the document's own name, marked when the pack rather than the player
    /// chose it, or the built-in screen.
    pub(crate) fn document_value(&self, config: &Config) -> String {
        let by_hand = config.skin.document(config.skin.screen);
        let Some(path) = self.document_path(config, config.skin.screen) else {
            return DEFAULT_VALUE.to_string();
        };
        let name = match self.chosen(config) {
            Some(header) if !header.name.trim().is_empty() => header.name.clone(),
            _ => skin_document_label(path),
        };
        if by_hand.is_some() { name } else { format!("{PACK_DOCUMENT_PREFIX}{name}") }
    }

    /// What the LOADED row shows: the size, parser and warning count of the document being drawn,
    /// or why the built-in screen is being drawn instead.
    pub(crate) fn info(&self, config: &Config) -> String {
        let screen = config.skin.screen;
        let Some(path) = self.document_path(config, screen) else {
            return BUILT_IN_INFO.to_string();
        };
        if let Some(failure) = self.errors.get(&screen).filter(|failure| failure.path == path) {
            return failure.reason.clone();
        }
        if !is_supported_skin_type(screen) {
            return UNSUPPORTED_INFO.to_string();
        }
        if self.waiting.get(&screen).is_some_and(|waiting| waiting == path) {
            return WAITING_INFO.to_string();
        }
        let Some(skin) = self.loaded.get(&screen).filter(|entry| entry.path == path && !self.stale.contains(&screen)).map(|entry| &entry.skin) else {
            return NOT_READ_INFO.to_string();
        };
        let parser = match skin.parser {
            ParserKind::Json => "JSON",
            ParserKind::Json5 => "JSON5",
            ParserKind::Lua => "Lua",
        };
        let lua = if uses_lua(skin) { format!(" - {LUA_INFO}") } else { String::new() };
        let author = if skin.def.author.trim().is_empty() { String::new() } else { format!(" - {}", skin.def.author) };
        format!("{}X{} - {}{} - {} WARNINGS{}", skin.def.w, skin.def.h, parser, lua, skin.warnings.len(), author)
    }

    /// The customisation rows the SKIN tab shows, top to bottom, in the order the chosen document
    /// declares them: its properties, its file slots, then one row per axis it lets the player nudge.
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
            return missing_row();
        };
        let stored = self.document_path(config, config.skin.screen).and_then(|path| config.skin.customisation(path));
        match row {
            SkinRow::Property(at) => match header.properties.get(at) {
                Some(property) => (
                    labelled(&property.category, &property.name),
                    property_value(property, stored.and_then(|entry| entry.properties.get(&property.name).copied())),
                ),
                None => missing_row(),
            },
            SkinRow::File(at) => match header.custom_files.get(at) {
                Some(file) => {
                    (labelled(&file.category, &file.name), file_value(file, stored.and_then(|entry| entry.filepaths.get(&file.name)).map(String::as_str)))
                }
                None => missing_row(),
            },
            SkinRow::Offset(at, axis) => match header.offsets.get(at) {
                Some(def) => {
                    let nudge = stored.and_then(|entry| entry.offsets.get(&def.id).copied()).unwrap_or_default();
                    (format!("{} {}", labelled(&def.category, &def.name), axis.label()), format!("{:+}", axis.get(nudge) as i32))
                }
                None => missing_row(),
            },
        }
    }

    /// Step one customisation row, and report whether anything moved.
    ///
    /// A property or a file changes what is drawn and which files are read, so the document has to
    /// be read again; an offset is read live out of the stored choices, so it does not.
    pub(crate) fn step(&mut self, config: &mut Config, row: SkinRow, delta: i32) -> bool {
        let Some(path) = self.document_path(config, config.skin.screen).map(str::to_owned) else {
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
        let Some(path) = self.document_path(config, config.skin.screen).map(str::to_owned) else {
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

    /// Step the SKIN row through leaving the screen alone -- which draws the pack's document for it,
    /// or the built-in screen -- and every document that declares this screen.
    pub(crate) fn cycle_document(&mut self, config: &mut Config, delta: i32) -> bool {
        let candidates: Vec<String> = self.candidates(config).iter().map(|header| header.path.to_string_lossy().into_owned()).collect();
        let current = config.skin.document(config.skin.screen).map(str::to_owned);
        let at = current.as_deref().and_then(|path| candidates.iter().position(|entry| Path::new(entry) == Path::new(path)));
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
        let Some(path) = self.document_path(config, config.skin.screen).map(str::to_owned) else {
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
    /// parse rather than one per entry to the screen. That holds for the document that failed and
    /// no other: a screen that is since drawn with a different one reads it.
    pub(crate) fn needs_reload(&self, config: &Config) -> bool {
        self.needs_reload_for(config, config.skin.screen)
    }

    /// The same question asked about a screen other than the one the tab is configuring, which is
    /// how a screen the player walks into gets its document read without the tab being opened.
    ///
    /// A Lua skin that is already waiting for its screen has been asked for and is not asked for
    /// again.
    pub(crate) fn needs_reload_for(&self, config: &Config, screen: i32) -> bool {
        let Some(path) = self.document_path(config, screen) else {
            return self.loaded.contains_key(&screen) || self.waiting.contains_key(&screen) || self.errors.contains_key(&screen);
        };
        if self.stale.contains(&screen) {
            return true;
        }
        if let Some(waiting) = self.waiting.get(&screen) {
            return waiting != path;
        }
        if let Some(failure) = self.errors.get(&screen) {
            return failure.path != path;
        }
        self.loaded.get(&screen).is_none_or(|entry| entry.path != path)
    }

    /// Ask for the chosen document to be read with the choices made for it, so what is on screen is
    /// what the rows say. A document that cannot be read leaves the built-in screen drawing and the
    /// reason on the LOADED row.
    pub(crate) fn reload(&mut self, config: &Config) {
        self.request_for(config, config.skin.screen);
    }

    /// Ask for one screen's document, whichever screen the tab happens to be configuring.
    ///
    /// A document that is only data is read here and now, with no game behind it. A Lua skin is left
    /// waiting: it is read by [`SkinLibrary::read_waiting`] on the first frame of the screen it
    /// draws, against that screen's state.
    pub(crate) fn request_for(&mut self, config: &Config, screen: i32) {
        self.forget_read(screen);
        let Some(path) = self.readable_path(config, screen) else {
            return;
        };
        if is_lua_skin(Path::new(&path)) {
            self.waiting.insert(screen, path);
            return;
        }
        let outcome = self.read(config, &path, screen, SkinRead::unhosted(self.seed));
        self.settle(screen, path, outcome);
    }

    /// Read one screen's document here and now, whatever kind it is, against the host and the seed
    /// the caller brings: what a test does in place of drawing the screen's first frame.
    #[cfg(test)]
    pub(crate) fn reload_for(&mut self, config: &Config, screen: i32, read: SkinRead<'_>) {
        self.forget_read(screen);
        let Some(path) = self.readable_path(config, screen) else {
            return;
        };
        let outcome = self.read(config, &path, screen, read);
        self.settle(screen, path, outcome);
    }

    /// Whether one screen's Lua skin is waiting for the screen's first frame, or has been read on it
    /// and not yet taken in.
    pub(crate) fn is_waiting(&self, screen: i32) -> bool {
        self.waiting.contains_key(&screen)
    }

    /// Read the Lua skin waiting for `screen`, against the state of the screen that is drawing its
    /// first frame. Does nothing when no skin is waiting or it has been read already, so a screen
    /// calls this every frame and pays for one read.
    ///
    /// The whole skin runs here, on the frame loop, which is where the reference runs it too: a
    /// screen change waits for its skin. What comes out is taken in by [`SkinLibrary::adopt`].
    pub(crate) fn read_waiting(&self, config: &Config, screen: i32, read: SkinRead<'_>) {
        let Some(path) = self.waiting.get(&screen) else {
            return;
        };
        if self.arrived.borrow().contains_key(&screen) {
            return;
        }
        let outcome = self.read(config, path, screen, read);
        self.arrived.borrow_mut().insert(screen, Arrival { path: path.clone(), outcome });
    }

    /// Take in what [`SkinLibrary::read_waiting`] read for one screen, if it has.
    ///
    /// A read of a path the screen is no longer waiting for -- the selection moved while the frame
    /// was being drawn -- is dropped.
    pub(crate) fn adopt(&mut self, screen: i32) {
        let Some(arrival) = self.arrived.get_mut().remove(&screen) else {
            return;
        };
        if self.waiting.get(&screen) != Some(&arrival.path) {
            return;
        }
        self.waiting.remove(&screen);
        self.settle(screen, arrival.path, arrival.outcome);
    }

    /// The first line of why one screen's document could not be read, the first time it is asked
    /// for after the read failed, and `None` every time after: the reason is shown once, not once a
    /// frame.
    pub(crate) fn take_failure(&mut self, screen: i32) -> Option<String> {
        self.unannounced.remove(&screen)
    }

    /// Why one screen's document could not be read.
    #[cfg(test)]
    pub(crate) fn failure(&self, screen: i32) -> Option<&str> {
        self.errors.get(&screen).map(|failure| failure.reason.as_str())
    }

    /// How many headers have been read from their files rather than answered from memory.
    #[cfg(test)]
    pub(crate) fn header_reads(&self) -> u64 {
        self.headers.reads
    }

    /// Mark every Lua skin that has been read as needing to be read again.
    ///
    /// A Lua skin builds its screen from the state the game was in when it ran, so one read for an
    /// earlier visit to a screen describes that visit. Called when a scene begins, which makes the
    /// next frame of each such screen ask for its skin afresh. A skin that failed to read is left
    /// alone: it is not run again on every entry to its screen.
    pub(crate) fn expire_scripted(&mut self) {
        let scripted = self.loaded.iter().filter(|(_, entry)| entry.skin.parser == ParserKind::Lua).map(|(screen, _)| *screen);
        self.stale.extend(scripted);
    }

    /// Forget everything one screen's last read left behind.
    fn forget_read(&mut self, screen: i32) {
        self.stale.remove(&screen);
        self.errors.remove(&screen);
        self.loaded.remove(&screen);
        self.waiting.remove(&screen);
        self.unannounced.remove(&screen);
        self.arrived.get_mut().remove(&screen);
    }

    /// The path of the document `screen` is to be drawn with, when this build has a screen to draw
    /// it on.
    fn readable_path(&self, config: &Config, screen: i32) -> Option<String> {
        let path = self.document_path(config, screen)?;
        is_supported_skin_type(screen).then(|| path.to_owned())
    }

    /// Read the document at `path` whole, for `screen`.
    ///
    /// A document found in the pack may read the pack's folder and nothing above it; any other may
    /// read the skin folder. Whatever the skin writes goes to the overlay kept for its pack beside
    /// the settings file, never into the skin's own folder.
    fn read(&self, config: &Config, path: &str, screen: i32, read: SkinRead<'_>) -> Result<Box<LoadedSkin>, String> {
        let document = Path::new(path);
        let root = match self.pack.as_deref() {
            Some(pack) if self.pack_paths.contains(document) => pack,
            _ => self.root.as_path(),
        };
        let user = config.skin.user_config(path);
        let overlay = skin_overlay_folder(&self.settings_path, document);
        let mode = skin_type_mode(screen).unwrap_or(crate::MODE);
        let options = SkinLoadOptions { rng_seed: read.seed, write_overlay: Some(&overlay), ..SkinLoadOptions::new(root, &user, mode) };
        load_skin_with_host(document, options, read.host).map(Box::new).map_err(|error| error.to_string())
    }

    /// Record what a read of one screen's document came to.
    fn settle(&mut self, screen: i32, path: String, outcome: Result<Box<LoadedSkin>, String>) {
        match outcome {
            Ok(skin) => {
                let build = self.next_build;
                self.next_build += 1;
                self.loaded.insert(screen, LoadedDocument { skin, path, build });
            }
            Err(reason) => {
                self.unannounced.insert(screen, reason.lines().next().unwrap_or_default().to_owned());
                self.errors.insert(screen, ReadFailure { path, reason });
            }
        }
    }
}

/// The pack that should be in effect: the one the run was started with, and otherwise the one the
/// configuration names.
fn wanted_pack<'a>(forced: Option<&'a Path>, config: &'a Config) -> Option<&'a Path> {
    forced.or_else(|| config.skin.pack_folder().map(Path::new))
}

/// The pack an environment names, or `None` for a variable that is unset or blank.
pub(crate) fn pack_from_environment(value: Option<OsString>) -> Option<PathBuf> {
    value.filter(|value| !value.to_string_lossy().trim().is_empty()).map(PathBuf::from)
}

/// What a pack's documents are ordered by when two of them declare the same screen: the file name,
/// ignoring case.
fn file_name_key(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().to_ascii_uppercase()).unwrap_or_default()
}

/// Whether the skin leaves any decision to Lua -- a function a Lua skin handed over, or a script a
/// document wrote as a string -- which is worth saying on the LOADED row because it is the one part
/// of a skin that runs rather than being read.
fn uses_lua(skin: &LoadedSkin) -> bool {
    skin.runtime().is_some_and(|runtime| runtime.function_count() > 0)
}

/// The directory documents are looked for in: the one the row names, or [`DEFAULT_SKIN_FOLDER`]
/// beside the settings file.
pub(crate) fn skin_root(settings_path: &Path, config: &Config) -> PathBuf {
    match config.skin.folder.as_deref() {
        Some(folder) => PathBuf::from(folder),
        None => settings_path.parent().unwrap_or(Path::new(".")).join(DEFAULT_SKIN_FOLDER),
    }
}

/// Whether a file name is one a skin document is written under.
fn is_document(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| DOCUMENT_EXTENSIONS.iter().any(|known| extension.eq_ignore_ascii_case(known)))
}

/// What the SKIN tab shows for a row whose declaration is no longer there, which is what a document
/// edited between two runs leaves behind.
fn missing_row() -> (String, String) {
    (NONE_VALUE.to_string(), NONE_VALUE.to_string())
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
