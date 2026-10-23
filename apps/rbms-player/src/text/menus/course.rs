//! Fixed words of the course tab of the song list and of the course result lines, by language.
//!
//! The badges of a course's constraints (GRADE, NO SPEED, LR2 GAUGE, ...) are the names the difficulty tables publish them under, and the name of a
//! course and its trophies are data; none of them is here.

rbms_model::translations! {
    /// Words of the course tab, the stage line and the rows of the course result.
    pub(crate) enum CourseText {
        TabSongs => ("SONGS", "곡", "曲"),
        TabCourses => ("COURSE", "코스", "コース"),
        /// The badge of a course whose file marks it as not for release.
        Unreleased => ("UNRELEASED", "미공개", "未公開"),
        Stages => ("{0} STAGES", "스테이지 {0}개", "{0}ステージ"),
        StagesMissing => ("{0} STAGES - {1} MISSING", "스테이지 {0}개 - {1}개 없음", "{0}ステージ - {1}件不足"),
        /// The stage line the loading and play screens show during a course.
        StageProgress => ("STAGE {0} / {1}", "스테이지 {0} / {1}", "ステージ {0} / {1}"),
        StageNumber => ("STAGE {0}", "스테이지 {0}", "ステージ {0}"),
        ScoreRate => ("SCORE RATE", "스코어 비율", "スコア率"),
        Gauge => ("GAUGE", "게이지", "ゲージ"),
        GaugeRule => ("GAUGE RULE", "게이지 규칙", "ゲージルール"),
        FailedAt => ("FAILED AT", "실패 지점", "失敗箇所"),
        Trophy => ("TROPHY", "트로피", "トロフィー"),
        /// The headings of the built-in course result, and the keys along its foot.
        ResultCleared => ("COURSE CLEAR", "코스 클리어", "コースクリア"),
        ResultFailed => ("COURSE FAILED", "코스 실패", "コース失敗"),
        ResultBack => ("ENTER / ESC  BACK TO SELECT", "ENTER / ESC  선곡으로 돌아가기", "ENTER / ESC  選曲に戻る"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(CourseText::ALL), Vec::<String>::new());
    }

    #[test]
    fn a_count_is_put_where_each_language_wants_it() {
        assert_eq!(CourseText::StagesMissing.fill(Language::English, &[&4, &1]), "4 STAGES - 1 MISSING");
        assert_eq!(CourseText::StagesMissing.fill(Language::Korean, &[&4, &1]), "스테이지 4개 - 1개 없음");
    }
}
