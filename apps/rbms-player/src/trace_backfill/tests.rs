//! Traces made from the replays of runs stored without one, checked on runs that are really
//! played: a chart file is written, played by hand through the engine, ended the way the play screen
//! ends a run, and the trace the live path kept is what the replay is judged again to.

use std::sync::Mutex;

use rbms_ir::{ChartId, CourseSubmission, IrError, PlayTrace, PlayerId, PlayerProfile, ReplayData, ServerCapabilities, ServerInfo, SubmitResponse, TraceGauge};
use rbms_judge::GaugeKind;
use rbms_store::scoredb::{BackfillBatch, KeptSubmission, SyncWalk, TRACE_BACKFILL_NOT_STARTED};

use super::*;
use crate::app_result::record_tests::HAND_OFFSETS_US;
use crate::ir_session::AccountSession;
use crate::stage::{PlayState, StageHandler, Transition};
use crate::{App, LaunchOptions};

/// A seven-key chart of three measures of eighth notes, each on a key of its own, with one
/// turntable note among them.
const CHART: &[u8] = b"#PLAYER 1\n#TITLE Backfill\n#BPM 120\n#RANK 2\n#TOTAL 300\n#WAV01 a.wav\n\
#00111:0101010101010101\n#00212:0101010101010101\n#00216:01000000\n#00313:0101010101010101\n";

/// The file name the chart is written as.
const CHART_FILE: &str = "backfill.bms";

/// How long a key stays down, and the step the engine is run at.
const KEY_HELD_US: i64 = 50_000;
const RUN_FRAME_US: i64 = 10_000;

/// How far into the chart a run left by Escape is played: into its second measure.
const LEFT_AT_US: i64 = 3_000_000;

/// Where the run that stops pressing stops: before the second measure's first notes.
const LAST_PRESS_BEFORE_US: i64 = 2_600_000;

/// The state a trace gives a note let go by.
const POOR_STATE: u8 = 5;

/// One note pressed in five, which empties a HARD gauge.
const FAILING_OFFSETS_US: [Option<i64>; 5] = [Some(0), None, None, None, None];

/// No waiting at all.
const UNPACED: BackfillPacing = BackfillPacing { start_delay: Duration::ZERO, between_runs: Duration::ZERO, check: Duration::from_millis(1) };

/// How long a test waits for the thread, or for a records sync, to end.
const WORK_WAIT: Duration = Duration::from_secs(20);

/// The score server the signed-in app sends to, and the account.
const SERVER_URL: &str = "http://score-server.invalid";
const ACCOUNT_TOKEN: &str = "token-of-a-test";
const ACCOUNT_ID: &str = "tester";

/// A folder of the test's own, emptied of what an earlier run of the tests left.
fn own_folder(tag: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("rbms-trace-backfill-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder is writable");
    folder
}

/// An app keeping its settings in `folder`, nobody signed in, on `gauge`.
fn app_in(folder: &Path, gauge: GaugeKind) -> App {
    let mut config = Config::default();
    config.play.autoplay = false;
    config.play.gauge = gauge;
    let mut app = App::new(String::new(), config, LaunchOptions::default(), folder.join("settings.ron"));
    app.shared.replay = None;
    app
}

/// The chart written into `folder`, and its path.
fn chart_in(folder: &Path) -> String {
    let path = folder.join(CHART_FILE);
    std::fs::write(&path, CHART).expect("the chart file is writable");
    path.to_string_lossy().into_owned()
}

/// The chart as the engine plays it.
fn model() -> rbms_model::Model {
    let source = rbms_parser::parse_with(CHART, Default::default());
    rbms_chart::to_model(&source, rbms_chart::detect_mode(&source, CHART_FILE))
}

/// Every key note pressed as far from its time as `offsets_us` says for its place among them, the
/// turntable note on its time.
fn inputs(model: &rbms_model::Model, offsets_us: &[Option<i64>]) -> Vec<(i64, usize, bool)> {
    let notes =
        model.timelines.iter().flat_map(|line| line.notes.iter().enumerate().filter(|(_, note)| note.is_some()).map(move |(lane, _)| (line.time_us, lane)));
    let mut key_notes = 0;
    let mut inputs = Vec::new();
    for (time_us, lane) in notes {
        let offset_us = if model.mode.is_scratch(lane) {
            Some(0)
        } else {
            key_notes += 1;
            offsets_us[(key_notes - 1) % offsets_us.len()]
        };
        if let Some(offset_us) = offset_us {
            inputs.extend([(time_us + offset_us, lane, true), (time_us + offset_us + KEY_HELD_US, lane, false)]);
        }
    }
    inputs.sort_unstable();
    inputs
}

/// A run of the chart on `gauge`, with `inputs` handed to the engine as their times come, a frame
/// at a time up to `until_us`.
fn played(gauge: GaugeKind, inputs: &[(i64, usize, bool)], until_us: i64) -> PlaySession {
    driven(PlaySession::new(model(), SessionOptions { gauge, ..SessionOptions::default() }), inputs, until_us)
}

/// `session` with `inputs` handed to the engine as their times come, a frame at a time up to
/// `until_us`.
fn driven(mut session: PlaySession, inputs: &[(i64, usize, bool)], until_us: i64) -> PlaySession {
    let mut next = 0;
    for frame in 0..=until_us / RUN_FRAME_US {
        let now_us = frame * RUN_FRAME_US;
        while let Some((at_us, lane, press)) = inputs.get(next).copied().filter(|(at_us, _, _)| *at_us <= now_us) {
            if press {
                session.press(lane, at_us, &mut NullSink);
            } else {
                session.release(lane, at_us);
            }
            next += 1;
        }
        session.tick(SessionClock::at(now_us), &mut NullSink);
    }
    session
}

/// A run of the chart on `gauge` played to its end as the play screen plays one: a frame at a time
/// until the engine says it is over.
fn played_to_its_end(gauge: GaugeKind, inputs: &[(i64, usize, bool)]) -> PlaySession {
    to_its_end(PlaySession::new(model(), SessionOptions { gauge, ..SessionOptions::default() }), inputs)
}

/// `session` driven by `inputs` as the play screen drives a run: a frame at a time until the engine
/// says it is over.
fn to_its_end(session: PlaySession, inputs: &[(i64, usize, bool)]) -> PlaySession {
    let end_us = (0..).map(|frame| frame * RUN_FRAME_US).find(|now_us| session.is_finished(*now_us)).expect("the chart ends");
    driven(session, inputs, end_us)
}

/// The screen that plays `session` of the chart at `path`.
fn playing(shared: &mut AppShared, session: PlaySession, path: &str) -> PlayState {
    shared.mode = session.model().mode;
    shared.chart_path = path.to_owned();
    PlayState::new(session, std::collections::HashMap::new(), 0, rbms_store::SCORE_LN_MODE_FROM_CHART.to_owned())
}

/// End `session` as the result screen does.
fn ended(shared: &mut AppShared, session: PlaySession, path: &str) {
    let mut state = playing(shared, session, path);
    let entered = crate::app_result::enter_result(&mut state, shared);
    assert!(matches!(entered, Transition::To(crate::stage::Stage::Result(_))), "the run did not reach its result screen");
}

/// Leave `session` by Escape, as the play screen does on the ESCAPE setting a fresh install has.
fn left(shared: &mut AppShared, session: PlaySession, path: &str) {
    let mut state = playing(shared, session, path);
    let key = crate::stage::KeyInput { code: winit::keyboard::KeyCode::Escape, pressed: true, released: false, text: None };
    state.handle_key(&mut crate::stage::FrameCtx { shared, now: Instant::now(), dt: 0.0 }, key);
}

/// The one run `shared` stored, with the trace and the body it was kept with.
fn the_run(shared: &AppShared) -> (UntracedPlay, PlayTrace) {
    let db = shared.scoredb.as_ref().expect("the score database");
    let mut logs = db.all_plays().expect("the history reads");
    assert_eq!(logs.len(), 1, "one run was stored");
    let log = logs.remove(0);
    let walk = SyncWalk { server: SERVER_URL, login_id: ACCOUNT_ID, ..SyncWalk::default() };
    let body = db.sync_candidates(&walk).expect("reads").into_iter().find(|candidate| candidate.log.id == log.id).and_then(|candidate| candidate.body);
    let trace = db.trace_body(log.id).expect("reads").and_then(|body| PlayTrace::from_json(&body)).expect("the run kept its trace");
    (UntracedPlay { log, body }, trace)
}

/// `made`, a trace made from a replay, is `live`, the trace the run kept, but for the scroll
/// settings, which it knows only as they stood at the end, and, for a run left before its end, the
/// gauge history after the run came to its row, which the replay does not reach.
fn is_the_live_trace(made: &str, live: &PlayTrace, left_before_its_end: bool) {
    let made = PlayTrace::from_json(made).expect("the trace made reads");
    assert_eq!(made.speed.start, made.speed.end, "the scroll settings start on how they ended");
    assert!(made.speed.changes.is_empty());
    assert_eq!((made.speed.end.hispeed, made.speed.end.green_ms), (live.speed.end.hispeed, live.speed.end.green_ms));
    if !left_before_its_end {
        assert_eq!(PlayTrace { speed: live.speed.clone(), ..made }, *live);
        return;
    }
    let shown = made.gauge.series.iter().map(Vec::len).min().unwrap_or_default();
    let as_far = |series: &[Vec<f32>]| series.iter().map(|one| one[..shown.min(one.len())].to_vec()).collect::<Vec<_>>();
    assert_eq!(as_far(&made.gauge.series), made.gauge.series, "every gauge is as long as the others");
    assert!(live.gauge.series.iter().all(|one| one.len() >= shown), "the run judged again stops no later than the run did");
    let gauge = TraceGauge { series: as_far(&live.gauge.series), ..live.gauge.clone() };
    assert_eq!(PlayTrace { speed: live.speed.clone(), ..made }, PlayTrace { gauge, ..live.clone() });
}

#[test]
fn a_finished_run_judged_again_makes_the_trace_it_kept() {
    let folder = own_folder("finished");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Normal);
    let session = played_to_its_end(GaugeKind::Normal, &inputs(&model(), &HAND_OFFSETS_US));
    assert!(session.all_notes_resolved());
    ended(&mut app.shared, session, &path);
    let (play, live) = the_run(&app.shared);
    assert!(live.notes.state.iter().all(|state| *state > 0), "a finished run reached every note");
    is_the_live_trace(&made_for(&app.shared, &play).expect("the run is judged again to its row"), &live, false);
}

#[test]
fn a_run_left_by_escape_judged_again_makes_the_trace_it_kept() {
    let folder = own_folder("left");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Normal);
    let session = played(GaugeKind::Normal, &inputs(&model(), &HAND_OFFSETS_US), LEFT_AT_US);
    let summary = session.summary();
    assert!(summary.total_judged > 0 && summary.total_judged < summary.total_notes, "the run is left part-way: {summary:?}");
    left(&mut app.shared, session, &path);
    let (play, live) = the_run(&app.shared);
    assert_eq!(play.log.clear, clear_type_id(ClearType::Failed));
    assert!(live.notes.state.contains(&0) && live.failed_at_ms.is_none(), "the notes not reached are at state 0");
    is_the_live_trace(&made_for(&app.shared, &play).expect("the run is judged again to its row"), &live, true);
}

/// A run left after the player stopped pressing: the notes that went by unhit between the last
/// input and Escape are POORs in the trace the run kept, and in the one made from its replay, which
/// is judged on past its last input until it comes to its row.
#[test]
fn a_run_left_after_notes_went_by_is_judged_on_to_where_it_was_left() {
    let folder = own_folder("left-after-misses");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Normal);
    let pressed: Vec<_> = inputs(&model(), &HAND_OFFSETS_US).into_iter().filter(|(at_us, _, _)| *at_us < LAST_PRESS_BEFORE_US).collect();
    let session = played(GaugeKind::Normal, &pressed, LEFT_AT_US);
    left(&mut app.shared, session, &path);
    let (play, live) = the_run(&app.shared);
    let last_pressed = pressed.iter().map(|(at_us, _, _)| *at_us).max().expect("some notes were pressed");
    let went_by = live.notes.time_us.iter().zip(&live.notes.state).filter(|(time_us, state)| **time_us > last_pressed && **state == POOR_STATE).count();
    assert!(went_by > 0, "notes went by unhit after the last input");
    is_the_live_trace(&made_for(&app.shared, &play).expect("the run is judged again to its row"), &live, true);
}

#[test]
fn a_run_the_gauge_ended_judged_again_makes_the_trace_it_kept() {
    let folder = own_folder("failed");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Hard);
    let session = played_to_its_end(GaugeKind::Hard, &inputs(&model(), &FAILING_OFFSETS_US));
    assert!(session.summary().failed, "the HARD gauge emptied");
    ended(&mut app.shared, session, &path);
    let (play, live) = the_run(&app.shared);
    assert!(live.failed_at_ms.is_some(), "the trace says where the gauge emptied");
    is_the_live_trace(&made_for(&app.shared, &play).expect("the run is judged again to its row"), &live, false);
}

/// A run judged again that is not the run its row is of, or that cannot be judged again, gets no
/// trace, whichever way it is not.
#[test]
fn a_run_that_is_not_its_row_or_cannot_be_judged_again_gets_no_trace() {
    let folder = own_folder("refused");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Normal);
    ended(&mut app.shared, played_to_its_end(GaugeKind::Normal, &inputs(&model(), &HAND_OFFSETS_US)), &path);
    let (play, _) = the_run(&app.shared);
    let replay_dir = app.shared.replay_dir.clone();
    assert!(made_for(&app.shared, &play).is_ok(), "the run as it was stored gets one");

    let mut judge = play.log.judge;
    judge.late[rbms_store::scoredb::JUDGE_POOR] += 1;
    let other_counts = UntracedPlay { log: PlayLog { judge, ..play.log.clone() }, ..play.clone() };
    assert!(matches!(made_for(&app.shared, &other_counts), Err(Refusal::Never(_))), "another POOR count");
    let other_gauge = UntracedPlay { log: PlayLog { gauge_value: play.log.gauge_value + 1.0, ..play.log.clone() }, ..play.clone() };
    assert!(matches!(made_for(&app.shared, &other_gauge), Err(Refusal::Never(_))), "another gauge");
    let mut sub = ScoreSubmission::from_json(play.body.as_deref().expect("a body")).expect("the body reads");
    sub.judge.fast += 1;
    let other_fast = UntracedPlay { body: sub.to_json(), ..play.clone() };
    assert!(matches!(made_for(&app.shared, &other_fast), Err(Refusal::Never(_))), "another FAST");
    assert!(matches!(made_for(&app.shared, &UntracedPlay { body: None, ..play.clone() }), Err(Refusal::Never(_))), "no submission kept");
    let no_replay = UntracedPlay { log: PlayLog { replay_file: Some("missing.ron".to_owned()), ..play.log.clone() }, ..play.clone() };
    assert!(matches!(made_for(&app.shared, &no_replay), Err(Refusal::Later(_))), "no replay, for now");

    let file = play.log.replay_file.clone().expect("the run kept its replay");
    let mut framed = Replay::load(&replay_dir.join(&file)).expect("the replay reads");
    framed.judge.clock = rbms_store::REPLAY_LEGACY_CLOCK.to_owned();
    std::fs::write(replay_dir.join("framed.ron"), framed.to_ron().expect("the replay writes")).expect("the replay folder is writable");
    let on_frames = UntracedPlay { log: PlayLog { replay_file: Some("framed.ron".to_owned()), ..play.log.clone() }, ..play.clone() };
    assert!(matches!(made_for(&app.shared, &on_frames), Err(Refusal::Never(_))), "a replay judged frame by frame");

    std::fs::write(&path, [CHART, b"#00411:01\n".as_slice()].concat()).expect("the chart file is writable");
    assert!(matches!(made_for(&app.shared, &play), Err(Refusal::Later(_))), "a chart file changed since the run, for now");
    std::fs::remove_file(&path).expect("the chart file is removable");
    assert!(matches!(made_for(&app.shared, &play), Err(Refusal::Later(_))), "no chart file, for now");
}

/// A score server that takes every score and every trace.
#[derive(Default)]
struct TakingServer {
    traced: Mutex<Vec<PlayTrace>>,
}

impl ScoreServer for TakingServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        let capabilities = ServerCapabilities { traces: true, ..ServerCapabilities::default() };
        Ok(ServerInfo { name: "rbms-ir".into(), version: "0.1.0".into(), ir_compat: "superset-1".into(), capabilities })
    }
    fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        Ok(SubmitResponse { accepted: true, score_id: Some("sc_backfill".into()), ..SubmitResponse::default() })
    }
    fn upload_trace(&self, _chart: &ChartId, trace: &PlayTrace) -> Result<String, IrError> {
        self.traced.lock().expect("the trace log").push(trace.clone());
        Ok("tr_backfill".into())
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
    fn submit_course(&self, _sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::Unsupported)
    }
    fn upload_replay(&self, _chart: &ChartId, _replay: &ReplayData) -> Result<String, IrError> {
        Err(IrError::Unsupported)
    }
}

/// An app of `folder` signed in to `server`, sending traces when `upload_traces`.
fn signed_in(folder: &Path, server: Arc<TakingServer>, upload_traces: bool) -> AppShared {
    let mut config = Config::default();
    config.network.server_url = Some(SERVER_URL.into());
    config.network.player_id = ACCOUNT_ID.into();
    config.network.auto_upload_trace = upload_traces;
    let mut shared = App::new(String::new(), config, LaunchOptions::default(), folder.join("settings.ron")).shared;
    shared.server = server;
    shared.session = AccountSession::restored(Some(ACCOUNT_TOKEN.into()), Some(ACCOUNT_ID.into()));
    shared.record_sync_pacing = crate::record_sync::SyncPacing {
        score_interval: Duration::ZERO,
        replay_interval: Duration::ZERO,
        trace_interval: Duration::ZERO,
        ..shared.record_sync_pacing
    };
    shared
}

/// A folder holding one run played by hand and kept with its trace, and a second row of the same
/// run stored as an older client stored it: with its replay and its body, and no trace. Answers the
/// second row's id.
fn a_run_stored_without_a_trace(folder: &Path) -> i64 {
    let path = chart_in(folder);
    let mut app = app_in(folder, GaugeKind::Normal);
    ended(&mut app.shared, played_to_its_end(GaugeKind::Normal, &inputs(&model(), &HAND_OFFSETS_US)), &path);
    let (play, _) = the_run(&app.shared);
    let body = play.body.expect("the run kept its body");
    let db = app.shared.scoredb.as_mut().expect("the score database");
    db.record_play_kept(&PlayLog { id: 0, ..play.log }, true, Some(KeptSubmission { body: &body, played_as: None })).expect("the older row is stored")
}

/// Poll `shared` as the frame loop does until the traces are made and any records sync has ended.
fn polled_to_the_end(shared: &mut AppShared) {
    let deadline = Instant::now() + WORK_WAIT;
    while shared.trace_backfill.run.is_some() || shared.record_sync.is_some() || shared.trace_backfill.sync_due {
        assert!(Instant::now() < deadline, "the traces were not made");
        shared.poll_ir_jobs(StageId::Select);
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// The thread makes the trace of the row without one, the save thread keeps it with the mark, a
/// records sync sends it linked to its score, and the next start looks at nothing again.
#[test]
fn the_backfill_keeps_the_trace_marks_its_place_and_has_it_sent() {
    let folder = own_folder("thread");
    let untraced = a_run_stored_without_a_trace(&folder);
    let server = Arc::new(TakingServer::default());
    let mut shared = signed_in(&folder, Arc::clone(&server), true);
    shared.start_trace_backfill(UNPACED);
    assert!(shared.trace_backfill.run.is_some(), "the thread started");
    polled_to_the_end(&mut shared);
    shared.finish_saves();

    let db = shared.scoredb.as_ref().expect("the score database");
    let kept = db.trace_body(untraced).expect("reads").and_then(|body| PlayTrace::from_json(&body)).expect("the row without one has a trace now");
    assert_eq!(db.trace_backfill_through().expect("reads"), untraced, "every row up to it is looked at");
    let sent = server.traced.lock().expect("the trace log").clone();
    assert!(sent.iter().any(|trace| PlayTrace { score_id: None, ..trace.clone() } == kept), "the trace went up: {} sent", sent.len());
    assert!(sent.iter().all(|trace| trace.score_id.as_deref() == Some("sc_backfill")), "every trace is linked to its score");

    shared.start_trace_backfill(UNPACED);
    polled_to_the_end(&mut shared);
    assert_eq!(db_mark(&shared), untraced, "nothing more to look at");
}

/// With AUTO UPLOAD TRACE off the trace is made and kept, and nothing is sent for it.
#[test]
fn with_auto_upload_trace_off_the_trace_is_kept_and_not_sent() {
    let folder = own_folder("not-sent");
    let untraced = a_run_stored_without_a_trace(&folder);
    let server = Arc::new(TakingServer::default());
    let mut shared = signed_in(&folder, Arc::clone(&server), false);
    shared.start_trace_backfill(UNPACED);
    polled_to_the_end(&mut shared);
    shared.finish_saves();
    let db = shared.scoredb.as_ref().expect("the score database");
    assert!(db.trace_body(untraced).expect("reads").is_some(), "the trace is kept");
    assert!(server.traced.lock().expect("the trace log").is_empty(), "no trace was sent");
}

/// A database with nothing to look at starts no thread at all when it holds no run.
#[test]
fn an_empty_history_starts_nothing() {
    let folder = own_folder("empty");
    let mut app = app_in(&folder, GaugeKind::Normal);
    app.shared.start_trace_backfill(UNPACED);
    assert!(app.shared.trace_backfill.run.is_none());
    assert_eq!(db_mark(&app.shared), TRACE_BACKFILL_NOT_STARTED);
}

fn db_mark(shared: &AppShared) -> i64 {
    shared.scoredb.as_ref().expect("the score database").trace_backfill_through().expect("reads")
}

/// A seven-key chart of plain notes and long notes it leaves to LN MODE (`#LNTYPE 1`, no
/// `#LNMODE`), on keys of their own, and a turntable note.
const LONG_NOTE_CHART: &[u8] = b"#PLAYER 1\n#TITLE Backfill Long\n#BPM 150\n#RANK 2\n#TOTAL 300\n#LNTYPE 1\n#WAV01 a.wav\n\
#00112:00010001\n#00151:01000100\n#00211:00100010\n#00253:01000001\n#00316:01000000\n#00314:01010000\n";

/// The file name the long-note chart is written as.
const LONG_NOTE_CHART_FILE: &str = "backfill-long.bms";

/// How late a long note's head is pressed: inside GREAT, so its timing is not nought.
const LONG_HEAD_LATE_US: i64 = 30_000;

/// The seed a run on a lane option is laid out by.
const LAYOUT_SEED: u64 = 0x5eed_0fba_cf11;

/// Where a shade stands while it is switched off, which the run does not show.
const SHADE_SWITCHED_OFF_AT: f32 = 0.3;

/// Every note on its time.
const ON_TIME_US: [Option<i64>; 1] = [Some(0)];

/// How long after a course ended a row stored by an older client is dated, so it is no stage of it.
const AFTER_THE_COURSE_S: i64 = 3_600;

/// The gauge a course opens on.
const COURSE_OPENING_GAUGE: f32 = 100.0;

/// How a run is set up on the app's settings.
#[derive(Clone, Copy)]
struct Setup {
    gauge: GaugeKind,
    random: rbms_chart::shuffle::NoteOption,
    scratch_auto: bool,
    ln_mode: rbms_judge::ln::LnMode,
}

impl Default for Setup {
    fn default() -> Setup {
        let play = Config::default();
        Setup { gauge: GaugeKind::Normal, random: play.play.random, scratch_auto: false, ln_mode: play.judge.ln_mode }
    }
}

/// An app keeping its settings in `folder`, nobody signed in, set up as `setup` says.
fn app_set_up(folder: &Path, setup: Setup) -> App {
    let mut app = app_in(folder, setup.gauge);
    app.shared.config.play.random = setup.random;
    app.shared.config.play.scratch_auto = setup.scratch_auto;
    app.shared.config.judge.ln_mode = setup.ln_mode;
    app
}

/// The inputs of a run of `laid_out`: every key note pressed as far from its time as `offsets_us`
/// says for its place among them, the turntable on its time unless SCRATCH AUTO plays it, a long
/// note's head [`LONG_HEAD_LATE_US`] late and held to its end, let go there on its time.
fn hand_inputs(laid_out: &rbms_model::Model, offsets_us: &[Option<i64>], scratch_auto: bool) -> Vec<(i64, usize, bool)> {
    let mut key_notes = 0;
    let mut inputs = Vec::new();
    for line in &laid_out.timelines {
        for (lane, note) in line.notes.iter().enumerate() {
            match note.as_ref().map(|note| &note.kind) {
                Some(rbms_model::NoteKind::LongStart { .. }) => inputs.push((line.time_us + LONG_HEAD_LATE_US, lane, true)),
                Some(rbms_model::NoteKind::LongEnd { .. }) => inputs.push((line.time_us, lane, false)),
                Some(rbms_model::NoteKind::Normal) if laid_out.mode.is_scratch(lane) => {
                    if !scratch_auto {
                        inputs.extend([(line.time_us, lane, true), (line.time_us + KEY_HELD_US, lane, false)]);
                    }
                }
                Some(rbms_model::NoteKind::Normal) => {
                    key_notes += 1;
                    if let Some(offset_us) = offsets_us[(key_notes - 1) % offsets_us.len()] {
                        inputs.extend([(line.time_us + offset_us, lane, true), (line.time_us + offset_us + KEY_HELD_US, lane, false)]);
                    }
                }
                Some(rbms_model::NoteKind::Mine { .. }) | None => {}
            }
        }
    }
    inputs.sort_unstable();
    inputs
}

/// Play the chart at `path` to its end as `AppShared::load` sets a run of it up on `shared`'s
/// settings, its notes pressed by `offsets_us`, and end it as the result screen does.
fn played_as_loaded(shared: &mut AppShared, path: &str, offsets_us: &[Option<i64>]) {
    let bytes = std::fs::read(path).expect("the chart file reads");
    let decoded = decode_chart(&bytes, path).expect("the chart decodes");
    let setup = crate::judge_setup::run_judge_setup(&shared.config, None);
    let mut model = decoded.model;
    let ln_mode_key = match rbms_chart::contains_undefined_long_note(&model) {
        true => rbms_config::ln_mode_token(setup.ln_mode),
        false => rbms_store::SCORE_LN_MODE_FROM_CHART,
    };
    let lntype = crate::judge_setup::run_lntype(decoded.lnmode, setup.ln_mode);
    rbms_chart::resolve_long_note_flavour(&mut model, setup.ln_mode.resolve());
    crate::app_play::lay_out_notes(&mut model, shared.config.play.random, LAYOUT_SEED, decoded.bmson.as_deref(), &decoded.mode);
    let options = SessionOptions {
        gauge: shared.config.play.gauge,
        judge_offset_us: shared.offset_us(),
        auto_lanes: crate::app_play::scratch_auto_lanes(decoded.mode, shared.config.play.scratch_auto),
        seed: LAYOUT_SEED,
        auto_calibration: shared.config.judge.auto_offset,
        ..SessionOptions::default()
    };
    let mut session = PlaySession::new(model.clone(), options);
    session.set_judge_setup(setup);
    let session = to_its_end(session, &hand_inputs(&model, offsets_us, shared.config.play.scratch_auto));
    shared.mode = decoded.mode;
    shared.chart_path = path.to_owned();
    let mut state = PlayState::new(session, std::collections::HashMap::new(), lntype, ln_mode_key.to_owned());
    let entered = crate::app_result::enter_result(&mut state, shared);
    assert!(matches!(entered, Transition::To(crate::stage::Stage::Result(_))), "the run did not reach its result screen");
}

/// Play `chart` written as `file` on `setup` as the app plays it, and check that the trace its replay
/// is judged again to is the trace the run kept, the history of its scroll settings aside.
fn judged_again_as_it_was_played(tag: &str, chart: &[u8], file: &str, setup: Setup) -> PlayTrace {
    let folder = own_folder(tag);
    let path = folder.join(file);
    std::fs::write(&path, chart).expect("the chart file is writable");
    let mut app = app_set_up(&folder, setup);
    played_as_loaded(&mut app.shared, &path.to_string_lossy(), &HAND_OFFSETS_US);
    let (play, live) = the_run(&app.shared);
    is_the_live_trace(&made_for(&app.shared, &play).expect("the run is judged again to its row"), &live, false);
    live
}

#[test]
fn a_long_note_run_judged_again_makes_the_trace_it_kept() {
    let setup = Setup { ln_mode: rbms_judge::ln::LnMode::LongNote, ..Setup::default() };
    let live = judged_again_as_it_was_played("ln", LONG_NOTE_CHART, LONG_NOTE_CHART_FILE, setup);
    assert!(live.notes.kind.contains(&rbms_ir::TRACE_KIND_LONG_END), "the run has long notes: {:?}", live.notes.kind);
}

#[test]
fn a_charge_note_run_judged_again_makes_the_trace_it_kept() {
    let setup = Setup { ln_mode: rbms_judge::ln::LnMode::ChargeNote, ..Setup::default() };
    let live = judged_again_as_it_was_played("cn", LONG_NOTE_CHART, LONG_NOTE_CHART_FILE, setup);
    assert!(live.notes.kind.contains(&rbms_ir::TRACE_KIND_CHARGE_END), "the run has charge notes: {:?}", live.notes.kind);
    assert_eq!(live.lntype, crate::judge_setup::run_lntype(0, setup.ln_mode));
}

#[test]
fn a_hell_charge_note_run_judged_again_makes_the_trace_it_kept() {
    let setup = Setup { ln_mode: rbms_judge::ln::LnMode::HellChargeNote, ..Setup::default() };
    let live = judged_again_as_it_was_played("hcn", LONG_NOTE_CHART, LONG_NOTE_CHART_FILE, setup);
    assert!(live.notes.kind.contains(&rbms_ir::TRACE_KIND_CHARGE_END), "the run has charge notes: {:?}", live.notes.kind);
    assert_eq!(live.lntype, crate::judge_setup::run_lntype(0, setup.ln_mode));
}

#[test]
fn a_scratch_auto_run_judged_again_makes_the_trace_it_kept() {
    let live = judged_again_as_it_was_played("scratch-auto", CHART, CHART_FILE, Setup { scratch_auto: true, ..Setup::default() });
    assert!(live.auto_lanes.iter().any(|auto| *auto), "SCRATCH AUTO played the turntable: {:?}", live.auto_lanes);
}

#[test]
fn a_random_run_judged_again_makes_the_trace_it_kept() {
    let random = rbms_chart::shuffle::NoteOption::Random;
    let live = judged_again_as_it_was_played("random", CHART, CHART_FILE, Setup { random, ..Setup::default() });
    assert_eq!(live.random, rbms_ir::mapping::ir_random(random));
}

/// A shade switched off is stated as the run showed it, at nought, though the setting under the
/// switch stands elsewhere and the submission states that setting.
#[test]
fn the_shades_are_stated_as_the_run_showed_them() {
    let folder = own_folder("shades");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Normal);
    let play = &mut app.shared.config.play;
    (play.cover, play.enable_cover, play.lift, play.enable_lift) = (SHADE_SWITCHED_OFF_AT, false, SHADE_SWITCHED_OFF_AT, false);
    ended(&mut app.shared, played_to_its_end(GaugeKind::Normal, &inputs(&model(), &HAND_OFFSETS_US)), &path);
    let (play, live) = the_run(&app.shared);
    assert_eq!((live.speed.end.lane_cover, live.speed.end.lift), (0.0, 0.0), "the run showed neither shade");
    let made = PlayTrace::from_json(&made_for(&app.shared, &play).expect("the run is judged again to its row")).expect("reads");
    assert_eq!(made.speed.end, live.speed.end);
}

/// The stage of a course is not judged again, even where its replay would come to its row, as the row
/// an older build stored on the gauge the settings chose does: the run started on the course's gauge
/// and combo, which the replay does not keep. A row of the same chart stored after the course is no
/// stage of it and gets its trace.
#[test]
fn a_course_stage_gets_no_trace_and_is_looked_past() {
    let folder = own_folder("course");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Normal);
    ended(&mut app.shared, played_to_its_end(GaugeKind::Normal, &inputs(&model(), &HAND_OFFSETS_US)), &path);
    let (single, _) = the_run(&app.shared);
    let course = rbms_course::Course {
        name: "BACKFILL COURSE".to_owned(),
        charts: vec![rbms_course::CourseChart { md5: model().md5, sha256: String::new(), title: "Backfill".to_owned() }],
        ..rbms_course::Course::default()
    };
    app.shared.course_run = Some(rbms_course::CourseRun::new(course, COURSE_OPENING_GAUGE));
    let options = SessionOptions { initial_gauge: Some((rbms_judge::gauge::GaugeIndex::Class, COURSE_OPENING_GAUGE)), ..SessionOptions::default() };
    let session = to_its_end(PlaySession::new(model(), options), &inputs(&model(), &ON_TIME_US));
    let mut state = playing(&mut app.shared, session, &path);
    let entered = crate::app_result::enter_result(&mut state, &mut app.shared);
    assert!(matches!(entered, Transition::To(crate::stage::Stage::CourseResult(_))), "the one-stage course ended on its result");
    let db = app.shared.scoredb.as_mut().expect("the score database");
    let stage = db.all_plays().expect("reads").into_iter().max_by_key(|log| log.id).expect("the stage is stored");
    assert_ne!(stage.id, single.log.id, "the stage is a row of its own");
    assert_eq!(stage.gauge, rbms_store::scoredb::GAUGE_CLASS, "the stage is stored on the course gauge it ran on");
    let stage_body = body_of(db, stage.id);
    let single_body = single.body.clone().expect("the run kept its body");
    let stage = PlayLog { id: 0, gauge: rbms_store::scoredb::gauge_id("normal"), ..stage };
    let stage_untraced =
        db.record_play_kept(&stage, true, Some(KeptSubmission { body: &stage_body, played_as: None })).expect("stored as an older build stored it");
    let later = PlayLog { id: 0, date: single.log.date + AFTER_THE_COURSE_S, ..single.log.clone() };
    let after_the_course = db.record_play_kept(&later, true, Some(KeptSubmission { body: &single_body, played_as: None })).expect("stored");
    let as_stored = UntracedPlay { log: PlayLog { id: stage_untraced, ..stage }, body: Some(stage_body) };
    assert!(made_for(&app.shared, &as_stored).is_ok(), "the stage's replay comes to its row by itself");

    app.shared.start_trace_backfill(UNPACED);
    polled_to_the_end(&mut app.shared);
    app.shared.finish_saves();
    let db = app.shared.scoredb.as_ref().expect("the score database");
    assert_eq!(db.trace_body(stage_untraced).expect("reads"), None, "the stage gets no trace");
    assert!(db.trace_body(after_the_course).expect("reads").is_some(), "the row after the course gets its trace");
    assert_eq!(db.trace_backfill_through().expect("reads"), after_the_course, "both are looked past");
}

/// The body `db` kept with the row `id`.
fn body_of(db: &ScoreDb, id: i64) -> String {
    let walk = SyncWalk { server: SERVER_URL, login_id: ACCOUNT_ID, ..SyncWalk::default() };
    db.sync_candidates(&walk).expect("reads").into_iter().find(|candidate| candidate.log.id == id).and_then(|candidate| candidate.body).expect("a body")
}

/// A run whose chart file is gone is not looked past: once the file is back, the next start makes
/// its trace.
#[test]
fn a_run_whose_chart_is_gone_is_looked_at_again_on_the_next_start() {
    let folder = own_folder("chart-gone");
    let untraced = a_run_stored_without_a_trace(&folder);
    let path = folder.join(CHART_FILE);
    let away = folder.join("away.bms");
    std::fs::rename(&path, &away).expect("the chart file moves");
    let mut app = app_in(&folder, GaugeKind::Normal);
    app.shared.start_trace_backfill(UNPACED);
    polled_to_the_end(&mut app.shared);
    app.shared.finish_saves();
    assert_eq!(app.shared.scoredb.as_ref().expect("the score database").trace_body(untraced).expect("reads"), None);
    assert!(db_mark(&app.shared) < untraced, "the run whose chart is gone is not marked as looked at");

    std::fs::rename(&away, &path).expect("the chart file moves back");
    app.shared.start_trace_backfill(UNPACED);
    polled_to_the_end(&mut app.shared);
    app.shared.finish_saves();
    assert!(app.shared.scoredb.as_ref().expect("the score database").trace_body(untraced).expect("reads").is_some(), "the trace is made");
    assert_eq!(db_mark(&app.shared), untraced);
}

/// A batch the save thread could not write leaves the mark where it was, though a later batch is
/// written: the runs of the first are looked at again on the next start.
#[test]
fn a_batch_that_was_not_written_moves_no_mark() {
    let folder = own_folder("unwritten");
    let first = a_run_stored_without_a_trace(&folder);
    let app = app_in(&folder, GaugeKind::Normal);
    let (db_path, _, _) = crate::scoredb_store::score_paths(&folder);
    let saves = crate::save_worker::SaveWorker::inline();
    let unwritten = BackfillBatch { from: TRACE_BACKFILL_NOT_STARTED, through: first, ..BackfillBatch::default() };
    saves.keep_backfilled_traces(folder.join(CHART_FILE).join("scoredb.sqlite"), unwritten);
    saves.keep_backfilled_traces(db_path, BackfillBatch { from: first, through: first + 1, ..BackfillBatch::default() });
    assert!(db_mark(&app.shared) < first, "the runs of the batch not written are not marked");
    let db = app.shared.scoredb.as_ref().expect("the score database");
    assert_eq!(db.untraced_plays(db_mark(&app.shared), first, 1).expect("reads").first().map(|play| play.log.id), Some(first), "the run is looked at again");
}

/// The trace made for `play` from what `shared` keeps, the thread neither held nor cancelled.
fn made_for(shared: &AppShared, play: &UntracedPlay) -> Result<String, Refusal> {
    trace_of(play, &Sources { replay_dir: &shared.replay_dir, songs: None, shades: ShadeSwitches::of(&shared.config), cancelled: &|| false })
}

/// Where a run left long after its last input is left: in its last measure.
const LEFT_LATE_AT_US: i64 = 6_000_000;

/// A run left long after its gauge had sunk to its floor gets no trace: the notes that went by
/// unhit moved the gauge no further, so the row is the same whichever of them the run reached, and
/// where it was left is not known.
#[test]
fn a_run_left_on_a_sunk_gauge_gets_no_trace() {
    let folder = own_folder("left-sunk");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Normal);
    let pressed: Vec<_> = inputs(&model(), &HAND_OFFSETS_US).into_iter().filter(|(at_us, _, _)| *at_us < LAST_PRESS_BEFORE_US).collect();
    left(&mut app.shared, played(GaugeKind::Normal, &pressed, LEFT_LATE_AT_US), &path);
    let (play, live) = the_run(&app.shared);
    let floor = live.gauge_bounds[usize::try_from(live.gauge.selected).expect("an index")].min;
    assert_eq!(play.log.gauge_value, floor, "the gauge had sunk to its floor");
    assert!(live.notes.state.contains(&0) && live.notes.state.contains(&POOR_STATE), "notes went by and notes were still to come");
    assert!(matches!(made_for(&app.shared, &play), Err(Refusal::Never(_))), "where the run was left is not known");
}

/// The search for where a run was left asks whether it is cancelled as it goes, and ends when it
/// is.
#[test]
fn the_search_for_where_a_run_was_left_looks_at_being_cancelled() {
    let folder = own_folder("left-cancelled");
    let path = chart_in(&folder);
    let mut app = app_in(&folder, GaugeKind::Normal);
    left(&mut app.shared, played(GaugeKind::Normal, &inputs(&model(), &HAND_OFFSETS_US), LEFT_AT_US), &path);
    let (play, _) = the_run(&app.shared);
    let asked = std::cell::Cell::new(0);
    let cancel_at_once = || {
        asked.set(asked.get() + 1);
        true
    };
    let sources = Sources { replay_dir: &app.shared.replay_dir, songs: None, shades: ShadeSwitches::of(&app.shared.config), cancelled: &cancel_at_once };
    assert_eq!(trace_of(&play, &sources), Err(Refusal::Stopped), "the search ends when the thread is cancelled");
    assert_eq!(asked.get(), 1, "it was asked once, and stopped");
}

/// A row stored on a course gauge, as a row imported from another client's history may be, is a
/// course's stage; a row on a gauge of its own is one only where a kept course had its stages.
#[test]
fn a_row_on_a_course_gauge_is_a_course_stage() {
    let on_class = UntracedPlay { log: PlayLog { id: 1, gauge: rbms_store::scoredb::gauge_id("class"), ..PlayLog::default() }, body: None };
    assert!(course_stage(&on_class, &HashSet::new()).is_some(), "a row on the class gauge");
    let on_its_own = UntracedPlay { log: PlayLog { gauge: rbms_store::scoredb::gauge_id("normal"), ..on_class.log.clone() }, ..on_class };
    assert_eq!(course_stage(&on_its_own, &HashSet::new()), None, "a row on a gauge of its own, no course kept");
    assert!(course_stage(&on_its_own, &HashSet::from([on_its_own.log.id])).is_some(), "a row where a kept course had a stage");
}
