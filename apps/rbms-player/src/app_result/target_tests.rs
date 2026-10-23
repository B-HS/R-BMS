//! Which score a run is paced against when the TARGET row names the local best, the account's best
//! or a rival's, on charts that have that score and charts that have none.
//!
//! The question these answer is what a player sees who set LOCAL BEST on a chart they have no best
//! on: one played only with the judge widened past the chart's own (such a run sets no best score),
//! or only with SCRATCH AUTO under the rule before it stopped counting the turntable. The target is
//! not there, and what they are paced against is the fallback they named -- the same number on the
//! play screen's score graph, on the figure over the judgement, and on the result screen.

use std::path::PathBuf;

use rbms_config::{Config, ScoreTarget};

use super::record_tests::{HAND_OFFSETS_US, end_run, played_by_hand, played_widened, recorded_model};
use super::*;
use crate::ir_ranking::{RankingBoard, RankingRow};
use crate::target::{band_target_ex, max_ex, target_ex};

/// The band of the score axis the shipped fallback (RATE AAA) asks for.
const AAA_BAND: u32 = 24;

/// An app with no server set up and nobody signed in, in a folder of this test's own.
fn app_in_own_folder(tag: &str) -> AppShared {
    let folder: PathBuf = std::env::temp_dir().join(format!("rbms-target-tests-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder is writable");
    let mut config = Config::default();
    config.play.autoplay = false;
    crate::App::new(String::new(), config, LaunchOptions::default(), folder.join("settings.ron")).shared
}

/// Play the recorded chart to its end in `shared`, by hand, judged wider than the chart asks when
/// `widened` says so, and answer what the engine said of the run and the screen it led to.
fn play(shared: &mut AppShared, widened: bool) -> (Transition, rbms_play::PlaySummary) {
    let session = if widened { played_widened(&HAND_OFFSETS_US) } else { played_by_hand(&HAND_OFFSETS_US, false) };
    end_run(shared, session)
}

/// What the result screen `entered` leads to paced the run against.
fn result_target(entered: &Transition) -> Option<(String, u32)> {
    match entered {
        Transition::To(Stage::Result(result)) => result.target().map(|target| (target.name.clone(), target.ex)),
        _ => None,
    }
}

/// The target the play screen would pace a run of the recorded chart against, from the records that
/// stand in `shared`.
fn pace_of_the_next_run(shared: &AppShared, total_notes: u32) -> crate::target::ResolvedTarget {
    let md5 = recorded_model().md5;
    let best = shared.scores.best_ex_for_md5_in_ln_mode(&md5, SCORE_LN_MODE_FROM_CHART);
    run_target(shared, &md5, total_notes, best)
}

fn ranking_row(player: &str, ex: u32) -> RankingRow {
    RankingRow {
        rank: None,
        player: player.to_string(),
        ex_score: ex,
        record_lamp: rbms_store::RecordLamp::default(),
        lamp: "CLEAR",
        lamp_color: Color::GREEN,
        fast_slow: None,
        replay_id: None,
        played_at: 0,
        ..Default::default()
    }
}

/// Put a ranking of the recorded chart into the cache, as the browser's panel leaves one.
fn rank_the_recorded_chart(shared: &mut AppShared, you: Option<u32>, rival: Option<u32>) {
    let board = RankingBoard {
        rows: Vec::new(),
        you: you.map(|ex| ranking_row("me", ex)),
        rivals: rival.map(|ex| vec![("friend".to_string(), Some(ranking_row("friend", ex)))]).unwrap_or_default(),
        ..Default::default()
    };
    shared.ranking_cache.insert(recorded_model().md5, RankingState::Ready(Box::new(board)));
}

/// A chart the player has a record of is paced against that record, named so, whatever the fallback
/// is: the fallback is for the chart that has none.
#[test]
fn a_chart_with_a_scoring_record_is_paced_against_that_record() {
    let mut shared = app_in_own_folder("record");
    shared.config.judge.target = ScoreTarget::LocalBest;
    shared.config.judge.target_fallback = ScoreTarget::Max;
    let (_, played) = play(&mut shared, false);
    let paced = pace_of_the_next_run(&shared, played.total_notes);
    assert_eq!((paced.used, paced.ex, paced.name.as_str()), (ScoreTarget::LocalBest, played.ex_score, "LOCAL BEST"));
    assert!(!paced.stood_in_for());
    assert!(played.ex_score < max_ex(played.total_notes), "the fixture's own score has to differ from the ceiling for this to tell the two apart");

    let (second, _) = play(&mut shared, false);
    assert_eq!(result_target(&second), Some(("LOCAL BEST".to_string(), played.ex_score)), "the result screen scores the second run against the first");
}

/// The state of a player who plays every run with the judge widened: no run is a local best, so
/// LOCAL BEST has no score to name and the run is paced against the fallback -- which is the player's
/// to choose, and which is the same on the play screen and the result screen.
#[test]
fn a_chart_with_no_scoring_record_is_paced_against_the_fallback_the_player_named() {
    let mut shared = app_in_own_folder("widened-only");
    shared.config.judge.target = ScoreTarget::LocalBest;
    let (_, played) = play(&mut shared, true);
    assert!(played.ex_score > 0, "the fixture run scores");
    let md5 = recorded_model().md5;
    assert_eq!(shared.scores.best_ex_for_md5_in_ln_mode(&md5, SCORE_LN_MODE_FROM_CHART), None, "a widened run is no local best");

    for fallback in ScoreTarget::FALLBACKS {
        shared.config.judge.target_fallback = fallback;
        let paced = pace_of_the_next_run(&shared, played.total_notes);
        let bare = crate::target::TargetContext {
            total_notes: played.total_notes,
            custom_rate_percent: shared.config.judge.target_rate_percent,
            ..Default::default()
        };
        assert_eq!((paced.asked, paced.used), (ScoreTarget::LocalBest, fallback), "{fallback:?}");
        assert_eq!(paced.name, crate::target::target_name(fallback, &bare), "{fallback:?}");
        assert_eq!(Some(paced.ex), target_ex(fallback, &bare), "{fallback:?}");
        assert!(paced.stood_in_for());
        let (entered, _) = play(&mut shared, true);
        assert_eq!(result_target(&entered), Some((paced.name.clone(), paced.ex)), "{fallback:?}: the result screen paced the run against another target");
    }
}

/// A fresh install that has never been told a fallback paces a LOCAL BEST with no record against
/// RATE AAA, the shipped target, and not against the whole chart as it did before the fallback was a
/// row: the ceiling is still there to choose, as MAX.
#[test]
fn the_shipped_fallback_is_rate_aaa_and_max_is_one_choice_of_it() {
    let mut shared = app_in_own_folder("shipped");
    shared.config.judge.target = ScoreTarget::LocalBest;
    let (_, played) = play(&mut shared, true);
    let shipped = pace_of_the_next_run(&shared, played.total_notes);
    assert_eq!((shipped.used, shipped.ex), (ScoreTarget::RateAaa, band_target_ex(played.total_notes, AAA_BAND)));
    assert_ne!(shipped.ex, max_ex(played.total_notes), "the fixture chart is too small to tell RATE AAA from the whole chart");
    shared.config.judge.target_fallback = ScoreTarget::Max;
    assert_eq!(pace_of_the_next_run(&shared, played.total_notes).ex, max_ex(played.total_notes));
}

/// The account's best that has not come from the server is replaced by the player's own record, and
/// only when there is none by the fallback; a rival's missing score goes to the fallback at once, as
/// the reference leaves a missing rival at nothing rather than at the player's own best.
#[test]
fn a_score_the_ranking_does_not_hold_is_replaced_in_the_order_the_target_row_documents() {
    let mut shared = app_in_own_folder("ranked");
    shared.config.judge.target_fallback = ScoreTarget::RateA;
    let (_, played) = play(&mut shared, false);
    let (own, fallback) = (played.ex_score, band_target_ex(played.total_notes, 18));

    shared.config.judge.target = ScoreTarget::IrBest;
    rank_the_recorded_chart(&mut shared, Some(own + 3), Some(own + 5));
    let paced = pace_of_the_next_run(&shared, played.total_notes);
    assert_eq!((paced.used, paced.ex), (ScoreTarget::IrBest, own + 3), "the account's score is there");

    rank_the_recorded_chart(&mut shared, None, None);
    let paced = pace_of_the_next_run(&shared, played.total_notes);
    assert_eq!((paced.used, paced.ex), (ScoreTarget::LocalBest, own), "no account score: the player's own record");

    shared.config.judge.target = ScoreTarget::Rival;
    let paced = pace_of_the_next_run(&shared, played.total_notes);
    assert_eq!((paced.used, paced.ex), (ScoreTarget::RateA, fallback), "no rival score: the fallback, not the player's own record");

    rank_the_recorded_chart(&mut shared, None, Some(own + 5));
    let paced = pace_of_the_next_run(&shared, played.total_notes);
    assert_eq!((paced.used, paced.ex, paced.name.as_str()), (ScoreTarget::Rival, own + 5, "RIVAL friend"));

    let mut unplayed = app_in_own_folder("ranked-unplayed");
    unplayed.config.judge.target = ScoreTarget::IrBest;
    unplayed.config.judge.target_fallback = ScoreTarget::RateA;
    let paced = pace_of_the_next_run(&unplayed, played.total_notes);
    assert_eq!((paced.used, paced.ex), (ScoreTarget::RateA, fallback), "no account score and no record of one's own: the fallback");
}

/// The share a player types is a target of its own, and a fallback they can choose: a chart with no
/// record is paced against that share of its ceiling, named for it, on the play screen and on the
/// result screen alike. Moving the share moves the target.
#[test]
fn the_custom_rate_is_a_fallback_the_player_sets_a_share_for() {
    let mut shared = app_in_own_folder("custom-rate");
    shared.config.judge.target = ScoreTarget::LocalBest;
    shared.config.judge.target_fallback = ScoreTarget::RateCustom;
    let (_, played) = play(&mut shared, true);
    for percent in [50, 85, 100] {
        shared.config.judge.target_rate_percent = percent;
        let paced = pace_of_the_next_run(&shared, played.total_notes);
        assert_eq!((paced.used, paced.name.as_str()), (ScoreTarget::RateCustom, format!("SCORE RATE {percent}%").as_str()));
        assert_eq!(paced.ex, crate::target::rate_target_ex(played.total_notes, f64::from(percent)), "{percent}%");
        let (entered, _) = play(&mut shared, true);
        assert_eq!(result_target(&entered), Some((paced.name.clone(), paced.ex)), "{percent}%: the result screen paced the run against another target");
    }
    assert_eq!(pace_of_the_next_run(&shared, played.total_notes).ex, max_ex(played.total_notes), "a hundred percent of the chart is the whole chart");
}

/// The browser counts the records a run of the chart would count before it has read the chart: a
/// chart that states its long notes has its records under the chart's own kind, whatever LN MODE
/// is set, and one that leaves them unstated has them under the LN MODE each run was played under,
/// so only the ones of the LN MODE now set count. The browser and the play screen then agree on
/// whether there is a local best to pace against.
#[test]
fn the_local_best_the_browser_counts_is_the_one_a_run_would_count() {
    let mut shared = app_in_own_folder("ln-kinds");
    let md5 = recorded_model().md5;
    let (_, summary) = play(&mut shared, false);
    for ln_mode in rbms_judge::ln::LnMode::ALL {
        shared.config.judge.ln_mode = ln_mode;
        assert_eq!(local_best_before_reading(&shared, &md5), Some(summary.ex_score), "a chart that states its long notes, under {ln_mode:?}");
    }

    let made_as_charge_notes = ScoreRecord { ln_mode: ln_mode_token(rbms_judge::ln::LnMode::ChargeNote).to_owned(), ..shared.scores.for_md5(&md5)[0].clone() };
    let mut unstated = app_in_own_folder("ln-kinds-unstated");
    unstated.scores.push(made_as_charge_notes);
    unstated.config.judge.ln_mode = rbms_judge::ln::LnMode::LongNote;
    assert_eq!(local_best_before_reading(&unstated, &md5), None, "a record made as charge notes counted for a run played as long notes");
    unstated.config.judge.ln_mode = rbms_judge::ln::LnMode::ChargeNote;
    assert_eq!(local_best_before_reading(&unstated, &md5), Some(summary.ex_score));
}
