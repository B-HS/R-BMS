//! The difficulty tables every player is given, and the copies of them the player carries.
//!
//! Which tables are given, and the rule that gives each once, are the settings' own
//! ([`rbms_config::LibraryOptions::seed_default_tables`]). What is here is the table itself: the
//! repository keeps a snapshot of the 発狂BMS難易度表 in the files the site serves it from
//! (`web/data/tables/<id>/header.json` and `data.json`), the binary carries those two files, and a
//! table is kept as its copy ([`TableStore::plant`]) the moment it is given. The first run of the
//! player shows that table's folder without asking the network anything, and with the site down;
//! asking for the table again (F6, R or A in the table manager) fetches it from its address as it
//! does any other table.
//!
//! The two tables of stellabms.xyz are not carried: they change every couple of weeks and are always
//! fetched from the original site, so their folders are there once the first fetch has landed.
//!
//! `include_bytes!` makes the two files part of what the binary is built from, so a build after the
//! snapshot changed carries the new one.

use std::path::Path;

use rbms_config::{Config, DEFAULT_TABLES, DefaultTable, INSANE_TABLE_ID};
use rbms_table::{DifficultyTable, parse_header};

use super::TableStore;

/// One table the binary carries: the id it is given under, and its header and body as the site
/// serves them.
struct Snapshot {
    id: &'static str,
    header: &'static [u8],
    body: &'static [u8],
}

/// Every table the binary carries.
const SNAPSHOTS: [Snapshot; 1] = [Snapshot {
    id: INSANE_TABLE_ID,
    header: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/data/tables/insane1/header.json")),
    body: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/data/tables/insane1/data.json")),
}];

/// What the binary carries of a default table.
enum Carried {
    /// A copy that reads.
    Snapshot(DifficultyTable),
    /// Nothing: the table is fetched from its address the first time it is wanted.
    Absent,
    /// A copy the binary was built with that does not read.
    Unreadable,
}

/// What the binary carries for `table`.
fn carried(table: &DefaultTable) -> Carried {
    let Some(snapshot) = SNAPSHOTS.iter().find(|snapshot| snapshot.id == table.id) else {
        return Carried::Absent;
    };
    let read = parse_header(snapshot.header).ok().and_then(|header| DifficultyTable::parse_body(snapshot.body, Some(header)).ok());
    read.map_or(Carried::Unreadable, Carried::Snapshot)
}

/// Give the player at the settings file `settings_path` every table they have not been given yet,
/// keeping the copy the binary carries of each one that went into their list and carries a copy of,
/// and answer whether the settings changed and are to be written.
///
/// The settings change when a table is given, and also when one is only written down as given
/// because the player has it at another address: either way the next run must not give it again.
pub(crate) fn seed_default_tables(config: &mut Config, settings_path: &Path) -> bool {
    let given_before = config.library.seeded_default_tables.len();
    let added = config.library.seed_default_tables(DEFAULT_TABLES);
    let store = TableStore::beside(settings_path);
    for table in added {
        match carried(table) {
            Carried::Snapshot(snapshot) => store.plant(table.location, &snapshot),
            Carried::Absent => {}
            Carried::Unreadable => {
                crate::notify::notify(crate::notify::Level::Warn, format!("the copy of {} the player carries does not read", table.location))
            }
        }
    }
    config.library.seeded_default_tables.len() != given_before
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use rbms_config::{SATELLITE_TABLE_ID, STELLA_TABLE_ID, TableSource};
    use rbms_library::Library;

    use super::*;
    use crate::tablesrc::{TableOrigin, TableStatus, read_and_match_all, read_table_source, split_matched, table_status};

    const INSANE_COPY: &str = "https://bms.hyuns.uk/tables/insane1";
    const STELLA_PAGE: &str = "https://stellabms.xyz/st/table.html";
    const SATELLITE_PAGE: &str = "https://stellabms.xyz/sl/table.html";

    /// A settings file in a folder of its own, removed when the test is over.
    struct Settings(std::path::PathBuf);

    impl Settings {
        fn new(tag: &str) -> Settings {
            let folder = std::env::temp_dir().join(format!("rbms-seed-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(&folder).expect("a scratch folder");
            Settings(folder.join("settings.ron"))
        }
    }

    impl Drop for Settings {
        fn drop(&mut self) {
            if let Some(folder) = self.0.parent() {
                let _ = std::fs::remove_dir_all(folder);
            }
        }
    }

    #[test]
    fn every_carried_snapshot_reads() {
        for snapshot in &SNAPSHOTS {
            let table =
                DEFAULT_TABLES.iter().find(|table| table.id == snapshot.id).unwrap_or_else(|| panic!("{} is carried but is no default table", snapshot.id));
            match carried(table) {
                Carried::Snapshot(read) => assert!(!read.entries.is_empty(), "{} is carried empty", table.id),
                Carried::Absent | Carried::Unreadable => panic!("{} is carried and does not read", table.id),
            }
        }
        let Carried::Snapshot(insane) = carried(&DEFAULT_TABLES[0]) else { panic!("the insane table is carried") };
        assert_eq!((insane.entries.len(), insane.level_order.len(), insane.skipped), (1_034, 26, 0));
    }

    /// The two tables of stellabms.xyz are fetched from the original site and never carried, so
    /// giving them keeps no copy and says nothing, and their folders wait for the first fetch.
    #[test]
    fn stella_is_not_carried_and_waits_for_its_first_fetch() {
        let settings = Settings::new("stella");
        let mut config = Config::first_run();
        let notified = crate::notify::exclusive(|| {
            seed_default_tables(&mut config, &settings.0);
            let mut notified = Vec::new();
            crate::notify::drain(&mut notified);
            notified
        });
        assert!(notified.is_empty(), "giving a table that is not carried is no failure: {notified:?}");

        let store = TableStore::beside(&settings.0);
        for table in &DEFAULT_TABLES[1..] {
            assert!(matches!(carried(table), Carried::Absent), "{} is not carried", table.id);
            assert!(!store.has_copy_file(table.location), "a copy was kept of {}", table.id);
        }
        let given: Vec<_> = config.library.tables.iter().map(|source| (source.location.as_str(), source.group.as_str())).collect();
        assert_eq!(given, vec![(INSANE_COPY, ""), (STELLA_PAGE, "STELLA"), (SATELLITE_PAGE, "STELLA")]);

        let matched = read_and_match_all(&store, &config.library.tables, &Library::default(), &AtomicUsize::new(0));
        assert_eq!(matched.iter().map(|table| table.levels.len()).collect::<Vec<_>>(), vec![26, 0, 0], "only the carried table has its folders");
        for location in [STELLA_PAGE, SATELLITE_PAGE] {
            assert!(matches!(read_table_source(&store, location), Ok(None)), "{location} has no copy and waits to be fetched");
        }
    }

    /// The first run has the table, and its folder, before anything has been fetched: the copy the
    /// binary carries is kept where a fetched one would be, and the network is never asked (a store
    /// made in a test reaches no network at all).
    #[test]
    fn a_first_run_has_the_table_and_its_folders_without_the_network() {
        let settings = Settings::new("first");
        let mut config = Config::first_run();
        assert!(seed_default_tables(&mut config, &settings.0), "the settings changed");
        assert_eq!(
            config.library.tables,
            vec![
                TableSource { name: String::new(), location: INSANE_COPY.into(), group: String::new() },
                TableSource { name: String::new(), location: STELLA_PAGE.into(), group: "STELLA".into() },
                TableSource { name: String::new(), location: SATELLITE_PAGE.into(), group: "STELLA".into() },
            ]
        );
        assert!(TableStore::beside(&settings.0).has_copy_file(INSANE_COPY), "the carried copy was kept");

        let (names, levels, _) =
            split_matched(read_and_match_all(&TableStore::beside(&settings.0), &config.library.tables, &Library::default(), &AtomicUsize::new(0)));
        assert_eq!(names[0], "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868}");
        assert_eq!(levels.iter().map(Vec::len).collect::<Vec<_>>(), vec![26, 0, 0], "every level of the carried table is a folder, owned or not");
        assert!(matches!(table_status(INSANE_COPY), Some(TableStatus::Loaded { entries: 1_034, origin: TableOrigin::Address { fetched_at: None }, .. })));

        assert!(!seed_default_tables(&mut config, &settings.0), "a second run gives nothing and writes nothing");
        assert_eq!(config.library.tables.len(), 3);
    }

    /// A first run with no song folder starts on the browser without a scan, and the table it was
    /// given is a folder of that browser from its first frame.
    #[test]
    fn a_first_run_without_a_song_folder_lists_the_table_it_was_given() {
        let settings = Settings::new("app");
        let mut config = Config::default();
        seed_default_tables(&mut config, &settings.0);
        let app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), settings.0.clone());
        let labels: Vec<String> = app
            .shared
            .select_items
            .iter()
            .filter_map(|item| match item {
                crate::SelectItem::Folder { label, .. } => Some(label.clone()),
                crate::SelectItem::Song(_) | crate::SelectItem::Missing(_) => None,
            })
            .collect();
        assert_eq!(labels, vec!["ALL SONGS (0)".to_string(), "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868} (0)".to_string()]);
        assert_eq!(app.shared.table_levels[0].len(), 26);
    }

    #[test]
    fn a_table_the_player_removed_stays_removed() {
        let settings = Settings::new("removed");
        let mut config = Config::default();
        seed_default_tables(&mut config, &settings.0);
        config.library.tables.clear();
        let store = TableStore::beside(&settings.0);
        store.forget(INSANE_COPY);
        assert!(!seed_default_tables(&mut config, &settings.0));
        assert!(config.library.tables.is_empty(), "the removed table came back");
        assert!(!store.has_copy_file(INSANE_COPY), "a copy was kept of a table that is not in the list");
    }

    #[test]
    fn a_player_with_the_original_address_is_not_given_the_copy() {
        let settings = Settings::new("original");
        let mut config = Config::default();
        let original = TableSource { name: "insane".into(), location: "https://darksabun.club/table/archive/insane1/".into(), group: String::new() };
        config.library.tables = vec![original.clone()];
        assert!(seed_default_tables(&mut config, &settings.0), "the table is written down as given");
        let locations: Vec<_> = config.library.tables.iter().map(|source| source.location.as_str()).collect();
        assert_eq!(
            locations,
            vec![original.location.as_str(), STELLA_PAGE, SATELLITE_PAGE],
            "the player's own row is left as it was, and the new ones follow it"
        );
        assert_eq!(config.library.tables[0], original);
        assert_eq!(config.library.seeded_default_tables, vec![INSANE_TABLE_ID.to_string(), STELLA_TABLE_ID.to_string(), SATELLITE_TABLE_ID.to_string()]);
        assert!(!TableStore::beside(&settings.0).has_copy_file(INSANE_COPY));
    }

    /// A copy already kept of the address -- fetched by a player who registered it by hand before it
    /// was given -- is newer than the one the binary carries, and is left as it is.
    #[test]
    fn a_copy_already_kept_is_not_replaced_by_the_carried_one() {
        let settings = Settings::new("kept");
        let store = TableStore::beside(&settings.0);
        store.keep_for_tests(INSANE_COPY, "FETCHED", br#"[{"md5":"aa","level":"1"}]"#, Some(7));
        let mut config = Config::default();
        seed_default_tables(&mut config, &settings.0);
        assert_eq!(store.copy_name(INSANE_COPY).as_deref(), Some("FETCHED"));
    }
}
