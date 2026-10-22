//! Traces for the runs stored before their client kept one, made from their replays.
//!
//! A run keeps its trace from the moment its client keeps traces ([`crate::play_trace`]). An older
//! run has only its row and its replay, and a replay recorded on the event clock plays back to the
//! run it records, note for note ([`crate::ghost`]), so its trace can still be made: the replay is
//! judged again on the chart it was played on and the trace is read off that run, as the play
//! screen reads it off a run it has just ended.
//!
//! A run judged again has to come to its row exactly: the lamp, every judgement count with its
//! early and late halves (the 空POOR among them), the note count, the combo, the gauge it ended on
//! and the assists, and the FAST and SLOW and the EX its kept submission states. Anything else is
//! not the run the row is of, and it gets no trace. A run whose replay was recorded on a frame
//! clock, whose replay or chart file is gone or is of another chart, or whose submission was not
//! kept gets none either.
//!
//! A run left before its end (Escape or quitting, [`rbms_play::PlaySummary::left_unfinished`])
//! stopped somewhere after its last input; its replay does not say where, and its row's playing time
//! is the chart's length, not where it stopped. It is judged again up to the first instant after
//! that input at which, every note not reached counted a POOR as the row counts it, it comes to the
//! row. Its notes not reached then stay at state 0 in the trace, which a server counts as the POORs
//! the row has for them. A run that comes to its row as well once a note more has gone by unhit, as
//! one whose gauge sat on its floor does, could have been left at either, and gets no trace. A run
//! an empty gauge ended stops by itself.
//!
//! The scroll settings of a run judged again are known only as its kept submission states them, which is
//! as they stood when it ended, and the shades as the settings' switches stand now show them, as
//! the play screen shows a shade it is switched off at nought ([`crate::app_input::shade_shown`]):
//! the trace starts and ends on those with no change between, the hidden band, which a submission
//! does not state, at nought.
//!
//! The stage of a course gets none: it started on the course's gauge and its combo, which its
//! replay does not keep. A row is a course's stage when it was stored on a course gauge, as this
//! client stores a stage on the gauge it ran on and a row imported from another client's history may
//! be, or when it stands where a course the score database keeps had its stages, the rows last stored
//! before the course ended being its charts in order ([`course_stages`]): a build before this one
//! stored a stage on the gauge the settings chose, a course gauge being none of those, and the
//! database is put right as it opens ([`crate::scoredb_store::open_score_db`]). The stage of a course
//! that was not kept, stored by such a build, is told by nothing but its judging again: started on
//! the course's gauge and carried combo, it comes to its row only where those leave no mark on the
//! gauge it ended on and its combo.
//!
//! The work goes on a thread of its own, a run at a time with a pause between, waiting while a
//! chart is played or loaded, and looking at that and at being cancelled while it searches for where
//! a run was left. The frame thread hands each batch to the save thread
//! ([`crate::save_worker::SaveWorker::keep_backfilled_traces`]), which keeps the traces and the runs
//! that will never get one, and marks how far the making has got, so the next start goes on from
//! there: every run up to the mark has a trace or never will. A run whose chart file or replay could
//! not be read now is not passed by the mark, and is looked at once more on each start and after a
//! library scan. A batch the save thread could not write leaves the mark where it was, and the batches
//! after it move it no further. A run that has a trace by then keeps its own. Once every run is looked
//! at, the frame thread says how many were filled in and a records sync sends them, while AUTO UPLOAD
//! TRACE is on.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::{Duration, Instant};

use rbms_ir::{CourseSubmission, PlayOptions, ScoreSubmission};
use rbms_library::songdb::SongDb;
use rbms_play::{PlayRecord, PlaySession, PlaySummary};
use rbms_store::scoredb::{BackfillBatch, JUDGE_BAD, JUDGE_GOOD, JUDGE_GREAT, JUDGE_PGREAT, PlayLog, ScoreDb, ScoreDbError, UntracedPlay};

use crate::app_input::shade_shown;
use crate::app_play::{DecodedChart, decode_chart};
use crate::ghost::session_of_replay;
use crate::play_trace::{SpeedLog, TraceRun, TraceSource, speed_of};
use crate::scoredb_store::{FinishedPlay, play_log, playtime_ms};
use crate::stage::StageId;
use crate::*;

/// How many stored runs the thread looks at before it hands what it made to the frame thread.
const BACKFILL_RUNS_PER_BATCH: usize = 16;

/// What the thread that makes the traces is called.
const BACKFILL_THREAD_NAME: &str = "rbms-trace-backfill";

/// How far a run left before its end is judged on at a time while its end is looked for: one
/// millisecond, finer than any two judgements a person could tell apart.
const LEFT_RUN_SEARCH_STEP_US: i64 = 1_000;

/// How many steps past its last input a run left before its end is judged on at most: twenty
/// minutes of chart, longer than any chart is played for. A run not found by then never is.
const LEFT_RUN_SEARCH_MAX_STEPS: i64 = 1_200_000;

/// How many steps the search goes between two looks at whether the thread is to wait or end: a
/// second of chart.
const LEFT_RUN_SEARCH_STEPS_PER_LOOK: i64 = 1_000;

/// How long one search may judge for, the waits aside. A search that takes longer is given up for
/// this start and tried again on the next.
const LEFT_RUN_SEARCH_MAX_WORK: Duration = Duration::from_secs(20);

/// The judgements that take a note by hitting it, PGREAT to BAD, as a row's tiers count them.
const HIT_TIERS: [usize; 4] = [JUDGE_PGREAT, JUDGE_GREAT, JUDGE_GOOD, JUDGE_BAD];

/// The hidden band a trace made from a kept submission states, which says nothing of it: nought,
/// as a band switched off shows.
const UNSTATED_HIDDEN: f32 = rbms_config::LANE_SHADE_MIN;

/// The gauges a course is played on, as a row names them.
const COURSE_GAUGE_TOKENS: [&str; 3] = ["class", "exclass", "exhardclass"];

/// Milliseconds in a second, which a kept course's end is stated in and a row's date is not.
const MILLIS_PER_SECOND: i64 = 1_000;

/// How slowly the traces are made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BackfillPacing {
    /// How long after the start the first run is looked at.
    pub(crate) start_delay: Duration,
    /// The pause after each run.
    pub(crate) between_runs: Duration,
    /// How often the thread looks again while it waits for a chart to end or to be cancelled.
    pub(crate) check: Duration,
}

impl BackfillPacing {
    /// The pacing of the game: a while after the start, then a run every quarter second.
    pub(crate) const GAME: BackfillPacing =
        BackfillPacing { start_delay: Duration::from_secs(30), between_runs: Duration::from_millis(250), check: Duration::from_millis(250) };
}

/// Which lane shades the settings have switched on, which a kept submission does not state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShadeSwitches {
    pub(crate) cover: bool,
    pub(crate) lift: bool,
}

impl ShadeSwitches {
    /// The switches as `config` holds them.
    pub(crate) fn of(config: &Config) -> ShadeSwitches {
        ShadeSwitches { cover: config.play.enable_cover, lift: config.play.enable_lift }
    }
}

/// Everything the thread needs, handed to it whole.
pub(crate) struct BackfillPlan {
    /// The score database file, read through a connection of the thread's own.
    pub(crate) db_path: PathBuf,
    /// The song database file, where a chart that is no longer where its replay says is looked for.
    pub(crate) songdb_path: PathBuf,
    pub(crate) replay_dir: PathBuf,
    /// The newest run stored when the thread was started: a run stored after it keeps its own trace.
    pub(crate) up_to: i64,
    /// Answered once the save thread has written what it was handed before the thread was started.
    pub(crate) saves_settled: Option<Receiver<()>>,
    pub(crate) shades: ShadeSwitches,
    pub(crate) runs_per_batch: usize,
    pub(crate) pacing: BackfillPacing,
}

/// What the thread reports.
#[derive(Debug, PartialEq)]
pub(crate) enum BackfillEvent {
    /// What some runs came to.
    Batch(BackfillBatch),
    /// Every run is looked at, or the score database could not be read (`unread`). `later` runs
    /// could not be looked at for now.
    Finished { unread: Option<String>, later: usize },
}

/// Why a stored run was given no trace.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// It never will be: it is not the run its row is of, or what it was made of is not kept.
    Never(String),
    /// Not for now: its chart file or its replay could not be read, which a later start or a
    /// library scan may change.
    Later(String),
    /// The thread was cancelled while it judged.
    Stopped,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Never(why) => write!(f, "{why}"),
            Refusal::Later(why) => write!(f, "{why}, for now"),
            Refusal::Stopped => write!(f, "the work was cancelled"),
        }
    }
}

/// Where a run's trace is made from: its replay, its chart file, the shades the settings switch on,
/// and what to do between steps of a long search, which waits while the thread is held and answers
/// whether it is cancelled.
pub(crate) struct Sources<'a> {
    pub(crate) replay_dir: &'a Path,
    pub(crate) songs: Option<&'a SongDb>,
    pub(crate) shades: ShadeSwitches,
    pub(crate) cancelled: &'a dyn Fn() -> bool,
}

/// The making of traces as the frame thread holds it.
#[derive(Default)]
pub(crate) struct TraceBackfill {
    run: Option<BackfillRun>,
    /// Answered once the save thread has written the last traces, before a records sync sends them.
    written: Option<Receiver<()>>,
    /// A records sync is to send the traces made, once none is running.
    sync_due: bool,
    /// How the last start was paced, which a start after a library scan is paced by too.
    pacing: Option<BackfillPacing>,
    /// The last run of the thread left runs it could not look at for now.
    later_left: bool,
    /// A library scan ended while the thread was at work: it starts again once it is done, when it
    /// left runs it could not look at.
    scanned_meanwhile: bool,
}

/// The thread at work, and how many traces it has made.
struct BackfillRun {
    rx: Receiver<BackfillEvent>,
    cancel: Arc<AtomicBool>,
    hold: Arc<AtomicBool>,
    db_path: PathBuf,
    filled: usize,
}

impl Drop for BackfillRun {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// What the message bus says once traces were made for `filled` earlier runs.
fn filled_line(filled: usize) -> String {
    format!("play traces filled in for {filled} earlier runs from their replays")
}

/// Start the thread that makes the traces of `plan`, reporting over the returned channel. `hold`
/// keeps it waiting, and `cancel` ends it. `None` when the thread could not be started.
fn spawn_backfill(plan: BackfillPlan, cancel: Arc<AtomicBool>, hold: Arc<AtomicBool>) -> Option<Receiver<BackfillEvent>> {
    let (tx, rx) = channel();
    let spawned = std::thread::Builder::new().name(BACKFILL_THREAD_NAME.to_string()).spawn(move || {
        let looked = backfill(plan, &cancel, &hold, &tx);
        let finished = match looked {
            Ok(later) => BackfillEvent::Finished { unread: None, later },
            Err(e) => BackfillEvent::Finished { unread: Some(e.to_string()), later: 0 },
        };
        let _ = tx.send(finished);
    });
    match spawned {
        Ok(_) => Some(rx),
        Err(e) => {
            println!("traces of earlier runs not made: the thread did not start ({e})");
            None
        }
    }
}

/// Wait `pause`, and after it for as long as `hold` is set, looking at `cancel` every `check`.
/// Returns whether the thread was cancelled.
fn wait(pause: Duration, check: Duration, cancel: &AtomicBool, hold: &AtomicBool) -> bool {
    let until = Instant::now() + pause;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return true;
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() && !hold.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(if left.is_zero() { check } else { left.min(check) });
    }
}

/// Look at every run of `plan` with no trace that was not found never to get one, oldest first, a
/// batch at a time. Answers how many could not be looked at for now.
fn backfill(plan: BackfillPlan, cancel: &AtomicBool, hold: &AtomicBool, tx: &Sender<BackfillEvent>) -> Result<usize, ScoreDbError> {
    let check = plan.pacing.check;
    if wait(plan.pacing.start_delay, check, cancel, hold) {
        return Ok(0);
    }
    if let Some(settled) = &plan.saves_settled {
        let _ = settled.recv();
    }
    let db = ScoreDb::open_read_only(&plan.db_path)?;
    let songs = SongDb::open(&plan.songdb_path).ok();
    let stages = course_stages(&db)?;
    let cancelled = || wait(Duration::ZERO, check, cancel, hold);
    let sources = Sources { replay_dir: &plan.replay_dir, songs: songs.as_ref(), shades: plan.shades, cancelled: &cancelled };
    let mut mark = db.trace_backfill_through()?;
    let mut after = mark;
    let mut first_later: Option<i64> = None;
    let mut later = 0;
    loop {
        let plays = db.untraced_plays(after, plan.up_to, plan.runs_per_batch)?;
        let Some(last) = plays.last().map(|play| play.log.id) else {
            return Ok(later);
        };
        let mut batch = BackfillBatch { from: mark, ..BackfillBatch::default() };
        for play in &plays {
            if cancelled() {
                return Ok(later);
            }
            let id = play.log.id;
            let made = match course_stage(play, &stages) {
                Some(why) => Err(Refusal::Never(why)),
                None => trace_of(play, &sources),
            };
            match made {
                Ok(body) => batch.traces.push((id, body)),
                Err(Refusal::Stopped) => return Ok(later),
                Err(Refusal::Never(why)) => {
                    println!("run {id} given no trace: {why}");
                    batch.refused.push(id);
                }
                Err(Refusal::Later(why)) => {
                    println!("run {id} given no trace for now: {why}");
                    later += 1;
                    first_later.get_or_insert(id);
                }
            }
            if wait(plan.pacing.between_runs, check, cancel, hold) {
                return Ok(later);
            }
        }
        batch.through = first_later.map_or(last, |id| id - 1);
        mark = batch.through;
        if tx.send(BackfillEvent::Batch(batch)).is_err() {
            return Ok(later);
        }
        after = last;
    }
}

/// Why `play` is a course's stage, which gets no trace, or `None` when it is not one: it is among
/// `stages`, or it was stored on a course gauge.
fn course_stage(play: &UntracedPlay, stages: &HashSet<i64>) -> Option<String> {
    if stages.contains(&play.log.id) {
        return Some("it is the stage of a course".to_string());
    }
    let gauge = rbms_store::scoredb::gauge_token(play.log.gauge);
    COURSE_GAUGE_TOKENS.contains(&gauge).then(|| format!("it was played on the course gauge {gauge}"))
}

/// The rows `db` holds as the stages of the courses it keeps. A course's stages are stored as they
/// end, one after the other with nothing between, and the course is kept as its last stage played
/// ends, or as it is left: the rows last stored by the second a course ended are its stages when
/// their charts are its first ones, in order. Where the same chart is a course's twice, the most
/// rows that fit are taken.
pub(crate) fn course_stages(db: &ScoreDb) -> Result<HashSet<i64>, ScoreDbError> {
    let mut stages = HashSet::new();
    for course in db.kept_courses()? {
        let Some(sub) = CourseSubmission::from_json(&course.body) else {
            continue;
        };
        let charts: Vec<&str> = sub.charts.iter().map(|chart| chart.md5.as_str()).collect();
        let newest_first = db.plays_ended_by(course.played_at_ms.div_euclid(MILLIS_PER_SECOND), charts.len())?;
        let last_played = (0..newest_first.len()).rev().find(|last| (0..=*last).all(|back| newest_first[back].md5.eq_ignore_ascii_case(charts[last - back])));
        if let Some(last) = last_played {
            stages.extend(newest_first[..=last].iter().map(|log| log.id));
        }
    }
    Ok(stages)
}

/// The trace of the stored run `play`, made from its replay as `sources` find it, or why it gets
/// none.
pub(crate) fn trace_of(play: &UntracedPlay, sources: &Sources<'_>) -> Result<String, Refusal> {
    let log = &play.log;
    let never = |why: String| Refusal::Never(why);
    let body = play.body.as_deref().ok_or_else(|| never("it was stored before its submission was kept".to_string()))?;
    let sub = ScoreSubmission::from_json(body).ok_or_else(|| never("its kept submission does not read".to_string()))?;
    let file = log.replay_file.as_deref().ok_or_else(|| never("it kept no replay".to_string()))?;
    let replay = Replay::load(&sources.replay_dir.join(file)).map_err(|e| Refusal::Later(format!("its replay could not be read ({e})")))?;
    if !replay.md5.eq_ignore_ascii_case(&log.md5) {
        return Err(never(format!("its replay is of another chart ({})", replay.md5)));
    }
    if replay.judge.clock != rbms_store::REPLAY_EVENT_CLOCK {
        return Err(never(format!("its replay was judged on the {} clock, which is not played back note for note", replay.judge.clock)));
    }
    let chart = chart_of(&log.md5, &replay.chart_path, sources.songs).ok_or_else(|| Refusal::Later("its chart file was not found".to_string()))?;
    if !log.sha256.is_empty() && !chart.model.sha256.eq_ignore_ascii_case(&log.sha256) {
        return Err(never(format!("its chart file hashes to another SHA-256 ({})", chart.model.sha256)));
    }
    if sub.mode != chart.mode.name {
        return Err(never(format!("its submission names the mode {}, and its chart is {}", sub.mode, chart.mode.name)));
    }
    let (session, record) = judged_again(&chart, replay, log, &sub, sources.cancelled)?;
    let speed_end = speed_stated_by(&sub.options, session.model().init_bpm, sources.shades);
    let speed = SpeedLog::default();
    let run = TraceRun { mode: chart.mode.name, lntype: sub.options.lntype, random: sub.options.random, speed: &speed, speed_end };
    TraceSource::of(&session, &record, run).body().ok_or_else(|| never("its trace does not serialise".to_string()))
}

/// The chart file of `md5`: where the replay says it was played from, or wherever the library
/// has a file of that hash.
fn chart_of(md5: &str, recorded_path: &str, songs: Option<&SongDb>) -> Option<DecodedChart> {
    let listed = songs.and_then(|songs| songs.by_md5(md5).ok()).unwrap_or_default().into_iter().map(|row| row.path);
    std::iter::once(recorded_path.to_string()).chain(listed).find_map(|path| {
        let bytes = std::fs::read(&path).ok()?;
        decode_chart(&bytes, &path).filter(|chart| chart.md5.eq_ignore_ascii_case(md5))
    })
}

/// The scroll settings `options` states, as a trace states them for a chart opening at `init_bpm`,
/// with the shades as `shades` switch them on, which is how the play screen shows them.
fn speed_stated_by(options: &PlayOptions, init_bpm: f64, shades: ShadeSwitches) -> rbms_ir::TraceSpeed {
    let mut config = Config::default();
    config.play.hispeed = options.hispeed;
    config.play.constant_speed = options.constant;
    config.judge.offset_ms = options.offset_ms;
    config.judge.auto_offset = options.auto_offset;
    speed_of(&config, shade_shown(shades.cover, options.lane_cover), shade_shown(shades.lift, options.lift), UNSTATED_HIDDEN, init_bpm)
}

/// What a run judged again from `replay` is checked against its row with.
struct Rejudge<'a> {
    chart: &'a DecodedChart,
    replay: &'a Replay,
    log: &'a PlayLog,
    sub: &'a ScoreSubmission,
    ln_mode_key: &'static str,
    custom_judge: bool,
}

impl Rejudge<'_> {
    /// A fresh run of the replay, nothing judged yet.
    fn session(&self) -> PlaySession {
        session_of_replay(self.chart.model.clone(), self.chart.bmson.as_deref(), self.chart.mode, self.replay.clone())
    }

    /// The row `session`, scored as `summary` and recorded as `record`, would have been stored as.
    fn stored_as(&self, session: &PlaySession, summary: &PlaySummary, record: &PlayRecord) -> PlayLog {
        let assist = rbms_ir::mapping::assist_level(session.has_auto_played_notes(), self.custom_judge);
        play_log(&FinishedPlay {
            md5: &self.log.md5,
            sha256: &self.log.sha256,
            title: &self.log.title,
            mode: self.chart.mode,
            ln_mode: self.ln_mode_key,
            lamp: assisted_lamp(summary.clear_lamp, assist),
            summary,
            gauge: record.finished_gauge,
            random: rbms_chart::shuffle::NoteOption::from_str(&self.replay.random),
            seed: self.replay.seed,
            assist,
            avg_duration_us: record.avg_duration_us,
            played_at_ms: 0,
            playtime_ms: playtime_ms(session.last_time_us()),
            ir_submitted: false,
            replay_file: None,
        })
    }

    /// The run judged up to `at_us` and left there, with its record.
    fn left_at(&self, at_us: i64) -> (PlaySession, PlayRecord) {
        let mut session = self.session();
        session.tick(SessionClock::at(at_us), &mut NullSink);
        let record = session.record();
        (session, record)
    }

    /// Whether `session`, scored as `summary`, is the row's run: what is cheap to look at first, and
    /// its record worked out only then.
    fn is_the_row(&self, session: &PlaySession, summary: &PlaySummary) -> bool {
        self.may_be_the_row(summary) && self.differs(session, summary, &session.record()).is_none()
    }

    /// Whether `summary` has taken more of some judgement that hits a note than the row has, which
    /// judging on cannot take back.
    fn outgrew_the_row(&self, summary: &PlaySummary) -> bool {
        let row = &self.log.judge;
        HIT_TIERS.iter().any(|tier| summary.early[*tier] > row.early[*tier] || summary.late[*tier] > row.late[*tier])
    }

    /// Whether `summary` has the judgements and the gauge of the row, the least a run that is the
    /// row's has: what is cheap to look at on every step of a search, before the run's record is
    /// worked out.
    fn may_be_the_row(&self, summary: &PlaySummary) -> bool {
        rbms_store::scoredb::JudgeCounts::from_early_late(summary.early, summary.late) == self.log.judge
            && summary.gauge_value.to_bits() == self.log.gauge_value.to_bits()
    }

    /// The first figure `session`, scored as `summary` and recorded as `record`, does not share with
    /// the row and its kept submission, or `None` when it shares them all.
    fn differs(&self, session: &PlaySession, summary: &PlaySummary, record: &PlayRecord) -> Option<String> {
        let judged = self.stored_as(session, summary, record);
        let stored = self.log;
        if judged.judge != stored.judge {
            return Some(format!("its judgements came to {:?}, and its row says {:?}", judged.judge, stored.judge));
        }
        if judged.gauge_value.to_bits() != stored.gauge_value.to_bits() {
            return Some(format!("its gauge ended at {}, and its row says {}", judged.gauge_value, stored.gauge_value));
        }
        if judged.ln_mode != stored.ln_mode {
            return Some(format!("it was judged as {}, and its row is of {}", judged.ln_mode, stored.ln_mode));
        }
        if summary.empty_poor != self.sub.judge.empty_poor {
            return Some(format!("it came to {} 空POOR, and its submission says {}", summary.empty_poor, self.sub.judge.empty_poor));
        }
        let mut figures = vec![
            ("lamp", i64::from(judged.clear), i64::from(stored.clear)),
            ("note count", i64::from(judged.notes), i64::from(stored.notes)),
            ("combo", i64::from(judged.combo), i64::from(stored.combo)),
            ("gauge", i64::from(judged.gauge), i64::from(stored.gauge)),
            ("assist", i64::from(judged.assist), i64::from(stored.assist)),
            ("mode", i64::from(judged.mode), i64::from(stored.mode)),
            ("seed", judged.seed, stored.seed),
            ("lane option", i64::from(judged.random), i64::from(stored.random)),
            ("FAST", i64::from(summary.fast), i64::from(self.sub.judge.fast)),
            ("SLOW", i64::from(summary.slow), i64::from(self.sub.judge.slow)),
            ("EX", i64::from(summary.ex_score), i64::from(self.sub.ex_score)),
        ];
        if stored.rule_version >= rbms_store::REFERENCE_RECORD_RULE_VERSION {
            figures.push(("bad-poor count", i64::from(judged.minbp), i64::from(stored.minbp)));
            figures.push(("mean distance", judged.avgjudge, stored.avgjudge));
        }
        figures
            .iter()
            .find(|(_, judged, stored)| judged != stored)
            .map(|(what, judged, stored)| format!("its {what} came to {judged}, and its row says {stored}"))
    }
}

/// The run `replay` records, judged again on `chart` until it is the run `log` stores, with its
/// record: played out whole, or, for a FAILED run left before its end, stopped where it was left.
///
/// A FAILED run is played out whole when the gauge emptied, which ends it by itself. Otherwise it was
/// left before its end, and where is looked for: from its last input on, a step of
/// [`LEFT_RUN_SEARCH_STEP_US`] at a time, the run is left at each instant until it comes to its row.
/// It has to come to it at one place only. A row it also comes to once one more note has gone by
/// says nothing of which of those notes the run reached, as when no note moved its gauge, sunk to its
/// floor: such a run gets no trace. The search asks `cancelled` every
/// [`LEFT_RUN_SEARCH_STEPS_PER_LOOK`] steps, which waits while the thread is held, and gives up after
/// [`LEFT_RUN_SEARCH_MAX_STEPS`] steps for good, or after [`LEFT_RUN_SEARCH_MAX_WORK`] of judging for
/// now. Otherwise, why it is not the row's run.
fn judged_again(
    chart: &DecodedChart,
    replay: Replay,
    log: &PlayLog,
    sub: &ScoreSubmission,
    cancelled: &dyn Fn() -> bool,
) -> Result<(PlaySession, PlayRecord), Refusal> {
    let setup = crate::judge_setup::replay_judge_setup(&replay.judge);
    let ln_mode_key = match rbms_chart::contains_undefined_long_note(&chart.model) {
        true => rbms_config::ln_mode_token(setup.ln_mode),
        false => rbms_store::SCORE_LN_MODE_FROM_CHART,
    };
    let lntype = run_lntype(chart.lnmode, setup.ln_mode);
    if lntype != sub.options.lntype {
        return Err(Refusal::Never(format!("it was judged again as long-note type {lntype}, and its submission says {}", sub.options.lntype)));
    }
    let custom_judge = judged_wider(&setup, replay.judge.total_override);
    let check = Rejudge { chart, replay: &replay, log, sub, ln_mode_key, custom_judge };

    let mut whole = check.session();
    whole.play_out();
    let summary = whole.summary();
    let record = whole.record();
    let failed_row = log.clear == clear_type_id(ClearType::Failed);
    let why = match check.differs(&whole, &summary, &record) {
        None if !failed_row || summary.failed => return Ok((whole, record)),
        None => "it was left before its end".to_string(),
        Some(why) if !failed_row => return Err(Refusal::Never(why)),
        Some(why) => why,
    };
    let from_us = replay.events.iter().map(|event| event.t).max().unwrap_or_default();
    let mut left = check.session();
    let mut found: Option<(i64, u32)> = None;
    let mut worked = Duration::ZERO;
    let mut since = Instant::now();
    for step in 0..LEFT_RUN_SEARCH_MAX_STEPS {
        if step % LEFT_RUN_SEARCH_STEPS_PER_LOOK == 0 {
            worked += since.elapsed();
            if worked > LEFT_RUN_SEARCH_MAX_WORK {
                return Err(Refusal::Later(format!("{why}; and where it was left was not found in {}s of judging", LEFT_RUN_SEARCH_MAX_WORK.as_secs())));
            }
            if cancelled() {
                return Err(Refusal::Stopped);
            }
            since = Instant::now();
        }
        let at_us = from_us + step * LEFT_RUN_SEARCH_STEP_US;
        left.tick(SessionClock::at(at_us), &mut NullSink);
        let reached = left.summary();
        let summary = reached.left_unfinished();
        match found {
            None if check.is_the_row(&left, &summary) => found = Some((at_us, reached.total_judged)),
            Some((_, judged)) if reached.total_judged > judged && check.is_the_row(&left, &summary) => {
                return Err(Refusal::Never(format!("{why}; and it comes to its row left with more notes gone by too, so where it was left is not known")));
            }
            Some((found_us, judged)) if reached.total_judged > judged => return Ok(check.left_at(found_us)),
            None | Some(_) => {}
        }
        if left.is_failed() || left.is_finished(at_us) || check.outgrew_the_row(&summary) {
            return found
                .map(|(found_us, _)| check.left_at(found_us))
                .ok_or_else(|| Refusal::Never(format!("{why}; nor did it come to its row left before its end")));
        }
    }
    Err(Refusal::Never(format!("{why}; nor did it come to its row in the {LEFT_RUN_SEARCH_MAX_STEPS} steps after its last input")))
}

impl AppShared {
    /// Start making the traces of the runs stored without one, unless the score database is not
    /// open or holds no run.
    pub(crate) fn start_trace_backfill(&mut self, pacing: BackfillPacing) {
        self.trace_backfill.pacing = Some(pacing);
        let Some(Ok(Some(up_to))) = self.scoredb.as_ref().map(ScoreDb::newest_play_id) else {
            return;
        };
        let config_dir = self.settings_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let (db_path, _, _) = crate::scoredb_store::score_paths(&config_dir);
        let plan = BackfillPlan {
            db_path: db_path.clone(),
            songdb_path: songdb_path(&self.settings_path),
            replay_dir: self.replay_dir.clone(),
            up_to,
            saves_settled: self.saves.settled(),
            shades: ShadeSwitches::of(&self.config),
            runs_per_batch: BACKFILL_RUNS_PER_BATCH,
            pacing,
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let hold = Arc::new(AtomicBool::new(false));
        let Some(rx) = spawn_backfill(plan, Arc::clone(&cancel), Arc::clone(&hold)) else {
            return;
        };
        self.trace_backfill.later_left = false;
        self.trace_backfill.scanned_meanwhile = false;
        self.trace_backfill.run = Some(BackfillRun { rx, cancel, hold, db_path, filled: 0 });
    }

    /// A library scan has ended: the runs whose chart file was not found are looked at again, once
    /// the thread is done if it is at work, and only when the last start left some.
    pub(crate) fn trace_backfill_after_scan(&mut self) {
        if self.trace_backfill.run.is_some() {
            self.trace_backfill.scanned_meanwhile = true;
            return;
        }
        if let (true, Some(pacing)) = (self.trace_backfill.later_left, self.trace_backfill.pacing) {
            self.start_trace_backfill(pacing);
        }
    }

    /// Take what the thread made since the last frame and hand it to the save thread; hold the
    /// thread while `stage` plays or loads a chart; once it is done, say how many runs it filled in
    /// and have a records sync send them, and start it again when a library scan ended meanwhile
    /// and it left runs it could not look at.
    pub(crate) fn poll_trace_backfill(&mut self, stage: StageId) {
        let finished = self.trace_backfill.run.as_mut().and_then(|run| {
            run.hold.store(matches!(stage, StageId::Decide | StageId::Loading | StageId::Play | StageId::Practice), Ordering::Relaxed);
            run.take_events(&self.saves)
        });
        if let Some((unread, later)) = finished {
            if let Some(why) = unread {
                notify(Level::Warn, format!("traces of earlier runs not made: {why}"));
            }
            let filled = self.trace_backfill.run.take().map_or(0, |run| run.filled);
            if filled > 0 {
                notify(Level::Info, filled_line(filled));
                self.trace_backfill.written = self.saves.settled();
                self.trace_backfill.sync_due = self.config.network.auto_upload_trace;
            }
            self.trace_backfill.later_left = later > 0;
            if std::mem::take(&mut self.trace_backfill.scanned_meanwhile) {
                self.trace_backfill_after_scan();
            }
        }
        if let Some(written) = self.trace_backfill.written.as_ref() {
            match written.try_recv() {
                Err(TryRecvError::Empty) => return,
                Ok(()) | Err(TryRecvError::Disconnected) => self.trace_backfill.written = None,
            }
        }
        if self.trace_backfill.sync_due && self.record_sync.is_none() {
            self.trace_backfill.sync_due = false;
            self.start_record_sync(crate::record_sync::SyncScope::All);
        }
    }
}

impl BackfillRun {
    /// Hand every batch the thread made since the last look to `saves`. Answers, once the thread
    /// is done, why it read nothing more, if it could not, and how many runs it could not look at
    /// for now.
    fn take_events(&mut self, saves: &crate::save_worker::SaveWorker) -> Option<(Option<String>, usize)> {
        loop {
            let event = match self.rx.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => BackfillEvent::Finished { unread: Some("the thread stopped".to_string()), later: 0 },
            };
            match event {
                BackfillEvent::Batch(batch) => {
                    self.filled += batch.traces.len();
                    saves.keep_backfilled_traces(self.db_path.clone(), batch);
                }
                BackfillEvent::Finished { unread, later } => return Some((unread, later)),
            }
        }
    }
}

#[cfg(test)]
mod tests;
