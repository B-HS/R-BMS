//! Property names, the reference's own list of what it implements, and the options that hold still.
//!
//! A skin addresses a property by number or, wherever the reference accepts one, by name:
//! `main_state.number("playlevel")`, `op = { "!autoplay_on" }`, `act = "favorite_chart"`. The names are
//! not the constant names of `SkinProperty.java`. They are the constants of the enums in the
//! property factories, which [`generated`](super::generated) extracts, plus the numbered families
//! the factories build at run time (`practice_item3`, `ranking_exscore10`), which no extraction can
//! see and which are therefore spelled out here, each beside the line it mirrors.
//!
//! Three questions are answered from the same tables:
//!
//! - [`id_of_name`] and [`name_of_id`] translate between the two ways of addressing a property.
//! - [`reference_implements`] says whether the reference has a property under an id at all. That
//!   is not the same as the id being declared: the reference declares constants it never
//!   implements and implements ids it never declared. A script can tell the difference --
//!   `main_state.number` answers zero for an id with no property and the "no value" sentinel for a
//!   property with nothing to report -- and so can a draw condition, which is checked against the
//!   skin's own options exactly when no property answers.
//! - [`static_scope`] says on which screens the reference settles an option once, when the skin is
//!   prepared, instead of on every frame (`BooleanProperty.isStatic`).

use std::borrow::Cow;

use serde::Deserialize;

use super::generated::*;

/// The exclusive upper end of the ids the reference's factories look up at all
/// (`ID_LENGTH` in `BooleanPropertyFactory` and `IntegerPropertyFactory`).
pub const PROPERTY_ID_LIMIT: i32 = 65_536;

/// The mark that negates a boolean name (`BooleanPropertyFactory.getBooleanProperty(String)`).
pub const NEGATION_MARK: char = '!';

/// One of the id spaces a skin may address by name.
///
/// These are the reference's lookup functions, not [`PropertyKind`](super::PropertyKind): the
/// integer ids split into the values a number shows and the indices an image set selects by, and
/// the float ids are asked for either as a rate alone or as a float with the rates as a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NameSpace {
    /// `BooleanPropertyFactory.getBooleanProperty`: draw conditions and `main_state.option`.
    Boolean,
    /// `IntegerPropertyFactory.getIntegerProperty`: number objects and `main_state.number`.
    Integer,
    /// `IntegerPropertyFactory.getImageIndexProperty`: image sets and `main_state.event_index`.
    ImageIndex,
    /// `FloatPropertyFactory.getRateProperty`: sliders, graphs and every script field of the
    /// reference's `FloatProperty` type.
    Rate,
    /// `FloatPropertyFactory.getFloatProperty`: float objects and `main_state.float_number`. A name
    /// or id the float space does not hold is looked up as a rate.
    Float,
    /// `StringPropertyFactory.getStringProperty`: text objects and `main_state.text`.
    Text,
    /// `EventFactory.getEvent`: click actions and `main_state.event_exec`.
    Event,
}

impl NameSpace {
    /// Every space, in the order the generated tables are emitted.
    pub const ALL: [NameSpace; 7] =
        [NameSpace::Boolean, NameSpace::Integer, NameSpace::ImageIndex, NameSpace::Rate, NameSpace::Float, NameSpace::Text, NameSpace::Event];

    /// The enum tables this space reads, in lookup order.
    const fn tables(self) -> &'static [&'static [(i32, &'static str)]] {
        match self {
            NameSpace::Boolean => &[ALL_BOOLEAN_NAME],
            NameSpace::Integer => &[ALL_INTEGER_NAME],
            NameSpace::ImageIndex => &[ALL_IMAGE_INDEX_NAME],
            NameSpace::Rate => &[ALL_RATE_NAME],
            NameSpace::Float => &[ALL_FLOAT_NAME, ALL_RATE_NAME],
            NameSpace::Text => &[ALL_STRING_NAME],
            NameSpace::Event => &[ALL_EVENT_NAME],
        }
    }
}

/// A family of names that differ only by a number: `<prefix><number><suffix>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberedNames {
    pub space: NameSpace,
    pub prefix: &'static str,
    pub suffix: &'static str,
    /// The number the first member of the family carries.
    pub first_number: i32,
    /// How many consecutive numbers the family covers.
    pub count: i32,
    /// The id of the member numbered [`Self::first_number`].
    pub first_id: i32,
    /// How far the id moves for each step of the number. It is negative for the one family the
    /// reference counts downward.
    pub id_step: i32,
    /// Whether the number must be written the one way the reference prints it. The reference
    /// matches some families against a list of prepared names, which `practice_item03` is not in,
    /// and parses the number out of the others, which accepts it.
    pub canonical: bool,
}

impl NumberedNames {
    /// A family numbered from one whose ids run upward from `first_id`, with the number parsed.
    const fn parsed(space: NameSpace, prefix: &'static str, suffix: &'static str, first_id: i32, count: i32) -> Self {
        Self { space, prefix, suffix, first_number: 1, count, first_id, id_step: 1, canonical: false }
    }

    /// A family numbered from one whose ids run upward from `first_id`, matched as prepared names.
    const fn listed(space: NameSpace, prefix: &'static str, first_id: i32, count: i32) -> Self {
        Self { space, prefix, suffix: "", first_number: 1, count, first_id, id_step: 1, canonical: true }
    }

    /// The id of the member `name` spells, or `None` when it is not one of the family.
    fn id_of(&self, name: &str) -> Option<i32> {
        let digits = name.strip_prefix(self.prefix)?.strip_suffix(self.suffix)?;
        let number: i32 = digits.parse().ok()?;
        if self.canonical && digits != number.to_string() {
            return None;
        }
        let index = number.checked_sub(self.first_number)?;
        (0..self.count).contains(&index).then(|| self.first_id + index * self.id_step)
    }

    /// The name of the member under `id`, or `None` when the family has no such member.
    fn name_of(&self, id: i32) -> Option<String> {
        let offset = id - self.first_id;
        let index = offset / self.id_step;
        (offset % self.id_step == 0 && (0..self.count).contains(&index)).then(|| format!("{}{}{}", self.prefix, self.first_number + index, self.suffix))
    }

    /// Whether the family has a member under `id`.
    fn covers(&self, id: i32) -> bool {
        self.name_of(id).is_some()
    }
}

/// How many rows a practice screen shows (`BooleanPropertyPattern`, `StringPropertyPattern`,
/// `EventPattern`).
const PRACTICE_ITEMS: i32 = 16;

/// How many entries of a ranking, a course or a customise page the reference numbers.
const NUMBERED_ROWS: i32 = 10;

/// The first ranking EX score id (`IntegerPropertyPattern.RANKING_EXSCORE`).
const RANKING_EXSCORE_FIRST: i32 = 380;

/// The first ranking position id (`IntegerPropertyPattern.RANKING_INDEX`), which in the image index
/// space is the first ranking clear type (`RANKING_CLEARTYPE`).
const RANKING_INDEX_FIRST: i32 = 390;

/// The first of the ten primary key names (`StringPropertyPattern.KEY_1_TO_10`).
const KEY_NAME_FIRST: i32 = 40;

/// The number the first extended key name carries (`StringPropertyPattern.KEY_11_TO_54`).
const KEY_NAME_EXTENDED_NUMBER: i32 = 11;

/// The first extended key name id.
const KEY_NAME_EXTENDED_FIRST: i32 = 240;

/// How many extended key names there are.
const KEY_NAME_EXTENDED_COUNT: i32 = 44;

/// The id of `targetnamep1`, the nearest target above the player's own. The family runs downward to
/// `targetnamep10` at 200 (`StringPropertyPattern.TARGET_NAME_PREVIOUS`, index `10 - number`).
const TARGET_NAME_PREVIOUS_NEAREST: i32 = 209;

/// The first of the targets below the player's own (`StringPropertyPattern.TARGET_NAME_NEXT`).
const TARGET_NAME_NEXT_FIRST: i32 = 210;

/// The first key assignment event (`EventPattern.keyAssignIds`).
const KEY_ASSIGN_FIRST: i32 = 101;

/// How many key assignment events follow [`KEY_ASSIGN_FIRST`] without a gap.
const KEY_ASSIGN_FIRST_RUN: i32 = 39;

/// The first key assignment event of the second run.
const KEY_ASSIGN_SECOND: i32 = 150;

/// How many key assignment events the second run holds.
const KEY_ASSIGN_SECOND_RUN: i32 = 15;

/// Every numbered family of the reference, in the order each factory tries them.
pub const NUMBERED_NAMES: &[NumberedNames] = &[
    NumberedNames::parsed(NameSpace::Boolean, "practice_item", "", OPTION_PRACTICE_ITEM1, PRACTICE_ITEMS),
    NumberedNames::parsed(NameSpace::Boolean, "practice_item", "_selected", OPTION_PRACTICE_ITEM1_SELECTED, PRACTICE_ITEMS),
    NumberedNames::listed(NameSpace::Integer, "ranking_exscore", RANKING_EXSCORE_FIRST, NUMBERED_ROWS),
    NumberedNames::listed(NameSpace::Integer, "ranking_index", RANKING_INDEX_FIRST, NUMBERED_ROWS),
    NumberedNames::listed(NameSpace::ImageIndex, "playertype_ranking", RANKING_EXSCORE_FIRST, NUMBERED_ROWS),
    NumberedNames::listed(NameSpace::ImageIndex, "cleartype_ranking", RANKING_INDEX_FIRST, NUMBERED_ROWS),
    NumberedNames::parsed(NameSpace::Text, "key", "", KEY_NAME_FIRST, NUMBERED_ROWS),
    NumberedNames {
        space: NameSpace::Text,
        prefix: "key",
        suffix: "",
        first_number: KEY_NAME_EXTENDED_NUMBER,
        count: KEY_NAME_EXTENDED_COUNT,
        first_id: KEY_NAME_EXTENDED_FIRST,
        id_step: 1,
        canonical: false,
    },
    NumberedNames::parsed(NameSpace::Text, "skincategory", "", STRING_SKIN_CUSTOMIZE_CATEGORY1, NUMBERED_ROWS),
    NumberedNames::parsed(NameSpace::Text, "skinitem", "", STRING_SKIN_CUSTOMIZE_ITEM1, NUMBERED_ROWS),
    NumberedNames::parsed(NameSpace::Text, "rankingname", "", STRING_RANKING1_NAME, NUMBERED_ROWS),
    NumberedNames::parsed(NameSpace::Text, "coursetitle", "", STRING_COURSE1_TITLE, NUMBERED_ROWS),
    NumberedNames {
        space: NameSpace::Text,
        prefix: "targetnamep",
        suffix: "",
        first_number: 1,
        count: NUMBERED_ROWS,
        first_id: TARGET_NAME_PREVIOUS_NEAREST,
        id_step: -1,
        canonical: false,
    },
    NumberedNames::parsed(NameSpace::Text, "targetnamen", "", TARGET_NAME_NEXT_FIRST, NUMBERED_ROWS),
    NumberedNames::parsed(NameSpace::Text, "practice_item", "", STRING_PRACTICE_ITEM1, PRACTICE_ITEMS),
    NumberedNames::parsed(NameSpace::Text, "practice_item", "_label", STRING_PRACTICE_ITEM_LABEL1, PRACTICE_ITEMS),
    NumberedNames::parsed(NameSpace::Text, "practice_item_label", "", STRING_PRACTICE_ITEM_LABEL1, PRACTICE_ITEMS),
    NumberedNames::parsed(NameSpace::Text, "practice_item", "_value", STRING_PRACTICE_ITEM_VALUE1, PRACTICE_ITEMS),
    NumberedNames::parsed(NameSpace::Text, "practice_item_value", "", STRING_PRACTICE_ITEM_VALUE1, PRACTICE_ITEMS),
    NumberedNames::parsed(NameSpace::Event, "keyassign", "", KEY_ASSIGN_FIRST, KEY_ASSIGN_FIRST_RUN),
    NumberedNames {
        space: NameSpace::Event,
        prefix: "keyassign",
        suffix: "",
        first_number: KEY_ASSIGN_FIRST_RUN + 1,
        count: KEY_ASSIGN_SECOND_RUN,
        first_id: KEY_ASSIGN_SECOND,
        id_step: 1,
        canonical: false,
    },
    NumberedNames::parsed(NameSpace::Event, "practice_item", "", BUTTON_PRACTICE_ITEM1, PRACTICE_ITEMS),
];

/// The player's clear counts by lamp on the ranking server (`IntegerPropertyPattern.IR_CLEAR_COUNT`).
const IR_CLEAR_COUNT_NAMES: &[(i32, &str)] = &[
    (202, "ir_player_noplay"),
    (210, "ir_player_failed"),
    (204, "ir_player_assist"),
    (206, "ir_player_lightassist"),
    (212, "ir_player_easy"),
    (214, "ir_player_normal"),
    (216, "ir_player_hard"),
    (208, "ir_player_exhard"),
    (218, "ir_player_fullcombo"),
    (222, "ir_player_perfect"),
    (224, "ir_player_max"),
];

/// The same counts as whole percentages (`IntegerPropertyPattern.IR_CLEAR_RATE`), and as shares in
/// the float space (`FloatPropertyPattern.IR_CLEAR_RATE`).
const IR_CLEAR_RATE_NAMES: &[(i32, &str)] = &[
    (203, "ir_player_noplay_rate"),
    (211, "ir_player_failed_rate"),
    (205, "ir_player_assist_rate"),
    (207, "ir_player_lightassist_rate"),
    (213, "ir_player_easy_rate"),
    (215, "ir_player_normal_rate"),
    (217, "ir_player_hard_rate"),
    (209, "ir_player_exhard_rate"),
    (219, "ir_player_fullcombo_rate"),
    (223, "ir_player_perfect_rate"),
    (225, "ir_player_max_rate"),
];

/// The first decimal of those percentages (`IntegerPropertyPattern.IR_CLEAR_RATE_AFTERDOT`).
const IR_CLEAR_RATE_AFTERDOT_NAMES: &[(i32, &str)] = &[
    (230, "ir_player_noplay_rate_afterdot"),
    (234, "ir_player_failed_rate_afterdot"),
    (231, "ir_player_assist_rate_afterdot"),
    (232, "ir_player_lightassist_rate_afterdot"),
    (235, "ir_player_easy_rate_afterdot"),
    (236, "ir_player_normal_rate_afterdot"),
    (237, "ir_player_hard_rate_afterdot"),
    (233, "ir_player_exhard_rate_afterdot"),
    (238, "ir_player_fullcombo_rate_afterdot"),
    (239, "ir_player_perfect_rate_afterdot"),
    (240, "ir_player_max_rate_afterdot"),
];

/// A folder's chart counts by lamp (`IntegerPropertyPattern.FOLDER_CLEAR_COUNT`). `folder_prefect`
/// is the reference's own spelling.
const FOLDER_CLEAR_COUNT_NAMES: &[(i32, &str)] = &[
    (320, "folder_noplay"),
    (321, "folder_failed"),
    (322, "folder_assist"),
    (323, "folder_lightassist"),
    (324, "folder_easy"),
    (325, "folder_normal"),
    (326, "folder_hard"),
    (327, "folder_exhard"),
    (328, "folder_fullcombo"),
    (329, "folder_prefect"),
    (330, "folder_max"),
];

/// The families the reference names one by one rather than by number, in the order each factory
/// tries them.
pub const LISTED_NAMES: &[(NameSpace, &[(i32, &str)])] = &[
    (NameSpace::Integer, IR_CLEAR_COUNT_NAMES),
    (NameSpace::Integer, IR_CLEAR_RATE_NAMES),
    (NameSpace::Integer, IR_CLEAR_RATE_AFTERDOT_NAMES),
    (NameSpace::Integer, FOLDER_CLEAR_COUNT_NAMES),
    (NameSpace::Float, IR_CLEAR_RATE_NAMES),
];

/// Runs of ids the reference answers from code instead of from an enum constant, so that no name
/// stands for them: the judgement counts of `ValueType.getProperty`, and the per-key judgements and
/// skin type buttons of `IntegerPropertyFactory.getImageIndexProperty`. Both ends are inclusive.
pub const UNNAMED_RUNS: &[(NameSpace, i32, i32)] = &[
    (NameSpace::Integer, NUMBER_DURATION_LANECOVER_ON, NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_OFF),
    (NameSpace::Integer, NUMBER_PERFECT2, NUMBER_POOR2),
    (NameSpace::Integer, NUMBER_PERFECT_RATE, NUMBER_POOR_RATE),
    (NameSpace::Integer, NUMBER_PERFECT, NUMBER_POOR),
    (NameSpace::Integer, NUMBER_EARLY_PERFECT, NUMBER_LATE_POOR),
    (NameSpace::Integer, NUMBER_RIVAL_PERFECT, NUMBER_RIVAL_POOR),
    (NameSpace::Integer, NUMBER_RIVAL_PERFECT_RATE, NUMBER_RIVAL_POOR_RATE),
    (NameSpace::ImageIndex, VALUE_JUDGE_1P_SCRATCH, VALUE_JUDGE_2P_KEY9),
    (NameSpace::ImageIndex, VALUE_JUDGE_1P_KEY10, VALUE_JUDGE_2P_KEY99),
    (NameSpace::ImageIndex, BUTTON_SKINSELECT_7KEY, BUTTON_SKINSELECT_COURSE_RESULT),
    (NameSpace::ImageIndex, BUTTON_SKINSELECT_24KEY, BUTTON_SKINSELECT_24KEY_BATTLE),
];

/// The id a name stands for in one space, or `None` when the reference knows no such name.
///
/// The numbered and listed families are tried before the enum constants, as each factory does. In
/// [`NameSpace::Boolean`] every leading [`NEGATION_MARK`] negates the property, which the answer
/// carries as its sign -- the form [`DrawStateSource::boolean`](crate::dst::DrawStateSource::boolean)
/// takes.
pub fn id_of_name(space: NameSpace, name: &str) -> Option<i32> {
    if space == NameSpace::Boolean {
        let bare = name.trim_start_matches(NEGATION_MARK);
        let negations = name.len() - bare.len();
        let id = plain_id_of_name(space, bare)?;
        return Some(if negations % 2 == 1 { -id } else { id });
    }
    plain_id_of_name(space, name)
}

/// [`id_of_name`] without the negation of a boolean name.
fn plain_id_of_name(space: NameSpace, name: &str) -> Option<i32> {
    let numbered = NUMBERED_NAMES.iter().filter(|family| family.space == space).find_map(|family| family.id_of(name));
    let listed = || LISTED_NAMES.iter().filter(|(family, _)| *family == space).flat_map(|(_, names)| names.iter()).find(|(_, listed)| *listed == name);
    let constant = || space.tables().iter().flat_map(|table| table.iter()).find(|(_, constant)| *constant == name);
    numbered.or_else(|| listed().map(|(id, _)| *id)).or_else(|| constant().map(|(id, _)| *id))
}

/// The name the reference gives an id in one space, or `None` when it has none: the id is not
/// implemented, or it is one of the [`UNNAMED_RUNS`]. A negative boolean id answers the name of its
/// positive with the [`NEGATION_MARK`] in front.
pub fn name_of_id(space: NameSpace, id: i32) -> Option<Cow<'static, str>> {
    if space == NameSpace::Boolean && id < 0 {
        return name_of_id(space, id.checked_neg()?).map(|name| Cow::Owned(format!("{NEGATION_MARK}{name}")));
    }
    let numbered = NUMBERED_NAMES.iter().filter(|family| family.space == space).find_map(|family| family.name_of(id));
    let listed = || LISTED_NAMES.iter().filter(|(family, _)| *family == space).flat_map(|(_, names)| names.iter()).find(|(listed, _)| *listed == id);
    let constant = || space.tables().iter().flat_map(|table| table.iter()).find(|(constant, _)| *constant == id);
    numbered.map(Cow::Owned).or_else(|| listed().map(|(_, name)| Cow::Borrowed(*name))).or_else(|| constant().map(|(_, name)| Cow::Borrowed(*name)))
}

/// Whether the reference has a property under this id in this space.
///
/// A boolean id is asked about without its sign. Every event id is implemented: an id no event is
/// defined for still runs, as a request the screen is free to ignore (`EventFactory.getEvent`).
pub fn reference_implements(space: NameSpace, id: i32) -> bool {
    if space == NameSpace::Event {
        return true;
    }
    let id = if space == NameSpace::Boolean { id.saturating_abs() } else { id };
    if !(0..PROPERTY_ID_LIMIT).contains(&id) {
        return false;
    }
    UNNAMED_RUNS.iter().any(|(run, first, last)| *run == space && (*first..=*last).contains(&id))
        || NUMBERED_NAMES.iter().any(|family| family.space == space && family.covers(id))
        || LISTED_NAMES.iter().any(|(family, names)| *family == space && names.iter().any(|(listed, _)| *listed == id))
        || space.tables().iter().any(|table| table.iter().any(|(constant, _)| *constant == id))
}

/// Whether the reference can write a value back under this id: a rate a slider drags
/// (`FloatPropertyFactory.getRateWriter`) or a string an editable text confirms
/// (`StringPropertyFactory.getStringWriter`). No other space is written to.
pub fn reference_writes(space: NameSpace, id: i32) -> bool {
    match space {
        NameSpace::Rate => WRITABLE_RATES.contains(&id),
        NameSpace::Text => WRITABLE_STRINGS.contains(&id),
        NameSpace::Boolean | NameSpace::Integer | NameSpace::ImageIndex | NameSpace::Float | NameSpace::Event => false,
    }
}

/// The kind of screen a static classification is asked for. In a scenario file it is written in
/// lower case: `"select"`, `"result"` or `"other"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticScreen {
    /// The song browser.
    Select,
    /// The result screen of a chart or of a course.
    Result,
    /// Every other screen.
    Other,
}

/// On which screens the reference settles an option once instead of on every frame
/// (`DrawConditionProperty.TYPE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticScope {
    /// `TYPE_NO_STATIC`: evaluated on every frame everywhere.
    Never,
    /// `TYPE_STATIC_WITHOUT_MUSICSELECT`: settled once on every screen but the song browser, where
    /// the chart it describes changes under the cursor.
    OutsideSelect,
    /// `TYPE_STATIC_ON_RESULT`: settled once on the two result screens, where the score is final.
    OnResult,
    /// `TYPE_STATIC_ALL`: settled once on every screen.
    Always,
}

impl StaticScope {
    /// Whether an option of this scope is settled once on `screen`.
    pub const fn holds_on(self, screen: StaticScreen) -> bool {
        match self {
            StaticScope::Never => false,
            StaticScope::OutsideSelect => !matches!(screen, StaticScreen::Select),
            StaticScope::OnResult => matches!(screen, StaticScreen::Result),
            StaticScope::Always => true,
        }
    }
}

/// On which screens the reference settles the option under `id` once. The sign of the id plays no
/// part, and an id the reference does not implement is never static.
///
/// This is what an implementation of [`SkinHost::is_static`](super::SkinHost::is_static) answers
/// from: `static_scope(id).holds_on(screen)`.
pub fn static_scope(id: i32) -> StaticScope {
    let id = id.saturating_abs();
    [(STATIC_OUTSIDE_SELECT, StaticScope::OutsideSelect), (STATIC_ON_RESULT, StaticScope::OnResult), (STATIC_ALWAYS, StaticScope::Always)]
        .into_iter()
        .find(|(ids, _)| ids.contains(&id))
        .map_or(StaticScope::Never, |(_, scope)| scope)
}

#[cfg(test)]
mod tests {
    use super::{NameSpace, PROPERTY_ID_LIMIT, StaticScope, StaticScreen, id_of_name, name_of_id, reference_implements, reference_writes, static_scope};
    use crate::property::generated::{NUMBER_PLAYLEVEL, OPTION_AUTOPLAYON, OPTION_PRACTICE_ITEM1, OPTION_PRACTICE_ITEM1_SELECTED, STRING_PRACTICE_ITEM_LABEL1};

    /// How many option ids the reference's factory answers, counted from its source by hand for the
    /// property survey this crate was written against. The four counts below were made the same way.
    const REFERENCE_BOOLEANS: usize = 248;

    /// How many number ids the reference's factory answers.
    const REFERENCE_INTEGERS: usize = 263;

    /// How many rate ids the reference's factory answers.
    const REFERENCE_RATES: usize = 31;

    /// How many float ids the reference's factory answers before it falls back to the rates.
    const REFERENCE_FLOATS: usize = 40;

    /// How many string ids the reference's factory answers.
    const REFERENCE_TEXTS: usize = 188;

    #[test]
    fn a_name_resolves_in_its_own_space_only() {
        assert_eq!(id_of_name(NameSpace::Boolean, "autoplay_on"), Some(OPTION_AUTOPLAYON));
        assert_eq!(id_of_name(NameSpace::Integer, "autoplay_on"), None);
        assert_eq!(id_of_name(NameSpace::Integer, "playlevel"), Some(NUMBER_PLAYLEVEL));
        assert_eq!(id_of_name(NameSpace::ImageIndex, "lnmode"), Some(308));
        assert_eq!(id_of_name(NameSpace::Event, "favorite_chart"), Some(90));
        assert_eq!(id_of_name(NameSpace::Text, "no_such_name"), None);
    }

    #[test]
    fn a_boolean_name_carries_its_negations_as_a_sign() {
        assert_eq!(id_of_name(NameSpace::Boolean, "!autoplay_on"), Some(-OPTION_AUTOPLAYON));
        assert_eq!(id_of_name(NameSpace::Boolean, "!!autoplay_on"), Some(OPTION_AUTOPLAYON));
        assert_eq!(name_of_id(NameSpace::Boolean, -OPTION_AUTOPLAYON).as_deref(), Some("!autoplay_on"));
        assert_eq!(id_of_name(NameSpace::Integer, "!playlevel"), None, "only a boolean name negates");
    }

    #[test]
    fn a_float_name_falls_back_to_the_rates_and_a_rate_name_does_not_reach_the_floats() {
        assert_eq!(id_of_name(NameSpace::Float, "score_rate"), Some(1102));
        assert_eq!(id_of_name(NameSpace::Float, "musicselect_position"), Some(1));
        assert_eq!(id_of_name(NameSpace::Rate, "score_rate"), None);
        assert_eq!(id_of_name(NameSpace::Float, "ir_player_easy_rate"), Some(213));
        assert_eq!(id_of_name(NameSpace::Integer, "ir_player_easy_rate"), Some(213));
        assert_eq!(id_of_name(NameSpace::Integer, "folder_prefect"), Some(329), "the reference's spelling is the name");
    }

    #[test]
    fn numbered_families_follow_each_factorys_own_arithmetic() {
        assert_eq!(id_of_name(NameSpace::Boolean, "practice_item1"), Some(OPTION_PRACTICE_ITEM1));
        assert_eq!(id_of_name(NameSpace::Boolean, "practice_item16_selected"), Some(OPTION_PRACTICE_ITEM1_SELECTED + 15));
        assert_eq!(id_of_name(NameSpace::Boolean, "practice_item17"), None);
        assert_eq!(id_of_name(NameSpace::Boolean, "practice_item"), None);
        assert_eq!(id_of_name(NameSpace::Text, "key1"), Some(40));
        assert_eq!(id_of_name(NameSpace::Text, "key10"), Some(49));
        assert_eq!(id_of_name(NameSpace::Text, "key11"), Some(240));
        assert_eq!(id_of_name(NameSpace::Text, "key54"), Some(283));
        assert_eq!(id_of_name(NameSpace::Text, "key55"), None);
        assert_eq!(id_of_name(NameSpace::Text, "targetnamep1"), Some(209));
        assert_eq!(id_of_name(NameSpace::Text, "targetnamep10"), Some(200));
        assert_eq!(id_of_name(NameSpace::Text, "targetnamen1"), Some(210));
        assert_eq!(id_of_name(NameSpace::Text, "practice_item3_label"), Some(STRING_PRACTICE_ITEM_LABEL1 + 2));
        assert_eq!(id_of_name(NameSpace::Text, "practice_item_label3"), Some(STRING_PRACTICE_ITEM_LABEL1 + 2));
        assert_eq!(id_of_name(NameSpace::Event, "keyassign39"), Some(139));
        assert_eq!(id_of_name(NameSpace::Event, "keyassign40"), Some(150));
        assert_eq!(id_of_name(NameSpace::Event, "keyassign54"), Some(164));
        assert_eq!(id_of_name(NameSpace::Event, "keyassign55"), None);
    }

    #[test]
    fn a_parsed_number_may_be_padded_and_a_prepared_name_may_not() {
        assert_eq!(id_of_name(NameSpace::Text, "key01"), Some(40));
        assert_eq!(id_of_name(NameSpace::Integer, "ranking_exscore10"), Some(389));
        assert_eq!(id_of_name(NameSpace::Integer, "ranking_exscore01"), None);
        assert_eq!(id_of_name(NameSpace::ImageIndex, "cleartype_ranking1"), Some(390));
    }

    #[test]
    fn an_id_answers_the_name_that_resolves_back_to_it() {
        for space in NameSpace::ALL {
            for (id, _) in space.tables()[0] {
                let name = name_of_id(space, *id).expect("a constant's id has a name");
                assert_eq!(id_of_name(space, &name), Some(*id), "{space:?} {name}");
            }
        }
        assert_eq!(name_of_id(NameSpace::Text, 205).as_deref(), Some("targetnamep5"));
        assert_eq!(name_of_id(NameSpace::Event, 150).as_deref(), Some("keyassign40"));
        assert_eq!(name_of_id(NameSpace::Integer, 110), None, "a judgement count is answered from code and has no name");
    }

    #[test]
    fn implemented_is_not_the_same_as_declared() {
        assert!(reference_implements(NameSpace::Integer, 110), "judgement counts are answered from code");
        assert!(reference_implements(NameSpace::Integer, 381), "a ranking score nobody declared a constant for");
        assert!(!reference_implements(NameSpace::Integer, 104), "a declared constant with no property");
        assert!(!reference_implements(NameSpace::Integer, -1));
        assert!(!reference_implements(NameSpace::Integer, 65_536));
        assert!(reference_implements(NameSpace::Boolean, -OPTION_AUTOPLAYON));
        assert!(!reference_implements(NameSpace::Boolean, 44), "a declared option with no property");
        assert!(reference_implements(NameSpace::ImageIndex, 505));
        assert!(reference_implements(NameSpace::ImageIndex, 170));
        assert!(!reference_implements(NameSpace::ImageIndex, 71));
        assert!(reference_implements(NameSpace::Float, 1), "a rate answers a float read");
        assert!(!reference_implements(NameSpace::Rate, 1102));
        assert!(reference_implements(NameSpace::Event, 99_999));
    }

    #[test]
    fn the_implemented_ids_are_as_many_as_the_reference_implements() {
        let implemented = |space: NameSpace| (0..PROPERTY_ID_LIMIT).filter(|id| reference_implements(space, *id)).count();
        assert_eq!(implemented(NameSpace::Boolean), REFERENCE_BOOLEANS);
        assert_eq!(implemented(NameSpace::Integer), REFERENCE_INTEGERS);
        assert_eq!(implemented(NameSpace::Rate), REFERENCE_RATES);
        assert_eq!(implemented(NameSpace::Float), REFERENCE_FLOATS + REFERENCE_RATES, "a float read reaches the rates too");
        assert_eq!(implemented(NameSpace::Text), REFERENCE_TEXTS);
    }

    #[test]
    fn only_a_few_rates_and_one_string_are_written_to() {
        assert!(reference_writes(NameSpace::Rate, 1), "the song list position is what its scroll bar drags");
        assert!(!reference_writes(NameSpace::Rate, 110), "a score rate is only read");
        assert!(reference_writes(NameSpace::Text, 30), "the search word is what the search box confirms");
        assert!(!reference_writes(NameSpace::Text, 10));
        assert!(!reference_writes(NameSpace::Float, 1), "a float read has no writer, even where it falls back to a rate");
    }

    #[test]
    fn stillness_follows_the_reference_classes() {
        assert_eq!(static_scope(40), StaticScope::OutsideSelect);
        assert_eq!(static_scope(-151), StaticScope::OutsideSelect, "the sign plays no part");
        assert_eq!(static_scope(300), StaticScope::OnResult);
        assert_eq!(static_scope(50), StaticScope::Always);
        assert_eq!(static_scope(OPTION_AUTOPLAYON), StaticScope::Never);
        assert_eq!(static_scope(OPTION_PRACTICE_ITEM1), StaticScope::Never);
        assert_eq!(static_scope(99_999), StaticScope::Never);

        assert!(!StaticScope::OutsideSelect.holds_on(StaticScreen::Select));
        assert!(StaticScope::OutsideSelect.holds_on(StaticScreen::Result));
        assert!(StaticScope::OnResult.holds_on(StaticScreen::Result));
        assert!(!StaticScope::OnResult.holds_on(StaticScreen::Other));
        assert!(StaticScope::Always.holds_on(StaticScreen::Select));
        assert!(!StaticScope::Never.holds_on(StaticScreen::Result));
    }
}
