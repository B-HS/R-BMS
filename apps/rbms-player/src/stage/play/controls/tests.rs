//! The reference's held controls, read a frame at a time from what the keyboard holds.

use std::time::{Duration, Instant};

use rbms_config::FixHiSpeed;
use rbms_judge::matcher::ScratchDir;
use rbms_model::Mode;
use rbms_play::{PlaySession, SessionOptions};
use rbms_store::SCORE_LN_MODE_FROM_CHART;
use winit::keyboard::KeyCode;

use super::*;
use crate::stage::select::tests::{press, release};
use crate::{App, Config, LaunchOptions};

/// The keys START and SELECT ship on.
const START_KEY: KeyCode = KeyCode::KeyA;
const SELECT_KEY: KeyCode = KeyCode::KeyW;

/// The lanes of a seven-key chart the tests press: its first two keys and its turntable.
const FIRST_KEY_LANE: usize = 0;
const SECOND_KEY_LANE: usize = 1;
const TURNTABLE_LANE: usize = 7;

/// The key the tests bind the turntable's second direction to, which ships unbound.
const TURNTABLE_BACK_KEY: KeyCode = KeyCode::ControlLeft;

/// One frame, and a little more than the time a held turntable waits between steps.
const FRAME: Duration = Duration::from_millis(16);
const PAST_A_STEP: Duration = Duration::from_millis(60);

/// How close two shares of the field have to be to count as the same.
const SHARE_TOLERANCE: f32 = 1e-6;

/// An app set to seven keys played by hand, with the shipped key layout in force.
fn app(tag: &str) -> App {
    let dir = std::env::temp_dir().join(format!("rbms-play-controls-{tag}-{}", std::process::id()));
    let mut config = Config::default();
    config.play.autoplay = false;
    config.play.fix_hispeed = FixHiSpeed::Off;
    config.play.enable_cover = false;
    let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = vec![(TURNTABLE_BACK_KEY, TURNTABLE_LANE)];
    app.shared.replay = None;
    app
}

fn state() -> PlayState {
    let src = rbms_parser::parse_with(b"#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#00111:0101\n#00118:01\n", Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "controls.bms"));
    PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// The key that plays `lane` in `dir`.
fn lane_key(app: &App, lane: usize, dir: ScratchDir) -> KeyCode {
    let keys = match dir {
        ScratchDir::Forward => &app.shared.active_keys,
        ScratchDir::Backward => &app.shared.active_reverse_keys,
    };
    keys.iter().find(|(_, of)| *of == lane).map(|(code, _)| *code).expect("the lane has a key")
}

/// Hold `code` down, or let it go, as far as the held-key set knows.
fn hold(app: &mut App, code: KeyCode, down: bool) {
    app.shared.note_key(&if down { press(code) } else { release(code) });
}

/// Press and let go of `code` over two frames of the controls, starting at `at`, and answer the
/// time after them.
fn tap(state: &mut PlayState, app: &mut App, code: KeyCode, at: Instant) -> Instant {
    hold(app, code, true);
    state.held_controls(&mut app.shared, at);
    hold(app, code, false);
    state.held_controls(&mut app.shared, at + FRAME);
    at + FRAME * 2
}

/// START and a key moves the scroll speed once a press: the even keys slower, the odd ones faster,
/// and a key that was already down when START went down does nothing.
#[test]
fn start_and_a_key_steps_the_scroll_speed_once_a_press() {
    let mut app = app("speed");
    let mut state = state();
    let (slower, faster) = (lane_key(&app, FIRST_KEY_LANE, ScratchDir::Forward), lane_key(&app, SECOND_KEY_LANE, ScratchDir::Forward));
    let before = app.shared.config.play.hispeed;
    let step = app.shared.config.play.hispeed_step;
    let mut now = Instant::now();

    hold(&mut app, faster, true);
    hold(&mut app, START_KEY, true);
    for _ in 0..3 {
        state.held_controls(&mut app.shared, now);
        now += FRAME;
    }
    assert_eq!(app.shared.config.play.hispeed, before, "a key that was down before START moved the speed");
    hold(&mut app, faster, false);
    state.held_controls(&mut app.shared, now);

    now = tap(&mut state, &mut app, faster, now + FRAME);
    assert!((app.shared.config.play.hispeed - (before + step)).abs() < 1e-9, "START and the second key did not speed the scroll up");
    hold(&mut app, slower, true);
    for _ in 0..3 {
        state.held_controls(&mut app.shared, now);
        now += FRAME;
    }
    assert!((app.shared.config.play.hispeed - before).abs() < 1e-9, "a key held with START moved the speed more than once");

    hold(&mut app, slower, false);
    hold(&mut app, START_KEY, false);
    state.held_controls(&mut app.shared, now);
    tap(&mut state, &mut app, faster, now + FRAME);
    assert!((app.shared.config.play.hispeed - before).abs() < 1e-9, "a key moved the speed with START up");
}

/// START pressed twice within half a second switches the lane cover; two presses further apart do
/// not, and the second of those is the first of a new pair.
#[test]
fn start_pressed_twice_quickly_switches_the_lane_cover() {
    let mut app = app("double");
    let mut state = state();
    assert!(!app.shared.config.play.enable_cover);
    let first = Instant::now();
    let after = tap(&mut state, &mut app, START_KEY, first);
    assert!(!app.shared.config.play.enable_cover, "one press switched the lane cover");
    tap(&mut state, &mut app, START_KEY, after + Duration::from_millis(100));
    assert!(app.shared.config.play.enable_cover, "the second press did not switch the lane cover on");

    let much_later = after + Duration::from_secs(5);
    let after = tap(&mut state, &mut app, START_KEY, much_later);
    assert!(app.shared.config.play.enable_cover, "the press after a pair counted as the third of it");
    tap(&mut state, &mut app, START_KEY, after + START_DOUBLE_TAP);
    assert!(app.shared.config.play.enable_cover, "two presses half a second apart switched the lane cover");
    let third = after + START_DOUBLE_TAP + FRAME * 2;
    tap(&mut state, &mut app, START_KEY, third + Duration::from_millis(100));
    assert!(!app.shared.config.play.enable_cover, "the press that came too late was not the first of a new pair");
}

/// START and the turntable moves the lane cover a fine step every fifty milliseconds, one way for
/// each of its keys, and never by the coarser step: the reference forgets since when the turntable
/// was held on every frame one of its two keys is up.
#[test]
fn start_and_the_turntable_moves_the_cover_a_fine_step_at_a_time() {
    let mut app = app("cover");
    let mut state = state();
    let fine = app.shared.config.play.lanecover_step_fine;
    let (up, down) = (lane_key(&app, TURNTABLE_LANE, ScratchDir::Forward), lane_key(&app, TURNTABLE_LANE, ScratchDir::Backward));
    let mut now = Instant::now();
    hold(&mut app, START_KEY, true);
    state.held_controls(&mut app.shared, now);

    hold(&mut app, up, true);
    let steps = 14;
    for _ in 0..steps {
        now += PAST_A_STEP;
        state.held_controls(&mut app.shared, now);
    }
    assert!(PAST_A_STEP * steps > COARSE_STEP_AFTER, "the turntable is held for longer than the coarse step waits");
    let raised = app.shared.config.play.cover;
    assert!((raised - fine * steps as f32).abs() < SHARE_TOLERANCE, "the cover moved {raised} in {steps} fine steps of {fine}");

    now += FRAME;
    state.held_controls(&mut app.shared, now);
    assert!((app.shared.config.play.cover - raised).abs() < SHARE_TOLERANCE, "the cover moved again before fifty milliseconds were up");

    hold(&mut app, up, false);
    hold(&mut app, down, true);
    now += PAST_A_STEP;
    state.held_controls(&mut app.shared, now);
    assert!((app.shared.config.play.cover - (raised - fine)).abs() < SHARE_TOLERANCE, "the turntable's other key did not move the cover back");
}

/// The cover the turntable and the wheel move is the lane cover while it is on or nothing else is,
/// then the lift, then the hidden cover, and START with SELECT chooses between those two when both
/// are on. The lift and the hidden cover move the other way round.
#[test]
fn the_cover_that_moves_follows_which_covers_are_on() {
    const BY: f32 = 0.1;
    let mut app = app("which");
    let mut state = state();
    state.move_cover(&mut app.shared, BY);
    assert!((app.shared.config.play.cover - BY).abs() < SHARE_TOLERANCE, "with nothing on, the lane cover is what moves");

    app.shared.config.play.enable_lift = true;
    app.shared.config.play.lift = 0.5;
    state.move_cover(&mut app.shared, BY);
    assert!((app.shared.config.play.lift - (0.5 - BY)).abs() < SHARE_TOLERANCE, "with the lift on and the lane cover off, the lift moves, the other way");

    app.shared.config.play.enable_hidden = true;
    app.shared.config.play.hidden = 0.5;
    state.move_cover(&mut app.shared, BY);
    assert!((app.shared.config.play.lift - (0.5 - BY * 2.0)).abs() < SHARE_TOLERANCE, "with both on, the lift is the one chosen to begin with");

    let now = Instant::now();
    hold(&mut app, START_KEY, true);
    hold(&mut app, SELECT_KEY, true);
    state.held_controls(&mut app.shared, now);
    state.held_controls(&mut app.shared, now + FRAME);
    hold(&mut app, START_KEY, false);
    hold(&mut app, SELECT_KEY, false);
    state.held_controls(&mut app.shared, now + FRAME * 2);
    state.move_cover(&mut app.shared, BY);
    assert!((app.shared.config.play.hidden - (0.5 - BY)).abs() < SHARE_TOLERANCE, "START with SELECT did not hand the turntable to the hidden cover");

    app.shared.config.play.enable_cover = true;
    let cover = app.shared.config.play.cover;
    state.wheel_cover(&mut app.shared, 2.0);
    assert!((app.shared.config.play.cover - (cover - 2.0 * WHEEL_STEP)).abs() < SHARE_TOLERANCE, "the wheel did not move the lane cover while it is on");
}

/// SELECT and a key moves the travel time a pinned speed holds by a millisecond, and the speed with
/// it; a run that pins its speed to no tempo has nothing to move.
#[test]
fn select_and_a_key_moves_the_travel_time_a_pinned_speed_holds() {
    let mut app = app("duration");
    let mut state = state();
    let longer = lane_key(&app, SECOND_KEY_LANE, ScratchDir::Forward);
    let unpinned = app.shared.config.play.hispeed;
    let mut now = Instant::now();
    hold(&mut app, SELECT_KEY, true);
    state.held_controls(&mut app.shared, now);
    state.held_controls(&mut app.shared, now + FRAME);
    now = tap(&mut state, &mut app, longer, now + FRAME * 2);
    assert_eq!(app.shared.config.play.hispeed, unpinned, "an unpinned speed moved");

    app.shared.config.play.fix_hispeed = FixHiSpeed::StartBpm;
    let mut pinned = state_pinned();
    let held = pinned.run_speed(&app.shared).fixed.expect("the run pins its speed").green;
    pinned.held_controls(&mut app.shared, now);
    pinned.held_controls(&mut app.shared, now + FRAME);
    tap(&mut pinned, &mut app, longer, now + FRAME * 2);
    let after = pinned.run_speed(&app.shared).fixed.expect("the run still pins its speed").green;
    assert!((after - (held + DURATION_STEP_MS)).abs() < 1e-9, "the travel time went from {held} to {after}");
    assert!(app.shared.config.play.hispeed < unpinned, "a longer travel time did not slow the scroll");
}

/// A fresh run for the pinned half of the test above, so its speed is settled from the settings as
/// they stand then.
fn state_pinned() -> PlayState {
    state()
}

/// A run that plays itself has its speed left alone by START and a key, as in the reference, but
/// START pressed twice still switches the lane cover.
#[test]
fn a_run_that_plays_itself_keeps_its_speed_under_start_and_a_key() {
    let mut app = app("auto");
    app.shared.config.play.autoplay = true;
    let mut state = state();
    let faster = lane_key(&app, SECOND_KEY_LANE, ScratchDir::Forward);
    let before = app.shared.config.play.hispeed;
    let now = Instant::now();
    hold(&mut app, START_KEY, true);
    state.held_controls(&mut app.shared, now);
    state.held_controls(&mut app.shared, now + FRAME);
    let now = tap(&mut state, &mut app, faster, now + FRAME * 2);
    assert_eq!(app.shared.config.play.hispeed, before);

    hold(&mut app, START_KEY, false);
    state.held_controls(&mut app.shared, now);
    tap(&mut state, &mut app, START_KEY, now + FRAME);
    assert!(app.shared.config.play.enable_cover, "the lane cover could not be switched on a run that plays itself");
}
