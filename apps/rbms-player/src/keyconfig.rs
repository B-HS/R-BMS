use std::collections::BTreeMap;
use std::path::Path;

use rbms_model::Mode;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

/// Parse a key token (the vocabulary used in the keyconfig file and `--keys`) into a `KeyCode`.
pub fn key_from_name(s: &str) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match s.to_ascii_uppercase().as_str() {
        "A" => KeyA,
        "B" => KeyB,
        "C" => KeyC,
        "D" => KeyD,
        "E" => KeyE,
        "F" => KeyF,
        "G" => KeyG,
        "H" => KeyH,
        "I" => KeyI,
        "J" => KeyJ,
        "K" => KeyK,
        "L" => KeyL,
        "M" => KeyM,
        "N" => KeyN,
        "O" => KeyO,
        "P" => KeyP,
        "Q" => KeyQ,
        "R" => KeyR,
        "S" => KeyS,
        "T" => KeyT,
        "U" => KeyU,
        "V" => KeyV,
        "W" => KeyW,
        "X" => KeyX,
        "Y" => KeyY,
        "Z" => KeyZ,
        "SPACE" => Space,
        "LSHIFT" | "SHIFT" => ShiftLeft,
        "RSHIFT" => ShiftRight,
        "LCTRL" | "CTRL" | "CONTROL" => ControlLeft,
        "RCTRL" => ControlRight,
        "1" => Digit1,
        "2" => Digit2,
        "3" => Digit3,
        "4" => Digit4,
        "5" => Digit5,
        "6" => Digit6,
        "7" => Digit7,
        "8" => Digit8,
        "9" => Digit9,
        "0" => Digit0,
        "COMMA" => Comma,
        "PERIOD" => Period,
        "SLASH" => Slash,
        "SEMICOLON" => Semicolon,
        "QUOTE" | "APOSTROPHE" => Quote,
        "UP" => ArrowUp,
        "DOWN" => ArrowDown,
        "LEFT" => ArrowLeft,
        "RIGHT" => ArrowRight,
        "LBRACKET" => BracketLeft,
        "RBRACKET" => BracketRight,
        "MINUS" => Minus,
        "EQUAL" => Equal,
        "PAGEUP" => PageUp,
        "PAGEDOWN" => PageDown,
        "BACKSLASH" => Backslash,
        _ => return None,
    })
}

/// Short display token for a key — the inverse of `key_from_name`, so configs round-trip.
pub fn key_name(code: KeyCode) -> &'static str {
    use KeyCode::*;
    match code {
        KeyA => "A",
        KeyB => "B",
        KeyC => "C",
        KeyD => "D",
        KeyE => "E",
        KeyF => "F",
        KeyG => "G",
        KeyH => "H",
        KeyI => "I",
        KeyJ => "J",
        KeyK => "K",
        KeyL => "L",
        KeyM => "M",
        KeyN => "N",
        KeyO => "O",
        KeyP => "P",
        KeyQ => "Q",
        KeyR => "R",
        KeyS => "S",
        KeyT => "T",
        KeyU => "U",
        KeyV => "V",
        KeyW => "W",
        KeyX => "X",
        KeyY => "Y",
        KeyZ => "Z",
        Space => "SPACE",
        ShiftLeft => "LSHIFT",
        ShiftRight => "RSHIFT",
        ControlLeft => "LCTRL",
        ControlRight => "RCTRL",
        Digit1 => "1",
        Digit2 => "2",
        Digit3 => "3",
        Digit4 => "4",
        Digit5 => "5",
        Digit6 => "6",
        Digit7 => "7",
        Digit8 => "8",
        Digit9 => "9",
        Digit0 => "0",
        Comma => "COMMA",
        Period => "PERIOD",
        Slash => "SLASH",
        Semicolon => "SEMICOLON",
        Quote => "QUOTE",
        ArrowUp => "UP",
        ArrowDown => "DOWN",
        ArrowLeft => "LEFT",
        ArrowRight => "RIGHT",
        BracketLeft => "LBRACKET",
        BracketRight => "RBRACKET",
        Minus => "MINUS",
        Equal => "EQUAL",
        PageUp => "PAGEUP",
        PageDown => "PAGEDOWN",
        Backslash => "BACKSLASH",
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
            (KeyZ, 0),
            (KeyS, 1),
            (KeyX, 2),
            (KeyD, 3),
            (KeyC, 4),
            (ShiftLeft, 5),
            (KeyM, 6),
            (KeyK, 7),
            (Comma, 8),
            (KeyL, 9),
            (Period, 10),
            (ShiftRight, 11),
        ],
        "BEAT_14K" => vec![
            (KeyZ, 0),
            (KeyS, 1),
            (KeyX, 2),
            (KeyD, 3),
            (KeyC, 4),
            (KeyF, 5),
            (KeyV, 6),
            (ShiftLeft, 7),
            (KeyM, 8),
            (KeyK, 9),
            (Comma, 10),
            (KeyL, 11),
            (Period, 12),
            (Semicolon, 13),
            (Slash, 14),
            (ShiftRight, 15),
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

/// The full user key configuration: per-mode lane bindings, the second key each scratch lane may
/// be spun backwards with, and the in-play control keys. Persisted as RON; absent fields fall back
/// to the built-in defaults.
///
/// `scratch_reverse` is a second map rather than a second token inside `lanes` so a key config
/// written before reverse spins existed keeps parsing byte for byte. It ships empty: a scratch lane
/// with no reverse key behaves exactly as it did with one key.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyConfig {
    pub lanes: BTreeMap<String, Vec<String>>,
    pub scratch_reverse: BTreeMap<String, Vec<String>>,
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
        KeyConfig { lanes, scratch_reverse: BTreeMap::new(), controls: ControlBinds::default() }
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

    /// The key each scratch lane of `mode` is spun backwards with, as `(key, lane)`. Only lanes the
    /// user has actually bound appear: a scratch lane with no reverse key is spun one way only,
    /// which is what every key config written before this existed asks for.
    pub fn scratch_reverse_keys(&self, mode: Mode) -> Vec<(KeyCode, usize)> {
        let Some(row) = self.scratch_reverse.get(mode_config_key(mode)) else {
            return Vec::new();
        };
        row.iter()
            .enumerate()
            .filter(|(lane, token)| *lane < mode.key && mode.is_scratch(*lane) && !token.is_empty())
            .filter_map(|(lane, token)| match key_from_name(token) {
                Some(code) => Some((code, lane)),
                None => {
                    eprintln!("keyconfig: mode {} scratch lane {lane} has unknown reverse key {token:?}; leaving it unbound", mode.name);
                    None
                }
            })
            .collect()
    }

    /// Rebind the reverse spin of one scratch lane; grows the mode's row to fit if needed.
    pub fn set_scratch_reverse(&mut self, mode: Mode, lane: usize, code: KeyCode) {
        let row = self.scratch_reverse.entry(mode_config_key(mode).to_string()).or_insert_with(|| vec![String::new(); mode.key]);
        if row.len() <= lane {
            row.resize(lane + 1, String::new());
        }
        row[lane] = key_name(code).to_string();
    }

    /// Current reverse-spin key token shown for a mode's scratch lane (empty string if unbound).
    pub fn scratch_reverse_token(&self, mode: Mode, lane: usize) -> String {
        self.scratch_reverse.get(mode_config_key(mode)).and_then(|row| row.get(lane)).cloned().unwrap_or_default()
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
        let all = ControlAction::ALL
            .into_iter()
            .filter_map(|a| self.control_key(a))
            .chain(self.lane_keys(mode).into_iter().map(|(k, _)| k))
            .chain(self.scratch_reverse_keys(mode).into_iter().map(|(k, _)| k));
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
