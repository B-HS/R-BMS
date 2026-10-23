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
fn the_download_folder_is_scanned_after_the_song_folders() {
    let mut library = LibraryOptions::default();
    assert!(library.scan_roots().is_empty(), "no song folder and no download folder scans nothing");
    library.folders = vec!["/a".into(), "/b".into()];
    assert_eq!(library.scan_roots(), vec!["/a".to_string(), "/b".into()], "without a download folder the song folders are the whole scan");
    library.download_folder = Some("/downloads".into());
    assert_eq!(library.scan_roots(), vec!["/a".to_string(), "/b".into(), "/downloads".into()]);
    library.folders.clear();
    assert_eq!(library.scan_roots(), vec!["/downloads".to_string()], "a player with only a download folder still has it scanned");
}

#[test]
fn a_download_folder_listed_already_is_scanned_once() {
    for spelling in ["/songs", "/songs/", "  /songs  ", "/songs\\"] {
        let library = LibraryOptions { folders: vec!["/songs".into()], download_folder: Some(spelling.into()), ..LibraryOptions::default() };
        assert_eq!(library.scan_roots(), vec!["/songs".to_string()], "{spelling:?} is the song folder listed");
    }
    let library = LibraryOptions { folders: vec!["D:\\songs\\".into()], download_folder: Some("D:\\songs".into()), ..LibraryOptions::default() };
    assert_eq!(library.scan_roots(), vec!["D:\\songs\\".to_string()]);
    let library = LibraryOptions { folders: vec!["/songs".into()], download_folder: Some("/songs/downloads".into()), ..LibraryOptions::default() };
    assert_eq!(library.scan_roots().len(), 2, "a folder inside a song folder is another folder");
}

#[test]
fn a_blank_download_folder_is_none_chosen() {
    for blank in ["", "   "] {
        let library = LibraryOptions { download_folder: Some(blank.into()), ..LibraryOptions::default() };
        assert_eq!(library.download_folder_path(), None, "{blank:?}");
        assert!(library.scan_roots().is_empty(), "{blank:?} was scanned as a folder");
    }
    let library = LibraryOptions { download_folder: Some("  /downloads ".into()), ..LibraryOptions::default() };
    assert_eq!(library.download_folder_path(), Some("/downloads"));
}

#[test]
fn a_file_without_the_download_settings_reads_as_none_chosen_and_missing_charts_shown() {
    let library: LibraryOptions = ron::from_str("(folders: [\"/songs\"], preview: false)").expect("a file from before the rows reads");
    assert_eq!(library.download_folder, None);
    assert!(library.show_missing_charts, "a file that never asked to hide them lists them");
    assert_eq!(library.folders, vec!["/songs".to_string()]);
}

#[test]
fn the_download_settings_survive_a_round_trip() {
    let mut c = Config::default();
    c.library.download_folder = Some("/downloads".into());
    c.library.show_missing_charts = false;
    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert_eq!(back.library.download_folder.as_deref(), Some("/downloads"));
    assert!(!back.library.show_missing_charts);
    let unset: Config = ron::from_str(&ron_of(&Config::default())).expect("a config round-trips");
    assert_eq!(unset.library.download_folder, None);
    assert!(unset.library.show_missing_charts);
}

#[test]
fn the_table_list_defaults_to_empty_and_keeps_its_order() {
    assert!(LibraryOptions::default().tables.is_empty());
    let mut c = Config::default();
    c.library.tables = vec![
        TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into(), group: String::new() },
        TableSource { name: String::new(), location: "/local/table.json".into(), group: String::new() },
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
    c.library.tables = vec![TableSource { name: "kept".into(), location: "/local/table.json".into(), group: String::new() }];
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
