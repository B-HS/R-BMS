//! Fixed words of the IR lines of the result screen and of the record modal, by language.
//!
//! A line is told in the language the screen is in when the score server answers, and the result screen keeps it as it was told. The leads that
//! colour a line (`UNRANKED`, `REPLAY ERROR`) are read back from a line told in any of the languages. The reason a run was skipped is one of the four
//! the program gives ([`SkipReasonText`]); the reasons it was left out of the ranking and the text of an error are the server's or the player's own
//! and stay as they come. `IR: OFF` and `NEW BEST` are
//! terms every language keeps, and are not here. The record modal's lines say where one stored play stands with the server in the same words the
//! result screen says it in (`REJECTED`, `SENT`), and are told afresh each time the modal is drawn.

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
        /// The summary of a fan-out to several score servers when there is none to send to, and what it says of one that answered without taking the score.
        NoProfiles => ("no IR profiles", "IR 프로필 없음", "IRプロファイルなし"),
        ProfileRejected => ("rejected", "거부됨", "拒否"),
    }
}

rbms_model::translations! {
    /// Words of the line of the record modal that says where one stored play stands with the score server.
    pub(crate) enum RecordIrLineText {
        NotSentYet => ("IR: NOT SENT YET", "IR: 아직 전송하지 않음", "IR: 未送信"),
        NotSentNotSignedIn => ("IR: NOT SENT (NOT SIGNED IN)", "IR: 전송하지 않음 (로그인하지 않음)", "IR: 未送信（未ログイン）"),
        /// The play is on the server, under another account than the one signed in now.
        OnServerAs => ("IR: ON SERVER (AS {0})", "IR: 서버에 있음 (계정 {0})", "IR: サーバーに登録済み（アカウント{0}）"),
        OnServer => ("IR: ON SERVER", "IR: 서버에 있음", "IR: サーバーに登録済み"),
        /// The server refused the play; the code, the field and the reason it gave follow.
        Rejected => ("IR: REJECTED {0}", "IR: 거부됨 {0}", "IR: 拒否 {0}"),
        NotSentWillRetry => ("IR: NOT SENT YET (WILL RETRY)", "IR: 아직 전송하지 않음 (재시도 예정)", "IR: 未送信（再試行予定）"),
        NotSentOldRule => ("IR: NOT SENT (OLD SCRATCH AUTO RULE)", "IR: 전송하지 않음 (이전 SCRATCH AUTO 규칙)", "IR: 未送信（旧SCRATCH AUTOルール）"),
        NotSentChartNotInLibrary => (
            "IR: NOT SENT (CHART NOT IN LIBRARY)",
            "IR: 전송하지 않음 (라이브러리에 없는 차트)",
            "IR: 未送信（ライブラリにない譜面）",
        ),
    }
}

rbms_model::translations! {
    /// The reasons the program itself gives for not sending a run, which the result screen puts in parentheses after `IR: SKIPPED`. The program
    /// names them in English where it decides ([`crate::ir_outcome::skip_reason_in`] writes them in a language).
    pub(crate) enum SkipReasonText {
        Autoplay => ("autoplay", "AUTOPLAY 사용", "AUTOPLAY使用"),
        ReplayPlayback => ("replay playback", "리플레이 재생", "リプレイ再生"),
        Practice => ("practice", "연습", "練習"),
        NotSignedIn => ("not logged in", "로그인하지 않음", "未ログイン"),
        PlaybackRate => ("playback rate", "재생 속도 변경", "再生速度変更"),
    }
}

rbms_model::translations! {
    /// What the result screen says under the run of what an assist did to it: kept and sent like any other run, but no best score. Each fits a
    /// line of the built-in screen's column in every language.
    pub(crate) enum AssistNoteText {
        ScratchAuto => (
            "SCRATCH AUTO: TURNTABLE NOT COUNTED, ASSIST LAMP",
            "SCRATCH AUTO: 턴테이블 제외, 어시스트 램프",
            "SCRATCH AUTO: ターンテーブル非集計、アシストランプ",
        ),
        WidenedJudge => (
            "WIDENED JUDGE: SENT AND KEPT, BUT NO BEST SCORE",
            "판정 폭 확대: 전송·보관됨, 최고 스코어 없음",
            "判定幅拡大: 送信・保管済み、ベストスコアなし",
        ),
        OwnTotal => (
            "OWN TOTAL: SENT AND KEPT, BUT NO BEST SCORE",
            "자체 TOTAL: 전송·보관됨, 최고 스코어 없음",
            "独自TOTAL: 送信・保管済み、ベストスコアなし",
        ),
        /// A run played at another PLAYBACK RATE than the chart's own, at the rate in percent: nothing of it is kept.
        PlaybackRate => (
            "PLAYBACK RATE {0}%: NOT SAVED, NOT SENT",
            "재생 속도 {0}%: 저장·전송 안 됨",
            "再生速度{0}%: 保存・送信なし",
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
        assert_eq!(catalog_problems(IrOutcomeText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(RecordIrLineText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(AssistNoteText::ALL), Vec::<String>::new());
        assert_eq!(catalog_problems(SkipReasonText::ALL), Vec::<String>::new());
    }

    #[test]
    fn an_assist_note_fits_a_line_of_the_result_screen_in_every_language() {
        for note in AssistNoteText::ALL {
            for language in Language::ALL {
                let text = note.get(language);
                assert!(text.chars().count() <= crate::ir_outcome::IR_LINE_MAX_CHARS, "{note:?} {language:?} is too long: {text}");
            }
        }
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
