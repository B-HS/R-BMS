use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::{Connection, params};

use super::migrate::ASSIST_UNRECORDED;
use super::{ReplayState, ScoreDbError, SyncState};
use crate::{SCRATCH_AUTO_ASSIST, SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION};

/// How many of a chart's most recent replays are kept by default. Three is enough to compare a run
/// against the two before it, which is what the replay list is for.
pub const DEFAULT_KEEP_RECENT_PER_CHART: usize = 3;

/// Bytes in a gibibyte.
const BYTES_PER_GIB: u64 = 1024 * 1024 * 1024;

/// Default ceiling on the replay directory: two gibibytes. Replays are input streams, so this is
/// tens of thousands of runs.
pub const DEFAULT_REPLAY_MAX_TOTAL_BYTES: u64 = 2 * BYTES_PER_GIB;

/// Which saved replays are worth keeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayPolicy {
    /// Keep the replay of every chart's best run, however old it is.
    pub keep_best_per_chart: bool,
    /// Keep this many of each chart's most recent replays.
    pub keep_recent_per_chart: usize,
    /// Stop the replay directory growing past this many bytes, oldest first. Zero is no ceiling.
    pub max_total_bytes: u64,
}

impl Default for ReplayPolicy {
    fn default() -> ReplayPolicy {
        ReplayPolicy { keep_best_per_chart: true, keep_recent_per_chart: DEFAULT_KEEP_RECENT_PER_CHART, max_total_bytes: DEFAULT_REPLAY_MAX_TOTAL_BYTES }
    }
}

/// The score server and account replays are still to go up to: the player has a server set, is
/// signed in to it and has AUTO UPLOAD REPLAY on. A replay a sync would still upload there is kept
/// like one of a chart's most recent ones, so it is not deleted before it has gone up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadHold {
    /// The server, as `ir_sync` keys it.
    pub server: String,
    /// The account the replays go up under.
    pub login_id: String,
}

/// What applying a [`ReplayPolicy`] would do. Nothing is deleted here — the caller removes the
/// files and then calls [`ScoreDb::clear_replay_refs`](super::ScoreDb::clear_replay_refs), so a
/// plan can be shown, logged or ignored.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplayGcPlan {
    /// Replay basenames to delete, oldest first.
    pub delete: Vec<String>,
    /// How many replays the policy keeps.
    pub kept: usize,
    /// How many bytes deleting them frees, as the files measure on disk right now.
    pub freed_bytes: u64,
}

/// One replay file as the database knows it.
struct ReplayRef {
    file: String,
    chart_key: String,
    /// Epoch second of the most recent play that produced it.
    date: i64,
}

/// Settle which replays the policy keeps (spec §4.3), holding the ones still to go up to `hold`.
///
/// A held replay is kept as a recent one is: the byte ceiling still lets it go, oldest first, so a
/// server that never takes the plays cannot grow the directory without end.
pub(super) fn plan(conn: &Connection, dir: &Path, policy: &ReplayPolicy, hold: Option<&UploadHold>) -> Result<ReplayGcPlan, ScoreDbError> {
    let referenced = referenced_replays(conn)?;
    if referenced.is_empty() {
        return Ok(ReplayGcPlan::default());
    }
    let protected = if policy.keep_best_per_chart { best_replays(conn)? } else { HashSet::new() };

    let held = match hold {
        Some(hold) => replays_still_to_go_up(conn, hold)?,
        None => HashSet::new(),
    };

    let mut kept: HashSet<&str> = protected.iter().map(String::as_str).collect();
    kept.extend(referenced.iter().map(|entry| entry.file.as_str()).filter(|file| held.contains(*file)));
    let mut seen_per_chart: HashMap<&str, usize> = HashMap::new();
    for entry in &referenced {
        let count = seen_per_chart.entry(entry.chart_key.as_str()).or_default();
        if *count < policy.keep_recent_per_chart {
            *count += 1;
            kept.insert(entry.file.as_str());
        }
    }

    let sizes: HashMap<&str, u64> = referenced.iter().map(|e| (e.file.as_str(), file_size(dir, &e.file))).collect();
    if policy.max_total_bytes > 0 {
        evict_over_budget(&referenced, &protected, &sizes, policy.max_total_bytes, &mut kept);
    }

    let mut delete: Vec<&ReplayRef> = referenced.iter().filter(|e| !kept.contains(e.file.as_str())).collect();
    delete.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.file.cmp(&b.file)));
    let freed_bytes = delete.iter().map(|e| sizes.get(e.file.as_str()).copied().unwrap_or(0)).sum();
    Ok(ReplayGcPlan { delete: delete.into_iter().map(|e| e.file.clone()).collect(), kept: kept.len(), freed_bytes })
}

/// Drop kept replays, oldest first, until what is left fits the byte ceiling. A replay the policy
/// protects as a chart's best is never dropped for space; if only protected replays are left the
/// directory is allowed to stay over budget rather than losing the bests.
fn evict_over_budget(referenced: &[ReplayRef], protected: &HashSet<String>, sizes: &HashMap<&str, u64>, budget: u64, kept: &mut HashSet<&str>) {
    let mut total: u64 = kept.iter().map(|f| sizes.get(f).copied().unwrap_or(0)).sum();
    if total <= budget {
        return;
    }
    let mut oldest_first: Vec<&ReplayRef> = referenced.iter().collect();
    oldest_first.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.file.cmp(&b.file)));
    for entry in oldest_first {
        if total <= budget {
            return;
        }
        if protected.contains(&entry.file) || !kept.contains(entry.file.as_str()) {
            continue;
        }
        kept.remove(entry.file.as_str());
        total = total.saturating_sub(sizes.get(entry.file.as_str()).copied().unwrap_or(0));
    }
}

/// Every replay the play log points at, newest first. A file played more than once is listed once,
/// under the most recent play that produced it.
fn referenced_replays(conn: &Connection) -> Result<Vec<ReplayRef>, ScoreDbError> {
    let mut stmt = conn.prepare(
        "SELECT replay_file, chart_key, MAX(date) AS date FROM scorelog WHERE replay_file IS NOT NULL AND replay_file <> '' \
         GROUP BY replay_file ORDER BY date DESC, replay_file DESC",
    )?;
    let rows = stmt.query_map([], |row| Ok(ReplayRef { file: row.get("replay_file")?, chart_key: row.get("chart_key")?, date: row.get("date")? }))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
}

/// The replay of every chart's best run.
fn best_replays(conn: &Connection) -> Result<HashSet<String>, ScoreDbError> {
    let mut stmt = conn.prepare("SELECT replay_file FROM score WHERE replay_file IS NOT NULL AND replay_file <> ''")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>("replay_file"))?;
    rows.collect::<rusqlite::Result<HashSet<_>>>().map_err(ScoreDbError::from)
}

/// The replays a sync to `hold` would still upload: of plays made under its account or as a guest,
/// that a sync sends at all, whose score the server does not hold yet or whose replay is still to go
/// up ([`ReplayState::still_to_send`]).
fn replays_still_to_go_up(conn: &Connection, hold: &UploadHold) -> Result<HashSet<String>, ScoreDbError> {
    let still_to_send: Vec<String> =
        ReplayState::ALL.into_iter().filter(|state| state.still_to_send()).map(|state| format!("'{}'", state.as_token())).collect();
    let sql = format!(
        "SELECT l.replay_file FROM scorelog l LEFT JOIN play_submission s ON s.log_id = l.id LEFT JOIN ir_sync y ON y.log_id = l.id AND y.server = ?1 \
         WHERE l.replay_file IS NOT NULL AND l.replay_file <> '' AND (s.played_as IS NULL OR s.played_as = ?2) \
         AND NOT ((l.assist = ?3 OR l.assist = ?5) AND l.rule_version < ?4) \
         AND (y.state IS NULL OR y.state <> ?6 OR y.replay_state IN ({}))",
        still_to_send.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![
            hold.server,
            hold.login_id,
            SCRATCH_AUTO_ASSIST,
            SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION,
            ASSIST_UNRECORDED,
            SyncState::Accepted.as_token()
        ],
        |row| row.get::<_, String>(0),
    )?;
    rows.collect::<rusqlite::Result<HashSet<_>>>().map_err(ScoreDbError::from)
}

/// How large a replay is on disk, or zero when it is already gone.
fn file_size(dir: &Path, file: &str) -> u64 {
    std::fs::metadata(dir.join(file)).map(|m| m.len()).unwrap_or(0)
}
