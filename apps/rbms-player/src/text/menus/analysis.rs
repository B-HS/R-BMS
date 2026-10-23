//! Fixed words of the bar a replay under analysis is drawn with, by language.
//!
//! The keys are named as the key config file names them, and the figures and units (`x`, `S`) are written as they are in every language.

rbms_model::translations! {
    /// What the analysis bar says of the replay: whether it is paused, where it is, and the keys that move it.
    pub(crate) enum AnalysisText {
        Paused => ("PAUSED", "일시정지", "一時停止"),
        /// The state (`PAUSED` or the rate), then the position and the length of the replay in seconds.
        Status => ("ANALYSIS  {0}   {1} / {2} S", "분석  {0}   {1} / {2} S", "分析  {0}   {1} / {2} S"),
        Keys => (
            "SPACE PAUSE  -/+ SPEED  PGUP/PGDN SEEK  ESC EXIT",
            "SPACE 일시정지  -/+ 속도  PGUP/PGDN 탐색  ESC 나가기",
            "SPACE 一時停止  -/+ 速度  PGUP/PGDN シーク  ESC 終了",
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(AnalysisText::ALL), Vec::<String>::new());
    }
}
