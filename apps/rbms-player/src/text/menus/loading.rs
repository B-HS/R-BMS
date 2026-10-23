//! Fixed words of the LOADING screen, by language.
//!
//! The title of a chart on the line under the heading, the name of a course and the reason a scan failed are data and stay as they come. The words of
//! the progress that count something (keysounds, images, files, tables) are the unit written after the count.

rbms_model::translations! {
    /// Words of the LOADING screen: its headings, the steps under them and the units of its counts.
    pub(crate) enum LoadingText {
        Loading => ("LOADING", "로딩", "読み込み中"),
        Course => ("COURSE", "코스", "コース"),
        MatchingTables => ("MATCHING TABLES", "난이도표 대조 중", "難易度表を照合中"),
        Scanning => ("SCANNING", "스캔 중", "スキャン中"),
        FoldersToScan => ("{0} folder(s)", "폴더 {0}개", "{0}フォルダ"),
        ChartsFound => ("{0} charts found", "차트 {0}개 발견", "{0}譜面を検出"),
        KeysoundsStep => ("{0} / {1} keysounds", "{0} / {1} 키음", "{0} / {1} キー音"),
        ImagesStep => ("{0} / {1} images", "{0} / {1} 이미지", "{0} / {1} 画像"),
        RecordRunStep => ("record run", "기록 플레이", "記録のプレー"),
        FilesUnit => ("files", "파일", "ファイル"),
        TablesUnit => ("tables", "난이도표", "難易度表"),
        Scanned => ("scanned {0} charts", "차트 {0}개를 스캔했습니다", "{0}譜面をスキャンしました"),
        ScanStoppedEarly => (
            "library scan stopped early ({0} charts read)",
            "라이브러리 스캔을 중간에 멈췄습니다 (차트 {0}개 읽음)",
            "ライブラリのスキャンを途中で停止しました（{0}譜面を読み込み済み）",
        ),
        LibraryRead => (
            "library: {0} chart(s) read, {1} dropped",
            "라이브러리: 차트 {0}개 읽음, {1}개 제외",
            "ライブラリ: {0}譜面を読み込み、{1}件を除外",
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(LoadingText::ALL), Vec::<String>::new());
    }
}
