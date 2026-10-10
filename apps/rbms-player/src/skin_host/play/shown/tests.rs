//! The play frame is made from a live session and the screen's own figures, and nothing else.

use rbms_chart::to_model;
use rbms_model::Mode;
use rbms_parser::parse;
use rbms_play::{NullSink, PlaySession, SessionClock, SessionOptions};
use rbms_skin::timer::{TimerId, TimerState};

use super::*;
use crate::skin_host::score::standing::PointFamily;

/// Four notes on one key lane at 2.0, 2.5, 3.0 and 3.5 seconds, judged at the mode's stock widths.
const CHART: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n";

/// Song time of the first note, and the gap between notes.
const FIRST_NOTE_US: i64 = 2_000_000;
const NOTE_GAP_US: i64 = 500_000;

/// A frame early in the song, before any note is near, and one between the third note and the fourth.
const PRELUDE_US: i64 = 1_000_000;
const BETWEEN_THIRD_AND_FOURTH_US: i64 = 3_200_000;

/// Where the fourth note ends up unjudged: a frame long after the last note and its windows.
const AFTER_EVERYTHING_US: i64 = 10_000_000;

/// How much earlier than the second note it is pressed, and how much later than the third. Both
/// are inside the widths of the judgement they earn at this chart's rank: a GREAT and a GOOD.
const EARLY_GREAT_US: i64 = 40_000;
const LATE_GOOD_US: i64 = 100_000;

/// How long the reference keeps this chart in its playing state: the last note and five seconds.
const PLAY_TIME_MS: i32 = 8_500;

fn fresh() -> PlaySession {
    let mut session = PlaySession::new(to_model(&parse(CHART), Mode::BEAT_7K), SessionOptions::default());
    session.tick(SessionClock::at(PRELUDE_US), &mut NullSink);
    session
}

/// The fixture with three of its four notes judged: a PGREAT on time, a GREAT early and a GOOD late.
fn midway() -> PlaySession {
    let mut session = fresh();
    for (note, offset_us) in [(0, 0), (1, -EARLY_GREAT_US), (2, LATE_GOOD_US)] {
        session.press(0, FIRST_NOTE_US + note * NOTE_GAP_US + offset_us, &mut NullSink);
    }
    session.tick(SessionClock::at(BETWEEN_THIRD_AND_FOURTH_US), &mut NullSink);
    session
}

#[test]
fn a_run_that_has_not_been_judged_has_no_score_data_yet() {
    let shown = PlayShown::of(&fresh(), &PlayLive { best_score: 4, target_score: 6, ..PlayLive::default() });

    assert!(!shown.run.scored, "the reference updates its score properties after a judgement only");
    assert_eq!((shown.run.sheet.notes, shown.run.sheet.ex_score(), shown.run.standing.now_ex), (4, 0, 0));
    assert_eq!((shown.run.standing.best_score, shown.run.standing.rival_score), (4, 6), "the best and the target stand from the first frame");
    assert_eq!(shown.run.standing.now_rate, 0.0, "and a run nothing has gone by in is not at 100% before its first judgement");
}

#[test]
fn a_run_in_progress_is_rated_over_the_notes_gone_by() {
    let shown = PlayShown::of(&midway(), &PlayLive { best_score: 4, target_score: 6, ..PlayLive::default() });

    assert!(shown.run.scored);
    let sheet = &shown.run.sheet;
    assert_eq!((sheet.early, sheet.late), ([1, 1, 0, 0, 0, 0], [0, 0, 1, 0, 0, 0]));
    assert_eq!((sheet.notes, sheet.max_combo, sheet.family), (4, 3, PointFamily::Beat7));
    assert_eq!(sheet.min_bp, 1, "the note the run has not reached counts as bad-poor");
    let standing = &shown.run.standing;
    assert_eq!((standing.now_ex, standing.now_rate), (3, 0.5), "3 of the 6 EX points the three notes could give");
    assert_eq!((standing.rate, standing.rate_int, standing.rate_after_dot), (0.375, 37, 50), "and 3 of the chart's 8");
    assert_eq!((standing.now_best_score, standing.now_rival_score), (3, 4), "the best and the target as they would stand three notes in");
}

#[test]
fn the_gauge_is_the_one_being_played() {
    let shown = PlayShown::of(&midway(), &PlayLive::default());

    assert_eq!(shown.gauge, GaugeReading { value: 100.0, max: 100.0, kind: 2, live: true }, "the normal gauge, filled by three good judgements");
    assert!(shown.qualified, "and over its border");
    let start = PlayShown::of(&fresh(), &PlayLive::default());
    assert_eq!((start.gauge.value, start.qualified), (20.0, false), "a normal gauge starts at 20% of 80% to clear");
}

#[test]
fn the_playing_time_is_the_references_not_the_songs_length() {
    assert_eq!(PlayShown::of(&fresh(), &PlayLive::default()).play_time_ms, PLAY_TIME_MS);
}

#[test]
fn the_frame_carries_what_the_screen_said_unchanged() {
    let live = PlayLive {
        phase: PlayPhase::Ready,
        kind: PlayKind::Replay,
        judgements: [Some(Judgement { judge: 1, timing_ms: 12 }), None, None],
        lanes: LaneSettings { hispeed: 2.5, lane_cover: 0.25, ..LaneSettings::default() },
        judge_timing_ms: -7,
        bga_on: true,
        long_note_mode: 2,
        ..PlayLive::default()
    };

    assert_eq!(PlayShown::of(&fresh(), &live).live, live);
}

#[test]
fn the_load_is_in_progress_until_the_screen_leaves_its_preload() {
    let loading = |phase| PlayShown::of(&fresh(), &PlayLive { phase, load_progress: 0.25, ..PlayLive::default() }).loading();

    assert_eq!(loading(PlayPhase::Preload), LoadingState { screen: LoadingScreen::Preload, progress: 0.25 });
    for phase in [PlayPhase::Ready, PlayPhase::Play, PlayPhase::Failed, PlayPhase::Finished] {
        assert_eq!(loading(phase), LoadingState { screen: LoadingScreen::Started, progress: 0.25 }, "{phase:?}");
    }
}

#[test]
fn the_clock_counts_whole_milliseconds_from_the_play_timer() {
    let mut timers = TimerState::new();
    assert_eq!(PlayClock::of(&timers, 9_000_000).elapsed_ms(), None, "no play timer, no elapsed time");
    timers.set_on(TimerId(41), 2_000_000);
    assert_eq!(PlayClock::of(&timers, 5_250_999).elapsed_ms(), Some(3_250), "cut, not rounded");
    assert_eq!(PlayClock::stopped(5_000_000).elapsed_ms(), None);
    assert_eq!(PlayClock::default(), PlayClock::stopped(0));
}

#[test]
fn the_time_a_note_takes_to_cross_the_lane_is_the_references_region() {
    assert_eq!(travel_region_ms(150.0, 2.0, 1.0), 800.0);
    assert_eq!(travel_region_ms(150.0, 2.0, 2.0), 400.0, "a faster scroll crosses sooner");
    assert_eq!(travel_region_ms(150.0, 2.0, 0.0), 0.0, "a chart that does not scroll has no region");
    assert_eq!(travel_region_ms(150.0, 2.0, -1.0), 0.0, "and neither has one that scrolls backwards");
}

#[test]
fn a_run_whose_notes_have_all_gone_by_is_rated_over_the_whole_chart() {
    let mut session = midway();
    session.tick(SessionClock::at(AFTER_EVERYTHING_US), &mut NullSink);
    let shown = PlayShown::of(&session, &PlayLive::default());

    assert_eq!(shown.run.standing.now_rate, shown.run.standing.rate, "with every note gone by the two rates are one");
    assert_eq!((shown.run.sheet.min_bp, shown.run.sheet.max_combo), (1, 3), "the unhit fourth note is a POOR");
}
