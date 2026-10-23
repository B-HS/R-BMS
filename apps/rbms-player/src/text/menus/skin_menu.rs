//! Fixed words of the SKIN menu, by language.
//!
//! The short names of the screens on its tabs (7K, SELECT, ...) are the names the settings table gives the screens and are not here. What a row of
//! the settings table says of itself comes from the settings table, and the name a skin gives its own rows is the skin's.

rbms_model::translations! {
    /// Words of the SKIN menu: its title, the keys along its foot, what is said while a folder is picked or the pack is exported, and what is said of
    /// the rows a skin document declares.
    pub(crate) enum SkinMenuText {
        Title => ("SKIN", "스킨", "スキン"),
        Hint => (
            "UP DOWN MOVE   LEFT RIGHT CHANGE   SHIFT x10   ENTER RUN OR TYPE   0 RESET   A ALL SCREENS   DRAG A SLIDER   TAB SCREEN   WHEEL SCROLL   RIGHT CLICK BACK   ESC SAVE",
            "UP DOWN 이동   LEFT RIGHT 변경   SHIFT x10   ENTER 실행 또는 입력   0 초기화   A 모든 화면   슬라이더 드래그   TAB 화면   휠 스크롤   오른쪽 클릭 뒤로   ESC 저장",
            "UP DOWN 移動   LEFT RIGHT 変更   SHIFT x10   ENTER 実行／入力   0 リセット   A すべての画面   スライダーをドラッグ   TAB 画面   ホイールでスクロール   右クリックで戻る   ESC 保存",
        ),

        PickerNote => (
            "CHOOSE THE PACK FOLDER IN THE WINDOW THAT OPENED",
            "열린 창에서 팩 폴더를 고르세요",
            "開いたウィンドウでパックのフォルダを選んでください",
        ),
        ExportPickerNote => (
            "CHOOSE WHERE TO EXPORT IN THE WINDOW THAT OPENED - A NEW FOLDER IS MADE THERE",
            "열린 창에서 내보낼 위치를 고르세요 - 그곳에 새 폴더가 만들어집니다",
            "開いたウィンドウでエクスポート先を選んでください - そこに新しいフォルダが作成されます",
        ),
        ExportHelp => (
            "Copy this pack, with the choices made in every screen, into a new folder to take elsewhere",
            "모든 화면에서 고른 선택과 함께 이 팩을 새 폴더로 복사해 다른 곳으로 가져갈 수 있게 합니다",
            "すべての画面で選んだ内容とともに、このパックを新しいフォルダにコピーして持ち出せるようにします",
        ),
        ExportListingNote => (
            "EXPORTING - LISTING THE PACK - ESC CANCELS",
            "내보내는 중 - 팩의 파일을 확인하는 중 - ESC 키로 취소",
            "エクスポート中 - パックのファイルを確認中 - ESCでキャンセル",
        ),
        ExportProgressNote => (
            "EXPORTING {0} / {1} FILES - {2} / {3} MB - ESC CANCELS",
            "내보내는 중 파일 {0} / {1} - {2} / {3} MB - ESC 키로 취소",
            "エクスポート中 {0} / {1} ファイル - {2} / {3} MB - ESCでキャンセル",
        ),
        SaidHelp => ("Any key or press clears this", "아무 키나 클릭하면 사라집니다", "キーを押すかクリックすると消えます"),
        ExportedHeading => (
            "EXPORTED {0} FILES ({1} MB{2}) TO",
            "파일 {0}개 ({1} MB{2}) 내보낸 위치:",
            "{0}ファイル（{1} MB{2}）のエクスポート先:",
        ),
        LinksLeftOut => (", {0} LINKS LEFT OUT", ", 링크 {0}개 제외", "、リンク{0}件を除外"),
        ExportFailedHeading => ("EXPORT FAILED", "내보내기 실패", "エクスポート失敗"),
        ExportCancelledHeading => ("EXPORT CANCELLED", "내보내기 취소됨", "エクスポートをキャンセル"),
        AppliedHeading => ("APPLIED TO ALL SCREENS", "모든 화면에 적용함", "すべての画面に適用"),
        ExportCancelledNotice => ("skin export cancelled", "스킨 내보내기를 취소했습니다", "スキンのエクスポートをキャンセルしました"),
        ExportedNotice => (
            "skin exported: {0} files to {1}",
            "스킨을 내보냈습니다: 파일 {0}개, 위치 {1}",
            "スキンをエクスポートしました: {0}ファイル、出力先 {1}",
        ),
        ExportFailedNotice => ("skin export failed: {0}", "스킨 내보내기에 실패했습니다: {0}", "スキンのエクスポートに失敗しました: {0}"),

        AlsoOnOneOtherScreen => (
            "ALSO ON 1 OTHER SCREEN - A APPLIES THIS VALUE TO IT",
            "다른 화면 1개에도 있습니다 - A 키로 이 값을 그 화면에 적용합니다",
            "他の1画面にもあります - Aキーでこの値をそちらに適用します",
        ),
        AlsoOnOtherScreens => (
            "ALSO ON {0} OTHER SCREENS - A APPLIES THIS VALUE TO THEM ALL",
            "다른 화면 {0}개에도 있습니다 - A 키로 이 값을 모두에 적용합니다",
            "他の{0}画面にもあります - Aキーでこの値をすべてに適用します",
        ),

        PropertyHelp => ("An option this skin declares", "이 스킨이 선언한 옵션입니다", "このスキンが宣言するオプションです"),
        FileHelp => (
            "A file slot this skin declares: which of its files is used",
            "이 스킨이 선언한 파일 슬롯입니다: 어느 파일을 쓸지 정합니다",
            "このスキンが宣言するファイルスロットです: 使用するファイルを選びます",
        ),
        OffsetPlaceHelp => (
            "An offset of this skin, in pixels - percent for the whole screen. The slider is coarse: arrows or ENTER set it exactly. 0 key resets",
            "이 스킨의 오프셋입니다. 단위는 픽셀이고, 화면 전체는 퍼센트입니다. 슬라이더는 거칠어서 화살표 키나 ENTER 키로 정확히 정합니다. 0 키로 초기화",
            "このスキンのオフセット。単位はピクセル（画面全体は%）。スライダーは粗いので、矢印キーかENTERで正確に設定。0キーでリセット",
        ),
        OffsetAngleHelp => (
            "An offset of this skin: a turn, in degrees. 0 key resets",
            "이 스킨의 오프셋입니다: 회전, 단위는 도. 0 키로 초기화",
            "このスキンのオフセット: 回転、単位は度。0キーでリセット",
        ),
        OffsetAlphaHelp => (
            "An offset of this skin: added to the opacity, from -255 (gone) to +255. 0 key resets",
            "이 스킨의 오프셋입니다: 불투명도에 더하며, -255(사라짐)부터 +255까지. 0 키로 초기화",
            "このスキンのオフセット: 不透明度に加算、-255（消える）から+255まで。0キーでリセット",
        ),
        OpacityHelp => (
            "How opaque this part is: 0% hides it, 100% is the skin's own. Typing past 100% (to 200%) adds alpha. 0 key resets",
            "이 부분의 불투명도입니다: 0%는 숨김, 100%는 스킨 원래 값입니다. 입력으로 100%를 넘기면(최대 200%) 알파가 더해집니다. 0 키로 초기화",
            "この部分の不透明度: 0%で非表示、100%でスキン本来の値。入力で100%超（最大200%）にするとアルファを加算。0キーでリセット",
        ),
        ColorHelp => (
            "Color of this part. Left/right: preset, ENTER: type a hex color, 0: back to the skin default. Alpha multiplies its opacity",
            "이 부분의 색상입니다. 좌우: 프리셋, ENTER: 16진수 색상 입력, 0: 스킨 기본값으로 되돌림. 알파는 불투명도에 곱해집니다",
            "この部分の色です。左右: プリセット、ENTER: 16進数の色を入力、0: スキンの初期値に戻す。アルファは不透明度に乗算されます",
        ),
        ChannelFoldHelp => (
            "The red, green, blue and alpha of each color, set one at a time. ENTER shows or hides the rows",
            "각 색상의 빨강, 초록, 파랑, 알파 값을 하나씩 정합니다. ENTER 키로 줄을 펼치거나 접습니다",
            "各色の赤・緑・青・アルファを1つずつ設定します。ENTERで行の表示／非表示を切り替えます",
        ),
        ChannelHelp => (
            "One channel of a color of this skin, from 0 to 255. Alpha multiplies the part's opacity. 0 key resets",
            "이 스킨 색상의 채널 하나입니다. 0부터 255까지. 알파는 그 부분의 불투명도에 곱해집니다. 0 키로 초기화",
            "このスキンの色のチャンネル1つです。0〜255。アルファはその部分の不透明度に乗算されます。0キーでリセット",
        ),
        AutomaticPlaceHelp => (
            "An offset the game adds to every play skin, in pixels - percent for the whole screen. Arrows or ENTER set it exactly. 0 key resets",
            "모든 플레이 스킨에 게임이 더하는 오프셋입니다. 단위는 픽셀이고, 화면 전체는 퍼센트입니다. 화살표 키나 ENTER 키로 정확히 정합니다. 0 키로 초기화",
            "ゲームが全プレースキンに加えるオフセット。単位はピクセル（画面全体は%）。矢印キーかENTERで正確に設定。0キーでリセット",
        ),
        AutomaticAlphaHelp => (
            "An offset the game adds to every play skin: added to the opacity, from -255 (gone) to +255. 0 key resets",
            "모든 플레이 스킨에 게임이 더하는 오프셋입니다: 불투명도에 더하며, -255(사라짐)부터 +255까지. 0 키로 초기화",
            "ゲームが全プレースキンに加えるオフセット: 不透明度に加算、-255（消える）から+255まで。0キーでリセット",
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(SkinMenuText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_export_heading_puts_the_counts_where_each_language_wants_them() {
        let links = SkinMenuText::LinksLeftOut.fill(Language::English, &[&2]);
        assert_eq!(SkinMenuText::ExportedHeading.fill(Language::English, &[&12, &"3.5", &links]), "EXPORTED 12 FILES (3.5 MB, 2 LINKS LEFT OUT) TO");
    }
}
