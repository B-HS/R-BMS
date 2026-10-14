//! What the settings screen says of the row under the cursor: the panel beside the list, and the
//! line of keys along the foot of the screen.
//!
//! Both are worked out from what the row is and holds at this moment. The keys named are the keys
//! that row hears, with what each does to it -- a switch is not said to be typed into, and a row
//! that opens a list is not said to be stepped -- and the keys of the screen itself follow them on
//! the line at the foot. The panel adds what the row holds beside what it ships as, the ends and the
//! step of a number, the values of a choice, and what a note beside the row means.

use rbms_config::{
    ACTION_VALUE, BUILT_IN_ONLY_NOTE, JUDGE_RATE_DEFAULT_PERCENT, LN_MARGIN_DEFAULT_PERCENT, SKIN_OPTION_NOTE, SettingId, SettingKind, cycle_values, descriptor,
};

use super::catalog::description;
use crate::ir_panel::is_secret_row;
use crate::number_row::{NumberValue, ONE_STEP, SHIFT_STEPS};
use crate::settings_ui::{is_network_row, is_resettable, shipped_value};
use crate::settings_view::{InfoScene, RESET_GROUP_KEY, RowControl, SettingsLine};

/// What stands between a key and the next on the line of keys.
const KEY_SEPARATOR: &str = "   ";

/// What a key that leaves the screen is said to do: the settings are written out on the way.
const SAVE_AND_CLOSE: &str = "SAVE AND CLOSE";

/// What the keys that put a row back are said to do.
const TO_DEFAULT: &str = "DEFAULT";

/// The keys every row of the screen hears whatever it holds, each with what it does, before the
/// three that open the other screens.
const SCREEN_KEYS: [(&str, &str); 4] = [("UP DOWN", "MOVE"), ("PGUP PGDN", "SECTION"), ("TAB", "GROUP"), (RESET_GROUP_KEY, "RESET GROUP")];

/// What the three keys that open the other screens are said to open, in the order the screen names
/// them: the play menu, the skin menu and the key editor.
const LINK_ACTIONS: [&str; 3] = ["PLAY", "SKINS", "KEYS"];

/// The key that leaves the screen, and what it does.
const LEAVE_KEY: (&str, &str) = ("ESC", "SAVE");

/// The keys of the screen while a line of text is being typed into a row.
pub(super) const TEXT_EDIT_HINT: &str = "TYPE   ENTER SET   ESC CANCEL";

/// The keys a number being typed into hears, as the panel lists them while it is typed.
pub(super) const NUMBER_EDIT_KEYS: [(&str, &str); 4] = [("DIGITS", "TYPE A NUMBER"), ("ENTER", "SET"), ("ESC", "CANCEL"), ("UP DOWN TAB", "SET AND MOVE ON")];

/// The keys a line of text being typed hears, as the panel lists them.
pub(super) const TEXT_EDIT_KEYS: [(&str, &str); 3] = [("KEYS", "TYPE"), ("ENTER", "SET"), ("ESC", "CANCEL")];

/// The keys a list drawn over the rows hears, as the panel lists them.
pub(super) const LIST_KEYS: [(&str, &str); 4] = [("UP DOWN", "MOVE"), ("ENTER", "ADD"), ("D", "REMOVE"), ("ESC", "SAVE AND CLOSE")];

/// The keys the question about a group hears, as the panel lists them.
pub(super) const CONFIRM_KEYS: [(&str, &str); 2] = [("ENTER", "RESET THE GROUP"), ("ESC", "KEEP IT AS IT IS")];

/// What the panel calls each kind of row.
const KIND_SWITCH: &str = "SWITCH";
const KIND_CHOICE: &str = "CHOICE";
const KIND_NUMBER: &str = "NUMBER";
const KIND_TEXT: &str = "TEXT";
const KIND_SECRET: &str = "TEXT, NEVER SHOWN";
const KIND_FILE: &str = "FILE";
const KIND_FOLDER: &str = "FOLDER";
const KIND_ACTION: &str = "ACTION";
const KIND_STATUS: &str = "STATUS";

/// The names the panel writes what a row holds under.
const FACT_VALUE: &str = "VALUE";
const FACT_DEFAULT: &str = "DEFAULT";
const FACT_RANGE: &str = "RANGE";
const FACT_STEP: &str = "STEP";
const FACT_CHOICES: &str = "CHOICES";

/// What is written after what a row ships as while the row is something else.
const RESTORE_MARK: &str = "(0 PUTS IT BACK)";

/// What stands between two values of a choice as the panel lists them.
const CHOICE_SEPARATOR: &str = ", ";

/// What a note beside a row means, said on the panel in full.
const BUILT_IN_ONLY_REMARK: &str = "A skin is set to draw this screen and does not read this row. It still holds wherever the built-in screen is drawn.";
const SKIN_OPTION_REMARK: &str =
    "A skin is set to draw this screen, and on it the skin's own option decides. This row still holds wherever the built-in screen is drawn.";

/// What is said of a judge window or a long-note margin while it stands over what the chart asks
/// for: a run judged that way is a custom judge.
const CUSTOM_JUDGE_CAUTION: &str = "OVER 100%: A RUN PLAYED LIKE THIS SETS NO BEST SCORE AND IS NOT SENT. ITS LAMP AND HISTORY ARE KEPT.";

/// The keys one row hears, each with what it does to that row. `control` is what sets the row.
///
/// Enter is the one key that differs most from row to row: it opens a number to be typed into,
/// opens a line of text, a list or the key editor, runs an action of the score server and turns a
/// switch of that group over, and on every other row it leaves the screen as Escape does.
pub(super) fn row_keys(id: SettingId, control: RowControl) -> Vec<(&'static str, &'static str)> {
    let leaves = ("ENTER", SAVE_AND_CLOSE);
    let resets = ("0", TO_DEFAULT);
    match (id, control) {
        (_, RowControl::Slider) => vec![("LEFT RIGHT", "1 STEP"), ("SHIFT", "x10"), ("ENTER", "TYPE"), resets, ("DRAG", "SLIDER")],
        (SettingId::KeyConfig, _) => vec![("ENTER", "OPEN")],
        (SettingId::Font, _) => vec![("ENTER", "CHOOSE A FILE"), resets],
        (SettingId::SoundFolder, _) => vec![("RIGHT", "CHOOSE A FOLDER"), ("0", "NONE"), leaves],
        (SettingId::Account, _) => vec![("ENTER", "SHOW ITS STATE")],
        (SettingId::Rivals | SettingId::IrProfiles, _) => vec![("ENTER", "OPEN THE LIST")],
        (_, RowControl::Field { .. }) => vec![("ENTER", "EDIT")],
        (_, RowControl::Action | RowControl::Path { .. }) => vec![("ENTER", "RUN")],
        (_, RowControl::Switch { .. }) if is_network_row(id) => vec![("LEFT RIGHT", "SWITCH"), ("ENTER", "SWITCH"), resets],
        (_, RowControl::Switch { .. }) => vec![("LEFT RIGHT", "SWITCH"), resets, leaves],
        (_, RowControl::Segments { .. } | RowControl::Stepper) => vec![("LEFT RIGHT", "CHANGE"), resets, leaves],
        (_, RowControl::Info) => Vec::new(),
    }
}

/// The line of keys along the foot of the screen: the keys the row under the cursor hears, the
/// keys of the screen, the three that open the play menu, the skin menu and the key editor -- each
/// named by the key it is on now -- and the key that leaves.
pub(super) fn hint(row: &[(&str, &str)], link_keys: &[String; 3]) -> String {
    let links = link_keys.iter().zip(LINK_ACTIONS).map(|(key, action)| (key.as_str(), action));
    let keys = row.iter().copied().chain(SCREEN_KEYS).chain(links).chain([LEAVE_KEY]);
    keys.map(|(key, action)| format!("{key} {action}")).collect::<Vec<_>>().join(KEY_SEPARATOR)
}

/// The values a choice steps through as the panel lists them: the ones the table declares, with
/// the output devices the host reported after the one the device row declares.
fn choices(id: SettingId, devices: &[String]) -> Vec<String> {
    let declared = cycle_values(id).iter().map(|value| (*value).to_string());
    if id == SettingId::AudioDevice { declared.chain(devices.iter().cloned()).collect() } else { declared.collect() }
}

/// What kind of row one is, in a word or two, with where a choice stands among its values.
fn kind(id: SettingId, line: &SettingsLine, devices: &[String]) -> String {
    let word = match line.control {
        RowControl::Switch { .. } => KIND_SWITCH,
        RowControl::Segments { .. } | RowControl::Stepper => KIND_CHOICE,
        RowControl::Slider => KIND_NUMBER,
        RowControl::Field { .. } if is_secret_row(id) => KIND_SECRET,
        RowControl::Field { .. } => KIND_TEXT,
        RowControl::Path { .. } if id == SettingId::SoundFolder => KIND_FOLDER,
        RowControl::Path { .. } => KIND_FILE,
        RowControl::Action => KIND_ACTION,
        RowControl::Info => KIND_STATUS,
    };
    let values = choices(id, devices);
    match (word, values.iter().position(|value| *value == line.value)) {
        (KIND_CHOICE, Some(at)) => format!("{word}  {} OF {}", at + 1, values.len()),
        _ => word.to_string(),
    }
}

/// How one step of a number, and ten of them, are written: in what the row shows, with its unit.
fn step_fact(number: &NumberValue) -> String {
    let spec = number.spec;
    let shown = |steps: u8| format!("{:.*}{}", usize::from(spec.decimals), spec.shown(i64::from(steps)), spec.unit);
    format!("{}, {} WITH SHIFT", shown(ONE_STEP), shown(SHIFT_STEPS))
}

/// What one row holds, each under a name: its value, what it ships as, the ends and the step of a
/// number, and the values of a choice. A row that holds nothing has none.
fn facts(id: SettingId, line: &SettingsLine, devices: &[String]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if line.value != ACTION_VALUE {
        out.push((FACT_VALUE, line.value.clone()));
    }
    if is_resettable(id) {
        let shipped = shipped_value(id);
        out.push((FACT_DEFAULT, if line.changed { format!("{shipped}  {RESTORE_MARK}") } else { shipped }));
    }
    if let Some(number) = &line.number {
        out.push((FACT_RANGE, format!("{} TO {}", number.spec.text(number.spec.min), number.spec.text(number.slider_max))));
        out.push((FACT_STEP, step_fact(number)));
    }
    if matches!(descriptor(id).kind, SettingKind::Cycle { .. }) {
        out.push((FACT_CHOICES, choices(id, devices).join(CHOICE_SEPARATOR)));
    }
    out
}

/// What is said of a row whose judge window or long-note margin stands over what the chart asks for.
fn caution(id: SettingId, line: &SettingsLine) -> Option<&'static str> {
    let ships_at = match id {
        SettingId::JudgeWidthKeyPGreat
        | SettingId::JudgeWidthKeyGreat
        | SettingId::JudgeWidthKeyGood
        | SettingId::JudgeWidthScratchPGreat
        | SettingId::JudgeWidthScratchGreat
        | SettingId::JudgeWidthScratchGood => JUDGE_RATE_DEFAULT_PERCENT,
        SettingId::LongNoteMargin => LN_MARGIN_DEFAULT_PERCENT,
        _ => return None,
    };
    line.number.filter(|number| number.steps > i64::from(ships_at)).map(|_| CUSTOM_JUDGE_CAUTION)
}

/// What the note beside a row means, or `None` for a row shown bare.
fn remark(line: &SettingsLine) -> Option<&'static str> {
    match line.note? {
        BUILT_IN_ONLY_NOTE => Some(BUILT_IN_ONLY_REMARK),
        SKIN_OPTION_NOTE => Some(SKIN_OPTION_REMARK),
        _ => None,
    }
}

/// What the panel beside the list says of one row: `line` is the row as the list draws it, and
/// `devices` the output devices the host reported, which are the values of the device row.
///
/// A path the list cuts short -- the file of the FONT row, the folder of the system sounds -- is
/// the row's value, and the panel writes it whole.
pub(super) fn info(id: SettingId, line: &SettingsLine, devices: &[String]) -> InfoScene {
    InfoScene {
        label: line.label.clone(),
        kind: kind(id, line, devices),
        description: description(id),
        facts: facts(id, line, devices),
        remark: remark(line),
        caution: caution(id, line),
        keys: row_keys(id, line.control).into_iter().map(|(key, action)| (key.to_string(), action)).collect(),
    }
}

/// The panel of a row while something other than the list has the keys -- a number or a line of
/// text being typed, a list drawn over the rows, the question about a group: the keys it lists are
/// the ones that hear, and the value is what is being typed, as the row draws it, where something
/// is.
pub(super) fn while_busy(info: InfoScene, keys: &[(&str, &'static str)], typed: Option<String>) -> InfoScene {
    let mut facts = info.facts;
    if let Some(typed) = typed {
        match facts.iter_mut().find(|(name, _)| *name == FACT_VALUE) {
            Some((_, value)) => *value = typed,
            None => facts.insert(0, (FACT_VALUE, typed)),
        }
    }
    InfoScene { facts, keys: keys.iter().map(|(key, action)| ((*key).to_string(), *action)).collect(), ..info }
}
