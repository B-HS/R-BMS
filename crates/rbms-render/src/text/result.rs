//! Fixed words of the built-in result screen and its three graph panels, by language.
//!
//! The judgement rows' labels, the clear lamp and the grade letters come from the palette and the
//! view. Terms every language keeps as they are (`EX SCORE`, `MAX COMBO`, `FAST`, `SLOW`, `TARGET`,
//! the grade letters) stay in the drawing code.

rbms_model::translations! {
    /// Words of the built-in result screen and its graph panels.
    pub(crate) enum ResultText {
        /// How far the run's EX score is from the chart's best before it: the signed difference.
        VsBest => ("{0} vs BEST", "최고 기록 대비 {0}", "自己ベスト比 {0}"),
        /// The run beat the chart's best before it.
        NewRecord => ("NEW RECORD", "신기록", "自己ベスト更新"),
        /// The chart had not been played before this run.
        FirstPlay => ("FIRST PLAY", "첫 플레이", "初プレー"),
        /// How far the run's EX score is from the play before it: the signed difference.
        VsPrev => ("{0} vs PREV", "이전 기록 대비 {0}", "前回比 {0}"),
        /// The number of notes the chart has, in the run's report.
        TotalNotes => ("TOTAL NOTES", "총 노트 수", "総ノーツ数"),
        /// The gauge the run ended on, in percent, under the judgement rows.
        GaugeValue => ("GAUGE {0}%", "게이지 {0}%", "ゲージ {0}%"),
        /// The keys of a result that cannot be run again.
        HintSelect => ("ENTER / ESC  SELECT", "ENTER / ESC  선곡", "ENTER / ESC  選曲"),
        /// The keys of a result that can be retried or followed on: each key, then what it does.
        HintSelectRetryNext => (
            "ENTER / ESC  SELECT    R  RETRY    N  NEXT SONG",
            "ENTER / ESC  선곡    R  재도전    N  다음 곡",
            "ENTER / ESC  選曲    R  リトライ    N  次の曲"
        ),
        /// The title of the panel plotting the gauge through the run.
        GraphGauge => ("GAUGE", "게이지", "ゲージ"),
        /// The title of the panel plotting where the inputs landed in time.
        GraphTiming => ("TIMING", "타이밍", "タイミング"),
        /// The title of the panel plotting the share of each judgement.
        GraphJudge => ("JUDGE", "판정", "判定"),
        /// A panel whose measurement is empty.
        NoData => ("NO DATA", "데이터 없음", "データなし"),
        /// The timing plot's left end: inputs that came before the note.
        Early => ("EARLY", "빠름", "早い"),
        /// The timing plot's right end: inputs that came after the note.
        Late => ("LATE", "느림", "遅い"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(ResultText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_screen_drew_before_it_had_a_catalog() {
        let english = Language::English;
        assert_eq!(ResultText::VsBest.fill(english, &[&"+12"]), "+12 vs BEST");
        assert_eq!(ResultText::VsPrev.fill(english, &[&"-7"]), "-7 vs PREV");
        assert_eq!(
            [ResultText::NewRecord, ResultText::FirstPlay, ResultText::TotalNotes].map(|entry| entry.get(english)),
            ["NEW RECORD", "FIRST PLAY", "TOTAL NOTES"]
        );
        assert_eq!(ResultText::GaugeValue.fill(english, &[&88]), "GAUGE 88%");
        assert_eq!(ResultText::HintSelect.get(english), "ENTER / ESC  SELECT");
        assert_eq!(ResultText::HintSelectRetryNext.get(english), "ENTER / ESC  SELECT    R  RETRY    N  NEXT SONG");
        assert_eq!(
            [ResultText::GraphGauge, ResultText::GraphTiming, ResultText::GraphJudge, ResultText::NoData, ResultText::Early, ResultText::Late]
                .map(|entry| entry.get(english)),
            ["GAUGE", "TIMING", "JUDGE", "NO DATA", "EARLY", "LATE"]
        );
    }

    #[test]
    fn the_keys_of_a_hint_are_the_same_in_every_language() {
        for language in Language::ALL {
            let hint = ResultText::HintSelectRetryNext.get(language);
            for key in ["ENTER / ESC  ", "  R  ", "  N  "] {
                assert!(hint.contains(key), "{language:?}: the hint lost the key {key:?}: {hint:?}");
            }
        }
    }

    #[test]
    fn the_signed_difference_is_put_where_each_language_wants_it() {
        assert_eq!(ResultText::VsBest.fill(Language::Korean, &[&"+12"]), "최고 기록 대비 +12");
        assert_eq!(ResultText::VsPrev.fill(Language::Japanese, &[&"-7"]), "前回比 -7");
    }
}
