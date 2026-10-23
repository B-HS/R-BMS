//! The difficulty tables every player is given without asking, and how each is given once.
//!
//! A table is put in front of the player's list the first time a build that ships it reads their
//! settings, and the table's id is written down in [`LibraryOptions::seeded_default_tables`] from
//! then on. Writing the id down is what lets a player remove the table for good: it is never put
//! back, whatever becomes of the list afterwards. A player who already has the same table at
//! another of its addresses is not given a second copy, and the id is written down all the same.
//!
//! The list of tables is left as it is in [`LibraryOptions::default`]: that value is what the tests,
//! the blanking of an upload and serde's filling of an absent field all start from, and a table
//! there would be fetched by every test and put back into every file whose list was emptied by
//! hand.

use crate::schema::{LibraryOptions, TableSource};

/// One table every player is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefaultTable {
    /// What the table is remembered by once it has been given. Never changed after a release:
    /// a new id is a new table, given again to every player.
    pub id: &'static str,
    /// The address the table is registered at. The copy kept of a table is named after its
    /// address, so this is never changed after a release either.
    pub location: &'static str,
    /// Other addresses of the same table, written as [`table_address_key`] writes them. A player
    /// whose list holds one of these has the table already.
    pub same_as: &'static [&'static str],
}

/// The id of the 発狂BMS難易度表 copy the R-BMS site serves.
pub const INSANE_TABLE_ID: &str = "insane1";

/// Every table a player is given. The 発狂BMS難易度表 is an archive that no longer changes; the site
/// serves the snapshot the repository keeps of it, so the player is not left without it when the
/// original site is down.
pub const DEFAULT_TABLES: &[DefaultTable] =
    &[DefaultTable { id: INSANE_TABLE_ID, location: "https://bms.hyuns.uk/tables/insane1", same_as: &["darksabun.club/table/archive/insane1"] }];

/// The parts an address of a table can end with that name the same table: its page, and the two
/// files a table is served as.
const SAME_TABLE_SUFFIXES: [&str; 3] = ["/index.html", "/header.json", "/data.json"];

/// The schemes an address can be written with that reach the same table.
const SCHEMES: [&str; 2] = ["https://", "http://"];

/// An address as it is compared with another: without its scheme, in lower case, and without a
/// trailing slash or a file name that every address of a table may or may not be written with.
pub fn table_address_key(location: &str) -> String {
    let lowered = location.trim().to_ascii_lowercase();
    let mut key = SCHEMES.iter().find_map(|scheme| lowered.strip_prefix(scheme)).unwrap_or(&lowered).to_string();
    loop {
        let trimmed = key.trim_end_matches('/');
        let trimmed = SAME_TABLE_SUFFIXES.iter().find_map(|suffix| trimmed.strip_suffix(suffix)).unwrap_or(trimmed);
        if trimmed.len() == key.len() {
            return key;
        }
        key = trimmed.to_string();
    }
}

impl DefaultTable {
    /// Whether the table registered at `location` is this one.
    pub fn is_at(&self, location: &str) -> bool {
        let key = table_address_key(location);
        key == table_address_key(self.location) || self.same_as.contains(&key.as_str())
    }

    /// The row this table is registered as. The name is left empty, so the list shows the name the
    /// table gives itself.
    pub fn source(&self) -> TableSource {
        TableSource { name: String::new(), location: self.location.to_string(), group: String::new() }
    }
}

impl LibraryOptions {
    /// Give the player every table of `tables` they have not been given yet, and answer the ones
    /// that were put into the list. A table goes in front of the list, in the order `tables` has
    /// them; one the list holds already at another address is only written down as given.
    pub fn seed_default_tables(&mut self, tables: &'static [DefaultTable]) -> Vec<&'static DefaultTable> {
        let mut added = Vec::new();
        for table in tables {
            if self.seeded_default_tables.iter().any(|id| id == table.id) {
                continue;
            }
            self.seeded_default_tables.push(table.id.to_string());
            if !self.tables.iter().any(|source| table.is_at(&source.location)) {
                added.push(table);
            }
        }
        let front: Vec<TableSource> = added.iter().map(|table| table.source()).collect();
        self.tables.splice(0..0, front);
        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTHER: DefaultTable = DefaultTable { id: "other", location: "https://tables.test/other", same_as: &[] };

    fn library_with(locations: &[&str]) -> LibraryOptions {
        let tables = locations.iter().map(|location| TableSource { name: String::new(), location: (*location).to_string(), group: String::new() }).collect();
        LibraryOptions { tables, ..LibraryOptions::default() }
    }

    #[test]
    fn an_address_is_compared_without_its_scheme_case_slash_or_file() {
        for written in [
            "https://darksabun.club/table/archive/insane1/",
            "http://DARKSABUN.club/table/archive/insane1",
            "https://darksabun.club/table/archive/insane1/header.json",
            "https://darksabun.club/table/archive/insane1/data.json",
            "https://darksabun.club/table/archive/insane1/index.html",
            "  https://darksabun.club/table/archive/insane1//  ",
        ] {
            assert_eq!(table_address_key(written), "darksabun.club/table/archive/insane1", "{written}");
        }
        assert_eq!(table_address_key("/local/insane.json"), "/local/insane.json", "a file keeps its name");
    }

    #[test]
    fn the_insane_table_is_known_at_its_copy_and_at_the_original() {
        let insane = &DEFAULT_TABLES[0];
        assert_eq!(insane.id, INSANE_TABLE_ID);
        assert!(insane.is_at("https://bms.hyuns.uk/tables/insane1"));
        assert!(insane.is_at("https://bms.hyuns.uk/tables/insane1/"));
        assert!(insane.is_at("https://bms.hyuns.uk/tables/insane1/header.json"));
        assert!(insane.is_at("https://darksabun.club/table/archive/insane1/"));
        assert!(insane.is_at("http://darksabun.club/table/archive/insane1/data.json"));
        assert!(!insane.is_at("https://darksabun.club/table/archive/insane2/"));
        assert!(!insane.is_at("https://stellabms.xyz/sl/table.html"));
    }

    #[test]
    fn a_new_player_is_given_the_table_once_in_front_of_the_list() {
        let mut library = library_with(&["https://tables.test/mine"]);
        let added = library.seed_default_tables(DEFAULT_TABLES);
        assert_eq!(added, vec![&DEFAULT_TABLES[0]]);
        assert_eq!(library.tables[0], TableSource { name: String::new(), location: "https://bms.hyuns.uk/tables/insane1".into(), group: String::new() });
        assert_eq!(library.tables[1].location, "https://tables.test/mine", "the player's own table stays, after it");
        assert_eq!(library.seeded_default_tables, vec![INSANE_TABLE_ID.to_string()]);

        assert!(library.seed_default_tables(DEFAULT_TABLES).is_empty(), "a table is given once");
        assert_eq!(library.tables.len(), 2);
    }

    #[test]
    fn a_table_the_player_removed_is_not_put_back() {
        let mut library = LibraryOptions::default();
        library.seed_default_tables(DEFAULT_TABLES);
        library.tables.clear();
        assert!(library.seed_default_tables(DEFAULT_TABLES).is_empty());
        assert!(library.tables.is_empty(), "the removed table came back");
    }

    #[test]
    fn a_player_who_has_the_original_is_not_given_the_copy() {
        let mut library = library_with(&["https://darksabun.club/table/archive/insane1/"]);
        assert!(library.seed_default_tables(DEFAULT_TABLES).is_empty());
        assert_eq!(library.tables.len(), 1, "the original stays the only one");
        assert_eq!(library.seeded_default_tables, vec![INSANE_TABLE_ID.to_string()], "the table counts as given");
    }

    #[test]
    fn a_table_shipped_later_is_given_without_touching_the_ones_given_before() {
        static LATER: [DefaultTable; 2] = [DEFAULT_TABLES[0], OTHER];
        let mut library = LibraryOptions::default();
        library.seed_default_tables(DEFAULT_TABLES);
        library.tables.clear();
        let added = library.seed_default_tables(&LATER);
        assert_eq!(added, vec![&OTHER], "only the new table is given");
        assert_eq!(library.tables, vec![OTHER.source()]);
        assert_eq!(library.seeded_default_tables, vec![INSANE_TABLE_ID.to_string(), "other".to_string()]);
    }
}
