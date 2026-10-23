//! Fixed words of the notices and the status lines the song select stage raises, by language.
//!
//! A notice is reported with [`crate::notify::notify_text`] and worded by the frame thread. A status line is set once and shown until
//! another replaces it, so it is worded when it is set ([`crate::text::notices::installed`]). What the operating system, a file or the
//! score server said about a failure is data and comes in as a value.

rbms_model::translations! {
    /// Words of the notices and status lines of the song select stage.
    pub(crate) enum SelectNoticeText {
        /// A course is not started: this many of its stages are charts the library does not hold.
        StagesMissing => (
            "{0} stage(s) missing from the library",
            "라이브러리에 없는 스테이지가 {0}개 있습니다",
            "ライブラリにないステージが{0}件あります",
        ),
        /// A saved replay of a record could not be read; the value is the reason.
        ReplayLoadFailed => ("replay load failed: {0}", "리플레이를 불러오지 못했습니다: {0}", "リプレイを読み込めませんでした: {0}"),

        /// The status line over the replay of a row of the ranking panel.
        NoReplayForThatRow => ("no replay for that row", "해당 줄에는 리플레이가 없습니다", "その行にはリプレイがありません"),
        DownloadingReplay => ("downloading replay {0}...", "리플레이 {0} 다운로드 중...", "リプレイ {0} をダウンロード中..."),
        DownloadedReplayHasNoInputs => (
            "downloaded replay has no inputs",
            "다운로드한 리플레이에 입력이 없습니다",
            "ダウンロードしたリプレイに入力がありません",
        ),
        ReplayDownloaded => ("replay downloaded", "리플레이를 다운로드했습니다", "リプレイをダウンロードしました"),
        ReplayDownloadFailed => ("replay download: {0}", "리플레이 다운로드: {0}", "リプレイのダウンロード: {0}"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(SelectNoticeText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_stage_said_before_it_had_a_catalog() {
        let table: Vec<(SelectNoticeText, Vec<&dyn std::fmt::Display>, &str)> = vec![
            (SelectNoticeText::StagesMissing, vec![&2], "2 stage(s) missing from the library"),
            (SelectNoticeText::ReplayLoadFailed, vec![&"gone"], "replay load failed: gone"),
            (SelectNoticeText::NoReplayForThatRow, vec![], "no replay for that row"),
            (SelectNoticeText::DownloadingReplay, vec![&"r-1"], "downloading replay r-1..."),
            (SelectNoticeText::DownloadedReplayHasNoInputs, vec![], "downloaded replay has no inputs"),
            (SelectNoticeText::ReplayDownloaded, vec![], "replay downloaded"),
            (SelectNoticeText::ReplayDownloadFailed, vec![&"offline"], "replay download: offline"),
        ];
        assert_eq!(table.len(), SelectNoticeText::ALL.len(), "an entry has no row in this table");
        for (text, args, expected) in table {
            assert_eq!(text.fill(Language::English, &args), expected, "{text:?}");
        }
    }

    #[test]
    fn a_value_is_put_where_each_language_wants_it() {
        assert_eq!(SelectNoticeText::StagesMissing.fill(Language::Korean, &[&2]), "라이브러리에 없는 스테이지가 2개 있습니다");
        assert_eq!(SelectNoticeText::DownloadingReplay.fill(Language::Japanese, &[&"r-1"]), "リプレイ r-1 をダウンロード中...");
    }
}
