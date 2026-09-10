use std::path::{Path, PathBuf};

use super::{JudgeCounts, PLAY_STATE_MIGRATED, PlayLog, ScoreDb, ScoreDbError, UNSET_MINIMUM, chart_key, gauge_id, mode_id, random_id};
use crate::{ScoreBook, ScoreRecord};

/// Suffix the RON score book is renamed with once its records are in the database. The file is kept
/// rather than deleted: it is the only copy of the pre-migration history, and a build that predates
/// the database still reads it (spec §4.4-6).
const MIGRATED_SUFFIX: &str = "ron.migrated";

/// Milliseconds per second. `ScoreRecord::played_at` is a millisecond stamp (the player formats it
/// with `fmt_datetime`, which divides by this); every date in the database is a second, as the
/// reference stores it (`PlayDataAccessor.java:261`).
const MILLIS_PER_SECOND: i64 = 1_000;

/// Assist level of a record written before the level was recorded, only that there had been one.
/// Negative so it can never be read as one of the real levels (0 none, 1 light, 2 custom judge).
pub const ASSIST_UNRECORDED: i32 = -1;

/// Seed of a run that did not record one, matching the `score.seed` column default.
pub const SEED_UNRECORDED: i64 = -1;

/// What one import of a RON score book did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MigrationReport {
    /// Records replayed into the database.
    pub records: usize,
    /// How many of them were assisted, and so did not set bests.
    pub assisted: usize,
}

/// Replay a RON score book into the database, oldest play first, so the merge that
/// [`ScoreDb::record_play`] performs arrives at the same bests the book's own queries answer.
///
/// Each record's `assisted` flag becomes `updates_best = !assisted`, which is the same rule
/// [`ScoreBook::best_ex_for_md5`] and [`ScoreBook::best_clear_for_md5`] apply while reading, so the
/// bests do not move across the migration.
pub fn migrate_score_book(db: &mut ScoreDb, book: &ScoreBook) -> Result<MigrationReport, ScoreDbError> {
    let mut order: Vec<&ScoreRecord> = book.records().iter().collect();
    order.sort_by_key(|r| r.played_at);
    let mut report = MigrationReport::default();
    for record in order {
        db.record_play(&log_from_record(record), !record.assisted)?;
        report.records += 1;
        report.assisted += usize::from(record.assisted);
    }
    Ok(report)
}

/// Open the score database at `db_path`, importing `ron_path` the first time it is created.
///
/// The import runs only when the database file is absent and the RON book is present, so it happens
/// exactly once. If any part of it fails the half-written database is removed and the error is
/// returned with the RON book untouched, leaving the caller free to keep reading the book (spec
/// §4.4-6). On success the book is renamed out of the way and the report is returned.
pub fn open_with_import(db_path: &Path, ron_path: &Path) -> Result<(ScoreDb, Option<MigrationReport>), ScoreDbError> {
    let import = !db_path.exists() && ron_path.exists();
    let mut db = ScoreDb::open(db_path)?;
    match db.migrate() {
        Ok(_) => {}
        Err(e) => {
            drop(db);
            if import {
                remove_database(db_path);
            }
            return Err(e);
        }
    }
    if !import {
        return Ok((db, None));
    }
    let book = ScoreBook::load(ron_path);
    match migrate_score_book(&mut db, &book) {
        Ok(report) => {
            let target = retire_path(ron_path);
            if let Err(e) = std::fs::rename(ron_path, &target) {
                return Err(ScoreDbError::Io(e));
            }
            Ok((db, Some(report)))
        }
        Err(e) => {
            drop(db);
            remove_database(db_path);
            Err(e)
        }
    }
}

/// Delete a database and the two sidecar files write-ahead logging leaves next to it, so a failed
/// import leaves nothing the next launch would mistake for a finished one.
fn remove_database(db_path: &Path) {
    let _ = std::fs::remove_file(db_path);
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = db_path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(sidecar));
    }
}

/// Where an imported score book is moved to. A second import (the database was deleted and rebuilt)
/// does not overwrite the first book: the name is numbered until it is free.
fn retire_path(ron_path: &Path) -> PathBuf {
    let first = ron_path.with_extension(MIGRATED_SUFFIX);
    if !first.exists() {
        return first;
    }
    let mut n = 2;
    loop {
        let candidate = ron_path.with_extension(format!("{MIGRATED_SUFFIX}.{n}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// One RON record as the play it was.
///
/// Three things the record never held are entered as their unrecorded markers rather than invented:
/// the early/late split (the whole run reads as late, which leaves the totals and EX right), the
/// mean timing error, and the assist level behind the `assisted` flag. The rows are stamped
/// [`PLAY_STATE_MIGRATED`] so a statistic that needs any of them can leave them out.
fn log_from_record(record: &ScoreRecord) -> PlayLog {
    let judge = JudgeCounts::all_late(record.counts);
    let key = chart_key(&record.md5, "");
    PlayLog {
        chart_key: key.clone(),
        mode: mode_id(&record.mode),
        ln_mode: record.ln_mode.clone(),
        md5: key,
        sha256: String::new(),
        title: record.title.clone(),
        clear: record.clear,
        judge,
        notes: record.total_notes,
        combo: record.max_combo,
        minbp: judge.combo_breaks(),
        avgjudge: UNSET_MINIMUM,
        gauge: gauge_id(&record.gauge),
        gauge_value: record.gauge_value,
        assist: if record.assisted { ASSIST_UNRECORDED } else { 0 },
        option: 0,
        seed: SEED_UNRECORDED,
        random: random_id(&record.random),
        playtime_ms: 0,
        date: record.played_at.div_euclid(MILLIS_PER_SECOND),
        state: PLAY_STATE_MIGRATED,
        rule_version: record.rule_version,
        ir_submitted: false,
        replay_file: record.replay_file.clone(),
    }
}
