//! Fixed words of the LAYOUT PICK screen, by language.
//!
//! RANDOM, AUTOPLAY, the side names 1P and 2P and the key names are terms every language keeps and are not here.

rbms_model::translations! {
    /// Words of LAYOUT PICK: its key, its heading, its key lines, what it says of the chart and the
    /// seed, and why a layout is not played.
    pub(crate) enum LayoutPickText {
        /// The key that opens the screen, as the key editor and the key guide name it, and the
        /// screen's heading.
        ShortcutLabel => ("LAYOUT PICK", "배치 지정", "配置指定"),
        /// The keys that write the layout, and the up and down keys that move the preview while the rows have the keys:
        /// while the list of captured layouts has them, up and down move its cursor ([`LayoutPickText::HintCaptured`]).
        HintTyping => (
            "1-9 OR LANE KEYS TYPE THE LANE FOR THE NEXT PLACE   BACKSPACE ERASE   DELETE CLEAR   0 IN ORDER   TAB SIDE   UP DOWN MEASURE   RIGHT OR SELECT CAPTURED",
            "1-9 또는 레인 키로 다음 자리에 올 레인 입력   BACKSPACE 지우기   DELETE 비우기   0 순서대로   TAB 쪽 전환   UP DOWN 한 마디   RIGHT 또는 SELECT 저장한 배치",
            "1-9 またはレーンキーで次の位置のレーンを入力   BACKSPACE 消去   DELETE 空にする   0 順番どおり   TAB サイド切替   UP DOWN 1小節   RIGHT または SELECT 保存した配置",
        ),
        /// The keys of the list of captured layouts, while it is the one the keys go to.
        HintCaptured => (
            "UP DOWN OR KEY 1 KEY 2 CHOOSE   TAB OR KEY 3 THIS CHART / ALL   DELETE TWICE OR KEY 4 TWICE REMOVE   LEFT OR SELECT BACK TO THE LAYOUT",
            "UP DOWN 또는 KEY 1 KEY 2 고르기   TAB 또는 KEY 3 이 차트 / 전체   DELETE 또는 KEY 4 두 번 삭제   LEFT 또는 SELECT 배치 입력으로",
            "UP DOWN または KEY 1 KEY 2 選択   TAB または KEY 3 この譜面 / すべて   DELETE または KEY 4 を2回押して削除   LEFT または SELECT 配置入力へ",
        ),
        /// The keys that move the preview and leave the screen, which work whichever of the rows and the list has the keys.
        HintMoving => (
            "SPACE PLAY / STOP   PAGE UP PAGE DOWN 4 MEASURES   HOME END START END   ENTER PLAY   ESC BACK",
            "SPACE 재생 / 정지   PAGE UP PAGE DOWN 네 마디   HOME END 처음 끝   ENTER 플레이   ESC 뒤로",
            "SPACE 再生 / 停止   PAGE UP PAGE DOWN 4小節   HOME END 最初 最後   ENTER プレー   ESC 戻る",
        ),
        /// The preview is stopped where it stands.
        Stopped => ("STOPPED", "정지", "停止中"),
        /// The heading of the list of captured layouts of this chart, and how many there are.
        CapturedOfChart => ("CAPTURED: THIS CHART ({0})", "저장한 배치: 이 차트 ({0})", "保存した配置: この譜面（{0}）"),
        /// The heading of the list of every captured layout, and how many there are.
        CapturedOfAll => ("CAPTURED: ALL CHARTS ({0})", "저장한 배치: 전체 차트 ({0})", "保存した配置: すべての譜面（{0}）"),
        /// The list has nothing in it, and the key of a result screen that captures a layout.
        CapturedNone => (
            "NOTHING CAPTURED: PRESS {0} ON A RESULT",
            "저장한 배치 없음: 결과 화면에서 {0}",
            "保存した配置なし: リザルト画面で {0}",
        ),
        /// The list has nothing in it, and no key of a result screen captures a layout: one is given in
        /// the key editor, on the line [`CaptureText::ShortcutLabel`] names.
        CapturedNoneWithoutKey => (
            "NOTHING CAPTURED: GIVE CAPTURE LAYOUT A KEY IN KEY CONFIG",
            "저장한 배치 없음: 키 설정에서 배치 저장 키를 지정하세요",
            "保存した配置なし: キー設定で配置を保存のキーを指定してください",
        ),
        /// The captured layout of another chart is played on that chart: Enter goes to it.
        CapturedElsewhere => ("ENTER GOES TO THIS CHART", "ENTER로 이 차트로 이동", "ENTERでこの譜面へ移動"),
        /// The first press of the key that removes a captured layout.
        RemoveArmed => (
            "press it again to remove this captured layout",
            "한 번 더 누르면 이 저장한 배치를 삭제합니다",
            "もう一度押すと、この保存した配置を削除します",
        ),
        /// A captured layout removed.
        Removed => ("captured layout removed", "저장한 배치를 삭제했습니다", "保存した配置を削除しました"),
        /// The chart of a captured layout is not in the library.
        CapturedChartMissing => (
            "the chart of this captured layout is not in the library",
            "이 저장한 배치의 차트가 라이브러리에 없음",
            "この保存した配置の譜面がライブラリにありません",
        ),
        /// The layout on show is a captured one, played as it was captured.
        CapturedSeed => ("PLAYED AS {0}, SEED {1} (CAPTURED)", "{0} 시드 {1}로 플레이 (저장한 배치)", "{0} シード {1} でプレー（保存した配置）"),
        /// The chart is being read for the preview.
        Reading => ("READING THE CHART", "차트를 읽는 중", "譜面を読み込み中"),
        /// The chart could not be read, so nothing is previewed or played.
        ReadFailed => ("the chart could not be read", "차트를 읽을 수 없음", "譜面を読み込めません"),
        /// The chart file is not the one the library scanned, so a run would not be laid out as picked.
        ChartChanged => (
            "the chart file changed since the library was scanned: scan it again before playing this layout",
            "라이브러리를 스캔한 뒤 차트 파일이 바뀜: 다시 스캔한 뒤 이 배치로 플레이하세요",
            "ライブラリのスキャン後に譜面ファイルが変わりました: 再スキャンしてからこの配置でプレーしてください",
        ),
        /// The seed RANDOM lays the layout out from is being looked for.
        Searching => ("FINDING THE RANDOM SEED", "RANDOM 시드를 찾는 중", "RANDOMシードを検索中"),
        /// The seed RANDOM lays the layout out from: the run is played and recorded as RANDOM with it.
        SeedFound => ("PLAYED AS RANDOM, SEED {0}", "RANDOM 시드 {0}로 플레이", "RANDOM シード {0} でプレー"),
        /// No seed gives the layout: on ten keys an eighth of the pairs of sides is never laid out by RANDOM.
        Unreachable => (
            "RANDOM never lays the lanes out this way: change one side",
            "RANDOM 으로는 만들 수 없는 배치: 한쪽을 바꾸세요",
            "RANDOMではこの配置になりません: 片方のサイドを変えてください",
        ),
        /// The chart's mode has more layouts than a seed can name.
        UnsupportedMode => (
            "LAYOUT PICK does not lay out {0} charts",
            "배치 지정은 {0} 차트를 지원하지 않음",
            "配置指定は{0}の譜面に対応していません",
        ),
        /// AUTOPLAY is on, so the layout can be looked at but a run on it would not be the player's.
        AutoplayOn => (
            "AUTOPLAY is on: the layout can be previewed but not played",
            "AUTOPLAY 사용 중: 배치를 미리 볼 수는 있지만 플레이할 수는 없음",
            "AUTOPLAY使用中: 配置のプレビューはできますが、プレーはできません",
        ),
        /// Enter before the seed is known.
        NotReady => ("the layout is not ready to play yet", "아직 이 배치로 플레이할 수 없음", "まだこの配置でプレーできません"),
        /// A lane number that is not one of the side's, or that the side already has.
        DigitRefused => ("{0}: already placed or not a lane of this side", "{0}: 이미 넣었거나 이 쪽의 레인이 아님", "{0}: 入力済みか、このサイドのレーンではありません"),
        /// Where the played layout came from, for the line that starts the run.
        Source => ("your pick {0}", "직접 지정한 {0}", "指定した{0}"),
        /// The measure at the judgement line of the preview, and how many the chart has.
        Measure => ("MEASURE {0} / {1}", "마디 {0} / {1}", "小節 {0} / {1}"),
        /// The browser's key pressed where there is no chart to pick a layout for.
        NotOnAChart => (
            "LAYOUT PICK opens on a chart, not on a folder, a course or a missing chart",
            "배치 지정은 폴더·코스·없는 차트가 아닌 차트에서 열립니다",
            "配置指定はフォルダ・コース・未所持の譜面ではなく、譜面の上で開きます",
        ),
    }
}

rbms_model::translations! {
    /// Words of capturing a run's layout on a result screen: its key, what came of a capture, why a
    /// run is not captured, and what became of the file the captures are kept in.
    pub(crate) enum CaptureText {
        /// The key that captures a layout, as the key editor and the key guide name it.
        ShortcutLabel => ("CAPTURE LAYOUT", "배치 저장", "配置を保存"),
        /// A layout captured, written as its arrangement and lane order.
        Captured => ("layout captured: {0}", "배치를 저장했습니다: {0}", "配置を保存しました: {0}"),
        /// The same layout of the chart was captured before.
        AlreadyCaptured => ("this layout is captured already: {0}", "이미 저장한 배치입니다: {0}", "この配置は保存済みです: {0}"),
        /// The screen was built without a run.
        RefusedNoRun => ("layout not captured: there is no run on this screen", "배치를 저장하지 않음: 이 화면에는 저장할 플레이가 없음", "配置を保存しません: この画面には保存するプレーがありません"),
        /// AUTOPLAY played the run.
        RefusedAutoplay => ("layout not captured: AUTOPLAY played this run", "배치를 저장하지 않음: AUTOPLAY가 친 플레이", "配置を保存しません: AUTOPLAYのプレーです"),
        /// The run is a replay being watched.
        RefusedReplay => ("layout not captured: this is a replay being watched", "배치를 저장하지 않음: 보고 있는 리플레이", "配置を保存しません: 再生中のリプレイです"),
        /// The run is a stage of a course or a practice slice.
        RefusedCourse => (
            "layout not captured: a course stage or a practice run is not captured",
            "배치를 저장하지 않음: 코스 스테이지와 연습 플레이는 저장하지 않음",
            "配置を保存しません: コースのステージと練習のプレーは保存しません",
        ),
        /// The run was played on OFF.
        RefusedOff => ("layout not captured: the run was played on OFF", "배치를 저장하지 않음: OFF로 친 플레이", "配置を保存しません: OFFのプレーです"),
        /// The arrangement lays the notes out one by one.
        RefusedNoteByNote => (
            "layout not captured: {0} lays the notes out one by one, not as a lane order",
            "배치를 저장하지 않음: {0} 옵션은 레인 순서가 아니라 노트마다 배치함",
            "配置を保存しません: {0}はレーン順ではなくノーツごとに配置します",
        ),
        /// LAYOUT PICK does not lay out charts of the mode.
        RefusedMode => (
            "layout not captured: LAYOUT PICK does not lay out {0} charts",
            "배치를 저장하지 않음: 배치 지정은 {0} 차트를 지원하지 않음",
            "配置を保存しません: 配置指定は{0}の譜面に対応していません",
        ),
        /// The file of the captures could not be read.
        NotLoaded => (
            "captured layouts not loaded ({0}); starting with none",
            "저장한 배치를 불러오지 못했습니다 ({0}). 비어 있는 상태로 시작합니다",
            "保存した配置を読み込めませんでした（{0}）。空の状態で開始します",
        ),
        /// The captures could not be written out.
        SaveFailed => ("captured layouts save failed: {0}", "저장한 배치를 기록하지 못했습니다: {0}", "保存した配置を書き込めませんでした: {0}"),
        /// The same, naming the file.
        SaveFailedAt => (
            "captured layouts save failed ({0}): {1}",
            "저장한 배치를 기록하지 못했습니다 ({0}): {1}",
            "保存した配置を書き込めませんでした（{0}）: {1}",
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
        assert_eq!(catalog_problems(LayoutPickText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(CaptureText::ALL), Vec::<String>::new());
    }

    /// Where the lines of keys over the field start, how large they are drawn, and how wide a
    /// character outside ASCII is in them, in pixels: `TEXT_X`, `HINT_SCALE` and `round(8.5 *
    /// HINT_SCALE)` of `stage::layout_pick`. The fonts of the tests have no Korean or Japanese to
    /// measure them with, so such a character counts as a whole em, which is as wide as the widest
    /// of them is.
    const HINT_LEFT_PX: f32 = 40.0;
    const HINT_SCALE: f32 = 1.0;
    const HINT_FULL_WIDTH_PX: f32 = 9.0;

    fn hint_width(text: &str) -> f32 {
        let latin: String = text.chars().filter(char::is_ascii).collect();
        let full_width = text.chars().filter(|ch| !ch.is_ascii()).count() as f32;
        rbms_render::text_width(&latin, HINT_SCALE) + full_width * HINT_FULL_WIDTH_PX
    }

    #[test]
    fn every_line_of_keys_fits_between_the_margins_of_the_screen_in_every_language() {
        rbms_render::font::use_embedded_fonts_only();
        for language in Language::ALL {
            for line in [LayoutPickText::HintTyping, LayoutPickText::HintCaptured, LayoutPickText::HintMoving] {
                let width = hint_width(line.get(language));
                assert!(width <= crate::CW as f32 - HINT_LEFT_PX * 2.0, "{line:?} {language:?} is {width} wide");
            }
        }
    }

    #[test]
    fn the_second_line_of_keys_names_no_key_the_list_of_captured_layouts_takes_for_itself() {
        for language in Language::ALL {
            assert!(!LayoutPickText::HintMoving.get(language).contains("UP DOWN"), "{language:?}: the second line is drawn while the list has the keys too");
            for line in [LayoutPickText::HintTyping, LayoutPickText::HintCaptured] {
                assert!(line.get(language).contains("UP DOWN"), "{line:?} {language:?} does not say what up and down do");
            }
        }
    }
}
