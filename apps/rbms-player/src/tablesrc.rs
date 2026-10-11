//! Difficulty-table loading and library matching: read a table from the copy kept of it or fetch
//! it (`rbms_table`), match its entries to the local song library by md5
//! (`DifficultyTable::match_levels_in`), and resolve display names.
//!
//! A table at an address is fetched once and kept ([`TableStore`]): every later run reads the copy
//! and asks the network nothing, so the song list does not wait on somebody else's server each time
//! the player is started. A table registered as a file on this machine is read from that file every
//! time and has no copy.
//!
//! Nothing that reaches the network is waited for by a screen. Reading the library again reads the
//! copies and the files and nothing else ([`read_and_match_all`]); a table at an address that has no
//! copy is left out, and fetched once the song list is up ([`start_first_fetches`]). A table being
//! registered ([`register_table`]) and one the player asks for again ([`reload_tables`]) are fetched
//! the same way: on a worker beside the frame loop ([`TableFetches`]), and what the worker comes
//! back with is taken by whichever screen is up ([`apply_finished_fetches`]).
//!
//! What became of each source — loaded, where from and when, not fetched yet, or failed and why — is
//! kept here per location ([`table_status`]) for the table manager to show beside the source, so a
//! table that did not load says so where the user registered it instead of only being missing from
//! the song list.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use rbms_config::TableSource;
use rbms_library::Library;
use rbms_table::{DifficultyTable, Md5Index, SavedTable, TableError};

use crate::{AppShared, SelectItem, SelectView};

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
/// moment. It holds across everything one player has out at a time, however many times fetches
/// were started.
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
    /// The table is at an address and there is no copy of it to read: it has not been fetched yet,
    /// or what was kept of it no longer reads as a table. It is no folder until it has arrived.
    NotFetched,
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

/// Note that the table at `location` has no copy to read, unless a fetch of it has failed in this
/// run already. That failure is what goes on being shown beside the source, and it is what keeps a
/// table whose address does not answer from being fetched again each time the library is read:
/// only a table that is [`TableStatus::NotFetched`] is fetched by itself.
fn record_not_fetched(location: &str) {
    if let Ok(mut statuses) = statuses().lock()
        && !matches!(statuses.get(location), Some(TableStatus::Failed { .. }))
    {
        statuses.insert(location.to_string(), TableStatus::NotFetched);
    }
}

/// What became of the last load of the source at `location`, if it has been loaded this run.
pub(crate) fn table_status(location: &str) -> Option<TableStatus> {
    statuses().lock().ok()?.get(location).cloned()
}

/// Whether the last load of the source at `location` failed.
#[cfg(test)]
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
pub(crate) fn is_address(location: &str) -> bool {
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

    /// Drop the copy of a table that has been removed from the list, wherever it is kept. A fetch
    /// of that table that is still out has to be told first ([`TableFetches::forget`]), or it keeps
    /// the table again when it arrives; [`forget_table`] does both in that order.
    pub(crate) fn forget(&self, location: &str) {
        if !is_address(location) {
            return;
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

/// A table that was read, and where from.
struct TableLoad {
    table: DifficultyTable,
    origin: TableOrigin,
}

/// The table the `data.json` body file at `location` on this machine holds now.
fn read_table_file(location: &str) -> Result<TableLoad, TableSourceError> {
    let bytes = std::fs::read(location)?;
    Ok(TableLoad { table: DifficultyTable::parse_body(&bytes, None)?, origin: TableOrigin::File })
}

/// Read the table at `location` without asking the network anything: a file on this machine from
/// the file, and a table at an http(s) address from the copy kept of it. `None` is a table at an
/// address that has no copy, which is not a failure: it has only not been fetched yet.
fn read_table_source(store: &TableStore, location: &str) -> Result<Option<TableLoad>, TableSourceError> {
    if !is_address(location) {
        return read_table_file(location).map(Some);
    }
    Ok(store.saved(location).map(|saved| TableLoad { table: saved.table, origin: TableOrigin::Address { fetched_at: saved.fetched_at } }))
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

/// Read one table source without asking the network ([`read_table_source`]) and match it against
/// the library's md5s, indexed once for every table.
///
/// Taking the index rather than the library is what lets a read run on a worker thread: the browser
/// keeps reading the library it has meanwhile.
///
/// A source that cannot be read still answers, with no levels, and its failure is reported once
/// here: as a notification, and as the status the table manager shows beside the source. A table at
/// an address that has no copy answers with no levels too, and is noted as not fetched yet rather
/// than as a failure; nothing is said of it here, since fetching it is what happens next.
pub(crate) fn read_and_match(store: &TableStore, src: &TableSource, library: &Md5Index) -> (String, TableLevels) {
    println!("loading table: {} ({})", src.name, src.location);
    match read_table_source(store, &src.location) {
        Ok(Some(load)) => match_and_record(src, &load, library),
        Ok(None) => {
            record_not_fetched(&src.location);
            (fallback_source_name(src), Vec::new())
        }
        Err(e) => {
            let reason = failure_reason(&src.location, &e);
            crate::notify::notify(crate::notify::Level::Warn, format!("table load failed ({}): {reason}", src.location));
            record_status(&src.location, TableStatus::Failed { reason });
            (fallback_source_name(src), Vec::new())
        }
    }
}

/// Read every table source (one entry per source, aligned with `sources`). Returns parallel
/// `(display name, per-level owned-chart groups)` vecs.
///
/// Nothing here asks the network: a table that has a copy is read from it, a table file is read
/// from the file, and a table at an address that has no copy keeps its slot with no levels until it
/// has been fetched ([`start_first_fetches`]). So this takes as long as reading a few files does,
/// whatever the servers the tables came from are doing. The sources are read side by side, a few at
/// a time, and a source that fails keeps its slot, with no levels. The library is indexed once for
/// all of them.
///
/// `done` is raised as each source is finished, so the screen that started this off-thread can say
/// how far through the list it is rather than showing a bar that never moves.
pub(crate) fn read_and_match_all(store: &TableStore, sources: &[TableSource], library: &Library, done: &AtomicUsize) -> (Vec<String>, Vec<TableLevels>) {
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
                        let matched = read_and_match(store, src, index);
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

/// Why a table is being fetched, which is how its arrival and its failure are worded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FetchWhy {
    /// The player asked for the table as its address serves it now.
    Reload,
    /// The table has no copy, and the song list fetches it the first time by itself.
    First,
    /// The table has just been registered.
    Registered,
}

impl FetchWhy {
    /// What a fetch that failed is called, ahead of the table's location and the reason.
    fn failure(self) -> &'static str {
        match self {
            FetchWhy::Reload => "table reload failed",
            FetchWhy::First => "table fetch failed",
            FetchWhy::Registered => "table load failed",
        }
    }

    /// The line that says how the whole of a fetch went. A table being registered is one table,
    /// and what is said of that table when it lands is all there is to say. So is what is said of
    /// each table of a first fetch that brought nothing: every one of them has said why it failed,
    /// and an address that does not answer is asked again each time the player is started, so a line
    /// counting the failures would be said over again on every start.
    fn tally(self, updated: usize, failed: usize) -> Option<String> {
        match self {
            FetchWhy::Reload => Some(format!("difficulty tables reloaded: {updated} updated, {failed} failed")),
            FetchWhy::First => (updated > 0).then(|| format!("difficulty tables fetched for the first time: {updated} added, {failed} failed")),
            FetchWhy::Registered => None,
        }
    }
}

/// What a fetch of one source came to.
enum FetchOutcome {
    /// The table as its address serves it now, or as its file reads now. It has been kept as the
    /// copy already when it has one.
    Fetched(TableLoad),
    /// The fetch failed and nothing was touched: the copy and the folder in the song list are
    /// what they were.
    Kept { reason: String },
    /// The table was removed from the list while it was being fetched. Nothing was kept of what
    /// arrived, and there is nothing to bring up to date.
    Removed,
}

/// One source of a fetch, finished.
struct FetchDone {
    location: String,
    outcome: FetchOutcome,
}

/// How the whole of one fetch went, once its last table is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FetchTally {
    why: FetchWhy,
    updated: usize,
    failed: usize,
}

/// A fetch in flight: the sources it was asked for that have not answered yet, and how the ones
/// that have came out.
struct FetchJob {
    why: FetchWhy,
    rx: Receiver<FetchDone>,
    pending: HashSet<String>,
    /// The sources that were removed from the list while this fetch was out. The worker reads it
    /// before it keeps a table, and holds it while it does, so a copy is either written before its
    /// table is removed -- and dropped by the removal -- or not written at all.
    removed: Removed,
    updated: usize,
    failed: usize,
    /// The thread the fetch runs on, waited for when the last of its tables has been taken. That
    /// is a wait of no length -- the thread has nothing left to do but end -- and it is what makes
    /// a fetch that is over a thread that is gone.
    worker: Option<JoinHandle<()>>,
}

/// The locations removed from the list while a fetch of them was out, shared between the fetch
/// and its worker.
type Removed = Arc<Mutex<HashSet<String>>>;

/// What a lock holds, whether or not a thread stopped while holding it: a set of locations and a
/// count of free slots are whole after any change that began.
fn held<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How many more fetches may be out at this moment. Every fetch of one player takes a slot before
/// it asks its address anything and gives it back when it has its answer, so the cap on how many
/// are out at once ([`MAX_PARALLEL_LOADS`]) holds across fetches that were started separately: the
/// first fetch of the tables that have no copy, a table registered meanwhile, a reload of them all.
struct Slots {
    free: Mutex<usize>,
    freed: Condvar,
}

impl Slots {
    fn of(count: usize) -> Slots {
        Slots { free: Mutex::new(count), freed: Condvar::new() }
    }

    /// Wait for a slot and take it. It is given back when what this answers is dropped.
    fn take(&self) -> Slot<'_> {
        let mut free = held(&self.free);
        while *free == 0 {
            free = self.freed.wait(free).unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        *free -= 1;
        Slot(self)
    }
}

/// One slot of [`Slots`], held by the fetch that took it.
struct Slot<'a>(&'a Slots);

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        *held(&self.0.free) += 1;
        self.0.freed.notify_one();
    }
}

/// Fetch one source. A table at an address is kept as the new copy when it arrives; one whose
/// address now lists nothing does not replace a copy that lists something, and one that was removed
/// from the list while it was out (`removed`) is not kept at all. A table that is a file is read
/// from the file.
fn fetch_one(store: &TableStore, src: &TableSource, removed: &Mutex<HashSet<String>>) -> FetchOutcome {
    println!("fetching table: {} ({})", src.name, src.location);
    let location = src.location.as_str();
    if !is_address(location) {
        return match read_table_file(location) {
            Ok(load) => FetchOutcome::Fetched(load),
            Err(error) => FetchOutcome::Kept { reason: failure_reason(location, &error) },
        };
    }
    match (store.outside.fetch)(location) {
        Ok(table) if table.entries.is_empty() && store.copy_of(location).is_some_and(|saved| !saved.table.entries.is_empty()) => {
            FetchOutcome::Kept { reason: LISTS_NOTHING_NOW.to_string() }
        }
        Ok(table) => {
            let removed = held(removed);
            if removed.contains(location) {
                return FetchOutcome::Removed;
            }
            let fetched_at = store.save(location, &table);
            FetchOutcome::Fetched(TableLoad { table, origin: TableOrigin::Address { fetched_at: Some(fetched_at) } })
        }
        Err(error) => FetchOutcome::Kept { reason: fetch_failure_reason(location, &error) },
    }
}

/// Every fetch one player has out, and whether the tables that have no copy are due their first.
///
/// A fetch outlives the screen that asked for it -- the table manager can be left, and the song
/// list is left for a chart -- so the fetches are kept with the state every screen shares and taken
/// by whichever screen is up when they land. Each player has its own, so one player's fetches are
/// never another's to wait for or to take.
///
/// Dropping this with fetches out waits for none of them: closing the player is not held up by an
/// address that does not answer. A worker that arrives afterwards finds nobody to hand its table
/// to; the copy it keeps is written whole and renamed into place, so being stopped partway leaves
/// the earlier copy or none, never half of one.
pub(crate) struct TableFetches {
    jobs: Vec<FetchJob>,
    slots: Arc<Slots>,
    first_due: bool,
}

impl Default for TableFetches {
    fn default() -> TableFetches {
        TableFetches { jobs: Vec::new(), slots: Arc::new(Slots::of(MAX_PARALLEL_LOADS)), first_due: false }
    }
}

impl TableFetches {
    /// Whether a fetch of the source at `location` is out: asked for, and not yet taken by a
    /// screen.
    pub(crate) fn is_fetching(&self, location: &str) -> bool {
        self.jobs.iter().any(|job| job.pending.contains(location))
    }

    /// Whether a fetch of the source at `location` is out that will be kept when it lands. One
    /// whose table was removed meanwhile is out too, and is nothing to a table registered at the
    /// same address since: that table needs a fetch of its own.
    fn will_land(&self, location: &str) -> bool {
        self.jobs.iter().any(|job| job.pending.contains(location) && !held(&job.removed).contains(location))
    }

    /// Start fetching `sources` on a worker, and answer how many were started: a source that is
    /// being fetched already is not started twice.
    ///
    /// Nothing waits for the worker. What it comes back with is taken by
    /// [`apply_finished_fetches`], which is also where the song list is brought up to date.
    pub(crate) fn start(&mut self, store: TableStore, sources: Vec<TableSource>, why: FetchWhy) -> usize {
        let mut pending = HashSet::new();
        let fresh: Vec<TableSource> = sources.into_iter().filter(|src| !self.will_land(&src.location) && pending.insert(src.location.clone())).collect();
        if fresh.is_empty() {
            return 0;
        }
        let started = fresh.len();
        let (tx, rx) = std::sync::mpsc::channel();
        let removed = Removed::default();
        let (slots, gone) = (Arc::clone(&self.slots), Arc::clone(&removed));
        let worker = std::thread::spawn(move || {
            for batch in fresh.chunks(MAX_PARALLEL_LOADS) {
                std::thread::scope(|scope| {
                    for src in batch {
                        let (store, slots, gone, tx) = (&store, &slots, &gone, tx.clone());
                        scope.spawn(move || {
                            let outcome = {
                                let _slot = slots.take();
                                fetch_one(store, src, gone)
                            };
                            let _ = tx.send(FetchDone { location: src.location.clone(), outcome });
                        });
                    }
                });
            }
        });
        self.jobs.push(FetchJob { why, rx, pending, removed, updated: 0, failed: 0, worker: Some(worker) });
        started
    }

    /// Tell every fetch of the source at `location` that is out that its table has been removed
    /// from the list, so the copy it would have kept on arriving is never written: a table removed
    /// while it is being fetched leaves nothing behind, whether or not a screen is up to take what
    /// the fetch comes back with.
    fn forget(&self, location: &str) {
        for job in self.jobs.iter().filter(|job| job.pending.contains(location)) {
            held(&job.removed).insert(location.to_string());
        }
    }

    /// Note that the library has just been read again, so the tables that turned out to have no
    /// copy are due their first fetch the next time the song list is up.
    pub(crate) fn first_fetches_are_due(&mut self) {
        self.first_due = true;
    }

    /// What has landed since the last look, with why each was fetched, and the tally of each fetch
    /// that is now over: how many of its tables were updated and how many failed.
    ///
    /// `is_listed` says whether a location is still one of the player's tables. A table that was
    /// removed while its fetch was out is in neither count -- it was not updated, and its fetch did
    /// not fail -- and a fetch none of whose tables are left has no tally to give. What such a fetch
    /// came back with is not the table's any more, whatever arrived: a table registered at the same
    /// address since is brought up to date by the fetch that was started for it.
    fn take_landed(&mut self, is_listed: &dyn Fn(&str) -> bool) -> (Vec<(FetchWhy, FetchDone)>, Vec<FetchTally>) {
        let mut landed = Vec::new();
        let mut finished = Vec::new();
        for job in &mut self.jobs {
            loop {
                match job.rx.try_recv() {
                    Ok(mut done) => {
                        job.pending.remove(&done.location);
                        if held(&job.removed).contains(&done.location) {
                            done.outcome = FetchOutcome::Removed;
                        }
                        match done.outcome {
                            FetchOutcome::Fetched(_) if is_listed(&done.location) => job.updated += 1,
                            FetchOutcome::Kept { .. } if is_listed(&done.location) => job.failed += 1,
                            FetchOutcome::Fetched(_) | FetchOutcome::Kept { .. } | FetchOutcome::Removed => {}
                        }
                        landed.push((job.why, done));
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        let removed = held(&job.removed);
                        for location in job.pending.drain() {
                            let outcome = if removed.contains(&location) {
                                FetchOutcome::Removed
                            } else {
                                job.failed += usize::from(is_listed(&location));
                                FetchOutcome::Kept { reason: LOADER_STOPPED.to_string() }
                            };
                            landed.push((job.why, FetchDone { location, outcome }));
                        }
                        break;
                    }
                }
            }
        }
        self.jobs.retain_mut(|job| {
            if !job.pending.is_empty() {
                return true;
            }
            if let Some(worker) = job.worker.take() {
                let _ = worker.join();
            }
            if job.updated + job.failed > 0 {
                finished.push(FetchTally { why: job.why, updated: job.updated, failed: job.failed });
            }
            false
        });
        (landed, finished)
    }
}

#[cfg(test)]
impl TableFetches {
    /// Whether nothing is out: every fetch that was started has been taken, and its thread is gone.
    pub(crate) fn is_idle(&self) -> bool {
        self.jobs.is_empty()
    }
}

/// Whether the source at `location` of this player is being fetched right now.
pub(crate) fn table_is_fetching(shared: &AppShared, location: &str) -> bool {
    shared.table_fetches.is_fetching(location)
}

/// Fetch again the table at `which` of the player's list, or every table of it, and say so.
pub(crate) fn reload_tables(shared: &mut AppShared, which: Option<usize>) {
    let tables = &shared.config.library.tables;
    let sources: Vec<TableSource> = match which {
        Some(at) => tables.get(at).cloned().into_iter().collect(),
        None => tables.clone(),
    };
    if sources.is_empty() {
        crate::notify::notify(crate::notify::Level::Info, "no difficulty tables to reload");
        return;
    }
    match shared.table_fetches.start(TableStore::beside(&shared.settings_path), sources, FetchWhy::Reload) {
        0 => crate::notify::notify(crate::notify::Level::Info, "the tables are being fetched already"),
        started => crate::notify::notify(crate::notify::Level::Info, format!("reloading {started} difficulty table(s)")),
    }
}

/// Start the first fetch of every table that has no copy, once after each time the library was
/// read ([`TableFetches::first_fetches_are_due`]); every other call does nothing.
///
/// The song list calls this each frame it is up, so the tables are fetched beside a list that is
/// already there to be used rather than ahead of it. Only a table that is not fetched yet is
/// started: one whose fetch has failed in this run stays failed until the player asks for it again
/// or the player is started again, so an address that does not answer is asked once a run.
pub(crate) fn start_first_fetches(shared: &mut AppShared) {
    if !std::mem::take(&mut shared.table_fetches.first_due) {
        return;
    }
    let sources: Vec<TableSource> =
        shared.config.library.tables.iter().filter(|source| table_status(&source.location) == Some(TableStatus::NotFetched)).cloned().collect();
    match shared.table_fetches.start(TableStore::beside(&shared.settings_path), sources, FetchWhy::First) {
        0 => {}
        started => crate::notify::notify(crate::notify::Level::Info, format!("fetching {started} difficulty table(s) for the first time")),
    }
}

/// Add `source` to the player's tables and start fetching it.
///
/// The source is a row of the list, and of the settings file, from here on, whatever becomes of its
/// table: one that cannot be fetched is there to be looked at, fetched again or removed. Its table
/// is fetched beside whichever screen is up, as it is when the player asks for it again, and is a
/// folder of the song list from the frame it lands. A table registered as a file is read on the
/// same worker rather than in the middle of a frame: a table file can be tens of thousands of
/// entries.
pub(crate) fn register_table(shared: &mut AppShared, source: TableSource) {
    let at = shared.config.library.tables.len();
    shared.config.library.tables.push(source.clone());
    set_table_slot(shared, at, fallback_source_name(&source), Vec::new());
    shared.save_settings();
    if is_address(&source.location) {
        record_status(&source.location, TableStatus::NotFetched);
    }
    shared.table_fetches.start(TableStore::beside(&shared.settings_path), vec![source], FetchWhy::Registered);
}

/// Let go of everything kept of the table at `location`, which has been removed from the list: what
/// became of it, the copy of it, and whatever a fetch of it that is still out would have kept.
pub(crate) fn forget_table(shared: &AppShared, location: &str) {
    forget_table_status(location);
    shared.table_fetches.forget(location);
    TableStore::beside(&shared.settings_path).forget(location);
}

/// Take what the fetches in flight have come back with since the last frame, and answer whether
/// anything was taken.
///
/// A table that arrived is matched against the library as it is now -- which may not be the library
/// its fetch was started beside -- and takes the place of the one in the song list, which is rebuilt
/// where it stands. A fetch that failed leaves the table, its copy and its folder as they were, and
/// says why: beside the source in the table manager, and as a notification. When the last table of
/// a fetch is in, one line says how the whole of it went.
///
/// A fetch is asked for by location and answered by location. A table that was removed while its
/// fetch was out is simply not there to update, and is not counted in how the fetch went; the copy
/// the fetch may have written of it is dropped. A location the list holds more than once -- a
/// settings file written by hand can -- is one table fetched once, and every row of it is brought
/// up to date.
pub(crate) fn apply_finished_fetches(shared: &mut AppShared) -> bool {
    let tables = &shared.config.library.tables;
    let (landed, finished) = shared.table_fetches.take_landed(&|location| tables.iter().any(|source| source.location == location));
    if landed.is_empty() && finished.is_empty() {
        return false;
    }
    let store = TableStore::beside(&shared.settings_path);
    let index = Md5Index::new(shared.library.md5s());
    let place = ListPlace::of(shared);
    let mut list_changed = false;
    for (why, done) in landed {
        let rows: Vec<usize> =
            shared.config.library.tables.iter().enumerate().filter(|(_, source)| source.location == done.location).map(|(at, _)| at).collect();
        match done.outcome {
            FetchOutcome::Removed => {}
            FetchOutcome::Fetched(_) | FetchOutcome::Kept { .. } if rows.is_empty() => store.forget(&done.location),
            FetchOutcome::Fetched(load) => {
                for (nth, at) in rows.into_iter().enumerate() {
                    let (name, levels) = match_and_record(&shared.config.library.tables[at], &load, &index);
                    if nth == 0 && why == FetchWhy::Registered {
                        announce_registered(&name, &levels);
                    }
                    set_table_slot(shared, at, name, levels);
                }
                list_changed = true;
            }
            FetchOutcome::Kept { reason } => {
                crate::notify::notify(crate::notify::Level::Warn, format!("{} ({}): {reason}", why.failure(), done.location));
                let kept = match table_status(&done.location) {
                    Some(TableStatus::Loaded { name, entries, levels, owned, origin, .. }) => {
                        TableStatus::Loaded { name, entries, levels, owned, origin, stale: Some(reason) }
                    }
                    Some(TableStatus::Failed { .. } | TableStatus::NotFetched) | None => TableStatus::Failed { reason },
                };
                record_status(&done.location, kept);
            }
        }
    }
    if list_changed {
        show_the_list_again(shared, &place);
    }
    for tally in finished {
        let level = if tally.failed == 0 { crate::notify::Level::Info } else { crate::notify::Level::Warn };
        if let Some(line) = tally.why.tally(tally.updated, tally.failed) {
            crate::notify::notify(level, line);
        }
    }
    true
}

/// Say what became of a table that was registered and has arrived. Only one that became a folder is
/// announced as added: saying so of one that was read and lists nothing is what makes a missing
/// folder look like a bug in the song list.
fn announce_registered(name: &str, levels: &TableLevels) {
    if levels.is_empty() {
        crate::notify::notify(crate::notify::Level::Warn, format!("table read, but it lists no charts and has no folder: {name}"));
        return;
    }
    let owned: usize = levels.iter().map(|(_, charts)| charts.len()).sum();
    crate::notify::notify(crate::notify::Level::Info, format!("table added: {name} ({owned} local charts)"));
}

/// Put a table in the slot of its source. The names and the levels are kept in the order of the
/// sources; a slot that is not there yet is made.
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

/// The folder the cursor of the song list is on, as something that can be found again in a list
/// that has been rebuilt around it.
#[derive(Clone, PartialEq)]
enum FocusedFolder {
    /// A level of a table, by the table's place and the level's name: a table that arrives again
    /// can have a level before this one that it did not have.
    Level(usize, String),
    /// Any other folder, by where it leads.
    Leading(SelectView),
}

impl FocusedFolder {
    /// What the row `item` of the song list is, when it is a folder.
    fn of(shared: &AppShared, item: &SelectItem) -> Option<FocusedFolder> {
        match item {
            SelectItem::Song(_) => None,
            SelectItem::Folder { target: SelectView::TableLevel(table, level), .. } => Some(
                shared
                    .table_levels
                    .get(*table)
                    .and_then(|levels| levels.get(*level))
                    .map_or(FocusedFolder::Leading(SelectView::TableLevel(*table, *level)), |(name, _)| FocusedFolder::Level(*table, name.clone())),
            ),
            SelectItem::Folder { target, .. } => Some(FocusedFolder::Leading(*target)),
        }
    }
}

/// A place in the song list -- what it is looking into and the row its cursor is on -- kept as
/// something that can be found again after the tables under the list have changed.
///
/// A level is kept by its name: a fetch can add a level before it or take one away, and the level
/// at the same place in the table that arrived is then another level. A folder under the cursor is
/// kept as the folder it is: a table that arrives for the first time is a folder the list did not
/// have, put among the folders in the order of the sources, and every folder after it is one place
/// further down than it was. A chart under the cursor is kept by its place in the list: a table
/// arriving changes no list of charts but its own levels', and the browser lays its filter over the
/// list after the tables have been put in, so a place found among the unfiltered charts would not
/// be the chart's place in the list that is shown.
///
/// The place is the list's own while a table lands under it ([`apply_finished_fetches`]), and it is
/// what the browser keeps of where a search was made from for as long as the search is on: a table
/// that arrives meanwhile must not move what the search goes back to.
pub(crate) struct ListPlace {
    view: SelectView,
    /// The level the list was looking into ([`shown_level`]).
    shown: Option<(usize, String)>,
    /// The folder the cursor was on, or `None` when it was on a chart or over an empty list.
    focused: Option<FocusedFolder>,
    sel: usize,
}

impl ListPlace {
    /// Where the song list is now.
    pub(crate) fn of(shared: &AppShared) -> ListPlace {
        ListPlace {
            view: shared.select_view,
            shown: shown_level(shared),
            focused: shared.select_items.get(shared.sel).and_then(|item| FocusedFolder::of(shared, item)),
            sel: shared.sel,
        }
    }

    /// What this place is looking into, as the tables stand now. A level the table no longer has is
    /// the table's levels, and a table that has no levels left is what is still there: the top of
    /// the library.
    pub(crate) fn view(&self, shared: &AppShared) -> SelectView {
        let levels_of = |table: usize| shared.table_levels.get(table).map_or(0, Vec::len);
        let place_of = |table: usize| {
            let (_, name) = self.shown.as_ref().filter(|(shown_table, _)| *shown_table == table)?;
            shared.table_levels.get(table)?.iter().position(|(level, _)| level == name)
        };
        match self.view {
            SelectView::TableLevels(table) | SelectView::TableLevel(table, _) if levels_of(table) == 0 => SelectView::Root,
            SelectView::TableLevel(table, _) => place_of(table).map_or(SelectView::TableLevels(table), |level| SelectView::TableLevel(table, level)),
            view => view,
        }
    }

    /// The row of this place in the list as it is now, the list having been built over
    /// [`ListPlace::view`]: the row of the folder the cursor was on, or the row the cursor was on
    /// when it was on a chart or its folder is gone, held to the rows there are.
    pub(crate) fn row(&self, shared: &AppShared) -> usize {
        let found =
            self.focused.as_ref().and_then(|focused| shared.select_items.iter().position(|item| FocusedFolder::of(shared, item).as_ref() == Some(focused)));
        found.unwrap_or(self.sel).min(shared.select_items.len().saturating_sub(1))
    }
}

/// Rebuild the song list where it stood after a table under it changed, `place` being where it
/// stood before the change: looking into the same level, with the cursor on the folder it was on.
fn show_the_list_again(shared: &mut AppShared, place: &ListPlace) {
    shared.select_view = place.view(shared);
    shared.rebuild_select_items();
    shared.sel = place.row(shared);
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

    /// Table addresses that live in a test: what each answers with, how often any was asked, how
    /// many were being asked at one moment at the most, and a gate a test can shut to hold every
    /// answer back until it opens it.
    #[derive(Clone, Default)]
    pub(crate) struct FakeTables {
        bodies: Arc<Mutex<HashMap<String, Answer>>>,
        asked: Arc<AtomicUsize>,
        out: Arc<AtomicUsize>,
        busiest: Arc<AtomicUsize>,
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

        /// How many fetches were out at one moment at the most: asked for, and not yet answered.
        pub(crate) fn busiest(&self) -> usize {
            self.busiest.load(Ordering::Relaxed)
        }

        /// One fetch. It is counted as out before it is counted as asked, so by the time a test has
        /// seen it asked for, it is in what [`FakeTables::busiest`] answers.
        fn fetch(&self, address: &str) -> Result<DifficultyTable, TableError> {
            let out = self.out.fetch_add(1, Ordering::SeqCst) + 1;
            self.busiest.fetch_max(out, Ordering::SeqCst);
            self.asked.fetch_add(1, Ordering::SeqCst);
            if let Some(shut) = self.gate.lock().expect("the gate").as_ref() {
                let _ = shut.recv();
            }
            let answer = self.answer(address);
            self.out.fetch_sub(1, Ordering::SeqCst);
            answer
        }

        fn answer(&self, address: &str) -> Result<DifficultyTable, TableError> {
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
        let error = match read_table_source(&nowhere(), missing.to_str().expect("utf-8 temp path")) {
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
        let error = match read_table_source(&nowhere(), path.to_str().expect("utf-8 temp path")) {
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
        let load =
            read_table_source(&nowhere(), path.to_str().expect("utf-8 temp path")).expect("a JSON body is a table").expect("a file is read, not waited for");
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

        let (name, levels) = read_and_match(&nowhere(), &source, &library_md5s(&library));
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
        let (names, levels) = read_and_match_all(&nowhere(), &sources, &library, &done);
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
        assert_eq!(read_and_match(&nowhere(), &source, &library_md5s(&library)), ("MINE".to_string(), vec![("1".to_string(), vec![1])]));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_source_that_cannot_be_loaded_keeps_its_slot_with_no_levels() {
        let library = Library::from_songs(vec![entry("aaaa")]);
        let source = TableSource { name: "gone".into(), location: "/no/such/table.json".into() };
        let (name, levels) = read_and_match(&nowhere(), &source, &library_md5s(&library));
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

        let (name, levels) = read_and_match(&nowhere(), &source, &library_md5s(&library));
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

        read_and_match(&nowhere(), &source, &library_md5s(&library));
        assert_eq!(
            table_status(&source.location),
            Some(TableStatus::Loaded { name: "MINE".to_string(), entries: 2, levels: 2, owned: 1, origin: TableOrigin::File, stale: None })
        );

        std::fs::write(&body.0, b"<html>not a table</html>").expect("rewrite the fixture");
        read_and_match(&nowhere(), &source, &library_md5s(&library));
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
        let (names, levels) = read_and_match_all(&nowhere(), &sources, &library, &done);
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

    /// Take what `fetches` come back with until nothing of them is out, and answer all of it and
    /// the tally of each fetch.
    fn landed(fetches: &mut TableFetches) -> (Vec<(FetchWhy, FetchDone)>, Vec<FetchTally>) {
        let (mut done, mut tallies) = (Vec::new(), Vec::new());
        while !fetches.is_idle() {
            let (more, finished) = fetches.take_landed(&|_| true);
            done.extend(more);
            tallies.extend(finished);
            std::thread::yield_now();
        }
        (done, tallies)
    }

    /// Fetch `sources` the way a screen asks for them, on a player of this test's own, and wait
    /// for all of it.
    fn fetched(store: &TableStore, sources: Vec<TableSource>, why: FetchWhy) -> (Vec<(FetchWhy, FetchDone)>, Vec<FetchTally>) {
        let mut fetches = TableFetches::default();
        fetches.start(store.clone(), sources, why);
        landed(&mut fetches)
    }

    /// The name of the table each fetch came back with, or how it came back with none.
    fn arrivals(done: &[(FetchWhy, FetchDone)]) -> Vec<(&str, String)> {
        done.iter()
            .map(|(_, done)| {
                let came_to = match &done.outcome {
                    FetchOutcome::Fetched(load) => load.table.name.clone(),
                    FetchOutcome::Kept { reason } => format!("kept: {reason}"),
                    FetchOutcome::Removed => "removed".to_string(),
                };
                (done.location.as_str(), came_to)
            })
            .collect()
    }

    /// A table at an address that is fetched is kept; that copy is the whole of it, with where and
    /// when it came from, and the next read of the table is the copy.
    #[test]
    fn a_table_that_is_fetched_is_kept_as_a_copy_beside_the_settings() {
        let address = "https://tables.test/kept/";
        let home = Home::of("kept");
        let tables = test_outside::FakeTables::default();
        tables.serve(address, "Kept", TWO_LEVELS);
        let store = home.store(&tables);
        let before = unix_now();

        let (done, tallies) = fetched(&store, vec![at(address)], FetchWhy::First);
        assert_eq!((arrivals(&done), tables.asked()), (vec![(address, "Kept".to_string())], 1));
        assert_eq!(tallies, vec![FetchTally { why: FetchWhy::First, updated: 1, failed: 0 }]);

        let copy = home.0.join(TABLES_FOLDER).join(format!("{}.{COPY_EXTENSION}", url_hash(address)));
        let saved = SavedTable::read(&copy).expect("the copy is in the table folder beside the settings, named after the address");
        assert_eq!((saved.table.name.as_str(), saved.table.entries.len(), saved.source.as_str()), ("Kept", 2, address));
        let fetched_at = saved.fetched_at.expect("the copy says when it was fetched");
        assert!(fetched_at >= before, "the copy is dated before it was fetched");
        let left_over: Vec<_> = std::fs::read_dir(home.0.join(TABLES_FOLDER)).expect("the table folder").flatten().map(|file| file.file_name()).collect();
        assert_eq!(left_over.len(), 1, "the copy is written whole and renamed into place, with nothing left beside it: {left_over:?}");

        let (name, levels) = read_and_match(&store, &at(address), &library_md5s(&two_charts()));
        assert_eq!((name.as_str(), tables.asked()), ("KEPT", 1), "the table that was kept is read from the copy");
        assert_eq!(levels, vec![("1".to_string(), vec![1]), ("2".to_string(), vec![0])]);
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
        fetched(&store, vec![at(address)], FetchWhy::First);
        assert_eq!(tables.asked(), 1);
        read_and_match(&store, &at(address), &library);
        let first = table_status(address);

        tables.serve(address, "Second", TWO_LEVELS);
        let (name, levels) = read_and_match(&store, &at(address), &library);
        assert_eq!(tables.asked(), 1, "a table that has a copy was fetched");
        assert_eq!((name.as_str(), levels.len()), ("FIRST", 1), "the table is the copy, not what the address serves now");
        assert_eq!(table_status(address), first, "the copy still says when it was fetched");

        tables.take_down(address);
        let (name, levels) = read_and_match(&store, &at(address), &library);
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
        let (names, levels) = read_and_match_all(&store, &sources, &two_charts(), &done);
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

    /// The same step with a table that has no copy among ones that have: nothing is asked of the
    /// network for it either, although its address would answer. It keeps its slot with no levels,
    /// it is counted as finished, and what is remembered of it is that it has not been fetched --
    /// not that anything failed, and nothing is said.
    #[test]
    fn a_table_with_no_copy_is_left_out_of_the_read_and_nothing_is_asked_of_its_address() {
        crate::notify::exclusive(|| {
            let home = Home::of("one-new");
            let tables = test_outside::FakeTables::default();
            let store = home.store(&tables);
            let sources = [at("https://tables.test/one-new/old/"), at("https://tables.test/one-new/new/")];
            store.keep_for_tests(&sources[0].location, "old", ONE_LEVEL, Some(1));
            tables.serve(&sources[1].location, "new", TWO_LEVELS);

            let done = AtomicUsize::new(0);
            let (names, levels) = read_and_match_all(&store, &sources, &two_charts(), &done);
            assert_eq!(tables.asked(), 0, "reading the library asked the address of a table that has no copy");
            assert_eq!((names, done.load(Ordering::Relaxed)), (vec!["OLD".to_string(), UNLOADED_TABLE.to_string()], 2));
            assert_eq!((levels[0].len(), levels[1].len()), (1, 0), "the table with no copy has no levels, and holds up none of the others");
            assert_eq!(table_status(&sources[1].location), Some(TableStatus::NotFetched));
            assert!(!table_load_failed(&sources[1].location), "a table that has not been fetched yet is reported as one that failed");
            assert!(!store.has_copy_file(&sources[1].location));
            read_and_match(&store, &sources[1], &library_md5s(&two_charts()));
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.is_empty(), "a table that is only not fetched yet was reported: {said:?}");
            sources.iter().for_each(|source| forget_table_status(&source.location));
        });
    }

    /// A table whose fetch failed in this run is not put back among the ones that are due a fetch
    /// when the library is read again: the failure stays, and with it the reason. A table that is a
    /// file is read again every time, whatever it came to before.
    #[test]
    fn a_table_whose_fetch_failed_this_run_stays_failed_when_the_library_is_read_again() {
        let address = "https://tables.test/stays-failed/";
        let home = Home::of("stays-failed");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let failed = TableStatus::Failed { reason: "address: no answer".into() };
        record_status(address, failed.clone());

        let (_, levels) = read_and_match(&store, &at(address), &library_md5s(&two_charts()));
        assert!(levels.is_empty());
        assert_eq!(table_status(address), Some(failed), "reading the library again forgot that the fetch had failed");
        assert_eq!(tables.asked(), 0);

        forget_table_status(address);
        read_and_match(&store, &at(address), &library_md5s(&two_charts()));
        assert_eq!(table_status(address), Some(TableStatus::NotFetched), "a table nothing is known of is one that has not been fetched");

        let body = BodyFile::written("stays-failed", ONE_LEVEL);
        record_status(&body.source("mine").location, TableStatus::Failed { reason: "file: gone".into() });
        read_and_match(&store, &body.source("mine"), &library_md5s(&two_charts()));
        assert!(!table_load_failed(&body.source("mine").location), "a table file that reads now is still shown as failed");
        forget_table_status(address);
        forget_table_status(&body.source("mine").location);
    }

    /// A copy that does not read as one — cut short by a run that was stopped, or damaged since — is
    /// no copy: the table reads as one that has not been fetched, never as half a table, and the
    /// fetch that follows replaces the damage.
    #[test]
    fn a_damaged_copy_is_no_table_and_is_replaced_by_the_fetch_that_follows() {
        let address = "https://tables.test/damaged/";
        let home = Home::of("damaged");
        let tables = test_outside::FakeTables::default();
        tables.serve(address, "Whole", ONE_LEVEL);
        let store = home.store(&tables);
        std::fs::create_dir_all(&store.folder).expect("the table folder");
        std::fs::write(store.copy_path(address), br#"{"name":"Whole","symbol":"*","level_order":["1"],"entr"#).expect("write half a copy");

        let (_, levels) = read_and_match(&store, &at(address), &library_md5s(&two_charts()));
        assert!(levels.is_empty(), "half a copy was read as a table");
        assert_eq!((table_status(address), tables.asked()), (Some(TableStatus::NotFetched), 0));

        let (done, _) = fetched(&store, vec![at(address)], FetchWhy::First);
        assert_eq!((arrivals(&done), tables.asked()), (vec![(address, "Whole".to_string())], 1));
        assert_eq!(store.copy_name(address).as_deref(), Some("Whole"), "the damaged copy was left in place");
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

        let (_, levels) = read_and_match(&store, &at(address), &library_md5s(&two_charts()));
        assert!(levels.is_empty(), "another address's copy was read as this table");
        assert_eq!(table_status(address), Some(TableStatus::NotFetched));

        let (done, _) = fetched(&store, vec![at(address)], FetchWhy::First);
        assert_eq!((arrivals(&done), tables.asked()), (vec![(address, "Mine".to_string())], 1));
        let (name, levels) = read_and_match(&store, &at(address), &library_md5s(&two_charts()));
        assert_eq!((name.as_str(), levels.len()), ("MINE", 1));
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

        let (name, levels) = read_and_match(&store, &at(address), &library_md5s(&two_charts()));
        assert_eq!((name.as_str(), tables.asked()), ("EARLIER", 0), "a table with an earlier copy was fetched");
        assert_eq!(levels, vec![("2".to_string(), vec![0]), ("1".to_string(), vec![1])], "the order the earlier copy kept");
        assert!(!legacy.exists(), "the earlier copy is moved, not copied");
        let moved = SavedTable::read(&store.copy_path(address)).expect("the copy is in the store now");
        assert_eq!((moved.source.as_str(), moved.fetched_at), (address, Some(written)), "the moved copy names its address and is dated by its file");

        read_and_match(&store, &at(address), &library_md5s(&two_charts()));
        assert_eq!(tables.asked(), 0, "the moved copy is read like any other afterwards");
        forget_table_status(address);
    }

    /// Registering a table fetches what its address serves now, whatever copy happens to be lying
    /// about. A table being registered is one table, so its fetch has no tally to give.
    #[test]
    fn registering_a_table_fetches_it_whatever_copy_is_lying_about() {
        let address = "https://tables.test/registered/";
        let home = Home::of("registered");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        store.keep_for_tests(address, "Lying about", ONE_LEVEL, Some(1));
        tables.serve(address, "Served", TWO_LEVELS);

        let (done, tallies) = fetched(&store, vec![at(address)], FetchWhy::Registered);
        assert_eq!((arrivals(&done), tables.asked()), (vec![(address, "Served".to_string())], 1));
        assert_eq!(store.copy_name(address).as_deref(), Some("Served"));
        assert_eq!(tallies.iter().filter_map(|tally| tally.why.tally(tally.updated, tally.failed)).count(), 0, "a registration was summed up like a reload");
    }

    /// A reload fetches the table again on a worker, replaces the copy, and hands the table back
    /// for the frame loop to take. While it is out the source reads as being fetched, and it is
    /// not started a second time. The fetches of one player are nothing to another.
    #[test]
    fn a_reload_fetches_the_table_again_and_replaces_the_copy() {
        let address = "https://tables.test/reloaded/";
        let home = Home::of("reloaded");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        store.keep_for_tests(address, "Before", ONE_LEVEL, Some(1));
        tables.serve(address, "After", TWO_LEVELS);

        let open = tables.hold();
        let mut fetches = TableFetches::default();
        assert_eq!(fetches.start(store.clone(), vec![at(address)], FetchWhy::Reload), 1);
        assert!(fetches.is_fetching(address));
        assert_eq!(fetches.start(store.clone(), vec![at(address)], FetchWhy::Reload), 0, "a table already on its way was started again");
        assert_eq!(fetches.start(store.clone(), vec![at(address)], FetchWhy::First), 0, "whatever it is fetched for, it is fetched once");
        let another = TableFetches::default();
        assert!(!another.is_fetching(address) && another.is_idle(), "another player's table reads as being fetched");
        drop(open);

        let (done, tallies) = landed(&mut fetches);
        assert_eq!(tallies, vec![FetchTally { why: FetchWhy::Reload, updated: 1, failed: 0 }]);
        assert_eq!(arrivals(&done), vec![(address, "After".to_string())]);
        assert!(done.iter().all(|(why, _)| *why == FetchWhy::Reload));
        assert!(!fetches.is_fetching(address));
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

        let (done, tallies) = fetched(&store, vec![at(down), at(emptied)], FetchWhy::Reload);
        assert_eq!(tallies, vec![FetchTally { why: FetchWhy::Reload, updated: 0, failed: 2 }]);
        let came_to = arrivals(&done);
        let reason_of = |address: &str| came_to.iter().find(|(location, _)| *location == address).map(|(_, came_to)| came_to.clone());
        assert_eq!(reason_of(down), Some(format!("kept: {}", test_outside::refused())), "the fetch's own failure is the reason");
        assert_eq!(reason_of(emptied), Some(format!("kept: {LISTS_NOTHING_NOW}")), "a table that lists nothing now replaced a copy that lists something");
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

        let (done, tallies) = fetched(&store, vec![body.source("mine")], FetchWhy::Reload);
        assert_eq!(tallies, vec![FetchTally { why: FetchWhy::Reload, updated: 1, failed: 0 }]);
        assert!(
            matches!(&done[..], [(_, FetchDone { outcome: FetchOutcome::Fetched(load), .. })] if load.origin == TableOrigin::File && load.table.entries.len() == 2)
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

    /// A table removed while its fetch is out is never kept: the worker that fetched it writes no
    /// copy, with no screen up to clear one away, and its fetch has no tally of a table that is
    /// neither updated nor failed.
    #[test]
    fn a_table_removed_while_its_fetch_is_out_is_not_kept_by_the_worker() {
        let home = Home::of("reload-removed");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let (gone, stays) = ("https://tables.test/reload-removed/gone/", "https://tables.test/reload-removed/stays/");
        for address in [gone, stays] {
            store.keep_for_tests(address, "Before", ONE_LEVEL, Some(1));
            tables.serve(address, "After", TWO_LEVELS);
        }

        let open = tables.hold();
        let mut fetches = TableFetches::default();
        assert_eq!(fetches.start(store.clone(), vec![at(gone), at(stays)], FetchWhy::Reload), 2);
        fetches.forget(gone);
        store.forget(gone);
        assert!(!store.has_copy_file(gone));
        drop(open);
        let is_listed = |location: &str| location == stays;
        let (mut done, mut tallies) = (Vec::new(), Vec::new());
        while !fetches.is_idle() {
            let (more, finished) = fetches.take_landed(&is_listed);
            done.extend(more);
            tallies.extend(finished);
            std::thread::yield_now();
        }
        assert_eq!(tallies, vec![FetchTally { why: FetchWhy::Reload, updated: 1, failed: 0 }], "a table that was removed is counted in how its fetch went");
        assert!(arrivals(&done).contains(&(gone, "removed".to_string())), "the worker kept a table that was removed");
        assert!(!store.has_copy_file(gone), "the worker wrote a copy of a table that was removed while it was out");
        assert_eq!(store.copy_name(stays).as_deref(), Some("After"), "removing one table held another's fetch back");

        let open = tables.hold();
        assert_eq!(fetches.start(store.clone(), vec![at(gone)], FetchWhy::First), 1);
        fetches.forget(gone);
        store.forget(gone);
        drop(open);
        while !fetches.is_idle() {
            let (_, finished) = fetches.take_landed(&|_| false);
            assert!(finished.is_empty(), "a fetch none of whose tables are left says how it went: {finished:?}");
            std::thread::yield_now();
        }
        assert!(!store.has_copy_file(gone));
    }

    /// A table removed while its fetch is out and registered again before that fetch is back is a
    /// new table at the same address: it is fetched for itself, the fetch of the table that was
    /// removed comes to nothing whatever arrives, and the copy that is left is the new table's.
    #[test]
    fn a_table_registered_again_while_the_fetch_of_the_one_removed_is_out_is_fetched_for_itself() {
        let address = "https://tables.test/again/";
        let home = Home::of("again");
        let tables = test_outside::FakeTables::default();
        tables.serve(address, "Again", ONE_LEVEL);
        let store = home.store(&tables);

        let open = tables.hold();
        let mut fetches = TableFetches::default();
        assert_eq!(fetches.start(store.clone(), vec![at(address)], FetchWhy::First), 1);
        fetches.forget(address);
        store.forget(address);
        assert_eq!(fetches.start(store.clone(), vec![at(address)], FetchWhy::Registered), 1, "the table registered again waits on a fetch that keeps nothing");
        assert_eq!(fetches.start(store.clone(), vec![at(address)], FetchWhy::Reload), 0, "the table registered again is fetched twice over");
        drop(open);

        let (done, tallies) = landed(&mut fetches);
        let by_why =
            |why: FetchWhy| done.iter().filter(|(of, _)| *of == why).map(|(_, done)| matches!(done.outcome, FetchOutcome::Fetched(_))).collect::<Vec<_>>();
        assert_eq!((by_why(FetchWhy::First), by_why(FetchWhy::Registered)), (vec![false], vec![true]));
        assert!(done.iter().any(|(why, done)| *why == FetchWhy::First && matches!(done.outcome, FetchOutcome::Removed)));
        assert_eq!(tallies, vec![FetchTally { why: FetchWhy::Registered, updated: 1, failed: 0 }], "the fetch of the removed table is counted");
        assert_eq!((tables.asked(), store.copy_name(address).as_deref()), (2, Some("Again")));
    }

    /// However many times fetches are started, no more than the cap are out at one moment: the
    /// tables of a second fetch wait for the first one's to come back, and every one of them is
    /// fetched in the end. A fetch that is over is a worker that is gone.
    #[test]
    fn fetches_started_separately_share_the_cap_on_how_many_are_out_at_once() {
        const EACH: usize = MAX_PARALLEL_LOADS - 2;
        let home = Home::of("cap");
        let tables = test_outside::FakeTables::default();
        let store = home.store(&tables);
        let sources = |batch: &str| (0..EACH).map(|i| at(&format!("https://tables.test/cap/{batch}/{i}/"))).collect::<Vec<_>>();
        for source in sources("first").iter().chain(&sources("second")) {
            tables.serve(&source.location, "Capped", ONE_LEVEL);
        }

        let open = tables.hold();
        let mut fetches = TableFetches::default();
        assert_eq!(fetches.start(store.clone(), sources("first"), FetchWhy::First), EACH);
        assert_eq!(fetches.start(store.clone(), sources("second"), FetchWhy::Reload), EACH);
        while tables.asked() < MAX_PARALLEL_LOADS {
            std::thread::yield_now();
        }
        drop(open);
        let (done, tallies) = landed(&mut fetches);

        assert_eq!(tables.busiest(), MAX_PARALLEL_LOADS, "more tables were being fetched at one moment than the cap allows");
        assert_eq!((done.len(), tables.asked()), (EACH * 2, EACH * 2), "a table that waited for a slot was never fetched");
        assert!(done.iter().all(|(_, done)| matches!(done.outcome, FetchOutcome::Fetched(_))));
        assert_eq!(tallies.len(), 2);
        assert!(fetches.is_idle(), "a fetch that is over is still held");
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
