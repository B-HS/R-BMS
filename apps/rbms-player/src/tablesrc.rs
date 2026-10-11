//! Difficulty-table loading and library matching: read a table from the copy kept of it or fetch
//! it (`rbms_table`), match its entries to the local song library by md5
//! (`DifficultyTable::match_levels_in`), and resolve display names.
//!
//! A table at an address is fetched once and kept ([`TableStore`]): every later run reads the copy
//! and asks the network nothing, so the song list does not wait on somebody else's server each time
//! the player is started. What the address serves now is fetched again only when the player asks
//! for it ([`start_reload`]), on a worker beside the frame loop; what the worker comes back with is
//! taken by whichever screen is up ([`apply_finished_reloads`]). A table registered as a file on
//! this machine is read from that file every time and has no copy.
//!
//! What became of each source — loaded, where from and when, or failed and why — is kept here per
//! location ([`table_status`]) for the table manager to show beside the source, so a table that did
//! not load says so where the user registered it instead of only being missing from the song list.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rbms_config::TableSource;
use rbms_library::Library;
use rbms_table::{DifficultyTable, Md5Index, SavedTable, TableError};

use crate::{AppShared, SelectView};

/// One difficulty table matched against the library: `(level label, indices of the owned charts)`
/// per level, in the table's own level order. A level none of whose charts are owned is there with
/// no indices; a table that could not be loaded has no levels at all.
pub(crate) type TableLevels = Vec<(String, Vec<usize>)>;

/// What a table is called when neither the user nor the table itself named it.
const UNNAMED_TABLE: &str = "DIFFICULTY TABLE";

/// What a source is called when its table could not be loaded and the user did not name it.
const UNLOADED_TABLE: &str = "TABLE";

/// How many sources are loaded at once. Each is a handful of http requests that mostly wait, so
/// a fetch of several tables takes as long as the slowest rather than all of them one after
/// another; the cap is what keeps a long list from opening a connection per entry at the same
/// moment.
const MAX_PARALLEL_LOADS: usize = 8;

/// The folder beside the settings file the copies of fetched tables are kept in.
const TABLES_FOLDER: &str = "tables";

/// What a copy's file is called after the hash of its table's address.
const COPY_EXTENSION: &str = "json";

/// What the copies an earlier version left in the temporary folder are called before the hash.
const LEGACY_COPY_PREFIX: &str = "rbms-table-";

/// How old a copy is before the table manager points its date out. A table is not fetched again by
/// itself however old its copy is; this is only when the player is reminded that they can.
pub(crate) const COPY_OLD_AFTER_DAYS: u64 = 30;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86_400;

/// What is said of a copy fetched less than a minute ago, and what follows a count of minutes,
/// hours or days.
const COPY_AGE_NONE: &str = "JUST NOW";
const COPY_AGE_AGO: &str = "AGO";

/// Why a reload left the copy alone although the address answered: it answered with a table that
/// lists nothing, and the copy has charts.
const LISTS_NOTHING_NOW: &str = "the address lists no charts now";

/// Why a source has no answer when the worker that was loading it stopped.
const LOADER_STOPPED: &str = "the table loader stopped unexpectedly";

/// Where the contents of a loaded table came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableOrigin {
    /// A file on this machine, read each time. Nothing is kept of it.
    File,
    /// An address, of which a copy is kept: when that copy was fetched, in seconds since the Unix
    /// epoch, or `None` for a copy that does not say.
    Address { fetched_at: Option<u64> },
}

/// What became of the last load of one table source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TableStatus {
    /// The table was read and matched against the library.
    Loaded {
        /// The table's display name, as the song list shows it.
        name: String,
        /// Charts the table lists.
        entries: usize,
        /// Level folders the table has.
        levels: usize,
        /// Library charts the table's entries matched.
        owned: usize,
        /// Where the table was read from.
        origin: TableOrigin,
        /// Why the table is still what it was before the player asked for it again, when that
        /// reload failed.
        stale: Option<String>,
    },
    /// The table could not be read; the step, the address and the reason, in a line.
    Failed { reason: String },
}

/// The process-wide record of each source's last load, by location. Loads finish on worker threads
/// and are read by the frame loop, the same way the notification queue is.
fn statuses() -> &'static Mutex<HashMap<String, TableStatus>> {
    static STATUSES: OnceLock<Mutex<HashMap<String, TableStatus>>> = OnceLock::new();
    STATUSES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn record_status(location: &str, status: TableStatus) {
    if let Ok(mut statuses) = statuses().lock() {
        statuses.insert(location.to_string(), status);
    }
}

/// What became of the last load of the source at `location`, if it has been loaded this run.
pub(crate) fn table_status(location: &str) -> Option<TableStatus> {
    statuses().lock().ok()?.get(location).cloned()
}

/// Whether the last load of the source at `location` failed.
pub(crate) fn table_load_failed(location: &str) -> bool {
    matches!(table_status(location), Some(TableStatus::Failed { .. }))
}

/// Drop what is remembered of a source that has been removed from the list.
pub(crate) fn forget_table_status(location: &str) {
    if let Ok(mut statuses) = statuses().lock() {
        statuses.remove(location);
    }
}

/// Stable per-URL filename for the on-disk table copy.
fn url_hash(s: &str) -> String {
    let mut h: u64 = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33) ^ b as u64;
    }
    format!("{h:016x}")
}

/// Whether a source is an address to fetch rather than a file on this machine.
fn is_address(location: &str) -> bool {
    location.starts_with("http://") || location.starts_with("https://")
}

/// The time now, in seconds since the Unix epoch.
pub(crate) fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

/// How long ago a copy was fetched, in the largest of minutes, hours and days it has been a whole
/// one of: `12 MINUTES AGO`, `1 HOUR AGO`, `45 DAYS AGO`, and `JUST NOW` under a minute.
///
/// An age reads the same in every time zone, which a date does not: the player has no way to tell
/// the application which zone a day is to be counted in, and a copy fetched in the small hours
/// would be dated the day before by anyone east of Greenwich. A copy dated after `now` -- the clock
/// was set back -- was fetched just now.
pub(crate) fn copy_age(fetched_at: u64, now: u64) -> String {
    let age = now.saturating_sub(fetched_at);
    let counted = |count: u64, unit: &str| format!("{count} {unit}{} {COPY_AGE_AGO}", if count == 1 { "" } else { "S" });
    match age {
        age if age < SECONDS_PER_MINUTE => COPY_AGE_NONE.to_string(),
        age if age < SECONDS_PER_HOUR => counted(age / SECONDS_PER_MINUTE, "MINUTE"),
        age if age < SECONDS_PER_DAY => counted(age / SECONDS_PER_HOUR, "HOUR"),
        age => counted(age / SECONDS_PER_DAY, "DAY"),
    }
}

/// How many whole days ago a copy was fetched.
pub(crate) fn copy_age_days(fetched_at: u64, now: u64) -> u64 {
    now.saturating_sub(fetched_at) / SECONDS_PER_DAY
}

/// How a table at an address is fetched: every request of it, and the parse of what they answered.
pub(crate) type Fetcher = Arc<dyn Fn(&str) -> Result<DifficultyTable, TableError> + Send + Sync>;

/// What a store reaches outside the settings folder: the folder an earlier version left its copies
/// in, and the network.
#[derive(Clone)]
struct Outside {
    legacy_folder: PathBuf,
    fetch: Fetcher,
}

/// The outside a run of the player reaches: the temporary folder and the real network.
#[cfg(not(test))]
fn outside() -> Outside {
    Outside { legacy_folder: std::env::temp_dir(), fetch: Arc::new(DifficultyTable::fetch) }
}

/// The outside a test reaches: whatever the test put there, and otherwise nothing at all, so no
/// test asks the network or the machine's temporary folder for a table by accident.
#[cfg(test)]
fn outside() -> Outside {
    test_outside::current()
}

/// Where the copies of fetched tables are kept, and how a table that has none is fetched.
///
/// A copy is one file in a folder beside the settings file, named after the hash of the address its
/// table was fetched from. It is written whole and renamed into place, so a run that is stopped
/// while writing leaves the earlier copy or none, never half of one; and a file that does not read
/// as a copy is no copy, so its table is fetched again.
#[derive(Clone)]
pub(crate) struct TableStore {
    folder: PathBuf,
    outside: Outside,
}

impl TableStore {
    /// The store beside the file at `beside`: the settings file, or any file kept next to it.
    pub(crate) fn beside(beside: &Path) -> TableStore {
        TableStore { folder: tables_folder(beside), outside: outside() }
    }

    fn copy_path(&self, location: &str) -> PathBuf {
        self.folder.join(format!("{}.{COPY_EXTENSION}", url_hash(location)))
    }

    /// Where an earlier version kept the copy of the table at `location`: the temporary folder,
    /// which the operating system is free to empty.
    fn legacy_path(&self, location: &str) -> PathBuf {
        self.outside.legacy_folder.join(format!("{LEGACY_COPY_PREFIX}{}.{COPY_EXTENSION}", url_hash(location)))
    }

    /// The copy kept of the table at `location`, taking over the one an earlier version left in
    /// the temporary folder when there is no other.
    fn saved(&self, location: &str) -> Option<SavedTable> {
        self.copy_of(location).or_else(|| self.adopt_legacy_copy(location))
    }

    /// The copy in this store's own folder. A copy that names another address is not this table's:
    /// two addresses came to one file name, and the file is the other's.
    fn copy_of(&self, location: &str) -> Option<SavedTable> {
        SavedTable::read(&self.copy_path(location)).filter(|saved| saved.source.is_empty() || saved.source == location)
    }

    /// Move the copy an earlier version left in the temporary folder into this store. That copy
    /// does not say when it was fetched, which is when its file was last written.
    fn adopt_legacy_copy(&self, location: &str) -> Option<SavedTable> {
        let legacy = self.legacy_path(location);
        let old = SavedTable::read(&legacy)?;
        let written = std::fs::metadata(&legacy).and_then(|file| file.modified()).ok();
        let fetched_at = written.and_then(|at| at.duration_since(UNIX_EPOCH).ok()).map(|since| since.as_secs());
        if self.write(location, &old.table, fetched_at) {
            let _ = std::fs::remove_file(&legacy);
        }
        Some(SavedTable { table: old.table, source: location.to_string(), fetched_at })
    }

    /// Keep `table` as the copy of the table at `location`, fetched just now, and answer when that
    /// was. A copy that cannot be written is reported and the table is used all the same; it is
    /// fetched again the next time, since there is nothing to read it from.
    fn save(&self, location: &str, table: &DifficultyTable) -> u64 {
        let fetched_at = unix_now();
        self.write(location, table, Some(fetched_at));
        fetched_at
    }

    fn write(&self, location: &str, table: &DifficultyTable, fetched_at: Option<u64>) -> bool {
        let written = SavedTable::json_of(table, location, fetched_at)
            .map_err(|error| error.to_string())
            .and_then(|json| crate::write_atomic(&self.copy_path(location), &json).map_err(|error| error.to_string()));
        if let Err(reason) = &written {
            crate::notify::notify(crate::notify::Level::Warn, format!("table copy not saved ({location}): {reason}"));
        }
        written.is_ok()
    }

    /// Drop the copy of a table that has been removed from the list, wherever it is kept.
    ///
    /// A reload of that table that is still out is told first, so the copy it would have kept on
    /// arriving is never written: a table removed while it is being fetched leaves nothing behind,
    /// whether or not a screen is up to take what the reload comes back with.
    pub(crate) fn forget(&self, location: &str) {
        if !is_address(location) {
            return;
        }
        if let Ok(jobs) = reload_jobs().lock() {
            for job in jobs.iter().filter(|job| job.folder == self.folder && job.pending.contains(location)) {
                held(&job.removed).insert(location.to_string());
            }
        }
        let _ = std::fs::remove_file(self.copy_path(location));
        let _ = std::fs::remove_file(self.legacy_path(location));
    }
}

#[cfg(test)]
impl TableStore {
    /// Whether a copy of the table at `location` is in this store's folder, readable or not.
    pub(crate) fn has_copy_file(&self, location: &str) -> bool {
        self.copy_path(location).exists()
    }

    /// Keep a table called `name` with the body `body` as the copy of the table at `location`, as
    /// if it had been fetched at `fetched_at`.
    pub(crate) fn keep_for_tests(&self, location: &str, name: &str, body: &[u8], fetched_at: Option<u64>) {
        let mut table = DifficultyTable::parse_body(body, None).expect("the fixture body is a body");
        table.name = name.to_string();
        assert!(self.write(location, &table, fetched_at), "the fixture copy is written");
    }

    /// The name of the table the copy of `location` holds, if there is a copy.
    pub(crate) fn copy_name(&self, location: &str) -> Option<String> {
        self.copy_of(location).map(|saved| saved.table.name)
    }
}

/// The folder the copies are kept in for the settings file at `beside`, or a file next to it.
fn tables_folder(beside: &Path) -> PathBuf {
    beside.parent().map(|folder| folder.join(TABLES_FOLDER)).unwrap_or_else(|| PathBuf::from(TABLES_FOLDER))
}

/// Why one difficulty-table source could not be loaded: the table crate's own failure, or the local
/// file the source named could not be read.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TableSourceError {
    #[error("{0}")]
    Table(#[from] rbms_table::TableError),
    #[error("file: {0}")]
    Read(#[from] std::io::Error),
}

/// The failure in a line for a source registered as `location`: the step and the reason, and the
/// address the step was asking when that is not the registered one itself.
fn failure_reason(location: &str, error: &TableSourceError) -> String {
    match error {
        TableSourceError::Table(error) => fetch_failure_reason(location, error),
        TableSourceError::Read(_) => error.to_string(),
    }
}

/// [`failure_reason`] for the table crate's own failure.
fn fetch_failure_reason(location: &str, error: &rbms_table::TableError) -> String {
    match error {
        rbms_table::TableError::At { stage, url, source } if url == location => format!("{stage}: {source}"),
        other => other.to_string(),
    }
}

/// Whether a table at an address is read from the copy kept of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadPolicy {
    /// Read the copy and ask the network nothing; fetch only a table that has no copy. This is
    /// every load the player did not ask for: starting up, and reading the library again.
    SavedFirst,
    /// Fetch what the address serves now, and keep that: a table being registered.
    Fetch,
}

/// A table that was read, and where from.
struct TableLoad {
    table: DifficultyTable,
    origin: TableOrigin,
}

/// Load a table from a `location`: an http(s) URL (read from its copy, or fetched and kept) or a
/// local `data.json` body file.
fn load_table_source(store: &TableStore, policy: LoadPolicy, location: &str) -> Result<TableLoad, TableSourceError> {
    if !is_address(location) {
        let bytes = std::fs::read(location)?;
        return Ok(TableLoad { table: DifficultyTable::parse_body(&bytes, None)?, origin: TableOrigin::File });
    }
    if policy == LoadPolicy::SavedFirst
        && let Some(saved) = store.saved(location)
    {
        return Ok(TableLoad { table: saved.table, origin: TableOrigin::Address { fetched_at: saved.fetched_at } });
    }
    let table = (store.outside.fetch)(location)?;
    let fetched_at = store.save(location, &table);
    Ok(TableLoad { table, origin: TableOrigin::Address { fetched_at: Some(fetched_at) } })
}

/// Display name for a loaded table: the user's source name if there is one, else the table's own
/// (header) name, else a generic label. The name is shown as written — the text renderer draws any
/// script a system font covers — with its Latin letters in the capitals the rest of the list uses.
fn pick_table_name(src: &TableSource, table: &rbms_table::DifficultyTable) -> String {
    [src.name.trim(), table.name.trim()].into_iter().find(|name| !name.is_empty()).map_or_else(|| UNNAMED_TABLE.to_string(), str::to_ascii_uppercase)
}

/// Display name for a source whose table failed to load (no header available).
fn fallback_source_name(src: &TableSource) -> String {
    let s = src.name.trim();
    if s.is_empty() { UNLOADED_TABLE.to_string() } else { s.to_ascii_uppercase() }
}

/// Match a table that was read against the library, and remember what became of it for the table
/// manager.
fn match_and_record(src: &TableSource, load: &TableLoad, library: &Md5Index) -> (String, TableLevels) {
    let table = &load.table;
    let lv = table.match_levels_in(library);
    let owned: usize = lv.iter().map(|(_, v)| v.len()).sum();
    println!("  {} entries ({} skipped), matched {owned} local charts across {} levels", table.entries.len(), table.skipped, lv.len());
    let name = pick_table_name(src, table);
    record_status(
        &src.location,
        TableStatus::Loaded { name: name.clone(), entries: table.entries.len(), levels: lv.len(), owned, origin: load.origin, stale: None },
    );
    (name, lv)
}

/// Load one table source and match it against the library's md5s, indexed once for every table.
///
/// Taking the index rather than the library is what lets a load run on a worker thread: the browser
/// keeps reading the library it has while the http request is out.
///
/// A source that cannot be loaded still answers, with no levels, and its failure is reported once
/// here: as a notification, and as the status the table manager shows beside the source.
pub(crate) fn load_and_match(store: &TableStore, policy: LoadPolicy, src: &TableSource, library: &Md5Index) -> (String, TableLevels) {
    println!("loading table: {} ({})", src.name, src.location);
    match load_table_source(store, policy, &src.location) {
        Ok(load) => match_and_record(src, &load, library),
        Err(e) => {
            let reason = failure_reason(&src.location, &e);
            crate::notify::notify(crate::notify::Level::Warn, format!("table load failed ({}): {reason}", src.location));
            record_status(&src.location, TableStatus::Failed { reason });
            (fallback_source_name(src), Vec::new())
        }
    }
}

/// Load every table source (one entry per source, aligned with `sources`). Returns parallel
/// `(display name, per-level owned-chart groups)` vecs.
///
/// A table that has a copy is read from it, so a run in which every table has one asks the network
/// nothing here. The ones that have none are fetched, and the sources are loaded side by side, a
/// few at a time: each fetch is a table's worth of http requests, and one host that does not answer
/// must not hold the others — or the song list they are all waited for with — behind its timeout.
/// A source that fails keeps its slot, with no levels. The library is indexed once for all of them.
///
/// `done` is raised as each source is finished, so the screen that started this off-thread can say
/// how far through the list it is rather than showing a bar that never moves.
pub(crate) fn fetch_and_match(store: &TableStore, sources: &[TableSource], library: &Library, done: &AtomicUsize) -> (Vec<String>, Vec<TableLevels>) {
    let index = Md5Index::new(library.md5s());
    let index = &index;
    let mut names = Vec::with_capacity(sources.len());
    let mut levels = Vec::with_capacity(sources.len());
    for batch in sources.chunks(MAX_PARALLEL_LOADS) {
        let loaded: Vec<(String, TableLevels)> = std::thread::scope(|scope| {
            let workers: Vec<_> = batch
                .iter()
                .map(|src| {
                    scope.spawn(move || {
                        let matched = load_and_match(store, LoadPolicy::SavedFirst, src, index);
                        done.fetch_add(1, Ordering::Relaxed);
                        matched
                    })
                })
                .collect();
            workers.into_iter().zip(batch).map(|(worker, src)| worker.join().unwrap_or_else(|_| stopped_load(src, done))).collect()
        });
        for (name, lv) in loaded {
            names.push(name);
            levels.push(lv);
        }
    }
    (names, levels)
}

/// The slot of a source whose loader stopped without answering, reported the way a failed load is.
fn stopped_load(src: &TableSource, done: &AtomicUsize) -> (String, TableLevels) {
    let reason = LOADER_STOPPED.to_string();
    crate::notify::notify(crate::notify::Level::Warn, format!("table load failed ({}): {reason}", src.location));
    record_status(&src.location, TableStatus::Failed { reason });
    done.fetch_add(1, Ordering::Relaxed);
    (fallback_source_name(src), Vec::new())
}

/// What a reload of one source came to.
enum ReloadOutcome {
    /// The table as its address serves it now, or as its file reads now. It has been kept as the
    /// copy already when it has one.
    Fetched(TableLoad),
    /// The reload failed and nothing was touched: the copy and the folder in the song list are
    /// what they were.
    Kept { reason: String },
    /// The table was removed from the list while it was being fetched. Nothing was kept of what
    /// arrived, and there is nothing to bring up to date.
    Removed,
}

/// One source of a reload, finished.
struct ReloadDone {
    location: String,
    outcome: ReloadOutcome,
}

/// A reload in flight: the sources it was asked for that have not answered yet, and how the ones
/// that have came out.
struct ReloadJob {
    /// The table folder of the player that asked, which is what tells its reloads from another's.
    folder: PathBuf,
    rx: Receiver<ReloadDone>,
    pending: HashSet<String>,
    /// The sources that were removed from the list while this reload was out. The worker reads it
    /// before it keeps a table, and holds it while it does, so a copy is either written before its
    /// table is removed -- and dropped by the removal -- or not written at all.
    removed: Removed,
    updated: usize,
    failed: usize,
}

/// The locations removed from the list while a reload of them was out, shared between the reload
/// and its worker.
type Removed = Arc<Mutex<HashSet<String>>>;

/// What a lock holds, whether or not a thread stopped while holding it: a set of locations is whole
/// after any insert that began.
fn held<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Every reload in flight. A reload outlives the screen that asked for it — the table manager can
/// be left, and the song list has no state of its own to keep one in — so they are kept here and
/// taken by whichever screen is up when they land.
fn reload_jobs() -> &'static Mutex<Vec<ReloadJob>> {
    static JOBS: OnceLock<Mutex<Vec<ReloadJob>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Fetch one source again. A table at an address is kept as the new copy when it arrives; one whose
/// address now lists nothing does not replace a copy that lists something, and one that was removed
/// from the list while it was out (`removed`) is not kept at all.
fn reload_one(store: &TableStore, src: &TableSource, removed: &Mutex<HashSet<String>>) -> ReloadOutcome {
    println!("reloading table: {} ({})", src.name, src.location);
    let location = src.location.as_str();
    if !is_address(location) {
        return match load_table_source(store, LoadPolicy::Fetch, location) {
            Ok(load) => ReloadOutcome::Fetched(load),
            Err(error) => ReloadOutcome::Kept { reason: failure_reason(location, &error) },
        };
    }
    match (store.outside.fetch)(location) {
        Ok(table) if table.entries.is_empty() && store.copy_of(location).is_some_and(|saved| !saved.table.entries.is_empty()) => {
            ReloadOutcome::Kept { reason: LISTS_NOTHING_NOW.to_string() }
        }
        Ok(table) => {
            let removed = held(removed);
            if removed.contains(location) {
                return ReloadOutcome::Removed;
            }
            let fetched_at = store.save(location, &table);
            ReloadOutcome::Fetched(TableLoad { table, origin: TableOrigin::Address { fetched_at: Some(fetched_at) } })
        }
        Err(error) => ReloadOutcome::Kept { reason: fetch_failure_reason(location, &error) },
    }
}

/// Start fetching `sources` again on a worker, and answer how many were started: a source that is
/// being reloaded already is not started twice.
///
/// Nothing waits for the worker. What it comes back with is taken by [`apply_finished_reloads`],
/// which is also where the song list is brought up to date.
pub(crate) fn start_reload(store: TableStore, sources: Vec<TableSource>) -> usize {
    let Ok(mut jobs) = reload_jobs().lock() else {
        return 0;
    };
    let mut pending = HashSet::new();
    let fresh: Vec<TableSource> = sources
        .into_iter()
        .filter(|src| !jobs.iter().any(|job| job.folder == store.folder && job.pending.contains(&src.location)) && pending.insert(src.location.clone()))
        .collect();
    if fresh.is_empty() {
        return 0;
    }
    let started = fresh.len();
    let (tx, rx) = std::sync::mpsc::channel();
    let removed = Removed::default();
    jobs.push(ReloadJob { folder: store.folder.clone(), rx, pending, removed: Arc::clone(&removed), updated: 0, failed: 0 });
    drop(jobs);
    std::thread::spawn(move || {
        for batch in fresh.chunks(MAX_PARALLEL_LOADS) {
            std::thread::scope(|scope| {
                for src in batch {
                    let (store, tx, removed) = (&store, tx.clone(), &removed);
                    scope.spawn(move || {
                        let _ = tx.send(ReloadDone { location: src.location.clone(), outcome: reload_one(store, src, removed) });
                    });
                }
            });
        }
    });
    started
}

/// Fetch again the table at `which` of the player's list, or every table of it, and say so.
pub(crate) fn reload_tables(shared: &AppShared, which: Option<usize>) {
    let tables = &shared.config.library.tables;
    let sources: Vec<TableSource> = match which {
        Some(at) => tables.get(at).cloned().into_iter().collect(),
        None => tables.clone(),
    };
    if sources.is_empty() {
        crate::notify::notify(crate::notify::Level::Info, "no difficulty tables to reload");
        return;
    }
    match start_reload(TableStore::beside(&shared.settings_path), sources) {
        0 => crate::notify::notify(crate::notify::Level::Info, "the tables are being reloaded already"),
        started => crate::notify::notify(crate::notify::Level::Info, format!("reloading {started} difficulty table(s)")),
    }
}

/// Whether the source at `location` of the player whose settings are at `settings_path` is being
/// reloaded right now: asked for, and not yet taken by a screen.
pub(crate) fn table_is_reloading(settings_path: &Path, location: &str) -> bool {
    let Ok(jobs) = reload_jobs().lock() else {
        return false;
    };
    if jobs.is_empty() {
        return false;
    }
    let folder = tables_folder(settings_path);
    jobs.iter().any(|job| job.folder == folder && job.pending.contains(location))
}

/// What has landed of the reloads of one player since the last look, and the tally of each reload
/// that is now over: how many of its tables were updated and how many failed.
///
/// `is_listed` says whether a location is still one of the player's tables. A table that was
/// removed while its reload was out is in neither count -- it was not updated, and its reload did
/// not fail -- and a reload none of whose tables are left has no tally to give.
fn take_landed(settings_path: &Path, is_listed: &dyn Fn(&str) -> bool) -> (Vec<ReloadDone>, Vec<(usize, usize)>) {
    let mut landed = Vec::new();
    let mut finished = Vec::new();
    let Ok(mut jobs) = reload_jobs().lock() else {
        return (landed, finished);
    };
    if jobs.is_empty() {
        return (landed, finished);
    }
    let folder = tables_folder(settings_path);
    for job in jobs.iter_mut().filter(|job| job.folder == folder) {
        loop {
            match job.rx.try_recv() {
                Ok(done) => {
                    job.pending.remove(&done.location);
                    match done.outcome {
                        ReloadOutcome::Fetched(_) if is_listed(&done.location) => job.updated += 1,
                        ReloadOutcome::Kept { .. } if is_listed(&done.location) => job.failed += 1,
                        ReloadOutcome::Fetched(_) | ReloadOutcome::Kept { .. } | ReloadOutcome::Removed => {}
                    }
                    landed.push(done);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    job.failed += job.pending.iter().filter(|location| is_listed(location)).count();
                    landed.extend(
                        job.pending.drain().map(|location| ReloadDone { location, outcome: ReloadOutcome::Kept { reason: LOADER_STOPPED.to_string() } }),
                    );
                    break;
                }
            }
        }
    }
    jobs.retain(|job| {
        let over = job.folder == folder && job.pending.is_empty();
        if over && job.updated + job.failed > 0 {
            finished.push((job.updated, job.failed));
        }
        !over
    });
    (landed, finished)
}

/// Take what the reloads in flight have come back with since the last frame, and answer whether
/// anything was taken.
///
/// A table that arrived is matched against the library as it is now and takes the place of the one
/// in the song list, which is rebuilt where it stands. A reload that failed leaves the table, its
/// copy and its folder as they were, and says why: beside the source in the table manager, and as a
/// notification. When the last table of a reload is in, one line says how the whole of it went.
///
/// A reload is asked for by location and answered by location. A table that was removed while its
/// reload was out is simply not there to update, and is not counted in how the reload went; the
/// copy the reload may have written of it is dropped. A location the list holds more than once --
/// a settings file written by hand can -- is one table fetched once, and every row of it is
/// brought up to date.
pub(crate) fn apply_finished_reloads(shared: &mut AppShared) -> bool {
    let tables = &shared.config.library.tables;
    let (landed, finished) = take_landed(&shared.settings_path, &|location| tables.iter().any(|source| source.location == location));
    if landed.is_empty() && finished.is_empty() {
        return false;
    }
    let store = TableStore::beside(&shared.settings_path);
    let index = Md5Index::new(shared.library.md5s());
    let shown = shown_level(shared);
    let mut list_changed = false;
    for done in landed {
        let rows: Vec<usize> =
            shared.config.library.tables.iter().enumerate().filter(|(_, source)| source.location == done.location).map(|(at, _)| at).collect();
        if rows.is_empty() {
            store.forget(&done.location);
            continue;
        }
        match done.outcome {
            ReloadOutcome::Fetched(load) => {
                for at in rows {
                    let (name, levels) = match_and_record(&shared.config.library.tables[at], &load, &index);
                    set_table_slot(shared, at, name, levels);
                }
                list_changed = true;
            }
            ReloadOutcome::Kept { reason } => {
                crate::notify::notify(crate::notify::Level::Warn, format!("table reload failed ({}): {reason}", done.location));
                let kept = match table_status(&done.location) {
                    Some(TableStatus::Loaded { name, entries, levels, owned, origin, .. }) => {
                        TableStatus::Loaded { name, entries, levels, owned, origin, stale: Some(reason) }
                    }
                    Some(TableStatus::Failed { .. }) | None => TableStatus::Failed { reason },
                };
                record_status(&done.location, kept);
            }
            ReloadOutcome::Removed => {}
        }
    }
    if list_changed {
        show_the_list_again(shared, shown);
    }
    for (updated, failed) in finished {
        let level = if failed == 0 { crate::notify::Level::Info } else { crate::notify::Level::Warn };
        crate::notify::notify(level, format!("difficulty tables reloaded: {updated} updated, {failed} failed"));
    }
    true
}

/// Put a reloaded table in the slot of its source. The names and the levels are kept in the order
/// of the sources; a slot that is not there yet is made.
fn set_table_slot(shared: &mut AppShared, at: usize, name: String, levels: TableLevels) {
    if shared.table_names.len() <= at {
        shared.table_names.resize(at + 1, UNLOADED_TABLE.to_string());
    }
    if shared.table_levels.len() <= at {
        shared.table_levels.resize(at + 1, Vec::new());
    }
    shared.table_names[at] = name;
    shared.table_levels[at] = levels;
}

/// The level of a table the song list is looking into, by the table's place and the level's name,
/// or `None` while it is looking anywhere else.
fn shown_level(shared: &AppShared) -> Option<(usize, String)> {
    let SelectView::TableLevel(table, level) = shared.select_view else {
        return None;
    };
    let (name, _) = shared.table_levels.get(table)?.get(level)?;
    Some((table, name.clone()))
}

/// Rebuild the song list where it stands after a table under it changed. `shown` is the level it
/// was looking into before the change ([`shown_level`]).
///
/// A level is followed by its name: a reload can add a level before it or take one away, and the
/// level at the same place in the reloaded table is then another level. A view into a level the
/// table no longer has steps out to the table's levels, and a view into a table that has no levels
/// left steps out to what is still there.
fn show_the_list_again(shared: &mut AppShared, shown: Option<(usize, String)>) {
    let levels_of = |table: usize| shared.table_levels.get(table).map_or(0, Vec::len);
    let place_of = |table: usize| {
        let (_, name) = shown.as_ref().filter(|(shown_table, _)| *shown_table == table)?;
        shared.table_levels.get(table)?.iter().position(|(level, _)| level == name)
    };
    shared.select_view = match shared.select_view {
        SelectView::TableLevels(table) | SelectView::TableLevel(table, _) if levels_of(table) == 0 => SelectView::Root,
        SelectView::TableLevel(table, _) => place_of(table).map_or(SelectView::TableLevels(table), |level| SelectView::TableLevel(table, level)),
        view => view,
    };
    shared.rebuild_select_items();
    shared.sel = shared.sel.min(shared.select_items.len().saturating_sub(1));
}

/// What the tests put in place of the network and of the temporary folder.
///
/// A store made on a test's thread reaches what that test installed, and nothing when it installed
/// nothing: every fetch is refused and no earlier copy is found. The workers a load is run on are
/// handed the store, so what the test installed goes with it onto their threads.
#[cfg(test)]
pub(crate) mod test_outside {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{Arc, Mutex};

    use rbms_table::{DifficultyTable, TableError};

    use super::{Fetcher, Outside};

    thread_local! {
        static INSTALLED: RefCell<Option<Outside>> = const { RefCell::new(None) };
    }

    /// The outside of the thread this is asked on.
    pub(super) fn current() -> Outside {
        INSTALLED
            .with(|installed| installed.borrow().clone())
            .unwrap_or_else(|| Outside { legacy_folder: no_such_folder(), fetch: Arc::new(|_| Err(refused())) })
    }

    /// A folder nothing is ever written to, for a test that has no earlier copies.
    pub(crate) fn no_such_folder() -> PathBuf {
        std::env::temp_dir().join("rbms-tables-of-no-earlier-version")
    }

    /// What a fetch nobody answers fails with.
    pub(crate) fn refused() -> TableError {
        TableError::TimedOut { limit: std::time::Duration::ZERO }
    }

    /// What a test installed, taken away again when the test lets go of it.
    pub(crate) struct Installed;

    impl Drop for Installed {
        fn drop(&mut self) {
            INSTALLED.with(|installed| *installed.borrow_mut() = None);
        }
    }

    /// Make every store this thread builds from here on reach `legacy_folder` and `fetch`.
    pub(crate) fn install(legacy_folder: PathBuf, fetch: Fetcher) -> Installed {
        INSTALLED.with(|installed| *installed.borrow_mut() = Some(Outside { legacy_folder, fetch }));
        Installed
    }

    /// What one address answers with: the name of its table, and the body.
    type Answer = (String, Vec<u8>);

    /// Table addresses that live in a test: what each answers with, how often any was asked, and a
    /// gate a test can shut to hold every answer back until it opens it.
    #[derive(Clone, Default)]
    pub(crate) struct FakeTables {
        bodies: Arc<Mutex<HashMap<String, Answer>>>,
        asked: Arc<AtomicUsize>,
        gate: Arc<Mutex<Option<Receiver<()>>>>,
    }

    impl FakeTables {
        /// Answer `address` with a table called `name` whose body is `body` from here on.
        pub(crate) fn serve(&self, address: &str, name: &str, body: &[u8]) {
            self.bodies.lock().expect("the fake tables").insert(address.to_string(), (name.to_string(), body.to_vec()));
        }

        /// Stop answering `address`.
        pub(crate) fn take_down(&self, address: &str) {
            self.bodies.lock().expect("the fake tables").remove(address);
        }

        /// How many fetches have been asked for, answered or not.
        pub(crate) fn asked(&self) -> usize {
            self.asked.load(Ordering::Relaxed)
        }

        /// Hold every answer back until what this hands over is dropped.
        pub(crate) fn hold(&self) -> Sender<()> {
            let (open, shut) = std::sync::mpsc::channel();
            *self.gate.lock().expect("the gate") = Some(shut);
            open
        }

        fn fetch(&self, address: &str) -> Result<DifficultyTable, TableError> {
            self.asked.fetch_add(1, Ordering::Relaxed);
            if let Some(shut) = self.gate.lock().expect("the gate").as_ref() {
                let _ = shut.recv();
            }
            let (name, body) = self.bodies.lock().expect("the fake tables").get(address).cloned().ok_or_else(refused)?;
            let mut table = DifficultyTable::parse_body(&body, None)?;
            table.name = name;
            Ok(table)
        }

        /// The fetcher that asks these addresses.
        pub(crate) fn fetcher(&self) -> Fetcher {
            let tables = self.clone();
            Arc::new(move |address| tables.fetch(address))
        }

        /// Make every store this thread builds from here on fetch from these addresses, with no
        /// earlier copies to find.
        pub(crate) fn install(&self) -> Installed {
            install(no_such_folder(), self.fetcher())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_source_that_is_not_there_reports_the_read_failure_by_type() {
        let missing = std::env::temp_dir().join(format!("rbms-no-such-table-{}.json", std::process::id()));
        let error = match load_table_source(&nowhere(), LoadPolicy::SavedFirst, missing.to_str().expect("utf-8 temp path")) {
            Ok(_) => panic!("a missing file is not a table"),
            Err(e) => e,
        };
        assert!(matches!(error, TableSourceError::Read(_)), "got {error:?}");
        assert!(error.to_string().starts_with("file: "), "the message says it was the file that could not be read: {error}");
    }

    #[test]
    fn a_local_source_that_is_not_json_reports_the_table_crates_own_error() {
        let path = std::env::temp_dir().join(format!("rbms-bad-table-{}.json", std::process::id()));
        std::fs::write(&path, b"not json").expect("write the fixture");
        let error = match load_table_source(&nowhere(), LoadPolicy::SavedFirst, path.to_str().expect("utf-8 temp path")) {
            Ok(_) => panic!("a non-JSON body is not a table"),
            Err(e) => e,
        };
        assert!(matches!(error, TableSourceError::Table(rbms_table::TableError::BodyParse(_))), "got {error:?}");
        assert!(error.to_string().starts_with("body parse:"), "the message the screen shows is unchanged: {error}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_local_source_that_parses_matches_the_library_by_md5() {
        let path = std::env::temp_dir().join(format!("rbms-good-table-{}.json", std::process::id()));
        std::fs::write(&path, br#"[{"md5":"AABB","level":"1"}]"#).expect("write the fixture");
        let load = load_table_source(&nowhere(), LoadPolicy::SavedFirst, path.to_str().expect("utf-8 temp path")).expect("a JSON body is a table");
        assert_eq!(load.origin, TableOrigin::File, "a file read off disk has no saved copy of itself");
        assert_eq!(load.table.match_levels(["aabb", "ccdd"].into_iter()), vec![("1".to_string(), vec![0])], "md5 matching is case-insensitive");
        let _ = std::fs::remove_file(&path);
    }

    /// One library entry, with only the fields the matcher and the levels read filled in.
    fn entry(md5: &str) -> rbms_library::SongEntry {
        rbms_library::SongEntry {
            path: std::path::PathBuf::from(format!("/songs/{md5}.bms")),
            title: md5.into(),
            subtitle: String::new(),
            artist: String::new(),
            genre: String::new(),
            maker: String::new(),
            level: "1".into(),
            difficulty: 3,
            init_bpm: 150.0,
            rank: 2,
            total: 300.0,
            mode: rbms_model::Mode::BEAT_7K,
            md5: md5.into(),
            stagefile: String::new(),
            banner: String::new(),
            preview: String::new(),
        }
    }

    fn library_md5s(library: &Library) -> Md5Index {
        Md5Index::new(library.md5s())
    }

    /// A store for the tests that load files only: it has no folder worth the name, finds no
    /// earlier copy and refuses every fetch.
    fn nowhere() -> TableStore {
        TableStore {
            folder: test_outside::no_such_folder(),
            outside: Outside { legacy_folder: test_outside::no_such_folder(), fetch: Arc::new(|_| Err(test_outside::refused())) },
        }
    }

    #[test]
    fn a_table_source_is_matched_against_the_library_index_and_reports_library_positions() {
        let path = std::env::temp_dir().join(format!("rbms-lib-table-{}.json", std::process::id()));
        std::fs::write(&path, br#"[{"md5":"BBBB","level":"1"},{"md5":"dddd","level":"2"}]"#).expect("write the fixture");
        let library = Library::from_songs(vec![entry("aaaa"), entry("bbbb"), entry("cccc"), entry("DDDD")]);
        let source = TableSource { name: "MINE".into(), location: path.to_string_lossy().to_string() };

        let (name, levels) = load_and_match(&nowhere(), LoadPolicy::SavedFirst, &source, &library_md5s(&library));
        assert_eq!(name, "MINE");
        assert_eq!(levels, vec![("1".to_string(), vec![1]), ("2".to_string(), vec![3])], "levels carry positions in the library's own order");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn every_source_is_counted_as_it_is_finished_so_the_screen_can_say_how_far_it_is() {
        let library = Library::from_songs(vec![entry("aaaa")]);
        let sources = vec![
            TableSource { name: "one".into(), location: "/no/such/one.json".into() },
            TableSource { name: "two".into(), location: "/no/such/two.json".into() },
        ];
        let done = AtomicUsize::new(0);
        let (names, levels) = fetch_and_match(&nowhere(), &sources, &library, &done);
        assert_eq!(done.load(Ordering::Relaxed), 2, "a source that failed is still one the screen no longer waits for");
        assert_eq!(names.len(), 2);
        assert_eq!(levels.len(), 2);
    }

    /// Every source, fetched or read off disk, is matched on a worker against a copy of the
    /// library's md5s in library order, so what a table row points at is a position in the library.
    #[test]
    fn matching_against_the_librarys_md5s_reports_library_positions() {
        let path = std::env::temp_dir().join(format!("rbms-md5s-table-{}.json", std::process::id()));
        std::fs::write(&path, br#"[{"md5":"BBBB","level":"1"}]"#).expect("write the fixture");
        let library = Library::from_songs(vec![entry("aaaa"), entry("bbbb")]);
        let source = TableSource { name: "MINE".into(), location: path.to_string_lossy().to_string() };
        assert_eq!(
            load_and_match(&nowhere(), LoadPolicy::SavedFirst, &source, &library_md5s(&library)),
            ("MINE".to_string(), vec![("1".to_string(), vec![1])])
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_source_that_cannot_be_loaded_keeps_its_slot_with_no_levels() {
        let library = Library::from_songs(vec![entry("aaaa")]);
        let source = TableSource { name: "gone".into(), location: "/no/such/table.json".into() };
        let (name, levels) = load_and_match(&nowhere(), LoadPolicy::SavedFirst, &source, &library_md5s(&library));
        assert_eq!(name, "GONE", "the source name still labels the row");
        assert!(levels.is_empty(), "a failed load must not drop the row and misalign the parallel vecs");
    }

    /// A body file written for one test, removed when the test is over.
    struct BodyFile(std::path::PathBuf);

    impl BodyFile {
        fn written(tag: &str, body: &[u8]) -> BodyFile {
            let path = std::env::temp_dir().join(format!("rbms-tablesrc-{tag}-{}.json", std::process::id()));
            std::fs::write(&path, body).expect("write the fixture");
            BodyFile(path)
        }

        fn source(&self, name: &str) -> TableSource {
            TableSource { name: name.into(), location: self.0.to_string_lossy().to_string() }
        }
    }

    impl Drop for BodyFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn table_named(name: &str) -> rbms_table::DifficultyTable {
        let mut table = rbms_table::DifficultyTable::from_parts(None, Vec::new());
        table.name = name.to_string();
        table
    }

    /// Most tables are named in Japanese. A name is shown as its table wrote it, so two of them are
    /// two folders that can be told apart rather than the same generic label twice.
    #[test]
    fn a_table_is_called_what_it_calls_itself_in_whatever_script_that_is() {
        let unnamed = TableSource { name: String::new(), location: "https://tables.test/a/".into() };
        let table_name = "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868}";
        assert_eq!(pick_table_name(&unnamed, &table_named(table_name)), table_name);
        assert_eq!(pick_table_name(&unnamed, &table_named("Satellite")), "SATELLITE", "Latin letters are in the list's capitals");
        assert_eq!(pick_table_name(&unnamed, &table_named("  ")), UNNAMED_TABLE, "only a table with no name gets the generic label");

        let named = TableSource { name: " \u{7b2c}2 ".into(), location: "https://tables.test/b/".into() };
        assert_eq!(pick_table_name(&named, &table_named("Satellite")), "\u{7b2c}2", "the name the user gave the source comes first");
        assert_eq!(fallback_source_name(&named), "\u{7b2c}2");
        assert_eq!(fallback_source_name(&unnamed), UNLOADED_TABLE);
    }

    /// A table none of whose charts are in the library is still a table that loaded: it comes back
    /// as its levels with nothing in them, which is what makes it a folder rather than nothing.
    #[test]
    fn a_table_nothing_is_owned_of_comes_back_as_its_levels_with_nothing_in_them() {
        let body = BodyFile::written("unowned", br#"[{"md5":"1111","level":"2"},{"md5":"2222","level":"1"},{"md5":"3333","level":"2"}]"#);
        let source = body.source("");
        let library = Library::from_songs(vec![entry("aaaa"), entry("bbbb")]);

        let (name, levels) = load_and_match(&nowhere(), LoadPolicy::SavedFirst, &source, &library_md5s(&library));
        assert_eq!(name, UNNAMED_TABLE);
        assert_eq!(levels, vec![("1".to_string(), Vec::new()), ("2".to_string(), Vec::new())]);
        assert_eq!(
            table_status(&source.location),
            Some(TableStatus::Loaded { name: UNNAMED_TABLE.to_string(), entries: 3, levels: 2, owned: 0, origin: TableOrigin::File, stale: None }),
            "the table manager can say the table loaded and that none of it is owned"
        );
        assert!(!table_load_failed(&source.location));
    }

    #[test]
    fn what_became_of_a_load_is_remembered_for_the_table_manager_until_the_source_is_removed() {
        let body = BodyFile::written("status", br#"[{"md5":"BBBB","level":"1"},{"md5":"cccc","level":7},{"level":"1"}]"#);
        let source = body.source("mine");
        let library = Library::from_songs(vec![entry("aaaa"), entry("bbbb")]);
        assert_eq!(table_status(&source.location), None, "nothing is known of a source that has not been loaded");

        load_and_match(&nowhere(), LoadPolicy::SavedFirst, &source, &library_md5s(&library));
        assert_eq!(
            table_status(&source.location),
            Some(TableStatus::Loaded { name: "MINE".to_string(), entries: 2, levels: 2, owned: 1, origin: TableOrigin::File, stale: None })
        );

        std::fs::write(&body.0, b"<html>not a table</html>").expect("rewrite the fixture");
        load_and_match(&nowhere(), LoadPolicy::SavedFirst, &source, &library_md5s(&library));
        let Some(TableStatus::Failed { reason }) = table_status(&source.location) else {
            panic!("the later failure replaces the earlier success");
        };
        assert!(reason.starts_with("body parse:"), "the reason is the loader's own: {reason}");
        assert!(table_load_failed(&source.location));

        forget_table_status(&source.location);
        assert_eq!(table_status(&source.location), None);
    }

    /// The failure of a step is reported with the address that step was asking — unless that is the
    /// address the user registered, which the message already leads with.
    #[test]
    fn a_failure_is_worded_as_the_step_the_address_and_the_reason() {
        let registered = "https://tables.test/t/";
        let failed_at = |stage, url: &str| rbms_table::TableError::At { stage, url: url.to_string(), source: Box::new(rbms_table::TableError::MissingDataUrl) };
        assert_eq!(fetch_failure_reason(registered, &failed_at(rbms_table::TableStage::Address, registered)), "address: header.json has no data_url");
        assert_eq!(
            fetch_failure_reason(registered, &failed_at(rbms_table::TableStage::Header, "https://tables.test/t/header.json")),
            "header https://tables.test/t/header.json: header.json has no data_url"
        );
    }

    /// The sources are loaded side by side, and what comes back is still one slot per source in the
    /// order the sources were given, whichever finished first.
    #[test]
    fn sources_loaded_side_by_side_keep_the_order_they_were_listed_in() {
        let library = Library::from_songs(vec![entry("aaaa"), entry("bbbb")]);
        let bodies: Vec<BodyFile> = (0..MAX_PARALLEL_LOADS + 3).map(|i| BodyFile::written(&format!("order-{i}"), br#"[{"md5":"bbbb","level":"1"}]"#)).collect();
        let mut sources: Vec<TableSource> = bodies.iter().enumerate().map(|(i, body)| body.source(&format!("t{i}"))).collect();
        sources.insert(2, TableSource { name: "gone".into(), location: "/no/such/side-by-side.json".into() });

        let done = AtomicUsize::new(0);
        let (names, levels) = fetch_and_match(&nowhere(), &sources, &library, &done);
        assert_eq!(done.load(Ordering::Relaxed), sources.len());
        let expected: Vec<String> = sources.iter().map(|source| source.name.to_ascii_uppercase()).collect();
        assert_eq!(names, expected);
        for (at, lv) in levels.iter().enumerate() {
            let expected: TableLevels = if at == 2 { Vec::new() } else { vec![("1".to_string(), vec![1])] };
            assert_eq!(*lv, expected, "slot {at}: the source that failed has no levels and holds up none of the others");
        }
    }

    /// A folder of one test's own, with a settings file's place in it, the table folder beside
    /// that and a folder standing in for the temporary one. Removed when the test is over.
    struct Home(PathBuf);

    impl Home {
        fn of(tag: &str) -> Home {
            let folder = std::env::temp_dir().join(format!("rbms-tablesrc-home-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(folder.join("legacy")).expect("the test's folder is made");
            Home(folder)
        }

        fn settings(&self) -> PathBuf {
            self.0.join("settings.ron")
        }

        fn legacy(&self) -> PathBuf {
            self.0.join("legacy")
        }

        fn store(&self, tables: &test_outside::FakeTables) -> TableStore {
            TableStore { folder: tables_folder(&self.settings()), outside: Outside { legacy_folder: self.legacy(), fetch: tables.fetcher() } }
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const ONE_LEVEL: &[u8] = br#"[{"md5":"bbbb","level":"1"}]"#;
    const TWO_LEVELS: &[u8] = br#"[{"md5":"bbbb","level":"1"},{"md5":"aaaa","level":"2"}]"#;

    fn two_charts() -> Library {
        Library::from_songs(vec![entry("aaaa"), entry("bbbb")])
    }

    fn at(address: &str) -> TableSource {
        TableSource { name: String::new(), location: address.to_string() }
    }

    /// Wait for the reloads of the player whose settings are at `settings`, and answer everything
    /// they came back with and the tally of each.
    fn landed(settings: &Path) -> (Vec<ReloadDone>, Vec<(usize, usize)>) {
        let (mut done, mut tallies) = (Vec::new(), Vec::new());
        while tallies.is_empty() {
            let (more, finished) = take_landed(settings, &|_| true);
            done.extend(more);
            tallies.extend(finished);
            std::thread::yield_now();
        }
        (done, tallies)
    }

    /// The first load of a table at an address fetches it and keeps it; that copy is the whole of
    /// it, with where and when it came from.
    #[test]
    fn a_table_that_is_fetched_is_kept_as_a_copy_beside_the_settings() {
        let address = "https://tables.test/kept/";
        let home = Home::of("kept");
        let tables = test_outside::FakeTables::default();
        tables.serve(address, "Kept", TWO_LEVELS);
        let store = home.store(&tables);
        let before = unix_now();

        let (name, levels) = load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library_md5s(&two_charts()));
        assert_eq!((name.as_str(), tables.asked()), ("KEPT", 1));
        assert_eq!(levels, vec![("1".to_string(), vec![1]), ("2".to_string(), vec![0])]);

        let copy = home.0.join(TABLES_FOLDER).join(format!("{}.{COPY_EXTENSION}", url_hash(address)));
        let saved = SavedTable::read(&copy).expect("the copy is in the table folder beside the settings, named after the address");
        assert_eq!((saved.table.name.as_str(), saved.table.entries.len(), saved.source.as_str()), ("Kept", 2, address));
        let fetched_at = saved.fetched_at.expect("the copy says when it was fetched");
        assert!(fetched_at >= before, "the copy is dated before it was fetched");
        assert_eq!(
            table_status(address),
            Some(TableStatus::Loaded {
                name: "KEPT".into(),
                entries: 2,
                levels: 2,
                owned: 2,
                origin: TableOrigin::Address { fetched_at: Some(fetched_at) },
                stale: None
            })
        );
        let left_over: Vec<_> = std::fs::read_dir(home.0.join(TABLES_FOLDER)).expect("the table folder").flatten().map(|file| file.file_name()).collect();
        assert_eq!(left_over.len(), 1, "the copy is written whole and renamed into place, with nothing left beside it: {left_over:?}");
        forget_table_status(address);
    }

    /// A table that has a copy is read from it. Nothing is asked of the network, whether or not the
    /// address would answer.
    #[test]
    fn a_table_that_has_a_copy_is_read_from_it_and_not_fetched() {
        let address = "https://tables.test/not-fetched/";
        let home = Home::of("not-fetched");
        let tables = test_outside::FakeTables::default();
        tables.serve(address, "First", ONE_LEVEL);
        let store = home.store(&tables);
        let library = library_md5s(&two_charts());
        load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library);
        let first = table_status(address);
        assert_eq!(tables.asked(), 1);

        tables.serve(address, "Second", TWO_LEVELS);
        let (name, levels) = load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library);
        assert_eq!(tables.asked(), 1, "a table that has a copy was fetched");
        assert_eq!((name.as_str(), levels.len()), ("FIRST", 1), "the table is the copy, not what the address serves now");
        assert_eq!(table_status(address), first, "the copy still says when it was fetched");

        tables.take_down(address);
        let (name, levels) = load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library);
        assert_eq!((name.as_str(), levels, tables.asked()), ("FIRST", vec![("1".to_string(), vec![1])], 1), "the copy is all a load needs");
        forget_table_status(address);
    }

    /// The step the LOADING screen calls MATCHING TABLES, in a run where every table has a copy:
    /// every table folder is there and no server was asked for anything.
    #[test]
    fn with_a_copy_of_every_table_and_no_server_every_table_folder_is_there() {
        let home = Home::of("no-server");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let sources: Vec<TableSource> = (0..MAX_PARALLEL_LOADS + 2).map(|i| at(&format!("https://tables.test/no-server/{i}/"))).collect();
        for (i, source) in sources.iter().enumerate() {
            store.keep_for_tests(&source.location, &format!("t{i}"), ONE_LEVEL, Some(1));
        }

        let done = AtomicUsize::new(0);
        let (names, levels) = fetch_and_match(&store, &sources, &two_charts(), &done);
        assert_eq!(tables.asked(), 0, "a run in which every table has a copy asked the network");
        assert_eq!(done.load(Ordering::Relaxed), sources.len());
        assert_eq!(names, (0..sources.len()).map(|i| format!("T{i}")).collect::<Vec<_>>());
        for (slot, lv) in levels.iter().enumerate() {
            assert_eq!(*lv, vec![("1".to_string(), vec![1])], "slot {slot}");
        }
        for source in &sources {
            assert!(!table_load_failed(&source.location));
            forget_table_status(&source.location);
        }
    }

    /// Only the tables that have no copy are fetched: one newly registered among ones that were
    /// fetched before costs one fetch.
    #[test]
    fn only_a_table_with_no_copy_is_fetched_among_ones_that_have_one() {
        let home = Home::of("one-new");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let sources = [at("https://tables.test/one-new/old/"), at("https://tables.test/one-new/new/")];
        store.keep_for_tests(&sources[0].location, "old", ONE_LEVEL, Some(1));
        tables.serve(&sources[1].location, "new", TWO_LEVELS);

        let (names, levels) = fetch_and_match(&store, &sources, &two_charts(), &AtomicUsize::new(0));
        assert_eq!((names, tables.asked()), (vec!["OLD".to_string(), "NEW".to_string()], 1));
        assert_eq!(levels[1].len(), 2);
        assert!(store.has_copy_file(&sources[1].location), "the table that was fetched now has a copy too");
        sources.iter().for_each(|source| forget_table_status(&source.location));
    }

    /// A copy that does not read as one — cut short by a run that was stopped, or damaged since — is
    /// no copy: the table is fetched again and the copy replaced.
    #[test]
    fn a_damaged_copy_is_fetched_again_and_replaced() {
        let address = "https://tables.test/damaged/";
        let home = Home::of("damaged");
        let tables = test_outside::FakeTables::default();
        tables.serve(address, "Whole", ONE_LEVEL);
        let store = home.store(&tables);
        std::fs::create_dir_all(&store.folder).expect("the table folder");
        std::fs::write(store.copy_path(address), br#"{"name":"Whole","symbol":"*","level_order":["1"],"entr"#).expect("write half a copy");

        let (name, levels) = load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library_md5s(&two_charts()));
        assert_eq!((name.as_str(), levels.len(), tables.asked()), ("WHOLE", 1, 1), "half a copy was read as a table, or not fetched again");
        assert_eq!(store.copy_name(address).as_deref(), Some("Whole"), "the damaged copy was left in place");
        forget_table_status(address);
    }

    /// With half a copy and no server there is no table, and the row says the fetch failed: the
    /// damage is not shown as a table.
    #[test]
    fn a_damaged_copy_with_no_server_is_a_failed_load_rather_than_half_a_table() {
        let address = "https://tables.test/damaged-offline/";
        let home = Home::of("damaged-offline");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        std::fs::create_dir_all(&store.folder).expect("the table folder");
        std::fs::write(store.copy_path(address), b"not a copy").expect("write the damage");

        let (_, levels) = crate::notify::exclusive(|| load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library_md5s(&two_charts())));
        assert!(levels.is_empty());
        assert!(table_load_failed(address));
        forget_table_status(address);
    }

    /// Two addresses can come to one file name. A copy that says it is another address's is not
    /// read as this one's.
    #[test]
    fn a_copy_that_names_another_address_is_not_this_tables() {
        let address = "https://tables.test/collision/";
        let home = Home::of("collision");
        let tables = test_outside::FakeTables::default();
        tables.serve(address, "Mine", ONE_LEVEL);
        let store = home.store(&tables);
        let other = DifficultyTable::parse_body(TWO_LEVELS, None).expect("a body");
        let json = SavedTable::json_of(&other, "https://tables.test/somebody-else/", Some(1)).expect("a table is JSON");
        crate::write_atomic(&store.copy_path(address), &json).expect("write the other table's copy");

        let (name, levels) = load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library_md5s(&two_charts()));
        assert_eq!((name.as_str(), levels.len(), tables.asked()), ("MINE", 1, 1));
        forget_table_status(address);
    }

    /// The copies an earlier version kept were in the temporary folder, which the operating system
    /// empties. One that is still there is moved into the store the first time its table is loaded,
    /// dated by when its file was written, and the table is not fetched.
    #[test]
    fn a_copy_an_earlier_version_left_in_the_temporary_folder_is_moved_into_the_store() {
        let address = "https://tables.test/legacy/";
        let home = Home::of("legacy");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let legacy = home.legacy().join(format!("{LEGACY_COPY_PREFIX}{}.{COPY_EXTENSION}", url_hash(address)));
        let old = br#"{"name":"Earlier","symbol":"s","level_order":["2","1"],"entries":[{"md5":"aaaa","level":"2","title":"","artist":"","url":"","url_diff":""},{"md5":"bbbb","level":"1","title":"","artist":"","url":"","url_diff":""}]}"#;
        std::fs::write(&legacy, old).expect("write the earlier copy");
        let written = std::fs::metadata(&legacy)
            .and_then(|file| file.modified())
            .expect("the file's time")
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch")
            .as_secs();

        let (name, levels) = load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library_md5s(&two_charts()));
        assert_eq!((name.as_str(), tables.asked()), ("EARLIER", 0), "a table with an earlier copy was fetched");
        assert_eq!(levels, vec![("2".to_string(), vec![0]), ("1".to_string(), vec![1])], "the order the earlier copy kept");
        assert!(!legacy.exists(), "the earlier copy is moved, not copied");
        let moved = SavedTable::read(&store.copy_path(address)).expect("the copy is in the store now");
        assert_eq!((moved.source.as_str(), moved.fetched_at), (address, Some(written)), "the moved copy names its address and is dated by its file");

        load_and_match(&store, LoadPolicy::SavedFirst, &at(address), &library_md5s(&two_charts()));
        assert_eq!(tables.asked(), 0, "the moved copy is read like any other afterwards");
        forget_table_status(address);
    }

    /// Registering a table fetches what its address serves now, whatever copy happens to be lying
    /// about, and a registration that fails keeps nothing.
    #[test]
    fn registering_a_table_fetches_it_whatever_copy_is_lying_about() {
        let address = "https://tables.test/registered/";
        let home = Home::of("registered");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        store.keep_for_tests(address, "Lying about", ONE_LEVEL, Some(1));
        tables.serve(address, "Served", TWO_LEVELS);

        let (name, levels) = load_and_match(&store, LoadPolicy::Fetch, &at(address), &library_md5s(&two_charts()));
        assert_eq!((name.as_str(), levels.len(), tables.asked()), ("SERVED", 2, 1));
        assert_eq!(store.copy_name(address).as_deref(), Some("Served"));
        forget_table_status(address);
    }

    /// A reload fetches the table again on a worker, replaces the copy, and hands the table back
    /// for the frame loop to take. While it is out the source reads as being reloaded, and it is
    /// not started a second time.
    #[test]
    fn a_reload_fetches_the_table_again_and_replaces_the_copy() {
        let address = "https://tables.test/reloaded/";
        let home = Home::of("reloaded");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        store.keep_for_tests(address, "Before", ONE_LEVEL, Some(1));
        tables.serve(address, "After", TWO_LEVELS);

        let open = tables.hold();
        assert_eq!(start_reload(store.clone(), vec![at(address)]), 1);
        assert!(table_is_reloading(&home.settings(), address));
        assert_eq!(start_reload(store.clone(), vec![at(address)]), 0, "a table already on its way was started again");
        assert!(!table_is_reloading(Path::new("/some/other/player/settings.ron"), address), "another player's table reads as being reloaded");
        drop(open);

        let (done, tallies) = landed(&home.settings());
        assert_eq!(tallies, vec![(1, 0)]);
        assert!(matches!(&done[..], [ReloadDone { location, outcome: ReloadOutcome::Fetched(load) }] if location == address && load.table.name == "After"));
        assert!(!table_is_reloading(&home.settings(), address));
        assert_eq!((store.copy_name(address).as_deref(), tables.asked()), (Some("After"), 1));
        let saved = store.copy_of(address).expect("the new copy");
        assert!(saved.fetched_at.is_some_and(|at| at > 1), "the new copy is dated by the reload");
    }

    /// A reload that fails touches nothing: the copy is the one from before, and the failure comes
    /// back with its reason for the frame loop to show.
    #[test]
    fn a_reload_that_fails_leaves_the_copy_as_it_was() {
        let home = Home::of("reload-failed");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let down = "https://tables.test/reload-failed/down/";
        let emptied = "https://tables.test/reload-failed/emptied/";
        store.keep_for_tests(down, "Down", ONE_LEVEL, Some(1));
        store.keep_for_tests(emptied, "Emptied", ONE_LEVEL, Some(1));
        tables.serve(emptied, "Emptied", b"[]");

        assert_eq!(start_reload(store.clone(), vec![at(down), at(emptied)]), 2);
        let (done, tallies) = landed(&home.settings());
        assert_eq!(tallies, vec![(0, 2)]);
        let reason_of = |address: &str| {
            done.iter().find(|done| done.location == address).map(|done| match &done.outcome {
                ReloadOutcome::Kept { reason } => reason.clone(),
                ReloadOutcome::Fetched(_) => "fetched".to_string(),
                ReloadOutcome::Removed => "removed".to_string(),
            })
        };
        assert_eq!(reason_of(down), Some(test_outside::refused().to_string()), "the fetch's own failure is the reason");
        assert_eq!(reason_of(emptied).as_deref(), Some(LISTS_NOTHING_NOW), "a table that lists nothing now replaced a copy that lists something");
        for address in [down, emptied] {
            let saved = store.copy_of(address).expect("the copy is still there");
            assert_eq!((saved.table.entries.len(), saved.fetched_at), (1, Some(1)), "{address}: the copy was touched");
        }
    }

    /// Reloading a table registered as a file reads the file again; there is no copy of it to
    /// write.
    #[test]
    fn reloading_a_table_file_reads_the_file_again() {
        let home = Home::of("reload-file");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let body = BodyFile::written("reload-file", ONE_LEVEL);
        std::fs::write(&body.0, TWO_LEVELS).expect("rewrite the fixture");

        assert_eq!(start_reload(store.clone(), vec![body.source("mine")]), 1);
        let (done, tallies) = landed(&home.settings());
        assert_eq!(tallies, vec![(1, 0)]);
        assert!(
            matches!(&done[..], [ReloadDone { outcome: ReloadOutcome::Fetched(load), .. }] if load.origin == TableOrigin::File && load.table.entries.len() == 2)
        );
        assert_eq!(tables.asked(), 0);
        assert!(!store.folder.exists(), "a copy was written of a table that is a file");
    }

    /// Removing a table drops its copy, in the store and in the temporary folder alike, and leaves
    /// a table file where it is.
    #[test]
    fn forgetting_a_table_drops_its_copy_and_never_a_table_file() {
        let address = "https://tables.test/forgotten/";
        let home = Home::of("forgotten");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        store.keep_for_tests(address, "Gone", ONE_LEVEL, Some(1));
        let legacy = store.legacy_path(address);
        std::fs::write(&legacy, b"{}").expect("write an earlier copy");
        let body = BodyFile::written("forgotten", ONE_LEVEL);

        store.forget(address);
        store.forget(&body.source("mine").location);
        assert!(!store.has_copy_file(address) && !legacy.exists(), "a copy of a removed table was left behind");
        assert!(body.0.exists(), "removing a table registered as a file deleted the file");
    }

    /// A table removed while its reload is out is never kept: the worker that fetched it writes no
    /// copy, with no screen up to clear one away, and its reload has no tally of a table that is
    /// neither updated nor failed.
    #[test]
    fn a_table_removed_while_its_reload_is_out_is_not_kept_by_the_worker() {
        let home = Home::of("reload-removed");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let (gone, stays) = ("https://tables.test/reload-removed/gone/", "https://tables.test/reload-removed/stays/");
        for address in [gone, stays] {
            store.keep_for_tests(address, "Before", ONE_LEVEL, Some(1));
            tables.serve(address, "After", TWO_LEVELS);
        }

        let open = tables.hold();
        assert_eq!(start_reload(store.clone(), vec![at(gone), at(stays)]), 2);
        store.forget(gone);
        assert!(!store.has_copy_file(gone));
        drop(open);
        let is_listed = |location: &str| location == stays;
        let (mut done, mut tallies) = (Vec::new(), Vec::new());
        while table_is_reloading(&home.settings(), stays) || table_is_reloading(&home.settings(), gone) {
            let (more, finished) = take_landed(&home.settings(), &is_listed);
            done.extend(more);
            tallies.extend(finished);
            std::thread::yield_now();
        }
        assert_eq!(tallies, vec![(1, 0)], "a table that was removed is counted in how its reload went");
        assert!(done.iter().any(|done| done.location == gone && matches!(done.outcome, ReloadOutcome::Removed)), "the worker kept a table that was removed");
        assert!(!store.has_copy_file(gone), "the worker wrote a copy of a table that was removed while it was out");
        assert_eq!(store.copy_name(stays).as_deref(), Some("After"), "removing one table held another's reload back");

        let open = tables.hold();
        assert_eq!(start_reload(store.clone(), vec![at(gone)]), 1);
        store.forget(gone);
        drop(open);
        while table_is_reloading(&home.settings(), gone) {
            let (_, finished) = take_landed(&home.settings(), &|_| false);
            assert!(finished.is_empty(), "a reload none of whose tables are left says how it went: {finished:?}");
            std::thread::yield_now();
        }
        assert!(!store.has_copy_file(gone));
    }

    #[test]
    fn a_copy_is_aged_in_the_largest_whole_unit_it_has_been_and_is_old_by_whole_days() {
        const NOW: u64 = 1_760_227_200;
        let aged = |seconds: u64| copy_age(NOW - seconds, NOW);
        assert_eq!((aged(0), aged(SECONDS_PER_MINUTE - 1)), ("JUST NOW".to_string(), "JUST NOW".to_string()));
        assert_eq!((aged(SECONDS_PER_MINUTE), aged(SECONDS_PER_HOUR - 1)), ("1 MINUTE AGO".to_string(), "59 MINUTES AGO".to_string()));
        assert_eq!((aged(SECONDS_PER_HOUR), aged(SECONDS_PER_DAY - 1)), ("1 HOUR AGO".to_string(), "23 HOURS AGO".to_string()));
        assert_eq!((aged(SECONDS_PER_DAY), aged(SECONDS_PER_DAY * 45 + 5)), ("1 DAY AGO".to_string(), "45 DAYS AGO".to_string()));
        assert_eq!(copy_age(NOW + SECONDS_PER_DAY, NOW), "JUST NOW", "a copy dated after now has an age");
        assert_eq!(copy_age_days(100, 100 + SECONDS_PER_DAY * 31 + 5), 31);
        assert_eq!(copy_age_days(100, 100 + SECONDS_PER_DAY - 1), 0);
        assert_eq!(copy_age_days(200, 100), 0, "a copy dated after now is not old");
    }

    /// The name of a copy's file is the hash it always was, so the copies an earlier version kept
    /// are found under the name that version gave them.
    #[test]
    fn the_file_a_copy_is_kept_in_is_named_after_its_address_the_same_way_every_run() {
        assert_eq!(url_hash("https://darksabun.club/table/archive/insane1/"), url_hash("https://darksabun.club/table/archive/insane1/"));
        assert_ne!(url_hash("https://tables.test/a/"), url_hash("https://tables.test/b/"));
        assert_eq!(url_hash(""), "0000000000001505", "the hash is the one the earlier copies were named with");
        assert_eq!(tables_folder(Path::new("/home/player/rbms/settings.ron")), Path::new("/home/player/rbms").join(TABLES_FOLDER));
        assert_eq!(tables_folder(Path::new("/home/player/rbms/songs.sqlite")), tables_folder(Path::new("/home/player/rbms/settings.ron")));
    }
}
