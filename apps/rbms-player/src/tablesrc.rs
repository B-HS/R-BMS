//! Difficulty-table loading and library matching. Extracted from `main.rs`: fetch/cache a table
//! (`rbms_table`), match its entries to the local song library by md5, and resolve ASCII-safe
//! display names. Only `load_and_match` / `fetch_and_match` are called by the app; the rest are
//! internal steps.

use crate::SongEntry;
use crate::tables::TableSource;

fn compute_table_levels(songs: &[SongEntry], table: &rbms_table::DifficultyTable) -> Vec<(String, Vec<usize>)> {
    let mut by_md5: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, s) in songs.iter().enumerate() {
        by_md5.entry(s.md5.to_ascii_lowercase()).or_default().push(i);
    }
    let mut out = Vec::new();
    for (level, entry_idxs) in table.by_level() {
        let mut idxs = Vec::new();
        for ei in entry_idxs {
            if let Some(v) = by_md5.get(&table.entries[ei].md5.to_ascii_lowercase()) {
                idxs.extend(v.iter().copied());
            }
        }
        idxs.sort_unstable();
        idxs.dedup();
        if !idxs.is_empty() {
            out.push((level, idxs));
        }
    }
    out
}

/// The font is ASCII-only, so non-ASCII table names (e.g. Japanese) can't render — fall back
/// to a generic ASCII label for those.
fn ascii_table_name(name: &str) -> String {
    let t = name.trim();
    if !t.is_empty() && t.is_ascii() { t.to_ascii_uppercase() } else { "DIFFICULTY TABLE".into() }
}

/// Stable per-URL filename for the on-disk table cache.
fn url_hash(s: &str) -> String {
    let mut h: u64 = 5381;
    for b in s.bytes() {
        h = h.wrapping_mul(33) ^ b as u64;
    }
    format!("{h:016x}")
}

/// Load a table from a `location`: an http(s) URL (fetched + cached) or a local `data.json` body file.
fn load_table_source(location: &str) -> Result<rbms_table::DifficultyTable, String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        let cache = std::env::temp_dir().join(format!("rbms-table-{}.json", url_hash(location)));
        rbms_table::DifficultyTable::fetch_or_cache(location, &cache)
    } else {
        let bytes = std::fs::read(location).map_err(|e| e.to_string())?;
        rbms_table::DifficultyTable::from_body_bytes(&bytes, None)
    }
}

/// ASCII display name for a loaded table: the user's source name if usable, else the table's
/// own (header) name, else a generic label (the font is ASCII-only).
fn pick_table_name(src: &TableSource, table: &rbms_table::DifficultyTable) -> String {
    let s = src.name.trim();
    if !s.is_empty() && s.is_ascii() {
        return s.to_ascii_uppercase();
    }
    ascii_table_name(&table.name)
}

/// Display name for a source whose table failed to load (no header available).
fn fallback_source_name(src: &TableSource) -> String {
    let s = src.name.trim();
    if !s.is_empty() && s.is_ascii() { s.to_ascii_uppercase() } else { "TABLE".into() }
}

/// Load one table source and match it against the library. A failed load yields the source's
/// fallback name and an empty level set (kept so the parallel vecs stay aligned with sources).
pub(crate) fn load_and_match(src: &TableSource, songs: &[SongEntry]) -> (String, Vec<(String, Vec<usize>)>) {
    println!("loading table: {} ({})", src.name, src.location);
    match load_table_source(&src.location) {
        Ok(t) => {
            let lv = compute_table_levels(songs, &t);
            let owned: usize = lv.iter().map(|(_, v)| v.len()).sum();
            println!("  {} entries, matched {owned} local charts across {} levels", t.entries.len(), lv.len());
            (pick_table_name(src, &t), lv)
        }
        Err(e) => {
            eprintln!("  table load failed: {e}");
            (fallback_source_name(src), Vec::new())
        }
    }
}

/// Load every table source (one entry per source, aligned with `sources`). Returns parallel
/// `(display name, per-level owned-chart groups)` vecs.
pub(crate) fn fetch_and_match(sources: &[TableSource], songs: &[SongEntry]) -> (Vec<String>, Vec<Vec<(String, Vec<usize>)>>) {
    let mut names = Vec::new();
    let mut levels = Vec::new();
    for src in sources {
        let (name, lv) = load_and_match(src, songs);
        names.push(name);
        levels.push(lv);
    }
    (names, levels)
}
