//! Headless proof that a replay reproduces the run it recorded.
//!
//! A [`PlaySession`] driven against [`NullSink`] needs no audio device and no window, so a whole
//! interactive run can be played, recorded, and then reproduced from its own recording inside a
//! test. The reproduction has to land on exactly the same judgement — same counts, same EX, same
//! early/late split, same gauge — or replays, the analysis screen and every score derived from them
//! are worthless.

use rbms_play::{NullSink, PlaySession, PlaySummary, SessionClock, SessionOptions};
use rbms_store::{Replay, ReplayEvent};

/// How finely both runs step the song clock. Inputs land on multiples of this, so a recorded input
/// and the live one it came from are judged in the same order relative to the miss sweep.
const STEP_US: i64 = 5_000;

/// How long past the last note both runs keep stepping, so every unhit note is swept to a miss.
const TAIL_US: i64 = 2_500_000;

/// Judge offset both runs use, chosen non-zero so a reproduction that ignored it would show up.
const JUDGE_OFFSET_US: i64 = 12_000;

/// Eight notes on lane 0 at 2.0 s .. 5.5 s, with an accompaniment channel so the run also books
/// non-note sounds.
const CHART: &[u8] = b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00101:02\r\n#00111:01010101\r\n#00211:01010101\r\n";

/// Presses covering the whole judgement range: dead on, slightly early, slightly late, far enough
/// out to break the combo, plus one note left untouched so a miss is swept.
const PRESSES_US: &[i64] = &[2_000_000, 2_515_000, 3_040_000, 3_500_000, 4_100_000, 4_500_000, 5_000_000];

/// How long each press is held.
const HOLD_US: i64 = 50_000;

fn model() -> rbms_model::Model {
    rbms_chart::to_model(&rbms_parser::parse(CHART), rbms_model::Mode::BEAT_7K)
}

fn options() -> SessionOptions {
    SessionOptions { judge_offset_us: JUDGE_OFFSET_US, ..Default::default() }
}

fn end_us() -> i64 {
    PlaySession::new(model(), options()).last_time_us() + TAIL_US
}

/// Play the chart by hand and hand back what was judged and what was recorded.
fn play_by_hand() -> (PlaySummary, Vec<ReplayEvent>) {
    let mut session = PlaySession::new(model(), options());
    let mut song_us = 0;
    while song_us <= end_us() {
        if PRESSES_US.contains(&song_us) {
            session.press(0, song_us, &mut NullSink);
        }
        if PRESSES_US.contains(&(song_us - HOLD_US)) {
            session.release(0, song_us);
        }
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    (session.summary(), session.recorded_events().to_vec())
}

fn replay_of(events: Vec<ReplayEvent>) -> Replay {
    Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: rbms_model::Mode::BEAT_7K.name.to_string(),
        random: String::new(),
        seed: 0,
        offset_ms: (JUDGE_OFFSET_US / 1000) as i32,
        scratch_auto: false,
        gauge: String::new(),
        events,
    }
}

/// Reproduce a recording, optionally scrubbing to `seek_to` partway through.
fn reproduce(events: Vec<ReplayEvent>, seek_to: Option<i64>) -> PlaySummary {
    let mut session = PlaySession::new(model(), SessionOptions { replay: Some(replay_of(events)), ..options() });
    if let Some(target_us) = seek_to {
        session.seek(target_us);
    }
    let start_us = seek_to.unwrap_or(0);
    let mut song_us = start_us;
    while song_us <= end_us() {
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    session.summary()
}

#[test]
fn a_replay_reproduces_the_run_it_recorded() {
    let (played, recorded) = play_by_hand();
    assert!(played.counts.iter().filter(|count| **count > 0).count() >= 3, "the hand-played run must span several judgements: {played:?}");
    assert!(played.counts[4] > 0, "one note is left unhit so the run also covers a swept miss: {played:?}");
    assert_eq!(recorded.len(), PRESSES_US.len() * 2, "every press and release is recorded");

    let reproduced = reproduce(recorded, None);
    assert_eq!(reproduced, played, "reproducing the recording must land on exactly the same judgement");
}

#[test]
fn reproducing_the_same_replay_twice_gives_the_same_judgement() {
    let (_, recorded) = play_by_hand();
    assert_eq!(reproduce(recorded.clone(), None), reproduce(recorded, None));
}

#[test]
fn scrubbing_into_the_middle_rebuilds_the_same_judgement_as_playing_straight_through() {
    let (played, recorded) = play_by_hand();
    for target_us in [0, 2_600_000, 4_000_000, 5_400_000] {
        assert_eq!(reproduce(recorded.clone(), Some(target_us)), played, "a seek to {target_us} µs must rebuild the run exactly");
    }
}

#[test]
fn a_replay_run_records_nothing_of_its_own() {
    let (_, recorded) = play_by_hand();
    let mut session = PlaySession::new(model(), SessionOptions { replay: Some(replay_of(recorded)), ..options() });
    session.tick(SessionClock::at(end_us()), &mut NullSink);
    assert!(session.recorded_events().is_empty(), "reproducing a replay must not record a second one on top of it");
}

#[test]
fn a_reproduction_is_judged_at_the_offset_the_run_used() {
    let (played, recorded) = play_by_hand();
    let mut session = PlaySession::new(model(), SessionOptions { replay: Some(replay_of(recorded)), ..options() });
    session.set_judge_offset_us(JUDGE_OFFSET_US + 40_000);
    let mut song_us = 0;
    while song_us <= end_us() {
        session.tick(SessionClock::at(song_us), &mut NullSink);
        song_us += STEP_US;
    }
    assert_ne!(session.summary(), played, "judging the same inputs at a different offset must not come out identical");
}
