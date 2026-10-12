use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use rbms_model::Mode;
use rbms_play::ScratchDir;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

use crate::gamepad::PadConfig;

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
        match self {
            ControlAction::HiSpeedUp => "HI-SPEED UP",
            ControlAction::HiSpeedDown => "HI-SPEED DOWN",
            ControlAction::CoverUp => "LANE COVER DOWN (COVER MORE)",
            ControlAction::CoverDown => "LANE COVER UP (COVER LESS)",
            ControlAction::CoverToggle => "LANE COVER ON/OFF",
            ControlAction::LiftUp => "LIFT UP (RAISE THE LINE)",
            ControlAction::LiftDown => "LIFT DOWN (LOWER THE LINE)",
            ControlAction::LiftToggle => "LIFT ON/OFF",
            ControlAction::HiddenUp => "HIDDEN+ UP (COVER MORE)",
            ControlAction::HiddenDown => "HIDDEN+ DOWN (COVER LESS)",
            ControlAction::HiddenToggle => "HIDDEN+ ON/OFF",
            ControlAction::GreenUp => "GREEN NUMBER UP (SLOWER)",
            ControlAction::GreenDown => "GREEN NUMBER DOWN (FASTER)",
            ControlAction::KeepGreenToggle => "KEEP GREEN NUMBER ON/OFF",
            ControlAction::OffsetUp => "JUDGE OFFSET +1 MS (HITTING LATE)",
            ControlAction::OffsetDown => "JUDGE OFFSET -1 MS (HITTING EARLY)",
            ControlAction::AutoAdjustToggle => "AUTO ADJUST ON/OFF",
            ControlAction::Fine => "FINE STEP WHILE HELD (UNSET: SHIFT)",
            ControlAction::Start => "START",
            ControlAction::Select => "SELECT",
        }
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
    /// The controls below ship unbound: each is a key a player asks for, not one a run needs.
    pub lift_toggle: String,
    pub hidden_toggle: String,
    pub green_up: String,
    pub green_down: String,
    pub keep_green_toggle: String,
    pub offset_up: String,
    pub offset_down: String,
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
            offset_up: String::new(),
            offset_down: String::new(),
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
        match self {
            ShortcutGroup::Browser => "SELECT",
            ShortcutGroup::Screens => "SCREENS (OPENED FROM SELECT)",
            ShortcutGroup::SkinBrowser => "SELECT - NUMBER KEYS OF A SKIN'S BROWSER",
            ShortcutGroup::Result => "RESULT",
            ShortcutGroup::Replay => "REPLAY ANALYSIS",
        }
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
}

impl Shortcut {
    /// Every shortcut, group by group, in the order the key editor and the key guide list them.
    pub const ALL: [Shortcut; 31] = [
        Shortcut::Search,
        Shortcut::Filter,
        Shortcut::Sort,
        Shortcut::Practice,
        Shortcut::Favourite,
        Shortcut::Records,
        Shortcut::Ranking,
        Shortcut::ReloadTables,
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
            Shortcut::ReplayPause | Shortcut::ReplayFaster | Shortcut::ReplaySlower | Shortcut::ReplayForward | Shortcut::ReplayBack => ShortcutGroup::Replay,
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
        match self {
            Shortcut::Search => "SEARCH",
            Shortcut::Filter => "FILTER PANEL",
            Shortcut::Sort => "SORT ORDER (SHIFT: BACKWARDS)",
            Shortcut::Practice => "PRACTICE",
            Shortcut::Favourite => "FAVOURITE",
            Shortcut::Records => "RECORDS OF THE CHART",
            Shortcut::Ranking => "IR RANKING",
            Shortcut::ReloadTables => "RELOAD DIFFICULTY TABLES",
            Shortcut::OptionsPin => "OPTIONS OVERLAY (PIN)",
            Shortcut::OptionsHold => "OPTIONS OVERLAY (HOLD)",
            Shortcut::Settings => "SETTINGS (SHIFT: COURSE LIST)",
            Shortcut::SkinMenu => "SKIN MENU",
            Shortcut::PlayMenu => "PLAY MENU",
            Shortcut::KeyConfig => "KEY CONFIG",
            Shortcut::Folders => "MUSIC FOLDERS",
            Shortcut::Tables => "DIFFICULTY TABLES",
            Shortcut::RefMode => "MODE FILTER",
            Shortcut::RefSort => "SORT ORDER",
            Shortcut::RefLnMode => "LONG NOTE MODE",
            Shortcut::RefReplay => "REPLAY SLOT",
            Shortcut::RefDetail => "DETAIL OPTIONS (HOLD)",
            Shortcut::RefKeyConfig => "KEY CONFIG",
            Shortcut::RefSearch => "SEARCH",
            Shortcut::RefFavourite => "FAVOURITE",
            Shortcut::Retry => "PLAY THE CHART AGAIN",
            Shortcut::NextSong => "NEXT SONG",
            Shortcut::ReplayPause => "PAUSE / RESUME",
            Shortcut::ReplayFaster => "PLAYBACK RATE UP",
            Shortcut::ReplaySlower => "PLAYBACK RATE DOWN",
            Shortcut::ReplayForward => "SEEK FORWARD",
            Shortcut::ReplayBack => "SEEK BACK",
        }
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
        }
    }

    /// The key the shortcut ships on. Every one of them but the keys to the key editor and to the
    /// play menu, and the key that reloads the tables, is the key the application read for it before
    /// any of them could be changed. The reload ships on a function key the browser read nothing
    /// from, next to the one the key editor is on.
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
            Bindable::Lane(lane) if mode.is_scratch(lane) => format!("SCRATCH {}", lane + 1),
            Bindable::Lane(lane) => format!("LANE {}", lane + 1),
            Bindable::ScratchReverse(lane) => format!("SCRATCH {} REVERSE", lane + 1),
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
/// written before reverse spins existed keeps parsing byte for byte. It ships empty: a scratch lane
/// with no reverse key behaves exactly as it did with one key.
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
/// `held` is not configuration. It sits beside the bindings because a held key only means
/// something read against them, and it is skipped by the file.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyConfig {
    pub lanes: BTreeMap<String, Vec<String>>,
    pub scratch_reverse: BTreeMap<String, Vec<String>>,
    pub controls: ControlBinds,
    pub shortcuts: BTreeMap<String, String>,
    pub pad: PadConfig,
    #[serde(skip)]
    pub held: HeldKeys,
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
        KeyConfig {
            lanes,
            scratch_reverse: BTreeMap::new(),
            controls: ControlBinds::default(),
            shortcuts: BTreeMap::new(),
            pad: PadConfig::default(),
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
                    None => crate::notify::notify(
                        crate::notify::Level::Warn,
                        format!("keyconfig: mode {} lane {} has unknown key {tok:?}; using default", mode.name, lane),
                    ),
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
                    crate::notify::notify(
                        crate::notify::Level::Warn,
                        format!("keyconfig: mode {} scratch lane {lane} has unknown reverse key {token:?}; leaving it unbound", mode.name),
                    );
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
    #[cfg(test)]
    pub fn scratch_reverse_token(&self, mode: Mode, lane: usize) -> String {
        self.scratch_reverse.get(mode_config_key(mode)).and_then(|row| row.get(lane)).cloned().unwrap_or_default()
    }

    pub fn control_key(&self, action: ControlAction) -> Option<KeyCode> {
        key_from_name(self.controls.token(action))
    }

    /// Rebind a control action; the change is in memory until `save`.
    pub fn set_control(&mut self, action: ControlAction, code: KeyCode) {
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
            Bindable::ScratchReverse(_) => None,
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
        self.pad.restore_bindings();
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
                        Ok(_) => crate::notify::notify(
                            crate::notify::Level::Warn,
                            format!("keyconfig parse failed ({e}); backed up to {} and using defaults", backup.display()),
                        ),
                        Err(err) => {
                            crate::notify::notify(crate::notify::Level::Error, format!("keyconfig parse failed ({e}); backup failed ({err}); using defaults"))
                        }
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
                Err(e) => crate::notify::notify(crate::notify::Level::Error, format!("keyconfig write failed ({}): {e}", path.display())),
            },
            Err(e) => crate::notify::notify(crate::notify::Level::Error, format!("keyconfig save failed: {e}")),
        }
    }
}
