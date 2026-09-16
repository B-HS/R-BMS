use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::Connection;

use super::ScoreDbError;

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

/// Settle which replays the policy keeps (spec §4.3).
pub(super) fn plan(conn: &Connection, dir: &Path, policy: &ReplayPolicy) -> Result<ReplayGcPlan, ScoreDbError> {
    let referenced = referenced_replays(conn)?;
    if referenced.is_empty() {
        return Ok(ReplayGcPlan::default());
    }
    let protected = if policy.keep_best_per_chart { best_replays(conn)? } else { HashSet::new() };

    let mut kept: HashSet<&str> = protected.iter().map(String::as_str).collect();
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

/// How large a replay is on disk, or zero when it is already gone.
fn file_size(dir: &Path, file: &str) -> u64 {
    std::fs::metadata(dir.join(file)).map(|m| m.len()).unwrap_or(0)
}
