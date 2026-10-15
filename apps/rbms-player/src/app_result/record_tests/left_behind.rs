//! What is kept of a run or a course that ends some other way than on its result screen: a course
//! left between two stages, the app quit in the middle of a run, a replay still being written when
//! the run it belongs to is sent, and the settings saved while a course narrows them.

use std::sync::mpsc::Receiver;

use super::*;
use crate::stage::{LoadingState, StageHandler};

/// The id the replay-taking server stores a score under, and the id it gives the replay.
const STORED_SCORE_ID: &str = "sc-left-behind";
const STORED_REPLAY_ID: &str = "rp-left-behind";

/// How long a test listens for a course submission that must not be made.
const NOTHING_SENT_WAIT: Duration = Duration::from_millis(200);

/// The hi-speed a course's NO SPEED constraint holds a run to, standing in for any setting a course
/// narrows.
const NARROWED_HISPEED: f64 = 1.0;

/// A score server that stores every score and takes every replay, without a network under it.
#[derive(Default)]
struct ReplayTakingServer {
    replays: Mutex<usize>,
}

impl ScoreServer for ReplayTakingServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        Err(IrError::Unsupported)
    }
    fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        Ok(SubmitResponse { accepted: true, score_id: Some(STORED_SCORE_ID.to_owned()), ..Default::default() })
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
        *self.replays.lock().expect("the replay count") += 1;
        Ok(STORED_REPLAY_ID.to_owned())
    }
}

/// An app signed in to `server`, whose save thread writes nothing until the returned sender is
/// dropped.
fn signed_in_with_held_saves(tag: &str, server: Arc<ReplayTakingServer>) -> (AppShared, Sender<()>) {
    let mut shared = signed_in(tag, Arc::new(AcceptingServer::default()));
    shared.server = server;
    shared.profile_servers = shared.multi_ir.build_servers_reusing(Some(shared.server.clone()));
    shared.record_sync_pacing = unpaced();
    let (release, held): (Sender<()>, Receiver<()>) = channel();
    shared.saves = crate::save_worker::SaveWorker::spawn_gated(held);
    (shared, release)
}

/// A run started again sends itself at once, while its replay is still waiting on the save thread.
/// The sync waits for the replay to be written rather than reading a file that is not there yet and
/// marking the replay missing for good: once the save thread gets to it, the replay is uploaded.
#[test]
fn a_replay_still_being_written_when_its_run_is_sent_is_uploaded_once_it_lands() {
    let server = Arc::new(ReplayTakingServer::default());
    let (mut shared, release) = signed_in_with_held_saves("replay-still-written", server.clone());
    shared.config.play.gauge = rbms_judge::GaugeKind::ExHard;
    let session = played_on(rbms_judge::GaugeKind::ExHard, &FAILING_OFFSETS_US, false);
    shared.mode = session.model().mode;
    shared.chart_path = RECORDED_CHART_NAME.to_owned();
    let failed = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    crate::app_result::record_retried_run(&failed, &mut shared);
    let (logs, _) = stored(&shared);
    let [log] = logs.as_slice() else {
        panic!("one run was started again and {} rows were stored", logs.len());
    };
    let replay = shared.replay_dir.join(log.replay_file.as_deref().expect("the run names its replay"));

    poll_until(&mut shared, crate::stage::StageId::Loading, |shared| {
        shared.record_sync_states.get(&log.id).is_some_and(|row| row.state == SyncState::Accepted)
    });
    assert!(!replay.exists(), "the fixture holds the replay on the save thread until the score is taken");
    drop(release);
    poll_until(&mut shared, crate::stage::StageId::Loading, |shared| shared.record_sync.is_none());

    assert!(replay.exists(), "the replay was written");
    assert_eq!(shared.record_sync_states[&log.id].replay_state, ReplayState::Uploaded, "the replay was read before it was written");
    assert_eq!(*server.replays.lock().expect("the replay count"), 1);
}

/// The player's own play settings, and the course of the recorded chart twice over standing on its
/// second stage with the first cleared, its constraints having narrowed them.
fn on_the_second_stage_of_a_course(shared: &mut AppShared) -> (rbms_config::PlayOptions, rbms_play::PlaySummary) {
    let own = shared.config.play.clone();
    let model = recorded_model();
    let first = played_by_hand(&HAND_OFFSETS_US, false).summary();
    let mut run = two_stage_course(model.md5.clone());
    run.course.release = true;
    let cleared = rbms_course::StageResult {
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
    assert_eq!(run.advance(&cleared), rbms_course::CourseStep::Next, "the first stage was cleared");
    shared.course_run = Some(run);
    shared.course_stage_reasons = vec![None];
    shared.course_settings_backup = Some(own.clone());
    shared.config.play.hispeed = NARROWED_HISPEED;
    (own, first)
}

/// The courses the score database holds, newest first, as the records sync would send them.
fn kept_courses(shared: &AppShared) -> Vec<CourseSubmission> {
    let db = shared.scoredb.as_ref().expect("the score database");
    let mut courses = db.course_candidates(&walk("")).expect("the courses read");
    courses.sort_by_key(|course| std::cmp::Reverse(course.id));
    courses.iter().map(|course| CourseSubmission::from_json(&course.body).expect("a kept course reads")).collect()
}

/// Escape on the LOADING screen of a course's next stage ends the course: the next chart the player
/// picks is played as a chart of its own and not folded into the course left behind, and the
/// settings the course narrowed are the player's own again. What was played of the course is kept
/// as a FAILED course, as a course left part-way with Escape is.
#[test]
fn escape_on_the_loading_screen_of_a_course_stage_ends_the_course() {
    let mut shared = signed_out("loading-escape-course");
    let (own, first) = on_the_second_stage_of_a_course(&mut shared);
    let mut loading = LoadingState::song(0);
    let key = crate::stage::KeyInput { code: KeyCode::Escape, pressed: true, released: false, text: None };
    let left = loading.handle_key(&mut crate::stage::FrameCtx { shared: &mut shared, now: std::time::Instant::now(), dt: 0.0 }, key);

    assert!(matches!(left, Transition::Back), "Escape leaves the LOADING screen");
    assert!(shared.course_run.is_none() && shared.course_settings_backup.is_none(), "the course was left standing");
    assert_eq!(shared.config.play, own, "the settings the course narrowed were kept");
    let courses = kept_courses(&shared);
    let [course] = courses.as_slice() else {
        panic!("one course was played part-way and {} were kept", courses.len());
    };
    assert_eq!(course.clear, rbms_ir::ClearLamp::Failed, "a course ended before its last stage is a FAILED course");
    assert_eq!(course.judge.pgreat, first.counts[0], "the stage played is the course's");
    assert_eq!(course.ex_score, first.ex_score, "the stage not played adds nothing");

    let (next, _) = end_run(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    assert!(matches!(next, Transition::To(Stage::Result(_))), "the next chart played was folded into the course left behind");
    assert_eq!(kept_courses(&shared).len(), 1, "a second course was kept out of a single chart");
}

/// A course whose next stage is not in the library ends there, as one left with Escape does.
#[test]
fn a_course_stage_the_library_does_not_hold_ends_the_course() {
    let mut shared = signed_out("course-stage-missing");
    let (own, _) = on_the_second_stage_of_a_course(&mut shared);
    let left = crate::load_course_stage(&mut shared);
    assert!(matches!(left, Transition::Back));
    assert!(shared.course_run.is_none(), "the course was left standing for the next chart picked");
    assert_eq!(shared.config.play, own);
    assert_eq!(kept_courses(&shared).len(), 1, "what was played of the course was not kept");
}

/// A course left on its first stage before anything was played keeps nothing.
#[test]
fn a_course_left_before_its_first_stage_keeps_nothing() {
    let mut shared = signed_out("course-left-unplayed");
    let model = recorded_model();
    shared.course_run = Some(two_stage_course(model.md5.clone()));
    crate::app_result::abandon_course(&mut shared, crate::app_result::Sending::Now);
    assert!(shared.course_run.is_none());
    assert!(kept_courses(&shared).is_empty(), "a course with nothing played was kept");
}

/// A course started while another was left standing takes the player's own settings as the ones to
/// put back, not the ones the course left behind narrowed them to, and keeps what was played of the
/// course left behind as a FAILED course, as a course left between two stages is.
#[test]
fn a_course_started_over_one_left_standing_puts_back_the_players_own_settings() {
    let mut shared = signed_out("course-over-course");
    let (own, first) = on_the_second_stage_of_a_course(&mut shared);
    let model = recorded_model();
    let entry = crate::course_ui::CourseEntry { course: two_stage_course(model.md5.clone()).course, missing: Vec::new() };
    crate::start_course(&mut shared, &entry);
    assert_eq!(shared.config.play, own, "the settings the first course narrowed became the player's own");
    let courses = kept_courses(&shared);
    let [course] = courses.as_slice() else {
        panic!("one course was left part-way and {} were kept", courses.len());
    };
    assert_eq!(course.clear, rbms_ir::ClearLamp::Failed, "a course left before its last stage is a FAILED course");
    assert_eq!(course.ex_score, first.ex_score, "the stage played is the course's");
}

/// The settings file written while a course narrows the play settings holds the player's own: the
/// course's constraints are the course's, and a course cut short by a crash or a quit leaves the
/// next start on what the player set.
#[test]
fn the_settings_saved_during_a_course_are_the_players_own() {
    let mut shared = signed_out("course-settings-file");
    shared.config.play.hispeed = NARROWED_HISPEED * 2.0;
    let (own, _) = on_the_second_stage_of_a_course(&mut shared);
    shared.save_settings();
    shared.finish_saves();
    let on_disk = rbms_config::load(&shared.settings_path).expect("the settings file reads").config;
    assert_eq!(on_disk.play, own, "the settings file holds the course's narrowed settings");
    assert!((shared.config.play.hispeed - NARROWED_HISPEED).abs() < f64::EPSILON, "the run still plays on the course's settings");
}

/// An app on the play screen of `session`, signed in to `server`, which takes courses and hands
/// each on to the returned receiver.
fn playing(tag: &str, session: PlaySession) -> (crate::App, Arc<AcceptingServer>, Receiver<CourseSubmission>) {
    let server = Arc::new(AcceptingServer::default());
    let (courses, sent) = channel();
    *server.courses.lock().expect("the course log") = Some(courses);
    let mut app = crate::App::new(String::new(), Config::default(), LaunchOptions::default(), settings_in_own_folder(&format!("{tag}-unused")));
    app.shared = signed_in(tag, server.clone());
    app.shared.mode = session.model().mode;
    app.shared.chart_path = RECORDED_CHART_NAME.to_owned();
    app.stage = Stage::Play(Box::new(PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned())));
    (app, server, sent)
}

/// A run the player was playing by hand when the app was told to quit is kept as Escape keeps it,
/// as a FAILED run whose notes not reached were let pass, and nothing is sent as the process ends:
/// the next start sends it.
#[test]
fn quitting_in_the_middle_of_a_run_keeps_it_as_a_failed_run_and_sends_nothing() {
    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    let left = session.summary();
    assert!(left.total_judged < left.total_notes, "the fixture run is cut short: {left:?}");
    let (mut app, server, _) = playing("quit-mid-run", session);

    app.keep_what_quitting_cuts_short();
    app.shared.finish_saves();

    let (logs, _) = stored(&app.shared);
    let [log] = logs.as_slice() else {
        panic!("one run was cut short and {} rows were stored", logs.len());
    };
    assert_eq!(log.clear, clear_type_id(ClearType::Failed));
    assert_eq!(log.minbp, left.min_bp_with_unreached(), "every note it never reached is charged");
    assert!(app.shared.replay_dir.join(log.replay_file.as_deref().expect("its replay is kept")).exists());
    assert!(app.shared.record_sync.is_none(), "a sync was started as the process ends");
    assert!(server.sent_for.lock().expect("the submission log").is_empty(), "the run was sent as the process ends");
    let db = app.shared.scoredb.as_ref().expect("the score database");
    let walk = SyncWalk { server: SIGNED_IN_SERVER, login_id: ACCOUNT_ID, ..SyncWalk::default() };
    assert_eq!(db.sync_candidates(&walk).expect("the plays to send read").len(), 1, "the next start's sync does not send the run");
}

/// The server the signed-in fixture sends to, as the records sync keys it.
const SIGNED_IN_SERVER: &str = "http://score-server.invalid";

/// Quitting keeps nothing of a run the engine plays, as Escape keeps nothing of one.
#[test]
fn quitting_in_the_middle_of_an_autoplay_keeps_nothing() {
    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    let (mut app, _, _) = playing("quit-autoplay", session);
    app.shared.config.play.autoplay = true;
    app.keep_what_quitting_cuts_short();
    assert_eq!(stored(&app.shared), (Vec::new(), None), "a run the engine played was kept");
}

/// Quitting in the middle of a course stage keeps the stage as a FAILED run and the course as a
/// FAILED course with the stage before it, sends neither, and puts the player's settings back.
#[test]
fn quitting_in_the_middle_of_a_course_keeps_a_failed_course_and_sends_nothing() {
    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    let left = session.summary();
    let (mut app, _, sent) = playing("quit-mid-course", session);
    let (own, first) = on_the_second_stage_of_a_course(&mut app.shared);

    app.keep_what_quitting_cuts_short();

    assert!(app.shared.course_run.is_none());
    assert_eq!(app.shared.config.play, own);
    let courses = kept_courses(&app.shared);
    let [course] = courses.as_slice() else {
        panic!("one course was cut short and {} were kept", courses.len());
    };
    assert_eq!(course.clear, rbms_ir::ClearLamp::Failed);
    assert_eq!(course.judge.pgreat, first.counts[0] + left.counts[0], "both stages are folded into the course");
    assert!(sent.recv_timeout(NOTHING_SENT_WAIT).is_err(), "the course was sent as the process ends");
}

/// Quitting on the LOADING screen between two stages of a course keeps the course as played so far.
#[test]
fn quitting_between_two_course_stages_keeps_the_course() {
    let model = recorded_model();
    let (mut app, _, sent) = playing("quit-between-stages", played_by_hand(&HAND_OFFSETS_US, false));
    app.stage = Stage::Loading(LoadingState::song(0));
    let (own, first) = on_the_second_stage_of_a_course(&mut app.shared);
    assert_eq!(app.shared.course_run.as_ref().map(|run| run.course.charts[0].md5.clone()), Some(model.md5.clone()));

    app.keep_what_quitting_cuts_short();

    assert!(app.shared.course_run.is_none());
    assert_eq!(app.shared.config.play, own);
    let courses = kept_courses(&app.shared);
    let [course] = courses.as_slice() else {
        panic!("one course was cut short and {} were kept", courses.len());
    };
    assert_eq!(course.clear, rbms_ir::ClearLamp::Failed);
    assert_eq!(course.ex_score, first.ex_score);
    assert!(sent.recv_timeout(NOTHING_SENT_WAIT).is_err(), "the course was sent as the process ends");
}
