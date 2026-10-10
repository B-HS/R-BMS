//! The score cluster answers each id as the reference's factories do, for a run that has ended and
//! for one still being played.

use rbms_judge::gauge::GaugeIndex;
use rbms_render::result::TargetView;
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, INTEGER_ABSENT, NameSpace, ScoreSlot, ScoreSnapshot, reference_implements};

use super::standing::{PointFamily, ScoreSheet, TargetPace};
use super::*;
use crate::skin_host::ResultScene;
use crate::skin_host::result::snapshot::{FinishedRun, GaugeEnds, PreviousScore, ResultInput, ResultSnapshot};
use crate::skin_host::{Cluster, IdSpace, ROUTES};

/// Ten notes: five PGREAT, three GREAT, one GOOD and one miss, 13 EX of 20, a combo of seven.
fn sheet() -> ScoreSheet {
    ScoreSheet { early: [3, 1, 0, 0, 0, 0], late: [2, 2, 1, 0, 0, 1], notes: 10, max_combo: 7, min_bp: 1, clear: 5, family: PointFamily::Beat7 }
}

/// The finished run of [`sheet`] with a best of 11 before it and a target of 14, on a normal gauge
/// that ended at 72.34.
fn snapshot() -> ResultSnapshot {
    let mut ends = [None; GaugeIndex::COUNT];
    ends[GaugeIndex::Normal.index()] = Some(72.34);
    ResultSnapshot::of(ResultInput {
        sheet: sheet(),
        gauge: GaugeEnds { ends },
        previous: PreviousScore { ex_score: 11, max_combo: 5, min_bp: Some(3), clear: 4 },
        target: Some(TargetView { name: "RIVAL".to_string(), ex: 14 }),
        ..ResultInput::default()
    })
}

/// The screen with the graph on the normal gauge.
fn scene() -> ResultScene {
    ResultScene { gauge_type: GaugeIndex::Normal.index(), ..ResultScene::default() }
}

/// The finished run on the screen, with the graph on the normal gauge.
fn on_screen(snapshot: &ResultSnapshot) -> ScoreState<'_> {
    ScoreState::of_result(FinishedRun::new(snapshot, scene()))
}

/// The numbers a finished run reads, each with the value the reference's factory gives it.
const NUMBERS: &[(i32, i32)] = &[
    (NUMBER_SCORE, 13),
    (NUMBER_SCORE2, 13),
    (NUMBER_SCORE3, 13),
    (NUMBER_MAXSCORE, 20),
    (NUMBER_MAXCOMBO, 7),
    (NUMBER_MAXCOMBO2, 7),
    (NUMBER_MAXCOMBO3, 7),
    (NUMBER_POINT, 142_000),
    (NUMBER_SCORE_RATE, 65),
    (NUMBER_SCORE_RATE_AFTERDOT, 0),
    (NUMBER_GROOVEGAUGE, 72),
    (NUMBER_GROOVEGAUGE_AFTERDOT, 3),
    (NUMBER_DIFF_EXSCORE, -1),
    (NUMBER_DIFF_EXSCORE2, -1),
    (NUMBER_DIFF_TARGETSCORE, -1),
    (NUMBER_PERFECT, 5),
    (NUMBER_GREAT, 3),
    (NUMBER_GOOD, 1),
    (NUMBER_BAD, 0),
    (NUMBER_POOR, 0),
    (NUMBER_PERFECT2, 5),
    (NUMBER_GREAT2, 3),
    (NUMBER_GOOD2, 1),
    (NUMBER_BAD2, 0),
    (NUMBER_POOR2, 0),
    (NUMBER_PERFECT_RATE, 50),
    (NUMBER_GREAT_RATE, 30),
    (NUMBER_GOOD_RATE, 10),
    (NUMBER_BAD_RATE, 0),
    (NUMBER_POOR_RATE, 0),
    (NUMBER_EARLY_PERFECT, 3),
    (NUMBER_LATE_PERFECT, 2),
    (NUMBER_EARLY_GREAT, 1),
    (NUMBER_LATE_GREAT, 2),
    (NUMBER_EARLY_GOOD, 0),
    (NUMBER_LATE_GOOD, 1),
    (NUMBER_EARLY_BAD, 0),
    (NUMBER_LATE_BAD, 0),
    (NUMBER_EARLY_POOR, 0),
    (NUMBER_LATE_POOR, 0),
    (NUMBER_MISS, 1),
    (NUMBER_EARLY_MISS, 0),
    (NUMBER_LATE_MISS, 1),
    (NUMBER_TOTALEARLY, 1),
    (NUMBER_TOTALLATE, 4),
    (NUMBER_COMBOBREAK, 0),
    (NUMBER_POOR_PLUS_MISS, 1),
    (NUMBER_BAD_PLUS_POOR_PLUS_MISS, 1),
    (NUMBER_TOTAL_RATE, 65),
    (NUMBER_TOTAL_RATE_AFTERDOT, 0),
    (NUMBER_SCORE_RATE2, 65),
    (NUMBER_SCORE_RATE_AFTERDOT2, 0),
    (NUMBER_TARGET_SCORE, 14),
    (NUMBER_TARGET_SCORE2, 14),
    (NUMBER_RIVAL_SCORE, 14),
    (NUMBER_TARGET_SCORE_RATE, 70),
    (NUMBER_TARGET_TOTAL_RATE, 70),
    (NUMBER_TARGET_SCORE_RATE2, 70),
    (NUMBER_TARGET_SCORE_RATE_AFTERDOT, 0),
    (NUMBER_TARGET_TOTAL_RATE_AFTERDOT, 0),
    (NUMBER_TARGET_SCORE_RATE_AFTERDOT2, 0),
    (NUMBER_HIGHSCORE, 11),
    (NUMBER_HIGHSCORE2, 11),
    (NUMBER_DIFF_HIGHSCORE, 2),
    (NUMBER_DIFF_HIGHSCORE2, 2),
    (NUMBER_DIFF_NEXTRANK, 1),
    (NUMBER_BEST_RATE, 55),
    (NUMBER_BEST_RATE_AFTERDOT, 0),
];

/// The ids this cluster leaves to others: the chart's note counts, and the comparisons with the best
/// that only a result screen makes. It answers none of them, and the reference implements no number
/// under the id of the combo.
const LEFT_TO_OTHERS: [i32; 10] = [
    NUMBER_TOTALNOTES,
    NUMBER_TOTALNOTES2,
    NUMBER_COMBO,
    NUMBER_MISSCOUNT,
    NUMBER_MISSCOUNT2,
    NUMBER_CLEAR,
    NUMBER_TARGET_MAXCOMBO,
    NUMBER_DIFF_MAXCOMBO,
    NUMBER_TARGET_MISSCOUNT,
    NUMBER_DIFF_MISSCOUNT,
];

#[test]
fn a_finished_run_reads_each_number_as_the_reference_defines_it() {
    let snapshot = snapshot();
    let state = on_screen(&snapshot);

    for (id, expected) in NUMBERS {
        assert_eq!(state.integer(*id), Some(*expected), "number {id}");
    }
    for id in LEFT_TO_OTHERS {
        assert_eq!(state.integer(id), None, "number {id} belongs to another cluster");
    }
}

#[test]
fn the_rate_digits_split_one_rate_into_its_whole_percent_and_the_two_digits_after_the_dot() {
    let sheet = ScoreSheet { early: [1, 0, 0, 0, 0, 0], late: [0; 6], notes: 3, ..ScoreSheet::default() };
    let snapshot = ResultSnapshot::of(ResultInput { sheet, target: Some(TargetView { name: String::new(), ex: 4 }), ..ResultInput::default() });
    let state = on_screen(&snapshot);

    let digits = |whole: i32, after_dot: i32| (state.integer(whole), state.integer(after_dot));
    assert_eq!(digits(NUMBER_SCORE_RATE, NUMBER_SCORE_RATE_AFTERDOT), (Some(33), Some(33)));
    assert_eq!(digits(NUMBER_TOTAL_RATE, NUMBER_TOTAL_RATE_AFTERDOT), (Some(33), Some(33)));
    assert_eq!(digits(NUMBER_SCORE_RATE2, NUMBER_SCORE_RATE_AFTERDOT2), (Some(33), Some(33)));
    assert_eq!(digits(NUMBER_TARGET_SCORE_RATE, NUMBER_TARGET_SCORE_RATE_AFTERDOT), (Some(66), Some(66)));
    assert_eq!(digits(NUMBER_BEST_RATE, NUMBER_BEST_RATE_AFTERDOT), (Some(0), Some(0)));
}

#[test]
fn the_gauge_digit_after_the_dot_shows_a_gauge_that_is_not_empty_as_not_empty() {
    let mut run = snapshot();
    let mut read = |value: f32| {
        run.gauge.ends[GaugeIndex::Normal.index()] = Some(value);
        let state = on_screen(&run);
        (state.integer(NUMBER_GROOVEGAUGE), state.integer(NUMBER_GROOVEGAUGE_AFTERDOT), state.float(FLOAT_GROOVEGAUGE_1P))
    };

    assert_eq!(read(100.0), (Some(100), Some(0), Some(100.0)));
    assert_eq!(read(0.05), (Some(0), Some(1), Some(0.05)), "a twentieth of a percent still reads as a tenth");
    assert_eq!(read(0.0), (Some(0), Some(0), Some(0.0)));
    assert_eq!(read(9.9), (Some(9), Some(9), Some(9.9)));
}

#[test]
fn a_gauge_with_no_history_reads_as_absent() {
    let mut run = snapshot();
    run.gauge.ends = [None; GaugeIndex::COUNT];
    let state = on_screen(&run);

    assert_eq!((state.integer(NUMBER_GROOVEGAUGE), state.integer(NUMBER_GROOVEGAUGE_AFTERDOT)), (Some(INTEGER_ABSENT), Some(INTEGER_ABSENT)));
    assert_eq!(state.float(FLOAT_GROOVEGAUGE_1P), Some(FLOAT_ABSENT));
}

#[test]
fn the_judgement_shares_have_no_value_for_a_chart_with_no_notes() {
    let snapshot = ResultSnapshot::of(ResultInput::default());
    let state = on_screen(&snapshot);

    for id in NUMBER_PERFECT_RATE..=NUMBER_POOR_RATE {
        assert_eq!(state.integer(id), Some(INTEGER_ABSENT), "number {id}");
    }
    for id in FLOAT_PERFECT_RATE..=FLOAT_POOR_RATE {
        assert_eq!(state.float(id), Some(FLOAT_ABSENT), "float {id}");
    }
    assert_eq!(state.integer(NUMBER_MAXSCORE), Some(0), "the maximum score is zero, not absent");
}

#[test]
fn a_finished_run_reads_each_float_and_rate_as_the_reference_defines_it() {
    let snapshot = snapshot();
    let state = on_screen(&snapshot);

    let floats = [
        (FLOAT_PERFECT_RATE, 0.5),
        (FLOAT_GREAT_RATE, 0.3),
        (FLOAT_GOOD_RATE, 0.1),
        (FLOAT_BAD_RATE, 0.0),
        (FLOAT_POOR_RATE, 0.0),
        (FLOAT_SCORE_RATE, 0.65),
        (FLOAT_TOTAL_RATE, 0.65),
        (FLOAT_SCORE_RATE2, 0.65),
        (FLOAT_BEST_RATE, 0.55),
        (FLOAT_RIVAL_RATE, 0.7),
        (FLOAT_TARGET_RATE, 0.7),
        (FLOAT_TARGET_RATE2, 0.7),
        (FLOAT_GROOVEGAUGE_1P, 72.34),
    ];
    for (id, expected) in floats {
        assert_eq!(state.float(id), Some(expected), "float {id}");
    }
    let rates = [
        (RATE_SCORE, 0.65),
        (RATE_SCORE_FINAL, 0.65),
        (RATE_BESTSCORE_NOW, 0.55),
        (RATE_BESTSCORE, 0.55),
        (RATE_TARGETSCORE_NOW, 0.7),
        (RATE_TARGETSCORE, 0.7),
    ];
    for (id, expected) in rates {
        assert_eq!(state.rate(id), Some(expected), "rate {id}");
    }
}

#[test]
fn the_rates_of_the_browser_read_zero_on_any_other_screen() {
    let snapshot = snapshot();
    let state = on_screen(&snapshot);

    for id in [RATE_PGREAT, RATE_GREAT, RATE_GOOD, RATE_BAD, RATE_POOR, RATE_MAXCOMBO, RATE_EXSCORE] {
        assert_eq!(state.rate(id), Some(0.0), "rate {id}");
    }
    assert_eq!(state.rate(RATE_EXSCORE - 1), None, "the rate between them has no property in the reference");
}

/// How many options of one band of eight rank options are on, and which.
fn on_in(state: &ScoreState<'_>, first: i32, count: i32) -> Vec<i32> {
    (first..first + count).filter(|id| state.boolean(*id) == Some(true)).collect()
}

#[test]
fn exactly_one_option_of_each_rank_band_is_on_and_the_cumulative_band_counts_down_from_the_rank() {
    let snapshot = snapshot();
    let state = on_screen(&snapshot);

    assert_eq!(on_in(&state, OPTION_1P_AAA, 8), [OPTION_1P_AAA + 3], "65% is the fourth band from the top, B");
    assert_eq!(on_in(&state, OPTION_RESULT_AAA_1P, 8), [OPTION_RESULT_AAA_1P + 3]);
    assert_eq!(on_in(&state, OPTION_NOW_AAA_1P, 8), [OPTION_NOW_AAA_1P + 3]);
    assert_eq!(on_in(&state, OPTION_BEST_AAA_1P, 8), [OPTION_BEST_AAA_1P + 4], "the best score's 55% is a band lower, C");
    assert_eq!(on_in(&state, OPTION_AAA, 8), [OPTION_AAA + 3, OPTION_AAA + 4, OPTION_AAA + 5, OPTION_AAA + 6, OPTION_AAA + 7], "B and every rank below it");
}

#[test]
fn the_options_of_the_gauge_ranges_are_off_for_a_finished_run_and_pick_one_range_for_a_live_gauge() {
    let snapshot = snapshot();
    let finished = on_screen(&snapshot);
    assert_eq!(on_in(&finished, OPTION_1P_0_9, 11), Vec::<i32>::new(), "only the gauge of a run in progress is in a range");

    let live = ScoreState { gauge: Some(GaugeReading { value: 55.0, max: 100.0, kind: 2, live: true }), ..finished };
    assert_eq!(on_in(&live, OPTION_1P_0_9, 11), [OPTION_1P_50_59]);
    let full = ScoreState { gauge: Some(GaugeReading { value: 100.0, max: 100.0, kind: 2, live: true }), ..finished };
    assert_eq!(on_in(&full, OPTION_1P_0_9, 11), [OPTION_1P_100]);
    let hard = ScoreState { gauge: Some(GaugeReading { value: 20.0, max: 100.0, kind: 3, live: true }), ..finished };
    assert_eq!(on_in(&hard, OPTION_1P_0_9, 11), [OPTION_1P_20_29]);
}

#[test]
fn a_judgement_exists_when_it_was_given_at_least_once() {
    let snapshot = snapshot();
    let state = on_screen(&snapshot);

    let exist: Vec<Option<bool>> = (OPTION_PERFECT_EXIST..=OPTION_MISS_EXIST).map(|id| state.boolean(id)).collect();
    assert_eq!(exist, [Some(true), Some(true), Some(true), Some(false), Some(false), Some(true)]);
}

#[test]
fn what_main_state_asks_of_a_finished_run() {
    let snapshot = snapshot();
    let state = on_screen(&snapshot);

    assert_eq!((state.gauge(), state.gauge_type()), (Some(0.0), Some(0)), "the reference answers zero for both outside play");
    assert_eq!((state.judge(0), state.judge(1), state.judge(5), state.judge(6)), (Some(5), Some(3), Some(1), Some(0)));
    assert_eq!(state.score(ScoreSlot::Current), Some(ScoreSnapshot { rate: 0.65, exscore: 13 }));
    assert_eq!(
        state.score(ScoreSlot::Best),
        Some(ScoreSnapshot { rate: 0.55, exscore: 11 }),
        "the rate is paced to the notes gone by, the score is the best itself"
    );
    assert_eq!(state.score(ScoreSlot::Rival), Some(ScoreSnapshot { rate: 0.7, exscore: 14 }));
}

#[test]
fn a_live_gauge_is_what_main_state_reports_while_a_run_is_in_progress() {
    let snapshot = snapshot();
    let live = ScoreState { gauge: Some(GaugeReading { value: 43.5, max: 100.0, kind: 3, live: true }), ..on_screen(&snapshot) };

    assert_eq!((live.gauge(), live.gauge_type()), (Some(43.5), Some(3)));
}

#[test]
fn without_a_run_nothing_is_answered() {
    let state = ScoreState::default();

    assert_eq!(state.integer(NUMBER_SCORE), None);
    assert_eq!(state.boolean(OPTION_NOW_AAA_1P), None);
    assert_eq!(state.rate(RATE_SCORE), None);
    assert_eq!(state.float(FLOAT_SCORE_RATE), None);
    assert_eq!((state.gauge(), state.gauge_type(), state.judge(0), state.score(ScoreSlot::Current)), (None, None, None, None));
}

#[test]
fn a_run_in_progress_is_rated_against_the_notes_gone_by() {
    let sheet = ScoreSheet { early: [2, 0, 0, 0, 0, 0], late: [1, 1, 0, 0, 0, 0], notes: 10, max_combo: 4, ..ScoreSheet::default() };
    let run = RunScore::in_progress(sheet, 4, TargetPace { best_score: 11, rival_score: 14, total_notes: 10 });
    let state = ScoreState { run: Some(&run), gauge: None };

    assert_eq!((state.integer(NUMBER_SCORE_RATE), state.integer(NUMBER_SCORE_RATE_AFTERDOT)), (Some(87), Some(50)), "87.5% of the notes gone by");
    assert_eq!(state.integer(NUMBER_TOTAL_RATE), Some(35), "and 35% of the chart");
    assert_eq!(state.integer(NUMBER_DIFF_HIGHSCORE), Some(3), "7 against a best that would stand at 4 by now");
    assert_eq!(state.integer(NUMBER_DIFF_TARGETSCORE), Some(2), "and a target that would stand at 5");
    assert_eq!(on_in(&state, OPTION_NOW_AAA_1P, 8), [OPTION_NOW_AAA_1P + 1], "AA over the notes gone by");
    assert_eq!(on_in(&state, OPTION_RESULT_AAA_1P, 8), [OPTION_RESULT_AAA_1P + 1]);
}

/// Whether the cluster answers an id of a space it can be routed in.
fn answered(state: &ScoreState<'_>, space: IdSpace, id: i32) -> Option<bool> {
    match space {
        IdSpace::Integer => Some(state.integer(id).is_some()),
        IdSpace::Boolean => Some(state.boolean(id).is_some()),
        IdSpace::Float => Some(state.float(id).is_some()),
        IdSpace::Rate => Some(state.rate(id).is_some()),
        IdSpace::ImageIndex | IdSpace::Text | IdSpace::Offset => None,
    }
}

#[test]
fn every_id_the_reference_implements_in_a_run_routed_to_this_cluster_is_answered_unless_it_is_left_to_another() {
    let snapshot = snapshot();
    let state = on_screen(&snapshot);

    for route in ROUTES.iter().filter(|route| route.cluster == Cluster::Score) {
        let Some(name_space) = (match route.space {
            IdSpace::Integer => Some(NameSpace::Integer),
            IdSpace::Boolean => Some(NameSpace::Boolean),
            IdSpace::Float => Some(NameSpace::Float),
            IdSpace::Rate => Some(NameSpace::Rate),
            _ => None,
        }) else {
            continue;
        };
        for id in route.first..=route.last {
            let left_to_others = route.space == IdSpace::Integer && LEFT_TO_OTHERS.contains(&id);
            if reference_implements(name_space, id) && !left_to_others {
                assert_eq!(answered(&state, route.space, id), Some(true), "{:?} id {id} in {route:?}", route.space);
            }
        }
    }
}
