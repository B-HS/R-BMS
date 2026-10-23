//! Fixed words of playing a chart laid out as a replay of it was (LAYOUT PLAY), by language.

rbms_model::translations! {
    /// Words of LAYOUT PLAY: its key, the target a run on a borrowed layout is held against, and
    /// why a layout is not borrowed.
    pub(crate) enum LayoutPlayText {
        /// The key that plays the chart of the replay being watched on the replay's layout, as the
        /// key editor names it.
        ShortcutLabel => ("PLAY THIS LAYOUT", "이 배치로 플레이", "この配置でプレー"),
        /// The target of a run on a layout borrowed from a run of this machine: that run, by when it
        /// was played.
        TargetName => ("REPLAY {0}", "리플레이 {0}", "リプレイ {0}"),
        /// Where a layout came from when the replay being watched is not one this machine recorded.
        WatchedReplay => ("the replay being watched", "보던 리플레이", "再生中のリプレイ"),
        /// A replay this machine did not record shows OFF, which it shows for CONVERGE as well.
        ArrangementUnknown => (
            "layout not borrowed: a replay not recorded on this machine shows OFF for CONVERGE too, so its arrangement is not known",
            "배치를 빌리지 않음: 이 기기에서 기록하지 않은 리플레이의 OFF 표시는 CONVERGE 배치일 수도 있어 배치를 알 수 없음",
            "配置を借りません: この機器で記録していないリプレイのOFF表示はCONVERGEの場合もあるため、配置が分かりません"
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert!(catalog_problems(LayoutPlayText::ALL).is_empty(), "{:?}", catalog_problems(LayoutPlayText::ALL));
    }
}
