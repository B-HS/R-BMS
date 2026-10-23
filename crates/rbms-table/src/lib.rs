#![forbid(unsafe_code)]

use std::collections::HashMap;

mod dto;
mod error;
mod fetch;
mod html;
mod saved;
pub use dto::{TableEntry, TableHeader};
pub use error::{TableError, TableStage};
pub use saved::SavedTable;

/// The mark some editors put in front of a UTF-8 file, which is not JSON.
const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// The symbol a table is given when its header names none, or when it has no header.
pub(crate) const ABSENT_SYMBOL: &str = "*";

/// A BMS difficulty table resolved into header metadata plus body entries, browsable by
/// level. Built from a `header.json` (+ its `data_url`) or directly from a body `data.json`.
pub struct DifficultyTable {
    pub name: String,
    pub symbol: String,
    pub level_order: Vec<String>,
    pub entries: Vec<TableEntry>,
    /// Body items that were left out because they name no chart: not an object, or an object with
    /// neither an `md5` nor a `sha256`.
    pub skipped: usize,
    /// The header's `course` field as the table wrote it, when it has one. Nothing reads the courses
    /// of a table yet; they are carried so the copy kept of a table is the whole of it.
    pub course: Option<serde_json::Value>,
}

/// The md5 of every chart of a library, each with the positions it is at. A library is indexed
/// once and every table is matched against the index, rather than each table going through the
/// library for itself.
pub struct Md5Index {
    by_md5: HashMap<String, Vec<usize>>,
}

impl Md5Index {
    /// Index `library_md5s`, which yields the md5 of every library chart in library order (case
    /// insensitive). The positions handed back by a match are positions in that iteration.
    pub fn new<'a>(library_md5s: impl Iterator<Item = &'a str>) -> Md5Index {
        let mut by_md5: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, md5) in library_md5s.enumerate() {
            by_md5.entry(md5.to_ascii_lowercase()).or_default().push(i);
        }
        Md5Index { by_md5 }
    }
}

/// One item of a body: an entry, or whatever else was written where an entry belongs.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum BodyItem {
    Entry(TableEntry),
    Other(serde::de::IgnoredAny),
}

/// `bytes` without the byte-order mark some servers and editors put in front of JSON.
pub(crate) fn without_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes)
}

/// Parse JSON that is meant to be UTF-8 but may not be. The tables that are not are the old
/// Japanese ones, so bytes that are not UTF-8 are read as Shift_JIS when they are well-formed
/// Shift_JIS: the name and the symbol are then what the table calls itself, and a character whose
/// second byte is a backslash does not escape the quote after it. Anything else is read lossily
/// rather than the table refused, since its hashes and levels are ASCII in any encoding.
fn parse_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_json::Error> {
    let bytes = without_bom(bytes);
    if std::str::from_utf8(bytes).is_ok() {
        return serde_json::from_slice(bytes);
    }
    let (as_shift_jis, malformed) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
    if malformed { serde_json::from_str(&String::from_utf8_lossy(bytes)) } else { serde_json::from_str(&as_shift_jis) }
}

/// The entries of a body, and how many of its items were left out for naming no chart.
pub(crate) fn parse_entries(bytes: &[u8]) -> Result<(Vec<TableEntry>, usize), TableError> {
    let items: Vec<BodyItem> = parse_json(bytes).map_err(TableError::BodyParse)?;
    let total = items.len();
    let entries: Vec<TableEntry> = items
        .into_iter()
        .filter_map(|item| match item {
            BodyItem::Entry(entry) if entry.names_a_chart() => Some(entry),
            BodyItem::Entry(_) | BodyItem::Other(_) => None,
        })
        .collect();
    let skipped = total - entries.len();
    Ok((entries, skipped))
}

/// A header, which is a JSON object and nothing else: a list is a body, however few fields a
/// header is required to have. With [`DifficultyTable::parse_body`] this reads a table kept as its
/// two files, the way a table's server serves it.
pub fn parse_header(bytes: &[u8]) -> Result<TableHeader, TableError> {
    let object: serde_json::Map<String, serde_json::Value> = parse_json(bytes).map_err(TableError::HeaderParse)?;
    serde_json::from_value(serde_json::Value::Object(object)).map_err(TableError::HeaderParse)
}

impl DifficultyTable {
    /// Combine an optional header with body entries. When the header omits `level_order`,
    /// it is derived from the entries with a numeric-aware sort (so "10" follows "9", and
    /// non-numeric levels like "???" sort last).
    pub fn from_parts(header: Option<TableHeader>, entries: Vec<TableEntry>) -> DifficultyTable {
        let (name, symbol, level_order, course) = match header {
            Some(h) => (h.name.unwrap_or_default(), h.symbol.unwrap_or_else(|| ABSENT_SYMBOL.into()), h.level_order.unwrap_or_default(), h.course),
            None => (String::new(), ABSENT_SYMBOL.into(), Vec::new(), None),
        };
        let level_order = if level_order.is_empty() { derive_level_order(&entries) } else { level_order };
        DifficultyTable { name, symbol, level_order, entries, skipped: 0, course }
    }

    /// The symbol the table's header names, which the reference titles each level folder with
    /// (`tag + level`), or `None` when it names none: no header, no symbol in it, or an empty one.
    pub fn named_symbol(&self) -> Option<&str> {
        let symbol = self.symbol.trim();
        (!symbol.is_empty() && symbol != ABSENT_SYMBOL).then_some(symbol)
    }

    /// This table, recording how many body items were left out of it.
    pub(crate) fn skipping(mut self, skipped: usize) -> DifficultyTable {
        self.skipped = skipped;
        self
    }

    /// Parse a body (`data.json`, a JSON array) with an optional header. An item that names no
    /// chart — neither an `md5` nor a `sha256` — is left out rather than failing the table.
    pub fn parse_body(bytes: &[u8], header: Option<TableHeader>) -> Result<DifficultyTable, TableError> {
        let (entries, skipped) = parse_entries(bytes)?;
        Ok(DifficultyTable::from_parts(header, entries).skipping(skipped))
    }

    /// Fetch a table over HTTP. `url` may be the table's web page (whose `bmstable` meta tag names
    /// the header), a `header.json` (a JSON object with a `data_url`) or a body `data.json` (a JSON
    /// array); the shape is detected from the response. When given a body, a sibling `header.json`
    /// is fetched best-effort for the display name/symbol. All the requests together are given
    /// a fixed time, so an address that never finishes answering is a failure rather than a wait.
    pub fn fetch(url: &str) -> Result<DifficultyTable, TableError> {
        let client = fetch::build_client().map_err(|e| e.at(TableStage::Address, url))?;
        let deadline = std::time::Instant::now() + fetch::LOAD_TIMEOUT;
        fetch::fetch_with(url, &mut |address| fetch::http_get(&client, address, deadline))
    }

    /// Entries grouped by level in `level_order`, as `(level, [entry index])`. Empty levels
    /// are dropped; entry levels not present in `level_order` are appended in first-seen order.
    pub fn by_level(&self) -> Vec<(String, Vec<usize>)> {
        let pos: HashMap<&str, usize> = self.level_order.iter().enumerate().map(|(i, l)| (l.as_str(), i)).collect();
        let mut groups: Vec<(String, Vec<usize>)> = self.level_order.iter().map(|l| (l.clone(), Vec::new())).collect();
        let mut extra: Vec<(String, Vec<usize>)> = Vec::new();
        for (ei, e) in self.entries.iter().enumerate() {
            match pos.get(e.level.as_str()) {
                Some(&gi) => groups[gi].1.push(ei),
                None => match extra.iter_mut().find(|(l, _)| *l == e.level) {
                    Some((_, v)) => v.push(ei),
                    None => extra.push((e.level.clone(), vec![ei])),
                },
            }
        }
        groups.extend(extra);
        groups.retain(|(_, v)| !v.is_empty());
        groups
    }

    /// Match the table against a local library and group the library positions by level.
    ///
    /// `library_md5s` yields the md5 of every library chart in library order (case
    /// insensitive); the returned indices are positions in that iteration. Levels follow
    /// [`DifficultyTable::by_level`] and each index list is sorted and deduplicated.
    ///
    /// A level that owns no local chart is kept, with an empty list: the levels are the table's,
    /// not the library's, so a table whose charts are not owned yet still shows as the folders it
    /// is made of rather than as nothing at all.
    pub fn match_levels<'a>(&self, library_md5s: impl Iterator<Item = &'a str>) -> Vec<(String, Vec<usize>)> {
        self.match_levels_in(&Md5Index::new(library_md5s))
    }

    /// [`DifficultyTable::match_levels`] against a library that has been indexed already, which is
    /// how several tables are matched against one library without each indexing it again.
    pub fn match_levels_in(&self, library: &Md5Index) -> Vec<(String, Vec<usize>)> {
        let by_md5 = &library.by_md5;
        let mut out = Vec::new();
        for (level, entry_idxs) in self.by_level() {
            let mut idxs = Vec::new();
            for ei in entry_idxs {
                let md5 = self.entries[ei].md5.trim();
                if !md5.is_empty()
                    && let Some(v) = by_md5.get(&md5.to_ascii_lowercase())
                {
                    idxs.extend(v.iter().copied());
                }
            }
            idxs.sort_unstable();
            idxs.dedup();
            out.push((level, idxs));
        }
        out
    }
}

pub(crate) fn derive_level_order(entries: &[TableEntry]) -> Vec<String> {
    let mut levels: Vec<String> = entries.iter().map(|e| e.level.clone()).collect();
    levels.sort_by_key(|l| level_key(l));
    levels.dedup_by(|a, b| level_key(a) == level_key(b));
    levels
}

/// Numeric levels sort ascending and before any non-numeric level (e.g. "???").
fn level_key(s: &str) -> (u8, i64, String) {
    match s.parse::<i64>() {
        Ok(n) => (0, n, String::new()),
        Err(_) => (1, 0, s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::join_url;

    const BODY: &[u8] = br#"[
        {"md5":"aa","level":"3","title":"A"},
        {"md5":"bb","level":"1","title":"B"},
        {"md5":"cc","level":"3","title":"C"},
        {"md5":"dd","level":"???","title":"D"},
        {"md5":"ee","level":"10","title":"E"}
    ]"#;

    #[test]
    fn derives_numeric_aware_level_order() {
        let t = DifficultyTable::parse_body(BODY, None).unwrap();
        assert_eq!(t.level_order, vec!["1", "3", "10", "???"], "numeric ascending then non-numeric last");
        assert_eq!(t.symbol, "*");
    }

    #[test]
    fn groups_by_level_dropping_empties() {
        let t = DifficultyTable::parse_body(BODY, None).unwrap();
        let g = t.by_level();
        assert_eq!(g.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(), vec!["1", "3", "10", "???"]);
        let three = g.iter().find(|(l, _)| l == "3").unwrap();
        assert_eq!(three.1.len(), 2, "two charts at level 3");
    }

    #[test]
    fn header_level_order_and_symbol_are_used() {
        let header = TableHeader {
            name: Some("Insane".into()),
            symbol: Some("★".into()),
            data_url: vec!["data.json".into()],
            course: None,
            level_order: Some(vec!["1".into(), "2".into(), "3".into(), "10".into(), "???".into()]),
        };
        let t = DifficultyTable::parse_body(BODY, Some(header)).unwrap();
        assert_eq!(t.symbol, "★");
        assert_eq!(t.name, "Insane");
        let g = t.by_level();
        assert_eq!(g.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(), vec!["1", "3", "10", "???"], "header order, empties dropped (no level 2 owned)");
    }

    #[test]
    fn join_url_handles_relative_root_relative_absolute_and_dotdot() {
        assert_eq!(join_url("https://x.club/t/insane1/header.json", "data.json"), "https://x.club/t/insane1/data.json");
        assert_eq!(join_url("https://x.club/t/insane1/header.json", "/abs/data.json"), "https://x.club/abs/data.json");
        assert_eq!(join_url("https://x.club/t/insane1/header.json", "../data.json"), "https://x.club/t/data.json");
        assert_eq!(join_url("https://x.club/t/insane1/header.json", "https://other/data.json"), "https://other/data.json");
    }

    fn entry(md5: &str, level: &str) -> TableEntry {
        serde_json::from_value(serde_json::json!({ "md5": md5, "level": level })).unwrap()
    }

    fn levels_of(g: &[(String, Vec<usize>)]) -> Vec<&str> {
        g.iter().map(|(l, _)| l.as_str()).collect()
    }

    #[test]
    fn level_key_numeric_sorts_before_non_numeric() {
        assert!(level_key("10") < level_key("???"));
        assert!(level_key("999999") < level_key("0a"));
    }

    #[test]
    fn level_key_numeric_ascending_not_lexicographic() {
        assert!(level_key("9") < level_key("10"));
        assert!(level_key("2") < level_key("10"));
    }

    #[test]
    fn level_key_negative_numbers_sort_before_positive() {
        assert!(level_key("-5") < level_key("0"));
        assert!(level_key("-1") < level_key("1"));
    }

    #[test]
    fn level_key_leading_zero_and_plus_and_negzero_parse_as_numeric() {
        assert_eq!(level_key("05").0, 0);
        assert_eq!(level_key("+5").0, 0);
        assert_eq!(level_key("-0").0, 0);
        assert_eq!(level_key("05"), level_key("5"));
        assert_eq!(level_key("-0"), level_key("0"));
    }

    #[test]
    fn level_key_whitespace_and_underscore_and_hex_are_non_numeric() {
        assert_eq!(level_key(" 5").0, 1, "leading space is not a valid i64");
        assert_eq!(level_key("5_000").0, 1, "underscore separator not parsed");
        assert_eq!(level_key("0x10").0, 1, "hex prefix not parsed");
        assert_eq!(level_key("").0, 1, "empty string not numeric");
    }

    #[test]
    fn level_key_overflow_falls_back_to_non_numeric() {
        assert_eq!(level_key("99999999999999999999").0, 1);
    }

    #[test]
    fn level_key_non_numeric_sorts_by_full_string_uppercase_first() {
        assert!(level_key("Apple") < level_key("apple"));
        assert!(level_key("apple") < level_key("zz"));
    }

    #[test]
    fn derive_level_order_empty_entries_is_empty() {
        assert!(derive_level_order(&[]).is_empty());
    }

    #[test]
    fn derive_level_order_dedups_and_sorts() {
        let entries = vec![entry("a", "3"), entry("b", "1"), entry("c", "3"), entry("d", "10"), entry("e", "1")];
        assert_eq!(derive_level_order(&entries), vec!["1", "3", "10"]);
    }

    #[test]
    fn derive_level_order_dedup_keeps_first_seen_textual_form() {
        let entries = vec![entry("a", "5"), entry("b", "05")];
        let order = derive_level_order(&entries);
        assert_eq!(order, vec!["5"], "first textual form ('5') survives dedup");
    }

    #[test]
    fn derive_level_order_dedup_keeps_first_seen_when_reversed() {
        let entries = vec![entry("a", "05"), entry("b", "5")];
        let order = derive_level_order(&entries);
        assert_eq!(order, vec!["05"], "first textual form ('05') survives dedup");
    }

    #[test]
    fn derive_level_order_numeric_before_non_numeric() {
        let entries = vec![entry("a", "???"), entry("b", "10"), entry("c", "2"), entry("d", "beginner")];
        let order = derive_level_order(&entries);
        assert_eq!(order, vec!["2", "10", "???", "beginner"]);
    }

    #[test]
    fn derive_level_order_negative_levels_sort_first() {
        let entries = vec![entry("a", "1"), entry("b", "-3"), entry("c", "0")];
        assert_eq!(derive_level_order(&entries), vec!["-3", "0", "1"]);
    }

    #[test]
    fn derive_level_order_length_never_exceeds_entry_count() {
        let entries = vec![entry("a", "1"), entry("b", "1"), entry("c", "2"), entry("d", "2")];
        let order = derive_level_order(&entries);
        assert!(order.len() <= entries.len());
        assert_eq!(order.len(), 2, "two distinct levels");
    }

    #[test]
    fn by_level_empty_table_yields_no_groups() {
        let t = DifficultyTable::from_parts(None, vec![]);
        assert!(t.by_level().is_empty());
    }

    #[test]
    fn by_level_preserves_total_entry_count() {
        let t = DifficultyTable::parse_body(BODY, None).unwrap();
        let total: usize = t.by_level().iter().map(|(_, v)| v.len()).sum();
        assert_eq!(total, t.entries.len(), "every entry appears exactly once across groups");
    }

    #[test]
    fn by_level_indices_are_valid_and_unique() {
        let t = DifficultyTable::parse_body(BODY, None).unwrap();
        let mut all: Vec<usize> = t.by_level().into_iter().flat_map(|(_, v)| v).collect();
        all.sort_unstable();
        let count = all.len();
        all.dedup();
        assert_eq!(all.len(), count, "no index appears twice");
        assert!(all.iter().all(|&i| i < t.entries.len()), "all indices in range");
    }

    #[test]
    fn by_level_index_points_to_matching_level() {
        let t = DifficultyTable::parse_body(BODY, None).unwrap();
        for (level, idxs) in t.by_level() {
            for i in idxs {
                assert_eq!(t.entries[i].level, level, "grouped index must match its level");
            }
        }
    }

    #[test]
    fn by_level_groups_follow_level_order_then_extras_first_seen() {
        let header = TableHeader { name: None, symbol: None, data_url: Vec::new(), course: None, level_order: Some(vec!["1".into(), "2".into()]) };
        let entries = vec![entry("a", "3"), entry("b", "x"), entry("c", "1"), entry("d", "3")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        let g = t.by_level();
        assert_eq!(levels_of(&g), vec!["1", "3", "x"]);
    }

    #[test]
    fn by_level_drops_listed_levels_with_no_entries() {
        let header = TableHeader { name: None, symbol: None, data_url: Vec::new(), course: None, level_order: Some(vec!["1".into(), "2".into(), "3".into()]) };
        let entries = vec![entry("a", "2")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        assert_eq!(levels_of(&t.by_level()), vec!["2"], "only the non-empty listed level remains");
    }

    #[test]
    fn by_level_extra_level_accumulates_all_its_entries() {
        let header = TableHeader { name: None, symbol: None, data_url: Vec::new(), course: None, level_order: Some(vec!["1".into()]) };
        let entries = vec![entry("a", "99"), entry("b", "99"), entry("c", "1"), entry("d", "99")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        let g = t.by_level();
        let extra = g.iter().find(|(l, _)| l == "99").unwrap();
        assert_eq!(extra.1.len(), 3, "all three '99' entries land in one extra group");
    }

    #[test]
    fn by_level_duplicate_levels_in_level_order_only_first_slot_is_filled() {
        let header = TableHeader { name: None, symbol: None, data_url: Vec::new(), course: None, level_order: Some(vec!["1".into(), "1".into()]) };
        let entries = vec![entry("a", "1")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        let g = t.by_level();
        assert_eq!(levels_of(&g), vec!["1"]);
        assert_eq!(g[0].1.len(), 1);
    }

    #[test]
    fn from_parts_none_header_defaults() {
        let t = DifficultyTable::from_parts(None, vec![entry("a", "1")]);
        assert_eq!(t.name, "");
        assert_eq!(t.symbol, "*");
        assert_eq!(t.level_order, vec!["1"], "derived from entries");
    }

    #[test]
    fn from_parts_header_with_all_none_fields_uses_defaults() {
        let header = TableHeader { name: None, symbol: None, data_url: Vec::new(), course: None, level_order: None };
        let t = DifficultyTable::from_parts(Some(header), vec![entry("a", "2"), entry("b", "1")]);
        assert_eq!(t.name, "", "missing name -> empty");
        assert_eq!(t.symbol, "*", "missing symbol -> '*'");
        assert_eq!(t.level_order, vec!["1", "2"], "missing level_order -> derived");
    }

    #[test]
    fn from_parts_empty_symbol_string_is_preserved_not_defaulted() {
        let header = TableHeader { name: Some("".into()), symbol: Some("".into()), data_url: Vec::new(), course: None, level_order: None };
        let t = DifficultyTable::from_parts(Some(header), vec![entry("a", "1")]);
        assert_eq!(t.symbol, "", "explicit empty symbol is NOT replaced by '*'");
        assert_eq!(t.name, "");
    }

    #[test]
    fn named_symbol_is_none_without_a_header_symbol() {
        let with_symbol =
            |symbol: Option<&str>| TableHeader { name: None, symbol: symbol.map(str::to_string), data_url: Vec::new(), course: None, level_order: None };
        let named = |header: Option<TableHeader>| DifficultyTable::from_parts(header, vec![entry("a", "1")]).named_symbol().map(str::to_string);
        assert_eq!(named(None), None, "a table with no header names no symbol");
        assert_eq!(named(Some(with_symbol(None))), None, "the stand-in for a header with no symbol is not a symbol");
        assert_eq!(named(Some(with_symbol(Some("")))), None, "an empty symbol is none");
        assert_eq!(named(Some(with_symbol(Some("\u{2605}")))), Some("\u{2605}".to_string()));
        assert_eq!(named(Some(with_symbol(Some("st")))), Some("st".to_string()));
    }

    #[test]
    fn from_parts_explicit_empty_level_order_is_derived() {
        let header = TableHeader { name: None, symbol: Some("X".into()), data_url: Vec::new(), course: None, level_order: Some(vec![]) };
        let t = DifficultyTable::from_parts(Some(header), vec![entry("a", "3"), entry("b", "1")]);
        assert_eq!(t.level_order, vec!["1", "3"], "empty level_order is derived from entries");
    }

    #[test]
    fn from_parts_non_empty_level_order_is_used_verbatim_even_if_unsorted() {
        let header = TableHeader { name: None, symbol: None, data_url: Vec::new(), course: None, level_order: Some(vec!["10".into(), "1".into(), "5".into()]) };
        let t = DifficultyTable::from_parts(Some(header), vec![entry("a", "1")]);
        assert_eq!(t.level_order, vec!["10", "1", "5"], "explicit order preserved as-is, not re-sorted");
    }

    #[test]
    fn from_parts_does_not_mutate_or_drop_entries() {
        let entries = vec![entry("a", "1"), entry("b", "2"), entry("c", "3")];
        let t = DifficultyTable::from_parts(None, entries.clone());
        assert_eq!(t.entries.len(), entries.len());
        assert_eq!(t.entries[0].md5, "a");
    }

    #[test]
    fn from_body_bytes_empty_array_yields_empty_table() {
        let t = DifficultyTable::parse_body(b"[]", None).unwrap();
        assert!(t.entries.is_empty());
        assert!(t.level_order.is_empty());
        assert!(t.by_level().is_empty());
        assert_eq!(t.symbol, "*");
    }

    fn body_err(bytes: &[u8]) -> String {
        match DifficultyTable::parse_body(bytes, None) {
            Ok(_) => panic!("expected Err for {:?}", String::from_utf8_lossy(bytes)),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn from_body_bytes_malformed_json_is_err() {
        let err = body_err(b"not json");
        assert!(err.starts_with("body parse:"), "error is prefixed: {err}");
    }

    #[test]
    fn from_body_bytes_object_instead_of_array_is_err() {
        let err = body_err(br#"{"md5":"a"}"#);
        assert!(err.starts_with("body parse:"));
    }

    #[test]
    fn an_item_that_names_no_chart_is_left_out_rather_than_failing_the_table() {
        let t = DifficultyTable::parse_body(br#"[{"level":"1"},{"md5":"aa","level":"1"},null,7,{"md5":"","sha256":" ","level":"2"}]"#, None).unwrap();
        assert_eq!(t.entries.len(), 1, "only the item with a hash is an entry");
        assert_eq!(t.entries[0].md5, "aa");
        assert_eq!(t.skipped, 4, "what was left out is counted, so the loader can say so");
        assert_eq!(t.level_order, vec!["1"], "a level only skipped items had is not a level");
    }

    #[test]
    fn an_entry_with_only_a_sha256_is_kept() {
        let t = DifficultyTable::parse_body(br#"[{"sha256":"ABCD","level":"3"}]"#, None).unwrap();
        assert_eq!(t.entries.len(), 1);
        assert_eq!(t.entries[0].sha256, "ABCD");
        assert_eq!(t.entries[0].md5, "");
        assert_eq!(t.skipped, 0);
        assert_eq!(t.match_levels(["", "aa"].into_iter()), vec![("3".to_string(), Vec::new())], "an empty md5 matches no chart, not the charts without one");
    }

    #[test]
    fn a_level_written_as_a_number_is_the_level_written_as_text() {
        let t = DifficultyTable::parse_body(br#"[{"md5":"aa","level":12},{"md5":"bb","level":"12"},{"md5":"cc","level":0.5},{"md5":"dd","level":null}]"#, None)
            .unwrap();
        let levels: Vec<&str> = t.entries.iter().map(|e| e.level.as_str()).collect();
        assert_eq!(levels, vec!["12", "12", "0.5", ""]);
        assert_eq!(t.by_level().iter().find(|(l, _)| l == "12").map(|(_, v)| v.len()), Some(2), "both spellings are one level");
    }

    #[test]
    fn a_body_with_a_byte_order_mark_or_stray_bytes_in_its_text_still_parses() {
        let marked = [UTF8_BOM, br#"[{"md5":"aa","level":"1"}]"#.as_slice()].concat();
        assert_eq!(DifficultyTable::parse_body(&marked, None).unwrap().entries.len(), 1, "a byte-order mark is not part of the JSON");
        let other_encoding = [br#"[{"md5":"aa","level":"1","title":""#.as_slice(), b"\x94\xad\x8b\xb6".as_slice(), br#""}]"#.as_slice()].concat();
        let t = DifficultyTable::parse_body(&other_encoding, None).unwrap();
        assert_eq!(t.entries[0].md5, "aa", "the hashes of a table in another encoding are still ASCII");
    }

    /// A table served in Shift_JIS is named in Shift_JIS. Read as damaged UTF-8 its name is a row of
    /// replacement characters, and a character whose second byte is a backslash takes the quote
    /// that closes the name with it.
    #[test]
    fn a_table_written_in_shift_jis_keeps_its_name_and_its_symbol() {
        let header = "{\"name\":\"\u{767a}\u{72c2}\u{96e3}\u{6613}\u{5ea6}\u{8868}\",\"symbol\":\"\u{2605}\",\"data_url\":\"data.json\"}";
        let (bytes, _, unmappable) = encoding_rs::SHIFT_JIS.encode(header);
        assert!(!unmappable && std::str::from_utf8(&bytes).is_err(), "the sample is Shift_JIS and not UTF-8");
        let header = parse_header(&bytes).unwrap_or_else(|e| panic!("a Shift_JIS header is a header: {e}"));
        assert_eq!(header.name.as_deref(), Some("\u{767a}\u{72c2}\u{96e3}\u{6613}\u{5ea6}\u{8868}"));
        assert_eq!(header.symbol.as_deref(), Some("\u{2605}"));

        let (body, _, _) = encoding_rs::SHIFT_JIS.encode("[{\"md5\":\"aa\",\"level\":\"1\",\"title\":\"\u{8868}\"}]");
        let table = DifficultyTable::parse_body(&body, None).unwrap_or_else(|e| panic!("a Shift_JIS body is a body: {e}"));
        assert_eq!(table.entries[0].md5, "aa");
    }

    #[test]
    fn from_body_bytes_entry_only_md5_uses_serde_defaults() {
        let t = DifficultyTable::parse_body(br#"[{"md5":"abc"}]"#, None).unwrap();
        assert_eq!(t.entries.len(), 1);
        let e = &t.entries[0];
        assert_eq!(e.md5, "abc");
        assert_eq!(e.level, "");
        assert_eq!(e.title, "");
        assert_eq!(e.artist, "");
        assert_eq!(e.url, "");
        assert_eq!(e.url_diff, "");
        assert_eq!(t.level_order, vec![""]);
    }

    #[test]
    fn from_body_bytes_ignores_unknown_fields() {
        let t = DifficultyTable::parse_body(br#"[{"md5":"x","extra":42,"sha256":"deadbeef"}]"#, None).unwrap();
        assert_eq!(t.entries.len(), 1);
        assert_eq!(t.entries[0].md5, "x");
    }

    #[test]
    fn from_body_bytes_empty_input_is_err() {
        assert!(DifficultyTable::parse_body(b"", None).is_err());
    }

    #[test]
    fn from_body_bytes_roundtrips_through_serde() {
        let t = DifficultyTable::parse_body(BODY, None).unwrap();
        let re = serde_json::to_vec(&t.entries).unwrap();
        let again = DifficultyTable::parse_body(&re, None).unwrap();
        assert_eq!(again.entries.len(), t.entries.len());
        assert_eq!(again.level_order, t.level_order, "re-serialized body derives the same order");
    }

    #[test]
    fn from_body_bytes_is_deterministic() {
        let a = DifficultyTable::parse_body(BODY, None).unwrap();
        let b = DifficultyTable::parse_body(BODY, None).unwrap();
        assert_eq!(a.level_order, b.level_order);
        assert_eq!(a.by_level(), b.by_level());
    }

    #[test]
    fn header_deserializes_from_empty_object_with_all_none() {
        let h: TableHeader = serde_json::from_str("{}").unwrap();
        assert!(h.name.is_none());
        assert!(h.symbol.is_none());
        assert!(h.data_url.is_empty());
        assert!(h.level_order.is_none());
    }

    #[test]
    fn header_deserializes_partial_fields() {
        let h: TableHeader = serde_json::from_str(r#"{"name":"N","data_url":"d.json"}"#).unwrap();
        assert_eq!(h.name.as_deref(), Some("N"));
        assert_eq!(h.data_url, vec!["d.json"]);
        assert!(h.symbol.is_none());
    }

    #[test]
    fn a_header_reads_the_fields_tables_write_in_more_than_one_way() {
        let h: TableHeader = parse_header(
            br#"{"name":"N","symbol":7,"data_url":["a.json","b.json"],"level_order":[0,1,"2","11+",null,12.5],"tag":"T","course":[[{"name":"c","md5":["aa"]}]],"last_update":"2001.01.01"}"#,
        )
        .unwrap();
        assert_eq!(h.symbol.as_deref(), Some("7"), "a symbol written as a number");
        assert_eq!(h.data_url, vec!["a.json", "b.json"], "a table split over several bodies");
        assert_eq!(h.level_order, Some(vec!["0".into(), "1".into(), "2".into(), "11+".into(), "12.5".into()]), "numbers and text mixed, in the order written");

        let odd: TableHeader = parse_header(br#"{"name":null,"data_url":null,"level_order":"1,2,3"}"#).unwrap();
        assert!(odd.name.is_none() && odd.data_url.is_empty() && odd.level_order.is_none(), "a field of the wrong kind reads as absent");
        assert!(parse_header(b"[]").is_err(), "a header is an object");
    }

    #[test]
    fn header_overrides_name_and_symbol_over_derived_defaults() {
        let header = TableHeader { name: Some("My Table".into()), symbol: Some("▼".into()), data_url: Vec::new(), course: None, level_order: None };
        let t = DifficultyTable::parse_body(br#"[{"md5":"a","level":"1"}]"#, Some(header)).unwrap();
        assert_eq!(t.name, "My Table");
        assert_eq!(t.symbol, "▼");
    }

    #[test]
    fn join_url_unparseable_base_passes_through_rel() {
        assert_eq!(join_url("not a url", "data.json"), "data.json");
        assert_eq!(join_url("", "data.json"), "data.json");
        assert_eq!(join_url("relative/path/header.json", "data.json"), "data.json");
    }

    #[test]
    fn join_url_query_only_rel_keeps_path() {
        let out = join_url("https://x.club/t/insane1/header.json", "?v=2");
        assert_eq!(out, "https://x.club/t/insane1/header.json?v=2");
    }

    #[test]
    fn join_url_empty_rel_returns_base_without_fragment() {
        let out = join_url("https://x.club/t/insane1/header.json", "");
        assert_eq!(out, "https://x.club/t/insane1/header.json");
    }

    #[test]
    fn join_url_excess_dotdot_clamps_at_root() {
        let out = join_url("https://x.club/t/insane1/header.json", "../../../../data.json");
        assert_eq!(out, "https://x.club/data.json", "dotdot does not escape the host");
    }

    #[test]
    fn join_url_protocol_relative_inherits_scheme() {
        let out = join_url("https://x.club/t/insane1/header.json", "//other.com/d.json");
        assert_eq!(out, "https://other.com/d.json");
    }

    #[test]
    fn join_url_absolute_rel_with_different_scheme_replaces_base() {
        let out = join_url("https://x.club/t/header.json", "http://plain.example/d.json");
        assert_eq!(out, "http://plain.example/d.json");
    }

    fn matched(table: &DifficultyTable, library_md5s: &[&str]) -> Vec<(String, Vec<usize>)> {
        table.match_levels(library_md5s.iter().copied())
    }

    #[test]
    fn match_levels_groups_library_positions_by_level_and_keeps_the_levels_nothing_is_owned_in() {
        let t = DifficultyTable::parse_body(BODY, None).unwrap();
        let out = matched(&t, &["bb", "AA", "zz", "cc", "ee", "aa"]);
        assert_eq!(levels_of(&out), vec!["1", "3", "10", "???"], "every level of the table, owned or not");
        let three = out.iter().find(|(l, _)| l == "3").unwrap();
        assert_eq!(three.1, vec![1, 3, 5], "duplicate md5 'aa' contributes both library positions, sorted");
        assert!(out.iter().find(|(l, _)| l == "???").unwrap().1.is_empty(), "'???' owns no local chart and says so by being empty");
    }

    #[test]
    fn match_levels_on_an_empty_library_is_the_tables_levels_with_nothing_in_them() {
        let t = DifficultyTable::parse_body(BODY, None).unwrap();
        let out = matched(&t, &[]);
        assert_eq!(levels_of(&out), vec!["1", "3", "10", "???"], "a table none of whose charts are owned is still its levels");
        assert!(out.iter().all(|(_, owned)| owned.is_empty()));
    }

    #[test]
    fn match_levels_on_an_empty_table_is_empty() {
        let t = DifficultyTable::from_parts(None, vec![]);
        assert!(matched(&t, &["aa", "bb"]).is_empty());
    }

    #[test]
    fn match_levels_is_case_insensitive_on_both_sides() {
        let t = DifficultyTable::from_parts(None, vec![entry("AA", "1"), entry("Bb", "2")]);
        assert_eq!(matched(&t, &["aa", "bB"]), vec![("1".to_string(), vec![0]), ("2".to_string(), vec![1])]);
    }

    #[test]
    fn match_levels_deduplicates_a_table_entry_repeated_in_one_level() {
        let t = DifficultyTable::from_parts(None, vec![entry("aa", "5"), entry("aa", "5"), entry("bb", "5")]);
        assert_eq!(matched(&t, &["aa", "bb"]), vec![("5".to_string(), vec![0, 1])]);
    }

    #[test]
    fn match_levels_follows_the_header_level_order_then_the_extras() {
        let header = TableHeader { name: None, symbol: None, data_url: Vec::new(), course: None, level_order: Some(vec!["2".into(), "1".into()]) };
        let entries = vec![entry("aa", "1"), entry("bb", "2"), entry("cc", "99"), entry("dd", "3")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        let out = matched(&t, &["cc", "aa", "bb"]);
        assert_eq!(levels_of(&out), vec!["2", "1", "99", "3"], "header order first, then extras as first seen");
        assert_eq!(out[3], ("3".to_string(), Vec::new()), "the unowned level is there and empty");
    }

    /// Several tables are matched against one library, which is indexed for all of them once.
    #[test]
    fn a_library_indexed_once_matches_every_table_as_matching_it_afresh_does() {
        let library = ["bb", "AA", "zz", "cc", "ee", "aa"];
        let index = Md5Index::new(library.iter().copied());
        let one = DifficultyTable::parse_body(BODY, None).unwrap();
        let other = DifficultyTable::from_parts(None, vec![entry("ZZ", "7"), entry("nope", "8")]);
        assert_eq!(one.match_levels_in(&index), matched(&one, &library));
        assert_eq!(other.match_levels_in(&index), vec![("7".to_string(), vec![2]), ("8".to_string(), Vec::new())]);
    }

    #[test]
    fn the_courses_a_header_lists_are_carried_with_the_table() {
        let header = parse_header(br#"{"name":"N","data_url":"d.json","course":[[{"name":"c","md5":["aa"]}]]}"#).unwrap();
        let table = DifficultyTable::from_parts(Some(header), vec![entry("aa", "1")]);
        assert_eq!(table.course, Some(serde_json::json!([[{ "name": "c", "md5": ["aa"] }]])));
        assert_eq!(DifficultyTable::from_parts(None, Vec::new()).course, None, "a table with no header has no courses");
    }

    #[test]
    fn match_levels_accepts_any_str_iterator() {
        let t = DifficultyTable::from_parts(None, vec![entry("aa", "1")]);
        let md5s = ["zz", "aa"];
        assert_eq!(t.match_levels(md5s.into_iter()), vec![("1".to_string(), vec![1])]);
    }
}
