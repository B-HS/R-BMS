//! The key editor binding two keys or two controller elements held together: how a capture tells
//! one key from two, what it refuses, and what it says of a key two rows share.

use std::time::Instant;

use super::table::{Line, lines};
use super::*;
use crate::keyconfig::{ControlAction, Shortcut};
use crate::{App, Config, LaunchOptions};

fn app(tag: &str) -> App {
    let dir = std::env::temp_dir().join(format!("rbms-keyconfig-chord-tests-{tag}-{}", std::process::id()));
    let mut app = App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.keyconfig_path = dir.join("keyconfig.ron");
    app.shared.keyconfig = KeyConfig::default();
    app
}

fn key(app: &mut App, state: &mut KeyConfigState, code: KeyCode, pressed: bool) {
    let input = KeyInput { code, pressed, released: !pressed, text: None };
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, input);
}

/// An editor with its cursor on the keyboard cell of `target`, waiting for its binding.
fn waiting_on(app: &mut App, target: Bindable) -> KeyConfigState {
    let sel = lines(app.shared.kc_edit_mode).iter().position(|line| matches!(line, Line::Bind { target: bound, .. } if *bound == target)).expect("listed");
    let mut state = KeyConfigState { sel, ..KeyConfigState::new() };
    key(app, &mut state, KeyCode::Enter, true);
    key(app, &mut state, KeyCode::Enter, false);
    assert!(state.capturing);
    state
}

fn bind_of(app: &App, target: Bindable) -> Option<KeyBind> {
    app.shared.keyconfig.bound_keys(app.shared.kc_edit_mode).into_iter().find(|(bound, _)| *bound == target).and_then(|(_, bind)| bind)
}

/// A control or a shortcut takes a key once it is let go, and two once the second goes down while
/// the first is held; a lane takes the first key that goes down.
#[test]
fn a_row_that_takes_a_chord_waits_to_tell_one_key_from_two() {
    let mut app = app("tell");
    let folders = Bindable::Shortcut(Shortcut::Folders);
    let mut state = waiting_on(&mut app, folders);
    key(&mut app, &mut state, KeyCode::F7, true);
    assert!(state.capturing, "one key held may still become two");
    key(&mut app, &mut state, KeyCode::F7, false);
    assert_eq!((state.capturing, bind_of(&app, folders)), (false, Some(KeyBind::Single(KeyCode::F7))));

    let mut state = waiting_on(&mut app, folders);
    key(&mut app, &mut state, KeyCode::ControlLeft, true);
    key(&mut app, &mut state, KeyCode::F10, true);
    assert!(!state.capturing);
    assert_eq!(bind_of(&app, folders), KeyBind::chord(KeyCode::ControlLeft, KeyCode::F10));
    key(&mut app, &mut state, KeyCode::ControlLeft, false);
    key(&mut app, &mut state, KeyCode::F10, false);
    assert_eq!(bind_of(&app, folders), KeyBind::chord(KeyCode::ControlLeft, KeyCode::F10), "letting the two go binds nothing else");

    let lane = Bindable::Lane(2);
    let mut state = waiting_on(&mut app, lane);
    key(&mut app, &mut state, KeyCode::KeyP, true);
    assert_eq!((state.capturing, bind_of(&app, lane)), (false, Some(KeyBind::Single(KeyCode::KeyP))), "a lane takes the press");
}

/// Escape calls a capture off even with a first key held; the same two keys another row of the
/// screen has, in either order, are refused and named; a key a screen keeps is refused in two.
#[test]
fn a_chord_another_row_has_or_with_a_kept_key_is_refused() {
    let mut app = app("refused");
    let tables = Bindable::Shortcut(Shortcut::Tables);
    let mut state = waiting_on(&mut app, tables);
    key(&mut app, &mut state, KeyCode::F7, true);
    key(&mut app, &mut state, KeyCode::Escape, true);
    assert!(!state.capturing);
    assert_eq!(bind_of(&app, tables), Some(KeyBind::Single(KeyCode::KeyT)));

    let folders = Bindable::Shortcut(Shortcut::Folders);
    app.shared.keyconfig.bind_keys(app.shared.kc_edit_mode, folders, KeyBind::chord(KeyCode::F7, KeyCode::F10).expect("two keys"));
    let mut state = waiting_on(&mut app, tables);
    key(&mut app, &mut state, KeyCode::F10, true);
    key(&mut app, &mut state, KeyCode::F7, true);
    assert_eq!(state.notice, Notice::ChordTaken(folders));
    assert_eq!(bind_of(&app, tables), Some(KeyBind::Single(KeyCode::KeyT)));
    let (said, _) = state.status(&app.shared, &Bindings::read(&app.shared.keyconfig, app.shared.kc_edit_mode));
    assert!(said.contains("COMBINATION") && said.contains(&folders.label(app.shared.kc_edit_mode)), "{said}");

    let mut state = waiting_on(&mut app, tables);
    key(&mut app, &mut state, KeyCode::F7, true);
    key(&mut app, &mut state, KeyCode::ArrowUp, true);
    assert_eq!(state.notice, Notice::Fixed);
}

/// A chord one of whose keys is another row's single key is taken, and both rows say that the key
/// pressed first does what it does alone.
#[test]
fn a_chord_sharing_a_key_with_a_single_key_is_taken_and_both_rows_say_so() {
    let mut app = app("overlap");
    let mode = app.shared.kc_edit_mode;
    let tables = Bindable::Shortcut(Shortcut::Tables);
    let mut state = waiting_on(&mut app, tables);
    key(&mut app, &mut state, KeyCode::KeyA, true);
    key(&mut app, &mut state, KeyCode::F7, true);
    assert_eq!(state.notice, Notice::None);
    assert_eq!(bind_of(&app, tables), KeyBind::chord(KeyCode::KeyA, KeyCode::F7));

    let mut state = KeyConfigState { sel: state.sel, ..KeyConfigState::new() };
    let (said, color) = state.status(&app.shared, &Bindings::read(&app.shared.keyconfig, mode));
    assert!(said.contains("ALONE") && said.contains("START"), "{said}");
    assert_eq!(color, rbms_render::theme().warn);

    let start = Bindable::Control(ControlAction::Start);
    state.sel = lines(mode).iter().position(|line| matches!(line, Line::Bind { target, .. } if *target == start)).expect("listed");
    let (said, _) = state.status(&app.shared, &Bindings::read(&app.shared.keyconfig, mode));
    assert!(said.contains("PART OF") && said.contains(&tables.label(mode)), "{said}");
}

/// The controller's cell takes elements as the keyboard's takes keys: one pressed and let go, or a
/// second pressed while the first is held; a lane takes the first press.
#[test]
fn the_controllers_cell_tells_one_element_from_two() {
    let mut app = app("pad");
    let escape = Bindable::Control(ControlAction::Escape);
    let mut state = KeyConfigState { capturing: true, column: Column::Pad, ..KeyConfigState::new() };
    state.capture_elements(&mut app.shared, escape, &[(PadBinding::Button(4), true)]);
    assert!(state.capturing);
    state.capture_elements(&mut app.shared, escape, &[(PadBinding::Button(5), true), (PadBinding::Button(4), false)]);
    assert!(!state.capturing);
    let pad = &app.shared.keyconfig.pad;
    assert_eq!(
        (pad.control_binding(ControlAction::Escape), pad.control_chord(ControlAction::Escape)),
        (Some(PadBinding::Button(4)), Some(PadBinding::Button(5)))
    );
    let bound = Bindings::read(&app.shared.keyconfig, app.shared.kc_edit_mode);
    assert_eq!(pad_token(bound.pad_of(escape), bound.pad_chord_of(escape)), "BUTTON 4 + BUTTON 5");

    let mut state = KeyConfigState { capturing: true, column: Column::Pad, ..KeyConfigState::new() };
    state.capture_elements(&mut app.shared, escape, &[(PadBinding::Button(6), true), (PadBinding::Button(6), false)]);
    assert_eq!(
        (app.shared.keyconfig.pad.control_binding(ControlAction::Escape), app.shared.keyconfig.pad.control_chord(ControlAction::Escape)),
        (Some(PadBinding::Button(6)), None)
    );

    let mut state = KeyConfigState { capturing: true, column: Column::Pad, ..KeyConfigState::new() };
    state.capture_elements(&mut app.shared, Bindable::Lane(0), &[(PadBinding::Button(7), true)]);
    assert!(!state.capturing);
    assert_eq!(app.shared.keyconfig.pad.lane_binding(app.shared.kc_edit_mode, 0), Some(PadBinding::Button(7)));
}

/// ESC is a line of the editor with a cell on each device.
#[test]
fn esc_is_a_line_of_the_editor_on_both_devices() {
    let escape = Bindable::Control(ControlAction::Escape);
    assert!(lines(Mode::BEAT_7K).contains(&Line::Bind { target: escape, pad: true }));
    assert_eq!(escape.label(Mode::BEAT_7K), "ESC");
}
