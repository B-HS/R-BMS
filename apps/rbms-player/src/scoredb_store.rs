//! Player-side score database wiring: opening it, turning a finished run into a row, reading the
//! rows back as the book the screens already work on, and applying the replay retention policy.
//!
//! The browser, the play HUD and the result screen all ask their questions of
//! [`ScoreBook`](rbms_store::ScoreBook) — best EX on this chart, best lamp, the last few runs — and
//! none of them has to change for the records to live in SQLite. What changes underneath is where
//! the book comes from: [`book_from_db`] builds it out of `scorelog` at startup instead of parsing
//! `scores.ron`, and [`record_finished_play`] writes a finished run straight into the database
//! instead of appending to the RON document and rewriting the whole of it.
//!
//! The database is the durable store; the book is a read-through cache of it, held for the
//! frame-rate queries the select list makes. Everything the book cannot express — the per-play log
//! behind the history graph, the daily statistics, the early/late split — is in the database and
//! reachable through [`rbms_store::scoredb::ScoreDb`] directly.
//!
//! A database that will not open is reported and then ignored: the player runs without recording,
//! which is what it did before there was a database, rather than refusing to start.
//!
//! The module is complete but not yet called: the call sites named in
//! `docs/plan/phase-g-wiring/G2-scoredb.md` belong to the integration branch, and the allow below
//! goes away with the first of them.

use std::path::{Path, PathBuf};

use rbms_chart::shuffle::NoteOption;
use rbms_config::gauge_token;
use rbms_judge::{ClearType, GaugeKind, clear_type_id};
use rbms_model::Mode;
use rbms_play::PlaySummary;
use rbms_store::scoredb::{
    JUDGE_MISS, JUDGE_TIER_COUNT, JudgeCounts, MigrationReport, PLAY_STATE_LIVE, PlayLog, ReplayGcPlan, ReplayPolicy, ScoreDb, chart_key, gauge_id, mode_id,
    mode_name, open_with_import, random_id, random_token,
};
use rbms_store::{SCORE_RULE_VERSION, ScoreBook, ScoreRecord};

use crate::notify::{Level, notify};

/// Name of the score database inside the config directory, next to the settings and the song
/// database.
pub(crate) const SCOREDB_FILE: &str = "scoredb.sqlite";

/// Name of the RON score book the database is built from the first time it is created.
pub(crate) const SCORES_RON_FILE: &str = "scores.ron";

/// Name of the directory saved replays live in, next to the two databases.
pub(crate) const REPLAY_DIR: &str = "replays";

/// Milliseconds per second: the player stamps a play in milliseconds
/// ([`ScoreRecord::played_at`]), the database stores seconds.
const MILLIS_PER_SECOND: i64 = 1_000;

/// EX points a fully judged note is worth, so a run's EX ceiling is its note count times this
/// (`rbms_play::PlaySummary::max_ex_score`).
const EX_PER_NOTE: u32 = 2;

/// Where the score database, the book it was built from and the replays live, given the directory
/// the settings file is in.
pub(crate) fn score_paths(config_dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (config_dir.join(SCOREDB_FILE), config_dir.join(SCORES_RON_FILE), config_dir.join(REPLAY_DIR))
}

/// Open the score database, importing the RON score book the first time it is created.
///
/// A failure is reported and swallowed: the caller gets `None` and the run records nothing, which
/// is strictly better than refusing to start over a score file. The import itself rolls back — a
/// half-written database is removed and the book left where it was — so a failed first launch can
/// simply be tried again.
pub(crate) fn open_score_db(db_path: &Path, ron_path: &Path) -> Option<ScoreDb> {
    match open_with_import(db_path, ron_path) {
        Ok((db, report)) => {
            if let Some(report) = report {
                report_import(&report, ron_path);
            }
            Some(db)
        }
        Err(e) => {
            notify(Level::Error, format!("score database not opened ({}): {e}", db_path.display()));
            None
        }
    }
}

/// Say what the one-time import did, so a player whose records moved can see where they went.
fn report_import(report: &MigrationReport, ron_path: &Path) {
    if report.records == 0 {
        return;
    }
    notify(
        Level::Info,
        format!(
            "{} score records imported into the database ({} assisted, kept as history only); {} moved aside",
            report.records,
            report.assisted,
            ron_path.display()
        ),
    );
}

/// Microseconds per millisecond, the step between the song clock and the play time a day
/// accumulates.
const MICROS_PER_MILLI: i64 = 1_000;

/// How long a run lasted, from the song time of the chart's last row
/// (`rbms_play::PlaySession::last_time_us`). The reference measures a play the same way — the time
/// of the last note that was actually played (`PlayDataAccessor.java:284-292`) — rather than by the
/// wall clock, so a paused or abandoned run does not inflate the day's play time.
pub(crate) fn playtime_ms(last_time_us: i64) -> i64 {
    last_time_us.div_euclid(MICROS_PER_MILLI).max(0)
}

/// Everything a finished run has to say about itself, as the result screen already holds it.
///
/// Nothing here is looked up or recomputed — the caller passes the values it has just shown the
/// player, so what is stored and what was displayed cannot disagree.
pub(crate) struct FinishedPlay<'a> {
    pub(crate) md5: &'a str,
    pub(crate) sha256: &'a str,
    pub(crate) title: &'a str,
    pub(crate) mode: Mode,
    /// The long-note flavour key the run has to be compared within
    /// ([`ScoreRecord::ln_mode`](rbms_store::ScoreRecord::ln_mode)).
    pub(crate) ln_mode: &'a str,
    /// The lamp awarded, after any assist demotion.
    pub(crate) lamp: ClearType,
    pub(crate) summary: &'a PlaySummary,
    /// The gauge the run finished on.
    pub(crate) gauge: GaugeKind,
    pub(crate) random: NoteOption,
    pub(crate) seed: u64,
    /// Assist level (`rbms_ir::mapping::assist_level`): zero for an unassisted run.
    pub(crate) assist: u8,
    /// Epoch milliseconds the run ended at.
    pub(crate) played_at_ms: i64,
    /// How long the run took, milliseconds.
    pub(crate) playtime_ms: i64,
    pub(crate) ir_submitted: bool,
    /// Basename of the saved replay, when one was recorded.
    pub(crate) replay_file: Option<String>,
}

/// One finished run as the row the database stores.
pub(crate) fn play_log(play: &FinishedPlay<'_>) -> PlayLog {
    PlayLog {
        chart_key: chart_key(play.md5, play.sha256),
        mode: mode_id(play.mode.name),
        ln_mode: play.ln_mode.to_string(),
        md5: play.md5.to_ascii_lowercase(),
        sha256: play.sha256.to_ascii_lowercase(),
        title: play.title.to_string(),
        clear: clear_type_id(play.lamp),
        judge: JudgeCounts::from_early_late(play.summary.early, play.summary.late),
        notes: play.summary.total_notes,
        combo: play.summary.max_combo,
        minbp: play.summary.min_bp,
        avgjudge: play.summary.avg_judge_us.abs(),
        gauge: gauge_id(gauge_token(play.gauge)),
        gauge_value: play.summary.gauge_value,
        assist: i32::from(play.assist),
        option: 0,
        seed: play.seed as i64,
        random: random_id(play.random.label()),
        playtime_ms: play.playtime_ms,
        date: play.played_at_ms.div_euclid(MILLIS_PER_SECOND),
        state: PLAY_STATE_LIVE,
        rule_version: SCORE_RULE_VERSION,
        ir_submitted: play.ir_submitted,
        replay_file: play.replay_file.clone(),
    }
}

/// Record a finished run, reporting rather than propagating a write failure: a score that cannot be
/// stored must not cost the player the result screen.
///
/// `updates_best` is decision 12's score flag — `crate::updates_score` — and is passed rather than
/// re-derived, so the database, the IR gate and the book can never disagree about one run.
pub(crate) fn record_finished_play(db: &mut ScoreDb, log: &PlayLog, updates_best: bool) -> bool {
    match db.record_play(log, updates_best) {
        Ok(_) => true,
        Err(e) => {
            notify(Level::Error, format!("score not recorded: {e}"));
            false
        }
    }
}

/// One stored play as the record the screens read.
///
/// The EX ceiling and the empty-poor tally are not columns of their own: both fall out of what is
/// stored, the ceiling from the note count and the tally from the miss tier, exactly as the play
/// session derives them.
pub(crate) fn record_of(log: &PlayLog) -> ScoreRecord {
    let mut counts = [0; JUDGE_TIER_COUNT];
    for (tier, count) in counts.iter_mut().enumerate() {
        *count = log.judge.tier(tier);
    }
    ScoreRecord {
        md5: log.md5.clone(),
        title: log.title.clone(),
        mode: mode_name(log.mode).to_string(),
        clear: log.clear,
        ex_score: log.judge.ex_score(),
        max_ex: log.notes * EX_PER_NOTE,
        counts,
        empty_poor: log.judge.tier(JUDGE_MISS),
        max_combo: log.combo,
        total_notes: log.notes,
        gauge: gauge_token_of(log.gauge),
        gauge_value: log.gauge_value,
        random: random_token(log.random).to_string(),
        played_at: log.date.saturating_mul(MILLIS_PER_SECOND),
        replay_file: log.replay_file.clone(),
        rule_version: log.rule_version,
        ln_mode: log.ln_mode.clone(),
        assisted: log.assist != 0,
    }
}

/// The settings-file spelling of a stored gauge id. The two crates keep the same table, so this is
/// a lookup rather than a conversion; a gauge id nothing names reads as the default gauge, which is
/// what [`rbms_config::gauge_from_name`] does with an unknown token.
fn gauge_token_of(id: i32) -> String {
    let token = rbms_store::scoredb::gauge_token(id);
    if token.is_empty() { gauge_token(GaugeKind::Normal).to_string() } else { token.to_string() }
}

/// Every stored play as the book the screens query.
///
/// The whole history is read, which is what loading `scores.ron` did; the difference is that it is
/// now read from an indexed store that also answers the questions the book cannot.
pub(crate) fn book_from_db(db: &ScoreDb) -> ScoreBook {
    match db.all_plays() {
        Ok(plays) => ScoreBook::from_records(plays.iter().map(record_of).collect()),
        Err(e) => {
            notify(Level::Error, format!("score records not read: {e}"));
            ScoreBook::default()
        }
    }
}

/// Apply the replay retention policy: settle the plan, delete the files it names and forget the
/// rows that pointed at them.
///
/// Deleting is done here rather than in the store crate so that a file that will not delete leaves
/// its row alone — the history keeps offering a replay that is still there, and the next run tries
/// again.
pub(crate) fn run_replay_gc(db: &mut ScoreDb, dir: &Path, policy: &ReplayPolicy) -> Option<ReplayGcPlan> {
    let plan = match db.replay_gc_plan(dir, policy) {
        Ok(plan) => plan,
        Err(e) => {
            notify(Level::Warn, format!("replay retention not applied: {e}"));
            return None;
        }
    };
    if plan.delete.is_empty() {
        return Some(plan);
    }
    let mut deleted = Vec::with_capacity(plan.delete.len());
    for file in &plan.delete {
        let path = dir.join(file);
        match std::fs::remove_file(&path) {
            Ok(()) => deleted.push(file.clone()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => deleted.push(file.clone()),
            Err(e) => notify(Level::Warn, format!("replay not deleted ({}): {e}", path.display())),
        }
    }
    if let Err(e) = db.clear_replay_refs(&deleted) {
        notify(Level::Warn, format!("deleted replays still referenced: {e}"));
    }
    Some(plan)
}

#[cfg(test)]
mod tests;
