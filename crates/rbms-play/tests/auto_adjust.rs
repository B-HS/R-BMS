//! Headless proof of the judge offset that adjusts itself: a player who follows the notes and is
//! steadily early or late is brought in, a run that plays itself or reproduces a replay is left
//! alone, and a replay comes out as the run it recorded whatever the offset is when it is watched.
//!
//! The rule is the reference's (`JudgeManager.java:697-703`): every judgement of GOOD or better
//! within 150 ms takes a thirtieth of its error, rounded to a whole millisecond, off the offset.
//! The offset is how far ahead of the song the notes are drawn (`LaneRenderer.java:300-301`), and
//! it is nothing else: an input is judged at the instant it arrived. So what brings a player in is
//! the player, following notes that are drawn later or earlier than they were.
//!
//! The three replays under `tests/fixtures` were written by the build before that, in which the
//! offset moved the instant an input was judged at instead. They state the offsets their inputs
//! were judged at, and have to come out as the runs they were.

use std::path::Path;

use rbms_play::{AUTO_ADJUST_OFFSET_MAX_MS, AUTO_ADJUST_OFFSET_MIN_MS, NullSink, PlaySession, PlaySummary, SessionClock, SessionOptions};
use rbms_store::{Replay, ReplayEvent, ReplayJudge};

/// How finely the song clock is stepped: the unit the offset is counted in, so a player who aims at
/// where a note is drawn always has a frame to press on.
const STEP_US: i64 = 1_000;

/// How finely the runs that reproduce a replay step the clock, which is how finely the runs the
/// fixtures were recorded from stepped it.
const REPLAY_STEP_US: i64 = 5_000;

/// How long past the last note a run keeps stepping.
const TAIL_US: i64 = 2_500_000;

/// How long each press is held.
const HOLD_US: i64 = 20_000;

/// Microseconds in a millisecond.
const US_PER_MS: i64 = 1_000;

/// The error an input has to be inside for the offset to leave it alone, in milliseconds
/// (`mfast + 15000` over 30000 is nought below it).
const DEAD_ZONE_MS: i64 = 15;

/// How far ahead of the note he follows the steady player of these tests presses, in milliseconds.
const HABIT_MS: i64 = 40;

/// What a player goes by.
#[derive(Clone, Copy)]
enum Follows {
    /// The notes: he presses his habit ahead of the moment a note is drawn on the judge line.
    Notes,
    /// The sound: he presses his habit ahead of the note's own time, wherever it is drawn.
    Sound,
}

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
    /// The offset after every press, in milliseconds.
    offsets_ms: Vec<i32>,
    /// How far ahead of its note every press was judged, in microseconds.
    errors_us: Vec<i64>,
}

/// Play every note `early_ms` ahead of what the player goes by (behind it when negative) on a run
/// set up by `options`. A note is drawn on the judge line the run's offset ahead of its own time,
/// and the offset is read afresh for every note, as a player's eyes read it.
fn play(options: SessionOptions, follows: Follows, early_ms: i64) -> Played {
    let mut session = PlaySession::new(model(), options);
    let notes = note_times();
    let (mut offsets_ms, mut errors_us) = (Vec::new(), Vec::new());
    let (mut next, mut held_since) = (0, None);
    let mut song_us = 0;
    while song_us <= end_us() {
        let drawn_ahead_ms = match follows {
            Follows::Notes => i64::from(session.judge_offset_ms()),
            Follows::Sound => 0,
        };
        if notes.get(next).is_some_and(|note_us| song_us >= note_us - (drawn_ahead_ms + early_ms) * US_PER_MS) {
            let hit = session.press(0, song_us, &mut NullSink);
            errors_us.extend(hit.map(|result| result.delta_us));
            offsets_ms.push(session.judge_offset_ms());
            held_since = Some(song_us);
            next += 1;
        }
        if held_since.is_some_and(|down_us| song_us >= down_us + HOLD_US) {
            session.release(0, song_us);
            held_since = None;
        }
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    Played { summary: session.summary(), events: session.recorded_events().to_vec(), offsets_ms, errors_us }
}

fn play_with_habit(options: SessionOptions, early_ms: i64) -> Played {
    play(options, Follows::Notes, early_ms)
}

fn adjusting(start_ms: i32) -> SessionOptions {
    SessionOptions { auto_calibration: true, judge_offset_us: i64::from(start_ms) * US_PER_MS, ..Default::default() }
}

/// The replay a run played by hand is saved as: its inputs, judged where they are.
fn replay_of(events: Vec<ReplayEvent>) -> Replay {
    Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: rbms_model::Mode::BEAT_7K.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: ReplayJudge::default(),
        events,
    }
}

/// Reproduce `replay` on a run set up by `options`, scrubbing to `seek_to` first when it is given.
fn reproduce(replay: Replay, options: SessionOptions, seek_to: Option<i64>) -> (PlaySummary, PlaySession) {
    let mut session = PlaySession::new(model(), SessionOptions { replay: Some(replay), ..options });
    if let Some(target_us) = seek_to {
        session.seek(target_us);
    }
    let mut song_us = seek_to.unwrap_or(0);
    while song_us <= end_us() {
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += REPLAY_STEP_US;
    }
    (session.summary(), session)
}

#[test]
fn a_player_who_is_steadily_early_is_brought_inside_the_dead_zone() {
    let played = play_with_habit(adjusting(0), HABIT_MS);
    assert_eq!(played.errors_us.len(), note_times().len(), "every note was hit");
    assert_eq!(played.errors_us[0], HABIT_MS * US_PER_MS, "the first input is judged as it arrived");
    assert_eq!(&played.offsets_ms[..4], [-1, -2, -3, -4], "40 ms early is the notes one millisecond later after every input");
    let settled = i32::try_from(-(HABIT_MS - DEAD_ZONE_MS + 1)).expect("a small number");
    assert_eq!(played.offsets_ms.last(), Some(&settled), "the offset stops where the error first falls inside 15 ms");
    for pair in played.errors_us.windows(2) {
        assert!(pair[1] <= pair[0], "the error only ever shrinks: {} then {}", pair[0], pair[1]);
    }
    assert_eq!(played.errors_us.last(), Some(&((DEAD_ZONE_MS - 1) * US_PER_MS)));
}

#[test]
fn a_player_who_is_steadily_late_is_brought_in_from_the_other_side() {
    let played = play_with_habit(adjusting(0), -HABIT_MS);
    assert_eq!(&played.offsets_ms[..4], [1, 2, 3, 4], "40 ms late is the notes one millisecond earlier after every input");
    let settled = i32::try_from(HABIT_MS - DEAD_ZONE_MS + 1).expect("a small number");
    assert_eq!(played.offsets_ms.last(), Some(&settled));
    for pair in played.errors_us.windows(2) {
        assert!(pair[1] >= pair[0], "the error only ever shrinks: {} then {}", pair[0], pair[1]);
    }
    assert_eq!(played.errors_us.last(), Some(&(-(DEAD_ZONE_MS - 1) * US_PER_MS)));
}

#[test]
fn a_bigger_error_is_closed_in_bigger_steps() {
    let played = play_with_habit(adjusting(0), 100);
    assert_eq!(&played.offsets_ms[..3], [-3, -6, -9], "100 ms early is three milliseconds, then 97 and 94 are three again");
}

/// With the row off the offset stays, and it still does what it is for: a player who follows the
/// notes presses that much earlier because they are drawn that much earlier, and every press is
/// judged where it landed.
#[test]
fn nothing_moves_while_the_offset_is_not_asked_to_adjust_itself() {
    let played = play_with_habit(SessionOptions { judge_offset_us: 7 * US_PER_MS, ..Default::default() }, HABIT_MS);
    assert!(played.offsets_ms.iter().all(|offset| *offset == 7), "{:?}", played.offsets_ms);
    assert!(played.errors_us.iter().all(|error| *error == (HABIT_MS + 7) * US_PER_MS), "{:?}", played.errors_us);
}

/// How far short of an end of the range the runs that walk onto it start, in milliseconds.
const SHORT_OF_THE_END_MS: i32 = 5;

/// A player whose inputs are still judged 40 ms out at the end of the range: the offset steps onto
/// the end and stays there, however many more inputs ask it to go on.
#[test]
fn the_offset_stops_at_both_ends_of_its_range() {
    let walked = usize::try_from(SHORT_OF_THE_END_MS).expect("a small number");
    let near_max = AUTO_ADJUST_OFFSET_MAX_MS - SHORT_OF_THE_END_MS;
    let top_end = play_with_habit(adjusting(near_max), -(i64::from(near_max) + HABIT_MS));
    assert_eq!(top_end.errors_us[0], -HABIT_MS * US_PER_MS, "the input is judged late however far ahead the notes already are");
    assert_eq!(top_end.offsets_ms.len(), note_times().len());
    assert!(top_end.offsets_ms[walked..].iter().all(|offset| *offset == AUTO_ADJUST_OFFSET_MAX_MS), "{:?}", top_end.offsets_ms);
    assert_eq!(top_end.offsets_ms.iter().max(), Some(&AUTO_ADJUST_OFFSET_MAX_MS));

    let near_min = AUTO_ADJUST_OFFSET_MIN_MS + SHORT_OF_THE_END_MS;
    let bottom_end = play_with_habit(adjusting(near_min), HABIT_MS - i64::from(near_min));
    assert_eq!(bottom_end.errors_us[0], HABIT_MS * US_PER_MS);
    assert!(bottom_end.offsets_ms[walked..].iter().all(|offset| *offset == AUTO_ADJUST_OFFSET_MIN_MS), "{:?}", bottom_end.offsets_ms);
    assert_eq!(bottom_end.offsets_ms.iter().min(), Some(&AUTO_ADJUST_OFFSET_MIN_MS));
}

/// The rule moves the notes, so it brings in a player who goes by them and no other. One who
/// presses by the sound alone is as early after every step as before it, and the offset walks on
/// for as long as he plays. This is the reference's rule as it stands; the test is here so the
/// walk is known rather than met.
#[test]
fn a_player_who_goes_by_the_sound_alone_is_not_brought_in() {
    let played = play(adjusting(0), Follows::Sound, HABIT_MS);
    assert!(played.errors_us.iter().all(|error| *error == HABIT_MS * US_PER_MS), "{:?}", played.errors_us);
    let walked: Vec<i32> = (1..=i32::try_from(note_times().len()).expect("a small chart")).map(|input| -input).collect();
    assert_eq!(played.offsets_ms, walked, "one millisecond an input, with nothing to stop it short of the end of the range");
}

#[test]
fn a_run_that_plays_itself_never_moves_the_offset() {
    let mut session = PlaySession::new(model(), SessionOptions { autoplay: true, ..adjusting(5) });
    let mut song_us = 0;
    while song_us <= end_us() {
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += REPLAY_STEP_US;
    }
    assert_eq!(session.summary().counts[0], u32::try_from(note_times().len()).expect("a small chart"), "the chart played itself");
    assert_eq!(session.judge_offset_ms(), 5);
    assert_eq!(session.adjusted_inputs(), 0);
}

/// A run whose offset moved under it is judged where its inputs landed like any other, so its
/// replay has no offset to state and comes out the same whatever the offset is when it is watched.
#[test]
fn the_replay_of_an_adjusted_run_lands_on_the_judgements_the_run_got() {
    let played = play_with_habit(adjusting(0), HABIT_MS);
    assert!(played.summary.counts[0] > 0 && played.summary.counts[1] > 0, "the run crossed from GREAT to PGREAT as it came in: {:?}", played.summary);
    let final_ms = *played.offsets_ms.last().expect("the run pressed");
    assert_ne!(final_ms, 0, "the run moved its offset");

    for watched_at_ms in [final_ms, 0, 150] {
        let (reproduced, session) = reproduce(replay_of(played.events.clone()), adjusting(watched_at_ms), None);
        assert_eq!(reproduced, played.summary, "watched at {watched_at_ms} ms the inputs came out otherwise");
        assert_eq!(session.judge_offset_ms(), watched_at_ms, "reproducing a replay moves no offset, even with the setting on");
        assert_eq!(session.adjusted_inputs(), 0);
    }

    let halfway = note_times()[note_times().len() / 2];
    let (scrubbed, _) = reproduce(replay_of(played.events), adjusting(final_ms), Some(halfway));
    assert_eq!(scrubbed, played.summary, "a scrub rebuilds the same run");
}

/// What the build that wrote a fixture counted for the run it was written from: the six
/// judgements, the EX score, the early and the late inputs, and the mean error in microseconds.
struct Recorded {
    file: &'static str,
    counts: [u32; 6],
    ex_score: u32,
    fast: u32,
    slow: u32,
    avg_judge_us: i64,
}

/// A run whose offset adjusted itself from nothing to +26 ms under a player 40 ms early, with the
/// twenty-seven offsets its inputs were judged at; one judged 12 ms later than its inputs from
/// first to last; and one judged 25 ms earlier than them, played 40 ms late.
const RECORDED: [Recorded; 3] = [
    Recorded { file: "replay-wave12-adjusting.ron", counts: [44, 20, 0, 0, 0, 0], ex_score: 108, fast: 64, slow: 0, avg_judge_us: 19_484 },
    Recorded { file: "replay-wave12-one-offset.ron", counts: [0, 64, 0, 0, 0, 0], ex_score: 64, fast: 64, slow: 0, avg_judge_us: 28_000 },
    Recorded { file: "replay-wave12-late-offset.ron", counts: [64, 0, 0, 0, 0, 0], ex_score: 128, fast: 0, slow: 64, avg_judge_us: -15_000 },
];

fn fixture(name: &str) -> Replay {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    Replay::load(&path).unwrap_or_else(|error| panic!("fixture {} is committed: {error}", path.display()))
}

fn counted(summary: &PlaySummary) -> ([u32; 6], u32, u32, u32, i64) {
    (summary.counts, summary.ex_score, summary.fast, summary.slow, summary.avg_judge_us)
}

/// A replay written while the offset moved the judgement states what its inputs were judged at,
/// and is judged at that: straight through and scrubbed, and whatever the offset is now.
#[test]
fn a_replay_written_before_the_offset_moved_the_notes_comes_out_as_the_run_it_was() {
    for recorded in RECORDED {
        let wanted = (recorded.counts, recorded.ex_score, recorded.fast, recorded.slow, recorded.avg_judge_us);
        for watched_at_ms in [0, 77, -120] {
            let (straight, _) = reproduce(fixture(recorded.file), adjusting(watched_at_ms), None);
            assert_eq!(counted(&straight), wanted, "{} watched at {watched_at_ms} ms", recorded.file);
        }
        let halfway = note_times()[note_times().len() / 2];
        let (scrubbed, _) = reproduce(fixture(recorded.file), SessionOptions::default(), Some(halfway));
        assert_eq!(counted(&scrubbed), wanted, "{} scrubbed", recorded.file);
    }
}

/// The offsets such a replay states are what decide its judgements: with them left out, or with
/// its one offset taken for nothing, the same inputs come out as another run.
#[test]
fn the_offsets_such_a_replay_states_are_what_it_is_judged_at() {
    let stepped = fixture(RECORDED[0].file);
    assert!(stepped.judge.offset_steps.len() > 2, "the fixture moved its offset: {:?}", stepped.judge.offset_steps);
    let flattened = Replay { judge: ReplayJudge { offset_steps: Vec::new(), ..stepped.judge.clone() }, ..stepped };
    assert_ne!(reproduce(flattened, SessionOptions::default(), None).0.counts, RECORDED[0].counts);

    let one = fixture(RECORDED[1].file);
    assert_eq!(one.offset_ms, 12);
    assert!(one.judge.offset_steps.is_empty());
    let unstated = Replay { offset_ms: 0, ..one };
    assert_ne!(reproduce(unstated, SessionOptions::default(), None).0.avg_judge_us, RECORDED[1].avg_judge_us);
}
