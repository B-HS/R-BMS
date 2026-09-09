//! Difficulty-table loading and library matching: fetch/cache a table (`rbms_table`), match its
//! entries to the local song library by md5 (`DifficultyTable::match_levels`), and resolve
//! ASCII-safe display names. Only `load_and_match` / `fetch_and_match` are called by the app; the
//! rest are internal steps.

use rbms_config::TableSource;
use rbms_library::Library;

/// One difficulty table matched against the library: `(level label, indices of the owned charts)`
/// per level, in the table's own level order.
pub(crate) type TableLevels = Vec<(String, Vec<usize>)>;

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

/// Why one difficulty-table source could not be loaded: the table crate's own failure, or the local
/// file the source named could not be read.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TableSourceError {
    #[error("{0}")]
    Table(#[from] rbms_table::TableError),
    #[error("{0}")]
    Read(#[from] std::io::Error),
}

/// Load a table from a `location`: an http(s) URL (fetched + cached) or a local `data.json` body file.
fn load_table_source(location: &str) -> Result<rbms_table::DifficultyTable, TableSourceError> {
    if location.starts_with("http://") || location.starts_with("https://") {
        let cache = std::env::temp_dir().join(format!("rbms-table-{}.json", url_hash(location)));
        Ok(rbms_table::DifficultyTable::fetch_cached(location, &cache)?)
    } else {
        let bytes = std::fs::read(location)?;
        Ok(rbms_table::DifficultyTable::parse_body(&bytes, None)?)
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
pub(crate) fn load_and_match(src: &TableSource, library: &Library) -> (String, TableLevels) {
    println!("loading table: {} ({})", src.name, src.location);
    match load_table_source(&src.location) {
        Ok(t) => {
            let lv = t.match_levels(library.md5s());
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
pub(crate) fn fetch_and_match(sources: &[TableSource], library: &Library) -> (Vec<String>, Vec<TableLevels>) {
    let mut names = Vec::new();
    let mut levels = Vec::new();
    for src in sources {
        let (name, lv) = load_and_match(src, library);
        names.push(name);
        levels.push(lv);
    }
    (names, levels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_source_that_is_not_there_reports_the_read_failure_by_type() {
        let missing = std::env::temp_dir().join(format!("rbms-no-such-table-{}.json", std::process::id()));
        let error = match load_table_source(missing.to_str().expect("utf-8 temp path")) {
            Ok(_) => panic!("a missing file is not a table"),
            Err(e) => e,
        };
        assert!(matches!(error, TableSourceError::Read(_)), "got {error:?}");
    }

    #[test]
    fn a_local_source_that_is_not_json_reports_the_table_crates_own_error() {
        let path = std::env::temp_dir().join(format!("rbms-bad-table-{}.json", std::process::id()));
        std::fs::write(&path, b"not json").expect("write the fixture");
        let error = match load_table_source(path.to_str().expect("utf-8 temp path")) {
            Ok(_) => panic!("a non-JSON body is not a table"),
            Err(e) => e,
        };
        assert!(matches!(error, TableSourceError::Table(rbms_table::TableError::BodyParse(_))), "got {error:?}");
        assert!(error.to_string().starts_with("body parse:"), "the message the screen shows is unchanged: {error}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_local_source_that_parses_matches_the_library_by_md5() {
        let path = std::env::temp_dir().join(format!("rbms-good-table-{}.json", std::process::id()));
        std::fs::write(&path, br#"[{"md5":"AABB","level":"1"}]"#).expect("write the fixture");
        let table = load_table_source(path.to_str().expect("utf-8 temp path")).expect("a JSON body is a table");
        assert_eq!(table.match_levels(["aabb", "ccdd"].into_iter()), vec![("1".to_string(), vec![0])], "md5 matching is case-insensitive");
        let _ = std::fs::remove_file(&path);
    }

    /// One library entry, with only the fields the matcher and the levels read filled in.
    fn entry(md5: &str) -> rbms_library::SongEntry {
        rbms_library::SongEntry {
            path: std::path::PathBuf::from(format!("/songs/{md5}.bms")),
            title: md5.into(),
            subtitle: String::new(),
            artist: String::new(),
            genre: String::new(),
            maker: String::new(),
            level: "1".into(),
            difficulty: 3,
            init_bpm: 150.0,
            rank: 2,
            total: 300.0,
            mode: rbms_model::Mode::BEAT_7K,
            md5: md5.into(),
            stagefile: String::new(),
            banner: String::new(),
            preview: String::new(),
        }
    }

    #[test]
    fn a_table_source_is_matched_against_the_library_index_and_reports_library_positions() {
        let path = std::env::temp_dir().join(format!("rbms-lib-table-{}.json", std::process::id()));
        std::fs::write(&path, br#"[{"md5":"BBBB","level":"1"},{"md5":"dddd","level":"2"}]"#).expect("write the fixture");
        let library = Library::from_songs(vec![entry("aaaa"), entry("bbbb"), entry("cccc"), entry("DDDD")]);
        let source = TableSource { name: "MINE".into(), location: path.to_string_lossy().to_string() };

        let (name, levels) = load_and_match(&source, &library);
        assert_eq!(name, "MINE");
        assert_eq!(levels, vec![("1".to_string(), vec![1]), ("2".to_string(), vec![3])], "levels carry positions in the library's own order");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_source_that_cannot_be_loaded_keeps_its_slot_with_no_levels() {
        let library = Library::from_songs(vec![entry("aaaa")]);
        let source = TableSource { name: "gone".into(), location: "/no/such/table.json".into() };
        let (name, levels) = load_and_match(&source, &library);
        assert_eq!(name, "GONE", "the source name still labels the row");
        assert!(levels.is_empty(), "a failed load must not drop the row and misalign the parallel vecs");
    }
}
