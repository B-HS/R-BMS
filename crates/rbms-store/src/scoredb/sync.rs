//! Where each play stands with a score server: the body it was finished with, kept so it can be sent
//! again as it was, and what each server it was sent to answered.
//!
//! Two tables, both added beside the play history rather than into it, so a build that predates them
//! reads the same database and ignores them:
//!
//! * `play_submission` — one row per play, the submission JSON the result screen built and the
//!   account the play was made under (`NULL` for a guest).
//! * `ir_sync` — one row per play and score server ([`IrSyncRow`]). A play with no row for a server
//!   has never been confirmed there, which is why the `scorelog.ir_submitted` flag of a play
//!   written before these tables needs no migration: it said only that the play was handed over.

use std::collections::HashMap;

use rusqlite::types::Value;
use rusqlite::{Row, params, params_from_iter};

use super::migrate::ASSIST_UNRECORDED;
use super::rows::{log_columns_of, log_from_row};
use super::{PlayLog, ScoreDb, ScoreDbError};
use crate::{SCRATCH_AUTO_ASSIST, SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION};

/// Where a play stands with one score server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncState {
    /// The server holds the play.
    Accepted,
    /// The server refused the play; it is sent again on the next sync, since a change on the server
    /// may let it through.
    Rejected,
    /// The play was not answered (no connection, a failure of the server's own, a rate limit); it is
    /// sent again on the next sync.
    Retry,
    /// The play was not sent, for a reason of this machine's (`error_code` says which).
    Skipped,
}

impl SyncState {
    pub(super) const ALL: [SyncState; 4] = [SyncState::Accepted, SyncState::Rejected, SyncState::Retry, SyncState::Skipped];

    /// The token the state is stored as.
    pub const fn as_token(self) -> &'static str {
        match self {
            SyncState::Accepted => "accepted",
            SyncState::Rejected => "rejected",
            SyncState::Retry => "retry",
            SyncState::Skipped => "skipped",
        }
    }

    /// A stored token as the state it names. A token this build does not know reads as
    /// [`SyncState::Retry`], so the play is sent again rather than taken for one the server holds.
    pub fn from_token(token: &str) -> SyncState {
        SyncState::ALL.into_iter().find(|state| state.as_token() == token).unwrap_or(SyncState::Retry)
    }
}

/// Where a play's replay stands with one score server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayState {
    /// The play kept no replay.
    None,
    /// The score was taken and the replay is still to go up (AUTO UPLOAD REPLAY off included).
    Pending,
    /// The server holds the replay.
    Uploaded,
    /// The upload was not answered; it goes up on a later sync.
    Failed,
    /// The server refused the replay.
    Rejected,
    /// The replay does not belong on the server: the score was kept out of the ranking, or the run
    /// was judged by widths the upload cannot state.
    Skipped,
    /// The replay file is no longer on the machine.
    Missing,
}

impl ReplayState {
    pub(super) const ALL: [ReplayState; 7] = [
        ReplayState::None,
        ReplayState::Pending,
        ReplayState::Uploaded,
        ReplayState::Failed,
        ReplayState::Rejected,
        ReplayState::Skipped,
        ReplayState::Missing,
    ];

    /// The token the state is stored as.
    pub const fn as_token(self) -> &'static str {
        match self {
            ReplayState::None => "none",
            ReplayState::Pending => "pending",
            ReplayState::Uploaded => "uploaded",
            ReplayState::Failed => "failed",
            ReplayState::Rejected => "rejected",
            ReplayState::Skipped => "skipped",
            ReplayState::Missing => "missing",
        }
    }

    /// A stored token as the state it names. A token this build does not know reads as
    /// [`ReplayState::Pending`], so the replay is tried again.
    pub fn from_token(token: &str) -> ReplayState {
        ReplayState::ALL.into_iter().find(|state| state.as_token() == token).unwrap_or(ReplayState::Pending)
    }

    /// Whether a later sync still has this replay to send.
    pub const fn still_to_send(self) -> bool {
        matches!(self, ReplayState::Pending | ReplayState::Failed | ReplayState::Rejected)
    }
}

/// One `ir_sync` row: what one score server last answered about one play.
#[derive(Clone, Debug, PartialEq)]
pub struct IrSyncRow {
    /// The `scorelog` id of the play.
    pub log_id: i64,
    /// The score server, as the URL the player set with its surrounding blanks and trailing `/` cut.
    pub server: String,
    /// The account the play was last sent under.
    pub login_id: String,
    pub state: SyncState,
    /// The id the server stored the play under, once it holds it.
    pub score_id: Option<String>,
    /// HTTP status of a refusal or of a failure that came with one.
    pub http_status: Option<u16>,
    /// The server's error code, or for [`SyncState::Skipped`] a token of this machine's own.
    pub error_code: Option<String>,
    /// The request field a refusal named.
    pub error_field: Option<String>,
    /// The reason a refusal gave, or the line that describes a failure.
    pub error_reason: Option<String>,
    /// How many times the play was sent to this server.
    pub attempts: u32,
    /// Epoch millisecond of the last send.
    pub last_attempt_at_ms: i64,
    pub replay_state: ReplayState,
    /// The id the server stored the replay under.
    pub replay_id: Option<String>,
    /// The last thing that went wrong with the replay.
    pub replay_error: Option<String>,
}

impl IrSyncRow {
    /// A row stating `state` for the play `log_id` on `server`, and nothing else yet.
    pub fn new(log_id: i64, server: &str, login_id: &str, state: SyncState) -> IrSyncRow {
        IrSyncRow {
            log_id,
            server: server.to_string(),
            login_id: login_id.to_string(),
            state,
            score_id: None,
            http_status: None,
            error_code: None,
            error_field: None,
            error_reason: None,
            attempts: 0,
            last_attempt_at_ms: 0,
            replay_state: ReplayState::None,
            replay_id: None,
            replay_error: None,
        }
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<IrSyncRow> {
        Ok(IrSyncRow {
            log_id: row.get("log_id")?,
            server: row.get("server")?,
            login_id: row.get("login_id")?,
            state: SyncState::from_token(&row.get::<_, String>("state")?),
            score_id: row.get("score_id")?,
            http_status: row.get("http_status")?,
            error_code: row.get("error_code")?,
            error_field: row.get("error_field")?,
            error_reason: row.get("error_reason")?,
            attempts: row.get("attempts")?,
            last_attempt_at_ms: row.get("last_attempt_at")?,
            replay_state: ReplayState::from_token(&row.get::<_, String>("replay_state")?),
            replay_id: row.get("replay_id")?,
            replay_error: row.get("replay_error")?,
        })
    }
}

/// One play a sync has to send: the play, the body it was finished with when one was kept, the
/// account it was made under, and where it stands with the server so far.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncCandidate {
    pub log: PlayLog,
    /// The submission JSON kept when the play ended; `None` for a play made before it was kept.
    pub body: Option<String>,
    /// The account the play was made under; `None` for a guest, or a play made before it was kept.
    pub played_as: Option<String>,
    pub state: Option<SyncState>,
    pub score_id: Option<String>,
    pub replay_state: Option<ReplayState>,
}

impl SyncCandidate {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<SyncCandidate> {
        Ok(SyncCandidate {
            log: log_from_row(row)?,
            body: row.get("body")?,
            played_as: row.get("played_as")?,
            state: row.get::<_, Option<String>>("sync_state")?.as_deref().map(SyncState::from_token),
            score_id: row.get("sync_score_id")?,
            replay_state: row.get::<_, Option<String>>("sync_replay_state")?.as_deref().map(ReplayState::from_token),
        })
    }
}

/// `error_code` of a SCRATCH AUTO play of a rule older than
/// [`SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION`], which a sync never sends: it counted the
/// turntable as perfect greats played for the player. A play carried over from `scores.ron` that
/// was assisted under such a rule, whose assist was not recorded ([`ASSIST_UNRECORDED`]), may have
/// been one of these and is held back the same way.
pub const SKIPPED_OLD_SCRATCH_AUTO_RULE: &str = "OLD_SCRATCH_AUTO_RULE";

impl ScoreDb {
    /// Keep the submission body a play was finished with, and the account it was made under.
    pub fn put_submission(&mut self, log_id: i64, body: &str, played_as: Option<&str>) -> Result<(), ScoreDbError> {
        self.conn.execute("INSERT OR REPLACE INTO play_submission(log_id, body, played_as) VALUES(?1, ?2, ?3)", params![log_id, body, played_as])?;
        Ok(())
    }

    /// Every play a sync to `server` under the account `login_id` has to send, oldest first: every
    /// play the server does not hold yet, whatever it answered before. Left out are the plays made
    /// under another account, and the SCRATCH AUTO plays of the rule before the turntable stopped
    /// counting (an imported play assisted under that rule, with the assist not recorded, among
    /// them). A guest's play, and one made before the account was kept, goes to whichever account
    /// syncs first. `only` narrows the walk to the named plays.
    pub fn sync_candidates(&self, server: &str, login_id: &str, only: Option<&[i64]>) -> Result<Vec<SyncCandidate>, ScoreDbError> {
        let mut bound: Vec<Value> = vec![
            Value::from(server.to_string()),
            Value::from(login_id.to_string()),
            Value::from(i64::from(SCRATCH_AUTO_ASSIST)),
            Value::from(i64::from(SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION)),
            Value::from(SyncState::Accepted.as_token().to_string()),
            Value::from(i64::from(ASSIST_UNRECORDED)),
        ];
        let narrowed = match only {
            Some(ids) => {
                let first = bound.len() + 1;
                bound.extend(ids.iter().map(|id| Value::from(*id)));
                format!(" AND l.id IN ({})", placeholders_from(first, ids.len()))
            }
            None => String::new(),
        };
        let sql = format!(
            "{} WHERE (y.state IS NULL OR y.state <> ?5) AND (s.played_as IS NULL OR s.played_as = ?2) AND NOT ((l.assist = ?3 OR l.assist = ?6) AND l.rule_version < ?4){narrowed} \
             ORDER BY l.date ASC, l.id ASC",
            candidate_select()
        );
        self.query_candidates(&sql, bound)
    }

    /// The plays the server holds whose replay is still to go up to it: taken under the account
    /// `login_id`, with a replay file kept, and an upload pending, failed or refused.
    pub fn replay_candidates(&self, server: &str, login_id: &str) -> Result<Vec<SyncCandidate>, ScoreDbError> {
        let still_to_send: Vec<&str> = ReplayState::ALL.into_iter().filter(|state| state.still_to_send()).map(ReplayState::as_token).collect();
        let mut bound: Vec<Value> =
            vec![Value::from(server.to_string()), Value::from(SyncState::Accepted.as_token().to_string()), Value::from(login_id.to_string())];
        let sql = format!(
            "{} WHERE y.state = ?2 AND y.login_id = ?3 AND l.replay_file IS NOT NULL AND y.replay_state IN ({}) ORDER BY l.date ASC, l.id ASC",
            candidate_select(),
            placeholders_from(bound.len() + 1, still_to_send.len())
        );
        bound.extend(still_to_send.into_iter().map(|token| Value::from(token.to_string())));
        self.query_candidates(&sql, bound)
    }

    fn query_candidates(&self, sql: &str, bound: Vec<Value>) -> Result<Vec<SyncCandidate>, ScoreDbError> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params_from_iter(bound), SyncCandidate::from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
    }

    /// Write what a server answered about a play's score, replacing what it said before and counting
    /// the send. `row.attempts` is not read: the count is kept here.
    pub fn put_sync_state(&mut self, row: &IrSyncRow) -> Result<(), ScoreDbError> {
        self.conn.execute(
            "INSERT INTO ir_sync(log_id, server, login_id, state, score_id, http_status, error_code, error_field, error_reason, attempts, \
             last_attempt_at, replay_state, replay_id, replay_error) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11, ?12, ?13) \
             ON CONFLICT(log_id, server) DO UPDATE SET login_id = excluded.login_id, state = excluded.state, score_id = excluded.score_id, \
             http_status = excluded.http_status, error_code = excluded.error_code, error_field = excluded.error_field, \
             error_reason = excluded.error_reason, attempts = ir_sync.attempts + 1, last_attempt_at = excluded.last_attempt_at, \
             replay_state = excluded.replay_state, replay_id = excluded.replay_id, replay_error = excluded.replay_error",
            params![
                row.log_id,
                row.server,
                row.login_id,
                row.state.as_token(),
                row.score_id,
                row.http_status,
                row.error_code,
                row.error_field,
                row.error_reason,
                row.last_attempt_at_ms,
                row.replay_state.as_token(),
                row.replay_id,
                row.replay_error,
            ],
        )?;
        Ok(())
    }

    /// Write what a server answered about a play's replay, leaving the score's columns as they are.
    pub fn put_replay_state(
        &mut self,
        log_id: i64,
        server: &str,
        state: ReplayState,
        replay_id: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), ScoreDbError> {
        self.conn.execute(
            "UPDATE ir_sync SET replay_state = ?3, replay_id = ?4, replay_error = ?5 WHERE log_id = ?1 AND server = ?2",
            params![log_id, server, state.as_token(), replay_id, error],
        )?;
        Ok(())
    }

    /// Where every play stands with `server`, keyed by its `scorelog` id. A play with no row has
    /// never been confirmed there.
    pub fn sync_states(&self, server: &str) -> Result<HashMap<i64, IrSyncRow>, ScoreDbError> {
        let mut stmt = self.conn.prepare("SELECT * FROM ir_sync WHERE server = ?1")?;
        let rows = stmt.query_map(params![server], IrSyncRow::from_row)?;
        let mut out = HashMap::new();
        for row in rows {
            let row = row?;
            out.insert(row.log_id, row);
        }
        Ok(out)
    }

    /// Mark every play [`ScoreDb::sync_candidates`] holds back for its SCRATCH AUTO rule as not sent
    /// to `server` ([`SKIPPED_OLD_SCRATCH_AUTO_RULE`]), so a screen can say why, leaving a play that
    /// already has a row as it is. Returns how many plays were marked.
    pub fn mark_old_scratch_auto_skipped(&mut self, server: &str, now_ms: i64) -> Result<usize, ScoreDbError> {
        Ok(self.conn.execute(
            "INSERT OR IGNORE INTO ir_sync(log_id, server, state, error_code, last_attempt_at) \
             SELECT id, ?1, ?2, ?3, ?4 FROM scorelog WHERE (assist = ?5 OR assist = ?7) AND rule_version < ?6",
            params![
                server,
                SyncState::Skipped.as_token(),
                SKIPPED_OLD_SCRATCH_AUTO_RULE,
                now_ms,
                SCRATCH_AUTO_ASSIST,
                SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION,
                ASSIST_UNRECORDED
            ],
        )?)
    }

    /// Fill in the SHA-256 of a play that was stored without one (a play carried over from
    /// `scores.ron`), leaving one that has a SHA-256 as it is.
    pub fn backfill_sha256(&mut self, log_id: i64, sha256: &str) -> Result<(), ScoreDbError> {
        self.conn.execute("UPDATE scorelog SET sha256 = ?2 WHERE id = ?1 AND sha256 = ''", params![log_id, sha256])?;
        Ok(())
    }
}

/// The select every candidate query starts from: the play's own columns, its kept body, and its row
/// for the server bound to `?1`.
fn candidate_select() -> String {
    format!(
        "SELECT {}, s.body AS body, s.played_as AS played_as, y.state AS sync_state, y.score_id AS sync_score_id, y.replay_state AS sync_replay_state \
         FROM scorelog l LEFT JOIN play_submission s ON s.log_id = l.id LEFT JOIN ir_sync y ON y.log_id = l.id AND y.server = ?1",
        log_columns_of("l")
    )
}

/// `?first, ?first+1, ...` for `count` bound values.
fn placeholders_from(first: usize, count: usize) -> String {
    (first..first + count).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ")
}
