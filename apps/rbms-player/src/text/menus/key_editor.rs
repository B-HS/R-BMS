//! Fixed words of the key editor's own headings, by language.
//!
//! The headings of the shortcut groups and the names of the lines are the key configuration's own and are not here.

rbms_model::translations! {
    /// The headings of the key editor's lines that are not shortcuts.
    pub(crate) enum KeyEditorText {
        Lanes => ("LANES", "레인", "レーン"),
        Play => ("PLAY", "플레이", "プレー"),
        Controller => ("CONTROLLER", "컨트롤러", "コントローラー"),
        Defaults => ("DEFAULTS", "기본값", "初期設定"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(KeyEditorText::ALL), Vec::<String>::new());
    }
}
