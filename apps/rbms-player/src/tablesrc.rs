//! Difficulty-table loading and library matching: fetch/cache a table (`rbms_table`), match its
//! entries to the local song library by md5 (`DifficultyTable::match_levels`), and resolve
//! display names. Only `load_and_match_md5s` / `fetch_and_match` are called by the app to load;
//! the rest are internal steps. Both take the library's md5s rather than the library itself, so
//! every caller can do the work on a worker thread.
//!
//! What became of each source — loaded, loaded from the copy kept from last time, or failed and
//! why — is kept here per location ([`table_status`]) for the table manager to show beside the
//! source, so a table that did not load says so where the user registered it instead of only
//! being missing from the song list.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use rbms_config::TableSource;
use rbms_library::Library;

/// One difficulty table matched against the library: `(level label, indices of the owned charts)`
/// per level, in the table's own level order. A level none of whose charts are owned is there with
/// no indices; a table that could not be loaded has no levels at all.
pub(crate) type TableLevels = Vec<(String, Vec<usize>)>;

/// What a table is called when neither the user nor the table itself named it.
const UNNAMED_TABLE: &str = "DIFFICULTY TABLE";

/// What a source is called when its table could not be loaded and the user did not name it.
const UNLOADED_TABLE: &str = "TABLE";

/// How many sources are loaded at once. Each is a handful of http requests that mostly wait, so
/// the startup match takes as long as the slowest table rather than all of them one after another;
/// the cap is what keeps a long list from opening a connection per entry at the same moment.
const MAX_PARALLEL_LOADS: usize = 8;

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
        /// Why the table is the copy kept from an earlier fetch, when this fetch failed.
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

/// Stable per-URL filename for the on-disk table cache.
fn url_hash(s: &str) -> String {
    let mut h: u64 = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33) ^ b as u64;
    }
    format!("{h:016x}")
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

/// Load a table from a `location`: an http(s) URL (fetched + cached) or a local `data.json` body file.
fn load_table_source(location: &str) -> Result<rbms_table::TableLoad, TableSourceError> {
    if location.starts_with("http://") || location.starts_with("https://") {
        let cache = std::env::temp_dir().join(format!("rbms-table-{}.json", url_hash(location)));
        Ok(rbms_table::DifficultyTable::fetch_cached(location, &cache)?)
    } else {
        let bytes = std::fs::read(location)?;
        Ok(rbms_table::TableLoad { table: rbms_table::DifficultyTable::parse_body(&bytes, None)?, stale: None })
    }
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

/// Load one table source and match it against a copy of the library's md5s, in library order.
///
/// Taking the md5s rather than the library is what lets a fetch run on a worker thread: the browser
/// keeps reading the library it has while the http request is out.
///
/// A source that cannot be loaded still answers, with no levels, and its failure is reported once
/// here: as a notification, and as the status the table manager shows beside the source.
pub(crate) fn load_and_match_md5s(src: &TableSource, md5s: &[String]) -> (String, TableLevels) {
    println!("loading table: {} ({})", src.name, src.location);
    match load_table_source(&src.location) {
        Ok(load) => {
            let table = &load.table;
            let lv = table.match_levels(md5s.iter().map(String::as_str));
            let owned: usize = lv.iter().map(|(_, v)| v.len()).sum();
            println!("  {} entries ({} skipped), matched {owned} local charts across {} levels", table.entries.len(), table.skipped, lv.len());
            let stale = load.stale.as_ref().map(|fetch| fetch_failure_reason(&src.location, fetch));
            if let Some(reason) = &stale {
                crate::notify::notify(crate::notify::Level::Warn, format!("table not refreshed, showing the saved copy ({}): {reason}", src.location));
            }
            let name = pick_table_name(src, table);
            record_status(&src.location, TableStatus::Loaded { name: name.clone(), entries: table.entries.len(), levels: lv.len(), owned, stale });
            (name, lv)
        }
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
/// The sources are loaded side by side, a few at a time: each is a table's worth of http requests,
/// and one host that does not answer must not hold the others — or the song list they are all
/// waited for with — behind its timeout. A source that fails keeps its slot, with no levels.
///
/// `done` is raised as each source is finished, so the screen that started this off-thread can say
/// how far through the list it is rather than showing a bar that never moves.
pub(crate) fn fetch_and_match(sources: &[TableSource], library: &Library, done: &AtomicUsize) -> (Vec<String>, Vec<TableLevels>) {
    let md5s: Vec<String> = library.md5s().map(str::to_string).collect();
    let md5s = md5s.as_slice();
    let mut names = Vec::with_capacity(sources.len());
    let mut levels = Vec::with_capacity(sources.len());
    for batch in sources.chunks(MAX_PARALLEL_LOADS) {
        let loaded: Vec<(String, TableLevels)> = std::thread::scope(|scope| {
            let workers: Vec<_> = batch
                .iter()
                .map(|src| {
                    scope.spawn(move || {
                        let matched = load_and_match_md5s(src, md5s);
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
    let reason = "the table loader stopped unexpectedly".to_string();
    crate::notify::notify(crate::notify::Level::Warn, format!("table load failed ({}): {reason}", src.location));
    record_status(&src.location, TableStatus::Failed { reason });
    done.fetch_add(1, Ordering::Relaxed);
    (fallback_source_name(src), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_source_that_is_not_there_reports_the_read_failure_by_type() {
        let missing = std::env::temp_dir().join(format!("rbms-no-such-table-{}.json", std::process::id()));
        let error = match load_table_source(missing.to_str().expect("utf-8 temp path")) {
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
        let error = match load_table_source(path.to_str().expect("utf-8 temp path")) {
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
        let load = load_table_source(path.to_str().expect("utf-8 temp path")).expect("a JSON body is a table");
        assert!(load.stale.is_none(), "a file read off disk is never a saved copy of itself");
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

    fn library_md5s(library: &Library) -> Vec<String> {
        library.md5s().map(str::to_string).collect()
    }

    #[test]
    fn a_table_source_is_matched_against_the_library_index_and_reports_library_positions() {
        let path = std::env::temp_dir().join(format!("rbms-lib-table-{}.json", std::process::id()));
        std::fs::write(&path, br#"[{"md5":"BBBB","level":"1"},{"md5":"dddd","level":"2"}]"#).expect("write the fixture");
        let library = Library::from_songs(vec![entry("aaaa"), entry("bbbb"), entry("cccc"), entry("DDDD")]);
        let source = TableSource { name: "MINE".into(), location: path.to_string_lossy().to_string() };

        let (name, levels) = load_and_match_md5s(&source, &library_md5s(&library));
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
        let (names, levels) = fetch_and_match(&sources, &library, &done);
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
        assert_eq!(load_and_match_md5s(&source, &library_md5s(&library)), ("MINE".to_string(), vec![("1".to_string(), vec![1])]));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_source_that_cannot_be_loaded_keeps_its_slot_with_no_levels() {
        let library = Library::from_songs(vec![entry("aaaa")]);
        let source = TableSource { name: "gone".into(), location: "/no/such/table.json".into() };
        let (name, levels) = load_and_match_md5s(&source, &library_md5s(&library));
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

        let (name, levels) = load_and_match_md5s(&source, &library_md5s(&library));
        assert_eq!(name, UNNAMED_TABLE);
        assert_eq!(levels, vec![("1".to_string(), Vec::new()), ("2".to_string(), Vec::new())]);
        assert_eq!(
            table_status(&source.location),
            Some(TableStatus::Loaded { name: UNNAMED_TABLE.to_string(), entries: 3, levels: 2, owned: 0, stale: None }),
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

        load_and_match_md5s(&source, &library_md5s(&library));
        assert_eq!(table_status(&source.location), Some(TableStatus::Loaded { name: "MINE".to_string(), entries: 2, levels: 2, owned: 1, stale: None }));

        std::fs::write(&body.0, b"<html>not a table</html>").expect("rewrite the fixture");
        load_and_match_md5s(&source, &library_md5s(&library));
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
        let (names, levels) = fetch_and_match(&sources, &library, &done);
        assert_eq!(done.load(Ordering::Relaxed), sources.len());
        let expected: Vec<String> = sources.iter().map(|source| source.name.to_ascii_uppercase()).collect();
        assert_eq!(names, expected);
        for (at, lv) in levels.iter().enumerate() {
            let expected: TableLevels = if at == 2 { Vec::new() } else { vec![("1".to_string(), vec![1])] };
            assert_eq!(*lv, expected, "slot {at}: the source that failed has no levels and holds up none of the others");
        }
    }
}
