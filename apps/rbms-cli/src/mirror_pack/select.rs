//! Choosing the song folders to pack: those that hold a chart a difficulty table lists, or all.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use rbms_table::DifficultyTable;
use serde::{Deserialize, Serialize};

use super::folders::SongFolder;

/// The md5 of every chart a table lists, lower case and without repeats.
pub struct TableMd5s {
    /// The table file's name, which is all of its path that is kept.
    pub source: String,
    pub md5s: BTreeSet<String>,
}

/// How much of one table the library has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCoverage {
    pub source: String,
    /// The charts the table lists by md5.
    pub md5s: usize,
    /// How many of them a song folder holds.
    pub matched: usize,
    /// The md5s no song folder holds.
    pub missing: Vec<String>,
}

/// A chart that more than one song folder holds, of which only the first is packed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Duplicate {
    pub md5: String,
    pub kept: String,
    pub also: Vec<String>,
}

/// What was chosen.
pub struct Selection {
    /// Indexes into the folders, in order.
    pub indexes: Vec<usize>,
    pub coverage: Vec<TableCoverage>,
    pub duplicates: Vec<Duplicate>,
}

/// Reads a table body (`data.json`).
pub fn read_table(path: &Path) -> Result<TableMd5s, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("table not read: {}: {e}", path.display()))?;
    let table = DifficultyTable::parse_body(&bytes, None).map_err(|e| format!("table not parsed: {}: {e}", path.display()))?;
    let md5s = table.entries.iter().map(|entry| entry.md5.trim().to_ascii_lowercase()).filter(|md5| !md5.is_empty()).collect();
    let source = path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned());
    Ok(TableMd5s { source, md5s })
}

/// Chooses the folders: with `all`, every one; and for each md5 of each table, the first folder
/// that holds it, in the order of `folders`.
pub fn select(folders: &[SongFolder], tables: &[TableMd5s], all: bool) -> Selection {
    let mut holders: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, folder) in folders.iter().enumerate() {
        for chart in &folder.charts {
            let held = holders.entry(chart.md5.as_str()).or_default();
            if held.last() != Some(&index) {
                held.push(index);
            }
        }
    }
    let mut chosen: BTreeSet<usize> = if all { (0..folders.len()).collect() } else { BTreeSet::new() };
    let mut duplicates: BTreeMap<&str, Duplicate> = BTreeMap::new();
    let mut coverage = Vec::new();
    for table in tables {
        let mut missing = Vec::new();
        let mut matched = 0;
        for md5 in &table.md5s {
            let Some(held) = holders.get(md5.as_str()) else {
                missing.push(md5.clone());
                continue;
            };
            matched += 1;
            chosen.insert(held[0]);
            if held.len() > 1 {
                duplicates.entry(md5.as_str()).or_insert_with(|| Duplicate {
                    md5: md5.clone(),
                    kept: folders[held[0]].relative.clone(),
                    also: held[1..].iter().map(|index| folders[*index].relative.clone()).collect(),
                });
            }
        }
        coverage.push(TableCoverage { source: table.source.clone(), md5s: table.md5s.len(), matched, missing });
    }
    Selection { indexes: chosen.into_iter().collect(), coverage, duplicates: duplicates.into_values().collect() }
}
