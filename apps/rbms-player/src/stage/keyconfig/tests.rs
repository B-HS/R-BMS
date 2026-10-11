//! The key editor: what it lists, how its cursor moves, what a capture binds and refuses, and what
//! clearing, restoring and leaving do. The frames it draws are saved for a person to look at when a
//! capture folder is named.

use std::time::Instant;

use rbms_render::Renderer;

use super::table::{Line, lines};
use super::*;
use crate::keyconfig::{ControlAction, Shortcut, ShortcutGroup};
use crate::stage::HeadlessCanvas;
use crate::{App, Config, LaunchOptions};

/// Names the folder the frames of the editor are saved in, as it does for the captures of a skin's
/// screens. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The two sizes the editor is captured at: the one it is laid out for, and a full HD window.
const CAPTURE_SIZES: [(u32, u32); 2] = [(CW, CH), (1920, 1080)];

fn app(tag: &str) -> App {
    rbms_render::font::use_embedded_fonts_only();
    let dir = std::env::temp_dir().join(format!("rbms-keyconfig-stage-tests-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp dir");
    let mut app = App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron"));
    app.shared.keyconfig_path = dir.join("keyconfig.ron");
    app.shared.keyconfig = KeyConfig::default();
    app
}

fn press(app: &mut App, state: &mut KeyConfigState, code: KeyCode) -> Transition {
    let key = KeyInput { code, pressed: true, released: false, text: None };
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, key)
}

/// An editor with its cursor on the keyboard cell of `target`.
fn on(app: &App, target: Bindable) -> KeyConfigState {
    let sel =
        lines(app.shared.kc_edit_mode).iter().position(|line| matches!(line, Line::Bind { target: bound, .. } if *bound == target)).expect("the row is listed");
    KeyConfigState { sel, ..KeyConfigState::new() }
}

fn rebind(app: &mut App, state: &mut KeyConfigState, code: KeyCode) {
    press(app, state, KeyCode::Enter);
    assert!(state.capturing, "Enter did not arm the capture");
    press(app, state, code);
    assert!(!state.capturing, "the capture stayed armed after a key");
}

fn key_of(app: &App, target: Bindable) -> Option<KeyCode> {
    app.shared.keyconfig.bound_keys(app.shared.kc_edit_mode).into_iter().find(|(bound, _)| *bound == target).and_then(|(_, key)| key)
}

/// Draw the editor at `size`, save the frame as `name` when a capture folder was asked for, and
/// answer the frame.
fn drawn(app: &mut App, state: &mut KeyConfigState, name: &str, size: (u32, u32)) -> HeadlessCanvas {
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    state.draw(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, &mut Canvas::Headless(&mut pixels));
    if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
        let folder = std::path::Path::new(&folder);
        std::fs::create_dir_all(folder).expect("the capture folder can be created");
        let file = folder.join(format!("{name}-{}x{}.png", size.0, size.1));
        image::save_buffer(file, pixels.rgba(), size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
    }
    pixels
}

/// How many pixels of the frame are exactly `color`.
fn pixels_of(frame: &HeadlessCanvas, color: Color) -> usize {
    let (width, height) = frame.size();
    (0..height).flat_map(|y| (0..width).map(move |x| (x, y))).filter(|(x, y)| frame.pixel_at(*x, *y) == color).count()
}

/// Everything that can be bound in the mode being edited has exactly one line, under a heading, and
/// the headings are the screens that read them, top to bottom.
#[test]
fn the_table_lists_everything_once_under_the_heading_of_the_screen_that_reads_it() {
    for &mode in Mode::ALL {
        let rows = lines(mode);
        assert_eq!(rows[0], Line::Mode, "{}", mode.name);
        for target in Bindable::all(mode) {
            let listed = rows.iter().filter(|line| matches!(line, Line::Bind { target: bound, .. } if *bound == target)).count();
            assert_eq!(listed, 1, "{} {target:?}", mode.name);
        }
        let headings: Vec<&str> = rows
            .iter()
            .filter_map(|line| match line {
                Line::Heading(title) => Some(*title),
                _ => None,
            })
            .collect();
        let mut expected = vec!["LANES", "PLAY"];
        expected.extend(ShortcutGroup::ALL.into_iter().map(ShortcutGroup::title));
        expected.extend(["CONTROLLER", "DEFAULTS"]);
        assert_eq!(headings, expected, "{}", mode.name);
        assert_eq!(rows.last(), Some(&Line::RestoreAll));
    }
}

/// A line has a cell for the controller unless it is one of the three shortcuts only the keyboard
/// reads, and every lane, reverse key and control has one.
#[test]
fn every_line_has_a_controller_cell_but_the_ones_the_keyboard_alone_reads() {
    for line in lines(Mode::BEAT_7K) {
        if let Line::Bind { target, pad } = line {
            let keyboard_only = matches!(target, Bindable::Shortcut(Shortcut::OptionsPin | Shortcut::OptionsHold | Shortcut::RefDetail));
            assert_eq!(pad, !keyboard_only, "{target:?}");
        }
    }
}

/// The cursor moves a line at a time, passes over the headings, and stops at either end of the list.
#[test]
fn the_cursor_passes_over_headings_and_stops_at_the_ends() {
    let mut app = app("cursor");
    let mut state = KeyConfigState::new();
    let rows = lines(app.shared.kc_edit_mode);
    press(&mut app, &mut state, KeyCode::ArrowUp);
    assert_eq!(state.sel, 0, "the cursor left the top of the list");
    let mut visited = vec![state.sel];
    for _ in 0..rows.len() * 2 {
        press(&mut app, &mut state, KeyCode::ArrowDown);
        if visited.last() != Some(&state.sel) {
            visited.push(state.sel);
        }
    }
    let expected: Vec<usize> = rows.iter().enumerate().filter(|(_, line)| line.takes_the_cursor()).map(|(index, _)| index).collect();
    assert_eq!(visited, expected, "the cursor did not rest on every line that takes it, in order, and on no heading");
    assert_eq!(rows[state.sel], Line::RestoreAll);
}

/// Page down goes to the first line of the next group and page up to the first line of the group
/// before, so a list this long is crossed in a few presses.
#[test]
fn the_page_keys_jump_from_group_to_group() {
    let mut app = app("groups");
    let mut state = KeyConfigState::new();
    let rows = lines(app.shared.kc_edit_mode);
    let first_of = |title: &str| rows.iter().position(|line| matches!(line, Line::Heading(heading) if *heading == title)).expect("the group is listed") + 1;

    press(&mut app, &mut state, KeyCode::PageDown);
    assert_eq!(state.sel, first_of("LANES"));
    press(&mut app, &mut state, KeyCode::PageDown);
    assert_eq!(state.sel, first_of("PLAY"));
    press(&mut app, &mut state, KeyCode::ArrowDown);
    press(&mut app, &mut state, KeyCode::PageDown);
    assert_eq!(state.sel, first_of(ShortcutGroup::Browser.title()), "from the middle of a group the next group is the one after it");
    press(&mut app, &mut state, KeyCode::PageUp);
    assert_eq!(state.sel, first_of("PLAY"));
    press(&mut app, &mut state, KeyCode::PageUp);
    press(&mut app, &mut state, KeyCode::PageUp);
    assert_eq!(state.sel, 0, "page up from the first group is the top of the list");
    for _ in 0..rows.len() {
        press(&mut app, &mut state, KeyCode::PageDown);
    }
    assert_eq!(rows[state.sel], Line::RestoreAll, "page down stops on the last group");
}

/// Every line that can be bound takes the key pressed after Enter: a lane, a reverse key, each
/// control of a run and each shortcut.
#[test]
fn enter_then_a_key_binds_every_kind_of_row() {
    let free = [KeyCode::F6, KeyCode::F7, KeyCode::F8, KeyCode::F10, KeyCode::F11, KeyCode::Insert, KeyCode::Home, KeyCode::End];
    let mode = Mode::BEAT_7K;
    for (index, target) in Bindable::all(mode).into_iter().enumerate() {
        let mut app = app(&format!("bind-{index}"));
        let to = free[index % free.len()];
        let mut state = on(&app, target);
        rebind(&mut app, &mut state, to);
        assert_eq!(state.notice, Notice::None, "{target:?}: {to:?} was refused");
        assert_eq!(key_of(&app, target), Some(to), "{target:?}");
    }
}

/// A key something read at the same moment already has is refused, the row keeps the key it had,
/// and the editor says what has the key. A key that is somebody else's on another screen is taken.
#[test]
fn a_key_that_is_taken_is_refused_and_the_row_says_by_what() {
    let taken = [
        (Bindable::Control(ControlAction::Start), KeyCode::KeyZ, Bindable::Lane(0)),
        (Bindable::Control(ControlAction::Start), KeyCode::ArrowUp, Bindable::Control(ControlAction::HiSpeedUp)),
        (Bindable::Control(ControlAction::CoverToggle), KeyCode::ArrowRight, Bindable::Control(ControlAction::CoverUp)),
        (Bindable::Shortcut(Shortcut::Search), KeyCode::F2, Bindable::Shortcut(Shortcut::Filter)),
        (Bindable::Shortcut(Shortcut::KeyConfig), KeyCode::Digit6, Bindable::Shortcut(Shortcut::RefKeyConfig)),
        (Bindable::Shortcut(Shortcut::Folders), KeyCode::KeyA, Bindable::Control(ControlAction::Start)),
        (Bindable::Lane(3), KeyCode::Backslash, Bindable::Control(ControlAction::CoverToggle)),
    ];
    for (index, (target, code, holder)) in taken.into_iter().enumerate() {
        let mut app = app(&format!("taken-{index}"));
        let before = key_of(&app, target);
        let mut state = on(&app, target);
        rebind(&mut app, &mut state, code);
        assert_eq!(state.notice, Notice::Taken(holder), "{target:?} onto {code:?}");
        assert_eq!(key_of(&app, target), before, "{code:?} replaced the binding of {target:?}");
        let (said, _) = state.status(&app.shared, &Bindings::read(&app.shared.keyconfig, app.shared.kc_edit_mode));
        assert!(said.contains(&holder.label(app.shared.kc_edit_mode)), "the editor does not name what has the key: {said}");
        press(&mut app, &mut state, KeyCode::ArrowDown);
        assert_eq!(state.notice, Notice::None, "the notice outlived the next key");
    }

    let mut app = app("taken-elsewhere");
    let mut state = on(&app, Bindable::Shortcut(Shortcut::Retry));
    rebind(&mut app, &mut state, KeyCode::Slash);
    assert_eq!(key_of(&app, Bindable::Shortcut(Shortcut::Retry)), Some(KeyCode::Slash), "a key of the browser was refused to the result screen");
}

/// A key the row's screen keeps for itself, and a key the file has no word for, are refused with
/// their own notices; Escape cancels a capture and binds nothing.
#[test]
fn a_fixed_key_and_a_key_with_no_word_are_refused_and_escape_cancels() {
    let mut app = app("fixed");
    let search = Bindable::Shortcut(Shortcut::Search);
    let mut state = on(&app, search);
    for code in [KeyCode::Enter, KeyCode::ArrowUp, crate::keyconfig::GUIDE_KEY] {
        rebind(&mut app, &mut state, code);
        assert_eq!(state.notice, Notice::Fixed, "{code:?}");
        assert_eq!(key_of(&app, search), Some(KeyCode::Slash));
    }
    rebind(&mut app, &mut state, KeyCode::F13);
    assert_eq!(state.notice, Notice::Unnamed);
    assert_eq!(key_of(&app, search), Some(KeyCode::Slash));

    press(&mut app, &mut state, KeyCode::Enter);
    assert!(matches!(press(&mut app, &mut state, KeyCode::Escape), Transition::Stay), "Escape left the editor from inside a capture");
    assert!(!state.capturing);
    assert_eq!((key_of(&app, search), state.notice), (Some(KeyCode::Slash), Notice::None));

    let mut state = on(&app, Bindable::Control(ControlAction::CoverToggle));
    rebind(&mut app, &mut state, KeyCode::Enter);
    assert_eq!(key_of(&app, Bindable::Control(ControlAction::CoverToggle)), Some(KeyCode::Enter), "Enter is not fixed in a run and was refused to a control");
}

/// Delete takes the key of the focused cell away and Backspace puts the shipped one back. A lane
/// cannot be without a key, so clearing one puts its shipped key back and the editor says so.
#[test]
fn delete_clears_a_cell_and_backspace_puts_the_shipped_key_back() {
    let mut app = app("clear");
    let cover = Bindable::Control(ControlAction::CoverUp);
    let mut state = on(&app, cover);
    press(&mut app, &mut state, KeyCode::Delete);
    assert_eq!(key_of(&app, cover), None);
    press(&mut app, &mut state, KeyCode::Backspace);
    assert_eq!(key_of(&app, cover), Some(KeyCode::ArrowRight));

    let folders = Bindable::Shortcut(Shortcut::Folders);
    let mut state = on(&app, folders);
    press(&mut app, &mut state, KeyCode::Delete);
    assert_eq!(key_of(&app, folders), None);
    rebind(&mut app, &mut state, KeyCode::F6);
    press(&mut app, &mut state, KeyCode::Backspace);
    assert_eq!(key_of(&app, folders), Some(KeyCode::KeyO));

    let lane = Bindable::Lane(1);
    let mut state = on(&app, lane);
    rebind(&mut app, &mut state, KeyCode::KeyP);
    press(&mut app, &mut state, KeyCode::Delete);
    assert_eq!(key_of(&app, lane), Some(KeyCode::KeyS), "a cleared lane is not on its shipped key");
    assert_eq!(state.notice, Notice::LaneKept);
}

/// The last line puts everything back as shipped, and only on a second Enter: any other key calls
/// it off.
#[test]
fn restoring_everything_asks_twice() {
    let mut app = app("restore");
    let cover = Bindable::Control(ControlAction::CoverUp);
    let mut state = on(&app, cover);
    rebind(&mut app, &mut state, KeyCode::KeyP);
    let mut state = KeyConfigState { sel: lines(app.shared.kc_edit_mode).len() - 1, ..KeyConfigState::new() };

    press(&mut app, &mut state, KeyCode::Enter);
    assert_eq!(state.notice, Notice::RestoreArmed);
    assert_eq!(key_of(&app, cover), Some(KeyCode::KeyP), "one Enter restored everything");
    press(&mut app, &mut state, KeyCode::ArrowLeft);
    assert_eq!(state.notice, Notice::None);
    press(&mut app, &mut state, KeyCode::Enter);
    assert_eq!(key_of(&app, cover), Some(KeyCode::KeyP), "an Enter after the restore was called off carried it out");
    press(&mut app, &mut state, KeyCode::Enter);
    assert_eq!(state.notice, Notice::Restored);
    assert_eq!(key_of(&app, cover), Some(KeyCode::ArrowRight));
}

/// Left and right move between the keyboard's cell and the controller's. A row the keyboard alone
/// reads says so instead of arming a capture nothing could answer, and a controller cell is not
/// bound by a key.
#[test]
fn the_two_columns_are_bound_separately() {
    let mut app = app("columns");
    let cover = Bindable::Control(ControlAction::CoverToggle);
    let mut state = on(&app, cover);
    press(&mut app, &mut state, KeyCode::ArrowRight);
    assert_eq!(state.column, Column::Pad);
    press(&mut app, &mut state, KeyCode::Enter);
    assert!(state.capturing);
    press(&mut app, &mut state, KeyCode::KeyP);
    assert!(state.capturing, "a key answered a capture that waits for the controller");
    assert_eq!(key_of(&app, cover), Some(KeyCode::Backslash), "a key was bound from the controller's cell");
    press(&mut app, &mut state, KeyCode::Escape);
    assert!(!state.capturing);

    app.shared.keyconfig.pad.bind(app.shared.kc_edit_mode, cover, Some(PadBinding::Button(7)));
    press(&mut app, &mut state, KeyCode::Delete);
    assert_eq!(app.shared.keyconfig.pad.binding_of(app.shared.kc_edit_mode, cover), None, "Delete on the controller's cell left its binding");
    assert_eq!(key_of(&app, cover), Some(KeyCode::Backslash), "Delete on the controller's cell cleared the key");
    press(&mut app, &mut state, KeyCode::ArrowLeft);
    assert_eq!(state.column, Column::Keyboard);

    let mut state = KeyConfigState { column: Column::Pad, ..on(&app, Bindable::Shortcut(Shortcut::OptionsHold)) };
    press(&mut app, &mut state, KeyCode::Enter);
    assert!(!state.capturing);
    assert_eq!(state.notice, Notice::KeyboardOnly);
}

/// Tab steps the mode whose lanes are edited from any line, and the cursor stays on the row it was
/// on when that row is in the new mode too.
#[test]
fn tab_steps_the_mode_and_keeps_the_row() {
    let mut app = app("mode");
    let search = Bindable::Shortcut(Shortcut::Search);
    let mut state = on(&app, search);
    let before = app.shared.kc_edit_mode;
    press(&mut app, &mut state, KeyCode::Tab);
    assert_ne!(app.shared.kc_edit_mode.name, before.name);
    assert_eq!(state.focused_target(&app.shared), Some(search), "the cursor left the row it was on");

    let mut state = KeyConfigState::new();
    press(&mut app, &mut state, KeyCode::ArrowRight);
    press(&mut app, &mut state, KeyCode::ArrowLeft);
    assert_eq!(app.shared.kc_edit_mode.name, Mode::ALL[1].name, "left and right on the mode row do not step it both ways");
    for _ in 0..Mode::ALL.len() {
        press(&mut app, &mut state, KeyCode::Tab);
        assert!(lines(app.shared.kc_edit_mode)[state.sel].takes_the_cursor());
    }
}

/// Escape writes the key config out and goes back, and what was written reads back with the keys
/// that were bound.
#[test]
fn escape_saves_and_goes_back() {
    let mut app = app("save");
    let toggle = Bindable::Control(ControlAction::LiftToggle);
    let mut state = on(&app, toggle);
    rebind(&mut app, &mut state, KeyCode::F6);
    let mut state = on(&app, Bindable::Shortcut(Shortcut::Search));
    rebind(&mut app, &mut state, KeyCode::F7);
    assert!(matches!(press(&mut app, &mut state, KeyCode::Escape), Transition::Back));
    let written = KeyConfig::load(&app.shared.keyconfig_path);
    assert_eq!(written.control_key(ControlAction::LiftToggle), Some(KeyCode::F6));
    assert_eq!(written.shortcut_key(Shortcut::Search), Some(KeyCode::F7));
}

/// The editor says which lane a shortcut of the browser shares its key with, without refusing it or
/// drawing it as a clash, and says of a key two rows share that one of them will not work.
#[test]
fn the_status_line_names_a_shared_lane_and_a_clash() {
    let mut app = app("status");
    let mode = app.shared.kc_edit_mode;
    let state = on(&app, Bindable::Shortcut(Shortcut::Favourite));
    let bound = Bindings::read(&app.shared.keyconfig, mode);
    assert_eq!(bound.key_clash(Bindable::Shortcut(Shortcut::Favourite)), None);
    let (said, _) = state.status(&app.shared, &bound);
    assert!(said.contains("LANE 6"), "{said}");

    app.shared.keyconfig.set_control(ControlAction::CoverUp, KeyCode::ArrowUp);
    let state = on(&app, Bindable::Control(ControlAction::CoverUp));
    let bound = Bindings::read(&app.shared.keyconfig, mode);
    assert_eq!(bound.key_clash(Bindable::Control(ControlAction::CoverUp)), Some(Clash::With(Bindable::Control(ControlAction::HiSpeedUp))));
    let (said, color) = state.status(&app.shared, &bound);
    assert!(said.contains("HI-SPEED UP") && color == Color::RED, "{said}");
}

/// The editor draws every state it has, at the size it is laid out for and at full HD: each group
/// with the list scrolled to it, a capture waiting for its key, a key that was refused, and a clash
/// a file brought in. The warnings are drawn in red and the waiting cell in yellow.
#[test]
fn every_state_of_the_editor_draws() {
    for size in CAPTURE_SIZES {
        let mut app = app(&format!("draw-{}", size.0));
        let mode = app.shared.kc_edit_mode;
        let mut top = KeyConfigState::new();
        let plain = drawn(&mut app, &mut top, "keys-1-top-lanes", size);
        assert!(plain.painted_pixels() > 0);
        assert_eq!(pixels_of(&plain, Color::RED), 0, "the shipped keys draw a row in red");

        for (name, target) in [
            ("keys-2-play", Bindable::Control(ControlAction::CoverToggle)),
            ("keys-3-select", Bindable::Shortcut(Shortcut::Favourite)),
            ("keys-4-screens-and-number-keys", Bindable::Shortcut(Shortcut::KeyConfig)),
            ("keys-5-result-replay-controller", Bindable::Shortcut(Shortcut::ReplayPause)),
        ] {
            let mut state = on(&app, target);
            let frame = drawn(&mut app, &mut state, name, size);
            assert_ne!(frame.pixel_checksum(), plain.pixel_checksum(), "{name} draws the top of the list");
        }
        let mut last = KeyConfigState { sel: lines(mode).len() - 1, ..KeyConfigState::new() };
        drawn(&mut app, &mut last, "keys-6-bottom-restore", size);

        let mut waiting = on(&app, Bindable::Control(ControlAction::LiftToggle));
        press(&mut app, &mut waiting, KeyCode::Enter);
        let frame = drawn(&mut app, &mut waiting, "keys-7-capture-waiting", size);
        assert!(pixels_of(&frame, Color::YELLOW) > 0, "a waiting capture is not drawn in yellow");

        let mut pad = KeyConfigState { column: Column::Pad, ..on(&app, Bindable::Lane(0)) };
        press(&mut app, &mut pad, KeyCode::Enter);
        drawn(&mut app, &mut pad, "keys-8-capture-waiting-controller", size);

        let mut refused = on(&app, Bindable::Control(ControlAction::CoverToggle));
        rebind(&mut app, &mut refused, KeyCode::ArrowRight);
        let frame = drawn(&mut app, &mut refused, "keys-9-refused-taken", size);
        assert!(pixels_of(&frame, Color::RED) > 0, "a refusal is not drawn in red");

        app.shared.keyconfig.set_control(ControlAction::CoverToggle, KeyCode::ArrowRight);
        app.shared.keyconfig.pad.bind(mode, Bindable::Control(ControlAction::CoverUp), Some(PadBinding::Button(3)));
        app.shared.keyconfig.pad.bind(mode, Bindable::Control(ControlAction::CoverDown), Some(PadBinding::Button(3)));
        let mut clashing = on(&app, Bindable::Control(ControlAction::CoverToggle));
        let frame = drawn(&mut app, &mut clashing, "keys-10-clash-from-a-file", size);
        assert!(pixels_of(&frame, Color::RED) > 0, "a clash is not drawn in red");
    }
}
