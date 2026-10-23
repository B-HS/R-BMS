//! Fixed words of the PRACTICE panel, by language.
//!
//! The name of the gauge a row shows, the gauge table and the arrangement are terms every language keeps and are not here. TOTAL and OPTION-1P are
//! the names the reference's practice panel gives its rows, and stay as they are.

rbms_model::translations! {
    /// Words of the PRACTICE panel: its heading, the row that starts the slice, the keys under the title and the labels of its rows.
    pub(crate) enum PracticeText {
        Heading => ("PRACTICE", "연습", "練習"),
        Start => ("START", "시작", "開始"),
        HintKeys => (
            "UP DOWN ROW   LEFT RIGHT VALUE   SHIFT x10   0 DEFAULT   ENTER START, OR TYPE ON A NUMBER   ESC BACK",
            "UP DOWN 줄   LEFT RIGHT 값   SHIFT x10   0 기본값   ENTER 시작, 숫자에서는 입력   ESC 뒤로",
            "UP DOWN 行   LEFT RIGHT 値   SHIFT x10   0 初期値   ENTER 開始、数値では入力   ESC 戻る",
        ),
        HintMouse => (
            "DRAG A SLIDER   CLICK A NUMBER TO TYPE   CLICK A ROW TO STEP IT   RIGHT CLICK STEP BACK",
            "슬라이더 드래그   숫자 클릭으로 입력   줄 클릭으로 한 단계 변경   오른쪽 클릭으로 한 단계 되돌리기",
            "スライダーをドラッグ   数値をクリックして入力   行をクリックして1段階変更   右クリックで1段階戻す",
        ),
        StartTime => ("START TIME", "시작 시간", "開始時間"),
        EndTime => ("END TIME", "종료 시간", "終了時間"),
        GaugeType => ("GAUGE TYPE", "게이지 종류", "ゲージの種類"),
        GaugeCategory => ("GAUGE CATEGORY", "게이지 분류", "ゲージの分類"),
        GaugeValue => ("GAUGE VALUE", "게이지 값", "ゲージの値"),
        JudgeRank => ("JUDGERANK", "판정 랭크", "判定ランク"),
        Total => ("TOTAL"),
        Frequency => ("FREQUENCY", "재생 속도", "再生速度"),
        Option1p => ("OPTION-1P"),
        ChartNoLongerLoaded => (
            "practice chart is no longer loaded",
            "연습할 차트가 더 이상 불러온 상태가 아닙니다",
            "練習する譜面はもう読み込まれていません",
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(PracticeText::ALL), Vec::<String>::new());
    }
}
