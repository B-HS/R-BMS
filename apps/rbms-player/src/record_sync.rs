//! Sending every play this machine keeps to the score server until the server holds it.
//!
//! A play is kept whether or not it reached the server: nobody was signed in, the server was down,
//! or it refused the play over a rule it has since changed. A records sync walks the plays the
//! server does not hold yet ([`ScoreDb::sync_candidates`]) and sends each one, oldest first, then
//! the replays of the plays it took. The server keeps one score per player, chart and moment, so a
//! play it already holds comes back as "already on the server" and costs nothing; and it is the
//! server's answer, written per play into `ir_sync`, that decides whether a play is sent again —
//! never the old `ir_submitted` flag, which said only that a play was handed over.
//!
//! A finished course is kept and sent the same way ([`ScoreDb::course_candidates`]), after the
//! plays.
//!
//! A play's trace, what it was note by note ([`crate::play_trace`]), goes up after the courses
//! once the server took the play's score, linked to that score: only to a server that advertises
//! it takes traces ([`rbms_ir::ServerCapabilities::traces`]) and only while AUTO UPLOAD TRACE is on.
//! Until then the trace stays pending, and a later sync sends it.
//!
//! The picking and the sending happen on a worker thread, paced under the server's rate limits. The
//! worker reads the database through a connection of its own that only reads, so picking the plays
//! out of a history of tens of thousands never stalls a frame; the frame thread hands it what it
//! asks of the library, picks up what it reports once per frame ([`AppShared::poll_record_sync`]),
//! and is the only one that writes.
//!
//! A refusal is written down with the builds it was given under, this client's and the server's
//! ([`refusal_key`]). The same play sent again by the same client to the same server would be
//! refused again, so a sync started on its own (at launch, at sign-in) leaves it out until one of
//! the two builds changes; SYNC RECORDS NOW sends every refusal again.
//!
//! A play stored before its submission body was kept is rebuilt from its row
//! ([`rebuilt_submission`]): the row has the judgements, the lamp, the gauge and the shuffle, and
//! not the fast/slow split, the play options, the build or the millisecond the play ended at.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use rbms_ir::mapping::{AUTO_SCRATCH_FLAG, CUSTOM_JUDGE_FLAG};
use rbms_ir::{
    ChartId, CourseSubmission, IrError, PlayTrace, Rejection, SubmitResponse, SubmitVerdict, TRACE_MAX_BYTES, UploadVerdict, submit_verdict, upload_verdict,
};
use rbms_store::scoredb::{
    CourseSyncRow, IrSyncRow, JUDGE_BAD, JUDGE_GOOD, JUDGE_GREAT, JUDGE_MISS, JUDGE_PGREAT, JUDGE_POOR, JUDGE_TIER_COUNT, KeptCourse, PlayLog, ReplayAnswer,
    ReplayState, SKIPPED_OLD_SCRATCH_AUTO_RULE, ScoreDb, SyncCandidate, SyncState, SyncWalk, TraceAnswer, UNSET_MINIMUM, charged_minbp, gauge_token, mode_name,
    random_token,
};
use rbms_store::{REFERENCE_RECORD_RULE_VERSION, SCRATCH_AUTO_ASSIST};

use crate::ir_outcome::{IrLineKind, truncate};
use crate::ir_replay::{to_ir_replay, wire_carries_judge};
use crate::*;

/// Gap between the starts of two score submissions. The server takes 60 a minute from an account;
/// this sends 40, leaving the rest to the runs played while a sync goes on.
pub(crate) const SCORE_SEND_INTERVAL: Duration = Duration::from_millis(1_500);
/// Gap between the starts of two replay uploads. The server takes 10 a minute from an account; this
/// sends 8, leaving the rest to the runs played while a sync goes on.
pub(crate) const REPLAY_SEND_INTERVAL: Duration = Duration::from_millis(7_500);
/// Gap between the starts of two trace uploads. The server is to take 30 a minute from an account;
/// this sends 24, leaving the rest to the runs played while a sync goes on.
pub(crate) const TRACE_SEND_INTERVAL: Duration = Duration::from_millis(2_500);
/// How long a sync waits after a rate limit: the server's window.
pub(crate) const RATE_LIMIT_WAIT: Duration = Duration::from_secs(60);
/// How many times a sync waits out a rate limit on one request. A request still rate limited after
/// that stops the sync, and the play is sent on the next one.
pub(crate) const RATE_LIMIT_WAITS_PER_REQUEST: u32 = 2;
/// How many requests in a row may go unanswered before the server is taken for unreachable and the
/// sync stops.
pub(crate) const TRANSIENT_FAILURES_BEFORE_STOP: u32 = 3;
/// How often a sync that is waiting looks at whether it was cancelled.
pub(crate) const CANCEL_CHECK_INTERVAL: Duration = Duration::from_millis(100);

/// `error_code` of a play not sent because its chart's SHA-256 is unknown: a play carried over from
/// `scores.ron` whose chart is not in the library to be hashed. It is looked for again next sync.
pub(crate) const SKIPPED_NO_CHART_HASH: &str = "NO_CHART_HASH";
const NO_CHART_HASH_REASON: &str = "chart not in the library";

/// The assist tag a rebuilt play states when its row says there was an assist and not which: the
/// server reads any tag but SCRATCH AUTO's as an assist and keeps the play off the ranking.
pub(crate) const UNRECORDED_ASSIST_FLAG: &str = "UNRECORDED_ASSIST";

/// Assist level of a play judged wider than the chart asks, as a row stores it.
const CUSTOM_JUDGE_ASSIST: i32 = rbms_ir::mapping::CUSTOM_JUDGE_ASSIST as i32;

/// What a rebuilt play states of the judge width it was played at: the chart's own. A play judged
/// wider already says so with its assist tag.
const REBUILT_JUDGE_RATE: i32 = 100;

/// EX points a fully judged note is worth, so a play's EX ceiling is its note count times this.
const EX_PER_NOTE: u32 = 2;

/// Milliseconds per second: a row's date is a second, a submission's moment a millisecond.
const MILLIS_PER_SECOND: i64 = 1_000;

/// What a saved replay's file name ends in (`<md5 head>-<epoch ms>.ron`).
const REPLAY_FILE_SUFFIX: &str = ".ron";
const REPLAY_FILE_STAMP_SEPARATOR: char = '-';

/// The client a rebuilt play is sent by: this build, which rebuilt it.
const CLIENT_NAME: &str = concat!("rbms/", env!("CARGO_PKG_VERSION"));

/// `error_code` of a course not sent because the server has no course endpoint. It is sent again
/// on a later sync, which a server that has gained one takes.
pub(crate) const SKIPPED_NO_COURSE_ENDPOINT: &str = "NO_COURSE_ENDPOINT";
const NO_COURSE_ENDPOINT_REASON: &str = "server takes no courses";

/// What separates the client's build from the server's in a [`refusal_key`].
const REFUSAL_KEY_SEPARATOR: char = '|';

/// The builds a refusal is written down under: this client's and the server's. A refusal under the
/// same two would be given again, so a sync started on its own leaves such a play out
/// ([`SyncWalk::refused_under`]). A server that does not say its build is the empty build: a
/// refusal it gives is sent again only by hand or by another client build.
pub(crate) fn refusal_key(client: &str, server: &str) -> String {
    format!("{client}{REFUSAL_KEY_SEPARATOR}{server}")
}

/// The build a server says it runs: its commit, or its name and API generation when it names no
/// commit, and nothing when it does not answer.
pub(crate) fn server_build_of(server: &dyn ScoreServer) -> String {
    match server.version() {
        Ok(info) => info.commit.unwrap_or_else(|| format!("{}@{}", info.server, info.api_version)),
        Err(_) => String::new(),
    }
}

/// How fast a sync sends. The constants above in the game; nothing in the tests.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SyncPacing {
    pub(crate) score_interval: Duration,
    pub(crate) replay_interval: Duration,
    pub(crate) trace_interval: Duration,
    pub(crate) rate_limit_wait: Duration,
    pub(crate) cancel_check: Duration,
}

impl SyncPacing {
    pub(crate) const GAME: SyncPacing = SyncPacing {
        score_interval: SCORE_SEND_INTERVAL,
        replay_interval: REPLAY_SEND_INTERVAL,
        trace_interval: TRACE_SEND_INTERVAL,
        rate_limit_wait: RATE_LIMIT_WAIT,
        cancel_check: CANCEL_CHECK_INTERVAL,
    };
}

/// What the library knows of a chart a play was made on, and its row does not.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ChartFacts {
    pub(crate) path: PathBuf,
    pub(crate) artist: String,
    pub(crate) level: String,
}

/// Everything a worker needs to send: the plays, the courses, the traces and the replays, and how.
pub(crate) struct SyncJob {
    pub(crate) login_id: String,
    pub(crate) scores: Vec<SyncCandidate>,
    pub(crate) courses: Vec<KeptCourse>,
    pub(crate) traces: Vec<SyncCandidate>,
    pub(crate) replays: Vec<SyncCandidate>,
    /// Whether traces go up this sync: AUTO UPLOAD TRACE is on and the server takes them. A play
    /// the server takes during the sync has its trace sent only then.
    pub(crate) upload_traces: bool,
    /// The score database file the traces are read from as they are sent.
    pub(crate) db_path: PathBuf,
    pub(crate) upload_replays: bool,
    pub(crate) replay_dir: PathBuf,
    /// Answered once the save thread has written everything it was handed before the sync began
    /// ([`crate::save_worker::SaveWorker::settled`]); `None` when nothing is waiting to be written.
    pub(crate) saves_settled: Option<Receiver<()>>,
    /// The library's facts of every chart a play to send was made on, keyed by lower-case MD5.
    pub(crate) charts: HashMap<String, ChartFacts>,
    pub(crate) pacing: SyncPacing,
}

/// What the server said of one play's score, as the worker reports it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ScoreOutcome {
    Accepted { score_id: Option<String>, already: bool, replay: ReplayState, trace: ReplayState },
    Rejected(Rejection),
    Retry { detail: String },
    Skipped { code: &'static str, reason: &'static str },
}

/// Why a sync ended before it had sent everything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SyncStop {
    Unauthorized(String),
    Unreachable(String),
    RateLimited,
    Cancelled,
    /// The worker could not read what to send.
    Unreadable(String),
}

/// What the worker reports, in the order it happens.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SyncEvent {
    /// The worker picked what to send ([`spawn_planned_sync`]): the build the server said it runs,
    /// and the charts (lower-case MD5s) whose library facts it waits for.
    Picked {
        server_build: String,
        md5s: Vec<String>,
    },
    Started {
        scores: usize,
        courses: usize,
        traces: usize,
        replays: usize,
    },
    /// A play stored without its chart's SHA-256 has it now.
    Backfilled {
        log_id: i64,
        sha256: String,
    },
    Score {
        log_id: i64,
        outcome: ScoreOutcome,
    },
    /// What the server said of a kept course.
    Course {
        course_id: i64,
        outcome: ScoreOutcome,
    },
    /// The trace stage is about to send this many traces.
    Traces {
        total: usize,
    },
    Trace {
        log_id: i64,
        state: ReplayState,
        trace_id: Option<String>,
        error: Option<String>,
    },
    /// The replay stage is about to send this many replays.
    Replays {
        total: usize,
    },
    Replay {
        log_id: i64,
        state: ReplayState,
        replay_id: Option<String>,
        error: Option<String>,
    },
    Finished {
        stop: Option<SyncStop>,
    },
}

/// Paces requests from the start of one to the start of the next, and waits so that a cancel is
/// seen.
struct Pacer<'a> {
    last_start: Option<Instant>,
    cancel: &'a AtomicBool,
    check: Duration,
}

impl Pacer<'_> {
    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Wait `span`, looking at the cancel flag as it goes. Returns whether the sync was cancelled.
    fn wait(&self, span: Duration) -> bool {
        let until = Instant::now() + span;
        loop {
            if self.cancelled() {
                return true;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            std::thread::sleep(left.min(self.check));
        }
    }

    /// Wait until `interval` has passed since the last request started, then mark this one as
    /// starting. Returns whether the sync was cancelled.
    fn before_request(&mut self, interval: Duration) -> bool {
        let since = self.last_start.map_or(interval, |start| start.elapsed());
        if self.wait(interval.saturating_sub(since)) {
            return true;
        }
        self.last_start = Some(Instant::now());
        false
    }
}

/// What a worker is to pick and send, read from the database by a connection of its own.
pub(crate) struct SyncPlan {
    /// The score database file the worker opens to read.
    pub(crate) db_path: PathBuf,
    /// The server, as `ir_sync` keys it.
    pub(crate) server_key: String,
    pub(crate) login_id: String,
    pub(crate) scope: SyncScope,
    /// Whether the player asked for the sync, which sends every refusal again.
    pub(crate) by_hand: bool,
    /// The play a result screen's submission is still carrying, which writes its own answer.
    pub(crate) left_out: Option<i64>,
    /// The newest play and course when the sync began: one stored after that is sent by whoever
    /// stored it.
    pub(crate) up_to: Option<i64>,
    pub(crate) courses_up_to: Option<i64>,
    /// This client's build ([`refusal_key`]).
    pub(crate) client_build: String,
    /// AUTO UPLOAD TRACE: whether traces go up to a server that takes them.
    pub(crate) upload_traces: bool,
    pub(crate) upload_replays: bool,
    pub(crate) replay_dir: PathBuf,
    /// Answered once the save thread has written everything it was handed before the sync began.
    pub(crate) saves_settled: Option<Receiver<()>>,
    pub(crate) pacing: SyncPacing,
}

/// Pick what `plan` names on a worker thread and send it ([`run_record_sync`]), reporting over the
/// returned channel. The worker asks the server its build and, when traces are to go up, whether it
/// takes them, reads the plays, courses, traces and replays to send, reports them picked
/// ([`SyncEvent::Picked`]) and waits on `charts` for what the library knows of their charts before
/// it sends.
pub(crate) fn spawn_planned_sync(
    server: Arc<dyn ScoreServer>,
    plan: SyncPlan,
    cancel: Arc<AtomicBool>,
    charts: Receiver<HashMap<String, ChartFacts>>,
) -> Receiver<SyncEvent> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut plan = plan;
        let saves_settled = plan.saves_settled.take();
        let stop = match pick(server.as_ref(), &plan, &tx) {
            Ok(Picked { scores, courses, traces, replays, upload_traces }) => match facts_from(&charts, &cancel, plan.pacing.cancel_check) {
                Some(charts) => {
                    let job = SyncJob {
                        login_id: plan.login_id.clone(),
                        scores,
                        courses,
                        traces,
                        replays,
                        upload_traces,
                        db_path: plan.db_path.clone(),
                        upload_replays: plan.upload_replays,
                        replay_dir: plan.replay_dir.clone(),
                        saves_settled,
                        charts,
                        pacing: plan.pacing,
                    };
                    run_record_sync(server.as_ref(), job, &cancel, &tx)
                }
                None => Some(SyncStop::Cancelled),
            },
            Err(e) => Some(SyncStop::Unreadable(e.to_string())),
        };
        let _ = tx.send(SyncEvent::Finished { stop });
    });
    rx
}

/// What a worker picked to send, and whether traces go up this sync at all.
struct Picked {
    scores: Vec<SyncCandidate>,
    courses: Vec<KeptCourse>,
    traces: Vec<SyncCandidate>,
    replays: Vec<SyncCandidate>,
    upload_traces: bool,
}

/// Whether `server` says it takes traces. A server that does not answer, or that answers without
/// the flag, takes none: an older server has nowhere to put one.
fn takes_traces(server: &dyn ScoreServer) -> bool {
    server.health().is_ok_and(|info| info.capabilities.traces)
}

/// Ask the server its build, read what `plan` sends, and report it picked.
///
/// The traces are those of the plays the server already holds; a sync narrowed to some plays sends
/// theirs alone, so the trace of a play the result screen sent goes up by a sync of that play. The
/// server is asked whether it takes traces only when there is one to send: one of a play it holds,
/// or one of a play this sync is to send.
fn pick(server: &dyn ScoreServer, plan: &SyncPlan, tx: &Sender<SyncEvent>) -> Result<Picked, rbms_store::scoredb::ScoreDbError> {
    let server_build = server_build_of(server);
    let refused_under = refusal_key(&plan.client_build, &server_build);
    let db = ScoreDb::open_read_only(&plan.db_path)?;
    let only = match &plan.scope {
        SyncScope::All => None,
        SyncScope::Logs(ids) => Some(ids.as_slice()),
    };
    let walk = SyncWalk {
        server: &plan.server_key,
        login_id: &plan.login_id,
        only,
        up_to: plan.up_to,
        courses_up_to: plan.courses_up_to,
        refused_under: (!plan.by_hand).then_some(refused_under.as_str()),
    };
    let mut scores = db.sync_candidates(&walk)?;
    scores.retain(|candidate| Some(candidate.log.id) != plan.left_out);
    let (courses, replays) = match plan.scope {
        SyncScope::All => (db.course_candidates(&walk)?, if plan.upload_replays { db.replay_candidates(&walk)? } else { Vec::new() }),
        SyncScope::Logs(_) => (Vec::new(), Vec::new()),
    };
    let mut traces = if plan.upload_traces { db.trace_candidates(&walk)? } else { Vec::new() };
    traces.retain(|candidate| Some(candidate.log.id) != plan.left_out);
    let traces_to_send = !traces.is_empty() || scores.iter().any(|candidate| candidate.has_trace);
    let upload_traces = plan.upload_traces && traces_to_send && takes_traces(server);
    if !upload_traces {
        traces.clear();
    }
    let md5s: HashSet<String> = scores.iter().chain(&replays).map(|candidate| candidate.log.md5.to_ascii_lowercase()).collect();
    let _ = tx.send(SyncEvent::Picked { server_build, md5s: md5s.into_iter().collect() });
    Ok(Picked { scores, courses, traces, replays, upload_traces })
}

/// Wait for the library's facts the frame thread hands back, looking at the cancel flag as it
/// waits. `None` when the sync was cancelled; a frame thread that went away hands nothing.
fn facts_from(charts: &Receiver<HashMap<String, ChartFacts>>, cancel: &AtomicBool, check: Duration) -> Option<HashMap<String, ChartFacts>> {
    loop {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        match charts.recv_timeout(check) {
            Ok(facts) => return Some(facts),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Some(HashMap::new()),
        }
    }
}

/// What a course submission's answer means for the course, read as a play's is, but for a server
/// that has no course endpoint: the course is not sent, and goes again on a later sync.
pub(crate) fn course_outcome(result: &Result<SubmitResponse, IrError>) -> ScoreOutcome {
    if matches!(result, Err(IrError::Unsupported)) {
        return ScoreOutcome::Skipped { code: SKIPPED_NO_COURSE_ENDPOINT, reason: NO_COURSE_ENDPOINT_REASON };
    }
    match submit_verdict(result) {
        SubmitVerdict::Accepted { score_id, already, .. } => ScoreOutcome::Accepted { score_id, already, replay: ReplayState::None, trace: ReplayState::None },
        SubmitVerdict::Rejected(rejection) => ScoreOutcome::Rejected(rejection),
        SubmitVerdict::RateLimited => ScoreOutcome::Retry { detail: RATE_LIMITED_DETAIL.to_string() },
        SubmitVerdict::Transient { detail } | SubmitVerdict::StopSync { detail } => ScoreOutcome::Retry { detail },
    }
}

/// Send the kept courses, by the rules plays are sent by. Returns why the sync stopped, if it did.
fn send_courses(
    server: &dyn ScoreServer,
    courses: Vec<KeptCourse>,
    login_id: &str,
    pacer: &mut Pacer<'_>,
    pacing: &SyncPacing,
    transient_in_a_row: &mut u32,
    tx: &Sender<SyncEvent>,
) -> Option<SyncStop> {
    for course in courses {
        if pacer.cancelled() {
            return Some(SyncStop::Cancelled);
        }
        let course_id = course.id;
        let report = |outcome: ScoreOutcome| {
            let _ = tx.send(SyncEvent::Course { course_id, outcome });
        };
        let Some(mut submission) = CourseSubmission::from_json(&course.body) else {
            report(ScoreOutcome::Skipped { code: SKIPPED_UNREADABLE_COURSE, reason: UNREADABLE_COURSE_REASON });
            continue;
        };
        submission.player.id = login_id.to_string();
        submission.clamp_to_server_bounds();
        let mut waits = 0;
        let result = loop {
            if pacer.before_request(pacing.score_interval) {
                return Some(SyncStop::Cancelled);
            }
            let result = server.submit_course(&submission);
            if matches!(result, Err(IrError::RateLimited(_))) && waits < RATE_LIMIT_WAITS_PER_REQUEST {
                waits += 1;
                if pacer.wait(pacing.rate_limit_wait) {
                    return Some(SyncStop::Cancelled);
                }
                continue;
            }
            break result;
        };
        let outcome = course_outcome(&result);
        match submit_verdict(&result) {
            SubmitVerdict::RateLimited => {
                report(outcome);
                return Some(SyncStop::RateLimited);
            }
            SubmitVerdict::StopSync { detail } if !matches!(result, Err(IrError::Unsupported)) => {
                report(outcome);
                return Some(SyncStop::Unauthorized(detail));
            }
            SubmitVerdict::Transient { detail } => {
                *transient_in_a_row += 1;
                report(outcome);
                if *transient_in_a_row >= TRANSIENT_FAILURES_BEFORE_STOP {
                    return Some(SyncStop::Unreachable(detail));
                }
            }
            _ => {
                *transient_in_a_row = 0;
                report(outcome);
            }
        }
    }
    None
}

/// `error_code` of a kept course whose body no longer reads as a course submission.
pub(crate) const SKIPPED_UNREADABLE_COURSE: &str = "UNREADABLE_COURSE";
const UNREADABLE_COURSE_REASON: &str = "kept course body unreadable";

/// Send `job`, reporting over `tx`: the plays, then the courses, then the traces, then the replays.
/// Setting `cancel` ends the sync at the next play or within [`SyncPacing::cancel_check`] of a wait;
/// a request already on the wire is let finish. Returns why the sync stopped, if it did.
fn run_record_sync(server: &dyn ScoreServer, job: SyncJob, cancel: &AtomicBool, tx: &Sender<SyncEvent>) -> Option<SyncStop> {
    let SyncJob { login_id, scores, courses, mut traces, mut replays, upload_traces, db_path, upload_replays, replay_dir, saves_settled, charts, pacing } = job;
    let _ = tx.send(SyncEvent::Started { scores: scores.len(), courses: courses.len(), traces: traces.len(), replays: replays.len() });
    let mut pacer = Pacer { last_start: None, cancel, check: pacing.cancel_check };
    let mut transient_in_a_row = 0;
    for candidate in scores {
        if pacer.cancelled() {
            return Some(SyncStop::Cancelled);
        }
        let log_id = candidate.log.id;
        let facts = charts.get(&candidate.log.md5.to_ascii_lowercase());
        let mut submission = match submission_of(&candidate, facts) {
            Ok((submission, backfilled)) => {
                if let Some(sha256) = backfilled {
                    let _ = tx.send(SyncEvent::Backfilled { log_id, sha256 });
                }
                submission
            }
            Err(skip) => {
                let _ = tx.send(SyncEvent::Score { log_id, outcome: skip });
                continue;
            }
        };
        submission.player.id.clone_from(&login_id);
        submission.clamp_to_server_bounds();
        let mut waits = 0;
        let verdict = loop {
            if pacer.before_request(pacing.score_interval) {
                return Some(SyncStop::Cancelled);
            }
            match submit_verdict(&server.submit_score(&submission)) {
                SubmitVerdict::RateLimited if waits < RATE_LIMIT_WAITS_PER_REQUEST => {
                    waits += 1;
                    if pacer.wait(pacing.rate_limit_wait) {
                        return Some(SyncStop::Cancelled);
                    }
                }
                verdict => break verdict,
            }
        };
        let outcome = match verdict {
            SubmitVerdict::Accepted { score_id, already, replay_warranted } => {
                transient_in_a_row = 0;
                let replay = match (&candidate.log.replay_file, replay_warranted) {
                    (None, _) => ReplayState::None,
                    (Some(_), false) => ReplayState::Skipped,
                    (Some(_), true) => ReplayState::Pending,
                };
                let trace = if candidate.has_trace { ReplayState::Pending } else { ReplayState::None };
                if trace == ReplayState::Pending && upload_traces {
                    let accepted =
                        SyncCandidate { score_id: score_id.clone(), state: Some(SyncState::Accepted), trace_state: Some(trace), ..candidate.clone() };
                    traces.push(accepted);
                }
                if replay == ReplayState::Pending {
                    replays.push(SyncCandidate { score_id: score_id.clone(), state: Some(SyncState::Accepted), replay_state: Some(replay), ..candidate });
                }
                ScoreOutcome::Accepted { score_id, already, replay, trace }
            }
            SubmitVerdict::Rejected(rejection) => {
                transient_in_a_row = 0;
                ScoreOutcome::Rejected(rejection)
            }
            SubmitVerdict::RateLimited => {
                let _ = tx.send(SyncEvent::Score { log_id, outcome: ScoreOutcome::Retry { detail: RATE_LIMITED_DETAIL.to_string() } });
                return Some(SyncStop::RateLimited);
            }
            SubmitVerdict::Transient { detail } => {
                transient_in_a_row += 1;
                let _ = tx.send(SyncEvent::Score { log_id, outcome: ScoreOutcome::Retry { detail: detail.clone() } });
                if transient_in_a_row >= TRANSIENT_FAILURES_BEFORE_STOP {
                    return Some(SyncStop::Unreachable(detail));
                }
                continue;
            }
            SubmitVerdict::StopSync { detail } => {
                let _ = tx.send(SyncEvent::Score { log_id, outcome: ScoreOutcome::Retry { detail: detail.clone() } });
                return Some(SyncStop::Unauthorized(detail));
            }
        };
        let _ = tx.send(SyncEvent::Score { log_id, outcome });
    }
    if let Some(stop) = send_courses(server, courses, &login_id, &mut pacer, &pacing, &mut transient_in_a_row, tx) {
        return Some(stop);
    }
    if upload_traces
        && !traces.is_empty()
        && let Some(stop) = send_traces(server, traces, &db_path, &mut pacer, &pacing, tx)
    {
        return Some(stop);
    }
    if !upload_replays {
        return None;
    }
    if !replays.is_empty()
        && let Some(settled) = &saves_settled
    {
        match saves_landed(settled, &pacer) {
            SavesLanded::Yes => {}
            SavesLanded::NotYet => return None,
            SavesLanded::Cancelled => return Some(SyncStop::Cancelled),
        }
    }
    let _ = tx.send(SyncEvent::Replays { total: replays.len() });
    transient_in_a_row = 0;
    for candidate in replays {
        if pacer.cancelled() {
            return Some(SyncStop::Cancelled);
        }
        let log_id = candidate.log.id;
        let report = |state: ReplayState, replay_id: Option<String>, error: Option<String>| {
            let _ = tx.send(SyncEvent::Replay { log_id, state, replay_id, error });
        };
        let Some(prepared) = replay_upload_of(&candidate, &replay_dir) else {
            report(ReplayState::Missing, None, None);
            continue;
        };
        let Some((chart, data)) = prepared else {
            report(ReplayState::Skipped, None, None);
            continue;
        };
        let mut waits = 0;
        let verdict = loop {
            if pacer.before_request(pacing.replay_interval) {
                return Some(SyncStop::Cancelled);
            }
            match upload_verdict(&server.upload_replay(&chart, &data)) {
                UploadVerdict::RateLimited if waits < RATE_LIMIT_WAITS_PER_REQUEST => {
                    waits += 1;
                    if pacer.wait(pacing.rate_limit_wait) {
                        return Some(SyncStop::Cancelled);
                    }
                }
                verdict => break verdict,
            }
        };
        match verdict {
            UploadVerdict::Uploaded { replay_id } => {
                transient_in_a_row = 0;
                report(ReplayState::Uploaded, Some(replay_id), None);
            }
            UploadVerdict::Rejected(rejection) => {
                transient_in_a_row = 0;
                report(ReplayState::Rejected, None, Some(rejection.detail));
            }
            UploadVerdict::RateLimited => {
                report(ReplayState::Failed, None, Some(RATE_LIMITED_DETAIL.to_string()));
                return Some(SyncStop::RateLimited);
            }
            UploadVerdict::Transient { detail } => {
                transient_in_a_row += 1;
                report(ReplayState::Failed, None, Some(detail.clone()));
                if transient_in_a_row >= TRANSIENT_FAILURES_BEFORE_STOP {
                    return Some(SyncStop::Unreachable(detail));
                }
            }
            UploadVerdict::StopSync { detail } => {
                report(ReplayState::Failed, None, Some(detail.clone()));
                return Some(SyncStop::Unauthorized(detail));
            }
        }
    }
    None
}

/// `trace_error` of a trace never sent because it is larger than the server takes.
pub(crate) const TRACE_TOO_LARGE: &str = "trace larger than the server takes";
/// `trace_error` of a trace never sent because the server took the score without naming it.
pub(crate) const TRACE_WITHOUT_A_SCORE: &str = "server named no score to link the trace to";
/// `trace_error` of a kept trace that no longer reads as one.
pub(crate) const TRACE_UNREADABLE: &str = "kept trace unreadable";
/// What `trace_error` starts with when the score database could not be read for the trace just
/// then; the reason follows.
pub(crate) const TRACE_NOT_READ: &str = "kept trace not read";

/// The trace of a play, ready to upload, or the state and the reason it is not sent: failed when the
/// database could not be read just then (busy, or a read that failed), which a later sync tries
/// again; missing when its row is gone; skipped when it cannot be read, has no score to be linked
/// to, or is larger than [`TRACE_MAX_BYTES`].
fn trace_upload_of(db: &ScoreDb, candidate: &SyncCandidate) -> Result<PlayTrace, (ReplayState, Option<String>)> {
    let kept = db.trace_body(candidate.log.id).map_err(|e| (ReplayState::Failed, Some(format!("{TRACE_NOT_READ}: {e}"))))?;
    let body = kept.ok_or((ReplayState::Missing, None))?;
    let mut trace = PlayTrace::from_json(&body).ok_or_else(|| (ReplayState::Skipped, Some(TRACE_UNREADABLE.to_string())))?;
    trace.score_id = Some(candidate.score_id.clone().ok_or_else(|| (ReplayState::Skipped, Some(TRACE_WITHOUT_A_SCORE.to_string())))?);
    let size = trace.to_json().map_or(usize::MAX, |text| text.len());
    if size > TRACE_MAX_BYTES {
        return Err((ReplayState::Skipped, Some(TRACE_TOO_LARGE.to_string())));
    }
    Ok(trace)
}

/// Send the traces of plays the server holds, each linked to its score, by the rules replays are
/// sent by. Returns why the sync stopped, if it did.
fn send_traces(
    server: &dyn ScoreServer,
    traces: Vec<SyncCandidate>,
    db_path: &Path,
    pacer: &mut Pacer<'_>,
    pacing: &SyncPacing,
    tx: &Sender<SyncEvent>,
) -> Option<SyncStop> {
    let db = match ScoreDb::open_read_only(db_path) {
        Ok(db) => db,
        Err(e) => return Some(SyncStop::Unreadable(e.to_string())),
    };
    let _ = tx.send(SyncEvent::Traces { total: traces.len() });
    let mut transient_in_a_row = 0;
    for candidate in traces {
        if pacer.cancelled() {
            return Some(SyncStop::Cancelled);
        }
        let log_id = candidate.log.id;
        let report = |state: ReplayState, trace_id: Option<String>, error: Option<String>| {
            let _ = tx.send(SyncEvent::Trace { log_id, state, trace_id, error });
        };
        let trace = match trace_upload_of(&db, &candidate) {
            Ok(trace) => trace,
            Err((state, reason)) => {
                report(state, None, reason);
                continue;
            }
        };
        let mut waits = 0;
        let verdict = loop {
            if pacer.before_request(pacing.trace_interval) {
                return Some(SyncStop::Cancelled);
            }
            match upload_verdict(&server.upload_trace(&trace.chart, &trace)) {
                UploadVerdict::RateLimited if waits < RATE_LIMIT_WAITS_PER_REQUEST => {
                    waits += 1;
                    if pacer.wait(pacing.rate_limit_wait) {
                        return Some(SyncStop::Cancelled);
                    }
                }
                verdict => break verdict,
            }
        };
        match verdict {
            UploadVerdict::Uploaded { replay_id } => {
                transient_in_a_row = 0;
                report(ReplayState::Uploaded, Some(replay_id), None);
            }
            UploadVerdict::Rejected(rejection) => {
                transient_in_a_row = 0;
                report(ReplayState::Rejected, None, Some(rejection.detail));
            }
            UploadVerdict::RateLimited => {
                report(ReplayState::Failed, None, Some(RATE_LIMITED_DETAIL.to_string()));
                return Some(SyncStop::RateLimited);
            }
            UploadVerdict::Transient { detail } => {
                transient_in_a_row += 1;
                report(ReplayState::Failed, None, Some(detail.clone()));
                if transient_in_a_row >= TRANSIENT_FAILURES_BEFORE_STOP {
                    return Some(SyncStop::Unreachable(detail));
                }
            }
            UploadVerdict::StopSync { detail } => {
                report(ReplayState::Failed, None, Some(detail.clone()));
                return Some(SyncStop::Unauthorized(detail));
            }
        }
    }
    None
}

/// How long the replay stage of a sync waits for the save thread to write the replays it was handed
/// before the sync began. Writing one takes milliseconds; a thread still busy after this is held up
/// by something the sync cannot wait out, and the replays are left for a later sync.
const REPLAY_SAVES_WAIT: Duration = Duration::from_secs(10);

/// Whether the writes handed to the save thread before a sync began have landed.
enum SavesLanded {
    Yes,
    /// Not within [`REPLAY_SAVES_WAIT`].
    NotYet,
    Cancelled,
}

/// Wait for the save thread to answer `settled`, looking at the cancel flag as it goes.
///
/// A run left with Escape or started again is sent by a sync of its own as soon as it is kept, while
/// its replay is still being written on the save thread. Read before then, the file is not there, and
/// a replay that is not there is one the sync marks missing for good. So the replay stage waits for
/// the writes handed over before the sync began; when they do not land in time, no replay is read,
/// and every one picked stays pending for the next sync. A save thread that is gone has nothing more
/// to write.
fn saves_landed(settled: &Receiver<()>, pacer: &Pacer<'_>) -> SavesLanded {
    let until = Instant::now() + REPLAY_SAVES_WAIT;
    loop {
        if pacer.cancelled() {
            return SavesLanded::Cancelled;
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return SavesLanded::NotYet;
        }
        match settled.recv_timeout(left.min(pacer.check)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return SavesLanded::Yes,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

/// What a play or replay left unsent by a rate limit is said to have hit.
pub(crate) const RATE_LIMITED_DETAIL: &str = "rate limited (429)";

/// The submission to send for a play, and the SHA-256 its row was missing when one was found: the
/// body kept when the play ended, or one rebuilt from its row. A play whose chart cannot be named
/// by SHA-256 is skipped.
fn submission_of(candidate: &SyncCandidate, facts: Option<&ChartFacts>) -> Result<(ScoreSubmission, Option<String>), ScoreOutcome> {
    if let Some(kept) = candidate.body.as_deref().and_then(ScoreSubmission::from_json) {
        return Ok((kept, None));
    }
    let mut submission = rebuilt_submission(&candidate.log, facts);
    if !candidate.log.sha256.is_empty() {
        return Ok((submission, None));
    }
    let sha256 = facts
        .and_then(|facts| chart_sha256(&facts.path, &candidate.log.md5))
        .ok_or(ScoreOutcome::Skipped { code: SKIPPED_NO_CHART_HASH, reason: NO_CHART_HASH_REASON })?;
    submission.chart.id.sha256.clone_from(&sha256);
    Ok((submission, Some(sha256)))
}

/// The SHA-256 of the chart at `path`, when it is the chart whose MD5 is `md5`.
fn chart_sha256(path: &Path, md5: &str) -> Option<String> {
    let source = rbms_parser::parse(&std::fs::read(path).ok()?);
    source.md5.eq_ignore_ascii_case(md5).then_some(source.sha256)
}

/// The replay of a play, ready to upload: `None` when its file is gone or unreadable, `Some(None)`
/// when the upload cannot state the widths it was judged by ([`wire_carries_judge`]).
fn replay_upload_of(candidate: &SyncCandidate, replay_dir: &Path) -> Option<Option<(ChartId, ReplayData)>> {
    let file = candidate.log.replay_file.as_deref()?;
    let replay = Replay::load(&replay_dir.join(file)).ok()?;
    if !wire_carries_judge(&replay.judge) {
        return Some(None);
    }
    let kept = candidate.body.as_deref().and_then(ScoreSubmission::from_json);
    let (constant, lntype, build) = match &kept {
        Some(sub) => (sub.options.constant, sub.options.lntype, sub.client_build_sha256.clone()),
        None => (false, rebuilt_lntype(&candidate.log.ln_mode), None),
    };
    let chart = ChartId { md5: candidate.log.md5.clone(), sha256: kept.map_or_else(|| candidate.log.sha256.clone(), |sub| sub.chart.id.sha256) };
    let mut data = to_ir_replay(&replay, &chart, constant, lntype, build);
    data.score_id.clone_from(&candidate.score_id);
    Some(Some((chart, data)))
}

/// The LN type a rebuilt play states: the flavour its row was compared within, and a plain long
/// note for a chart that states its own. A chart that declares charge notes is not read here, so
/// such a play is sent as plain long notes.
fn rebuilt_lntype(ln_mode_key: &str) -> i32 {
    if ln_mode_key == SCORE_LN_MODE_FROM_CHART {
        return i32::from(rbms_judge::ln::LnMode::LongNote.id());
    }
    i32::from(rbms_config::ln_mode_from_token(ln_mode_key).id())
}

/// The millisecond a play ended at: the one its replay is named by when the replay is of the same
/// second, and the start of the second its row holds otherwise. The server takes a play sent again
/// within the same second for the one it holds.
pub(crate) fn rebuilt_played_at(log: &PlayLog) -> i64 {
    let stamp = log
        .replay_file
        .as_deref()
        .and_then(|file| file.strip_suffix(REPLAY_FILE_SUFFIX))
        .and_then(|stem| stem.rsplit_once(REPLAY_FILE_STAMP_SEPARATOR))
        .and_then(|(_, ms)| ms.parse::<i64>().ok());
    stamp.filter(|ms| ms.div_euclid(MILLIS_PER_SECOND) == log.date).unwrap_or_else(|| log.date.saturating_mul(MILLIS_PER_SECOND))
}

/// A play's submission rebuilt from its row and from what the library knows of its chart.
///
/// What the row never kept is sent as nothing: no fast/slow split, no play options beyond the
/// gauge, the shuffle and the assists, no build, no judge algorithm and no skin. The body says it was
/// rebuilt ([`ScoreSubmission::mark_reconstructed`]).
pub(crate) fn rebuilt_submission(log: &PlayLog, facts: Option<&ChartFacts>) -> ScoreSubmission {
    let mut counts = [0; JUDGE_TIER_COUNT];
    for (tier, count) in counts.iter_mut().enumerate() {
        *count = log.judge.tier(tier);
    }
    let mode = Mode::ALL.iter().find(|mode| mode.name == mode_name(log.mode));
    let (scratch_auto, assist) = match log.assist {
        0 => (false, Vec::new()),
        SCRATCH_AUTO_ASSIST => (true, vec![AUTO_SCRATCH_FLAG.to_string()]),
        CUSTOM_JUDGE_ASSIST => (false, vec![CUSTOM_JUDGE_FLAG.to_string()]),
        _ => (false, vec![UNRECORDED_ASSIST_FLAG.to_string()]),
    };
    let notes_taken: u32 = counts[JUDGE_PGREAT..=JUDGE_POOR].iter().sum();
    let (artist, level) = facts.map_or(("", ""), |facts| (facts.artist.as_str(), facts.level.as_str()));
    let mut submission = ScoreSubmission {
        api_version: API_VERSION,
        chart: rbms_ir::SubmittedChart::described(ChartId { md5: log.md5.clone(), sha256: log.sha256.clone() }, &log.title, artist, level),
        player: PlayerId { id: String::new() },
        mode: mode_name(log.mode).to_string(),
        clear: ir_clear(clear_type_from_id(log.clear)),
        ex_score: log.judge.ex_score(),
        max_ex_score: log.notes * EX_PER_NOTE,
        judge: JudgeBreakdown {
            pgreat: counts[JUDGE_PGREAT],
            great: counts[JUDGE_GREAT],
            good: counts[JUDGE_GOOD],
            bad: counts[JUDGE_BAD],
            poor: counts[JUDGE_POOR],
            miss: counts[JUDGE_MISS],
            fast: 0,
            slow: 0,
            combobreak: mode.map_or(log.minbp, |mode| combo_breaks(mode, counts)),
            epg: log.judge.early[JUDGE_PGREAT],
            lpg: log.judge.late[JUDGE_PGREAT],
            egr: log.judge.early[JUDGE_GREAT],
            lgr: log.judge.late[JUDGE_GREAT],
            egd: log.judge.early[JUDGE_GOOD],
            lgd: log.judge.late[JUDGE_GOOD],
            ebd: log.judge.early[JUDGE_BAD],
            lbd: log.judge.late[JUDGE_BAD],
            epr: log.judge.early[JUDGE_POOR],
            lpr: log.judge.late[JUDGE_POOR],
            ems: log.judge.early[JUDGE_MISS],
            lms: log.judge.late[JUDGE_MISS],
            avgjudge: rebuilt_avgjudge(log),
            empty_poor: counts[JUDGE_MISS],
        },
        max_combo: log.combo,
        total_notes: log.notes,
        passnotes: notes_taken.min(log.notes),
        minbp: u32::try_from(charged_minbp(log)).unwrap_or(u32::MAX),
        gauge_value: log.gauge_value,
        options: PlayOptions {
            gauge: usize::try_from(log.gauge)
                .ok()
                .and_then(rbms_judge::gauge::GaugeIndex::from_index)
                .map_or_else(|| ir_gauge(rbms_config::gauge_from_name(gauge_token(log.gauge))), rbms_ir::mapping::ir_gauge_of_index),
            random: ir_random(NoteOption::from_str(random_token(log.random))),
            random_p2: None,
            scratch_auto,
            lntype: rebuilt_lntype(&log.ln_mode),
            input_device: String::new(),
            assist,
            option: 0,
            judge_rate: REBUILT_JUDGE_RATE,
            offset_ms: 0,
            constant: false,
            hispeed: 0.0,
            lift: 0.0,
            lane_cover: 0.0,
            total_override: 0.0,
            autoplay: false,
            auto_offset: false,
            scratch_left: false,
            green_number: 0.0,
        },
        played_at: rebuilt_played_at(log),
        client: CLIENT_NAME.to_string(),
        replay_id: None,
        seed: u64::try_from(log.seed).unwrap_or(0),
        judge_algorithm: String::new(),
        rule: String::new(),
        skin: String::new(),
        client_build_sha256: None,
        client_platform: None,
        extra: Default::default(),
    };
    submission.mark_reconstructed();
    submission.state_rule_version(log.rule_version);
    submission
}

/// The `avgjudge` a rebuilt play states: the row's own when the row holds the reference's
/// (`REFERENCE_RECORD_RULE_VERSION` on), and nothing for an older row or one that never held any.
/// An older row holds the size of its hits' mean signed offset, which says nothing of the mean note
/// distance the field now carries, and the body names the row's rule version so a server can tell.
fn rebuilt_avgjudge(log: &PlayLog) -> i64 {
    if log.rule_version < REFERENCE_RECORD_RULE_VERSION || log.avgjudge == UNSET_MINIMUM {
        return 0;
    }
    log.avgjudge
}

/// Which plays a sync sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SyncScope {
    /// Every play the server lacks, and every replay still to go up.
    All,
    /// These plays alone.
    Logs(Vec<i64>),
}

/// What a finished or running sync has done so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SyncTally {
    pub(crate) sent: usize,
    pub(crate) already: usize,
    pub(crate) rejected: usize,
    pub(crate) failed: usize,
    pub(crate) skipped: usize,
    pub(crate) replays_uploaded: usize,
    pub(crate) replays_failed: usize,
    pub(crate) traces_uploaded: usize,
    pub(crate) traces_failed: usize,
    pub(crate) courses_sent: usize,
    pub(crate) courses_rejected: usize,
    pub(crate) courses_failed: usize,
}

impl SyncTally {
    fn count_score(&mut self, outcome: &ScoreOutcome) {
        match outcome {
            ScoreOutcome::Accepted { already: true, .. } => self.already += 1,
            ScoreOutcome::Accepted { .. } => self.sent += 1,
            ScoreOutcome::Rejected(_) => self.rejected += 1,
            ScoreOutcome::Retry { .. } => self.failed += 1,
            ScoreOutcome::Skipped { .. } => self.skipped += 1,
        }
    }

    fn count_course(&mut self, outcome: &ScoreOutcome) {
        match outcome {
            ScoreOutcome::Accepted { .. } => self.courses_sent += 1,
            ScoreOutcome::Rejected(_) => self.courses_rejected += 1,
            ScoreOutcome::Retry { .. } => self.courses_failed += 1,
            ScoreOutcome::Skipped { .. } => {}
        }
    }

    fn count_replay(&mut self, state: ReplayState) {
        match state {
            ReplayState::Uploaded => self.replays_uploaded += 1,
            ReplayState::Failed | ReplayState::Rejected | ReplayState::Missing => self.replays_failed += 1,
            ReplayState::None | ReplayState::Pending | ReplayState::Skipped => {}
        }
    }

    fn count_trace(&mut self, state: ReplayState) {
        match state {
            ReplayState::Uploaded => self.traces_uploaded += 1,
            ReplayState::Failed | ReplayState::Rejected | ReplayState::Missing => self.traces_failed += 1,
            ReplayState::None | ReplayState::Pending | ReplayState::Skipped => {}
        }
    }

    fn scores_done(&self) -> usize {
        self.sent + self.already + self.rejected + self.failed + self.skipped
    }

    fn is_troubled(&self) -> bool {
        self.rejected + self.failed + self.replays_failed + self.traces_failed + self.courses_rejected + self.courses_failed > 0
    }

    /// The status line of a sync that ran to its end: what happened to the plays and to the
    /// replays, leaving out what did not happen at all.
    pub(crate) fn summary(&self) -> String {
        let part = |count: usize, label: &str| (count > 0).then(|| format!("{count} {label}"));
        let records: Vec<String> = [
            part(self.sent, "sent"),
            part(self.already, "already on server"),
            part(self.rejected, "rejected"),
            part(self.failed, "failed"),
            part(self.skipped, "skipped"),
        ]
        .into_iter()
        .flatten()
        .collect();
        let replays: Vec<String> = [part(self.replays_uploaded, "uploaded"), part(self.replays_failed, "failed")].into_iter().flatten().collect();
        let traces: Vec<String> = [part(self.traces_uploaded, "uploaded"), part(self.traces_failed, "failed")].into_iter().flatten().collect();
        let courses: Vec<String> =
            [part(self.courses_sent, "sent"), part(self.courses_rejected, "rejected"), part(self.courses_failed, "failed")].into_iter().flatten().collect();
        let records = if records.is_empty() { NOTHING_SENT.to_string() } else { records.join(", ") };
        let mut line = format!("records: {records}");
        if !courses.is_empty() {
            line.push_str(&format!("; courses: {}", courses.join(", ")));
        }
        if !traces.is_empty() {
            line.push_str(&format!("; traces: {}", traces.join(", ")));
        }
        if !replays.is_empty() {
            line.push_str(&format!("; replays: {}", replays.join(", ")));
        }
        line
    }
}

/// What a sync that sent no play at all says of the plays.
const NOTHING_SENT: &str = "nothing sent";

/// What SYNC RECORDS NOW says when the walk found nothing the server lacks.
pub(crate) const NOTHING_TO_SEND: &str = "records: nothing to send";

/// What SYNC RECORDS NOW says while a token is held and the server has not yet named its owner.
pub(crate) const ACCOUNT_NOT_CONFIRMED: &str = "account not confirmed yet";

/// The status line of a sync that stopped before its end.
pub(crate) fn stop_line(stop: &SyncStop) -> String {
    match stop {
        SyncStop::Unauthorized(detail) => format!("records sync stopped: {}", truncate(detail)),
        SyncStop::Unreachable(_) => "records sync stopped: server unreachable".to_string(),
        SyncStop::RateLimited => "records sync stopped: rate limited".to_string(),
        SyncStop::Cancelled => "records sync cancelled".to_string(),
        SyncStop::Unreadable(detail) => format!("records sync not started: {}", truncate(detail)),
    }
}

/// A sync in flight, as the frame thread follows it.
pub(crate) struct RecordSyncRun {
    rx: Receiver<SyncEvent>,
    cancel: Arc<AtomicBool>,
    /// Where the library's facts of the picked charts are handed back to the worker; taken once
    /// they are.
    charts: Option<Sender<HashMap<String, ChartFacts>>>,
    server_key: String,
    login_id: String,
    /// The builds a refusal this sync is given is written down under, once the server said its own.
    refused_under: String,
    /// Whether the player asked for this sync.
    by_hand: bool,
    /// Whether the sync is narrowed to named plays ([`SyncScope::Logs`]).
    named: bool,
    /// A sync started on its own that found nothing to send, or a sync of named plays started on its
    /// own that has only their traces to send, which says nothing but why it stopped.
    quiet: bool,
    scores_total: usize,
    traces_total: Option<usize>,
    traces_done: usize,
    replays_total: Option<usize>,
    replays_done: usize,
    tally: SyncTally,
}

impl RecordSyncRun {
    /// The value of the SYNC RECORDS NOW row while this sync runs.
    pub(crate) fn progress_value(&self) -> String {
        match (self.replays_total, self.traces_total) {
            (Some(total), _) => format!("REPLAYS {}/{total}", self.replays_done),
            (None, Some(total)) => format!("TRACES {}/{total}", self.traces_done),
            (None, None) => format!("SENDING {}/{}", self.tally.scores_done(), self.scores_total),
        }
    }

    fn progress_line(&self) -> String {
        match (self.replays_total, self.traces_total) {
            (Some(total), _) => format!("syncing replays {}/{total}", self.replays_done),
            (None, Some(total)) => format!("syncing traces {}/{total}", self.traces_done),
            (None, None) => format!("syncing records {}/{}", self.tally.scores_done(), self.scores_total),
        }
    }
}

/// The key a server's states are kept under: its URL as the player set it, with surrounding blanks
/// and a trailing `/` cut, so the same server spelt twice is one.
pub(crate) fn server_key(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// The library's facts of the charts `md5s` name, keyed by lower-case MD5: one index lookup per
/// chart, whatever the size of the library.
fn chart_facts(library: &Library, md5s: &[String]) -> HashMap<String, ChartFacts> {
    md5s.iter()
        .filter_map(|md5| {
            let song = library.indices_for_md5(md5).first().and_then(|&index| library.songs().get(index))?;
            Some((md5.to_ascii_lowercase(), ChartFacts { path: song.path.clone(), artist: song.artist.clone(), level: song.level.clone() }))
        })
        .collect()
}

/// Epoch milliseconds now.
fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

/// One `ir_sync` row of what a server said of a play's score; a refusal is written down under
/// `refused_under` ([`refusal_key`]).
pub(crate) fn sync_row(log_id: i64, server: &str, login_id: &str, outcome: &ScoreOutcome, refused_under: &str) -> IrSyncRow {
    let base = |state| IrSyncRow { last_attempt_at_ms: now_ms(), ..IrSyncRow::new(log_id, server, login_id, state) };
    match outcome {
        ScoreOutcome::Accepted { score_id, replay, trace, .. } => {
            IrSyncRow { score_id: score_id.clone(), replay_state: *replay, trace_state: *trace, ..base(SyncState::Accepted) }
        }
        ScoreOutcome::Rejected(rejection) => IrSyncRow {
            http_status: rejection.status,
            error_code: rejection.code.clone(),
            error_field: rejection.field.clone(),
            error_reason: rejection.reason.clone().or_else(|| Some(rejection.detail.clone())),
            refused_under: Some(refused_under.to_string()),
            ..base(SyncState::Rejected)
        },
        ScoreOutcome::Retry { detail } => IrSyncRow { error_reason: Some(detail.clone()), ..base(SyncState::Retry) },
        ScoreOutcome::Skipped { code, reason } => {
            IrSyncRow { error_code: Some((*code).to_string()), error_reason: Some((*reason).to_string()), ..base(SyncState::Skipped) }
        }
    }
}

/// One `course_sync` row of what a server said of a kept course, as [`sync_row`] writes a play's.
pub(crate) fn course_sync_row(course_id: i64, server: &str, login_id: &str, outcome: &ScoreOutcome, refused_under: &str) -> CourseSyncRow {
    let row = sync_row(course_id, server, login_id, outcome, refused_under);
    CourseSyncRow {
        score_id: row.score_id,
        http_status: row.http_status,
        error_code: row.error_code,
        error_field: row.error_field,
        error_reason: row.error_reason,
        refused_under: row.refused_under,
        last_attempt_at_ms: row.last_attempt_at_ms,
        ..CourseSyncRow::new(course_id, server, login_id, row.state)
    }
}

impl AppShared {
    /// This client's build, as a refusal is written down under: the version and, when the
    /// executable could be hashed, its hash.
    pub(crate) fn client_build(&self) -> String {
        match &self.build_sha256 {
            Some(hash) => format!("{CLIENT_NAME}+{hash}"),
            None => CLIENT_NAME.to_string(),
        }
    }

    /// The builds a refusal given now is written down under: this client's, and the server's as the
    /// last sync heard it.
    pub(crate) fn refusal_key(&self) -> String {
        refusal_key(&self.client_build(), self.server_build.as_deref().unwrap_or_default())
    }

    /// The server records are synced to and the account they are sent under, when there is both.
    ///
    /// The account is the one the server named for the token ([`AccountSession::login_id`]) and no
    /// other: a PLAYER ID typed into the settings is not the token's owner, and every play sent
    /// under it would be refused for it.
    ///
    /// [`AccountSession::login_id`]: crate::ir_session::AccountSession::login_id
    pub(crate) fn record_sync_target(&self) -> Option<(String, String)> {
        let url = self.config.network.server_url.as_deref().map(server_key).filter(|key| !key.is_empty())?;
        if !self.session.is_logged_in() {
            return None;
        }
        let login_id = self.session.login_id().map(str::trim).filter(|login_id| !login_id.is_empty())?;
        Some((url, login_id.to_string()))
    }

    /// SYNC RECORDS NOW: sync every play, every refusal included, saying why when it cannot and
    /// when there is nothing to send.
    pub(crate) fn sync_records_now(&mut self) {
        if self.config.network.server_url.is_none() {
            self.net_status = "set SERVER URL first".to_string();
            return;
        }
        if !self.session.is_logged_in() {
            self.net_status = "log in first".to_string();
            return;
        }
        if self.session.login_id().is_none_or(|login_id| login_id.trim().is_empty()) {
            self.net_status = ACCOUNT_NOT_CONFIRMED.to_string();
            return;
        }
        if self.record_sync.is_some() {
            self.net_status = "records sync already running".to_string();
            return;
        }
        self.start_sync(SyncScope::All, true);
    }

    /// Start sending `scope` to the score server on its own, unless nobody is signed in to it. A
    /// play the server refused under the builds the sync sends under is left out. A sync already
    /// running takes named plays after it ends, and a second full sync is not started. Returns
    /// whether a sync was started.
    pub(crate) fn start_record_sync(&mut self, scope: SyncScope) -> bool {
        self.start_sync(scope, false)
    }

    /// Start a sync of `scope`, `by_hand` when the player asked for it.
    ///
    /// The play a result screen's submission is still carrying is left out: that submission writes
    /// its own answer, and sending the same play twice at once would race the two answers.
    ///
    /// Nothing is read here: the worker picks the plays through a connection of its own
    /// ([`spawn_planned_sync`]), so a history of any size costs the frame thread two lookups of the
    /// newest ids.
    fn start_sync(&mut self, scope: SyncScope, by_hand: bool) -> bool {
        let Some((server, login_id)) = self.record_sync_target() else {
            return false;
        };
        if self.record_sync.is_some() {
            if let SyncScope::Logs(ids) = scope {
                self.record_sync_pending.extend(ids);
            }
            return false;
        }
        let Some(db) = self.scoredb.as_ref() else {
            return false;
        };
        let newest = db.newest_play_id().and_then(|play| Ok((play, db.newest_course_id()?)));
        let (up_to, courses_up_to) = match newest {
            Ok(newest) => newest,
            Err(e) => {
                notify(Level::Warn, format!("records sync not started: {e}"));
                return false;
            }
        };
        let (db_path, _, _) = crate::scoredb_store::score_paths(self.settings_path.parent().unwrap_or_else(|| Path::new(".")));
        let named = matches!(scope, SyncScope::Logs(_));
        let plan = SyncPlan {
            db_path,
            server_key: server.clone(),
            login_id: login_id.clone(),
            scope,
            by_hand,
            left_out: self.submit_log.as_ref().map(|live| live.log_id),
            up_to,
            courses_up_to,
            client_build: self.client_build(),
            upload_traces: self.config.network.auto_upload_trace,
            upload_replays: self.config.network.auto_upload_replay,
            replay_dir: self.replay_dir.clone(),
            saves_settled: self.saves.settled(),
            pacing: self.record_sync_pacing,
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let (charts_tx, charts_rx) = mpsc::channel();
        let rx = spawn_planned_sync(self.server.clone(), plan, Arc::clone(&cancel), charts_rx);
        let refused_under = self.refusal_key();
        self.record_sync = Some(RecordSyncRun {
            rx,
            cancel,
            charts: Some(charts_tx),
            server_key: server,
            login_id,
            refused_under,
            by_hand,
            named,
            quiet: false,
            scores_total: 0,
            traces_total: None,
            traces_done: 0,
            replays_total: None,
            replays_done: 0,
            tally: SyncTally::default(),
        });
        if by_hand {
            self.net_status = "records: picking what to send".to_string();
        }
        true
    }

    /// Stop a running sync: it ends at its next play, and what it reports from now on is let go.
    /// What it already reported was written as it came.
    pub(crate) fn cancel_record_sync(&mut self) {
        if let Some(run) = self.record_sync.take() {
            run.cancel.store(true, Ordering::Relaxed);
        }
        self.record_sync_pending.clear();
    }

    /// Read where every play stands with the server now set, for the record screens.
    pub(crate) fn reload_record_sync_states(&mut self) {
        let server = self.config.network.server_url.as_deref().map(server_key).unwrap_or_default();
        self.record_sync_states = match (&self.scoredb, server.is_empty()) {
            (Some(db), false) => db.sync_states(&server).unwrap_or_default(),
            _ => HashMap::new(),
        };
    }

    /// Take everything the sync worker reported since the last frame, writing each answer down as
    /// it comes.
    pub(crate) fn poll_record_sync(&mut self) {
        loop {
            let Some(run) = self.record_sync.as_mut() else {
                return;
            };
            let event = match run.rx.try_recv() {
                Ok(event) => event,
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => SyncEvent::Finished { stop: Some(SyncStop::Unreachable("records sync worker stopped".to_string())) },
            };
            self.take_sync_event(event);
        }
    }

    fn take_sync_event(&mut self, event: SyncEvent) {
        let client_build = self.client_build();
        let Some(run) = self.record_sync.as_mut() else {
            return;
        };
        let (server, login_id, refused_under) = (run.server_key.clone(), run.login_id.clone(), run.refused_under.clone());
        match event {
            SyncEvent::Picked { server_build, md5s } => {
                run.refused_under = refusal_key(&client_build, &server_build);
                if let Some(charts) = run.charts.take() {
                    let _ = charts.send(chart_facts(&self.library, &md5s));
                }
                self.server_build = Some(server_build);
            }
            SyncEvent::Started { scores, courses, traces, replays } => {
                run.scores_total = scores;
                let only_traces = run.named && scores + courses + replays == 0;
                run.quiet = !run.by_hand && (only_traces || scores + courses + traces + replays == 0);
                if !run.quiet {
                    self.net_status = run.progress_line();
                }
            }
            SyncEvent::Backfilled { log_id, sha256 } => {
                if let Some(db) = self.scoredb.as_mut()
                    && let Err(e) = db.backfill_sha256(log_id, &sha256)
                {
                    notify(Level::Warn, format!("chart hash not kept: {e}"));
                }
            }
            SyncEvent::Score { log_id, outcome } => {
                run.tally.count_score(&outcome);
                self.net_status = run.progress_line();
                let row = sync_row(log_id, &server, &login_id, &outcome, &refused_under);
                self.write_sync_row(row);
            }
            SyncEvent::Course { course_id, outcome } => {
                run.tally.count_course(&outcome);
                let row = course_sync_row(course_id, &server, &login_id, &outcome, &refused_under);
                self.write_course_sync_row(&row);
            }
            SyncEvent::Traces { total } => {
                run.traces_total = Some(total);
                if !run.quiet {
                    self.net_status = run.progress_line();
                }
            }
            SyncEvent::Trace { log_id, state, trace_id, error } => {
                run.tally.count_trace(state);
                run.traces_done += 1;
                if !run.quiet {
                    self.net_status = run.progress_line();
                }
                let trace_refused_under = (state == ReplayState::Rejected).then_some(refused_under);
                let answer = TraceAnswer { state, trace_id: trace_id.as_deref(), error: error.as_deref(), refused_under: trace_refused_under.as_deref() };
                if let Some(db) = self.scoredb.as_mut()
                    && let Err(e) = db.put_trace_state(log_id, &server, &answer)
                {
                    notify(Level::Warn, format!("trace state not kept: {e}"));
                }
                if let Some(row) = self.record_sync_states.get_mut(&log_id) {
                    row.trace_state = state;
                    row.trace_id = trace_id;
                    row.trace_error = error;
                    row.trace_refused_under = trace_refused_under;
                }
            }
            SyncEvent::Replays { total } => {
                run.replays_total = Some(total);
                if !run.quiet {
                    self.net_status = run.progress_line();
                }
            }
            SyncEvent::Replay { log_id, state, replay_id, error } => {
                run.tally.count_replay(state);
                run.replays_done += 1;
                self.net_status = run.progress_line();
                let replay_refused_under = (state == ReplayState::Rejected).then_some(refused_under);
                let answer = ReplayAnswer { state, replay_id: replay_id.as_deref(), error: error.as_deref(), refused_under: replay_refused_under.as_deref() };
                if let Some(db) = self.scoredb.as_mut()
                    && let Err(e) = db.put_replay_state(log_id, &server, &answer)
                {
                    notify(Level::Warn, format!("replay state not kept: {e}"));
                }
                if let Some(row) = self.record_sync_states.get_mut(&log_id) {
                    row.replay_state = state;
                    row.replay_id = replay_id;
                    row.replay_error = error;
                    row.replay_refused_under = replay_refused_under;
                }
            }
            SyncEvent::Finished { stop } => {
                let (tally, quiet, by_hand) = (run.tally, run.quiet, run.by_hand);
                let found_nothing = run.scores_total == 0
                    && run.traces_total.unwrap_or_default() == 0
                    && run.replays_total.unwrap_or_default() == 0
                    && tally == SyncTally::default();
                self.record_sync = None;
                match &stop {
                    Some(stop) => {
                        let line = stop_line(stop);
                        self.net_status = line.clone();
                        notify(Level::Warn, line);
                    }
                    None if quiet => {}
                    None if by_hand && found_nothing => self.net_status = NOTHING_TO_SEND.to_string(),
                    None => {
                        let line = tally.summary();
                        let level = if tally.is_troubled() { Level::Warn } else { Level::Info };
                        self.net_status = line.clone();
                        notify(level, line);
                    }
                }
                if stop.is_none() && !self.record_sync_pending.is_empty() {
                    let pending = std::mem::take(&mut self.record_sync_pending);
                    self.start_record_sync(SyncScope::Logs(pending));
                }
            }
        }
    }

    /// Write one course's answer to the database.
    pub(crate) fn write_course_sync_row(&mut self, row: &CourseSyncRow) {
        if let Some(db) = self.scoredb.as_mut()
            && let Err(e) = db.put_course_sync_state(row)
        {
            notify(Level::Warn, format!("course sync state not kept: {e}"));
        }
    }

    /// Write one play's answer to the database and to the states the record screens read.
    pub(crate) fn write_sync_row(&mut self, row: IrSyncRow) {
        if let Some(db) = self.scoredb.as_mut()
            && let Err(e) = db.put_sync_state(&row)
        {
            notify(Level::Warn, format!("records sync state not kept: {e}"));
        }
        let attempts = self.record_sync_states.get(&row.log_id).map_or(0, |old| old.attempts) + 1;
        self.record_sync_states.insert(row.log_id, IrSyncRow { attempts, ..row });
    }

    /// The SYNC RECORDS NOW row's value: the progress of a running sync, or the action mark.
    pub(crate) fn sync_records_value(&self) -> Option<String> {
        self.record_sync.as_ref().map(RecordSyncRun::progress_value)
    }

    /// What the record modal says of where the play `log_id` stands with the score server, or
    /// `None` when no server is set up.
    pub(crate) fn record_ir_line(&self, log_id: Option<i64>) -> Option<(String, IrLineKind)> {
        self.config.network.server_url.as_deref().map(server_key).filter(|key| !key.is_empty())?;
        let row = log_id.and_then(|id| self.record_sync_states.get(&id));
        let signed_in = self.session.is_logged_in();
        let account = self.session.login_id().filter(|_| signed_in);
        Some(ir_line_of(row, signed_in, account))
    }
}

/// The record modal's line for a play whose state is `row`, for the account `account` signed in
/// now. A play the server holds under another account is not said to be on the server plainly: it
/// is not in this account's records.
pub(crate) fn ir_line_of(row: Option<&IrSyncRow>, signed_in: bool, account: Option<&str>) -> (String, IrLineKind) {
    let Some(row) = row else {
        return match signed_in {
            true => ("IR: NOT SENT YET".to_string(), IrLineKind::Info),
            false => ("IR: NOT SENT (NOT SIGNED IN)".to_string(), IrLineKind::Info),
        };
    };
    match row.state {
        SyncState::Accepted => match account {
            Some(account) if !row.login_id.is_empty() && row.login_id != account => {
                (truncate(&format!("IR: ON SERVER (AS {})", row.login_id)), IrLineKind::Info)
            }
            _ => ("IR: ON SERVER".to_string(), IrLineKind::Good),
        },
        SyncState::Rejected => (truncate(&format!("IR: REJECTED {}", rejection_words(row))), IrLineKind::Bad),
        SyncState::Retry => ("IR: NOT SENT YET (WILL RETRY)".to_string(), IrLineKind::Warn),
        SyncState::Skipped if row.error_code.as_deref() == Some(SKIPPED_OLD_SCRATCH_AUTO_RULE) => {
            ("IR: NOT SENT (OLD SCRATCH AUTO RULE)".to_string(), IrLineKind::Info)
        }
        SyncState::Skipped => ("IR: NOT SENT (CHART NOT IN LIBRARY)".to_string(), IrLineKind::Warn),
    }
}

/// What a refusal is said to be over: the code, the field and the reason as far as the server gave
/// them, and the HTTP status when it gave no code.
fn rejection_words(row: &IrSyncRow) -> String {
    match (&row.error_code, &row.error_field, &row.error_reason) {
        (Some(code), Some(field), Some(reason)) => format!("{code}: {field} ({reason})"),
        (Some(code), Some(field), None) => format!("{code}: {field}"),
        (Some(code), None, _) => code.clone(),
        (None, ..) => row.http_status.map_or_else(String::new, |status| format!("(HTTP {status})")),
    }
}

#[cfg(test)]
mod tests;
