//! Fixed words of the built-in song select, by language: its head, its buttons, the list and the
//! detail panel, and the record modal.
//!
//! The words a chart, a folder or the application supplies are not here: the sort order and the
//! filter summary, the guide line, the onboarding hint, the stat labels and the difficulty name
//! arrive in the view already written. Terms every language keeps as they are (judgements, `EX`,
//! `BP`, `COMBO`, `Lv`, key names) stay in the drawing code.

rbms_model::translations! {
    /// Words of the built-in song select.
    pub(crate) enum SelectText {
        /// The button that plays the chart laid out as the shown record's replay was.
        PlayThisLayout => ("PLAY THIS LAYOUT", "이 배치로 플레이", "この配置でプレー"),
        /// The key that runs the button, in the line of keys under the record: the key first.
        ModalLayoutKey => ("J  PLAY THIS LAYOUT", "J  이 배치로 플레이", "J  この配置でプレー"),
        /// The title of the screen, top left.
        Title => ("MUSIC SELECT", "선곡", "選曲"),
        /// How many rows the open search matched, beside the search box.
        SearchHits => ("{0} HITS", "결과 {0}개", "{0}件ヒット"),
        /// The sort order, top right and on the sort button: the order's name.
        SortLine => ("SORT: {0}", "정렬: {0}", "ソート：{0}"),
        /// The sort order and what the filter takes out, top right: the order's name, then the summary.
        SortAndFilter => ("SORT: {0}   FILTER: {1}", "정렬: {0}   필터: {1}", "ソート：{0}   フィルター：{1}"),
        /// The search button, and the search box's word once it is open.
        Search => ("SEARCH", "검색", "検索"),
        /// The button that opens the folder list.
        Folders => ("FOLDERS", "폴더", "フォルダ"),
        /// The button that opens the difficulty tables.
        Tables => ("TABLES", "난이도표", "難易度表"),
        /// The records button, and the heading of the records in the detail panel.
        Records => ("RECORDS", "기록", "記録"),
        /// The button that opens the settings.
        Settings => ("SETTINGS", "설정", "設定"),
        /// The list when there is nothing to list.
        NoCharts => ("NO CHARTS", "차트 없음", "譜面なし"),
        /// How many charts a folder holds, on its row and in its detail panel.
        ChartCount => ("{0} CHARTS", "차트 {0}개", "{0}譜面"),
        /// The cover's place while its image is being read.
        Loading => ("LOADING", "불러오는 중", "読み込み中"),
        /// The cover's place when the chart has no image.
        NoImage => ("NO IMAGE", "이미지 없음", "画像なし"),
        /// The heading of the note-density histogram.
        Density => ("DENSITY", "노트 밀도", "ノーツ密度"),
        /// The histogram's three figures, notes a second: the busiest second, the average and the
        /// end of the chart.
        DensityLine => ("PEAK {0}  AVG {1}  END {2}  /s", "최대 {0}  평균 {1}  끝 {2}  /초", "最大{0}  平均{1}  終盤{2}  /秒"),
        /// The chart's clears and plays, beside the records heading.
        ClearAndPlays => ("{0} CLEAR / {1} PLAYS", "클리어 {0} / 플레이 {1}", "クリア{0} / プレー{1}"),
        /// The records panel of a chart nobody has played.
        NoPlayYet => ("NO PLAY YET", "아직 플레이하지 않음", "未プレー"),
        /// The DJ rank of the best record, beside its rank bar: the rank's letters.
        RankLine => ("RANK {0}", "랭크 {0}", "ランク{0}"),
        /// The title of a folder's detail panel.
        FolderTitle => ("FOLDER", "폴더", "フォルダ"),
        /// What the detail panel says under a chart a table lists that the library does not hold.
        MissingChartNotice => (
            "NOT IN THE LIBRARY - ENTER FOR THE DOWNLOAD",
            "라이브러리에 없음 - ENTER 키로 다운로드",
            "ライブラリにありません - ENTERでダウンロード"
        ),
        /// The record modal's gauge line: the gauge the run ended on, in percent.
        ModalGauge => ("GAUGE  {0}%", "게이지  {0}%", "ゲージ  {0}%"),
        /// The record modal's button that plays the record's replay back.
        PlayReplay => ("PLAY REPLAY", "리플레이 재생", "リプレイ再生"),
        /// The record modal's word for a record that kept no replay.
        NoReplaySaved => ("NO REPLAY SAVED", "저장된 리플레이 없음", "保存されたリプレイなし"),
        /// The record modal's line of keys: each key, then what it does.
        ModalKeys => (
            "UP DOWN PREV/NEXT   ENTER REPLAY   ESC CLOSE",
            "UP DOWN 이전/다음   ENTER 리플레이   ESC 닫기",
            "UP DOWN 前へ/次へ   ENTER リプレイ   ESC 閉じる"
        ),
        /// The record modal's close button.
        Close => ("CLOSE", "닫기", "閉じる"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(SelectText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_screen_drew_before_it_had_a_catalog() {
        let english = Language::English;
        assert_eq!(SelectText::Title.get(english), "MUSIC SELECT");
        assert_eq!(SelectText::SearchHits.fill(english, &[&12]), "12 HITS");
        assert_eq!(SelectText::SortLine.fill(english, &[&"TITLE"]), "SORT: TITLE");
        assert_eq!(SelectText::SortAndFilter.fill(english, &[&"TITLE", &"LV 10\u{2013}12  FAVOURITES"]), "SORT: TITLE   FILTER: LV 10\u{2013}12  FAVOURITES");
        assert_eq!(
            [SelectText::Search, SelectText::Folders, SelectText::Tables, SelectText::Records, SelectText::Settings].map(|entry| entry.get(english)),
            ["SEARCH", "FOLDERS", "TABLES", "RECORDS", "SETTINGS"]
        );
        assert_eq!(SelectText::NoCharts.get(english), "NO CHARTS");
        assert_eq!(SelectText::ChartCount.fill(english, &[&86]), "86 CHARTS");
        assert_eq!([SelectText::Loading, SelectText::NoImage, SelectText::Density].map(|entry| entry.get(english)), ["LOADING", "NO IMAGE", "DENSITY"]);
        assert_eq!(SelectText::DensityLine.fill(english, &[&17, &9, &11]), "PEAK 17  AVG 9  END 11  /s");
        assert_eq!(SelectText::ClearAndPlays.fill(english, &[&8, &12]), "8 CLEAR / 12 PLAYS");
        assert_eq!(SelectText::NoPlayYet.get(english), "NO PLAY YET");
        assert_eq!(SelectText::RankLine.fill(english, &[&"AA"]), "RANK AA");
        assert_eq!(SelectText::FolderTitle.get(english), "FOLDER");
        assert_eq!(SelectText::MissingChartNotice.get(english), "NOT IN THE LIBRARY - ENTER FOR THE DOWNLOAD");
        assert_eq!(SelectText::ModalGauge.fill(english, &[&80]), "GAUGE  80%");
        assert_eq!(
            [SelectText::PlayReplay, SelectText::NoReplaySaved, SelectText::ModalKeys, SelectText::Close].map(|entry| entry.get(english)),
            ["PLAY REPLAY", "NO REPLAY SAVED", "UP DOWN PREV/NEXT   ENTER REPLAY   ESC CLOSE", "CLOSE"]
        );
    }

    #[test]
    fn a_number_is_put_where_each_language_wants_it() {
        assert_eq!(SelectText::ChartCount.fill(Language::Korean, &[&86]), "차트 86개");
        assert_eq!(SelectText::ChartCount.fill(Language::Japanese, &[&86]), "86譜面");
        assert_eq!(SelectText::ClearAndPlays.fill(Language::Korean, &[&8, &12]), "클리어 8 / 플레이 12");
    }
}
