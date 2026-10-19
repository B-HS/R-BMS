//! The plays stored before their client kept a trace with them, and the traces later made for them
//! from their replays.
//!
//! A trace is kept beside a play as the play is stored ([`ScoreDb::keep_trace`]). A play stored by
//! an older client has none, and the player's own client can make one by judging the play's replay
//! again. How far that has got is marked in `meta`: every play up to the mark has a trace or will
//! never get one, so the next start goes on from there. A play past the mark that will never get
//! one is kept in `trace_backfill_refused`, so it is not judged again either; a play that could not
//! be looked at for now (its chart file or its replay not there) is in neither, and the next start
//! looks at it again. A trace made this way never replaces one the play already has.

use rusqlite::{Connection, OptionalExtension, params};

use super::rows::{log_columns_of, log_from_row};
use super::{PlayLog, ScoreDb, ScoreDbError};

/// `meta` key holding the newest `scorelog` id up to which every play has a trace or never will.
const META_TRACE_BACKFILL_THROUGH: &str = "trace_backfill_through";

/// What [`ScoreDb::trace_backfill_through`] answers before any play has been looked at.
pub const TRACE_BACKFILL_NOT_STARTED: i64 = 0;

/// A stored play with no trace, as a trace is made for it: its row, the submission body it was
/// kept with (`None` for a play stored before bodies were kept) and its replay file, which the
/// row names.
#[derive(Clone, Debug, PartialEq)]
pub struct UntracedPlay {
    pub log: PlayLog,
    pub body: Option<String>,
}

/// What one batch of the making came to, written in one transaction
/// ([`ScoreDb::keep_backfilled_traces`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BackfillBatch {
    /// The traces made, each with its play's id.
    pub traces: Vec<(i64, String)>,
    /// The plays that will never get one.
    pub refused: Vec<i64>,
    /// The mark the batch was made on, as the making read it or as the batch before set it.
    pub from: i64,
    /// The mark the batch moves to.
    pub through: i64,
}

/// What [`ScoreDb::keep_backfilled_traces`] wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackfillWritten {
    /// How many traces were kept: a play that has one by then keeps its own.
    pub kept: usize,
    /// Whether the mark moved, which it does only from where the batch was made on.
    pub marked: bool,
}

impl ScoreDb {
    /// The newest play id up to which every play has a trace or never will, or
    /// [`TRACE_BACKFILL_NOT_STARTED`].
    pub fn trace_backfill_through(&self) -> Result<i64, ScoreDbError> {
        mark_in(&self.conn)
    }

    /// Up to `limit` plays after the play `after` and up to the play `up_to` that kept a replay, have
    /// no trace and were not found never to get one, oldest first.
    pub fn untraced_plays(&self, after: i64, up_to: i64, limit: usize) -> Result<Vec<UntracedPlay>, ScoreDbError> {
        let sql = format!(
            "SELECT {}, s.body AS body FROM scorelog l LEFT JOIN play_submission s ON s.log_id = l.id \
             WHERE l.id > ?1 AND l.id <= ?2 AND l.replay_file IS NOT NULL AND NOT EXISTS(SELECT 1 FROM play_trace t WHERE t.log_id = l.id) \
             AND NOT EXISTS(SELECT 1 FROM trace_backfill_refused r WHERE r.log_id = l.id) \
             ORDER BY l.id ASC LIMIT ?3",
            log_columns_of("l")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![after, up_to, limit as i64], |row| Ok(UntracedPlay { log: log_from_row(row)?, body: row.get("body")? }))?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
    }

    /// Up to `limit` plays dated at or before the epoch second `ended_at_s`, newest stored first:
    /// what stood last in the history at that second.
    pub fn plays_ended_by(&self, ended_at_s: i64, limit: usize) -> Result<Vec<PlayLog>, ScoreDbError> {
        let sql = format!("SELECT {} FROM scorelog l WHERE l.date <= ?1 ORDER BY l.id DESC LIMIT ?2", log_columns_of("l"));
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![ended_at_s, limit as i64], log_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
    }

    /// Keep what `batch` came to in one transaction: its traces, its plays that will never get one,
    /// and its mark, which moves only when the mark stands where the batch was made on. A batch
    /// written after one that was not finds the mark where that one left it, and leaves it there,
    /// so the plays of the batch not written are looked at again on the next start.
    pub fn keep_backfilled_traces(&mut self, batch: &BackfillBatch) -> Result<BackfillWritten, ScoreDbError> {
        let tx = self.conn.transaction()?;
        let mut kept = 0;
        for (log_id, body) in &batch.traces {
            kept += tx.execute("INSERT OR IGNORE INTO play_trace(log_id, body) VALUES(?1, ?2)", params![log_id, body])?;
        }
        for log_id in &batch.refused {
            tx.execute("INSERT OR IGNORE INTO trace_backfill_refused(log_id) VALUES(?1)", [log_id])?;
        }
        let marked = mark_in(&tx)? == batch.from;
        if marked {
            tx.execute("INSERT OR REPLACE INTO meta(key, value) VALUES(?1, ?2)", params![META_TRACE_BACKFILL_THROUGH, batch.through.to_string()])?;
            tx.execute("DELETE FROM trace_backfill_refused WHERE log_id <= ?1", [batch.through])?;
        }
        tx.commit()?;
        Ok(BackfillWritten { kept, marked })
    }
}

/// The mark `conn` holds, or [`TRACE_BACKFILL_NOT_STARTED`].
fn mark_in(conn: &Connection) -> Result<i64, ScoreDbError> {
    let raw: Option<String> = conn.query_row("SELECT value FROM meta WHERE key = ?1", [META_TRACE_BACKFILL_THROUGH], |row| row.get(0)).optional()?;
    Ok(raw.and_then(|value| value.parse().ok()).unwrap_or(TRACE_BACKFILL_NOT_STARTED))
}
