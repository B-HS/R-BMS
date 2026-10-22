//! Finished courses kept until a score server holds them: the body each was sent as and what each
//! server answered, as `play_submission` and `ir_sync` keep a play's.
//!
//! A course is kept by the body it is submitted with, which carries the millisecond it ended at. The
//! server takes the same course sent again at the same millisecond for the one it holds, so a course
//! is sent again exactly as it was until a server answers it.

use std::collections::HashMap;

use rusqlite::types::Value;
use rusqlite::{Row, params, params_from_iter};

use super::sync::SyncWalk;
use super::{GAUGE_CLASS, ScoreDb, ScoreDbError, SyncState};

/// One finished course as it is kept: its body and the account it was played under, and where it
/// stands with the server a walk reads it for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCourse {
    /// The `course_submission` row id.
    pub id: i64,
    pub course_hash: String,
    /// Epoch millisecond the course ended at, the one its body states.
    pub played_at_ms: i64,
    /// The course submission JSON.
    pub body: String,
    /// The account the course was played under; `None` for a guest.
    pub played_as: Option<String>,
    pub state: Option<SyncState>,
}

impl KeptCourse {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<KeptCourse> {
        Ok(KeptCourse {
            id: row.get("id")?,
            course_hash: row.get("course_hash")?,
            played_at_ms: row.get("played_at")?,
            body: row.get("body")?,
            played_as: row.get("played_as")?,
            state: row.get::<_, Option<String>>("sync_state")?.as_deref().map(SyncState::from_token),
        })
    }
}

/// One `course_sync` row: what one score server last answered about one course.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourseSyncRow {
    /// The `course_submission` id of the course.
    pub course_id: i64,
    pub server: String,
    pub login_id: String,
    pub state: SyncState,
    pub score_id: Option<String>,
    pub http_status: Option<u16>,
    pub error_code: Option<String>,
    pub error_field: Option<String>,
    pub error_reason: Option<String>,
    /// The builds a refusal was given under ([`SyncWalk::refused_under`]).
    pub refused_under: Option<String>,
    /// How many times the course was sent to this server; counted by the database.
    pub attempts: u32,
    pub last_attempt_at_ms: i64,
}

impl CourseSyncRow {
    /// A row stating `state` for the course `course_id` on `server`, and nothing else yet.
    pub fn new(course_id: i64, server: &str, login_id: &str, state: SyncState) -> CourseSyncRow {
        CourseSyncRow {
            course_id,
            server: server.to_string(),
            login_id: login_id.to_string(),
            state,
            score_id: None,
            http_status: None,
            error_code: None,
            error_field: None,
            error_reason: None,
            refused_under: None,
            attempts: 0,
            last_attempt_at_ms: 0,
        }
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<CourseSyncRow> {
        Ok(CourseSyncRow {
            course_id: row.get("course_id")?,
            server: row.get("server")?,
            login_id: row.get("login_id")?,
            state: SyncState::from_token(&row.get::<_, String>("state")?),
            score_id: row.get("score_id")?,
            http_status: row.get("http_status")?,
            error_code: row.get("error_code")?,
            error_field: row.get("error_field")?,
            error_reason: row.get("error_reason")?,
            refused_under: row.get("refused_under")?,
            attempts: row.get("attempts")?,
            last_attempt_at_ms: row.get("last_attempt_at")?,
        })
    }
}

impl ScoreDb {
    /// Keep a finished course's submission body, and the account it was played under. Returns the
    /// id it is kept under.
    pub fn put_course(&mut self, course_hash: &str, played_at_ms: i64, body: &str, played_as: Option<&str>) -> Result<i64, ScoreDbError> {
        self.conn.execute(
            "INSERT INTO course_submission(course_hash, played_at, body, played_as) VALUES(?1, ?2, ?3, ?4)",
            params![course_hash, played_at_ms, body, played_as],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Every course kept, oldest first, with no server's answer read.
    pub fn kept_courses(&self) -> Result<Vec<KeptCourse>, ScoreDbError> {
        let mut stmt = self.conn.prepare("SELECT id, course_hash, played_at, body, played_as, NULL AS sync_state FROM course_submission ORDER BY id ASC")?;
        let rows = stmt.query_map([], KeptCourse::from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
    }

    /// Put the course stages `stages` names on [`GAUGE_CLASS`], the gauge every course this client
    /// keeps ran its stages on, and fold the assist grades afresh from the plays
    /// ([`super::refold_assist_grades`]). A build before the stage was stored on the gauge it ran on
    /// stored it on the gauge the settings chose, which grades a SCRATCH AUTO clear of it by a gauge
    /// it never played. A row already on a course gauge is left as it is, so the same stages put
    /// again write nothing. Answers how many rows moved; the grades are folded only when one did.
    pub fn put_course_stages_on_class(&mut self, stages: &[i64]) -> Result<usize, ScoreDbError> {
        let tx = self.conn.transaction()?;
        let mut moved = 0;
        {
            let mut stmt = tx.prepare("UPDATE scorelog SET gauge = ?1 WHERE id = ?2 AND gauge < ?1")?;
            for id in stages {
                moved += stmt.execute(params![GAUGE_CLASS, id])?;
            }
        }
        if moved > 0 {
            super::refold_assist_grades(&tx)?;
        }
        tx.commit()?;
        Ok(moved)
    }

    /// The newest course kept, or `None` when there is none ([`SyncWalk::courses_up_to`]).
    pub fn newest_course_id(&self) -> Result<Option<i64>, ScoreDbError> {
        Ok(self.conn.query_row("SELECT MAX(id) FROM course_submission", [], |row| row.get(0))?)
    }

    /// Every course a walk has to send, by the rules [`ScoreDb::sync_candidates`] picks plays by:
    /// what the server does not hold, made under the walk's account or as a guest, but for one the
    /// server refused under the walk's builds; oldest first, a course the server failed to answer
    /// behind the others. A walk narrowed to named plays sends no course.
    pub fn course_candidates(&self, walk: &SyncWalk<'_>) -> Result<Vec<KeptCourse>, ScoreDbError> {
        if walk.only.is_some() {
            return Ok(Vec::new());
        }
        let mut bound: Vec<Value> = vec![
            Value::from(walk.server.to_string()),
            Value::from(walk.login_id.to_string()),
            Value::from(SyncState::Accepted.as_token().to_string()),
            Value::from(SyncState::Retry.as_token().to_string()),
        ];
        let mut narrowed = String::new();
        if let Some(up_to) = walk.courses_up_to {
            bound.push(Value::from(up_to));
            narrowed.push_str(&format!(" AND c.id <= ?{}", bound.len()));
        }
        if let Some(key) = walk.refused_under {
            bound.push(Value::from(SyncState::Rejected.as_token().to_string()));
            bound.push(Value::from(key.to_string()));
            narrowed.push_str(&format!(" AND NOT COALESCE(y.state = ?{} AND y.refused_under = ?{}, 0)", bound.len() - 1, bound.len()));
        }
        let sql = format!(
            "SELECT c.id, c.course_hash, c.played_at, c.body, c.played_as, y.state AS sync_state FROM course_submission c \
             LEFT JOIN course_sync y ON y.course_id = c.id AND y.server = ?1 \
             WHERE (y.state IS NULL OR y.state <> ?3) AND (c.played_as IS NULL OR c.played_as = ?2){narrowed} \
             ORDER BY CASE WHEN y.state = ?4 THEN y.attempts ELSE 0 END ASC, c.played_at ASC, c.id ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(bound), KeptCourse::from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
    }

    /// Write what a server answered about a course, replacing what it said before and counting the
    /// send. `row.attempts` is not read.
    pub fn put_course_sync_state(&mut self, row: &CourseSyncRow) -> Result<(), ScoreDbError> {
        self.conn.execute(
            "INSERT INTO course_sync(course_id, server, login_id, state, score_id, http_status, error_code, error_field, error_reason, refused_under, \
             attempts, last_attempt_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11) \
             ON CONFLICT(course_id, server) DO UPDATE SET login_id = excluded.login_id, state = excluded.state, score_id = excluded.score_id, \
             http_status = excluded.http_status, error_code = excluded.error_code, error_field = excluded.error_field, \
             error_reason = excluded.error_reason, refused_under = excluded.refused_under, attempts = course_sync.attempts + 1, \
             last_attempt_at = excluded.last_attempt_at",
            params![
                row.course_id,
                row.server,
                row.login_id,
                row.state.as_token(),
                row.score_id,
                row.http_status,
                row.error_code,
                row.error_field,
                row.error_reason,
                row.refused_under,
                row.last_attempt_at_ms,
            ],
        )?;
        Ok(())
    }

    /// Where every kept course stands with `server`, keyed by its id.
    pub fn course_sync_states(&self, server: &str) -> Result<HashMap<i64, CourseSyncRow>, ScoreDbError> {
        let mut stmt = self.conn.prepare("SELECT * FROM course_sync WHERE server = ?1")?;
        let rows = stmt.query_map(params![server], CourseSyncRow::from_row)?;
        let mut out = HashMap::new();
        for row in rows {
            let row = row?;
            out.insert(row.course_id, row);
        }
        Ok(out)
    }
}
