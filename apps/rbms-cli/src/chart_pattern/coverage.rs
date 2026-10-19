//! How much of a difficulty table the library's charts cover, matched on md5.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use rbms_library::songdb::PARSER_VERSION;
use rbms_table::DifficultyTable;
use serde::Serialize;

/// The `format` tag of the coverage summary.
const FORMAT: &str = "rbms-chart-pattern-coverage";

/// The generation of the summary's shape.
const VERSION: u32 = 1;

/// The md5 of every chart a table lists, lower case and without repeats.
pub struct TableMd5s {
    pub md5s: BTreeSet<String>,
    /// Entries that name a chart by its sha256 alone, which an md5 match cannot reach.
    pub entries_without_md5: usize,
}

/// Reads a table body (`data.json`).
pub fn read_table(path: &Path) -> Result<TableMd5s, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("table not read: {}: {e}", path.display()))?;
    let table = DifficultyTable::parse_body(&bytes, None).map_err(|e| format!("table not parsed: {}: {e}", path.display()))?;
    let mut md5s = BTreeSet::new();
    let mut entries_without_md5 = 0;
    for entry in &table.entries {
        let md5 = entry.md5.trim().to_ascii_lowercase();
        if md5.is_empty() {
            entries_without_md5 += 1;
        } else {
            md5s.insert(md5);
        }
    }
    Ok(TableMd5s { md5s, entries_without_md5 })
}

/// What the summary file and the printed report say about a table.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Coverage {
    format: &'static str,
    version: u32,
    pub mode: &'static str,
    parser_version: u32,
    pub table_md5s: usize,
    pub table_entries_without_md5: usize,
    /// Table md5s a pattern was written for (or already stood for).
    pub matched: usize,
    /// Table md5s that no chart read in this run has.
    pub missing: Vec<String>,
    /// Table md5s whose charts are all of another mode.
    pub other_mode: Vec<String>,
    /// Files that could not be read, which may have been charts of the missing md5s.
    pub failed_files: usize,
}

/// Matches the table against the md5s of the charts of the mode (`patterned`) and of the charts of
/// other modes (`elsewhere`).
pub fn coverage_of(table: &TableMd5s, mode: &'static str, patterned: &HashSet<String>, elsewhere: &HashSet<String>, failed_files: usize) -> Coverage {
    let mut matched = 0;
    let mut missing = Vec::new();
    let mut other_mode = Vec::new();
    for md5 in &table.md5s {
        if patterned.contains(md5) {
            matched += 1;
        } else if elsewhere.contains(md5) {
            other_mode.push(md5.clone());
        } else {
            missing.push(md5.clone());
        }
    }
    Coverage {
        format: FORMAT,
        version: VERSION,
        mode,
        parser_version: PARSER_VERSION,
        table_md5s: table.md5s.len(),
        table_entries_without_md5: table.entries_without_md5,
        matched,
        missing,
        other_mode,
        failed_files,
    }
}
