//! Fixed words of the IR lines of the result screen, by language.
//!
//! A line is told in the language the screen is in when the score server answers, and the result screen keeps it as it was told. The leads that
//! colour a line (`UNRANKED`, `REPLAY ERROR`) are read back from a line told in any of the languages. The reason a run was skipped, the reasons it
//! was left out of the ranking and the text of an error are the server's or the player's own and stay as they come. `IR: OFF` and `NEW BEST` are
//! terms every language keeps, and are not here.

rbms_model::translations! {
    /// Words of the IR lines of the result screen.
    pub(crate) enum IrOutcomeText {
        /// The run was never eligible; the reason follows in parentheses.
        Skipped => ("IR: SKIPPED ({0})", "IR: 건너뜀 ({0})", "IR: スキップ（{0}）"),
        Sending => ("IR: SENDING...", "IR: 전송 중...", "IR: 送信中..."),
        /// The lead of the line of a transport or server failure; the text of the error follows the colon.
        ErrorLead => ("IR ERROR", "IR 오류", "IRエラー"),
        RejectedWithReason => ("IR: REJECTED ({0})", "IR: 거부됨 ({0})", "IR: 拒否（{0}）"),
        Rejected => ("IR: REJECTED", "IR: 거부됨", "IR: 拒否"),
        /// The lead of the line of a submission the server kept out of the ranking; its reasons follow the colon.
        UnrankedLead => ("UNRANKED", "랭킹 제외", "ランキング外"),
        Rank => ("IR: RANK #{0}", "IR: {0}위", "IR: {0}位"),
        /// The placement of a SCRATCH AUTO run, whose rank is its place on the list that includes SCRATCH AUTO runs.
        RankScratchAuto => ("IR: RANK #{0} (SCRATCH AUTO)", "IR: {0}위 (SCRATCH AUTO)", "IR: {0}位 (SCRATCH AUTO)"),
        Sent => ("IR: SENT", "IR: 전송 완료", "IR: 送信済み"),
        ReplayUploaded => ("REPLAY UPLOADED", "리플레이 업로드 완료", "リプレイ送信済み"),
        /// The lead of the line of a failed replay upload; the text of the error follows the colon.
        ReplayErrorLead => ("REPLAY ERROR", "리플레이 오류", "リプレイエラー"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(IrOutcomeText::ALL), Vec::<String>::new());
    }

    #[test]
    fn no_lead_is_the_start_of_another_languages_lead() {
        for lead in [IrOutcomeText::ErrorLead, IrOutcomeText::UnrankedLead, IrOutcomeText::ReplayErrorLead] {
            for language in Language::ALL {
                for other in [IrOutcomeText::ErrorLead, IrOutcomeText::UnrankedLead, IrOutcomeText::ReplayErrorLead] {
                    for other_language in Language::ALL {
                        let same = lead == other && language == other_language;
                        assert!(
                            same || !lead.get(language).starts_with(other.get(other_language)),
                            "{lead:?} {language:?} starts with {other:?} {other_language:?}"
                        );
                    }
                }
            }
        }
    }
}
