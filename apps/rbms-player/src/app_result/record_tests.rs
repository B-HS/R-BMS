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
use std::sync::{Arc, Mutex};

use rbms_config::ScoreTarget;
use rbms_ir::{CourseSubmission, IrError, PlayerProfile, ReplayData, ServerInfo, SubmitResponse};
use rbms_store::scoredb::{BestScore, PlayLog, chart_key, mode_id};

use super::*;
use crate::app_play::OwnPlayOptions;
use crate::ir_session::AccountSession;

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
}

impl ScoreServer for AcceptingServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_score(&self, sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        self.sent_for.lock().expect("the submission log").push(sub.player.id.clone());
        self.assists.lock().expect("the assist log").push((sub.options.scratch_auto, sub.options.assist.clone()));
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
    fn submit_course(&self, _sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::Unsupported)
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

/// An app with a score server set up and an account signed in, whose server is `server`.
fn signed_in(tag: &str, server: Arc<AcceptingServer>) -> AppShared {
    let mut config = Config::default();
    config.play.autoplay = false;
    config.network.server_url = Some("http://score-server.invalid".to_owned());
    config.network.player_id = ACCOUNT_ID.to_owned();
    let mut shared = crate::App::new(String::new(), config, LaunchOptions::default(), settings_in_own_folder(tag)).shared;
    shared.server = server;
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

/// Every row the score database holds of the recorded chart's plays, newest first, and its best.
fn stored(shared: &AppShared) -> (Vec<PlayLog>, Option<BestScore>) {
    let db = shared.scoredb.as_ref().expect("the score database opened in the test's folder");
    let key = recorded_key();
    (db.history(&key, HISTORY_ROWS).expect("the history reads"), db.best(&key, mode_id(Mode::BEAT_7K.name)).expect("the best reads"))
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
/// combo, its bad-poor count, its note count and the lamp it was awarded.
fn assert_log_is_the_run(log: &PlayLog, summary: &rbms_play::PlaySummary, lamp: ClearType) {
    assert_eq!((log.judge.early, log.judge.late), (summary.early, summary.late), "the judgements");
    assert_eq!(log.judge.ex_score(), summary.ex_score, "the EX score");
    assert_eq!((log.combo, log.minbp, log.notes), (summary.max_combo, summary.min_bp, summary.total_notes), "the combo, the bad-poor count and the notes");
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

/// A course stage played with SCRATCH AUTO is kept and sent as a chart like any other, and keeps
/// its course from the course board, whose submission cannot say the stage had an assist. A stage
/// played without one holds nothing back.
#[test]
fn a_course_stage_played_with_scratch_auto_keeps_the_course_from_the_board() {
    let one_stage_course = |md5: String| {
        let course = rbms_course::Course {
            name: "RECORDED COURSE".to_owned(),
            charts: vec![rbms_course::CourseChart { md5, sha256: String::new(), title: "Recorded".to_owned() }],
            ..rbms_course::Course::default()
        };
        rbms_course::CourseRun::new(course, COURSE_OPENING_GAUGE)
    };
    for (scratch_auto, held_back) in [(false, false), (true, true)] {
        let server = Arc::new(AcceptingServer::default());
        let mut shared = signed_in(&format!("course-stage-{scratch_auto}"), server.clone());
        shared.config.play.scratch_auto = scratch_auto;
        shared.course_run = Some(one_stage_course(recorded_model().md5));
        let (entered, _) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, scratch_auto));
        assert!(matches!(entered, Transition::To(Stage::CourseResult(_))), "the one-stage course did not end on its result");
        shared.submit_rx.take().expect("the stage was handed to the score server").recv().expect("the submission came back");
        assert_eq!(server.sent_for.lock().expect("the submission log").len(), 1, "the stage was sent as a chart");
        let reasons = &shared.course_stage_reasons;
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons[0].is_some(), held_back, "SCRATCH AUTO {scratch_auto}: {reasons:?}");
    }
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
        rule_version: rbms_store::SCORE_RULE_VERSION - 1,
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
/// whatever the lamp).
#[test]
fn a_run_that_fails_is_stored_with_the_score_it_reached() {
    let mut shared = signed_out("failed");
    shared.config.play.gauge = rbms_judge::GaugeKind::ExHard;
    let (_, summary) = end_run(&mut shared, played_on(rbms_judge::GaugeKind::ExHard, &FAILING_OFFSETS_US, false));
    assert!(summary.ex_score > 0 && !summary.clear_lamp.is_cleared(), "the fixture run scores and fails: {summary:?}");

    let (logs, best) = stored(&shared);
    let best = best.expect("the chart has a best");
    assert_eq!(logs.len(), 1);
    assert_eq!((best.clear, best.ex_score(), best.playcount, best.clearcount), (clear_type_id(ClearType::Failed), summary.ex_score, 1, 0));
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
    crate::scoredb_store::record_finished_play(shared.scoredb.as_mut().expect("the copy"), &new_run, true);
    shared.scores.push(crate::scoredb_store::record_of(&new_run));
    let best = shared.scoredb.as_ref().and_then(|db| db.best_in_ln_mode(&first.chart_key, first.mode, &first.ln_mode).ok().flatten()).expect("a best");
    println!("a new SCRATCH AUTO run on {}: best EX {} of {} notes, lamp {:?} -> {}", first.title, best.ex_score(), best.notes, lamp_before, best.clear);
    assert_eq!((best.ex_score(), best.notes), (new_run.judge.ex_score(), new_run.notes), "the new run is the chart's first best");
    assert_eq!(shared.scores.best_ex_for_md5(&first.md5), Some(new_run.judge.ex_score()));
}
