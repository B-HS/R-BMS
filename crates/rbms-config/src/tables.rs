//! The difficulty tables every player is given without asking, and how each is given once.
//!
//! A table is put into the player's list the first time a build that ships it reads their settings,
//! right after the table shipped before it (in front of the list when that one is not there), and
//! the table's id is written down in [`LibraryOptions::seeded_default_tables`] from then on.
//! Writing the id down is what lets a player remove the table for good: it is never put back,
//! whatever becomes of the list afterwards. A player who already has the same table at another of
//! its addresses is not given a second copy, and the id is written down all the same.
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
    /// The row of the song list's root the table is shown under together with the tables of the
    /// same name, or empty for a row of its own. Copied into the row the table is registered as.
    pub group: &'static str,
}

/// The id of the 発狂BMS難易度表 copy the R-BMS site serves.
pub const INSANE_TABLE_ID: &str = "insane1";

/// The id of the Stella table of stellabms.xyz.
pub const STELLA_TABLE_ID: &str = "stella-st";

/// The id of the Satellite table of stellabms.xyz.
pub const SATELLITE_TABLE_ID: &str = "stella-sl";

/// The row the two tables of stellabms.xyz are shown under.
pub const STELLA_GROUP: &str = "STELLA";

/// Every table a player is given, in the order they are listed. The 発狂BMS難易度表 is an archive that
/// no longer changes; the site serves the snapshot the repository keeps of it, so the player is not
/// left without it when the original site is down. The two tables of stellabms.xyz change every
/// couple of weeks and are always fetched from the original site, so the player carries no copy of
/// either.
pub const DEFAULT_TABLES: &[DefaultTable] = &[
    DefaultTable { id: INSANE_TABLE_ID, location: "https://bms.hyuns.uk/tables/insane1", same_as: &["darksabun.club/table/archive/insane1"], group: "" },
    DefaultTable {
        id: STELLA_TABLE_ID,
        location: "https://stellabms.xyz/st/table.html",
        same_as: &["stellabms.xyz/st", "stellabms.xyz/st/score.json"],
        group: STELLA_GROUP,
    },
    DefaultTable {
        id: SATELLITE_TABLE_ID,
        location: "https://stellabms.xyz/sl/table.html",
        same_as: &["stellabms.xyz/sl", "stellabms.xyz/sl/score.json"],
        group: STELLA_GROUP,
    },
];

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
        TableSource { name: String::new(), location: self.location.to_string(), group: self.group.to_string() }
    }
}

impl LibraryOptions {
    /// Give the player every table of `tables` they have not been given yet, and answer the ones
    /// that were put into the list, in the order `tables` has them. A table goes right after the
    /// nearest table before it in `tables` that the list holds, or in front of the list when it
    /// holds none of them; one the list holds already at another address is only written down as
    /// given.
    pub fn seed_default_tables(&mut self, tables: &'static [DefaultTable]) -> Vec<&'static DefaultTable> {
        let mut added = Vec::new();
        for (index, table) in tables.iter().enumerate() {
            if self.seeded_default_tables.iter().any(|id| id == table.id) {
                continue;
            }
            self.seeded_default_tables.push(table.id.to_string());
            if self.tables.iter().any(|source| table.is_at(&source.location)) {
                continue;
            }
            let after = tables[..index].iter().rev().find_map(|earlier| self.tables.iter().position(|source| earlier.is_at(&source.location)));
            self.tables.insert(after.map_or(0, |position| position + 1), table.source());
            added.push(table);
        }
        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTHER: DefaultTable = DefaultTable { id: "other", location: "https://tables.test/other", same_as: &[], group: "" };

    const INSANE_LOCATION: &str = "https://bms.hyuns.uk/tables/insane1";
    const STELLA_LOCATION: &str = "https://stellabms.xyz/st/table.html";
    const SATELLITE_LOCATION: &str = "https://stellabms.xyz/sl/table.html";
    const MINE_LOCATION: &str = "https://tables.test/mine";

    fn library_with(locations: &[&str]) -> LibraryOptions {
        let tables = locations.iter().map(|location| TableSource { name: String::new(), location: (*location).to_string(), group: String::new() }).collect();
        LibraryOptions { tables, ..LibraryOptions::default() }
    }

    fn locations_of(library: &LibraryOptions) -> Vec<&str> {
        library.tables.iter().map(|source| source.location.as_str()).collect()
    }

    fn every_default_id() -> Vec<String> {
        DEFAULT_TABLES.iter().map(|table| table.id.to_string()).collect()
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
        assert!(!insane.is_at(SATELLITE_LOCATION));
    }

    #[test]
    fn stella_and_satellite_are_known_at_their_page_header_and_body() {
        let [_, stella, satellite] = DEFAULT_TABLES else { panic!("the default tables are the insane table, Stella and Satellite") };
        for written in [
            "https://stellabms.xyz/st/table.html",
            "https://stellabms.xyz/st/",
            "https://stellabms.xyz/st",
            "https://stellabms.xyz/st/header.json",
            "https://stellabms.xyz/st/score.json",
            "http://STELLABMS.xyz/st/score.json",
            "  https://stellabms.xyz/st/index.html  ",
        ] {
            assert!(stella.is_at(written), "{written}");
            assert!(!satellite.is_at(written), "{written} is not Satellite");
        }
        for written in [SATELLITE_LOCATION, "https://stellabms.xyz/sl/", "https://stellabms.xyz/sl/header.json", "https://stellabms.xyz/sl/score.json"] {
            assert!(satellite.is_at(written), "{written}");
            assert!(!stella.is_at(written), "{written} is not Stella");
        }
        assert!(!stella.is_at("https://stellabms.xyz/"));
        assert!(!stella.is_at("https://stellabms.xyz/st2/table.html"));
        assert!(!stella.is_at(INSANE_LOCATION));
    }

    #[test]
    fn the_default_ids_and_addresses_are_the_released_ones() {
        let released: Vec<_> = DEFAULT_TABLES.iter().map(|table| (table.id, table.location, table.group)).collect();
        assert_eq!(
            released,
            vec![
                ("insane1", "https://bms.hyuns.uk/tables/insane1", ""),
                ("stella-st", "https://stellabms.xyz/st/table.html", "STELLA"),
                ("stella-sl", "https://stellabms.xyz/sl/table.html", "STELLA"),
            ]
        );
        assert_eq!((INSANE_TABLE_ID, STELLA_TABLE_ID, SATELLITE_TABLE_ID, STELLA_GROUP), ("insane1", "stella-st", "stella-sl", "STELLA"));
    }

    #[test]
    fn a_default_table_is_registered_with_its_group() {
        assert_eq!(DEFAULT_TABLES[0].source(), TableSource { name: String::new(), location: INSANE_LOCATION.into(), group: String::new() });
        assert_eq!(DEFAULT_TABLES[1].source(), TableSource { name: String::new(), location: STELLA_LOCATION.into(), group: STELLA_GROUP.into() });
        assert_eq!(DEFAULT_TABLES[2].source(), TableSource { name: String::new(), location: SATELLITE_LOCATION.into(), group: STELLA_GROUP.into() });
    }

    #[test]
    fn a_new_player_is_given_every_default_table_once_in_order() {
        let mut library = library_with(&[MINE_LOCATION]);
        let added = library.seed_default_tables(DEFAULT_TABLES);
        assert_eq!(added, DEFAULT_TABLES.iter().collect::<Vec<_>>());
        assert_eq!(locations_of(&library), vec![INSANE_LOCATION, STELLA_LOCATION, SATELLITE_LOCATION, MINE_LOCATION]);
        let groups: Vec<_> = library.tables.iter().map(|source| source.group.as_str()).collect();
        assert_eq!(groups, vec!["", STELLA_GROUP, STELLA_GROUP, ""], "the player's own table stays as it was, after the given ones");
        assert_eq!(library.seeded_default_tables, every_default_id());

        assert!(library.seed_default_tables(DEFAULT_TABLES).is_empty(), "a table is given once");
        assert_eq!(library.tables.len(), 4);
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
    fn a_player_given_the_insane_table_before_is_given_stella_after_it() {
        let mut library = library_with(&[INSANE_LOCATION, MINE_LOCATION]);
        library.seeded_default_tables = vec![INSANE_TABLE_ID.to_string()];
        let added = library.seed_default_tables(DEFAULT_TABLES);
        assert_eq!(added, vec![&DEFAULT_TABLES[1], &DEFAULT_TABLES[2]]);
        assert_eq!(locations_of(&library), vec![INSANE_LOCATION, STELLA_LOCATION, SATELLITE_LOCATION, MINE_LOCATION]);
        assert_eq!(library.seeded_default_tables, every_default_id());
    }

    #[test]
    fn a_new_table_goes_in_front_when_no_table_shipped_before_it_is_in_the_list() {
        let mut library = library_with(&[MINE_LOCATION]);
        library.seeded_default_tables = vec![INSANE_TABLE_ID.to_string()];
        library.seed_default_tables(DEFAULT_TABLES);
        assert_eq!(
            locations_of(&library),
            vec![STELLA_LOCATION, SATELLITE_LOCATION, MINE_LOCATION],
            "the insane table was removed, so there is nothing to go after"
        );
    }

    #[test]
    fn a_table_goes_after_the_nearest_earlier_table_the_list_still_holds() {
        let mut library = library_with(&[INSANE_LOCATION, MINE_LOCATION]);
        library.seeded_default_tables = vec![INSANE_TABLE_ID.to_string(), STELLA_TABLE_ID.to_string()];
        let added = library.seed_default_tables(DEFAULT_TABLES);
        assert_eq!(added, vec![&DEFAULT_TABLES[2]], "Stella was removed and stays removed");
        assert_eq!(locations_of(&library), vec![INSANE_LOCATION, SATELLITE_LOCATION, MINE_LOCATION]);
    }

    #[test]
    fn a_player_who_has_the_original_is_not_given_the_copy() {
        let mut library = library_with(&["https://darksabun.club/table/archive/insane1/"]);
        let added = library.seed_default_tables(DEFAULT_TABLES);
        assert_eq!(added, vec![&DEFAULT_TABLES[1], &DEFAULT_TABLES[2]], "only the two Stella tables are given");
        assert_eq!(
            locations_of(&library),
            vec!["https://darksabun.club/table/archive/insane1/", STELLA_LOCATION, SATELLITE_LOCATION],
            "the original stays the only insane table, and the new ones follow it"
        );
        assert_eq!(library.seeded_default_tables, every_default_id(), "the insane table counts as given");
    }

    #[test]
    fn a_player_who_registered_stella_by_hand_is_not_given_a_second_copy() {
        let mut library = library_with(&["https://stellabms.xyz/st/"]);
        let added = library.seed_default_tables(DEFAULT_TABLES);
        assert_eq!(added, vec![&DEFAULT_TABLES[0], &DEFAULT_TABLES[2]], "Stella is only written down as given");
        assert_eq!(locations_of(&library), vec![INSANE_LOCATION, "https://stellabms.xyz/st/", SATELLITE_LOCATION]);
        assert_eq!(library.tables[1].group, "", "the player's own row is left as it was");
        assert_eq!(library.seeded_default_tables, every_default_id());
    }

    #[test]
    fn a_table_shipped_later_is_given_without_touching_the_ones_given_before() {
        static LATER: [DefaultTable; 4] = [DEFAULT_TABLES[0], DEFAULT_TABLES[1], DEFAULT_TABLES[2], OTHER];
        let mut library = LibraryOptions::default();
        library.seed_default_tables(DEFAULT_TABLES);
        library.tables.clear();
        let added = library.seed_default_tables(&LATER);
        assert_eq!(added, vec![&OTHER], "only the new table is given");
        assert_eq!(library.tables, vec![OTHER.source()]);
        assert_eq!(library.seeded_default_tables, every_default_id().into_iter().chain(["other".to_string()]).collect::<Vec<_>>());
    }
}
