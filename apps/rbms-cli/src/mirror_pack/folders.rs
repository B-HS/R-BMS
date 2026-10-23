//! Finding the song folders of a library and listing what each would pack.
//!
//! A song folder is a folder that holds a chart directly. What is packed of it is every file below
//! it except the files that are only the file system's, the folders that are song folders of their
//! own, and anything that is a link.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use md5::Md5;
use rbms_download::{ExtractLimits, is_kept_extension};
use sha2::{Digest, Sha256};

/// File names a file system or a file manager puts in a folder on its own.
const SKIPPED_FILE_NAMES: [&str; 3] = [".DS_Store", "Thumbs.db", "desktop.ini"];

/// What the name of a file begins with when it is the resource-fork stand-in macOS writes beside a file.
const SKIPPED_FILE_PREFIX: &str = "._";

/// Folder names that hold nothing but such stand-ins.
const SKIPPED_FOLDER_NAMES: [&str; 1] = ["__MACOSX"];

/// What the folder of a library that is itself a song folder is called in the manifest.
const LIBRARY_ROOT_RELATIVE: &str = ".";

/// What the pack of a library that is itself a song folder is named when the library's folder has no name.
const LIBRARY_ROOT_FALLBACK_TOP: &str = "library";

/// A file or folder that could not be read, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub path: PathBuf,
    pub reason: String,
}

/// A chart file of a song folder, with the hashes the player and the website know it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartFile {
    pub path: PathBuf,
    /// The MD5 of the file's bytes, in lower case.
    pub md5: String,
    /// The SHA-256 of the file's bytes, in lower case.
    pub sha256: String,
}

/// A folder that holds at least one chart directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongFolder {
    pub root: PathBuf,
    /// The path from the library folder, `/`-separated; [`LIBRARY_ROOT_RELATIVE`] for the library folder itself.
    pub relative: String,
    /// The folder's own name, which the pack's entries are all under.
    pub top: String,
    /// The charts that could be read, in path order.
    pub charts: Vec<ChartFile>,
}

/// The song folders under a folder and what could not be listed or read.
pub struct Walked {
    /// In the byte order of [`SongFolder::relative`], so that the first of two folders with one md5 is the same on every run.
    pub folders: Vec<SongFolder>,
    pub failures: Vec<Failure>,
}

/// Why a song folder is not packed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The files together are more than the player unpacks.
    TooLarge { uncompressed_bytes: u64 },
    /// One file is more than the player unpacks of a single file.
    EntryTooLarge { name: String, bytes: u64 },
    /// More files than the player unpacks.
    TooManyEntries { entries: usize },
    /// A name that cannot be written into an archive as text. The path is from the song folder.
    NonUtf8Name { path: PathBuf },
    /// A folder or file that could not be read. The path is from the song folder.
    Unreadable { path: PathBuf, reason: String },
}

impl Refusal {
    /// The reason as the manifest names it.
    pub fn code(&self) -> &'static str {
        match self {
            Refusal::TooLarge { .. } => "too_large",
            Refusal::EntryTooLarge { .. } => "entry_too_large",
            Refusal::TooManyEntries { .. } => "too_many_entries",
            Refusal::NonUtf8Name { .. } => "non_utf8_name",
            Refusal::Unreadable { .. } => "unreadable",
        }
    }

    /// What the reason is about: a size, a count or a path below the song folder.
    pub fn detail(&self) -> String {
        match self {
            Refusal::TooLarge { uncompressed_bytes } => uncompressed_bytes.to_string(),
            Refusal::EntryTooLarge { name, bytes } => format!("{name}: {bytes}"),
            Refusal::TooManyEntries { entries } => entries.to_string(),
            Refusal::NonUtf8Name { path } => path.to_string_lossy().into_owned(),
            Refusal::Unreadable { path, reason } => format!("{}: {reason}", path.to_string_lossy()),
        }
    }
}

/// What a song folder may come to. The default is what the player's unpacker allows, since a pack
/// the player would refuse to unpack is no use to anyone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackLimits {
    /// The most bytes all the files may come to.
    pub max_uncompressed_bytes: u64,
    /// The most bytes one file may come to.
    pub max_entry_bytes: u64,
    /// The most files.
    pub max_entries: usize,
}

impl Default for PackLimits {
    fn default() -> PackLimits {
        let client = ExtractLimits::default();
        PackLimits { max_uncompressed_bytes: client.max_total_bytes, max_entry_bytes: client.max_entry_bytes, max_entries: client.max_entries }
    }
}

/// One file of a pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackEntry {
    pub source: PathBuf,
    /// The name in the archive: the song folder's name, then the path below it, separated by `/`.
    pub name: String,
    pub bytes: u64,
    /// When the file was last changed, in nanoseconds since the epoch; 0 when the file system does not say.
    pub modified_nanos: u128,
}

/// What a song folder would pack, in the byte order of the entries' names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gathered {
    pub entries: Vec<PackEntry>,
    /// Links met and left out.
    pub symlinks: usize,
}

fn is_left_out(name: &str, is_dir: bool) -> bool {
    if is_dir { SKIPPED_FOLDER_NAMES.contains(&name) } else { SKIPPED_FILE_NAMES.contains(&name) || name.starts_with(SKIPPED_FILE_PREFIX) }
}

fn hash_chart(path: &Path) -> Result<ChartFile, String> {
    let bytes = fs::read(path).map_err(|e| format!("read error: {e}"))?;
    Ok(ChartFile { path: path.to_path_buf(), md5: format!("{:x}", Md5::digest(&bytes)), sha256: format!("{:x}", Sha256::digest(&bytes)) })
}

fn song_folder(library: &Path, dir: &Path, charts: Vec<ChartFile>) -> SongFolder {
    let own_name = dir.file_name().map(|name| name.to_string_lossy().into_owned());
    let below_library: Vec<String> =
        dir.strip_prefix(library).map_or_else(|_| Vec::new(), |path| path.iter().map(|part| part.to_string_lossy().into_owned()).collect());
    let relative = if below_library.is_empty() { LIBRARY_ROOT_RELATIVE.to_string() } else { below_library.join("/") };
    SongFolder { root: dir.to_path_buf(), relative, top: own_name.unwrap_or_else(|| LIBRARY_ROOT_FALLBACK_TOP.to_string()), charts }
}

/// Whether `dir` holds a chart directly, as a song folder does.
fn holds_chart(dir: &Path) -> bool {
    let Ok(listing) = fs::read_dir(dir) else {
        return false;
    };
    listing.flatten().any(|entry| {
        entry.file_type().is_ok_and(|kind| kind.is_file()) && !is_left_out(&entry.file_name().to_string_lossy(), false) && rbms_library::is_chart(&entry.path())
    })
}

/// Walks `library` depth-first and collects the folders that hold a chart directly, each with the
/// hashes of its charts. Symbolic links are not followed, as the library scan does not follow them,
/// so a link pointing back up its own tree cannot make the walk run forever.
pub fn song_folders_under(library: &Path) -> Walked {
    let mut folders = Vec::new();
    let mut failures = Vec::new();
    let mut stack = vec![library.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let listing = match fs::read_dir(&dir) {
            Ok(listing) => listing,
            Err(e) => {
                failures.push(Failure { path: dir, reason: format!("folder not listed: {e}") });
                continue;
            }
        };
        let mut chart_paths = Vec::new();
        for entry in listing.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() || is_left_out(&entry.file_name().to_string_lossy(), kind.is_dir()) {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() && rbms_library::is_chart(&path) {
                chart_paths.push(path);
            }
        }
        if chart_paths.is_empty() {
            continue;
        }
        chart_paths.sort();
        let mut charts = Vec::new();
        for path in chart_paths {
            match hash_chart(&path) {
                Ok(chart) => charts.push(chart),
                Err(reason) => failures.push(Failure { path, reason }),
            }
        }
        folders.push(song_folder(library, &dir, charts));
    }
    folders.sort_by(|a, b| a.relative.cmp(&b.relative).then_with(|| a.root.cmp(&b.root)));
    failures.sort_by(|a, b| a.path.cmp(&b.path));
    Walked { folders, failures }
}

/// The path from `root` to `path`, or `path` itself when it is not below `root`.
fn below(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn modified_nanos(metadata: &fs::Metadata) -> u128 {
    metadata.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map_or(0, |since| since.as_nanos())
}

/// Lists what `folder` would pack and holds it to `limits`.
///
/// Folders below it that are song folders themselves are left out whole, as they are packed on
/// their own. Every file is held to the limits, whether or not the player would unpack it, so that
/// a pack never needs more than the plain archive format and a file of any kind that is too large to
/// unpack is told apart from a folder that merely has many files.
pub fn pack_entries(folder: &SongFolder, limits: &PackLimits) -> Result<Gathered, Refusal> {
    let unreadable = |path: &Path, e: &dyn std::fmt::Display| Refusal::Unreadable { path: below(&folder.root, path), reason: e.to_string() };
    if let Some(name) = folder.root.file_name().filter(|name| name.to_str().is_none()) {
        return Err(Refusal::NonUtf8Name { path: PathBuf::from(name) });
    }
    let mut entries = Vec::new();
    let mut symlinks = 0;
    let mut stack = vec![(folder.root.clone(), folder.top.clone())];
    while let Some((dir, prefix)) = stack.pop() {
        let listing = fs::read_dir(&dir).map_err(|e| unreadable(&dir, &e))?;
        for entry in listing {
            let entry = entry.map_err(|e| unreadable(&dir, &e))?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| unreadable(&path, &e))?;
            let file_name = entry.file_name();
            if is_left_out(&file_name.to_string_lossy(), kind.is_dir()) {
                continue;
            }
            if kind.is_symlink() {
                symlinks += 1;
                continue;
            }
            let Some(name) = file_name.to_str() else {
                return Err(Refusal::NonUtf8Name { path: below(&folder.root, &path) });
            };
            if kind.is_dir() {
                if !holds_chart(&path) {
                    stack.push((path, format!("{prefix}/{name}")));
                }
            } else if kind.is_file() {
                let metadata = entry.metadata().map_err(|e| unreadable(&path, &e))?;
                entries.push(PackEntry { name: format!("{prefix}/{name}"), bytes: metadata.len(), modified_nanos: modified_nanos(&metadata), source: path });
            }
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    if entries.len() > limits.max_entries {
        return Err(Refusal::TooManyEntries { entries: entries.len() });
    }
    if let Some(largest) = entries.iter().find(|entry| entry.bytes > limits.max_entry_bytes) {
        return Err(Refusal::EntryTooLarge { name: largest.name.clone(), bytes: largest.bytes });
    }
    let uncompressed_bytes = entries.iter().map(|entry| entry.bytes).sum::<u64>();
    if uncompressed_bytes > limits.max_uncompressed_bytes {
        return Err(Refusal::TooLarge { uncompressed_bytes });
    }
    Ok(Gathered { entries, symlinks })
}

/// How many of `entries` the player's unpacker leaves out for being a kind of file it does not keep.
pub fn skipped_by_client(entries: &[PackEntry]) -> usize {
    entries.iter().filter(|entry| !Path::new(&entry.name).extension().and_then(OsStr::to_str).is_some_and(is_kept_extension)).count()
}
