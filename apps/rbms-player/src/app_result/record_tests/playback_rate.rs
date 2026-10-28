//! What is kept of a run played at another PLAYBACK RATE than the chart's own: nothing (the
//! player's decision of 2026-10-29). Not a row, a replay, a trace or a submission, whichever of the
//! three ways in the run reaches: its result screen, the app quit during it, or a failed run started
//! again at once. Its result screen is still shown, and says why nothing was kept. A course and a
//! replay being watched play at the chart's own speed whatever the setting says.

use super::*;

/// A PLAYBACK RATE other than the chart's own, as a player sets it.
const FASTER_PERCENT: i32 = 115;

/// The run of `session` on the play screen, played at [`FASTER_PERCENT`].
fn played_faster(shared: &mut AppShared, session: PlaySession) -> PlayState {
    shared.mode = session.model().mode;
    shared.chart_path = RECORDED_CHART_NAME.to_owned();
    shared.replay = None;
    let mut played = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    played.set_playback_rate(FASTER_PERCENT);
    played
}

/// Nothing of the recorded chart's runs is anywhere a run is kept: no row, no replay file, no run
/// kept on the side, and no submission made.
fn assert_nothing_kept(shared: &mut AppShared, server: &AcceptingServer) {
    shared.finish_saves();
    assert_eq!(stored(shared), (Vec::new(), None), "a run at another rate was recorded");
    assert!(every_play(shared).is_empty(), "a run at another rate was stored");
    assert_eq!(std::fs::read_dir(&shared.replay_dir).map_or(0, Iterator::count), 0, "a replay of a run at another rate was written");
    assert!(shared.scores.for_md5(&recorded_model().md5).is_empty(), "the book the screens read gained the run");
    assert!(server.sent_for.lock().expect("the submission log").is_empty(), "a run at another rate was sent");
    assert!(shared.record_sync.is_none(), "a records sync was started for a run at another rate");
}

/// A run played to its end at another rate is shown its result screen, which says it was neither
/// kept nor sent and why, sets no best, and keeps nothing of it, signed in or not.
#[test]
fn a_run_at_another_playback_rate_shows_its_result_and_keeps_and_sends_nothing() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("rate-result", server.clone());
    let mut played = played_faster(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    assert_eq!(played.changed_playback_percent(), Some(FASTER_PERCENT));
    assert!(played.keeps_no_record());

    let entered = enter_result(&mut played, &mut shared);
    assert!(matches!(entered, Transition::To(Stage::Result(_))), "a run at another rate does not reach its result screen");
    assert_eq!(shared.ir_status, IrStatus::Skipped(crate::practice::PLAYBACK_RATE_BLOCK_REASON));
    let notices = result_notices(&entered, &shared);
    assert!(notices.iter().any(|line| line == "PLAYBACK RATE 115%: NOT SAVED, NOT SENT"), "the result screen does not say why: {notices:?}");
    assert!(notices.iter().any(|line| line.contains("SKIPPED") && line.contains("playback rate")), "the submission line does not say why: {notices:?}");
    let Transition::To(Stage::Result(result)) = &entered else {
        unreachable!("the run reached its result screen above");
    };
    assert!(!result.updates_score(), "a run at another rate says it set a best");
    assert_eq!(result.reported_playback_percent(), Some(FASTER_PERCENT), "a skin is not told the rate the run was played at");
    assert_nothing_kept(&mut shared, &server);
}

/// The app quit during a run at another rate keeps nothing of it, where a run at the chart's own
/// speed is kept as a FAILED run.
#[test]
fn quitting_in_the_middle_of_a_run_at_another_playback_rate_keeps_nothing() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("rate-quit", server.clone());
    let model = recorded_model();
    let session = run_until(model.clone(), SessionOptions::default(), &inputs_by_hand(&model, &HAND_OFFSETS_US, false), LEFT_AT_US);
    assert!(session.summary().total_judged > 0, "the fixture run has judged notes, which a run at the chart's own speed keeps");
    let played = played_faster(&mut shared, session);
    keep_run_cut_short_by_quitting(&played, false, &mut shared);
    assert_nothing_kept(&mut shared, &server);
}

/// A failed run at another rate started again at once with START or SELECT keeps nothing either,
/// where a run at the chart's own speed is kept and sent on the way out.
#[test]
fn a_failed_run_at_another_playback_rate_started_again_keeps_nothing() {
    let server = Arc::new(AcceptingServer::default());
    let mut shared = signed_in("rate-retried", server.clone());
    shared.config.play.gauge = rbms_judge::GaugeKind::ExHard;
    let played = played_faster(&mut shared, played_on(rbms_judge::GaugeKind::ExHard, &FAILING_OFFSETS_US, false));
    record_retried_run(&played, &mut shared);
    assert_nothing_kept(&mut shared, &server);
}

/// The rate a run about to be loaded is played at: the setting's, except for a course and a replay
/// being watched, which play at the chart's own speed.
#[test]
fn a_course_and_a_watched_replay_play_at_the_charts_own_speed() {
    let mut shared = signed_out("rate-forced");
    assert_eq!(shared.run_playback_percent(), rbms_config::PLAYBACK_RATE_UNCHANGED_PERCENT, "a fresh install");
    shared.config.play.playback_percent = FASTER_PERCENT;
    assert_eq!(shared.run_playback_percent(), FASTER_PERCENT, "a run by hand or by the engine plays at the setting's rate");

    shared.course_run = Some(one_stage_course(recorded_model().md5));
    assert_eq!(shared.run_playback_percent(), rbms_config::PLAYBACK_RATE_UNCHANGED_PERCENT, "a course stage");
    shared.course_run = None;

    shared.replay = Some(Replay {
        chart_path: RECORDED_CHART_NAME.to_owned(),
        md5: recorded_model().md5,
        mode: Mode::BEAT_7K.name.to_owned(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    });
    assert_eq!(shared.run_playback_percent(), rbms_config::PLAYBACK_RATE_UNCHANGED_PERCENT, "a replay being watched");
}

/// A practice slice plays at its own FREQUENCY whatever the PLAYBACK RATE says, and keeps nothing as
/// a practice slice, not as a run at another rate.
#[test]
fn a_practice_slice_ignores_the_playback_rate() {
    let mut shared = signed_out("rate-practice");
    let mut played = played_faster(&mut shared, played_by_hand(&HAND_OFFSETS_US, false));
    let mut panel = crate::practice::PracticePanel::new(recorded_model().md5, Mode::BEAT_7K, PRACTISED_CHART_MS, None, PRACTISED_TOTAL);
    played.set_practice(panel.start());
    assert_eq!(played.changed_playback_percent(), None, "the slice is not a run at another rate");
    assert!(played.keeps_no_record(), "a practice slice keeps nothing");
}

/// The chart a practice slice is cut from, as its panel is told of it: how long it is in
/// milliseconds and its TOTAL.
const PRACTISED_CHART_MS: i32 = 60_000;
const PRACTISED_TOTAL: f64 = 300.0;
