//! The play screen's clusters answer each id as the reference's factories do, for a fixture run
//! written for the purpose, and the host routes them together with the score and the load.

use rbms_chart::to_model;
use rbms_model::Mode;
use rbms_parser::parse;
use rbms_play::{NullSink, PlaySession, SessionClock, SessionOptions};
use rbms_skin::dst::DrawStateSource;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT, SkinHost};
use rbms_skin::timer::{TIMER_OFF, TimerState};

use super::*;
use crate::skin_host::options::PlayedOptions;
use crate::skin_host::score::GaugeReading;
use crate::skin_host::{Cluster, IdSpace, ROUTES, ScreenHost, clusters_of};

/// Four notes on one key lane at 2.0, 2.5, 3.0 and 3.5 seconds, judged at the mode's stock widths.
const CHART: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n";

const FIRST_NOTE_US: i64 = 2_000_000;
const NOTE_GAP_US: i64 = 500_000;
const PRELUDE_US: i64 = 1_000_000;
const BETWEEN_THIRD_AND_FOURTH_US: i64 = 3_200_000;
const EARLY_GREAT_US: i64 = 40_000;
const LATE_GOOD_US: i64 = 100_000;

/// How long the reference keeps the fixture chart in its playing state, in milliseconds.
const PLAY_TIME_MS: i32 = 8_500;

/// When the play timer of the host switched on, and the frame the host is built at: 3.25 seconds in.
const PLAY_STARTED_US: i64 = 2_000_000;
const FRAME_US: i64 = 5_250_000;

/// The fixture with three of its four notes judged: a PGREAT on time, a GREAT early and a GOOD late.
fn midway() -> PlaySession {
    let mut session = PlaySession::new(to_model(&parse(CHART), Mode::BEAT_7K), SessionOptions::default());
    session.tick(SessionClock::at(PRELUDE_US), &mut NullSink);
    for (note, offset_us) in [(0, 0), (1, -EARLY_GREAT_US), (2, LATE_GOOD_US)] {
        session.press(0, FIRST_NOTE_US + note * NOTE_GAP_US + offset_us, &mut NullSink);
    }
    session.tick(SessionClock::at(BETWEEN_THIRD_AND_FOURTH_US), &mut NullSink);
    session
}

/// What the screen says about the fixture run: a seven-key run 3.25 seconds into its play timer, at
/// a scroll speed of 2 over a tempo of 150, with the cover a quarter of the lane and the last
/// judgement a GOOD 100 ms late.
fn live() -> PlayLive {
    PlayLive {
        phase: PlayPhase::Play,
        kind: PlayKind::Play,
        best_score: 4,
        target_score: 6,
        judgements: [Some(Judgement { judge: 2, timing_ms: -100 }), None, None],
        lanes: LaneSettings {
            hispeed: 2.0,
            lane_cover: 0.25,
            lift: 0.5,
            hidden: 0.375,
            lane_cover_on: true,
            lift_on: false,
            hidden_on: true,
            constant_on: false,
            fix_hispeed: 3,
        },
        scroll: Scroll { now_bpm: 150.0, main_bpm: 120.0, min_bpm: 60.0, max_bpm: 240.0, region_ms: travel_region_ms(150.0, 2.0, 1.0) },
        clock: PlayClock { now_us: FRAME_US, started_us: PLAY_STARTED_US },
        judge_timing_ms: -12,
        bga_on: true,
        cover_keys_held: false,
        long_note_mode: 1,
        played: PlayedOptions { random: 2, random_2p: 3, double_option: 1, custom_judge: true, bpm_guide: true, ..PlayedOptions::default() },
        load_progress: 1.0,
    }
}

fn shown_with(live: PlayLive) -> PlayShown {
    PlayShown::of(&midway(), &live)
}

fn shown() -> PlayShown {
    shown_with(live())
}

/// The cluster over a frame, asked directly.
fn state(shown: &PlayShown) -> PlayState<'_> {
    PlayState::of(shown)
}

fn host_over<'a>(timers: &'a TimerState, shown: &'a PlayShown) -> ScreenHost<'a> {
    let mut host = ScreenHost::new(FRAME_US, timers);
    host.show_play(shown);
    host
}

#[test]
fn a_cluster_with_no_frame_answers_nothing() {
    let state = PlayState::default();

    assert_eq!((state.boolean(OPTION_AUTOPLAYOFF), state.integer(NUMBER_NOWBPM), state.image_index(BUTTON_GAUGE_1P)), (None, None, None));
    assert_eq!((state.rate(RATE_LANECOVER), state.float(FLOAT_HISPEED)), (None, None));
}

#[test]
fn the_kind_of_run_sets_the_autoplay_and_replay_options() {
    let kinds = [
        (PlayKind::Play, [false, true, true, false]),
        (PlayKind::Practice, [false, true, true, false]),
        (PlayKind::Autoplay, [true, false, false, false]),
        (PlayKind::Replay, [false, true, false, true]),
    ];
    for (kind, expected) in kinds {
        let shown = shown_with(PlayLive { kind, ..live() });
        let state = state(&shown);
        let got = [OPTION_AUTOPLAYON, OPTION_AUTOPLAYOFF, OPTION_REPLAY_OFF, OPTION_REPLAY_PLAYING].map(|id| state.boolean(id));
        assert_eq!(got, expected.map(Some), "{kind:?}");
    }
}

#[test]
fn no_state_of_the_screen_is_the_practice_state_and_a_background_animation_is_on_or_off() {
    for phase in [PlayPhase::Preload, PlayPhase::Ready, PlayPhase::Play, PlayPhase::Failed, PlayPhase::Finished] {
        assert_eq!(state(&shown_with(PlayLive { phase, ..live() })).boolean(OPTION_STATE_PRACTICE), Some(false), "{phase:?}");
    }
    let practised = shown_with(PlayLive { kind: PlayKind::Practice, ..live() });
    assert_eq!(state(&practised).boolean(OPTION_STATE_PRACTICE), Some(false), "a practice slice being played is not a practice range being chosen");

    let on = shown();
    assert_eq!((state(&on).boolean(OPTION_BGAON), state(&on).boolean(OPTION_BGAOFF)), (Some(true), Some(false)));
    let off = shown_with(PlayLive { bga_on: false, ..live() });
    assert_eq!((state(&off).boolean(OPTION_BGAON), state(&off).boolean(OPTION_BGAOFF)), (Some(false), Some(true)));
}

#[test]
fn the_gauge_options_follow_the_gauge_by_the_number_the_reference_gives_it() {
    for (kind, groove, hard, ex) in [
        (0, true, false, true),
        (1, true, false, true),
        (2, true, false, false),
        (3, false, true, false),
        (4, false, true, true),
        (5, false, true, true),
        (6, false, true, false),
        (7, false, true, true),
        (8, false, true, true),
    ] {
        let mut shown = shown();
        shown.gauge = GaugeReading { kind, ..shown.gauge };
        let state = state(&shown);
        assert_eq!(
            (state.boolean(OPTION_GAUGE_GROOVE), state.boolean(OPTION_GAUGE_HARD), state.boolean(OPTION_GAUGE_EX)),
            (Some(groove), Some(hard), Some(ex)),
            "gauge {kind}"
        );
    }
    assert_eq!(state(&shown()).boolean(OPTION_GAUGE_GROOVE_2P), None, "the reference has no second player's gauge option");
    assert_eq!(state(&shown()).boolean(OPTION_GAUGE_HARD_2P), None);
    assert_eq!(state(&shown()).boolean(OPTION_GAUGE_EX_2P), None);
}

#[test]
fn the_border_option_is_on_while_the_gauge_is_over_its_border() {
    let mut shown = shown();
    assert_eq!(state(&shown).boolean(OPTION_1P_BORDER_OR_MORE), Some(true));
    shown.qualified = false;
    assert_eq!(state(&shown).boolean(OPTION_1P_BORDER_OR_MORE), Some(false));
}

/// What a judgement of one kind and one timing turns on, as `(perfect, early, late)` of the first side.
fn side_options(judge: usize, timing_ms: i64) -> (bool, bool, bool) {
    let shown = shown_with(PlayLive { judgements: [Some(Judgement { judge, timing_ms }), None, None], ..live() });
    let state = state(&shown);
    let on = |id: i32| state.boolean(id) == Some(true);
    (on(OPTION_1P_PERFECT), on(OPTION_1P_EARLY), on(OPTION_1P_LATE))
}

#[test]
fn at_most_one_judgement_option_of_a_side_is_on_and_a_perfect_has_no_timing() {
    assert_eq!(side_options(0, 25), (true, false, false), "a PGREAT, early or not, is only perfect");
    assert_eq!(side_options(0, -25), (true, false, false));
    assert_eq!(side_options(1, 40), (false, true, false));
    assert_eq!(side_options(1, -40), (false, false, true));
    assert_eq!(side_options(2, 100), (false, true, false));
    assert_eq!(side_options(3, -150), (false, false, true));
    assert_eq!(side_options(4, -300), (false, false, true), "a note that went by unhit is late");
    assert_eq!(side_options(5, 90), (false, true, false), "and a press that took no note has a timing too");
    assert_eq!(side_options(1, 0), (false, false, false), "a judgement within a millisecond of the note is neither early nor late");
}

#[test]
fn each_side_answers_for_its_own_last_judgement_and_a_side_never_judged_answers_for_none() {
    let shown = shown_with(PlayLive { judgements: [Some(Judgement { judge: 0, timing_ms: 3 }), Some(Judgement { judge: 2, timing_ms: -50 }), None], ..live() });
    let state = state(&shown);
    let all = [
        OPTION_1P_PERFECT,
        OPTION_1P_EARLY,
        OPTION_1P_LATE,
        OPTION_2P_PERFECT,
        OPTION_2P_EARLY,
        OPTION_2P_LATE,
        OPTION_3P_PERFECT,
        OPTION_3P_EARLY,
        OPTION_3P_LATE,
    ];

    assert_eq!(all.map(|id| state.boolean(id)), [true, false, false, false, false, true, false, false, false].map(Some));
}

#[test]
fn only_the_first_of_the_six_judgements_has_an_option_of_its_own() {
    let shown = shown();
    let state = state(&shown);

    for id in [OPTION_1P_GREAT, OPTION_1P_GOOD, OPTION_1P_BAD, OPTION_1P_POOR, OPTION_1P_MISS, OPTION_2P_GREAT, OPTION_2P_GOOD, OPTION_3P_GREAT] {
        assert_eq!(state.boolean(id), None, "option {id} has no property in the reference");
    }
}

#[test]
fn the_lane_options_follow_what_is_switched_on() {
    let on = shown_with(PlayLive {
        lanes: LaneSettings { lane_cover_on: true, lift_on: true, hidden_on: true, constant_on: true, ..live().lanes },
        cover_keys_held: true,
        ..live()
    });
    let off = shown_with(PlayLive {
        lanes: LaneSettings { lane_cover_on: false, lift_on: false, hidden_on: false, constant_on: false, ..live().lanes },
        cover_keys_held: false,
        ..live()
    });
    let ids = [OPTION_LANECOVER1_CHANGING, OPTION_LANECOVER1_ON, OPTION_LIFT1_ON, OPTION_HIDDEN1_ON, OPTION_CONSTANT];

    assert_eq!(ids.map(|id| state(&on).boolean(id)), [Some(true); 5]);
    assert_eq!(ids.map(|id| state(&off).boolean(id)), [Some(false); 5]);
}

#[test]
fn the_scroll_speed_is_read_whole_and_in_hundredths() {
    let shown = shown_with(PlayLive { lanes: LaneSettings { hispeed: 3.5, ..live().lanes }, ..live() });
    let state = state(&shown);

    assert_eq!(state.integer(NUMBER_HISPEED), Some(3));
    assert_eq!(state.integer(NUMBER_HISPEED_AFTERDOT), Some(50));
    assert_eq!(state.integer(NUMBER_HISPEED_LR2), Some(350));
    assert_eq!(state.float(FLOAT_HISPEED), Some(3.5));
    assert_eq!(state.integer(NUMBER_JUDGETIMING), Some(-12));
}

#[test]
fn the_lane_figures_are_read_in_thousandths_of_the_share_each_is_set_to_whether_or_not_it_is_on() {
    let shown = shown_with(PlayLive {
        lanes: LaneSettings { lane_cover: 0.25, lift: 0.1, hidden: 0.375, lane_cover_on: false, lift_on: false, hidden_on: false, ..live().lanes },
        ..live()
    });
    let state = state(&shown);

    assert_eq!(state.integer(NUMBER_LANECOVER1), Some(250));
    assert_eq!(state.integer(NUMBER_LIFT1), Some(100));
    assert_eq!(state.integer(NUMBER_HIDDEN1), Some(375));
    assert_eq!(state.integer(NUMBER_LANECOVER2), Some(224), "the lift is a float widened to a double, so (1 - 0.1) * 0.25 * 1000 falls just short of 225");
}

#[test]
fn the_duration_is_the_travel_time_under_the_cover_and_the_green_number_three_fifths_of_it() {
    let state_with = |cover_on: bool| {
        let shown = shown_with(PlayLive { lanes: LaneSettings { lane_cover_on: cover_on, ..live().lanes }, ..live() });
        (state(&shown).integer(NUMBER_DURATION), state(&shown).integer(NUMBER_DURATION_GREEN))
    };

    assert_eq!(state_with(true), (Some(600), Some(360)), "800 ms across the lane with a quarter of it covered");
    assert_eq!(state_with(false), (Some(800), Some(480)), "and with the cover off");
}

#[test]
fn the_duration_at_a_tempo_counts_the_cover_when_the_id_says_so_whether_or_not_it_is_on() {
    let shown = shown_with(PlayLive { lanes: LaneSettings { lane_cover_on: false, ..live().lanes }, ..live() });
    let state = state(&shown);

    let expected = [
        (NUMBER_DURATION_LANECOVER_ON, 600),
        (NUMBER_DURATION_GREEN_LANECOVER_ON, 360),
        (NUMBER_DURATION_LANECOVER_OFF, 800),
        (NUMBER_DURATION_GREEN_LANECOVER_OFF, 480),
        (NUMBER_MAINBPM_DURATION_LANECOVER_ON, 750),
        (NUMBER_MAINBPM_DURATION_GREEN_LANECOVER_ON, 450),
        (NUMBER_MAINBPM_DURATION_LANECOVER_OFF, 1000),
        (NUMBER_MAINBPM_DURATION_GREEN_LANECOVER_OFF, 600),
        (NUMBER_MINBPM_DURATION_LANECOVER_ON, 1500),
        (NUMBER_MINBPM_DURATION_GREEN_LANECOVER_ON, 900),
        (NUMBER_MINBPM_DURATION_LANECOVER_OFF, 2000),
        (NUMBER_MINBPM_DURATION_GREEN_LANECOVER_OFF, 1200),
        (NUMBER_MAXBPM_DURATION_LANECOVER_ON, 375),
        (NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_ON, 225),
        (NUMBER_MAXBPM_DURATION_LANECOVER_OFF, 500),
        (NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_OFF, 300),
    ];
    for (id, duration) in expected {
        assert_eq!(state.integer(id), Some(duration), "number {id}");
    }
    assert_eq!(NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_OFF - NUMBER_DURATION_LANECOVER_ON + 1, expected.len() as i32, "every id of the run is covered");
}

#[test]
fn the_tempo_is_read_whole() {
    let shown = shown_with(PlayLive { scroll: Scroll { now_bpm: 172.9, ..live().scroll }, ..live() });

    assert_eq!(state(&shown).integer(NUMBER_NOWBPM), Some(172));
}

#[test]
fn the_clock_figures_count_from_the_play_timer_and_the_time_left_runs_a_second_past_the_end() {
    let figures = |shown: &PlayShown| {
        let state = state(shown);
        [NUMBER_PLAYTIME_MINUTE, NUMBER_PLAYTIME_SECOND, NUMBER_TIMELEFT_MINUTE, NUMBER_TIMELEFT_SECOND].map(|id| state.integer(id))
    };

    assert_eq!(figures(&shown()), [0, 3, 0, 6].map(Some), "3.25 s in, 8.5 s of playing time: 6.25 s and a second of grace left");
    let late = shown_with(PlayLive { clock: PlayClock { now_us: PLAY_STARTED_US + 75_500_000, ..live().clock }, ..live() });
    assert_eq!(figures(&late), [1, 15, 0, 0].map(Some), "75.5 s in: nothing is left");
    let stopped = shown_with(PlayLive { clock: PlayClock::stopped(FRAME_US), ..live() });
    assert_eq!(figures(&stopped), [0, 0, 0, 9].map(Some), "before the play timer is on the time played is zero and all of it is left");
    assert_eq!(PLAY_TIME_MS, shown().play_time_ms);
}

#[test]
fn the_duration_of_the_last_judgement_of_each_side_is_its_timing_in_milliseconds() {
    let shown =
        shown_with(PlayLive { judgements: [Some(Judgement { judge: 2, timing_ms: -100 }), Some(Judgement { judge: 1, timing_ms: 37 }), None], ..live() });
    let state = state(&shown);

    assert_eq!(
        (state.integer(VALUE_JUDGE_1P_DURATION), state.integer(VALUE_JUDGE_2P_DURATION), state.integer(VALUE_JUDGE_3P_DURATION)),
        (Some(-100), Some(37), Some(0))
    );
}

#[test]
fn the_lane_cover_slider_is_the_cover_less_what_the_lift_takes() {
    let share = |cover_on: bool, lift_on: bool| {
        let shown = shown_with(PlayLive { lanes: LaneSettings { lane_cover_on: cover_on, lift_on, ..live().lanes }, ..live() });
        (state(&shown).rate(RATE_LANECOVER), state(&shown).rate(RATE_LANECOVER2))
    };

    assert_eq!(share(true, false), (Some(0.25), Some(0.25)));
    assert_eq!(share(true, true), (Some(0.125), Some(0.125)), "a half of the lane is under the lift");
    assert_eq!(share(false, true), (Some(0.0), Some(0.0)), "a cover that is off reads nothing, whatever the lift");
}

#[test]
fn the_music_progress_is_the_share_of_the_playing_time_gone_and_stops_at_the_whole() {
    let progress = |clock: PlayClock| {
        let shown = shown_with(PlayLive { clock, ..live() });
        (state(&shown).rate(RATE_MUSIC_PROGRESS), state(&shown).rate(RATE_MUSIC_PROGRESS_BAR))
    };

    assert_eq!(progress(PlayClock { now_us: PLAY_STARTED_US + 4_250_000, started_us: PLAY_STARTED_US }), (Some(0.5), Some(0.5)));
    assert_eq!(progress(PlayClock { now_us: PLAY_STARTED_US + 20_000_000, started_us: PLAY_STARTED_US }), (Some(1.0), Some(1.0)));
    assert_eq!(progress(PlayClock::stopped(FRAME_US)), (Some(0.0), Some(0.0)), "before the play timer is on");
}

#[test]
fn the_images_that_show_what_was_chosen_read_the_options_the_run_was_laid_out_with() {
    let shown = shown_with(PlayLive { lanes: LaneSettings { fix_hispeed: 3, ..live().lanes }, ..live() });
    let state = state(&shown);

    let expected = [
        (BUTTON_GAUGE_1P, 2),
        (BUTTON_RANDOM_1P, 2),
        (BUTTON_RANDOM_2P, 3),
        (BUTTON_DPOPTION, 1),
        (BUTTON_HSFIX, 3),
        (BUTTON_ASSIST_EXJUDGE, 1),
        (BUTTON_ASSIST_CONSTANT, 0),
        (BUTTON_ASSIST_JUDGEAREA, 0),
        (BUTTON_ASSIST_LEGACY, 0),
        (BUTTON_ASSIST_MARKNOTE, 0),
        (BUTTON_ASSIST_BPMGUIDE, 1),
        (BUTTON_ASSIST_NOMINE, 0),
        (BUTTON_LNMODE, 1),
    ];
    for (id, index) in expected {
        assert_eq!(state.image_index(id), Some(index), "image index {id}");
    }
    assert_eq!(state.image_index(BUTTON_GAUGE_1P + 1), None, "the reference has no second player's gauge image");
}

#[test]
fn the_key_judgement_images_and_the_ids_other_clusters_own_are_not_answered() {
    let shown = shown();
    let state = state(&shown);

    assert_eq!(state.image_index(VALUE_JUDGE_1P_SCRATCH), None, "the judge engine keeps no judgement per key");
    assert_eq!(state.image_index(VALUE_JUDGE_2P_KEY9), None);
    assert_eq!(state.image_index(VALUE_JUDGE_1P_KEY10), None);
    assert_eq!(state.boolean(OPTION_NOW_LOADING), None, "the load is cluster M's");
    assert_eq!(state.boolean(OPTION_REPLAY_RECORDING), None, "the reference implements no recording option");
    assert_eq!(state.integer(NUMBER_SCORE), None, "and the score is cluster B's");
    assert_eq!(state.rate(RATE_SCORE), None);
}

#[test]
fn every_id_routed_to_the_play_cluster_that_the_reference_implements_is_answered_unless_it_has_no_source_here() {
    use rbms_skin::property::{NameSpace, reference_implements};

    let shown = shown();
    let state = state(&shown);
    let unsourced = |space: IdSpace, id: i32| {
        space == IdSpace::ImageIndex
            && ((VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY9).contains(&id) || (VALUE_JUDGE_1P_KEY10..=VALUE_JUDGE_2P_KEY99).contains(&id))
    };

    for route in ROUTES.iter().filter(|route| route.cluster == Cluster::Play) {
        let name_space = match route.space {
            IdSpace::Boolean => NameSpace::Boolean,
            IdSpace::Integer => NameSpace::Integer,
            IdSpace::ImageIndex => NameSpace::ImageIndex,
            IdSpace::Rate => NameSpace::Rate,
            IdSpace::Float => NameSpace::Float,
            IdSpace::Text | IdSpace::Offset => continue,
        };
        for id in route.first..=route.last {
            if !reference_implements(name_space, id) || unsourced(route.space, id) {
                continue;
            }
            let answered = match route.space {
                IdSpace::Boolean => state.boolean(id).is_some(),
                IdSpace::Integer => state.integer(id).is_some(),
                IdSpace::ImageIndex => state.image_index(id).is_some(),
                IdSpace::Rate => state.rate(id).is_some(),
                IdSpace::Float => state.float(id).is_some(),
                IdSpace::Text | IdSpace::Offset => continue,
            };
            assert!(answered, "{:?} id {id} in {route:?} is routed to the play cluster", route.space);
        }
    }
}

#[test]
fn the_play_ids_are_routed_to_the_cluster_that_reads_them_ahead_of_the_one_that_answers_them_elsewhere() {
    assert_eq!(clusters_of(IdSpace::Integer, NUMBER_HISPEED).collect::<Vec<_>>(), [Cluster::Play, Cluster::Options]);
    assert_eq!(clusters_of(IdSpace::Integer, NUMBER_JUDGETIMING).collect::<Vec<_>>(), [Cluster::Play, Cluster::Options]);
    assert_eq!(clusters_of(IdSpace::Boolean, OPTION_GAUGE_HARD).collect::<Vec<_>>(), [Cluster::Result, Cluster::Play]);
    assert_eq!(clusters_of(IdSpace::ImageIndex, BUTTON_RANDOM_1P).collect::<Vec<_>>(), [Cluster::Play, Cluster::Options]);
    assert_eq!(clusters_of(IdSpace::Rate, RATE_MUSIC_PROGRESS).collect::<Vec<_>>(), [Cluster::Play]);
    assert_eq!(clusters_of(IdSpace::Integer, NUMBER_NOWBPM).collect::<Vec<_>>(), [Cluster::Play]);
}

/// The options the skins of the pack read on the play screen (m2 §5.1, m3 §5.1) that a play frame
/// answers, with the value each has in the fixture run: seven keys, three of four notes gone by, the
/// normal gauge full, a 50% rate over the notes gone by and 37.5% over the chart.
const OPTIONS: &[(i32, bool)] = &[
    (OPTION_AUTOPLAYOFF, true),
    (OPTION_AUTOPLAYON, false),
    (OPTION_NOW_LOADING, false),
    (OPTION_LOADED, true),
    (OPTION_REPLAY_PLAYING, false),
    (OPTION_STATE_PRACTICE, false),
    (OPTION_GAUGE_HARD, false),
    (OPTION_1P_0_9, false),
    (OPTION_1P_10_19, false),
    (OPTION_1P_20_29, false),
    (OPTION_1P_100, true),
    (OPTION_1P_AAA, false),
    (OPTION_1P_AA, false),
    (OPTION_1P_A, false),
    (OPTION_1P_B, false),
    (OPTION_1P_C, true),
    (OPTION_1P_D, false),
    (OPTION_1P_E, false),
    (OPTION_1P_F, false),
    (OPTION_AAA, false),
    (OPTION_AA, false),
    (OPTION_A, false),
    (OPTION_1P_PERFECT, false),
    (OPTION_1P_EARLY, false),
    (OPTION_1P_LATE, true),
    (OPTION_2P_EARLY, false),
    (OPTION_2P_LATE, false),
    (OPTION_GOOD_EXIST, true),
    (OPTION_BAD_EXIST, false),
    (OPTION_POOR_EXIST, false),
    (OPTION_LANECOVER1_CHANGING, false),
    (OPTION_LANECOVER1_ON, true),
    (OPTION_LIFT1_ON, false),
    (OPTION_CONSTANT, false),
];

/// The numbers the skins of the pack read on the play screen (m2 §5.2, m3 §5.2) that a play frame
/// answers, with the value each has in the fixture run. The 100 and the 407 are the full gauge's.
const NUMBERS: &[(i32, i32)] = &[
    (NUMBER_JUDGETIMING, -12),
    (NUMBER_LANECOVER1, 250),
    (NUMBER_MAXCOMBO, 3),
    (NUMBER_MAXCOMBO2, 3),
    (NUMBER_POINT, 105_000),
    (NUMBER_SCORE2, 3),
    (NUMBER_TARGET_SCORE, 6),
    (NUMBER_DIFF_HIGHSCORE, 0),
    (NUMBER_DIFF_TARGETSCORE, -1),
    (NUMBER_GROOVEGAUGE, 100),
    (NUMBER_GROOVEGAUGE_AFTERDOT, 0),
    (NUMBER_NOWBPM, 150),
    (NUMBER_TIMELEFT_MINUTE, 0),
    (NUMBER_TIMELEFT_SECOND, 6),
    (NUMBER_LOADING_PROGRESS, 100),
    (NUMBER_DURATION, 600),
    (NUMBER_DURATION_GREEN, 360),
    (NUMBER_LIFT1, 500),
    (NUMBER_PERFECT, 1),
    (NUMBER_GREAT, 1),
    (NUMBER_GOOD, 1),
    (NUMBER_BAD, 0),
    (NUMBER_POOR, 0),
    (NUMBER_EARLY_GREAT, 1),
    (NUMBER_LATE_GREAT, 0),
    (NUMBER_EARLY_GOOD, 0),
    (NUMBER_LATE_GOOD, 1),
    (NUMBER_EARLY_BAD, 0),
    (NUMBER_LATE_BAD, 0),
    (NUMBER_EARLY_POOR, 0),
    (NUMBER_LATE_POOR, 0),
    (NUMBER_TOTALEARLY, 1),
    (NUMBER_TOTALLATE, 1),
    (NUMBER_BAD_PLUS_POOR_PLUS_MISS, 0),
    (VALUE_JUDGE_1P_DURATION, -100),
    (VALUE_JUDGE_2P_DURATION, 0),
    (NUMBER_MAINBPM_DURATION_GREEN_LANECOVER_ON, 450),
    (NUMBER_MINBPM_DURATION_GREEN_LANECOVER_ON, 900),
    (NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_ON, 225),
];

#[test]
fn the_host_answers_the_options_of_the_pack_for_a_play_frame() {
    let timers = TimerState::new();
    let shown = shown();
    let host = host_over(&timers, &shown);

    for (id, expected) in OPTIONS {
        assert_eq!(host.boolean(*id), Some(*expected), "option {id}");
        assert_eq!(host.boolean(-*id), Some(!*expected), "option {id} negated");
    }
}

#[test]
fn the_host_answers_the_numbers_of_the_pack_for_a_play_frame() {
    let timers = TimerState::new();
    let shown = shown();
    let host = host_over(&timers, &shown);

    for (id, expected) in NUMBERS {
        assert_eq!(host.integer(*id), *expected, "number {id}");
    }
}

#[test]
fn the_host_answers_the_graphs_and_sliders_of_the_pack_for_a_play_frame() {
    let timers = TimerState::new();
    let shown = shown();
    let host = host_over(&timers, &shown);

    let rates = [
        (RATE_SCORE, 0.375),
        (RATE_SCORE_FINAL, 0.5),
        (RATE_BESTSCORE_NOW, 0.375),
        (RATE_BESTSCORE, 0.5),
        (RATE_TARGETSCORE_NOW, 0.5),
        (RATE_TARGETSCORE, 0.75),
        (RATE_LANECOVER, 0.25),
        (RATE_MUSIC_PROGRESS, 3_250.0 / 8_500.0),
        (RATE_LOAD_PROGRESS, 1.0),
    ];
    for (id, expected) in rates {
        assert_eq!(host.rate(id), Some(expected), "rate {id}");
        assert_eq!(host.float(id), expected, "rate {id} read as a float");
    }
    assert_eq!(host.float(FLOAT_GROOVEGAUGE_1P), 100.0);
    assert_eq!(host.float(FLOAT_HISPEED), 2.0);
    assert_eq!(host.float(FLOAT_SCORE_RATE), 0.5, "the rate over the notes gone by");
    assert_eq!((host.gauge(), host.gauge_type()), (100.0, 2), "and what main_state reports of the run");
    assert_eq!((host.judge(0), host.judge(1), host.judge(2), host.judge(4)), (1, 1, 1, 0));
}

#[test]
fn the_host_reads_the_rank_the_run_stands_at_over_the_notes_gone_by() {
    let timers = TimerState::new();
    let shown = shown();
    let host = host_over(&timers, &shown);

    let on = |first: i32| (first..first + 8).filter(|id| host.boolean(*id) == Some(true)).collect::<Vec<_>>();
    assert_eq!(on(OPTION_NOW_AAA_1P), [OPTION_NOW_C_1P], "50% is the C band");
    assert_eq!(on(OPTION_1P_AAA), [OPTION_1P_C]);
    assert_eq!(on(OPTION_BEST_AAA_1P), [OPTION_BEST_C_1P], "the best score's 50% too");
    assert_eq!(on(OPTION_AAA), [OPTION_D, OPTION_E, OPTION_F], "the chart's 37.5% has secured D and below");
    assert_eq!(host.integer(NUMBER_DIFF_NEXTRANK), 1);
}

#[test]
fn the_gauge_ranges_follow_the_live_gauge() {
    let timers = TimerState::new();
    let mut shown = shown();
    shown.gauge = GaugeReading { value: 37.5, max: 100.0, kind: 3, live: true };
    let host = host_over(&timers, &shown);

    let in_range: Vec<i32> = (OPTION_1P_0_9..=OPTION_1P_100).filter(|id| host.boolean(*id) == Some(true)).collect();
    assert_eq!(in_range, [OPTION_1P_30_39]);
    assert_eq!((host.integer(NUMBER_GROOVEGAUGE), host.integer(NUMBER_GROOVEGAUGE_AFTERDOT)), (37, 5));
    assert_eq!((host.gauge(), host.gauge_type()), (37.5, 3));
    assert_eq!((host.boolean(OPTION_GAUGE_HARD), host.boolean(OPTION_GAUGE_GROOVE)), (Some(true), Some(false)));
}

#[test]
fn before_the_first_judgement_the_scores_are_absent_and_the_rest_read_a_fresh_field() {
    let timers = TimerState::new();
    let fresh = {
        let session = PlaySession::new(to_model(&parse(CHART), Mode::BEAT_7K), SessionOptions::default());
        PlayShown::of(&session, &PlayLive { judgements: [None; PLAYER_SIDES], ..live() })
    };
    let host = host_over(&timers, &fresh);

    for id in [NUMBER_SCORE, NUMBER_SCORE2, NUMBER_SCORE_RATE, NUMBER_SCORE_RATE_AFTERDOT, NUMBER_TOTAL_RATE, NUMBER_PERFECT2, NUMBER_PERFECT_RATE] {
        assert_eq!(host.integer(id), INTEGER_ABSENT, "number {id}");
    }
    assert_eq!(host.float(FLOAT_SCORE_RATE), FLOAT_ABSENT);
    assert_eq!(host.float(FLOAT_TOTAL_RATE), FLOAT_ABSENT);
    assert_eq!(host.integer(NUMBER_MAXSCORE), 0, "the reference answers zero for the most the run can score");
    assert_eq!(host.integer(NUMBER_MAXCOMBO), 0);
    assert_eq!((host.integer(NUMBER_POINT), host.integer(NUMBER_DIFF_HIGHSCORE), host.integer(NUMBER_DIFF_NEXTRANK)), (0, 0, 0));
    assert_eq!(host.integer(NUMBER_HIGHSCORE), 4, "the best and the target are set from the start");
    assert_eq!(host.integer(NUMBER_TARGET_SCORE), 6);
    assert_eq!(host.rate(RATE_SCORE_FINAL), Some(0.0));
    for first in [OPTION_1P_AAA, OPTION_NOW_AAA_1P, OPTION_AAA] {
        assert!((first..first + 8).all(|id| host.boolean(id) == Some(false)), "no rank is reached before a judgement ({first})");
    }
    assert_eq!(host.integer(NUMBER_PERFECT), 0, "the judgement counts come from the judge manager, which has them from the start");
}

#[test]
fn what_the_play_frame_does_not_know_stays_absent() {
    let timers = TimerState::new();
    let shown = shown();
    let host = host_over(&timers, &shown);

    assert_eq!(host.integer(NUMBER_PLAYLEVEL), INTEGER_ABSENT, "the chart's level is cluster A's");
    assert_eq!(host.integer(NUMBER_CLEAR), INTEGER_ABSENT, "and the lamp of a finished run is the result's");
    assert_eq!(host.boolean(OPTION_1P_GREAT), None);
    assert_eq!(host.boolean(OPTION_GAUGE_EX_2P), None);
    assert_eq!(host.image_index(VALUE_JUDGE_1P_KEY1), IMAGE_INDEX_ABSENT);
    assert_eq!(host.timer_us(41), TIMER_OFF);
}

#[test]
fn a_host_over_a_play_frame_takes_the_load_from_the_frame() {
    let timers = TimerState::new();
    let preloading = shown_with(PlayLive { phase: PlayPhase::Preload, load_progress: 0.4, ..live() });
    let host = host_over(&timers, &preloading);

    assert_eq!((host.boolean(OPTION_NOW_LOADING), host.boolean(OPTION_LOADED)), (Some(true), Some(false)));
    assert_eq!(host.integer(NUMBER_LOADING_PROGRESS), 40);
    assert_eq!(host.rate(RATE_LOAD_PROGRESS), Some(0.4));
}
