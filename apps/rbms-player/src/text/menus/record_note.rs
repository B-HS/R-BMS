//! Fixed words written beside a record, by language.

rbms_model::translations! {
    /// What is written beside a record's timestamp in the record-detail modal.
    pub(crate) enum RecordNoteText {
        /// The record was judged under a rule version this build no longer produces.
        OldRule => ("   OLD RULE", "   이전 규칙", "   旧ルール"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language() {
        assert_eq!(catalog_problems(RecordNoteText::ALL), Vec::<String>::new());
    }
}
