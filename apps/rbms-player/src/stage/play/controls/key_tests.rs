//! The controls of a run that a key or a controller button can be given outright: each does its
//! job from the key it is given, on the play screen, by the same path a controller takes.

use std::time::Instant;

use rbms_config::FixHiSpeed;
use rbms_model::Mode;
use rbms_play::{PlaySession, SessionOptions};
use rbms_store::SCORE_LN_MODE_FROM_CHART;
use winit::keyboard::KeyCode;

use super::*;
use crate::gamepad::PadEvent;
use crate::keyconfig::{Bindable, Shortcut};
use crate::stage::play::shade_is_changing;
use crate::stage::select::tests::press;
use crate::stage::{FrameCtx, StageHandler};
use crate::{App, Config, LaunchOptions};

/// The keys the tests give the controls that ship with none, none of which plays a lane or is
/// another control's.
const LIFT_KEY: KeyCode = KeyCode::F6;
const HIDDEN_KEY: KeyCode = KeyCode::F7;
const KEEP_GREEN_KEY: KeyCode = KeyCode::F8;
const AUTO_ADJUST_KEY: KeyCode = KeyCode::F9;
const OFFSET_UP_KEY: KeyCode = KeyCode::F10;
const OFFSET_DOWN_KEY: KeyCode = KeyCode::F11;
const GREEN_UP_KEY: KeyCode = KeyCode::Insert;
const GREEN_DOWN_KEY: KeyCode = KeyCode::Home;

/// An app set to seven keys played by hand, with every control that ships unbound given a key.
fn app(tag: &str) -> App {
    let dir = std::env::temp_dir().join(format!("rbms-play-control-keys-{tag}-{}", std::process::id()));
    let mut config = Config::default();
    config.play.autoplay = false;
    config.play.fix_hispeed = FixHiSpeed::Off;
    config.judge.auto_offset = false;
    let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.keyconfig = crate::keyconfig::KeyConfig::default();
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = Vec::new();
    app.shared.replay = None;
    for (action, code) in [
        (ControlAction::LiftToggle, LIFT_KEY),
        (ControlAction::HiddenToggle, HIDDEN_KEY),
        (ControlAction::KeepGreenToggle, KEEP_GREEN_KEY),
        (ControlAction::AutoAdjustToggle, AUTO_ADJUST_KEY),
        (ControlAction::OffsetUp, OFFSET_UP_KEY),
        (ControlAction::OffsetDown, OFFSET_DOWN_KEY),
        (ControlAction::GreenUp, GREEN_UP_KEY),
        (ControlAction::GreenDown, GREEN_DOWN_KEY),
    ] {
        app.shared.keyconfig.set_control(action, code);
    }
    app
}

fn run(options: SessionOptions) -> PlayState {
    let src = rbms_parser::parse_with(b"#PLAYER 1\n#BPM 150\n#WAV01 a.wav\n#00111:0101\n#00118:01\n", Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "keys.bms"));
    PlayState::new(PlaySession::new(model, options), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

fn state() -> PlayState {
    run(SessionOptions::default())
}

/// One key going down on the play screen.
fn key(state: &mut PlayState, app: &mut App, code: KeyCode) {
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, press(code));
}

/// Each switch flips the row it names from the key it was given, and flips it back on the next
/// press: the lane cover on the key it ships on, and the lift and the hidden cover on keys of the
/// player's choosing. Switching a shade off leaves the amount it had.
#[test]
fn each_shade_switches_on_and_off_from_its_own_key() {
    let mut app = app("switches");
    let mut state = state();
    app.shared.config.play.cover = 0.3;
    let shades = |app: &App| (app.shared.config.play.enable_cover, app.shared.config.play.enable_lift, app.shared.config.play.enable_hidden);
    assert_eq!(shades(&app), (true, false, false), "the shipped rows are not the ones this test starts from");

    key(&mut state, &mut app, KeyCode::Backslash);
    assert_eq!(shades(&app), (false, false, false), "the lane cover did not go off on the key it ships on");
    assert_eq!(app.shared.effective_cover(), 0.0);
    key(&mut state, &mut app, KeyCode::Backslash);
    assert_eq!(shades(&app), (true, false, false));
    assert!((app.shared.effective_cover() - 0.3).abs() < 1e-6, "the cover came back at another height");

    key(&mut state, &mut app, LIFT_KEY);
    assert_eq!(shades(&app), (true, true, false));
    key(&mut state, &mut app, HIDDEN_KEY);
    assert_eq!(shades(&app), (true, true, true));
    key(&mut state, &mut app, LIFT_KEY);
    key(&mut state, &mut app, HIDDEN_KEY);
    assert_eq!(shades(&app), (true, false, false));
}

/// A key the player moved a control to is the one that is read, and the key it shipped on is not.
#[test]
fn a_control_moved_to_another_key_is_read_from_there() {
    let mut app = app("moved");
    let mut state = state();
    app.shared.keyconfig.bind(Mode::BEAT_7K, Bindable::Control(ControlAction::CoverToggle), Some(KeyCode::KeyP));
    key(&mut state, &mut app, KeyCode::Backslash);
    assert!(app.shared.config.play.enable_cover, "the key the switch shipped on still switches the cover");
    key(&mut state, &mut app, KeyCode::KeyP);
    assert!(!app.shared.config.play.enable_cover);

    app.shared.keyconfig.bind(Mode::BEAT_7K, Bindable::Control(ControlAction::CoverUp), Some(KeyCode::KeyU));
    let before = app.shared.config.play.cover;
    key(&mut state, &mut app, KeyCode::ArrowRight);
    assert_eq!(app.shared.config.play.cover, before, "the key the cover shipped on still moves it");
    key(&mut state, &mut app, KeyCode::KeyU);
    assert!(app.shared.config.play.cover > before, "the amount going up is the cover coming down over more of the field");
}

/// The lane cover switch gives way to a lane on its key, so a key config that already played a
/// column on the key the switch ships on keeps playing it.
#[test]
fn the_cover_switch_gives_way_to_a_lane_on_its_key() {
    let mut app = app("yields");
    let mut state = state();
    app.shared.keyconfig.set_lane(Mode::BEAT_7K, 3, KeyCode::Backslash);
    app.shared.active_keys = app.shared.keyconfig.lane_keys(Mode::BEAT_7K);
    assert_eq!(app.shared.control_for(KeyCode::Backslash), None);
    key(&mut state, &mut app, KeyCode::Backslash);
    assert!(app.shared.config.play.enable_cover, "a lane key switched the cover off");
}

/// The rows a run is judged by move from their keys: the offset a millisecond a press inside the
/// range its row holds, and the row that lets it adjust itself on and off.
#[test]
fn the_judge_rows_move_from_their_keys() {
    let mut app = app("judge");
    let mut state = state();
    app.shared.config.judge.offset_ms = 0;
    key(&mut state, &mut app, OFFSET_UP_KEY);
    key(&mut state, &mut app, OFFSET_UP_KEY);
    assert_eq!(app.shared.config.judge.offset_ms, 2);
    for _ in 0..5 {
        key(&mut state, &mut app, OFFSET_DOWN_KEY);
    }
    assert_eq!(app.shared.config.judge.offset_ms, -3);
    app.shared.config.judge.offset_ms = rbms_config::JUDGE_OFFSET_MAX_MS;
    key(&mut state, &mut app, OFFSET_UP_KEY);
    assert_eq!(app.shared.config.judge.offset_ms, rbms_config::JUDGE_OFFSET_MAX_MS, "the key took the offset out of its range");

    key(&mut state, &mut app, AUTO_ADJUST_KEY);
    assert!(app.shared.config.judge.auto_offset);
    key(&mut state, &mut app, AUTO_ADJUST_KEY);
    assert!(!app.shared.config.judge.auto_offset);
}

/// A replay is judged at the offsets it was recorded with and an autoplay hits nothing early or
/// late, so the offset keys leave the player's own offset alone on both.
#[test]
fn the_offset_keys_move_nothing_on_a_run_the_player_is_not_playing() {
    let mut app = app("not-by-hand");
    let mut state = state();
    app.shared.config.judge.offset_ms = 7;
    app.shared.config.play.autoplay = true;
    key(&mut state, &mut app, OFFSET_UP_KEY);
    key(&mut state, &mut app, OFFSET_DOWN_KEY);
    key(&mut state, &mut app, OFFSET_DOWN_KEY);
    assert_eq!(app.shared.config.judge.offset_ms, 7);
}

/// The row that keeps the green number switches from its key, and with it on the two green number
/// keys move the travel time that is held a millisecond a press while the speed follows.
#[test]
fn the_green_number_rows_move_from_their_keys() {
    let mut app = app("green");
    let mut state = state();
    assert!(!app.shared.config.play.keep_green_number);
    let free = (app.shared.config.play.duration_ms, app.shared.config.play.hispeed);
    key(&mut state, &mut app, GREEN_UP_KEY);
    assert_eq!((app.shared.config.play.duration_ms, app.shared.config.play.hispeed), free, "a run with a free speed has no travel time to move");

    key(&mut state, &mut app, KEEP_GREEN_KEY);
    assert!(app.shared.config.play.keep_green_number);
    let held = app.shared.config.play.duration_ms;
    key(&mut state, &mut app, GREEN_UP_KEY);
    key(&mut state, &mut app, GREEN_UP_KEY);
    assert_eq!(app.shared.config.play.duration_ms, held + 2);
    let slower = app.shared.config.play.hispeed;
    key(&mut state, &mut app, GREEN_DOWN_KEY);
    key(&mut state, &mut app, GREEN_DOWN_KEY);
    key(&mut state, &mut app, GREEN_DOWN_KEY);
    assert_eq!(app.shared.config.play.duration_ms, held - 1);
    assert!(app.shared.config.play.hispeed > slower, "a shorter travel time is a faster scroll");

    key(&mut state, &mut app, KEEP_GREEN_KEY);
    assert!(!app.shared.config.play.keep_green_number);
}

/// On a run that keeps its green number, switching the lane cover from its key keeps the number and
/// moves the speed, as moving the cover does.
#[test]
fn switching_the_cover_from_its_key_keeps_a_held_green_number() {
    let mut app = app("green-cover");
    let mut state = state();
    app.shared.config.play.keep_green_number = true;
    app.shared.config.play.cover = 0.4;
    key(&mut state, &mut app, GREEN_UP_KEY);
    let (number, covered) = (app.shared.config.play.duration_ms, app.shared.config.play.hispeed);
    key(&mut state, &mut app, KeyCode::Backslash);
    assert_eq!(app.shared.config.play.duration_ms, number, "the number that is held moved with the cover");
    assert!(app.shared.config.play.hispeed > covered, "an uncovered lane needs a faster scroll for the same travel time");
}

/// A controller button bound to a control takes the same path a key does.
#[test]
fn a_controller_button_does_what_the_key_does() {
    let mut app = app("pad");
    let mut state = state();
    let pad = |state: &mut PlayState, app: &mut App, action: ControlAction| {
        state.handle_pad(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, PadEvent::Control(action));
    };
    pad(&mut state, &mut app, ControlAction::CoverToggle);
    assert!(!app.shared.config.play.enable_cover);
    pad(&mut state, &mut app, ControlAction::LiftToggle);
    assert!(app.shared.config.play.enable_lift);
    pad(&mut state, &mut app, ControlAction::OffsetUp);
    assert_eq!(app.shared.config.judge.offset_ms, 1);
}

/// The transport of a replay under analysis is read from the keys the key configuration gives it:
/// the shipped ones, and after a move the new one and not the old. A run that is not under analysis
/// has no use for them, and they do nothing there.
#[test]
fn the_replay_transport_is_read_from_its_own_keys() {
    let mut app = app("replay");
    let mut state = state();
    key(&mut state, &mut app, KeyCode::Space);
    assert!(!state.session.analysis_paused(), "a run that is not under analysis paused");

    state.session.set_analysis_enabled(true);
    key(&mut state, &mut app, KeyCode::Space);
    assert!(state.session.analysis_paused());
    key(&mut state, &mut app, KeyCode::Space);
    assert!(!state.session.analysis_paused());
    let rate = state.session.analysis_rate();
    key(&mut state, &mut app, KeyCode::Equal);
    assert!(state.session.analysis_rate() > rate);
    key(&mut state, &mut app, KeyCode::Minus);
    assert!((state.session.analysis_rate() - rate).abs() < 1e-9);

    app.shared.keyconfig.bind(Mode::BEAT_7K, Bindable::Shortcut(Shortcut::ReplayPause), Some(KeyCode::KeyP));
    key(&mut state, &mut app, KeyCode::Space);
    assert!(!state.session.analysis_paused(), "the key the pause shipped on still pauses");
    key(&mut state, &mut app, KeyCode::KeyP);
    assert!(state.session.analysis_paused());

    state.handle_pad(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, PadEvent::Shortcut(Shortcut::ReplayPause));
    assert!(!state.session.analysis_paused(), "a controller button bound to the pause did not resume");
}

/// How long after a shade key these tests look again, in microseconds of the skin's clock: inside
/// the moment the figures stay up, and past it.
const SOON_US: i64 = 200_000;
const MUCH_LATER_US: i64 = 5_000_000;

/// A shade moved from a key of its own shows a skin the figures it shows while a shade is being
/// changed, for a moment, and a control that moves no shade does not.
#[test]
fn a_shade_key_shows_the_figures_of_the_shades_for_a_moment() {
    let mut app = app("shade-figures");
    let mut state = state();
    app.shared.config.play.enable_cover = false;
    assert!(state.shade_moved_us.is_none());
    key(&mut state, &mut app, AUTO_ADJUST_KEY);
    key(&mut state, &mut app, KeyCode::ArrowUp);
    assert!(state.shade_moved_us.is_none(), "a control that moves no shade put the figures of the shades up");

    key(&mut state, &mut app, KeyCode::ArrowRight);
    let moved_us = state.shade_moved_us.expect("the cover key was read as a move of a shade");
    assert!(shade_is_changing(Some(moved_us), moved_us), "the figures are up at once");
    assert!(shade_is_changing(Some(moved_us), moved_us + SOON_US));
    assert!(!shade_is_changing(Some(moved_us), moved_us + MUCH_LATER_US), "the figures stayed up long after the key");
    assert!(shade_is_changing(Some(moved_us), moved_us - SOON_US), "a clock put back a little is still close to the move");
    assert!(!shade_is_changing(Some(moved_us), moved_us - MUCH_LATER_US), "a clock that started again long before is not");
    assert!(!shade_is_changing(None, MUCH_LATER_US));
}
