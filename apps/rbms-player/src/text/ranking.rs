//! Fixed words of the IR ranking overlay, by language.
//!
//! These are the words the overlay composes itself: the head's account of the player's own place,
//! the messages it puts in place of rows, the label of a rival's row, the lines that say how much
//! of the leaderboard the list holds and the lines of keys under the title. The overlay's title
//! (`IR RANKING`), the tag beside a row that has a replay (`REP`) and the label of the player's own
//! row (`YOU`) are terms every language keeps, so they are written as they are where the overlay
//! is drawn. Names, dans, lamps and DJ LEVEL letters are data or terms kept in every language, and
//! are not here.

rbms_model::translations! {
    /// Words of the IR ranking overlay.
    pub(crate) enum RankingText {
        YouRank => ("YOU {0}"),
        /// The message shown instead of rows while the fetch is in flight.
        Loading => ("LOADING...", "불러오는 중...", "読み込み中..."),
        /// The message shown when the panel is open but no chart is focused.
        SelectAChart => ("SELECT A CHART", "차트를 선택하세요", "譜面を選んでください"),
        /// The message shown when the leaderboard came back empty.
        NoScoresYet => ("NO SCORES YET", "아직 스코어가 없습니다", "まだスコアがありません"),
        /// The message shown when no score server is set up, so there is nothing to fetch.
        Offline => (
            "IR OFF - SET SERVER URL IN SETTINGS",
            "IR OFF - 설정에서 SERVER URL을 입력하세요",
            "IR OFF - 設定でSERVER URLを入力してください",
        ),
        /// The message shown when the fetch failed; the value is the reason the server or the network gave.
        Error => ("ERROR: {0}", "오류: {0}", "エラー: {0}"),
        /// The label of the row of a rival: the rival's id.
        Rival => ("RIVAL {0}", "라이벌 {0}", "ライバル {0}"),
        /// The keys the panel is driven by while it has focus, with the profile switch and without it, and the one key that gives it focus.
        KeysWithProfiles => (
            "UP DOWN MOVE   Q/E PROFILE   ENTER REPLAY   I CLOSE",
            "UP DOWN 이동   Q/E 프로필   ENTER 리플레이   I 닫기",
            "UP DOWN 移動   Q/E プロファイル   ENTER リプレイ   I 閉じる",
        ),
        Keys => ("UP DOWN MOVE   ENTER REPLAY   I CLOSE", "UP DOWN 이동   ENTER 리플레이   I 닫기", "UP DOWN 移動   ENTER リプレイ   I 閉じる"),
        FocusPanel => ("I FOCUS PANEL", "I 패널 선택", "I パネルを選択"),
        TopShownWithMore => (
            "TOP {0} SHOWN, MORE ON THE NEXT PAGE",
            "상위 {0}명을 표시했습니다. 다음 페이지가 더 있습니다",
            "上位{0}人を表示しています。次のページに続きがあります",
        ),
        YourPageFrom => (
            "YOUR PAGE, FROM RANK {0}",
            "내 순위가 있는 페이지 ({0}위부터)",
            "自分の順位があるページ（{0}位から）",
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn the_catalog_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(RankingText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_overlay_drew_before_it_had_them_in_a_catalog() {
        let english = Language::English;
        assert_eq!(
            [RankingText::Loading, RankingText::SelectAChart, RankingText::NoScoresYet, RankingText::Offline].map(|entry| entry.get(english)),
            ["LOADING...", "SELECT A CHART", "NO SCORES YET", "IR OFF - SET SERVER URL IN SETTINGS"]
        );
        assert_eq!(RankingText::Error.fill(english, &[&"server down"]), "ERROR: server down");
        assert_eq!(RankingText::Rival.fill(english, &[&"friend"]), "RIVAL friend");
        assert_eq!(
            [RankingText::KeysWithProfiles, RankingText::Keys, RankingText::FocusPanel].map(|entry| entry.get(english)),
            ["UP DOWN MOVE   Q/E PROFILE   ENTER REPLAY   I CLOSE", "UP DOWN MOVE   ENTER REPLAY   I CLOSE", "I FOCUS PANEL"]
        );
    }

    #[test]
    fn a_rival_and_a_reason_are_put_where_each_language_wants_them() {
        assert_eq!(RankingText::Rival.fill(Language::Korean, &[&"friend"]), "라이벌 friend");
        assert_eq!(RankingText::Error.fill(Language::Japanese, &[&"server down"]), "エラー: server down");
    }

    #[test]
    fn the_head_keeps_the_term_you_in_every_language() {
        for language in Language::ALL {
            assert_eq!(RankingText::YouRank.fill(language, &[&"#7"]), "YOU #7", "{language:?}");
        }
    }

    #[test]
    fn a_number_is_put_where_each_language_wants_it() {
        assert_eq!(RankingText::YourPageFrom.fill(Language::English, &[&101]), "YOUR PAGE, FROM RANK 101");
        assert_eq!(RankingText::YourPageFrom.fill(Language::Korean, &[&101]), "내 순위가 있는 페이지 (101위부터)");
        assert_eq!(RankingText::TopShownWithMore.fill(Language::Japanese, &[&100]), "上位100人を表示しています。次のページに続きがあります");
    }
}
