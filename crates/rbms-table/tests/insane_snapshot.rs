//! The 発狂BMS難易度表 snapshot the repository keeps (`web/data/tables/insane1`), read the way the
//! player reads a table: the header and the body as its server serves them. The site serves these
//! two files and the player carries them inside its binary, so what is checked here is what both
//! of them hand on.

use std::path::PathBuf;

use rbms_table::{DifficultyTable, SavedTable, parse_header};

/// Charts the archived table lists.
const CHARTS: usize = 1_035;

/// Its levels: ★1 to ★25, then `???`.
const NUMBERED_LEVELS: usize = 25;
const UNKNOWN_LEVEL: &str = "???";

fn snapshot(file: &str) -> Vec<u8> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "web", "data", "tables", "insane1", file].iter().collect();
    std::fs::read(&path).unwrap_or_else(|error| panic!("{} cannot be read: {error}", path.display()))
}

fn the_table() -> DifficultyTable {
    let header = parse_header(&snapshot("header.json")).expect("the snapshot's header is a header");
    DifficultyTable::parse_body(&snapshot("data.json"), Some(header)).expect("the snapshot's body is a body")
}

#[test]
fn the_snapshot_reads_as_the_whole_table() {
    let table = the_table();
    assert_eq!(table.name, "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868}");
    assert_eq!(table.symbol, "\u{2605}");
    assert_eq!(table.entries.len(), CHARTS);
    assert_eq!(table.skipped, 0, "every item of the snapshot names a chart");

    let mut expected: Vec<String> = (1..=NUMBERED_LEVELS).map(|level| level.to_string()).collect();
    expected.push(UNKNOWN_LEVEL.to_string());
    assert_eq!(table.level_order, expected);
    let levels: Vec<String> = table.by_level().into_iter().map(|(level, _)| level).collect();
    assert_eq!(levels, expected, "every level holds a chart and none is outside the order");
    assert!(table.entries.iter().all(|entry| entry.md5.len() == 32 && entry.md5.bytes().all(|byte| byte.is_ascii_hexdigit())), "every chart is matched by md5");
}

/// What the player keeps of the snapshot is the table itself: the copy reads back with every chart
/// and level, and the charts match a library by md5.
#[test]
fn the_snapshot_kept_as_a_copy_is_the_same_table() {
    let table = the_table();
    let json = SavedTable::json_of(&table, "https://bms.hyuns.uk/tables/insane1", None).expect("a table is JSON");
    let saved = SavedTable::from_bytes(json.as_bytes()).expect("the copy reads");
    assert_eq!(saved.table.entries.len(), CHARTS);
    assert_eq!(saved.table.by_level(), table.by_level());
    assert_eq!(saved.fetched_at, None);

    let first = table.entries[0].md5.to_ascii_uppercase();
    let matched = saved.table.match_levels([first.as_str()].into_iter());
    assert_eq!(matched.iter().map(|(_, owned)| owned.len()).sum::<usize>(), 1, "one owned chart, found in its level");
}
