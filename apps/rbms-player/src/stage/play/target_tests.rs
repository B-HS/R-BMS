//! The target a run on the play screen is paced against, as the screen settles it: the figure the
//! score graph and the figure over the judgement are drawn from, and the name the skin writes
//! beside them.
//!
//! The target is settled once, from the TARGET and TARGET FALLBACK rows and the records that stand
//! when the run starts, and it is the same for every kind of run: one played by hand, one the game
//! plays, a replay being watched and a stage of a course.

use rbms_config::ScoreTarget;
use rbms_play::{NullSink, SessionClock, SessionOptions};

use super::*;
use crate::app_result::record_tests::{HAND_OFFSETS_US, end_run, played_by_hand, recorded_model};
use crate::skin_host::score::RunScore;
use crate::skin_host::score::standing::PacedScores;
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
    let run = RunScore::of_session(&state.session, standing.best_score, standing.target_score, standing.paced_now);
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

/// Where the chart is stopped for the runs below to have judged some of its notes and not all.
const PART_WAY_US: i64 = 3_600_000;

/// The record runs of the recorded chart in `app`, judged again as the load judges them.
fn ghosts_of(app: &App) -> crate::ghost::RunGhosts {
    let model = recorded_model();
    let sources = crate::ghost::ghost_sources(&app.shared, &model.md5, SCORE_LN_MODE_FROM_CHART);
    let job = crate::ghost::GhostJob {
        md5: model.md5.clone(),
        mode: model.mode,
        model,
        bmson: None,
        replay_dir: app.shared.replay_dir.clone(),
        sources,
        settled: app.shared.saves.settled(),
    };
    job.spawn().rx.recv().expect("the thread answers")
}

/// The recorded chart's run as the play screen takes it, with the record runs judged again handed to
/// it as a load hands them over, and its target settled.
fn run_paced_by_its_record_runs(app: &App) -> (PlayState, crate::target::ResolvedTarget) {
    let mut state = run_of_the_recorded_chart(false);
    state.ghosts = Some(ghosts_of(app));
    let paced = paced_by(&mut state, app);
    (state, paced)
}

/// Let the chart run up to [`PART_WAY_US`] with nobody playing, and answer how many notes it judged.
fn let_part_of_it_go_by(state: &mut PlayState) -> u32 {
    state.session.tick(SessionClock::at(PART_WAY_US), &mut NullSink);
    let judge = state.session.judge();
    assert!(judge.total_judged() > 0 && judge.total_judged() < judge.total_notes(), "the run must be part-way: {}", judge.total_judged());
    judge.total_judged()
}

/// A run paced against its LOCAL BEST holds the best, in the HUD's score difference and in every
/// figure the skin reads, at what the best run had scored after as many notes as this run has
/// judged, rather than at its final score earned evenly; and a fixed target is still paced evenly
/// beside a best that is not.
#[test]
fn a_run_paced_against_its_local_best_reads_the_best_runs_score_note_by_note() {
    let mut app = app("ghost-best");
    let best_run = played_by_hand(&HAND_OFFSETS_US, false);
    let ghost = crate::ghost::GhostPace::of_judges(best_run.judge().vanished(), best_run.judge().total_notes());
    end_run(&mut app.shared, best_run);

    let (mut state, paced) = run_paced_by_its_record_runs(&app);
    assert_eq!((paced.used, paced.ex), (ScoreTarget::LocalBest, ghost.final_ex()));
    let judged = let_part_of_it_go_by(&mut state);
    let total = state.session.judge().total_notes();
    let best_by_now = ghost.ex_after(judged);
    assert_ne!(i64::from(best_by_now), pace_ex_at(paced.ex, judged, total), "the fixture must tell the two paces apart");
    assert_eq!(state.target_ex_now(&paced), i64::from(best_by_now), "the HUD's score difference");
    let standing = state.standing(&app.shared);
    assert_eq!(standing.paced_now, PacedScores { best: Some(best_by_now), rival: Some(best_by_now) });
    let run = RunScore::of_session(&state.session, standing.best_score, standing.target_score, standing.paced_now);
    assert_eq!((run.standing.now_best_score, run.standing.now_rival_score), (best_by_now as i32, best_by_now as i32));

    app.shared.config.judge.target = ScoreTarget::RateAaa;
    let (mut fixed, paced) = run_paced_by_its_record_runs(&app);
    assert_eq!(paced.used, ScoreTarget::RateAaa);
    let judged = let_part_of_it_go_by(&mut fixed);
    assert_eq!(fixed.target_ex_now(&paced), pace_ex_at(paced.ex, judged, total), "a fixed target is earned evenly");
    assert_eq!(fixed.standing(&app.shared).paced_now, PacedScores { best: Some(ghost.ex_after(judged)), rival: None });
}

/// A best whose run was not of this run's note count -- played with SCRATCH AUTO, which leaves the
/// turntable notes out -- is paced evenly, as the reference uses a ghost only of the chart's own
/// note count (`ScoreDataProperty.java:171-172`).
#[test]
fn a_scratch_auto_best_is_no_ghost_for_a_run_played_whole() {
    let mut app = app("ghost-scratch-auto");
    app.shared.config.play.scratch_auto = true;
    end_run(&mut app.shared, played_by_hand(&HAND_OFFSETS_US, true));
    app.shared.config.play.scratch_auto = false;
    let ghosts = ghosts_of(&app);
    let best = ghosts.best.clone().expect("the best run is judged again on its own notes");
    let mut state = run_of_the_recorded_chart(false);
    assert_ne!(best.total_notes(), state.session.judge().total_notes());
    state.ghosts = Some(ghosts);
    assert_eq!(paced_by(&mut state, &app).used, ScoreTarget::LocalBest);
    assert!(state.best_ghost.is_none() && state.target_ghost.is_none());
    assert_eq!(state.standing(&app.shared).paced_now, PacedScores::EVEN);
}

/// A practice slice is paced evenly whatever runs stand, as the reference gives practice no ghost
/// (`BMSPlayer.java:441-442`).
#[test]
fn practice_is_never_paced_against_a_ghost() {
    let mut app = app("ghost-practice");
    end_run(&mut app.shared, played_by_hand(&HAND_OFFSETS_US, false));
    let ghosts = ghosts_of(&app);
    assert!(ghosts.best.is_some());
    let mut state = run_of_the_recorded_chart(false);
    let mut panel = crate::practice::PracticePanel::new(recorded_model().md5, rbms_model::Mode::BEAT_7K, PRACTICE_CHART_MS, None, PRACTICE_TOTAL);
    state.set_practice(panel.start());
    state.ghosts = Some(ghosts);
    paced_by(&mut state, &app);
    assert!(state.best_ghost.is_none() && state.target_ghost.is_none());
}

/// How long the practice panel above is told the chart is, and its TOTAL.
const PRACTICE_CHART_MS: i32 = 8_000;
const PRACTICE_TOTAL: f64 = 300.0;
