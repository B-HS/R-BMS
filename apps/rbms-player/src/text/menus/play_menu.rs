//! Fixed words of the PLAY menu, by language.
//!
//! The figures under the preview (BPM, HI-SPEED, GREEN, WHITE), the units (MS, x) and the names of the options and of the targets are terms every
//! language keeps and are not here. What a row of the settings table says of itself comes from the settings table.

rbms_model::translations! {
    /// Words of the PLAY menu: its title, the keys along its foot, the groups and folds of its list, what is written beside the figures under the
    /// preview and what is said of the rows the settings table does not explain.
    pub(crate) enum PlayMenuText {
        Title => ("PLAY", "플레이", "プレー"),
        Hint => (
            "UP DOWN MOVE   LEFT RIGHT CHANGE   SHIFT x10   ENTER NEXT OR TYPE   0 DEFAULT   A ALL SCREENS   DRAG A SLIDER   TAB MODE   WHEEL SCROLL   RIGHT CLICK BACK   ESC SAVE",
            "UP DOWN 이동   LEFT RIGHT 변경   SHIFT x10   ENTER 다음 또는 입력   0 기본값   A 모든 화면   슬라이더 드래그   TAB 모드   휠 스크롤   오른쪽 클릭 뒤로   ESC 저장",
            "UP DOWN 移動   LEFT RIGHT 変更   SHIFT x10   ENTER 次へ／入力   0 初期値   A すべての画面   スライダーをドラッグ   TAB モード   ホイールでスクロール   右クリックで戻る   ESC 保存",
        ),

        /// The name of the figure for how long a note is on show.
        FigureTravel => ("TRAVEL", "이동 시간", "移動時間"),
        /// What is written after the tempo while the field scrolls at one tempo of its own.
        ConstantMark => ("(CONSTANT)"),
        /// What is written after the scroll speed while it is worked out from a held green number.
        HeldMark => ("(HELD)", "(유지)", "(維持)"),
        /// What is written after the scroll speed while the speed that would hold the green number is outside what a speed can be.
        LimitMark => ("(LIMIT)", "(한계)", "(上限)"),
        /// What is written after the white number while the lane cover it measures is switched off.
        CoverOffMark => ("(COVER OFF)", "(커버 OFF)", "(カバーOFF)"),
        LimitNote => (
            "HI-SPEED IS AT THE END OF ITS RANGE: NO SPEED HOLDS THIS GREEN NUMBER AT THIS TEMPO",
            "HI-SPEED 값이 범위의 끝입니다: 이 템포에서는 이 GREEN NUMBER 값을 유지할 속도가 없습니다",
            "HI-SPEEDが範囲の端にあります: このテンポではこの緑数字を保てる速度がありません",
        ),
        TargetRateUnusedNote => (
            "NOT IN USE: NO TARGET IS RATE CUSTOM",
            "사용 안 함: RATE CUSTOM 타깃이 없음",
            "未使用: RATE CUSTOMのターゲットがありません",
        ),
        ScratchAutoNote => (
            "WHILE ON: TURNTABLE NOT COUNTED, ASSIST LAMP BY GAUGE (E.G. ASSIST HARD)",
            "ON인 동안: 턴테이블은 집계되지 않고 게이지에 따른 ASSIST 램프가 붙음 (예: ASSIST HARD)",
            "ON中: ターンテーブルはカウントされず、ゲージに応じたASSISTランプになります（例: ASSIST HARD）",
        ),

        GroupPreview => ("PREVIEW", "미리보기", "プレビュー"),
        GroupSkin => ("SKIN", "스킨", "スキン"),
        PreviewBpm => ("PREVIEW BPM", "미리보기 BPM", "プレビューBPM"),
        FoldColor => ("COLOR", "색상", "色"),
        FoldOpacity => ("OPACITY", "불투명도", "不透明度"),
        FoldPlace => ("POSITION AND SIZE", "위치와 크기", "位置とサイズ"),
        FoldAutoChannels => ("SCRATCH AUTO CHANNELS", "SCRATCH AUTO 색상 채널", "SCRATCH AUTOの色チャンネル"),
        ScratchAutoColor => ("SCRATCH AUTO COLOR", "SCRATCH AUTO 색상", "SCRATCH AUTOの色"),

        ColorFoldHelp => (
            "The colors of the play skin of this mode, one row each. ENTER shows or hides the rows",
            "이 모드의 플레이 스킨 색상을 한 줄씩 보여 줍니다. ENTER 키로 줄을 펼치거나 접습니다",
            "このモードのプレースキンの色を1行ずつ表示します。ENTERで行の表示／非表示を切り替えます",
        ),
        OpacityFoldHelp => (
            "How opaque each part of the play skin of this mode is drawn. ENTER shows or hides the rows",
            "이 모드의 플레이 스킨 각 부분을 얼마나 불투명하게 그릴지 정합니다. ENTER 키로 줄을 펼치거나 접습니다",
            "このモードのプレースキンの各部分をどれだけ不透明に描くかを設定します。ENTERで行の表示／非表示を切り替えます",
        ),
        PlaceFoldHelp => (
            "The offsets the game adds to every play skin: its screen, notes and judgement. ENTER shows or hides the rows",
            "모든 플레이 스킨에 게임이 더하는 오프셋입니다: 화면, 노트, 판정. ENTER 키로 줄을 펼치거나 접습니다",
            "ゲームがすべてのプレースキンに加えるオフセットです: 画面、ノーツ、判定。ENTERで行の表示／非表示を切り替えます",
        ),
        ScratchAutoColorHelp => (
            "The color SCRATCH AUTO's notes are drawn in, on every skin. Its alpha is how opaque they are. ENTER types a hex color, 0 gives back the green",
            "모든 스킨에서 SCRATCH AUTO 노트를 그리는 색상입니다. 알파는 불투명도입니다. ENTER 키로 16진수 색상을 입력하고, 0 키로 초록색으로 되돌립니다",
            "すべてのスキンでSCRATCH AUTOのノーツを描く色です。アルファは不透明度です。ENTERで16進数の色を入力し、0で緑に戻します",
        ),
        ScratchAutoChannelsHelp => (
            "The red, green, blue and alpha of SCRATCH AUTO's color. ENTER shows or hides the rows",
            "SCRATCH AUTO 색상의 빨강, 초록, 파랑, 알파 값입니다. ENTER 키로 줄을 펼치거나 접습니다",
            "SCRATCH AUTOの色の赤・緑・青・アルファです。ENTERで行の表示／非表示を切り替えます",
        ),
        ScratchAutoChannelHelp => (
            "One channel of SCRATCH AUTO's color, 0 to 255",
            "SCRATCH AUTO 색상의 채널 하나, 0부터 255까지",
            "SCRATCH AUTOの色のチャンネル1つ、0〜255",
        ),
        PreviewBpmHelp => (
            "The tempo of the sample chart. It is not a setting: it shows what the speed options come to on another chart",
            "샘플 차트의 템포입니다. 설정이 아니라, 다른 차트에서 속도 옵션이 어떤 값이 되는지 보여 줍니다",
            "サンプル譜面のテンポです。設定ではなく、別の譜面で速度オプションがどうなるかを表示します",
        ),
        SkinOptionHelp => (
            "An option the play skin of this mode declares. It is stored for that skin document",
            "이 모드의 플레이 스킨이 선언한 옵션입니다. 그 스킨별로 저장됩니다",
            "このモードのプレースキンが宣言するオプションです。そのスキンごとに保存されます",
        ),
        HispeedWorkedOutHelp => (
            "Worked out from GREEN NUMBER at the preview tempo. Turn KEEP GREEN NUMBER off to set it",
            "미리보기 템포에서 GREEN NUMBER 값으로 계산한 값입니다. 직접 정하려면 KEEP GREEN NUMBER 항목을 OFF 상태로 바꾸세요",
            "プレビューのテンポで緑数字から算出した値です。設定するにはKEEP GREEN NUMBERをOFFにしてください",
        ),
        GreenNumberWorkedOutHelp => (
            "What HI-SPEED comes to at the preview tempo. Turn KEEP GREEN NUMBER on to set it",
            "미리보기 템포에서 HI-SPEED 값이 나타내는 수치입니다. 직접 정하려면 KEEP GREEN NUMBER 항목을 ON 상태로 바꾸세요",
            "プレビューのテンポでHI-SPEEDから算出した値です。設定するにはKEEP GREEN NUMBERをONにしてください",
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(PlayMenuText::ALL), Vec::<String>::new());
    }
}
