//! The play timer driver checked against the reference's own timer code: every row of its timer
//! table, the lane numbering of every mode, and the turntable angles.

use rbms_judge::Judge;
use rbms_judge::matcher::ScratchDir;
use rbms_model::{Mode, Model, ModelMeta, Note, TimeLine};
use rbms_skin::dst::{OffsetSource, SkinOffset};
use rbms_skin::model::{Destination, JudgeDef, SkinDef};
use rbms_skin::property::generated::{OFFSET_HIDDEN_COVER, OFFSET_LANECOVER, OFFSET_LIFT, OFFSET_SCRATCHANGLE_1P, OFFSET_SCRATCHANGLE_2P};
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerId, TimerState, timer_id};

use super::*;
use crate::skin_host::ScreenHost;
use crate::skin_host::score::standing::{PointFamily, ScoreSheet, ScoreStanding, TargetPace};

/// A second on the scene clock.
const SECOND_US: i64 = 1_000_000;

/// When the runs below went ready and when they started.
const READY_US: i64 = 3 * SECOND_US;
const START_US: i64 = 4 * SECOND_US;

/// Three moments after the start, in order.
const FIRST_US: i64 = 5 * SECOND_US;
const NEXT_US: i64 = 6 * SECOND_US;
const LAST_US: i64 = 7 * SECOND_US;

/// How far into the chart the run of the table starts, and when the chart's first note is.
const START_OFFSET_US: i64 = 2 * SECOND_US;
const FIRST_NOTE_US: i64 = 3 * SECOND_US;

/// The tempo of the runs below, at which a beat is half a second and a bar two.
const TEMPO: f64 = 120.0;
const BEAT_US: i64 = SECOND_US / 2;
const BAR_US: i64 = 4 * BEAT_US;

/// What the rhythm timer reads after one beat: a thousand milliseconds, whatever the tempo.
const RHYTHM_BEAT_US: i64 = SECOND_US;

/// The lanes of seven keys: the first key, the last key and the turntable.
const FIRST_KEY_LANE: usize = 0;
const LAST_KEY_LANE: usize = 6;
const SCRATCH_LANE: usize = 7;

/// The lanes of fourteen keys on the second side: its first key and its turntable.
const SECOND_SIDE_FIRST_KEY_LANE: usize = 8;
const SECOND_SIDE_SCRATCH_LANE: usize = 15;

/// Every lane of the widest mode with nothing down, to cut a frame of keys from.
const ALL_UP: [LaneKeys; 26] = [LaneKeys::UP; 26];

fn at_ms(millis: i64) -> i64 {
    millis * MICROS_PER_MILLI
}

/// A frame of keys for `lanes` lanes with the lanes in `down` held the way each says.
fn keys(lanes: usize, down: &[(usize, LaneKeys)]) -> Vec<LaneKeys> {
    let mut frame = vec![LaneKeys::UP; lanes];
    for (lane, held) in down {
        frame[*lane] = *held;
    }
    frame
}

/// A lane held by its one key, or a turntable turned forward.
const HELD: LaneKeys = LaneKeys::down(ScratchDir::Forward);
const TURNED_BACK: LaneKeys = LaneKeys::down(ScratchDir::Backward);
const TURNED_BOTH_WAYS: LaneKeys = LaneKeys { forward: true, backward: true };

/// A setup of `mode` whose skin has `regions` judge regions.
fn setup(mode: Mode, regions: usize) -> PlaySetup {
    PlaySetup { judge_regions: regions, ..PlaySetup::new(mode) }
}

/// A run of `setup` that went ready and then started at the top of its chart.
fn started(setup: PlaySetup) -> (PlayTimerDriver, TimerState) {
    let mut driver = PlayTimerDriver::new(setup);
    let mut timers = TimerState::new();
    driver.apply(&mut timers, READY_US, SceneEvent::Ready);
    driver.apply(&mut timers, START_US, SceneEvent::Started { start_offset_us: 0 });
    (driver, timers)
}

/// A judgement in `lane` that reaches nothing.
fn judged(lane: usize, judge: Judge) -> SceneEvent<'static> {
    SceneEvent::Judged(Judgement { lane, judge, fast_us: 0, reached: Reached::default() })
}

/// Every built-in timer that is on, with the moment it went on, to hold two timer tables against
/// each other.
fn running(timers: &TimerState) -> Vec<(TimerId, i64)> {
    (0..=timer_id::MAX.get()).map(TimerId).filter(|timer| timers.is_on(*timer)).map(|timer| (timer, timers.value_us(timer))).collect()
}

/// A frame of the run at `chart_us`, at the tempo of these tests and as written.
fn frame(chart_us: i64) -> PlayingFrame {
    PlayingFrame { chart_us, bpm: TEMPO, play_speed: FULL_PERCENT, gauge_max: false, past_notes: 0 }
}

/// What sending a row's event a second time does to a timer that is already on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Again {
    /// It starts over (`setTimerOn`).
    Restarts,
    /// It keeps the moment it first went on (`switchTimer`).
    Keeps,
}

/// One row of the reference's timer table: what switches a timer on, what it then reads, what the
/// same thing happening again does, and what switches it off.
struct Row {
    timer: TimerId,
    /// Whether the run is started before the row's event.
    running: bool,
    on: SceneEvent<'static>,
    /// How far behind the moment of the event the timer is set.
    behind_us: i64,
    again: Again,
    /// What switches it off, or `None` for a timer nothing switches off.
    off: Option<SceneEvent<'static>>,
}

impl Row {
    const fn at(timer: TimerId, running: bool, on: SceneEvent<'static>, again: Again, off: Option<SceneEvent<'static>>) -> Row {
        Row { timer, running, on, behind_us: 0, again, off }
    }
}

const FULL_GAUGE: PlayingFrame = PlayingFrame { chart_us: 0, bpm: TEMPO, play_speed: FULL_PERCENT, gauge_max: true, past_notes: 0 };
const PLAIN_FRAME: PlayingFrame = PlayingFrame { gauge_max: false, ..FULL_GAUGE };
const NOTHING_REACHED: SceneEvent<'static> = SceneEvent::Judged(Judgement { lane: FIRST_KEY_LANE, judge: Judge::Poor, fast_us: 0, reached: NO_REACH });
const NO_REACH: Reached = Reached { full_combo: false, rank_a: false, rank_aa: false, rank_aaa: false, best: false, target: false };

const fn reached_event(reached: Reached) -> SceneEvent<'static> {
    SceneEvent::Judged(Judgement { lane: FIRST_KEY_LANE, judge: Judge::Poor, fast_us: 0, reached })
}

/// The reference's timer table for a seven-key run drawn by a skin with one judge region, row by
/// row.
fn timer_table() -> Vec<Row> {
    let perfect_in = |lane: usize| SceneEvent::Judged(Judgement { lane, judge: Judge::PerfectGreat, fast_us: 0, reached: NO_REACH });
    vec![
        Row::at(timer_id::STARTINPUT, false, SceneEvent::InputOpen, Again::Keeps, None),
        Row::at(timer_id::FADEOUT, true, SceneEvent::Fadeout, Again::Keeps, None),
        Row::at(timer_id::FAILED, true, SceneEvent::Failed, Again::Restarts, None),
        Row::at(timer_id::READY, false, SceneEvent::Ready, Again::Restarts, None),
        Row {
            timer: timer_id::PLAY,
            running: false,
            on: SceneEvent::Started { start_offset_us: START_OFFSET_US },
            behind_us: START_OFFSET_US,
            again: Again::Restarts,
            off: None,
        },
        Row {
            timer: timer_id::RHYTHM,
            running: false,
            on: SceneEvent::Started { start_offset_us: START_OFFSET_US },
            behind_us: START_OFFSET_US,
            again: Again::Restarts,
            off: None,
        },
        Row::at(timer_id::GAUGE_MAX_1P, true, SceneEvent::Playing(FULL_GAUGE), Again::Keeps, Some(SceneEvent::Playing(PLAIN_FRAME))),
        Row::at(timer_id::FULLCOMBO_1P, true, reached_event(Reached { full_combo: true, ..NO_REACH }), Again::Keeps, Some(NOTHING_REACHED)),
        Row {
            timer: CHART_PREVIEW,
            running: false,
            on: SceneEvent::Preview { pressed: true },
            behind_us: FIRST_NOTE_US - CHART_PREVIEW_LEAD_US,
            again: Again::Keeps,
            off: Some(SceneEvent::Preview { pressed: false }),
        },
        Row::at(timer_id::ENDOFNOTE_1P, true, SceneEvent::LastNotePassed, Again::Keeps, None),
        Row::at(timer_id::SCORE_A, true, reached_event(Reached { rank_a: true, ..NO_REACH }), Again::Keeps, Some(NOTHING_REACHED)),
        Row::at(timer_id::SCORE_AA, true, reached_event(Reached { rank_aa: true, ..NO_REACH }), Again::Keeps, Some(NOTHING_REACHED)),
        Row::at(timer_id::SCORE_AAA, true, reached_event(Reached { rank_aaa: true, ..NO_REACH }), Again::Keeps, Some(NOTHING_REACHED)),
        Row::at(timer_id::SCORE_BEST, true, reached_event(Reached { best: true, ..NO_REACH }), Again::Keeps, Some(NOTHING_REACHED)),
        Row::at(timer_id::SCORE_TARGET, true, reached_event(Reached { target: true, ..NO_REACH }), Again::Keeps, Some(NOTHING_REACHED)),
        Row::at(timer_id::MUSIC_END, true, SceneEvent::MusicEnded, Again::Restarts, None),
        Row::at(timer_id::PM_CHARA_1P_NEUTRAL, false, SceneEvent::Standby, Again::Keeps, Some(SceneEvent::MusicEnded)),
        Row::at(timer_id::PM_CHARA_2P_NEUTRAL, false, SceneEvent::Standby, Again::Keeps, Some(SceneEvent::MusicEnded)),
        Row::at(timer_id::PM_CHARA_DANCE, true, SceneEvent::Playing(PLAIN_FRAME), Again::Keeps, Some(SceneEvent::MusicEnded)),
        Row::at(timer_id::JUDGE_1P, true, perfect_in(FIRST_KEY_LANE), Again::Restarts, None),
        Row::at(timer_id::COMBO_1P, true, perfect_in(FIRST_KEY_LANE), Again::Restarts, None),
        Row::at(timer_id::BOMB_1P_KEY1, true, perfect_in(FIRST_KEY_LANE), Again::Restarts, None),
        Row::at(timer_id::BOMB_1P_SCRATCH, true, perfect_in(SCRATCH_LANE), Again::Restarts, None),
        Row::at(
            timer_id::HOLD_1P_KEY1,
            true,
            SceneEvent::LongNote { lane: FIRST_KEY_LANE, held: true },
            Again::Keeps,
            Some(SceneEvent::LongNote { lane: FIRST_KEY_LANE, held: false }),
        ),
        Row::at(
            timer_id::HOLD_1P_SCRATCH,
            true,
            SceneEvent::LongNote { lane: SCRATCH_LANE, held: true },
            Again::Keeps,
            Some(SceneEvent::LongNote { lane: SCRATCH_LANE, held: false }),
        ),
        Row::at(timer_id::KEYON_1P_KEY1, true, SceneEvent::Pressed { lane: FIRST_KEY_LANE }, Again::Keeps, Some(SceneEvent::Input(&ALL_UP))),
        Row::at(timer_id::KEYON_1P_SCRATCH, true, SceneEvent::Pressed { lane: SCRATCH_LANE }, Again::Restarts, Some(SceneEvent::Input(&ALL_UP))),
        Row::at(
            timer_id::HCN_ACTIVE_1P_KEY1,
            true,
            SceneEvent::Hcn { lane: FIRST_KEY_LANE, pass: HcnPass::Active },
            Again::Keeps,
            Some(SceneEvent::Hcn { lane: FIRST_KEY_LANE, pass: HcnPass::Damage }),
        ),
        Row::at(
            timer_id::HCN_DAMAGE_1P_KEY1,
            true,
            SceneEvent::Hcn { lane: FIRST_KEY_LANE, pass: HcnPass::Damage },
            Again::Keeps,
            Some(SceneEvent::Hcn { lane: FIRST_KEY_LANE, pass: HcnPass::None }),
        ),
    ]
}

/// Everything the reference's play screen can do to its timers short of going back to practice,
/// which is what a timer nothing switches off has to survive.
fn everything_else() -> Vec<SceneEvent<'static>> {
    vec![
        SceneEvent::InputOpen,
        SceneEvent::Standby,
        SceneEvent::Preview { pressed: false },
        SceneEvent::Playing(PLAIN_FRAME),
        SceneEvent::LastNotePassed,
        SceneEvent::Input(&ALL_UP),
        SceneEvent::LongNote { lane: FIRST_KEY_LANE, held: false },
        SceneEvent::Hcn { lane: FIRST_KEY_LANE, pass: HcnPass::None },
        NOTHING_REACHED,
        SceneEvent::Failed,
        SceneEvent::MusicEnded,
        SceneEvent::Fadeout,
        SceneEvent::JudgeStopped,
        SceneEvent::Input(&ALL_UP),
    ]
}

/// A seven-key run for one row of the table: started or not, as the row asks.
fn table_run(row: &Row) -> (PlayTimerDriver, TimerState) {
    let table_setup = PlaySetup { first_note_us: FIRST_NOTE_US, ..setup(Mode::BEAT_7K, 1) };
    if row.running { started(table_setup) } else { (PlayTimerDriver::new(table_setup), TimerState::new()) }
}

#[test]
fn every_row_of_the_timer_table_goes_on_at_its_event_and_reads_what_the_reference_sets() {
    for row in timer_table() {
        let (mut driver, mut timers) = table_run(&row);
        assert!(!timers.is_on(row.timer), "{:?} was on before {:?}", row.timer, row.on);

        driver.apply(&mut timers, FIRST_US, row.on);

        assert_eq!(timers.value_us(row.timer), FIRST_US - row.behind_us, "{:?} after {:?}", row.timer, row.on);
    }
}

#[test]
fn every_row_restarts_or_keeps_its_timer_when_its_event_comes_again_as_the_reference_does() {
    for row in timer_table() {
        let (mut driver, mut timers) = table_run(&row);
        driver.apply(&mut timers, FIRST_US, row.on);

        driver.apply(&mut timers, NEXT_US, row.on);

        let since = if row.again == Again::Restarts { NEXT_US } else { FIRST_US };
        assert_eq!(timers.value_us(row.timer), since - row.behind_us, "{:?} after {:?} twice: {:?}", row.timer, row.on, row.again);
    }
}

#[test]
fn every_row_goes_off_at_the_event_the_reference_switches_it_off_at_and_at_nothing_else() {
    for row in timer_table() {
        let (mut driver, mut timers) = table_run(&row);
        if !row.running {
            driver.apply(&mut timers, START_US, SceneEvent::Started { start_offset_us: 0 });
        }
        driver.apply(&mut timers, FIRST_US, row.on);

        match row.off {
            Some(off) => {
                driver.apply(&mut timers, NEXT_US, off);
                assert_eq!(timers.value_us(row.timer), TIMER_OFF, "{:?} after {off:?}", row.timer);
            }
            None => {
                for other in everything_else() {
                    driver.apply(&mut timers, NEXT_US, other);
                    assert!(timers.is_on(row.timer), "{:?} went off at {other:?}", row.timer);
                }
            }
        }
    }
}

#[test]
fn the_timers_the_reference_declares_and_never_sets_stay_off_through_a_whole_run() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_14K, 2));
    let lanes = driver.lanes().len();
    let everything_reached = Reached { full_combo: true, rank_a: true, rank_aa: true, rank_aaa: true, best: true, target: true };

    for lane in 0..lanes {
        let down = keys(lanes, &[(lane, HELD)]);
        driver.apply(&mut timers, FIRST_US, SceneEvent::Input(&down));
        driver.apply(&mut timers, FIRST_US, SceneEvent::Pressed { lane });
        driver.apply(&mut timers, FIRST_US, SceneEvent::Judged(Judgement { lane, judge: Judge::PerfectGreat, fast_us: 0, reached: everything_reached }));
        driver.apply(&mut timers, FIRST_US, SceneEvent::LongNote { lane, held: true });
        driver.apply(&mut timers, FIRST_US, SceneEvent::Hcn { lane, pass: HcnPass::Active });
    }
    driver.apply(&mut timers, FIRST_US, SceneEvent::Playing(FULL_GAUGE));
    driver.apply(&mut timers, FIRST_US, SceneEvent::LastNotePassed);
    driver.apply(&mut timers, NEXT_US, SceneEvent::MusicEnded);
    driver.apply(&mut timers, LAST_US, SceneEvent::Fadeout);

    let never_set = [timer_id::GAUGE_INCLEASE_1P, timer_id::GAUGE_INCLEASE_2P, timer_id::GAUGE_MAX_2P, timer_id::FULLCOMBO_2P, timer_id::ENDOFNOTE_2P];
    for timer in never_set {
        assert!(!timers.is_on(timer), "{timer:?} is declared and never set");
    }
    assert!(timers.is_on(timer_id::GAUGE_MAX_1P) && timers.is_on(timer_id::FULLCOMBO_1P), "the run did reach the first side's");
}

#[test]
fn ready_stays_on_from_the_moment_the_run_is_ready_to_the_end_of_the_screen() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 1));

    assert_eq!(timers.value_us(timer_id::READY), READY_US, "starting the run leaves the ready timer running");
    for (step, event) in everything_else().into_iter().enumerate() {
        driver.apply(&mut timers, FIRST_US + at_ms(step as i64), event);
    }

    assert_eq!(timers.value_us(timer_id::READY), READY_US);
}

#[test]
fn the_play_timer_reads_the_chart_time_of_every_frame_whatever_speed_the_chart_runs_at() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 0));
    let mut timers = TimerState::new();
    driver.apply(&mut timers, START_US, SceneEvent::Started { start_offset_us: START_OFFSET_US });
    assert_eq!(timers.value_us(timer_id::PLAY), START_US - START_OFFSET_US, "a run that starts into its chart has already run that far");

    let half_speed = FULL_PERCENT / 2;
    let chart_us = START_OFFSET_US + SECOND_US / 2;
    driver.apply(&mut timers, START_US + SECOND_US, SceneEvent::Playing(PlayingFrame { chart_us, play_speed: half_speed, ..PLAIN_FRAME }));

    let moved_as_the_reference_moves_it = START_US - START_OFFSET_US + SECOND_US * i64::from(FULL_PERCENT - half_speed) / i64::from(FULL_PERCENT);
    assert_eq!(timers.value_us(timer_id::PLAY), moved_as_the_reference_moves_it, "TIMER_PLAY += deltatime * (100 - playspeed) / 100");
    assert_eq!(START_US + SECOND_US - timers.value_us(timer_id::PLAY), chart_us, "which is what keeps the timer's elapsed time the chart time");
}

#[test]
fn the_rhythm_timer_reads_a_thousand_milliseconds_a_beat_and_starts_over_at_every_bar_line() {
    let (mut driver, mut timers) = started(PlaySetup { sections_us: vec![0, BAR_US], ..setup(Mode::BEAT_7K, 0) });

    driver.apply(&mut timers, FIRST_US, SceneEvent::Playing(frame(0)));
    assert_eq!(timers.value_us(timer_id::RHYTHM), FIRST_US, "the bar line at the top of the chart starts it");

    let a_beat_on = FIRST_US + BEAT_US;
    driver.apply(&mut timers, a_beat_on, SceneEvent::Playing(frame(BEAT_US)));
    assert_eq!(a_beat_on - timers.value_us(timer_id::RHYTHM), RHYTHM_BEAT_US, "half a second at 120 is one beat");

    let three_beats_on = FIRST_US + 3 * BEAT_US;
    driver.apply(&mut timers, three_beats_on, SceneEvent::Playing(frame(3 * BEAT_US)));
    assert_eq!(three_beats_on - timers.value_us(timer_id::RHYTHM), 3 * RHYTHM_BEAT_US);

    let a_bar_on = FIRST_US + BAR_US;
    driver.apply(&mut timers, a_bar_on, SceneEvent::Playing(frame(BAR_US)));
    assert_eq!(timers.value_us(timer_id::RHYTHM), a_bar_on, "the next bar line starts it over");
}

#[test]
fn a_beat_is_a_thousand_milliseconds_at_any_tempo_and_any_play_speed() {
    let double_time = 2 * FULL_PERCENT;
    let cases = [
        (TEMPO, FULL_PERCENT, BEAT_US),
        (150.0, FULL_PERCENT, 400_000),
        (60.0, FULL_PERCENT, SECOND_US),
        (TEMPO, FULL_PERCENT / 2, SECOND_US),
        (TEMPO, double_time, BEAT_US / 2),
    ];
    for (bpm, play_speed, beat_on_the_scene_clock_us) in cases {
        let (mut driver, mut timers) = started(PlaySetup { sections_us: vec![0], ..setup(Mode::BEAT_7K, 0) });
        driver.apply(&mut timers, FIRST_US, SceneEvent::Playing(PlayingFrame { bpm, play_speed, ..PLAIN_FRAME }));

        let a_beat_on = FIRST_US + beat_on_the_scene_clock_us;
        driver.apply(&mut timers, a_beat_on, SceneEvent::Playing(PlayingFrame { bpm, play_speed, ..PLAIN_FRAME }));

        assert_eq!(a_beat_on - timers.value_us(timer_id::RHYTHM), RHYTHM_BEAT_US, "{bpm} at {play_speed}%");
    }
}

#[test]
fn the_rhythm_timer_stands_still_while_the_chart_does() {
    let (mut driver, mut timers) = started(PlaySetup { sections_us: vec![0], ..setup(Mode::BEAT_7K, 0) });
    driver.apply(&mut timers, FIRST_US, SceneEvent::Playing(frame(0)));
    driver.apply(&mut timers, FIRST_US + BEAT_US, SceneEvent::Playing(frame(BEAT_US)));

    let stopped_at = FIRST_US + BEAT_US + SECOND_US;
    driver.apply(&mut timers, stopped_at, SceneEvent::Playing(PlayingFrame { chart_us: BEAT_US, play_speed: 0, ..PLAIN_FRAME }));

    assert_eq!(stopped_at - timers.value_us(timer_id::RHYTHM), RHYTHM_BEAT_US, "a second of standing still moved the timer on by a second");
}

#[test]
fn the_rhythm_timer_counts_from_nothing_until_the_first_bar_line_as_the_reference_does() {
    let late_bar_us = BAR_US;
    let (mut driver, mut timers) = started(PlaySetup { sections_us: vec![late_bar_us], ..setup(Mode::BEAT_7K, 0) });
    assert_eq!(timers.value_us(timer_id::RHYTHM), START_US, "starting the run sets the timer");

    driver.apply(&mut timers, START_US + BEAT_US, SceneEvent::Playing(frame(BEAT_US)));

    assert_eq!(timers.value_us(timer_id::RHYTHM), -BEAT_US, "the first frame overwrites it with the count, which began at nought");
}

#[test]
fn a_run_that_starts_past_several_bar_lines_passes_one_of_them_a_frame() {
    let bars = vec![0, BAR_US, 2 * BAR_US];
    let mut driver = PlayTimerDriver::new(PlaySetup { sections_us: bars.clone(), ..setup(Mode::BEAT_7K, 0) });
    let mut timers = TimerState::new();
    let into_the_chart_us = 2 * BAR_US + BEAT_US;
    driver.apply(&mut timers, START_US, SceneEvent::Started { start_offset_us: into_the_chart_us });

    for passed in 0..bars.len() as i64 {
        let now_us = FIRST_US + at_ms(passed);
        driver.apply(&mut timers, now_us, SceneEvent::Playing(frame(into_the_chart_us)));
        assert_eq!(timers.value_us(timer_id::RHYTHM), now_us, "bar line {passed} started the timer over on its own frame");
    }
    let caught_up_us = FIRST_US + at_ms(bars.len() as i64);
    driver.apply(&mut timers, caught_up_us, SceneEvent::Playing(frame(into_the_chart_us)));
    assert_ne!(timers.value_us(timer_id::RHYTHM), caught_up_us, "with every bar line behind the run the timer runs on");
}

#[test]
fn a_bar_line_is_weighed_by_a_hundred_over_the_practice_frequency_cut_to_a_whole_number() {
    let half = FULL_PERCENT / 2;
    let three_quarters = 75;
    let double = 2 * FULL_PERCENT;
    let cases = [(FULL_PERCENT, Some(BAR_US)), (half, Some(2 * BAR_US)), (three_quarters, Some(BAR_US)), (double, Some(0)), (0, None)];
    for (practice_freq, reached_at_us) in cases {
        let (mut driver, mut timers) = started(PlaySetup { sections_us: vec![BAR_US], practice_freq, ..setup(Mode::BEAT_7K, 0) });
        let passes = |driver: &mut PlayTimerDriver, timers: &mut TimerState, now_us: i64, chart_us: i64| {
            driver.apply(timers, now_us, SceneEvent::Playing(frame(chart_us)));
            timers.value_us(timer_id::RHYTHM) == now_us
        };
        match reached_at_us {
            Some(reached_us) => {
                let just_short = reached_us - 1;
                assert!(reached_us == 0 || !passes(&mut driver, &mut timers, FIRST_US, just_short), "{practice_freq}%: not yet at {just_short}");
                assert!(passes(&mut driver, &mut timers, NEXT_US, reached_us), "{practice_freq}%: reached at {reached_us}");
            }
            None => assert!(!passes(&mut driver, &mut timers, FIRST_US, i64::MAX / 2), "a frequency of nought reaches no bar line"),
        }
    }
}

#[test]
fn the_lanes_of_every_mode_are_numbered_by_the_reference_table() {
    let key = |player: usize, key: usize| LaneSlot { player, key, scratch: None };
    let turntable = |player: usize, index: usize| LaneSlot { player, key: 0, scratch: Some(index) };
    let side = |player: usize, keys: usize| (1..=keys).map(move |number| key(player, number));

    let five: Vec<LaneSlot> = side(0, 5).chain([turntable(0, 0)]).collect();
    let seven: Vec<LaneSlot> = side(0, 7).chain([turntable(0, 0)]).collect();
    let ten: Vec<LaneSlot> = side(0, 5).chain([turntable(0, 0)]).chain(side(1, 5)).chain([turntable(1, 1)]).collect();
    let fourteen: Vec<LaneSlot> = side(0, 7).chain([turntable(0, 0)]).chain(side(1, 7)).chain([turntable(1, 1)]).collect();
    let nine: Vec<LaneSlot> = side(0, 9).collect();
    let twenty_four: Vec<LaneSlot> = side(0, 26).collect();

    assert_eq!(lane_slots(Mode::BEAT_5K), five);
    assert_eq!(lane_slots(Mode::BEAT_7K), seven);
    assert_eq!(lane_slots(Mode::BEAT_10K), ten);
    assert_eq!(lane_slots(Mode::BEAT_14K), fourteen);
    assert_eq!(lane_slots(Mode::POPN_9K), nine);
    assert_eq!(lane_slots(Mode::KEYBOARD_24K), twenty_four, "the reference numbers all twenty-six lanes as keys");
}

#[test]
fn a_lane_timer_id_follows_the_reference_formula_in_both_bands_and_on_both_sides() {
    let cases = [
        (LaneTimer::Bomb, [50, 59, 60, 69], [1010, 1099, 1110, 1199]),
        (LaneTimer::Hold, [70, 79, 80, 89], [1210, 1299, 1310, 1399]),
        (LaneTimer::KeyOn, [100, 109, 110, 119], [1410, 1499, 1510, 1599]),
        (LaneTimer::KeyOff, [120, 129, 130, 139], [1610, 1699, 1710, 1799]),
        (LaneTimer::HcnActive, [250, 259, 260, 269], [1810, 1899, 1910, 1999]),
        (LaneTimer::HcnDamage, [270, 279, 280, 289], [2010, 2099, 2110, 2199]),
    ];
    for (kind, low, high) in cases {
        let ids = |keys: [usize; 2]| [kind.id(0, keys[0]), kind.id(0, keys[1]), kind.id(1, keys[0]), kind.id(1, keys[1])];
        assert_eq!(ids([0, 9]), low.map(|id| Some(TimerId(id))), "{kind:?}: the turntable and the ninth key of each side");
        assert_eq!(ids([10, 99]), high.map(|id| Some(TimerId(id))), "{kind:?}: the tenth and the ninety-ninth key of each side");
        assert_eq!(kind.id(0, 100), None, "{kind:?}: there is no hundredth key");
        assert_eq!(kind.id(2, 1), None, "{kind:?}: there is no third side");
    }
}

/// The bomb timer each lane of each mode fires, in the chart's lane order.
fn bomb_ids_by_mode() -> Vec<(Mode, Vec<i32>)> {
    vec![
        (Mode::BEAT_5K, vec![51, 52, 53, 54, 55, 50]),
        (Mode::BEAT_7K, vec![51, 52, 53, 54, 55, 56, 57, 50]),
        (Mode::POPN_9K, vec![51, 52, 53, 54, 55, 56, 57, 58, 59]),
        (Mode::BEAT_10K, vec![51, 52, 53, 54, 55, 50, 61, 62, 63, 64, 65, 60]),
        (Mode::BEAT_14K, vec![51, 52, 53, 54, 55, 56, 57, 50, 61, 62, 63, 64, 65, 66, 67, 60]),
        (Mode::KEYBOARD_24K, (51..=59).chain(1010..=1026).collect()),
    ]
}

/// How far a lane's hold, key on and key off timers are from its bomb timer, in the low band.
const HOLD_FROM_BOMB: i32 = 20;
const KEY_ON_FROM_BOMB: i32 = 50;
const KEY_OFF_FROM_BOMB: i32 = 70;

/// The id of the first key numbered in the high band, and how far apart the bands of the kinds are
/// there.
const HIGH_BAND_FIRST_ID: i32 = 1010;
const HIGH_BAND_KIND_STRIDE: i32 = 200;

/// The id a lane's other timers have, from its bomb id: a fixed step away in the low band, and so
/// many whole bands away in the high one.
fn from_bomb(bomb: i32, low_step: i32, high_bands: i32) -> i32 {
    if bomb < HIGH_BAND_FIRST_ID { bomb + low_step } else { bomb + high_bands * HIGH_BAND_KIND_STRIDE }
}

#[test]
fn a_judgement_in_each_lane_of_each_mode_fires_the_bomb_the_reference_numbers_for_it() {
    for (mode, expected) in bomb_ids_by_mode() {
        for (lane, bomb) in expected.iter().enumerate() {
            let (mut driver, mut timers) = started(setup(mode, 0));

            driver.apply(&mut timers, FIRST_US, judged(lane, Judge::PerfectGreat));

            let fired: Vec<i32> = expected.iter().copied().filter(|id| timers.is_on(TimerId(*id))).collect();
            assert_eq!(fired, [*bomb], "{} lane {lane}", mode.name);
        }
    }
}

#[test]
fn a_held_long_note_in_each_lane_of_each_mode_runs_the_hold_timer_the_reference_numbers_for_it() {
    for (mode, bombs) in bomb_ids_by_mode() {
        for (lane, bomb) in bombs.iter().enumerate() {
            let (mut driver, mut timers) = started(setup(mode, 0));

            driver.apply(&mut timers, FIRST_US, SceneEvent::LongNote { lane, held: true });

            assert_eq!(timers.value_us(TimerId(from_bomb(*bomb, HOLD_FROM_BOMB, 1))), FIRST_US, "{} lane {lane}", mode.name);
        }
    }
}

#[test]
fn a_key_going_down_and_coming_up_in_each_lane_of_each_mode_switches_the_pair_the_reference_numbers_for_it() {
    for (mode, bombs) in bomb_ids_by_mode() {
        for (lane, bomb) in bombs.iter().enumerate() {
            let mut driver = PlayTimerDriver::new(setup(mode, 0));
            let mut timers = TimerState::new();
            let (key_on, key_off) = (TimerId(from_bomb(*bomb, KEY_ON_FROM_BOMB, 2)), TimerId(from_bomb(*bomb, KEY_OFF_FROM_BOMB, 3)));

            driver.apply(&mut timers, FIRST_US, SceneEvent::Input(&keys(bombs.len(), &[(lane, HELD)])));
            assert_eq!((timers.value_us(key_on), timers.value_us(key_off)), (FIRST_US, TIMER_OFF), "{} lane {lane} down", mode.name);

            driver.apply(&mut timers, NEXT_US, SceneEvent::Input(&keys(bombs.len(), &[])));
            assert_eq!((timers.value_us(key_on), timers.value_us(key_off)), (TIMER_OFF, NEXT_US), "{} lane {lane} up", mode.name);
        }
    }
}

#[test]
fn the_second_side_of_fourteen_keys_switches_the_second_sides_timers() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_14K, 2));

    driver.apply(&mut timers, FIRST_US, SceneEvent::Pressed { lane: SECOND_SIDE_FIRST_KEY_LANE });
    driver.apply(&mut timers, FIRST_US, SceneEvent::Pressed { lane: SECOND_SIDE_SCRATCH_LANE });
    driver.apply(&mut timers, FIRST_US, judged(SECOND_SIDE_FIRST_KEY_LANE, Judge::Great));
    driver.apply(&mut timers, FIRST_US, SceneEvent::LongNote { lane: SECOND_SIDE_SCRATCH_LANE, held: true });
    driver.apply(&mut timers, FIRST_US, SceneEvent::Hcn { lane: SECOND_SIDE_FIRST_KEY_LANE, pass: HcnPass::Active });
    driver.apply(&mut timers, FIRST_US, SceneEvent::Hcn { lane: SECOND_SIDE_SCRATCH_LANE, pass: HcnPass::Damage });

    let second_side = [
        timer_id::KEYON_2P_KEY1,
        timer_id::KEYON_2P_SCRATCH,
        timer_id::BOMB_2P_KEY1,
        timer_id::HOLD_2P_SCRATCH,
        TimerId(261),
        TimerId(280),
        timer_id::JUDGE_2P,
        timer_id::COMBO_2P,
    ];
    for timer in second_side {
        assert_eq!(timers.value_us(timer), FIRST_US, "{timer:?}");
    }
    let first_side =
        [timer_id::KEYON_1P_KEY1, timer_id::KEYON_1P_SCRATCH, timer_id::BOMB_1P_KEY1, timer_id::HOLD_1P_SCRATCH, timer_id::JUDGE_1P, timer_id::COMBO_1P];
    for timer in first_side {
        assert!(!timers.is_on(timer), "{timer:?} is the first side's");
    }
}

#[test]
fn a_bomb_starts_over_at_every_judgement_good_enough_for_it_and_is_never_switched_off() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 1));

    driver.apply(&mut timers, FIRST_US, judged(FIRST_KEY_LANE, Judge::PerfectGreat));
    assert_eq!(timers.value_us(timer_id::BOMB_1P_KEY1), FIRST_US);

    driver.apply(&mut timers, NEXT_US, judged(FIRST_KEY_LANE, Judge::Great));
    assert_eq!(timers.value_us(timer_id::BOMB_1P_KEY1), NEXT_US, "a second hit in the same lane starts the bomb over");

    for worse in [Judge::Good, Judge::Bad, Judge::Poor, Judge::Miss] {
        driver.apply(&mut timers, LAST_US, judged(FIRST_KEY_LANE, worse));
        assert_eq!(timers.value_us(timer_id::BOMB_1P_KEY1), NEXT_US, "{worse:?} neither fires the bomb nor puts it out");
    }
    for event in everything_else() {
        driver.apply(&mut timers, LAST_US, event);
    }
    assert_eq!(timers.value_us(timer_id::BOMB_1P_KEY1), NEXT_US, "nothing the screen does puts a bomb out");
}

#[test]
fn the_skin_says_how_good_a_judgement_has_to_be_for_a_bomb() {
    let ladder = [Judge::PerfectGreat, Judge::Great, Judge::Good, Judge::Bad, Judge::Poor, Judge::Miss];
    for judge_timer in -1..ladder.len() as i32 {
        for (code, judge) in ladder.into_iter().enumerate() {
            let (mut driver, mut timers) = started(PlaySetup { judge_timer, ..setup(Mode::BEAT_7K, 1) });

            driver.apply(&mut timers, FIRST_US, judged(FIRST_KEY_LANE, judge));

            assert_eq!(timers.is_on(timer_id::BOMB_1P_KEY1), code as i32 <= judge_timer, "{judge:?} with judgetimer {judge_timer}");
        }
    }
}

#[test]
fn a_judgement_restarts_the_judge_and_combo_timers_of_its_region_whatever_it_was() {
    for judge in [Judge::PerfectGreat, Judge::Bad, Judge::Poor, Judge::Miss] {
        let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 1));
        driver.apply(&mut timers, FIRST_US, judged(SCRATCH_LANE, judge));
        driver.apply(&mut timers, NEXT_US, judged(FIRST_KEY_LANE, judge));

        assert_eq!((timers.value_us(timer_id::JUDGE_1P), timers.value_us(timer_id::COMBO_1P)), (NEXT_US, NEXT_US), "{judge:?}");
        assert!(!timers.is_on(timer_id::JUDGE_2P) && !timers.is_on(timer_id::COMBO_2P), "one region takes every lane");
    }
}

#[test]
fn a_skin_with_no_judge_object_has_no_judge_or_combo_timer() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 0));

    driver.apply(&mut timers, FIRST_US, judged(FIRST_KEY_LANE, Judge::PerfectGreat));

    for timer in JUDGE_TIMERS.into_iter().chain(COMBO_TIMERS) {
        assert!(!timers.is_on(timer), "{timer:?}");
    }
    assert!(timers.is_on(timer_id::BOMB_1P_KEY1), "the bomb does not wait for a judge object");
}

#[test]
fn two_regions_split_the_lanes_in_half_and_leave_each_others_combo_timers_alone() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_14K, 2));

    driver.apply(&mut timers, FIRST_US, judged(SCRATCH_LANE, Judge::Great));
    driver.apply(&mut timers, NEXT_US, judged(SECOND_SIDE_FIRST_KEY_LANE, Judge::Great));

    assert_eq!((timers.value_us(timer_id::JUDGE_1P), timers.value_us(timer_id::COMBO_1P)), (FIRST_US, FIRST_US), "the last lane of the first half");
    assert_eq!((timers.value_us(timer_id::JUDGE_2P), timers.value_us(timer_id::COMBO_2P)), (NEXT_US, NEXT_US), "the first of the second");
    assert!(!timers.is_on(timer_id::JUDGE_3P) && !timers.is_on(timer_id::COMBO_3P));
}

#[test]
fn three_regions_split_the_lanes_in_thirds_and_keep_one_combo_timer_at_a_time() {
    let (mut driver, mut timers) = started(setup(Mode::POPN_9K, 3));
    let regions = [(0, timer_id::JUDGE_1P, timer_id::COMBO_1P), (4, timer_id::JUDGE_2P, timer_id::COMBO_2P), (8, timer_id::JUDGE_3P, timer_id::COMBO_3P)];

    for (step, (lane, judge_timer, combo_timer)) in regions.into_iter().enumerate() {
        let now_us = FIRST_US + at_ms(step as i64);
        driver.apply(&mut timers, now_us, judged(lane, Judge::Great));

        assert_eq!((timers.value_us(judge_timer), timers.value_us(combo_timer)), (now_us, now_us), "lane {lane}");
        let combos_on: Vec<TimerId> = COMBO_TIMERS.into_iter().filter(|combo| timers.is_on(*combo)).collect();
        assert_eq!(combos_on, [combo_timer], "the other regions lose their combo timers");
    }
    for judge_timer in JUDGE_TIMERS {
        assert!(timers.is_on(judge_timer), "{judge_timer:?}: a judge timer is not switched off by another region");
    }
}

#[test]
fn a_region_is_the_lane_over_the_lanes_in_a_region_with_both_divisions_whole() {
    assert_eq!(judge_region(7, 8, 1), Some(0));
    assert_eq!((judge_region(7, 16, 2), judge_region(8, 16, 2)), (Some(0), Some(1)));
    assert_eq!((judge_region(2, 9, 3), judge_region(3, 9, 3), judge_region(8, 9, 3)), (Some(0), Some(1), Some(2)));
    assert_eq!(judge_region(5, 8, 3), Some(2), "eight lanes in three regions are two lanes a region");
    assert_eq!(judge_region(6, 8, 3), None, "which leaves the last two lanes past the third region, where there is no timer");
    assert_eq!(judge_region(0, 8, 0), None, "no judge object, no region");
    assert_eq!(judge_region(0, 2, 3), None, "more regions than lanes has no answer in the reference either");
}

#[test]
fn a_lane_past_the_third_region_fires_its_bomb_and_reaches_what_it_reaches_but_no_judge_timer() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 3));

    driver.apply(
        &mut timers,
        FIRST_US,
        SceneEvent::Judged(Judgement { lane: SCRATCH_LANE, judge: Judge::PerfectGreat, fast_us: 0, reached: Reached { best: true, ..NO_REACH } }),
    );

    assert!(JUDGE_TIMERS.into_iter().chain(COMBO_TIMERS).all(|timer| !timers.is_on(timer)));
    assert!(timers.is_on(timer_id::BOMB_1P_SCRATCH) && timers.is_on(timer_id::SCORE_BEST));
}

#[test]
fn the_judge_regions_of_a_skin_are_one_more_than_the_highest_index_a_destination_places() {
    let judge = |id: &str, index: i32| JudgeDef { id: id.to_owned(), index, ..JudgeDef::default() };
    let placed = |id: &str| Destination { id: id.to_owned(), ..Destination::default() };
    let skin = |judges: Vec<JudgeDef>, destinations: Vec<Destination>| SkinDef { judge: judges, destination: destinations, ..SkinDef::default() };

    assert_eq!(judge_regions(&skin(Vec::new(), vec![placed("judge")])), 0, "a skin with no judge object");
    assert_eq!(judge_regions(&skin(vec![judge("judge", 0)], Vec::new())), 0, "a judge object no destination places is not built");
    assert_eq!(judge_regions(&skin(vec![judge("judge", 0)], vec![placed("judge"), placed("judge")])), 1);
    assert_eq!(judge_regions(&skin(vec![judge("left", 0), judge("right", 1)], vec![placed("right"), placed("left")])), 2);
    assert_eq!(judge_regions(&skin(vec![judge("left", 0), judge("right", 1)], vec![placed("left")])), 1, "only the placed one counts");
    assert_eq!(judge_regions(&skin(vec![judge("third", 2)], vec![placed("third")])), 3, "the index alone decides, not how many there are");
    assert_eq!(judge_regions(&skin(vec![judge("odd", -1)], vec![placed("odd")])), 0, "an index below nought raises nothing");
}

#[test]
fn nothing_the_judge_reports_is_taken_before_it_starts_or_after_it_stops() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 1));
    let mut timers = TimerState::new();
    let reports = [
        SceneEvent::Pressed { lane: FIRST_KEY_LANE },
        SceneEvent::Judged(Judgement { lane: FIRST_KEY_LANE, judge: Judge::PerfectGreat, fast_us: 0, reached: Reached { full_combo: true, ..NO_REACH } }),
        SceneEvent::LongNote { lane: FIRST_KEY_LANE, held: true },
        SceneEvent::Hcn { lane: FIRST_KEY_LANE, pass: HcnPass::Active },
    ];

    for report in reports {
        driver.apply(&mut timers, FIRST_US, report);
    }
    assert!(running(&timers).is_empty(), "the judge has not started");

    driver.apply(&mut timers, START_US, SceneEvent::Started { start_offset_us: 0 });
    driver.apply(&mut timers, FIRST_US, SceneEvent::LongNote { lane: FIRST_KEY_LANE, held: true });
    driver.apply(&mut timers, NEXT_US, SceneEvent::JudgeStopped);
    let stopped = running(&timers);
    for report in reports {
        driver.apply(&mut timers, LAST_US, report);
    }
    driver.apply(&mut timers, LAST_US, SceneEvent::LongNote { lane: FIRST_KEY_LANE, held: false });

    assert_eq!(running(&timers), stopped, "the judge has stopped");
    assert_eq!(timers.value_us(timer_id::HOLD_1P_KEY1), FIRST_US, "a long note held as the judge stopped stays held");
}

#[test]
fn before_the_judge_starts_a_lane_lights_its_beam_from_the_frame_of_keys() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 1));
    let mut timers = TimerState::new();
    let lanes = driver.lanes().len();

    driver.apply(&mut timers, FIRST_US, SceneEvent::Input(&keys(lanes, &[(LAST_KEY_LANE, HELD)])));
    driver.apply(&mut timers, NEXT_US, SceneEvent::Input(&keys(lanes, &[(LAST_KEY_LANE, HELD)])));
    assert_eq!(timers.value_us(timer_id::KEYON_1P_KEY7), FIRST_US, "a lane that stays down keeps the moment it went down");
    assert!(!timers.is_on(timer_id::KEYOFF_1P_KEY7));

    driver.apply(&mut timers, LAST_US, SceneEvent::Input(&keys(lanes, &[])));
    assert_eq!((timers.value_us(timer_id::KEYON_1P_KEY7), timers.value_us(timer_id::KEYOFF_1P_KEY7)), (TIMER_OFF, LAST_US));
}

#[test]
fn while_the_judge_runs_a_lane_lights_its_beam_when_the_judge_takes_the_key_and_not_before() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 1));
    let lanes = driver.lanes().len();
    let down = keys(lanes, &[(FIRST_KEY_LANE, HELD)]);

    driver.apply(&mut timers, FIRST_US, SceneEvent::Input(&down));
    assert!(!timers.is_on(timer_id::KEYON_1P_KEY1), "the frame of keys leaves a lane going down to the judge");

    driver.apply(&mut timers, NEXT_US, SceneEvent::Pressed { lane: FIRST_KEY_LANE });
    driver.apply(&mut timers, LAST_US, SceneEvent::Input(&down));
    assert_eq!(timers.value_us(timer_id::KEYON_1P_KEY1), NEXT_US);

    driver.apply(&mut timers, LAST_US + SECOND_US, SceneEvent::Input(&keys(lanes, &[])));
    assert_eq!(
        (timers.value_us(timer_id::KEYON_1P_KEY1), timers.value_us(timer_id::KEYOFF_1P_KEY1)),
        (TIMER_OFF, LAST_US + SECOND_US),
        "coming up is the frame's"
    );
}

#[test]
fn a_game_playing_by_itself_lights_its_beams_from_the_frame_of_keys_while_the_judge_runs() {
    let (mut driver, mut timers) = started(PlaySetup { autoplay: true, ..setup(Mode::BEAT_7K, 1) });
    let lanes = driver.lanes().len();

    driver.apply(&mut timers, FIRST_US, SceneEvent::Input(&keys(lanes, &[(FIRST_KEY_LANE, HELD)])));

    assert_eq!(timers.value_us(timer_id::KEYON_1P_KEY1), FIRST_US);
}

#[test]
fn a_turntable_relights_its_beam_when_it_changes_hands_and_every_frame_it_is_turned_both_ways() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 1));
    let mut timers = TimerState::new();
    let lanes = driver.lanes().len();
    let turned = |held: LaneKeys| keys(lanes, &[(SCRATCH_LANE, held)]);
    let steps = [(HELD, true), (HELD, false), (TURNED_BACK, true), (TURNED_BACK, false), (TURNED_BOTH_WAYS, true), (TURNED_BOTH_WAYS, true), (HELD, true)];

    let mut lit_us = TIMER_OFF;
    for (step, (held, relit)) in steps.into_iter().enumerate() {
        let now_us = FIRST_US + at_ms(step as i64);
        driver.apply(&mut timers, now_us, SceneEvent::Input(&turned(held)));

        lit_us = if relit { now_us } else { lit_us };
        assert_eq!(timers.value_us(timer_id::KEYON_1P_SCRATCH), lit_us, "frame {step}: {held:?}");
    }
}

#[test]
fn the_judge_relights_a_turntables_beam_at_every_key_it_takes_and_a_keys_only_when_it_is_out() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 1));

    for now_us in [FIRST_US, NEXT_US] {
        driver.apply(&mut timers, now_us, SceneEvent::Pressed { lane: SCRATCH_LANE });
        driver.apply(&mut timers, now_us, SceneEvent::Pressed { lane: FIRST_KEY_LANE });
    }

    assert_eq!(timers.value_us(timer_id::KEYON_1P_SCRATCH), NEXT_US);
    assert_eq!(timers.value_us(timer_id::KEYON_1P_KEY1), FIRST_US);
}

#[test]
fn stopping_the_judge_puts_every_beam_out_and_keeps_them_out() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 1));
    let lanes = driver.lanes().len();
    let down = keys(lanes, &[(FIRST_KEY_LANE, HELD)]);
    driver.apply(&mut timers, FIRST_US, SceneEvent::Pressed { lane: FIRST_KEY_LANE });

    driver.apply(&mut timers, NEXT_US, SceneEvent::JudgeStopped);
    driver.apply(&mut timers, NEXT_US, SceneEvent::Input(&down));
    assert_eq!((timers.value_us(timer_id::KEYON_1P_KEY1), timers.value_us(timer_id::KEYOFF_1P_KEY1)), (TIMER_OFF, NEXT_US), "a lane still held goes out");

    driver.apply(&mut timers, LAST_US, SceneEvent::Input(&down));
    assert!(!timers.is_on(timer_id::KEYON_1P_KEY1), "and stays out, though no judge runs now");
}

#[test]
fn stopping_a_judge_that_never_ran_stops_no_beam() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 1));
    let mut timers = TimerState::new();
    let lanes = driver.lanes().len();

    driver.apply(&mut timers, FIRST_US, SceneEvent::JudgeStopped);
    driver.apply(&mut timers, NEXT_US, SceneEvent::Input(&keys(lanes, &[(FIRST_KEY_LANE, HELD)])));

    assert_eq!(timers.value_us(timer_id::KEYON_1P_KEY1), NEXT_US);
}

#[test]
fn a_hell_charge_note_runs_one_of_its_two_timers_and_keeps_the_moment_it_began() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 1));
    let read = |timers: &TimerState| (timers.value_us(timer_id::HCN_ACTIVE_1P_KEY1), timers.value_us(timer_id::HCN_DAMAGE_1P_KEY1));
    let steps = [
        (HcnPass::Active, (FIRST_US, TIMER_OFF)),
        (HcnPass::Active, (FIRST_US, TIMER_OFF)),
        (HcnPass::Damage, (TIMER_OFF, FIRST_US + at_ms(2))),
        (HcnPass::Damage, (TIMER_OFF, FIRST_US + at_ms(2))),
        (HcnPass::Active, (FIRST_US + at_ms(4), TIMER_OFF)),
        (HcnPass::None, (TIMER_OFF, TIMER_OFF)),
    ];

    for (step, (pass, expected)) in steps.into_iter().enumerate() {
        driver.apply(&mut timers, FIRST_US + at_ms(step as i64), SceneEvent::Hcn { lane: FIRST_KEY_LANE, pass });
        assert_eq!(read(&timers), expected, "step {step}: {pass:?}");
    }
}

#[test]
fn the_chart_preview_runs_from_a_second_before_the_first_note_while_start_or_select_is_held() {
    let mut driver = PlayTimerDriver::new(PlaySetup { first_note_us: FIRST_NOTE_US, ..setup(Mode::BEAT_7K, 1) });
    let mut timers = TimerState::new();

    driver.apply(&mut timers, FIRST_US, SceneEvent::Preview { pressed: false });
    assert!(!timers.is_on(CHART_PREVIEW));

    driver.apply(&mut timers, FIRST_US, SceneEvent::Preview { pressed: true });
    driver.apply(&mut timers, NEXT_US, SceneEvent::Preview { pressed: true });
    assert_eq!(FIRST_US - timers.value_us(CHART_PREVIEW), FIRST_NOTE_US - SECOND_US, "the preview's first frame is a second before the first note");

    driver.apply(&mut timers, LAST_US, SceneEvent::Preview { pressed: false });
    assert!(!timers.is_on(CHART_PREVIEW), "letting go ends it");

    driver.apply(&mut timers, LAST_US, SceneEvent::Preview { pressed: true });
    driver.apply(&mut timers, LAST_US + SECOND_US, SceneEvent::Ready);
    assert!(!timers.is_on(CHART_PREVIEW), "and so does the run going ready");
}

#[test]
fn a_jump_switches_every_lanes_timers_off_and_finds_the_next_bar_line_again() {
    let (mut driver, mut timers) = started(PlaySetup { sections_us: vec![0, BAR_US, 2 * BAR_US], ..setup(Mode::BEAT_14K, 2) });
    let lanes = driver.lanes().len();
    for lane in 0..lanes {
        driver.apply(&mut timers, FIRST_US, SceneEvent::Pressed { lane });
        driver.apply(&mut timers, FIRST_US, judged(lane, Judge::PerfectGreat));
        driver.apply(&mut timers, FIRST_US, SceneEvent::LongNote { lane, held: true });
        driver.apply(&mut timers, FIRST_US, SceneEvent::Hcn { lane, pass: HcnPass::Active });
    }
    for bar in 0..3 {
        driver.apply(&mut timers, FIRST_US + at_ms(bar), SceneEvent::Playing(frame(2 * BAR_US)));
    }

    driver.apply(&mut timers, NEXT_US, SceneEvent::Seeked { chart_us: BEAT_US });

    for slot in driver.lanes() {
        for kind in LaneTimer::ALL {
            assert_eq!(kind.of(*slot).map(|timer| timers.is_on(timer)), Some(false), "{kind:?} of {slot:?}");
        }
    }
    assert!(timers.is_on(timer_id::READY) && timers.is_on(timer_id::PLAY) && timers.is_on(timer_id::JUDGE_1P), "the run's own timers are left");

    driver.apply(&mut timers, LAST_US, SceneEvent::Playing(frame(BAR_US)));
    assert_eq!(timers.value_us(timer_id::RHYTHM), LAST_US, "the second bar line is ahead of the run again");
}

#[test]
fn a_turntable_turns_backwards_left_alone_and_its_two_keys_turn_it_each_way() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 0));
    let mut timers = TimerState::new();
    let lanes = driver.lanes().len();
    let frame_ms = 16;
    let steps = [(LaneKeys::UP, 0.0), (LaneKeys::UP, 357.0), (HELD, 349.0), (TURNED_BACK, 352.0), (TURNED_BOTH_WAYS, 354.0)];

    for (step, (held, degrees)) in steps.into_iter().enumerate() {
        driver.apply(&mut timers, at_ms(frame_ms * step as i64), SceneEvent::Input(&keys(lanes, &[(SCRATCH_LANE, held)])));

        let offsets = driver.offsets(FieldOffsets::default());
        assert_eq!(offsets.scratch_angle, [Some(degrees), None], "frame {step}: {held:?}");
        assert_eq!(offsets.offset(OFFSET_SCRATCHANGLE_1P), Some(SkinOffset { r: degrees, ..SkinOffset::default() }));
        assert_eq!(offsets.offset(OFFSET_SCRATCHANGLE_2P), None, "seven keys have one turntable");
    }
}

#[test]
fn the_second_turntable_turns_forwards_left_alone() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_14K, 0));
    let mut timers = TimerState::new();
    let lanes = driver.lanes().len();
    let frame_ms = 16;
    let steps = [(LaneKeys::UP, 0.0), (LaneKeys::UP, 2.0), (TURNED_BACK, 10.0), (HELD, 8.0)];

    for (step, (held, degrees)) in steps.into_iter().enumerate() {
        driver.apply(&mut timers, at_ms(frame_ms * step as i64), SceneEvent::Input(&keys(lanes, &[(SECOND_SIDE_SCRATCH_LANE, held)])));

        let second = driver.offsets(FieldOffsets::default()).offset(OFFSET_SCRATCHANGLE_2P);
        assert_eq!(second, Some(SkinOffset { r: degrees, ..SkinOffset::default() }), "frame {step}: {held:?}");
    }
    let first = driver.offsets(FieldOffsets::default()).offset(OFFSET_SCRATCHANGLE_1P);
    assert_eq!(first, Some(SkinOffset { r: 352.0, ..SkinOffset::default() }), "the first one went its own way, three frames backwards");
}

#[test]
fn a_turntable_left_alone_goes_once_round_in_two_thousand_one_hundred_and_sixty_milliseconds() {
    let turn_ms = SCRATCH_TURN;
    for (after_ms, degrees) in [(turn_ms / 2, 180.0), (turn_ms, 0.0)] {
        let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 0));
        let mut timers = TimerState::new();
        driver.apply(&mut timers, 0, SceneEvent::Input(&ALL_UP));

        driver.apply(&mut timers, at_ms(after_ms), SceneEvent::Input(&ALL_UP));

        assert_eq!(driver.offsets(FieldOffsets::default()).scratch_angle[0], Some(degrees), "after {after_ms} ms");
    }
}

#[test]
fn a_turntable_turns_by_whole_milliseconds_of_the_scene_clock() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 0));
    let mut timers = TimerState::new();
    let almost_a_milli_us = MICROS_PER_MILLI - 1;
    let angle = |driver: &PlayTimerDriver| driver.offsets(FieldOffsets::default()).scratch_angle[0];

    driver.apply(&mut timers, almost_a_milli_us, SceneEvent::Input(&ALL_UP));
    driver.apply(&mut timers, MICROS_PER_MILLI, SceneEvent::Input(&ALL_UP));
    assert_eq!(angle(&driver), Some(359.0), "the clock went from millisecond nought to millisecond one");

    driver.apply(&mut timers, MICROS_PER_MILLI + almost_a_milli_us, SceneEvent::Input(&ALL_UP));
    assert_eq!(angle(&driver), Some(359.0), "and is still in millisecond one");
}

#[test]
fn a_frame_longer_than_a_whole_turn_leaves_the_angle_below_nought_as_the_reference_does() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 0));
    let mut timers = TimerState::new();
    let hitch_ms = 3_000;
    driver.apply(&mut timers, 0, SceneEvent::Input(&ALL_UP));

    driver.apply(&mut timers, at_ms(hitch_ms), SceneEvent::Input(&ALL_UP));

    assert_eq!(driver.offsets(FieldOffsets::default()).scratch_angle[0], Some(-140.0), "(2160 - 3000) % 2160 is -840, a sixth of which is -140");
}

#[test]
fn stopped_key_beams_do_not_stop_a_turntable() {
    let (mut driver, mut timers) = started(setup(Mode::BEAT_7K, 0));
    let lanes = driver.lanes().len();
    driver.apply(&mut timers, NEXT_US, SceneEvent::JudgeStopped);
    driver.apply(&mut timers, 0, SceneEvent::Input(&ALL_UP));

    driver.apply(&mut timers, at_ms(16), SceneEvent::Input(&keys(lanes, &[(SCRATCH_LANE, TURNED_BACK)])));

    assert_eq!(driver.offsets(FieldOffsets::default()).scratch_angle[0], Some(2.0), "sixteen back and thirty-two forward");
    assert!(!timers.is_on(timer_id::KEYON_1P_SCRATCH), "its beam stays out all the same");
}

#[test]
fn a_mode_with_no_turntable_has_no_angle() {
    let mut driver = PlayTimerDriver::new(setup(Mode::POPN_9K, 0));
    let mut timers = TimerState::new();
    driver.apply(&mut timers, 0, SceneEvent::Input(&ALL_UP));
    driver.apply(&mut timers, at_ms(16), SceneEvent::Input(&ALL_UP));

    let offsets = driver.offsets(FieldOffsets::default());

    assert_eq!(offsets.scratch_angle, [None, None]);
    assert_eq!((offsets.offset(OFFSET_SCRATCHANGLE_1P), offsets.offset(OFFSET_SCRATCHANGLE_2P)), (None, None));
}

#[test]
fn the_field_offsets_are_answered_as_they_were_handed_over_and_not_at_all_when_they_were_not() {
    let driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 0));
    let lift = SkinOffset { y: 72.0, ..SkinOffset::default() };
    let lane_cover = SkinOffset { y: -216.0, ..SkinOffset::default() };
    let hidden_cover = SkinOffset { a: -255.0, ..SkinOffset::default() };

    let nothing = driver.offsets(FieldOffsets::default());
    assert_eq!((nothing.offset(OFFSET_LIFT), nothing.offset(OFFSET_LANECOVER), nothing.offset(OFFSET_HIDDEN_COVER)), (None, None, None));

    let handed = driver.offsets(FieldOffsets { lift: Some(lift), lane_cover: Some(lane_cover), hidden_cover: Some(hidden_cover) });
    assert_eq!(
        (handed.offset(OFFSET_LIFT), handed.offset(OFFSET_LANECOVER), handed.offset(OFFSET_HIDDEN_COVER)),
        (Some(lift), Some(lane_cover), Some(hidden_cover))
    );
    assert_eq!(handed.offset(OFFSET_HIDDEN_COVER + 1), None, "an offset past the five is not the game's");
    assert_eq!(handed.offset(0), None);
}

/// The player's own nudge for every offset, which the game's offsets are asked before.
struct Nudges;

/// What [`Nudges`] moves everything by.
const NUDGE: SkinOffset = SkinOffset { x: 3.0, y: 0.0, w: 0.0, h: 0.0, r: 0.0, a: 0.0 };

impl OffsetSource for Nudges {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        Some(NUDGE)
    }
}

#[test]
fn the_host_answers_the_games_play_offsets_first_and_leaves_the_rest_to_whoever_answered_before() {
    let mut driver = PlayTimerDriver::new(setup(Mode::BEAT_7K, 0));
    let mut timers = TimerState::new();
    driver.apply(&mut timers, 0, SceneEvent::Input(&ALL_UP));
    driver.apply(&mut timers, at_ms(16), SceneEvent::Input(&ALL_UP));
    let lift = SkinOffset { y: 72.0, ..SkinOffset::default() };
    let offsets = driver.offsets(FieldOffsets { lift: Some(lift), ..FieldOffsets::default() });

    let mut host = ScreenHost::new(FIRST_US, &timers);
    host.offsets = Some(&Nudges);
    assert_eq!(host.offset(OFFSET_SCRATCHANGLE_1P), Some(NUDGE), "without the play offsets the host answers as it did");

    host.play_offsets = Some(&offsets);
    assert_eq!(host.offset(OFFSET_SCRATCHANGLE_1P), Some(SkinOffset { r: 357.0, ..SkinOffset::default() }));
    assert_eq!(host.offset(OFFSET_LIFT), Some(lift));
    assert_eq!(host.offset(OFFSET_SCRATCHANGLE_2P), Some(NUDGE), "a turntable the mode does not have");
    assert_eq!(host.offset(OFFSET_LANECOVER), Some(NUDGE), "an offset the play screen has not worked out");
}

/// A sheet of a ten-note chart with `perfects` PGREATs and nothing else.
fn sheet(perfects: u32) -> ScoreSheet {
    ScoreSheet { early: [perfects, 0, 0, 0, 0, 0], late: [0; 6], notes: 10, max_combo: perfects, min_bp: 0, clear: 0, family: PointFamily::Beat7 }
}

#[test]
fn what_a_run_has_reached_is_read_off_its_standing_the_way_the_reference_reads_it() {
    let pace = TargetPace { best_score: 12, rival_score: 16, total_notes: 10 };
    let cases = [
        (0, Reached { ..NO_REACH }),
        (6, Reached { best: true, ..NO_REACH }),
        (7, Reached { rank_a: true, best: true, ..NO_REACH }),
        (8, Reached { rank_a: true, rank_aa: true, best: true, target: true, ..NO_REACH }),
        (9, Reached { rank_a: true, rank_aa: true, rank_aaa: true, best: true, target: true, ..NO_REACH }),
        (10, Reached { full_combo: true, rank_a: true, rank_aa: true, rank_aaa: true, best: true, target: true }),
    ];
    for (perfects, expected) in cases {
        let standing = ScoreStanding::of(&sheet(perfects), perfects, pace);

        assert_eq!(Reached::of(&standing, perfects, 10, perfects), expected, "{perfects} of ten notes, all PGREAT");
    }
}

#[test]
fn a_full_combo_needs_every_note_gone_by_and_a_combo_as_long_as_the_chart() {
    let standing = ScoreStanding::of(&sheet(10), 10, TargetPace { best_score: 0, rival_score: 0, total_notes: 10 });

    assert!(Reached::of(&standing, 10, 10, 10).full_combo);
    assert!(!Reached::of(&standing, 10, 10, 9).full_combo, "a broken combo");
    assert!(!Reached::of(&standing, 9, 10, 9).full_combo, "a note still to come");
}

#[test]
fn a_run_with_no_best_and_no_target_has_reached_both_from_its_first_judgement() {
    let standing = ScoreStanding::of(&sheet(0), 1, TargetPace { best_score: 0, rival_score: 0, total_notes: 10 });

    let reached = Reached::of(&standing, 1, 10, 0);

    assert!(reached.best && reached.target, "nought is at least nought");
}

/// A timeline at `time_us` with a bar line or without, and with a note in its first lane or none.
fn line(time_us: i64, section_line: bool, note: bool) -> TimeLine {
    let mut line = TimeLine::empty(Mode::BEAT_7K.key, time_us, 0.0, TEMPO);
    line.section_line = section_line;
    line.notes[FIRST_KEY_LANE] = note.then(|| Note::normal(0, time_us, 0.0));
    line
}

#[test]
fn the_bar_lines_and_the_first_note_are_read_off_the_charts_timelines() {
    let timelines =
        [line(0, true, false), line(BEAT_US, false, false), line(BAR_US, true, true), line(BAR_US + BEAT_US, false, true), line(2 * BAR_US, true, false)];

    assert_eq!(section_times_us(&timelines), [0, BAR_US, 2 * BAR_US]);
    assert_eq!(first_note_us(&timelines), BAR_US);
    assert_eq!(first_note_us(&timelines[..2]), 0, "a chart with no note previews from its top");
}

#[test]
fn the_setup_of_a_run_with_no_skin_takes_the_charts_shape_and_the_references_defaults() {
    let model = Model {
        mode: Mode::BEAT_14K,
        meta: ModelMeta::default(),
        wavmap: Vec::new(),
        bgamap: Vec::new(),
        init_bpm: TEMPO,
        timelines: vec![line(0, true, false), line(BAR_US, true, true)],
        md5: String::new(),
        sha256: String::new(),
    };

    let setup = PlaySetup::of(&model, None, true);

    let expected = PlaySetup { autoplay: true, sections_us: vec![0, BAR_US], first_note_us: BAR_US, ..PlaySetup::new(Mode::BEAT_14K) };
    assert_eq!(setup, expected);
}

#[test]
fn a_setup_with_nothing_said_is_the_references_own_defaults() {
    let setup = PlaySetup::new(Mode::BEAT_7K);

    assert_eq!((setup.judge_timer, setup.judge_regions, setup.practice_freq), (1, 0, FULL_PERCENT));
    assert!(!setup.autoplay && setup.sections_us.is_empty());
    assert_eq!(setup.pm_motion_cycles_ms, [1; PM_MOTION_COUNT]);
}

/// A pop'n run whose character's motions each last `cycle_ms`, standing in neutral since
/// [`READY_US`] and started.
fn pm_run(cycle_ms: i64) -> (PlayTimerDriver, TimerState) {
    let mut driver = PlayTimerDriver::new(PlaySetup { pm_motion_cycles_ms: [cycle_ms; PM_MOTION_COUNT], ..setup(Mode::POPN_9K, 1) });
    let mut timers = TimerState::new();
    driver.apply(&mut timers, READY_US, SceneEvent::Standby);
    driver.apply(&mut timers, READY_US, SceneEvent::Ready);
    driver.apply(&mut timers, START_US, SceneEvent::Started { start_offset_us: 0 });
    (driver, timers)
}

/// The pop'n character timers that are on.
fn pm_on(timers: &TimerState) -> Vec<TimerId> {
    PM_TIMERS.into_iter().filter(|motion| timers.is_on(*motion)).collect()
}

#[test]
fn standing_by_puts_both_sides_of_a_character_in_neutral_once() {
    let mut driver = PlayTimerDriver::new(setup(Mode::POPN_9K, 1));
    let mut timers = TimerState::new();

    driver.apply(&mut timers, FIRST_US, SceneEvent::Standby);
    driver.apply(&mut timers, NEXT_US, SceneEvent::Standby);
    assert_eq!((timers.value_us(timer_id::PM_CHARA_1P_NEUTRAL), timers.value_us(timer_id::PM_CHARA_2P_NEUTRAL)), (FIRST_US, FIRST_US));

    timers.off(timer_id::PM_CHARA_2P_NEUTRAL);
    driver.apply(&mut timers, LAST_US, SceneEvent::Standby);
    assert_eq!(
        (timers.value_us(timer_id::PM_CHARA_1P_NEUTRAL), timers.value_us(timer_id::PM_CHARA_2P_NEUTRAL)),
        (LAST_US, LAST_US),
        "one side out puts both in again"
    );
}

#[test]
fn a_character_leaves_neutral_for_the_motion_of_the_last_judgement_and_comes_back_after_a_cycle() {
    let cycle_ms = 1_000;
    let round_us = READY_US + 2 * at_ms(cycle_ms);
    let cases = [
        (Judge::PerfectGreat, true, timer_id::PM_CHARA_1P_FEVER, timer_id::PM_CHARA_2P_BAD),
        (Judge::PerfectGreat, false, timer_id::PM_CHARA_1P_GREAT, timer_id::PM_CHARA_2P_BAD),
        (Judge::Great, false, timer_id::PM_CHARA_1P_GREAT, timer_id::PM_CHARA_2P_BAD),
        (Judge::Good, false, timer_id::PM_CHARA_1P_GOOD, timer_id::PM_CHARA_2P_BAD),
        (Judge::Bad, false, timer_id::PM_CHARA_1P_BAD, timer_id::PM_CHARA_2P_GREAT),
        (Judge::Poor, true, timer_id::PM_CHARA_1P_BAD, timer_id::PM_CHARA_2P_GREAT),
    ];
    for (judge, gauge_max, first_side, second_side) in cases {
        let (mut driver, mut timers) = pm_run(cycle_ms);
        driver.apply(&mut timers, FIRST_US, judged(FIRST_KEY_LANE, judge));

        driver.apply(&mut timers, round_us, SceneEvent::Playing(PlayingFrame { gauge_max, past_notes: 1, ..PLAIN_FRAME }));
        assert_eq!(pm_on(&timers), [first_side, second_side, timer_id::PM_CHARA_DANCE], "{judge:?} with the gauge full: {gauge_max}");
        assert_eq!((timers.value_us(first_side), timers.value_us(second_side)), (round_us, round_us));

        let back_us = round_us + at_ms(cycle_ms);
        driver.apply(&mut timers, back_us, SceneEvent::Playing(PlayingFrame { gauge_max, past_notes: 1, ..PLAIN_FRAME }));
        assert_eq!(pm_on(&timers), [timer_id::PM_CHARA_1P_NEUTRAL, timer_id::PM_CHARA_2P_NEUTRAL, timer_id::PM_CHARA_DANCE], "{judge:?}: back in neutral");
        assert_eq!(timers.value_us(timer_id::PM_CHARA_1P_NEUTRAL), back_us);

        driver.apply(&mut timers, back_us + at_ms(cycle_ms), SceneEvent::Playing(PlayingFrame { gauge_max, past_notes: 1, ..PLAIN_FRAME }));
        assert!(timers.is_on(timer_id::PM_CHARA_1P_NEUTRAL), "{judge:?}: no note has gone by since, so it stays there");
    }
}

#[test]
fn a_character_leaves_neutral_only_as_a_cycle_comes_round_and_only_after_a_judgement() {
    let cycle_ms = 1_000;
    let (mut driver, mut timers) = pm_run(cycle_ms);
    let neutral = [timer_id::PM_CHARA_1P_NEUTRAL, timer_id::PM_CHARA_2P_NEUTRAL, timer_id::PM_CHARA_DANCE];
    let playing = PlayingFrame { past_notes: 1, ..PLAIN_FRAME };

    driver.apply(&mut timers, READY_US + 2 * at_ms(cycle_ms), SceneEvent::Playing(playing));
    assert_eq!(pm_on(&timers), neutral, "nothing has been judged yet");

    driver.apply(&mut timers, FIRST_US, judged(FIRST_KEY_LANE, Judge::Great));
    driver.apply(&mut timers, READY_US + 2 * at_ms(cycle_ms) + at_ms(PM_NEUTRAL_WINDOW_MS), SceneEvent::Playing(playing));
    assert_eq!(pm_on(&timers), neutral, "seventeen milliseconds past the cycle is past the window");

    driver.apply(&mut timers, READY_US + 3 * at_ms(cycle_ms) + at_ms(PM_NEUTRAL_WINDOW_MS - 1), SceneEvent::Playing(playing));
    assert_eq!(pm_on(&timers), [timer_id::PM_CHARA_1P_GREAT, timer_id::PM_CHARA_2P_BAD, timer_id::PM_CHARA_DANCE], "sixteen is inside it");
}

#[test]
fn the_chart_running_out_stops_a_character_and_leaves_the_music_end_timer_on() {
    let (mut driver, mut timers) = pm_run(1);
    driver.apply(&mut timers, FIRST_US, SceneEvent::Playing(PLAIN_FRAME));
    assert!(!pm_on(&timers).is_empty());

    driver.apply(&mut timers, NEXT_US, SceneEvent::MusicEnded);

    assert!(pm_on(&timers).is_empty());
    assert_eq!(timers.value_us(timer_id::MUSIC_END), NEXT_US);
}
