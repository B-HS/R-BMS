//! Headless proof of the judge offset that adjusts itself: a player who is steadily early or late
//! is brought in, a run that plays itself or reproduces a replay is left alone, and the replay of
//! an adjusted run still lands on the judgements the run got.
//!
//! The rule is the reference's (`JudgeManager.java:697-703`): every judgement of GOOD or better
//! within 150 ms moves the offset by a thirtieth of its error, rounded to a whole millisecond.

use rbms_play::{AUTO_ADJUST_OFFSET_MAX_MS, AUTO_ADJUST_OFFSET_MIN_MS, NullSink, PlaySession, PlaySummary, SessionClock, SessionOptions};
use rbms_store::{Replay, ReplayEvent, ReplayJudge, ReplayOffsetStep};

/// How finely the song clock is stepped. Inputs land on multiples of it.
const STEP_US: i64 = 5_000;

/// How long past the last note a run keeps stepping.
const TAIL_US: i64 = 2_500_000;

/// How long each press is held.
const HOLD_US: i64 = 20_000;

/// Microseconds in a millisecond.
const US_PER_MS: i64 = 1_000;

/// The error an input has to be inside for the offset to leave it alone, in milliseconds
/// (`mfast + 15000` over 30000 is nought below it).
const DEAD_ZONE_MS: i64 = 15;

/// How far off the note the steady player of these tests presses, in milliseconds.
const HABIT_MS: i64 = 40;

/// Sixty-four notes on lane 0, a quarter of a second apart at 120 BPM, from 2.0 s on.
fn model() -> rbms_model::Model {
    let mut chart = String::from("#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n");
    for measure in 1..=8 {
        chart.push_str(&format!("#{measure:03}11:0101010101010101\r\n"));
    }
    rbms_chart::to_model(&rbms_parser::parse(chart.as_bytes()), rbms_model::Mode::BEAT_7K)
}

/// When every note of the chart is due, in microseconds.
fn note_times() -> Vec<i64> {
    model().timelines.iter().filter(|timeline| timeline.notes.first().is_some_and(Option::is_some)).map(|timeline| timeline.time_us).collect()
}

fn end_us() -> i64 {
    PlaySession::new(model(), SessionOptions::default()).last_time_us() + TAIL_US
}

/// What one hand-played run came to.
struct Played {
    summary: PlaySummary,
    events: Vec<ReplayEvent>,
    steps: Vec<ReplayOffsetStep>,
    /// The offset after every press, in milliseconds.
    offsets_ms: Vec<i32>,
    /// How far ahead of its note every press was judged, in microseconds.
    errors_us: Vec<i64>,
}

/// Play every note `early_ms` ahead of its time (behind it when negative) on a run set up by
/// `options`.
fn play_with_habit(options: SessionOptions, early_ms: i64) -> Played {
    let mut session = PlaySession::new(model(), options);
    let presses: Vec<i64> = note_times().iter().map(|time_us| time_us - early_ms * US_PER_MS).collect();
    let (mut offsets_ms, mut errors_us) = (Vec::new(), Vec::new());
    let mut song_us = 0;
    while song_us <= end_us() {
        if presses.contains(&song_us) {
            let hit = session.press(0, song_us, &mut NullSink);
            errors_us.extend(hit.map(|result| result.delta_us));
            offsets_ms.push(session.judge_offset_ms());
        }
        if presses.contains(&(song_us - HOLD_US)) {
            session.release(0, song_us);
        }
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    Played { summary: session.summary(), events: session.recorded_events().to_vec(), steps: session.recorded_offset_steps(), offsets_ms, errors_us }
}

fn adjusting(start_ms: i32) -> SessionOptions {
    SessionOptions { auto_calibration: true, judge_offset_us: i64::from(start_ms) * US_PER_MS, ..Default::default() }
}

fn replay_of(events: Vec<ReplayEvent>, offset_ms: i32, steps: Vec<ReplayOffsetStep>) -> Replay {
    Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: rbms_model::Mode::BEAT_7K.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms,
        scratch_auto: false,
        gauge: String::new(),
        judge: ReplayJudge { offset_steps: steps, ..ReplayJudge::default() },
        events,
    }
}

/// Reproduce `replay` on a run set up by `options`, scrubbing to `seek_to` first when it is given.
fn reproduce(replay: Replay, options: SessionOptions, seek_to: Option<i64>) -> (PlaySummary, PlaySession) {
    let offset_us = i64::from(replay.offset_ms) * US_PER_MS;
    let mut session = PlaySession::new(model(), SessionOptions { replay: Some(replay), judge_offset_us: offset_us, ..options });
    if let Some(target_us) = seek_to {
        session.seek(target_us);
    }
    let mut song_us = seek_to.unwrap_or(0);
    while song_us <= end_us() {
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    (session.summary(), session)
}

#[test]
fn a_player_who_is_steadily_early_is_brought_inside_the_dead_zone() {
    let played = play_with_habit(adjusting(0), HABIT_MS);
    assert_eq!(played.errors_us.len(), note_times().len(), "every note was hit");
    assert_eq!(played.errors_us[0], HABIT_MS * US_PER_MS, "the first input is judged as it arrived");
    assert_eq!(&played.offsets_ms[..4], [1, 2, 3, 4], "40 ms early is one millisecond later after every input");
    let settled = i32::try_from(HABIT_MS - DEAD_ZONE_MS + 1).expect("a small number");
    assert_eq!(played.offsets_ms.last(), Some(&settled), "the offset stops where the error first falls inside 15 ms");
    for pair in played.errors_us.windows(2) {
        assert!(pair[1] <= pair[0], "the error only ever shrinks: {} then {}", pair[0], pair[1]);
    }
    assert_eq!(played.errors_us.last(), Some(&((DEAD_ZONE_MS - 1) * US_PER_MS)));
}

#[test]
fn a_player_who_is_steadily_late_is_brought_in_from_the_other_side() {
    let played = play_with_habit(adjusting(0), -HABIT_MS);
    assert_eq!(&played.offsets_ms[..4], [-1, -2, -3, -4]);
    let settled = i32::try_from(-(HABIT_MS - DEAD_ZONE_MS + 1)).expect("a small number");
    assert_eq!(played.offsets_ms.last(), Some(&settled));
    assert_eq!(played.errors_us.last(), Some(&(-(DEAD_ZONE_MS - 1) * US_PER_MS)));
}

#[test]
fn a_bigger_error_is_closed_in_bigger_steps() {
    let played = play_with_habit(adjusting(0), 100);
    assert_eq!(&played.offsets_ms[..3], [3, 6, 9], "100 ms early is three milliseconds, then 97 and 94 are three again");
}

#[test]
fn nothing_moves_while_the_offset_is_not_asked_to_adjust_itself() {
    let played = play_with_habit(SessionOptions { judge_offset_us: 7 * US_PER_MS, ..Default::default() }, HABIT_MS);
    assert!(played.offsets_ms.iter().all(|offset| *offset == 7), "{:?}", played.offsets_ms);
    assert!(played.steps.is_empty());
    assert!(played.errors_us.iter().all(|error| *error == (HABIT_MS - 7) * US_PER_MS));
}

/// How far short of an end of the range the runs that walk onto it start, in milliseconds: a
/// multiple of the clock step, so their inputs land on it.
const SHORT_OF_THE_END_MS: i32 = 5;

/// A player whose inputs are still judged 40 ms out at the end of the range: the offset steps onto
/// the end and stays there, however many more inputs ask it to go on.
#[test]
fn the_offset_stops_at_both_ends_of_its_range() {
    let walked = usize::try_from(SHORT_OF_THE_END_MS).expect("a small number");
    let near_max = AUTO_ADJUST_OFFSET_MAX_MS - SHORT_OF_THE_END_MS;
    let late_end = play_with_habit(adjusting(near_max), i64::from(near_max) + HABIT_MS);
    assert_eq!(late_end.errors_us[0], HABIT_MS * US_PER_MS, "the input is judged early however large the offset already is");
    assert_eq!(late_end.offsets_ms.len(), note_times().len());
    assert!(late_end.offsets_ms[walked..].iter().all(|offset| *offset == AUTO_ADJUST_OFFSET_MAX_MS), "{:?}", late_end.offsets_ms);
    assert_eq!(late_end.offsets_ms.iter().max(), Some(&AUTO_ADJUST_OFFSET_MAX_MS));
    assert_eq!(late_end.steps.len(), walked + 1, "the start and the steps onto the end, and none past it: {:?}", late_end.steps);

    let near_min = AUTO_ADJUST_OFFSET_MIN_MS + SHORT_OF_THE_END_MS;
    let early_end = play_with_habit(adjusting(near_min), i64::from(near_min) - HABIT_MS);
    assert_eq!(early_end.errors_us[0], -HABIT_MS * US_PER_MS);
    assert!(early_end.offsets_ms[walked..].iter().all(|offset| *offset == AUTO_ADJUST_OFFSET_MIN_MS), "{:?}", early_end.offsets_ms);
    assert_eq!(early_end.offsets_ms.iter().min(), Some(&AUTO_ADJUST_OFFSET_MIN_MS));
    assert_eq!(early_end.steps.len(), walked + 1);
}

#[test]
fn a_run_that_plays_itself_never_moves_the_offset() {
    let mut session = PlaySession::new(model(), SessionOptions { autoplay: true, ..adjusting(5) });
    let mut song_us = 0;
    while song_us <= end_us() {
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    assert_eq!(session.summary().counts[0], u32::try_from(note_times().len()).expect("a small chart"), "the chart played itself");
    assert_eq!(session.judge_offset_ms(), 5);
    assert!(session.recorded_offset_steps().is_empty());
    assert_eq!(session.adjusted_inputs(), 0);
}

#[test]
fn the_replay_of_an_adjusted_run_lands_on_the_judgements_the_run_got() {
    let played = play_with_habit(adjusting(0), HABIT_MS);
    assert!(played.steps.len() > 2, "the run moved its offset: {:?}", played.steps);
    assert_eq!(played.steps[0], ReplayOffsetStep { from_event: 0, offset_ms: 0 });
    assert!(played.summary.counts[0] > 0 && played.summary.counts[1] > 0, "the run crossed from GREAT to PGREAT as it came in: {:?}", played.summary);
    let final_ms = *played.offsets_ms.last().expect("the run pressed");

    let (reproduced, session) = reproduce(replay_of(played.events.clone(), final_ms, played.steps.clone()), adjusting(final_ms), None);
    assert_eq!(reproduced, played.summary, "every input is judged at the offset it was played under");
    assert_eq!(session.judge_offset_ms(), final_ms, "reproducing a replay moves no offset, even with the setting on");
    assert!(session.recorded_offset_steps().is_empty());

    let halfway = note_times()[note_times().len() / 2];
    let (scrubbed, _) = reproduce(replay_of(played.events.clone(), final_ms, played.steps.clone()), adjusting(final_ms), Some(halfway));
    assert_eq!(scrubbed, played.summary, "a scrub rebuilds the run under the same offsets");

    let (flattened, _) = reproduce(replay_of(played.events, final_ms, Vec::new()), SessionOptions::default(), None);
    assert_ne!(flattened, played.summary, "judged at the final offset alone the early inputs would read as better than they were");
}

#[test]
fn a_replay_with_no_steps_is_judged_at_its_one_offset_as_it_always_was() {
    let played = play_with_habit(SessionOptions { judge_offset_us: 12 * US_PER_MS, ..Default::default() }, HABIT_MS);
    assert!(played.steps.is_empty());
    let (reproduced, _) = reproduce(replay_of(played.events, 12, Vec::new()), adjusting(12), None);
    assert_eq!(reproduced, played.summary);
}
