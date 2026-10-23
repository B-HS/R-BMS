use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use zip::ZipArchive;
use zip::result::ZipError;

use crate::error::DownloadError;
use crate::name::entry_path;
use crate::{BYTES_PER_MIB, GIB};

/// The most entries an archive may hold.
pub const EXTRACT_MAX_ENTRIES: usize = 10_000;

/// The most bytes one file in an archive may unpack to.
pub const EXTRACT_MAX_ENTRY_BYTES: u64 = GIB;

/// The most bytes an archive may unpack to in all.
pub const EXTRACT_MAX_TOTAL_BYTES: u64 = 2 * GIB;

const COPY_CHUNK_BYTES: usize = 64 * 1024;

/// What the list of entries of an archive is allowed to cost per entry that may be in it: the fixed
/// part of an entry's record and a name much longer than the ones a chart pack has.
const INDEX_BYTES_PER_ENTRY: u64 = 1024;

/// What finding the list of entries costs besides the list: the search for the end of the archive.
const INDEX_FIXED_BYTES: u64 = BYTES_PER_MIB;

const LANGUAGE_ENCODING_FLAG: u16 = 1 << 11;

const CHART_EXTENSIONS: [&str; 5] = ["bms", "bme", "bml", "pms", "bmson"];
const COMPANION_EXTENSIONS: [&str; 18] =
    ["wav", "ogg", "mp3", "flac", "bmp", "png", "jpg", "jpeg", "gif", "mpg", "mpeg", "avi", "mp4", "wmv", "m4v", "webm", "mkv", "txt"];

/// What bounds the unpacking of an archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtractLimits {
    /// The most entries the archive may hold.
    pub max_entries: usize,
    /// The most bytes one file may unpack to.
    pub max_entry_bytes: u64,
    /// The most bytes all the files may unpack to together.
    pub max_total_bytes: u64,
}

impl Default for ExtractLimits {
    fn default() -> ExtractLimits {
        ExtractLimits { max_entries: EXTRACT_MAX_ENTRIES, max_entry_bytes: EXTRACT_MAX_ENTRY_BYTES, max_total_bytes: EXTRACT_MAX_TOTAL_BYTES }
    }
}

/// What unpacking an archive came to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExtractReport {
    /// Files written.
    pub written: usize,
    /// Entries not written: a kind of file that is not kept, a link, a name that could not be a path
    /// on every platform, a name that is taken. Folders are neither written nor skipped.
    pub skipped: usize,
    /// Of the files written, how many are charts.
    pub charts: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Plan {
    Directory,
    Skip,
    Write { relative: PathBuf, is_chart: bool },
}

/// The name of an entry as text. The archive says whether the name is UTF-8; when it does not and
/// the bytes are not UTF-8 either, they are the Shift_JIS that a Japanese archiver writes, and read
/// as anything else they would no longer match the files a chart names.
fn entry_name(raw: &[u8], is_flagged_utf8: bool) -> String {
    if let Ok(text) = std::str::from_utf8(raw) {
        return text.to_string();
    }
    if !is_flagged_utf8 && let Some(text) = encoding_rs::SHIFT_JIS.decode_without_bom_handling_and_without_replacement(raw) {
        return text.into_owned();
    }
    String::from_utf8_lossy(raw).into_owned()
}

/// Whether files with this extension, in any case, are what a chart pack is made of, and so are the
/// ones that are unpacked.
pub fn is_kept_extension(extension: &str) -> bool {
    let lower = extension.to_ascii_lowercase();
    CHART_EXTENSIONS.contains(&lower.as_str()) || COMPANION_EXTENSIONS.contains(&lower.as_str())
}

/// What to do with an entry, judged from what the archive says of it and nothing else.
pub(crate) fn plan_entry(raw_name: &[u8], is_flagged_utf8: bool, is_directory: bool, is_link: bool) -> Plan {
    if is_directory {
        return Plan::Directory;
    }
    if is_link {
        return Plan::Skip;
    }
    let Some(relative) = entry_path(&entry_name(raw_name, is_flagged_utf8)) else {
        return Plan::Skip;
    };
    let extension = relative.extension().and_then(|extension| extension.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    let is_chart = CHART_EXTENSIONS.contains(&extension.as_str());
    if is_kept_extension(&extension) { Plan::Write { relative, is_chart } } else { Plan::Skip }
}

/// A reader that gives up once it has handed out its budget, shared so that the budget can be lifted
/// after the archive is open.
struct Budgeted<R> {
    inner: R,
    left: Arc<AtomicU64>,
}

impl<R: Read> Read for Budgeted<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.left.load(Ordering::Relaxed);
        if left == 0 {
            return Err(io::Error::other("the budget for reading the archive's list of entries is spent"));
        }
        let allowed = usize::try_from(left).map_or(buf.len(), |left| left.min(buf.len()));
        let read = self.inner.read(&mut buf[..allowed])?;
        self.left.fetch_sub(read as u64, Ordering::Relaxed);
        Ok(read)
    }
}

impl<R: Seek> Seek for Budgeted<R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

/// Open the archive at `path`, reading at most `index_budget_bytes` to find its list of entries.
///
/// The list is held in memory as it is read, and it is as long as the archive says, so an archive
/// that lists millions of empty files would fill memory before any limit on entries was looked at.
pub(crate) fn open_archive(path: &Path, index_budget_bytes: u64) -> Result<ZipArchive<impl Read + Seek>, DownloadError> {
    let left = Arc::new(AtomicU64::new(index_budget_bytes));
    let reader = Budgeted { inner: File::open(path)?, left: Arc::clone(&left) };
    match ZipArchive::new(reader) {
        Ok(archive) => {
            left.store(u64::MAX, Ordering::Relaxed);
            Ok(archive)
        }
        Err(_) if left.load(Ordering::Relaxed) == 0 => Err(DownloadError::IndexTooLarge { limit: index_budget_bytes }),
        Err(error) => Err(DownloadError::Archive(error)),
    }
}

fn prepare_staging(into: &Path) -> Result<(), DownloadError> {
    match fs::read_dir(into) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(DownloadError::StagingNotEmpty(into.to_path_buf()));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    fs::create_dir_all(into)?;
    Ok(())
}

/// Copy at most `allowed` bytes of `from` to `to`, and one more if there is one, so that the caller
/// can tell an entry that ends at the limit from one that goes past it. Returns the bytes written.
fn copy_up_to(from: &mut impl Read, to: &mut File, allowed: u64, cancel: &AtomicBool, chunk: &mut [u8]) -> Result<u64, DownloadError> {
    let mut limited = from.take(allowed.saturating_add(1));
    let mut written = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        let read = match limited.read(chunk) {
            Ok(0) => return Ok(written),
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(DownloadError::Archive(ZipError::Io(error))),
        };
        to.write_all(&chunk[..read])?;
        written += read as u64;
    }
}

/// Whether writing `relative` under `into` would meet something already there: the name itself, or a file where one of its
/// folders should be. Asking the file system up front gives the same answer everywhere, where the error a refused create
/// returns differs (Windows reports a folder in the way as access denied, not as an existing name).
fn is_taken(into: &Path, relative: &Path) -> bool {
    let mut at = into.to_path_buf();
    let mut parts = relative.components().peekable();
    while let Some(part) = parts.next() {
        at.push(part);
        let Ok(found) = fs::symlink_metadata(&at) else {
            return false;
        };
        if parts.peek().is_none() || !found.is_dir() {
            return true;
        }
    }
    false
}

fn unpack(archive: &mut ZipArchive<impl Read + Seek>, into: &Path, limits: &ExtractLimits, cancel: &AtomicBool) -> Result<ExtractReport, DownloadError> {
    let mut report = ExtractReport::default();
    let mut unpacked_bytes = 0u64;
    let mut chunk = vec![0; COPY_CHUNK_BYTES];
    for index in 0..archive.len() {
        if cancel.load(Ordering::Relaxed) {
            return Err(DownloadError::Cancelled);
        }
        let plan = {
            let entry = archive.by_index_data(index).map_err(DownloadError::Archive)?;
            let is_flagged_utf8 = entry.flags().as_u16() & LANGUAGE_ENCODING_FLAG != 0;
            plan_entry(entry.name_raw(), is_flagged_utf8, entry.is_dir(), entry.is_symlink())
        };
        let (relative, is_chart) = match plan {
            Plan::Directory => continue,
            Plan::Skip => {
                report.skipped += 1;
                continue;
            }
            Plan::Write { relative, is_chart } => (relative, is_chart),
        };
        if is_taken(into, &relative) {
            report.skipped += 1;
            continue;
        }
        let target = into.join(&relative);
        let mut entry = archive.by_index(index).map_err(DownloadError::Archive)?;
        let created = fs::create_dir_all(target.parent().unwrap_or(into)).and_then(|()| OpenOptions::new().write(true).create_new(true).open(&target));
        let mut out = match created {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                report.skipped += 1;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let allowed = limits.max_entry_bytes.min(limits.max_total_bytes.saturating_sub(unpacked_bytes));
        let written = copy_up_to(&mut entry, &mut out, allowed, cancel, &mut chunk)?;
        if written > limits.max_entry_bytes {
            return Err(DownloadError::EntryTooLarge { name: relative.display().to_string(), limit: limits.max_entry_bytes });
        }
        if written > allowed {
            return Err(DownloadError::UnpackedTooLarge { limit: limits.max_total_bytes });
        }
        unpacked_bytes += written;
        report.written += 1;
        report.charts += usize::from(is_chart);
    }
    Ok(report)
}

/// Unpack the zip archive `archive` into the folder `into`, which does not exist yet or is empty.
///
/// Only what a chart pack is made of is written: charts, sounds, pictures, movies and text. Every
/// entry is placed by a path that stays inside `into` on every platform, and nothing that is there
/// is overwritten. How much is written is counted from the bytes that come out, not from the sizes
/// the archive states, and the unpacking stops at the first limit it passes.
///
/// When it does not succeed, or when the archive holds no chart, `into` is removed.
pub fn extract_zip(archive: &Path, into: &Path, limits: &ExtractLimits, cancel: &AtomicBool) -> Result<ExtractReport, DownloadError> {
    let index_budget_bytes = (limits.max_entries as u64).saturating_mul(INDEX_BYTES_PER_ENTRY).saturating_add(INDEX_FIXED_BYTES);
    let mut opened = open_archive(archive, index_budget_bytes)?;
    if opened.len() > limits.max_entries {
        return Err(DownloadError::TooManyEntries { limit: limits.max_entries });
    }
    prepare_staging(into)?;
    let result = unpack(&mut opened, into, limits, cancel).and_then(|report| if report.charts == 0 { Err(DownloadError::NoChart) } else { Ok(report) });
    if result.is_err() {
        let _ = fs::remove_dir_all(into);
    }
    result
}
