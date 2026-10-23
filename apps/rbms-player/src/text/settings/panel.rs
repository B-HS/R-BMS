//! The panel beside the settings list and the lines of keys along the foot of the screen, by
//! language.
//!
//! A key is named by the key it is on, which is written as it is in every language. What the key
//! does, what kind of row the panel is describing and the names it puts what a row holds under are
//! the words here.

rbms_model::translations! {
    /// What a key does, as the panel and the line along the foot of the screen say it.
    pub(crate) enum KeyActionText {
        Move => ("MOVE", "이동", "移動"),
        Section => ("SECTION", "구역", "セクション"),
        Group => ("GROUP", "그룹", "グループ"),
        ResetGroup => ("RESET GROUP", "그룹 초기화", "グループ初期化"),
        Play => ("PLAY", "플레이", "プレー"),
        Skins => ("SKINS", "스킨", "スキン"),
        KeyEditor => ("KEYS", "키 설정", "キー設定"),
        Save => ("SAVE", "저장", "保存"),
        OneStep => ("1 STEP", "1단계", "1ステップ"),
        TenSteps => ("x10"),
        Type => ("TYPE", "입력", "入力"),
        ToDefault => ("DEFAULT", "기본값", "既定値"),
        Slider => ("SLIDER", "슬라이더", "スライダー"),
        Open => ("OPEN", "열기", "開く"),
        ChooseFile => ("CHOOSE A FILE", "파일 선택", "ファイルを選ぶ"),
        ChooseFolder => ("CHOOSE A FOLDER", "폴더 선택", "フォルダを選ぶ"),
        Unset => ("NONE", "없음", "なし"),
        ShowState => ("SHOW ITS STATE", "상태 표시", "状態を表示"),
        OpenList => ("OPEN THE LIST", "목록 열기", "リストを開く"),
        Edit => ("EDIT", "편집", "編集"),
        Run => ("RUN", "실행", "実行"),
        Switch => ("SWITCH", "전환", "切替"),
        Change => ("CHANGE", "변경", "変更"),
        Next => ("NEXT", "다음", "次へ"),
        Set => ("SET", "확정", "決定"),
        Cancel => ("CANCEL", "취소", "キャンセル"),
        TypeNumber => ("TYPE A NUMBER", "숫자 입력", "数値を入力"),
        SetAndMoveOn => ("SET AND MOVE ON", "확정 후 이동", "決定して移動"),
        Add => ("ADD", "추가", "追加"),
        Remove => ("REMOVE", "삭제", "削除"),
        SaveAndClose => ("SAVE AND CLOSE", "저장 후 닫기", "保存して閉じる"),
        ResetTheGroup => ("RESET THE GROUP", "그룹 초기화", "グループを初期化"),
        KeepAsItIs => ("KEEP IT AS IT IS", "그대로 두기", "そのままにする"),
    }
}

rbms_model::translations! {
    /// The names of keys that are no key of the keyboard: what is dragged, and what is typed.
    pub(crate) enum KeyNameText {
        Drag => ("DRAG", "드래그", "ドラッグ"),
        Digits => ("DIGITS", "숫자", "数字"),
        Keys => ("KEYS", "키", "キー"),
    }
}

rbms_model::translations! {
    /// What the panel calls each kind of row, and the names it writes what a row holds under.
    pub(crate) enum PanelText {
        KindSwitch => ("SWITCH", "스위치", "スイッチ"),
        KindChoice => ("CHOICE", "선택", "選択"),
        KindChoiceAt => ("CHOICE  {0} OF {1}", "선택  {1}개 중 {0}번째", "選択  {1}件中{0}番目"),
        KindNumber => ("NUMBER", "숫자", "数値"),
        KindText => ("TEXT", "텍스트", "テキスト"),
        KindSecret => ("TEXT, NEVER SHOWN", "텍스트(표시 안 함)", "テキスト（非表示）"),
        KindFile => ("FILE", "파일", "ファイル"),
        KindFolder => ("FOLDER", "폴더", "フォルダ"),
        KindAction => ("ACTION", "실행", "アクション"),
        KindStatus => ("STATUS", "상태", "ステータス"),
        FactValue => ("VALUE", "값", "値"),
        FactDefault => ("DEFAULT", "기본값", "既定値"),
        FactRange => ("RANGE", "범위", "範囲"),
        FactStep => ("STEP", "간격", "刻み"),
        FactChoices => ("CHOICES", "선택지", "選択肢"),
        DefaultWhileChanged => ("{0}  (0 PUTS IT BACK)", "{0}  (0을 누르면 되돌림)", "{0}  （0キーで戻す）"),
        RangeFrom => ("{0} TO {1}", "{0}에서 {1}까지", "{0}〜{1}"),
        StepWithShift => ("{0}, {1} WITH SHIFT", "{0}, SHIFT를 누르면 {1}", "{0}、SHIFTで{1}"),
        BuiltInOnlyRemark => (
            "A skin is set to draw this screen and does not read this row. It still holds wherever the built-in screen is drawn.",
            "이 화면은 스킨이 그리도록 설정되어 있어 스킨은 이 행을 읽지 않습니다. 내장 화면이 그려지는 곳에서는 이 행이 그대로 적용됩니다.",
            "この画面はスキンが描画する設定のため、スキンはこの行を読み取りません。内蔵画面が描画される場面では、この行は引き続き有効です。"
        ),
        SkinOptionRemark => (
            "A skin is set to draw this screen, and on it the skin's own option decides. This row still holds wherever the built-in screen is drawn.",
            "이 화면은 스킨이 그리도록 설정되어 있어 스킨 자체의 옵션이 우선합니다. 내장 화면이 그려지는 곳에서는 이 행이 그대로 적용됩니다.",
            "この画面はスキンが描画する設定のため、スキン自身のオプションが優先されます。内蔵画面が描画される場面では、この行は引き続き有効です。"
        ),
        CustomJudgeCaution => (
            "OVER 100%: A RUN PLAYED LIKE THIS SETS NO BEST SCORE. IT IS STILL KEPT, WITH ITS REPLAY, AND SENT.",
            "100% 초과: 이렇게 플레이한 기록은 최고 기록이 되지 않습니다. 리플레이와 함께 저장·전송은 됩니다.",
            "100%超: この設定でのプレーはベストスコアになりません。リプレイとともに保存・送信はされます。"
        ),
        BuiltInOnlyNote => ("BUILT-IN SCREENS ONLY", "내장 화면 전용", "内蔵画面のみ"),
        SkinOptionNote => ("SKIN OPTION COMES FIRST", "스킨 옵션 우선", "スキンのオプションが優先"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_catalog_is_written_in_every_language() {
        assert_eq!(catalog_problems(KeyActionText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(KeyNameText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(PanelText::ALL), Vec::<String>::new());
    }
}
