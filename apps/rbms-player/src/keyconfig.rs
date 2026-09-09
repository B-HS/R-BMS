use std::collections::BTreeMap;
use std::path::Path;

use rbms_model::Mode;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

/// Parse a key token (the vocabulary used in the keyconfig file and `--keys`) into a `KeyCode`.
pub fn key_from_name(s: &str) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match s.to_ascii_uppercase().as_str() {
        "A" => KeyA, "B" => KeyB, "C" => KeyC, "D" => KeyD, "E" => KeyE, "F" => KeyF, "G" => KeyG,
        "H" => KeyH, "I" => KeyI, "J" => KeyJ, "K" => KeyK, "L" => KeyL, "M" => KeyM, "N" => KeyN,
        "O" => KeyO, "P" => KeyP, "Q" => KeyQ, "R" => KeyR, "S" => KeyS, "T" => KeyT, "U" => KeyU,
        "V" => KeyV, "W" => KeyW, "X" => KeyX, "Y" => KeyY, "Z" => KeyZ,
        "SPACE" => Space, "LSHIFT" | "SHIFT" => ShiftLeft, "RSHIFT" => ShiftRight,
        "LCTRL" | "CTRL" | "CONTROL" => ControlLeft, "RCTRL" => ControlRight,
        "1" => Digit1, "2" => Digit2, "3" => Digit3, "4" => Digit4, "5" => Digit5,
        "6" => Digit6, "7" => Digit7, "8" => Digit8, "9" => Digit9, "0" => Digit0,
        "COMMA" => Comma, "PERIOD" => Period, "SLASH" => Slash, "SEMICOLON" => Semicolon,
        "QUOTE" | "APOSTROPHE" => Quote,
        "UP" => ArrowUp, "DOWN" => ArrowDown, "LEFT" => ArrowLeft, "RIGHT" => ArrowRight,
        "LBRACKET" => BracketLeft, "RBRACKET" => BracketRight, "MINUS" => Minus, "EQUAL" => Equal,
        "PAGEUP" => PageUp, "PAGEDOWN" => PageDown, "BACKSLASH" => Backslash,
        _ => return None,
    })
}

/// Short display token for a key — the inverse of `key_from_name`, so configs round-trip.
pub fn key_name(code: KeyCode) -> &'static str {
    use KeyCode::*;
    match code {
        KeyA => "A", KeyB => "B", KeyC => "C", KeyD => "D", KeyE => "E", KeyF => "F", KeyG => "G",
        KeyH => "H", KeyI => "I", KeyJ => "J", KeyK => "K", KeyL => "L", KeyM => "M", KeyN => "N",
        KeyO => "O", KeyP => "P", KeyQ => "Q", KeyR => "R", KeyS => "S", KeyT => "T", KeyU => "U",
        KeyV => "V", KeyW => "W", KeyX => "X", KeyY => "Y", KeyZ => "Z",
        Space => "SPACE", ShiftLeft => "LSHIFT", ShiftRight => "RSHIFT", ControlLeft => "LCTRL", ControlRight => "RCTRL",
        Digit1 => "1", Digit2 => "2", Digit3 => "3", Digit4 => "4", Digit5 => "5",
        Digit6 => "6", Digit7 => "7", Digit8 => "8", Digit9 => "9", Digit0 => "0",
        Comma => "COMMA", Period => "PERIOD", Slash => "SLASH", Semicolon => "SEMICOLON", Quote => "QUOTE",
        ArrowUp => "UP", ArrowDown => "DOWN", ArrowLeft => "LEFT", ArrowRight => "RIGHT",
        BracketLeft => "LBRACKET", BracketRight => "RBRACKET", Minus => "MINUS", Equal => "EQUAL",
        PageUp => "PAGEUP", PageDown => "PAGEDOWN", Backslash => "BACKSLASH",
        _ => "?",
    }
}

/// Reference-style default lane bindings (Z-row) for a mode, as `(key, lane)`. Lanes are rbms
/// `Mode` lane indices: 7K keys 0..6 + scratch 7; 14K P1 keys 0..6 scratch 7, P2 keys 8..14
/// scratch 15; PMS/9K all 9 lanes, no scratch.
pub fn default_keys_for_mode(mode: Mode) -> Vec<(KeyCode, usize)> {
    use KeyCode::*;
    match mode.name {
        "BEAT_5K" => vec![(KeyZ, 0), (KeyS, 1), (KeyX, 2), (KeyD, 3), (KeyC, 4), (ShiftLeft, 5)],
        "POPN_9K" => vec![(KeyZ, 0), (KeyS, 1), (KeyX, 2), (KeyD, 3), (KeyC, 4), (KeyF, 5), (KeyV, 6), (KeyG, 7), (KeyB, 8)],
        "BEAT_10K" => vec![
            (KeyZ, 0), (KeyS, 1), (KeyX, 2), (KeyD, 3), (KeyC, 4), (ShiftLeft, 5),
            (KeyM, 6), (KeyK, 7), (Comma, 8), (KeyL, 9), (Period, 10), (ShiftRight, 11),
        ],
        "BEAT_14K" => vec![
            (KeyZ, 0), (KeyS, 1), (KeyX, 2), (KeyD, 3), (KeyC, 4), (KeyF, 5), (KeyV, 6), (ShiftLeft, 7),
            (KeyM, 8), (KeyK, 9), (Comma, 10), (KeyL, 11), (Period, 12), (Semicolon, 13), (Slash, 14), (ShiftRight, 15),
        ],
        _ => vec![(KeyZ, 0), (KeyS, 1), (KeyX, 2), (KeyD, 3), (KeyC, 4), (KeyF, 5), (KeyV, 6), (ShiftLeft, 7)],
    }
}

/// Stable, human-readable config-file key for a mode's lane bindings. Dispatches on the mode
/// identity (name), not its lane count, so two modes can never collide on the same file key.
pub fn mode_config_key(mode: Mode) -> &'static str {
    match mode.name {
        "BEAT_5K" => "5K",
        "BEAT_7K" => "7K",
        "POPN_9K" => "9K",
        "BEAT_10K" => "10K",
        "BEAT_14K" => "14K",
        _ => mode.name,
    }
}

/// A non-lane, in-play control whose key is user-configurable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlAction {
    HiSpeedUp,
    HiSpeedDown,
    CoverUp,
    CoverDown,
    LiftUp,
    LiftDown,
}

impl ControlAction {
    pub const ALL: [ControlAction; 6] = [
        ControlAction::HiSpeedUp,
        ControlAction::HiSpeedDown,
        ControlAction::CoverUp,
        ControlAction::CoverDown,
        ControlAction::LiftUp,
        ControlAction::LiftDown,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ControlAction::HiSpeedUp => "HI-SPEED UP",
            ControlAction::HiSpeedDown => "HI-SPEED DOWN",
            ControlAction::CoverUp => "COVER UP",
            ControlAction::CoverDown => "COVER DOWN",
            ControlAction::LiftUp => "LIFT UP",
            ControlAction::LiftDown => "LIFT DOWN",
        }
    }
}

/// User-configurable in-play control key tokens.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlBinds {
    pub hispeed_up: String,
    pub hispeed_down: String,
    pub cover_up: String,
    pub cover_down: String,
    pub lift_up: String,
    pub lift_down: String,
}

impl Default for ControlBinds {
    fn default() -> Self {
        ControlBinds {
            hispeed_up: "UP".into(),
            hispeed_down: "DOWN".into(),
            cover_up: "RIGHT".into(),
            cover_down: "LEFT".into(),
            lift_up: "RBRACKET".into(),
            lift_down: "LBRACKET".into(),
        }
    }
}

impl ControlBinds {
    fn slot(&mut self, action: ControlAction) -> &mut String {
        match action {
            ControlAction::HiSpeedUp => &mut self.hispeed_up,
            ControlAction::HiSpeedDown => &mut self.hispeed_down,
            ControlAction::CoverUp => &mut self.cover_up,
            ControlAction::CoverDown => &mut self.cover_down,
            ControlAction::LiftUp => &mut self.lift_up,
            ControlAction::LiftDown => &mut self.lift_down,
        }
    }

    fn token(&self, action: ControlAction) -> &str {
        match action {
            ControlAction::HiSpeedUp => &self.hispeed_up,
            ControlAction::HiSpeedDown => &self.hispeed_down,
            ControlAction::CoverUp => &self.cover_up,
            ControlAction::CoverDown => &self.cover_down,
            ControlAction::LiftUp => &self.lift_up,
            ControlAction::LiftDown => &self.lift_down,
        }
    }
}

/// The full user key configuration: per-mode lane bindings plus the in-play control keys.
/// Persisted as RON; absent fields fall back to the built-in defaults.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyConfig {
    pub lanes: BTreeMap<String, Vec<String>>,
    pub controls: ControlBinds,
}

impl Default for KeyConfig {
    fn default() -> Self {
        let mut lanes = BTreeMap::new();
        for &mode in Mode::ALL {
            let mut row = vec![String::new(); mode.key];
            for (code, lane) in default_keys_for_mode(mode) {
                if lane < row.len() {
                    row[lane] = key_name(code).to_string();
                }
            }
            lanes.insert(mode_config_key(mode).to_string(), row);
        }
        KeyConfig { lanes, controls: ControlBinds::default() }
    }
}

impl KeyConfig {
    /// Lane bindings for a mode as `(key, lane)`. Every lane `0..mode.key` is guaranteed to be
    /// bound: a lane whose config token is missing, empty, or unrecognized is filled from the
    /// built-in default (a bad token is warned about), so a short or partly-invalid row can
    /// never leave a column unhittable.
    pub fn lane_keys(&self, mode: Mode) -> Vec<(KeyCode, usize)> {
        let mut by_lane: Vec<Option<KeyCode>> = vec![None; mode.key];
        for (code, lane) in default_keys_for_mode(mode) {
            if lane < by_lane.len() {
                by_lane[lane] = Some(code);
            }
        }
        if let Some(row) = self.lanes.get(mode_config_key(mode)) {
            for (lane, tok) in row.iter().enumerate() {
                if lane >= mode.key || tok.is_empty() {
                    continue;
                }
                match key_from_name(tok) {
                    Some(code) => by_lane[lane] = Some(code),
                    None => eprintln!("keyconfig: mode {} lane {} has unknown key {tok:?}; using default", mode.name, lane),
                }
            }
        }
        by_lane.into_iter().enumerate().filter_map(|(lane, code)| code.map(|c| (c, lane))).collect()
    }

    pub fn control_key(&self, action: ControlAction) -> Option<KeyCode> {
        key_from_name(self.controls.token(action))
    }

    /// Keys bound to more than one action for `mode` (across all controls + that mode's lanes),
    /// so the editor can flag conflicts. Control-first input resolution means a collided lane
    /// would otherwise be silently unplayable.
    pub fn collisions(&self, mode: Mode) -> std::collections::HashSet<KeyCode> {
        let mut seen = std::collections::HashSet::new();
        let mut dup = std::collections::HashSet::new();
        let all = ControlAction::ALL.into_iter().filter_map(|a| self.control_key(a)).chain(self.lane_keys(mode).into_iter().map(|(k, _)| k));
        for code in all {
            if !seen.insert(code) {
                dup.insert(code);
            }
        }
        dup
    }

    /// Rebind a control action; the change is in memory until `save`.
    pub fn set_control(&mut self, action: ControlAction, code: KeyCode) {
        *self.controls.slot(action) = key_name(code).to_string();
    }

    /// Rebind one lane of a mode; grows the mode's row to fit if needed.
    pub fn set_lane(&mut self, mode: Mode, lane: usize, code: KeyCode) {
        let row = self.lanes.entry(mode_config_key(mode).to_string()).or_insert_with(|| vec![String::new(); mode.key]);
        if row.len() <= lane {
            row.resize(lane + 1, String::new());
        }
        row[lane] = key_name(code).to_string();
    }

    /// Current key token shown for a mode's lane (empty string if unbound).
    pub fn lane_token(&self, mode: Mode, lane: usize) -> String {
        self.lanes.get(mode_config_key(mode)).and_then(|row| row.get(lane)).cloned().unwrap_or_default()
    }

    pub fn control_token(&self, action: ControlAction) -> &str {
        self.controls.token(action)
    }

    pub fn load(path: &Path) -> KeyConfig {
        match std::fs::read_to_string(path) {
            Ok(s) => match ron::from_str::<KeyConfig>(&s) {
                Ok(kc) => kc,
                Err(e) => {
                    // Preserve the user's (malformed) edits before falling back: a later save()
                    // must not silently overwrite a recoverable file.
                    let backup = path.with_extension("ron.bak");
                    match std::fs::rename(path, &backup) {
                        Ok(_) => eprintln!("keyconfig parse failed ({e}); backed up to {} and using defaults", backup.display()),
                        Err(err) => eprintln!("keyconfig parse failed ({e}); backup failed ({err}); using defaults"),
                    }
                    KeyConfig::default()
                }
            },
            Err(_) => {
                let kc = KeyConfig::default();
                kc.save(path);
                kc
            }
        }
    }

    pub fn save(&self, path: &Path) {
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => match crate::write_atomic(path, &s) {
                Ok(()) => println!("keyconfig saved: {}", path.display()),
                Err(e) => eprintln!("keyconfig write failed ({}): {e}", path.display()),
            },
            Err(e) => eprintln!("keyconfig save failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let kc = KeyConfig { lanes, controls: ControlBinds::default() };
        let keys = kc.lane_keys(Mode::BEAT_7K);
        assert_eq!(keys.len(), Mode::BEAT_7K.key, "every lane bound despite a short row with a bad token");
        let mut got: Vec<usize> = keys.iter().map(|(_, l)| *l).collect();
        got.sort_unstable();
        assert_eq!(got, (0..Mode::BEAT_7K.key).collect::<Vec<_>>(), "no lane left unbound");
    }

    #[test]
    fn collisions_flags_key_shared_by_control_and_lane() {
        let mut kc = KeyConfig::default();
        kc.set_control(ControlAction::CoverUp, KeyCode::KeyZ); // KeyZ is also 7K lane 0
        let dup = kc.collisions(Mode::BEAT_7K);
        assert!(dup.contains(&KeyCode::KeyZ), "shared key reported as a collision");
        assert!(!dup.contains(&KeyCode::ArrowDown), "an unshared key is not a collision");
    }

    // --- key_from_name / key_name round-trip and aliases ---

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
        // Several human-friendly spellings collapse to one canonical KeyCode.
        assert_eq!(key_from_name("SHIFT"), key_from_name("LSHIFT"));
        assert_eq!(key_from_name("CTRL"), key_from_name("LCTRL"));
        assert_eq!(key_from_name("CONTROL"), key_from_name("LCTRL"));
        assert_eq!(key_from_name("QUOTE"), key_from_name("APOSTROPHE"));
    }

    #[test]
    fn key_name_round_trips_through_key_from_name_for_every_token() {
        // Every token key_name can emit must parse back to the same code (configs round-trip).
        let codes = [
            KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD, KeyCode::KeyE,
            KeyCode::KeyF, KeyCode::KeyG, KeyCode::KeyH, KeyCode::KeyI, KeyCode::KeyJ,
            KeyCode::KeyK, KeyCode::KeyL, KeyCode::KeyM, KeyCode::KeyN, KeyCode::KeyO,
            KeyCode::KeyP, KeyCode::KeyQ, KeyCode::KeyR, KeyCode::KeyS, KeyCode::KeyT,
            KeyCode::KeyU, KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX, KeyCode::KeyY,
            KeyCode::KeyZ, KeyCode::Space, KeyCode::ShiftLeft, KeyCode::ShiftRight,
            KeyCode::ControlLeft, KeyCode::ControlRight,
            KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5,
            KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9, KeyCode::Digit0,
            KeyCode::Comma, KeyCode::Period, KeyCode::Slash, KeyCode::Semicolon, KeyCode::Quote,
            KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::ArrowLeft, KeyCode::ArrowRight,
            KeyCode::BracketLeft, KeyCode::BracketRight, KeyCode::Minus, KeyCode::Equal,
            KeyCode::PageUp, KeyCode::PageDown, KeyCode::Backslash,
        ];
        for code in codes {
            let tok = key_name(code);
            assert_ne!(tok, "?", "{code:?} should have a real token, not the sentinel");
            assert_eq!(key_from_name(tok), Some(code), "{code:?} round-trips via {tok:?}");
        }
    }

    #[test]
    fn key_name_unmapped_code_is_question_mark() {
        // A KeyCode with no token maps to the "?" sentinel (and would not round-trip).
        assert_eq!(key_name(KeyCode::Enter), "?");
        assert_eq!(key_from_name(key_name(KeyCode::Enter)), None);
    }

    // --- default_keys_for_mode covers every lane, per mode ---

    #[test]
    fn default_keys_for_mode_binds_every_lane_once() {
        for &mode in Mode::ALL {
            let binds = default_keys_for_mode(mode);
            assert_eq!(binds.len(), mode.key, "{} default has one bind per lane", mode.name);
            let mut lanes: Vec<usize> = binds.iter().map(|(_, l)| *l).collect();
            lanes.sort_unstable();
            assert_eq!(lanes, (0..mode.key).collect::<Vec<_>>(), "{} covers lanes 0..{}", mode.name, mode.key);
            // every default key must be a token that key_name/key_from_name understand
            for (code, _) in &binds {
                assert_ne!(key_name(*code), "?", "{} default key {code:?} has a token", mode.name);
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

    #[test]
    fn unknown_mode_falls_back_to_seven_lane_default() {
        // mode_config_key / default_keys_for_mode dispatch on mode.name; a synthetic 8-lane mode
        // not in the match takes the `_` arm (BEAT_7K-shaped 8 lanes).
        let custom = Mode { name: "CUSTOM_8K", key: 8, ..Mode::BEAT_7K };
        assert_eq!(mode_config_key(custom), "CUSTOM_8K", "unknown mode keyed by its own name");
        let binds = default_keys_for_mode(custom);
        assert_eq!(binds.len(), 8, "fallback default is 8 lanes");
    }

    // --- mode_config_key uniqueness ---

    #[test]
    fn mode_config_keys_are_unique_across_all_modes() {
        let mut keys: Vec<&str> = Mode::ALL.iter().map(|&m| mode_config_key(m)).collect();
        let n = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), n, "no two modes share a config key");
    }

    // --- lane_keys backfill / invariants ---

    #[test]
    fn lane_keys_empty_row_falls_fully_back_to_default() {
        let mut lanes = BTreeMap::new();
        lanes.insert("7K".to_string(), Vec::<String>::new());
        let kc = KeyConfig { lanes, controls: ControlBinds::default() };
        let keys = kc.lane_keys(Mode::BEAT_7K);
        assert_eq!(keys, default_keys_for_mode(Mode::BEAT_7K), "empty row == pure default");
    }

    #[test]
    fn lane_keys_ignores_extra_trailing_entries_beyond_mode_key() {
        // A row longer than mode.key: lanes >= mode.key are skipped, never added.
        let mut row: Vec<String> = vec![String::new(); Mode::BEAT_5K.key];
        row.push("A".into()); // lane 6, beyond 5K's 6 lanes
        row.push("B".into()); // lane 7
        let mut lanes = BTreeMap::new();
        lanes.insert("5K".to_string(), row);
        let kc = KeyConfig { lanes, controls: ControlBinds::default() };
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
        // The fold over `by_lane` enumerates 0..mode.key, so result lanes are sorted ascending.
        for &mode in Mode::ALL {
            let keys = KeyConfig::default().lane_keys(mode);
            let lanes: Vec<usize> = keys.iter().map(|(_, l)| *l).collect();
            let mut sorted = lanes.clone();
            sorted.sort_unstable();
            assert_eq!(lanes, sorted, "{} lanes returned in ascending order", mode.name);
        }
    }

    // --- collisions ---

    #[test]
    fn collisions_empty_when_no_key_shared() {
        // Defaults: arrow/bracket controls never collide with letter/shift lane keys.
        let kc = KeyConfig::default();
        for &mode in Mode::ALL {
            assert!(kc.collisions(mode).is_empty(), "{} has no default collisions", mode.name);
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

    // --- set_lane growth ---

    #[test]
    fn set_lane_grows_a_short_or_missing_row() {
        let mut kc = KeyConfig::default();
        // remove the row entirely so the entry must be created from scratch
        kc.lanes.remove(mode_config_key(Mode::BEAT_7K));
        kc.set_lane(Mode::BEAT_7K, 5, KeyCode::KeyP);
        assert_eq!(kc.lane_token(Mode::BEAT_7K, 5), "P", "lane 5 bound after row created");
        // setting a lane beyond the current row length resizes with empty strings between
        kc.lanes.insert("EXTRA".into(), vec!["A".into()]);
        let custom = Mode { name: "EXTRA", key: 1, ..Mode::BEAT_7K };
        kc.set_lane(custom, 4, KeyCode::KeyB);
        assert_eq!(kc.lane_token(custom, 4), "B");
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

    // --- ControlAction metadata ---

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

    // --- full RON round-trip incl. per-mode lanes + controls ---

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
        // serde(default) on KeyConfig: an empty struct literal yields all defaults.
        let kc: KeyConfig = ron::from_str("()").expect("empty unit parses via serde(default)");
        let def = KeyConfig::default();
        for &mode in Mode::ALL {
            assert_eq!(kc.lane_keys(mode), def.lane_keys(mode), "{} defaulted", mode.name);
        }
        for a in ControlAction::ALL {
            assert_eq!(kc.control_key(a), def.control_key(a));
        }
    }

    // --- load() default-on-missing-file + persists ---

    #[test]
    fn load_missing_file_writes_defaults() {
        let dir = std::env::temp_dir().join(format!("rbms_kc_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("keyconfig.ron");
        assert!(!path.exists());
        let kc = KeyConfig::load(&path);
        assert!(path.exists(), "load() of a missing file writes the defaults out");
        assert_eq!(kc.control_key(ControlAction::HiSpeedUp), Some(KeyCode::ArrowUp));
        // and the written file re-loads to an equivalent config
        let kc2 = KeyConfig::load(&path);
        assert_eq!(kc2.lane_keys(Mode::BEAT_7K), kc.lane_keys(Mode::BEAT_7K));
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
        // returns defaults
        assert_eq!(kc.control_key(ControlAction::HiSpeedUp), Some(KeyCode::ArrowUp));
        // original malformed file is renamed to a .bak so it is not lost
        assert!(path.with_extension("ron.bak").exists(), "malformed file backed up to .bak");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
