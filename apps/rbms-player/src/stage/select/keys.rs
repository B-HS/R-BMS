//! The song browser's key table: which of the nine keys of a controller does what, and how a key
//! held down is told from one that has just gone down.
//!
//! This is the reference's own table (`MusicSelectKeyProperty`) and its own way of reading it
//! (`MusicSelectKeyProperty.isPressed`, `BMSPlayerInputProcessor.resetKeyChangedTime`), used when a
//! skin draws the browser, except that the keys carry no `NEXT_REPLAY` and that the option panels'
//! keys the reference gives to nothing this player has are given to options it does have
//! ([`BEAT_TABLE`]). A key is known by its index ([`crate::keyconfig::key_index_of`]): keys one to
//! seven are 0 to 6 and the two turntable directions are 7 and 8, on the keyboard and on a
//! controller alike. Each index carries one role for the list and one for each option panel, and
//! which of them a key press means is decided by whether START and SELECT are held
//! ([`Panel::held`]).
//!
//! A five-key or ten-key mode has no lane on key indices 5 and 6, so in those modes the roles of
//! those two indices are on no key, of the keyboard or of a controller.
//!
//! The reference reads the keys once a frame rather than as they arrive. A role is asked for by
//! name; the first key that carries it and is down answers, either with "it is down" or with "it
//! has changed since it was last asked about", which is what makes one press one action. The
//! second answer is taken from the key, not from the role: a key held through two of its roles in
//! turn acts once.

use rbms_model::Mode;
use rbms_render::skin_render::frame::BarHold;

use crate::AppShared;
use crate::keyconfig::KEY_INDEX_COUNT;

/// What a key can stand for on the browser (`MusicSelectKeyProperty.MusicSelectKey`). Only the
/// roles a table assigns or the browser asks about are here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectKey {
    Play,
    Auto,
    Replay,
    Up,
    Down,
    FolderOpen,
    FolderClose,
    Practice,
    Option1Up,
    Option1Down,
    GaugeUp,
    GaugeDown,
    HsFixUp,
    HsFixDown,
    Option2Down,
    TargetUp,
    TargetDown,
    LegacyNote,
    Constant,
    JudgeWindowUp,
    /// This player's own: SCRATCH AUTO, KEEP GREEN NUMBER and the lane cover, the lift and the
    /// HIDDEN+ cover, each switched on or off, on the second panel's keys the reference gives to
    /// assists this player has no setting for.
    ScratchAuto,
    KeepGreenNumber,
    LaneCover,
    Lift,
    Hidden,
    BgaDown,
    GaugeAutoShiftDown,
    DurationUp,
    DurationDown,
    NotesDisplayTimingUp,
    NotesDisplayTimingDown,
    NotesDisplayTimingAutoAdjust,
    /// This player's own: the judge algorithm, the one before and the one after, on the turntable
    /// of the third panel.
    JudgeAlgorithmUp,
    JudgeAlgorithmDown,
}

/// The roles of each key index, in the order the reference lists them.
type KeyTable = [&'static [SelectKey]; KEY_INDEX_COUNT];

/// The table of a layout with a turntable (`BEAT_7K`). The reference's two-sided table
/// (`BEAT_14K`) is this one twice, which is what folding both sides onto the same nine indices
/// comes to.
///
/// It is the reference's with these intended differences, each on a key whose role in the reference
/// does nothing that reaches a run here:
///
/// - `NEXT_REPLAY` is not on key six. The replay slot moves on only with the number key `4`, so key
///   six only stars the chart. A click on one of a skin's replay buttons plays the replay in that
///   button's slot and leaves the selected slot where it was.
/// - On the first panel key four is the gauge before (`GAUGE_UP`) in place of the double option,
///   which no run applies, and key six is the fixed tempo before (`HSFIX_UP`) in place of the second
///   side's random option going back: this player deals both sides by one random option, which keys
///   one and two already step both ways. Key seven keeps the second side's next random option.
/// - On the second panel key one keeps the custom judge window and keys two and four keep the
///   constant speed and the legacy notes. Key three is SCRATCH AUTO in place of the judge area, key
///   five KEEP GREEN NUMBER in place of the processed note mark, and keys six and seven the lane
///   cover and the lift switched on or off in place of the BPM guide and no mines, none of which
///   this player has. The turntable switches the HIDDEN+ cover on or off.
/// - On the third panel the turntable steps the judge algorithm, the one before forwards and the one
///   after backwards, as it steps the target on the first.
const BEAT_TABLE: KeyTable = [
    &[SelectKey::Play, SelectKey::FolderOpen, SelectKey::Option1Down, SelectKey::JudgeWindowUp, SelectKey::BgaDown],
    &[SelectKey::FolderClose, SelectKey::Option1Up, SelectKey::Constant, SelectKey::GaugeAutoShiftDown],
    &[SelectKey::Practice, SelectKey::FolderOpen, SelectKey::GaugeDown, SelectKey::ScratchAuto, SelectKey::NotesDisplayTimingAutoAdjust],
    &[SelectKey::FolderClose, SelectKey::GaugeUp, SelectKey::LegacyNote, SelectKey::DurationDown],
    &[SelectKey::FolderOpen, SelectKey::Auto, SelectKey::HsFixDown, SelectKey::KeepGreenNumber, SelectKey::NotesDisplayTimingDown],
    &[SelectKey::HsFixUp, SelectKey::LaneCover, SelectKey::DurationUp],
    &[SelectKey::FolderOpen, SelectKey::Replay, SelectKey::Option2Down, SelectKey::Lift, SelectKey::NotesDisplayTimingUp],
    &[SelectKey::Up, SelectKey::TargetUp, SelectKey::Hidden, SelectKey::JudgeAlgorithmUp],
    &[SelectKey::Down, SelectKey::TargetDown, SelectKey::Hidden, SelectKey::JudgeAlgorithmDown],
];

/// The table of a layout of nine buttons and no turntable (`POPN_9K`). Like [`BEAT_TABLE`] it has
/// no `NEXT_REPLAY`, which the reference puts on button eight, and its panel roles are
/// [`BEAT_TABLE`]'s on the same indices, buttons eight and nine standing for the turntable.
const POPN_TABLE: KeyTable = [
    &[SelectKey::Auto, SelectKey::Option1Down, SelectKey::JudgeWindowUp, SelectKey::BgaDown],
    &[SelectKey::Option1Up, SelectKey::Constant, SelectKey::GaugeAutoShiftDown],
    &[SelectKey::FolderClose, SelectKey::GaugeDown, SelectKey::ScratchAuto, SelectKey::NotesDisplayTimingAutoAdjust],
    &[SelectKey::Down, SelectKey::GaugeUp, SelectKey::LegacyNote, SelectKey::DurationDown],
    &[SelectKey::Play, SelectKey::FolderOpen, SelectKey::HsFixDown, SelectKey::KeepGreenNumber, SelectKey::NotesDisplayTimingDown],
    &[SelectKey::Up, SelectKey::HsFixUp, SelectKey::LaneCover, SelectKey::DurationUp],
    &[SelectKey::Practice, SelectKey::FolderOpen, SelectKey::Option2Down, SelectKey::Lift, SelectKey::NotesDisplayTimingUp],
    &[SelectKey::TargetUp, SelectKey::Hidden, SelectKey::JudgeAlgorithmUp],
    &[SelectKey::Replay, SelectKey::TargetDown, SelectKey::Hidden, SelectKey::JudgeAlgorithmDown],
];

/// The roles the first panel takes a press of, in the order the reference asks about them
/// (`MusicSelectInputProcessor.input`, the START branch), without the double option's two, which
/// no key carries here.
const PLAY_PANEL_PRESSES: [SelectKey; 7] =
    [SelectKey::Option1Down, SelectKey::Option1Up, SelectKey::GaugeDown, SelectKey::GaugeUp, SelectKey::Option2Down, SelectKey::HsFixDown, SelectKey::HsFixUp];

/// The roles the second panel takes a press of (the SELECT branch), the turntable's last.
const ASSIST_PANEL_PRESSES: [SelectKey; 8] = [
    SelectKey::JudgeWindowUp,
    SelectKey::Constant,
    SelectKey::ScratchAuto,
    SelectKey::LegacyNote,
    SelectKey::KeepGreenNumber,
    SelectKey::LaneCover,
    SelectKey::Lift,
    SelectKey::Hidden,
];

/// The roles the third panel takes a press of (the START and SELECT branch), the turntable's last.
/// The two that change the note display time are read as held rather than as pressed, so they are
/// not here: a key held on one of them is still a fresh press to the list when the panel closes, as
/// it is in the reference.
const DETAIL_PANEL_PRESSES: [SelectKey; 7] = [
    SelectKey::BgaDown,
    SelectKey::GaugeAutoShiftDown,
    SelectKey::NotesDisplayTimingDown,
    SelectKey::NotesDisplayTimingUp,
    SelectKey::NotesDisplayTimingAutoAdjust,
    SelectKey::JudgeAlgorithmUp,
    SelectKey::JudgeAlgorithmDown,
];

/// Which of the reference's key tables is in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyLayout {
    /// Seven keys and a turntable, on one side or two.
    Beat,
    /// Nine buttons.
    Popn,
}

impl KeyLayout {
    /// The table the keys of `mode` are read by. The reference has a setting for this
    /// (`musicselectinput`); here it follows the mode whose keys are bound, which is the only one
    /// whose indices mean anything.
    pub(super) fn of(mode: Mode) -> KeyLayout {
        if mode.scratch.is_empty() { KeyLayout::Popn } else { KeyLayout::Beat }
    }

    fn table(self) -> &'static KeyTable {
        match self {
            KeyLayout::Beat => &BEAT_TABLE,
            KeyLayout::Popn => &POPN_TABLE,
        }
    }

    /// Whether the key `index` carries `role`.
    pub(super) fn carries(self, index: usize, role: SelectKey) -> bool {
        self.table().get(index).is_some_and(|roles| roles.contains(&role))
    }

    /// The key indices that carry `role`, in table order, which is what the key guide names.
    pub(super) fn indices_of(self, role: SelectKey) -> Vec<usize> {
        self.table().iter().enumerate().filter(|(_, roles)| roles.contains(&role)).map(|(index, _)| index).collect()
    }
}

/// Which option panel the held keys call up (`MusicSelector.panelstate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Panel {
    /// None: the keys belong to the list.
    Closed,
    /// The play options, while START is held alone.
    Play,
    /// The assist options, while SELECT is held alone.
    Assist,
    /// The detail options, while both are held or the key for it is.
    Detail,
}

impl Panel {
    /// The panel the held keys call up, in the order the reference tests them: START alone, SELECT
    /// alone, then the detail key or both.
    pub(super) fn held(start: bool, select: bool, detail: bool) -> Panel {
        match (start, select) {
            (true, false) => Panel::Play,
            (false, true) => Panel::Assist,
            (true, true) => Panel::Detail,
            (false, false) if detail => Panel::Detail,
            (false, false) => Panel::Closed,
        }
    }

    /// The roles this panel takes a press of.
    fn presses(self) -> &'static [SelectKey] {
        match self {
            Panel::Closed => &[],
            Panel::Play => &PLAY_PANEL_PRESSES,
            Panel::Assist => &ASSIST_PANEL_PRESSES,
            Panel::Detail => &DETAIL_PANEL_PRESSES,
        }
    }
}

/// The nine keys as the browser last read them: which are down, and which have changed since they
/// were last asked about (`BMSPlayerInputProcessor.keystate` and `keytime`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct SelectKeys {
    down: [bool; KEY_INDEX_COUNT],
    changed: [bool; KEY_INDEX_COUNT],
    /// The keys that went down since the last reading, whether or not they are still down: a tap
    /// shorter than a frame is still a press.
    tapped: [bool; KEY_INDEX_COUNT],
}

impl SelectKeys {
    /// The keys as they stand, with none of them counting as changed: what is held when the
    /// browser starts listening is not a press.
    pub(super) fn settled(shared: &AppShared) -> SelectKeys {
        let mut keys = SelectKeys::default();
        for (index, down) in keys.down.iter_mut().enumerate() {
            *down = shared.key_index_pressed(index);
        }
        keys
    }

    /// Notes that the key `index` went down, for the reading that follows.
    pub(super) fn note_press(&mut self, index: usize) {
        if let Some(tapped) = self.tapped.get_mut(index) {
            *tapped = true;
        }
    }

    /// Reads the keys for one frame. A key whose state differs from the last reading has changed,
    /// and so has one that went down in between, which also counts as down for this frame.
    pub(super) fn read(&mut self, shared: &AppShared) {
        for index in 0..KEY_INDEX_COUNT {
            let tapped = std::mem::take(&mut self.tapped[index]);
            let down = tapped || shared.key_index_pressed(index);
            if tapped || down != self.down[index] {
                self.changed[index] = true;
            }
            self.down[index] = down;
        }
    }

    /// Whether a key carrying `role` is down, and with `fresh`, whether it has changed since it
    /// was last asked about -- which is then forgotten (`MusicSelectKeyProperty.isPressed`).
    ///
    /// The first key that carries the role and is down gives the answer, so a second key of the
    /// same role pressed while the first is held is not a press.
    pub(super) fn pressed(&mut self, layout: KeyLayout, role: SelectKey, fresh: bool) -> bool {
        let held = layout.table().iter().enumerate().find(|(index, roles)| roles.contains(&role) && self.down[*index]);
        match held {
            Some((index, _)) if fresh => self.forget_change(index),
            Some(_) => true,
            None => false,
        }
    }

    /// Whether the key `index` had changed, which is forgotten by asking
    /// (`BMSPlayerInputProcessor.resetKeyChangedTime`).
    pub(super) fn forget_change(&mut self, index: usize) -> bool {
        self.changed.get_mut(index).is_some_and(std::mem::take)
    }

    /// Which way the keys that move the list are holding it. The key for the next bar wins over
    /// the key for the one before, as it does in the reference (`BarRenderer.input`).
    pub(super) fn hold(&mut self, layout: KeyLayout) -> BarHold {
        if self.pressed(layout, SelectKey::Up, false) {
            BarHold::Next
        } else if self.pressed(layout, SelectKey::Down, false) {
            BarHold::Previous
        } else {
            BarHold::None
        }
    }

    /// Takes the presses an open panel takes: each key down on one of the panel's roles stops
    /// counting as changed, so it is not a press to the list when the panel closes. Answers the
    /// roles that were pressed, in the order the reference asks about them.
    pub(super) fn panel_presses(&mut self, layout: KeyLayout, panel: Panel) -> Vec<SelectKey> {
        panel.presses().iter().copied().filter(|role| self.pressed(layout, *role, true)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The index of key one, which is `PLAY` on a chart and `FOLDER_OPEN` on a folder, and of key
    /// three, which is `FOLDER_OPEN` too.
    const KEY_ONE: usize = 0;
    const KEY_THREE: usize = 2;
    const KEY_FOUR: usize = 3;
    const TURNTABLE_FORWARD: usize = 7;
    const TURNTABLE_BACKWARD: usize = 8;

    /// Keys with `indices` down, each having just gone down.
    fn keys_down(indices: &[usize]) -> SelectKeys {
        let mut keys = SelectKeys::default();
        for index in indices {
            keys.down[*index] = true;
            keys.changed[*index] = true;
        }
        keys
    }

    /// The reference's seven-key table (`MusicSelectKeyProperty.BEAT_7K`) index by index, without
    /// `NEXT_REPLAY`: each role by the reference's own name.
    const REFERENCE_BEAT: [&[&str]; KEY_INDEX_COUNT] = [
        &["PLAY", "FOLDER_OPEN", "OPTION1_DOWN", "JUDGEWINDOW_UP", "BGA_DOWN"],
        &["FOLDER_CLOSE", "OPTION1_UP", "CONSTANT", "GAUGEAUTOSHIFT_DOWN"],
        &["PRACTICE", "FOLDER_OPEN", "GAUGE_DOWN", "JUDGEAREA", "NOTESDISPLAYTIMING_AUTOADJUST"],
        &["FOLDER_CLOSE", "OPTIONDP_DOWN", "LEGACYNOTE", "DURATION_DOWN"],
        &["FOLDER_OPEN", "AUTO", "HSFIX_DOWN", "MARKNOTE", "NOTESDISPLAYTIMING_DOWN"],
        &["OPTION2_UP", "BPMGUIDE", "DURATION_UP"],
        &["FOLDER_OPEN", "REPLAY", "OPTION2_DOWN", "NOMINE", "NOTESDISPLAYTIMING_UP"],
        &["UP", "TARGET_UP"],
        &["DOWN", "TARGET_DOWN"],
    ];

    /// The role this player keeps under each of the reference's names, for the names it keeps.
    const KEPT_ROLES: [(&str, SelectKey); 25] = [
        ("PLAY", SelectKey::Play),
        ("AUTO", SelectKey::Auto),
        ("REPLAY", SelectKey::Replay),
        ("UP", SelectKey::Up),
        ("DOWN", SelectKey::Down),
        ("FOLDER_OPEN", SelectKey::FolderOpen),
        ("FOLDER_CLOSE", SelectKey::FolderClose),
        ("PRACTICE", SelectKey::Practice),
        ("OPTION1_UP", SelectKey::Option1Up),
        ("OPTION1_DOWN", SelectKey::Option1Down),
        ("GAUGE_DOWN", SelectKey::GaugeDown),
        ("HSFIX_DOWN", SelectKey::HsFixDown),
        ("OPTION2_DOWN", SelectKey::Option2Down),
        ("TARGET_UP", SelectKey::TargetUp),
        ("TARGET_DOWN", SelectKey::TargetDown),
        ("LEGACYNOTE", SelectKey::LegacyNote),
        ("CONSTANT", SelectKey::Constant),
        ("JUDGEWINDOW_UP", SelectKey::JudgeWindowUp),
        ("BGA_DOWN", SelectKey::BgaDown),
        ("GAUGEAUTOSHIFT_DOWN", SelectKey::GaugeAutoShiftDown),
        ("DURATION_UP", SelectKey::DurationUp),
        ("DURATION_DOWN", SelectKey::DurationDown),
        ("NOTESDISPLAYTIMING_DOWN", SelectKey::NotesDisplayTimingDown),
        ("NOTESDISPLAYTIMING_UP", SelectKey::NotesDisplayTimingUp),
        ("NOTESDISPLAYTIMING_AUTOADJUST", SelectKey::NotesDisplayTimingAutoAdjust),
    ];

    /// Every place the seven-key table differs from the reference's: the key index, the reference's
    /// role there (empty for a role this player adds past the reference's list) and this player's.
    const CHANGED_SLOTS: [(usize, &str, SelectKey); 10] = [
        (2, "JUDGEAREA", SelectKey::ScratchAuto),
        (3, "OPTIONDP_DOWN", SelectKey::GaugeUp),
        (4, "MARKNOTE", SelectKey::KeepGreenNumber),
        (5, "OPTION2_UP", SelectKey::HsFixUp),
        (5, "BPMGUIDE", SelectKey::LaneCover),
        (6, "NOMINE", SelectKey::Lift),
        (7, "", SelectKey::Hidden),
        (7, "", SelectKey::JudgeAlgorithmUp),
        (8, "", SelectKey::Hidden),
        (8, "", SelectKey::JudgeAlgorithmDown),
    ];

    /// The seven-key table is the reference's with the changes [`CHANGED_SLOTS`] lists and nothing
    /// else: a role of the reference this player keeps is on its key at its place, a role it gives
    /// up is replaced where it stood, and what is added comes after the reference's list.
    #[test]
    fn the_seven_key_table_is_the_references_with_only_the_listed_slots_changed() {
        let kept = |name: &str| KEPT_ROLES.iter().find(|(kept, _)| *kept == name).map(|(_, role)| *role);
        for (index, reference) in REFERENCE_BEAT.iter().enumerate() {
            let mut changes = CHANGED_SLOTS.iter().filter(|(at, ..)| *at == index);
            let mut expected: Vec<SelectKey> = Vec::new();
            for name in *reference {
                match kept(name) {
                    Some(role) => expected.push(role),
                    None => {
                        let (_, replaced, role) = changes.next().unwrap_or_else(|| panic!("{name} on key index {index} is gone and no change says so"));
                        assert_eq!(replaced, name, "the change on key index {index} names the wrong role");
                        expected.push(*role);
                    }
                }
            }
            for (_, replaced, role) in changes {
                assert!(replaced.is_empty(), "{replaced} on key index {index} is listed as changed and the reference's role is still there");
                expected.push(*role);
            }
            assert_eq!(KeyLayout::Beat.table()[index], expected.as_slice(), "key index {index}");
        }
    }

    /// The panel roles of the nine-button table are the seven-key table's on the same indices, and
    /// its list roles are the reference's (`POPN_9K`) without `NEXT_REPLAY`.
    #[test]
    fn the_nine_button_table_has_the_seven_key_panels_and_the_references_list_roles() {
        use SelectKey::*;
        let list_roles = [Play, Auto, Replay, Up, Down, FolderOpen, FolderClose, Practice];
        let popn_list_roles: [&[SelectKey]; KEY_INDEX_COUNT] =
            [&[Auto], &[], &[FolderClose], &[Down], &[Play, FolderOpen], &[Up], &[Practice, FolderOpen], &[], &[Replay]];
        let panel_roles = |table: &KeyTable, index: usize| table[index].iter().copied().filter(|role| !list_roles.contains(role)).collect::<Vec<_>>();
        for (index, expected) in popn_list_roles.iter().enumerate() {
            let listed: Vec<SelectKey> = KeyLayout::Popn.table()[index].iter().copied().filter(|role| list_roles.contains(role)).collect();
            assert_eq!(listed.as_slice(), *expected, "the list roles of button {index}");
            assert_eq!(panel_roles(KeyLayout::Popn.table(), index), panel_roles(KeyLayout::Beat.table(), index), "the panel roles of button {index}");
        }
    }

    /// Every role a panel takes a press of is on a key of both tables, and every role on a key is
    /// one the list or a panel reads.
    #[test]
    fn every_panel_role_is_on_a_key_and_every_role_on_a_key_is_read() {
        use SelectKey::*;
        let read_as_held = [TargetUp, TargetDown, DurationUp, DurationDown];
        let list_roles = [Play, Auto, Replay, Up, Down, FolderOpen, FolderClose, Practice];
        let pressed: Vec<SelectKey> = [Panel::Play, Panel::Assist, Panel::Detail].into_iter().flat_map(|panel| panel.presses().iter().copied()).collect();
        for layout in [KeyLayout::Beat, KeyLayout::Popn] {
            for role in &pressed {
                assert!(!layout.indices_of(*role).is_empty(), "{role:?} is on no key of {layout:?}");
            }
            for role in layout.table().iter().flat_map(|roles| roles.iter()) {
                assert!(pressed.contains(role) || read_as_held.contains(role) || list_roles.contains(role), "{role:?} on {layout:?} is read by nothing");
            }
        }
    }

    #[test]
    fn a_layout_without_a_turntable_reads_the_nine_button_table() {
        assert_eq!(KeyLayout::of(Mode::POPN_9K), KeyLayout::Popn);
        for mode in [Mode::BEAT_5K, Mode::BEAT_7K, Mode::BEAT_10K, Mode::BEAT_14K, Mode::KEYBOARD_24K] {
            assert_eq!(KeyLayout::of(mode), KeyLayout::Beat, "{}", mode.name);
        }
    }

    /// START alone, SELECT alone, both, and the detail key with neither.
    #[test]
    fn the_held_keys_call_up_the_panel_the_reference_would() {
        assert_eq!(Panel::held(false, false, false), Panel::Closed);
        assert_eq!(Panel::held(true, false, false), Panel::Play);
        assert_eq!(Panel::held(true, false, true), Panel::Play, "START alone is tested before the detail key");
        assert_eq!(Panel::held(false, true, true), Panel::Assist);
        assert_eq!(Panel::held(true, true, false), Panel::Detail);
        assert_eq!(Panel::held(false, false, true), Panel::Detail);
    }

    #[test]
    fn a_held_key_is_one_press() {
        let mut keys = keys_down(&[KEY_ONE]);
        assert!(keys.pressed(KeyLayout::Beat, SelectKey::Play, true));
        assert!(!keys.pressed(KeyLayout::Beat, SelectKey::Play, true), "the key is still down and was asked about already");
        assert!(keys.pressed(KeyLayout::Beat, SelectKey::Play, false), "it is still down");
        assert!(!keys.pressed(KeyLayout::Beat, SelectKey::FolderOpen, true), "its other role is the same key, which has not changed again");
        assert!(!keys.pressed(KeyLayout::Beat, SelectKey::Practice, true), "no key of that role is down");
    }

    /// The first key of a role that is down answers for the role, so a second one pressed while it
    /// is held is nothing -- and stays unasked, so it is a press once the first is let go.
    #[test]
    fn the_first_key_down_answers_for_its_role() {
        let mut keys = keys_down(&[KEY_ONE]);
        assert!(keys.pressed(KeyLayout::Beat, SelectKey::FolderOpen, true));
        keys.down[KEY_THREE] = true;
        keys.changed[KEY_THREE] = true;
        assert!(!keys.pressed(KeyLayout::Beat, SelectKey::FolderOpen, true), "key one is down and unchanged, and it is asked first");

        keys.down[KEY_ONE] = false;
        assert!(keys.pressed(KeyLayout::Beat, SelectKey::FolderOpen, true), "key three had never been asked about");
    }

    #[test]
    fn the_key_for_the_next_bar_wins_over_the_key_for_the_one_before() {
        assert_eq!(keys_down(&[]).hold(KeyLayout::Beat), BarHold::None);
        assert_eq!(keys_down(&[TURNTABLE_FORWARD]).hold(KeyLayout::Beat), BarHold::Next);
        assert_eq!(keys_down(&[TURNTABLE_BACKWARD]).hold(KeyLayout::Beat), BarHold::Previous);
        assert_eq!(keys_down(&[TURNTABLE_FORWARD, TURNTABLE_BACKWARD]).hold(KeyLayout::Beat), BarHold::Next);

        let mut held = keys_down(&[TURNTABLE_FORWARD]);
        held.hold(KeyLayout::Beat);
        assert!(held.changed[TURNTABLE_FORWARD], "holding the list is read off the key's state and asks nothing of its change");
    }

    /// A panel takes the presses of the keys its roles are on, so a key pressed for the panel is
    /// not a press to the list afterwards -- except the two keys the detail panel reads as held.
    #[test]
    fn an_open_panel_takes_the_presses_of_its_own_keys() {
        let mut keys = keys_down(&[KEY_ONE, KEY_FOUR]);
        assert_eq!(keys.panel_presses(KeyLayout::Beat, Panel::Play), [SelectKey::Option1Down, SelectKey::GaugeUp]);
        assert!(!keys.pressed(KeyLayout::Beat, SelectKey::Play, true), "the panel took key one's press");
        assert!(!keys.pressed(KeyLayout::Beat, SelectKey::FolderClose, true), "and key four's");

        let mut keys = keys_down(&[KEY_ONE, KEY_FOUR]);
        assert_eq!(keys.panel_presses(KeyLayout::Beat, Panel::Assist), [SelectKey::JudgeWindowUp, SelectKey::LegacyNote]);

        let mut keys = keys_down(&[KEY_ONE, KEY_FOUR]);
        assert_eq!(keys.panel_presses(KeyLayout::Beat, Panel::Detail), [SelectKey::BgaDown]);
        assert!(keys.pressed(KeyLayout::Beat, SelectKey::FolderClose, true), "the detail panel reads key four as held and leaves its press");

        let mut keys = keys_down(&[KEY_ONE]);
        assert!(keys.panel_presses(KeyLayout::Beat, Panel::Closed).is_empty());
        assert!(keys.pressed(KeyLayout::Beat, SelectKey::Play, true));
    }

    #[test]
    fn a_tap_between_two_readings_is_a_press_for_the_reading_that_follows() {
        let app = crate::stage::select::tests::app();
        let mut keys = SelectKeys::settled(&app.shared);
        keys.read(&app.shared);
        assert!(!keys.pressed(KeyLayout::Beat, SelectKey::Play, true), "nothing is down");

        keys.note_press(KEY_ONE);
        keys.read(&app.shared);
        assert!(keys.pressed(KeyLayout::Beat, SelectKey::Play, true), "the tap was lost");
        keys.read(&app.shared);
        assert!(!keys.pressed(KeyLayout::Beat, SelectKey::Play, false), "a tap is down for one reading");
        keys.note_press(KEY_INDEX_COUNT);
    }
}
