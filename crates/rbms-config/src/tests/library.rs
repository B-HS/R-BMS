//! The library lists: the song folders and the difficulty tables, and how a legacy sibling file is
//! folded into them.

use super::*;

#[test]
fn the_folder_list_defaults_to_empty_and_keeps_its_order() {
    assert!(LibraryOptions::default().folders.is_empty());
    let mut c = Config::default();
    c.library.folders = vec!["/a".into(), "/b/c".into(), "D:\\songs".into()];
    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert_eq!(back.library.folders, vec!["/a".to_string(), "/b/c".into(), "D:\\songs".into()]);
}

#[test]
fn the_table_list_defaults_to_empty_and_keeps_its_order() {
    assert!(LibraryOptions::default().tables.is_empty());
    let mut c = Config::default();
    c.library.tables = vec![
        TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into() },
        TableSource { name: String::new(), location: "/local/table.json".into() },
    ];
    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert_eq!(back.library.tables.len(), 2, "count preserved");
    assert_eq!(back.library.tables[0].name, "Insane");
    assert_eq!(back.library.tables[0].location, "https://example.com/insane.json");
    assert_eq!(back.library.tables[1].name, "", "empty name preserved");
    assert_eq!(back.library.tables[1].location, "/local/table.json");
}

#[test]
fn merging_absent_or_empty_legacy_lists_leaves_the_library_alone() {
    let mut c = Config::default();
    merge_legacy_lists(&mut c, None, None);
    assert!(c.library.folders.is_empty(), "a missing folders file adds nothing");
    assert!(c.library.tables.is_empty(), "a missing tables file adds nothing");
    merge_legacy_lists(&mut c, Some("()"), Some("()"));
    assert!(c.library.folders.is_empty(), "an empty unit list adds nothing");
    assert!(c.library.tables.is_empty());
}

#[test]
fn merging_a_malformed_legacy_list_is_skipped_rather_than_fatal() {
    let mut c = Config::default();
    c.library.folders = vec!["/songs".into()];
    merge_legacy_lists(&mut c, Some("@@@ not ron @@@"), Some("not ron at all"));
    assert_eq!(c.library.folders, vec!["/songs".to_string()], "an unreadable list leaves the library as it was");
    assert!(c.library.tables.is_empty());
}

#[test]
fn merging_legacy_lists_appends_only_what_is_missing() {
    let mut c = Config::default();
    c.library.folders = vec!["/songs".into()];
    c.library.tables = vec![TableSource { name: "kept".into(), location: "/local/table.json".into() }];
    merge_legacy_lists(
        &mut c,
        Some(r#"(folders: ["/songs", "/more"])"#),
        Some(r#"(tables: [(name: "renamed", location: "/local/table.json"), (name: "Insane", location: "https://example.com/insane.json")])"#),
    );
    assert_eq!(c.library.folders, vec!["/songs".to_string(), "/more".into()], "a folder already listed is not duplicated");
    assert_eq!(c.library.tables.len(), 2, "a table with a known location is not duplicated");
    assert_eq!(c.library.tables[0].name, "kept", "the configuration's own entry wins");
    assert_eq!(c.library.tables[1].location, "https://example.com/insane.json");
}
