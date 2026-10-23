//! What the settings screen says of the row under the cursor: the panel beside the list, and the
//! line of keys along the foot of the screen.
//!
//! Both are worked out from what the row is and holds at this moment. The keys named are the keys
//! that row hears, with what each does to it -- a switch is not said to be typed into, and a row
//! that opens a list is not said to be stepped -- and the keys of the screen itself follow them on
//! the line at the foot. The panel adds what the row holds beside what it ships as, the ends and the
//! step of a number, the values of a choice, and what a note beside the row means.
//!
//! What a row holds is told, here as everywhere on the screen, by the English of the settings table
//! (`line.value`); what the panel writes is `line.shown`, in the language the screens are drawn in.
//! The words are [`crate::text::settings`]'s.

use rbms_config::{
    ACTION_VALUE, BUILT_IN_ONLY_NOTE, JUDGE_RATE_DEFAULT_PERCENT, LN_MARGIN_DEFAULT_PERCENT, SKIN_OPTION_NOTE, SettingId, SettingKind, cycle_values, descriptor,
};
use rbms_model::translations::Catalog;
use rbms_render::text::language;

use super::catalog::description;
use crate::ir_panel::is_secret_row;
use crate::number_row::{NumberValue, ONE_STEP, SHIFT_STEPS};
use crate::settings_ui::{is_resettable, shipped_value};
use crate::settings_view::{InfoScene, RESET_GROUP_KEY, RowControl, SettingsLine};
use crate::text::settings::panel::{KeyActionText, KeyNameText, PanelText};
use crate::text::settings::screen::ScreenText;
use crate::text::settings::{shown_choice, shown_value};

/// A key, and what it does.
type KeyHint = (&'static str, &'static str);

/// What stands between a key and the next on the line of keys.
const KEY_SEPARATOR: &str = "   ";

/// What stands between two values of a choice as the panel lists them.
const CHOICE_SEPARATOR: &str = ", ";

/// The keys every row of the screen hears whatever it holds, each with what it does, before the
/// three that open the other screens.
fn screen_keys() -> [KeyHint; 4] {
    let language = language();
    [
        ("UP DOWN", KeyActionText::Move.get(language)),
        ("PGUP PGDN", KeyActionText::Section.get(language)),
        ("TAB", KeyActionText::Group.get(language)),
        (RESET_GROUP_KEY, KeyActionText::ResetGroup.get(language)),
    ]
}

/// What the three keys that open the other screens are said to open, in the order the screen names
/// them: the play menu, the skin menu and the key editor.
fn link_actions() -> [&'static str; 3] {
    let language = language();
    [KeyActionText::Play.get(language), KeyActionText::Skins.get(language), KeyActionText::KeyEditor.get(language)]
}

/// The key that leaves the screen, and what it does.
fn leave_key() -> KeyHint {
    ("ESC", KeyActionText::Save.get(language()))
}

/// The keys of the screen while a line of text is being typed into a row.
pub(super) fn text_edit_hint() -> &'static str {
    ScreenText::TextEditHint.get(language())
}

/// The keys a number being typed into hears, as the panel lists them while it is typed.
pub(super) fn number_edit_keys() -> [KeyHint; 4] {
    let language = language();
    [
        (KeyNameText::Digits.get(language), KeyActionText::TypeNumber.get(language)),
        ("ENTER", KeyActionText::Set.get(language)),
        ("ESC", KeyActionText::Cancel.get(language)),
        ("UP DOWN TAB", KeyActionText::SetAndMoveOn.get(language)),
    ]
}

/// The keys a line of text being typed hears, as the panel lists them.
pub(super) fn text_edit_keys() -> [KeyHint; 3] {
    let language = language();
    [
        (KeyNameText::Keys.get(language), KeyActionText::Type.get(language)),
        ("ENTER", KeyActionText::Set.get(language)),
        ("ESC", KeyActionText::Cancel.get(language)),
    ]
}

/// The keys a list drawn over the rows hears, as the panel lists them.
pub(super) fn list_keys() -> [KeyHint; 4] {
    let language = language();
    [
        ("UP DOWN", KeyActionText::Move.get(language)),
        ("ENTER", KeyActionText::Add.get(language)),
        ("D", KeyActionText::Remove.get(language)),
        ("ESC", KeyActionText::SaveAndClose.get(language)),
    ]
}

/// The keys the question about a group hears, as the panel lists them.
pub(super) fn confirm_keys() -> [KeyHint; 2] {
    let language = language();
    [("ENTER", KeyActionText::ResetTheGroup.get(language)), ("ESC", KeyActionText::KeepAsItIs.get(language))]
}

/// What is said of a judge window or a long-note margin while it stands over what the chart asks
/// for: a run judged that way is a custom judge.
fn custom_judge_caution() -> &'static str {
    PanelText::CustomJudgeCaution.get(language())
}

/// The keys one row hears, each with what it does to that row. `control` is what sets the row.
///
/// Enter does what a step to the right does, as it does on the skin and play menus: it turns a
/// switch over, takes a choice to its next value, picks the folder of the system sounds or of the
/// downloads, runs an action of the score server and opens a line of text or a list; on a number it
/// opens the number to be typed into, and it opens the key editor and the font picker. No row
/// leaves the screen on Enter: Escape does, and the line of keys says so.
pub(super) fn row_keys(id: SettingId, control: RowControl) -> Vec<KeyHint> {
    let language = language();
    let resets = ("0", KeyActionText::ToDefault.get(language));
    match (id, control) {
        (_, RowControl::Slider) => vec![
            ("LEFT RIGHT", KeyActionText::OneStep.get(language)),
            ("SHIFT", KeyActionText::TenSteps.get(language)),
            ("ENTER", KeyActionText::Type.get(language)),
            resets,
            (KeyNameText::Drag.get(language), KeyActionText::Slider.get(language)),
        ],
        (SettingId::KeyConfig, _) => vec![("ENTER", KeyActionText::Open.get(language))],
        (SettingId::Font, _) => vec![("ENTER", KeyActionText::ChooseFile.get(language)), resets],
        (SettingId::SoundFolder | SettingId::DownloadFolder, _) => {
            vec![("ENTER RIGHT", KeyActionText::ChooseFolder.get(language)), ("0", KeyActionText::Unset.get(language))]
        }
        (SettingId::Account, _) => vec![("ENTER", KeyActionText::ShowState.get(language))],
        (SettingId::Rivals | SettingId::IrProfiles, _) => vec![("ENTER", KeyActionText::OpenList.get(language))],
        (_, RowControl::Field { .. }) => vec![("ENTER", KeyActionText::Edit.get(language))],
        (_, RowControl::Action | RowControl::Path { .. }) => vec![("ENTER", KeyActionText::Run.get(language))],
        (_, RowControl::Switch { .. }) => vec![("LEFT RIGHT", KeyActionText::Switch.get(language)), ("ENTER", KeyActionText::Switch.get(language)), resets],
        (_, RowControl::Segments { .. } | RowControl::Stepper) => {
            vec![("LEFT RIGHT", KeyActionText::Change.get(language)), ("ENTER", KeyActionText::Next.get(language)), resets]
        }
        (_, RowControl::Info) => Vec::new(),
    }
}

/// The line of keys along the foot of the screen: the keys the row under the cursor hears, the
/// keys of the screen, the three that open the play menu, the skin menu and the key editor -- each
/// named by the key it is on now -- and the key that leaves.
pub(super) fn hint(row: &[KeyHint], link_keys: &[String; 3]) -> String {
    let links = link_keys.iter().zip(link_actions()).map(|(key, action)| (key.as_str(), action));
    let keys = row.iter().copied().chain(screen_keys()).chain(links).chain([leave_key()]);
    keys.map(|(key, action)| format!("{key} {action}")).collect::<Vec<_>>().join(KEY_SEPARATOR)
}

/// The values a choice steps through as the settings table writes them: the ones the table
/// declares, with the output devices the host reported after the one the device row declares.
fn choices(id: SettingId, devices: &[String]) -> Vec<String> {
    let declared = cycle_values(id).iter().map(|value| (*value).to_string());
    if id == SettingId::AudioDevice { declared.chain(devices.iter().cloned()).collect() } else { declared.collect() }
}

/// The same values as the panel lists them, in the language the screens are drawn in.
fn shown_choices(id: SettingId, devices: &[String]) -> Vec<String> {
    let declared = cycle_values(id).iter().map(|value| shown_choice(id, value).to_string());
    if id == SettingId::AudioDevice { declared.chain(devices.iter().cloned()).collect() } else { declared.collect() }
}

/// What kind of row one is, in a word or two, with where a choice stands among its values.
fn kind(id: SettingId, line: &SettingsLine, devices: &[String]) -> String {
    let language = language();
    let word = match line.control {
        RowControl::Switch { .. } => PanelText::KindSwitch,
        RowControl::Segments { .. } | RowControl::Stepper => PanelText::KindChoice,
        RowControl::Slider => PanelText::KindNumber,
        RowControl::Field { .. } if is_secret_row(id) => PanelText::KindSecret,
        RowControl::Field { .. } => PanelText::KindText,
        RowControl::Path { .. } if matches!(id, SettingId::SoundFolder | SettingId::DownloadFolder) => PanelText::KindFolder,
        RowControl::Path { .. } => PanelText::KindFile,
        RowControl::Action => PanelText::KindAction,
        RowControl::Info => PanelText::KindStatus,
    };
    let values = choices(id, devices);
    match (word, values.iter().position(|value| *value == line.value)) {
        (PanelText::KindChoice, Some(at)) => PanelText::KindChoiceAt.fill(language, &[&(at + 1), &values.len()]),
        _ => word.get(language).to_string(),
    }
}

/// How one step of a number, and ten of them, are written: in what the row shows, with its unit.
fn step_fact(number: &NumberValue) -> String {
    let spec = number.spec;
    let shown = |steps: u8| format!("{:.*}{}", usize::from(spec.decimals), spec.shown(i64::from(steps)), spec.unit);
    PanelText::StepWithShift.fill(language(), &[&shown(ONE_STEP), &shown(SHIFT_STEPS)])
}

/// What one row holds, each under a name: its value, what it ships as, the ends and the step of a
/// number, and the values of a choice. A row that holds nothing has none.
fn facts(id: SettingId, line: &SettingsLine, devices: &[String]) -> Vec<(&'static str, String)> {
    let language = language();
    let mut out = Vec::new();
    if line.value != ACTION_VALUE {
        out.push((PanelText::FactValue.get(language), line.shown.clone()));
    }
    if is_resettable(id) {
        let shipped = shown_value(id, &shipped_value(id));
        out.push((PanelText::FactDefault.get(language), if line.changed { PanelText::DefaultWhileChanged.fill(language, &[&shipped]) } else { shipped }));
    }
    if let Some(number) = &line.number {
        out.push((
            PanelText::FactRange.get(language),
            PanelText::RangeFrom.fill(language, &[&number.spec.text(number.spec.min), &number.spec.text(number.slider_max)]),
        ));
        out.push((PanelText::FactStep.get(language), step_fact(number)));
    }
    if matches!(descriptor(id).kind, SettingKind::Cycle { .. }) {
        out.push((PanelText::FactChoices.get(language), shown_choices(id, devices).join(CHOICE_SEPARATOR)));
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
    line.number.filter(|number| number.steps > i64::from(ships_at)).map(|_| custom_judge_caution())
}

/// What the note beside a row means, or `None` for a row shown bare.
fn remark(line: &SettingsLine) -> Option<&'static str> {
    match line.note? {
        BUILT_IN_ONLY_NOTE => Some(PanelText::BuiltInOnlyRemark.get(language())),
        SKIN_OPTION_NOTE => Some(PanelText::SkinOptionRemark.get(language())),
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
        let value_name = PanelText::FactValue.get(language());
        match facts.iter_mut().find(|(name, _)| *name == value_name) {
            Some((_, value)) => *value = typed,
            None => facts.insert(0, (value_name, typed)),
        }
    }
    InfoScene { facts, keys: keys.iter().map(|(key, action)| ((*key).to_string(), *action)).collect(), ..info }
}
