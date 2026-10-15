//! When a lane input is judged: at the instant the input carries, read off the song clock as it
//! stood then, not at whatever later instant the frame thread got round to it. A controller change
//! carries the instant the controller layer stamped it with, so a frame thread held up by a present,
//! a disk write or a slow frame adds nothing to the timing of what arrived meanwhile.
//!
//! A key carries the instant the event loop takes it in. The loop no longer waits for the display
//! (the presenter's thread does, `gpu::present`, whose tests hold a display up and show a key is
//! taken in meanwhile), so that is the instant the window had it, short of a frame the loop is busy
//! drawing. The key tests here pin that a lane key is judged at the instant it is handed, and how
//! KEY DEBOUNCE hands a key over: a press where it came, a release the window keeps out at the
//! instant the window ends.

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

/// A one-measure seven-key chart whose first key holds one long note, from the second beat to the
/// fourth of a 120 BPM measure.
const LONG_NOTE_CHART: &str = "#PLAYER 1\n#TITLE t\n#BPM 120\n#WAV01 a.wav\n#00151:00010001\n";

/// How long after the press a chattering switch bounces open, and how long after that it closes
/// again: well inside any debounce window, far shorter than a press a player makes.
const BOUNCE_OPEN_AFTER: Duration = Duration::from_millis(2);
const BOUNCE_CLOSED_AFTER: Duration = Duration::from_millis(3);

/// How long past the end of the long note the run is ticked, enough for every judgment to settle.
const SETTLE_AFTER_US: i64 = 1_000_000;

/// A key whose switch chatters as it goes down -- down, up two milliseconds later, down again a
/// millisecond after that -- is one press: the long note it starts is held to its end and judged
/// once, a PGREAT, rather than let go of the instant the switch bounced open.
#[test]
fn a_key_that_chatters_as_it_goes_down_holds_its_long_note() {
    let mut app = app_on("input-time-chatter", Config::default());
    let mut state = run_of(LONG_NOTE_CHART, SessionOptions::default());
    let times = note_times(&state);
    let (head_us, end_us) = (times[0], times[1]);
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    let pressed = a_while_ago();
    song_reads_at(&mut app, pressed, head_us);

    let let_go = pressed + Duration::from_micros((end_us - head_us) as u64);
    for (at, down) in [(pressed, true), (pressed + BOUNCE_OPEN_AFTER, false), (pressed + BOUNCE_CLOSED_AFTER, true), (let_go, false)] {
        let key = KeyInput { code: lane_key, pressed: down, released: !down, text: None };
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: at, dt: 0.0 }, key);
    }
    state.session.tick(rbms_play::SessionClock::at(end_us + SETTLE_AFTER_US), &mut rbms_play::NullSink);

    let recorded: Vec<(i64, bool)> = state.session.recorded_events().iter().map(|event| (event.t, event.press)).collect();
    assert_eq!(recorded, [(head_us, true), (end_us, false)], "the bounce is no part of what was played");
    let counts = state.session.judge().counts;
    assert_eq!(counts.iter().sum::<u32>(), 1, "the long note is judged once: {counts:?}");
    assert_eq!(counts[rbms_judge::Judge::PerfectGreat as usize], 1, "and held to its end: {counts:?}");
}

/// A release that comes inside the window after the press -- a tap shorter than KEY DEBOUNCE -- is
/// played the instant the window ends, by the frame that comes after that, however late it comes.
#[test]
fn a_key_change_the_debounce_kept_out_is_played_where_the_window_ended_by_the_next_frame() {
    let mut app = app_on("input-time-kept-out", Config::default());
    let mut state = run_of(CHART, SessionOptions::default());
    let note_us = note_times(&state)[0];
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    let pressed = a_while_ago();
    song_reads_at(&mut app, pressed, note_us);
    for (at, down) in [(pressed, true), (pressed + BOUNCE_CLOSED_AFTER, false)] {
        let key = KeyInput { code: lane_key, pressed: down, released: !down, text: None };
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: at, dt: 0.0 }, key);
    }
    assert_eq!(state.session.recorded_events().len(), 1, "the release came inside the window and waits for it");

    state.update(&mut FrameCtx { shared: &mut app.shared, now: pressed + HANDLED_LATE_BY, dt: 0.0 });

    let window_us = i64::from(app.shared.config.play.key_debounce_ms) * US_PER_MS;
    let recorded: Vec<(i64, bool)> = state.session.recorded_events().iter().map(|event| (event.t, event.press)).collect();
    assert_eq!(recorded, [(note_us, true), (note_us + window_us, false)], "the release is played where the window ended, not where the frame noticed");
}

/// KEY DEBOUNCE at 0 plays every change of a key the instant it comes, the bounce of a switch
/// included.
#[test]
fn no_key_debounce_plays_every_change_where_it_came() {
    let mut config = Config::default();
    config.play.key_debounce_ms = 0;
    let mut app = app_on("input-time-no-debounce", config);
    let mut state = run_of(CHART, SessionOptions::default());
    let note_us = note_times(&state)[0];
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    let pressed = a_while_ago();
    song_reads_at(&mut app, pressed, note_us);
    for (at, down) in [(pressed, true), (pressed + BOUNCE_OPEN_AFTER, false), (pressed + BOUNCE_CLOSED_AFTER, true)] {
        let key = KeyInput { code: lane_key, pressed: down, released: !down, text: None };
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: at, dt: 0.0 }, key);
    }
    let us = |by: Duration| note_us + by.as_micros() as i64;
    let recorded: Vec<(i64, bool)> = state.session.recorded_events().iter().map(|event| (event.t, event.press)).collect();
    assert_eq!(recorded, [(note_us, true), (us(BOUNCE_OPEN_AFTER), false), (us(BOUNCE_CLOSED_AFTER), true)]);
}

/// How soon after a release a switch bouncing as it opens reads closed again: inside one of LR2's
/// polls, so it is the same release's bounce rather than a press.
const BOUNCE_BACK_AFTER: Duration = Duration::from_millis(1);

/// What KEY DEBOUNCE does not do, as LR2's, the reference's and the controller's own debounce do
/// not: a key held down past the window that bounces open is let go of the instant it does. Only a
/// release inside the window after the press waits, so the release is played at once, and the press
/// that comes back within one poll of it is the bounce and is not played. A long note held that way
/// is let go of; the decision to keep it so is
/// `docs/acknowledge/2026-10-15-adr-keyboard-input-timing.md`.
#[test]
fn a_key_held_past_the_window_that_bounces_open_is_let_go_of_at_once() {
    let mut app = app_on("input-time-held-bounce", Config::default());
    let mut state = run_of(LONG_NOTE_CHART, SessionOptions::default());
    let head_us = note_times(&state)[0];
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    let pressed = a_while_ago();
    song_reads_at(&mut app, pressed, head_us);
    let bounced = pressed + Duration::from_millis(u64::from(app.shared.config.play.key_debounce_ms)) + BOUNCE_OPEN_AFTER;
    for (at, down) in [(pressed, true), (bounced, false), (bounced + BOUNCE_BACK_AFTER, true)] {
        let key = KeyInput { code: lane_key, pressed: down, released: !down, text: None };
        state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: at, dt: 0.0 }, key);
    }
    state.update(&mut FrameCtx { shared: &mut app.shared, now: bounced + HANDLED_LATE_BY, dt: 0.0 });
    let presses: Vec<bool> = state.session.recorded_events().iter().map(|event| event.press).collect();
    assert_eq!(presses, [true, false], "the release is played and the press back within a poll of it is not");
}

/// A one-measure seven-key chart at 200 BPM whose first key holds four thirty-second notes in a
/// row, 37.5 ms apart.
const RAPID_CHART: &str = "#PLAYER 1\n#TITLE t\n#BPM 200\n#WAV01 a.wav\n#00111:0101010100000000000000000000000000000000000000000000000000000000\n";

/// How long each of the rapid presses is held: more than KEY DEBOUNCE, less than the gap to the
/// next note, so each next press comes less than the window after the release before it.
const RAPID_HELD_FOR: Duration = Duration::from_millis(30);

/// The same lane hit four times in thirty-second notes at 200 BPM is played where every press came,
/// dead on each note: KEY DEBOUNCE never makes a press late, though each press follows the release
/// before it by less than the window.
#[test]
fn a_lane_hit_in_quick_succession_is_played_where_every_press_came() {
    let mut app = app_on("input-time-rapid", Config::default());
    let mut state = run_of(RAPID_CHART, SessionOptions::default());
    let times = note_times(&state);
    assert_eq!(times.len(), 4, "the chart holds four notes: {times:?}");
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    let first = a_while_ago();
    song_reads_at(&mut app, first, times[0]);
    let held_us = RAPID_HELD_FOR.as_micros() as i64;
    assert!(
        times.windows(2).all(|pair| pair[1] - pair[0] - held_us < i64::from(app.shared.config.play.key_debounce_ms) * US_PER_MS),
        "each press follows a release by less than the window"
    );

    for note_us in &times {
        let pressed = first + Duration::from_micros((note_us - times[0]) as u64);
        for (at, down) in [(pressed, true), (pressed + RAPID_HELD_FOR, false)] {
            let key = KeyInput { code: lane_key, pressed: down, released: !down, text: None };
            state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: at, dt: 0.0 }, key);
        }
    }

    let presses: Vec<i64> = state.session.recorded_events().iter().filter(|event| event.press).map(|event| event.t).collect();
    assert_eq!(presses, times, "every press is played dead on its note");
    assert_eq!(state.session.judge().counts[rbms_judge::Judge::PerfectGreat as usize], 4, "and judged so");
}
