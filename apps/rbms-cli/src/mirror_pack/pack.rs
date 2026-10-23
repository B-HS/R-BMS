//! Writing one song folder as a zip archive that is the same bytes whenever the folder is the same.
//!
//! Nothing in the archive depends on when or where it was made: the entries are in the byte order
//! of their names, each carries the same fixed time, permissions and system, and there are no
//! comments and no extra fields. The archive is named by the SHA-256 of its bytes, so an archive
//! that is already there under its name is the archive that would be written.

use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, System, ZipWriter};

use super::folders::PackEntry;

/// The extension of a pack.
pub const PACK_EXTENSION: &str = "zip";

/// Kinds of file that are compressed already, which deflating would only make slower.
const STORED_EXTENSIONS: [&str; 14] = ["ogg", "mp3", "flac", "mp4", "m4a", "mpg", "mpeg", "wmv", "avi", "webm", "png", "jpg", "jpeg", "zip"];

/// How hard the rest is deflated.
const DEFLATE_LEVEL: i64 = 6;

/// The permissions every entry carries, whatever the file had.
const PACK_FILE_MODE: u32 = 0o644;

const COPY_CHUNK_BYTES: usize = 64 * 1024;

/// What a pack is called while it is being written, after the process and the pack's turn in it.
const TEMPORARY_SUFFIX: &str = "zip.tmp";

static TEMPORARY_SERIAL: AtomicU64 = AtomicU64::new(0);

/// A pack in the folder of packs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenPack {
    pub sha256: String,
    /// The pack's size on disk.
    pub bytes: u64,
    pub path: PathBuf,
    /// Whether the pack was there already, so nothing was written.
    pub reused: bool,
}

/// Why a pack was not written.
#[derive(Debug, PartialEq, Eq)]
pub enum WriteError {
    /// A file that was to go in could not be read. The run goes on without this folder.
    Source { name: String, reason: String },
    /// The archive could not be written. The run cannot go on.
    Output(String),
}

/// The path of the pack whose bytes have this SHA-256.
pub fn pack_path(packs_dir: &Path, sha256: &str) -> PathBuf {
    packs_dir.join(format!("{sha256}.{PACK_EXTENSION}"))
}

/// A digest of what a folder would pack: the names, sizes and times of its files. When it is the
/// same as one an earlier run kept, the folder has not changed and need not be packed again.
pub fn fingerprint(entries: &[PackEntry]) -> String {
    let mut hasher = Sha256::new();
    for entry in entries {
        hasher.update(format!("{}\t{}\t{}\n", entry.name, entry.bytes, entry.modified_nanos));
    }
    format!("{:x}", hasher.finalize())
}

fn method_of(name: &str) -> CompressionMethod {
    let extension = Path::new(name).extension().and_then(|extension| extension.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    if STORED_EXTENSIONS.contains(&extension.as_str()) { CompressionMethod::Stored } else { CompressionMethod::Deflated }
}

fn options_of(name: &str) -> SimpleFileOptions {
    let method = method_of(name);
    let options =
        SimpleFileOptions::default().compression_method(method).last_modified_time(DateTime::default()).system(System::Unix).unix_permissions(PACK_FILE_MODE);
    if method == CompressionMethod::Deflated { options.compression_level(Some(DEFLATE_LEVEL)) } else { options }
}

fn output_error(what: &str, e: impl std::fmt::Display) -> WriteError {
    WriteError::Output(format!("{what}: {e}"))
}

fn write_entries(entries: &[PackEntry], file: File) -> Result<(), WriteError> {
    let mut zip = ZipWriter::new(BufWriter::new(file));
    let mut chunk = vec![0; COPY_CHUNK_BYTES];
    for entry in entries {
        let source_error = |e: io::Error| WriteError::Source { name: entry.name.clone(), reason: e.to_string() };
        let mut source = File::open(&entry.source).map_err(source_error)?;
        zip.start_file(&entry.name, options_of(&entry.name)).map_err(|e| output_error(&format!("{} not started", entry.name), e))?;
        loop {
            let read = match source.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => read,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(source_error(e)),
            };
            zip.write_all(&chunk[..read]).map_err(|e| output_error(&format!("{} not written", entry.name), e))?;
        }
    }
    let mut finished = zip.finish().map_err(|e| output_error("archive not finished", e))?;
    finished.flush().map_err(|e| output_error("archive not flushed", e))
}

fn sha256_of_file(path: &Path) -> io::Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut chunk = vec![0; COPY_CHUNK_BYTES];
    let mut total = 0u64;
    loop {
        let read = match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        hasher.update(&chunk[..read]);
        total += read as u64;
    }
    Ok((format!("{:x}", hasher.finalize()), total))
}

/// A file that is removed when it goes out of scope, which leaves nothing behind whatever way a write ends.
struct Temporary(PathBuf);

impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Writes `entries` (in the order they are given) as a zip archive in `packs_dir`, under the name
/// of its SHA-256. The archive is written to a temporary file and renamed, so a run that is
/// stopped halfway leaves no half-written pack under a pack's name. When a pack of that name and
/// size is there already it is left alone.
pub fn write_pack(entries: &[PackEntry], packs_dir: &Path) -> Result<WrittenPack, WriteError> {
    fs::create_dir_all(packs_dir).map_err(|e| output_error(&format!("{} not created", packs_dir.display()), e))?;
    let serial = TEMPORARY_SERIAL.fetch_add(1, Ordering::Relaxed);
    let temporary = Temporary(packs_dir.join(format!(".{}-{serial}.{TEMPORARY_SUFFIX}", std::process::id())));
    let file = File::create(&temporary.0).map_err(|e| output_error(&format!("{} not created", temporary.0.display()), e))?;
    write_entries(entries, file)?;
    let (sha256, bytes) = sha256_of_file(&temporary.0).map_err(|e| output_error(&format!("{} not read back", temporary.0.display()), e))?;
    let path = pack_path(packs_dir, &sha256);
    if fs::metadata(&path).is_ok_and(|existing| existing.len() == bytes) {
        return Ok(WrittenPack { sha256, bytes, path, reused: true });
    }
    fs::rename(&temporary.0, &path).map_err(|e| output_error(&format!("{} not written", path.display()), e))?;
    Ok(WrittenPack { sha256, bytes, path, reused: false })
}
