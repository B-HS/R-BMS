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
//! A database that will not open is reported and the player runs on rather than refusing to start.
//! Every run it finishes meanwhile is kept on the side ([`PENDING_RUNS_DIR`]) and read into the
//! database the next time it opens ([`import_pending_runs`]), so no run is lost with the session.
//!
//! The application opens the database at startup and reads the book from it (`App::new`), and the
//! result screen writes every finished run through [`record_finished_play`]
//! (`app_result::record_run`).

use std::path::{Path, PathBuf};

use rbms_chart::shuffle::NoteOption;
use rbms_config::NetworkOptions;
use rbms_config::gauge_token;
use rbms_judge::gauge::GaugeIndex;
use rbms_judge::{ClearType, GaugeKind, clear_type_id};
use rbms_model::Mode;
use rbms_play::PlaySummary;
use rbms_store::scoredb::{
    JUDGE_MISS, JUDGE_TIER_COUNT, JudgeCounts, KeptSubmission, MigrationReport, PLAY_STATE_LIVE, PlayLog, ReplayGcPlan, ReplayPolicy, ScoreDb, UNSET_MINIMUM,
    UploadHold, chart_key, mode_id, mode_name, open_with_import, random_id, random_token,
};
use rbms_store::{SCORE_RULE_VERSION, ScoreBook, ScoreRecord};

use crate::notify::{Level, notify, notify_text};
use crate::text::notices::NoticeText;

/// Name of the score database inside the config directory, next to the settings and the song
/// database.
pub(crate) const SCOREDB_FILE: &str = "scoredb.sqlite";

/// Name of the RON score book the database is built from the first time it is created.
pub(crate) const SCORES_RON_FILE: &str = "scores.ron";

/// Name of the directory saved replays live in, next to the two databases.
pub(crate) const REPLAY_DIR: &str = "replays";

/// Name of the directory a run is kept in while the score database cannot take it, next to the
/// database ([`rbms_store::scoredb::PendingRun`]).
pub(crate) const PENDING_RUNS_DIR: &str = "pending-runs";

/// Where runs are kept on the side, given the directory the settings file is in.
pub(crate) fn pending_runs_dir(config_dir: &Path) -> PathBuf {
    config_dir.join(PENDING_RUNS_DIR)
}

/// Read the runs kept on the side while the database was closed into it, saying what came in and
/// what could not.
pub(crate) fn import_pending_runs(db: &mut ScoreDb, dir: &Path) {
    match db.import_pending_runs(dir) {
        Ok(report) => {
            if report.imported > 0 {
                notify_text(Level::Info, NoticeText::ImportedPendingRuns, &[&report.imported]);
            }
            for file in report.unreadable {
                notify_text(Level::Warn, NoticeText::KeptRunNotRead, &[&file.display()]);
            }
            for file in report.stuck {
                notify_text(Level::Warn, NoticeText::KeptRunNotRemoved, &[&file.display()]);
            }
            for (file, error) in report.refused {
                notify_text(Level::Warn, NoticeText::KeptRunNotReadIn, &[&file.display(), &error]);
            }
        }
        Err(e) => notify_text(Level::Error, NoticeText::KeptRunsNotReadIn, &[&dir.display(), &e]),
    }
}

/// The server and account whose replays the retention policy holds until they have gone up: the
/// server set, an account signed in to it and AUTO UPLOAD REPLAY on. With any of them missing no
/// sync uploads a replay, and holding one would only stop the policy from pruning.
pub(crate) fn upload_hold(network: &NetworkOptions) -> Option<UploadHold> {
    let server = network.server_url.as_deref().map(crate::record_sync::server_key).filter(|key| !key.is_empty())?;
    let login_id = network.ir_login_id.as_deref().map(str::trim).filter(|login| !login.is_empty())?;
    (network.auto_upload_replay && network.ir_token.is_some()).then(|| UploadHold { server, login_id: login_id.to_string() })
}

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
        Ok((mut db, report)) => {
            if let Some(report) = report {
                report_import(&report, ron_path);
            }
            put_course_stages_on_their_gauge(&mut db);
            Some(db)
        }
        Err(e) => {
            notify_text(Level::Error, NoticeText::ScoreDatabaseNotOpened, &[&db_path.display(), &e]);
            None
        }
    }
}

/// Put the stages of the courses the database keeps on the CLASS gauge they ran on
/// ([`ScoreDb::put_course_stages_on_class`]), a build before this one having stored each on the gauge
/// the settings chose. The stages are the rows the courses kept were played as
/// ([`crate::trace_backfill::course_stages`]); a stage of a course that was not kept is told by
/// nothing and stays as it was. A failure is reported and the database used as it stands.
fn put_course_stages_on_their_gauge(db: &mut ScoreDb) {
    let moved = crate::trace_backfill::course_stages(db).and_then(|stages| db.put_course_stages_on_class(&stages.into_iter().collect::<Vec<_>>()));
    match moved {
        Ok(0) => {}
        Ok(moved) => notify_text(Level::Info, NoticeText::CourseStagesMoved, &[&moved]),
        Err(e) => notify(Level::Warn, format!("course stages not put on their gauge: {e}")),
    }
}

/// Say what the one-time import did, so a player whose records moved can see where they went.
fn report_import(report: &MigrationReport, ron_path: &Path) {
    if report.records == 0 {
        return;
    }
    notify_text(Level::Info, NoticeText::ScoreRecordsImported, &[&report.records, &report.assisted, &ron_path.display()]);
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
    /// The gauge the run finished on ([`rbms_play::PlayRecord::finished_gauge`]): the one GAUGE AUTO
    /// SHIFT left it on, and for a course's stage the course gauge it ran on.
    pub(crate) gauge: GaugeIndex,
    pub(crate) random: NoteOption,
    pub(crate) seed: u64,
    /// Assist level (`rbms_ir::mapping::assist_level`): zero for an unassisted run.
    pub(crate) assist: u8,
    /// `ScoreData.avgjudge` of the run ([`rbms_play::PlayRecord::avg_duration_us`]):
    /// [`rbms_play::AVG_DURATION_ABSENT_US`] for a chart with nothing to judge, which is stored as
    /// no value at all.
    pub(crate) avg_duration_us: i64,
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
        id: 0,
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
        minbp: play.summary.min_bp_with_unreached(),
        avgjudge: if play.avg_duration_us == rbms_play::AVG_DURATION_ABSENT_US { UNSET_MINIMUM } else { play.avg_duration_us },
        gauge: play.gauge.index() as i32,
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

/// Record a finished run with the submission it is to be sent as, in one transaction, reporting
/// rather than propagating a write failure: a score that cannot be stored must not cost the player
/// the result screen. Returns the `scorelog` id the run was stored under, or `None` when it was not
/// stored. The run's trace is kept after it, on the save thread
/// ([`crate::save_worker::SaveWorker::keep_trace`]).
///
/// `updates_best` is decision 12's score flag — `crate::updates_score` — and is passed rather than
/// re-derived. The book reads the same answer back off the row ([`rbms_store::sets_best`] of its
/// assist level and rule version, in [`record_of`]), so the two cannot disagree about one run.
pub(crate) fn record_finished_play(db: &mut ScoreDb, log: &PlayLog, updates_best: bool, kept: Option<KeptSubmission<'_>>) -> Option<i64> {
    match db.record_play_kept(log, updates_best, kept) {
        Ok(id) => Some(id),
        Err(e) => {
            notify_text(Level::Error, NoticeText::ScoreNotRecorded, &[&e]);
            None
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
        assisted: !rbms_store::sets_best(log.assist, log.rule_version),
        assist: log.assist,
        log_id: (log.id > 0).then_some(log.id),
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
            notify_text(Level::Error, NoticeText::ScoreRecordsNotRead, &[&e]);
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
///
/// A replay still to go up to the server the player syncs to (`hold`) is kept until it has.
pub(crate) fn run_replay_gc(db: &mut ScoreDb, dir: &Path, policy: &ReplayPolicy, hold: Option<&UploadHold>) -> Option<ReplayGcPlan> {
    let plan = match db.replay_gc_plan(dir, policy, hold) {
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
