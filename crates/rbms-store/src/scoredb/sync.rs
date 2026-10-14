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
//!
//! A refusal is written down with the builds it was given under ([`IrSyncRow::refused_under`]): this
//! client's and the server's. The same play sent again by the same client to the same server is
//! refused again, so a walk that is not asked to send every refusal again leaves it out until one of
//! the two builds changes ([`SyncWalk::refused_under`]).

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
    /// The server refused the play. It is sent again once the client or the server is another
    /// build, or when the player asks for every play to be sent again.
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
    /// The builds a refusal of the score was given under ([`SyncWalk::refused_under`]); `None` for
    /// a row that is no refusal, or one written before the builds were.
    pub refused_under: Option<String>,
    /// The same for a refusal of the replay.
    pub replay_refused_under: Option<String>,
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
            refused_under: None,
            replay_refused_under: None,
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
            refused_under: row.get("refused_under")?,
            replay_refused_under: row.get("replay_refused_under")?,
        })
    }
}

/// The submission body a play is to be sent as, and the account it was made under (`None` for a
/// guest), kept beside the play ([`ScoreDb::record_play_kept`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeptSubmission<'a> {
    pub body: &'a str,
    pub played_as: Option<&'a str>,
}

/// Keep `kept` as the body the play `log_id` is sent as, replacing one it had.
pub(super) fn insert_submission(conn: &rusqlite::Connection, log_id: i64, kept: KeptSubmission<'_>) -> Result<(), ScoreDbError> {
    conn.execute("INSERT OR REPLACE INTO play_submission(log_id, body, played_as) VALUES(?1, ?2, ?3)", params![log_id, kept.body, kept.played_as])?;
    Ok(())
}

/// What one walk of the history picks to send.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncWalk<'a> {
    /// The score server, as [`IrSyncRow::server`] keys it.
    pub server: &'a str,
    /// The account the walk sends under.
    pub login_id: &'a str,
    /// The plays the walk is narrowed to; `None` walks every play.
    pub only: Option<&'a [i64]>,
    /// The newest play the walk reaches ([`ScoreDb::newest_play_id`] when it began); a play stored
    /// after that is sent by whoever stored it. `None` reaches every play.
    pub up_to: Option<i64>,
    /// The newest course the walk reaches, as `up_to` is for plays.
    pub courses_up_to: Option<i64>,
    /// The builds the walk sends under, this client's and the server's: a play the server refused
    /// under the same builds is left out, since it would be refused again. `None` sends every
    /// refusal again, which is what the player asks for by hand.
    pub refused_under: Option<&'a str>,
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
        insert_submission(&self.conn, log_id, KeptSubmission { body, played_as })
    }

    /// Every play a walk has to send: every play the server does not hold yet, whatever it answered
    /// before, but for one it refused under the builds the walk sends under. Left out are the plays
    /// made under another account, and the SCRATCH AUTO plays of the rule before the turntable
    /// stopped counting (an imported play assisted under that rule, with the assist not recorded,
    /// among them). A guest's play, and one made before the account was kept, goes to whichever
    /// account syncs first.
    ///
    /// Oldest first, but a play the server failed to answer goes behind the plays it has not been
    /// asked about, and behind the plays it failed fewer times: a play that keeps failing on the
    /// server's side does not hold back every play after it.
    pub fn sync_candidates(&self, walk: &SyncWalk<'_>) -> Result<Vec<SyncCandidate>, ScoreDbError> {
        let mut bound: Vec<Value> = vec![
            Value::from(walk.server.to_string()),
            Value::from(walk.login_id.to_string()),
            Value::from(i64::from(SCRATCH_AUTO_ASSIST)),
            Value::from(i64::from(SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION)),
            Value::from(SyncState::Accepted.as_token().to_string()),
            Value::from(i64::from(ASSIST_UNRECORDED)),
            Value::from(SyncState::Retry.as_token().to_string()),
        ];
        let mut narrowed = String::new();
        if let Some(ids) = walk.only {
            let first = bound.len() + 1;
            bound.extend(ids.iter().map(|id| Value::from(*id)));
            narrowed.push_str(&format!(" AND l.id IN ({})", placeholders_from(first, ids.len())));
        }
        if let Some(up_to) = walk.up_to {
            bound.push(Value::from(up_to));
            narrowed.push_str(&format!(" AND l.id <= ?{}", bound.len()));
        }
        if let Some(key) = walk.refused_under {
            bound.push(Value::from(SyncState::Rejected.as_token().to_string()));
            bound.push(Value::from(key.to_string()));
            narrowed.push_str(&format!(" AND NOT COALESCE(y.state = ?{} AND y.refused_under = ?{}, 0)", bound.len() - 1, bound.len()));
        }
        let sql = format!(
            "{} WHERE (y.state IS NULL OR y.state <> ?5) AND (s.played_as IS NULL OR s.played_as = ?2) AND NOT ((l.assist = ?3 OR l.assist = ?6) AND l.rule_version < ?4){narrowed} \
             ORDER BY CASE WHEN y.state = ?7 THEN y.attempts ELSE 0 END ASC, l.date ASC, l.id ASC",
            candidate_select()
        );
        self.query_candidates(&sql, bound)
    }

    /// The plays the server holds whose replay is still to go up to it: taken under the walk's
    /// account, with a replay file kept, and an upload pending, failed or refused — but for one the
    /// server refused under the builds the walk sends under ([`SyncWalk::refused_under`]).
    pub fn replay_candidates(&self, walk: &SyncWalk<'_>) -> Result<Vec<SyncCandidate>, ScoreDbError> {
        let still_to_send: Vec<&str> = ReplayState::ALL.into_iter().filter(|state| state.still_to_send()).map(ReplayState::as_token).collect();
        let mut bound: Vec<Value> =
            vec![Value::from(walk.server.to_string()), Value::from(SyncState::Accepted.as_token().to_string()), Value::from(walk.login_id.to_string())];
        let states = placeholders_from(bound.len() + 1, still_to_send.len());
        bound.extend(still_to_send.into_iter().map(|token| Value::from(token.to_string())));
        let mut narrowed = String::new();
        if let Some(up_to) = walk.up_to {
            bound.push(Value::from(up_to));
            narrowed.push_str(&format!(" AND l.id <= ?{}", bound.len()));
        }
        if let Some(key) = walk.refused_under {
            bound.push(Value::from(ReplayState::Rejected.as_token().to_string()));
            bound.push(Value::from(key.to_string()));
            narrowed.push_str(&format!(" AND NOT COALESCE(y.replay_state = ?{} AND y.replay_refused_under = ?{}, 0)", bound.len() - 1, bound.len()));
        }
        let sql = format!(
            "{} WHERE y.state = ?2 AND y.login_id = ?3 AND l.replay_file IS NOT NULL AND y.replay_state IN ({states}){narrowed} ORDER BY l.date ASC, l.id ASC",
            candidate_select(),
        );
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
             last_attempt_at, replay_state, replay_id, replay_error, refused_under, replay_refused_under) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11, ?12, ?13, ?14, ?15) \
             ON CONFLICT(log_id, server) DO UPDATE SET login_id = excluded.login_id, state = excluded.state, score_id = excluded.score_id, \
             http_status = excluded.http_status, error_code = excluded.error_code, error_field = excluded.error_field, \
             error_reason = excluded.error_reason, attempts = ir_sync.attempts + 1, last_attempt_at = excluded.last_attempt_at, \
             replay_state = excluded.replay_state, replay_id = excluded.replay_id, replay_error = excluded.replay_error, \
             refused_under = excluded.refused_under, replay_refused_under = excluded.replay_refused_under",
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
                row.refused_under,
                row.replay_refused_under,
            ],
        )?;
        Ok(())
    }

    /// Write what a server answered about a play's replay, leaving the score's columns as they are.
    /// `refused_under` is the builds a refusal of the replay was given under.
    pub fn put_replay_state(&mut self, log_id: i64, server: &str, answer: &ReplayAnswer<'_>) -> Result<(), ScoreDbError> {
        self.conn.execute(
            "UPDATE ir_sync SET replay_state = ?3, replay_id = ?4, replay_error = ?5, replay_refused_under = ?6 WHERE log_id = ?1 AND server = ?2",
            params![log_id, server, answer.state.as_token(), answer.replay_id, answer.error, answer.refused_under],
        )?;
        Ok(())
    }

    /// Where every play stands with `server`, keyed by its `scorelog` id. A play with no row has
    /// never been confirmed there, but for one a sync never sends for its SCRATCH AUTO rule
    /// ([`ScoreDb::sync_candidates`]): that one reads as [`SyncState::Skipped`] for
    /// [`SKIPPED_OLD_SCRATCH_AUTO_RULE`], so a screen can say why without anything being written.
    pub fn sync_states(&self, server: &str) -> Result<HashMap<i64, IrSyncRow>, ScoreDbError> {
        let mut stmt = self.conn.prepare("SELECT * FROM ir_sync WHERE server = ?1")?;
        let rows = stmt.query_map(params![server], IrSyncRow::from_row)?;
        let mut out = HashMap::new();
        for row in rows {
            let row = row?;
            out.insert(row.log_id, row);
        }
        let mut held_back = self.conn.prepare(
            "SELECT l.id FROM scorelog l LEFT JOIN ir_sync y ON y.log_id = l.id AND y.server = ?1 \
             WHERE y.log_id IS NULL AND (l.assist = ?2 OR l.assist = ?4) AND l.rule_version < ?3",
        )?;
        let ids = held_back
            .query_map(params![server, SCRATCH_AUTO_ASSIST, SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION, ASSIST_UNRECORDED], |row| row.get::<_, i64>(0))?;
        for id in ids {
            let id = id?;
            let row = IrSyncRow { error_code: Some(SKIPPED_OLD_SCRATCH_AUTO_RULE.to_string()), ..IrSyncRow::new(id, server, "", SyncState::Skipped) };
            out.insert(id, row);
        }
        Ok(out)
    }

    /// Fill in the SHA-256 of a play that was stored without one (a play carried over from
    /// `scores.ron`), leaving one that has a SHA-256 as it is.
    pub fn backfill_sha256(&mut self, log_id: i64, sha256: &str) -> Result<(), ScoreDbError> {
        self.conn.execute("UPDATE scorelog SET sha256 = ?2 WHERE id = ?1 AND sha256 = ''", params![log_id, sha256])?;
        Ok(())
    }
}

/// What a server answered about one play's replay ([`ScoreDb::put_replay_state`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayAnswer<'a> {
    pub state: ReplayState,
    pub replay_id: Option<&'a str>,
    pub error: Option<&'a str>,
    /// The builds a refusal was given under ([`IrSyncRow::replay_refused_under`]).
    pub refused_under: Option<&'a str>,
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
