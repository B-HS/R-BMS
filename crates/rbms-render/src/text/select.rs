//! Fixed words of the built-in song select, by language. Only the words of the record modal's layout
//! button are here so far; the rest are moved here as the select screen's words are.

rbms_model::translations! {
    /// Words of the record modal's PLAY THIS LAYOUT button and the key that runs it.
    pub(crate) enum SelectText {
        /// The button that plays the chart laid out as the shown record's replay was.
        PlayThisLayout => ("PLAY THIS LAYOUT", "이 배치로 플레이", "この配置でプレー"),
        /// The key that runs the button, in the line of keys under the record: the key first.
        ModalLayoutKey => ("J  PLAY THIS LAYOUT", "J  이 배치로 플레이", "J  この配置でプレー"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(SelectText::ALL), Vec::<String>::new());
    }
}
