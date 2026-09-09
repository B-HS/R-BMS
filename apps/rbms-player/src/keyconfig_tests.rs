use crate::keyconfig::{ControlAction, KeyConfig, default_keys_for_mode, key_from_name, key_name, mode_config_key};
use rbms_model::Mode;
use rbms_play::ScratchDir;
use std::collections::BTreeMap;
use winit::keyboard::KeyCode;

/// An app whose key config is the one this test wrote, so the reverse binding it sets really is the
/// one the input path resolves against.
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

/// Without a reverse binding nothing changes: every bound key is a forward spin, which is what a
/// key config written before the row existed asks for.
#[test]
fn every_key_is_a_forward_spin_until_a_reverse_key_is_bound() {
    let shared = shared_with(&KeyConfig::default());
    for (code, lane) in KeyConfig::default().lane_keys(Mode::BEAT_7K) {
        assert_eq!(shared.lane_input_for(code), Some((lane, ScratchDir::Forward)), "{code:?}");
    }
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
        assert!(kc.control_key(action).is_some(), "{:?} has a default key", action);
    }
}

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
    let dup = kc.collisions(Mode::BEAT_7K);
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
    for bad in ["", "BOGUS", "F13", "  ", "AA", "KEYA", "ENTER", "ESCAPE"] {
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
    assert_eq!(key_name(KeyCode::Enter), "?", "a code with no token gets the sentinel");
    assert_eq!(key_from_name(key_name(KeyCode::Enter)), None, "and the sentinel does not round-trip back to a code");
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
        assert!(kc.collisions(mode).is_empty(), "{} arrow/bracket controls never share a key with a letter/shift lane", mode.name);
    }
}

#[test]
fn collisions_detects_two_controls_sharing_a_key() {
    let mut kc = KeyConfig::default();
    kc.set_control(ControlAction::HiSpeedUp, KeyCode::KeyP);
    kc.set_control(ControlAction::HiSpeedDown, KeyCode::KeyP);
    let dup = kc.collisions(Mode::BEAT_7K);
    assert!(dup.contains(&KeyCode::KeyP), "two controls on the same key collide");
}

#[test]
fn collisions_detects_two_lanes_sharing_a_key() {
    let mut kc = KeyConfig::default();
    kc.set_lane(Mode::BEAT_7K, 0, KeyCode::KeyP);
    kc.set_lane(Mode::BEAT_7K, 1, KeyCode::KeyP);
    let dup = kc.collisions(Mode::BEAT_7K);
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
}

#[test]
fn control_action_all_has_distinct_nonempty_labels() {
    assert_eq!(ControlAction::ALL.len(), 6);
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

/// A key config ships with no reverse-spin bindings, so a scratch lane is spun one way only —
/// exactly what every config written before the row existed asks for.
#[test]
fn no_scratch_lane_is_bound_to_a_reverse_key_until_one_is_set() {
    let kc = KeyConfig::default();
    for &mode in Mode::ALL {
        assert!(kc.scratch_reverse_keys(mode).is_empty(), "{} ships with no reverse spin", mode.name);
        for lane in 0..mode.key {
            assert!(kc.scratch_reverse_token(mode, lane).is_empty(), "{} lane {lane}", mode.name);
        }
    }
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
    assert!(kc.scratch_reverse_keys(Mode::BEAT_5K).is_empty(), "a binding on one mode does not leak into another");
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
    assert!(kc.collisions(mode).contains(&lane_key), "a reverse key that shadows a lane is flagged");

    let mut kc = KeyConfig::default();
    kc.set_scratch_reverse(mode, scratch, KeyCode::KeyQ);
    assert!(!kc.collisions(mode).contains(&KeyCode::KeyQ), "a reverse key of its own is not a collision");
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
    assert!(older.scratch_reverse_keys(mode).is_empty());
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
