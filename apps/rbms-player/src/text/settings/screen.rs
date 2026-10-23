//! The fixed words the settings screen draws itself: its title and buttons, the question it asks
//! before it puts a group back, the lists drawn over the rows, the audio status line, what a row
//! that holds a number says while it is typed into, and the option panel the browser puts over
//! itself. By language.
//!
//! Keys are named by the key they are on and are written as they are in every language; the words
//! beside them are the ones here, with a numbered placeholder where a name or a number belongs.

rbms_model::translations! {
    /// The words of the settings screen's own furniture.
    pub(crate) enum ScreenText {
        Title => ("SETTINGS", "설정", "設定"),
        ResetGroup => ("RESET GROUP", "그룹 초기화", "グループ初期化"),
        OtherScreens => ("OTHER SCREENS", "다른 화면", "他の画面"),
        PlayMenu => ("PLAY MENU", "플레이 메뉴", "プレーメニュー"),
        SkinMenu => ("SKIN MENU", "스킨 메뉴", "スキンメニュー"),
        KeyConfig => ("KEY CONFIG", "키 설정", "キー設定"),
        KeysHeading => ("KEYS", "키", "キー"),
        StatusHeading => ("STATUS", "상태", "ステータス"),
        RivalsTitle => ("RIVALS", "라이벌", "ライバル"),
        RivalsHint => (
            "UP DOWN MOVE   ENTER ADD   D REMOVE   ESC SAVE/CLOSE",
            "UP DOWN 이동   ENTER 추가   D 삭제   ESC 저장/닫기",
            "UP DOWN 移動   ENTER 追加   D 削除   ESC 保存/閉じる"
        ),
        RivalAddRow => ("+ ADD RIVAL (PLAYER ID)", "+ 라이벌 추가 (플레이어 ID)", "+ ライバルを追加（プレーヤーID）"),
        ProfileAddRow => ("+ ADD IR PROFILE (BASE URL)", "+ IR 프로필 추가 (기본 URL)", "+ IRプロファイルを追加（ベースURL）"),
        ConfirmYes => ("ENTER  RESET", "ENTER  초기화", "ENTER  リセット"),
        ConfirmNo => ("ESC  KEEP", "ESC  유지", "ESC  維持"),
        ConfirmHint => (
            "ENTER RESET THE GROUP   ESC KEEP IT AS IT IS",
            "ENTER 그룹 초기화   ESC 그대로 두기",
            "ENTER グループを初期化   ESC そのままにする"
        ),
        ConfirmTitle => ("RESET {0}?", "{0} 초기화?", "{0}を初期化しますか？"),
        ConfirmOne => (
            "{0} CHANGED ROW GOES BACK TO DEFAULT.",
            "변경된 행 {0}개를 기본값으로 되돌립니다.",
            "変更された行{0}件を既定値に戻します。"
        ),
        ConfirmMany => (
            "{0} CHANGED ROWS GO BACK TO DEFAULT.",
            "변경된 행 {0}개를 기본값으로 되돌립니다.",
            "変更された行{0}件を既定値に戻します。"
        ),
        ConfirmOneKeepsText => (
            "{0} CHANGED ROW GOES BACK TO DEFAULT. TEXT IS KEPT.",
            "변경된 행 {0}개를 기본값으로 되돌립니다. 텍스트는 그대로 둡니다.",
            "変更された行{0}件を既定値に戻します。テキストはそのままです。"
        ),
        ConfirmManyKeepsText => (
            "{0} CHANGED ROWS GO BACK TO DEFAULT. TEXT IS KEPT.",
            "변경된 행 {0}개를 기본값으로 되돌립니다. 텍스트는 그대로 둡니다.",
            "変更された行{0}件を既定値に戻します。テキストはそのままです。"
        ),
        TextEditHint => ("TYPE   ENTER SET   ESC CANCEL", "입력   ENTER 확정   ESC 취소", "入力   ENTER 決定   ESC キャンセル"),
    }
}

rbms_model::translations! {
    /// The line the AUDIO group keeps at the foot of the panel about the output stream.
    pub(crate) enum AudioStatusText {
        Unavailable => ("NO OUTPUT DEVICE - RUNNING SILENT", "출력 장치 없음 - 무음으로 실행 중", "出力デバイスなし - 無音で動作中"),
        Matched => (
            "OUTPUT MATCHES THESE SETTINGS: {0}",
            "출력이 이 설정과 일치합니다: {0}",
            "出力はこの設定と一致しています: {0}"
        ),
    }
}

rbms_model::translations! {
    /// What a row that holds a number says while it is typed into or refused, and the keys the
    /// screen names while one is.
    pub(crate) enum NumberRowText {
        EditHint => (
            "TYPE A NUMBER   ENTER SET   ESC CANCEL   UP DOWN TAB SET AND MOVE ON",
            "숫자 입력   ENTER 확정   ESC 취소   UP DOWN TAB 확정 후 이동",
            "数値を入力   ENTER 決定   ESC キャンセル   UP DOWN TAB 決定して移動"
        ),
        EditHintClock => (
            "TYPE SECONDS OR MIN:SEC   ENTER SET   ESC CANCEL   UP DOWN TAB SET AND MOVE ON",
            "초 또는 분:초 입력   ENTER 확정   ESC 취소   UP DOWN TAB 확정 후 이동",
            "秒または分:秒を入力   ENTER 決定   ESC キャンセル   UP DOWN TAB 決定して移動"
        ),
        EditHintHex => (
            "TYPE A HEX COLOR: RRGGBB OR RRGGBBAA   ENTER SET   ESC CANCEL   UP DOWN TAB SET AND MOVE ON",
            "16진수 색 입력: RRGGBB 또는 RRGGBBAA   ENTER 확정   ESC 취소   UP DOWN TAB 확정 후 이동",
            "16進数の色を入力: RRGGBB または RRGGBBAA   ENTER 決定   ESC キャンセル   UP DOWN TAB 決定して移動"
        ),
        NotANumber => ("NOT A NUMBER", "숫자가 아닙니다", "数値ではありません"),
        NotAColor => ("NOT A COLOR", "색이 아닙니다", "色ではありません"),
    }
}

rbms_model::translations! {
    /// The option panel the browser puts over itself.
    pub(crate) enum OptionsText {
        Title => ("OPTIONS", "옵션", "オプション"),
        Hint => (
            "UP DOWN MOVE   LEFT RIGHT CHANGE   {0} PIN   ESC CLOSE",
            "UP DOWN 이동   LEFT RIGHT 변경   {0} 고정   ESC 닫기",
            "UP DOWN 移動   LEFT RIGHT 変更   {0} 固定   ESC 閉じる"
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_catalog_is_written_in_every_language() {
        assert_eq!(catalog_problems(ScreenText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(AudioStatusText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(NumberRowText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(OptionsText::ALL), Vec::<String>::new());
    }
}
