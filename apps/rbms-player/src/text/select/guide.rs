//! Fixed words of the key guide a skin's browser can put up, by language.
//!
//! The keys are read from the key configuration as the guide is drawn and are not here. The terms every language keeps as they are
//! (HI-SPEED, LIFT, HIDDEN+, GREEN NUMBER, KEEP GREEN NUMBER, ON/OFF, MS, IR RANKING) are written as they are, inside a translated line
//! where the line has other words and alone where it has none. The names of the shortcuts are the ones the key editor gives them, said
//! in the language here because the key editor's own list is not worded by language.

rbms_model::translations! {
    /// Words of the key guide.
    pub(crate) enum GuideText {
        Title => ("R-BMS KEYS", "R-BMS 키", "R-BMSキー"),
        SectionSelect => ("SELECT", "선곡", "選曲"),
        SectionScreens => ("SCREENS", "화면", "画面"),
        SectionNumberKeys => ("NUMBER KEYS", "숫자 키", "数字キー"),
        SectionOptionPanels => ("OPTION PANELS", "옵션 패널", "オプションパネル"),
        SectionKeysAndTurntable => ("KEYS 1-7 AND THE TURNTABLE", "건반 1-7 및 턴테이블", "鍵盤1-7とターンテーブル"),
        SectionInPlay => ("IN PLAY", "플레이 중", "プレー中"),
        SectionInPlayTogether => ("IN PLAY, HELD TOGETHER", "플레이 중, 함께 누르기", "プレー中、同時押し"),

        /// The line above the guide's columns: the key that closes it, what is fixed and where the rest is set.
        Hint => (
            "{0} OR ESC CLOSES     EVERY KEY BUT THE ARROWS, ENTER, ESC AND {1} IS SET IN KEY CONFIG ({2})",
            "{0} 또는 ESC로 닫기     방향키, ENTER, ESC, {1} 외의 모든 키는 키 설정 ({2})에서 정합니다",
            "{0}またはESCで閉じる     矢印キー、ENTER、ESC、{1}以外のキーはキー設定（{2}）で決めます",
        ),

        MoveTheList => ("MOVE THE LIST", "목록 이동", "リストを移動"),
        OpenFolderOrPlay => ("OPEN A FOLDER / PLAY", "폴더 열기 / 플레이", "フォルダを開く / プレー"),
        BackTwiceQuits => ("BACK - TWICE QUITS", "뒤로 - 두 번 누르면 종료", "戻る - 2回押すと終了"),
        OptionsOverlay => ("OPTIONS OVERLAY", "옵션 오버레이", "オプションオーバーレイ"),
        ThisGuide => ("THIS GUIDE", "이 안내", "このガイド"),

        /// What the shortcuts say where the guide's line is shorter than the key editor's row.
        ShortcutSort => ("SORT (SHIFT: BACKWARDS)", "정렬 (SHIFT: 역순)", "ソート（SHIFT：逆順）"),
        ShortcutRecords => ("RECORDS", "기록", "記録"),
        ShortcutSettings => ("SETTINGS (SHIFT: COURSES)", "설정 (SHIFT: 코스)", "設定（SHIFT：コース）"),
        ShortcutDetailOptions => ("HOLD: DETAIL OPTIONS", "누르고 있기: 상세 옵션", "長押し：詳細オプション"),

        /// What the other shortcuts the guide lists are called.
        Search => ("SEARCH", "검색", "検索"),
        FilterPanel => ("FILTER PANEL", "필터 패널", "フィルターパネル"),
        Practice => ("PRACTICE", "연습", "練習"),
        Favourite => ("FAVOURITE", "즐겨찾기", "お気に入り"),
        IrRanking => ("IR RANKING"),
        ReloadTables => ("RELOAD DIFFICULTY TABLES", "난이도표 다시 받기", "難易度表を再取得"),
        SkinMenu => ("SKIN MENU", "스킨 메뉴", "スキンメニュー"),
        PlayMenu => ("PLAY MENU", "플레이 메뉴", "プレーメニュー"),
        KeyConfig => ("KEY CONFIG", "키 설정", "キー設定"),
        MusicFolders => ("MUSIC FOLDERS", "곡 폴더", "曲フォルダ"),
        DifficultyTables => ("DIFFICULTY TABLES", "난이도표", "難易度表"),
        ModeFilter => ("MODE FILTER", "모드 필터", "モードフィルター"),
        SortOrder => ("SORT ORDER", "정렬 순서", "ソート順"),
        LongNoteMode => ("LONG NOTE MODE", "롱노트 모드", "ロングノートモード"),
        ReplaySlot => ("REPLAY SLOT", "리플레이 슬롯", "リプレイスロット"),

        /// What each role of the reference's key table does.
        PlayTheChart => ("PLAY THE CHART", "차트 플레이", "譜面をプレー"),
        PlayItByItself => ("PLAY IT BY ITSELF", "자동으로 플레이", "自動でプレー"),
        OpenAFolder => ("OPEN A FOLDER", "폴더 열기", "フォルダを開く"),
        BackOneFolder => ("BACK ONE FOLDER", "폴더 하나 뒤로", "フォルダを1つ戻る"),
        NextBar => ("NEXT BAR", "다음 막대", "次のバー"),
        PreviousBar => ("PREVIOUS BAR", "이전 막대", "前のバー"),

        /// The controls of a run that come as a pair of keys.
        HiSpeedDownUp => ("HI-SPEED DOWN / UP", "HI-SPEED 내리기 / 올리기", "HI-SPEED下げる / 上げる"),
        LaneCoverLessMore => ("LANE COVER LESS / MORE", "레인 커버 줄이기 / 늘리기", "レーンカバー減らす / 増やす"),
        LiftDownUp => ("LIFT DOWN / UP", "LIFT 내리기 / 올리기", "LIFT下げる / 上げる"),
        HiddenLessMore => ("HIDDEN+ LESS / MORE", "HIDDEN+ 줄이기 / 늘리기", "HIDDEN+減らす / 増やす"),
        GreenNumberDownUp => ("GREEN NUMBER DOWN / UP", "GREEN NUMBER 내리기 / 올리기", "GREEN NUMBER下げる / 上げる"),
        JudgeOffsetHold => ("JUDGE OFFSET -1 / +1 MS (HOLD)", "판정 오프셋 -1 / +1 MS (누르고 있기)", "判定オフセット -1 / +1 MS（長押し）"),

        /// The controls of a run that are one key each.
        LaneCoverOnOff => ("LANE COVER ON/OFF", "레인 커버 ON/OFF", "レーンカバーON/OFF"),
        LiftOnOff => ("LIFT ON/OFF"),
        HiddenOnOff => ("HIDDEN+ ON/OFF"),
        KeepGreenNumber => ("KEEP GREEN NUMBER"),
        AutoAdjust => ("AUTO ADJUST", "자동 보정", "自動補正"),

        /// The modifier held for the finer step, and what it is called while it has no key of its own.
        HoldFineStep => ("HOLD: FINE STEP", "누르고 있기: 미세 조정", "長押し：微調整"),
        ShiftNoLane => ("SHIFT (NO LANE)", "SHIFT (레인 없음)", "SHIFT（レーンなし）"),

        /// The line that names the turntable's two keys.
        TurntableDownUp => ("THE TURNTABLE: DOWN / UP", "턴테이블: 내리기 / 올리기", "ターンテーブル：下げる / 上げる"),

        /// What the keys held for each option panel open.
        HoldForPlayOptions => ("HOLD FOR THE PLAY OPTIONS", "누르고 있으면 플레이 옵션", "長押しでプレーオプション"),
        HoldForAssistOptions => ("HOLD FOR THE ASSIST OPTIONS", "누르고 있으면 어시스트 옵션", "長押しでアシストオプション"),
        HoldForDetailOptions => ("HOLD FOR THE DETAIL OPTIONS", "누르고 있으면 상세 옵션", "長押しで詳細オプション"),
        Target => ("TARGET", "타깃", "ターゲット"),
        JudgeAlgorithm => ("JUDGE ALGORITHM", "판정 알고리즘", "判定アルゴリズム"),

        /// A line's keys with the word that names the lane keys or the turntable, the first key or two first.
        PlusTurntable => ("{0} + TURNTABLE", "{0} + 턴테이블", "{0} + ターンテーブル"),
        PlusKeys => ("{0} + KEYS", "{0} + 건반", "{0} + 鍵盤"),
        Twice => ("{0} TWICE", "{0} 두 번", "{0} 2回"),
        MouseWheel => ("MOUSE WHEEL", "마우스 휠", "マウスホイール"),

        /// What the keys held together do in a run.
        HiSpeed => ("HI-SPEED"),
        LaneCoverByOneGreenKept => ("LANE COVER BY 1, GREEN KEPT", "레인 커버 1씩, GREEN NUMBER 유지", "レーンカバー1ずつ、GREEN NUMBER維持"),
        LiftByOne => ("LIFT BY 1", "LIFT 1씩", "LIFT1ずつ"),
        GreenNumberByOneMs => ("GREEN NUMBER BY 1 MS", "GREEN NUMBER 1 MS씩", "GREEN NUMBER1 MSずつ"),
        LiftOrHiddenTurns => ("LIFT OR HIDDEN+ TURNS", "LIFT/HIDDEN+ 조절 대상 전환", "LIFT/HIDDEN+の調整対象を切り替え"),
        LaneCover => ("LANE COVER", "레인 커버", "レーンカバー"),
        StopTheChart => ("STOP THE CHART", "차트 중지", "譜面を停止"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(GuideText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_guide_drew_before_it_had_a_catalog() {
        let english = Language::English;
        let table: Vec<(GuideText, Vec<&dyn std::fmt::Display>, &str)> = vec![
            (GuideText::Title, vec![], "R-BMS KEYS"),
            (GuideText::SectionSelect, vec![], "SELECT"),
            (GuideText::SectionScreens, vec![], "SCREENS"),
            (GuideText::SectionNumberKeys, vec![], "NUMBER KEYS"),
            (GuideText::SectionOptionPanels, vec![], "OPTION PANELS"),
            (GuideText::SectionKeysAndTurntable, vec![], "KEYS 1-7 AND THE TURNTABLE"),
            (GuideText::SectionInPlay, vec![], "IN PLAY"),
            (GuideText::SectionInPlayTogether, vec![], "IN PLAY, HELD TOGETHER"),
            (GuideText::Hint, vec![&"H", &"H", &"F5"], "H OR ESC CLOSES     EVERY KEY BUT THE ARROWS, ENTER, ESC AND H IS SET IN KEY CONFIG (F5)"),
            (GuideText::MoveTheList, vec![], "MOVE THE LIST"),
            (GuideText::OpenFolderOrPlay, vec![], "OPEN A FOLDER / PLAY"),
            (GuideText::BackTwiceQuits, vec![], "BACK - TWICE QUITS"),
            (GuideText::OptionsOverlay, vec![], "OPTIONS OVERLAY"),
            (GuideText::ThisGuide, vec![], "THIS GUIDE"),
            (GuideText::ShortcutSort, vec![], "SORT (SHIFT: BACKWARDS)"),
            (GuideText::ShortcutRecords, vec![], "RECORDS"),
            (GuideText::ShortcutSettings, vec![], "SETTINGS (SHIFT: COURSES)"),
            (GuideText::ShortcutDetailOptions, vec![], "HOLD: DETAIL OPTIONS"),
            (GuideText::Search, vec![], "SEARCH"),
            (GuideText::FilterPanel, vec![], "FILTER PANEL"),
            (GuideText::Practice, vec![], "PRACTICE"),
            (GuideText::Favourite, vec![], "FAVOURITE"),
            (GuideText::IrRanking, vec![], "IR RANKING"),
            (GuideText::ReloadTables, vec![], "RELOAD DIFFICULTY TABLES"),
            (GuideText::SkinMenu, vec![], "SKIN MENU"),
            (GuideText::PlayMenu, vec![], "PLAY MENU"),
            (GuideText::KeyConfig, vec![], "KEY CONFIG"),
            (GuideText::MusicFolders, vec![], "MUSIC FOLDERS"),
            (GuideText::DifficultyTables, vec![], "DIFFICULTY TABLES"),
            (GuideText::ModeFilter, vec![], "MODE FILTER"),
            (GuideText::SortOrder, vec![], "SORT ORDER"),
            (GuideText::LongNoteMode, vec![], "LONG NOTE MODE"),
            (GuideText::ReplaySlot, vec![], "REPLAY SLOT"),
            (GuideText::PlayTheChart, vec![], "PLAY THE CHART"),
            (GuideText::PlayItByItself, vec![], "PLAY IT BY ITSELF"),
            (GuideText::OpenAFolder, vec![], "OPEN A FOLDER"),
            (GuideText::BackOneFolder, vec![], "BACK ONE FOLDER"),
            (GuideText::NextBar, vec![], "NEXT BAR"),
            (GuideText::PreviousBar, vec![], "PREVIOUS BAR"),
            (GuideText::HiSpeedDownUp, vec![], "HI-SPEED DOWN / UP"),
            (GuideText::LaneCoverLessMore, vec![], "LANE COVER LESS / MORE"),
            (GuideText::LiftDownUp, vec![], "LIFT DOWN / UP"),
            (GuideText::HiddenLessMore, vec![], "HIDDEN+ LESS / MORE"),
            (GuideText::GreenNumberDownUp, vec![], "GREEN NUMBER DOWN / UP"),
            (GuideText::JudgeOffsetHold, vec![], "JUDGE OFFSET -1 / +1 MS (HOLD)"),
            (GuideText::LaneCoverOnOff, vec![], "LANE COVER ON/OFF"),
            (GuideText::LiftOnOff, vec![], "LIFT ON/OFF"),
            (GuideText::HiddenOnOff, vec![], "HIDDEN+ ON/OFF"),
            (GuideText::KeepGreenNumber, vec![], "KEEP GREEN NUMBER"),
            (GuideText::AutoAdjust, vec![], "AUTO ADJUST"),
            (GuideText::HoldFineStep, vec![], "HOLD: FINE STEP"),
            (GuideText::ShiftNoLane, vec![], "SHIFT (NO LANE)"),
            (GuideText::TurntableDownUp, vec![], "THE TURNTABLE: DOWN / UP"),
            (GuideText::HoldForPlayOptions, vec![], "HOLD FOR THE PLAY OPTIONS"),
            (GuideText::HoldForAssistOptions, vec![], "HOLD FOR THE ASSIST OPTIONS"),
            (GuideText::HoldForDetailOptions, vec![], "HOLD FOR THE DETAIL OPTIONS"),
            (GuideText::Target, vec![], "TARGET"),
            (GuideText::JudgeAlgorithm, vec![], "JUDGE ALGORITHM"),
            (GuideText::PlusTurntable, vec![&"F1"], "F1 + TURNTABLE"),
            (GuideText::PlusKeys, vec![&"F1"], "F1 + KEYS"),
            (GuideText::Twice, vec![&"F1"], "F1 TWICE"),
            (GuideText::MouseWheel, vec![], "MOUSE WHEEL"),
            (GuideText::HiSpeed, vec![], "HI-SPEED"),
            (GuideText::LaneCoverByOneGreenKept, vec![], "LANE COVER BY 1, GREEN KEPT"),
            (GuideText::LiftByOne, vec![], "LIFT BY 1"),
            (GuideText::GreenNumberByOneMs, vec![], "GREEN NUMBER BY 1 MS"),
            (GuideText::LiftOrHiddenTurns, vec![], "LIFT OR HIDDEN+ TURNS"),
            (GuideText::LaneCover, vec![], "LANE COVER"),
            (GuideText::StopTheChart, vec![], "STOP THE CHART"),
        ];
        assert_eq!(table.len(), GuideText::ALL.len(), "an entry has no row in this table");
        for (text, args, expected) in table {
            assert_eq!(text.fill(english, &args), expected, "{text:?}");
        }
    }

    #[test]
    fn a_key_is_put_where_each_language_wants_it() {
        assert_eq!(GuideText::PlusKeys.fill(Language::Korean, &[&"F1"]), "F1 + 건반");
        assert_eq!(GuideText::Twice.fill(Language::Japanese, &[&"F1"]), "F1 2回");
    }

    #[test]
    fn the_terms_every_language_keeps_stay_inside_the_translated_lines() {
        for language in Language::ALL {
            assert!(GuideText::HiSpeedDownUp.get(language).starts_with("HI-SPEED"), "{language:?}");
            assert!(GuideText::GreenNumberDownUp.get(language).starts_with("GREEN NUMBER"), "{language:?}");
            assert!(GuideText::JudgeOffsetHold.get(language).contains("-1 / +1 MS"), "{language:?}");
            assert_eq!(GuideText::LiftOnOff.get(language), "LIFT ON/OFF");
            assert_eq!(GuideText::IrRanking.get(language), "IR RANKING");
        }
    }
}
