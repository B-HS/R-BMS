//! Fixed words of what the application draws over a browser a skin draws, by language: the search box and the mark of a search on
//! show, and the mark of the course tab.
//!
//! The name of the course tab is the course screen's own ([`crate::text::menus::course`]).

rbms_model::translations! {
    /// Words of the overlays on a skin's browser.
    pub(crate) enum OverlayText {
        /// The search box's word before the query, and the first part of the mark of a search on show.
        Search => ("SEARCH", "검색", "検索"),
        /// How many charts a search found.
        Hits => ("{0} HITS", "결과 {0}개", "{0}件ヒット"),
        /// What the mark of a search on show says after the count, and what the mark of the course tab says after its name.
        SearchMarkHint => ("ESC CLEARS", "ESC로 해제", "ESCで解除"),
        CourseTabHint => ("SHIFT+TAB FOR SONGS", "SHIFT+TAB 곡 목록", "SHIFT+TAB 曲リスト"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(OverlayText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_overlays_drew_before_they_had_a_catalog() {
        assert_eq!(OverlayText::Search.get(Language::English), "SEARCH");
        assert_eq!(OverlayText::Hits.fill(Language::English, &[&12]), "12 HITS");
        assert_eq!(OverlayText::SearchMarkHint.get(Language::English), "ESC CLEARS");
        assert_eq!(OverlayText::CourseTabHint.get(Language::English), "SHIFT+TAB FOR SONGS");
    }

    #[test]
    fn the_hit_count_is_the_one_the_built_in_browser_says() {
        assert_eq!(OverlayText::Hits.fill(Language::Korean, &[&3]), "결과 3개");
        assert_eq!(OverlayText::Hits.fill(Language::Japanese, &[&3]), "3件ヒット");
    }
}
