//! The record run a run is held against, note by note.
//!
//! The reference holds a run against a stored run note by note: every run keeps the judgment of
//! each note it consumed, in the order it consumed them (`JudgeManager.java:137-140, 639-646`), and
//! while a later run goes on, the stored run stands at what it had scored after as many notes as
//! this one has judged (`ScoreDataProperty.java:114-118`). Where that list is missing, or is not of
//! the chart's note count, the stored score is taken as earned evenly instead (`:119-121`).
//!
//! The score database here keeps no such list, so the stored run is judged again from its replay,
//! on a thread of its own while the chart loads: the replay played out with no screen and no sound
//! on the chart laid out as it was recorded ([`rejudge`]). A replay that does not come out as the
//! run it was recorded as -- another chart, another note count, another PGREAT or GREAT tally -- is
//! no ghost, and that score is paced evenly as before. See `docs/reference/record-target-pace.md`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};

use rbms_chart::shuffle::NoteOption;
use rbms_config::{ScoreTarget, gauge_from_name};
use rbms_judge::Judge;
use rbms_model::{Mode, Model};
use rbms_parser::bmson::BmsonChart;
use rbms_play::{PlaySession, SessionOptions};
use rbms_store::{Replay, ScoreRecord};

use crate::AppShared;
use crate::app_play::{lay_out_notes, scratch_auto_lanes};
use crate::ir_ranking::RankingState;
use crate::judge_setup::replay_judge_setup;

/// EX a PGREAT and a GREAT are worth (`ScoreDataProperty.java:137-144`); every other judgment is
/// worth none.
const PGREAT_EX: u32 = 2;
const GREAT_EX: u32 = 1;

/// Where a judge engine's tally keeps its PGREATs and its GREATs.
const PGREAT_TIER: usize = 0;
const GREAT_TIER: usize = 1;

/// A played-at time of the score server is in milliseconds; a local record keeps whole seconds.
const MILLIS_PER_SECOND: i64 = 1_000;

/// What the thread that judges stored runs again is called.
const GHOST_THREAD_NAME: &str = "rbms-ghost";

/// The EX a judgment adds to a run.
fn judgment_ex(judge: Judge) -> u32 {
    match judge {
        Judge::PerfectGreat => PGREAT_EX,
        Judge::Great => GREAT_EX,
        _ => 0,
    }
}

/// A stored run's EX after each number of notes it consumed, from none to all of them: the
/// reference's ghost (`ScoreData.ghost`), summed as `ScoreDataProperty.update` sums it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GhostPace {
    /// `ex_after[k]` is the EX of the run's first `k` consumed notes; there are `total_notes + 1`.
    ex_after: Box<[u32]>,
}

impl GhostPace {
    /// The pace of a run of `total_notes` notes that consumed `judges`, in the order it did. A note
    /// the run never reached -- it failed, or was left -- is a POOR, as the reference starts every
    /// note of its ghost at one (`JudgeManager.java:137-140`).
    pub(crate) fn of_judges(judges: &[Judge], total_notes: u32) -> GhostPace {
        let consumed = (0..total_notes as usize).map(|at| judges.get(at).map_or(0, |judge| judgment_ex(*judge)));
        let ex_after = std::iter::once(0)
            .chain(consumed.scan(0, |ex, gained| {
                *ex += gained;
                Some(*ex)
            }))
            .collect();
        GhostPace { ex_after }
    }

    /// The EX the run had after `judged` notes; its final EX for anything past its last note.
    pub(crate) fn ex_after(&self, judged: u32) -> u32 {
        let last = self.ex_after.len() - 1;
        self.ex_after[(judged as usize).min(last)]
    }

    /// How many notes the run was played over.
    pub(crate) fn total_notes(&self) -> u32 {
        (self.ex_after.len() - 1) as u32
    }

    /// The EX the run finished on.
    pub(crate) fn final_ex(&self) -> u32 {
        self.ex_after[self.ex_after.len() - 1]
    }
}

/// A stored run to judge again, and what its record says it came to, which the run judged again
/// has to come to as well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GhostSource {
    /// The replay's file name, under the replay folder.
    file: String,
    total_notes: u32,
    pgreat: u32,
    great: u32,
}

impl GhostSource {
    /// The run `record` is of, when it kept a replay.
    fn of(record: &ScoreRecord) -> Option<GhostSource> {
        let file = record.replay_file.clone()?;
        Some(GhostSource { file, total_notes: record.total_notes, pgreat: record.counts[PGREAT_TIER], great: record.counts[GREAT_TIER] })
    }
}

/// The stored runs a run of one chart may be held against note by note: the player's best here,
/// which the skin's BEST figures pace against whatever the target is, and the run the score server
/// holds as this account's best, when the TARGET row asks for that and the run was played here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GhostSources {
    best: Option<GhostSource>,
    account: Option<GhostSource>,
}

impl GhostSources {
    /// Whether there is any run to judge again.
    pub(crate) fn any(&self) -> bool {
        self.best.is_some() || self.account.is_some()
    }
}

/// The stored runs a run of the chart `md5`, played under `ln_mode`, may be held against.
///
/// The best is the run the record book keeps for it ([`rbms_store::ScoreBook::best_record_with_replay_in_ln_mode`]).
/// The account's best is the run of this machine that was played at the same second as the best
/// the ranking cache holds for this account and scored the same EX: a run the score server got from
/// another machine has no replay here, and one from the server itself is not judged again
/// ([`crate::ir_replay`] carries no judge algorithm, long-note mode or clock), so it is paced evenly.
pub(crate) fn ghost_sources(shared: &AppShared, md5: &str, ln_mode: &str) -> GhostSources {
    let best = shared.scores.best_record_with_replay_in_ln_mode(md5, ln_mode).and_then(GhostSource::of);
    let account = (shared.config.judge.target == ScoreTarget::IrBest).then(|| account_best_played_here(shared, md5, ln_mode)).flatten();
    GhostSources { best, account }
}

/// The run of this machine that is this account's best on the score server, as the ranking cache
/// holds it: of the chart `md5`, under `ln_mode`, played at the same second and for the same EX.
fn account_best_played_here(shared: &AppShared, md5: &str, ln_mode: &str) -> Option<GhostSource> {
    let Some(RankingState::Ready(board)) = shared.ranking_cache.peek(md5) else {
        return None;
    };
    let you = board.you.as_ref()?;
    let second = you.played_at.div_euclid(MILLIS_PER_SECOND);
    shared
        .scores
        .for_md5(md5)
        .into_iter()
        .filter(|record| record.ln_mode == ln_mode && record.ex_score == you.ex_score)
        .filter(|record| record.played_at.div_euclid(MILLIS_PER_SECOND) == second)
        .find_map(GhostSource::of)
}

/// The stored runs of one chart, judged again: their paces, or `None` for one that could not be.
#[derive(Debug, Clone, Default)]
pub(crate) struct RunGhosts {
    pub(crate) best: Option<Arc<GhostPace>>,
    pub(crate) account: Option<Arc<GhostPace>>,
}

/// Stored runs being judged again on their own thread, and the cancel that stops it.
pub(crate) struct GhostLoad {
    pub(crate) rx: Receiver<RunGhosts>,
    pub(crate) cancel: Arc<AtomicBool>,
}

/// Everything judging a chart's stored runs again needs, handed to its thread whole.
pub(crate) struct GhostJob {
    /// The chart as it was read, before any long-note flavour, lane option or TOTAL was put on it.
    pub(crate) model: Model,
    pub(crate) bmson: Option<Box<BmsonChart>>,
    pub(crate) mode: Mode,
    /// The hash of the chart's file, which a replay of it states.
    pub(crate) md5: String,
    pub(crate) replay_dir: PathBuf,
    pub(crate) sources: GhostSources,
    /// Answers once every write handed to the save thread before the chart was loaded is done, so
    /// the replay of a best set by the run just before is on disk before it is read.
    pub(crate) settled: Option<Receiver<()>>,
}

impl GhostJob {
    /// Judge the runs again on a thread of their own. The answer is `None` for every run when the
    /// thread could not be started, which leaves both scores paced evenly.
    pub(crate) fn spawn(self) -> GhostLoad {
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let spawned = std::thread::Builder::new().name(GHOST_THREAD_NAME.to_string()).spawn(move || {
            let ghosts = self.judge_all(&stop);
            let _ = tx.send(ghosts);
        });
        if let Err(e) = spawned {
            println!("the record runs could not be judged again ({e}); their scores are paced evenly");
        }
        GhostLoad { rx, cancel }
    }

    /// Judge every run again, the same file once.
    fn judge_all(self, cancel: &AtomicBool) -> RunGhosts {
        if let Some(settled) = &self.settled {
            let _ = settled.recv();
        }
        let judge_again = |source: &GhostSource| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            match rejudge(&self, source) {
                Ok(pace) => Some(Arc::new(pace)),
                Err(why) => {
                    println!("record run {} is paced evenly: {why}", source.file);
                    None
                }
            }
        };
        let best = self.sources.best.as_ref().and_then(judge_again);
        let account = match (&self.sources.account, &self.sources.best) {
            (Some(account), Some(best_run)) if account.file == best_run.file => best.clone(),
            (account, _) => account.as_ref().and_then(judge_again),
        };
        RunGhosts { best, account }
    }
}

/// A run of `model`, the chart as it was read, set up to play `replay` back with no screen and no
/// sound: the chart laid out as the replay was recorded on (its long-note mode, lane option, seed
/// and TOTAL), judged by every JUDGE row it recorded, on its gauge and its SCRATCH AUTO, as a
/// watched replay is ([`AppShared::load`]). Nothing is judged until the caller moves its clock.
pub(crate) fn session_of_replay(mut model: Model, bmson: Option<&BmsonChart>, mode: Mode, replay: Replay) -> PlaySession {
    let setup = replay_judge_setup(&replay.judge);
    rbms_chart::resolve_long_note_flavour(&mut model, setup.ln_mode.resolve());
    lay_out_notes(&mut model, NoteOption::from_str(&replay.random), replay.seed, bmson, &mode);
    if crate::overrides_total(replay.judge.total_override) {
        model.meta.total = replay.judge.total_override;
    }
    let options = SessionOptions {
        gauge: gauge_from_name(&replay.gauge),
        auto_lanes: scratch_auto_lanes(mode, replay.scratch_auto),
        seed: replay.seed,
        replay: Some(replay),
        ..SessionOptions::default()
    };
    let mut session = PlaySession::new(model, options);
    session.set_judge_setup(setup);
    session
}

/// Judge the stored run `source` again from its replay, played out at once ([`session_of_replay`]).
/// The run that comes out has to be of this chart and come to what its record says, note count, PGREATs
/// and GREATs, or it is not the run the record is of and is no ghost.
fn rejudge(job: &GhostJob, source: &GhostSource) -> Result<GhostPace, String> {
    let replay = Replay::load(&job.replay_dir.join(&source.file)).map_err(|e| format!("its replay could not be read ({e})"))?;
    if !replay.md5.eq_ignore_ascii_case(&job.md5) {
        return Err(format!("its replay is of another chart ({})", replay.md5));
    }
    if replay.judge.clock != rbms_store::REPLAY_EVENT_CLOCK {
        return Err(format!("its replay was judged on the {} clock, which is not played back note for note", replay.judge.clock));
    }
    let mut session = session_of_replay(job.model.clone(), job.bmson.as_deref(), job.mode, replay);
    session.play_out();
    let judge = session.judge();
    if judge.total_notes() != source.total_notes {
        return Err(format!("it was judged again over {} notes, and its record is of {}", judge.total_notes(), source.total_notes));
    }
    let (pgreat, great) = (judge.counts[PGREAT_TIER], judge.counts[GREAT_TIER]);
    if (pgreat, great) != (source.pgreat, source.great) {
        return Err(format!("it was judged again to {pgreat} PGREAT {great} GREAT, and its record says {} PGREAT {} GREAT", source.pgreat, source.great));
    }
    Ok(GhostPace::of_judges(judge.vanished(), judge.total_notes()))
}

#[cfg(test)]
mod tests;
