//! Fixed words of the browser's filter panel and of the line that says what the filter takes out, by language.
//!
//! The play modes (`7K`), the level prefix `LV` and the lamp names the glossary keeps (`NO PLAY`, `FULL COMBO`) are written as they are.

rbms_model::translations! {
    /// Words of the filter panel.
    pub(crate) enum FilterText {
        Title => ("FILTER", "필터", "フィルター"),
        Hint => (
            "\u{2191}\u{2193} AXIS   \u{2190}\u{2192} VALUE   BACKSPACE RESET   F2 CLOSE",
            "\u{2191}\u{2193} 항목   \u{2190}\u{2192} 값   BACKSPACE 초기화   F2 닫기",
            "\u{2191}\u{2193} 項目   \u{2190}\u{2192} 値   BACKSPACE リセット   F2 閉じる",
        ),

        /// The names of the axes the panel edits.
        LevelFrom => ("LEVEL FROM", "레벨 하한", "レベル下限"),
        LevelTo => ("LEVEL TO", "레벨 상한", "レベル上限"),
        Mode => ("MODE", "모드", "モード"),
        Clear => ("CLEAR", "클리어", "クリア"),
        Favourites => ("FAVOURITES", "즐겨찾기", "お気に入り"),

        /// What an axis reads while it takes nothing out: every value, or no bound on a level.
        All => ("ALL", "전체", "すべて"),
        AnyLevel => ("ANY", "제한 없음", "指定なし"),
        StarredOnly => ("STARRED ONLY", "즐겨찾기만", "お気に入りのみ"),

        /// The lamp filters the CLEAR axis steps through.
        NoPlay => ("NO PLAY"),
        NotCleared => ("NOT CLEARED", "미클리어", "未クリア"),
        Cleared => ("CLEARED", "클리어 완료", "クリア済み"),
        FullCombo => ("FULL COMBO"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(FilterText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_panel_drew_before_it_had_a_catalog() {
        let table = [
            (FilterText::Title, "FILTER"),
            (FilterText::Hint, "\u{2191}\u{2193} AXIS   \u{2190}\u{2192} VALUE   BACKSPACE RESET   F2 CLOSE"),
            (FilterText::LevelFrom, "LEVEL FROM"),
            (FilterText::LevelTo, "LEVEL TO"),
            (FilterText::Mode, "MODE"),
            (FilterText::Clear, "CLEAR"),
            (FilterText::Favourites, "FAVOURITES"),
            (FilterText::All, "ALL"),
            (FilterText::AnyLevel, "ANY"),
            (FilterText::StarredOnly, "STARRED ONLY"),
            (FilterText::NoPlay, "NO PLAY"),
            (FilterText::NotCleared, "NOT CLEARED"),
            (FilterText::Cleared, "CLEARED"),
            (FilterText::FullCombo, "FULL COMBO"),
        ];
        assert_eq!(table.len(), FilterText::ALL.len(), "an entry has no row in this table");
        for (text, expected) in table {
            assert_eq!(text.get(Language::English), expected, "{text:?}");
        }
    }

    #[test]
    fn the_lamp_names_the_glossary_keeps_are_the_same_in_every_language() {
        for language in Language::ALL {
            assert_eq!(FilterText::NoPlay.get(language), "NO PLAY", "{language:?}");
            assert_eq!(FilterText::FullCombo.get(language), "FULL COMBO", "{language:?}");
        }
    }
}
