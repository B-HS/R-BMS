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

/// The app's library as the one recorded chart, under the name the runs above were played from.
fn library_of_the_recorded_chart(app: &mut App) {
    let model = recorded_model();
    let entry = rbms_library::SongEntry {
        path: std::path::PathBuf::from(&app.shared.chart_path),
        title: model.meta.title.clone(),
        subtitle: String::new(),
        artist: String::new(),
        genre: String::new(),
        maker: String::new(),
        level: String::new(),
        difficulty: 0,
        init_bpm: model.init_bpm,
        rank: 0,
        total: model.meta.total,
        mode: model.mode,
        md5: model.md5,
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    };
    app.shared.library = crate::Library::from_songs(vec![entry]);
}

/// A replay of the recorded chart being watched under analysis, as the play screen holds it.
fn watching(app: &mut App, replay: rbms_store::Replay) -> PlayState {
    app.shared.replay = Some(replay.clone());
    PlayState::new(
        PlaySession::new(recorded_model(), SessionOptions { analysis: true, replay: Some(replay), ..SessionOptions::default() }),
        std::collections::HashMap::new(),
        0,
        SCORE_LN_MODE_FROM_CHART.to_string(),
    )
}

/// Press PLAY THIS LAYOUT on the replay being watched.
fn press_play_this_layout(state: &mut PlayState, app: &mut App) -> Transition {
    let code = app.shared.keyconfig.shortcut_key(Shortcut::ReplayPlayLayout).expect("the key ships bound");
    let key = KeyInput { code, pressed: true, released: false, text: None };
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, key)
}

/// The seed a replay from elsewhere was laid out from.
const ELSEWHERE_SEED: u64 = 0x5EED;

/// A replay of the recorded chart that no run of this machine kept, laid out on `random`.
fn replay_from_elsewhere(random: rbms_chart::shuffle::NoteOption) -> rbms_store::Replay {
    rbms_store::Replay {
        chart_path: String::new(),
        md5: recorded_model().md5,
        mode: recorded_model().mode.name.to_string(),
        random: random.label().to_string(),
        seed: ELSEWHERE_SEED,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: rbms_store::ReplayJudge::default(),
        events: Vec::new(),
    }
}

/// A run of the recorded chart played by hand and kept, on the TARGET row RATE AAA, with the chart
/// in the library: its record, the replay it kept as the record modal reads it, and its pace.
fn a_kept_run_in_the_library(app: &mut App) -> (rbms_store::ScoreRecord, rbms_store::Replay, crate::ghost::GhostPace) {
    app.shared.config.judge.target = ScoreTarget::RateAaa;
    let kept_run = played_by_hand(&HAND_OFFSETS_US, false);
    let ghost = crate::ghost::GhostPace::of_judges(kept_run.judge().vanished(), kept_run.judge().total_notes());
    end_run(&mut app.shared, kept_run);
    if let Some(saved) = app.shared.saves.settled() {
        saved.recv().expect("the save thread answers");
    }
    library_of_the_recorded_chart(app);
    let record = app.shared.scores.for_md5(&recorded_model().md5).first().map(|record| (*record).clone()).expect("the run is kept");
    let file = record.replay_file.clone().expect("the run kept its replay");
    let replay = rbms_store::Replay::load(&app.shared.replay_dir.join(file)).expect("the kept replay reads");
    (record, replay, ghost)
}

/// A kept run whose replay cannot be judged again note for note -- one judged on the frame-by-frame
/// clock of an old replay -- still lends its layout and its score, which the run on it is then held
/// against earned evenly.
#[test]
fn a_kept_run_that_cannot_be_judged_again_is_paced_evenly() {
    let mut app = app("borrowed-framed");
    let (record, mut replay, _) = a_kept_run_in_the_library(&mut app);
    replay.judge.clock = rbms_store::REPLAY_LEGACY_CLOCK.to_string();
    let file = app.shared.replay_dir.join(record.replay_file.as_deref().expect("the run kept its replay"));
    std::fs::write(file, replay.to_ron().expect("the replay writes")).expect("the replay folder is writable");
    let md5 = recorded_model().md5;
    let mut watched = watching(&mut app, replay);
    assert!(matches!(press_play_this_layout(&mut watched, &mut app), Transition::To(Stage::Loading(_))));
    app.shared.layout_to_load(&md5, None);

    let (mut state, paced) = run_paced_by_its_record_runs(&app);
    assert_eq!((paced.ex, paced.of_borrowed_replay), (record.ex_score, true));
    assert!(state.target_ghost.is_none(), "a replay on the frame clock was taken for a ghost");
    let judged = let_part_of_it_go_by(&mut state);
    assert_eq!(state.target_ex_now(&paced), pace_ex_at(paced.ex, judged, state.session.judge().total_notes()));
}

/// PLAY THIS LAYOUT over a replay a run of this machine kept starts a run of the chart in the
/// replay's place, and that run, and the result screen after it, are held against the kept run under
/// its name, by its replay note by note, whatever the TARGET row asks for, which stays as it is.
#[test]
fn a_run_on_the_layout_of_a_kept_run_is_held_against_that_run_note_by_note() {
    let mut app = app("borrowed-kept");
    let (record, replay, ghost) = a_kept_run_in_the_library(&mut app);
    let md5 = recorded_model().md5;
    let mut watched = watching(&mut app, replay);
    assert!(matches!(press_play_this_layout(&mut watched, &mut app), Transition::To(Stage::Loading(_))), "the run does not take the replay's place");
    assert!(app.shared.replay.is_none(), "the replay is still being watched");
    app.shared.layout_to_load(&md5, None);

    let (mut state, paced) = run_paced_by_its_record_runs(&app);
    let name = format!("REPLAY {}", crate::format::fmt_datetime(record.played_at));
    assert_eq!((paced.name.as_str(), paced.ex, paced.of_borrowed_replay), (name.as_str(), record.ex_score, true));
    assert_eq!(paced.ex, ghost.final_ex());
    assert!(!paced.stood_in_for(), "the kept run was read as standing in for the TARGET row");
    assert_eq!(app.shared.config.judge.target, ScoreTarget::RateAaa, "the TARGET row moved");
    let judged = let_part_of_it_go_by(&mut state);
    let total = state.session.judge().total_notes();
    assert_ne!(i64::from(ghost.ex_after(judged)), pace_ex_at(paced.ex, judged, total), "the fixture must tell the two paces apart");
    assert_eq!(state.target_ex_now(&paced), i64::from(ghost.ex_after(judged)), "the HUD's score difference");
    assert_eq!(state.standing(&app.shared).paced_now.rival, Some(ghost.ex_after(judged)));
    assert_eq!(run_target(&app.shared, &md5, total, Some(record.ex_score)), paced, "the result screen holds the run against another target");
}

/// A replay no run of this machine kept may have come from the score server: its layout is borrowed
/// as it shows it, but the run on it is held against the TARGET row, and one that shows OFF, which
/// the score server's CONVERGE is shown as too, is refused and the replay plays on.
#[test]
fn a_replay_from_elsewhere_lends_its_layout_but_no_run_and_never_an_off_it_cannot_vouch_for() {
    let mut app = app("borrowed-elsewhere");
    app.shared.config.judge.target = ScoreTarget::RateAaa;
    library_of_the_recorded_chart(&mut app);
    let md5 = recorded_model().md5;

    let mut watched = watching(&mut app, replay_from_elsewhere(rbms_chart::shuffle::NoteOption::Off));
    assert!(matches!(press_play_this_layout(&mut watched, &mut app), Transition::Stay));
    assert!(app.shared.replay.is_some() && app.shared.borrowed_layout().is_none(), "an OFF from elsewhere was borrowed");

    let mut watched = watching(&mut app, replay_from_elsewhere(rbms_chart::shuffle::NoteOption::Random));
    assert!(matches!(press_play_this_layout(&mut watched, &mut app), Transition::To(Stage::Loading(_))));
    assert_eq!(app.shared.layout_to_load(&md5, None), (rbms_chart::shuffle::NoteOption::Random, ELSEWHERE_SEED));
    assert!(app.shared.borrowed_kept_run(&md5).is_none(), "a replay from elsewhere was taken for a run of this machine");
    let (_, paced) = run_paced_by_its_record_runs(&app);
    assert_eq!((paced.used, paced.of_borrowed_replay), (ScoreTarget::RateAaa, false));
}
