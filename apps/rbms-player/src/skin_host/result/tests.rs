//! The result cluster answers each id as `AbstractResult` and the reference's factories do, and a
//! host with a finished run on it has a value for everything a result skin reads.

use rbms_judge::gauge::GaugeIndex;
use rbms_model::Mode;
use rbms_render::result::TargetView;
use rbms_skin::dst::DrawStateSource;
use rbms_skin::lua::LocalTime;
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT, SkinHost, StaticScreen};
use rbms_skin::timer::TimerState;

use super::snapshot::{FinishedRun, GaugeEnds, PreviousScore, REPLAY_SLOT_COUNT, ReplaySlot, ResultInput, ResultSnapshot, TimingFigures};
use super::*;
use crate::skin_host::chart::{ChartContents, ChartMeta, ChartState};
use crate::skin_host::ir::{IrLink, IrPhase};
use crate::skin_host::options::PlayedOptions;
use crate::skin_host::score::standing::{PointFamily, ScoreSheet};
use crate::skin_host::system::{CourseStage, SystemState};
use crate::skin_host::{Cluster, IdSpace, ROUTES, ResultScene, ScreenHost};

/// The reference's maximum `int`, and what its `(int)` cast makes of the largest `long` divided by
/// a thousand and of the largest `float`.
const MAX_FLOAT_AS_INT_AFTER_DOT: i32 = 47;
const MAX_LONG_MILLIS_AS_INT: i32 = -1_511_828_489;
const MAX_LONG_HUNDREDTH_MILLIS_DIGITS: i32 = 80;

/// Ten notes: five PGREAT, three GREAT, one GOOD and one miss, 13 EX of 20, a combo of seven and
/// one bad-poor.
fn sheet() -> ScoreSheet {
    ScoreSheet { early: [3, 1, 0, 0, 0, 0], late: [2, 2, 1, 0, 0, 1], notes: 10, max_combo: 7, min_bp: 1, clear: 5, family: PointFamily::Beat7 }
}

/// A finished run held against a best that it beats in score and in lamp but not in combo, and
/// that it ties in bad-poor.
fn improved() -> ResultInput {
    ResultInput {
        sheet: sheet(),
        previous: PreviousScore { ex_score: 11, max_combo: 9, min_bp: Some(1), clear: 4 },
        target: Some(TargetView { name: "RIVAL".to_string(), ex: 14 }),
        ..ResultInput::default()
    }
}

fn snapshot_of(input: ResultInput) -> ResultSnapshot {
    ResultSnapshot::of(input)
}

/// The screen with the graph on the normal gauge and nothing saved.
fn scene() -> ResultScene {
    ResultScene { gauge_type: GaugeIndex::Normal.index(), ..ResultScene::default() }
}

/// The finished run on the screen, with the graph on the normal gauge.
fn on_screen(snapshot: &ResultSnapshot) -> ResultState<'_> {
    ResultState::of(FinishedRun::new(snapshot, scene()))
}

#[test]
fn a_run_is_compared_with_the_best_it_replaced_one_figure_at_a_time() {
    let snapshot = snapshot_of(improved());
    let state = on_screen(&snapshot);

    let options = [
        (OPTION_UPDATE_SCORE, true),
        (OPTION_DRAW_SCORE, false),
        (OPTION_UPDATE_MAXCOMBO, false),
        (OPTION_DRAW_MAXCOMBO, false),
        (OPTION_UPDATE_MISSCOUNT, false),
        (OPTION_DRAW_MISSCOUNT, true),
        (OPTION_UPDATE_SCORERANK, true),
        (OPTION_DRAW_SCORERANK, false),
        (OPTION_UPDATE_TARGET, false),
        (OPTION_DRAW_TARGET, false),
        (OPTION_1PWIN, false),
        (OPTION_2PWIN, true),
        (OPTION_DRAW, false),
    ];
    for (id, expected) in options {
        assert_eq!(state.boolean(id), Some(expected), "option {id}");
    }
    let numbers = [
        (NUMBER_MISSCOUNT, 1),
        (NUMBER_MISSCOUNT2, 1),
        (NUMBER_TARGET_MAXCOMBO, 9),
        (NUMBER_DIFF_MAXCOMBO, -2),
        (NUMBER_TARGET_MISSCOUNT, 1),
        (NUMBER_DIFF_MISSCOUNT, 0),
        (NUMBER_CLEAR, 5),
        (NUMBER_TARGET_CLEAR, 4),
    ];
    for (id, expected) in numbers {
        assert_eq!(state.integer(id), Some(expected), "number {id}");
    }
    assert_eq!((state.image_index(INDEX_CLEAR), state.image_index(INDEX_TARGET_CLEAR)), (Some(5), Some(4)));
}

#[test]
fn a_chart_never_played_is_improved_on_by_everything_and_has_nothing_to_compare_with() {
    let snapshot = snapshot_of(ResultInput { previous: PreviousScore::default(), ..improved() });
    let state = on_screen(&snapshot);

    for id in [OPTION_UPDATE_SCORE, OPTION_UPDATE_MAXCOMBO, OPTION_UPDATE_MISSCOUNT] {
        assert_eq!(state.boolean(id), Some(true), "option {id}: a score of nothing is beaten by any, and the worst bad-poor count by any");
    }
    for id in [OPTION_DRAW_SCORE, OPTION_DRAW_MAXCOMBO, OPTION_DRAW_MISSCOUNT] {
        assert_eq!(state.boolean(id), Some(false), "option {id}");
    }
    for id in [NUMBER_TARGET_MAXCOMBO, NUMBER_DIFF_MAXCOMBO, NUMBER_TARGET_MISSCOUNT, NUMBER_DIFF_MISSCOUNT] {
        assert_eq!(state.integer(id), Some(INTEGER_ABSENT), "number {id} has no best to read");
    }
    assert_eq!(
        (state.integer(NUMBER_TARGET_CLEAR), state.image_index(INDEX_TARGET_CLEAR)),
        (Some(0), Some(0)),
        "the lamp of a chart never played is the first"
    );
}

#[test]
fn a_run_that_ties_the_best_draws_and_does_not_update() {
    let snapshot = snapshot_of(ResultInput { previous: PreviousScore { ex_score: 13, max_combo: 7, min_bp: Some(1), clear: 5 }, ..improved() });
    let state = on_screen(&snapshot);

    for (update, draw) in
        [(OPTION_UPDATE_SCORE, OPTION_DRAW_SCORE), (OPTION_UPDATE_MAXCOMBO, OPTION_DRAW_MAXCOMBO), (OPTION_UPDATE_MISSCOUNT, OPTION_DRAW_MISSCOUNT)]
    {
        assert_eq!((state.boolean(update), state.boolean(draw)), (Some(false), Some(true)), "option {update} and {draw}");
    }
    assert_eq!((state.boolean(OPTION_UPDATE_SCORERANK), state.boolean(OPTION_DRAW_SCORERANK)), (Some(false), Some(true)));
}

#[test]
fn the_target_is_beaten_tied_or_missed() {
    let outcomes = |target: u32| {
        let snapshot = snapshot_of(ResultInput { target: Some(TargetView { name: String::new(), ex: target }), ..improved() });
        let state = on_screen(&snapshot);
        [OPTION_1PWIN, OPTION_2PWIN, OPTION_DRAW, OPTION_UPDATE_TARGET, OPTION_DRAW_TARGET].map(|id| state.boolean(id))
    };
    assert_eq!(outcomes(12), [Some(true), Some(false), Some(false), Some(true), Some(false)]);
    assert_eq!(outcomes(13), [Some(false), Some(false), Some(true), Some(false), Some(true)]);
    assert_eq!(outcomes(14), [Some(false), Some(true), Some(false), Some(false), Some(false)]);
}

#[test]
fn a_clear_and_a_failure_are_each_others_opposite() {
    let read = |stage_clear: u8, course_clear: Option<u8>| {
        let snapshot = snapshot_of(ResultInput { sheet: ScoreSheet { clear: stage_clear, ..sheet() }, course_clear, ..improved() });
        let state = on_screen(&snapshot);
        (state.boolean(OPTION_RESULT_CLEAR), state.boolean(OPTION_RESULT_FAIL))
    };

    assert_eq!(read(5, None), (Some(true), Some(false)));
    assert_eq!(read(1, None), (Some(false), Some(true)));
    assert_eq!(read(5, Some(1)), (Some(false), Some(true)), "a failed course fails the stage that did not");
    assert_eq!(read(5, Some(5)), (Some(true), Some(false)));
}

#[test]
fn the_kind_of_gauge_the_graph_is_on_is_one_of_groove_hard_and_ex_rate() {
    let snapshot = snapshot_of(improved());
    let on_gauge = |kind: GaugeIndex| {
        let state = ResultState::of(FinishedRun::new(&snapshot, ResultScene { gauge_type: kind.index(), ..ResultScene::default() }));
        [OPTION_GAUGE_GROOVE, OPTION_GAUGE_HARD, OPTION_GAUGE_EX].map(|id| state.boolean(id) == Some(true))
    };

    let expected = [
        (GaugeIndex::AssistEasy, [true, false, true]),
        (GaugeIndex::Easy, [true, false, true]),
        (GaugeIndex::Normal, [true, false, false]),
        (GaugeIndex::Hard, [false, true, false]),
        (GaugeIndex::ExHard, [false, true, true]),
        (GaugeIndex::Hazard, [false, true, true]),
        (GaugeIndex::Class, [false, true, false]),
        (GaugeIndex::ExClass, [false, true, true]),
        (GaugeIndex::ExHardClass, [false, true, true]),
    ];
    for (kind, groove_hard_ex) in expected {
        assert_eq!(on_gauge(kind), groove_hard_ex, "{kind:?}");
    }
}

#[test]
fn each_replay_slot_is_in_exactly_one_of_its_three_states() {
    let snapshot = snapshot_of(improved());
    let slots = |replay: [ReplaySlot; REPLAY_SLOT_COUNT]| {
        let state = ResultState::of(FinishedRun::new(&snapshot, ResultScene { replay, ..scene() }));
        let ids: [[i32; 3]; REPLAY_SLOT_COUNT] = [
            [OPTION_NO_REPLAYDATA, OPTION_REPLAYDATA, OPTION_REPLAYDATA_SAVED],
            [OPTION_NO_REPLAYDATA2, OPTION_REPLAYDATA2, OPTION_REPLAYDATA2_SAVED],
            [OPTION_NO_REPLAYDATA3, OPTION_REPLAYDATA3, OPTION_REPLAYDATA3_SAVED],
            [OPTION_NO_REPLAYDATA4, OPTION_REPLAYDATA4, OPTION_REPLAYDATA4_SAVED],
        ];
        ids.map(|triple| triple.map(|id| state.boolean(id)))
    };

    let all_missing = slots([ReplaySlot::Missing; REPLAY_SLOT_COUNT]);
    assert!(all_missing.iter().all(|slot| *slot == [Some(true), Some(false), Some(false)]));
    let mixed = slots([ReplaySlot::Saved, ReplaySlot::Exists, ReplaySlot::Missing, ReplaySlot::Saved]);
    assert_eq!(mixed[0], [Some(false), Some(false), Some(true)], "the slot this run was saved to");
    assert_eq!(mixed[1], [Some(false), Some(true), Some(false)]);
    assert_eq!(mixed[2], [Some(true), Some(false), Some(false)]);
    assert_eq!(mixed[3], [Some(false), Some(false), Some(true)]);
}

#[test]
fn the_timing_figures_are_cut_to_whole_numbers_and_their_digits_after_the_dot() {
    let read = |timing: TimingFigures| {
        let snapshot = snapshot_of(ResultInput { timing, ..improved() });
        let state = on_screen(&snapshot);
        [
            NUMBER_AVERAGE_DURATION,
            NUMBER_AVERAGE_DURATION_AFTERDOT,
            NUMBER_AVERAGE_TIMING,
            NUMBER_AVERAGE_TIMING_AFTERDOT,
            NUMBER_STDDEV_TIMING,
            NUMBER_STDDEV_TIMING_AFTERDOT,
        ]
        .map(|id| state.integer(id))
    };

    let early = TimingFigures { average_ms: 4.25, std_dev_ms: 7.5, center_ms: 150, avg_duration_us: 12_345 };
    assert_eq!(read(early), [Some(12), Some(34), Some(4), Some(25), Some(7), Some(50)]);
    let late = TimingFigures { average_ms: -3.75, ..early };
    assert_eq!(read(late)[2..4], [Some(-3), Some(-75)], "a late average has negative digits");
}

#[test]
fn a_run_with_nothing_judged_reads_the_reference_s_maximums() {
    let snapshot =
        snapshot_of(ResultInput { timing: TimingFigures { average_ms: f32::MAX, std_dev_ms: -1.0, center_ms: 150, avg_duration_us: i64::MAX }, ..improved() });
    let state = on_screen(&snapshot);

    assert_eq!(state.integer(NUMBER_AVERAGE_TIMING), Some(i32::MAX));
    assert_eq!(state.integer(NUMBER_AVERAGE_TIMING_AFTERDOT), Some(MAX_FLOAT_AS_INT_AFTER_DOT));
    assert_eq!(state.integer(NUMBER_STDDEV_TIMING), Some(-1));
    assert_eq!(state.integer(NUMBER_AVERAGE_DURATION), Some(MAX_LONG_MILLIS_AS_INT), "the long is cut to its low 32 bits as a Java int cast does");
    assert_eq!(state.integer(NUMBER_AVERAGE_DURATION_AFTERDOT), Some(MAX_LONG_HUNDREDTH_MILLIS_DIGITS));
}

#[test]
fn the_timing_floats_read_the_distribution_s_reach_where_the_reference_means_the_average() {
    let snapshot =
        snapshot_of(ResultInput { timing: TimingFigures { average_ms: 4.25, std_dev_ms: 7.5, center_ms: 150, avg_duration_us: 12_345 }, ..improved() });
    let state = on_screen(&snapshot);

    assert_eq!(state.float(FLOAT_DURATION_AVERAGE), Some(12.345));
    assert_eq!(state.float(FLOAT_TIMING_AVERAGE), Some(0.15), "the array centre over a thousand, as FloatPropertyFactory reads it");
    assert_eq!(state.float(FLOAT_TIMIGN_STDDEV), Some(7.5));
}

#[test]
fn the_course_titles_fill_the_first_of_ten_texts_and_the_rest_are_empty() {
    let snapshot = snapshot_of(ResultInput { course_titles: vec!["First".to_string(), "Second".to_string()], ..improved() });
    let state = on_screen(&snapshot);

    assert_eq!(state.text(STRING_COURSE1_TITLE).as_deref(), Some("First"));
    assert_eq!(state.text(STRING_COURSE2_TITLE).as_deref(), Some("Second"));
    assert_eq!(state.text(STRING_COURSE3_TITLE).as_deref(), Some(""));
    assert_eq!(state.text(STRING_COURSE10_TITLE).as_deref(), Some(""));
    assert_eq!((state.text(STRING_COURSE1_TITLE - 1), state.text(STRING_COURSE10_TITLE + 1)), (None, None));
}

#[test]
fn without_a_snapshot_nothing_is_answered() {
    let state = ResultState::default();

    assert_eq!(state.boolean(OPTION_RESULT_CLEAR), None);
    assert_eq!(state.integer(NUMBER_CLEAR), None);
    assert_eq!(state.image_index(INDEX_CLEAR), None);
    assert_eq!(state.float(FLOAT_DURATION_AVERAGE), None);
    assert_eq!(state.text(STRING_COURSE1_TITLE), None);
}

/// Whether the cluster answers an id of one of the spaces it is routed in.
fn answered(state: &ResultState<'_>, space: IdSpace, id: i32) -> Option<bool> {
    match space {
        IdSpace::Integer => Some(state.integer(id).is_some()),
        IdSpace::Boolean => Some(state.boolean(id).is_some()),
        IdSpace::ImageIndex => Some(state.image_index(id).is_some()),
        IdSpace::Float => Some(state.float(id).is_some()),
        IdSpace::Text => Some(state.text(id).is_some()),
        IdSpace::Rate | IdSpace::Offset => None,
    }
}

/// The ids routed to the result cluster that the reference gives no property: the ones between the
/// update options it never defined.
const ROUTED_WITHOUT_A_PROPERTY: [(IdSpace, i32); 4] = [
    (IdSpace::Boolean, OPTION_UPDATE_TRIAL),
    (IdSpace::Boolean, OPTION_UPDATE_IRRANK),
    (IdSpace::Boolean, OPTION_UPDATE_TRIAL + DRAW_OPTION_OFFSET),
    (IdSpace::Boolean, OPTION_UPDATE_IRRANK + DRAW_OPTION_OFFSET),
];

/// How far the draw option of a figure sits from its update option (1330 from 330).
const DRAW_OPTION_OFFSET: i32 = OPTION_DRAW_SCORE - OPTION_UPDATE_SCORE;

#[test]
fn every_id_routed_to_the_result_cluster_is_answered_unless_the_reference_defines_none() {
    let snapshot = snapshot_of(improved());
    let state = on_screen(&snapshot);

    for route in ROUTES.iter().filter(|route| route.cluster == Cluster::Result) {
        for id in route.first..=route.last {
            let undefined = ROUTED_WITHOUT_A_PROPERTY.contains(&(route.space, id));
            let owned_by_score = route.space == IdSpace::Integer && matches!(id, NUMBER_HIGHSCORE2 | NUMBER_SCORE3 | NUMBER_DIFF_HIGHSCORE2 | NUMBER_MAXCOMBO3);
            if undefined || owned_by_score {
                assert_eq!(answered(&state, route.space, id), Some(false), "{:?} id {id} is left to another cluster", route.space);
            } else if let Some(answer) = answered(&state, route.space, id) {
                assert!(answer, "{:?} id {id} in {route:?}", route.space);
            }
        }
    }
}

/// A chart the player knows everything about, with a difficulty and a judge rank, for the host
/// tests.
fn chart() -> ChartMeta<'static> {
    ChartMeta {
        title: "Result Title",
        subtitle: "[SP ANOTHER]",
        genre: "TRANCE",
        artist: "Composer",
        subartist: "feat. Singer",
        md5: "md5-of-the-chart",
        sha256: "sha256-of-the-chart",
        table_name: "Satellite",
        table_level: "sl",
        level: 9,
        difficulty: 3,
        mode: Some(Mode::BEAT_7K),
        judge: Some(2),
        notes: Some(10),
        total: Some(300.0),
        contents: ChartContents { stagefile: Some(true), ..ChartContents::default() },
        ..ChartMeta::default()
    }
}

/// The clock the system cluster is given.
const CLOCK: LocalTime = LocalTime { year: 2026, month: 10, day: 10, hour: 12, minute: 30, second: 45 };

/// A snapshot with everything a result skin reads set: a gauge with a history, the settings the run
/// was played with, a replay saved into the first slot and a course of two stages.
fn full_snapshot(previous: PreviousScore) -> ResultSnapshot {
    let mut ends = [None; GaugeIndex::COUNT];
    ends[GaugeIndex::Normal.index()] = Some(72.34);
    snapshot_of(ResultInput {
        gauge: GaugeEnds { ends },
        previous,
        options: PlayedOptions { random: 2, random_2p: 1, ..PlayedOptions::default() },
        favorite_chart: Some(false),
        course_titles: vec!["Stage 1".to_string(), "Stage 2".to_string()],
        timing: TimingFigures { average_ms: 2.5, std_dev_ms: 6.0, center_ms: 150, avg_duration_us: 54_321 },
        ..improved()
    })
}

/// Puts a snapshot on a host the way the result screen does, with a chart in hand, a course in
/// progress and the ranking offline.
fn host_for<'a>(timers: &'a TimerState, meta: &'a ChartMeta<'a>, snapshot: &'a ResultSnapshot) -> ScreenHost<'a> {
    let mut host = ScreenHost::new(1_000_000, timers);
    host.static_screen = Some(StaticScreen::Result);
    host.chart = ChartState::Chart(meta);
    host.system = SystemState { clock: Some(CLOCK), player_name: Some("guest"), course: Some(CourseStage { index: 1, count: 3 }), ..SystemState::default() };
    let shown = ResultScene { replay: [ReplaySlot::Saved, ReplaySlot::Missing, ReplaySlot::Missing, ReplaySlot::Missing], ..scene() };
    host.show_result(FinishedRun::new(snapshot, shown));
    host.ir.link = Some(IrLink::new(false, IrPhase::Offline));
    host
}

#[test]
fn exactly_one_option_of_each_group_the_result_skin_picks_among_is_on() {
    let meta = chart();
    let timers = TimerState::new();
    let snapshot = full_snapshot(improved().previous);
    let host = host_for(&timers, &meta, &snapshot);

    let on = |ids: Vec<i32>| ids.into_iter().filter(|id| host.boolean(*id) == Some(true)).count();
    assert_eq!(on(vec![OPTION_RESULT_CLEAR, OPTION_RESULT_FAIL]), 1);
    assert_eq!(on((OPTION_RESULT_AAA_1P..=OPTION_RESULT_F_1P).collect()), 1);
    assert_eq!(on((OPTION_BEST_AAA_1P..=OPTION_BEST_F_1P).collect()), 1);
    assert_eq!(on((OPTION_NOW_AAA_1P..=OPTION_NOW_F_1P).collect()), 1);
    assert_eq!(on((OPTION_1P_AAA..=OPTION_1P_F).collect()), 1);
    assert_eq!(on((OPTION_JUDGE_VERYHARD..=OPTION_JUDGE_VERYEASY).collect()), 1, "the chart's judge rank is one of five");
    assert_eq!(on((OPTION_DIFFICULTY0..=OPTION_DIFFICULTY5).collect()), 1);
    assert_eq!(on(vec![OPTION_7KEYSONG, OPTION_5KEYSONG, OPTION_14KEYSONG, OPTION_10KEYSONG, OPTION_9KEYSONG, OPTION_24KEYSONG, OPTION_24KEYDPSONG]), 1);
    assert_eq!(on(vec![OPTION_NO_REPLAYDATA, OPTION_REPLAYDATA, OPTION_REPLAYDATA_SAVED]), 1);
    assert_eq!(on(vec![OPTION_COURSE_STAGE1, OPTION_COURSE_STAGE2, OPTION_COURSE_STAGE3, OPTION_COURSE_STAGE4, OPTION_COURSE_STAGE_FINAL]), 1);
}

/// The options `m5-decide-result.md` appendix A lists for a result screen, one by one.
fn appendix_options() -> Vec<i32> {
    [
        vec![OPTION_RESULT_CLEAR, OPTION_RESULT_FAIL],
        (OPTION_RESULT_AAA_1P..=OPTION_RESULT_F_1P).collect(),
        (OPTION_BEST_AAA_1P..=OPTION_BEST_F_1P).collect(),
        vec![OPTION_UPDATE_SCORE, OPTION_UPDATE_MAXCOMBO, OPTION_UPDATE_MISSCOUNT],
        (OPTION_JUDGE_VERYHARD..=OPTION_JUDGE_VERYEASY).collect(),
        vec![OPTION_NO_REPLAYDATA, OPTION_REPLAYDATA, OPTION_REPLAYDATA_SAVED],
        (OPTION_NO_REPLAYDATA2..=OPTION_REPLAYDATA4_SAVED).collect(),
        (OPTION_7KEYSONG..=OPTION_9KEYSONG).collect(),
        vec![OPTION_24KEYSONG, OPTION_24KEYDPSONG],
        (OPTION_DIFFICULTY0..=OPTION_DIFFICULTY5).collect(),
        vec![OPTION_STAGEFILE],
        vec![OPTION_COURSE_STAGE1, OPTION_COURSE_STAGE2, OPTION_COURSE_STAGE3, OPTION_COURSE_STAGE_FINAL, OPTION_MODE_COURSE],
        vec![OPTION_ONLINE, OPTION_IR_WAITING],
    ]
    .concat()
}

/// The numbers the same appendix lists.
fn appendix_numbers() -> Vec<i32> {
    [
        vec![NUMBER_TOTALNOTES, NUMBER_SCORE_RATE, NUMBER_SCORE_RATE_AFTERDOT, NUMBER_MAXCOMBO2, NUMBER_GROOVEGAUGE, NUMBER_GROOVEGAUGE_AFTERDOT],
        (NUMBER_PERFECT..=NUMBER_POOR).collect(),
        (NUMBER_EARLY_GREAT..=NUMBER_TOTALLATE).collect(),
        vec![NUMBER_SONGGAUGE_TOTAL, NUMBER_CLEAR, NUMBER_TARGET_CLEAR],
        (NUMBER_AVERAGE_TIMING..=NUMBER_STDDEV_TIMING_AFTERDOT).collect(),
        vec![NUMBER_HIGHSCORE2, NUMBER_SCORE3, NUMBER_DIFF_HIGHSCORE2, NUMBER_DIFF_MAXCOMBO, NUMBER_MISSCOUNT2, NUMBER_DIFF_MISSCOUNT],
        (NUMBER_TIME_YEAR..=NUMBER_TIME_SECOND).collect(),
        vec![NUMBER_PLAYLEVEL],
    ]
    .concat()
}

#[test]
fn a_host_with_a_finished_run_on_it_has_a_value_for_every_id_the_result_skin_reads() {
    let meta = chart();
    let timers = TimerState::new();
    let snapshot = full_snapshot(improved().previous);
    let host = host_for(&timers, &meta, &snapshot);

    for id in appendix_options() {
        assert!(host.boolean(id).is_some() && host.boolean(-id).is_some(), "option {id}");
    }
    for id in appendix_numbers() {
        assert_ne!(host.integer(id), INTEGER_ABSENT, "number {id}");
    }
    for id in [
        STRING_PLAYER,
        STRING_TITLE,
        STRING_FULLTITLE,
        STRING_GENRE,
        STRING_ARTIST,
        STRING_SUBARTIST,
        STRING_TABLE_FULL,
        STRING_SONG_HASH_MD5,
        STRING_SONG_HASH_SHA256,
    ] {
        assert!(!host.text(id).is_empty(), "text {id}");
    }
    for id in STRING_COURSE1_TITLE..=STRING_COURSE10_TITLE {
        assert!(host.result.text(id).is_some(), "course text {id}");
    }
    for id in (STRING_RANKING1_NAME..=STRING_RANKING10_NAME).chain([STRING_IR_NAME]) {
        assert!(host.ir.text(id).is_some(), "ranking text {id}");
    }
    assert_eq!((host.text(STRING_COURSE1_TITLE), host.text(STRING_COURSE2_TITLE)), (Cow::Borrowed("Stage 1"), Cow::Borrowed("Stage 2")));

    assert_eq!((host.image_index(INDEX_CLEAR), host.image_index(BUTTON_RANDOM_1P), host.image_index(BUTTON_RANDOM_2P)), (5, 2, 1));
    assert_eq!(host.image_index(BUTTON_FAVORITTE_CHART), 0);
    for id in BUTTON_ASSIST_EXJUDGE..=BUTTON_ASSIST_NOMINE {
        assert_eq!(host.image_index(id), 0, "assist button {id}");
    }
    assert_eq!((host.boolean(OPTION_ONLINE), host.boolean(OPTION_OFFLINE), host.boolean(-OPTION_IR_WAITING)), (Some(false), Some(true), Some(true)));
    assert_eq!(host.boolean(OPTION_REPLAYDATA_SAVED), Some(true), "the first slot holds the run that was just saved");
    assert_ne!(host.float(FLOAT_DURATION_AVERAGE), FLOAT_ABSENT);
}

#[test]
fn on_a_first_run_only_the_figures_of_a_best_that_does_not_exist_are_absent() {
    let meta = chart();
    let timers = TimerState::new();
    let snapshot = full_snapshot(PreviousScore::default());
    let host = host_for(&timers, &meta, &snapshot);

    let absent: Vec<i32> = appendix_numbers().into_iter().filter(|id| host.integer(*id) == INTEGER_ABSENT).collect();
    assert_eq!(absent, [NUMBER_DIFF_MAXCOMBO, NUMBER_DIFF_MISSCOUNT]);
    for id in appendix_options() {
        assert!(host.boolean(id).is_some(), "option {id}");
    }
}

#[test]
fn a_host_with_no_snapshot_falls_through_as_before() {
    let timers = TimerState::new();
    let host = ScreenHost::new(1_000_000, &timers);

    assert_eq!(host.integer(NUMBER_SCORE), INTEGER_ABSENT);
    assert_eq!(host.image_index(INDEX_CLEAR), IMAGE_INDEX_ABSENT);
    assert_eq!(host.boolean(OPTION_RESULT_CLEAR), None);
}
