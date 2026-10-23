//! What a row of the settings screen reads, written for the eye, by language.
//!
//! A value is stored and compared in the English the settings table gives it (`ON`, `(none)`, the
//! names a choice steps through), because the screen tells a switch that is on from one that is
//! off, and a choice's place among its values, by comparing them. What is drawn is this file's
//! rewriting of that value, which leaves it alone wherever the glossary keeps the term: `ON`,
//! `OFF`, `AUTO`, `DEFAULT`, the names of options, gauges and judgements, numbers and their units.

use rbms_config::{BUILT_IN_ONLY_NOTE, NONE_VALUE, SKIN_OPTION_NOTE, SettingId, SettingKind, UNSET_VALUE, descriptor};
use rbms_model::translations::Catalog;
use rbms_render::text::language;

use super::panel::PanelText;

rbms_model::translations! {
    /// The values of a choice that are plain words rather than terms of the game.
    pub(crate) enum ChoiceText {
        SortTitle => ("TITLE", "제목", "タイトル"),
        SortArtist => ("ARTIST", "아티스트", "アーティスト"),
        SortLength => ("LENGTH", "길이", "長さ"),
        SortLevel => ("LEVEL", "레벨", "レベル"),
        SortScore => ("SCORE", "스코어", "スコア"),
        SortLastUpdate => ("LAST UPDATE", "최종 갱신", "最終更新"),
        SideRight => ("RIGHT", "오른쪽", "右"),
        SideLeft => ("LEFT", "왼쪽", "左"),
    }
}

rbms_model::translations! {
    /// What a row reads when it holds nothing.
    pub(crate) enum EmptyValueText {
        Empty => ("(none)", "(없음)", "(なし)"),
        NotSet => ("(not set)", "(설정 안 됨)", "(未設定)"),
    }
}

/// The value `english` of the choice `id` is written as, or `english` itself where the glossary
/// keeps the term.
pub(crate) fn shown_choice(id: SettingId, english: &'static str) -> &'static str {
    let entry = match (id, english) {
        (SettingId::Sort, "TITLE") => ChoiceText::SortTitle,
        (SettingId::Sort, "ARTIST") => ChoiceText::SortArtist,
        (SettingId::Sort, "LENGTH") => ChoiceText::SortLength,
        (SettingId::Sort, "LEVEL") => ChoiceText::SortLevel,
        (SettingId::Sort, "SCORE") => ChoiceText::SortScore,
        (SettingId::Sort, "LAST UPDATE") => ChoiceText::SortLastUpdate,
        (SettingId::ScratchSide, "RIGHT") => ChoiceText::SideRight,
        (SettingId::ScratchSide, "LEFT") => ChoiceText::SideLeft,
        _ => return english,
    };
    entry.get(language())
}

/// What a value that belongs to no row of the table is drawn as, such as the value of a row a skin
/// document declares: the empty marks are written in the language, and anything else is a name the
/// skin or the program supplied and is drawn as it is.
pub(crate) fn shown_skin_value(value: &str) -> String {
    match value {
        NONE_VALUE => EmptyValueText::Empty.get(language()).to_string(),
        UNSET_VALUE => EmptyValueText::NotSet.get(language()).to_string(),
        _ => value.to_string(),
    }
}

/// What a row whose stored value is `value` is drawn as. The empty marks are written in the
/// language; a value of a choice is written by [`shown_choice`]; anything else is a number, a
/// name or a path the user or the program supplied, and is drawn as it is.
pub(crate) fn shown_value(id: SettingId, value: &str) -> String {
    if matches!(value, NONE_VALUE | UNSET_VALUE) {
        return shown_skin_value(value);
    }
    match descriptor(id).kind {
        SettingKind::Cycle { values } => {
            values.iter().find(|entry| **entry == value).map_or_else(|| value.to_string(), |entry| shown_choice(id, entry).to_string())
        }
        _ => value.to_string(),
    }
}

/// The note beside a row, which the table gives in English, written in the language. A note the
/// table does not know is drawn as it was given.
pub(crate) fn shown_note(note: &'static str) -> &'static str {
    match note {
        BUILT_IN_ONLY_NOTE => PanelText::BuiltInOnlyNote.get(language()),
        SKIN_OPTION_NOTE => PanelText::SkinOptionNote.get(language()),
        _ => note,
    }
}

#[cfg(test)]
mod tests {
    use rbms_config::cycle_values;
    use rbms_model::Language;
    use rbms_model::translations::catalog_problems;

    use super::*;
    use crate::text::settings::InstalledLanguage;

    #[test]
    fn every_catalog_is_written_in_every_language() {
        assert_eq!(catalog_problems(ChoiceText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(EmptyValueText::ALL), Vec::<String>::new());
    }

    #[test]
    fn a_translated_value_is_one_the_table_steps_through() {
        let _korean = InstalledLanguage::new(Language::Korean);
        for entry in ChoiceText::ALL {
            let english = entry.get(Language::English);
            let rows = [SettingId::Sort, SettingId::ScratchSide];
            let found =
                rows.iter().any(|id| cycle_values(*id).iter().any(|value| *value == english && shown_choice(*id, value) == entry.get(Language::Korean)));
            assert!(found, "{entry:?} names a value no row steps through");
        }
    }

    #[test]
    fn a_value_of_a_skin_row_is_drawn_as_it_is_but_for_the_empty_marks() {
        assert_eq!(shown_skin_value(NONE_VALUE), NONE_VALUE);
        assert_eq!(shown_skin_value("DEFAULT"), "DEFAULT");
        let _korean = InstalledLanguage::new(Language::Korean);
        assert_eq!(shown_skin_value(NONE_VALUE), "(없음)");
        assert_eq!(shown_skin_value(UNSET_VALUE), "(설정 안 됨)");
        assert_eq!(shown_skin_value("DEFAULT"), "DEFAULT");
        assert_eq!(shown_skin_value("PACK: ARCADE"), "PACK: ARCADE", "the name a skin gives an item is the skin's own");
    }

    #[test]
    fn the_marks_of_an_empty_row_are_the_ones_the_table_uses() {
        assert_eq!(EmptyValueText::Empty.get(Language::English), NONE_VALUE);
        assert_eq!(EmptyValueText::NotSet.get(Language::English), UNSET_VALUE);
    }

    #[test]
    fn a_value_is_drawn_as_the_table_says_it_in_english() {
        for id in SettingId::ALL {
            for value in cycle_values(id) {
                assert_eq!(shown_value(id, value), *value, "{id:?} {value}");
            }
        }
        assert_eq!(shown_value(SettingId::ServerUrl, NONE_VALUE), NONE_VALUE);
        assert_eq!(shown_value(SettingId::Password, UNSET_VALUE), UNSET_VALUE);
        assert_eq!(shown_note(BUILT_IN_ONLY_NOTE), BUILT_IN_ONLY_NOTE);
        assert_eq!(shown_note(SKIN_OPTION_NOTE), SKIN_OPTION_NOTE);
    }

    #[test]
    fn a_value_is_written_in_the_language_where_the_glossary_does_not_keep_it() {
        let _korean = InstalledLanguage::new(Language::Korean);
        assert_eq!(shown_value(SettingId::Sort, "TITLE"), "제목");
        assert_eq!(shown_value(SettingId::Sort, "BPM"), "BPM");
        assert_eq!(shown_value(SettingId::Sort, "MISS COUNT"), "MISS COUNT");
        assert_eq!(shown_value(SettingId::ScratchSide, "LEFT"), "왼쪽");
        assert_eq!(shown_value(SettingId::ServerUrl, NONE_VALUE), "(없음)");
        assert_eq!(shown_value(SettingId::Password, UNSET_VALUE), "(설정 안 됨)");
        assert_eq!(shown_value(SettingId::AudioDevice, "Built-in Output"), "Built-in Output", "a device is named by the system");
        assert_eq!(shown_value(SettingId::Random, "S-RANDOM"), "S-RANDOM", "the name of an option is kept");
        assert_eq!(shown_value(SettingId::MasterVolume, "55%"), "55%");
        assert_eq!(shown_note(BUILT_IN_ONLY_NOTE), "내장 화면 전용");
    }

    #[test]
    fn a_value_of_another_row_with_the_same_word_is_not_rewritten() {
        let _japanese = InstalledLanguage::new(Language::Japanese);
        assert_eq!(shown_value(SettingId::JudgeAlgorithm, "SCORE"), "SCORE", "SCORE names a judge algorithm there");
        assert_eq!(shown_value(SettingId::Sort, "SCORE"), "スコア");
    }
}
