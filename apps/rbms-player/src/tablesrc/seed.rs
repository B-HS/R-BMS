//! The difficulty tables every player is given, and the copies of them the player carries.
//!
//! Which tables are given, and the rule that gives each once, are the settings' own
//! ([`rbms_config::LibraryOptions::seed_default_tables`]). What is here is the table itself: the
//! repository keeps a snapshot of each in the files the site serves it from
//! (`web/data/tables/<id>/header.json` and `data.json`), the binary carries those two files, and a
//! table is kept as its copy ([`TableStore::plant`]) the moment it is given. The first run of the
//! player shows the table's folder without asking the network anything, and with the site down;
//! asking for the table again (F6, R or A in the table manager) fetches it from its address as it
//! does any other table.
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

/// The table the binary carries for `table`, when it carries one that reads.
fn carried(table: &DefaultTable) -> Option<DifficultyTable> {
    let snapshot = SNAPSHOTS.iter().find(|snapshot| snapshot.id == table.id)?;
    let header = parse_header(snapshot.header).ok()?;
    DifficultyTable::parse_body(snapshot.body, Some(header)).ok()
}

/// Give the player at the settings file `settings_path` every table they have not been given yet,
/// keeping the copy the binary carries of each one that went into their list, and answer whether
/// the settings changed and are to be written.
///
/// The settings change when a table is given, and also when one is only written down as given
/// because the player has it at another address: either way the next run must not give it again.
pub(crate) fn seed_default_tables(config: &mut Config, settings_path: &Path) -> bool {
    let given_before = config.library.seeded_default_tables.len();
    let added = config.library.seed_default_tables(DEFAULT_TABLES);
    let store = TableStore::beside(settings_path);
    for table in added {
        match carried(table) {
            Some(snapshot) => store.plant(table.location, &snapshot),
            None => crate::notify::notify(crate::notify::Level::Warn, format!("the copy of {} the player carries does not read", table.location)),
        }
    }
    config.library.seeded_default_tables.len() != given_before
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use rbms_config::TableSource;
    use rbms_library::Library;

    use super::*;
    use crate::tablesrc::{TableOrigin, TableStatus, read_and_match_all, split_matched, table_status};

    const INSANE_COPY: &str = "https://bms.hyuns.uk/tables/insane1";

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
    fn every_table_given_is_carried_by_the_binary() {
        for table in DEFAULT_TABLES {
            let carried = carried(table).unwrap_or_else(|| panic!("{} is not carried", table.id));
            assert!(!carried.entries.is_empty(), "{} is carried empty", table.id);
        }
        let insane = carried(&DEFAULT_TABLES[0]).expect("the insane table is carried");
        assert_eq!((insane.entries.len(), insane.level_order.len(), insane.skipped), (1_035, 26, 0));
    }

    /// The first run has the table, and its folder, before anything has been fetched: the copy the
    /// binary carries is kept where a fetched one would be, and the network is never asked (a store
    /// made in a test reaches no network at all).
    #[test]
    fn a_first_run_has_the_table_and_its_folders_without_the_network() {
        let settings = Settings::new("first");
        let mut config = Config::first_run();
        assert!(seed_default_tables(&mut config, &settings.0), "the settings changed");
        assert_eq!(config.library.tables, vec![TableSource { name: String::new(), location: INSANE_COPY.into(), group: String::new() }]);
        assert!(TableStore::beside(&settings.0).has_copy_file(INSANE_COPY), "the carried copy was kept");

        let (names, levels, _) =
            split_matched(read_and_match_all(&TableStore::beside(&settings.0), &config.library.tables, &Library::default(), &AtomicUsize::new(0)));
        assert_eq!(names, vec!["\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868}".to_string()]);
        assert_eq!(levels[0].len(), 26, "every level is a folder, owned or not");
        assert!(matches!(table_status(INSANE_COPY), Some(TableStatus::Loaded { entries: 1_035, origin: TableOrigin::Address { fetched_at: None }, .. })));

        assert!(!seed_default_tables(&mut config, &settings.0), "a second run gives nothing and writes nothing");
        assert_eq!(config.library.tables.len(), 1);
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
                crate::SelectItem::Song(_) => None,
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
        assert_eq!(config.library.tables, vec![original], "the player's own row is left as it was, and alone");
        assert_eq!(config.library.seeded_default_tables, vec![INSANE_TABLE_ID.to_string()]);
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
