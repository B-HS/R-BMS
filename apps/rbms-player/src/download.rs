//! Getting a chart a difficulty table links to, and bringing it into the library.
//!
//! The song list asks for a chart the library does not hold from its download modal. One job runs
//! at a time, on a worker thread of its own: the folder in the download folder a job works in
//! ([`DOWNLOAD_WORK_FOLDER`], which no scan of the library walks into) is emptied, the zip archive
//! the chart's link names is fetched into it ([`rbms_download::download_to`]), unpacked beside it
//! ([`rbms_download::extract_zip`]) and moved to a folder of its own in the download folder that
//! takes nothing else's place ([`rbms_download::settle`]). An archive with no chart in it leaves
//! nothing behind.
//!
//! What a finished job unpacked is then scanned on another worker thread, and only that folder
//! ([`ScanRequest::partial`]): a scan of part of the library does not record the parser version, so
//! the full rescan another folder may still need is not taken as done. The scan goes through
//! [`crate::library::scan_now`], which holds [`crate::library::LIBRARY_WRITE`], so it never writes
//! the song database while a full scan does. The tables are matched again against the library it
//! read, and the song list takes the two in from its own update alone, keeping the folders it is in
//! and the bar under the cursor: the bar of a chart that was missing is that chart from then on. A
//! job that ends while a chart is being played waits for the song list.
//!
//! Each time a library is taken in, the library generation goes up, and a full scan's does too. A
//! scan that began under another generation read a database the full scan has changed since, so
//! what it read is not taken in and the same folders are scanned again, the chart waiting for them
//! with it. A table that arrived while the scan ran raised the table generation, and the copy of it
//! the scan matched may be older than the one the song list holds: that scan is run again too.
//!
//! The same scan of part of the library brings in the download folder itself when the settings
//! name another one, which the full scans at start-up and after the song folders change would
//! otherwise not see until then.
//!
//! The frame thread only starts the workers and reads what they report: nothing here reads the
//! disk or the network on it (`docs/acknowledge/2026-10-14-adr-play-hitch-criteria.md`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use rbms_config::TableSource;
use rbms_download::{DownloadError, ExtractLimits, LinkKind, Progress, classify_link, extract_zip, safe_folder_name, settle};
use rbms_library::Library;
use rbms_library::scan::{DOWNLOAD_WORK_FOLDER, ScanRequest};
use rbms_table::TableEntry;

use crate::AppShared;
use crate::notify::{Level, notify};
use crate::select_tree::{ListPlace, show_the_list_again};
use crate::stage::select::missing_chart_title;
use crate::tablesrc::{MatchedTable, TableStore, read_and_match_all};

#[cfg(test)]
pub(crate) mod tests;

/// The file the archive is fetched into, and the folder it is unpacked into, inside the folder in
/// the download folder a job works in ([`DOWNLOAD_WORK_FOLDER`]), which is emptied when a job
/// starts, removed when it ends and never read by a scan of the library. Only one job runs at a
/// time, so the names are fixed rather than taken from a table's entry.
const ARCHIVE_FILE: &str = "archive.zip";
const STAGING_FOLDER: &str = "unpacked";

/// Why a download could not be started.
pub(crate) const ANOTHER_DOWNLOAD_IS_RUNNING: &str = "another download is running";
const NOT_A_ZIP_LINK: &str = "the chart's link is not a zip archive";
const NO_DOWNLOAD_FOLDER: &str = "no download folder is chosen";

/// What a worker that stopped without a word is said to have done.
const JOB_STOPPED: &str = "the download stopped without reporting";
const SCAN_STOPPED: &str = "the library scan stopped without reporting";

/// The units the progress line gives sizes in.
const BYTES_PER_KB: u64 = 1024;
const BYTES_PER_MB: u64 = 1024 * BYTES_PER_KB;

/// How a job fetches an archive into a file: [`rbms_download::download_to`] in a run of the player,
/// something of a test's own in a test.
pub(crate) type Fetch = Arc<dyn Fn(&str, &Path, &Progress, &AtomicBool) -> Result<(), DownloadError> + Send + Sync>;

/// The fetch a run of the player uses: the network, within the limits a chart archive is held to.
#[cfg(not(test))]
fn default_fetch() -> Fetch {
    Arc::new(|url, file, progress, cancel| rbms_download::download_to(url, file, &rbms_download::DownloadLimits::default(), progress, cancel))
}

/// A test is never handed the network: one that has not put a fetch of its own in has every
/// download fail.
#[cfg(test)]
fn default_fetch() -> Fetch {
    Arc::new(|_, _, _, _| Err(DownloadError::Io(io::Error::other(tests::NO_NETWORK_IN_TESTS))))
}

/// What a worker thread hands back, once, and whether it has.
struct Landing<T> {
    rx: Receiver<T>,
    landed: Option<T>,
}

impl<T: Send + 'static> Landing<T> {
    /// Do `work` on a thread of its own.
    fn spawn(work: impl FnOnce() -> T + Send + 'static) -> Landing<T> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(work());
        });
        Landing { rx, landed: None }
    }

    /// What the worker handed back, once it has: `Some(None)` when it stopped without handing
    /// anything back, `None` while it is at work.
    fn take(&mut self) -> Option<Option<T>> {
        if let Some(landed) = self.landed.take() {
            return Some(Some(landed));
        }
        match self.rx.try_recv() {
            Ok(landed) => Some(Some(landed)),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(None),
        }
    }

    /// Wait for the worker to hand back what it does, so that the next [`Landing::take`] has it.
    #[cfg(test)]
    fn wait(&mut self) {
        if self.landed.is_none() {
            self.landed = self.rx.recv().ok();
        }
    }
}

/// The chart a job is getting.
#[derive(Clone, Debug)]
struct Wanted {
    md5: String,
    title: String,
}

/// What a job left in the download folder.
struct Settled {
    folder: PathBuf,
    charts: usize,
}

/// A job fetching and unpacking an archive.
struct DownloadJob {
    chart: Wanted,
    progress: Arc<Progress>,
    cancel: Arc<AtomicBool>,
    landing: Landing<Result<Settled, DownloadError>>,
}

/// A chart whose archive has been unpacked and whose folder waits to be scanned, with how many
/// charts the archive held.
struct Awaited {
    chart: Wanted,
    charts: usize,
}

/// What a scan of part of the library came back with.
struct Refreshed {
    library: Library,
    tables: Vec<MatchedTable>,
    failure: Option<String>,
}

/// A scan of part of the library and the match of the tables after it, with what it was started
/// under: the folders, the chart it is for, the library generation, the tables and the table
/// generation.
struct LibraryRefresh {
    roots: Vec<String>,
    awaited: Option<Awaited>,
    generation: u64,
    sources: Vec<TableSource>,
    table_generation: u64,
    landing: Landing<Refreshed>,
}

/// The download job and the scan after it, kept beside the frame loop: a job outlives the screen
/// that started it.
pub(crate) struct Downloads {
    job: Option<DownloadJob>,
    refresh: Option<LibraryRefresh>,
    /// Folders to scan once no job and no scan is running, and the chart the first of them is for.
    queued: Vec<String>,
    awaited: Option<Awaited>,
    /// How the job that ended last ended, for the modal to say.
    last: Option<JobStatus>,
    /// The download folder the library was last read with.
    folder_seen: Option<String>,
    fetch: Fetch,
}

impl Downloads {
    /// Nothing under way, with the library read with `folder` as the download folder.
    pub(crate) fn new(folder: Option<&str>) -> Downloads {
        Downloads { job: None, refresh: None, queued: Vec::new(), awaited: None, last: None, folder_seen: folder.map(str::to_owned), fetch: default_fetch() }
    }

    /// A full scan has read the library with `folder` as the download folder.
    pub(crate) fn scanned_with(&mut self, folder: Option<&str>) {
        self.folder_seen = folder.map(str::to_owned);
    }

    /// Whether a job is getting a chart: fetching it, unpacking it, or waiting for its folder to be
    /// scanned.
    fn is_busy(&self) -> bool {
        self.job.is_some() || self.awaited.is_some() || self.refresh.as_ref().is_some_and(|refresh| refresh.awaited.is_some())
    }

    fn status(&self) -> Option<JobStatus> {
        if let Some(job) = &self.job {
            let phase = if job.progress.phase.load(Ordering::Relaxed) == Progress::EXTRACTING { Phase::Extracting } else { Phase::Downloading };
            let expected = job.progress.expected.load(Ordering::Relaxed);
            return Some(JobStatus {
                phase,
                received: job.progress.received.load(Ordering::Relaxed),
                expected: (expected > 0).then_some(expected),
                md5: job.chart.md5.clone(),
                title: job.chart.title.clone(),
            });
        }
        let scanning = self.awaited.as_ref().or_else(|| self.refresh.as_ref().and_then(|refresh| refresh.awaited.as_ref()));
        if let Some(awaited) = scanning {
            return Some(JobStatus { phase: Phase::Scanning, received: 0, expected: None, md5: awaited.chart.md5.clone(), title: awaited.chart.title.clone() });
        }
        self.last.clone()
    }

    /// Put `root` among the folders the next scan reads, once.
    fn queue(&mut self, root: String) {
        if !self.queued.contains(&root) {
            self.queued.push(root);
        }
    }
}

/// Which step a job is on, or how it ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Downloading,
    Extracting,
    Scanning,
    /// The archive held `charts` charts, and `found` says whether the one asked for is among them.
    Done {
        found: bool,
        charts: usize,
    },
    Failed(String),
    Cancelled,
}

/// What the screen is shown of the download job: the chart it is for, the step it is on and how
/// much of the archive has arrived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JobStatus {
    pub(crate) phase: Phase,
    pub(crate) received: u64,
    pub(crate) expected: Option<u64>,
    pub(crate) md5: String,
    pub(crate) title: String,
}

impl JobStatus {
    /// Whether the job is still at work.
    pub(crate) fn is_running(&self) -> bool {
        matches!(self.phase, Phase::Downloading | Phase::Extracting | Phase::Scanning)
    }

    /// Whether asking the job to stop can still stop it: the archive has not been put in place yet.
    pub(crate) fn can_cancel(&self) -> bool {
        matches!(self.phase, Phase::Downloading | Phase::Extracting)
    }

    /// The line the job is described by, which is also what is said when it ends.
    pub(crate) fn line(&self) -> String {
        match &self.phase {
            Phase::Downloading => match self.expected {
                Some(expected) => format!("DOWNLOADING {} / {}", size_text(self.received), size_text(expected)),
                None => format!("DOWNLOADING {}", size_text(self.received)),
            },
            Phase::Extracting => "EXTRACTING".to_owned(),
            Phase::Scanning => "SCANNING".to_owned(),
            Phase::Done { found: true, charts } => format!("downloaded: {} ({charts} chart(s))", self.title),
            Phase::Done { found: false, charts } => format!("the archive had {charts} chart(s) but not this one"),
            Phase::Failed(reason) => format!("download failed: {reason}"),
            Phase::Cancelled => "download cancelled".to_owned(),
        }
    }
}

/// A number of bytes in kilobytes, or in megabytes once it is one.
fn size_text(bytes: u64) -> String {
    if bytes >= BYTES_PER_MB { format!("{:.1} MB", bytes as f64 / BYTES_PER_MB as f64) } else { format!("{} KB", bytes.div_ceil(BYTES_PER_KB)) }
}

/// The library and the tables were replaced: anything that remembers a chart by its place in the
/// library has to look again.
pub(crate) struct LibraryChange;

/// Start fetching the chart archive `entry` links to and unpacking it into the download folder.
/// Answers why it could not be started: another job is running, the link is no zip archive, or no
/// download folder is chosen.
pub(crate) fn start_download(shared: &mut AppShared, entry: &TableEntry) -> Result<(), &'static str> {
    if shared.downloads.is_busy() {
        return Err(ANOTHER_DOWNLOAD_IS_RUNNING);
    }
    if classify_link(&entry.url_diff) != LinkKind::DirectZip {
        return Err(NOT_A_ZIP_LINK);
    }
    let Some(folder) = shared.config.library.download_folder_path().map(PathBuf::from) else {
        return Err(NO_DOWNLOAD_FOLDER);
    };
    let chart = Wanted { md5: entry.md5.trim().to_owned(), title: missing_chart_title(entry).to_owned() };
    let name = safe_folder_name(entry.title.trim(), &chart.md5);
    let progress = Arc::new(Progress::default());
    let cancel = Arc::new(AtomicBool::new(false));
    let (url, fetch, worker_progress, worker_cancel) =
        (entry.url_diff.trim().to_owned(), Arc::clone(&shared.downloads.fetch), Arc::clone(&progress), Arc::clone(&cancel));
    let landing = Landing::spawn(move || fetch_and_unpack(&url, &folder, &name, &fetch, &worker_progress, &worker_cancel));
    shared.downloads.last = None;
    shared.downloads.job = Some(DownloadJob { chart, progress, cancel, landing });
    Ok(())
}

/// Ask the job to stop, if one is fetching or unpacking an archive.
pub(crate) fn cancel_download(shared: &mut AppShared) {
    if let Some(job) = &shared.downloads.job {
        job.cancel.store(true, Ordering::Relaxed);
    }
}

/// How the download job is getting on, or how the last one ended.
pub(crate) fn status(shared: &AppShared) -> Option<JobStatus> {
    shared.downloads.status()
}

/// Take what the download and the scan after it came back with. Called by the browser only, which
/// is what keeps a chart being played from having its library replaced under it.
///
/// A job that ended has its folder scanned once no other scan is running; a job that failed or was
/// cancelled is said to have. The download folder is scanned the same way when the settings name
/// another one. A scan that came back under the library generation, the tables and the table
/// generation it began with is taken in; one that did not is run again, for the chart waiting for
/// its folder as well.
pub(crate) fn apply_finished(shared: &mut AppShared) -> Option<LibraryChange> {
    take_finished_job(shared);
    let folder = shared.config.library.download_folder_path().map(str::to_owned);
    if folder != shared.downloads.folder_seen {
        shared.downloads.folder_seen.clone_from(&folder);
        if let Some(folder) = folder {
            shared.downloads.queue(folder);
        }
    }
    let change = take_finished_refresh(shared);
    let downloads = &shared.downloads;
    if downloads.refresh.is_none() && downloads.job.is_none() && !downloads.queued.is_empty() {
        let roots = std::mem::take(&mut shared.downloads.queued);
        let awaited = shared.downloads.awaited.take();
        start_refresh(shared, roots, awaited);
    }
    change
}

fn take_finished_job(shared: &mut AppShared) {
    let Some(ended) = shared.downloads.job.as_mut().and_then(|job| job.landing.take()) else {
        return;
    };
    let Some(job) = shared.downloads.job.take() else {
        return;
    };
    let failure = match ended {
        Some(Ok(settled)) => {
            shared.downloads.queue(settled.folder.to_string_lossy().into_owned());
            shared.downloads.awaited = Some(Awaited { chart: job.chart, charts: settled.charts });
            return;
        }
        Some(Err(DownloadError::Cancelled)) => Phase::Cancelled,
        Some(Err(error)) => Phase::Failed(error.to_string()),
        None => Phase::Failed(JOB_STOPPED.to_owned()),
    };
    end_job(shared, job.chart, failure);
}

/// Keep how the job for `chart` ended and say it.
fn end_job(shared: &mut AppShared, chart: Wanted, phase: Phase) {
    let level = match phase {
        Phase::Done { found: true, .. } | Phase::Cancelled => Level::Info,
        _ => Level::Warn,
    };
    let ended = JobStatus { phase, received: 0, expected: None, md5: chart.md5, title: chart.title };
    notify(level, ended.line());
    shared.downloads.last = Some(ended);
}

fn take_finished_refresh(shared: &mut AppShared) -> Option<LibraryChange> {
    let refreshed = shared.downloads.refresh.as_mut().and_then(|refresh| refresh.landing.take())?;
    let refresh = shared.downloads.refresh.take()?;
    let refreshed = match refreshed {
        Some(Refreshed { failure: None, library, tables }) => (library, tables),
        Some(Refreshed { failure: Some(failure), .. }) => {
            notify(Level::Warn, format!("library scan failed: {failure}"));
            if let Some(awaited) = refresh.awaited {
                end_job(shared, awaited.chart, Phase::Failed(format!("the library scan failed: {failure}")));
            }
            return None;
        }
        None => {
            if let Some(awaited) = refresh.awaited {
                end_job(shared, awaited.chart, Phase::Failed(SCAN_STOPPED.to_owned()));
            }
            return None;
        }
    };
    if refresh.generation != shared.library_generation || refresh.sources != shared.config.library.tables || refresh.table_generation != shared.table_generation
    {
        let mut roots = refresh.roots;
        for queued in std::mem::take(&mut shared.downloads.queued) {
            if !roots.contains(&queued) {
                roots.push(queued);
            }
        }
        let awaited = refresh.awaited.or_else(|| shared.downloads.awaited.take());
        start_refresh(shared, roots, awaited);
        return None;
    }
    let (library, tables) = refreshed;
    take_library_in(shared, library, tables);
    if let Some(awaited) = refresh.awaited {
        let found = !shared.library.indices_for_md5(&awaited.chart.md5).is_empty();
        end_job(shared, awaited.chart, Phase::Done { found, charts: awaited.charts });
    }
    Some(LibraryChange)
}

/// Put the library a scan read and the tables matched against it in place of the ones the list
/// was showing, keeping the folders the list is in and the bar under the cursor.
fn take_library_in(shared: &mut AppShared, library: Library, tables: Vec<MatchedTable>) {
    let place = ListPlace::of(shared);
    shared.library = library;
    shared.set_tables(tables);
    crate::user_folders::rematch(shared);
    shared.trace_backfill_after_scan();
    shared.library_generation = shared.library_generation.wrapping_add(1);
    show_the_list_again(shared, &place);
}

/// Scan `roots` and match the tables again on a worker thread. A folder inside another of them is
/// scanned with that one.
fn start_refresh(shared: &mut AppShared, roots: Vec<String>, awaited: Option<Awaited>) {
    let roots: Vec<String> = roots.iter().filter(|root| !roots.iter().any(|other| other != *root && Path::new(root).starts_with(other))).cloned().collect();
    let db_path = crate::songdb_path(&shared.settings_path);
    let store = TableStore::beside(&shared.settings_path);
    let sources = shared.config.library.tables.clone();
    let (scanned, matched) = (roots.clone(), sources.clone());
    let landing = Landing::spawn(move || refresh_library(&db_path, scanned, &store, &matched));
    shared.downloads.refresh =
        Some(LibraryRefresh { roots, awaited, generation: shared.library_generation, sources, table_generation: shared.table_generation, landing });
}

/// The worker of a scan of part of the library: scan `roots` into the song database at `db_path`,
/// then match `sources` against the library it holds after.
fn refresh_library(db_path: &Path, roots: Vec<String>, store: &TableStore, sources: &[TableSource]) -> Refreshed {
    let report = crate::library::scan_now(db_path, &ScanRequest::partial(roots));
    if let Some(failure) = report.failure {
        return Refreshed { library: report.library, tables: Vec::new(), failure: Some(failure) };
    }
    let tables = read_and_match_all(store, sources, &report.library, &AtomicUsize::new(0));
    Refreshed { library: report.library, tables, failure: None }
}

/// The worker of a job: fetch `url` and unpack it into a new folder called `name` in `folder`.
/// Whatever happens, nothing is left in the folder the job works in.
fn fetch_and_unpack(url: &str, folder: &Path, name: &str, fetch: &Fetch, progress: &Progress, cancel: &AtomicBool) -> Result<Settled, DownloadError> {
    let partial = folder.join(DOWNLOAD_WORK_FOLDER);
    let result = empty_folder(&partial).map_err(DownloadError::from).and_then(|()| unpack_into(url, &partial, folder, name, fetch, progress, cancel));
    let _ = fs::remove_dir_all(&partial);
    result
}

fn unpack_into(
    url: &str,
    partial: &Path,
    folder: &Path,
    name: &str,
    fetch: &Fetch,
    progress: &Progress,
    cancel: &AtomicBool,
) -> Result<Settled, DownloadError> {
    let archive = partial.join(ARCHIVE_FILE);
    let staging = partial.join(STAGING_FOLDER);
    fetch(url, &archive, progress, cancel)?;
    progress.phase.store(Progress::EXTRACTING, Ordering::Relaxed);
    let report = extract_zip(&archive, &staging, &ExtractLimits::default(), cancel)?;
    if cancel.load(Ordering::Relaxed) {
        return Err(DownloadError::Cancelled);
    }
    Ok(Settled { folder: settle(&staging, folder, name)?, charts: report.charts })
}

/// Make `folder` an empty folder, whatever was left in it.
fn empty_folder(folder: &Path) -> io::Result<()> {
    match fs::remove_dir_all(folder) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::create_dir_all(folder)
}
