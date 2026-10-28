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
        /// The keys that write the layout.
        HintTyping => (
            "1-9 OR LANE KEYS TYPE THE LANE FOR THE NEXT PLACE   BACKSPACE ERASE   DELETE CLEAR   0 IN ORDER   TAB SIDE",
            "1-9 또는 레인 키로 다음 자리에 올 레인 입력   BACKSPACE 지우기   DELETE 비우기   0 순서대로   TAB 쪽 전환",
            "1-9 またはレーンキーで次の位置のレーンを入力   BACKSPACE 消去   DELETE 空にする   0 順番どおり   TAB サイド切替",
        ),
        /// The keys that move the preview and leave the screen.
        HintMoving => (
            "UP DOWN MEASURE   PAGE UP PAGE DOWN 4 MEASURES   HOME END START END   ENTER PLAY   ESC BACK",
            "UP DOWN 한 마디   PAGE UP PAGE DOWN 네 마디   HOME END 처음 끝   ENTER 플레이   ESC 뒤로",
            "UP DOWN 1小節   PAGE UP PAGE DOWN 4小節   HOME END 最初 最後   ENTER プレー   ESC 戻る",
        ),
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

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(LayoutPickText::ALL), Vec::<String>::new());
    }
}
