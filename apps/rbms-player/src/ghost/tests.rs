//! The stored run a run is held against, judged again from its replay: what it adds up to note by
//! note, and every way a replay can fail to be the run its record is of.

use rbms_config::ScoreTarget;
use rbms_judge::Judge;
use rbms_store::SCORE_LN_MODE_FROM_CHART;

use super::*;
use crate::app_result::record_tests::{HAND_OFFSETS_US, end_run, played_by_hand, recorded_model};
use crate::{App, Config, LaunchOptions};

/// The hash of a chart other than the recorded one.
const OTHER_CHART_MD5: &str = "00000000000000000000000000000000";

/// An app in a folder of this test's own, with nobody signed in and the TARGET row on `target`.
fn app(tag: &str, target: ScoreTarget) -> App {
    let dir = std::env::temp_dir().join(format!("rbms-ghost-tests-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut config = Config::default();
    config.play.autoplay = false;
    config.judge.target = target;
    let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.replay = None;
    app
}

/// The job that judges again the record runs of the recorded chart in `app`.
fn job_of(app: &App) -> GhostJob {
    let model = recorded_model();
    GhostJob {
        md5: model.md5.clone(),
        mode: model.mode,
        model,
        bmson: None,
        replay_dir: app.shared.replay_dir.clone(),
        sources: ghost_sources(&app.shared, &recorded_model().md5, SCORE_LN_MODE_FROM_CHART),
        settled: app.shared.saves.settled(),
    }
}

/// The best run of `job`, which the test has just stored.
fn best_of(job: &GhostJob) -> GhostSource {
    job.sources.best.clone().expect("the run just stored is the best, and it kept its replay")
}

#[test]
fn a_ghost_adds_two_for_a_pgreat_and_one_for_a_great() {
    let pace = GhostPace::of_judges(&[Judge::PerfectGreat, Judge::Great, Judge::Good, Judge::Bad, Judge::Poor, Judge::PerfectGreat], 6);
    let every: Vec<u32> = (0..=6).map(|judged| pace.ex_after(judged)).collect();
    assert_eq!(every, [0, 2, 3, 3, 3, 3, 5]);
    assert_eq!((pace.total_notes(), pace.final_ex()), (6, 5));
    assert_eq!(pace.ex_after(7), 5, "past the last note the run stands at its final score");
}

#[test]
fn a_run_that_ended_early_is_padded_with_poors() {
    let pace = GhostPace::of_judges(&[Judge::PerfectGreat, Judge::Great], 5);
    let every: Vec<u32> = (0..=5).map(|judged| pace.ex_after(judged)).collect();
    assert_eq!(every, [0, 2, 3, 3, 3, 3], "the notes the run never reached are POORs");
    assert_eq!(pace.total_notes(), 5);
}

/// A run stored with its replay, judged again, adds up note by note to what the run itself
/// consumed, and so does the thread that judges it: the replay is the run.
#[test]
fn a_ghost_is_the_saved_run_judged_again() {
    let mut app = app("saved-run", ScoreTarget::LocalBest);
    let session = played_by_hand(&HAND_OFFSETS_US, false);
    let played = GhostPace::of_judges(session.judge().vanished(), session.judge().total_notes());
    assert!(played.final_ex() > 0 && played.final_ex() < 2 * played.total_notes(), "the run must be neither nothing nor everything");
    end_run(&mut app.shared, session);

    let job = job_of(&app);
    assert_eq!(rejudge(&job, &best_of(&job)), Ok(played.clone()));
    assert!(job.sources.account.is_none(), "the TARGET row does not ask for the account's best");
    let ghosts = job.spawn().rx.recv().expect("the thread answers");
    assert_eq!(ghosts.best.as_deref(), Some(&played));
    assert!(ghosts.account.is_none());
}

/// A replay that is not the run its record is of is no ghost, whichever way it is not: another
/// chart, another note count, another PGREAT or GREAT tally, the frame-by-frame clock of an old
/// replay, no file, or a file that does not read.
#[test]
fn a_replay_of_another_chart_or_count_is_not_a_ghost() {
    let mut app = app("not-a-ghost", ScoreTarget::LocalBest);
    end_run(&mut app.shared, played_by_hand(&HAND_OFFSETS_US, false));
    let job = job_of(&app);
    let best = best_of(&job);
    assert!(rejudge(&job, &best).is_ok(), "the run as it was stored is a ghost");

    let other_chart = GhostJob { md5: OTHER_CHART_MD5.to_string(), ..job_of(&app) };
    assert!(rejudge(&other_chart, &best).is_err(), "a replay of another chart");
    assert!(rejudge(&job, &GhostSource { total_notes: best.total_notes + 1, ..best.clone() }).is_err(), "another note count");
    assert!(rejudge(&job, &GhostSource { pgreat: best.pgreat + 1, ..best.clone() }).is_err(), "another PGREAT tally");
    assert!(rejudge(&job, &GhostSource { great: best.great + 1, ..best.clone() }).is_err(), "another GREAT tally");
    assert!(rejudge(&job, &GhostSource { file: "missing.ron".to_string(), ..best.clone() }).is_err(), "no replay");
    let mut framed = Replay::load(&app.shared.replay_dir.join(&best.file)).expect("the stored replay reads");
    framed.judge.clock = rbms_store::REPLAY_LEGACY_CLOCK.to_string();
    let framed_file = "framed.ron";
    std::fs::write(app.shared.replay_dir.join(framed_file), framed.to_ron().expect("the replay writes")).expect("the replay folder is writable");
    assert!(rejudge(&job, &GhostSource { file: framed_file.to_string(), ..best.clone() }).is_err(), "a replay judged frame by frame");
    let broken = "broken.ron";
    std::fs::write(app.shared.replay_dir.join(broken), "(not a replay").expect("the replay folder is writable");
    assert!(rejudge(&job, &GhostSource { file: broken.to_string(), ..best }).is_err(), "a replay that does not read");
}

/// The account's best on the score server is judged again from this machine's run of the same
/// second and the same EX, and from nothing when no run here is that one.
#[test]
fn the_account_best_is_the_local_run_played_at_the_same_second() {
    use crate::ir_ranking::{RankingBoard, RankingRow};
    let mut app = app("account-best", ScoreTarget::IrBest);
    end_run(&mut app.shared, played_by_hand(&HAND_OFFSETS_US, false));
    let md5 = recorded_model().md5;
    let stored = app.shared.scores.for_md5(&md5)[0].clone();
    let row = |played_at: i64, ex_score: u32| RankingRow {
        rank: None,
        player: "tester".to_string(),
        ex_score,
        lamp: "CLEAR",
        lamp_color: rbms_render::Color::GREEN,
        fast_slow: None,
        replay_id: None,
        played_at,
    };
    let account_with = |app: &mut App, you: RankingRow| {
        app.shared.ranking_cache.insert(md5.clone(), RankingState::Ready(Box::new(RankingBoard { rows: Vec::new(), you: Some(you), rivals: Vec::new() })));
        ghost_sources(&app.shared, &md5, SCORE_LN_MODE_FROM_CHART).account
    };
    let same_second = stored.played_at + MILLIS_PER_SECOND - 1;
    assert_eq!(account_with(&mut app, row(same_second, stored.ex_score)), GhostSource::of(&stored), "the same run, sent with its milliseconds");
    assert_eq!(account_with(&mut app, row(stored.played_at + MILLIS_PER_SECOND, stored.ex_score)), None, "a run of another second");
    assert_eq!(account_with(&mut app, row(stored.played_at, stored.ex_score + 1)), None, "a run of another score");
}
