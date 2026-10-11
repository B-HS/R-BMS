//! The copy of a fetched table that is kept on disk.
//!
//! A table is a few requests to somebody else's server, and what comes back changes seldom, so a
//! table that has been fetched is kept and read from the copy until it is asked for again. The copy
//! is everything a later run needs to be the table without the network: its name, its symbol, its
//! level order, its entries, its courses when the header lists any, the address it was fetched
//! from and when.
//!
//! Only the reading is done here. A copy is written by whoever keeps it, with the durable write the
//! rest of their files go through; [`SavedTable::json_of`] is what they write.

use std::io::Read;
use std::path::Path;

use crate::dto::TableEntry;
use crate::fetch::MAX_RESPONSE_BYTES;
use crate::{DifficultyTable, derive_level_order};

/// A copy as it is written.
#[derive(serde::Serialize)]
struct Writing<'a> {
    name: &'a str,
    symbol: &'a str,
    level_order: &'a [String],
    entries: &'a [TableEntry],
    #[serde(skip_serializing_if = "Option::is_none")]
    course: &'a Option<serde_json::Value>,
    skipped: usize,
    source: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    fetched_at: Option<u64>,
}

/// A copy as it is read. The fields a copy written before they existed does not have are read as
/// absent rather than refusing the copy: such a copy is still the table.
#[derive(serde::Deserialize)]
struct Reading {
    name: String,
    symbol: String,
    level_order: Vec<String>,
    entries: Vec<TableEntry>,
    #[serde(default)]
    course: Option<serde_json::Value>,
    #[serde(default)]
    skipped: usize,
    #[serde(default)]
    source: String,
    #[serde(default)]
    fetched_at: Option<u64>,
}

/// A table read back from the copy kept of it.
pub struct SavedTable {
    pub table: DifficultyTable,
    /// The address the table was fetched from. Empty in a copy written before copies recorded it.
    pub source: String,
    /// When the table was fetched, in seconds since the Unix epoch. `None` in a copy written before
    /// copies recorded it.
    pub fetched_at: Option<u64>,
}

impl SavedTable {
    /// What is written to keep `table`, fetched from `source` at `fetched_at` (seconds since the
    /// Unix epoch, or `None` when nobody knows any more).
    pub fn json_of(table: &DifficultyTable, source: &str, fetched_at: Option<u64>) -> Result<String, serde_json::Error> {
        serde_json::to_string(&Writing {
            name: &table.name,
            symbol: &table.symbol,
            level_order: &table.level_order,
            entries: &table.entries,
            course: &table.course,
            skipped: table.skipped,
            source,
            fetched_at,
        })
    }

    /// The table a copy holds, or `None` when `bytes` are not a copy: cut short, damaged, or some
    /// other file altogether.
    pub fn from_bytes(bytes: &[u8]) -> Option<SavedTable> {
        let read: Reading = serde_json::from_slice(bytes).ok()?;
        let level_order = if read.level_order.is_empty() { derive_level_order(&read.entries) } else { read.level_order };
        let table = DifficultyTable { name: read.name, symbol: read.symbol, level_order, entries: read.entries, skipped: read.skipped, course: read.course };
        Some(SavedTable { table, source: read.source, fetched_at: read.fetched_at })
    }

    /// The copy kept at `path`, if there is a readable one no larger than a table's answer may be.
    /// A file that is missing, damaged or too large is no copy.
    pub fn read(path: &Path) -> Option<SavedTable> {
        SavedTable::from_bytes(&read_within(path, MAX_RESPONSE_BYTES)?)
    }
}

/// The bytes of the file at `path` when there are no more than `limit` of them.
fn read_within(path: &Path, limit: usize) -> Option<Vec<u8>> {
    let limit = u64::try_from(limit).ok()?;
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > limit {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes).ok()?;
    (u64::try_from(bytes.len()).ok()? <= limit).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_header;

    /// A file written for one test, removed when the test is over.
    struct CopyFile(std::path::PathBuf);

    impl CopyFile {
        fn written(tag: &str, bytes: &[u8]) -> CopyFile {
            let path = std::env::temp_dir().join(format!("rbms-table-saved-{tag}-{}.json", std::process::id()));
            std::fs::write(&path, bytes).expect("write the fixture");
            CopyFile(path)
        }
    }

    impl Drop for CopyFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn sample() -> DifficultyTable {
        let header = parse_header(
            "{\"name\":\"\u{767a}\u{72c2}\",\"symbol\":\"\u{2605}\",\"data_url\":\"d.json\",\"level_order\":[\"2\",\"1\",\"???\"],\"course\":[[{\"name\":\"dan\",\"md5\":[\"aa\"]}]]}"
                .as_bytes(),
        )
        .expect("the sample header is a header");
        DifficultyTable::parse_body(
            br#"[{"md5":"aa","level":"1","title":"A","artist":"x","url":"u","url_diff":"d"},{"sha256":"ff","level":"2"},{"level":"1"}]"#,
            Some(header),
        )
        .expect("the sample body is a body")
    }

    /// Everything a later run needs to be the table without the network comes back from the copy.
    #[test]
    fn a_copy_reads_back_as_the_whole_table_with_where_and_when_it_came_from() {
        let table = sample();
        let json = SavedTable::json_of(&table, "https://tables.test/t/", Some(1_760_000_000)).expect("a table is JSON");
        let saved = SavedTable::from_bytes(json.as_bytes()).expect("what was written is a copy");

        assert_eq!(saved.source, "https://tables.test/t/");
        assert_eq!(saved.fetched_at, Some(1_760_000_000));
        assert_eq!(saved.table.name, table.name);
        assert_eq!(saved.table.symbol, "\u{2605}");
        assert_eq!(saved.table.level_order, vec!["2", "1", "???"], "the header's order, not one derived again");
        assert_eq!(saved.table.skipped, 1);
        assert_eq!(saved.table.course, table.course);
        assert!(saved.table.course.is_some(), "the sample lists a course");
        assert_eq!(saved.table.entries.len(), 2);
        let first = &saved.table.entries[0];
        assert_eq!((first.md5.as_str(), first.title.as_str(), first.artist.as_str(), first.url.as_str(), first.url_diff.as_str()), ("aa", "A", "x", "u", "d"));
        assert_eq!(saved.table.entries[1].sha256, "ff", "an entry known only by its sha256 is kept");
        assert_eq!(saved.table.by_level(), table.by_level());

        let undated = SavedTable::json_of(&table, "https://tables.test/t/", None).expect("a table is JSON");
        assert_eq!(SavedTable::from_bytes(undated.as_bytes()).expect("what was written is a copy").fetched_at, None);
    }

    /// The copies an earlier version left in the temporary folder hold the table and nothing of
    /// where or when it came from. They are still read, so a table is not fetched again only
    /// because the copy is an old one.
    #[test]
    fn a_copy_written_before_the_address_and_the_time_were_kept_is_still_the_table() {
        let old = br#"{"name":"Saved","symbol":"s","level_order":[],"entries":[{"md5":"aa","level":"2","title":"","artist":"","url":"","url_diff":""},{"md5":"bb","level":"1","title":"","artist":"","url":"","url_diff":""}]}"#;
        let saved = SavedTable::from_bytes(old).expect("an older copy is a copy");
        assert_eq!(saved.table.name, "Saved");
        assert_eq!(saved.table.level_order, vec!["1", "2"], "a copy saved without an order derives one");
        assert_eq!(saved.source, "");
        assert_eq!(saved.fetched_at, None);
        assert_eq!(saved.table.course, None);
    }

    /// A copy that was cut short while it was written, or is not a copy at all, is no copy: the
    /// table is fetched again rather than shown as whatever could be made of the damage.
    #[test]
    fn a_damaged_copy_is_no_copy() {
        let json = SavedTable::json_of(&sample(), "https://tables.test/t/", Some(7)).expect("a table is JSON");
        let cut = &json.as_bytes()[..json.len() / 2];
        assert!(SavedTable::from_bytes(cut).is_none(), "half a copy");
        assert!(SavedTable::from_bytes(b"").is_none(), "an empty file");
        assert!(SavedTable::from_bytes(b"not a saved table").is_none());
        assert!(SavedTable::from_bytes(br#"[{"md5":"aa","level":"1"}]"#).is_none(), "a body is not a copy");
    }

    #[test]
    fn a_copy_is_read_from_its_file_and_a_missing_file_is_no_copy() {
        let json = SavedTable::json_of(&sample(), "https://tables.test/t/", Some(7)).expect("a table is JSON");
        let file = CopyFile::written("read", json.as_bytes());
        assert_eq!(SavedTable::read(&file.0).expect("the file is a copy").table.entries.len(), 2);

        let missing = std::env::temp_dir().join(format!("rbms-table-saved-missing-{}.json", std::process::id()));
        assert!(SavedTable::read(&missing).is_none());
    }

    /// The limit an answer from a server is held to is the limit a copy is read within, so a file
    /// that grew or was swapped for something else cannot fill memory either.
    #[test]
    fn a_file_larger_than_the_limit_is_not_read() {
        let file = CopyFile::written("large", &[b'x'; 64]);
        assert_eq!(read_within(&file.0, 64).map(|bytes| bytes.len()), Some(64), "a file of exactly the limit is read");
        assert!(read_within(&file.0, 63).is_none(), "one byte over is refused");
    }
}
