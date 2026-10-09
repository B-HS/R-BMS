//! The Lua runtime a skin is loaded into and drawn from.
//!
//! A `.luaskin` is a program. Loading one runs its entry file twice in a single interpreter -- once
//! with no configuration to learn what the skin offers, once with the player's choices to build the
//! screen -- and the table the second run returns is full of function values that are then called
//! on every frame for as long as the screen is shown. [`SkinLua`] is that interpreter: it owns the
//! state, the environment a skin expects to find in it, the registry of the functions the skin
//! handed over, and the record of everything that went wrong inside it.
//!
//! How the pieces meet:
//!
//! - The loader builds a [`SkinLua`] from a [`SkinLuaConfig`], calls [`SkinLua::run_entry`] for the
//!   header pass, publishes the configuration with [`SkinLua::set_skin_config`] and calls
//!   [`SkinLua::run_entry`] again for the body pass. Both passes share every global and every loaded
//!   module.
//! - A JSON document has no entry file to run, but any field of it may hold a script. The loader
//!   builds the same interpreter for it, calls [`SkinLua::publish_globals`] so the script finds the
//!   game state where the reference puts it for a JSON skin, and compiles each script with
//!   [`SkinLua::compile`].
//! - The table converter walks each returned value and hands every function it finds to
//!   [`SkinLua::register`], keeping the [`LuaFnId`] in the model.
//! - The loaded skin keeps the [`SkinLua`]. Each frame the renderer opens one
//!   [`SkinLua::frame`], which binds the host for that frame, and makes every call of the frame
//!   through the [`BoundFrame`] it is given.
//!
//! Everything a skin can reach is built here out of parts this crate controls: `env` the base
//! library, `pattern` the four pattern functions of `string`, `package` `require`, `io` and `os`
//! their namesakes, `luajava` the Java facade, [`main_state`] the game state. The three ways this differs from the reference on purpose are all
//! about containment: file access cannot leave the skin root, writes land in an overlay directory
//! instead of the skin folder, and nothing reaches the network.
//!
//! The whole module is compiled only with the `lua` feature, which is on by default. A build without
//! it rejects a skin that needs Lua with [`SkinError::LuaUnavailable`] rather than silently reading
//! its expressions as false.

pub mod budget;
pub mod coerce;
mod env;
mod io;
mod luajava;
pub mod main_state;
mod os;
mod package;
mod pattern;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use mlua::chunk::ChunkMode;
use mlua::{Function, IntoLuaMulti, Lua, Value};

use crate::SkinError;
use crate::dst::LuaDrawEval;
pub use crate::dst::LuaFnId;
use crate::property::{NameSpace, SkinHost};
use crate::resolve;
use crate::timer::TIMER_OFF;
pub use budget::{BUDGET_EXHAUSTED, FrameBudget, LoadBudget, LuaBudget, Meter, Phase};
pub use env::{SKIN_CONFIG_GLOBAL, SkinConfigGlobal};
pub use os::{LocalTime, local_time};

/// Distinct printed lines the log keeps. Later ones are counted but not stored.
const MAX_LOGGED_PRINTS: usize = 1_024;

/// Distinct swallowed error messages the log keeps. Later ones are counted but not stored.
const MAX_LOGGED_SWALLOWED: usize = 256;

/// Bytes of one printed line or one error message the log keeps. The rest is cut, so a skin that
/// prints a whole file cannot make the log as large as the interpreter.
const MAX_LOGGED_TEXT_BYTES: usize = 4_096;

/// The name a compiled script field carries in the interpreter's own messages.
pub const SCRIPT_CHUNK_NAME: &str = "=[skin script]";

/// The name the chunk that publishes the modules as globals carries in the interpreter's messages.
const GLOBAL_EXPORT_CHUNK_NAME: &str = "=[skin globals]";

/// Copies every member of the three injected modules into the globals, in the order the reference
/// exports them (`JSONSkinLoader`'s constructor: `exportMainStateAccessor`, then `exportUtilities`
/// with the timer helpers before the event helpers), so a later module wins a shared name.
const GLOBAL_EXPORT: &str = r#"
local modules = ...
for index = 1, #modules do
    for name, member in pairs(package.loaded[modules[index]]) do
        _G[name] = member
    end
end
"#;

/// The prefix that turns a script field's expression into a chunk (`SkinLuaAccessor.load*Property`).
const SCRIPT_RETURN_PREFIX: &str = "return ";

/// The separator every path handed to a skin, and every chunk name, is written with.
const LUA_PATH_SEPARATOR: &str = "/";

/// The separator a skin written on Windows may have used instead.
const FOREIGN_PATH_SEPARATOR: char = '\\';

/// The marker that makes the interpreter read a chunk name as a file name.
const FILE_CHUNK_MARKER: char = '@';

/// Bytes the overlay may hold in all unless the configuration says otherwise.
const DEFAULT_OVERLAY_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Files and directories the overlay may hold in all unless the configuration says otherwise.
const DEFAULT_OVERLAY_MAX_ENTRIES: u64 = 4_096;

/// How long the tracked size of the overlay is trusted before the folder is measured again.
const DEFAULT_OVERLAY_RESYNC_INTERVAL: Duration = Duration::from_secs(5);

/// What a write is refused with when the skin has no overlay.
const NO_OVERLAY: &str = "the skin has no write overlay";

/// Where the call stack begins in the message of an error that reached Rust.
const TRACEBACK_MARKER: &str = "\nstack traceback:";

/// Which of the two ways a skin is being run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LuaMode {
    /// The skin is being loaded to be drawn: both passes run and `main_state` is live.
    #[default]
    Full,
    /// Only the header is wanted, for a skin list or a settings screen. The entry file runs once,
    /// `main_state`, `timer_util` and `event_util` are empty tables, and no host is consulted.
    HeaderOnly,
}

/// Which run of the entry file an error or a budget belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuaPass {
    /// The first run, with `skin_config` unset.
    Header,
    /// The second run, with the player's configuration published.
    Body,
}

impl LuaPass {
    /// The name this pass goes by in an error message.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Header => "header pass",
            Self::Body => "body pass",
        }
    }
}

/// How one skin's interpreter is to be set up.
#[derive(Debug, Clone)]
pub struct SkinLuaConfig {
    /// The skin root: the only directory the skin may read from.
    pub root: PathBuf,
    /// Where the skin's writes go. `None` leaves the skin read-only: every attempt to write fails
    /// the way a full disk would, and nothing is created anywhere.
    pub overlay: Option<PathBuf>,
    /// The seed `math.random` starts from. `None` seeds from the machine, which is what play wants;
    /// tests and captures pass a fixed value so a skin's random choices repeat.
    pub seed: Option<u64>,
    /// What the interpreter may spend.
    pub budget: LuaBudget,
    pub mode: LuaMode,
    /// How much the overlay may hold in all.
    pub overlay_limits: OverlayLimits,
}

/// How much a skin may leave in its write overlay.
///
/// The size is the overlay folder's current total, not the bytes written so far: a skin that
/// rewrites a small state file every frame never reaches the limit, and one that shrinks or
/// replaces what it wrote has room again at once. Every write, append and directory creation is
/// checked before it happens and, when it would pass a limit, is answered the way a full disk is.
///
/// The limits are the folder's rather than one interpreter's: the interpreters of a thread that
/// write to the same overlay -- a pack's screens, each with a skin of its own -- count against one
/// total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayLimits {
    /// Bytes the files of the overlay may hold in all.
    pub max_bytes: u64,
    /// Files and directories the overlay may hold in all.
    pub max_entries: u64,
    /// How long the tracked total is trusted before the folder is measured again, which is how
    /// files another process deleted give their room back. Zero measures before every check.
    pub resync_interval: Duration,
}

impl Default for OverlayLimits {
    fn default() -> Self {
        Self { max_bytes: DEFAULT_OVERLAY_MAX_BYTES, max_entries: DEFAULT_OVERLAY_MAX_ENTRIES, resync_interval: DEFAULT_OVERLAY_RESYNC_INTERVAL }
    }
}

impl SkinLuaConfig {
    /// A read-only, machine-seeded, full-mode configuration for the skin at `root`.
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            overlay: None,
            seed: None,
            budget: LuaBudget::default(),
            mode: LuaMode::Full,
            overlay_limits: OverlayLimits::default(),
        }
    }
}

/// The skin's view of the filesystem: its root and its write overlay, seen as one tree.
///
/// A skin names files by an absolute path under the root (which is what `skin_config.get_path`
/// hands it) or by a path relative to the root. Either way the name is first reduced to a *logical*
/// path, relative to the root, and refused with [`SkinError::PathEscape`] if `..` segments or a
/// symlink would carry it outside. What happens next depends on what the file is wanted for, and
/// every library in this module goes through one of the methods below rather than touching a path
/// itself.
///
/// The root is made absolute and free of `.` and `..` once, on construction, and that one spelling
/// is the prefix of every absolute path handed back to a skin. It is deliberately not the canonical
/// spelling: on Windows that is a verbatim path, in which the `/` a skin joins its own paths with
/// would no longer separate anything. The canonical form is kept beside it and used only to decide
/// containment. A path that does not exist yet is checked against its deepest existing ancestor, as
/// [`crate::resolve::contained`] does, and nothing here creates a file or a directory: the overlay
/// appears when the first write creates it.
#[derive(Debug, Clone)]
pub struct SkinPaths {
    root: PathBuf,
    root_real: PathBuf,
    overlay: Option<PathBuf>,
}

impl SkinPaths {
    /// The paths of the skin at `root`, writing to `overlay` when there is one. A root that does
    /// not exist is [`SkinError::Read`].
    pub fn new(root: &Path, overlay: Option<&Path>) -> Result<Self, SkinError> {
        let absolute = std::path::absolute(root).map_err(SkinError::Read)?;
        let root = resolve::contained(&absolute, &absolute)?;
        let root_real = root.canonicalize().map_err(SkinError::Read)?;
        Ok(Self { root, root_real, overlay: overlay.map(Path::to_path_buf) })
    }

    /// The skin root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The write overlay, when the skin has one.
    pub fn overlay(&self) -> Option<&Path> {
        self.overlay.as_deref()
    }

    /// The root-relative form of a path a skin named, or [`SkinError::PathEscape`].
    ///
    /// An absolute path is taken as it is and a relative one is joined onto the root; the working
    /// directory plays no part. A backslash is read as a separator on every platform, because skins
    /// are mostly written on Windows.
    pub fn logical(&self, path: &str) -> Result<PathBuf, SkinError> {
        let escape = || SkinError::PathEscape(path.to_owned());
        let named = PathBuf::from(path.replace(FOREIGN_PATH_SEPARATOR, LUA_PATH_SEPARATOR));
        let candidate = if named.is_absolute() { named } else { self.root.join(named) };
        let normalized = match resolve::contained(&self.root, &candidate) {
            Ok(normalized) => normalized,
            Err(SkinError::PathEscape(_)) => return Err(escape()),
            Err(other) => return Err(other),
        };
        if let Ok(relative) = normalized.strip_prefix(&self.root) {
            return Ok(relative.to_path_buf());
        }
        let real = real_path(&normalized).ok_or_else(escape)?;
        real.strip_prefix(&self.root_real).map(Path::to_path_buf).map_err(|_| escape())
    }

    /// A logical path in the form a skin and an error message read it: `/` separated, whatever the
    /// platform.
    pub fn display(logical: &Path) -> String {
        logical.components().map(|component| component.as_os_str().to_string_lossy()).collect::<Vec<_>>().join(LUA_PATH_SEPARATOR)
    }

    /// The name a file's chunk carries, so an error raised in it reads `<path from the root>:<line>:`.
    pub fn chunk_name(logical: &Path) -> String {
        format!("{FILE_CHUNK_MARKER}{}", Self::display(logical))
    }

    /// The file to run as code: always the one in the skin root, never the overlay, so nothing a
    /// skin wrote at run time is ever executed. Used by `require`, `dofile` and `loadfile`.
    pub fn script(&self, path: &str) -> Result<PathBuf, SkinError> {
        self.logical(path).map(|logical| self.root.join(logical))
    }

    /// The file to read as data: the overlay's copy when it exists, the root's otherwise. The file
    /// need not exist; the caller's open reports that. Used by `io.open` for reading, `io.lines`,
    /// the `main_state.file_*` readers and the audio paths.
    pub fn readable(&self, path: &str) -> Result<PathBuf, SkinError> {
        let logical = self.logical(path)?;
        let copy = self.overlay.as_ref().map(|overlay| overlay.join(&logical)).filter(|copy| copy.exists());
        Ok(copy.unwrap_or_else(|| self.root.join(logical)))
    }

    /// The file to write: always the overlay's copy. Fails when the skin has no overlay. Used by
    /// `io.open` for writing and appending, `io.tmpfile`, `File:mkdir` and the `main_state.file_*`
    /// writers.
    ///
    /// Only the path is worked out. The caller creates whatever directories its own rule calls for
    /// -- every missing parent for `io.open`, exactly one level for `mkdir` -- and can ask
    /// [`Self::readable`] whether a parent exists in the tree the skin sees.
    pub fn writable(&self, path: &str) -> Result<PathBuf, SkinError> {
        let logical = self.logical(path)?;
        match &self.overlay {
            Some(overlay) => Ok(overlay.join(logical)),
            None => Err(SkinError::Read(std::io::Error::new(std::io::ErrorKind::ReadOnlyFilesystem, NO_OVERLAY))),
        }
    }

    /// The entries of a directory as the skin sees it: the overlay's and the root's merged, each
    /// once, sorted, as absolute paths under the root with `/` separators. Used by `File:listFiles`
    /// and `main_state.file_list`. A directory that is in neither tree is [`SkinError::Read`].
    pub fn entries(&self, directory: &str) -> Result<Vec<String>, SkinError> {
        let logical = self.logical(directory)?;
        let mut names = BTreeSet::new();
        let mut failure = None;
        let mut listed = false;
        for base in self.overlay.iter().chain(std::iter::once(&self.root)) {
            match std::fs::read_dir(base.join(&logical)) {
                Ok(entries) => {
                    listed = true;
                    names.extend(entries.flatten().filter_map(|entry| entry.file_name().into_string().ok()));
                }
                Err(error) => failure = Some(error),
            }
        }
        if let (false, Some(error)) = (listed, failure) {
            return Err(SkinError::Read(error));
        }
        let directory = resolve::pattern_for(&self.root, &Self::display(&logical));
        Ok(names.into_iter().map(|name| format!("{directory}{LUA_PATH_SEPARATOR}{name}")).collect())
    }
}

/// The canonical form of a path whose tail may not exist yet: its deepest existing ancestor made
/// canonical, with the missing part put back.
fn real_path(path: &Path) -> Option<PathBuf> {
    let mut missing = Vec::new();
    let mut current = path;
    loop {
        if let Ok(real) = current.canonicalize() {
            return Some(missing.iter().rev().fold(real, |joined, part| joined.join(part)));
        }
        missing.push(current.file_name()?);
        current = current.parent()?;
    }
}

/// What an argument of the wrong kind is refused with, worded as the interpreter words its own.
fn bad_argument_message(position: usize, function: &str, reason: &str) -> String {
    format!("bad argument #{position} to '{function}' ({reason})")
}

/// The error an argument of the wrong kind raises.
fn bad_argument(position: usize, function: &str, reason: &str) -> mlua::Error {
    mlua::Error::runtime(bad_argument_message(position, function, reason))
}

/// An interpreter error as the interpreter worded it, with the call stack the binding appends to an
/// error that reaches Rust. This is what a failed load is reported with: the stack is the path
/// through the skin's own files that led to the failure.
fn error_text(error: &mlua::Error) -> String {
    match error {
        mlua::Error::RuntimeError(message) | mlua::Error::MemoryError(message) | mlua::Error::SyntaxError { message, .. } => message.clone(),
        mlua::Error::CallbackError { cause, .. } | mlua::Error::WithContext { cause, .. } => error_text(cause),
        other => other.to_string(),
    }
}

/// The message of an interpreter error and nothing else: no prefix naming the kind of error and no
/// call stack.
///
/// The binding adds both around what the interpreter raised. Neither belongs in a log that keeps
/// one entry per distinct message, and a message stripped of them starts with the `file:line:` that
/// [`error_location`] reads.
pub fn error_message(error: &mlua::Error) -> String {
    let text = error_text(error);
    match text.find(TRACEBACK_MARKER) {
        Some(end) => text[..end].to_owned(),
        None => text,
    }
}

/// Where a message says it was raised: the chunk and the line of the `chunk:line:` an interpreter
/// error starts with. `None` for a message without one, such as an error raised with a level of
/// zero or with a value that is not a string.
pub fn error_location(message: &str) -> Option<(&str, u32)> {
    let mut searched = 0;
    while let Some(colon) = message[searched..].find(':') {
        let digits_start = searched + colon + 1;
        let digits = message[digits_start..].bytes().take_while(u8::is_ascii_digit).count();
        let digits_end = digits_start + digits;
        if digits > 0
            && message[digits_end..].starts_with(':')
            && let Ok(line) = message[digits_start..digits_end].parse()
        {
            return Some((&message[..searched + colon], line));
        }
        searched = digits_start;
    }
    None
}

/// What a function value is called as, which decides its arguments, how its result is read and what
/// it yields when it fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LuaFnKind {
    /// A `draw` or `op` condition: no arguments, read with `toboolean`, `false` on failure.
    Boolean,
    /// An integer `value`: no arguments, read with `toint`, zero on failure.
    Integer,
    /// A float `value`: no arguments, read with `tofloat`, zero on failure.
    Float,
    /// A text `value`: no arguments, read with `tojstring`, empty on failure.
    Text,
    /// A `timer`: no arguments, read with `tolong` as the microsecond the timer switched on, off on
    /// failure. A function that returns nothing therefore reads as "on since zero".
    Timer,
    /// An `act` or a custom event's action: called with one integer argument and its result ignored.
    /// The reference always passes exactly one, whatever the function declares.
    Event,
    /// A slider's `event`: called with the new share as one number.
    FloatWriter,
    /// A text's `event`: called with the new text as one string.
    TextWriter,
}

/// One function that failed while being called, as the log keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionFailure {
    pub function: LuaFnId,
    pub kind: LuaFnKind,
    /// The message of the first failure. Later ones are counted, not kept.
    pub first_message: String,
    /// How many calls have failed, the first included.
    pub count: u64,
}

/// One error a skin's own `pcall` or `xpcall` caught, as the log keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwallowedError {
    /// The whole message, position prefix included.
    pub message: String,
    /// The file the message says it was raised in, as a path from the skin root. `None` when the
    /// message carries no position.
    pub file: Option<String>,
    /// The line the message says it was raised on.
    pub line: Option<u32>,
    /// How many times an error with this message was caught.
    pub count: u64,
}

/// One line a skin printed, as the log keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintedLine {
    pub text: String,
    /// How many times this exact line was printed.
    pub count: u64,
}

/// A copy of everything the log holds at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LuaDiagnostics {
    /// One entry per function that has ever failed, in id order.
    pub function_failures: Vec<FunctionFailure>,
    /// One entry per distinct message, in the order each first appeared.
    pub swallowed: Vec<SwallowedError>,
    /// Errors caught after the store of distinct messages was full.
    pub swallowed_overflow: u64,
    /// What the skin printed: one entry per distinct line, in the order each first appeared.
    pub prints: Vec<PrintedLine>,
    /// Lines printed after the store of distinct lines was full.
    pub prints_overflow: u64,
    /// Frames in which the budget refused or cut off at least one call.
    pub frames_over_budget: u64,
}

/// Distinct texts in the order each first appeared, with how often each was seen.
///
/// A skin repeats itself: a function that prints or fails does so on every frame. Keeping the first
/// occurrence and a count keeps the log the size of what is wrong rather than of how long the
/// screen has been up.
#[derive(Debug)]
struct Tally {
    capacity: usize,
    entries: Vec<(String, u64)>,
    index: BTreeMap<String, usize>,
    overflow: u64,
}

impl Tally {
    /// An empty tally that stores at most `capacity` distinct texts.
    fn new(capacity: usize) -> Self {
        Self { capacity, entries: Vec::new(), index: BTreeMap::new(), overflow: 0 }
    }

    /// Counts one occurrence of `text`, cut to the length the log keeps.
    fn record(&mut self, text: &str) {
        let mut end = text.len().min(MAX_LOGGED_TEXT_BYTES);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let text = &text[..end];
        if let Some(position) = self.index.get(text) {
            self.entries[*position].1 += 1;
        } else if self.entries.len() < self.capacity {
            self.index.insert(text.to_owned(), self.entries.len());
            self.entries.push((text.to_owned(), 1));
        } else {
            self.overflow += 1;
        }
    }
}

/// The record of what went wrong inside one skin's interpreter.
///
/// The reference logs every failed call on every frame and never logs what a skin's own `pcall`
/// caught. Neither is useful for a skin that calls a thousand functions a frame and wraps every
/// part it loads in `pcall`, so this log keeps the first message and a count per failing function,
/// and records caught errors and printed lines the same way. It only records; whether anything is
/// shown to the player is the caller's decision.
#[derive(Debug)]
pub struct LuaLog {
    failures: RefCell<BTreeMap<LuaFnId, FunctionFailure>>,
    swallowed: RefCell<Tally>,
    prints: RefCell<Tally>,
}

impl Default for LuaLog {
    fn default() -> Self {
        Self {
            failures: RefCell::new(BTreeMap::new()),
            swallowed: RefCell::new(Tally::new(MAX_LOGGED_SWALLOWED)),
            prints: RefCell::new(Tally::new(MAX_LOGGED_PRINTS)),
        }
    }
}

impl LuaLog {
    /// Records that a call of `function` raised `message`. True when this is the function's first
    /// failure, which is the caller's cue to warn once.
    pub fn function_failed(&self, function: LuaFnId, kind: LuaFnKind, message: &str) -> bool {
        let mut failures = self.failures.borrow_mut();
        match failures.get_mut(&function) {
            Some(failure) => {
                failure.count += 1;
                false
            }
            None => {
                failures.insert(function, FunctionFailure { function, kind, first_message: message.to_owned(), count: 1 });
                true
            }
        }
    }

    /// Records an error the skin's own `pcall` or `xpcall` caught.
    pub fn swallowed(&self, message: &str) {
        self.swallowed.borrow_mut().record(message);
    }

    /// Records one line the skin printed.
    pub fn printed(&self, line: &str) {
        self.prints.borrow_mut().record(line);
    }

    /// Everything recorded so far, with the given count of over-budget frames.
    fn snapshot(&self, frames_over_budget: u64) -> LuaDiagnostics {
        let swallowed = self.swallowed.borrow();
        let prints = self.prints.borrow();
        LuaDiagnostics {
            function_failures: self.failures.borrow().values().cloned().collect(),
            swallowed: swallowed
                .entries
                .iter()
                .map(|(message, count)| {
                    let location = error_location(message);
                    SwallowedError {
                        message: message.clone(),
                        file: location.map(|(file, _)| file.to_owned()),
                        line: location.map(|(_, line)| line),
                        count: *count,
                    }
                })
                .collect(),
            swallowed_overflow: swallowed.overflow,
            prints: prints.entries.iter().map(|(text, count)| PrintedLine { text: text.clone(), count: *count }).collect(),
            prints_overflow: prints.overflow,
            frames_over_budget,
        }
    }
}

/// What every library installed into one interpreter shares.
///
/// One of these is built per [`SkinLua`] and handed to each module's `install` behind an `Rc`, so a
/// function a module creates can keep a clone and reach the paths, the log and the meter for as
/// long as the interpreter lives.
#[derive(Debug)]
pub(crate) struct LuaShared {
    pub(crate) paths: SkinPaths,
    pub(crate) log: LuaLog,
    pub(crate) meter: Meter,
    pub(crate) mode: LuaMode,
    pub(crate) seed: Option<u64>,
    /// How many files of the disk the skin holds open through `io`.
    pub(crate) open_files: io::OpenFiles,
    /// How much the skin has left in the overlay, and how much more it may.
    pub(crate) quota: Rc<io::OverlayQuota>,
}

/// One registered function value and what it is called as.
struct RegisteredFn {
    function: Function,
    kind: LuaFnKind,
}

/// What a timer function answered the last time a frame read it, and which frame that was.
#[derive(Debug, Clone, Copy)]
struct TimerAnswer {
    /// The frame the answer belongs to, as [`SkinLua::frame`] counts them.
    frame: u64,
    /// The microsecond the timer switched on, or [`TIMER_OFF`].
    started_us: i64,
}

/// What the calls of one frame came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameCost {
    /// Function calls the frame made. A call the budget refused is not one of them.
    pub calls: u32,
    /// Timer reads the frame answered from an earlier read of the same timer, with no call
    /// ([`BoundFrame::timer`]).
    pub reused: u32,
    /// Wall clock the calls spent inside Lua.
    pub spent: Duration,
}

/// One skin's interpreter, with everything the skin left in it.
///
/// A loaded skin owns exactly one of these for as long as its screen is shown, because the
/// functions the skin's objects are made of live inside it. It is not `Send`: a skin is loaded and
/// drawn on one thread.
pub struct SkinLua {
    lua: Lua,
    shared: Rc<LuaShared>,
    functions: RefCell<Vec<RegisteredFn>>,
    by_identity: RefCell<BTreeMap<(usize, LuaFnKind), LuaFnId>>,
    /// What each function answered the last time it ran, by id: the value a frame gets when the
    /// budget will not let it call the function again.
    last_values: RefCell<Vec<Option<Value>>>,
    /// How many frames have been bound, which is also the number of the one running or last run.
    frames: Cell<u64>,
    /// What each timer function answered on the frame that last read it, by id.
    timer_answers: RefCell<Vec<Option<TimerAnswer>>>,
    /// Timer reads the frame running, or last run, answered without a call.
    timer_reuses: Cell<u32>,
}

impl std::fmt::Debug for SkinLua {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SkinLua")
            .field("root", &self.shared.paths.root())
            .field("mode", &self.shared.mode)
            .field("functions", &self.functions.borrow().len())
            .finish_non_exhaustive()
    }
}

impl SkinLua {
    /// Builds the interpreter and installs the whole environment into it.
    ///
    /// The libraries go in in a fixed order -- base, `package`, `io`, `os`, `main_state` with its
    /// prelude, then `luajava` -- because each later one publishes itself through the one before.
    pub fn new(config: SkinLuaConfig) -> Result<Self, SkinError> {
        let root = config.root.to_string_lossy().into_owned();
        let failed = |error: mlua::Error| SkinError::LuaLoad { path: root.clone(), message: error_message(&error) };

        let paths = SkinPaths::new(&config.root, config.overlay.as_deref())?;
        let shared = Rc::new(LuaShared {
            paths,
            log: LuaLog::default(),
            meter: Meter::new(config.budget),
            mode: config.mode,
            seed: config.seed,
            open_files: io::OpenFiles::default(),
            quota: Rc::new(io::OverlayQuota::measure(config.overlay.as_deref(), config.overlay_limits)),
        });
        let lua = env::new_state().map_err(failed)?;
        shared.meter.install(&lua).map_err(failed)?;
        env::install(&lua, &shared).map_err(failed)?;
        package::install(&lua, &shared).map_err(failed)?;
        io::install(&lua, &shared).map_err(failed)?;
        os::install(&lua, &shared).map_err(failed)?;
        main_state::install(&lua, &shared).map_err(failed)?;
        luajava::install(&lua, &shared).map_err(failed)?;

        Ok(Self {
            lua,
            shared,
            functions: RefCell::new(Vec::new()),
            by_identity: RefCell::new(BTreeMap::new()),
            last_values: RefCell::new(Vec::new()),
            frames: Cell::new(0),
            timer_answers: RefCell::new(Vec::new()),
            timer_reuses: Cell::new(0),
        })
    }

    /// The interpreter itself, for the table converter and for tests. Nothing outside this crate's
    /// loader should need it.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// The skin's view of the filesystem.
    pub fn paths(&self) -> &SkinPaths {
        &self.shared.paths
    }

    /// Which way this interpreter was set up.
    pub fn mode(&self) -> LuaMode {
        self.shared.mode
    }

    /// The record of what has gone wrong so far.
    pub fn log(&self) -> &LuaLog {
        &self.shared.log
    }

    /// A copy of the log, with the meter's count of over-budget frames.
    pub fn diagnostics(&self) -> LuaDiagnostics {
        self.shared.log.snapshot(self.shared.meter.frames_over_budget())
    }

    /// Bytes the interpreter currently holds.
    pub fn memory_used(&self) -> usize {
        self.lua.used_memory()
    }

    /// Runs the skin's entry file once and returns the value it returns.
    ///
    /// The loader calls this twice on the same interpreter: first as [`LuaPass::Header`] with
    /// `skin_config` unset, then, after [`Self::set_skin_config`], as [`LuaPass::Body`]. Globals and
    /// `package.loaded` carry over, so a module the header pass required is not run again and the
    /// body pass executes little more than the entry file's own few lines.
    ///
    /// `host` is bound for the whole pass, because a skin reads `main_state` while it builds its
    /// tables. The pass is metered against the load budget. A compile error, a runtime error and a
    /// budget overrun all abandon the skin: [`SkinError::LuaBudget`] for the overrun and
    /// [`SkinError::LuaLoad`] for the rest.
    pub fn run_entry(&self, entry: &Path, pass: LuaPass, host: &dyn SkinHost) -> Result<Value, SkinError> {
        let meter = &self.shared.meter;
        meter.begin_load();
        let outcome = self.lua.scope(|scope| {
            main_state::bind(&self.lua, &self.shared, scope, host)?;
            env::run_file(&self.lua, &self.shared, entry)
        });
        let overran = meter.exhausted();
        meter.finish();

        let name = format!("{} ({})", entry.to_string_lossy(), pass.label());
        match outcome {
            Ok(value) => Ok(value),
            Err(error) if overran || matches!(error, mlua::Error::MemoryError(_)) => Err(SkinError::LuaBudget { expr: name }),
            Err(error) => Err(SkinError::LuaLoad { path: name, message: error_text(&error) }),
        }
    }

    /// Runs `run` with `host` bound and the load budget metering it.
    ///
    /// This is for the work a loader does on what a pass returned: reading a table can run a skin's
    /// metamethods, and [`Self::compile`] calls a timer script once, so both want a host to read
    /// and a ceiling to stop at. An overrun is [`SkinError::LuaBudget`], whatever `run` made of it.
    pub fn with_host<R>(&self, host: &dyn SkinHost, run: impl FnOnce() -> R) -> Result<R, SkinError> {
        let meter = &self.shared.meter;
        meter.begin_load();
        let outcome = self.lua.scope(|scope| {
            main_state::bind(&self.lua, &self.shared, scope, host)?;
            Ok(run())
        });
        let overran = meter.exhausted();
        meter.finish();
        match outcome {
            Ok(_) if overran => Err(SkinError::LuaBudget { expr: main_state::MAIN_STATE_MODULE.to_owned() }),
            Ok(value) => Ok(value),
            Err(error) => Err(SkinError::Lua { expr: main_state::MAIN_STATE_MODULE.to_owned(), message: error_message(&error) }),
        }
    }

    /// Publishes every member of `main_state`, `timer_util` and `event_util` as a global of the same
    /// name, beside the modules themselves.
    ///
    /// This is where a JSON document's scripts find the game state: the reference builds a JSON
    /// skin's interpreter with `SkinLuaAccessor(true)`, which exports straight into the globals, so
    /// such a script writes `number(10) > 0` and `is_timer_on(41)` with no `require` in front. A Lua
    /// skin is never given these; it requires the modules.
    pub fn publish_globals(&self) -> Result<(), SkinError> {
        let modules = [main_state::MAIN_STATE_MODULE, main_state::TIMER_UTIL_MODULE, main_state::EVENT_UTIL_MODULE];
        self.lua
            .load(GLOBAL_EXPORT)
            .set_name(GLOBAL_EXPORT_CHUNK_NAME)
            .set_mode(ChunkMode::Text)
            .call::<()>(modules.as_slice())
            .map_err(|error| SkinError::LuaLoad { path: GLOBAL_EXPORT_CHUNK_NAME.to_owned(), message: error_message(&error) })
    }

    /// Publishes the `skin_config` global the body pass reads. Called once, between the two passes.
    pub fn set_skin_config(&self, config: SkinConfigGlobal) -> Result<(), SkinError> {
        env::set_skin_config(&self.lua, config).map_err(|error| SkinError::LuaLoad { path: SKIN_CONFIG_GLOBAL.to_owned(), message: error.to_string() })
    }

    /// Takes a function value into the registry and answers the id the model keeps for it.
    ///
    /// The same function registered again as the same kind answers the same id, so a timer function
    /// shared by many objects is one entry and is called once per frame for all of them
    /// ([`BoundFrame::timer`]). The same function registered as a different kind is a different
    /// entry.
    pub fn register(&self, function: Function, kind: LuaFnKind) -> LuaFnId {
        let identity = (function.to_pointer() as usize, kind);
        if let Some(id) = self.by_identity.borrow().get(&identity) {
            return *id;
        }
        let mut functions = self.functions.borrow_mut();
        let id = LuaFnId(functions.len() as u32);
        functions.push(RegisteredFn { function, kind });
        self.last_values.borrow_mut().push(None);
        self.timer_answers.borrow_mut().push(None);
        self.by_identity.borrow_mut().insert(identity, id);
        id
    }

    /// Compiles a script a skin wrote as a string where a function could have gone, and registers
    /// the result.
    ///
    /// The value kinds compile `return <source>`; [`LuaFnKind::Event`] and the two writer kinds
    /// compile the source as written. [`LuaFnKind::Timer`] compiles `return <source>` and calls it
    /// once: when that yields a function, the function is the timer and the chunk is discarded
    /// (`SkinLuaAccessor.loadTimerProperty`). Only text chunks are accepted. A source that does not
    /// compile, or a timer script whose one call raises, is [`SkinError::Lua`], which costs the one
    /// field that carried it and not the skin.
    ///
    /// Every call compiles afresh, even for a source seen before: a timer script may build a
    /// function with state of its own, and two objects that wrote the same script each get theirs.
    ///
    /// A timer script that reads `main_state` in its one call needs a host bound, so the loader
    /// compiles inside [`Self::with_host`]. Outside any binding the call is still metered against
    /// the load budget.
    pub fn compile(&self, source: &str, kind: LuaFnKind) -> Result<LuaFnId, SkinError> {
        let failed = |error: mlua::Error| SkinError::Lua { expr: source.to_owned(), message: error_message(&error) };
        let text = match kind {
            LuaFnKind::Event | LuaFnKind::FloatWriter | LuaFnKind::TextWriter => source.to_owned(),
            LuaFnKind::Boolean | LuaFnKind::Integer | LuaFnKind::Float | LuaFnKind::Text | LuaFnKind::Timer => format!("{SCRIPT_RETURN_PREFIX}{source}"),
        };
        let chunk = self.lua.load(text).set_name(SCRIPT_CHUNK_NAME).set_mode(ChunkMode::Text).into_function().map_err(failed)?;
        if kind != LuaFnKind::Timer {
            return Ok(self.register(chunk, kind));
        }

        let meter = &self.shared.meter;
        let unmetered = meter.phase() == Phase::Idle;
        if unmetered {
            meter.begin_load();
        }
        let trial = chunk.call::<Value>(());
        if unmetered {
            meter.finish();
        }
        match trial.map_err(failed)? {
            Value::Function(timer) => Ok(self.register(timer, kind)),
            _ => Ok(self.register(chunk, kind)),
        }
    }

    /// How many functions the registry holds.
    pub fn function_count(&self) -> usize {
        self.functions.borrow().len()
    }

    /// What `function` was registered as, or `None` for an id this registry never handed out.
    pub fn kind_of(&self, function: LuaFnId) -> Option<LuaFnKind> {
        self.functions.borrow().get(function.0 as usize).map(|registered| registered.kind)
    }

    /// The registered function behind an id.
    fn function(&self, function: LuaFnId) -> Option<Function> {
        self.functions.borrow().get(function.0 as usize).map(|registered| registered.function.clone())
    }

    /// What `function` answered the last time it ran to its end.
    fn last_value(&self, function: LuaFnId) -> Option<Value> {
        self.last_values.borrow().get(function.0 as usize).cloned().flatten()
    }

    /// Keeps `value` as what `function` last answered.
    fn remember(&self, function: LuaFnId, value: &Value) {
        if let Some(slot) = self.last_values.borrow_mut().get_mut(function.0 as usize) {
            *slot = Some(value.clone());
        }
    }

    /// What `function` answered as a timer on frame `frame`, when that frame has read it already.
    fn timer_answer(&self, function: LuaFnId, frame: u64) -> Option<i64> {
        let answer = self.timer_answers.borrow().get(function.0 as usize).copied().flatten()?;
        (answer.frame == frame).then_some(answer.started_us)
    }

    /// Keeps `started_us` as what `function` answered as a timer on frame `frame`.
    fn keep_timer_answer(&self, function: LuaFnId, frame: u64, started_us: i64) {
        if let Some(slot) = self.timer_answers.borrow_mut().get_mut(function.0 as usize) {
            *slot = Some(TimerAnswer { frame, started_us });
        }
    }

    /// What the calls of the frame now running have come to so far, or those of the last frame once
    /// it is over. Nothing before the first frame.
    pub fn frame_cost(&self) -> FrameCost {
        let meter = &self.shared.meter;
        FrameCost { calls: meter.frame_calls(), reused: self.timer_reuses.get(), spent: meter.frame_spent() }
    }

    /// Binds `host` for one frame and runs `run` inside the binding.
    ///
    /// Every call a frame makes -- conditions, timers, values, and the events a click fires -- goes
    /// through the [`BoundFrame`] handed to `run`, inside this one binding, so the host is attached
    /// once per frame however many functions are called. The frame is metered against the frame
    /// budget from the moment this is entered, but its wall clock runs only while one of those calls
    /// does: what `run` spends between two calls, drawing included, is not charged to the skin.
    ///
    /// A binding is also the span a timer function's answer is reused over ([`BoundFrame::timer`]):
    /// each binding starts with none kept.
    ///
    /// The only error is a failure to bind at all; a function that fails inside the frame is
    /// recorded in the log and answered with its kind's default.
    pub fn frame<R>(&self, host: &dyn SkinHost, run: impl FnOnce(&BoundFrame<'_>) -> R) -> Result<R, SkinError> {
        let meter = &self.shared.meter;
        meter.begin_frame();
        let serial = self.frames.get() + 1;
        self.frames.set(serial);
        self.timer_reuses.set(0);
        let outcome = self.lua.scope(|scope| {
            main_state::bind(&self.lua, &self.shared, scope, host)?;
            Ok(run(&BoundFrame { owner: self, host, serial }))
        });
        meter.finish();
        outcome.map_err(|error| SkinError::Lua { expr: main_state::MAIN_STATE_MODULE.to_owned(), message: error_message(&error) })
    }
}

/// One frame's handle on a skin's functions, valid while the host is bound.
///
/// Each `call_*`:
///
/// 1. asks [`Meter::begin_call`], and when the frame's budget is spent does not call at all;
/// 2. calls the function with the arguments of its [`LuaFnKind`];
/// 3. reads the first result with the coercion of its kind from [`coerce`];
/// 4. on a Lua error records it with [`LuaLog::function_failed`] and answers the kind's default.
///    The function is not disabled: it is called again on the next frame, as the reference does.
///
/// A call the budget refuses in step 1, or cuts off while it runs, answers what the function
/// answered the last time it ran to its end, and the kind's default only when it never has. The
/// object it belongs to therefore holds still for a frame instead of blinking out.
///
/// Every `call_*` is one call of the function, however often it is asked. [`Self::timer`] is the one
/// reader that is not: it calls a timer function the first time a frame reads it and answers every
/// later read of the frame with what that call answered. That is the reader a frame is drawn through
/// (the [`LuaDrawEval`] this implements), because drawing reads one timer many times over.
///
/// Calling a function as a kind other than the one it was registered as is allowed and simply
/// applies that call's own rules.
pub struct BoundFrame<'a> {
    owner: &'a SkinLua,
    host: &'a dyn SkinHost,
    /// Which frame this is, as [`SkinLua::frame`] counts them.
    serial: u64,
}

impl std::fmt::Debug for BoundFrame<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("BoundFrame").field("functions", &self.owner.function_count()).finish()
    }
}

impl BoundFrame<'_> {
    /// The interpreter this frame is bound in.
    pub fn skin_lua(&self) -> &SkinLua {
        self.owner
    }

    /// The host this frame is bound to.
    pub fn host(&self) -> &dyn SkinHost {
        self.host
    }

    /// Makes one metered call and answers its first result. A call the budget refused or cut off
    /// answers the function's previous result instead; `None` means there is nothing to read, because
    /// the id is unknown, the function raised, or the budget stopped a function that has never
    /// finished.
    fn invoke(&self, function: LuaFnId, kind: LuaFnKind, arguments: impl IntoLuaMulti) -> Option<Value> {
        let callee = self.owner.function(function)?;
        let shared = &self.owner.shared;
        if !shared.meter.begin_call() {
            return self.owner.last_value(function);
        }
        let outcome = callee.call::<Value>(arguments);
        let cut_off = shared.meter.exhausted();
        shared.meter.end_call();
        match outcome {
            Ok(value) => {
                if !matches!(kind, LuaFnKind::Event | LuaFnKind::FloatWriter | LuaFnKind::TextWriter) {
                    self.owner.remember(function, &value);
                }
                Some(value)
            }
            Err(error) => {
                shared.log.function_failed(function, kind, &error_message(&error));
                if cut_off { self.owner.last_value(function) } else { None }
            }
        }
    }

    /// Calls a condition. `false` when it fails or returns nothing.
    pub fn call_boolean(&self, function: LuaFnId) -> bool {
        self.invoke(function, LuaFnKind::Boolean, ()).is_some_and(|value| coerce::to_boolean(&value))
    }

    /// Calls an integer value. Zero when it fails.
    pub fn call_integer(&self, function: LuaFnId) -> i32 {
        self.invoke(function, LuaFnKind::Integer, ()).map_or(0, |value| coerce::to_int(&value))
    }

    /// Calls a float value. Zero when it fails.
    pub fn call_float(&self, function: LuaFnId) -> f32 {
        self.invoke(function, LuaFnKind::Float, ()).map_or(0.0, |value| coerce::to_float(&value))
    }

    /// Calls a text value. Empty when it fails.
    pub fn call_text(&self, function: LuaFnId) -> String {
        self.invoke(function, LuaFnKind::Text, ()).map_or_else(String::new, |value| coerce::to_jstring(&value))
    }

    /// Calls a timer function and answers the microsecond it reports, or [`TIMER_OFF`] when it
    /// fails. A function that returns nothing answers zero, a timer on since the scene began.
    pub fn call_timer(&self, function: LuaFnId) -> i64 {
        self.invoke(function, LuaFnKind::Timer, ()).map_or(TIMER_OFF, |value| coerce::to_long(&value))
    }

    /// The microsecond a timer function reports on this frame: [`Self::call_timer`] the first time
    /// the frame reads the function, and that same answer every time after.
    ///
    /// The reference calls the function on every read, and it reads a timer twice for each object
    /// that follows one and twice more for each image that animates on one (`isOff` and then `get`,
    /// `SkinObject.prepareRegion` and `SkinSourceImage.getImageIndex`). One call serves all of those
    /// here. The answers are the reference's as long as a timer function reports the same moment
    /// however often one frame asks, which holds for everything the reference builds a timer from:
    /// the frame clock and every built-in timer are fixed before the first object is prepared, and
    /// `timer_util.timer_observe_boolean` latches against that same clock, so its second call of a
    /// frame changes nothing its first did not. Two kinds of function could tell the difference: one
    /// that counts its own calls, and one that several objects share and that reads something
    /// another object's function changes between two of them, which here shows a frame later.
    ///
    /// What is reused is whatever the one call came to. A function that raised reads as off for the
    /// rest of the frame and is logged once for it; one the budget refused or cut off reads as what
    /// it answered on an earlier frame. Either is called again on the next frame.
    pub fn timer(&self, function: LuaFnId) -> i64 {
        if let Some(started_us) = self.owner.timer_answer(function, self.serial) {
            self.owner.timer_reuses.set(self.owner.timer_reuses.get().saturating_add(1));
            return started_us;
        }
        let started_us = self.call_timer(function);
        self.owner.keep_timer_answer(function, self.serial, started_us);
        started_us
    }

    /// Calls an event function with its one argument. Failures are recorded and otherwise ignored.
    pub fn call_event(&self, function: LuaFnId, argument: i32) {
        self.invoke(function, LuaFnKind::Event, argument);
    }

    /// Calls a slider's writer with the new share.
    pub fn call_float_writer(&self, function: LuaFnId, value: f32) {
        self.invoke(function, LuaFnKind::FloatWriter, value);
    }

    /// Calls a text's writer with the new text.
    pub fn call_text_writer(&self, function: LuaFnId, value: &str) {
        self.invoke(function, LuaFnKind::TextWriter, value);
    }
}

/// A frame is the evaluator the interpolator and the renderer ask about everything a skin wrote
/// where a property id belongs. A function is called through the frame's own methods; a name is
/// turned into its id and read from the host, as the reference resolves a named property once and
/// reads it like any other.
///
/// A condition and a value are called every time they are asked for, because a skin hangs work on
/// them and counts on each object's own call. A timer is called once a frame however many objects
/// and images read it ([`BoundFrame::timer`]).
impl LuaDrawEval for BoundFrame<'_> {
    fn call_boolean(&self, function: LuaFnId) -> bool {
        BoundFrame::call_boolean(self, function)
    }

    fn call_integer(&self, function: LuaFnId) -> i32 {
        BoundFrame::call_integer(self, function)
    }

    fn call_float(&self, function: LuaFnId) -> f32 {
        BoundFrame::call_float(self, function)
    }

    fn call_text(&self, function: LuaFnId) -> String {
        BoundFrame::call_text(self, function)
    }

    fn call_timer(&self, function: LuaFnId) -> i64 {
        self.timer(function)
    }

    fn named_boolean(&self, name: &str) -> bool {
        main_state::named_id(&self.owner.lua, NameSpace::Boolean, name).and_then(|id| self.host.boolean(id)).unwrap_or(false)
    }

    fn named_integer(&self, name: &str) -> i32 {
        main_state::named_id(&self.owner.lua, NameSpace::Integer, name).map_or(0, |id| self.host.integer(id))
    }

    fn named_float(&self, name: &str) -> f32 {
        main_state::named_id(&self.owner.lua, NameSpace::Rate, name).and_then(|id| self.host.rate(id)).unwrap_or(0.0)
    }

    fn named_text(&self, name: &str) -> String {
        main_state::named_id(&self.owner.lua, NameSpace::Text, name).map_or_else(String::new, |id| self.host.text(id).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{LuaFnKind, LuaMode, LuaPass, SkinLua, SkinLuaConfig};
    use crate::SkinError;
    use crate::property::DefaultState;
    use crate::timer::TIMER_OFF;

    fn runtime(mode: LuaMode) -> SkinLua {
        SkinLua::new(SkinLuaConfig { mode, seed: Some(1), ..SkinLuaConfig::new(Path::new(env!("CARGO_MANIFEST_DIR"))) }).expect("the runtime builds")
    }

    #[test]
    fn the_three_modules_are_published_in_both_modes() {
        for mode in [LuaMode::Full, LuaMode::HeaderOnly] {
            let runtime = runtime(mode);
            let published: bool = runtime
                .lua()
                .load("return type(package.loaded.main_state) == 'table' and type(package.loaded.timer_util) == 'table' and type(package.loaded.event_util) == 'table'")
                .eval()
                .expect("the check runs");
            assert!(published, "{mode:?} publishes main_state, timer_util and event_util");
        }
    }

    #[test]
    fn a_function_registered_twice_as_one_kind_keeps_one_id() {
        let runtime = runtime(LuaMode::Full);
        let function: mlua::Function = runtime.lua().load("return function() return true end").eval().expect("the function is created");
        let first = runtime.register(function.clone(), LuaFnKind::Boolean);
        assert_eq!(runtime.register(function.clone(), LuaFnKind::Boolean), first);
        assert_ne!(runtime.register(function, LuaFnKind::Timer), first, "another kind is another entry");
        assert_eq!(runtime.function_count(), 2);
        assert_eq!(runtime.kind_of(first), Some(LuaFnKind::Boolean));
    }

    #[test]
    fn an_entry_file_that_cannot_run_abandons_the_skin() {
        let runtime = runtime(LuaMode::Full);
        let outcome = runtime.run_entry(Path::new("missing.luaskin"), LuaPass::Header, &DefaultState);
        assert!(matches!(outcome, Err(SkinError::LuaLoad { .. })), "got {outcome:?}");
    }

    #[test]
    fn a_frame_reads_each_kind_by_its_own_rule_and_logs_a_failure_once() {
        let runtime = runtime(LuaMode::Full);
        let make = |source: &str, kind: LuaFnKind| runtime.register(runtime.lua().load(source).eval().expect("the function is created"), kind);
        let zero = make("return function() return 0 end", LuaFnKind::Boolean);
        let ratio = make("return function() return 1920 / 7 end", LuaFnKind::Integer);
        let silent = make("return function() end", LuaFnKind::Timer);
        let broken = make("return function() error('boom') end", LuaFnKind::Timer);

        runtime
            .frame(&DefaultState, |frame| {
                assert!(frame.call_boolean(zero), "zero is true");
                assert_eq!(frame.call_integer(ratio), 274);
                assert_eq!(frame.call_timer(silent), 0, "a timer that returns nothing is on since zero");
                assert_eq!(frame.call_timer(broken), TIMER_OFF);
                assert_eq!(frame.call_timer(broken), TIMER_OFF);
            })
            .expect("the frame binds");

        let failures = runtime.diagnostics().function_failures;
        assert_eq!(failures.len(), 1);
        assert_eq!((failures[0].function, failures[0].count), (broken, 2));
    }
}
