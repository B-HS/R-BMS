//! Fixed words of the modal over a chart a difficulty table or one of the player's folders lists and the library does not hold, by
//! language.
//!
//! How far the download job is, and why it could not start, are the download's own lines and are not here. The title of the chart, its
//! artist and the table and level it is listed under are data.

rbms_model::translations! {
    /// Words of the modal over a chart the library does not hold.
    pub(crate) enum DownloadModalText {
        /// What the modal says above the chart's title.
        Heading => ("NOT IN THE LIBRARY", "라이브러리에 없음", "ライブラリにありません"),
        /// What the modal says of the two links of a table's chart.
        NoteChartFile => (
            "THE TABLE'S DL IS THE CHART FILE ONLY (A DIFFERENCE CHART).",
            "난이도표의 DL은 차트 파일뿐입니다 (차분 차트).",
            "難易度表のDLは譜面ファイルのみです（差分譜面）。",
        ),
        NoteSongUrl => (
            "THE SONG ITSELF - ITS SOUNDS AND IMAGES - HAS TO BE GOT FROM THE SONG URL AND PUT BESIDE THE CHART.",
            "곡 자체(음원과 이미지)는 곡 URL에서 받아 차트 옆에 두어야 합니다.",
            "曲そのもの（音源と画像）は曲のURLから入手し、譜面の隣に置く必要があります。",
        ),
        /// What the modal says of a chart one of the player's folders lists: a training chart the server made, which comes with its
        /// sounds, and any other chart, with or without a link the folder gives for it.
        NoteTrainingChart => (
            "A TRAINING CHART THE SERVER MADE FOR YOU. GET CHART DOWNLOADS IT WITH ITS CLICK SOUNDS.",
            "서버가 만들어 준 훈련곡입니다. 차트 받기로 클릭음과 함께 받습니다.",
            "サーバーが作成したトレーニング譜面です。譜面を取得でクリック音と一緒にダウンロードします。",
        ),
        NoteFolderChart => (
            "THIS CHART IS IN YOUR FOLDER BUT NOT IN THE LIBRARY.",
            "내 폴더에 있지만 라이브러리에 없는 차트입니다.",
            "自分のフォルダにありますが、ライブラリにない譜面です。",
        ),
        NoteFolderNoLink => (
            "THE FOLDER HAS NO DOWNLOAD LINK FOR IT. GET IT ELSEWHERE AND RESCAN.",
            "폴더에 이 차트의 다운로드 링크가 없습니다. 다른 곳에서 받은 뒤 다시 스캔하세요.",
            "フォルダにこの譜面のダウンロードリンクがありません。別の場所で入手して再スキャンしてください。",
        ),
        /// What the modal says while the folder picker it opened is up, and while a job is getting another chart.
        PickingTheFolder => (
            "CHOOSE THE FOLDER THE CHART IS UNPACKED INTO",
            "차트를 압축 해제할 폴더를 고르세요",
            "譜面を展開するフォルダを選んでください",
        ),
        AnotherDownloadIsRunning => ("ANOTHER DOWNLOAD IS RUNNING", "다른 다운로드가 진행 중입니다", "別のダウンロードが実行中です"),
        /// The keys the modal is driven by.
        KeyHint => (
            "LEFT RIGHT CHOOSE   ENTER PRESS   ESC CLOSE",
            "LEFT RIGHT 선택   ENTER 누르기   ESC 닫기",
            "LEFT RIGHT 選択   ENTER 決定   ESC 閉じる",
        ),
        /// The buttons.
        GetChart => ("GET CHART", "차트 받기", "譜面を取得"),
        OpenSongUrl => ("OPEN SONG URL", "곡 URL 열기", "曲のURLを開く"),
        OpenChartUrl => ("OPEN CHART URL", "차트 URL 열기", "譜面のURLを開く"),
        Close => ("CLOSE", "닫기", "閉じる"),
        Cancel => ("CANCEL", "취소", "キャンセル"),
        /// Said as a notice when a chart is asked for that the table no longer lists where the bar said.
        NotInTheLibrary => ("not in the library", "라이브러리에 없습니다", "ライブラリにありません"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(DownloadModalText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_ones_the_modal_said_before_it_had_a_catalog() {
        let table = [
            (DownloadModalText::Heading, "NOT IN THE LIBRARY"),
            (DownloadModalText::NoteChartFile, "THE TABLE'S DL IS THE CHART FILE ONLY (A DIFFERENCE CHART)."),
            (DownloadModalText::NoteSongUrl, "THE SONG ITSELF - ITS SOUNDS AND IMAGES - HAS TO BE GOT FROM THE SONG URL AND PUT BESIDE THE CHART."),
            (DownloadModalText::NoteTrainingChart, "A TRAINING CHART THE SERVER MADE FOR YOU. GET CHART DOWNLOADS IT WITH ITS CLICK SOUNDS."),
            (DownloadModalText::NoteFolderChart, "THIS CHART IS IN YOUR FOLDER BUT NOT IN THE LIBRARY."),
            (DownloadModalText::NoteFolderNoLink, "THE FOLDER HAS NO DOWNLOAD LINK FOR IT. GET IT ELSEWHERE AND RESCAN."),
            (DownloadModalText::PickingTheFolder, "CHOOSE THE FOLDER THE CHART IS UNPACKED INTO"),
            (DownloadModalText::AnotherDownloadIsRunning, "ANOTHER DOWNLOAD IS RUNNING"),
            (DownloadModalText::KeyHint, "LEFT RIGHT CHOOSE   ENTER PRESS   ESC CLOSE"),
            (DownloadModalText::GetChart, "GET CHART"),
            (DownloadModalText::OpenSongUrl, "OPEN SONG URL"),
            (DownloadModalText::OpenChartUrl, "OPEN CHART URL"),
            (DownloadModalText::Close, "CLOSE"),
            (DownloadModalText::Cancel, "CANCEL"),
            (DownloadModalText::NotInTheLibrary, "not in the library"),
        ];
        assert_eq!(table.len(), DownloadModalText::ALL.len(), "an entry has no row in this table");
        for (text, expected) in table {
            assert_eq!(text.get(Language::English), expected, "{text:?}");
        }
    }
}
