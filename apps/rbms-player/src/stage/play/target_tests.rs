//! The target a run on the play screen is paced against, as the screen settles it: the figure the
//! score graph and the figure over the judgement are drawn from, and the name the skin writes
//! beside them.
//!
//! The target is settled once, from the TARGET and TARGET FALLBACK rows and the records that stand
//! when the run starts, and it is the same for every kind of run: one played by hand, one the game
//! plays, a replay being watched and a stage of a course.

use rbms_config::ScoreTarget;
use rbms_play::SessionOptions;

use super::*;
use crate::app_result::record_tests::{HAND_OFFSETS_US, end_run, played_by_hand, recorded_model};
use crate::skin_host::score::RunScore;
use crate::target::{band_target_ex, max_ex};
use crate::{App, Config, LaunchOptions};

/// The band of the score axis RATE AAA asks for.
const AAA_BAND: u32 = 24;

/// An app in a folder of this test's own, with nobody signed in and the TARGET row on LOCAL BEST.
fn app(tag: &str) -> App {
    let dir = std::env::temp_dir().join(format!("rbms-play-target-tests-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut config = Config::default();
    config.play.autoplay = false;
    config.judge.target = ScoreTarget::LocalBest;
    let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.replay = None;
    app
}

/// A run of the recorded chart as the play screen takes it, not yet played.
fn run_of_the_recorded_chart(autoplay: bool) -> PlayState {
    PlayState::new(
        PlaySession::new(recorded_model(), SessionOptions { autoplay, ..SessionOptions::default() }),
        std::collections::HashMap::new(),
        0,
        SCORE_LN_MODE_FROM_CHART.to_string(),
    )
}

/// What the run is paced against once the screen has settled it.
fn paced_by(state: &mut PlayState, app: &App) -> crate::target::ResolvedTarget {
    state.ensure_pace_target(&app.shared);
    state.pace_target.clone().expect("a run always has something to pace against")
}

/// A LOCAL BEST with no record to name is the fallback, on the figure the skin's timers are told,
/// on the figures its numbers and graph are drawn from, and in the name its text carries.
#[test]
fn a_run_with_no_local_best_is_paced_against_the_fallback_in_every_figure_the_skin_reads() {
    let mut app = app("figures");
    let mut state = run_of_the_recorded_chart(false);
    let total = state.session.judge().total_notes();
    let paced = paced_by(&mut state, &app);
    assert_eq!((paced.used, paced.name.as_str(), paced.ex), (ScoreTarget::RateAaa, "RATE AAA", band_target_ex(total, AAA_BAND)), "the shipped fallback");
    assert!(paced.stood_in_for());

    let standing = state.standing(&app.shared);
    assert_eq!((standing.target_score, standing.best_score), (paced.ex, 0), "the timers are told another target than the HUD");
    let run = RunScore::of_session(&state.session, standing.best_score, standing.target_score);
    assert_eq!(run.standing.rival_score, i32::try_from(paced.ex).expect("a small score"), "the figure over the judgement is of another target");
    assert!((run.standing.rival_rate - paced.ex as f32 / max_ex(total) as f32).abs() < 1e-6, "the score graph's bar is of another target");

    for fallback in [ScoreTarget::Max, ScoreTarget::RateA, ScoreTarget::RankNext] {
        app.shared.config.judge.target_fallback = fallback;
        let mut fresh = run_of_the_recorded_chart(false);
        let paced = paced_by(&mut fresh, &app);
        assert_eq!((paced.used, paced.name.as_str()), (fallback, fallback.label()), "{fallback:?}");
        assert_eq!(fresh.standing(&app.shared).target_score, paced.ex, "{fallback:?}");
    }
}

/// The target a run is paced against does not move with the rows once the run has begun: it is the
/// one the result screen scores the run against.
#[test]
fn the_fallback_a_run_began_with_is_the_one_it_ends_with() {
    let mut app = app("fixed");
    app.shared.config.judge.target_fallback = ScoreTarget::RateA;
    let mut state = run_of_the_recorded_chart(false);
    let began = paced_by(&mut state, &app);
    app.shared.config.judge.target_fallback = ScoreTarget::Max;
    assert_eq!(paced_by(&mut state, &app), began, "the fallback moved the target of a run in progress");
}

/// A record of the player's own is the target when there is one, and the fallback is not looked at.
#[test]
fn a_run_with_a_local_best_is_paced_against_it() {
    let mut app = app("best");
    app.shared.config.judge.target_fallback = ScoreTarget::Max;
    let (_, played) = end_run(&mut app.shared, played_by_hand(&HAND_OFFSETS_US, false));
    let mut state = run_of_the_recorded_chart(false);
    let paced = paced_by(&mut state, &app);
    assert_eq!((paced.used, paced.name.as_str(), paced.ex), (ScoreTarget::LocalBest, "LOCAL BEST", played.ex_score));
    assert_eq!(state.best_score, played.ex_score, "the graph's BEST bar is of another score");
    assert_eq!(state.standing(&app.shared).target_score, played.ex_score);
}

/// A run the game plays, a replay being watched and a stage of a course are paced as a run by hand
/// is: the target belongs to the chart and the rows, not to how the chart is played.
#[test]
fn autoplay_a_replay_and_a_course_stage_settle_the_target_as_a_run_by_hand_does() {
    let hand = {
        let app = app("by-hand");
        paced_by(&mut run_of_the_recorded_chart(false), &app)
    };
    assert_eq!(hand.used, ScoreTarget::RateAaa);

    let mut autoplayed = app("autoplay");
    autoplayed.shared.config.play.autoplay = true;
    assert_eq!(paced_by(&mut run_of_the_recorded_chart(true), &autoplayed), hand, "autoplay");

    let mut watched = app("replay");
    watched.shared.replay = Some(rbms_store::Replay {
        chart_path: String::new(),
        md5: recorded_model().md5,
        mode: recorded_model().mode.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: rbms_store::ReplayJudge::default(),
        events: Vec::new(),
    });
    assert_eq!(paced_by(&mut run_of_the_recorded_chart(false), &watched), hand, "replay");

    let mut stage = app("course");
    let chart = rbms_course::CourseChart { md5: recorded_model().md5, ..Default::default() };
    stage.shared.course_run = Some(rbms_course::CourseRun::new(rbms_course::Course { charts: vec![chart.clone(), chart], ..Default::default() }, 0.0));
    assert_eq!(paced_by(&mut run_of_the_recorded_chart(false), &stage), hand, "course");
}
