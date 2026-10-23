//! Fixed words of the song-folder manager, by language.

rbms_model::translations! {
    /// Words of the song-folder manager.
    pub(crate) enum FoldersText {
        Title => ("SONG FOLDERS", "곡 폴더", "曲フォルダ"),
        PickInDialog => (
            "PICK A SONG FOLDER IN THE OPEN DIALOG",
            "열린 대화상자에서 곡 폴더를 고르세요",
            "開いたダイアログで曲フォルダを選んでください",
        ),
        Keys => (
            "UP DOWN MOVE   ENTER ADD FOLDER   D REMOVE   ESC SAVE/RESCAN",
            "UP DOWN 이동   ENTER 폴더 추가   D 삭제   ESC 저장/다시 스캔",
            "UP DOWN 移動   ENTER フォルダを追加   D 削除   ESC 保存/再スキャン",
        ),
        /// What the row of the download folder reads before its path.
        DownloadsLabel => ("DOWNLOADS  ", "다운로드  ", "ダウンロード  "),
        AddFolder => ("+ ADD FOLDER", "+ 폴더 추가", "+ フォルダを追加"),
        NoFolders => (
            "No folders yet — ENTER on \"+ ADD FOLDER\" to pick one.",
            "아직 폴더가 없습니다 — \"+ 폴더 추가\" 줄에서 ENTER 키로 고를 수 있습니다.",
            "フォルダがまだありません — 「+ フォルダを追加」でENTERを押して選べます。",
        ),
        AlreadyAdded => ("folder already added: {0}", "이미 추가된 폴더입니다: {0}", "追加済みのフォルダです: {0}"),
        DownloadFolderElsewhere => (
            "the download folder is changed in SETTINGS > SELECT",
            "다운로드 폴더는 설정 > 선곡에서 바꿉니다",
            "ダウンロードフォルダは設定 > 選曲で変更します",
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(FoldersText::ALL), Vec::<String>::new());
    }
}
