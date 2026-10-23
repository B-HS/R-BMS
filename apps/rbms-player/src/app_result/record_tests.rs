//! What a run the player played leaves behind, checked on runs that are really played: a chart is
//! judged by the engine against inputs given at the times written here, the application is handed
//! the run the way the play screen hands one over, and the score database of the test's own folder
//! is read back.
//!
//! The question these answer is whether a player who never signed in keeps their records. They do:
//! nothing on the way from a finished run to the database asks whether a score server is set up or
//! an account is signed in, and the one column that differs is the one that says whether the run
//! was sent.
//!
//! Every run the player played by hand is kept and sent (the player's decision of 2026-10-14). A
//! SCRATCH AUTO run sets bests like any other, its turntable notes counting for nothing; a judge
//! widened past the chart's own still sets none, which is the reference's rule
//! (`ScoreData.update(newscore, updateScore)`): such a run keeps its row in the history and its
//! replay, raises the lamp and the play count, and leaves the best score as it was.

use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rbms_config::ScoreTarget;
use rbms_ir::{CourseSubmission, IrError, PlayerProfile, ReplayData, ServerInfo, SubmitResponse};
use rbms_judge::gauge::GaugeIndex;
use rbms_store::scoredb::{BestScore, PlayLog, ReplayState, SyncState, SyncWalk, chart_key, mode_id};

use super::*;
use crate::app_play::OwnPlayOptions;
use crate::ir_session::AccountSession;

mod contract_requests;
mod course_gauge;
mod left_behind;

/// A seven-key chart of two measures of eighth notes on the first key with one turntable note among
/// them, and a last note on the seventh key.
const RECORDED_CHART: &[u8] =
    b"#PLAYER 1\n#TITLE Recorded\n#BPM 120\n#RANK 2\n#TOTAL 300\n#WAV01 a.wav\n#00111:0101010101010101\n#00211:0101010101010101\n#00216:01000000\n#00319:01\n";

/// The file name the chart is parsed under.
const RECORDED_CHART_NAME: &str = "recorded.bms";

/// How far from its note each key is pressed, in microseconds, by the note's place among the key
/// notes: late and early enough for the second judgement, on the note and a little off it either
/// way, and late enough for the third. `None` is a note nobody presses.
///
/// The EX these add up to is on none of the lines a rank target draws, so a target that fell back
/// to one cannot be taken for a target that is this run's score.
pub(crate) const HAND_OFFSETS_US: [Option<i64>; 8] = [Some(40_000), Some(0), Some(5_000), Some(-40_000), Some(-5_000), Some(100_000), Some(40_000), None];

/// The same for a second run on the chart that scores less: every other note is let go by.
const WEAKER_OFFSETS_US: [Option<i64>; 2] = [Some(0), None];

/// The same for a run that fails on the hardest gauge: one note in four is pressed.
const FAILING_OFFSETS_US: [Option<i64>; 4] = [Some(0), None, None, None];

/// How long a key stays down, the step the engine is run at, and how far past the chart's own
/// playing time it is run.
const KEY_HELD_US: i64 = 50_000;
const RUN_FRAME_US: i64 = 10_000;
const RUN_MARGIN_US: i64 = 1_000_000;

/// How many rows of a chart's history a test reads back, which is more than any of them writes.
const HISTORY_ROWS: usize = 8;

/// The gauge the course a test plays opens on.
const COURSE_OPENING_GAUGE: f32 = 100.0;

/// How long a test waits for the course its run finished to reach the server, which sends it from a
/// thread of its own.
const COURSE_SUBMIT_WAIT: Duration = Duration::from_secs(10);

/// The account the signed-in runs are played under.
const ACCOUNT_TOKEN: &str = "token-of-a-test";
const ACCOUNT_ID: &str = "tester";

/// A score server that takes every score it is sent and keeps the players they were sent for,
/// without a network under it.
#[derive(Default)]
struct AcceptingServer {
    sent_for: Mutex<Vec<String>>,
    /// What each submission said of the assists it was played with: SCRATCH AUTO, and the tags.
    assists: Mutex<Vec<(bool, Vec<String>)>>,
    /// The gauge each submission said its run finished on.
    gauges: Mutex<Vec<rbms_ir::GaugeType>>,
    /// Where each course submission is handed on, when a test listens for one.
    courses: Mutex<Option<Sender<CourseSubmission>>>,
}

impl ScoreServer for AcceptingServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_score(&self, sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        self.sent_for.lock().expect("the submission log").push(sub.player.id.clone());
        self.assists.lock().expect("the assist log").push((sub.options.scratch_auto, sub.options.assist.clone()));
        self.gauges.lock().expect("the gauge log").push(sub.options.gauge);
        Ok(SubmitResponse { accepted: true, ..Default::default() })
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
        let Some(courses) = self.courses.lock().expect("the course log").clone() else {
            return Err(IrError::Unsupported);
        };
        let _ = courses.send(sub.clone());
        Ok(SubmitResponse { accepted: true, ..Default::default() })
    }
    fn upload_replay(&self, _chart: &ChartId, _replay: &ReplayData) -> Result<String, IrError> {
        Err(IrError::Unsupported)
    }
}

/// The settings file of a folder that belongs to one test, emptied of whatever an earlier run of
/// the tests left in it, so the score database and the replays beside it are this test's alone.
fn settings_in_own_folder(tag: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("rbms-record-tests-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder is writable");
    folder.join("settings.ron")
}

/// An app with no score server set up and nobody signed in, which is what a fresh install is.
fn signed_out(tag: &str) -> AppShared {
    let mut config = Config::default();
    config.play.autoplay = false;
    let shared = crate::App::new(String::new(), config, LaunchOptions::default(), settings_in_own_folder(tag)).shared;
    assert!(!shared.has_primary_ir_server() && !shared.session.is_logged_in(), "the fixture is a player who never signed in");
    shared
}

/// An app with a score server set up and an account signed in, whose server is `server`: the one
/// a chart is sent to, and the one the profile list a course is sent through reuses, as
/// [`AppShared::rebuild_server`] lays them out.
fn signed_in(tag: &str, server: Arc<AcceptingServer>) -> AppShared {
    let mut config = Config::default();
    config.play.autoplay = false;
    config.network.server_url = Some("http://score-server.invalid".to_owned());
    config.network.player_id = ACCOUNT_ID.to_owned();
    let mut shared = crate::App::new(String::new(), config, LaunchOptions::default(), settings_in_own_folder(tag)).shared;
    shared.server = server;
    shared.profile_servers = shared.multi_ir.build_servers_reusing(Some(shared.server.clone()));
    shared.session = AccountSession::restored(Some(ACCOUNT_TOKEN.to_owned()), Some(ACCOUNT_ID.to_owned()));
    assert!(shared.has_primary_ir_server() && shared.session.is_logged_in(), "the fixture is a player who signed in");
    shared
}

/// The recorded chart as the engine plays it.
pub(crate) fn recorded_model() -> rbms_model::Model {
    let source = rbms_parser::parse_with(RECORDED_CHART, Default::default());
    rbms_chart::to_model(&source, Mode::BEAT_7K)
}

/// One input of a run: when it lands, the lane it is on, and whether the key goes down.
type Input = (i64, usize, bool);

/// The inputs of a run of the recorded chart played by hand: every key note is pressed as far from
/// its time as `offsets_us` says for its place among them, and the turntable note is played on its
/// time unless `scratch_auto` leaves it to the engine.
fn inputs_by_hand(model: &rbms_model::Model, offsets_us: &[Option<i64>], scratch_auto: bool) -> Vec<Input> {
    let mut inputs = Vec::new();
    let mut key_notes = 0;
    for line in &model.timelines {
        for (lane, _) in line.notes.iter().enumerate().filter(|(_, note)| note.is_some()) {
            let offset_us = if model.mode.is_scratch(lane) {
                (!scratch_auto).then_some(0)
            } else {
                key_notes += 1;
                offsets_us[(key_notes - 1) % offsets_us.len()]
            };
            if let Some(offset_us) = offset_us {
                inputs.push((line.time_us + offset_us, lane, true));
                inputs.push((line.time_us + offset_us + KEY_HELD_US, lane, false));
            }
        }
    }
    inputs.sort_unstable();
    inputs
}

/// Run the recorded chart from its first moment to past its end under `options`, with `inputs`
/// handed to the engine as their times come.
fn run_to_its_end(options: SessionOptions, inputs: &[Input]) -> PlaySession {
    run_model_to_its_end(recorded_model(), options, inputs)
}

/// Run `model` from its first moment to past its end under `options`, with `inputs` handed to the
/// engine as their times come.
fn run_model_to_its_end(model: rbms_model::Model, options: SessionOptions, inputs: &[Input]) -> PlaySession {
    replay_inputs(PlaySession::new(model, options), inputs)
}

/// Run `session` from its first moment to past its end, with `inputs` handed to the engine as their
/// times come.
fn replay_inputs(mut session: PlaySession, inputs: &[Input]) -> PlaySession {
    let run_us = session.play_time_ms() * rbms_skin::timer::MICROS_PER_MILLI + RUN_MARGIN_US;
    let mut next = 0;
    for frame in 0..=run_us / RUN_FRAME_US {
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

/// Play the recorded chart by hand on `gauge`, the turntable note left to the engine when
/// `scratch_auto` is set, as SCRATCH AUTO has it played.
fn played_on(gauge: rbms_judge::GaugeKind, offsets_us: &[Option<i64>], scratch_auto: bool) -> PlaySession {
    let model = recorded_model();
    let mode = model.mode;
    let auto_lanes = if scratch_auto { (0..mode.key).map(|lane| mode.is_scratch(lane)).collect() } else { Vec::new() };
    run_to_its_end(SessionOptions { gauge, auto_lanes, ..SessionOptions::default() }, &inputs_by_hand(&model, offsets_us, scratch_auto))
}

/// Play the recorded chart by hand on the gauge a fresh install plays on.
pub(crate) fn played_by_hand(offsets_us: &[Option<i64>], scratch_auto: bool) -> PlaySession {
    played_on(Config::default().play.gauge, offsets_us, scratch_auto)
}

/// Play the recorded chart by hand judged wider than it asks (JUDGE WIDTH over 100%), which is the
/// one run by hand that sets no best.
pub(crate) fn played_widened(offsets_us: &[Option<i64>]) -> PlaySession {
    let widened = rbms_play::JudgeSetup { judge_rate_key: [rbms_config::JUDGE_RATE_MAX_PERCENT, 100, 100], ..rbms_play::JudgeSetup::default() };
    let model = recorded_model();
    let mut session = PlaySession::new(model.clone(), SessionOptions::default());
    session.set_judge_setup(widened);
    replay_inputs(session, &inputs_by_hand(&model, offsets_us, false))
}

/// Hand `session` to the application as a run that has just ended on the play screen, and answer
/// the screen it goes to and what the engine says of the run.
pub(crate) fn end_run(shared: &mut AppShared, session: PlaySession) -> (Transition, rbms_play::PlaySummary) {
    shared.mode = session.model().mode;
    shared.chart_path = RECORDED_CHART_NAME.to_owned();
    shared.replay = None;
    let summary = session.summary();
    let mut played = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    (enter_result(&mut played, shared), summary)
}

/// The key the recorded chart's rows are stored under.
fn recorded_key() -> String {
    chart_key(&recorded_model().md5, "")
}

/// A walk of every play to `server` under the test account, every refusal included.
fn walk(server: &str) -> SyncWalk<'_> {
    SyncWalk { server, login_id: ACCOUNT_ID, ..SyncWalk::default() }
}

/// Every row the score database holds of the recorded chart's plays, newest first, and its best.
fn stored(shared: &AppShared) -> (Vec<PlayLog>, Option<BestScore>) {
    let db = shared.scoredb.as_ref().expect("the score database opened in the test's folder");
    let key = recorded_key();
    (db.history(&key, HISTORY_ROWS).expect("the history reads"), db.best(&key, mode_id(Mode::BEAT_7K.name)).expect("the best reads"))
}

/// Every row the score database holds, of whatever chart.
fn every_play(shared: &AppShared) -> Vec<PlayLog> {
    shared.scoredb.as_ref().expect("the score database opened in the test's folder").all_plays().expect("the history reads")
}

/// The target the result screen `entered` leads to scores its run against.
fn result_target_ex(entered: &Transition) -> Option<u32> {
    match entered {
        Transition::To(Stage::Result(result)) => result.target().map(|target| target.ex),
        _ => None,
    }
}

/// What the result screen `entered` leads to says of its run in its own words.
fn result_notices(entered: &Transition, shared: &AppShared) -> Vec<String> {
    match entered {
        Transition::To(Stage::Result(result)) => result.notices(shared),
        _ => Vec::new(),
    }
}

/// Hold a stored row to the run the engine reported: its judgements early and late, its EX, its
/// combo, its bad-poor count with every note it never reached (`BMSPlayer.java:909`), its note
/// count and the lamp it was awarded.
fn assert_log_is_the_run(log: &PlayLog, summary: &rbms_play::PlaySummary, lamp: ClearType) {
    assert_eq!((log.judge.early, log.judge.late), (summary.early, summary.late), "the judgements");
    assert_eq!(log.judge.ex_score(), summary.ex_score, "the EX score");
    assert_eq!(
        (log.combo, log.minbp, log.notes),
        (summary.max_combo, summary.min_bp_with_unreached(), summary.total_notes),
        "the combo, the bad-poor count and the notes"
    );
    assert_eq!(log.clear, clear_type_id(lamp), "the lamp");
    assert!(log.date > 0, "the run has no date");
}

/// A player who never signed in plays a chart to its end. Everything the run was is stored: its
/// row in the history, the best the browser's score window is read from, the day it was played on,
/// and its replay; and the book the next run's LOCAL BEST and the score graph's BEST are read from
/// holds its EX.
#[test]
fn a_run_played_without_signing_in_is_stored_whole() {
    let mut shared = signed_out("signed-out");
    let (entered, summary) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    assert!(matches!(entered, Transition::To(Stage::Result(_))), "the run did not reach the result screen");
    assert!(summary.counts[..3].iter().all(|count| *count > 0) && summary.min_bp > 0, "the fixture run takes every kind of judgement: {:?}", summary.counts);
    assert!(summary.ex_score > 0 && summary.max_combo > 1 && summary.clear_lamp.is_cleared(), "the fixture run scores and clears: {summary:?}");
    assert_eq!(shared.ir_status, IrStatus::Off, "no score server is set up");
    assert_eq!(result_notices(&entered, &shared), Vec::<String>::new(), "a run that was stored whole has nothing said against it");

    let (logs, best) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one run was played and {} rows were stored", logs.len());
    };
    assert_log_is_the_run(log, &summary, summary.clear_lamp);
    assert!(!log.ir_submitted, "nothing was sent anywhere");

    let best = best.expect("the chart has a best");
    assert_eq!(best.judge, log.judge, "the best holds the run's judgements, which the judgement bars are drawn from");
    assert_eq!(best.ex_score(), summary.ex_score);
    assert_eq!((best.combo, best.minbp, best.notes), (summary.max_combo, i64::from(summary.min_bp), summary.total_notes));
    assert_eq!((best.clear, best.playcount, best.clearcount), (clear_type_id(summary.clear_lamp), 1, 1));
    assert_eq!(best.date, log.date);

    let replay = log.replay_file.as_deref().expect("the run saved its replay");
    assert_eq!(best.replay_file.as_deref(), Some(replay));
    let saved = Replay::load(&shared.replay_dir.join(replay)).expect("the replay reads back");
    assert!(!saved.events.is_empty() && saved.md5 == recorded_model().md5, "the replay is not this run's");

    let md5 = recorded_model().md5;
    assert_eq!(shared.scores.best_ex_for_md5_in_ln_mode(&md5, SCORE_LN_MODE_FROM_CHART), Some(summary.ex_score), "the next run's LOCAL BEST and BEST");
    assert_eq!(shared.scores.for_md5(&md5).len(), 1, "the record list");
    let day = shared.scoredb.as_ref().and_then(|db| db.day_stats(i64::MIN, i64::MAX).ok()).unwrap_or_default();
    assert_eq!(day.iter().map(|day| day.playcount).sum::<u32>(), 1, "the day's play count");
}

/// The records are read back by an app that starts afresh on the same folder, which is what the
/// next launch is: the book the browser lists by is built from the database, and holds the run.
#[test]
fn a_run_played_without_signing_in_is_there_on_the_next_launch() {
    let mut shared = signed_out("next-launch");
    let settings = shared.settings_path.clone();
    let (_, summary) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    drop(shared);

    let relaunched = crate::App::new(String::new(), Config::default(), LaunchOptions::default(), settings).shared;
    let md5 = recorded_model().md5;
    assert_eq!(relaunched.scores.best_ex_for_md5_in_ln_mode(&md5, SCORE_LN_MODE_FROM_CHART), Some(summary.ex_score));
    assert_eq!(relaunched.scores.best_clear_for_md5(&md5), Some(clear_type_id(summary.clear_lamp)));
    let (logs, best) = stored(&relaunched);
    assert_eq!((logs.len(), best.map(|best| best.ex_score())), (1, Some(summary.ex_score)));
}

/// The same run played by an account that is signed in stores the same rows. The one difference
/// is the column that says the run was sent to a score server.
#[test]
fn signing_in_changes_nothing_of_what_a_run_stores_but_that_it_was_sent() {
    let mut offline = signed_out("compare-signed-out");
    let (_, offline_summary) = end_run(&mut offline, played_by_hand(&HAND_OFFSETS_US, false));
    let server = Arc::new(AcceptingServer::default());
    let mut online = signed_in("compare-signed-in", server.clone());
    let (_, online_summary) = end_run(&mut online, played_by_hand(&HAND_OFFSETS_US, false));
    assert_eq!(offline_summary, online_summary, "the two runs are the same run");
    online.submit_rx.take().expect("the run was handed to the score server").recv().expect("the submission came back");
    assert_eq!(server.sent_for.lock().expect("the submission log").as_slice(), [ACCOUNT_ID.to_owned()], "the run was sent once, for the account");

    let (offline_logs, offline_best) = stored(&offline);
    let (online_logs, online_best) = stored(&online);
    let comparable = |log: &PlayLog| PlayLog { date: 0, replay_file: None, ir_submitted: false, ..log.clone() };
    assert_eq!(offline_logs.iter().map(comparable).collect::<Vec<_>>(), online_logs.iter().map(comparable).collect::<Vec<_>>());
    assert_eq!((offline_logs[0].ir_submitted, online_logs[0].ir_submitted), (false, true), "only the signed-in run says it was sent");
    assert!(offline_logs[0].replay_file.is_some() && online_logs[0].replay_file.is_some(), "both runs saved a replay");
    let comparable = |best: Option<BestScore>| best.map(|best| BestScore { date: 0, replay_file: None, ..best });
    assert_eq!(comparable(offline_best), comparable(online_best), "the bests differ");
}

/// A second, weaker run on the chart is another row and another play, and takes nothing from the
/// best: the result screen scores it against the first run's EX when the target is the local best.
#[test]
fn a_weaker_run_joins_the_history_and_is_scored_against_the_best_that_stood() {
    let mut shared = signed_out("weaker");
    shared.config.judge.target = ScoreTarget::LocalBest;
    let (_, best_summary) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    let (second, weaker_summary) = end_run(&mut shared, played_by_hand(&WEAKER_OFFSETS_US, false));
    assert!(weaker_summary.ex_score < best_summary.ex_score, "the second run is the weaker one");
    assert_eq!(result_target_ex(&second), Some(best_summary.ex_score), "the second run is scored against the first");

    let (logs, best) = stored(&shared);
    let best = best.expect("the chart has a best");
    assert_eq!(logs.len(), 2);
    assert_eq!((best.ex_score(), best.playcount), (best_summary.ex_score, 2));
    assert_eq!(i64::from(best_summary.min_bp.min(weaker_summary.min_bp)), best.minbp, "the bad-poor count is the least of the two");
}

/// SCRATCH AUTO leaves the turntable to the chart and the run is judged on the notes that are
/// left: it is kept, replayed and sets the best like any other run, its lamp is the one an assisted
/// clear is demoted to, and the next run is paced against it. Whether an account is signed in has no
/// part in it.
#[test]
fn a_run_played_with_scratch_auto_is_stored_whole_and_sets_the_best() {
    let mut shared = signed_out("scratch-auto");
    shared.config.play.scratch_auto = true;
    shared.config.judge.target = ScoreTarget::LocalBest;
    let (first, summary) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, true));
    let by_hand = played_by_hand(&HAND_OFFSETS_US, false).summary();
    assert!(summary.ex_score > 0 && summary.clear_lamp.is_cleared(), "the fixture run scores and clears: {summary:?}");
    assert_eq!(summary.total_notes + 1, by_hand.total_notes, "the turntable note is not counted");
    assert_eq!(shared.ir_status, IrStatus::Off, "nothing holds the run back but that no server is set up");
    let notices = result_notices(&first, &shared);
    assert!(notices.iter().any(|line| line.contains("SCRATCH AUTO") && line.contains("NOT COUNTED")), "the screen does not say what was left out: {notices:?}");

    let (logs, best) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one run was played and {} rows were stored", logs.len());
    };
    assert_log_is_the_run(log, &summary, ClearType::LightAssistEasy);
    assert_eq!((log.assist, log.rule_version), (i32::from(LIGHT_ASSIST), rbms_store::SCORE_RULE_VERSION), "the history says the run was SCRATCH AUTO");
    let replay = log.replay_file.as_deref().expect("a run with SCRATCH AUTO on keeps its replay");
    assert!(Replay::load(&shared.replay_dir.join(replay)).expect("the replay reads back").scratch_auto, "and its replay says so too");

    let best = best.expect("the chart has a best");
    assert_eq!((best.clear, best.playcount, best.clearcount), (clear_type_id(ClearType::LightAssistEasy), 1, 1));
    assert_eq!((best.ex_score(), best.combo, best.notes, best.minbp), (summary.ex_score, summary.max_combo, summary.total_notes, i64::from(summary.min_bp)));

    let md5 = recorded_model().md5;
    assert_eq!(shared.scores.best_ex_for_md5_in_ln_mode(&md5, SCORE_LN_MODE_FROM_CHART), Some(summary.ex_score), "the run is the local best");
    assert_eq!(shared.scores.best_clear_for_md5(&md5), Some(clear_type_id(ClearType::LightAssistEasy)));

    let (second, _) = end_run(&mut shared, played_by_hand(&WEAKER_OFFSETS_US, true));
    assert_eq!(result_target_ex(&second), Some(summary.ex_score), "the next run is scored against it");
}

/// A run played with SCRATCH AUTO is sent like any other, and what it is sent with says so: the
/// switch and the assist tag. What the server makes of it is the server's.
#[test]
fn a_run_played_with_scratch_auto_is_sent_saying_so() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("scratch-auto-sent", server.clone());
    shared.config.play.scratch_auto = true;
    end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, true));
    shared.submit_rx.take().expect("the run was handed to the score server").recv().expect("the submission came back");
    assert_eq!(server.assists.lock().expect("the assist log").as_slice(), [(true, vec![rbms_ir::mapping::AUTO_SCRATCH_FLAG.to_owned()])]);
    let (logs, _) = stored(&shared);
    assert!(logs[0].ir_submitted, "the history says it was sent");
}

/// A player who has the score server set, as every install has by default, and has not signed in:
/// the run is kept whole and is not sent, and its row says it was not, so nothing marks it sent
/// that never left the computer.
#[test]
fn a_run_nobody_was_signed_in_to_send_is_kept_as_not_sent() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("guest-of-the-server", server.clone());
    shared.session = AccountSession::restored(None, None);
    assert!(shared.has_primary_ir_server() && !shared.sends_to_primary_ir_server(), "the fixture has a server and nobody signed in to it");
    let (_, summary) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    assert!(shared.submit_rx.is_none() && matches!(shared.ir_status, IrStatus::Skipped(_)), "the run was handed over: {:?}", shared.ir_status);
    assert!(server.sent_for.lock().expect("the submission log").is_empty(), "the run reached the server");
    let (logs, best) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one run was played and {} rows were stored", logs.len());
    };
    assert_log_is_the_run(log, &summary, summary.clear_lamp);
    assert!(!log.ir_submitted, "the row says a run that was never sent was sent");
    assert_eq!(best.map(|best| best.ex_score()), Some(summary.ex_score));
}

/// A course of the recorded chart alone, which is what the course tests play.
fn one_stage_course(md5: String) -> rbms_course::CourseRun {
    let course = rbms_course::Course {
        name: "RECORDED COURSE".to_owned(),
        charts: vec![rbms_course::CourseChart { md5, sha256: String::new(), title: "Recorded".to_owned() }],
        ..rbms_course::Course::default()
    };
    rbms_course::CourseRun::new(course, COURSE_OPENING_GAUGE)
}

/// Play a one-stage course of the recorded chart, signed in, through `session`, and answer the
/// stage reasons the course kept, whether it may set a best, and the course submission the server
/// was sent.
fn course_played(tag: &str, scratch_auto: bool, session: impl FnOnce() -> PlaySession) -> (Vec<Option<String>>, bool, CourseSubmission) {
    let server = Arc::new(AcceptingServer::default());
    let (courses, sent) = channel();
    *server.courses.lock().expect("the course log") = Some(courses);
    let mut shared = signed_in(tag, server.clone());
    shared.config.play.scratch_auto = scratch_auto;
    shared.course_run = Some(one_stage_course(recorded_model().md5));
    let (entered, _) = end_run(&mut shared, session());
    assert!(matches!(entered, Transition::To(Stage::CourseResult(_))), "the one-stage course did not end on its result");
    shared.submit_rx.take().expect("the stage was handed to the score server").recv().expect("the submission came back");
    assert_eq!(server.sent_for.lock().expect("the submission log").len(), 1, "the stage was sent as a chart");
    let course = sent.recv_timeout(COURSE_SUBMIT_WAIT).expect("the course was sent as a course");
    let sets_best = crate::course_ir::course_updates_score(&shared.course_stage_reasons, shared.course_assist);
    (shared.course_stage_reasons.clone(), sets_best, course)
}

/// A course stage played with SCRATCH AUTO is kept and sent as a chart like any other, and its
/// course is sent as a course, saying so, and sets a course best the way the chart sets one (the
/// player's decision of 2026-10-14). A course played without an assist sends what it always sent.
#[test]
fn a_course_with_a_scratch_auto_stage_is_sent_saying_so_and_sets_a_best() {
    for scratch_auto in [false, true] {
        let (reasons, sets_best, course) =
            course_played(&format!("course-stage-{scratch_auto}"), scratch_auto, || played_by_hand(&HAND_OFFSETS_US, scratch_auto));
        assert_eq!(reasons, [None], "SCRATCH AUTO {scratch_auto}: no stage holds the course back");
        assert!(sets_best, "SCRATCH AUTO {scratch_auto}: the course may set a best");
        assert_eq!(course.scratch_auto, scratch_auto);
        let tags = if scratch_auto { vec![rbms_ir::mapping::AUTO_SCRATCH_FLAG.to_owned()] } else { Vec::new() };
        assert_eq!(course.assist, tags, "SCRATCH AUTO {scratch_auto}");
    }
}

/// A course stage runs on the course's CLASS gauge whatever gauge the settings choose, and it is
/// stored and sent on that gauge: a SCRATCH AUTO clear of it is the ASSIST CLEAR the result screen
/// shows, not the grade of the gauge the settings name.
#[test]
fn a_course_stage_is_stored_and_sent_on_the_course_gauge_it_ran_on() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("course-stage-gauge", server.clone());
    shared.config.play.scratch_auto = true;
    shared.config.play.gauge = rbms_judge::GaugeKind::ExHard;
    let model = recorded_model();
    shared.course_run = Some(one_stage_course(model.md5.clone()));
    let auto_lanes = (0..model.mode.key).map(|lane| model.mode.is_scratch(lane)).collect();
    let course_gauge = Some((GaugeIndex::Class, COURSE_OPENING_GAUGE));
    let options = SessionOptions { gauge: rbms_judge::GaugeKind::ExHard, auto_lanes, initial_gauge: course_gauge, ..SessionOptions::default() };
    let session = run_model_to_its_end(model.clone(), options, &inputs_by_hand(&model, &HAND_OFFSETS_US, true));
    assert_eq!(session.record().finished_gauge, GaugeIndex::Class, "the stage ran on the course gauge");
    let (_, summary) = end_run(&mut shared, session);
    assert!(summary.clear_lamp.is_cleared(), "the fixture stage clears: {summary:?}");
    shared.submit_rx.take().expect("the stage was handed to the score server").recv().expect("the submission came back");
    assert_eq!(server.gauges.lock().expect("the gauge log").as_slice(), [rbms_ir::GaugeType::Class], "the stage is sent on the gauge it ran on");

    let (logs, best) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one stage was played and {} rows were stored", logs.len());
    };
    let class_id = rbms_store::scoredb::gauge_id("class");
    assert_eq!((log.clear, log.gauge), (clear_type_id(ClearType::LightAssistEasy), class_id), "the row names the course gauge");
    assert_eq!(log.assist_grade(), rbms_judge::AssistGrade::Clear.id(), "a CLASS clear is ASSIST CLEAR");
    assert_eq!(best.map(|best| best.assist_grade), Some(rbms_judge::AssistGrade::Clear.id()), "and so is the best it set");
}

/// A course with a stage judged wider is sent as a course too, saying so, and like the chart sets
/// no best.
#[test]
fn a_course_with_a_widened_stage_is_sent_saying_so_and_sets_no_best() {
    let (reasons, sets_best, course) = course_played("course-widened", false, || played_widened(&HAND_OFFSETS_US));
    assert_eq!(reasons, [None]);
    assert!(!sets_best, "a stage judged wider keeps the course from setting a best");
    assert!(!course.scratch_auto);
    assert_eq!(course.assist, [rbms_ir::mapping::CUSTOM_JUDGE_FLAG.to_owned()]);
}

/// A run judged wider than the chart asks is kept whole and sent, and still sets no best: its row
/// and its replay are kept, its lamp is the lower assisted one, and the best is left as it was.
#[test]
fn a_widened_judge_is_kept_and_sent_and_sets_no_best() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("widened", server.clone());
    let model = recorded_model();
    let (entered, summary) = end_run(&mut shared, played_widened(&HAND_OFFSETS_US));
    assert!(summary.ex_score > 0 && summary.clear_lamp.is_cleared(), "the fixture run scores and clears: {summary:?}");
    shared.submit_rx.take().expect("the run was handed to the score server").recv().expect("the submission came back");
    assert_eq!(server.assists.lock().expect("the assist log").as_slice(), [(false, vec![rbms_ir::mapping::CUSTOM_JUDGE_FLAG.to_owned()])]);
    let notices = result_notices(&entered, &shared);
    assert!(notices.iter().any(|line| line.contains("WIDENED JUDGE") && line.contains("NO BEST SCORE")), "{notices:?}");

    let (logs, best) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one run was played and {} rows were stored", logs.len());
    };
    assert_log_is_the_run(log, &summary, ClearType::AssistEasy);
    assert!(log.ir_submitted && log.replay_file.is_some(), "the run was sent and its replay kept");
    let best = best.expect("the chart has a row for its lamp");
    assert_eq!((best.clear, best.ex_score(), best.notes), (clear_type_id(ClearType::AssistEasy), 0, 0), "a widened run moved the best score");
    assert_eq!(shared.scores.best_ex_for_md5_in_ln_mode(&model.md5, SCORE_LN_MODE_FROM_CHART), None, "a widened run is no local best");
}

/// The rows a SCRATCH AUTO run left before its turntable stopped counting -- the player's 37 of
/// wave 16 -- counted the turntable as perfect greats played for the player. They stay in the
/// history with their lamps, and no best, LOCAL BEST or score graph is read off them: the first run
/// of the new rule is the chart's first best.
#[test]
fn a_scratch_auto_row_of_the_rule_before_sets_no_best_and_keeps_its_lamp() {
    let mut shared = signed_out("old-rule");
    shared.config.judge.target = ScoreTarget::LocalBest;
    let model = recorded_model();
    let old = PlayLog {
        chart_key: chart_key(&model.md5, ""),
        mode: mode_id(Mode::BEAT_7K.name),
        ln_mode: SCORE_LN_MODE_FROM_CHART.to_owned(),
        md5: model.md5.clone(),
        clear: clear_type_id(ClearType::LightAssistEasy),
        judge: rbms_store::scoredb::JudgeCounts::all_late([17, 0, 0, 0, 0, 0]),
        notes: 17,
        combo: 17,
        assist: i32::from(LIGHT_ASSIST),
        date: 1,
        rule_version: rbms_store::SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION - 1,
        ..PlayLog::default()
    };
    shared.scoredb.as_mut().expect("the score database").record_play(&old, false).expect("the old row writes");
    let settings = shared.settings_path.clone();
    drop(shared);

    let mut relaunched = crate::App::new(String::new(), Config::default(), LaunchOptions::default(), settings).shared;
    relaunched.config.play.autoplay = false;
    relaunched.config.judge.target = ScoreTarget::LocalBest;
    assert_eq!(relaunched.scores.for_md5(&model.md5).len(), 1, "the old row is in the history");
    assert_eq!(relaunched.scores.best_ex_for_md5_in_ln_mode(&model.md5, SCORE_LN_MODE_FROM_CHART), None, "it is no local best");
    assert_eq!(relaunched.scores.best_clear_for_md5(&model.md5), Some(clear_type_id(ClearType::LightAssistEasy)), "its lamp stands");

    relaunched.config.play.scratch_auto = true;
    let (entered, summary) = end_run(&mut relaunched, played_by_hand(&WEAKER_OFFSETS_US, true));
    assert_ne!(result_target_ex(&entered), Some(34), "the old row's EX was taken for a local best");
    let Transition::To(Stage::Result(result)) = &entered else {
        panic!("the run did not reach the result screen");
    };
    assert_eq!((result.view().prev_ex, result.view().prev_best_ex), (None, None), "the built-in screen compares the run with the old row");
    let (logs, best) = stored(&relaunched);
    assert_eq!(logs.len(), 2);
    let best = best.expect("the chart has a best");
    assert_eq!((best.ex_score(), best.notes, best.playcount), (summary.ex_score, summary.total_notes, 2), "the new run is the first best");
}

/// A run nobody played by hand is no record at all: one the engine played for the player leaves
/// the database as it was.
#[test]
fn a_run_the_engine_played_stores_nothing() {
    let mut shared = signed_out("autoplayed");
    shared.config.play.autoplay = true;
    let (entered, summary) = end_run(&mut shared, run_to_its_end(SessionOptions { autoplay: true, ..SessionOptions::default() }, &[]));
    assert!(matches!(entered, Transition::To(Stage::Result(_))) && summary.ex_score == summary.max_ex_score, "the engine plays every note: {summary:?}");
    assert_eq!(stored(&shared), (Vec::new(), None), "an autoplayed run was recorded");
    assert_eq!(shared.ir_status, IrStatus::Skipped("autoplay"));
}

/// A run that fails is a record like any other: it is a play and a row, its lamp is FAILED, and
/// the EX it reached stands as the best until a run beats it (`ScoreData.update` compares the EX
/// whatever the lamp). Its bad-poor count takes in every note the gauge left it short of, as the
/// reference records it (`BMSPlayer.java:909`), so dying early is never the better MISS COUNT.
#[test]
fn a_run_that_fails_is_stored_with_the_score_it_reached() {
    let mut shared = signed_out("failed");
    shared.config.play.gauge = rbms_judge::GaugeKind::ExHard;
    let session = played_on(rbms_judge::GaugeKind::ExHard, &FAILING_OFFSETS_US, false);
    assert!(session.is_failed(), "the fixture run runs out of gauge");
    let (_, summary) = end_run(&mut shared, session);
    assert!(summary.ex_score > 0 && !summary.clear_lamp.is_cleared(), "the fixture run scores and fails: {summary:?}");
    let unreached = summary.total_notes - summary.total_judged;
    assert!(unreached > 0, "the gauge ended the run before the chart did: {summary:?}");

    let (logs, best) = stored(&shared);
    let best = best.expect("the chart has a best");
    assert_eq!(logs.len(), 1);
    assert_eq!((best.clear, best.ex_score(), best.playcount, best.clearcount), (clear_type_id(ClearType::Failed), summary.ex_score, 1, 0));
    assert_eq!(logs[0].minbp, summary.min_bp + unreached, "the notes never reached are charged");
    assert_eq!(best.minbp, i64::from(summary.min_bp + unreached));
    let candidates = shared.scoredb.as_ref().expect("the database").sync_candidates(&walk("http://score-server.invalid")).expect("reads");
    let body = candidates[0].body.as_deref().and_then(ScoreSubmission::from_json).expect("the body the run is to be sent with is kept");
    assert_eq!((body.minbp, body.passnotes), (summary.min_bp + unreached, summary.total_judged));
}

/// What a run records of its timing is the reference's `avgjudge` (`BMSPlayer.java:911-930`): the
/// mean distance of every note from its input, a note never hit counting a second. Hits early and
/// late do not cancel out in it. How far the hits leaned rides beside it in the body, as does the
/// rule generation that says which `avgjudge` it is, and a sync that rebuilds the run from its row
/// sends the same `avgjudge` the result screen sent.
#[test]
fn a_stored_run_keeps_the_reference_average_duration() {
    let mut shared = signed_out("avgjudge");
    let session = played_by_hand(&HAND_OFFSETS_US, false);
    let record = session.record();
    assert!(record.avg_duration_us > record.summary.avg_judge_us.abs(), "early and late cancel out in the lean: {record:?}");
    let (_, summary) = end_run(&mut shared, session);

    let (logs, best) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one run was played and {} rows were stored", logs.len());
    };
    assert_eq!(log.avgjudge, record.avg_duration_us);
    assert_eq!(best.map(|best| best.avgjudge), Some(record.avg_duration_us));
    let candidates = shared.scoredb.as_ref().expect("the database").sync_candidates(&walk("http://score-server.invalid")).expect("reads");
    let body = candidates[0].body.as_deref().and_then(ScoreSubmission::from_json).expect("the body the run is to be sent with is kept");
    assert_eq!(body.judge.avgjudge, record.avg_duration_us);
    assert_eq!(body.mean_offset(), Some(summary.avg_judge_us), "the lean rides beside it");
    assert_eq!(body.rule_version(), Some(rbms_store::SCORE_RULE_VERSION));
    let rebuilt = crate::record_sync::rebuilt_submission(log, None);
    assert_eq!(rebuilt.judge.avgjudge, body.judge.avgjudge, "a rebuilt body sends what the result screen sent");
    assert_eq!(rebuilt.rule_version(), Some(rbms_store::SCORE_RULE_VERSION));
}

/// The streak a course stage is started on, carried from the stage before.
const CARRIED_COMBO: u32 = 100;

/// A course stage's chart record is the stage's own (`JudgeManager.java:668-676`): its max combo is
/// its own notes, not the course's streak carried into it, and a stage played without a break is
/// the full combo it is, whatever the course carried in.
#[test]
fn a_course_stage_records_its_own_combo_and_full_combo() {
    let mut shared = signed_out("course-stage-combo");
    let model = recorded_model();
    shared.course_run = Some(one_stage_course(model.md5.clone()));
    let course_gauge = Some((rbms_judge::gauge::GaugeIndex::Class, COURSE_OPENING_GAUGE));
    let options = SessionOptions { initial_gauge: course_gauge, initial_combo: CARRIED_COMBO, ..SessionOptions::default() };
    let session = run_model_to_its_end(model.clone(), options, &inputs_by_hand(&model, &[Some(0)], false));
    assert_eq!(session.standing_combo(), CARRIED_COMBO + session.summary().total_notes, "the course carries the whole streak on");
    let (_, summary) = end_run(&mut shared, session);
    assert_eq!(summary.counts[0], summary.total_notes, "every note on time is a PGREAT: {summary:?}");
    let (logs, _) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one stage was played and {} rows were stored", logs.len());
    };
    assert_eq!(log.combo, summary.total_notes, "the stage's max combo is its own");
    assert_eq!(log.clear, clear_type_id(ClearType::Max), "the stage is the MAX it was played as");
}

/// The TOTAL the run is played on in place of the chart's, well past what the chart asks.
const OWN_TOTAL: f64 = 5_000.0;

/// A run played on a TOTAL of the player's own is eased the way a widened judge eases it (the
/// player's decision of 2026-10-15): it is kept and sent, its lamp is the assist lamp, it sets no
/// best, and its replay records the TOTAL so playback moves the gauge as the run did.
#[test]
fn a_run_with_a_total_override_is_assisted_and_never_a_best() {
    let mut shared = signed_out("own-total");
    shared.config.play.total_override = OWN_TOTAL;
    let model = recorded_model();
    let (entered, summary) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    assert!(summary.clear_lamp.is_cleared(), "the fixture run clears: {summary:?}");
    let notices = result_notices(&entered, &shared);
    assert!(notices.iter().any(|line| line.contains("OWN TOTAL") && line.contains("NO BEST SCORE")), "{notices:?}");

    let (logs, best) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one run was played and {} rows were stored", logs.len());
    };
    assert_eq!(log.clear, clear_type_id(ClearType::AssistEasy), "the lamp is the assist lamp");
    assert_eq!(log.assist, i32::from(rbms_ir::mapping::CUSTOM_JUDGE_ASSIST));
    assert_eq!(best.map(|best| best.ex_score()), Some(0), "the run is no best score");
    assert_eq!(shared.scores.best_ex_for_md5_in_ln_mode(&model.md5, SCORE_LN_MODE_FROM_CHART), None, "the run is no local best");

    shared.config.play.total_override = OWN_TOTAL;
    let session = played_by_hand(&HAND_OFFSETS_US, false);
    shared.mode = session.model().mode;
    shared.chart_path = RECORDED_CHART_NAME.to_owned();
    let summary = session.summary();
    let ended = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    let recorded = record_run(&ended, &mut shared, &summary, &ended.session.record());
    assert_eq!(recorded.sub.options.total_override, OWN_TOTAL, "the submission says so, which the server ranks as unranked");
    let replay = recorded.recorded_replay.expect("the run's replay is kept");
    assert_eq!(replay.judge.total_override, OWN_TOTAL, "the replay records the TOTAL it was played on");
    assert_eq!(recorded.replay_plan, ReplayState::Skipped, "the wire cannot state the TOTAL, so the replay is not uploaded");
    assert_eq!(crate::run_total_override(&Config::default(), Some(&replay)), OWN_TOTAL, "playback takes the TOTAL the replay recorded");
}

/// A replay is watched with the SCRATCH AUTO, the gauge and the arrangement it was recorded under,
/// and the player's own are what the settings file holds while it plays and what the settings hold
/// once it is over. Watching a replay recorded with SCRATCH AUTO on used to leave the switch on for
/// every run after it, which keeps those runs from setting a best.
#[test]
fn watching_a_replay_leaves_the_players_own_play_options_as_they_were() {
    let mut shared = signed_out("replay-options");
    let own = OwnPlayOptions { scratch_auto: false, gauge: rbms_judge::GaugeKind::Easy, random: rbms_chart::shuffle::NoteOption::Off };
    own.put_into(&mut shared.config.play);
    let recorded = OwnPlayOptions { scratch_auto: true, gauge: rbms_judge::GaugeKind::Hard, random: rbms_chart::shuffle::NoteOption::Mirror };
    let replay = |seed: u64| Replay {
        chart_path: RECORDED_CHART_NAME.to_owned(),
        md5: recorded_model().md5,
        mode: Mode::BEAT_7K.name.to_owned(),
        random: recorded.random.label().to_owned(),
        seed,
        offset_ms: 0,
        scratch_auto: recorded.scratch_auto,
        gauge: gauge_token(recorded.gauge).to_owned(),
        judge: Default::default(),
        events: Vec::new(),
    };
    shared.replay = Some(replay(1));
    assert_eq!(shared.play_as_recorded(), Some((recorded.random, 1)));
    shared.replay = Some(replay(2));
    assert_eq!(shared.play_as_recorded(), Some((recorded.random, 2)), "a second replay watched before the first ended");
    assert_eq!(OwnPlayOptions::of(&shared.config.play), recorded, "the replay is not watched as it was recorded");

    let session = played_by_hand(&HAND_OFFSETS_US, true);
    shared.mode = session.model().mode;
    shared.chart_path = RECORDED_CHART_NAME.to_owned();
    let mut watched = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    assert!(matches!(enter_result(&mut watched, &mut shared), Transition::To(Stage::Result(_))), "the replay did not reach the result screen");
    assert_eq!(stored(&shared), (Vec::new(), None), "a replay being watched was recorded");
    let written = rbms_config::load(&shared.settings_path).expect("the settings file reads back").config;
    assert_eq!(OwnPlayOptions::of(&written.play), own, "the result screen wrote the replay's options to the settings file");

    shared.end_replay();
    assert_eq!(OwnPlayOptions::of(&shared.config.play), own, "the replay's options outlived it");
    assert!(shared.replay.is_none());
    shared.end_replay();
    assert_eq!(OwnPlayOptions::of(&shared.config.play), own, "ending no replay moved the options");
}

/// The recorded chart with its one turntable note taken out, and its own name.
const NO_TURNTABLE_CHART: &[u8] =
    b"#PLAYER 1\n#TITLE No turntable\n#BPM 120\n#RANK 2\n#TOTAL 300\n#WAV01 a.wav\n#00111:0101010101010101\n#00211:0101010101010101\n#00319:01\n";

/// SCRATCH AUTO plays the turntable, and a chart with nothing on the turntable leaves it nothing to
/// play: the run is the player's own, as the reference counts it (`AutoplayModifier` raises the
/// assist only where it moved a note), so its lamp is not demoted and nothing is said of an assist.
/// The same switch on a chart with a turntable note is still an assist.
#[test]
fn scratch_auto_on_a_chart_with_nothing_on_the_turntable_is_no_assist() {
    let source = rbms_parser::parse_with(NO_TURNTABLE_CHART, Default::default());
    let model = rbms_chart::to_model(&source, Mode::BEAT_7K);
    let mut shared = signed_out("no-turntable");
    shared.config.play.scratch_auto = true;
    let mode = model.mode;
    let auto_lanes = (0..mode.key).map(|lane| mode.is_scratch(lane)).collect();
    let inputs = inputs_by_hand(&model, &HAND_OFFSETS_US, true);
    let session = run_model_to_its_end(model.clone(), SessionOptions { auto_lanes, ..SessionOptions::default() }, &inputs);
    assert!(!crate::app_result::scratch_auto_played(&session), "the fixture has a turntable note");
    assert!(
        crate::app_result::scratch_auto_played(&played_by_hand(&HAND_OFFSETS_US, true))
            && !crate::app_result::scratch_auto_played(&played_by_hand(&HAND_OFFSETS_US, false))
    );
    let summary = session.summary();
    shared.mode = mode;
    shared.chart_path = "no-turntable.bms".to_owned();
    shared.replay = None;
    let mut played = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    let entered = enter_result(&mut played, &mut shared);
    assert!(summary.ex_score > 0 && summary.clear_lamp.is_cleared(), "the fixture run scores and clears: {summary:?}");
    assert_eq!(shared.ir_status, IrStatus::Off, "the run was taken for an assisted one");
    assert_eq!(result_notices(&entered, &shared), Vec::<String>::new(), "the result screen speaks of an assist");
    assert_eq!(shared.scores.best_ex_for_md5_in_ln_mode(&model.md5, SCORE_LN_MODE_FROM_CHART), Some(summary.ex_score), "the run set no best");
    let db = shared.scoredb.as_ref().expect("the score database opened in the test's folder");
    let best = db.best(&chart_key(&model.md5, ""), mode_id(Mode::BEAT_7K.name)).expect("the best reads").expect("the chart has a best");
    assert_eq!((best.ex_score(), best.clear), (summary.ex_score, clear_type_id(summary.clear_lamp)));
}

/// Names a copy of a player's own score database the test below reads. The copy is copied again
/// into the test's own folder, so not even it is written.
const SCOREDB_COPY_ENV: &str = "RBMS_SCOREDB_COPY";

/// A player's own database, as this build reads it on its next launch: every SCRATCH AUTO row of the
/// rule before is history with its lamp, none is a best, and the first run of the new rule on one of
/// those charts is its best. Prints what it counted, for a person to read.
///
/// Run by hand with `--ignored` and the copy named in [`SCOREDB_COPY_ENV`]; a run without the copy
/// fails rather than passing over it.
#[test]
#[ignore = "reads a copy of a player's own score database named by RBMS_SCOREDB_COPY"]
fn a_players_own_database_reads_its_old_scratch_auto_rows_as_lamps_alone() {
    let copy = std::env::var_os(SCOREDB_COPY_ENV).expect("RBMS_SCOREDB_COPY names a copy of a score database");
    let settings = settings_in_own_folder("own-database");
    let folder = settings.parent().expect("the settings file is in a folder").to_path_buf();
    let (db_path, _, _) = crate::scoredb_store::score_paths(&folder);
    std::fs::copy(&copy, &db_path).expect("the copy is copied into the test's folder");
    let mut shared = crate::App::new(String::new(), Config::default(), LaunchOptions::default(), settings).shared;
    let db = shared.scoredb.as_ref().expect("the copy opens");
    let plays = db.all_plays().expect("the history reads");
    let old_rule = plays.iter().filter(|log| log.assist == i32::from(LIGHT_ASSIST) && log.rule_version < rbms_store::SCORE_RULE_VERSION).count();
    let charts: std::collections::BTreeSet<String> = plays.iter().map(|log| log.md5.clone()).collect();
    let with_best = charts.iter().filter(|md5| shared.scores.best_ex_for_md5(md5).is_some()).count();
    let with_lamp = charts.iter().filter(|md5| shared.scores.best_clear_for_md5(md5).is_some_and(|lamp| lamp > 0)).count();
    let scored_rows = charts.iter().filter_map(|md5| db.chart_best(&chart_key(md5, "")).ok().flatten()).filter(|best| best.ex_score > 0).count();
    println!("own database: {} plays, {old_rule} of them SCRATCH AUTO of the rule before, {} charts,", plays.len(), charts.len());
    println!("  {with_best} with a local best, {with_lamp} with a lamp, {scored_rows} best rows with a score");
    assert_eq!((with_best, scored_rows), (0, 0), "an old SCRATCH AUTO row was read as a score");
    assert_eq!(with_lamp, charts.len(), "a lamp was lost");

    let Some(first) = plays.iter().find(|log| log.assist == i32::from(LIGHT_ASSIST)).cloned() else {
        return;
    };
    let lamp_before = shared.scores.best_clear_for_md5(&first.md5);
    let new_run = PlayLog { date: first.date + 1, rule_version: rbms_store::SCORE_RULE_VERSION, replay_file: None, ..first.clone() };
    crate::scoredb_store::record_finished_play(shared.scoredb.as_mut().expect("the copy"), &new_run, true, None);
    shared.scores.push(crate::scoredb_store::record_of(&new_run));
    let best = shared.scoredb.as_ref().and_then(|db| db.best_in_ln_mode(&first.chart_key, first.mode, &first.ln_mode).ok().flatten()).expect("a best");
    println!("a new SCRATCH AUTO run on {}: best EX {} of {} notes, lamp {:?} -> {}", first.title, best.ex_score(), best.notes, lamp_before, best.clear);
    assert_eq!((best.ex_score(), best.notes), (new_run.judge.ex_score(), new_run.notes), "the new run is the chart's first best");
    assert_eq!(shared.scores.best_ex_for_md5(&first.md5), Some(new_run.judge.ex_score()));
}

/// How far into the recorded chart a run left mid-song is played: past its first measure of eighth
/// notes and short of the second, so some notes are judged and the rest are still to come.
const LEFT_AT_US: i64 = 3_000_000;

/// The recorded chart's run on `options`, with `inputs` handed to the engine as their times come,
/// stopped at `until_us` rather than at its end.
fn run_until(model: rbms_model::Model, options: SessionOptions, inputs: &[Input], until_us: i64) -> PlaySession {
    let mut session = PlaySession::new(model, options);
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

/// Press Escape on `session` as the play screen has it, on the ESCAPE setting a fresh install has,
/// and answer where it leaves the app.
fn leave_by_escape(shared: &mut AppShared, session: PlaySession) -> Transition {
    use crate::stage::StageHandler;
    shared.mode = session.model().mode;
    shared.chart_path = RECORDED_CHART_NAME.to_owned();
    let mut playing = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    let key = crate::stage::KeyInput { code: KeyCode::Escape, pressed: true, released: false, text: None };
    playing.handle_key(&mut crate::stage::FrameCtx { shared, now: std::time::Instant::now(), dt: 0.0 }, key)
}

/// A run left part-way ends on its result screen as a FAILED run (the player's decision of
/// 2026-10-23): what it judged stands, every note it never reached is a POOR let pass, and so the
/// judgements that took a note add up to the chart's. The screen a skin draws reads the same counts
/// the history row holds, and its replay and the body it is to be sent with are kept.
#[test]
fn leaving_mid_song_shows_a_failed_result_with_the_counts_it_keeps() {
    let mut shared = signed_out("left-mid-song");
    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    let summary = session.summary();
    assert!(summary.total_judged > 0 && summary.total_judged < summary.total_notes, "the fixture run is left part-way: {summary:?}");
    let left = leave_by_escape(&mut shared, session);
    assert!(matches!(left, Transition::To(Stage::Result(_))), "Escape ends the run on its result screen");

    let (logs, best) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one run was left and {} rows were stored", logs.len());
    };
    let unreached = summary.total_notes - summary.total_judged;
    assert_eq!(log.clear, clear_type_id(ClearType::Failed));
    assert_eq!(log.judge.tier(rbms_store::scoredb::JUDGE_POOR), summary.counts[4] + unreached, "every note not reached is a POOR");
    assert_eq!((0..=rbms_store::scoredb::JUDGE_POOR).map(|tier| log.judge.tier(tier)).sum::<u32>(), summary.total_notes);
    assert_eq!((log.judge.ex_score(), log.notes, log.minbp), (summary.ex_score, summary.total_notes, summary.min_bp + unreached));
    assert!(log.replay_file.is_some(), "the replay of what was played is kept");
    assert_eq!(best.map(|best| best.playcount), Some(1));
    let candidates = shared.scoredb.as_ref().expect("the database").sync_candidates(&walk("http://score-server.invalid")).expect("reads");
    let body = candidates[0].body.as_deref().and_then(ScoreSubmission::from_json).expect("the body the run is to be sent with is kept");
    assert_eq!((body.clear, body.judge.poor, body.passnotes), (rbms_ir::ClearLamp::Failed, summary.counts[4] + unreached, summary.total_notes));
    assert_eq!(shared.scores.for_md5(&model.md5)[0].log_id, Some(log.id), "the book knows the row");

    let shown = shared.run_records.last().expect("the result screen has the run's record");
    let shown_tiers = shown.summary.counts[..=rbms_store::scoredb::JUDGE_POOR].to_vec();
    let kept_tiers: Vec<u32> = (0..=rbms_store::scoredb::JUDGE_POOR).map(|tier| log.judge.tier(tier)).collect();
    assert_eq!(shown_tiers, kept_tiers, "the result screen shows the counts the history keeps");
    assert_eq!(shown.summary.counts[4], summary.counts[4] + unreached);
    assert!(shown.summary.failed && shown.failed_at_ms.is_none(), "the run is FAILED with no empty gauge to have ended it");
    assert_eq!(shown.gauge_log.last(shown.finished_gauge), Some(0.0), "the gauge graph falls to zero where the run was left");
}

/// A run left signed in is sent once, by its result screen's submission: no records sync of it is
/// started besides, and the server's answer is written down against it.
#[test]
fn leaving_mid_song_signed_in_sends_the_run_once_from_its_result() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("left-signed-in", server.clone());
    shared.record_sync_pacing = crate::record_sync::SyncPacing {
        score_interval: Duration::ZERO,
        replay_interval: Duration::ZERO,
        trace_interval: Duration::ZERO,
        rate_limit_wait: Duration::ZERO,
        cancel_check: Duration::from_millis(1),
    };
    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    let left = leave_by_escape(&mut shared, session);
    assert!(matches!(left, Transition::To(Stage::Result(_))), "Escape ends the run on its result screen");
    assert!(shared.submit_rx.is_some(), "the result screen sends the run");
    assert!(shared.record_sync.is_none(), "a records sync of the run was started besides");
    let deadline = std::time::Instant::now() + COURSE_SUBMIT_WAIT;
    while shared.submit_rx.is_some() || shared.record_sync.is_some() {
        assert!(std::time::Instant::now() < deadline, "the run was not sent");
        shared.poll_ir_jobs(crate::stage::StageId::Result);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(server.sent_for.lock().expect("the submission log").as_slice(), [ACCOUNT_ID.to_owned()], "the run was sent once");
    let (logs, _) = stored(&shared);
    let server_key = crate::record_sync::server_key("http://score-server.invalid");
    let states = shared.scoredb.as_ref().expect("the database").sync_states(&server_key).expect("reads");
    assert_eq!(states[&logs[0].id].state, SyncState::Accepted);
}

/// A player who never signed in sees the result of a run they left too: it is kept, and nothing is
/// sent or set to be sent but by a later records sync.
#[test]
fn leaving_mid_song_signed_out_still_shows_its_result() {
    let mut shared = signed_out("left-signed-out");
    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    let left = leave_by_escape(&mut shared, session);
    assert!(matches!(left, Transition::To(Stage::Result(_))), "Escape ends the run on its result screen");
    assert_eq!(stored(&shared).0.len(), 1, "the run is kept");
    assert!(shared.submit_rx.is_none(), "a run was sent with no server to send it to");
    assert!(shared.record_sync.is_none(), "a records sync was started");
}

/// A run left before any note was judged was never played, and nothing is kept of it (the player's
/// decision of 2026-10-19, which replaces the one of 2026-10-15 that kept it as a FAILED run): no
/// row in the history, no best, no replay, and no records sync to send it.
#[test]
fn leaving_before_any_note_is_judged_keeps_nothing() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("left-at-once", server.clone());
    let session = run_until(recorded_model(), SessionOptions::default(), &[], 0);
    assert_eq!(session.summary().counts, [0; 6], "the fixture run has judged nothing");
    let left = leave_by_escape(&mut shared, session);
    assert!(!matches!(left, Transition::To(Stage::Result(_)) | Transition::Stay), "Escape still leaves at once");
    shared.finish_saves();
    assert_eq!(stored(&shared), (Vec::new(), None), "a run that judged nothing was kept");
    assert!(every_play(&shared).is_empty(), "a row was stored under some other key");
    assert_eq!(std::fs::read_dir(&shared.replay_dir).map_or(0, Iterator::count), 0, "a replay of a run that judged nothing was written");
    assert!(shared.record_sync.is_none(), "a records sync of the run was started");
    assert!(server.sent_for.lock().expect("the submission log").is_empty(), "the run was sent");
}

/// The chart's first key note: when it lands and the lane it is on.
fn first_key_note(model: &rbms_model::Model) -> (i64, usize) {
    model
        .timelines
        .iter()
        .flat_map(|line| {
            line.notes.iter().enumerate().filter(|(lane, note)| note.is_some() && !model.mode.is_scratch(*lane)).map(|(lane, _)| (line.time_us, lane))
        })
        .min()
        .expect("the chart has a key note")
}

/// How far ahead of a note a press lands in the 空POOR band: before the note's own windows, so it
/// judges no note.
const EMPTY_POOR_AHEAD_US: i64 = 300_000;

/// A press that reached no note is an empty POOR, which hits nothing: a run left after nothing but
/// one is a run the player hit nothing in, and is not kept (the player's decision of 2026-10-19).
#[test]
fn leaving_after_only_an_empty_poor_keeps_nothing() {
    let mut shared = signed_out("left-after-empty-poor");
    let model = recorded_model();
    let (note_us, lane) = first_key_note(&model);
    let pressed_us = note_us - EMPTY_POOR_AHEAD_US;
    let inputs = [(pressed_us, lane, true), (pressed_us + KEY_HELD_US, lane, false)];
    let session = run_until(model, SessionOptions::default(), &inputs, pressed_us + KEY_HELD_US);
    let summary = session.summary();
    assert_eq!(
        (summary.counts[5], summary.counts[..5].iter().sum::<u32>(), summary.total_judged),
        (1, 0, 0),
        "the run judged an empty POOR and no note: {summary:?}"
    );
    leave_by_escape(&mut shared, session);
    shared.finish_saves();
    assert_eq!(stored(&shared), (Vec::new(), None), "a run with only an empty POOR was kept");
    assert!(every_play(&shared).is_empty(), "a row was stored under some other key");
}

/// A note that went by unhit is a POOR the player did not hit, so a run left after notes went by
/// and nothing was hit is not kept either (the player's decision of 2026-10-19).
#[test]
fn leaving_after_a_note_went_by_unhit_keeps_nothing() {
    let mut shared = signed_out("left-after-a-miss");
    let session = run_until(recorded_model(), SessionOptions::default(), &[], LEFT_AT_US);
    let summary = session.summary();
    assert_eq!((summary.counts[..4].iter().sum::<u32>(), summary.counts[4] > 0), (0, true), "notes went by unhit and none was hit: {summary:?}");
    assert!(summary.total_judged < summary.total_notes, "the fixture run is left part-way: {summary:?}");
    leave_by_escape(&mut shared, session);
    shared.finish_saves();
    assert_eq!(stored(&shared), (Vec::new(), None), "a run that hit nothing was kept");
    assert!(every_play(&shared).is_empty(), "a row was stored under some other key");
}

/// A plain long note's head counts no judgment until its end is judged, but taking it is a hit: a
/// run left while holding the chart's first long note is kept as a FAILED run.
#[test]
fn leaving_while_holding_the_first_long_note_keeps_a_failed_run() {
    let chart = b"#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#00251:01000001\n";
    let model = rbms_chart::to_model(&rbms_parser::parse_with(chart, Default::default()), Mode::BEAT_7K);
    let (head_us, lane) = first_key_note(&model);
    let mut shared = signed_out("left-holding-first-long-note");
    let session = run_until(model, SessionOptions::default(), &[(head_us, lane, true)], head_us + LEFT_WHILE_HOLDING_US);
    let summary = session.summary();
    assert_eq!(summary.counts, [0; 6], "a plain long note's head judges nothing while it is held: {summary:?}");
    leave_by_escape(&mut shared, session);
    let logs = shared.scoredb.as_ref().expect("the database").all_plays().expect("the history reads");
    let [log] = logs.as_slice() else {
        panic!("one run was left and {} rows were stored", logs.len());
    };
    assert_eq!(log.clear, clear_type_id(ClearType::Failed));
}

/// A chart with no note to judge has nothing left to hit from the start: a turntable-only chart
/// played on SCRATCH AUTO, which never ends by itself, is ended by Escape as a run played to its
/// end, on its result screen, and not kept as a FAILED run (`BMSPlayer.java:953-958`: `getPastNotes()
/// == getNotes()` finishes it).
#[test]
fn leaving_a_chart_with_no_note_to_judge_ends_it_on_its_result() {
    let mut shared = signed_out("left-no-notes");
    let model = rbms_chart::to_model(&rbms_parser::parse_with(b"#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#00116:01\n", Default::default()), Mode::BEAT_7K);
    let auto_lanes = (0..Mode::BEAT_7K.key).map(|lane| Mode::BEAT_7K.is_scratch(lane)).collect();
    let session = run_until(model, SessionOptions { auto_lanes, ..SessionOptions::default() }, &[], LEFT_AT_US);
    assert_eq!(session.summary().total_notes, 0, "SCRATCH AUTO took the chart's only note");
    let left = leave_by_escape(&mut shared, session);
    assert!(matches!(left, Transition::To(Stage::Result(_))), "the run ended on its result screen");
    let logs = shared.scoredb.as_ref().expect("the database").all_plays().expect("the history reads");
    let [log] = logs.as_slice() else {
        panic!("one run was played and {} rows were stored", logs.len());
    };
    assert_eq!(log.clear, clear_type_id(ClearType::NoPlay), "the run was judged as played, with no note to earn a lamp, and not left as FAILED");
}

/// A long note one end of which was still to be judged when the run was left counts that end once,
/// whether the long note is judged once (plain) or at both ends (charge). A note before it is hit, so
/// the run judged something and is kept.
#[test]
fn leaving_while_holding_a_long_note_counts_it_once() {
    let chart = b"#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#00111:01\n#00251:01000001\n#00411:01\n";
    let model = rbms_chart::to_model(&rbms_parser::parse_with(chart, Default::default()), Mode::BEAT_7K);
    let notes: Vec<(i64, usize)> = model
        .timelines
        .iter()
        .flat_map(|line| line.notes.iter().enumerate().filter(|(_, note)| note.is_some()).map(move |(lane, _)| (line.time_us, lane)))
        .collect();
    let [(first_us, first_lane), (head_us, head_lane), ..] = notes.as_slice() else {
        panic!("the chart has a note and a long note: {notes:?}");
    };
    let inputs = [(*first_us, *first_lane, true), (*first_us + KEY_HELD_US, *first_lane, false), (*head_us, *head_lane, true)];
    for ln_mode in [rbms_judge::ln::LnMode::LongNote, rbms_judge::ln::LnMode::ChargeNote] {
        let mut shared = signed_out(&format!("left-holding-{ln_mode:?}"));
        let mut session = PlaySession::new(model.clone(), SessionOptions::default());
        session.set_judge_setup(rbms_play::JudgeSetup { ln_mode, ..rbms_play::JudgeSetup::default() });
        let mut next = 0;
        for frame in 0..=(head_us + LEFT_WHILE_HOLDING_US) / RUN_FRAME_US {
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
        let summary = session.summary();
        assert!(summary.total_judged > 0 && summary.total_judged < summary.total_notes, "{ln_mode:?}: the long note is still held: {summary:?}");
        leave_by_escape(&mut shared, session);
        let logs = shared.scoredb.as_ref().expect("the database").all_plays().expect("the history reads");
        let [log] = logs.as_slice() else {
            panic!("{ln_mode:?}: one run was left and {} rows were stored", logs.len());
        };
        let taken: u32 = (0..=rbms_store::scoredb::JUDGE_POOR).map(|tier| log.judge.tier(tier)).sum();
        assert_eq!(taken, summary.total_notes, "{ln_mode:?}: every judged object is counted once");
    }
}

/// How long after a long note's head the run holding it is left: inside the long note.
const LEFT_WHILE_HOLDING_US: i64 = 100_000;

/// A run nobody played by hand is no record whether it ends or is left: an autoplay run and a
/// replay being watched leave the database as it was.
#[test]
fn leaving_a_watched_replay_or_autoplay_keeps_nothing() {
    let mut shared = signed_out("left-autoplay");
    shared.config.play.autoplay = true;
    let session = run_until(recorded_model(), SessionOptions { autoplay: true, ..SessionOptions::default() }, &[], LEFT_AT_US);
    assert!(session.summary().total_judged > 0);
    let left = leave_by_escape(&mut shared, session);
    assert!(!matches!(left, Transition::To(Stage::Result(_)) | Transition::Stay), "an autoplay run left part-way has no result screen");
    assert_eq!(stored(&shared), (Vec::new(), None), "an autoplay run was kept");

    let mut shared = signed_out("left-replay");
    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    shared.replay = Some(Replay {
        chart_path: RECORDED_CHART_NAME.to_owned(),
        md5: model.md5.clone(),
        mode: Mode::BEAT_7K.name.to_owned(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    });
    let left = leave_by_escape(&mut shared, session);
    assert!(!matches!(left, Transition::To(Stage::Result(_)) | Transition::Stay), "a replay left part-way has no result screen");
    assert_eq!(stored(&shared), (Vec::new(), None), "a replay being watched was kept");
}

/// A course left part-way with notes still to come keeps everything that was played (the player's
/// decision of 2026-10-15) and ends on the course's result (the player's decision of 2026-10-23): the
/// stage it was left on is kept and sent as a FAILED chart, and the course fails with it, kept and
/// sent as a FAILED course with the stage finished before it and the one left folded in.
#[test]
fn leaving_mid_course_ends_on_a_failed_course_result() {
    let server = Arc::new(AcceptingServer::default());
    let (courses, sent) = channel();
    *server.courses.lock().expect("the course log") = Some(courses);
    let mut shared = signed_in("left-course", server.clone());
    let model = recorded_model();
    let first = played_by_hand(&HAND_OFFSETS_US, false).summary();
    let mut run = two_stage_course(model.md5.clone());
    let finished_first = rbms_course::StageResult {
        ex_score: first.ex_score,
        max_ex_score: first.max_ex_score,
        notes: first.total_notes,
        counts: first.counts,
        max_combo: first.max_combo,
        gauge_value: first.gauge_value,
        clear: clear_type_id(first.clear_lamp),
        survived: true,
        ..rbms_course::StageResult::default()
    };
    assert_eq!(run.advance(&finished_first), rbms_course::CourseStep::Next, "the first stage was cleared");
    shared.course_run = Some(run);
    shared.course_stage_reasons = vec![None];

    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    let left = session.summary();
    assert!(left.total_judged < left.total_notes, "the second stage is left part-way: {left:?}");
    let leaving = leave_by_escape(&mut shared, session);
    assert!(matches!(leaving, Transition::To(Stage::CourseResult(_))), "Escape ends the course on its result");
    assert!(shared.course_run.as_ref().is_some_and(|run| run.failed_at.is_some()), "the course result holds the failed course");
    assert!(shared.record_sync.is_none(), "a records sync of the stage was started besides its submission");

    let (logs, _) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one stage was left and {} rows were stored", logs.len());
    };
    assert_eq!(log.clear, clear_type_id(ClearType::Failed), "the stage left part-way is a FAILED chart");
    assert_eq!(log.minbp, left.min_bp_with_unreached(), "every note it never reached is charged");

    let course = sent.recv_timeout(COURSE_SUBMIT_WAIT).expect("the course was sent as a course");
    let unreached = left.total_notes - left.total_judged;
    assert_eq!(course.clear, rbms_ir::ClearLamp::Failed);
    assert_eq!(course.judge.pgreat, first.counts[0] + left.counts[0], "both stages are folded into the course");
    assert_eq!(course.judge.poor, first.counts[4] + left.counts[4] + unreached, "the notes left are POORs of the course too");
    assert!(shared.scoredb.as_ref().expect("the database").newest_course_id().expect("reads").is_some(), "the course is kept");
    let deadline = std::time::Instant::now() + COURSE_SUBMIT_WAIT;
    while shared.submit_rx.is_some() {
        assert!(std::time::Instant::now() < deadline, "the stage was not sent");
        shared.poll_ir_jobs(crate::stage::StageId::CourseResult);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(server.sent_for.lock().expect("the submission log").as_slice(), [ACCOUNT_ID.to_owned()], "the stage was sent once");
    assert!(sent.try_recv().is_err(), "the course was sent twice");
}

/// A course stage left with nothing more to hit is not abandoned: it ends as a stage played to its
/// end does, which for the last stage is the course's result.
#[test]
fn leaving_a_course_stage_with_no_note_left_ends_it_as_played() {
    let mut shared = signed_out("left-course-resolved");
    let model = recorded_model();
    shared.course_run = Some(one_stage_course(model.md5.clone()));
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_RESOLVED_AT_US);
    assert!(session.all_notes_resolved(), "every note of the stage has gone by");
    let left = leave_by_escape(&mut shared, session);
    assert!(matches!(left, Transition::To(Stage::CourseResult(_))), "the course ended on its result");
    let (logs, _) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one stage was played and {} rows were stored", logs.len());
    };
    assert_ne!(log.clear, clear_type_id(ClearType::Failed), "the stage was judged as played, not left");
}

/// How far into the recorded chart a stage is left once every note has gone by: a second past its
/// last note and short of the end of its playing time.
const LEFT_RESOLVED_AT_US: i64 = 7_000_000;

/// A course of the recorded chart twice over, which is what the course left part-way is.
fn two_stage_course(md5: String) -> rbms_course::CourseRun {
    let stage = rbms_course::CourseChart { md5, sha256: String::new(), title: "Recorded".to_owned() };
    let course = rbms_course::Course { name: "RECORDED TWICE".to_owned(), charts: vec![stage.clone(), stage], ..rbms_course::Course::default() };
    rbms_course::CourseRun::new(course, COURSE_OPENING_GAUGE)
}

/// A run that ends keeps the body it is sent with, under the account it was played under, so a
/// later sync sends exactly what the result screen sent.
#[test]
fn a_finished_run_stores_its_submission_body() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("body", server);
    let (_, summary) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    let db = shared.scoredb.as_ref().expect("the database");
    let candidates = db.sync_candidates(&walk("http://elsewhere.invalid")).expect("reads");
    let [candidate] = candidates.as_slice() else {
        panic!("one run was played and {} were candidates", candidates.len());
    };
    assert_eq!(candidate.played_as.as_deref(), Some(ACCOUNT_ID));
    let body = candidate.body.as_deref().and_then(ScoreSubmission::from_json).expect("the body reads back");
    assert_eq!((body.ex_score, body.total_notes, body.player.id.as_str()), (summary.ex_score, summary.total_notes, ACCOUNT_ID));
    assert!(!body.is_reconstructed());

    let mut guest = signed_out("body-guest");
    end_run(&mut guest, played_by_hand(&HAND_OFFSETS_US, false));
    let candidates = guest.scoredb.as_ref().expect("the database").sync_candidates(&walk("http://elsewhere.invalid")).expect("reads");
    assert_eq!(candidates[0].played_as, None, "a guest's run is kept for whoever signs in");
    assert!(candidates[0].body.is_some());
}

/// The result screen's submission writes what the server answered against the run, so the next
/// records sync leaves a run the server took alone.
#[test]
fn a_live_submission_writes_its_sync_state() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("live-state", server.clone());
    end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    let deadline = std::time::Instant::now() + COURSE_SUBMIT_WAIT;
    while shared.submit_rx.is_some() {
        assert!(std::time::Instant::now() < deadline, "the submission did not come back");
        shared.poll_ir_jobs(crate::stage::StageId::Result);
        std::thread::sleep(Duration::from_millis(1));
    }
    let (logs, _) = stored(&shared);
    let server_key = crate::record_sync::server_key("http://score-server.invalid");
    let states = shared.scoredb.as_ref().expect("the database").sync_states(&server_key).expect("reads");
    let row = &states[&logs[0].id];
    assert_eq!((row.state, row.login_id.as_str(), row.attempts), (rbms_store::scoredb::SyncState::Accepted, ACCOUNT_ID, 1));
    assert_eq!(row.replay_state, rbms_store::scoredb::ReplayState::Failed, "the upload this server did not take goes up on a later sync");
    assert_eq!(shared.record_sync_states.get(&logs[0].id).map(|row| row.state), Some(rbms_store::scoredb::SyncState::Accepted));
    let db = shared.scoredb.as_ref().expect("the database");
    assert!(db.sync_candidates(&walk(&server_key)).expect("reads").is_empty(), "the next sync sends nothing");
}

/// How long a test waits for what it sent to come back.
const ANSWER_WAIT: Duration = Duration::from_secs(10);

/// Poll the app as the frame loop does until `done` says so.
fn poll_until(shared: &mut AppShared, stage: crate::stage::StageId, done: impl Fn(&AppShared) -> bool) {
    let deadline = std::time::Instant::now() + ANSWER_WAIT;
    while !done(shared) {
        assert!(std::time::Instant::now() < deadline, "what was sent did not come back");
        shared.poll_ir_jobs(stage);
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// A sync that sends at once.
fn unpaced() -> crate::record_sync::SyncPacing {
    crate::record_sync::SyncPacing {
        score_interval: Duration::ZERO,
        replay_interval: Duration::ZERO,
        trace_interval: Duration::ZERO,
        rate_limit_wait: Duration::ZERO,
        cancel_check: Duration::from_millis(1),
    }
}

/// A replay whose folder cannot be written is not claimed: the row offers no replay, the player is
/// told, and the run is still sent with the replay it played, which the result screen's submission
/// carries from memory.
#[test]
fn a_replay_that_cannot_be_written_leaves_no_replay_on_the_row() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("replay-unwritable", server);
    shared.config.network.auto_upload_replay = true;
    let in_the_way = shared.replay_dir.clone();
    std::fs::create_dir_all(in_the_way.parent().expect("the replay folder has a parent")).expect("the test folder");
    std::fs::write(&in_the_way, "a file where the replay folder should be").expect("written");
    let notices = crate::notify::exclusive(|| {
        end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
        let mut out = Vec::new();
        crate::notify::drain(&mut out);
        out
    });
    assert!(notices.iter().any(|(level, line)| *level == crate::notify::Level::Error && line.starts_with("replay not saved")), "{notices:?}");
    let (logs, _) = stored(&shared);
    assert_eq!(logs[0].replay_file, None, "the row offers no replay that is not there");
    assert_eq!(shared.scores.for_md5(&recorded_model().md5)[0].replay_file, None);
    poll_until(&mut shared, crate::stage::StageId::Result, |shared| shared.submit_rx.is_none());
    let server_key = crate::record_sync::server_key("http://score-server.invalid");
    let row = &shared.scoredb.as_ref().expect("the database").sync_states(&server_key).expect("reads")[&logs[0].id];
    assert_eq!(row.replay_state, ReplayState::Failed, "the replay played was offered to the server from memory");
}

/// A replay whose write fails once it has been handed to the save thread -- the disk filled, the
/// folder went -- is let go of once the thread says so: the row and the book stop offering it.
#[test]
fn a_replay_the_save_thread_could_not_write_is_let_go() {
    let mut shared = signed_out("replay-write-failed");
    let (gate, held) = std::sync::mpsc::channel();
    shared.saves = crate::save_worker::SaveWorker::spawn_gated(held);
    end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    let (logs, _) = stored(&shared);
    assert!(logs[0].replay_file.is_some(), "the replay is on its way");
    std::fs::remove_dir_all(&shared.replay_dir).expect("the replay folder goes before the write lands");
    drop(gate);
    shared.saves.flush();
    crate::notify::exclusive(|| shared.poll_saves());
    let (logs, _) = stored(&shared);
    assert_eq!(logs[0].replay_file, None);
    assert_eq!(shared.scores.for_md5(&recorded_model().md5)[0].replay_file, None);
}

/// Neither the result screen's replay, its settings nor the run's trace are written on the frame
/// thread: all three are handed to the save thread, and land once it gets to them. The run's row is
/// in the history already.
#[test]
fn entering_the_result_writes_no_file_on_the_frame_thread() {
    let mut shared = signed_out("result-no-frame-write");
    let (gate, held) = std::sync::mpsc::channel();
    shared.saves = crate::save_worker::SaveWorker::spawn_gated(held);
    let _ = std::fs::remove_file(&shared.settings_path);
    end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    let (logs, _) = stored(&shared);
    let replay = shared.replay_dir.join(logs[0].replay_file.as_deref().expect("a replay is named"));
    assert!(!replay.exists() && !shared.settings_path.exists(), "a file was written before the save thread got to it");
    let trace_of_the_run = |shared: &AppShared| shared.scoredb.as_ref().expect("the score database").trace_body(logs[0].id).expect("reads");
    assert_eq!(trace_of_the_run(&shared), None, "the trace was written before the save thread got to it");
    drop(gate);
    shared.saves.flush();
    assert!(replay.exists() && shared.settings_path.exists(), "the save thread wrote both");
    Replay::load(&replay).expect("the replay reads back whole");
    assert!(trace_of_the_run(&shared).as_deref().and_then(rbms_ir::PlayTrace::from_json).is_some(), "the save thread kept the trace");
}

/// A run played while the score database cannot take it is kept on the side, its replay with it,
/// and read in the next time the database opens; the player is told where it went.
#[test]
fn a_run_played_while_the_score_database_is_unavailable_is_kept_and_read_in_later() {
    let mut shared = signed_out("database-closed");
    shared.scoredb = None;
    let notices = crate::notify::exclusive(|| {
        end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
        let mut out = Vec::new();
        crate::notify::drain(&mut out);
        out
    });
    assert!(notices.iter().any(|(_, line)| line == crate::app_result::RUN_KEPT_ON_THE_SIDE), "{notices:?}");
    let pending = crate::scoredb_store::pending_runs_dir(shared.settings_path.parent().expect("a folder"));
    assert_eq!(std::fs::read_dir(&pending).expect("the run is kept on the side").count(), 1);
    let replay = shared.scores.for_md5(&recorded_model().md5)[0].replay_file.clone().expect("the replay is named");
    assert!(shared.replay_dir.join(&replay).exists(), "the replay is written, not left behind");

    let settings_path = shared.settings_path.clone();
    drop(shared);
    let mut config = Config::default();
    config.play.autoplay = false;
    let reopened = crate::App::new(String::new(), config, LaunchOptions::default(), settings_path).shared;
    let (logs, _) = stored(&reopened);
    assert_eq!(logs.len(), 1, "the run kept on the side is in the database");
    assert_eq!(logs[0].replay_file.as_deref(), Some(replay.as_str()));
    assert_eq!(std::fs::read_dir(&pending).expect("the folder stays").count(), 0, "and its file is gone");
    assert_eq!(reopened.scores.for_md5(&recorded_model().md5).len(), 1, "the book reads it too");
}

/// The trace a stored run was kept with, read back.
fn kept_trace(shared: &AppShared, log_id: i64) -> rbms_ir::PlayTrace {
    let body = shared.scoredb.as_ref().expect("the score database").trace_body(log_id).expect("reads").expect("the run kept its trace");
    rbms_ir::PlayTrace::from_json(&body).expect("the kept trace reads as one")
}

/// What a kept trace's states say the run judged: per tier from PGREAT to POOR, every judged object
/// nothing consumed counted a POOR let pass, as the summary the run is kept with counts it.
fn traced_counts(trace: &rbms_ir::PlayTrace) -> [u32; 5] {
    let mut counts = [0; 5];
    for (state, kind) in trace.notes.state.iter().zip(&trace.notes.kind) {
        match (state, *kind == rbms_ir::TRACE_KIND_LONG_END) {
            (_, true) => {}
            (0, false) => counts[rbms_judge::Judge::Poor as usize] += 1,
            (state, false) => counts[usize::from(*state) - 1] += 1,
        }
    }
    counts
}

/// Every run kept is kept with its trace, written after its row, whether or not anybody is
/// signed in: one the result screen keeps and one left with Escape alike. No score server is asked
/// for anything to keep it.
#[test]
fn every_run_kept_keeps_its_trace_with_it() {
    let mut shared = signed_out("trace-kept");
    let (_, summary) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    let (logs, _) = stored(&shared);
    let finished = kept_trace(&shared, logs[0].id);
    assert_eq!(traced_counts(&finished)[..], summary.counts[..5], "the finished run's trace adds up to what it was kept as");
    assert_eq!(finished.chart.md5, recorded_model().md5);
    assert_eq!(finished.score_id, None, "nothing is linked before a server takes the score");

    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    let left = session.summary().left_unfinished();
    leave_by_escape(&mut shared, session);
    let (logs, _) = stored(&shared);
    assert_eq!(logs.len(), 2);
    let left_trace = kept_trace(&shared, logs[0].id);
    assert_eq!(traced_counts(&left_trace)[..], left.counts[..5], "the left run's trace adds up to the FAILED run it was kept as");
    assert!(left_trace.notes.state.contains(&0), "the notes it never reached are not judged in the trace");
}

/// A run the score database could not take carries its trace in the file it is kept in, and is
/// read in with it.
#[test]
fn a_run_kept_on_the_side_keeps_its_trace_in_its_file() {
    let mut shared = signed_out("trace-on-the-side");
    shared.scoredb = None;
    crate::notify::exclusive(|| {
        end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
        crate::notify::drain(&mut Vec::new());
    });
    let pending = crate::scoredb_store::pending_runs_dir(shared.settings_path.parent().expect("a folder"));
    let file = std::fs::read_dir(&pending).expect("the run is kept on the side").next().expect("one file").expect("an entry").path();
    let run = rbms_store::scoredb::PendingRun::from_ron(&std::fs::read_to_string(file).expect("reads")).expect("parses");
    assert!(run.trace.as_deref().and_then(rbms_ir::PlayTrace::from_json).is_some(), "the file holds the run's trace");
}

/// A run nobody played by hand keeps nothing, so it keeps no trace either.
#[test]
fn a_run_the_engine_played_keeps_no_trace() {
    let mut shared = signed_out("trace-autoplay");
    shared.config.play.autoplay = true;
    let session = run_to_its_end(SessionOptions { autoplay: true, ..SessionOptions::default() }, &[]);
    end_run(&mut shared, session);
    assert!(every_play(&shared).is_empty(), "an autoplay run is no record");
}

/// A run that failed and is started again at once with START or SELECT is kept first, as its result
/// screen would have kept it, and sent.
#[test]
fn retrying_a_failed_run_keeps_the_failed_run() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("retried", server.clone());
    shared.record_sync_pacing = unpaced();
    shared.config.play.gauge = rbms_judge::GaugeKind::ExHard;
    let session = played_on(rbms_judge::GaugeKind::ExHard, &FAILING_OFFSETS_US, false);
    let summary = session.summary();
    assert!(!summary.clear_lamp.is_cleared());
    shared.mode = session.model().mode;
    shared.chart_path = RECORDED_CHART_NAME.to_owned();
    let failed = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    crate::app_result::record_retried_run(&failed, &mut shared);
    let (logs, _) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one failed run was started again and {} rows were stored", logs.len());
    };
    assert_log_is_the_run(log, &summary, ClearType::Failed);
    assert!(log.replay_file.is_some());
    poll_until(&mut shared, crate::stage::StageId::Loading, |shared| shared.record_sync.is_none());
    assert_eq!(server.sent_for.lock().expect("the submission log").as_slice(), [ACCOUNT_ID.to_owned()]);
    assert_eq!(shared.record_sync_states[&log.id].state, SyncState::Accepted);

    let mut autoplay = signed_out("retried-autoplay");
    autoplay.config.play.autoplay = true;
    let session = played_on(rbms_judge::GaugeKind::ExHard, &FAILING_OFFSETS_US, false);
    let demo = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    crate::app_result::record_retried_run(&demo, &mut autoplay);
    assert_eq!(stored(&autoplay), (Vec::new(), None), "a run the engine played keeps nothing");
}

/// A course is kept in the score database when it ends, played signed out included, and a course
/// the server did not take is sent by the next records sync until it is held.
#[test]
fn a_finished_course_is_kept_even_when_the_server_does_not_answer() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_out("course-kept");
    shared.course_run = Some(one_stage_course(recorded_model().md5));
    shared.course_stage_reasons.clear();
    let (entered, _) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    assert!(matches!(entered, Transition::To(Stage::CourseResult(_))));
    let server_key = crate::record_sync::server_key("http://score-server.invalid");
    let db = shared.scoredb.as_ref().expect("the database");
    let kept = db.course_candidates(&walk(&server_key)).expect("reads");
    let [course] = kept.as_slice() else {
        panic!("one course was finished and {} were kept", kept.len());
    };
    assert_eq!(course.played_as, None, "a course finished signed out is kept for whoever signs in");
    let first_body = rbms_ir::CourseSubmission::from_json(&course.body).expect("the body reads back");

    let (tx, rx) = channel();
    *server.courses.lock().expect("the course log") = Some(tx);
    let mut config = Config::default();
    config.play.autoplay = false;
    config.network.server_url = Some("http://score-server.invalid".to_owned());
    let settings_path = shared.settings_path.clone();
    drop(shared);
    let mut signed = crate::App::new(String::new(), config, LaunchOptions::default(), settings_path).shared;
    signed.server = server.clone();
    signed.session = AccountSession::restored(Some(ACCOUNT_TOKEN.to_owned()), Some(ACCOUNT_ID.to_owned()));
    signed.record_sync_pacing = unpaced();
    assert!(signed.start_record_sync(crate::record_sync::SyncScope::All));
    poll_until(&mut signed, crate::stage::StageId::Select, |shared| shared.record_sync.is_none());
    let sent = rx.recv_timeout(ANSWER_WAIT).expect("the course was sent");
    assert_eq!((sent.played_at, sent.course_hash.as_str(), sent.player.id.as_str()), (first_body.played_at, first_body.course_hash.as_str(), ACCOUNT_ID));
    let states = signed.scoredb.as_ref().expect("the database").course_sync_states(&server_key).expect("reads");
    assert_eq!(states[&course.id].state, SyncState::Accepted);
    assert!(signed.scoredb.as_ref().expect("the database").course_candidates(&walk(&server_key)).expect("reads").is_empty(), "the server holds it now");
}

/// A course sent as it ends whose answer is a failure is written down to be sent again; the answer
/// is not let go of.
#[test]
fn a_course_the_server_failed_to_take_is_written_down_to_send_again() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("course-unanswered", server);
    shared.course_run = Some(one_stage_course(recorded_model().md5));
    shared.course_stage_reasons.clear();
    end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    assert_eq!(shared.course_submits.len(), 1, "the course is on its way");
    poll_until(&mut shared, crate::stage::StageId::CourseResult, |shared| shared.course_submits.is_empty());
    let server_key = crate::record_sync::server_key("http://score-server.invalid");
    let db = shared.scoredb.as_ref().expect("the database");
    let kept = db.course_candidates(&walk(&server_key)).expect("reads");
    assert_eq!(kept.len(), 1, "a course the server has no endpoint for is sent again later");
    let states = db.course_sync_states(&server_key).expect("reads");
    assert_eq!(states[&kept[0].id].state, SyncState::Skipped);
    assert_eq!(kept[0].played_as.as_deref(), Some(ACCOUNT_ID));
}
