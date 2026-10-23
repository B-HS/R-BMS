//! A records sync against a score server that lives in the test: what is sent, in what order, what
//! is written down of each answer, and when the sync stops.

use std::collections::VecDeque;
use std::sync::Mutex;

use rbms_ir::{CourseSubmission, IrError, PlayTrace, PlayerProfile, ReplayData, ServerCapabilities, ServerInfo, SubmitResponse, VersionInfo};
use rbms_store::scoredb::{ASSIST_UNRECORDED, JudgeCounts, SEED_UNRECORDED, mode_id};

use super::*;
use crate::ir_session::AccountSession;

/// The server and account the app fixtures are signed in to.
const SERVER_URL: &str = "https://records.test/";
const ACCOUNT_ID: &str = "dj";
const ACCOUNT_TOKEN: &str = "token-of-a-test";

/// How long a test waits for a sync to end before it calls it lost.
const SYNC_WAIT: Duration = Duration::from_secs(10);
/// How long a test sleeps between two looks at a sync that has not ended.
const SYNC_POLL: Duration = Duration::from_millis(2);

/// How often a test's sync looks at whether it was cancelled while it waits.
const CANCEL_LOOK: Duration = Duration::from_millis(1);

/// No waiting at all, so a sync of many plays runs at once.
const NO_PACING: SyncPacing = SyncPacing {
    score_interval: Duration::ZERO,
    replay_interval: Duration::ZERO,
    trace_interval: Duration::ZERO,
    rate_limit_wait: Duration::ZERO,
    cancel_check: CANCEL_LOOK,
};

/// The status a server failing on its own side answers with.
const SERVER_FAILURE_STATUS: u16 = 500;

/// The play the player reported as #52: a finished run with more 空POOR than POORs let pass.
const RUN_52_COUNTS: [u32; JUDGE_TIER_COUNT] = [1_500, 250, 50, 15, 20, 43];
const RUN_52_NOTES: u32 = 1_835;

/// The second a fixture play ended in, and the millisecond within it its replay is named by.
const PLAYED_SECOND: i64 = 1_700_000_000;
const PLAYED_MS_IN_SECOND: i64 = 456;

/// A four-note chart, which is all a play needs to have a chart file to hash.
const HASHED_CHART: &[u8] = b"#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#00111:01010101\n";

/// What the test server answers each submission, in order; once the script runs out, it takes every
/// play as new.
#[derive(Default)]
struct ScriptedServer {
    answers: Mutex<VecDeque<Result<SubmitResponse, IrError>>>,
    uploads_answers: Mutex<VecDeque<Result<String, IrError>>>,
    submitted: Mutex<Vec<ScoreSubmission>>,
    uploaded: Mutex<Vec<(ChartId, ReplayData)>>,
    /// Set when the first submission arrives, so a test can cancel a sync in the middle.
    on_submit: Mutex<Option<Arc<AtomicBool>>>,
    /// The commit the server says it runs; `None` for a server that does not say.
    commit: Mutex<Option<String>>,
    /// Charts whose every submission the server fails on its side.
    failing: Mutex<Vec<String>>,
    /// Holds the server's answer to which build it runs until the test lets go, which is the first
    /// thing a sync the app starts asks.
    version_gate: Mutex<Option<Receiver<()>>>,
    /// Whether the server says it takes traces.
    takes_traces: Mutex<bool>,
    /// How many times the server was asked what it takes.
    health_asked: Mutex<usize>,
    traces_answers: Mutex<VecDeque<Result<String, IrError>>>,
    traced: Mutex<Vec<(ChartId, PlayTrace)>>,
    /// Every course body sent, as it went on the wire. The server takes no course: it answers each
    /// as a server without a course endpoint does.
    courses_sent: Mutex<Vec<String>>,
}

impl ScriptedServer {
    fn answering(answers: Vec<Result<SubmitResponse, IrError>>) -> Arc<ScriptedServer> {
        let server = ScriptedServer::default();
        *server.answers.lock().expect("the script") = answers.into();
        Arc::new(server)
    }

    fn submitted(&self) -> Vec<ScoreSubmission> {
        self.submitted.lock().expect("the submission log").clone()
    }

    fn uploaded(&self) -> Vec<(ChartId, ReplayData)> {
        self.uploaded.lock().expect("the upload log").clone()
    }

    fn traced(&self) -> Vec<(ChartId, PlayTrace)> {
        self.traced.lock().expect("the trace log").clone()
    }

    fn taking_traces(self: Arc<Self>) -> Arc<Self> {
        *self.takes_traces.lock().expect("the capability") = true;
        self
    }
}

fn taken(score_id: &str) -> SubmitResponse {
    SubmitResponse { accepted: true, ranked: true, score_id: Some(score_id.into()), message: Some("saved".into()), ..Default::default() }
}

fn already(score_id: &str) -> SubmitResponse {
    SubmitResponse { message: Some(rbms_ir::DUPLICATE_SUBMISSION_MESSAGE.into()), ..taken(score_id) }
}

fn judge_refusal() -> IrError {
    IrError::Server(
        400,
        r#"{"success":false,"error":{"code":"VALIDATION_ERROR","message":"m","details":{"field":"judge","reason":"judge count exceeds total_notes"}}}"#.into(),
    )
}

impl ScoreServer for ScriptedServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        *self.health_asked.lock().expect("the health count") += 1;
        if !*self.takes_traces.lock().expect("the capability") {
            return Err(IrError::Unsupported);
        }
        let capabilities = ServerCapabilities { traces: true, ..ServerCapabilities::default() };
        Ok(ServerInfo { name: "rbms-ir".into(), version: "0.1.0".into(), ir_compat: "superset-1".into(), capabilities })
    }
    fn upload_trace(&self, chart: &ChartId, trace: &PlayTrace) -> Result<String, IrError> {
        let count = {
            let mut log = self.traced.lock().expect("the trace log");
            log.push((chart.clone(), trace.clone()));
            log.len()
        };
        self.traces_answers.lock().expect("the trace script").pop_front().unwrap_or_else(|| Ok(format!("tr_{count}")))
    }
    fn version(&self) -> Result<VersionInfo, IrError> {
        if let Some(gate) = self.version_gate.lock().expect("the version gate").take() {
            let _ = gate.recv();
        }
        match self.commit.lock().expect("the commit").clone() {
            Some(commit) => Ok(VersionInfo { api_version: rbms_ir::API_VERSION, server: "rbms-web".into(), commit: Some(commit) }),
            None => Err(IrError::Unsupported),
        }
    }
    fn submit_score(&self, sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        let count = {
            let mut log = self.submitted.lock().expect("the submission log");
            log.push(sub.clone());
            log.len()
        };
        if let Some(cancel) = self.on_submit.lock().expect("the cancel hook").as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        if self.failing.lock().expect("the failing charts").contains(&sub.chart.id.md5) {
            return Err(IrError::Server(SERVER_FAILURE_STATUS, String::new()));
        }
        self.answers.lock().expect("the script").pop_front().unwrap_or_else(|| Ok(taken(&format!("sc_{count}"))))
    }
    fn chart_ranking(&self, _chart: &ChartId, _limit: u32) -> Result<Vec<rbms_ir::ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    fn player_best(&self, _chart: &ChartId, _player: &PlayerId) -> Result<Option<rbms_ir::ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
    fn player_profile(&self, _player: &PlayerId) -> Result<PlayerProfile, IrError> {
        Err(IrError::Unsupported)
    }
    fn rivals(&self, _player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_course(&self, sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        self.courses_sent.lock().expect("the course log").push(sub.to_json().expect("the course serialises"));
        Err(IrError::Unsupported)
    }
    fn upload_replay(&self, chart: &ChartId, replay: &ReplayData) -> Result<String, IrError> {
        let count = {
            let mut log = self.uploaded.lock().expect("the upload log");
            log.push((chart.clone(), replay.clone()));
            log.len()
        };
        self.uploads_answers.lock().expect("the upload script").pop_front().unwrap_or_else(|| Ok(format!("rp_{count}")))
    }
}

/// A play stored on the chart `md5` at `date`: judged as `counts`, every one late.
fn play(id: i64, md5: &str, date: i64, counts: [u32; JUDGE_TIER_COUNT], notes: u32) -> PlayLog {
    let judge = JudgeCounts::all_late(counts);
    PlayLog {
        id,
        chart_key: md5.into(),
        mode: mode_id("BEAT_7K"),
        ln_mode: SCORE_LN_MODE_FROM_CHART.into(),
        md5: md5.into(),
        sha256: format!("sha-of-{md5}"),
        title: "Chart".into(),
        clear: 5,
        judge,
        notes,
        combo: counts[0],
        minbp: judge.combo_breaks(),
        avgjudge: UNSET_MINIMUM,
        gauge: rbms_store::scoredb::gauge_id("hard"),
        gauge_value: 81.5,
        seed: 42,
        date,
        rule_version: rbms_store::SCORE_RULE_VERSION,
        ..PlayLog::default()
    }
}

fn candidate(log: PlayLog) -> SyncCandidate {
    SyncCandidate { log, body: None, played_as: None, state: None, score_id: None, replay_state: None, has_trace: false, trace_state: None }
}

fn job(scores: Vec<SyncCandidate>) -> SyncJob {
    SyncJob {
        login_id: ACCOUNT_ID.into(),
        scores,
        courses: Vec::new(),
        traces: Vec::new(),
        replays: Vec::new(),
        upload_traces: false,
        db_path: std::env::temp_dir().join("rbms-record-sync-no-database.sqlite"),
        upload_replays: true,
        replay_dir: std::env::temp_dir(),
        saves_settled: None,
        charts: HashMap::new(),
        pacing: NO_PACING,
    }
}

/// Send `job` on a worker thread, as a sync the frame thread starts does once it has picked it.
fn spawn_record_sync(server: Arc<dyn ScoreServer>, job: SyncJob, cancel: Arc<AtomicBool>) -> Receiver<SyncEvent> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stop = run_record_sync(server.as_ref(), job, &cancel, &tx);
        let _ = tx.send(SyncEvent::Finished { stop });
    });
    rx
}

/// Run `job` against `server` to its end and answer everything it reported.
fn run(server: Arc<ScriptedServer>, job: SyncJob) -> Vec<SyncEvent> {
    run_cancellable(server, job, Arc::new(AtomicBool::new(false)))
}

fn run_cancellable(server: Arc<ScriptedServer>, job: SyncJob, cancel: Arc<AtomicBool>) -> Vec<SyncEvent> {
    let rx = spawn_record_sync(server, job, cancel);
    let mut events = Vec::new();
    loop {
        let event = rx.recv_timeout(SYNC_WAIT).expect("the sync reports until it ends");
        let finished = matches!(event, SyncEvent::Finished { .. });
        events.push(event);
        if finished {
            return events;
        }
    }
}

fn score_outcomes(events: &[SyncEvent]) -> Vec<(i64, ScoreOutcome)> {
    events
        .iter()
        .filter_map(|event| match event {
            SyncEvent::Score { log_id, outcome } => Some((*log_id, outcome.clone())),
            _ => None,
        })
        .collect()
}

fn finished(events: &[SyncEvent]) -> Option<SyncStop> {
    match events.last() {
        Some(SyncEvent::Finished { stop }) => stop.clone(),
        other => panic!("the sync did not end: {other:?}"),
    }
}

/// A folder of the test's own, emptied of whatever an earlier run left in it.
fn own_folder(tag: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("rbms-record-sync-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder is writable");
    folder
}

fn saved_replay(dir: &Path, name: &str) {
    let replay = Replay {
        chart_path: String::new(),
        md5: "aa".into(),
        mode: Mode::BEAT_7K.name.into(),
        random: "OFF".into(),
        seed: 42,
        offset_ms: 0,
        scratch_auto: false,
        gauge: "hard".into(),
        judge: Default::default(),
        events: vec![rbms_store::ReplayEvent { t: 1_000, lane: 1, press: true, backward: false }],
    };
    replay.try_save(&dir.join(name)).expect("the replay is written");
}

#[test]
fn a_stored_body_is_sent_under_the_signed_in_account() {
    let mut kept = rebuilt_submission(&play(1, "aa", PLAYED_SECOND, [10, 0, 0, 0, 0, 0], 10), None);
    kept.player.id = "guest".into();
    kept.skin = "KEPT".into();
    kept.options.hispeed = 3.5;
    let server = ScriptedServer::answering(Vec::new());
    let mut stored = candidate(play(1, "aa", PLAYED_SECOND, [10, 0, 0, 0, 0, 0], 10));
    stored.body = kept.to_json();
    let events = run(Arc::clone(&server), job(vec![stored]));
    let sent = server.submitted();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].player.id, ACCOUNT_ID, "a body kept as a guest goes under the account that syncs it");
    assert_eq!((sent[0].skin.as_str(), sent[0].options.hispeed), ("KEPT", 3.5), "the kept body is sent as it was");
    assert_eq!(
        score_outcomes(&events),
        [(1, ScoreOutcome::Accepted { score_id: Some("sc_1".into()), already: false, replay: ReplayState::None, trace: ReplayState::None })]
    );
    assert_eq!(finished(&events), None);
}

/// A course an older build kept states neither the gauge it finished on, the COURSE GAUGE rule nor
/// its stages, and is sent again as it was kept: the build that sends it has no way to know them.
#[test]
fn a_course_kept_by_an_older_build_is_sent_without_the_gauge_fields() {
    let kept_by_an_older_build = format!(
        r#"{{"api_version":{},"course_hash":"course","player":{{"id":"guest"}},"clear":"Normal","ex_score":10,"judge":{{"pgreat":5,"great":0,"good":0,"bad":0,"poor":0,"miss":0,"fast":0,"slow":0,"combobreak":0}},"max_combo":10,"gauge_value":80.0,"charts":[{{"md5":"aa","sha256":""}}],"played_at":{},"lntype":0,"max_ex_score":20,"minbp":0,"trophy":null,"extra":{{}}}}"#,
        rbms_ir::API_VERSION,
        PLAYED_SECOND * MILLIS_PER_SECOND
    );
    let course = KeptCourse {
        id: 1,
        course_hash: "course".into(),
        played_at_ms: PLAYED_SECOND * MILLIS_PER_SECOND,
        body: kept_by_an_older_build,
        played_as: None,
        state: None,
    };
    let server = ScriptedServer::answering(Vec::new());
    run(Arc::clone(&server), SyncJob { courses: vec![course], ..job(Vec::new()) });
    let sent = server.courses_sent.lock().expect("the course log").clone();
    assert_eq!(sent.len(), 1, "the kept course went up once");
    for key in ["gauge", "course_gauge", "stages"] {
        assert!(!sent[0].contains(&format!("\"{key}\":")), "{key} is not made up for a course that never stated it: {}", sent[0]);
    }
    let resent = CourseSubmission::from_json(&sent[0]).expect("the body reads as a course");
    assert_eq!(resent.player.id, ACCOUNT_ID, "it goes under the account that syncs it");
    assert_eq!((resent.gauge, resent.course_gauge, resent.stages.len()), (None, None, 0));
}

#[test]
fn a_row_without_a_body_is_rebuilt_from_the_log() {
    let log = play(52, "aa", PLAYED_SECOND, RUN_52_COUNTS, RUN_52_NOTES);
    let facts = ChartFacts { path: PathBuf::from("/songs/aa.bms"), artist: "Artist".into(), level: "12".into() };
    let sub = rebuilt_submission(&log, Some(&facts));
    assert_eq!((sub.judge.pgreat + sub.judge.great + sub.judge.good + sub.judge.bad), 1_815);
    assert_eq!((sub.judge.poor, sub.judge.miss, sub.judge.empty_poor), (20, 43, 43), "POOR is the notes let pass, MISS the 空POOR");
    assert_eq!((sub.total_notes, sub.passnotes, sub.max_ex_score), (RUN_52_NOTES, RUN_52_NOTES, RUN_52_NOTES * 2));
    assert_eq!(sub.ex_score, 1_500 * 2 + 250);
    assert_eq!(sub.judge.combobreak, 15 + 20, "on seven keys a 空POOR keeps the combo");
    assert_eq!((sub.judge.lpg, sub.judge.epg, sub.judge.avgjudge, sub.judge.fast), (1_500, 0, 0, 0));
    assert_eq!((sub.chart.id.md5.as_str(), sub.chart.id.sha256.as_str()), ("aa", "sha-of-aa"));
    assert_eq!((sub.chart.artist.as_deref(), sub.chart.level), (Some("Artist"), Some(12)));
    assert_eq!((sub.mode.as_str(), sub.clear, sub.options.gauge), ("BEAT_7K", rbms_ir::ClearLamp::Normal, rbms_ir::GaugeType::Hard));
    assert_eq!((sub.options.scratch_auto, sub.options.assist.len(), sub.options.judge_rate), (false, 0, REBUILT_JUDGE_RATE));
    assert_eq!((sub.seed, sub.client_build_sha256.as_deref(), sub.client.as_str()), (42, None, CLIENT_NAME));
    assert!(sub.is_reconstructed(), "the body says it was rebuilt");
    let unrecorded_seed = rebuilt_submission(&PlayLog { seed: SEED_UNRECORDED, ..log }, None);
    assert_eq!(unrecorded_seed.seed, 0);
}

#[test]
fn a_rebuilt_row_takes_its_millisecond_stamp_from_the_replay_file_name() {
    let exact = PLAYED_SECOND * MILLIS_PER_SECOND + PLAYED_MS_IN_SECOND;
    let log = PlayLog { replay_file: Some(format!("abcdef01-{exact}.ron")), ..play(1, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1) };
    assert_eq!(rebuilt_submission(&log, None).played_at, exact);
    let other_second = PlayLog { replay_file: Some(format!("abcdef01-{}.ron", exact + MILLIS_PER_SECOND)), ..log.clone() };
    assert_eq!(rebuilt_played_at(&other_second), PLAYED_SECOND * MILLIS_PER_SECOND, "a replay of another second names another moment");
    let odd_name = PlayLog { replay_file: Some("renamed.ron".into()), ..log };
    assert_eq!(rebuilt_played_at(&odd_name), PLAYED_SECOND * MILLIS_PER_SECOND);
}

#[test]
fn a_rebuilt_row_without_a_replay_uses_the_second_stamp() {
    let log = play(1, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1);
    assert_eq!(rebuilt_submission(&log, None).played_at, PLAYED_SECOND * MILLIS_PER_SECOND);
}

#[test]
fn an_unknown_assist_is_sent_as_an_unrecorded_assist() {
    let base = play(1, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1);
    let assists = |assist| {
        let sub = rebuilt_submission(&PlayLog { assist, ..base.clone() }, None);
        (sub.options.scratch_auto, sub.options.assist)
    };
    assert_eq!(assists(ASSIST_UNRECORDED), (false, vec![UNRECORDED_ASSIST_FLAG.to_string()]));
    assert_eq!(assists(SCRATCH_AUTO_ASSIST), (true, vec![AUTO_SCRATCH_FLAG.to_string()]));
    assert_eq!(assists(CUSTOM_JUDGE_ASSIST), (false, vec![CUSTOM_JUDGE_FLAG.to_string()]));
    assert_eq!(assists(0), (false, Vec::new()));
}

#[test]
fn a_row_with_no_sha256_is_filled_from_the_library_or_skipped() {
    let folder = own_folder("hash");
    let chart_path = folder.join("hashed.bms");
    std::fs::write(&chart_path, HASHED_CHART).expect("the chart is written");
    let source = rbms_parser::parse(HASHED_CHART);
    let in_library = PlayLog { sha256: String::new(), ..play(1, &source.md5, PLAYED_SECOND, [4, 0, 0, 0, 0, 0], 4) };
    let elsewhere = PlayLog { sha256: String::new(), ..play(2, "ffffffffffffffffffffffffffffffff", PLAYED_SECOND, [4, 0, 0, 0, 0, 0], 4) };
    let mut sync = job(vec![candidate(in_library), candidate(elsewhere)]);
    sync.charts.insert(source.md5.clone(), ChartFacts { path: chart_path, ..ChartFacts::default() });
    let server = ScriptedServer::answering(Vec::new());
    let events = run(Arc::clone(&server), sync);
    assert!(events.contains(&SyncEvent::Backfilled { log_id: 1, sha256: source.sha256.clone() }), "{events:?}");
    let sent = server.submitted();
    assert_eq!(sent.len(), 1, "a play whose chart cannot be hashed is not sent");
    assert_eq!(sent[0].chart.id.sha256, source.sha256);
    assert_eq!(score_outcomes(&events)[1], (2, ScoreOutcome::Skipped { code: SKIPPED_NO_CHART_HASH, reason: NO_CHART_HASH_REASON }));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_replay_goes_up_after_its_score_with_the_score_id() {
    let folder = own_folder("replay");
    saved_replay(&folder, "aa-1.ron");
    let log = PlayLog { replay_file: Some("aa-1.ron".into()), ..play(7, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1) };
    let mut sync = job(vec![candidate(log)]);
    sync.replay_dir.clone_from(&folder);
    let server = ScriptedServer::answering(vec![Ok(taken("sc_seven"))]);
    let events = run(Arc::clone(&server), sync);
    assert_eq!(
        score_outcomes(&events),
        [(7, ScoreOutcome::Accepted { score_id: Some("sc_seven".into()), already: false, replay: ReplayState::Pending, trace: ReplayState::None })]
    );
    let uploads = server.uploaded();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].1.score_id.as_deref(), Some("sc_seven"), "the replay is linked to the score the server holds");
    assert_eq!(uploads[0].0, ChartId { md5: "aa".into(), sha256: "sha-of-aa".into() });
    assert!(events.contains(&SyncEvent::Replay { log_id: 7, state: ReplayState::Uploaded, replay_id: Some("rp_1".into()), error: None }), "{events:?}");

    let mut off = job(vec![candidate(PlayLog { replay_file: Some("aa-1.ron".into()), ..play(8, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1) })]);
    off.upload_replays = false;
    off.replay_dir.clone_from(&folder);
    let quiet = ScriptedServer::answering(Vec::new());
    run(Arc::clone(&quiet), off);
    assert!(quiet.uploaded().is_empty(), "AUTO UPLOAD REPLAY off leaves the replay pending");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_missing_replay_file_is_marked_missing() {
    let folder = own_folder("missing");
    let log = PlayLog { replay_file: Some("gone-1.ron".into()), ..play(3, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1) };
    let mut sync = job(Vec::new());
    sync.replays = vec![SyncCandidate { score_id: Some("sc_3".into()), ..candidate(log) }];
    sync.replay_dir.clone_from(&folder);
    let server = ScriptedServer::answering(Vec::new());
    let events = run(Arc::clone(&server), sync);
    assert!(events.contains(&SyncEvent::Replay { log_id: 3, state: ReplayState::Missing, replay_id: None, error: None }), "{events:?}");
    assert!(server.uploaded().is_empty());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_rate_limit_is_waited_out_and_the_same_run_sent_again() {
    let server = ScriptedServer::answering(vec![Err(IrError::RateLimited(String::new())), Ok(taken("sc_a"))]);
    let events = run(Arc::clone(&server), job(vec![candidate(play(1, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1))]));
    assert_eq!(server.submitted().len(), 2, "the same run went twice");
    assert_eq!(
        score_outcomes(&events),
        [(1, ScoreOutcome::Accepted { score_id: Some("sc_a".into()), already: false, replay: ReplayState::None, trace: ReplayState::None })]
    );
    assert_eq!(finished(&events), None);
}

#[test]
fn a_rate_limit_after_every_allowed_wait_stops_the_sync() {
    let limited = (0..=RATE_LIMIT_WAITS_PER_REQUEST).map(|_| Err(IrError::RateLimited(String::new()))).collect();
    let server = ScriptedServer::answering(limited);
    let plays = vec![candidate(play(1, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1)), candidate(play(2, "bb", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1))];
    let events = run(Arc::clone(&server), job(plays));
    assert_eq!(server.submitted().len(), RATE_LIMIT_WAITS_PER_REQUEST as usize + 1);
    assert_eq!(score_outcomes(&events), [(1, ScoreOutcome::Retry { detail: RATE_LIMITED_DETAIL.into() })], "the second play was not sent");
    assert_eq!(finished(&events), Some(SyncStop::RateLimited));
}

#[test]
fn three_transient_failures_in_a_row_stop_the_sync() {
    let down = (0..TRANSIENT_FAILURES_BEFORE_STOP).map(|_| Err(IrError::Server(503, String::new()))).collect();
    let server = ScriptedServer::answering(down);
    let plays: Vec<_> = (1..=5).map(|id| candidate(play(id, "aa", PLAYED_SECOND + id, [1, 0, 0, 0, 0, 0], 1))).collect();
    let events = run(Arc::clone(&server), job(plays));
    let outcomes = score_outcomes(&events);
    assert_eq!(outcomes.len(), TRANSIENT_FAILURES_BEFORE_STOP as usize);
    assert!(outcomes.iter().all(|(_, outcome)| matches!(outcome, ScoreOutcome::Retry { .. })));
    assert!(matches!(finished(&events), Some(SyncStop::Unreachable(_))));

    let flaky = ScriptedServer::answering(vec![Err(IrError::Network("reset".into())), Err(judge_refusal()), Err(IrError::Network("reset".into()))]);
    let plays: Vec<_> = (1..=4).map(|id| candidate(play(id, "aa", PLAYED_SECOND + id, [1, 0, 0, 0, 0, 0], 1))).collect();
    let events = run(Arc::clone(&flaky), job(plays));
    assert_eq!(finished(&events), None, "an answer between two failures starts the count again");
    assert_eq!(flaky.submitted().len(), 4);
}

#[test]
fn unauthorized_stops_the_sync_and_keeps_the_rest_for_next_time() {
    let server = ScriptedServer::answering(vec![Ok(taken("sc_1")), Err(IrError::Unauthorized(String::new()))]);
    let plays: Vec<_> = (1..=3).map(|id| candidate(play(id, "aa", PLAYED_SECOND + id, [1, 0, 0, 0, 0, 0], 1))).collect();
    let events = run(Arc::clone(&server), job(plays));
    let outcomes = score_outcomes(&events);
    assert_eq!(outcomes.len(), 2, "the third play was not sent");
    assert!(matches!(outcomes[1], (2, ScoreOutcome::Retry { .. })), "{outcomes:?}");
    assert!(matches!(finished(&events), Some(SyncStop::Unauthorized(_))));
}

#[test]
fn a_refused_play_is_written_down_with_its_reason_and_the_sync_goes_on() {
    let server = ScriptedServer::answering(vec![Err(judge_refusal()), Ok(already("sc_old"))]);
    let plays = vec![candidate(play(1, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1)), candidate(play(2, "bb", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1))];
    let events = run(Arc::clone(&server), job(plays));
    let outcomes = score_outcomes(&events);
    let (_, ScoreOutcome::Rejected(rejection)) = &outcomes[0] else {
        panic!("the first play was not refused: {outcomes:?}");
    };
    let row = sync_row(1, "s", ACCOUNT_ID, &outcomes[0].1, "rbms/test|c1");
    assert_eq!((rejection.field.as_deref(), row.state, row.refused_under.as_deref()), (Some("judge"), SyncState::Rejected, Some("rbms/test|c1")));
    assert_eq!(
        (row.http_status, row.error_code.as_deref(), row.error_field.as_deref(), row.error_reason.as_deref()),
        (Some(400), Some("VALIDATION_ERROR"), Some("judge"), Some("judge count exceeds total_notes"))
    );
    assert_eq!(
        outcomes[1],
        (2, ScoreOutcome::Accepted { score_id: Some("sc_old".into()), already: true, replay: ReplayState::None, trace: ReplayState::None })
    );
    assert_eq!(finished(&events), None);
}

#[test]
fn cancelling_ends_the_sync_between_runs() {
    let server = ScriptedServer::answering(Vec::new());
    let cancel = Arc::new(AtomicBool::new(false));
    *server.on_submit.lock().expect("the cancel hook") = Some(Arc::clone(&cancel));
    let plays: Vec<_> = (1..=3).map(|id| candidate(play(id, "aa", PLAYED_SECOND + id, [1, 0, 0, 0, 0, 0], 1))).collect();
    let events = run_cancellable(Arc::clone(&server), job(plays), cancel);
    assert_eq!(server.submitted().len(), 1, "the request on the wire is let finish and nothing after it is sent");
    assert_eq!(score_outcomes(&events).len(), 1, "the answer that came back is still reported");
    assert_eq!(finished(&events), Some(SyncStop::Cancelled));
}

#[test]
fn the_summary_names_only_what_happened() {
    let tally = SyncTally { sent: 120, already: 30, rejected: 5, failed: 2, skipped: 1, replays_uploaded: 80, replays_failed: 3, ..SyncTally::default() };
    assert_eq!(tally.summary(), "records: 120 sent, 30 already on server, 5 rejected, 2 failed, 1 skipped; replays: 80 uploaded, 3 failed");
    let courses = SyncTally { sent: 1, courses_sent: 2, courses_failed: 1, replays_uploaded: 1, ..SyncTally::default() };
    assert_eq!(courses.summary(), "records: 1 sent; courses: 2 sent, 1 failed; replays: 1 uploaded");
    assert_eq!(SyncTally { sent: 2, ..SyncTally::default() }.summary(), "records: 2 sent");
    assert_eq!(SyncTally { replays_uploaded: 1, ..SyncTally::default() }.summary(), "records: nothing sent; replays: 1 uploaded");
    assert_eq!(stop_line(&SyncStop::RateLimited), "records sync stopped: rate limited");
    assert_eq!(stop_line(&SyncStop::Unreachable("x".into())), "records sync stopped: server unreachable");
    assert_eq!(stop_line(&SyncStop::Unauthorized("not signed in (401)".into())), "records sync stopped: not signed in (401)");
    assert_eq!(stop_line(&SyncStop::Cancelled), "records sync cancelled");
    assert_eq!(stop_line(&SyncStop::Unreadable("locked".into())), "records sync not started: locked");
}

#[test]
fn the_record_modal_says_why_the_server_refused_the_run() {
    let refused = IrSyncRow {
        http_status: Some(400),
        error_code: Some("VALIDATION_ERROR".into()),
        error_field: Some("judge".into()),
        error_reason: Some("judge count exceeds total_notes".into()),
        ..IrSyncRow::new(1, "s", ACCOUNT_ID, SyncState::Rejected)
    };
    let (line, kind) = ir_line_of(Some(&refused), true, Some(ACCOUNT_ID));
    assert_eq!(kind, IrLineKind::Bad);
    assert_eq!(line, truncate("IR: REJECTED VALIDATION_ERROR: judge (judge count exceeds total_notes)"));
    assert!(line.starts_with("IR: REJECTED VALIDATION_ERROR: judge ("), "{line}");
    let bare = IrSyncRow { error_code: None, error_field: None, error_reason: None, ..refused.clone() };
    assert_eq!(ir_line_of(Some(&bare), true, Some(ACCOUNT_ID)).0, "IR: REJECTED (HTTP 400)");
    let code_only = IrSyncRow { error_field: None, ..refused };
    assert_eq!(ir_line_of(Some(&code_only), true, Some(ACCOUNT_ID)).0, "IR: REJECTED VALIDATION_ERROR");
}

#[test]
fn the_record_modal_of_a_run_on_the_server_says_so() {
    let row = |state| IrSyncRow::new(1, "s", ACCOUNT_ID, state);
    assert_eq!(ir_line_of(Some(&row(SyncState::Accepted)), true, Some(ACCOUNT_ID)), ("IR: ON SERVER".to_string(), IrLineKind::Good));
    assert_eq!(ir_line_of(Some(&row(SyncState::Retry)), true, Some(ACCOUNT_ID)), ("IR: NOT SENT YET (WILL RETRY)".to_string(), IrLineKind::Warn));
    let old_rule = IrSyncRow { error_code: Some(SKIPPED_OLD_SCRATCH_AUTO_RULE.into()), ..row(SyncState::Skipped) };
    assert_eq!(ir_line_of(Some(&old_rule), true, Some(ACCOUNT_ID)).0, "IR: NOT SENT (OLD SCRATCH AUTO RULE)");
    let no_hash = IrSyncRow { error_code: Some(SKIPPED_NO_CHART_HASH.into()), ..row(SyncState::Skipped) };
    assert_eq!(ir_line_of(Some(&no_hash), true, Some(ACCOUNT_ID)).0, "IR: NOT SENT (CHART NOT IN LIBRARY)");
    assert_eq!(ir_line_of(None, true, Some(ACCOUNT_ID)).0, "IR: NOT SENT YET");
    assert_eq!(ir_line_of(None, false, None).0, "IR: NOT SENT (NOT SIGNED IN)");
}

/// A play the server holds under another account is said to be that account's, so the signed-in
/// player does not take it for one of their own.
#[test]
fn the_record_modal_names_the_account_a_play_is_on_the_server_under() {
    let accepted = |login_id: &str| IrSyncRow::new(1, "s", login_id, SyncState::Accepted);
    assert_eq!(ir_line_of(Some(&accepted("someone-else")), true, Some(ACCOUNT_ID)), ("IR: ON SERVER (AS someone-else)".to_string(), IrLineKind::Info));
    assert_eq!(ir_line_of(Some(&accepted(ACCOUNT_ID)), true, Some(ACCOUNT_ID)).0, "IR: ON SERVER");
    assert_eq!(ir_line_of(Some(&accepted("someone-else")), false, None).0, "IR: ON SERVER", "nobody is signed in to be told apart from");
    assert_eq!(ir_line_of(Some(&accepted("")), true, Some(ACCOUNT_ID)).0, "IR: ON SERVER", "a row that names no account is nobody else's");
}

/// An app signed in to [`SERVER_URL`] as [`ACCOUNT_ID`] whose server is `server`, syncing at once.
fn signed_in(tag: &str, server: Arc<ScriptedServer>) -> AppShared {
    let folder = own_folder(tag);
    let mut config = Config::default();
    config.network.server_url = Some(SERVER_URL.into());
    config.network.player_id = ACCOUNT_ID.into();
    let mut shared = crate::App::new(String::new(), config, LaunchOptions::default(), folder.join("settings.ron")).shared;
    shared.server = server;
    shared.session = AccountSession::restored(Some(ACCOUNT_TOKEN.into()), Some(ACCOUNT_ID.into()));
    shared.record_sync_pacing = NO_PACING;
    shared
}

fn store(shared: &mut AppShared, log: &PlayLog) -> i64 {
    shared.scoredb.as_mut().expect("the score database opened in the test's folder").record_play(log, true).expect("the play is stored")
}

/// Poll the app's sync as the frame loop does until it ends.
fn poll_to_the_end(shared: &mut AppShared) {
    let deadline = Instant::now() + SYNC_WAIT;
    while shared.record_sync.is_some() {
        assert!(Instant::now() < deadline, "the records sync did not end");
        shared.poll_record_sync();
        std::thread::sleep(SYNC_POLL);
    }
}

#[test]
fn a_sync_writes_every_answer_down_and_sends_only_what_the_server_lacks_next_time() {
    let server = ScriptedServer::answering(vec![Ok(taken("sc_a")), Err(judge_refusal())]);
    let mut shared = signed_in("app-flow", Arc::clone(&server));
    let first = store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    let second = store(&mut shared, &play(0, "bb", PLAYED_SECOND + 1, [1, 0, 0, 0, 0, 0], 1));
    let old_rule = PlayLog {
        assist: SCRATCH_AUTO_ASSIST,
        rule_version: rbms_store::SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1,
        ..play(0, "cc", 1, [1, 0, 0, 0, 0, 0], 1)
    };
    let old = store(&mut shared, &old_rule);
    shared.reload_record_sync_states();

    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 2, "the SCRATCH AUTO play of the rule before is not sent");
    assert_eq!(shared.net_status, "records: 1 sent, 1 rejected");
    let states = shared.scoredb.as_ref().expect("the database").sync_states(&server_key(SERVER_URL)).expect("reads");
    assert_eq!((states[&first].state, states[&first].score_id.as_deref()), (SyncState::Accepted, Some("sc_a")));
    assert_eq!((states[&second].state, states[&second].error_field.as_deref()), (SyncState::Rejected, Some("judge")));
    assert_eq!(states[&old].error_code.as_deref(), Some(SKIPPED_OLD_SCRATCH_AUTO_RULE));
    assert_eq!(shared.record_sync_states, states, "the record screens read what was written");
    assert_eq!(shared.record_ir_line(Some(first)).map(|(line, _)| line).as_deref(), Some("IR: ON SERVER"));

    let status = shared.net_status.clone();
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 2, "a sync started on its own does not send a refusal the same builds gave again");
    assert_eq!(shared.net_status, status, "and a sync that found nothing to send says nothing");

    shared.sync_records_now();
    poll_to_the_end(&mut shared);
    let sent = server.submitted();
    assert_eq!(sent.len(), 3, "by hand the refused play is sent again, and the play the server holds is not");
    assert_eq!(sent[2].chart.id.md5, "bb");
    assert_eq!(shared.record_sync_states[&second].state, SyncState::Accepted);
    shared.sync_records_now();
    poll_to_the_end(&mut shared);
    assert_eq!((server.submitted().len(), shared.net_status.as_str()), (3, NOTHING_TO_SEND), "nothing is left to send");
}

/// A server that names its build: a refusal it gave is not sent again on its own until the server
/// or this client is another build.
#[test]
fn an_automatic_sync_does_not_resend_a_rejection_the_same_server_build_gave() {
    let server = ScriptedServer::answering(vec![Err(judge_refusal())]);
    *server.commit.lock().expect("the commit") = Some("c1".into());
    let mut shared = signed_in("refused-same-build", Arc::clone(&server));
    let refused = store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    for _ in 0..2 {
        assert!(shared.start_record_sync(SyncScope::All));
        poll_to_the_end(&mut shared);
    }
    assert_eq!(server.submitted().len(), 1, "the second sync on its own left the refusal out");
    let row = &shared.scoredb.as_ref().expect("the database").sync_states(&server_key(SERVER_URL)).expect("reads")[&refused];
    assert_eq!(row.refused_under.as_deref(), Some(refusal_key(&shared.client_build(), "c1").as_str()));

    *server.commit.lock().expect("the commit") = Some("c2".into());
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 2, "a new server build may take it, so it goes again");
    assert_eq!(shared.record_sync_states[&refused].state, SyncState::Accepted);
}

#[test]
fn a_new_client_build_resends_every_rejection() {
    let server = ScriptedServer::answering(vec![Err(judge_refusal())]);
    *server.commit.lock().expect("the commit") = Some("c1".into());
    let mut shared = signed_in("refused-new-client", Arc::clone(&server));
    store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    shared.build_sha256 = Some("first-build".into());
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    shared.build_sha256 = Some("next-build".into());
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 2, "another client build may send what the server takes");
}

#[test]
fn a_rejected_replay_is_not_uploaded_again_by_an_automatic_sync() {
    let server = ScriptedServer::answering(Vec::new());
    *server.commit.lock().expect("the commit") = Some("c1".into());
    *server.uploads_answers.lock().expect("the upload script") = vec![Err(IrError::PayloadTooLarge(String::new()))].into();
    let mut shared = signed_in("replay-refused", Arc::clone(&server));
    shared.config.network.auto_upload_replay = true;
    saved_replay(&shared.replay_dir.clone(), "aa-1.ron");
    let id = store(&mut shared, &PlayLog { replay_file: Some("aa-1.ron".into()), ..play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1) });
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.uploaded().len(), 1);
    let row = &shared.scoredb.as_ref().expect("the database").sync_states(&server_key(SERVER_URL)).expect("reads")[&id];
    assert_eq!((row.replay_state, row.http_status), (ReplayState::Rejected, None));
    assert!(row.replay_refused_under.is_some(), "the refusal is written down under the builds");

    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.uploaded().len(), 1, "a sync on its own does not upload a refused replay again");
    shared.sync_records_now();
    poll_to_the_end(&mut shared);
    assert_eq!(server.uploaded().len(), 2, "by hand it goes again");
    assert_eq!(shared.record_sync_states[&id].replay_state, ReplayState::Uploaded);
}

/// Three plays a server keeps failing on its own side stop a sync, but the next one sends the plays
/// it has not been asked about before them, so the plays after them still go up.
#[test]
fn plays_that_always_fail_do_not_hold_back_the_plays_after_them() {
    let server = ScriptedServer::answering(Vec::new());
    *server.failing.lock().expect("the failing charts") = vec!["bad".into()];
    let mut shared = signed_in("always-failing", Arc::clone(&server));
    for second in 0..3 {
        store(&mut shared, &play(0, "bad", PLAYED_SECOND + second, [1, 0, 0, 0, 0, 0], 1));
    }
    let good = store(&mut shared, &play(0, "good", PLAYED_SECOND + 10, [1, 0, 0, 0, 0, 0], 1));
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(shared.net_status, "records sync stopped: server unreachable");
    assert!(!shared.record_sync_states.contains_key(&good), "the first sync stopped before the good play");

    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(shared.record_sync_states[&good].state, SyncState::Accepted, "the good play went before the failing ones");
}

/// The frame thread picks nothing: a full sync is started before the worker has even asked the
/// server its build, and a play stored meanwhile is left to whoever stored it.
#[test]
fn starting_a_full_sync_picks_nothing_on_the_frame_thread() {
    let server = ScriptedServer::answering(Vec::new());
    let (release, gate) = std::sync::mpsc::channel();
    *server.version_gate.lock().expect("the version gate") = Some(gate);
    let mut shared = signed_in("picked-elsewhere", Arc::clone(&server));
    for second in 0..3 {
        store(&mut shared, &play(0, "aa", PLAYED_SECOND + second, [1, 0, 0, 0, 0, 0], 1));
    }
    assert!(shared.start_record_sync(SyncScope::All), "the sync starts without waiting on anything");
    shared.poll_record_sync();
    assert!(shared.record_sync.as_ref().is_some_and(|run| run.scores_total == 0), "nothing was picked yet");
    let later = store(&mut shared, &play(0, "zz", PLAYED_SECOND + 10, [1, 0, 0, 0, 0, 0], 1));
    drop(release);
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().iter().map(|sub| sub.chart.id.md5.as_str()).collect::<Vec<_>>(), ["aa", "aa", "aa"]);
    assert!(!shared.record_sync_states.contains_key(&later), "the play stored after the sync began is not in it");
}

#[test]
fn sync_records_now_while_signed_out_says_log_in_first() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("signed-out", Arc::clone(&server));
    store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    shared.session = AccountSession::restored(None, None);
    shared.sync_records_now();
    assert_eq!(shared.net_status, "log in first");
    assert!(shared.record_sync.is_none());
    assert_eq!(shared.record_ir_line(None).map(|(line, _)| line).as_deref(), Some("IR: NOT SENT (NOT SIGNED IN)"));
}

#[test]
fn a_guest_never_starts_the_records_sync() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("guest", Arc::clone(&server));
    store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    shared.session = AccountSession::restored(None, None);
    assert!(!shared.start_record_sync(SyncScope::All));
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Ok(ACCOUNT_ID.to_string())).expect("the probe receiver is open");
    shared.whoami_rx = Some(rx);
    shared.poll_network();
    assert!(shared.record_sync.is_none() && server.submitted().is_empty(), "a guest's plays wait for an account");
}

#[test]
fn a_confirmed_session_at_start_starts_the_records_sync() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("whoami", Arc::clone(&server));
    store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Err(IrError::Network("offline".into()))).expect("the probe receiver is open");
    shared.whoami_rx = Some(rx);
    shared.poll_network();
    assert!(shared.record_sync.is_none(), "a session that could not be confirmed sends nothing");

    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Ok(ACCOUNT_ID.to_string())).expect("the probe receiver is open");
    shared.whoami_rx = Some(rx);
    shared.poll_network();
    assert!(shared.record_sync.is_some(), "a confirmed session starts the sync");
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 1);
}

#[test]
fn a_successful_login_starts_the_records_sync() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("login", Arc::clone(&server));
    store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    let (tx, rx) = std::sync::mpsc::channel();
    let response = rbms_ir::AuthResponse { token: ACCOUNT_TOKEN.into(), player: PlayerId { id: ACCOUNT_ID.into() }, name: ACCOUNT_ID.into() };
    tx.send(Ok(response)).expect("the auth receiver is open");
    shared.auth_rx = Some(crate::app_network::AuthInFlight::new(crate::ir_session::AuthAction::Login, rx, |_| {}));
    shared.poll_network();
    assert!(shared.record_sync.is_some(), "signing in starts the sync");
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 1);
}

#[test]
fn logging_out_cancels_a_running_sync() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("logout", Arc::clone(&server));
    shared.record_sync_pacing = SyncPacing { score_interval: SYNC_WAIT, ..NO_PACING };
    for second in 0..3 {
        store(&mut shared, &play(0, "aa", PLAYED_SECOND + second, [1, 0, 0, 0, 0, 0], 1));
    }
    assert!(shared.start_record_sync(SyncScope::All));
    let cancel = Arc::clone(&shared.record_sync.as_ref().expect("a sync runs").cancel);
    shared.finish_logout();
    assert!(shared.record_sync.is_none(), "the sync is let go with the session");
    assert!(cancel.load(Ordering::Relaxed), "the worker is told to stop");
}

#[test]
fn a_named_play_waits_for_the_running_sync_and_follows_it() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("pending", Arc::clone(&server));
    let first = store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    assert!(shared.start_record_sync(SyncScope::Logs(vec![first])));
    let later = store(&mut shared, &play(0, "bb", PLAYED_SECOND + 1, [1, 0, 0, 0, 0, 0], 1));
    assert!(!shared.start_record_sync(SyncScope::Logs(vec![later])), "a second sync waits");
    assert_eq!(shared.record_sync_pending, [later]);
    poll_to_the_end(&mut shared);
    assert!(shared.record_sync_pending.is_empty());
    assert_eq!(server.submitted().iter().map(|sub| sub.chart.id.md5.clone()).collect::<Vec<_>>(), ["aa", "bb"]);
}

#[test]
fn the_sync_row_shows_its_progress_while_it_runs() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("progress", Arc::clone(&server));
    assert_eq!(shared.network_setting_line(rbms_config::SettingId::SyncRecords).map(|(_, value)| value).as_deref(), Some(rbms_config::ACTION_VALUE));
    shared.record_sync_pacing = SyncPacing { score_interval: SYNC_WAIT, ..NO_PACING };
    for second in 0..2 {
        store(&mut shared, &play(0, "aa", PLAYED_SECOND + second, [1, 0, 0, 0, 0, 0], 1));
    }
    assert!(shared.start_record_sync(SyncScope::All));
    let deadline = Instant::now() + SYNC_WAIT;
    while shared.record_sync.as_ref().is_some_and(|run| run.scores_total == 0) {
        assert!(Instant::now() < deadline, "the sync did not pick its plays");
        shared.poll_record_sync();
        std::thread::sleep(SYNC_POLL);
    }
    let value = shared.network_setting_line(rbms_config::SettingId::SyncRecords).map(|(_, value)| value).expect("the row has a value");
    assert!(value.starts_with("SENDING ") && value.ends_with("/2"), "{value}");
    shared.cancel_record_sync();
}

/// The account a sync sends under is the token's owner as the server named it. A PLAYER ID typed
/// into the settings is no stand-in: sent under it, every play is refused as another player's.
#[test]
fn a_typed_player_id_is_never_the_account_a_sync_sends_under() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("unconfirmed-account", Arc::clone(&server));
    store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    shared.session = AccountSession::restored(Some(ACCOUNT_TOKEN.into()), None);
    shared.config.network.player_id = "typed-by-hand".into();
    assert_eq!(shared.record_sync_target(), None);
    assert!(!shared.start_record_sync(SyncScope::All));
    shared.sync_records_now();
    assert_eq!(shared.net_status, ACCOUNT_NOT_CONFIRMED);
    assert!(shared.record_sync.is_none() && server.submitted().is_empty());

    shared.session = AccountSession::restored(Some(ACCOUNT_TOKEN.into()), Some(ACCOUNT_ID.into()));
    assert_eq!(shared.record_sync_target(), Some((server_key(SERVER_URL), ACCOUNT_ID.to_string())));
}

/// A play a result screen's submission is still carrying writes its own answer, so a sync that
/// starts meanwhile leaves it out and picks it up once that answer is down.
#[test]
fn a_play_in_a_live_submission_is_left_out_of_the_walk() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("live-submit", Arc::clone(&server));
    let live = store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    let other = store(&mut shared, &play(0, "bb", PLAYED_SECOND + 1, [1, 0, 0, 0, 0, 0], 1));
    shared.submit_log = Some(crate::app_ranking::LiveSubmit {
        log_id: live,
        server: server_key(SERVER_URL),
        login_id: ACCOUNT_ID.into(),
        replay_plan: ReplayState::None,
        trace_kept: false,
    });

    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().iter().map(|sub| sub.chart.id.md5.clone()).collect::<Vec<_>>(), ["bb"], "only the other play was sent");
    assert_eq!(shared.record_sync_states.keys().copied().collect::<Vec<_>>(), [other]);

    shared.submit_log = None;
    assert!(shared.start_record_sync(SyncScope::All), "the play follows once its submission is over");
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 2);
    assert_eq!(shared.record_sync_states[&live].state, SyncState::Accepted);
}

/// A sync of named plays does not walk the whole history: the answers the record screens read are
/// kept current as they are written, so they are not read again.
#[test]
fn a_sync_of_named_plays_leaves_the_states_the_screens_read_as_they_are() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("named-walk", Arc::clone(&server));
    let old_rule = PlayLog {
        assist: SCRATCH_AUTO_ASSIST,
        rule_version: rbms_store::SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1,
        ..play(0, "cc", 1, [1, 0, 0, 0, 0, 0], 1)
    };
    let old = store(&mut shared, &old_rule);
    let named = store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));

    assert!(shared.start_record_sync(SyncScope::Logs(vec![named])));
    poll_to_the_end(&mut shared);
    assert_eq!(shared.record_sync_states.keys().copied().collect::<Vec<_>>(), [named], "the held-back play was not marked by a walk of one play");

    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 1, "nothing else is left to send");
    shared.reload_record_sync_states();
    assert_eq!(shared.record_sync_states[&old].error_code.as_deref(), Some(SKIPPED_OLD_SCRATCH_AUTO_RULE), "reading the states names why it is held back");
}

/// A trace as a play keeps it: `notes` judged objects of the chart `md5`, every one a PGREAT.
fn trace_body(md5: &str, notes: usize) -> String {
    let trace = PlayTrace {
        format: rbms_ir::TRACE_FORMAT.into(),
        version: rbms_ir::TRACE_FORMAT_VERSION,
        score_id: None,
        chart: ChartId { md5: md5.into(), sha256: format!("sha-of-{md5}") },
        mode: Mode::BEAT_7K.name.into(),
        lntype: 0,
        random: rbms_ir::RandomOption::Off,
        auto_lanes: Vec::new(),
        notes: rbms_ir::TraceNotes {
            time_us: (0..notes as i64).collect(),
            lane: vec![1; notes],
            kind: vec![rbms_ir::TRACE_KIND_NORMAL; notes],
            state: vec![1; notes],
            err_us: vec![0; notes],
        },
        empty_poors: rbms_ir::TraceEmptyPoors::default(),
        gauge: rbms_ir::TraceGauge::default(),
        gauge_bounds: Vec::new(),
        failed_at_ms: None,
        play_time_ms: 0,
        speed: rbms_ir::TraceSpeeds::default(),
        tempo: Vec::new(),
    };
    trace.to_json().expect("the trace serialises")
}

/// Store `log` with the trace [`trace_body`] makes of it.
fn store_traced(shared: &mut AppShared, log: &PlayLog, notes: usize) -> i64 {
    let trace = trace_body(&log.md5, notes);
    let db = shared.scoredb.as_mut().expect("the score database opened in the test's folder");
    let id = db.record_play_kept(log, true, None).expect("stored");
    db.keep_trace(id, &trace).expect("its trace kept");
    id
}

fn sync_row_of(shared: &AppShared, log_id: i64) -> IrSyncRow {
    shared.scoredb.as_ref().expect("the database").sync_states(&server_key(SERVER_URL)).expect("reads").remove(&log_id).expect("the play's row")
}

/// How many judged objects a trace has to hold to be larger than the server takes.
const TRACE_NOTES_PAST_THE_CAP: usize = rbms_ir::TRACE_MAX_BYTES / 8;

#[test]
fn a_play_the_server_takes_sends_its_trace_linked_to_the_score() {
    let server = ScriptedServer::answering(vec![Ok(taken("sc_linked"))]).taking_traces();
    let mut shared = signed_in("trace-linked", Arc::clone(&server));
    let traced = store_traced(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1), 1);
    let plain = store(&mut shared, &play(0, "bb", PLAYED_SECOND + 1, [1, 0, 0, 0, 0, 0], 1));
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    let sent = server.traced();
    assert_eq!(sent.len(), 1, "only the play that kept a trace sends one");
    assert_eq!((sent[0].0.md5.as_str(), sent[0].1.score_id.as_deref()), ("aa", Some("sc_linked")));
    let row = sync_row_of(&shared, traced);
    assert_eq!((row.trace_state, row.trace_id.as_deref()), (ReplayState::Uploaded, Some("tr_1")));
    assert_eq!(sync_row_of(&shared, plain).trace_state, ReplayState::None);
    assert_eq!(shared.net_status, "records: 2 sent; traces: 1 uploaded");
    assert_eq!(shared.record_sync_states[&traced].trace_state, ReplayState::Uploaded, "the record screens read what was written");

    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.traced().len(), 1, "a trace the server holds is not sent again");
}

/// An older server says nothing of traces: none is sent to it, every one stays to be sent, and a
/// sync after the server starts taking them sends them for the plays it took before.
#[test]
fn a_server_that_takes_no_traces_is_sent_none_until_it_does() {
    let server = ScriptedServer::answering(Vec::new());
    let mut shared = signed_in("trace-old-server", Arc::clone(&server));
    let traced = store_traced(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1), 1);
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 1);
    assert!(server.traced().is_empty(), "a server that does not take traces is sent none");
    assert_eq!(sync_row_of(&shared, traced).trace_state, ReplayState::Pending);

    *server.takes_traces.lock().expect("the capability") = true;
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 1, "the score is not sent again");
    assert_eq!(server.traced().iter().map(|(_, trace)| trace.score_id.clone()).collect::<Vec<_>>(), [Some("sc_1".to_string())]);
    assert_eq!(sync_row_of(&shared, traced).trace_state, ReplayState::Uploaded);
}

#[test]
fn auto_upload_trace_off_keeps_every_trace_on_the_machine_to_send_later() {
    let server = ScriptedServer::answering(Vec::new()).taking_traces();
    let mut shared = signed_in("trace-off", Arc::clone(&server));
    shared.config.network.auto_upload_trace = false;
    let traced = store_traced(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1), 1);
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert!(server.traced().is_empty());
    assert_eq!(sync_row_of(&shared, traced).trace_state, ReplayState::Pending, "switched off, the trace waits");
    assert!(shared.scoredb.as_ref().expect("the database").trace_body(traced).expect("reads").is_some(), "and stays on the machine");

    shared.config.network.auto_upload_trace = true;
    shared.sync_records_now();
    poll_to_the_end(&mut shared);
    assert_eq!(server.traced().len(), 1);
}

/// The server refusing a trace (it does not add up to the score, say) is written down under the
/// builds, as a refused replay is: a sync on its own leaves it out, by hand it goes again.
#[test]
fn a_refused_trace_is_not_sent_again_by_an_automatic_sync() {
    let server = ScriptedServer::answering(Vec::new()).taking_traces();
    let mismatch = r#"{"success":false,"error":{"code":"IR_TRACE_MISMATCH","message":"m","details":{"field":"notes.state"}}}"#;
    *server.traces_answers.lock().expect("the trace script") = vec![Err(IrError::Server(400, mismatch.into()))].into();
    let mut shared = signed_in("trace-refused", Arc::clone(&server));
    let traced = store_traced(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1), 1);
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    let row = sync_row_of(&shared, traced);
    assert_eq!(row.trace_state, ReplayState::Rejected);
    assert!(row.trace_refused_under.is_some() && row.trace_error.as_deref().is_some_and(|error| error.contains("IR_TRACE_MISMATCH")), "{row:?}");
    assert!(shared.net_status.contains("traces: 1 failed"), "{}", shared.net_status);

    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.traced().len(), 1, "a sync on its own does not send a refused trace again");
    shared.sync_records_now();
    poll_to_the_end(&mut shared);
    assert_eq!(server.traced().len(), 2, "by hand it goes again");
    assert_eq!(sync_row_of(&shared, traced).trace_state, ReplayState::Uploaded);
}

#[test]
fn a_trace_larger_than_the_server_takes_is_kept_and_not_sent() {
    let server = ScriptedServer::answering(Vec::new()).taking_traces();
    let mut shared = signed_in("trace-too-large", Arc::clone(&server));
    let traced = store_traced(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1), TRACE_NOTES_PAST_THE_CAP);
    assert!(trace_body("aa", TRACE_NOTES_PAST_THE_CAP).len() > rbms_ir::TRACE_MAX_BYTES, "the fixture is past the cap");
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert!(server.traced().is_empty());
    let row = sync_row_of(&shared, traced);
    assert_eq!((row.trace_state, row.trace_error.as_deref()), (ReplayState::Skipped, Some(TRACE_TOO_LARGE)));
    assert!(shared.scoredb.as_ref().expect("the database").trace_body(traced).expect("reads").is_some(), "the run keeps its trace");
}

/// The result screen sends the score and the replay; once the server took the score, the trace of
/// that run goes up by a sync of that one run.
#[test]
fn a_run_the_result_screen_sent_has_its_trace_sent_by_a_sync_of_that_run() {
    let server = ScriptedServer::answering(Vec::new()).taking_traces();
    let mut shared = signed_in("trace-live", Arc::clone(&server));
    let live = store_traced(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1), 1);
    let other = store_traced(&mut shared, &play(0, "bb", PLAYED_SECOND + 1, [1, 0, 0, 0, 0, 0], 1), 1);
    let submit = crate::app_ranking::LiveSubmit {
        log_id: live,
        server: server_key(SERVER_URL),
        login_id: ACCOUNT_ID.into(),
        replay_plan: ReplayState::None,
        trace_kept: true,
    };
    shared.write_live_outcome(&submit, &rbms_ir::SubmitOutcome { submit: Ok(taken("sc_live")), replay: None });
    assert!(shared.record_sync.is_some(), "a sync of the run was started for its trace");
    let status = shared.net_status.clone();
    poll_to_the_end(&mut shared);
    assert_eq!(shared.net_status, status, "the trace of a run the result screen reported on goes up without a word");
    assert!(server.submitted().is_empty(), "the score the server took is not sent again, and no other play is");
    assert_eq!(server.traced().iter().map(|(_, trace)| trace.score_id.clone()).collect::<Vec<_>>(), [Some("sc_live".to_string())]);
    assert_eq!(sync_row_of(&shared, live).trace_state, ReplayState::Uploaded);
    assert!(!shared.record_sync_states.contains_key(&other), "the other play waits for a sync of its own");
}

/// A sync with no trace to send does not ask the server whether it takes traces: the question
/// would only hold up the scores. One with a trace of a play it is to send asks once.
#[test]
fn the_server_is_asked_whether_it_takes_traces_only_when_there_is_one_to_send() {
    let server = ScriptedServer::answering(Vec::new()).taking_traces();
    let mut shared = signed_in("trace-health", Arc::clone(&server));
    store(&mut shared, &play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1));
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(server.submitted().len(), 1);
    assert_eq!(*server.health_asked.lock().expect("the health count"), 0, "no trace, no question");

    store_traced(&mut shared, &play(0, "bb", PLAYED_SECOND + 1, [1, 0, 0, 0, 0, 0], 1), 1);
    assert!(shared.start_record_sync(SyncScope::All));
    poll_to_the_end(&mut shared);
    assert_eq!(*server.health_asked.lock().expect("the health count"), 1, "a play to send with a trace asks once");
    assert_eq!(server.traced().len(), 1);
}

/// A trace the database cannot be read for just then is one to send on a later sync, not one gone
/// for good; one whose row is gone is missing.
#[test]
fn a_trace_the_database_could_not_read_is_tried_again_and_a_gone_one_is_missing() {
    let candidate = |id| SyncCandidate {
        log: PlayLog { id, ..play(0, "aa", PLAYED_SECOND, [1, 0, 0, 0, 0, 0], 1) },
        body: None,
        played_as: None,
        state: Some(SyncState::Accepted),
        score_id: Some("sc_1".into()),
        replay_state: None,
        has_trace: true,
        trace_state: Some(ReplayState::Pending),
    };
    let unreadable = ScoreDb::open_in_memory().expect("a database with no tables");
    let (state, reason) = trace_upload_of(&unreadable, &candidate(1)).expect_err("nothing reads");
    assert_eq!(state, ReplayState::Failed, "a read that failed is tried again");
    assert!(reason.as_deref().is_some_and(|reason| reason.starts_with(TRACE_NOT_READ)), "{reason:?}");
    assert!(state.still_to_send());

    let mut db = ScoreDb::open_in_memory().expect("opens");
    db.migrate().expect("the schema applies");
    assert_eq!(trace_upload_of(&db, &candidate(1)).expect_err("no row"), (ReplayState::Missing, None));
}
