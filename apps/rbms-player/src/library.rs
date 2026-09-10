//! Player-side song database wiring: opening the library database, driving background scans and
//! serving the select screen its rows.
//!
//! The browser still works on [`Library`] and [`SongEntry`], so nothing above this module has to
//! know where the songs came from. What changes underneath is that the list is read back from
//! SQLite rather than rebuilt by re-parsing every chart on every run, and that the detail panel of
//! a focused song is a lookup rather than a full timing integration — the scan computed it already.
//!
//! A background scan opens its own connection rather than borrowing the one the browser reads
//! through: the database is in write-ahead-log mode exactly so a scan can commit batch after batch
//! while the browser keeps reading. The scan thread hands back the finished library, but a caller
//! that wants to show partial results can read [`stored_library`] through its own connection at any
//! point while the scan runs.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use rbms_library::scan::{ScanCounts, ScanOutcomeG, ScanProgress, ScanRequest, normalize, scan_into};
use rbms_library::songdb::{DetailRow, SongDb, SongDbError, SongRow, mode_from_id};
use rbms_library::{ChartDetail, Library, SongEntry, compute_chart_detail};
use rbms_model::Mode;

use crate::notify::{Level, notify};

/// Name of the song database inside the config directory, next to the settings and the scores.
pub(crate) const SONGDB_FILE: &str = "songdb.sqlite";

/// The mode a stored row is read as when its id belongs to no mode this build knows, which only
/// happens to a database written by a build with more modes than this one.
const FALLBACK_MODE: Mode = Mode::BEAT_7K;

/// Open the song database and bring its layout up to date, reporting a failure rather than taking
/// the player down with it: a run whose database cannot be opened still browses whatever a scan
/// hands it, exactly as the player behaved before there was a database at all.
pub(crate) fn open_song_db(path: &Path) -> Option<SongDb> {
    match open_migrated(path) {
        Ok(db) => Some(db),
        Err(e) => {
            notify(Level::Error, format!("song database not opened ({}): {e}", path.display()));
            None
        }
    }
}

/// Open and migrate, with the error left for the caller to report or carry.
fn open_migrated(path: &Path) -> Result<SongDb, SongDbError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return Err(SongDbError::Schema(format!("{e}")));
    }
    let mut db = SongDb::open(path)?;
    db.migrate()?;
    Ok(db)
}

/// One stored row as the browser's entry.
pub(crate) fn entry_of(row: &SongRow) -> SongEntry {
    SongEntry {
        path: PathBuf::from(&row.path),
        title: row.title.clone(),
        subtitle: row.subtitle.clone(),
        artist: row.artist.clone(),
        genre: row.genre.clone(),
        maker: row.maker.clone(),
        level: row.level.clone(),
        difficulty: row.difficulty,
        init_bpm: row.init_bpm,
        rank: row.judge,
        total: row.total,
        mode: mode_from_id(row.mode).unwrap_or(FALLBACK_MODE),
        md5: row.md5.clone(),
        stagefile: row.stagefile.clone(),
        banner: row.banner.clone(),
        preview: row.preview.clone(),
    }
}

/// Index stored rows into the library the browser reads, in the title order the in-memory scan
/// used to produce.
pub(crate) fn library_of(rows: Vec<SongRow>) -> Library {
    let mut songs: Vec<SongEntry> = rows.iter().map(entry_of).collect();
    songs.sort_by_key(|s| s.title.to_lowercase());
    Library::from_songs(songs)
}

/// Every chart the database holds, indexed for the browser. Safe to call while a scan is running:
/// it returns the batches committed so far.
pub(crate) fn stored_library(db: &SongDb) -> Library {
    match db.all_songs() {
        Ok(rows) => library_of(rows),
        Err(e) => {
            notify(Level::Error, format!("song list not read: {e}"));
            Library::default()
        }
    }
}

/// The detail panel of one chart: the cached row when the scan already computed it, and otherwise
/// a fresh integration that is cached on the way out so the next visit is a lookup.
pub(crate) fn chart_detail(db: &SongDb, path: &Path, mode: Mode) -> Option<ChartDetail> {
    let key = normalize(path);
    let row = db.song(&key).ok().flatten();
    if let Some(row) = &row
        && let Some(cached) = db.detail(&key).ok().flatten()
    {
        return Some(detail_of(row, &cached));
    }
    let computed = compute_chart_detail(path, mode)?;
    if row.is_some()
        && let Err(e) = db.put_detail(&key, &detail_row_of(&computed))
    {
        notify(Level::Warn, format!("chart detail not cached: {e}"));
    }
    Some(computed)
}

/// Assemble the browser's detail from the two rows that hold it: the note counts and BPM range on
/// the song, the integrated length and densities on its cached detail.
pub(crate) fn detail_of(row: &SongRow, cached: &DetailRow) -> ChartDetail {
    ChartDetail {
        notes: row.notes.max(0) as usize,
        long_notes: row.long_notes.max(0) as usize,
        duration_us: cached.duration_us,
        bpm_min: f64::from(row.min_bpm),
        bpm_max: f64::from(row.max_bpm),
        density: cached.density.clone(),
        peak_density: cached.peak_density,
        avg_density: cached.avg_density,
        end_density: cached.end_density,
    }
}

/// The cacheable half of a freshly computed detail.
pub(crate) fn detail_row_of(detail: &ChartDetail) -> DetailRow {
    DetailRow {
        duration_us: detail.duration_us,
        peak_density: detail.peak_density,
        avg_density: detail.avg_density,
        end_density: detail.end_density,
        density: detail.density.clone(),
    }
}

/// A background scan in flight: what it has done so far, the flag that stops it, and the report it
/// sends when it is over.
pub(crate) struct LibraryScan {
    rx: Receiver<ScanReport>,
    progress: Arc<ScanProgress>,
    cancel: Arc<AtomicBool>,
}

/// What a finished scan leaves behind.
pub(crate) struct ScanReport {
    /// The library as the database holds it now, including anything a cancelled scan committed.
    pub(crate) library: Library,
    pub(crate) upserted: usize,
    pub(crate) removed: usize,
    /// The scan stopped early, so nothing was deleted and the parse generation was not stamped.
    pub(crate) cancelled: bool,
    /// Why the scan could not run, if it could not.
    pub(crate) failure: Option<String>,
}

impl LibraryScan {
    /// How far the scan has got, for the progress screen.
    pub(crate) fn counts(&self) -> ScanCounts {
        self.progress.counts()
    }

    /// Ask the scan to stop at its next check. The thread finishes on its own; nothing is left
    /// walking the disk behind a screen that has been left.
    pub(crate) fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// The report, once the scan is over. `None` while it is still running; a scan whose thread
    /// died without reporting also reads as over, with an empty report.
    pub(crate) fn poll(&self) -> Option<ScanReport> {
        match self.rx.try_recv() {
            Ok(report) => Some(report),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(ScanReport {
                library: Library::default(),
                upserted: 0,
                removed: 0,
                cancelled: true,
                failure: Some("the library scan stopped without reporting".to_string()),
            }),
        }
    }
}

/// Start a background scan of `roots` into the database at `db_path`.
///
/// `full` re-reads every chart rather than only the ones whose files moved, which is what a parser
/// change needs; [`SongDb::needs_full_rescan`] is what answers that.
pub(crate) fn spawn_scan(db_path: PathBuf, roots: Vec<String>, full: bool) -> LibraryScan {
    let request = ScanRequest::new(roots, full);
    let progress = Arc::clone(&request.progress);
    let cancel = Arc::clone(&request.cancel);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(scan_now(&db_path, &request));
    });
    LibraryScan { rx, progress, cancel }
}

/// Run a scan on this thread, on its own connection, and report what it left behind.
///
/// This is what [`spawn_scan`] does inside the thread it starts. A caller that already owns a
/// worker thread and a cancel flag of its own — the loading screen does — drives the scan with this
/// and keeps its own plumbing, passing those into the [`ScanRequest`].
pub(crate) fn scan_now(db_path: &Path, request: &ScanRequest) -> ScanReport {
    let mut db = match open_migrated(db_path) {
        Ok(db) => db,
        Err(e) => {
            return ScanReport { library: Library::default(), upserted: 0, removed: 0, cancelled: false, failure: Some(e.to_string()) };
        }
    };
    let outcome = scan_into(&mut db, request);
    let library = stored_library(&db);
    match outcome {
        Ok(ScanOutcomeG::Completed { upserted, removed }) => ScanReport { library, upserted, removed, cancelled: false, failure: None },
        Ok(ScanOutcomeG::Cancelled) => ScanReport { library, upserted: 0, removed: 0, cancelled: true, failure: None },
        Err(e) => ScanReport { library, upserted: 0, removed: 0, cancelled: false, failure: Some(e.to_string()) },
    }
}

#[cfg(test)]
mod tests;
