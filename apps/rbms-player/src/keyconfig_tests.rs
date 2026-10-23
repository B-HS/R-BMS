use crate::gamepad::{ANALOG_THRESHOLD_RANGE, AXIS_DEADZONE_RANGE, AnalogMode, DEBOUNCE_MS_RANGE, PadBinding, PadConfig};
use crate::keyconfig::{
    Bindable, Clash, ControlAction, GUIDE_KEY, HeldKeys, KEY_INDEX_COUNT, KeyConfig, SCRATCH_BACKWARD_INDEX, SCRATCH_FORWARD_INDEX, Shortcut, ShortcutScreen,
    UNNAMED_KEY, belongs_to_a_typed_line, clash_among, default_keys_for_mode, default_reverse_keys_for_mode, fixed_keys_of, key_from_name, key_index_of,
    key_name, mode_config_key,
};
use rbms_model::Mode;
use rbms_play::ScratchDir;
use std::collections::BTreeMap;
use winit::keyboard::KeyCode;

/// An app whose key config is the one this test wrote, so the reverse binding it sets really is the
/// one the input path resolves against.
/// The keys of `mode` that two things read at the same moment both have, which is what the key
/// editor draws in red.
fn collisions(keys: &KeyConfig, mode: Mode) -> std::collections::HashSet<KeyCode> {
    let bound = keys.bound_keys(mode);
    bound.iter().filter_map(|(target, key)| key.filter(|code| clash_among(bound.iter().copied(), *target, *code).is_some())).collect()
}

fn shared_with(keyconfig: &KeyConfig) -> crate::AppShared {
    let dir = std::env::temp_dir().join(format!("rbms-keyconfig-tests-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).expect("a temp directory for the key config");
    let path = dir.join("keyconfig.ron");
    keyconfig.save(&path);
    let launch = crate::LaunchOptions { keyconfig_path: Some(path.to_string_lossy().to_string()), ..crate::LaunchOptions::default() };
    let shared = crate::App::new(String::new(), crate::Config::default(), launch, dir.join("settings.ron")).shared;
    let _ = std::fs::remove_dir_all(&dir);
    shared
}

/// The reverse binding is not just stored: the input path has to resolve it to its lane and report
/// the backward direction, or the key would be dead in play while still blocking every other use.
#[test]
fn a_reverse_key_dispatches_as_a_backward_spin_on_its_scratch_lane() {
    let mode = Mode::BEAT_7K;
    let scratch = (0..mode.key).find(|&lane| mode.is_scratch(lane)).expect("7K has a scratch lane");
    let mut kc = KeyConfig::default();
    kc.set_scratch_reverse(mode, scratch, KeyCode::KeyQ);
    let forward = kc.lane_keys(mode).iter().find(|(_, lane)| *lane == scratch).map(|(code, _)| *code).expect("the scratch lane is bound");

    let shared = shared_with(&kc);
    assert_eq!(shared.lane_input_for(KeyCode::KeyQ), Some((scratch, ScratchDir::Backward)), "the reverse key spins its lane the other way");
    assert_eq!(shared.lane_input_for(forward), Some((scratch, ScratchDir::Forward)));
    assert_eq!(shared.lane_input_for(KeyCode::F13), None, "an unbound key still plays no lane");
}

/// A key of a lane is a forward spin whatever the reverse keys are, and the key the turntable's
/// other direction ships on is a backward spin of its lane.
#[test]
fn every_lane_key_is_a_forward_spin_and_the_shipped_reverse_key_a_backward_one() {
    let shared = shared_with(&KeyConfig::default());
    for (code, lane) in KeyConfig::default().lane_keys(Mode::BEAT_7K) {
        assert_eq!(shared.lane_input_for(code), Some((lane, ScratchDir::Forward)), "{code:?}");
    }
    assert_eq!(shared.lane_input_for(KeyCode::ControlLeft), Some((7, ScratchDir::Backward)));
}

#[test]
fn key_name_round_trips_through_key_from_name() {
    for &mode in Mode::ALL {
        for (code, _) in default_keys_for_mode(mode) {
            assert_eq!(key_from_name(key_name(code)), Some(code), "round-trip {code:?}");
        }
    }
    for token in ["UP", "DOWN", "LEFT", "RIGHT", "LBRACKET", "RBRACKET"] {
        let code = key_from_name(token).unwrap();
        assert_eq!(key_name(code), token);
    }
}

#[test]
fn default_config_has_every_mode_and_control() {
    let kc = KeyConfig::default();
    for &mode in Mode::ALL {
        let keys = kc.lane_keys(mode);
        assert_eq!(keys.len(), mode.key, "{} binds all lanes", mode.name);
        let mut lanes: Vec<usize> = keys.iter().map(|(_, l)| *l).collect();
        lanes.sort_unstable();
        lanes.dedup();
        assert_eq!(lanes.len(), mode.key, "{} has no duplicate/missing lane", mode.name);
    }
    for action in ControlAction::ALL {
        let bound = kc.control_key(action).is_some();
        assert_eq!(bound, !UNBOUND_BY_DEFAULT.contains(&action), "{action:?} default binding");
    }
}

/// The controls a fresh key config leaves unbound: the hidden band, and every control that was
/// added after the ones a run cannot do without, but for the two keys of the judge offset. Each is
/// a key a player asks for in the key editor rather than one shipped where a stray press in a run
/// would move something -- the switch that lets the offset adjust itself above all. The modifier
/// for the finer step is among them because no one key is free on every shipped layout: unbound,
/// it is the shift that plays no lane.
const UNBOUND_BY_DEFAULT: [ControlAction; 9] = [
    ControlAction::HiddenUp,
    ControlAction::HiddenDown,
    ControlAction::LiftToggle,
    ControlAction::HiddenToggle,
    ControlAction::GreenUp,
    ControlAction::GreenDown,
    ControlAction::KeepGreenToggle,
    ControlAction::AutoAdjustToggle,
    ControlAction::Fine,
];

#[test]
fn rebinding_a_control_round_trips_via_ron() {
    let mut kc = KeyConfig::default();
    kc.set_control(ControlAction::HiSpeedUp, KeyCode::Equal);
    let s = ron::ser::to_string_pretty(&kc, ron::ser::PrettyConfig::default()).unwrap();
    let back: KeyConfig = ron::from_str(&s).unwrap();
    assert_eq!(back.control_key(ControlAction::HiSpeedUp), Some(KeyCode::Equal));
}

#[test]
fn partial_control_block_keeps_edits_and_fills_rest() {
    let kc: KeyConfig = ron::from_str(r#"(controls: (hispeed_up: "Q"))"#).expect("partial control block must parse");
    assert_eq!(kc.control_key(ControlAction::HiSpeedUp), Some(KeyCode::KeyQ), "explicit field kept");
    assert_eq!(kc.control_key(ControlAction::HiSpeedDown), Some(KeyCode::ArrowDown), "missing field defaulted");
    assert_eq!(kc.lane_keys(Mode::BEAT_7K).len(), Mode::BEAT_7K.key, "lanes still complete");
}

#[test]
fn lane_keys_backfills_short_and_invalid_rows() {
    let mut lanes = BTreeMap::new();
    lanes.insert("7K".to_string(), vec!["Z".into(), "BOGUS".into(), "X".into()]);
    let kc = KeyConfig { lanes, ..KeyConfig::default() };
    let keys = kc.lane_keys(Mode::BEAT_7K);
    assert_eq!(keys.len(), Mode::BEAT_7K.key, "every lane bound despite a short row with a bad token");
    let mut got: Vec<usize> = keys.iter().map(|(_, l)| *l).collect();
    got.sort_unstable();
    assert_eq!(got, (0..Mode::BEAT_7K.key).collect::<Vec<_>>(), "no lane left unbound");
}

#[test]
fn collisions_flags_key_shared_by_control_and_lane() {
    let mut kc = KeyConfig::default();
    kc.set_control(ControlAction::CoverUp, KeyCode::KeyZ);
    let dup = collisions(&kc, Mode::BEAT_7K);
    assert!(dup.contains(&KeyCode::KeyZ), "shared key reported as a collision");
    assert!(!dup.contains(&KeyCode::ArrowDown), "an unshared key is not a collision");
}

#[test]
fn key_from_name_is_case_insensitive() {
    assert_eq!(key_from_name("a"), Some(KeyCode::KeyA));
    assert_eq!(key_from_name("A"), Some(KeyCode::KeyA));
    assert_eq!(key_from_name("space"), Some(KeyCode::Space));
    assert_eq!(key_from_name("SpAcE"), Some(KeyCode::Space));
    assert_eq!(key_from_name("lshift"), Some(KeyCode::ShiftLeft));
}

#[test]
fn key_from_name_unknown_token_is_none() {
    for bad in ["", "BOGUS", "F13", "  ", "AA", "KEYA", "RETURN", "ESCAPE"] {
        assert_eq!(key_from_name(bad), None, "{bad:?} is not a known key token");
    }
}

#[test]
fn key_from_name_aliases_map_to_same_code() {
    assert_eq!(key_from_name("SHIFT"), key_from_name("LSHIFT"));
    assert_eq!(key_from_name("CTRL"), key_from_name("LCTRL"));
    assert_eq!(key_from_name("CONTROL"), key_from_name("LCTRL"));
    assert_eq!(key_from_name("QUOTE"), key_from_name("APOSTROPHE"));
}

#[test]
fn key_name_round_trips_through_key_from_name_for_every_token() {
    let codes = [
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
        KeyCode::Space,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::Digit0,
        KeyCode::Comma,
        KeyCode::Period,
        KeyCode::Slash,
        KeyCode::Semicolon,
        KeyCode::Quote,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::BracketLeft,
        KeyCode::BracketRight,
        KeyCode::Minus,
        KeyCode::Equal,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Backslash,
    ];
    for code in codes {
        let tok = key_name(code);
        assert_ne!(tok, "?", "{code:?} should have a real token, not the sentinel");
        assert_eq!(key_from_name(tok), Some(code), "{code:?} round-trips via {tok:?}");
    }
}

#[test]
fn key_name_unmapped_code_is_question_mark() {
    assert_eq!(key_name(KeyCode::Escape), "?", "a code with no token gets the sentinel");
    assert_eq!(key_from_name(key_name(KeyCode::Escape)), None, "and the sentinel does not round-trip back to a code");
}

#[test]
fn default_keys_for_mode_binds_every_lane_once() {
    for &mode in Mode::ALL {
        let binds = default_keys_for_mode(mode);
        assert_eq!(binds.len(), mode.key, "{} default has one bind per lane", mode.name);
        let mut lanes: Vec<usize> = binds.iter().map(|(_, l)| *l).collect();
        lanes.sort_unstable();
        assert_eq!(lanes, (0..mode.key).collect::<Vec<_>>(), "{} covers lanes 0..{}", mode.name, mode.key);
        for (code, _) in &binds {
            assert_ne!(key_name(*code), "?", "{} default key {code:?} is a token key_name and key_from_name understand", mode.name);
        }
    }
}

#[test]
fn default_keys_have_no_duplicate_keys_within_a_mode() {
    for &mode in Mode::ALL {
        let mut codes: Vec<KeyCode> = default_keys_for_mode(mode).into_iter().map(|(c, _)| c).collect();
        let n = codes.len();
        codes.sort_unstable_by_key(|c| format!("{c:?}"));
        codes.dedup();
        assert_eq!(codes.len(), n, "{} default lane keys are all distinct", mode.name);
    }
}

/// `mode_config_key` and `default_keys_for_mode` dispatch on `mode.name`, so a synthetic mode that
/// is in neither match takes the fallback arm and comes out BEAT_7K-shaped.
#[test]
fn unknown_mode_falls_back_to_seven_lane_default() {
    let custom = Mode { name: "CUSTOM_8K", key: 8, ..Mode::BEAT_7K };
    assert_eq!(mode_config_key(custom), "CUSTOM_8K", "unknown mode keyed by its own name");
    let binds = default_keys_for_mode(custom);
    assert_eq!(binds.len(), 8, "fallback default is 8 lanes");
}

#[test]
fn mode_config_keys_are_unique_across_all_modes() {
    let mut keys: Vec<&str> = Mode::ALL.iter().map(|&m| mode_config_key(m)).collect();
    let n = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), n, "no two modes share a config key");
}

#[test]
fn lane_keys_empty_row_falls_fully_back_to_default() {
    let mut lanes = BTreeMap::new();
    lanes.insert("7K".to_string(), Vec::<String>::new());
    let kc = KeyConfig { lanes, ..KeyConfig::default() };
    let keys = kc.lane_keys(Mode::BEAT_7K);
    assert_eq!(keys, default_keys_for_mode(Mode::BEAT_7K), "empty row == pure default");
}

#[test]
fn lane_keys_ignores_extra_trailing_entries_beyond_mode_key() {
    let mut row: Vec<String> = vec![String::new(); Mode::BEAT_5K.key];
    row.push("A".into());
    row.push("B".into());
    let mut lanes = BTreeMap::new();
    lanes.insert("5K".to_string(), row);
    let kc = KeyConfig { lanes, ..KeyConfig::default() };
    let keys = kc.lane_keys(Mode::BEAT_5K);
    assert_eq!(keys.len(), Mode::BEAT_5K.key, "trailing out-of-range entries are dropped");
    assert!(keys.iter().all(|(_, l)| *l < Mode::BEAT_5K.key), "no lane index exceeds mode.key");
}

#[test]
fn lane_keys_override_replaces_default_for_that_lane() {
    let mut kc = KeyConfig::default();
    kc.set_lane(Mode::BEAT_7K, 0, KeyCode::KeyP);
    let keys = kc.lane_keys(Mode::BEAT_7K);
    let lane0 = keys.iter().find(|(_, l)| *l == 0).map(|(c, _)| *c);
    assert_eq!(lane0, Some(KeyCode::KeyP), "explicit lane binding overrides the default");
    assert_eq!(keys.len(), Mode::BEAT_7K.key, "still complete");
}

#[test]
fn lane_keys_returned_in_ascending_lane_order() {
    for &mode in Mode::ALL {
        let keys = KeyConfig::default().lane_keys(mode);
        let lanes: Vec<usize> = keys.iter().map(|(_, l)| *l).collect();
        let mut sorted = lanes.clone();
        sorted.sort_unstable();
        assert_eq!(lanes, sorted, "{} lanes returned in ascending order", mode.name);
    }
}

#[test]
fn collisions_empty_when_no_key_shared() {
    let kc = KeyConfig::default();
    for &mode in Mode::ALL {
        assert!(collisions(&kc, mode).is_empty(), "{} arrow/bracket controls never share a key with a letter/shift lane", mode.name);
    }
}

#[test]
fn collisions_detects_two_controls_sharing_a_key() {
    let mut kc = KeyConfig::default();
    kc.set_control(ControlAction::HiSpeedUp, KeyCode::KeyP);
    kc.set_control(ControlAction::HiSpeedDown, KeyCode::KeyP);
    let dup = collisions(&kc, Mode::BEAT_7K);
    assert!(dup.contains(&KeyCode::KeyP), "two controls on the same key collide");
}

#[test]
fn collisions_detects_two_lanes_sharing_a_key() {
    let mut kc = KeyConfig::default();
    kc.set_lane(Mode::BEAT_7K, 0, KeyCode::KeyP);
    kc.set_lane(Mode::BEAT_7K, 1, KeyCode::KeyP);
    let dup = collisions(&kc, Mode::BEAT_7K);
    assert!(dup.contains(&KeyCode::KeyP), "two lanes on the same key collide");
}

#[test]
fn set_lane_grows_a_short_or_missing_row() {
    let mut kc = KeyConfig::default();
    kc.lanes.remove(mode_config_key(Mode::BEAT_7K));
    kc.set_lane(Mode::BEAT_7K, 5, KeyCode::KeyP);
    assert_eq!(kc.lane_token(Mode::BEAT_7K, 5), "P", "a missing row is created from scratch");
    kc.lanes.insert("EXTRA".into(), vec!["A".into()]);
    let custom = Mode { name: "EXTRA", key: 1, ..Mode::BEAT_7K };
    kc.set_lane(custom, 4, KeyCode::KeyB);
    assert_eq!(kc.lane_token(custom, 4), "B", "a lane past the row's length resizes it");
    assert_eq!(kc.lane_token(custom, 2), "", "gap lanes resized to empty token");
}

#[test]
fn lane_token_out_of_range_is_empty_string() {
    let kc = KeyConfig::default();
    assert_eq!(kc.lane_token(Mode::BEAT_7K, 999), "", "out-of-range lane is empty");
}

#[test]
fn control_token_matches_default_strings() {
    let kc = KeyConfig::default();
    assert_eq!(kc.control_token(ControlAction::HiSpeedUp), "UP");
    assert_eq!(kc.control_token(ControlAction::HiSpeedDown), "DOWN");
    assert_eq!(kc.control_token(ControlAction::CoverUp), "RIGHT");
    assert_eq!(kc.control_token(ControlAction::CoverDown), "LEFT");
    assert_eq!(kc.control_token(ControlAction::LiftUp), "RBRACKET");
    assert_eq!(kc.control_token(ControlAction::LiftDown), "LBRACKET");
    assert_eq!(kc.control_token(ControlAction::HiddenUp), "");
    assert_eq!(kc.control_token(ControlAction::HiddenDown), "");
}

#[test]
fn control_action_all_has_distinct_nonempty_labels() {
    assert_eq!(ControlAction::ALL.len(), 20);
    let mut labels: Vec<&str> = ControlAction::ALL.iter().map(|a| a.label()).collect();
    for l in &labels {
        assert!(!l.is_empty(), "label non-empty");
    }
    let n = labels.len();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), n, "labels distinct");
}

#[test]
fn full_keyconfig_ron_round_trip_preserves_all_modes_and_controls() {
    let mut kc = KeyConfig::default();
    kc.set_lane(Mode::BEAT_7K, 3, KeyCode::KeyP);
    kc.set_control(ControlAction::LiftUp, KeyCode::Equal);
    let s = ron::ser::to_string_pretty(&kc, ron::ser::PrettyConfig::default()).unwrap();
    let back: KeyConfig = ron::from_str(&s).unwrap();
    for &mode in Mode::ALL {
        assert_eq!(back.lane_keys(mode), kc.lane_keys(mode), "{} lanes survive RON", mode.name);
    }
    for a in ControlAction::ALL {
        assert_eq!(back.control_key(a), kc.control_key(a), "{a:?} survives RON");
    }
}

#[test]
fn empty_ron_unit_parses_to_full_defaults() {
    let kc: KeyConfig = ron::from_str("()").expect("empty unit parses via serde(default)");
    let def = KeyConfig::default();
    for &mode in Mode::ALL {
        assert_eq!(kc.lane_keys(mode), def.lane_keys(mode), "{} defaulted", mode.name);
    }
    for a in ControlAction::ALL {
        assert_eq!(kc.control_key(a), def.control_key(a));
    }
}

#[test]
fn load_missing_file_writes_defaults() {
    let dir = std::env::temp_dir().join(format!("rbms_kc_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("keyconfig.ron");
    assert!(!path.exists());
    let kc = KeyConfig::load(&path);
    assert!(path.exists(), "load() of a missing file writes the defaults out");
    assert_eq!(kc.control_key(ControlAction::HiSpeedUp), Some(KeyCode::ArrowUp));
    let kc2 = KeyConfig::load(&path);
    assert_eq!(kc2.lane_keys(Mode::BEAT_7K), kc.lane_keys(Mode::BEAT_7K), "the written file re-loads to an equivalent config");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_writes_atomically_and_leaves_no_temp_file() {
    let dir = std::env::temp_dir().join(format!("rbms_kc_atomic_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("nested/keyconfig.ron");
    KeyConfig::default().save(&path);
    assert!(path.exists(), "save creates the file and its parent dir");
    let leftovers: Vec<String> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "no leftover temp file, got {leftovers:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_malformed_file_backs_up_and_returns_defaults() {
    let dir = std::env::temp_dir().join(format!("rbms_kc_bad_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("keyconfig.ron");
    std::fs::write(&path, "this is not ron {{{").unwrap();
    let kc = KeyConfig::load(&path);
    assert_eq!(kc.control_key(ControlAction::HiSpeedUp), Some(KeyCode::ArrowUp), "a malformed file falls back to the defaults");
    assert!(path.with_extension("ron.bak").exists(), "and the original is renamed to .bak rather than lost");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every scratch lane ships with a key for its other direction, the reference's
/// (`PlayModeConfig.KeyboardConfig.setKeyAssign`): the Control key beside the Shift key the lane
/// itself ships on. With one key the turntable is spun one way only, which leaves a shade that is
/// set with START and the turntable moving one way only. No other lane has one, and none of the
/// keys is one anything else of its mode ships on.
#[test]
fn every_scratch_lane_ships_with_a_key_for_its_other_direction() {
    let kc = KeyConfig::default();
    assert_eq!(kc.scratch_reverse_keys(Mode::BEAT_5K), vec![(KeyCode::ControlLeft, 5)]);
    assert_eq!(kc.scratch_reverse_keys(Mode::BEAT_7K), vec![(KeyCode::ControlLeft, 7)]);
    assert_eq!(kc.scratch_reverse_keys(Mode::BEAT_10K), vec![(KeyCode::ControlLeft, 5), (KeyCode::ControlRight, 11)]);
    assert_eq!(kc.scratch_reverse_keys(Mode::BEAT_14K), vec![(KeyCode::ControlLeft, 7), (KeyCode::ControlRight, 15)]);
    assert!(kc.scratch_reverse_keys(Mode::POPN_9K).is_empty(), "a mode with no turntable has no other direction");
    for &mode in Mode::ALL {
        assert_eq!(kc.scratch_reverse_keys(mode), default_reverse_keys_for_mode(mode), "{}", mode.name);
        assert!(collisions(&kc, mode).is_empty(), "{}", mode.name);
        for lane in (0..mode.key).filter(|lane| !mode.is_scratch(*lane)) {
            assert!(kc.scratch_reverse_token(mode, lane).is_empty(), "{} lane {lane}", mode.name);
        }
        for (code, lane) in default_reverse_keys_for_mode(mode) {
            assert_eq!(KeyConfig::default_key_of(mode, Bindable::ScratchReverse(lane)), Some(code));
            assert_eq!(kc.scratch_reverse_token(mode, lane), key_name(code));
        }
    }
    assert_eq!(kc.generation, crate::keyconfig::KEY_GENERATION);
}

/// A reverse binding is remembered per mode and per lane, and reads back as the key it was set to.
#[test]
fn a_reverse_binding_is_kept_for_the_scratch_lane_it_was_set_on() {
    let mut kc = KeyConfig::default();
    let mode = Mode::BEAT_7K;
    let scratch = (0..mode.key).find(|&lane| mode.is_scratch(lane)).expect("7K has a scratch lane");
    kc.set_scratch_reverse(mode, scratch, KeyCode::KeyQ);
    assert_eq!(kc.scratch_reverse_token(mode, scratch), "Q");
    assert_eq!(kc.scratch_reverse_keys(mode), vec![(KeyCode::KeyQ, scratch)]);
    assert_eq!(kc.scratch_reverse_keys(Mode::BEAT_5K), default_reverse_keys_for_mode(Mode::BEAT_5K), "a binding on one mode does not leak into another");
}

/// A reverse key is a binding like any other: sharing it with a lane or a control is a collision the
/// editor must flag, because the input path would silently shadow one of them.
#[test]
fn a_reverse_key_collides_with_the_lane_and_control_keys() {
    let mut kc = KeyConfig::default();
    let mode = Mode::BEAT_7K;
    let scratch = (0..mode.key).find(|&lane| mode.is_scratch(lane)).expect("7K has a scratch lane");
    let lane_key = kc.lane_keys(mode)[0].0;
    kc.set_scratch_reverse(mode, scratch, lane_key);
    assert!(collisions(&kc, mode).contains(&lane_key), "a reverse key that shadows a lane is flagged");

    let mut kc = KeyConfig::default();
    kc.set_scratch_reverse(mode, scratch, KeyCode::KeyQ);
    assert!(!collisions(&kc, mode).contains(&KeyCode::KeyQ), "a reverse key of its own is not a collision");
}

/// The reverse map is a second field, so a config written before it existed still parses and a
/// config carrying one round-trips through the file.
#[test]
fn the_reverse_map_survives_the_file_and_an_older_file_still_loads() {
    let mut kc = KeyConfig::default();
    let mode = Mode::BEAT_7K;
    let scratch = (0..mode.key).find(|&lane| mode.is_scratch(lane)).expect("7K has a scratch lane");
    kc.set_scratch_reverse(mode, scratch, KeyCode::KeyQ);
    let text = ron::ser::to_string_pretty(&kc, ron::ser::PrettyConfig::default()).expect("a key config serialises");
    let back: KeyConfig = ron::from_str(&text).expect("a key config parses");
    assert_eq!(back.scratch_reverse_keys(mode), vec![(KeyCode::KeyQ, scratch)]);

    let older: KeyConfig = ron::from_str(r#"(lanes: {"7K": ["Z"]}, controls: (hispeed_up: "UP"))"#).expect("a pre-reverse config parses");
    assert!(older.scratch_reverse.is_empty(), "a file that holds no reverse keys was read as holding the shipped ones");
    assert_eq!(older.scratch_reverse_keys(mode), default_reverse_keys_for_mode(mode), "a file from before the reverse keys reads them where they ship");
    assert_eq!(older.control_key(ControlAction::HiSpeedUp), Some(KeyCode::ArrowUp));
}

/// A reverse token naming a key that does not exist, or naming a lane that is not a scratch, is
/// dropped rather than bound: a bad file must not make a lane unhittable.
#[test]
fn a_reverse_binding_on_a_non_scratch_lane_or_an_unknown_key_is_dropped() {
    let mut lanes = BTreeMap::new();
    lanes.insert(mode_config_key(Mode::BEAT_7K).to_string(), vec!["Z".to_string()]);
    let mut scratch_reverse = BTreeMap::new();
    scratch_reverse.insert(mode_config_key(Mode::BEAT_7K).to_string(), vec!["Q".to_string(); Mode::BEAT_7K.key]);
    let kc = KeyConfig { lanes, scratch_reverse, ..KeyConfig::default() };
    let bound = kc.scratch_reverse_keys(Mode::BEAT_7K);
    assert!(bound.iter().all(|(_, lane)| Mode::BEAT_7K.is_scratch(*lane)), "only scratch lanes take a reverse key");

    let mut scratch_reverse = BTreeMap::new();
    scratch_reverse.insert(mode_config_key(Mode::BEAT_7K).to_string(), vec!["NOSUCHKEY".to_string(); Mode::BEAT_7K.key]);
    let kc = KeyConfig { scratch_reverse, ..KeyConfig::default() };
    assert!(kc.scratch_reverse_keys(Mode::BEAT_7K).is_empty(), "an unknown token leaves the lane unbound");
}

/// The pad block is a fourth field, so a key config written before a controller was readable still
/// parses — and gets a table that binds nothing, which plays exactly as it did.
#[test]
fn an_older_key_config_without_a_pad_block_still_loads() {
    let older: KeyConfig = ron::from_str(r#"(lanes: {"7K": ["Z"]}, controls: (hispeed_up: "UP"))"#).expect("a pre-pad config parses");
    assert_eq!(older.pad, PadConfig::default());
    for &mode in Mode::ALL {
        assert!((0..mode.key).all(|lane| older.pad.lane_binding(mode, lane).is_none()), "{} is unbound on the pad", mode.name);
    }
    assert_eq!(older.control_key(ControlAction::HiSpeedUp), Some(KeyCode::ArrowUp), "and the keyboard rows it did carry are kept");
}

/// A pad binding is stored with the rest of the key config and reads back as what it was set to.
#[test]
fn the_pad_block_survives_the_file() {
    let mode = Mode::BEAT_7K;
    let scratch = (0..mode.key).find(|&lane| mode.is_scratch(lane)).expect("7K has a scratch lane");
    let mut kc = KeyConfig::default();
    kc.pad.set_lane(mode, 0, Some(PadBinding::Button(4)));
    kc.pad.set_lane(mode, scratch, Some(PadBinding::AnalogScratch { axis: 1 }));
    kc.pad.analog_mode = AnalogMode::V2;

    let text = ron::ser::to_string_pretty(&kc, ron::ser::PrettyConfig::default()).expect("a key config serialises");
    let back: KeyConfig = ron::from_str(&text).expect("a key config parses");
    assert_eq!(back.pad, kc.pad);
    assert_eq!(back.lane_keys(mode), kc.lane_keys(mode), "and the keyboard rows are untouched by it");
}

/// A hand-edited pad block is pulled back into range as it is read, so an out-of-range threshold
/// cannot reach the turntable machines.
#[test]
fn loading_clamps_an_out_of_range_pad_block() {
    let dir = std::env::temp_dir().join(format!("rbms_kc_pad_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("keyconfig.ron");
    std::fs::write(&path, r#"(pad: (analog_threshold: 99999, debounce_ms: 4000, axis_deadzone: 8.0))"#).unwrap();

    let kc = KeyConfig::load(&path);
    assert_eq!(kc.pad.analog_threshold, *ANALOG_THRESHOLD_RANGE.end());
    assert_eq!(kc.pad.debounce_ms, *DEBOUNCE_MS_RANGE.end());
    assert!(kc.pad.axis_deadzone <= *AXIS_DEADZONE_RANGE.end());
    let _ = std::fs::remove_dir_all(&dir);
}

/// START and SELECT ship bound, one key each, so a fresh install can use them without opening the
/// editor.
#[test]
fn start_and_select_ship_bound_to_distinct_keys() {
    let kc = KeyConfig::default();
    assert_eq!(kc.control_key(ControlAction::Start), Some(KeyCode::KeyA));
    assert_eq!(kc.control_key(ControlAction::Select), Some(KeyCode::KeyW));
    assert_eq!(kc.control_token(ControlAction::Start), "A");
    assert_eq!(kc.control_token(ControlAction::Select), "W");
}

/// The reference implementation ships START on Q, but the IR ranking panel already reads Q as its
/// previous-profile key, so a held START would page the panel. The shipped keys must not be a key
/// that panel reads, and must not be a default lane or control key of any mode.
#[test]
fn the_shipped_start_and_select_keys_collide_with_nothing() {
    use crate::ir_ranking_view::panel_action;
    assert!(panel_action(KeyCode::KeyQ).is_some(), "Q is the key the reference START default would have fought over");

    let kc = KeyConfig::default();
    for action in [ControlAction::Start, ControlAction::Select] {
        let code = kc.control_key(action).expect("shipped bound");
        assert!(panel_action(code).is_none(), "{action:?} is a key the ranking panel reads");
        for &mode in Mode::ALL {
            assert!(collisions(&kc, mode).is_empty(), "{} has a shared key once {action:?} is counted", mode.name);
            assert!(default_keys_for_mode(mode).iter().all(|(lane_key, _)| *lane_key != code), "{action:?} sits on a default {} lane", mode.name);
        }
    }
}

/// A key config written before START and SELECT existed has no field for either; reading it fills
/// both from the defaults and keeps every field it did carry.
#[test]
fn an_older_key_config_without_start_and_select_still_loads_with_them_bound() {
    let older: KeyConfig = ron::from_str(
        r#"(
            lanes: {"7K": ["Z", "S", "X", "D", "C", "F", "V", "LSHIFT"]},
            scratch_reverse: {},
            controls: (
                hispeed_up: "UP", hispeed_down: "DOWN", cover_up: "RIGHT", cover_down: "LEFT",
                lift_up: "RBRACKET", lift_down: "LBRACKET", hidden_up: "", hidden_down: "",
            ),
            pad: (),
        )"#,
    )
    .expect("a config from before START and SELECT parses");
    assert_eq!(older.control_key(ControlAction::Start), Some(KeyCode::KeyA));
    assert_eq!(older.control_key(ControlAction::Select), Some(KeyCode::KeyW));
    assert_eq!(older.control_key(ControlAction::CoverUp), Some(KeyCode::ArrowRight), "and the controls it did carry are kept");
    assert_eq!(older.pad.control_binding(ControlAction::Start), KeyConfig::default().pad.control_binding(ControlAction::Start));
}

/// The same older file read from disk is not mistaken for a broken one: nothing is backed up and the
/// defaults are not written over it.
#[test]
fn loading_an_older_file_without_start_and_select_neither_backs_it_up_nor_rewrites_it() {
    let dir = std::env::temp_dir().join(format!("rbms_kc_old_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("keyconfig.ron");
    let text = r#"(controls: (hispeed_up: "UP", hispeed_down: "DOWN", cover_up: "RIGHT", cover_down: "LEFT", lift_up: "RBRACKET", lift_down: "LBRACKET"))"#;
    std::fs::write(&path, text).unwrap();

    let kc = KeyConfig::load(&path);
    assert_eq!(kc.control_key(ControlAction::Start), Some(KeyCode::KeyA));
    assert_eq!(kc.control_key(ControlAction::Select), Some(KeyCode::KeyW));
    assert!(!path.with_extension("ron.bak").exists(), "an older file is not a broken one");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "and it is left as the player wrote it");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A rebound START round-trips through the file, and the held-key set — runtime state — never
/// reaches it.
#[test]
fn a_rebound_start_survives_the_file_and_the_held_keys_do_not() {
    let mut kc = KeyConfig::default();
    kc.set_control(ControlAction::Start, KeyCode::KeyP);
    kc.held.press(KeyCode::KeyZ);
    let text = ron::ser::to_string_pretty(&kc, ron::ser::PrettyConfig::default()).expect("a key config serialises");
    assert!(!text.contains("held"), "the held keys leaked into the file: {text}");
    let back: KeyConfig = ron::from_str(&text).expect("a key config parses");
    assert_eq!(back.control_key(ControlAction::Start), Some(KeyCode::KeyP));
    assert!(!back.held.is_down(KeyCode::KeyZ));
}

/// START and SELECT share a key with a lane or with each other exactly like any other control, so
/// the editor flags them.
#[test]
fn start_and_select_collide_like_any_other_control() {
    let mut kc = KeyConfig::default();
    kc.set_control(ControlAction::Start, KeyCode::KeyZ);
    assert!(collisions(&kc, Mode::BEAT_7K).contains(&KeyCode::KeyZ), "START on a lane key is flagged");

    let mut kc = KeyConfig::default();
    kc.set_control(ControlAction::Select, KeyCode::KeyA);
    assert!(collisions(&kc, Mode::BEAT_7K).contains(&KeyCode::KeyA), "SELECT on START's key is flagged");
}

#[test]
fn start_and_select_are_the_controls_that_give_way_to_a_lane() {
    for action in ControlAction::ALL {
        let yields = matches!(action, ControlAction::Start | ControlAction::Select | ControlAction::CoverToggle);
        assert_eq!(action.yields_to_lanes(), yields, "{action:?}");
    }
}

#[test]
fn held_keys_follow_press_and_release() {
    let mut held = HeldKeys::default();
    assert!(!held.is_down(KeyCode::KeyZ));
    held.press(KeyCode::KeyZ);
    held.press(KeyCode::KeyZ);
    held.press(KeyCode::KeyX);
    assert!(held.is_down(KeyCode::KeyZ));
    assert_eq!(held.iter().count(), 2, "a key pressed twice is down once");
    held.release(KeyCode::KeyZ);
    assert!(!held.is_down(KeyCode::KeyZ), "one release is enough");
    assert!(held.is_down(KeyCode::KeyX));
    held.release(KeyCode::KeyQ);
    assert_eq!(held.iter().count(), 1, "releasing a key that was not down changes nothing");
}

/// Keys one to seven are indices 0 to 6 and the turntable is 7 forward and 8 backward, in every
/// mode that has a turntable on its first side.
#[test]
fn a_seven_key_layout_numbers_its_keys_the_way_the_reference_does() {
    let mode = Mode::BEAT_7K;
    for lane in 0..7 {
        assert_eq!(key_index_of(mode, lane, ScratchDir::Forward), Some(lane), "key {}", lane + 1);
    }
    assert_eq!(key_index_of(mode, 7, ScratchDir::Forward), Some(SCRATCH_FORWARD_INDEX));
    assert_eq!(key_index_of(mode, 7, ScratchDir::Backward), Some(SCRATCH_BACKWARD_INDEX));
    assert_eq!((SCRATCH_FORWARD_INDEX, SCRATCH_BACKWARD_INDEX, KEY_INDEX_COUNT), (7, 8, 9));
    assert_eq!(key_index_of(mode, 8, ScratchDir::Forward), None, "a lane the mode does not have has no index");
}

/// Five keys keep their positions and keep the turntable on 7 and 8, so a five-key player scrolls
/// with the same two inputs as a seven-key one.
#[test]
fn a_five_key_layout_keeps_the_turntable_on_seven_and_eight() {
    let mode = Mode::BEAT_5K;
    for lane in 0..5 {
        assert_eq!(key_index_of(mode, lane, ScratchDir::Forward), Some(lane));
    }
    assert_eq!(key_index_of(mode, 5, ScratchDir::Forward), Some(SCRATCH_FORWARD_INDEX));
    assert_eq!(key_index_of(mode, 5, ScratchDir::Backward), Some(SCRATCH_BACKWARD_INDEX));
}

/// Pop'n has no turntable: its nine buttons are 0 to 8.
#[test]
fn a_nine_button_layout_numbers_its_buttons_zero_to_eight() {
    let mode = Mode::POPN_9K;
    for lane in 0..9 {
        assert_eq!(key_index_of(mode, lane, ScratchDir::Forward), Some(lane), "button {}", lane + 1);
    }
}

/// A double-play mode repeats the first side's indices on the second (`BEAT_14K` in
/// `MusicSelectKeyProperty`), turntables included.
#[test]
fn a_double_play_layout_repeats_its_first_side_on_the_second() {
    for (mode, second_side_start, first_scratch, second_scratch) in [(Mode::BEAT_14K, 8, 7, 15), (Mode::BEAT_10K, 6, 5, 11)] {
        let keys_per_side = first_scratch;
        for offset in 0..keys_per_side {
            let first = key_index_of(mode, offset, ScratchDir::Forward);
            let second = key_index_of(mode, second_side_start + offset, ScratchDir::Forward);
            assert_eq!(first, Some(offset), "{} first side key {offset}", mode.name);
            assert_eq!(second, first, "{} second side key {offset}", mode.name);
        }
        for dir in [ScratchDir::Forward, ScratchDir::Backward] {
            assert_eq!(key_index_of(mode, first_scratch, dir), key_index_of(mode, second_scratch, dir), "{} turntables {dir:?}", mode.name);
        }
    }
}

/// The 24-key layout has two turntables and twenty-four keys; only the first seven keys can take
/// indices, because 7 and 8 belong to a turntable.
#[test]
fn a_layout_with_more_keys_than_indices_leaves_the_extra_ones_without_one() {
    let mode = Mode::KEYBOARD_24K;
    assert_eq!(key_index_of(mode, 6, ScratchDir::Forward), Some(6));
    assert_eq!(key_index_of(mode, 7, ScratchDir::Forward), None);
    assert_eq!(key_index_of(mode, 8, ScratchDir::Forward), None);
    assert_eq!(key_index_of(mode, 24, ScratchDir::Forward), Some(SCRATCH_FORWARD_INDEX));
}

/// Every index a mode hands out is inside the range the screens ask about.
#[test]
fn no_mode_hands_out_an_index_past_the_last_one() {
    for &mode in Mode::ALL {
        for lane in 0..mode.key {
            for dir in [ScratchDir::Forward, ScratchDir::Backward] {
                if let Some(index) = key_index_of(mode, lane, dir) {
                    assert!(index < KEY_INDEX_COUNT, "{} lane {lane} {dir:?} -> {index}", mode.name);
                }
            }
        }
    }
}

/// A key config as the application wrote it before the shortcuts and the newer controls could be
/// bound: its lanes, the ten controls it knew, and a pad block that names its controls by the words
/// they were then drawn with.
const FILE_FROM_BEFORE_SHORTCUTS: &str = r#"(
    lanes: {"7K": ["Q", "S", "X", "D", "C", "F", "V", "LSHIFT"]},
    scratch_reverse: {"7K": ["", "", "", "", "", "", "", "LCTRL"]},
    controls: (
        hispeed_up: "EQUAL", hispeed_down: "MINUS", cover_up: "RIGHT", cover_down: "LEFT",
        lift_up: "RBRACKET", lift_down: "LBRACKET", hidden_up: "", hidden_down: "",
        start: "A", select: "W",
    ),
    pad: (controls: {"COVER UP": Button(4), "HIDDEN+ DOWN": Button(5), "START": Button(9)}),
)"#;

fn shortcut(of: Shortcut) -> Bindable {
    Bindable::Shortcut(of)
}

fn control(of: ControlAction) -> Bindable {
    Bindable::Control(of)
}

/// Every key a shortcut or a control ships on has a word in the file, so a config that is written
/// out reads back onto the same keys, and so do the function keys and the editing keys a player can
/// now give one.
#[test]
fn every_key_that_ships_and_every_key_that_was_added_has_a_word_in_the_file() {
    let shipped = Shortcut::ALL.into_iter().map(Shortcut::default_key);
    let added = [
        KeyCode::F1,
        KeyCode::F5,
        KeyCode::F12,
        KeyCode::Tab,
        KeyCode::Enter,
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::Insert,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Backquote,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::Numpad0,
        KeyCode::Numpad9,
    ];
    for code in shipped.chain(added) {
        assert_ne!(key_name(code), UNNAMED_KEY, "{code:?} has no word");
        assert_eq!(key_from_name(key_name(code)), Some(code), "{code:?} does not read back");
    }
}

/// A fresh config and a file written before the shortcuts could be moved read every shortcut from
/// the key it ships on, the newer controls where they ship, and everything the file did hold as it
/// held it.
#[test]
fn a_file_from_before_the_shortcuts_reads_every_key_where_it_always_was() {
    let older: KeyConfig = ron::from_str(FILE_FROM_BEFORE_SHORTCUTS).expect("a config from before the shortcuts parses");
    for keys in [&KeyConfig::default(), &older] {
        for shortcut in Shortcut::ALL {
            assert_eq!(keys.shortcut_key(shortcut), Some(shortcut.default_key()), "{shortcut:?}");
        }
        assert_eq!(keys.control_key(ControlAction::CoverToggle), Some(KeyCode::Backslash));
        for action in [ControlAction::LiftToggle, ControlAction::GreenUp, ControlAction::KeepGreenToggle, ControlAction::AutoAdjustToggle] {
            assert_eq!(keys.control_key(action), None, "{action:?} ships with no key");
        }
        assert_eq!(keys.control_key(ControlAction::OffsetDown), Some(KeyCode::Digit9));
        assert_eq!(keys.control_key(ControlAction::OffsetUp), Some(KeyCode::Digit0));
    }
    assert!(older.shortcuts.is_empty(), "reading an older file stored a shortcut nobody moved");
    assert_eq!(older.control_key(ControlAction::HiSpeedUp), Some(KeyCode::Equal), "a control the file moved was put back");
    assert_eq!(older.control_key(ControlAction::CoverUp), Some(KeyCode::ArrowRight));
    assert_eq!(older.lane_keys(Mode::BEAT_7K)[0], (KeyCode::KeyQ, 0), "a lane the file moved was put back");
    assert_eq!(older.scratch_reverse_keys(Mode::BEAT_7K), vec![(KeyCode::ControlLeft, 7)]);
    assert_eq!(older.pad.control_binding(ControlAction::CoverUp), Some(PadBinding::Button(4)), "the pad block's own word for the control is not read");
    assert_eq!(older.pad.control_binding(ControlAction::HiddenDown), Some(PadBinding::Button(5)));
    assert_eq!(older.pad.control_binding(ControlAction::Start), Some(PadBinding::Button(9)));
}

/// The same file read from disk is not mistaken for a broken one, and what the new version writes
/// reads back the same.
#[test]
fn a_file_from_before_the_shortcuts_loads_from_disk_and_survives_being_written_again() {
    let dir = std::env::temp_dir().join(format!("rbms_kc_before_shortcuts_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("keyconfig.ron");
    std::fs::write(&path, FILE_FROM_BEFORE_SHORTCUTS).unwrap();
    let loaded = KeyConfig::load(&path);
    assert!(!path.with_extension("ron.bak").exists(), "the older file was backed up as if it were broken");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), FILE_FROM_BEFORE_SHORTCUTS, "the older file was rewritten on load");
    assert_eq!(loaded.control_key(ControlAction::HiSpeedUp), Some(KeyCode::Equal));

    loaded.save(&path);
    let again = KeyConfig::load(&path);
    assert_eq!(again.bound_keys(Mode::BEAT_7K), loaded.bound_keys(Mode::BEAT_7K));
    assert_eq!(again.pad, loaded.pad);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A shortcut the player moved is written out and read back on its new key, one left without a key
/// stays without one, and one put back on its shipped key is no longer stored at all.
#[test]
fn a_moved_shortcut_survives_the_file_and_one_left_without_a_key_stays_without() {
    let mode = Mode::BEAT_7K;
    let mut keys = KeyConfig::default();
    keys.bind(mode, shortcut(Shortcut::Search), Some(KeyCode::F6));
    keys.bind(mode, shortcut(Shortcut::Folders), None);
    keys.bind(mode, shortcut(Shortcut::Tables), Some(KeyCode::F7));
    keys.restore(mode, shortcut(Shortcut::Tables));
    let text = ron::ser::to_string_pretty(&keys, ron::ser::PrettyConfig::default()).unwrap();
    let back: KeyConfig = ron::from_str(&text).unwrap();
    assert_eq!(back.shortcut_key(Shortcut::Search), Some(KeyCode::F6));
    assert_eq!(back.shortcut_on(ShortcutScreen::Browser, KeyCode::F6), Some(Shortcut::Search));
    assert_eq!(back.shortcut_on(ShortcutScreen::Browser, KeyCode::Slash), None, "the key the search shipped on still opens it");
    assert_eq!(back.shortcut_key(Shortcut::Folders), None);
    assert_eq!(back.shortcut_key(Shortcut::Tables), Some(Shortcut::Tables.default_key()));
    assert!(!back.shortcuts.contains_key(Shortcut::Tables.config_key()), "a shortcut on its shipped key is stored");
}

/// A word in the file that names no key leaves the shortcut on the key it ships on, so a hand-edited
/// file cannot take away the way into a screen.
#[test]
fn a_word_that_names_no_key_reads_the_shipped_key() {
    let edited: KeyConfig = ron::from_str(r#"(shortcuts: {"settings": "NO SUCH KEY", "skin_menu": ""})"#).expect("the file parses");
    assert_eq!(edited.shortcut_key(Shortcut::Settings), Some(Shortcut::Settings.default_key()));
    assert_eq!(edited.shortcut_key(Shortcut::SkinMenu), None, "an empty word is the player taking the key away");
}

/// A file written before a shortcut existed has no word for it. When the player had already put
/// another shortcut of the same screen on the key the new one ships on, the player's choice keeps
/// the key and the new shortcut is read from none, which the key editor shows as a row to fill in:
/// a key the file names is never taken over by one it does not.
#[test]
fn a_shortcut_the_file_says_nothing_of_leaves_its_shipped_key_to_the_shortcut_the_player_put_there() {
    let shipped = Shortcut::PlayMenu.default_key();
    let word = key_name(shipped);
    let older: KeyConfig = ron::from_str(&format!(r#"(shortcuts: {{"folders": "{word}"}})"#)).expect("the file parses");
    assert_eq!(older.shortcut_key(Shortcut::Folders), Some(shipped));
    assert_eq!(older.shortcut_key(Shortcut::PlayMenu), None, "a shortcut the file has no word for took a key the player gave another");
    assert_eq!(older.shortcut_on(ShortcutScreen::Browser, shipped), Some(Shortcut::Folders));
    assert!(collisions(&older, Mode::BEAT_7K).is_empty(), "two shortcuts of one screen are read from one key");

    let elsewhere: KeyConfig = ron::from_str(&format!(r#"(shortcuts: {{"replay_pause": "{word}"}})"#)).expect("the file parses");
    assert_eq!(elsewhere.shortcut_key(Shortcut::PlayMenu), Some(shipped), "a shortcut of another screen, which is never read with it, took the key away");

    let mut restored = older.clone();
    restored.restore(Mode::BEAT_7K, shortcut(Shortcut::PlayMenu));
    assert_eq!(restored.shortcut_key(Shortcut::PlayMenu), None, "putting a shortcut back took the key from the one the player put there");
    restored.restore(Mode::BEAT_7K, shortcut(Shortcut::Folders));
    assert_eq!(restored.shortcut_key(Shortcut::PlayMenu), Some(shipped), "the key is not the shortcut's again once the other left it");
    assert_eq!(restored.shortcut_key(Shortcut::Folders), Some(Shortcut::Folders.default_key()));
}

/// The key that fetches the difficulty tables again is a shortcut of the browser like the others. A
/// key config written before it existed reads as it always did and leaves the reload on the key it
/// ships on -- unless the player had put another shortcut of the browser there, which keeps it. The
/// file gains nothing until the player moves the reload.
#[test]
fn a_key_config_written_before_the_table_reload_reads_as_it_did() {
    let shipped = Shortcut::ReloadTables.default_key();
    assert_eq!((shipped, Shortcut::ReloadTables.config_key(), Shortcut::ReloadTables.label()), (KeyCode::F6, "reload_tables", "RELOAD DIFFICULTY TABLES"));
    assert_eq!((Shortcut::ReloadTables.group(), Shortcut::ReloadTables.screen()), (crate::keyconfig::ShortcutGroup::Browser, ShortcutScreen::Browser));
    assert!(Shortcut::ReloadTables.takes_a_pad(), "a controller can be given the reload");

    let earlier = r#"(lanes: {"7k": ["LSHIFT", "Z", "S", "X", "D", "C", "F", "V"]}, shortcuts: {"tables": "F7", "folders": ""})"#;
    let older: KeyConfig = ron::from_str(earlier).expect("an earlier file parses");
    assert_eq!(older.shortcut_key(Shortcut::Tables), Some(KeyCode::F7), "what the file says is read as before");
    assert_eq!(older.shortcut_key(Shortcut::Folders), None);
    assert_eq!(older.shortcut_key(Shortcut::ReloadTables), Some(shipped), "a file with no word for the reload leaves it on the key it ships on");
    assert_eq!(older.shortcut_on(ShortcutScreen::Browser, shipped), Some(Shortcut::ReloadTables));
    assert!(collisions(&older, Mode::BEAT_7K).is_empty());

    let taken: KeyConfig = ron::from_str(r#"(shortcuts: {"filter": "F6"})"#).expect("the file parses");
    assert_eq!(taken.shortcut_key(Shortcut::Filter), Some(shipped));
    assert_eq!(taken.shortcut_key(Shortcut::ReloadTables), None, "the reload took a key the player had given another shortcut of the browser");
    assert_eq!(taken.shortcut_on(ShortcutScreen::Browser, shipped), Some(Shortcut::Filter));

    let written = ron::ser::to_string(&older).expect("the key config is written");
    assert!(!written.contains("reload_tables"), "a reload nobody moved was written to the file: {written}");
    let mut moved = older.clone();
    moved.bind(Mode::BEAT_7K, shortcut(Shortcut::ReloadTables), Some(KeyCode::F10));
    let back: KeyConfig = ron::from_str(&ron::ser::to_string(&moved).expect("the key config is written")).expect("it reads back");
    assert_eq!(back.shortcut_key(Shortcut::ReloadTables), Some(KeyCode::F10));
    assert_eq!(back.shortcut_key(Shortcut::Tables), Some(KeyCode::F7));
}

/// The key that plays the chart of a replay being watched on the replay's layout is a key of the
/// replay under analysis. It ships on a key no shipped layout plays a lane on and no control is read
/// from, and a key config written before it existed reads as it did: the key is the new one's unless
/// the player had put another key of the replay there, which keeps it.
#[test]
fn the_layout_play_key_ships_on_a_free_key_and_an_earlier_file_reads_as_it_did() {
    let shipped = Shortcut::ReplayPlayLayout.default_key();
    assert_eq!((shipped, Shortcut::ReplayPlayLayout.config_key()), (KeyCode::KeyJ, "replay_play_layout"));
    assert_eq!((Shortcut::ReplayPlayLayout.group(), Shortcut::ReplayPlayLayout.screen()), (crate::keyconfig::ShortcutGroup::Replay, ShortcutScreen::Replay));
    let fresh = KeyConfig::default();
    for &mode in Mode::ALL {
        let lanes = default_keys_for_mode(mode).into_iter().chain(default_reverse_keys_for_mode(mode));
        assert!(lanes.into_iter().all(|(lane_key, _)| lane_key != shipped), "{} plays a lane on it", mode.name);
        assert!(collisions(&fresh, mode).is_empty(), "{}", mode.name);
    }
    assert!(ControlAction::ALL.into_iter().all(|action| fresh.control_key(action) != Some(shipped)), "a control is read from it");
    assert_eq!(fresh.shortcut_on(ShortcutScreen::Replay, shipped), Some(Shortcut::ReplayPlayLayout));

    let earlier: KeyConfig = ron::from_str(&format!(r#"(shortcuts: {{"replay_pause": "{}"}})"#, key_name(KeyCode::Enter))).expect("the file parses");
    assert_eq!(earlier.shortcut_key(Shortcut::ReplayPlayLayout), Some(shipped), "a file with no word for it leaves it on the key it ships on");
    let taken: KeyConfig = ron::from_str(&format!(r#"(shortcuts: {{"replay_back": "{}"}})"#, key_name(shipped))).expect("the file parses");
    assert_eq!(taken.shortcut_key(Shortcut::ReplayPlayLayout), None, "it took a key the player had given another key of the replay");
    assert_eq!(taken.shortcut_on(ShortcutScreen::Replay, shipped), Some(Shortcut::ReplayBack));
}

/// The shortcuts and the controls are stored under names that do not change with what the rows are
/// called on screen, each name is used once, and the ten controls an older pad block names keep the
/// names it knows them by.
#[test]
fn the_names_in_the_file_are_stable_and_distinct() {
    let older = [
        (ControlAction::HiSpeedUp, "HI-SPEED UP"),
        (ControlAction::HiSpeedDown, "HI-SPEED DOWN"),
        (ControlAction::CoverUp, "COVER UP"),
        (ControlAction::CoverDown, "COVER DOWN"),
        (ControlAction::LiftUp, "LIFT UP"),
        (ControlAction::LiftDown, "LIFT DOWN"),
        (ControlAction::HiddenUp, "HIDDEN+ UP"),
        (ControlAction::HiddenDown, "HIDDEN+ DOWN"),
        (ControlAction::Start, "START"),
        (ControlAction::Select, "SELECT"),
    ];
    for (action, name) in older {
        assert_eq!(action.config_key(), name, "{action:?}");
    }
    let controls: std::collections::HashSet<&str> = ControlAction::ALL.into_iter().map(ControlAction::config_key).collect();
    assert_eq!(controls.len(), ControlAction::ALL.len());
    let shortcuts: std::collections::HashSet<&str> = Shortcut::ALL.into_iter().map(Shortcut::config_key).collect();
    assert_eq!(shortcuts.len(), Shortcut::ALL.len());
}

/// The rows of the lane cover say which way the cover moves and what that does: the amount going up
/// is the cover coming down over more of the field.
#[test]
fn the_cover_rows_say_which_way_the_cover_moves() {
    assert_eq!(ControlAction::CoverUp.label(), "LANE COVER DOWN (COVER MORE)");
    assert_eq!(ControlAction::CoverDown.label(), "LANE COVER UP (COVER LESS)");
    assert_eq!(ControlAction::CoverToggle.label(), "LANE COVER ON/OFF");
}

/// Two things read on one screen cannot share a key, and two things read on different screens can.
#[test]
fn a_key_is_refused_only_to_something_read_with_what_already_has_it() {
    let mode = Mode::BEAT_7K;
    let keys = KeyConfig::default();
    let refused = [
        (shortcut(Shortcut::Search), KeyCode::F2, shortcut(Shortcut::Filter), "two shortcuts of the browser"),
        (shortcut(Shortcut::Search), KeyCode::Digit1, shortcut(Shortcut::RefMode), "a shortcut and a number key of the browser"),
        (shortcut(Shortcut::Search), KeyCode::KeyA, control(ControlAction::Start), "a shortcut of the browser and START"),
        (control(ControlAction::Select), KeyCode::KeyO, shortcut(Shortcut::Folders), "SELECT and a shortcut of the browser"),
        (shortcut(Shortcut::Retry), KeyCode::KeyN, shortcut(Shortcut::NextSong), "the two keys of the result"),
        (shortcut(Shortcut::ReplayPause), KeyCode::ArrowUp, control(ControlAction::HiSpeedUp), "a replay key and a control of a run"),
        (control(ControlAction::HiSpeedUp), KeyCode::Space, shortcut(Shortcut::ReplayPause), "a control of a run and a replay key"),
        (control(ControlAction::CoverToggle), KeyCode::ArrowLeft, control(ControlAction::CoverDown), "two controls of a run"),
        (control(ControlAction::CoverToggle), KeyCode::KeyZ, Bindable::Lane(0), "a control and a lane"),
        (Bindable::Lane(0), KeyCode::KeyF, Bindable::Lane(5), "two lanes"),
        (Bindable::ScratchReverse(7), KeyCode::Backslash, control(ControlAction::CoverToggle), "a reverse key and a control"),
    ];
    for (target, code, holder, what) in refused {
        assert_eq!(keys.clash(mode, target, code), Some(Clash::With(holder)), "{what}");
    }
    let allowed = [
        (shortcut(Shortcut::Search), KeyCode::BracketLeft, "a shortcut of the browser and a control of a run"),
        (shortcut(Shortcut::Retry), KeyCode::KeyR, "a result key on the key a browser shortcut has"),
        (shortcut(Shortcut::Retry), KeyCode::Slash, "a result key and a browser shortcut"),
        (shortcut(Shortcut::ReplayPause), KeyCode::KeyZ, "a replay key and a lane, which a replay does not read"),
        (control(ControlAction::HiSpeedUp), KeyCode::F2, "a control of a run and a browser shortcut"),
        (control(ControlAction::HiSpeedUp), KeyCode::ArrowUp, "a control and the key it already has"),
        (Bindable::Lane(0), KeyCode::Slash, "a lane and a browser shortcut"),
    ];
    for (target, code, what) in allowed {
        assert_eq!(keys.clash(mode, target, code), None, "{what}");
    }
}

/// The keys a screen keeps for itself cannot be given to anything it reads, and can be given to what
/// another screen reads.
#[test]
fn a_screens_fixed_keys_cannot_be_given_to_what_it_reads() {
    let mode = Mode::BEAT_7K;
    let keys = KeyConfig::default();
    assert!(fixed_keys_of(ShortcutScreen::Browser).contains(&GUIDE_KEY), "the guide key is the browser's own");
    for screen in [ShortcutScreen::Browser, ShortcutScreen::Result, ShortcutScreen::Replay] {
        let read_there = Shortcut::ALL.into_iter().find(|shortcut| shortcut.screen() == screen).expect("every screen reads a shortcut");
        for code in fixed_keys_of(screen) {
            assert_eq!(keys.clash(mode, shortcut(read_there), *code), Some(Clash::Fixed), "{read_there:?} onto {code:?}");
        }
    }
    assert_eq!(keys.clash(mode, shortcut(Shortcut::ReplayPause), KeyCode::Enter), None, "Enter is not the replay's");
    assert_eq!(keys.clash(mode, control(ControlAction::CoverToggle), KeyCode::Enter), None, "Enter is not a run's");
    assert_eq!(keys.clash(mode, control(ControlAction::CoverToggle), KeyCode::Escape), Some(Clash::Fixed));
    assert_eq!(keys.clash(mode, Bindable::Lane(0), KeyCode::Escape), Some(Clash::Fixed));
}

/// As shipped nothing clashes in any mode, so a fresh install opens the key editor with no row in
/// red.
#[test]
fn the_shipped_keys_clash_with_nothing_in_any_mode() {
    let keys = KeyConfig::default();
    for &mode in Mode::ALL {
        for (target, key) in keys.bound_keys(mode) {
            let Some(code) = key else { continue };
            assert_eq!(keys.clash(mode, target, code), None, "{} {target:?} on {code:?}", mode.name);
        }
    }
}

/// A shortcut of the browser on a key that also plays a lane is not refused -- the shipped layout
/// has one -- and the editor is told which lane it is.
#[test]
fn a_browser_shortcut_on_a_lane_key_is_named_rather_than_refused() {
    let keys = KeyConfig::default();
    assert_eq!(keys.lane_sharing(Mode::BEAT_7K, shortcut(Shortcut::Favourite)), Some(5), "F stars a chart and plays key six");
    assert_eq!(keys.lane_sharing(Mode::BEAT_7K, shortcut(Shortcut::OptionsHold)), Some(7), "left shift holds the overlay and is the turntable");
    assert_eq!(keys.lane_sharing(Mode::BEAT_7K, shortcut(Shortcut::Search)), None);
    assert_eq!(keys.lane_sharing(Mode::BEAT_7K, shortcut(Shortcut::Retry)), None, "the result does not read the lanes as the browser does");
    assert_eq!(keys.lane_sharing(Mode::BEAT_7K, control(ControlAction::Start)), None);
}

/// Taking a key away leaves a control or a shortcut with none, and puts a lane back on the key it
/// ships with, since a lane is never left unhittable.
#[test]
fn clearing_leaves_a_control_unbound_and_a_lane_on_its_shipped_key() {
    let mode = Mode::BEAT_7K;
    let mut keys = KeyConfig::default();
    keys.set_lane(mode, 2, KeyCode::KeyP);
    keys.bind(mode, Bindable::Lane(2), None);
    assert_eq!(keys.lane_keys(mode)[2], (KeyCode::KeyX, 2));
    keys.bind(mode, control(ControlAction::CoverUp), None);
    assert_eq!(keys.control_key(ControlAction::CoverUp), None);
    keys.bind(mode, Bindable::ScratchReverse(7), None);
    assert!(keys.scratch_reverse_keys(mode).is_empty(), "a reverse key that is taken away is left with none");
    keys.restore(mode, Bindable::ScratchReverse(7));
    assert_eq!(keys.scratch_reverse_keys(mode), vec![(KeyCode::ControlLeft, 7)]);
}

/// Restoring puts one row back on the key it ships with, and restoring everything puts every key
/// and every controller binding back while leaving the controller's own tuning alone.
#[test]
fn restoring_puts_a_row_or_everything_back_as_shipped() {
    let mode = Mode::BEAT_7K;
    let mut keys = KeyConfig::default();
    keys.bind(mode, control(ControlAction::CoverUp), Some(KeyCode::KeyP));
    keys.bind(mode, control(ControlAction::LiftToggle), Some(KeyCode::KeyU));
    keys.bind(mode, Bindable::Lane(0), Some(KeyCode::KeyQ));
    keys.bind(mode, shortcut(Shortcut::Search), Some(KeyCode::F6));
    keys.pad.set_lane(mode, 0, Some(PadBinding::Button(1)));
    keys.pad.set_shortcut(Shortcut::Search, Some(PadBinding::Button(2)));
    keys.pad.debounce_ms = 40;

    keys.restore(mode, control(ControlAction::CoverUp));
    assert_eq!(keys.control_key(ControlAction::CoverUp), Some(KeyCode::ArrowRight));
    keys.restore(mode, control(ControlAction::LiftToggle));
    assert_eq!(keys.control_key(ControlAction::LiftToggle), None, "a control that ships with no key is restored to none");
    keys.restore(mode, Bindable::Lane(0));
    assert_eq!(keys.lane_keys(mode)[0], (KeyCode::KeyZ, 0));

    keys.bind(mode, control(ControlAction::CoverUp), Some(KeyCode::KeyP));
    keys.restore_everything();
    let shipped = KeyConfig::default();
    assert_eq!(keys.bound_keys(mode), shipped.bound_keys(mode));
    assert_eq!(keys.pad.lane_binding(mode, 0), None);
    assert_eq!(keys.pad.shortcut_binding(Shortcut::Search), None);
    assert_eq!(keys.pad.debounce_ms, 40, "the controller's tuning was thrown away with its bindings");
}

/// A key the file has no word for is not bound: the row keeps what it had.
#[test]
fn a_key_with_no_word_is_not_bound() {
    let mode = Mode::BEAT_7K;
    let mut keys = KeyConfig::default();
    for target in [control(ControlAction::CoverUp), Bindable::Lane(0), shortcut(Shortcut::Search)] {
        let before = keys.bound_keys(mode);
        keys.bind(mode, target, Some(KeyCode::F13));
        assert_eq!(keys.bound_keys(mode), before, "{target:?}");
    }
}

/// The keys that put a character on a line or edit it are the line's while one is typed; the keys
/// a shortcut can be read from under a search box are the rest.
#[test]
fn the_keys_of_a_typed_line_are_the_ones_that_type_and_edit() {
    for code in [KeyCode::KeyF, KeyCode::Digit0, KeyCode::Slash, KeyCode::Space, KeyCode::Numpad3, KeyCode::Backspace, KeyCode::Delete, KeyCode::ArrowLeft] {
        assert!(belongs_to_a_typed_line(code), "{code:?}");
    }
    for code in [KeyCode::F2, KeyCode::F12, KeyCode::Tab, KeyCode::Insert, KeyCode::ShiftLeft, KeyCode::ArrowUp, KeyCode::Enter, KeyCode::Escape] {
        assert!(!belongs_to_a_typed_line(code), "{code:?}");
    }
}

/// A key config as the application wrote it once the newer controls could be bound and before the
/// judge offset shipped with keys: every control it knew, the two offset keys among them with none.
const FILE_FROM_BEFORE_THE_OFFSET_KEYS: &str = r#"(
    lanes: {"7K": ["Z", "S", "X", "D", "C", "F", "V", "LSHIFT"]},
    scratch_reverse: {},
    controls: (
        hispeed_up: "UP", hispeed_down: "DOWN", cover_up: "RIGHT", cover_down: "LEFT",
        lift_up: "RBRACKET", lift_down: "LBRACKET", hidden_up: "", hidden_down: "",
        start: "A", select: "W", cover_toggle: "BACKSLASH", lift_toggle: "", hidden_toggle: "",
        green_up: "", green_down: "", keep_green_toggle: "", offset_up: "", offset_down: "",
        auto_adjust_toggle: "", fine: "",
    ),
    shortcuts: {},
    pad: (),
)"#;

/// The same file with one line of its controls or its lanes put in place of another.
fn offset_file_with(from: &str, to: &str) -> KeyConfig {
    assert!(FILE_FROM_BEFORE_THE_OFFSET_KEYS.contains(from), "the file has no {from}");
    ron::from_str(&FILE_FROM_BEFORE_THE_OFFSET_KEYS.replace(from, to)).expect("the edited file parses")
}

fn offset_keys(keys: &KeyConfig) -> (Option<KeyCode>, Option<KeyCode>) {
    (keys.control_key(ControlAction::OffsetDown), keys.control_key(ControlAction::OffsetUp))
}

/// The judge offset ships on the two digits at the end of the number row, a millisecond down and a
/// millisecond up, on keys nothing a run reads is shipped on in any mode. The switch that lets the
/// offset adjust itself ships with no key.
#[test]
fn the_judge_offset_ships_on_two_keys_and_its_switch_on_none() {
    let keys = KeyConfig::default();
    assert_eq!(offset_keys(&keys), (Some(KeyCode::Digit9), Some(KeyCode::Digit0)));
    assert_eq!((keys.control_token(ControlAction::OffsetDown), keys.control_token(ControlAction::OffsetUp)), ("9", "0"));
    assert_eq!(keys.control_key(ControlAction::AutoAdjustToggle), None);
    assert_eq!(keys.generation, crate::keyconfig::KEY_GENERATION);
    for action in ControlAction::ALL {
        if let Some((token, _)) = action.given_later() {
            assert_eq!(keys.control_token(action), token, "{action:?} ships on another key than a file from before it is given");
        }
    }
    for &mode in Mode::ALL {
        assert!(collisions(&keys, mode).is_empty(), "{}", mode.name);
        for code in [KeyCode::Digit9, KeyCode::Digit0] {
            assert!(default_keys_for_mode(mode).iter().all(|(lane_key, _)| *lane_key != code), "{code:?} plays a lane of {}", mode.name);
        }
    }
    for shortcut in Shortcut::ALL.into_iter().filter(|shortcut| shortcut.screen() == ShortcutScreen::Replay) {
        assert!(![KeyCode::Digit9, KeyCode::Digit0].contains(&shortcut.default_key()), "{shortcut:?} ships on a key of the offset");
    }
}

/// A key config written while the offset keys shipped with none holds them with none, and reads
/// them on the keys they ship on now: there was nothing else that file could hold. So does a file
/// from before the controls existed at all. Reading takes nothing from what the file does hold.
#[test]
fn a_key_config_from_before_the_offset_keys_reads_them_where_they_ship() {
    let older: KeyConfig = ron::from_str(FILE_FROM_BEFORE_THE_OFFSET_KEYS).expect("a config from before the offset keys parses");
    assert_eq!(older.generation, crate::keyconfig::FIRST_KEY_GENERATION, "a file that names no generation has taken none");
    assert_eq!((older.control_token(ControlAction::OffsetDown), older.control_token(ControlAction::OffsetUp)), ("", ""));
    assert_eq!(offset_keys(&older), (Some(KeyCode::Digit9), Some(KeyCode::Digit0)));
    assert_eq!(older.control_key(ControlAction::AutoAdjustToggle), None);
    assert_eq!(older.control_key(ControlAction::CoverToggle), Some(KeyCode::Backslash));
    for &mode in Mode::ALL {
        assert!(collisions(&older, mode).is_empty(), "{}", mode.name);
    }

    let oldest: KeyConfig = ron::from_str(FILE_FROM_BEFORE_SHORTCUTS).expect("a config from before the shortcuts parses");
    assert_eq!(offset_keys(&oldest), (Some(KeyCode::Digit9), Some(KeyCode::Digit0)));
    assert_eq!(oldest.control_key(ControlAction::HiSpeedUp), Some(KeyCode::Equal));
}

/// A key the file gives to something a run reads beside the offset keys stays with what the player
/// gave it to, and the offset key that ships there is left with none: a lane of any mode, a reverse
/// spin, another control, a key of a replay under analysis. The other offset key still ships.
#[test]
fn an_offset_key_does_not_take_a_key_the_player_gave_to_something_else() {
    let lane = offset_file_with(r#""V", "LSHIFT"]"#, r#""9", "LSHIFT"]"#);
    assert_eq!(offset_keys(&lane), (None, Some(KeyCode::Digit0)), "a lane on 9");
    assert_eq!(lane.lane_keys(Mode::BEAT_7K)[6], (KeyCode::Digit9, 6));
    assert!(collisions(&lane, Mode::BEAT_7K).is_empty());

    let other_mode = offset_file_with(r#"lanes: {"#, r#"lanes: {"9K": ["Z", "S", "X", "D", "C", "F", "V", "G", "0"], "#);
    assert_eq!(offset_keys(&other_mode), (Some(KeyCode::Digit9), None), "a lane of another mode on 0");

    let reverse = offset_file_with("scratch_reverse: {}", r#"scratch_reverse: {"7K": ["", "", "", "", "", "", "", "0"]}"#);
    assert_eq!(offset_keys(&reverse), (Some(KeyCode::Digit9), None), "a reverse spin on 0");

    let control = offset_file_with(r#"lift_toggle: """#, r#"lift_toggle: "9""#);
    assert_eq!(offset_keys(&control), (None, Some(KeyCode::Digit0)), "another control on 9");
    assert_eq!(control.control_key(ControlAction::LiftToggle), Some(KeyCode::Digit9));

    let replay = offset_file_with("shortcuts: {}", r#"shortcuts: {"replay_pause": "0", "search": "9"}"#);
    assert_eq!(offset_keys(&replay), (Some(KeyCode::Digit9), None), "a key of a replay under analysis on 0, and a shortcut of the browser on 9");
}

/// A key the player gave an offset key in an older file is the key it keeps, and one the player
/// swapped onto the other's shipped key leaves that other with none.
#[test]
fn an_offset_key_the_player_chose_in_an_older_file_is_kept() {
    let chosen = offset_file_with(r#"offset_up: """#, r#"offset_up: "P""#);
    assert_eq!(offset_keys(&chosen), (Some(KeyCode::Digit9), Some(KeyCode::KeyP)));
    let swapped = offset_file_with(r#"offset_up: """#, r#"offset_up: "9""#);
    assert_eq!(offset_keys(&swapped), (None, Some(KeyCode::Digit9)), "the key ships on 9, and the player put the other one there");
}

/// Once the configuration has taken the generation, a control left without a key stays without
/// one: taking a key away in the key editor writes down what every newly shipped control was read
/// from first, and what is written is what the file then says.
#[test]
fn an_offset_key_the_player_takes_away_stays_away() {
    let mode = Mode::BEAT_7K;
    let mut keys: KeyConfig = ron::from_str(FILE_FROM_BEFORE_THE_OFFSET_KEYS).expect("the file parses");
    keys.bind(mode, control(ControlAction::OffsetUp), None);
    assert_eq!(offset_keys(&keys), (Some(KeyCode::Digit9), None));
    assert_eq!(keys.generation, crate::keyconfig::KEY_GENERATION);
    assert_eq!(keys.control_token(ControlAction::OffsetDown), "9", "the key the other one was read from was not written down");

    let written = ron::ser::to_string(&keys).expect("the key config is written");
    let back: KeyConfig = ron::from_str(&written).expect("it reads back");
    assert_eq!(back.generation, crate::keyconfig::KEY_GENERATION);
    assert_eq!(offset_keys(&back), (Some(KeyCode::Digit9), None), "a key taken away came back with the file");

    let mut fresh = KeyConfig::default();
    fresh.bind(mode, control(ControlAction::OffsetDown), None);
    let back: KeyConfig = ron::from_str(&ron::ser::to_string(&fresh).expect("the key config is written")).expect("it reads back");
    assert_eq!(offset_keys(&back), (None, Some(KeyCode::Digit0)));
}

/// Giving something else a key in an older configuration settles the offset keys first, so the
/// key editor refuses a key an offset key is on as it refuses any other, and a lane given a key
/// afterwards takes it from nothing.
#[test]
fn changing_any_key_of_an_older_configuration_settles_the_offset_keys() {
    let mode = Mode::BEAT_7K;
    let mut keys: KeyConfig = ron::from_str(FILE_FROM_BEFORE_THE_OFFSET_KEYS).expect("the file parses");
    assert_eq!(keys.clash(mode, Bindable::Lane(0), KeyCode::Digit9), Some(Clash::With(control(ControlAction::OffsetDown))));
    keys.set_lane(mode, 0, KeyCode::KeyQ);
    assert_eq!(keys.generation, crate::keyconfig::KEY_GENERATION);
    assert_eq!((keys.control_token(ControlAction::OffsetDown), keys.control_token(ControlAction::OffsetUp)), ("9", "0"));

    let mut given_away = offset_file_with(r#"lift_toggle: """#, r#"lift_toggle: "9""#);
    given_away.set_control(ControlAction::LiftToggle, KeyCode::KeyU);
    assert_eq!(offset_keys(&given_away), (None, Some(KeyCode::Digit0)), "an offset key that was left with none took the key the moment it was free");

    let mut restored = offset_file_with(r#"lift_toggle: """#, r#"lift_toggle: "9""#);
    restored.restore_everything();
    assert_eq!(restored.bound_keys(mode), KeyConfig::default().bound_keys(mode));
    assert_eq!(restored.generation, crate::keyconfig::KEY_GENERATION);
}

/// A key config from before the turntable's other direction shipped with a key holds none for it,
/// as there was nothing else for it to hold, and reads it on the key it ships on now -- in every
/// mode, and wherever the player put the turntable itself: a file that plays the scratch lane from
/// a digit reads its other direction from the same Control key.
#[test]
fn a_key_config_from_before_the_reverse_keys_reads_them_where_they_ship() {
    let older: KeyConfig = ron::from_str(FILE_FROM_BEFORE_THE_OFFSET_KEYS).expect("a config from before the reverse keys parses");
    assert!(older.scratch_reverse.is_empty());
    for &mode in Mode::ALL {
        assert_eq!(older.scratch_reverse_keys(mode), default_reverse_keys_for_mode(mode), "{}", mode.name);
        assert!(collisions(&older, mode).is_empty(), "{}", mode.name);
    }

    let moved = offset_file_with(r#""V", "LSHIFT"]"#, r#""V", "1"]"#);
    assert_eq!(moved.lane_keys(Mode::BEAT_7K)[7], (KeyCode::Digit1, 7));
    assert_eq!(moved.scratch_reverse_keys(Mode::BEAT_7K), vec![(KeyCode::ControlLeft, 7)], "a turntable moved to another key has no other direction");

    let row_of_nothing = offset_file_with("scratch_reverse: {}", r#"scratch_reverse: {"7K": ["", "", "", "", "", "", "", ""]}"#);
    assert_eq!(row_of_nothing.scratch_reverse_keys(Mode::BEAT_7K), vec![(KeyCode::ControlLeft, 7)]);

    let second_generation = offset_file_with("shortcuts: {},", "shortcuts: {}, generation: 1,");
    assert_eq!(second_generation.generation, crate::keyconfig::OFFSET_KEYS_GENERATION);
    assert_eq!(second_generation.scratch_reverse_keys(Mode::BEAT_14K), default_reverse_keys_for_mode(Mode::BEAT_14K));
}

/// A reverse key the file holds is the one that is read, and a shipped one the file gives to
/// something else a run reads stays with what the player gave it to: a lane, another control, a
/// key of a replay under analysis. The reverse key of the other side still ships.
#[test]
fn a_reverse_key_does_not_take_a_key_the_player_gave_to_something_else() {
    let chosen = offset_file_with("scratch_reverse: {}", r#"scratch_reverse: {"7K": ["", "", "", "", "", "", "", "Q"], "10K": ["", "", "", "", "", "RCTRL"]}"#);
    assert_eq!(chosen.scratch_reverse_keys(Mode::BEAT_7K), vec![(KeyCode::KeyQ, 7)]);
    assert_eq!(
        chosen.scratch_reverse_keys(Mode::BEAT_10K),
        vec![(KeyCode::ControlRight, 5)],
        "a key the other side ships on, given to this side, was given twice"
    );
    assert_eq!(chosen.scratch_reverse_keys(Mode::BEAT_14K), default_reverse_keys_for_mode(Mode::BEAT_14K));

    let lane = offset_file_with(r#""V", "LSHIFT"]"#, r#""LCTRL", "LSHIFT"]"#);
    assert!(lane.scratch_reverse_keys(Mode::BEAT_7K).is_empty(), "a lane on the left Control key");
    assert_eq!(lane.lane_keys(Mode::BEAT_7K)[6], (KeyCode::ControlLeft, 6));
    assert_eq!(lane.scratch_reverse_keys(Mode::BEAT_14K), default_reverse_keys_for_mode(Mode::BEAT_14K), "a lane of one mode took the key from another");

    let control = offset_file_with(r#"lift_toggle: """#, r#"lift_toggle: "RCTRL""#);
    assert_eq!(control.scratch_reverse_keys(Mode::BEAT_14K), vec![(KeyCode::ControlLeft, 7)], "another control on the right Control key");
    assert_eq!(control.control_key(ControlAction::LiftToggle), Some(KeyCode::ControlRight));

    let fine = offset_file_with(r#"fine: """#, r#"fine: "LCTRL""#);
    assert!(fine.scratch_reverse_keys(Mode::BEAT_7K).is_empty(), "the key held for the finer step on the left Control key");

    let replay = offset_file_with("shortcuts: {}", r#"shortcuts: {"replay_pause": "LCTRL"}"#);
    assert!(replay.scratch_reverse_keys(Mode::BEAT_5K).is_empty(), "a key of a replay under analysis on the left Control key");
    for keys in [&lane, &control, &fine, &replay] {
        for &mode in Mode::ALL {
            assert!(collisions(keys, mode).is_empty(), "{}", mode.name);
        }
    }
}

/// Once the configuration has taken the generation a reverse key left with none stays with none:
/// changing any key writes down what every reverse key was read from first, taking one away leaves
/// it away through the file, and a key written down is refused to anything else as any key is.
#[test]
fn a_reverse_key_the_player_takes_away_stays_away() {
    let mode = Mode::BEAT_14K;
    let mut keys: KeyConfig = ron::from_str(FILE_FROM_BEFORE_THE_OFFSET_KEYS).expect("the file parses");
    assert_eq!(keys.clash(mode, Bindable::Lane(0), KeyCode::ControlRight), Some(Clash::With(Bindable::ScratchReverse(15))));
    keys.bind(mode, Bindable::ScratchReverse(7), None);
    assert_eq!(keys.generation, crate::keyconfig::KEY_GENERATION);
    assert_eq!(keys.scratch_reverse_keys(mode), vec![(KeyCode::ControlRight, 15)]);
    assert_eq!(keys.scratch_reverse_token(mode, 15), "RCTRL", "the key the other side was read from was not written down");
    assert_eq!(keys.scratch_reverse_token(Mode::BEAT_7K, 7), "LCTRL", "the key of another mode was not written down");

    let back: KeyConfig = ron::from_str(&ron::ser::to_string(&keys).expect("the key config is written")).expect("it reads back");
    assert_eq!(back.scratch_reverse_keys(mode), vec![(KeyCode::ControlRight, 15)], "a key taken away came back with the file");
    assert_eq!(back.scratch_reverse_keys(Mode::BEAT_7K), vec![(KeyCode::ControlLeft, 7)]);

    let mut given_away = offset_file_with(r#"lift_toggle: """#, r#"lift_toggle: "LCTRL""#);
    given_away.set_control(ControlAction::LiftToggle, KeyCode::KeyU);
    assert!(given_away.scratch_reverse_keys(Mode::BEAT_7K).is_empty(), "a reverse key that was left with none took the key the moment it was free");
}

/// The older file read from disk is not mistaken for a broken one and is not rewritten, and what
/// the new version writes reads back the same.
#[test]
fn a_file_from_before_the_offset_keys_loads_from_disk_and_survives_being_written_again() {
    let dir = std::env::temp_dir().join(format!("rbms_kc_before_offset_keys_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("keyconfig.ron");
    std::fs::write(&path, FILE_FROM_BEFORE_THE_OFFSET_KEYS).unwrap();
    let loaded = KeyConfig::load(&path);
    assert!(!path.with_extension("ron.bak").exists(), "the older file was backed up as if it were broken");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), FILE_FROM_BEFORE_THE_OFFSET_KEYS, "the older file was rewritten on load");
    assert_eq!(offset_keys(&loaded), (Some(KeyCode::Digit9), Some(KeyCode::Digit0)));

    loaded.save(&path);
    let again = KeyConfig::load(&path);
    assert_eq!(again.bound_keys(Mode::BEAT_7K), loaded.bound_keys(Mode::BEAT_7K));
    let _ = std::fs::remove_dir_all(&dir);
}
