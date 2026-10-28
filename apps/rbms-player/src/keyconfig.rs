use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use rbms_model::Mode;
use rbms_model::translations::Catalog;
use rbms_play::ScratchDir;
use rbms_render::text::language;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

use crate::gamepad::PadConfig;
use crate::text::layout_play::LayoutPlayText;
use crate::text::menus::key_editor::{ControlLabelText, LaneLabelText, ShortcutGroupText, ShortcutLabelText};
use crate::text::menus::layout_pick::LayoutPickText;
use crate::text::notices::NoticeText;

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
        "BACKQUOTE" => Backquote,
        "TAB" => Tab,
        "ENTER" => Enter,
        "BACKSPACE" => Backspace,
        "DELETE" => Delete,
        "INSERT" => Insert,
        "HOME" => Home,
        "END" => End,
        "LALT" | "ALT" => AltLeft,
        "RALT" => AltRight,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        "NUM0" => Numpad0,
        "NUM1" => Numpad1,
        "NUM2" => Numpad2,
        "NUM3" => Numpad3,
        "NUM4" => Numpad4,
        "NUM5" => Numpad5,
        "NUM6" => Numpad6,
        "NUM7" => Numpad7,
        "NUM8" => Numpad8,
        "NUM9" => Numpad9,
        _ => return None,
    })
}

/// What [`key_name`] answers for a key the key config file has no word for. Such a key cannot be
/// bound: the word would not read back as a key.
pub const UNNAMED_KEY: &str = "?";

/// Whether a key puts a character on a line that is being typed: a letter, a digit, a mark, the
/// space bar or a digit of the number pad. While a search word is typed these are the word's
/// whatever is bound to them, and every other key is free to be a shortcut.
pub fn types_a_character(code: KeyCode) -> bool {
    use KeyCode::*;
    matches!(
        code,
        KeyA | KeyB
            | KeyC
            | KeyD
            | KeyE
            | KeyF
            | KeyG
            | KeyH
            | KeyI
            | KeyJ
            | KeyK
            | KeyL
            | KeyM
            | KeyN
            | KeyO
            | KeyP
            | KeyQ
            | KeyR
            | KeyS
            | KeyT
            | KeyU
            | KeyV
            | KeyW
            | KeyX
            | KeyY
            | KeyZ
            | Digit0
            | Digit1
            | Digit2
            | Digit3
            | Digit4
            | Digit5
            | Digit6
            | Digit7
            | Digit8
            | Digit9
            | Numpad0
            | Numpad1
            | Numpad2
            | Numpad3
            | Numpad4
            | Numpad5
            | Numpad6
            | Numpad7
            | Numpad8
            | Numpad9
            | Space
            | Comma
            | Period
            | Slash
            | Semicolon
            | Quote
            | BracketLeft
            | BracketRight
            | Minus
            | Equal
            | Backslash
            | Backquote
    )
}

/// Whether a key is the line's while a line is being typed: one that types a character, or one of
/// the keys a line is edited with. A shortcut on such a key is not read under a search box.
pub fn belongs_to_a_typed_line(code: KeyCode) -> bool {
    types_a_character(code) || matches!(code, KeyCode::Backspace | KeyCode::Delete | KeyCode::Home | KeyCode::End | KeyCode::ArrowLeft | KeyCode::ArrowRight)
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
        Backquote => "BACKQUOTE",
        Tab => "TAB",
        Enter => "ENTER",
        Backspace => "BACKSPACE",
        Delete => "DELETE",
        Insert => "INSERT",
        Home => "HOME",
        End => "END",
        AltLeft => "LALT",
        AltRight => "RALT",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        Numpad0 => "NUM0",
        Numpad1 => "NUM1",
        Numpad2 => "NUM2",
        Numpad3 => "NUM3",
        Numpad4 => "NUM4",
        Numpad5 => "NUM5",
        Numpad6 => "NUM6",
        Numpad7 => "NUM7",
        Numpad8 => "NUM8",
        Numpad9 => "NUM9",
        _ => UNNAMED_KEY,
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

/// The key the other direction of each scratch lane of `mode` ships on, as `(key, lane)`: the
/// Control key beside the Shift key the lane itself ships on, which is where the reference puts it
/// (`PlayModeConfig.KeyboardConfig.setKeyAssign`). A turntable with one key is spun one way only,
/// and so is a shade that is set by turning it with START held.
pub fn default_reverse_keys_for_mode(mode: Mode) -> Vec<(KeyCode, usize)> {
    let beside = |code: KeyCode| match code {
        KeyCode::ShiftLeft => Some(KeyCode::ControlLeft),
        KeyCode::ShiftRight => Some(KeyCode::ControlRight),
        _ => None,
    };
    default_keys_for_mode(mode).into_iter().filter(|(_, lane)| mode.is_scratch(*lane)).filter_map(|(code, lane)| Some((beside(code)?, lane))).collect()
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

/// A non-lane control of a run whose key is user-configurable.
///
/// The in-play controls move a run's scroll speed and lane shades, switch each shade on and off,
/// and move what the run is judged and scrolled by. START and SELECT are the two menu buttons of
/// the reference implementation: they are read as held states (see [`HeldKeys`]) rather than fired
/// once, and the screens combine them with the lane keys.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ControlAction {
    HiSpeedUp,
    HiSpeedDown,
    CoverUp,
    CoverDown,
    CoverToggle,
    LiftUp,
    LiftDown,
    LiftToggle,
    HiddenUp,
    HiddenDown,
    HiddenToggle,
    GreenUp,
    GreenDown,
    KeepGreenToggle,
    OffsetUp,
    OffsetDown,
    AutoAdjustToggle,
    Fine,
    Start,
    Select,
}

impl ControlAction {
    /// Every control, in the order the key editor and the key guide list them.
    pub const ALL: [ControlAction; 20] = [
        ControlAction::HiSpeedUp,
        ControlAction::HiSpeedDown,
        ControlAction::CoverUp,
        ControlAction::CoverDown,
        ControlAction::CoverToggle,
        ControlAction::LiftUp,
        ControlAction::LiftDown,
        ControlAction::LiftToggle,
        ControlAction::HiddenUp,
        ControlAction::HiddenDown,
        ControlAction::HiddenToggle,
        ControlAction::GreenUp,
        ControlAction::GreenDown,
        ControlAction::KeepGreenToggle,
        ControlAction::OffsetUp,
        ControlAction::OffsetDown,
        ControlAction::AutoAdjustToggle,
        ControlAction::Fine,
        ControlAction::Start,
        ControlAction::Select,
    ];

    /// What the control is called on screen.
    ///
    /// The lane cover comes down from the top of the field, so the amount going up is the cover
    /// coming down: the two cover rows say which way the cover moves and what that does. The lift
    /// raises the judgment line and the hidden cover grows upwards from it, and their rows say so
    /// the same way.
    pub fn label(self) -> &'static str {
        let text = match self {
            ControlAction::HiSpeedUp => ControlLabelText::HiSpeedUp,
            ControlAction::HiSpeedDown => ControlLabelText::HiSpeedDown,
            ControlAction::CoverUp => ControlLabelText::CoverUp,
            ControlAction::CoverDown => ControlLabelText::CoverDown,
            ControlAction::CoverToggle => ControlLabelText::CoverToggle,
            ControlAction::LiftUp => ControlLabelText::LiftUp,
            ControlAction::LiftDown => ControlLabelText::LiftDown,
            ControlAction::LiftToggle => ControlLabelText::LiftToggle,
            ControlAction::HiddenUp => ControlLabelText::HiddenUp,
            ControlAction::HiddenDown => ControlLabelText::HiddenDown,
            ControlAction::HiddenToggle => ControlLabelText::HiddenToggle,
            ControlAction::GreenUp => ControlLabelText::GreenUp,
            ControlAction::GreenDown => ControlLabelText::GreenDown,
            ControlAction::KeepGreenToggle => ControlLabelText::KeepGreenToggle,
            ControlAction::OffsetUp => ControlLabelText::OffsetUp,
            ControlAction::OffsetDown => ControlLabelText::OffsetDown,
            ControlAction::AutoAdjustToggle => ControlLabelText::AutoAdjustToggle,
            ControlAction::Fine => ControlLabelText::Fine,
            ControlAction::Start => ControlLabelText::Start,
            ControlAction::Select => ControlLabelText::Select,
        };
        text.get(language())
    }

    /// The name the control is stored under in the pad block of the key config file.
    ///
    /// It is not the name on screen: the first ten are the words a file written before the rows
    /// were renamed already holds, and they must keep meaning the same controls.
    pub fn config_key(self) -> &'static str {
        match self {
            ControlAction::HiSpeedUp => "HI-SPEED UP",
            ControlAction::HiSpeedDown => "HI-SPEED DOWN",
            ControlAction::CoverUp => "COVER UP",
            ControlAction::CoverDown => "COVER DOWN",
            ControlAction::CoverToggle => "COVER TOGGLE",
            ControlAction::LiftUp => "LIFT UP",
            ControlAction::LiftDown => "LIFT DOWN",
            ControlAction::LiftToggle => "LIFT TOGGLE",
            ControlAction::HiddenUp => "HIDDEN+ UP",
            ControlAction::HiddenDown => "HIDDEN+ DOWN",
            ControlAction::HiddenToggle => "HIDDEN+ TOGGLE",
            ControlAction::GreenUp => "GREEN UP",
            ControlAction::GreenDown => "GREEN DOWN",
            ControlAction::KeepGreenToggle => "KEEP GREEN TOGGLE",
            ControlAction::OffsetUp => "JUDGE OFFSET UP",
            ControlAction::OffsetDown => "JUDGE OFFSET DOWN",
            ControlAction::AutoAdjustToggle => "AUTO ADJUST TOGGLE",
            ControlAction::Fine => "FINE STEP",
            ControlAction::Start => "START",
            ControlAction::Select => "SELECT",
        }
    }

    /// Whether the control moves or switches the lane cover, the lift or the hidden cover.
    pub fn moves_a_shade(self) -> bool {
        matches!(
            self,
            ControlAction::CoverUp
                | ControlAction::CoverDown
                | ControlAction::CoverToggle
                | ControlAction::LiftUp
                | ControlAction::LiftDown
                | ControlAction::LiftToggle
                | ControlAction::HiddenUp
                | ControlAction::HiddenDown
                | ControlAction::HiddenToggle
        )
    }

    /// Whether this is one of the two menu buttons, which the song browser reads as well as a run.
    pub fn is_menu_button(self) -> bool {
        matches!(self, ControlAction::Start | ControlAction::Select)
    }

    /// Whether a key that also plays a lane keeps playing it instead of acting as this control.
    ///
    /// START, SELECT and the lane cover switch ship bound to keys that a key config written before
    /// they existed may already have given to a lane; the lane has to win, or loading an old file
    /// would silently kill a column. The older controls are resolved before lanes, as they always
    /// were.
    pub fn yields_to_lanes(self) -> bool {
        matches!(self, ControlAction::Start | ControlAction::Select | ControlAction::CoverToggle)
    }

    /// The key token a control was given after it had shipped without one, and the generation of
    /// shipped keys ([`KEY_GENERATION`]) it was given it in, or `None` for a control that ships as
    /// it first did. A key config that has taken an older generation was written while the control
    /// shipped with no key, so its having none there is not the player's choice
    /// ([`KeyConfig::control_key`]).
    pub fn given_later(self) -> Option<(&'static str, u32)> {
        match self {
            ControlAction::OffsetUp => Some((DEFAULT_OFFSET_UP_KEY, OFFSET_KEYS_GENERATION)),
            ControlAction::OffsetDown => Some((DEFAULT_OFFSET_DOWN_KEY, OFFSET_KEYS_GENERATION)),
            _ => None,
        }
    }
}

/// User-configurable control key tokens.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlBinds {
    pub hispeed_up: String,
    pub hispeed_down: String,
    pub cover_up: String,
    pub cover_down: String,
    pub lift_up: String,
    pub lift_down: String,
    /// The hidden band has no shipped binding, so it starts unbound and is picked in the key
    /// editor.
    pub hidden_up: String,
    pub hidden_down: String,
    /// START and SELECT. The reference implementation ships them on Q and W
    /// (`PlayModeConfig.java:320-321`); START takes A here because Q already belongs to the IR ranking
    /// panel's previous-profile key, and a held START must not also page that panel.
    pub start: String,
    pub select: String,
    /// The lane cover on or off, which START pressed twice also does.
    pub cover_toggle: String,
    /// The controls below ship unbound but for the two judge offset keys: each is a key a player
    /// asks for, not one a run needs.
    pub lift_toggle: String,
    pub hidden_toggle: String,
    pub green_up: String,
    pub green_down: String,
    pub keep_green_toggle: String,
    /// The judge offset a millisecond up and down. They shipped unbound until the offset became
    /// where the notes are drawn, which a player sets while watching them; a key config from before
    /// that takes the keys on the terms of [`KeyConfig::control_key`].
    pub offset_up: String,
    pub offset_down: String,
    /// The switch that lets the offset adjust itself stays unbound: a stray press in a run would
    /// leave the offset drifting with nothing but a caption to say so.
    pub auto_adjust_toggle: String,
    /// The key held for the finer step of the covers and of a held green number. It ships unbound
    /// because no one key is free on every shipped layout: left without a key, the modifier is a
    /// shift key that plays no lane and runs no control ([`crate::AppShared::fine_pressed`]).
    pub fine: String,
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
            hidden_up: String::new(),
            hidden_down: String::new(),
            start: DEFAULT_START_KEY.into(),
            select: DEFAULT_SELECT_KEY.into(),
            cover_toggle: DEFAULT_COVER_TOGGLE_KEY.into(),
            lift_toggle: String::new(),
            hidden_toggle: String::new(),
            green_up: String::new(),
            green_down: String::new(),
            keep_green_toggle: String::new(),
            offset_up: DEFAULT_OFFSET_UP_KEY.into(),
            offset_down: DEFAULT_OFFSET_DOWN_KEY.into(),
            auto_adjust_toggle: String::new(),
            fine: String::new(),
        }
    }
}

/// The key token START ships bound to.
pub const DEFAULT_START_KEY: &str = "A";

/// The key token SELECT ships bound to, which is the reference implementation's own.
pub const DEFAULT_SELECT_KEY: &str = "W";

/// The key token the lane cover switch ships bound to: the key beside the two the lift ships on,
/// and one no shipped layout plays a lane or reads a control from.
pub const DEFAULT_COVER_TOGGLE_KEY: &str = "BACKSLASH";

/// The key tokens the judge offset ships bound to, down and up: the two digits at the right end of
/// the number row, beside the keys the lift and the lane cover switch ship on. No shipped layout
/// plays a lane on a digit, no control and no key of a replay under analysis is read from one, and
/// every keyboard has them.
pub const DEFAULT_OFFSET_DOWN_KEY: &str = "9";
pub const DEFAULT_OFFSET_UP_KEY: &str = "0";

/// The generation of shipped keys a key config written before generations were counted has taken.
pub const FIRST_KEY_GENERATION: u32 = 0;

/// The generation the two judge offset keys first shipped bound in.
pub const OFFSET_KEYS_GENERATION: u32 = 1;

/// The generation the other direction of every scratch lane first shipped bound in
/// ([`default_reverse_keys_for_mode`]).
pub const REVERSE_KEYS_GENERATION: u32 = 2;

/// The generation of shipped keys this build ships: how many times something that shipped without
/// a key has been given one since a key config first held controls. A key config remembers the
/// generation it has taken ([`KeyConfig::generation`]).
pub const KEY_GENERATION: u32 = REVERSE_KEYS_GENERATION;

/// What a key config file that does not say which generation it has taken is read as having taken.
const fn first_key_generation() -> u32 {
    FIRST_KEY_GENERATION
}

/// The reverse keys a key config file that holds none is read as holding: none, and not the ones a
/// fresh configuration ships with. What such a file reads them from is settled against the keys it
/// does hold ([`KeyConfig::scratch_reverse_keys`]).
fn no_reverse_keys() -> BTreeMap<String, Vec<String>> {
    BTreeMap::new()
}

impl ControlBinds {
    fn slot(&mut self, action: ControlAction) -> &mut String {
        match action {
            ControlAction::HiSpeedUp => &mut self.hispeed_up,
            ControlAction::HiSpeedDown => &mut self.hispeed_down,
            ControlAction::CoverUp => &mut self.cover_up,
            ControlAction::CoverDown => &mut self.cover_down,
            ControlAction::CoverToggle => &mut self.cover_toggle,
            ControlAction::LiftUp => &mut self.lift_up,
            ControlAction::LiftDown => &mut self.lift_down,
            ControlAction::LiftToggle => &mut self.lift_toggle,
            ControlAction::HiddenUp => &mut self.hidden_up,
            ControlAction::HiddenDown => &mut self.hidden_down,
            ControlAction::HiddenToggle => &mut self.hidden_toggle,
            ControlAction::GreenUp => &mut self.green_up,
            ControlAction::GreenDown => &mut self.green_down,
            ControlAction::KeepGreenToggle => &mut self.keep_green_toggle,
            ControlAction::OffsetUp => &mut self.offset_up,
            ControlAction::OffsetDown => &mut self.offset_down,
            ControlAction::AutoAdjustToggle => &mut self.auto_adjust_toggle,
            ControlAction::Fine => &mut self.fine,
            ControlAction::Start => &mut self.start,
            ControlAction::Select => &mut self.select,
        }
    }

    fn token(&self, action: ControlAction) -> &str {
        match action {
            ControlAction::HiSpeedUp => &self.hispeed_up,
            ControlAction::HiSpeedDown => &self.hispeed_down,
            ControlAction::CoverUp => &self.cover_up,
            ControlAction::CoverDown => &self.cover_down,
            ControlAction::CoverToggle => &self.cover_toggle,
            ControlAction::LiftUp => &self.lift_up,
            ControlAction::LiftDown => &self.lift_down,
            ControlAction::LiftToggle => &self.lift_toggle,
            ControlAction::HiddenUp => &self.hidden_up,
            ControlAction::HiddenDown => &self.hidden_down,
            ControlAction::HiddenToggle => &self.hidden_toggle,
            ControlAction::GreenUp => &self.green_up,
            ControlAction::GreenDown => &self.green_down,
            ControlAction::KeepGreenToggle => &self.keep_green_toggle,
            ControlAction::OffsetUp => &self.offset_up,
            ControlAction::OffsetDown => &self.offset_down,
            ControlAction::AutoAdjustToggle => &self.auto_adjust_toggle,
            ControlAction::Fine => &self.fine,
            ControlAction::Start => &self.start,
            ControlAction::Select => &self.select,
        }
    }
}

/// Which screen reads a shortcut, which is what decides the keys it may not share.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ShortcutScreen {
    /// The song browser, whichever of the built-in one and a skin's draws it.
    Browser,
    /// The result of a run.
    Result,
    /// A replay being played back under analysis.
    Replay,
}

/// The headings the key editor and the key guide list the shortcuts under.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ShortcutGroup {
    Browser,
    Screens,
    SkinBrowser,
    Result,
    Replay,
}

impl ShortcutGroup {
    pub const ALL: [ShortcutGroup; 5] =
        [ShortcutGroup::Browser, ShortcutGroup::Screens, ShortcutGroup::SkinBrowser, ShortcutGroup::Result, ShortcutGroup::Replay];

    pub fn title(self) -> &'static str {
        let text = match self {
            ShortcutGroup::Browser => ShortcutGroupText::Browser,
            ShortcutGroup::Screens => ShortcutGroupText::Screens,
            ShortcutGroup::SkinBrowser => ShortcutGroupText::SkinBrowser,
            ShortcutGroup::Result => ShortcutGroupText::Result,
            ShortcutGroup::Replay => ShortcutGroupText::Replay,
        };
        text.get(language())
    }
}

/// A key of the application's own, outside a run's controls, whose key is user-configurable: the
/// shortcuts of the song browser, the keys that open its screens, the two ways on from a result,
/// and the transport of a replay under analysis.
///
/// The second browser group is the reference implementation's number keys
/// (`MusicSelectInputProcessor.java:71-110`, `MusicSelector.java:296`), which only a browser a skin
/// draws reads. They are entries of their own rather than second keys of the entries above them
/// because that is what they are in the browser: a second set, read on one kind of browser only.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shortcut {
    Search,
    Filter,
    Sort,
    Practice,
    Favourite,
    Records,
    Ranking,
    /// Fetch every registered difficulty table again, beside the browser.
    ReloadTables,
    /// Open the screen that lays the chart under the cursor out as the player writes it, and plays it
    /// so as RANDOM with the seed that gives that layout.
    LayoutPick,
    OptionsPin,
    OptionsHold,
    Settings,
    SkinMenu,
    PlayMenu,
    KeyConfig,
    Folders,
    Tables,
    RefMode,
    RefSort,
    RefLnMode,
    RefReplay,
    RefDetail,
    RefKeyConfig,
    RefSearch,
    RefFavourite,
    Retry,
    NextSong,
    ReplayPause,
    ReplayFaster,
    ReplaySlower,
    ReplayForward,
    ReplayBack,
    /// Play the chart of the replay being watched by hand, laid out as the replay was.
    ReplayPlayLayout,
}

impl Shortcut {
    /// Every shortcut, group by group, in the order the key editor and the key guide list them.
    pub const ALL: [Shortcut; 33] = [
        Shortcut::Search,
        Shortcut::Filter,
        Shortcut::Sort,
        Shortcut::Practice,
        Shortcut::Favourite,
        Shortcut::Records,
        Shortcut::Ranking,
        Shortcut::ReloadTables,
        Shortcut::LayoutPick,
        Shortcut::OptionsPin,
        Shortcut::OptionsHold,
        Shortcut::Settings,
        Shortcut::SkinMenu,
        Shortcut::PlayMenu,
        Shortcut::KeyConfig,
        Shortcut::Folders,
        Shortcut::Tables,
        Shortcut::RefMode,
        Shortcut::RefSort,
        Shortcut::RefLnMode,
        Shortcut::RefReplay,
        Shortcut::RefDetail,
        Shortcut::RefKeyConfig,
        Shortcut::RefSearch,
        Shortcut::RefFavourite,
        Shortcut::Retry,
        Shortcut::NextSong,
        Shortcut::ReplayPause,
        Shortcut::ReplayFaster,
        Shortcut::ReplaySlower,
        Shortcut::ReplayForward,
        Shortcut::ReplayBack,
        Shortcut::ReplayPlayLayout,
    ];

    pub fn group(self) -> ShortcutGroup {
        match self {
            Shortcut::Search
            | Shortcut::Filter
            | Shortcut::Sort
            | Shortcut::Practice
            | Shortcut::Favourite
            | Shortcut::Records
            | Shortcut::Ranking
            | Shortcut::ReloadTables
            | Shortcut::LayoutPick
            | Shortcut::OptionsPin
            | Shortcut::OptionsHold => ShortcutGroup::Browser,
            Shortcut::Settings | Shortcut::SkinMenu | Shortcut::PlayMenu | Shortcut::KeyConfig | Shortcut::Folders | Shortcut::Tables => ShortcutGroup::Screens,
            Shortcut::RefMode
            | Shortcut::RefSort
            | Shortcut::RefLnMode
            | Shortcut::RefReplay
            | Shortcut::RefDetail
            | Shortcut::RefKeyConfig
            | Shortcut::RefSearch
            | Shortcut::RefFavourite => ShortcutGroup::SkinBrowser,
            Shortcut::Retry | Shortcut::NextSong => ShortcutGroup::Result,
            Shortcut::ReplayPause
            | Shortcut::ReplayFaster
            | Shortcut::ReplaySlower
            | Shortcut::ReplayForward
            | Shortcut::ReplayBack
            | Shortcut::ReplayPlayLayout => ShortcutGroup::Replay,
        }
    }

    /// The screen that reads it.
    pub fn screen(self) -> ShortcutScreen {
        match self.group() {
            ShortcutGroup::Browser | ShortcutGroup::Screens | ShortcutGroup::SkinBrowser => ShortcutScreen::Browser,
            ShortcutGroup::Result => ShortcutScreen::Result,
            ShortcutGroup::Replay => ShortcutScreen::Replay,
        }
    }

    /// What the shortcut is called in the key editor.
    pub fn label(self) -> &'static str {
        let text = match self {
            Shortcut::Search | Shortcut::RefSearch => ShortcutLabelText::Search,
            Shortcut::Filter => ShortcutLabelText::Filter,
            Shortcut::Sort => ShortcutLabelText::Sort,
            Shortcut::Practice => ShortcutLabelText::Practice,
            Shortcut::Favourite | Shortcut::RefFavourite => ShortcutLabelText::Favourite,
            Shortcut::Records => ShortcutLabelText::Records,
            Shortcut::Ranking => ShortcutLabelText::Ranking,
            Shortcut::ReloadTables => ShortcutLabelText::ReloadTables,
            Shortcut::OptionsPin => ShortcutLabelText::OptionsPin,
            Shortcut::OptionsHold => ShortcutLabelText::OptionsHold,
            Shortcut::Settings => ShortcutLabelText::Settings,
            Shortcut::SkinMenu => ShortcutLabelText::SkinMenu,
            Shortcut::PlayMenu => ShortcutLabelText::PlayMenu,
            Shortcut::KeyConfig | Shortcut::RefKeyConfig => ShortcutLabelText::KeyConfig,
            Shortcut::Folders => ShortcutLabelText::Folders,
            Shortcut::Tables => ShortcutLabelText::Tables,
            Shortcut::RefMode => ShortcutLabelText::ModeFilter,
            Shortcut::RefSort => ShortcutLabelText::SortOrder,
            Shortcut::RefLnMode => ShortcutLabelText::LongNoteMode,
            Shortcut::RefReplay => ShortcutLabelText::ReplaySlot,
            Shortcut::RefDetail => ShortcutLabelText::DetailOptions,
            Shortcut::Retry => ShortcutLabelText::Retry,
            Shortcut::NextSong => ShortcutLabelText::NextSong,
            Shortcut::ReplayPause => ShortcutLabelText::ReplayPause,
            Shortcut::ReplayFaster => ShortcutLabelText::ReplayFaster,
            Shortcut::ReplaySlower => ShortcutLabelText::ReplaySlower,
            Shortcut::ReplayForward => ShortcutLabelText::ReplayForward,
            Shortcut::ReplayBack => ShortcutLabelText::ReplayBack,
            Shortcut::ReplayPlayLayout => return LayoutPlayText::ShortcutLabel.get(language()),
            Shortcut::LayoutPick => return LayoutPickText::ShortcutLabel.get(language()),
        };
        text.get(language())
    }

    /// The name the shortcut is stored under in the key config file.
    pub fn config_key(self) -> &'static str {
        match self {
            Shortcut::Search => "search",
            Shortcut::Filter => "filter",
            Shortcut::Sort => "sort",
            Shortcut::Practice => "practice",
            Shortcut::Favourite => "favourite",
            Shortcut::Records => "records",
            Shortcut::Ranking => "ranking",
            Shortcut::ReloadTables => "reload_tables",
            Shortcut::LayoutPick => "layout_pick",
            Shortcut::OptionsPin => "options_pin",
            Shortcut::OptionsHold => "options_hold",
            Shortcut::Settings => "settings",
            Shortcut::SkinMenu => "skin_menu",
            Shortcut::PlayMenu => "play_menu",
            Shortcut::KeyConfig => "key_config",
            Shortcut::Folders => "folders",
            Shortcut::Tables => "tables",
            Shortcut::RefMode => "number_mode",
            Shortcut::RefSort => "number_sort",
            Shortcut::RefLnMode => "number_ln_mode",
            Shortcut::RefReplay => "number_replay",
            Shortcut::RefDetail => "number_detail",
            Shortcut::RefKeyConfig => "number_key_config",
            Shortcut::RefSearch => "number_search",
            Shortcut::RefFavourite => "number_favourite",
            Shortcut::Retry => "result_retry",
            Shortcut::NextSong => "result_next",
            Shortcut::ReplayPause => "replay_pause",
            Shortcut::ReplayFaster => "replay_faster",
            Shortcut::ReplaySlower => "replay_slower",
            Shortcut::ReplayForward => "replay_forward",
            Shortcut::ReplayBack => "replay_back",
            Shortcut::ReplayPlayLayout => "replay_play_layout",
        }
    }

    /// The key the shortcut ships on. Every one of them but the keys to the key editor and to the
    /// play menu, and the key that reloads the tables, is the key the application read for it before
    /// any of them could be changed. The reload ships on a function key the browser read nothing
    /// from, next to the one the key editor is on. The key that plays a watched replay's layout
    /// ships on one no shipped layout plays a lane on and no control or other key of a replay under
    /// analysis is read from. The key to LAYOUT PICK ships on a letter no shipped layout plays a lane
    /// on and nothing else of the browser is read from.
    pub fn default_key(self) -> KeyCode {
        match self {
            Shortcut::Search => KeyCode::Slash,
            Shortcut::Filter => KeyCode::F2,
            Shortcut::Sort => KeyCode::F3,
            Shortcut::Practice => KeyCode::F4,
            Shortcut::Favourite => KeyCode::KeyF,
            Shortcut::Records => KeyCode::KeyR,
            Shortcut::Ranking => KeyCode::KeyI,
            Shortcut::ReloadTables => KeyCode::F6,
            Shortcut::LayoutPick => KeyCode::KeyY,
            Shortcut::OptionsPin => crate::app_options::TOGGLE_KEY,
            Shortcut::OptionsHold => crate::app_options::HOLD_KEY,
            Shortcut::Settings => KeyCode::Tab,
            Shortcut::SkinMenu => crate::stage::select::SKIN_MENU_KEY,
            Shortcut::PlayMenu => crate::stage::select::PLAY_MENU_KEY,
            Shortcut::KeyConfig => KeyCode::F5,
            Shortcut::Folders => KeyCode::KeyO,
            Shortcut::Tables => KeyCode::KeyT,
            Shortcut::RefMode => KeyCode::Digit1,
            Shortcut::RefSort => KeyCode::Digit2,
            Shortcut::RefLnMode => KeyCode::Digit3,
            Shortcut::RefReplay => KeyCode::Digit4,
            Shortcut::RefDetail => KeyCode::Digit5,
            Shortcut::RefKeyConfig => KeyCode::Digit6,
            Shortcut::RefSearch => KeyCode::Digit0,
            Shortcut::RefFavourite => KeyCode::F9,
            Shortcut::Retry => KeyCode::KeyR,
            Shortcut::NextSong => KeyCode::KeyN,
            Shortcut::ReplayPause => KeyCode::Space,
            Shortcut::ReplayFaster => KeyCode::Equal,
            Shortcut::ReplaySlower => KeyCode::Minus,
            Shortcut::ReplayForward => KeyCode::PageUp,
            Shortcut::ReplayBack => KeyCode::PageDown,
            Shortcut::ReplayPlayLayout => KeyCode::KeyJ,
        }
    }

    /// Whether a controller can be given this shortcut. The three that are read as a key held down
    /// and let go -- the option overlay's two keys and the detail panel's -- are the keyboard's
    /// alone: the overlay is driven by key events, and a controller has START and SELECT for the
    /// panel.
    pub fn takes_a_pad(self) -> bool {
        !matches!(self, Shortcut::OptionsPin | Shortcut::OptionsHold | Shortcut::RefDetail)
    }
}

/// The keys a screen reads for itself and that nothing on it can be rebound onto: the ones that
/// move through a list, confirm and cancel, and on the browser the key to the guide that lists
/// every other key, which the skin's footer names.
pub fn fixed_keys_of(screen: ShortcutScreen) -> &'static [KeyCode] {
    match screen {
        ShortcutScreen::Browser => {
            &[KeyCode::Escape, KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::ArrowLeft, KeyCode::ArrowRight, GUIDE_KEY]
        }
        ShortcutScreen::Result => &[KeyCode::Escape, KeyCode::Enter, KeyCode::NumpadEnter],
        ShortcutScreen::Replay => &[KeyCode::Escape],
    }
}

/// The key that puts the browser's key guide up and takes it down. It is not rebound: the guide is
/// where every key that can be is listed, and a skin's footer has to be able to name the way to it.
pub const GUIDE_KEY: KeyCode = KeyCode::KeyH;

/// One thing a key or a controller element can be bound to, for the key editor and the rules that
/// keep two of them off one key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bindable {
    Lane(usize),
    /// The second key a scratch lane may be spun backwards with.
    ScratchReverse(usize),
    Control(ControlAction),
    Shortcut(Shortcut),
}

impl Bindable {
    /// What it is called in a message of the key editor.
    pub fn label(self, mode: Mode) -> String {
        match self {
            Bindable::Lane(lane) if mode.is_scratch(lane) => LaneLabelText::Scratch.fill(language(), &[&(lane + 1)]),
            Bindable::Lane(lane) => LaneLabelText::Lane.fill(language(), &[&(lane + 1)]),
            Bindable::ScratchReverse(lane) => LaneLabelText::ScratchReverse.fill(language(), &[&(lane + 1)]),
            Bindable::Control(action) => action.label().to_string(),
            Bindable::Shortcut(shortcut) => shortcut.label().to_string(),
        }
    }

    /// Whether the two are read at the same moment on some screen, which is when one key for both
    /// would do both at once or leave one of them dead.
    ///
    /// A run reads its lanes and its controls together, and a replay under analysis reads its
    /// transport beside the controls. The browser reads its shortcuts beside START and SELECT. A
    /// lane key is read on the browser too, as a key of the reference's key table, but there it
    /// only ever adds to what a shortcut does and the shipped layout already shares one, so that
    /// pair is not refused ([`KeyConfig::lane_sharing`] names it instead).
    pub fn read_with(self, other: Bindable) -> bool {
        use Bindable::{Control, Lane, ScratchReverse, Shortcut};
        match (self, other) {
            (Lane(_) | ScratchReverse(_) | Control(_), Lane(_) | ScratchReverse(_) | Control(_)) => true,
            (Shortcut(shortcut), Control(action)) | (Control(action), Shortcut(shortcut)) => match shortcut.screen() {
                ShortcutScreen::Browser => action.is_menu_button(),
                ShortcutScreen::Replay => true,
                ShortcutScreen::Result => false,
            },
            (Shortcut(one), Shortcut(two)) => one.screen() == two.screen(),
            (Shortcut(_), Lane(_) | ScratchReverse(_)) | (Lane(_) | ScratchReverse(_), Shortcut(_)) => false,
        }
    }

    /// The keys the screen that reads this keeps for itself, which it cannot be bound to: Escape
    /// for a lane or a control of a run, and for a shortcut the fixed keys of its screen
    /// ([`fixed_keys_of`]).
    pub fn fixed_keys(self) -> &'static [KeyCode] {
        match self {
            Bindable::Shortcut(shortcut) => fixed_keys_of(shortcut.screen()),
            Bindable::Lane(_) | Bindable::ScratchReverse(_) | Bindable::Control(_) => &[KeyCode::Escape],
        }
    }

    /// Everything that can be bound for `mode`, in the order the key editor lists it.
    pub fn all(mode: Mode) -> Vec<Bindable> {
        let lanes = (0..mode.key).map(Bindable::Lane);
        let reverse = (0..mode.key).filter(|&lane| mode.is_scratch(lane)).map(Bindable::ScratchReverse);
        let controls = ControlAction::ALL.into_iter().map(Bindable::Control);
        let shortcuts = Shortcut::ALL.into_iter().map(Bindable::Shortcut);
        lanes.chain(reverse).chain(controls).chain(shortcuts).collect()
    }
}

/// Why a key cannot be given to something.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Clash {
    /// Something read at the same moment already has the key.
    With(Bindable),
    /// The screen reads the key for itself.
    Fixed,
}

/// Why `code` cannot be given to `target`, if it cannot, among everything in `bound` with the key
/// each has: the screen that reads `target` keeps the key for itself, or something read at the same
/// moment already has it ([`Bindable::read_with`]). What `target` itself has is never a clash.
pub fn clash_among(bound: impl IntoIterator<Item = (Bindable, Option<KeyCode>)>, target: Bindable, code: KeyCode) -> Option<Clash> {
    if target.fixed_keys().contains(&code) {
        return Some(Clash::Fixed);
    }
    bound.into_iter().find(|(other, key)| *other != target && *key == Some(code) && target.read_with(*other)).map(|(other, _)| Clash::With(other))
}

/// How many key indices the screens ask about. The reference implementation numbers the keys of a
/// select-screen layout 0 to 8: keys one to seven are 0 to 6 and the two turntable directions are
/// 7 and 8 (`MusicSelectKeyProperty.java`, `BEAT_7K`).
pub const KEY_INDEX_COUNT: usize = 9;

/// The key index of a turntable spun forward, which the select screen reads as "next row".
pub const SCRATCH_FORWARD_INDEX: usize = 7;

/// The key index of a turntable spun backward, which the select screen reads as "previous row".
pub const SCRATCH_BACKWARD_INDEX: usize = 8;

/// The reference implementation's key index (`0..KEY_INDEX_COUNT`) of one input on a lane of
/// `mode`, or `None` for a lane that has no index.
///
/// A scratch lane is index 7 spun forward and index 8 spun backward. Every other lane is its
/// position on its own side: a double-play mode repeats the first side's indices on the second,
/// which is what `MusicSelectKeyProperty.BEAT_14K` does with its second set of nine. Pop'n has no
/// scratch, so its nine buttons are 0 to 8. A mode with a scratch keeps 7 and 8 for it, so a lane
/// that would land there (the 24-key layout) has no index.
pub fn key_index_of(mode: Mode, lane: usize, dir: ScratchDir) -> Option<usize> {
    if lane >= mode.key {
        return None;
    }
    if mode.is_scratch(lane) {
        return Some(match dir {
            ScratchDir::Forward => SCRATCH_FORWARD_INDEX,
            ScratchDir::Backward => SCRATCH_BACKWARD_INDEX,
        });
    }
    let position = lane % (mode.key / usize::from(mode.player).max(1));
    let limit = if mode.scratch.is_empty() { KEY_INDEX_COUNT } else { SCRATCH_FORWARD_INDEX };
    (position < limit).then_some(position)
}

/// The keyboard keys that are down right now, kept from the press and release events the window
/// reports.
///
/// It holds physical keys rather than lanes, so a key rebound while it is down (the editor does
/// this) is read against the new binding on the next query instead of leaving a lane held. It is
/// runtime state, never written to the key config file.
#[derive(Clone, Debug, Default)]
pub struct HeldKeys {
    down: HashSet<KeyCode>,
}

impl HeldKeys {
    pub fn press(&mut self, code: KeyCode) {
        self.down.insert(code);
    }

    pub fn release(&mut self, code: KeyCode) {
        self.down.remove(&code);
    }

    pub fn is_down(&self, code: KeyCode) -> bool {
        self.down.contains(&code)
    }

    pub fn iter(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.down.iter().copied()
    }

    /// Forget every key, for a window that lost focus and so will never see them released.
    pub fn clear(&mut self) {
        self.down.clear();
    }
}

/// The full user key configuration: per-mode lane bindings, the second key each scratch lane may
/// be spun backwards with, and the control keys. Persisted as RON; absent fields fall back to the
/// built-in defaults.
///
/// `scratch_reverse` is a second map rather than a second token inside `lanes` so a key config
/// written before reverse spins existed keeps parsing byte for byte. It ships with the key each
/// scratch lane's other direction is on ([`default_reverse_keys_for_mode`]); it shipped empty at
/// first, and a file from then reads those keys by its `generation`.
///
/// `pad` holds the same three things for a controller. It lives here rather than in the settings
/// file because it is the same question the rest of this file answers — what plays which lane —
/// and a key config written before a controller was readable still loads, filling it with a table
/// that binds nothing.
///
/// `shortcuts` holds the application's own keys, by [`Shortcut::config_key`], and only the ones the
/// player has moved: a shortcut with no entry reads the key it ships on, and one with an empty
/// entry reads none. A key config written before the shortcuts could be moved has no such map and
/// so reads every one of them where it always was.
///
/// `generation` is which generation of shipped keys the configuration has taken
/// ([`KEY_GENERATION`]). A file written before a control or a reverse spin shipped with a key holds
/// it with none, which was then the only thing it could hold; the generation is what tells that
/// apart from a key the player took away. A file that does not hold it is one from before any were
/// counted.
///
/// `held` is not configuration. It sits beside the bindings because a held key only means
/// something read against them, and it is skipped by the file.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyConfig {
    pub lanes: BTreeMap<String, Vec<String>>,
    #[serde(default = "no_reverse_keys")]
    pub scratch_reverse: BTreeMap<String, Vec<String>>,
    pub controls: ControlBinds,
    pub shortcuts: BTreeMap<String, String>,
    pub pad: PadConfig,
    #[serde(default = "first_key_generation")]
    pub generation: u32,
    #[serde(skip)]
    pub held: HeldKeys,
}

impl Default for KeyConfig {
    fn default() -> Self {
        let mut lanes = BTreeMap::new();
        let mut scratch_reverse = BTreeMap::new();
        for &mode in Mode::ALL {
            let mut row = vec![String::new(); mode.key];
            for (code, lane) in default_keys_for_mode(mode) {
                if lane < row.len() {
                    row[lane] = key_name(code).to_string();
                }
            }
            lanes.insert(mode_config_key(mode).to_string(), row);
            for (code, lane) in default_reverse_keys_for_mode(mode) {
                set_token(&mut scratch_reverse, mode, lane, key_name(code).to_string());
            }
        }
        KeyConfig {
            lanes,
            scratch_reverse,
            controls: ControlBinds::default(),
            shortcuts: BTreeMap::new(),
            pad: PadConfig::default(),
            generation: KEY_GENERATION,
            held: HeldKeys::default(),
        }
    }
}

/// Write one lane's word into a per-mode row, growing the row to fit.
fn set_token(rows: &mut BTreeMap<String, Vec<String>>, mode: Mode, lane: usize, token: String) {
    let row = rows.entry(mode_config_key(mode).to_string()).or_insert_with(|| vec![String::new(); mode.key]);
    if row.len() <= lane {
        row.resize(lane + 1, String::new());
    }
    row[lane] = token;
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
                    None => crate::notify::notify_text(crate::notify::Level::Warn, NoticeText::KeyConfigUnknownKey, &[&mode.name, &lane, &format!("{tok:?}")]),
                }
            }
        }
        by_lane.into_iter().enumerate().filter_map(|(lane, code)| code.map(|c| (c, lane))).collect()
    }

    /// The key each scratch lane of `mode` is spun backwards with, as `(key, lane)`. A scratch lane
    /// with no reverse key is left out, and is spun one way only.
    ///
    /// It is the key the configuration holds. A lane it holds none for reads the key its other
    /// direction ships on when the configuration was written before that key shipped
    /// ([`KeyConfig::reverse_key_given_later`]): the rule a control given a key later follows
    /// ([`KeyConfig::control_key`]).
    pub fn scratch_reverse_keys(&self, mode: Mode) -> Vec<(KeyCode, usize)> {
        let row = self.scratch_reverse.get(mode_config_key(mode));
        (0..mode.key)
            .filter(|lane| mode.is_scratch(*lane))
            .filter_map(|lane| {
                let token = row.and_then(|row| row.get(lane)).map_or("", String::as_str);
                if token.is_empty() {
                    return self.reverse_key_given_later(mode, lane).map(|code| (code, lane));
                }
                let code = key_from_name(token);
                if code.is_none() {
                    crate::notify::notify_text(crate::notify::Level::Warn, NoticeText::KeyConfigUnknownReverseKey, &[&mode.name, &lane, &format!("{token:?}")]);
                }
                code.map(|code| (code, lane))
            })
            .collect()
    }

    /// The key a scratch lane the configuration holds no reverse key for is spun backwards with all
    /// the same: the one its other direction ships on, when the configuration was written while it
    /// shipped with none ([`REVERSE_KEYS_GENERATION`]) and gives that key to nothing else a run of
    /// `mode` reads -- a lane or another reverse spin of that mode, a control, a key of a replay
    /// under analysis. `None` once the configuration has taken the generation: a lane it then holds
    /// no reverse key for is one the player took the key from.
    ///
    /// The lanes asked are the ones the file names. A lane it names no key for plays the key it
    /// ships on, and no lane ships on a key a reverse spin ships on.
    fn reverse_key_given_later(&self, mode: Mode, lane: usize) -> Option<KeyCode> {
        if self.generation >= REVERSE_KEYS_GENERATION {
            return None;
        }
        let (code, _) = default_reverse_keys_for_mode(mode).into_iter().find(|(_, shipped)| *shipped == lane)?;
        let rows = [&self.lanes, &self.scratch_reverse].map(|rows| rows.get(mode_config_key(mode)));
        let played = rows.into_iter().flatten().flatten().any(|token| key_from_name(token) == Some(code));
        (!played && !self.reads_beside_the_lanes(None, code)).then_some(code)
    }

    /// Rebind the reverse spin of one scratch lane; grows the mode's row to fit if needed.
    pub fn set_scratch_reverse(&mut self, mode: Mode, lane: usize, code: KeyCode) {
        self.settle_shipped_keys();
        let row = self.scratch_reverse.entry(mode_config_key(mode).to_string()).or_insert_with(|| vec![String::new(); mode.key]);
        if row.len() <= lane {
            row.resize(lane + 1, String::new());
        }
        row[lane] = key_name(code).to_string();
    }

    /// The reverse-spin key token the configuration holds for a mode's scratch lane (empty when it
    /// holds none).
    pub fn scratch_reverse_token(&self, mode: Mode, lane: usize) -> &str {
        self.scratch_reverse.get(mode_config_key(mode)).and_then(|row| row.get(lane)).map_or("", String::as_str)
    }

    /// The key a control is read from, or `None` for one that has none.
    ///
    /// It is the key the configuration holds, but for a control that shipped without a key when the
    /// configuration was written and ships with one now ([`ControlAction::given_later`]). Such a
    /// control reads the key it now ships on unless the player gave it another -- and unless the
    /// configuration gives that very key to something a run reads beside it
    /// ([`KeyConfig::takes_from`]), when it stays without a key, as a row of the key editor to fill
    /// in: a key the file names is the player's choice. It is the rule a shortcut the file says
    /// nothing of follows ([`KeyConfig::shortcut_key`]).
    pub fn control_key(&self, action: ControlAction) -> Option<KeyCode> {
        let own = key_from_name(self.controls.token(action));
        let Some((token, _)) = action.given_later().filter(|(_, since)| self.generation < *since) else {
            return own;
        };
        let shipped = key_from_name(token);
        match own {
            Some(code) if Some(code) != shipped => Some(code),
            _ => shipped.filter(|code| !self.takes_from(action, *code)),
        }
    }

    /// Whether the configuration gives `code` to something a run reads beside `action`: a lane or a
    /// reverse spin of any mode -- a control is read in every mode -- another control, or a key of a
    /// replay under analysis. These are the keys the file itself names; the keys of two generations
    /// are never the same key, so one given later is never taken by another that is.
    fn takes_from(&self, action: ControlAction, code: KeyCode) -> bool {
        let lane = self.lanes.values().chain(self.scratch_reverse.values()).flatten().any(|token| key_from_name(token) == Some(code));
        lane || self.reads_beside_the_lanes(Some(action), code)
    }

    /// Whether the configuration gives `code` to something a run of any mode reads that is not a
    /// lane: a control other than `asking`, or a key of a replay under analysis.
    fn reads_beside_the_lanes(&self, asking: Option<ControlAction>, code: KeyCode) -> bool {
        let names = |token: &str| key_from_name(token) == Some(code);
        let control = ControlAction::ALL.into_iter().any(|other| Some(other) != asking && names(self.controls.token(other)));
        let replay = Shortcut::ALL.into_iter().any(|shortcut| shortcut.screen() == ShortcutScreen::Replay && self.shortcut_key(shortcut) == Some(code));
        control || replay
    }

    /// Write down the keys the controls and the reverse spins are read from and count the
    /// configuration as having taken every generation of shipped keys. Whatever is about to change
    /// a key does this first, so that what it changes is what the player sees: a control or a
    /// reverse spin the player then leaves without a key stays without one, and a lane given a key
    /// afterwards takes it from nothing behind the player's back.
    fn settle_shipped_keys(&mut self) {
        if self.generation >= KEY_GENERATION {
            return;
        }
        for action in ControlAction::ALL.into_iter().filter(|action| action.given_later().is_some_and(|(_, since)| self.generation < since)) {
            let token = self.control_key(action).map(key_name).unwrap_or_default();
            *self.controls.slot(action) = token.to_string();
        }
        for &mode in Mode::ALL {
            let given: Vec<(KeyCode, usize)> = default_reverse_keys_for_mode(mode)
                .into_iter()
                .filter_map(|(_, lane)| {
                    self.scratch_reverse_token(mode, lane).is_empty().then(|| self.reverse_key_given_later(mode, lane)).flatten().map(|code| (code, lane))
                })
                .collect();
            for (code, lane) in given {
                set_token(&mut self.scratch_reverse, mode, lane, key_name(code).to_string());
            }
        }
        self.generation = KEY_GENERATION;
    }

    /// Rebind a control action; the change is in memory until `save`.
    pub fn set_control(&mut self, action: ControlAction, code: KeyCode) {
        self.settle_shipped_keys();
        *self.controls.slot(action) = key_name(code).to_string();
    }

    /// The key a shortcut is read from: the one the player gave it, none when they took its key
    /// away, and otherwise the one it ships on. A word the file holds that names no key is treated
    /// as the shipped key, so a hand-edited file cannot leave a screen with no way into it.
    ///
    /// A shortcut the file says nothing of does not take its shipped key from a shortcut the player
    /// put there ([`KeyConfig::shipped_key_unless_given_away`]).
    pub fn shortcut_key(&self, shortcut: Shortcut) -> Option<KeyCode> {
        match self.shortcuts.get(shortcut.config_key()) {
            None => self.shipped_key_unless_given_away(shortcut),
            Some(token) if token.is_empty() => None,
            Some(token) => Some(key_from_name(token).unwrap_or_else(|| shortcut.default_key())),
        }
    }

    /// The key a shortcut ships on, or `None` when the file gives that very key to another shortcut
    /// read on the same screen.
    ///
    /// A file written before a shortcut existed has no word for it, and the player may long since
    /// have moved another shortcut onto the key the new one ships on. A key the file names is the
    /// player's choice and keeps its shortcut; the new one is left without a key, which the key
    /// editor shows as a row to fill in, rather than quietly reading the key first.
    fn shipped_key_unless_given_away(&self, shortcut: Shortcut) -> Option<KeyCode> {
        let shipped = shortcut.default_key();
        let given_to = |other: Shortcut| self.shortcuts.get(other.config_key()).and_then(|token| key_from_name(token)) == Some(shipped);
        let given_away = Shortcut::ALL.into_iter().any(|other| other != shortcut && other.screen() == shortcut.screen() && given_to(other));
        (!given_away).then_some(shipped)
    }

    /// The shortcut `screen` reads from `code`, if one is on it.
    pub fn shortcut_on(&self, screen: ShortcutScreen, code: KeyCode) -> Option<Shortcut> {
        Shortcut::ALL.into_iter().find(|shortcut| shortcut.screen() == screen && self.shortcut_key(*shortcut) == Some(code))
    }

    /// The key something ships on, or `None` for what ships with no key.
    pub fn default_key_of(mode: Mode, target: Bindable) -> Option<KeyCode> {
        match target {
            Bindable::Lane(lane) => default_keys_for_mode(mode).into_iter().find(|(_, bound)| *bound == lane).map(|(code, _)| code),
            Bindable::ScratchReverse(lane) => default_reverse_keys_for_mode(mode).into_iter().find(|(_, bound)| *bound == lane).map(|(code, _)| code),
            Bindable::Control(action) => key_from_name(ControlBinds::default().token(action)),
            Bindable::Shortcut(shortcut) => Some(shortcut.default_key()),
        }
    }

    /// Give something a key, or with `None` take its key away.
    ///
    /// A lane cannot be left without one -- [`KeyConfig::lane_keys`] fills an empty lane from the
    /// shipped layout so that no column is ever unhittable -- so taking a lane's key away puts its
    /// shipped key back. A key the file has no word for is not bound at all.
    pub fn bind(&mut self, mode: Mode, target: Bindable, code: Option<KeyCode>) {
        self.settle_shipped_keys();
        let token = match code {
            Some(code) if key_name(code) == UNNAMED_KEY => return,
            Some(code) => key_name(code).to_string(),
            None => String::new(),
        };
        match (target, code) {
            (Bindable::Lane(lane), Some(code)) => self.set_lane(mode, lane, code),
            (Bindable::ScratchReverse(lane), Some(code)) => self.set_scratch_reverse(mode, lane, code),
            (Bindable::Control(action), Some(code)) => self.set_control(action, code),
            (Bindable::Lane(lane), None) => set_token(&mut self.lanes, mode, lane, token),
            (Bindable::ScratchReverse(lane), None) => set_token(&mut self.scratch_reverse, mode, lane, token),
            (Bindable::Control(action), None) => *self.controls.slot(action) = token,
            (Bindable::Shortcut(shortcut), _) => {
                self.shortcuts.insert(shortcut.config_key().to_string(), token);
            }
        }
    }

    /// Put something back on the key it ships on.
    pub fn restore(&mut self, mode: Mode, target: Bindable) {
        match target {
            Bindable::Shortcut(shortcut) => {
                self.shortcuts.remove(shortcut.config_key());
            }
            _ => self.bind(mode, target, KeyConfig::default_key_of(mode, target)),
        }
    }

    /// Everything that can be bound for `mode`, with the key each is read from right now.
    pub fn bound_keys(&self, mode: Mode) -> Vec<(Bindable, Option<KeyCode>)> {
        let lanes = self.lane_keys(mode);
        let reverse = self.scratch_reverse_keys(mode);
        let on = |keys: &[(KeyCode, usize)], lane: usize| keys.iter().find(|(_, bound)| *bound == lane).map(|(code, _)| *code);
        Bindable::all(mode)
            .into_iter()
            .map(|target| {
                let key = match target {
                    Bindable::Lane(lane) => on(&lanes, lane),
                    Bindable::ScratchReverse(lane) => on(&reverse, lane),
                    Bindable::Control(action) => self.control_key(action),
                    Bindable::Shortcut(shortcut) => self.shortcut_key(shortcut),
                };
                (target, key)
            })
            .collect()
    }

    /// Why `code` cannot be given to `target` in `mode`, if it cannot ([`clash_among`]).
    pub fn clash(&self, mode: Mode, target: Bindable, code: KeyCode) -> Option<Clash> {
        clash_among(self.bound_keys(mode), target, code)
    }

    /// The lane of `mode` that is on the same key as a shortcut of the browser, if one is. The two
    /// are not kept apart ([`Bindable::read_with`]); the key editor says so beside the row.
    pub fn lane_sharing(&self, mode: Mode, target: Bindable) -> Option<usize> {
        let Bindable::Shortcut(shortcut) = target else {
            return None;
        };
        if shortcut.screen() != ShortcutScreen::Browser {
            return None;
        }
        let code = self.shortcut_key(shortcut)?;
        self.lane_keys(mode).into_iter().chain(self.scratch_reverse_keys(mode)).find(|(key, _)| *key == code).map(|(_, lane)| lane)
    }

    /// Put every key and every controller binding back as shipped, leaving the controller's own
    /// tuning (which device, the debounce, the turntable algorithm) as the player set it.
    pub fn restore_everything(&mut self) {
        let shipped = KeyConfig::default();
        self.lanes = shipped.lanes;
        self.scratch_reverse = shipped.scratch_reverse;
        self.controls = shipped.controls;
        self.shortcuts = shipped.shortcuts;
        self.generation = shipped.generation;
        self.pad.restore_bindings();
    }

    /// Rebind one lane of a mode; grows the mode's row to fit if needed.
    pub fn set_lane(&mut self, mode: Mode, lane: usize, code: KeyCode) {
        self.settle_shipped_keys();
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

    #[cfg(test)]
    pub fn control_token(&self, action: ControlAction) -> &str {
        self.controls.token(action)
    }

    pub fn load(path: &Path) -> KeyConfig {
        match std::fs::read_to_string(path) {
            Ok(s) => match ron::from_str::<KeyConfig>(&s) {
                Ok(mut kc) => {
                    kc.pad.sanitise();
                    kc
                }
                Err(e) => {
                    let backup = path.with_extension("ron.bak");
                    match std::fs::rename(path, &backup) {
                        Ok(_) => crate::notify::notify_text(crate::notify::Level::Warn, NoticeText::KeyConfigParseFailedBackedUp, &[&e, &backup.display()]),
                        Err(err) => crate::notify::notify_text(crate::notify::Level::Error, NoticeText::KeyConfigParseFailedNoBackup, &[&e, &err]),
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
                Err(e) => crate::notify::notify_text(crate::notify::Level::Error, NoticeText::KeyConfigWriteFailed, &[&path.display(), &e]),
            },
            Err(e) => crate::notify::notify_text(crate::notify::Level::Error, NoticeText::KeyConfigSaveFailed, &[&e]),
        }
    }
}
