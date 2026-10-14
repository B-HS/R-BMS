//! When a lane input is judged: at the instant the input carries, read off the song clock as it
//! stood then, not at whatever later instant the frame thread got round to it. A controller change
//! carries the instant the controller layer stamped it with, so a frame thread held up by a present,
//! a disk write or a slow frame adds nothing to the timing of what arrived meanwhile.
//!
//! A key carries the instant the window hands it over, which is the instant the frame thread
//! handles it: the window hands keys over only between frames, so a frame held up still delays a
//! key's timing (audit PERF-2, not fixed; test-debt 3.53). The key test here only pins that a lane
//! key reads the instant it is handed, which is what an input source stamping keys earlier would
//! rely on.

use std::time::{Duration, Instant};

use rbms_play::SessionOptions;

use super::hold_tests::{app_on, note_times, run_of};
use super::*;
use crate::Config;
use crate::gamepad::{PadBinding, PadChange, PadConfig, PadMapper, PadSnapshot};
use crate::stage::KeyInput;

/// A one-measure seven-key chart with two notes in its first key.
const CHART: &str = "#PLAYER 1\n#TITLE t\n#BPM 150\n#WAV01 a.wav\n#00111:0101\n";

/// How long the frame thread was busy before it handled the input: longer than a GREAT is wide,
/// so a judgement that took it in could not still be a PGREAT.
const HANDLED_LATE_BY: Duration = Duration::from_millis(20);

/// The first key lane.
const FIRST_LANE: usize = 0;

/// The native code of the controller button bound to the first lane in these runs.
const BUTTON: u32 = 7;

/// How long the controller tap of these runs is held, in microseconds: far shorter than a frame.
const TAP_US: i64 = 3_000;

/// Put the song clock where the instant `at` reads as `song_us` and nothing earlier has been read.
fn song_reads_at(app: &mut App, at: Instant, song_us: i64) {
    app.shared.clock = at.checked_sub(Duration::from_micros(song_us as u64)).expect("the process has been up for longer than the note's time");
    app.shared.song_us_last.set(0);
}

/// An input that happened `HANDLED_LATE_BY` ago, and is only being handled now.
fn a_while_ago() -> Instant {
    Instant::now().checked_sub(HANDLED_LATE_BY).expect("the process has been up for longer than a frame")
}

/// A lane key handed with an instant earlier than the one it is handled at is judged at the instant
/// it was handed with. In the shipped app the two instants of a key are the same.
#[test]
fn a_key_is_judged_at_the_instant_it_is_handed_with_not_the_one_it_is_handled_at() {
    let mut app = app_on("input-time-key", Config::default());
    let mut state = run_of(CHART, SessionOptions::default());
    let note_us = note_times(&state)[0];
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    let arrived = a_while_ago();
    song_reads_at(&mut app, arrived, note_us);

    let key = KeyInput { code: lane_key, pressed: true, released: false, text: None };
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: arrived, dt: 0.0 }, key);

    assert_eq!(state.session.recorded_events().iter().map(|event| event.t).collect::<Vec<_>>(), [note_us], "the press is recorded where it arrived");
    assert_eq!(state.session.judge().counts[rbms_judge::Judge::PerfectGreat as usize], 1, "and judged there, dead on the note");
}

/// A controller tap shorter than a frame, pressed and let go between two polls, is played as a
/// press and a release, each at the instant the controller layer stamped it: the release where the
/// debounce window let it in.
#[test]
fn a_controller_tap_inside_one_frame_is_judged_at_the_instants_it_happened() {
    let mut app = app_on("input-time-pad", Config::default());
    let mut state = run_of(CHART, SessionOptions::default());
    let note_us = note_times(&state)[0];
    let epoch = a_while_ago();
    song_reads_at(&mut app, epoch, note_us);

    let mut cfg = PadConfig::default();
    cfg.set_lane(Mode::BEAT_7K, FIRST_LANE, Some(PadBinding::Button(BUTTON)));
    let changes =
        [(0, PadChange::Button { code: BUTTON, standard: None, pressed: true }), (TAP_US, PadChange::Button { code: BUTTON, standard: None, pressed: false })];
    let polled_at_us = HANDLED_LATE_BY.as_micros() as i64;
    let events = PadMapper::new().resolve_changes(&changes, polled_at_us, &cfg, Mode::BEAT_7K, &mut PadSnapshot::default());
    assert_eq!(events.len(), 2, "both halves of the tap come out of one poll: {events:?}");
    for (event, at_us) in events {
        let at = epoch + Duration::from_micros(at_us as u64);
        state.handle_pad(&mut FrameCtx { shared: &mut app.shared, now: at, dt: 0.0 }, event);
    }

    let window_us = i64::from(cfg.effective_debounce_ms()) * US_PER_MS;
    let recorded: Vec<(i64, bool)> = state.session.recorded_events().iter().map(|event| (event.t, event.press)).collect();
    assert_eq!(recorded, [(note_us, true), (note_us + window_us, false)], "the release waits out the debounce window from the press");
    assert_eq!(state.session.judge().counts[rbms_judge::Judge::PerfectGreat as usize], 1, "the tap is a PGREAT on the note");
}

/// Microseconds in one millisecond, the unit the debounce window is set in.
const US_PER_MS: i64 = 1_000;
