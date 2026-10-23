//! Fixed words of the folders the browser opens, by language: the names of the folders it makes itself.
//!
//! The name of a difficulty table, of a level of one and of one of the player's own folders is data. So is the prefix `LV` a level is
//! titled with, which every language keeps. The folder of the courses a table lists is named as the course tab is
//! ([`crate::text::menus::course`]).

rbms_model::translations! {
    /// The names of the folders of the song list.
    pub(crate) enum TreeText {
        /// The folder that holds every chart.
        AllSongs => ("ALL SONGS", "모든 곡", "すべての曲"),
        /// The folder of the player's own folders, which the web site holds.
        MyFolders => ("MY FOLDERS", "내 폴더", "自分のフォルダ"),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;
    use rbms_model::translations::{Catalog, catalog_problems};

    use super::*;

    #[test]
    fn every_entry_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(TreeText::ALL), Vec::<String>::new());
    }

    #[test]
    fn the_english_words_are_the_names_the_folders_always_had() {
        assert_eq!(TreeText::AllSongs.get(Language::English), "ALL SONGS");
        assert_eq!(TreeText::MyFolders.get(Language::English), "MY FOLDERS");
    }
}
