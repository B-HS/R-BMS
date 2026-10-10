//! The score properties checked against the reference's own formulas.
//!
//! The expected rates and digits were worked out with an emulation of the reference's `float`
//! arithmetic, so they hold the truncations the reference makes rather than the exact quotients.

use rbms_model::Mode;

use super::*;

/// A run of ten notes: five PGREAT, three GREAT, one GOOD and one miss, seven in the longest
/// combo, with the PGREAT and GREAT split between early and late.
fn finished_sheet() -> ScoreSheet {
    ScoreSheet { early: [3, 1, 0, 0, 0, 0], late: [2, 2, 1, 0, 0, 1], notes: 10, max_combo: 7, min_bp: 1, clear: 5, family: PointFamily::Beat7 }
}

/// The same run seen four notes in: three PGREAT and a GREAT have gone by.
fn partial_sheet() -> ScoreSheet {
    ScoreSheet { early: [2, 0, 0, 0, 0, 0], late: [1, 1, 0, 0, 0, 0], notes: 10, max_combo: 4, min_bp: 0, clear: 0, family: PointFamily::Beat7 }
}

/// The best score the player had before, the target's, and the notes both were set over.
const PACE: TargetPace = TargetPace { best_score: 11, rival_score: 14, total_notes: 10 };

#[test]
fn a_sheet_adds_the_early_and_late_sides_and_scores_two_for_a_perfect_and_one_for_a_great() {
    let sheet = finished_sheet();

    assert_eq!((sheet.count(0), sheet.count(1), sheet.count(2), sheet.count(5)), (5, 3, 1, 1));
    assert_eq!((sheet.count_on(0, true), sheet.count_on(0, false)), (3, 2));
    assert_eq!(sheet.ex_score(), 13);
    assert_eq!(sheet.max_ex_score(), 20);
    assert_eq!(sheet.count(6), 0, "a judgement that does not exist was never given");
    assert_eq!(sheet.count(-1), 0);
}

#[test]
fn a_finished_run_is_rated_against_the_whole_chart_and_held_against_the_best_and_the_target() {
    let standing = ScoreStanding::of(&finished_sheet(), 10, PACE);

    assert_eq!(standing.now_ex, 13);
    assert_eq!((standing.rate, standing.rate_int, standing.rate_after_dot), (0.65, 65, 0));
    assert_eq!((standing.now_rate, standing.now_rate_int, standing.now_rate_after_dot), (0.65, 65, 0), "every note has gone by");
    assert_eq!((standing.best_score, standing.best_rate_int, standing.best_rate_after_dot), (11, 55, 0));
    assert_eq!((standing.rival_score, standing.rival_rate_int, standing.rival_rate_after_dot), (14, 70, 0));
    assert_eq!((standing.now_best_score, standing.now_rival_score), (11, 14), "with every note gone by the scores stand where they were set");
    assert_eq!(standing.now_point, 142_000);
    assert_eq!(standing.next_rank, 1, "one more EX point reaches the next rank floor");
}

#[test]
fn the_ranks_of_a_finished_run_are_exclusive_by_band_and_cumulative_by_step() {
    let standing = ScoreStanding::of(&finished_sheet(), 10, PACE);

    let now_bands: Vec<usize> = (0..RANK_COUNT).filter(|rank| standing.now_rank_band(*rank)).collect();
    assert_eq!(now_bands, [4], "a rate of 65% is in the B band, the fifth rank up from F");
    let best_bands: Vec<usize> = (0..RANK_COUNT).filter(|rank| standing.best_rank_band(*rank)).collect();
    assert_eq!(best_bands, [3], "and the best score's 55% in the C band");
    let secured: Vec<bool> = CUMULATIVE_RANK_STEPS.iter().map(|step| standing.secured(*step)).collect();
    assert_eq!(secured, [false, false, false, true, true, true, true, true], "B and everything below it is secured");
}

#[test]
fn a_run_in_progress_is_rated_against_the_notes_gone_by_and_the_scores_are_paced_to_them() {
    let standing = ScoreStanding::of(&partial_sheet(), 4, PACE);

    assert_eq!((standing.rate, standing.rate_int, standing.rate_after_dot), (0.35, 35, 0), "over the whole chart the run stands at 35%");
    assert_eq!((standing.now_rate, standing.now_rate_int, standing.now_rate_after_dot), (0.875, 87, 50), "over the notes gone by, at 87.5%");
    assert_eq!((standing.now_best_score, standing.now_rival_score), (4, 5), "a best of 11 and a target of 14 over ten notes are 4 and 5 over four");
    assert_eq!((standing.now_best_rate, standing.now_rival_rate), (0.2, 0.25));
    let now_bands: Vec<usize> = (0..RANK_COUNT).filter(|rank| standing.now_rank_band(*rank)).collect();
    assert_eq!(now_bands, [6], "87.5% is AA while the whole-chart rate is still D");
    assert!(standing.secured(9) && !standing.secured(10), "the whole chart has secured the steps up to 9 only");
}

#[test]
fn a_rate_is_cut_not_rounded() {
    let sheet = ScoreSheet { early: [1, 0, 0, 0, 0, 0], late: [0; JUDGEMENT_KINDS], notes: 3, ..ScoreSheet::default() };
    let standing = ScoreStanding::of(&sheet, 3, TargetPace { best_score: 3, rival_score: 4, total_notes: 3 });

    assert_eq!((standing.rate_int, standing.rate_after_dot), (33, 33), "two EX over six is 33.33%");
    assert_eq!((standing.rival_rate_int, standing.rival_rate_after_dot), (66, 66), "and four over six is 66.66%, not 67");
    assert_eq!((standing.best_rate, standing.now_best_score), (0.5, 3));
}

#[test]
fn a_perfect_run_has_every_step_and_nothing_left_to_gain() {
    let sheet = ScoreSheet { early: [6, 0, 0, 0, 0, 0], late: [4, 0, 0, 0, 0, 0], notes: 10, max_combo: 10, ..ScoreSheet::default() };
    let standing = ScoreStanding::of(&sheet, 10, TargetPace { total_notes: 10, ..TargetPace::default() });

    assert_eq!((standing.rate, standing.rate_int), (1.0, 100));
    assert!((0..RANK_COUNT).all(|rank| standing.now_rank_band(rank) == (rank == RANK_COUNT - 1)), "only the top band holds");
    assert!(standing.rank.iter().all(|reached| *reached));
    assert_eq!(standing.next_rank, 0);
    assert_eq!(standing.now_point, 200_000, "the share of a full count and a full combo");
}

#[test]
fn a_chart_with_no_notes_reads_as_a_full_rate_with_no_rank() {
    let standing = ScoreStanding::of(&ScoreSheet::default(), 0, TargetPace::default());

    assert_eq!((standing.rate, standing.now_rate), (1.0, 1.0));
    assert!(standing.rank.iter().all(|reached| !reached) && standing.now_rank.iter().all(|reached| !reached));
    assert!((0..RANK_COUNT).all(|rank| !standing.now_rank_band(rank)));
    assert_eq!((standing.now_point, standing.now_best_score, standing.now_best_rate), (0, 0, 0.0));
}

#[test]
fn the_points_are_weighed_for_the_kind_of_play() {
    let weighed = |family| ScoreStanding::of(&ScoreSheet { family, ..finished_sheet() }, 10, PACE).now_point;

    assert_eq!(weighed(PointFamily::Beat5), 85_000);
    assert_eq!(weighed(PointFamily::Beat7), 142_000, "seven keys add 50000 for a full combo, here seven of ten");
    assert_eq!(weighed(PointFamily::Popn), 75_000);
    assert_eq!(weighed(PointFamily::Other), 750_000);
}

#[test]
fn a_mode_is_scored_in_the_family_the_reference_gives_it() {
    assert_eq!(PointFamily::of_mode(Mode::BEAT_5K), PointFamily::Beat5);
    assert_eq!(PointFamily::of_mode(Mode::BEAT_10K), PointFamily::Beat5);
    assert_eq!(PointFamily::of_mode(Mode::BEAT_7K), PointFamily::Beat7);
    assert_eq!(PointFamily::of_mode(Mode::BEAT_14K), PointFamily::Beat7);
    assert_eq!(PointFamily::of_mode(Mode::POPN_9K), PointFamily::Popn);
    assert_eq!(PointFamily::of_mode(Mode::KEYBOARD_24K), PointFamily::Other);
}

#[test]
fn a_rank_band_that_does_not_exist_holds_for_no_run() {
    let standing = ScoreStanding::of(&finished_sheet(), 10, PACE);

    assert!(!standing.now_rank_band(RANK_COUNT));
    assert!(!standing.best_rank_band(RANK_COUNT + 3));
    assert!(!standing.secured(RANK_STEP_COUNT));
}

#[test]
fn a_run_not_yet_judged_has_the_zeros_of_a_fresh_field_and_only_the_targets_set() {
    let untouched = ScoreSheet { notes: 10, ..ScoreSheet::default() };
    let standing = ScoreStanding::before_first_judgement(&untouched, PACE);

    assert_eq!((standing.now_point, standing.now_ex, standing.next_rank), (0, 0, 0));
    assert_eq!((standing.rate, standing.rate_int, standing.rate_after_dot), (0.0, 0, 0));
    assert_eq!((standing.now_rate, standing.now_rate_int, standing.now_rate_after_dot), (0.0, 0, 0), "not the full rate of a run with nothing gone by");
    assert!(standing.rank.iter().chain(standing.now_rank.iter()).all(|reached| !*reached), "no rank is reached, not even the lowest");
    assert_eq!((standing.now_best_score, standing.now_rival_score), (0, 0));
    assert_eq!((standing.best_score, standing.best_rate_int, standing.rival_score, standing.rival_rate_int), (11, 55, 14, 70));
    assert!(standing.best_rank_band(3), "the best score's rank stands from the start");
}

#[test]
fn a_run_judged_once_is_updated_and_the_fresh_field_is_gone() {
    let sheet = ScoreSheet { early: [0, 0, 0, 0, 1, 0], notes: 10, ..ScoreSheet::default() };
    let standing = ScoreStanding::of(&sheet, 0, PACE);

    assert_eq!(standing.now_rate, 1.0, "an empty POOR consumes no note, so nothing has gone by and the rate over them is whole");
    assert!(standing.now_rank[0], "and the lowest rank is reached");
}

#[test]
fn an_engines_tally_becomes_a_sheet_with_the_unreached_notes_counted_as_bad_poor() {
    let model = rbms_chart::to_model(&rbms_parser::parse(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n"), Mode::BEAT_7K);
    let mut session = rbms_play::PlaySession::new(model, rbms_play::SessionOptions::default());
    session.tick(rbms_play::SessionClock::at(1_000_000), &mut rbms_play::NullSink);
    session.press(0, 2_000_000, &mut rbms_play::NullSink);

    let sheet = ScoreSheet::of_engine(session.judge(), Mode::BEAT_7K);

    assert_eq!((sheet.early, sheet.late), ([1, 0, 0, 0, 0, 0], [0; 6]));
    assert_eq!((sheet.notes, sheet.max_combo, sheet.min_bp), (4, 1, 3), "the three notes it has not reached are bad-poor already");
    assert_eq!(sheet.family, PointFamily::Beat7);
}
