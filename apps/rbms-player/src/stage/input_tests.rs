//! What the common entry points make of a key or a controller element before the screen sees it:
//! two keys held together, the key given to ESC, and ESC on a controller, on a running chart and in
//! the key editor.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use rbms_config::{PLAY_ESCAPE_DOUBLE_MS, PLAY_ESCAPE_HOLD_MS, PlayEscape};
use rbms_model::Mode;
use rbms_play::{PlaySession, SessionOptions};
use winit::keyboard::KeyCode;

use super::*;
use crate::gamepad::{PadBinding, PadChange, PadConfig, PadMapper, PadMark, PadSnapshot};
use crate::keyconfig::{Bindable, ChordEvent, KeyBind, Shortcut, ShortcutScreen};
use crate::{App, Config, LaunchOptions};

/// A one-measure 7-key chart, enough for a run with real lanes.
const CHART: &str = "#PLAYER 1\n#TITLE t\n#BPM 120\n#WAV01 a.wav\n#00111:0101\n";

/// Two keys no shipped layout gives anything, and the key the first lane ships on.
const F7: KeyCode = KeyCode::F7;
const F10: KeyCode = KeyCode::F10;
const LANE_KEY: KeyCode = KeyCode::KeyZ;

/// The button ESC is given on the controller.
const ESC_BUTTON: u32 = 5;

/// Far enough apart that KEY DEBOUNCE takes every press and release.
const STEP: Duration = Duration::from_millis(100);

fn fresh_app() -> App {
    let dir = std::env::temp_dir().join(format!("rbms-stage-input-tests-{}-{:?}", std::process::id(), std::thread::current().id()));
    let mut config = Config::default();
    config.play.autoplay = false;
    let mut app = App::new(String::new(), config, LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.replay = None;
    app
}

fn bind(app: &mut App, target: Bindable, bind: KeyBind) {
    app.shared.keyconfig.bind_keys(Mode::BEAT_7K, target, bind);
}

fn chord(first: KeyCode, second: KeyCode) -> KeyBind {
    KeyBind::chord(first, second).expect("two keys")
}

/// The app on a running chart.
fn running(app: &mut App) {
    let src = rbms_parser::parse_with(CHART.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&src, "t.bms");
    let model = rbms_chart::to_model(&src, mode);
    let state = PlayState::new(PlaySession::new(model, SessionOptions::default()), HashMap::new(), 0, rbms_store::SCORE_LN_MODE_FROM_CHART.to_string());
    app.stage = Stage::Play(Box::new(state));
}

fn key(app: &mut App, code: KeyCode, pressed: bool, now: Instant) -> Transition {
    let input = KeyInput { code, pressed, released: !pressed, text: None };
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, input)
}

/// What the running chart recorded of the first lane, press by press.
fn lane_presses(app: &App) -> Vec<bool> {
    match &app.stage {
        Stage::Play(play) => play.session.recorded_events().iter().filter(|event| event.lane == 0).map(|event| event.press).collect(),
        _ => Vec::new(),
    }
}

fn leaves(transition: &Transition) -> bool {
    !matches!(transition, Transition::Stay)
}

/// A control on two keys runs once when the second goes down, in either order; that press does
/// not run what its key alone is bound to, and the lane on it is still played.
#[test]
fn a_chord_runs_its_control_once_in_either_order_and_still_plays_the_lane_of_its_key() {
    let mut app = fresh_app();
    bind(&mut app, Bindable::Control(ControlAction::HiSpeedUp), chord(F7, LANE_KEY));
    bind(&mut app, Bindable::Control(ControlAction::HiSpeedDown), KeyBind::Single(F7));
    app.shared.config.play.hispeed = 2.0;
    running(&mut app);
    let now = Instant::now();
    let speed = |app: &App| app.shared.config.play.hispeed;

    key(&mut app, LANE_KEY, true, now);
    assert_eq!(speed(&app), 2.0, "one key of the chord is not the chord");
    key(&mut app, F7, true, now + STEP);
    let up = speed(&app);
    assert!(up > 2.0, "the chord ran HI-SPEED UP and not F7's own HI-SPEED DOWN beside it: {up}");
    key(&mut app, F7, false, now + STEP * 2);
    key(&mut app, LANE_KEY, false, now + STEP * 3);
    assert_eq!(speed(&app), up, "letting go runs nothing");

    key(&mut app, F7, true, now + STEP * 4);
    let down = speed(&app);
    assert!(down < up, "F7 pressed first runs what it alone is bound to");
    key(&mut app, LANE_KEY, true, now + STEP * 5);
    assert!(speed(&app) > down, "the other order makes the chord too");
    key(&mut app, LANE_KEY, false, now + STEP * 6);
    assert_eq!(lane_presses(&app), [true, false, true, false], "the lane key was played both times, the press that made the chord too");
}

/// A control on two keys is held while both are, and repeats as long; a key that is one of two held
/// together is the chord's, and is neither START nor a key of the key table while it is.
#[test]
fn a_chord_is_held_while_both_keys_are_and_takes_its_keys_from_what_they_hold_alone() {
    let mut app = fresh_app();
    bind(&mut app, Bindable::Control(ControlAction::HiSpeedUp), chord(F7, F10));
    bind(&mut app, Bindable::Shortcut(Shortcut::Folders), chord(KeyCode::KeyA, LANE_KEY));
    let down = |app: &mut App, code: KeyCode| app.shared.note_key(&KeyInput { code, pressed: true, released: false, text: None });
    let up = |app: &mut App, code: KeyCode| app.shared.note_key(&KeyInput { code, pressed: false, released: true, text: None });

    down(&mut app, F7);
    assert!(!app.shared.control_repeating(ControlAction::HiSpeedUp));
    down(&mut app, F10);
    assert!(app.shared.control_repeating(ControlAction::HiSpeedUp), "both keys down");
    up(&mut app, F7);
    assert!(!app.shared.control_repeating(ControlAction::HiSpeedUp), "either let go stops it");

    down(&mut app, KeyCode::KeyA);
    assert!(app.shared.start_pressed());
    down(&mut app, LANE_KEY);
    assert!(!app.shared.start_pressed(), "the chord the player bound wins over START held with a key");
    assert!(!app.shared.keyboard_index_pressed(0), "and its lane key is no key of the table while it is held");
    assert!(app.shared.shortcut_held(Shortcut::Folders));
    up(&mut app, LANE_KEY);
    assert!(app.shared.start_pressed(), "START is START again once the chord is broken");
}

/// The press that makes a chord reads the chord's shortcut on the screen that reads it and never
/// the key's own; its release is the chord's key going up, for a shortcut held and let go.
#[test]
fn the_press_that_makes_a_chord_is_the_chords_shortcut_and_not_its_keys_own() {
    let mut app = fresh_app();
    bind(&mut app, Bindable::Shortcut(Shortcut::Folders), chord(F7, F10));
    bind(&mut app, Bindable::Shortcut(Shortcut::Tables), KeyBind::Single(F10));
    bind(&mut app, Bindable::Shortcut(Shortcut::OptionsHold), chord(F7, KeyCode::F11));
    let note = |app: &mut App, code: KeyCode, pressed: bool| app.shared.note_key(&KeyInput { code, pressed, released: !pressed, text: None });

    note(&mut app, F10, true);
    assert_eq!(app.shared.shortcut_for(ShortcutScreen::Browser, F10), Some(Shortcut::Tables), "alone, the key is its own");
    note(&mut app, F10, false);
    note(&mut app, F7, true);
    assert_eq!(app.shared.shortcut_for(ShortcutScreen::Browser, F7), None);
    note(&mut app, F10, true);
    assert_eq!(app.shared.shortcut_for(ShortcutScreen::Browser, F10), Some(Shortcut::Folders));
    assert!(app.shared.is_shortcut(Shortcut::Folders, F10) && !app.shared.is_shortcut(Shortcut::Tables, F10));
    assert_eq!(app.shared.shortcut_for(ShortcutScreen::Result, F10), None, "a screen reads only its own shortcuts");

    note(&mut app, KeyCode::F11, true);
    assert!(app.shared.is_shortcut(Shortcut::OptionsHold, KeyCode::F11));
    note(&mut app, F7, false);
    assert!(app.shared.is_shortcut(Shortcut::OptionsHold, F7), "the release that breaks a held chord is its key going up");
}

/// Outside a run the press that makes a chord is the chord's alone: its lane is played only in a
/// run, where a note is never eaten by a binding.
#[test]
fn the_lane_of_the_press_that_makes_a_chord_is_played_only_in_a_run() {
    let mut app = fresh_app();
    app.shared.keyconfig.held.press(LANE_KEY);
    for (keeps_lanes, lane) in [(true, Some((0, rbms_play::ScratchDir::Forward))), (false, None)] {
        app.shared.keyconfig.held.note_chord_event(ChordEvent {
            code: LANE_KEY,
            pressed: true,
            chords: vec![Bindable::Shortcut(Shortcut::Folders)],
            keeps_lanes,
        });
        assert_eq!(app.shared.lane_input_for(LANE_KEY), lane, "keeps lanes: {keeps_lanes}");
    }
    app.shared.keyconfig.held.forget_chord_event();
    assert!(app.shared.lane_input_for(LANE_KEY).is_some(), "a key that made nothing is its lane");
}

/// A window that loses focus lets go of every key through the path of a real key-up, so a chord
/// held then is broken: ESC held on two keys comes up, and a run held to abandon stays.
#[test]
fn losing_focus_breaks_a_chord_and_lets_esc_come_up() {
    let mut app = fresh_app();
    app.shared.config.play.play_escape = PlayEscape::Hold;
    bind(&mut app, Bindable::Control(ControlAction::Escape), chord(F7, F10));
    bind(&mut app, Bindable::Control(ControlAction::HiSpeedUp), chord(KeyCode::F11, KeyCode::F12));
    running(&mut app);
    let now = Instant::now();
    key(&mut app, F7, true, now);
    key(&mut app, F10, true, now);
    app.shared.note_key(&KeyInput { code: KeyCode::F11, pressed: true, released: false, text: None });
    app.shared.note_key(&KeyInput { code: KeyCode::F12, pressed: true, released: false, text: None });
    assert!(app.shared.control_repeating(ControlAction::HiSpeedUp));

    let released = app.stage.release_keys_lost_with_focus(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
    assert!(released.iter().all(|transition| !leaves(transition)));
    app.shared.release_held_inputs();
    assert!(!app.shared.control_repeating(ControlAction::HiSpeedUp), "a chord was left held");
    let hold = Duration::from_millis(PLAY_ESCAPE_HOLD_MS);
    let later = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now: now + hold * 2, dt: 0.0 });
    assert!(!leaves(&later), "ESC came up with the focus, so the hold was let go");
}

/// The key given to ESC is Escape on every screen but the key editor, whose own keys are what binds
/// every other: there it stays the key it is.
#[test]
fn the_key_given_to_esc_is_escape_but_in_the_key_editor() {
    let mut app = fresh_app();
    app.shared.keyconfig.set_control(ControlAction::Escape, KeyCode::Backspace);
    running(&mut app);
    assert!(leaves(&key(&mut app, KeyCode::Backspace, true, Instant::now())), "the key left the run as Escape does");

    let mut app = fresh_app();
    app.shared.keyconfig.set_control(ControlAction::Escape, KeyCode::Backspace);
    app.stage = Stage::KeyConfig(KeyConfigState::new());
    assert!(!leaves(&key(&mut app, KeyCode::Backspace, true, Instant::now())), "the editor read the key as Escape");
    assert!(leaves(&key(&mut app, KeyCode::Escape, true, Instant::now())));
}

/// Hand the screen what the controller makes of ESC's button going down and up at each of `at`,
/// through the debounce and with the mark each event carries, and answer what each event did.
fn esc_button(app: &mut App, mapper: &mut PadMapper, presses: &[(i64, bool)], epoch: Instant) -> Vec<Transition> {
    let mut cfg = PadConfig { debounce_ms: 0, ..PadConfig::default() };
    cfg.set_control(ControlAction::Escape, Some(PadBinding::Button(ESC_BUTTON)));
    let changes: Vec<(i64, PadChange)> =
        presses.iter().map(|&(at_us, pressed)| (at_us, PadChange::Button { code: ESC_BUTTON, standard: None, pressed })).collect();
    let last_us = presses.last().map_or(0, |(at_us, _)| *at_us);
    let events = mapper.resolve_changes_marked(&changes, last_us, &cfg, Mode::BEAT_7K, &mut PadSnapshot::default());
    events
        .into_iter()
        .map(|(event, at_us, mark)| {
            let now = epoch + Duration::from_micros(at_us.unsigned_abs());
            app.stage.handle_marked_pad(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, event, mark)
        })
        .collect()
}

const MS_US: i64 = 1_000;

/// ESC on a controller button leaves a run as the Escape key does under every ESCAPE setting: at
/// once, once held long enough and not when let go before, and on a second press in time.
#[test]
fn esc_on_a_controller_does_what_the_escape_key_does_under_every_setting() {
    let epoch = Instant::now();
    let hold_us = i64::try_from(PLAY_ESCAPE_HOLD_MS).expect("a hold in range") * MS_US;
    let double_us = i64::try_from(PLAY_ESCAPE_DOUBLE_MS).expect("a window in range") * MS_US;

    let mut app = fresh_app();
    running(&mut app);
    assert!(esc_button(&mut app, &mut PadMapper::new(), &[(0, true)], epoch).iter().any(leaves), "IMMEDIATE leaves on the press");

    let mut app = fresh_app();
    app.shared.config.play.play_escape = PlayEscape::Hold;
    running(&mut app);
    let mut mapper = PadMapper::new();
    assert!(!esc_button(&mut app, &mut mapper, &[(0, true), (hold_us / 2, false)], epoch).iter().any(leaves));
    let later = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now: epoch + Duration::from_micros((hold_us * 2).unsigned_abs()), dt: 0.0 });
    assert!(!leaves(&later), "HOLD let go early stays");
    let pressed_us = hold_us * 3;
    assert!(!esc_button(&mut app, &mut mapper, &[(pressed_us, true)], epoch).iter().any(leaves));
    let held = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now: epoch + Duration::from_micros((pressed_us + hold_us).unsigned_abs()), dt: 0.0 });
    assert!(leaves(&held), "HOLD held long enough leaves");

    let mut app = fresh_app();
    app.shared.config.play.play_escape = PlayEscape::Double;
    running(&mut app);
    let mut mapper = PadMapper::new();
    assert!(!esc_button(&mut app, &mut mapper, &[(0, true), (MS_US, false)], epoch).iter().any(leaves), "one press arms the pair");
    let second_us = double_us / 2;
    assert!(esc_button(&mut app, &mut mapper, &[(second_us, true)], epoch).iter().any(leaves), "DOUBLE TAP leaves on the second press");
}

/// The key editor waiting for a controller element takes ESC's button as the element; otherwise
/// ESC on the controller leaves it as Escape does.
#[test]
fn the_key_editor_takes_escs_button_while_it_waits_for_an_element() {
    let mut app = fresh_app();
    app.stage = Stage::KeyConfig(KeyConfigState::new());
    let event = crate::gamepad::PadEvent::Control(ControlAction::Escape);
    let left = app.stage.handle_marked_pad(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, event, PadMark::default());
    assert!(leaves(&left), "ESC on the controller saves and leaves the editor");
}
