//! The SQLite score database: one merged best per chart, one row per play, and the daily play
//! statistics behind them.
//!
//! [`ScoreBook`](crate::ScoreBook) keeps every play in one RON document and answers a per-chart
//! question by scanning it. That is fine for a few hundred plays and wrong for a library of
//! thousands, so the same records live here in three tables instead:
//!
//! * `score` — the merged bests of one chart, one row per `(chart_key, mode, ln_mode)`.
//! * `scorelog` — one row per play, the history, the graphs and the replay retention plan read it.
//! * `player_stat` — one row per day, the judgement counts and play time of that day accumulated.
//!
//! The merge rules are the reference implementation's, element for element (`ScoreData.java:541-585`
//! `update`, `PlayDataAccessor.java:195-265` `writeScoreData`): a higher lamp always wins, and the
//! score columns (the twelve judgement counts, `avgjudge`, `minbp` and `combo`) only move when the
//! run was allowed to set bests at all. Decision 12 is exactly that flag — an assisted or
//! custom-judged run raises the lamp and the play count and touches nothing else — so it arrives
//! here as `updates_best` and is not re-derived.
//!
//! `chart_key` is the chart's MD5, lowercased, and its SHA-256 only for a chart with no MD5. rbms
//! has keyed play records, favourites and difficulty tables by MD5 since before it hashed charts
//! with SHA-256, so keying on anything else here would strand every record written so far. Both
//! hashes are columns of their own, so the key can be moved to SHA-256 later without losing a row.
//!
//! `ln_mode` splits a chart's bests the way [`ScoreRecord::ln_mode`](crate::ScoreRecord::ln_mode)
//! does: a chart played as charge notes is judged at both ends, so it has twice the notes and twice
//! the EX ceiling of the same chart played as plain long notes and the two cannot share a best.
//! The reference splits on the same axis and calls the column `mode`
//! (`PlayDataAccessor.java:200-205`); rbms already had a play mode under that name, so the axis
//! keeps the name rbms gives it and joins the primary key beside the play mode.

mod migrate;
mod replay_gc;
mod rows;
mod schema;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter};

use rows::{BEST_COLUMNS, JUDGE_COLUMNS, LOG_COLUMNS, accumulate_day, insert_log, log_from_row, read_best, write_best};

pub use migrate::{ASSIST_UNRECORDED, MigrationReport, SEED_UNRECORDED, migrate_score_book, open_with_import};
pub use replay_gc::{DEFAULT_KEEP_RECENT_PER_CHART, DEFAULT_REPLAY_MAX_TOTAL_BYTES, ReplayGcPlan, ReplayPolicy};
pub use schema::{GAUGE_UNKNOWN, MODE_UNKNOWN, RANDOM_OFF, SCORE_DB_SCHEMA_VERSION, gauge_id, gauge_token, mode_id, mode_name, random_id, random_token};

/// Everything that can go wrong reading or writing the score database.
#[derive(Debug, thiserror::Error)]
pub enum ScoreDbError {
    /// The database rejected a statement, or could not be opened.
    #[error("{0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The database was written by a build with a newer schema generation. It is left untouched.
    #[error("score database schema {found} is newer than this build reads ({supported})")]
    FutureSchema { found: u32, supported: u32 },
    /// The database file or its directory could not be created or removed.
    #[error("{0}")]
    Io(#[source] std::io::Error),
}

/// How many judgement tiers a play is counted in: PGREAT, GREAT, GOOD, BAD, POOR and MISS, the
/// order `rbms_judge::Judge` is declared in and the order the `e*`/`l*` column pairs follow.
pub const JUDGE_TIER_COUNT: usize = 6;

/// Index of the PGREAT tier in [`JudgeCounts`].
pub const JUDGE_PGREAT: usize = 0;
/// Index of the GREAT tier in [`JudgeCounts`].
pub const JUDGE_GREAT: usize = 1;
/// Index of the GOOD tier in [`JudgeCounts`].
pub const JUDGE_GOOD: usize = 2;
/// Index of the BAD tier in [`JudgeCounts`].
pub const JUDGE_BAD: usize = 3;
/// Index of the POOR tier in [`JudgeCounts`].
pub const JUDGE_POOR: usize = 4;
/// Index of the MISS tier in [`JudgeCounts`].
pub const JUDGE_MISS: usize = 5;

/// EX points a PGREAT is worth (`ScoreData.java:405-407`).
const EX_WEIGHT_PGREAT: u32 = 2;
/// EX points a GREAT is worth (`ScoreData.java:405-407`).
const EX_WEIGHT_GREAT: u32 = 1;

/// Lamp id of a failed run. Anything above it is a clear, which is what `clearcount` and the daily
/// clear tally count (`PlayDataAccessor.java:211-213`, `:131-133`). Kept as a number rather than
/// reached for through `rbms_judge`, which this crate deliberately does not depend on.
const CLEAR_FAILED_ID: u8 = 1;

/// Value `minbp` and `avgjudge` carry until a run that may set bests has produced one. Both are
/// minimums, so they start at the largest value SQLite stores in an `INTEGER` column without
/// widening, which is also the reference's own unset marker for `minbp`.
pub const UNSET_MINIMUM: i64 = 2_147_483_647;

/// Seconds in a day, the width of one `player_stat` bucket.
const SECONDS_PER_DAY: i64 = 86_400;
/// Per-tier judgement counts of one run, split into the hits taken before the note (early) and
/// after it (late) — the `e*`/`l*` column pairs the reference stores and the split
/// `rbms_play::PlaySummary` already produces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JudgeCounts {
    pub early: [u32; JUDGE_TIER_COUNT],
    pub late: [u32; JUDGE_TIER_COUNT],
}

impl JudgeCounts {
    /// Counts split into early and late, as the play session reports them.
    pub fn from_early_late(early: [u32; JUDGE_TIER_COUNT], late: [u32; JUDGE_TIER_COUNT]) -> JudgeCounts {
        JudgeCounts { early, late }
    }

    /// Counts of a run whose early/late split was never recorded, entered as late. A record written
    /// before the split existed cannot be turned into one that has it, and calling the whole run
    /// late is the reading that leaves the totals — and so EX — correct.
    pub fn all_late(counts: [u32; JUDGE_TIER_COUNT]) -> JudgeCounts {
        JudgeCounts { early: [0; JUDGE_TIER_COUNT], late: counts }
    }

    /// Both halves of one tier.
    pub fn tier(&self, tier: usize) -> u32 {
        self.early[tier] + self.late[tier]
    }

    /// EX score: two per PGREAT, one per GREAT (`ScoreData.java:405-407`).
    pub fn ex_score(&self) -> u32 {
        self.tier(JUDGE_PGREAT) * EX_WEIGHT_PGREAT + self.tier(JUDGE_GREAT) * EX_WEIGHT_GREAT
    }

    /// Every judgement the run took.
    pub fn total(&self) -> u32 {
        (0..JUDGE_TIER_COUNT).map(|t| self.tier(t)).sum()
    }

    /// Bad, poor and miss — the reference's bad-poor count (`rbms_play::PlaySummary::min_bp`).
    pub fn combo_breaks(&self) -> u32 {
        self.tier(JUDGE_BAD) + self.tier(JUDGE_POOR) + self.tier(JUDGE_MISS)
    }

    /// The twelve counts in schema order, for binding.
    pub(super) fn columns(&self) -> [u32; JUDGE_TIER_COUNT * 2] {
        let mut out = [0; JUDGE_TIER_COUNT * 2];
        for tier in 0..JUDGE_TIER_COUNT {
            out[tier * 2] = self.early[tier];
            out[tier * 2 + 1] = self.late[tier];
        }
        out
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<JudgeCounts> {
        let mut counts = JudgeCounts::default();
        for tier in 0..JUDGE_TIER_COUNT {
            counts.early[tier] = row.get(JUDGE_COLUMNS[tier * 2])?;
            counts.late[tier] = row.get(JUDGE_COLUMNS[tier * 2 + 1])?;
        }
        Ok(counts)
    }
}

/// One finished play, exactly as it is written to `scorelog` and folded into the bests.
///
/// Everything here is a value the result screen already holds; nothing is derived from the chart or
/// the engine inside this crate, so a play recorded from a replay of an old record and one recorded
/// live take the same path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayLog {
    /// SHA-256 of the chart where one is known, its MD5 otherwise. Lowercase.
    pub chart_key: String,
    /// Play mode id, see [`mode_id`].
    pub mode: i32,
    /// Long-note flavour key, see [`crate::ScoreRecord::ln_mode`].
    pub ln_mode: String,
    pub md5: String,
    pub sha256: String,
    pub title: String,
    /// `ClearType` id of the lamp the run was awarded.
    pub clear: u8,
    pub judge: JudgeCounts,
    /// Judged objects in the chart, the EX denominator.
    pub notes: u32,
    pub combo: u32,
    /// Bad + poor + miss.
    pub minbp: u32,
    /// Mean absolute timing error in microseconds (`BMSPlayer.java:922-930`).
    pub avgjudge: i64,
    /// Gauge id the run was played on, see [`gauge_id`].
    pub gauge: i32,
    /// Gauge percentage the run ended on.
    pub gauge_value: f32,
    /// Assist level of the run (`rbms_ir::mapping::assist_level`).
    pub assist: i32,
    /// Reference option encoding, kept for parity; rbms writes the shuffle in `random`.
    pub option: i32,
    /// Shuffle seed, so a replay reproduces the lane layout.
    pub seed: i64,
    /// Note-shuffle id, see [`random_id`].
    pub random: i32,
    /// How long the run took, milliseconds, accumulated into the day's play time.
    pub playtime_ms: i64,
    /// Epoch second the run ended at.
    pub date: i64,
    /// Where the row came from: [`PLAY_STATE_LIVE`] or [`PLAY_STATE_MIGRATED`].
    pub state: i32,
    /// Judging-rule generation the run was produced under ([`crate::SCORE_RULE_VERSION`]).
    pub rule_version: u32,
    /// Whether the run reached a score server.
    pub ir_submitted: bool,
    /// Basename of the saved replay, when one was recorded.
    pub replay_file: Option<String>,
}

/// `state` of a play this build recorded as it happened.
pub const PLAY_STATE_LIVE: i32 = 0;

/// `state` of a play carried over from `scores.ron`, whose early/late split and timing error were
/// never recorded. Statistics that need those exclude it.
pub const PLAY_STATE_MIGRATED: i32 = 1;

/// The merged bests of one chart in one long-note flavour: one `score` row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BestScore {
    pub chart_key: String,
    pub mode: i32,
    pub ln_mode: String,
    pub md5: String,
    pub sha256: String,
    pub clear: u8,
    pub judge: JudgeCounts,
    pub notes: u32,
    pub combo: u32,
    pub minbp: i64,
    pub avgjudge: i64,
    pub playcount: u32,
    pub clearcount: u32,
    pub option: i32,
    pub seed: i64,
    pub random: i32,
    pub date: i64,
    pub state: i32,
    pub gauge: i32,
    pub assist: i32,
    pub rule_version: u32,
    pub replay_file: Option<String>,
}

impl BestScore {
    /// EX of the best scoring run on this row.
    pub fn ex_score(&self) -> u32 {
        self.judge.ex_score()
    }

    /// A row that has never been played, which every merge starts from.
    fn empty(chart_key: &str, mode: i32, ln_mode: &str) -> BestScore {
        BestScore {
            chart_key: chart_key.to_string(),
            mode,
            ln_mode: ln_mode.to_string(),
            minbp: UNSET_MINIMUM,
            avgjudge: UNSET_MINIMUM,
            ..BestScore::default()
        }
    }

    pub(super) fn from_row(row: &Row<'_>) -> rusqlite::Result<BestScore> {
        Ok(BestScore {
            chart_key: row.get("chart_key")?,
            mode: row.get("mode")?,
            ln_mode: row.get("ln_mode")?,
            md5: row.get("md5")?,
            sha256: row.get("sha256")?,
            clear: row.get("clear")?,
            judge: JudgeCounts::from_row(row)?,
            notes: row.get("notes")?,
            combo: row.get("combo")?,
            minbp: row.get("minbp")?,
            avgjudge: row.get("avgjudge")?,
            playcount: row.get("playcount")?,
            clearcount: row.get("clearcount")?,
            option: row.get("option")?,
            seed: row.get("seed")?,
            random: row.get("random")?,
            date: row.get("date")?,
            state: row.get("state")?,
            gauge: row.get("gauge")?,
            assist: row.get("assist")?,
            rule_version: row.get("rule_version")?,
            replay_file: row.get("replay_file")?,
        })
    }
}

/// A chart's bests folded across every play mode and long-note flavour it was played in — the
/// question the select list asks, where one chart shows one lamp and one EX.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChartBest {
    pub chart_key: String,
    pub clear: u8,
    pub ex_score: u32,
    pub minbp: i64,
    pub combo: u32,
    pub playcount: u32,
    pub clearcount: u32,
    /// Epoch second of the most recent play.
    pub last_played: i64,
}

/// One day of play, accumulated (`ScoreDatabaseAccessor.java:36-53`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DayStat {
    /// Epoch second the UTC day starts at.
    pub date: i64,
    pub playcount: u32,
    pub clear: u32,
    pub judge: JudgeCounts,
    pub playtime_ms: i64,
    pub maxcombo: u32,
}

/// The player's own identity row (`ScoreDatabaseAccessor.java:31-34`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub rank: String,
}

/// The score database.
#[derive(Debug)]
pub struct ScoreDb {
    conn: Connection,
}

impl ScoreDb {
    /// Open (creating if absent) the database at `path`, along with its parent directory. The
    /// schema is not touched here; call [`ScoreDb::migrate`] before writing.
    pub fn open(path: &Path) -> Result<ScoreDb, ScoreDbError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(ScoreDbError::Io)?;
        }
        let conn = Connection::open(path)?;
        schema::apply_pragmas(&conn)?;
        Ok(ScoreDb { conn })
    }

    /// A database that lives only as long as the value, for tests and for a run that must not
    /// persist anything.
    pub fn open_in_memory() -> Result<ScoreDb, ScoreDbError> {
        let conn = Connection::open_in_memory()?;
        schema::apply_pragmas(&conn)?;
        Ok(ScoreDb { conn })
    }

    /// Create the tables if they are absent and stamp the schema generation, returning it. Refuses
    /// a database written by a newer build rather than rewriting it.
    pub fn migrate(&mut self) -> Result<u32, ScoreDbError> {
        schema::migrate(&mut self.conn)
    }

    /// Record one finished play: append it to `scorelog`, fold it into the chart's bests and
    /// accumulate it into the day, all in one transaction. Returns the `scorelog` id.
    ///
    /// `updates_best` is decision 12's score flag. When it is false the run still appends its log
    /// row, still counts towards `playcount` and may still raise the lamp, but leaves the twelve
    /// judgement counts, `notes`, `avgjudge`, `minbp` and `combo` exactly as they were
    /// (`ScoreData.java:541-585`).
    pub fn record_play(&mut self, log: &PlayLog, updates_best: bool) -> Result<i64, ScoreDbError> {
        let tx = self.conn.transaction()?;
        let id = insert_log(&tx, log)?;
        let existing = read_best(&tx, &log.chart_key, log.mode, &log.ln_mode)?;
        let merged = merge_best(existing.unwrap_or_else(|| BestScore::empty(&log.chart_key, log.mode, &log.ln_mode)), log, updates_best);
        write_best(&tx, &merged)?;
        accumulate_day(&tx, log)?;
        tx.commit()?;
        Ok(id)
    }

    /// The stored bests of one chart in one long-note flavour.
    pub fn best_in_ln_mode(&self, chart_key: &str, mode: i32, ln_mode: &str) -> Result<Option<BestScore>, ScoreDbError> {
        read_best(&self.conn, chart_key, mode, ln_mode)
    }

    /// The chart's strongest stored row in this play mode: the highest lamp, and among equal lamps
    /// the highest EX. This is how the reference picks one row out of several
    /// (`ScoreDatabaseAccessor.java:127-135`).
    pub fn best(&self, chart_key: &str, mode: i32) -> Result<Option<BestScore>, ScoreDbError> {
        let sql = format!("SELECT {BEST_COLUMNS} FROM score WHERE chart_key = ?1 AND mode = ?2 ORDER BY clear DESC, (epg + lpg) * 2 + egr + lgr DESC LIMIT 1");
        Ok(self.conn.query_row(&sql, params![chart_key, mode], BestScore::from_row).optional()?)
    }

    /// A chart's bests folded across every mode and flavour it was played in.
    pub fn chart_best(&self, chart_key: &str) -> Result<Option<ChartBest>, ScoreDbError> {
        Ok(self.chart_best_many(std::slice::from_ref(&chart_key.to_string()))?.remove(chart_key))
    }

    /// [`ScoreDb::chart_best`] for a screenful of charts at once: one statement instead of one per
    /// row. Charts with no plays are absent from the map.
    pub fn chart_best_many(&self, chart_keys: &[String]) -> Result<HashMap<String, ChartBest>, ScoreDbError> {
        if chart_keys.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = std::iter::repeat_n("?", chart_keys.len()).collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT chart_key, MAX(clear) AS clear, MAX((epg + lpg) * 2 + egr + lgr) AS ex, MIN(minbp) AS minbp, MAX(combo) AS combo, \
             SUM(playcount) AS playcount, SUM(clearcount) AS clearcount, MAX(date) AS date \
             FROM score WHERE chart_key IN ({placeholders}) GROUP BY chart_key"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(chart_keys.iter()), |row| {
            Ok(ChartBest {
                chart_key: row.get("chart_key")?,
                clear: row.get("clear")?,
                ex_score: row.get("ex")?,
                minbp: row.get("minbp")?,
                combo: row.get("combo")?,
                playcount: row.get("playcount")?,
                clearcount: row.get("clearcount")?,
                last_played: row.get("date")?,
            })
        })?;
        let mut out = HashMap::new();
        for row in rows {
            let row = row?;
            out.insert(row.chart_key.clone(), row);
        }
        Ok(out)
    }

    /// A chart's plays, newest first.
    pub fn history(&self, chart_key: &str, limit: usize) -> Result<Vec<PlayLog>, ScoreDbError> {
        let sql = format!("SELECT {LOG_COLUMNS} FROM scorelog WHERE chart_key = ?1 ORDER BY date DESC, id DESC LIMIT ?2");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![chart_key, limit as i64], log_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
    }

    /// Every play in the database, oldest first. The whole history is small enough to walk when a
    /// statistic has to be rebuilt from it.
    pub fn all_plays(&self) -> Result<Vec<PlayLog>, ScoreDbError> {
        let sql = format!("SELECT {LOG_COLUMNS} FROM scorelog ORDER BY date ASC, id ASC");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], log_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
    }

    /// Daily statistics between two epoch seconds, inclusive, oldest first.
    pub fn day_stats(&self, from: i64, to: i64) -> Result<Vec<DayStat>, ScoreDbError> {
        let judge = JUDGE_COLUMNS.join(", ");
        let sql = format!("SELECT date, playcount, clear, {judge}, playtime, maxcombo FROM player_stat WHERE date BETWEEN ?1 AND ?2 ORDER BY date ASC");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![from, to], |row| {
            Ok(DayStat {
                date: row.get("date")?,
                playcount: row.get("playcount")?,
                clear: row.get("clear")?,
                judge: JudgeCounts::from_row(row)?,
                playtime_ms: row.get("playtime")?,
                maxcombo: row.get("maxcombo")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(ScoreDbError::from)
    }

    /// The player identity row, when one has been written.
    pub fn profile(&self) -> Result<Option<Profile>, ScoreDbError> {
        Ok(self
            .conn
            .query_row("SELECT id, name, rank FROM profile LIMIT 1", [], |row| {
                Ok(Profile { id: row.get("id")?, name: row.get("name")?, rank: row.get("rank")? })
            })
            .optional()?)
    }

    /// Replace the player identity row. The database holds one, as the reference does
    /// (`ScoreDatabaseAccessor.java:116-123`).
    pub fn set_profile(&mut self, profile: &Profile) -> Result<(), ScoreDbError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM profile", [])?;
        tx.execute("INSERT INTO profile(id, name, rank) VALUES(?1, ?2, ?3)", params![profile.id, profile.name, profile.rank])?;
        tx.commit()?;
        Ok(())
    }

    /// Which saved replays the retention policy keeps and which it lets go, given the directory the
    /// replays live in. Deleting the files is the caller's, so a plan can be shown before it runs.
    pub fn replay_gc_plan(&self, dir: &Path, policy: &ReplayPolicy) -> Result<ReplayGcPlan, ScoreDbError> {
        replay_gc::plan(&self.conn, dir, policy)
    }

    /// Forget the named replays: every `score` and `scorelog` row that pointed at one is left
    /// pointing at nothing, so history never offers a replay whose file has been deleted.
    pub fn clear_replay_refs(&mut self, files: &[String]) -> Result<usize, ScoreDbError> {
        if files.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction()?;
        let mut cleared = 0;
        for file in files {
            cleared += tx.execute("UPDATE scorelog SET replay_file = NULL WHERE replay_file = ?1", params![file])?;
            tx.execute("UPDATE score SET replay_file = NULL WHERE replay_file = ?1", params![file])?;
        }
        tx.commit()?;
        Ok(cleared)
    }

    /// How many plays the database holds.
    pub fn play_count(&self) -> Result<u32, ScoreDbError> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM scorelog", [], |row| row.get(0))?)
    }

    /// Copy the whole database into a single file at `path`, WAL and all. `VACUUM INTO` is the only
    /// way to snapshot a write-ahead-logged database as one file, which is what the `.bak` copies
    /// the RON stores keep are.
    pub fn backup_to(&self, path: &Path) -> Result<(), ScoreDbError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(ScoreDbError::Io)?;
        }
        if path.exists() {
            std::fs::remove_file(path).map_err(ScoreDbError::Io)?;
        }
        self.conn.execute("VACUUM INTO ?1", params![path.to_string_lossy()])?;
        Ok(())
    }
}

/// Fold one play into a chart's bests.
///
/// The lamp, the play count and the clear count move for every run. Everything else moves only for
/// a run allowed to set bests, and then only when it actually beats what is stored — the four
/// comparisons of `ScoreData.update` (`ScoreData.java:541-585`), each of which also re-stamps the
/// option and seed that produced the winning value.
///
/// The one addition is the rule generation: an assisted run recorded under a judging rule this
/// build no longer produces cannot raise the lamp, because nothing demoted it the way this build
/// demotes an assisted clear. [`ScoreBook::best_clear_for_md5`](crate::ScoreBook::best_clear_for_md5)
/// leaves such a record out for the same reason, and the two must agree across the migration.
fn merge_best(mut best: BestScore, log: &PlayLog, updates_best: bool) -> BestScore {
    let lamp_counts = updates_best || !crate::is_stale_rule_version(log.rule_version);
    if lamp_counts && best.clear < log.clear {
        best.clear = log.clear;
        best.option = log.option;
        best.seed = log.seed;
        best.gauge = log.gauge;
        best.assist = log.assist;
    }
    if updates_best {
        best.notes = log.notes;
        if best.ex_score() < log.judge.ex_score() {
            best.judge = log.judge;
            best.option = log.option;
            best.seed = log.seed;
            best.rule_version = log.rule_version;
            best.replay_file = log.replay_file.clone();
        }
        if best.avgjudge > log.avgjudge {
            best.avgjudge = log.avgjudge;
            best.option = log.option;
            best.seed = log.seed;
        }
        if best.minbp > i64::from(log.minbp) {
            best.minbp = i64::from(log.minbp);
            best.option = log.option;
            best.seed = log.seed;
        }
        if best.combo < log.combo {
            best.combo = log.combo;
            best.option = log.option;
            best.seed = log.seed;
        }
    }
    if log.clear > CLEAR_FAILED_ID {
        best.clearcount += 1;
    }
    best.playcount += 1;
    best.date = log.date;
    best.state = log.state;
    if !log.md5.is_empty() {
        best.md5.clone_from(&log.md5);
    }
    if !log.sha256.is_empty() {
        best.sha256.clone_from(&log.sha256);
    }
    best
}

/// The epoch second the UTC day containing `epoch_seconds` starts at. `rem_euclid` keeps a date
/// before the epoch inside its own day rather than rounding it towards zero into the next one.
pub fn utc_day_start(epoch_seconds: i64) -> i64 {
    epoch_seconds - epoch_seconds.rem_euclid(SECONDS_PER_DAY)
}

/// The key a chart's records are stored under: its MD5 where it has one, its SHA-256 otherwise,
/// lowercased so two spellings of one hash are one chart. Both hashes are kept as columns of their
/// own; this only decides which of them the rows are found by.
pub fn chart_key(md5: &str, sha256: &str) -> String {
    if md5.is_empty() { sha256.to_ascii_lowercase() } else { md5.to_ascii_lowercase() }
}
