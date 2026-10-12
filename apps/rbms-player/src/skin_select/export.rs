//! Exporting a pack: a copy of the pack in effect, with the choices made in its documents written
//! into it, in a folder that can be taken to another machine or handed to somebody else.
//!
//! What is written is one pack folder: every file of the pack -- its documents, their Lua, images,
//! fonts and sounds -- and one more that holds the choices ([`ADJUSTMENTS_FILE`]). Left out are what
//! is this machine's and not the pack's: the record the installer keeps of the default pack, hidden
//! files, and any adjustments the pack came with, which the new file replaces. A link inside the
//! pack is never followed out of it: one that leads to a file of the pack is written as that file,
//! and any other is passed over and counted.
//!
//! The folder is made beside nothing it could harm. It goes under the folder the player chose,
//! named after the pack ([`EXPORT_SUFFIX`]), and under a name that is free: an export never writes
//! into a folder that is already there. It is written under another name ([`INCOMPLETE_SUFFIX`])
//! and given its own once every file is in it, so a folder under the final name is always whole.
//! An export that fails or is called off takes away what it wrote; if it cannot, what is left still
//! says by its name that it is not whole.
//!
//! A pack may be hundreds of megabytes, so the copy runs off the frame loop ([`ExportJob`]) and says
//! how far it has got. There is a ceiling on how much one export writes ([`ExportLimits`]): a pack
//! past it is refused before anything is written.

use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use super::adjustments::ADJUSTMENTS_FILE;
use crate::assets::default_pack::MANIFEST_FILE;

/// What an exported folder's name ends with, after the name of the pack it is a copy of.
pub(crate) const EXPORT_SUFFIX: &str = "-export";

/// What the folder is named with, after its own name, until every file is in it.
pub(crate) const INCOMPLETE_SUFFIX: &str = ".incomplete";

/// What a pack with no name of its own -- a drive, a root -- is exported as.
const UNNAMED_PACK: &str = "skin";

/// What a file or a folder the pack's owner hid begins with. Such an entry is this machine's -- an
/// index a file manager keeps, a version history -- and is not part of the pack.
const HIDDEN_PREFIX: char = '.';

/// The first number a folder's name is given when the name is taken, and the last that is tried.
const FIRST_NUMBERED: u32 = 2;
const LAST_NUMBERED: u32 = 999;

/// The most one export writes: more bytes or more files than any skin pack holds, and few enough
/// that a folder chosen by mistake -- a whole disk -- is refused rather than copied.
pub(crate) const EXPORT_MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub(crate) const EXPORT_MAX_FILES: usize = 50_000;

/// How deep below the pack's folder a file may be and still be exported.
const EXPORT_MAX_DEPTH: usize = 32;

/// How much of a file is read at a time, which is how often a copy looks at whether it was called
/// off and says how far it has got.
const COPY_CHUNK_BYTES: usize = 256 * 1024;

/// How much one export may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExportLimits {
    pub(crate) bytes: u64,
    pub(crate) files: usize,
}

impl Default for ExportLimits {
    fn default() -> Self {
        ExportLimits { bytes: EXPORT_MAX_BYTES, files: EXPORT_MAX_FILES }
    }
}

/// One export to make.
#[derive(Debug, Clone)]
pub(crate) struct ExportRequest {
    /// The pack to copy.
    pub(crate) pack: PathBuf,
    /// The folder the exported folder is made in.
    pub(crate) destination: PathBuf,
    /// The text of the adjustments file written into the copy.
    pub(crate) adjustments: String,
    pub(crate) limits: ExportLimits,
}

/// How far an export has got, read by the screen while the copy runs, and where the screen says
/// that the export is called off.
#[derive(Debug, Default)]
pub(crate) struct ExportProgress {
    files: AtomicUsize,
    files_done: AtomicUsize,
    bytes: AtomicU64,
    bytes_done: AtomicU64,
    called_off: AtomicBool,
}

/// How far an export has got, at one moment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ExportStanding {
    /// How many files there are to write, or nothing while the pack is still being listed.
    pub(crate) files: usize,
    pub(crate) files_done: usize,
    pub(crate) bytes: u64,
    pub(crate) bytes_done: u64,
}

impl ExportProgress {
    pub(crate) fn standing(&self) -> ExportStanding {
        ExportStanding {
            files: self.files.load(Ordering::Relaxed),
            files_done: self.files_done.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            bytes_done: self.bytes_done.load(Ordering::Relaxed),
        }
    }

    /// Call the export off. The copy stops at the next piece of a file and takes away what it wrote.
    pub(crate) fn call_off(&self) {
        self.called_off.store(true, Ordering::Relaxed);
    }

    /// Whether the export has been called off.
    pub(crate) fn is_called_off(&self) -> bool {
        self.called_off.load(Ordering::Relaxed)
    }
}

/// An export that was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exported {
    /// The folder that was written, under its final name.
    pub(crate) folder: PathBuf,
    /// How many files are in it, the adjustments file among them.
    pub(crate) files: usize,
    pub(crate) bytes: u64,
    /// How many links of the pack were passed over because they lead out of it, or to a folder.
    pub(crate) links_skipped: usize,
}

/// Why an export was not made. Nothing is left under a final name by any of these.
#[derive(Debug)]
pub(crate) enum ExportError {
    /// The pack's folder is not there to be read.
    NoPack(PathBuf),
    /// The folder to export into is not a folder that can be written into.
    NoDestination(PathBuf),
    /// The folder to export into is the pack's own or one inside it: the copy would be made of
    /// itself.
    InsidePack,
    /// The pack holds more than one export writes.
    TooLarge {
        bytes: u64,
        limit: u64,
    },
    TooManyFiles {
        limit: usize,
    },
    /// A file of the pack is further below its folder than an export follows.
    TooDeep(PathBuf),
    /// Every name the folder could be given is taken.
    NoFreeName(PathBuf),
    /// The player called it off.
    CalledOff,
    /// A file could not be read or written.
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::NoPack(pack) => write!(f, "the pack folder {} cannot be read", pack.display()),
            ExportError::NoDestination(folder) => write!(f, "{} is not a folder that can be exported into", folder.display()),
            ExportError::InsidePack => write!(f, "the folder chosen is the pack's own or inside it - choose a folder outside the pack"),
            ExportError::TooLarge { bytes, limit } => {
                write!(f, "the pack is {} MB, and an export writes at most {} MB", megabytes(*bytes), megabytes(*limit))
            }
            ExportError::TooManyFiles { limit } => write!(f, "the pack holds more than {limit} files"),
            ExportError::TooDeep(path) => write!(f, "{} is more than {EXPORT_MAX_DEPTH} folders below the pack", path.display()),
            ExportError::NoFreeName(folder) => write!(f, "every name up to {} is taken", folder.display()),
            ExportError::CalledOff => write!(f, "cancelled - nothing was left behind"),
            ExportError::Io { path, error } => write!(f, "{}: {error}", path.display()),
        }
    }
}

/// How many bytes a megabyte is, as sizes are said on screen.
const BYTES_PER_MEGABYTE: u64 = 1024 * 1024;

/// A count of bytes in whole megabytes, rounded up so that anything at all is not said to be none.
pub(crate) fn megabytes(bytes: u64) -> u64 {
    bytes.div_ceil(BYTES_PER_MEGABYTE)
}

/// One file to write: where it is read from, and its path inside the pack.
struct PlannedFile {
    source: PathBuf,
    inside: PathBuf,
    bytes: u64,
}

/// Everything one export writes.
#[derive(Default)]
struct Plan {
    /// The folders of the pack, each before anything inside it.
    folders: Vec<PathBuf>,
    files: Vec<PlannedFile>,
    bytes: u64,
    links_skipped: usize,
}

/// Make one export, here and now, and answer what was written.
///
/// The pack is listed whole before anything is written, so a pack too large is refused with the
/// disk untouched. `progress` is told how much there is and how far the copy has got, and is where
/// the export is called off from.
pub(crate) fn export_pack(request: &ExportRequest, progress: &ExportProgress) -> Result<Exported, ExportError> {
    let pack = request.pack.canonicalize().ok().filter(|pack| pack.is_dir()).ok_or_else(|| ExportError::NoPack(request.pack.clone()))?;
    let no_destination = || ExportError::NoDestination(request.destination.clone());
    let destination = request.destination.canonicalize().ok().filter(|folder| folder.is_dir()).ok_or_else(no_destination)?;
    if destination.starts_with(&pack) {
        return Err(ExportError::InsidePack);
    }
    let plan = list_pack(&pack, request, progress)?;
    progress.files.store(plan.files.len() + 1, Ordering::Relaxed);
    progress.bytes.store(plan.bytes, Ordering::Relaxed);

    let name = request.pack.file_name().or_else(|| pack.file_name()).map_or_else(|| UNNAMED_PACK.to_owned(), |name| name.to_string_lossy().into_owned());
    let (partial, whole) = claim_folder(&destination, &name)?;
    let written = write_plan(&plan, &partial, &request.adjustments, progress)
        .and_then(|()| std::fs::rename(&partial, &whole).map_err(|error| ExportError::Io { path: whole.clone(), error }));
    if let Err(error) = written {
        let _ = std::fs::remove_dir_all(&partial);
        return Err(error);
    }
    Ok(Exported { folder: whole, files: plan.files.len() + 1, bytes: plan.bytes, links_skipped: plan.links_skipped })
}

/// List what an export of the pack in `pack` writes, which is a canonical path, and refuse a pack
/// that holds more than `request` allows.
///
/// A folder is listed in the order of its entries' names, so two exports of one pack write the same
/// files in the same order.
fn list_pack(pack: &Path, request: &ExportRequest, progress: &ExportProgress) -> Result<Plan, ExportError> {
    let limits = request.limits;
    let mut plan = Plan { bytes: request.adjustments.len() as u64, ..Plan::default() };
    let mut pending = vec![(PathBuf::new(), 0usize)];
    while let Some((inside, depth)) = pending.pop() {
        if progress.is_called_off() {
            return Err(ExportError::CalledOff);
        }
        let folder = pack.join(&inside);
        let read = |error| ExportError::Io { path: folder.clone(), error };
        let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(&folder).map_err(read)?.collect::<Result<_, _>>().map_err(read)?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name();
            if is_left_out(&name.to_string_lossy(), depth) {
                continue;
            }
            let below = inside.join(&name);
            let found = std::fs::symlink_metadata(entry.path()).map_err(|error| ExportError::Io { path: entry.path(), error })?;
            let kind = found.file_type();
            let file = if kind.is_dir() {
                if depth + 1 >= EXPORT_MAX_DEPTH {
                    return Err(ExportError::TooDeep(below));
                }
                plan.folders.push(below.clone());
                pending.push((below, depth + 1));
                continue;
            } else if kind.is_file() {
                Some((entry.path(), found.len()))
            } else if kind.is_symlink() {
                let led_to = linked_file(pack, &entry.path());
                plan.links_skipped += usize::from(led_to.is_none());
                led_to
            } else {
                None
            };
            let Some((source, bytes)) = file else {
                continue;
            };
            plan.bytes += bytes;
            plan.files.push(PlannedFile { source, inside: below, bytes });
            if plan.files.len() >= limits.files {
                return Err(ExportError::TooManyFiles { limit: limits.files });
            }
            if plan.bytes > limits.bytes {
                return Err(ExportError::TooLarge { bytes: plan.bytes, limit: limits.bytes });
            }
        }
    }
    plan.folders.sort();
    Ok(plan)
}

/// Whether an entry of a pack is not part of what is exported: something hidden, wherever it is,
/// and directly in the pack's folder the installer's record and the adjustments the pack came with.
fn is_left_out(name: &str, depth: usize) -> bool {
    name.starts_with(HIDDEN_PREFIX) || (depth == 0 && (name == MANIFEST_FILE || name == ADJUSTMENTS_FILE))
}

/// The regular file a link of the pack leads to, with its length, when that file is inside the
/// pack. A link that leads out of the pack, to a folder, or nowhere is answered with `None`.
fn linked_file(pack: &Path, link: &Path) -> Option<(PathBuf, u64)> {
    let target = link.canonicalize().ok()?;
    let found = std::fs::metadata(&target).ok()?;
    let is_inside = target.starts_with(pack) && found.is_file();
    is_inside.then_some((target, found.len()))
}

/// Make the folder an export is written into, under a name nothing has: the pack's name with
/// [`EXPORT_SUFFIX`], and with a number after it when that is taken. Answers the folder as it is
/// named while it is written, and the name it is given once it is whole.
///
/// Making the folder is what claims the name: one that is already there, whole or not, is never
/// written into, and two exports begun at once cannot take the same one.
fn claim_folder(destination: &Path, pack_name: &str) -> Result<(PathBuf, PathBuf), ExportError> {
    let numbered = std::iter::once(String::new()).chain((FIRST_NUMBERED..=LAST_NUMBERED).map(|number| format!("-{number}")));
    for number in numbered {
        let whole = destination.join(format!("{pack_name}{EXPORT_SUFFIX}{number}"));
        let partial = destination.join(format!("{pack_name}{EXPORT_SUFFIX}{number}{INCOMPLETE_SUFFIX}"));
        if std::fs::symlink_metadata(&whole).is_ok() {
            continue;
        }
        match std::fs::create_dir(&partial) {
            Ok(()) => return Ok((partial, whole)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(ExportError::Io { path: partial, error }),
        }
    }
    Err(ExportError::NoFreeName(destination.join(format!("{pack_name}{EXPORT_SUFFIX}-{LAST_NUMBERED}"))))
}

/// Write everything in `plan` into `folder`, and the adjustments file last.
fn write_plan(plan: &Plan, folder: &Path, adjustments: &str, progress: &ExportProgress) -> Result<(), ExportError> {
    for inside in &plan.folders {
        let path = folder.join(inside);
        std::fs::create_dir_all(&path).map_err(|error| ExportError::Io { path, error })?;
    }
    for file in &plan.files {
        let target = folder.join(&file.inside);
        let before = progress.bytes_done.load(Ordering::Relaxed);
        copy_file(&file.source, &target, progress)?;
        progress.bytes_done.store(before + file.bytes, Ordering::Relaxed);
        progress.files_done.fetch_add(1, Ordering::Relaxed);
    }
    let path = folder.join(ADJUSTMENTS_FILE);
    std::fs::write(&path, adjustments).map_err(|error| ExportError::Io { path, error })?;
    progress.bytes_done.fetch_add(adjustments.len() as u64, Ordering::Relaxed);
    progress.files_done.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

/// Copy one file a piece at a time, saying how far it has got and stopping when the export is
/// called off. The target must not be there: an export writes into a folder of its own.
fn copy_file(source: &Path, target: &Path, progress: &ExportProgress) -> Result<(), ExportError> {
    let reading = |error| ExportError::Io { path: source.to_path_buf(), error };
    let writing = |error| ExportError::Io { path: target.to_path_buf(), error };
    let mut from = std::fs::File::open(source).map_err(reading)?;
    let mut to = std::fs::OpenOptions::new().write(true).create_new(true).open(target).map_err(writing)?;
    let mut piece = vec![0u8; COPY_CHUNK_BYTES];
    loop {
        if progress.is_called_off() {
            return Err(ExportError::CalledOff);
        }
        let read = from.read(&mut piece).map_err(reading)?;
        if read == 0 {
            return Ok(());
        }
        to.write_all(&piece[..read]).map_err(writing)?;
        progress.bytes_done.fetch_add(read as u64, Ordering::Relaxed);
    }
}

/// An export running beside the frame loop.
///
/// Held by the screen that began it and asked once a frame. Dropping it does not stop the copy: a
/// screen that is left calls it off first.
pub(crate) struct ExportJob {
    progress: Arc<ExportProgress>,
    outcome: Receiver<Result<Exported, ExportError>>,
}

impl ExportJob {
    /// Begin an export on a thread of its own.
    pub(crate) fn begin(request: ExportRequest) -> ExportJob {
        let progress = Arc::new(ExportProgress::default());
        let (tx, outcome) = std::sync::mpsc::channel();
        let told = Arc::clone(&progress);
        std::thread::spawn(move || {
            let _ = tx.send(export_pack(&request, &told));
        });
        ExportJob { progress, outcome }
    }

    /// What the export came to, once it has ended, and `None` while it runs. Never blocks.
    pub(crate) fn poll(&self) -> Option<Result<Exported, ExportError>> {
        match self.outcome.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(ExportError::CalledOff)),
        }
    }

    /// How far the export has got.
    pub(crate) fn standing(&self) -> ExportStanding {
        self.progress.standing()
    }

    /// Call the export off. What it came to is still collected with [`ExportJob::poll`].
    pub(crate) fn call_off(&self) {
        self.progress.call_off();
    }

    /// Wait for the export to end, which a test does in place of drawing frames until it has.
    #[cfg(test)]
    pub(crate) fn wait(self) -> Result<Exported, ExportError> {
        self.outcome.recv().unwrap_or(Err(ExportError::CalledOff))
    }

    /// Whether the export has been called off.
    #[cfg(test)]
    pub(crate) fn is_called_off(&self) -> bool {
        self.progress.is_called_off()
    }

    /// Where the export says how far it has got, for a test that outlives the job.
    #[cfg(test)]
    pub(crate) fn progress(&self) -> Arc<ExportProgress> {
        Arc::clone(&self.progress)
    }
}

/// A job with no copy behind it: it stands where `standing` says, and comes to whatever the test
/// sends, so a screen's waiting can be drawn and checked without a pack large enough to wait for.
#[cfg(test)]
pub(crate) fn job_for_tests(standing: ExportStanding) -> (std::sync::mpsc::Sender<Result<Exported, ExportError>>, ExportJob) {
    let progress = ExportProgress::default();
    progress.files.store(standing.files, Ordering::Relaxed);
    progress.files_done.store(standing.files_done, Ordering::Relaxed);
    progress.bytes.store(standing.bytes, Ordering::Relaxed);
    progress.bytes_done.store(standing.bytes_done, Ordering::Relaxed);
    let (tx, outcome) = std::sync::mpsc::channel();
    (tx, ExportJob { progress: Arc::new(progress), outcome })
}
