//! Fixed words of the built-in play HUD, by language.
//!
//! The judgement labels come from the skin. Terms every language keeps as they are (`EX`, `GREEN`,
//! `WHITE`, `FAST`, `SLOW`, the grade letters on the score graph) stay in the drawing code.

rbms_model::translations! {
    /// Words of the built-in play HUD.
    pub(crate) enum HudText {
        /// The chart's best EX score so far, in the column on the left: the score.
        BestLine => ("BEST {0}", "최고 {0}", "ベスト {0}"),
        /// How far the run's EX score is from the best, over the score graph: the signed difference.
        VsBest => ("vs BEST {0}", "최고 기록 대비 {0}", "自己ベスト比 {0}"),
        /// The label of the score graph's bar for the run so far.
        Now => ("NOW", "현재", "現在"),
        /// The label of the score graph's bar for the chart's best.
        Best => ("BEST", "최고", "ベスト"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(HudText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_hud_drew_before_it_had_a_catalog() {
        let english = Language::English;
        assert_eq!(HudText::BestLine.fill(english, &[&1234]), "BEST 1234");
        assert_eq!(HudText::VsBest.fill(english, &[&"+12"]), "vs BEST +12");
        assert_eq!([HudText::Now, HudText::Best].map(|entry| entry.get(english)), ["NOW", "BEST"]);
    }
}
