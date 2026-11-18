//! Two keys held together: how a chord is written, made, broken and refused, what a key config file
//! from before chords reads as, and how the key editor tells one key from two.

use rbms_model::Mode;
use winit::keyboard::KeyCode;

use super::*;
use crate::gamepad::{PadBinding, PadConfig};
use crate::keyconfig::{Clash, ControlAction, KeyConfig, Shortcut};

const LCTRL: KeyCode = KeyCode::ControlLeft;
const F7: KeyCode = KeyCode::F7;
const F10: KeyCode = KeyCode::F10;

fn chord(first: KeyCode, second: KeyCode) -> KeyBind {
    KeyBind::chord(first, second).expect("two different keys")
}

fn held(keys: &[KeyCode]) -> HeldKeys {
    let mut held = HeldKeys::default();
    keys.iter().for_each(|code| held.press(*code));
    held
}

#[test]
fn a_chord_is_written_as_two_key_words_and_read_back() {
    let bind = chord(LCTRL, KeyCode::KeyF);
    assert_eq!(bind.token(), "LCTRL+F");
    assert_eq!(KeyBind::parse("LCTRL+F"), Some(bind));
    assert_eq!(KeyBind::parse("lctrl + f"), Some(bind), "the words are read as a single key's are");
    assert_eq!(bind.label(), "LCTRL + F");
    assert_eq!(KeyBind::parse("F"), Some(KeyBind::Single(KeyCode::KeyF)));
    for broken in ["F+F", "F+", "+F", "F+NOSUCHKEY", "NOSUCHKEY", ""] {
        assert_eq!(KeyBind::parse(broken), None, "{broken:?}");
    }
    assert_eq!(KeyBind::chord(F7, F7), None, "one key twice is no chord");
}

#[test]
fn a_chord_is_the_same_keys_in_either_order_and_never_a_single_key() {
    assert!(chord(F7, F10).same_keys(chord(F10, F7)));
    assert!(!chord(F7, F10).same_keys(chord(F7, LCTRL)));
    assert!(!chord(F7, F10).same_keys(KeyBind::Single(F7)));
    assert!(KeyBind::Single(F7).same_keys(KeyBind::Single(F7)));
}

/// Two keys make the chord the moment both are down, whichever went down first, and the release of
/// either breaks it.
#[test]
fn a_chord_is_made_by_its_second_key_in_either_order_and_broken_by_either() {
    let target = Bindable::Shortcut(Shortcut::Folders);
    let bound = [(target, chord(F7, F10))];
    assert!(held_chords_with(&bound, &held(&[F7]), F7).is_empty(), "one key is not the chord");
    assert_eq!(held_chords_with(&bound, &held(&[F7, F10]), F10), [target]);
    assert_eq!(held_chords_with(&bound, &held(&[F10, F7]), F7), [target], "the other order makes it too");
    assert!(held_chords_with(&bound, &held(&[F7, F10]), LCTRL).is_empty(), "a key that is not the chord's makes nothing");
    assert!(held_chords_with(&[(target, KeyBind::Single(F7))], &held(&[F7]), F7).is_empty(), "a single key is no chord");
}

/// While both keys of a chord are down they are the chord's, and neither counts as down for what
/// it alone is bound to.
#[test]
fn the_keys_of_a_held_chord_are_the_chords_alone() {
    let bound = [(Bindable::Control(ControlAction::HiSpeedUp), chord(F7, F10))];
    assert!(keys_in_held_chords(&bound, &held(&[F7])).is_empty());
    let both = keys_in_held_chords(&bound, &held(&[F7, F10, LCTRL]));
    assert!(both.contains(&F7) && both.contains(&F10) && !both.contains(&LCTRL), "{both:?}");
}

/// One key pressed and let go is that key; a second pressed while the first is held is the two.
#[test]
fn a_capture_tells_one_key_from_two_held_together() {
    let mut capture = ChordCapture::default();
    assert_eq!(capture.press(F7), None, "a key is held on to until it is let go or joined");
    assert_eq!(capture.press(F7), None, "a repeat of it is not a second key");
    assert_eq!(capture.release(F7), Some(Captured::Single(F7)));

    let mut capture = ChordCapture::default();
    assert_eq!(capture.release(KeyCode::Enter), None, "a key let go that went down before the capture is not taken");
    capture.press(F7);
    assert_eq!(capture.press(F10), Some(Captured::Chord(F7, F10)));
    assert_eq!(capture.release(F7), None, "the keys let go after the chord was taken take nothing");
}

/// A key config written before chords and ESC existed reads every control and shortcut as it did,
/// ESC with no key and the controller with no chord; one carrying a chord reads it back.
#[test]
fn a_key_config_from_before_chords_still_loads_and_a_chord_survives_the_file() {
    let older: KeyConfig = ron::from_str(
        r#"(controls: (hispeed_up: "UP", start: "A"), shortcuts: {"folders": "F7"}, pad: (controls: {"START": Button(9)}, shortcuts: {"search": Button(4)}))"#,
    )
    .expect("a file from before chords parses");
    assert_eq!(older.control_bind(ControlAction::HiSpeedUp), Some(KeyBind::Single(KeyCode::ArrowUp)));
    assert_eq!(older.shortcut_bind(Shortcut::Folders), Some(KeyBind::Single(F7)));
    assert_eq!(older.control_bind(ControlAction::Escape), None, "ESC ships with no key");
    assert!(older.chords().is_empty());
    assert_eq!(older.pad.control_binding(ControlAction::Start), Some(PadBinding::Button(9)));
    assert_eq!((older.pad.control_chord(ControlAction::Start), older.pad.shortcut_chord(Shortcut::Search)), (None, None));
    assert!(older.pad.control_chords.is_empty() && older.pad.shortcut_chords.is_empty());

    let mut keys = KeyConfig::default();
    keys.bind_keys(Mode::BEAT_7K, Bindable::Control(ControlAction::Escape), chord(LCTRL, KeyCode::KeyQ));
    keys.bind_keys(Mode::BEAT_7K, Bindable::Shortcut(Shortcut::Folders), chord(F10, F7));
    keys.pad.bind_chord(Bindable::Control(ControlAction::Escape), PadBinding::Button(8), PadBinding::Button(9));
    let text = ron::ser::to_string_pretty(&keys, ron::ser::PrettyConfig::default()).expect("a key config is written");
    assert!(text.contains(r#"escape: "LCTRL+Q""#) && text.contains(r#""folders": "F10+F7""#), "{text}");
    let back: KeyConfig = ron::from_str(&text).expect("it reads back");
    assert_eq!(back.control_bind(ControlAction::Escape), Some(chord(LCTRL, KeyCode::KeyQ)));
    assert_eq!(back.shortcut_bind(Shortcut::Folders), Some(chord(F10, F7)));
    assert_eq!(back.control_key(ControlAction::Escape), None, "a chord is no single key");
    assert_eq!(back.pad.control_binding(ControlAction::Escape), Some(PadBinding::Button(8)));
    assert_eq!(back.pad.control_chord(ControlAction::Escape), Some(PadBinding::Button(9)));
}

/// A lane and a reverse spin are played and take no chord; a key the file has no word for is not
/// bound in one.
#[test]
fn only_a_control_or_a_shortcut_takes_a_chord() {
    let mode = Mode::BEAT_7K;
    let mut keys = KeyConfig::default();
    keys.bind_keys(mode, Bindable::Lane(0), chord(F7, F10));
    assert_eq!(keys.lane_token(mode, 0), "Z");
    keys.bind_keys(mode, Bindable::Control(ControlAction::LiftToggle), chord(F7, KeyCode::F13));
    assert_eq!(keys.control_bind(ControlAction::LiftToggle), None);

    let mut pad = PadConfig::default();
    pad.bind_chord(Bindable::Lane(0), PadBinding::Button(1), PadBinding::Button(2));
    pad.bind_chord(Bindable::Control(ControlAction::CoverUp), PadBinding::Button(1), PadBinding::AnalogScratch { axis: 0 });
    pad.bind_chord(Bindable::Shortcut(Shortcut::OptionsHold), PadBinding::Button(1), PadBinding::Button(2));
    assert_eq!(pad.lane_binding(mode, 0), None);
    assert_eq!(pad.control_binding(ControlAction::CoverUp), None, "a turntable is no element to hold");
    assert_eq!(pad.shortcut_binding(Shortcut::OptionsHold), None, "a shortcut the controller cannot be given");

    pad.bind_chord(Bindable::Control(ControlAction::CoverUp), PadBinding::Button(1), PadBinding::Button(2));
    pad.bind(mode, Bindable::Control(ControlAction::CoverUp), Some(PadBinding::Button(3)));
    assert_eq!(
        (pad.control_binding(ControlAction::CoverUp), pad.control_chord(ControlAction::CoverUp)),
        (Some(PadBinding::Button(3)), None),
        "one element drops the second"
    );
}

/// Two things read on one screen may not have the same two keys, in either order. A chord one of
/// whose keys is something else's single key is not refused, and the key editor is told; a key a
/// screen keeps for itself is refused in a chord too; and a chord of another screen is free.
#[test]
fn the_same_chord_is_refused_on_one_screen_and_a_shared_key_is_only_named() {
    let mode = Mode::BEAT_7K;
    let mut keys = KeyConfig::default();
    let folders = Bindable::Shortcut(Shortcut::Folders);
    keys.bind_keys(mode, folders, chord(F7, F10));
    assert_eq!(keys.clash_of(mode, Bindable::Shortcut(Shortcut::Tables), chord(F10, F7)), Some(Clash::With(folders)), "the same keys in the other order");
    assert_eq!(keys.clash_of(mode, Bindable::Shortcut(Shortcut::Retry), chord(F10, F7)), None, "the result screen never reads the browser's keys");
    assert_eq!(keys.clash_of(mode, folders, chord(F10, F7)), None, "a row's own chord is no clash");

    let start = Bindable::Control(ControlAction::Start);
    assert_eq!(keys.clash_of(mode, Bindable::Shortcut(Shortcut::Tables), chord(KeyCode::KeyA, F7)), None, "START's key in a chord is not refused");
    keys.bind_keys(mode, Bindable::Shortcut(Shortcut::Tables), chord(KeyCode::KeyA, F7));
    assert_eq!(keys.chord_overlap(mode, Bindable::Shortcut(Shortcut::Tables)), Some((KeyCode::KeyA, start)), "the editor names what the key does alone");
    assert_eq!(keys.chord_overlap(mode, start), Some((KeyCode::KeyA, Bindable::Shortcut(Shortcut::Tables))), "and START's row names the chord");
    assert_eq!(keys.chord_overlap(mode, folders), None, "two chords sharing a key do nothing alone");

    let search = Bindable::Shortcut(Shortcut::Search);
    assert_eq!(keys.clash_of(mode, search, chord(KeyCode::Enter, F7)), Some(Clash::Fixed));
    assert_eq!(keys.clash_of(mode, Bindable::Control(ControlAction::HiSpeedUp), chord(KeyCode::Escape, F7)), Some(Clash::Fixed));
}

/// ESC is read on every screen, so its key is refused to every shortcut and every lane and control.
#[test]
fn esc_is_read_beside_everything() {
    let escape = Bindable::Control(ControlAction::Escape);
    for shortcut in Shortcut::ALL {
        assert!(escape.read_with(Bindable::Shortcut(shortcut)) && Bindable::Shortcut(shortcut).read_with(escape), "{shortcut:?}");
    }
    let mode = Mode::BEAT_7K;
    let mut keys = KeyConfig::default();
    keys.set_control(ControlAction::Escape, KeyCode::Backspace);
    assert_eq!(keys.clash(mode, Bindable::Shortcut(Shortcut::Retry), KeyCode::Backspace), Some(Clash::With(escape)));
    assert_eq!(keys.clash(mode, Bindable::Lane(0), KeyCode::Backspace), Some(Clash::With(escape)));
    assert_eq!(keys.clash(mode, escape, KeyCode::Escape), Some(Clash::Fixed), "Escape itself is no key for ESC");
}
