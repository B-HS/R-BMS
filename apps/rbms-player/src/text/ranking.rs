//! Fixed words of the IR ranking overlay, by language.
//!
//! These are the words the overlay composes itself: the head's account of the player's own place,
//! and the lines that say how much of the leaderboard the list holds. The overlay's title, its
//! messages and the tags beside a row were written before the catalogs and stay as they were until
//! the select screen's words are moved. Names, dans, lamps and DJ LEVEL letters are data or terms
//! kept in every language, and are not here.

rbms_model::translations! {
    /// Words of the IR ranking overlay.
    pub(crate) enum RankingText {
        YouRank => ("YOU {0}"),
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
