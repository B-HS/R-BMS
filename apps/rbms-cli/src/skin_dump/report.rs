//! What a dump found, as plain data. [`super::render`] prints it as tables and `--json` serialises it
//! as it is.

use serde::Serialize;

/// One customisation row of a header: an option, a file slot or an offset.
#[derive(Debug, Clone, Serialize)]
pub struct RowReport {
    pub name: String,
    /// The item or file the document names as the default, when it names one.
    pub default: Option<String>,
}

/// What a skin says about itself, from the header pass alone.
#[derive(Debug, Clone, Serialize)]
pub struct HeaderReport {
    pub skin_type: i32,
    pub type_label: String,
    pub name: String,
    pub author: String,
    pub width: i32,
    pub height: i32,
    pub parser: String,
    pub categories: usize,
    pub options: Vec<RowReport>,
    pub file_slots: Vec<RowReport>,
    pub offsets: Vec<RowReport>,
    /// How many of the offsets the player's skin always has and the document did not declare.
    pub automatic_offsets: usize,
}

impl HeaderReport {
    pub fn defaults(rows: &[RowReport]) -> usize {
        rows.iter().filter(|row| row.default.is_some()).count()
    }
}

/// How many of one kind of thing a document declares.
#[derive(Debug, Clone, Serialize)]
pub struct CountReport {
    pub kind: &'static str,
    pub count: usize,
}

/// A declared file and whether it is there.
#[derive(Debug, Clone, Serialize)]
pub struct FileReport {
    pub id: String,
    /// The path from the skin folder.
    pub path: String,
    pub exists: bool,
}

/// The files a document names, split into what it declared and what the loader resolved.
#[derive(Debug, Clone, Default, Serialize)]
pub struct FilesReport {
    pub sources_declared: usize,
    pub sources: Vec<FileReport>,
    pub fonts_declared: usize,
    pub fonts: Vec<FileReport>,
}

impl FilesReport {
    pub fn missing(files: &[FileReport]) -> usize {
        files.iter().filter(|file| !file.exists).count()
    }
}

/// One line of a diagnostics list: a message and how often it happened.
#[derive(Debug, Clone, Serialize)]
pub struct EntryReport {
    pub text: String,
    pub count: u64,
}

/// What went wrong, or was said, while a skin loaded.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DiagnosticsReport {
    pub warnings: Vec<String>,
    pub swallowed: Vec<EntryReport>,
    pub swallowed_overflow: u64,
    pub function_failures: Vec<EntryReport>,
    pub prints: Vec<EntryReport>,
    pub prints_overflow: u64,
    pub frames_over_budget: u64,
}

/// The times a document's header sets for its scene, in the document's own milliseconds. They are
/// read off the loaded document, so a document whose body fails does not report them.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct TimingsReport {
    pub scene: i32,
    pub input: i32,
    pub fadeout: i32,
    pub loadend: i32,
    pub playstart: i32,
    pub close: i32,
}

/// A document's objects and function values.
#[derive(Debug, Clone, Serialize)]
pub struct BodyReport {
    pub timings: TimingsReport,
    pub destinations: usize,
    pub destinations_assembled: usize,
    pub objects: Vec<CountReport>,
    pub functions: usize,
    pub function_kinds: Vec<CountReport>,
    pub files: FilesReport,
    pub options_selected: usize,
    pub lua_memory_bytes: Option<usize>,
    pub diagnostics: DiagnosticsReport,
}

/// One function that took the longest, from the per-call timing pass.
#[derive(Debug, Clone, Serialize)]
pub struct SlowCallReport {
    pub function: u32,
    pub kind: &'static str,
    pub max_micros: f64,
    pub mean_micros: f64,
}

/// What calling every function value once per frame cost.
#[derive(Debug, Clone, Serialize)]
pub struct FramesReport {
    pub frames: usize,
    /// Function values called in each frame: every boolean, integer, float, text and timer.
    pub calls_per_frame: usize,
    pub mean_micros: f64,
    pub median_micros: f64,
    pub p99_micros: f64,
    pub max_micros: f64,
    /// Frames in which the budget refused or cut off at least one call.
    pub frames_over_budget: u64,
    /// Calls that failed during the frames, with how many times each failed.
    pub function_failures: Vec<EntryReport>,
    /// Errors the skin's own `pcall` caught while the frames ran.
    pub swallowed_during_frames: u64,
    pub slowest_calls: Vec<SlowCallReport>,
}

/// The smallest budgets a skin ran inside, found by loading it again under smaller ones.
#[derive(Debug, Clone, Serialize)]
pub struct ProbeReport {
    /// Instructions one pass of the entry file needs, and the default it is allowed.
    pub load_instructions: Option<u64>,
    pub load_instructions_limit: u64,
    /// Instructions the most expensive single call of a frame needs, and the default it is allowed.
    pub call_instructions: Option<u64>,
    pub call_instructions_limit: u64,
    pub probe_frames: usize,
    pub note: Option<String>,
}

/// One skin document.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentReport {
    /// The file name from the pack folder.
    pub file: String,
    pub header: Option<HeaderReport>,
    pub header_error: Option<String>,
    pub scenario: String,
    pub load_millis: f64,
    pub load_error: Option<String>,
    pub body: Option<BodyReport>,
    pub frames: Option<FramesReport>,
    pub frames_error: Option<String>,
    pub probe: Option<ProbeReport>,
}

/// Whether the pack folder was left as it was found.
#[derive(Debug, Clone, Serialize)]
pub struct PackCheckReport {
    pub folder: String,
    pub files_before: usize,
    pub files_after: usize,
    /// Files that appeared, vanished, or changed size or modification time, as paths from the folder.
    pub changed: Vec<String>,
}

impl PackCheckReport {
    pub fn unchanged(&self) -> bool {
        self.changed.is_empty()
    }
}

/// A whole dump.
#[derive(Debug, Clone, Serialize)]
pub struct DumpReport {
    pub target: String,
    pub seed: u64,
    pub frame_micros: i64,
    pub documents: Vec<DocumentReport>,
    pub pack: PackCheckReport,
}
