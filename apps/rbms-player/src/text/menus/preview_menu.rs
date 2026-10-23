//! Fixed words the PLAY and SKIN menus share, by language.

rbms_model::translations! {
    /// What a row that shows or hides other rows reads, before how many they are.
    pub(crate) enum PreviewMenuText {
        FoldShown => ("SHOWN", "표시 중", "表示中"),
        FoldHidden => ("HIDDEN", "숨김", "非表示"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(PreviewMenuText::ALL), Vec::<String>::new());
    }
}
