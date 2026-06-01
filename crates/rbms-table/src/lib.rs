use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

mod dto;
pub use dto::{TableEntry, TableHeader};

/// On-disk cache envelope: the resolved header metadata plus the body, so an offline reload
/// preserves the table's name/symbol/level order rather than re-deriving them.
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedTable {
    name: String,
    symbol: String,
    level_order: Vec<String>,
    entries: Vec<TableEntry>,
}

/// A BMS difficulty table resolved into header metadata plus body entries, browsable by
/// level. Built from a `header.json` (+ its `data_url`) or directly from a body `data.json`.
pub struct DifficultyTable {
    pub name: String,
    pub symbol: String,
    pub level_order: Vec<String>,
    pub entries: Vec<TableEntry>,
}

impl DifficultyTable {
    /// Combine an optional header with body entries. When the header omits `level_order`,
    /// it is derived from the entries with a numeric-aware sort (so "10" follows "9", and
    /// non-numeric levels like "???" sort last).
    pub fn from_parts(header: Option<TableHeader>, entries: Vec<TableEntry>) -> DifficultyTable {
        let (name, symbol, level_order) = match header {
            Some(h) => (h.name.unwrap_or_default(), h.symbol.unwrap_or_else(|| "*".into()), h.level_order.unwrap_or_default()),
            None => (String::new(), "*".into(), Vec::new()),
        };
        let level_order = if level_order.is_empty() { derive_level_order(&entries) } else { level_order };
        DifficultyTable { name, symbol, level_order, entries }
    }

    /// Parse a body (`data.json`, a JSON array) with an optional header.
    pub fn from_body_bytes(bytes: &[u8], header: Option<TableHeader>) -> Result<DifficultyTable, String> {
        let entries: Vec<TableEntry> = serde_json::from_slice(bytes).map_err(|e| format!("body parse: {e}"))?;
        Ok(DifficultyTable::from_parts(header, entries))
    }

    /// Fetch a table over HTTP. `url` may point at a `header.json` (a JSON object with a
    /// `data_url`) or directly at a body `data.json` (a JSON array); the shape is detected
    /// from the response. When given a body, a sibling `header.json` is fetched best-effort
    /// for the display name/symbol.
    pub fn fetch(url: &str) -> Result<DifficultyTable, String> {
        let client = build_client()?;
        let raw = get_bytes(&client, url)?;
        if serde_json::from_slice::<Vec<TableEntry>>(&raw).is_ok() {
            let header = sibling_header(&client, url);
            return DifficultyTable::from_body_bytes(&raw, header);
        }
        let header: TableHeader = serde_json::from_slice(&raw).map_err(|e| format!("header parse: {e}"))?;
        let data_url = header.data_url.clone().ok_or_else(|| "header.json has no data_url".to_string())?;
        let body = get_bytes(&client, &join_url(url, &data_url))?;
        DifficultyTable::from_body_bytes(&body, Some(header))
    }

    /// Fetch a table, caching the resolved body to `cache_path`; on a failed fetch, fall
    /// back to that cache so a previously-seen table still works offline.
    pub fn fetch_or_cache(url: &str, cache_path: &Path) -> Result<DifficultyTable, String> {
        match DifficultyTable::fetch(url) {
            Ok(table) => {
                if !table.entries.is_empty() {
                    let cached = CachedTable {
                        name: table.name.clone(),
                        symbol: table.symbol.clone(),
                        level_order: table.level_order.clone(),
                        entries: table.entries.clone(),
                    };
                    if let Ok(json) = serde_json::to_vec(&cached) {
                        let _ = std::fs::write(cache_path, json);
                    }
                }
                Ok(table)
            }
            Err(e) => {
                let bytes = std::fs::read(cache_path).map_err(|_| format!("table fetch failed ({e}); no cache at {}", cache_path.display()))?;
                let cached: CachedTable = serde_json::from_slice(&bytes).map_err(|err| format!("table fetch failed ({e}); cache parse: {err}"))?;
                Ok(DifficultyTable {
                    level_order: if cached.level_order.is_empty() { derive_level_order(&cached.entries) } else { cached.level_order },
                    name: cached.name,
                    symbol: cached.symbol,
                    entries: cached.entries,
                })
            }
        }
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
}

fn derive_level_order(entries: &[TableEntry]) -> Vec<String> {
    let mut levels: Vec<String> = entries.iter().map(|e| e.level.clone()).collect();
    levels.sort_by(|a, b| level_key(a).cmp(&level_key(b)));
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

/// Resolve a possibly-relative `data_url`/header location against the base URL per RFC 3986
/// (handles relative, root-relative `/abs`, absolute, and `..` segments). Uses reqwest's
/// re-exported `url::Url`; falls back to the raw `rel` if the base is unparseable.
fn join_url(base: &str, rel: &str) -> String {
    match reqwest::Url::parse(base).and_then(|b| b.join(rel)) {
        Ok(u) => u.to_string(),
        Err(_) => rel.to_string(),
    }
}

fn build_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("rbms-table")
        .build()
        .map_err(|e| e.to_string())
}

fn get_bytes(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} for {url}", resp.status()));
    }
    Ok(resp.bytes().map_err(|e| e.to_string())?.to_vec())
}

fn sibling_header(client: &reqwest::blocking::Client, url: &str) -> Option<TableHeader> {
    let bytes = get_bytes(client, &join_url(url, "header.json")).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &[u8] = br#"[
        {"md5":"aa","level":"3","title":"A"},
        {"md5":"bb","level":"1","title":"B"},
        {"md5":"cc","level":"3","title":"C"},
        {"md5":"dd","level":"???","title":"D"},
        {"md5":"ee","level":"10","title":"E"}
    ]"#;

    #[test]
    fn derives_numeric_aware_level_order() {
        let t = DifficultyTable::from_body_bytes(BODY, None).unwrap();
        assert_eq!(t.level_order, vec!["1", "3", "10", "???"], "numeric ascending then non-numeric last");
        assert_eq!(t.symbol, "*");
    }

    #[test]
    fn groups_by_level_dropping_empties() {
        let t = DifficultyTable::from_body_bytes(BODY, None).unwrap();
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
            data_url: Some("data.json".into()),
            level_order: Some(vec!["1".into(), "2".into(), "3".into(), "10".into(), "???".into()]),
        };
        let t = DifficultyTable::from_body_bytes(BODY, Some(header)).unwrap();
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

    // ----- helpers --------------------------------------------------------

    fn entry(md5: &str, level: &str) -> TableEntry {
        serde_json::from_value(serde_json::json!({ "md5": md5, "level": level })).unwrap()
    }

    fn levels_of(g: &[(String, Vec<usize>)]) -> Vec<&str> {
        g.iter().map(|(l, _)| l.as_str()).collect()
    }

    // ----- level_key ------------------------------------------------------

    #[test]
    fn level_key_numeric_sorts_before_non_numeric() {
        assert!(level_key("10") < level_key("???"));
        assert!(level_key("999999") < level_key("0a"));
    }

    #[test]
    fn level_key_numeric_ascending_not_lexicographic() {
        // "10" must come AFTER "9" numerically, not before it lexicographically.
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
        // tag 0 == numeric group
        assert_eq!(level_key("05").0, 0);
        assert_eq!(level_key("+5").0, 0);
        assert_eq!(level_key("-0").0, 0);
        // equal numeric value yields equal key regardless of textual form
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
        // beyond i64 range -> treated as a non-numeric label, sorts in the alpha bucket
        assert_eq!(level_key("99999999999999999999").0, 1);
    }

    #[test]
    fn level_key_non_numeric_sorts_by_full_string_uppercase_first() {
        // Rust string Ord is by Unicode scalar; 'A' (0x41) < 'a' (0x61).
        assert!(level_key("Apple") < level_key("apple"));
        assert!(level_key("apple") < level_key("zz"));
    }

    // ----- derive_level_order --------------------------------------------

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
        // "5" and "05" share a numeric key; stable sort + dedup keeps the first occurrence.
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
        // numerics ascending first, then the two alpha labels in string order
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

    // ----- by_level -------------------------------------------------------

    #[test]
    fn by_level_empty_table_yields_no_groups() {
        let t = DifficultyTable::from_parts(None, vec![]);
        assert!(t.by_level().is_empty());
    }

    #[test]
    fn by_level_preserves_total_entry_count() {
        let t = DifficultyTable::from_body_bytes(BODY, None).unwrap();
        let total: usize = t.by_level().iter().map(|(_, v)| v.len()).sum();
        assert_eq!(total, t.entries.len(), "every entry appears exactly once across groups");
    }

    #[test]
    fn by_level_indices_are_valid_and_unique() {
        let t = DifficultyTable::from_body_bytes(BODY, None).unwrap();
        let mut all: Vec<usize> = t.by_level().into_iter().flat_map(|(_, v)| v).collect();
        all.sort_unstable();
        let count = all.len();
        all.dedup();
        assert_eq!(all.len(), count, "no index appears twice");
        assert!(all.iter().all(|&i| i < t.entries.len()), "all indices in range");
    }

    #[test]
    fn by_level_index_points_to_matching_level() {
        let t = DifficultyTable::from_body_bytes(BODY, None).unwrap();
        for (level, idxs) in t.by_level() {
            for i in idxs {
                assert_eq!(t.entries[i].level, level, "grouped index must match its level");
            }
        }
    }

    #[test]
    fn by_level_groups_follow_level_order_then_extras_first_seen() {
        // level_order lists "1","2" only; entries also have "3" then "x" (unlisted).
        let header = TableHeader {
            name: None,
            symbol: None,
            data_url: None,
            level_order: Some(vec!["1".into(), "2".into()]),
        };
        let entries = vec![entry("a", "3"), entry("b", "x"), entry("c", "1"), entry("d", "3")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        let g = t.by_level();
        // "2" listed but empty -> dropped. "1" listed and present -> kept first.
        // extras appended in first-seen order of the entry list: "3" then "x".
        assert_eq!(levels_of(&g), vec!["1", "3", "x"]);
    }

    #[test]
    fn by_level_drops_listed_levels_with_no_entries() {
        let header = TableHeader {
            name: None,
            symbol: None,
            data_url: None,
            level_order: Some(vec!["1".into(), "2".into(), "3".into()]),
        };
        let entries = vec![entry("a", "2")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        assert_eq!(levels_of(&t.by_level()), vec!["2"], "only the non-empty listed level remains");
    }

    #[test]
    fn by_level_extra_level_accumulates_all_its_entries() {
        let header = TableHeader { name: None, symbol: None, data_url: None, level_order: Some(vec!["1".into()]) };
        let entries = vec![entry("a", "99"), entry("b", "99"), entry("c", "1"), entry("d", "99")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        let g = t.by_level();
        let extra = g.iter().find(|(l, _)| l == "99").unwrap();
        assert_eq!(extra.1.len(), 3, "all three '99' entries land in one extra group");
    }

    #[test]
    fn by_level_duplicate_levels_in_level_order_only_first_slot_is_filled() {
        // pos map collapses duplicate keys to the LAST index, but the matching group
        // gets the entries; document the actual behavior rather than guessing.
        let header = TableHeader { name: None, symbol: None, data_url: None, level_order: Some(vec!["1".into(), "1".into()]) };
        let entries = vec![entry("a", "1")];
        let t = DifficultyTable::from_parts(Some(header), entries);
        let g = t.by_level();
        // Only one "1" group survives (the other is empty and dropped).
        assert_eq!(levels_of(&g), vec!["1"]);
        assert_eq!(g[0].1.len(), 1);
    }

    // ----- from_parts -----------------------------------------------------

    #[test]
    fn from_parts_none_header_defaults() {
        let t = DifficultyTable::from_parts(None, vec![entry("a", "1")]);
        assert_eq!(t.name, "");
        assert_eq!(t.symbol, "*");
        assert_eq!(t.level_order, vec!["1"], "derived from entries");
    }

    #[test]
    fn from_parts_header_with_all_none_fields_uses_defaults() {
        let header = TableHeader { name: None, symbol: None, data_url: None, level_order: None };
        let t = DifficultyTable::from_parts(Some(header), vec![entry("a", "2"), entry("b", "1")]);
        assert_eq!(t.name, "", "missing name -> empty");
        assert_eq!(t.symbol, "*", "missing symbol -> '*'");
        assert_eq!(t.level_order, vec!["1", "2"], "missing level_order -> derived");
    }

    #[test]
    fn from_parts_empty_symbol_string_is_preserved_not_defaulted() {
        // Default only applies when symbol is None; an explicit empty string stays empty.
        let header = TableHeader { name: Some("".into()), symbol: Some("".into()), data_url: None, level_order: None };
        let t = DifficultyTable::from_parts(Some(header), vec![entry("a", "1")]);
        assert_eq!(t.symbol, "", "explicit empty symbol is NOT replaced by '*'");
        assert_eq!(t.name, "");
    }

    #[test]
    fn from_parts_explicit_empty_level_order_is_derived() {
        // An empty Vec triggers the same derivation path as None.
        let header = TableHeader { name: None, symbol: Some("X".into()), data_url: None, level_order: Some(vec![]) };
        let t = DifficultyTable::from_parts(Some(header), vec![entry("a", "3"), entry("b", "1")]);
        assert_eq!(t.level_order, vec!["1", "3"], "empty level_order is derived from entries");
    }

    #[test]
    fn from_parts_non_empty_level_order_is_used_verbatim_even_if_unsorted() {
        let header = TableHeader { name: None, symbol: None, data_url: None, level_order: Some(vec!["10".into(), "1".into(), "5".into()]) };
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

    // ----- from_body_bytes ------------------------------------------------

    #[test]
    fn from_body_bytes_empty_array_yields_empty_table() {
        let t = DifficultyTable::from_body_bytes(b"[]", None).unwrap();
        assert!(t.entries.is_empty());
        assert!(t.level_order.is_empty());
        assert!(t.by_level().is_empty());
        assert_eq!(t.symbol, "*");
    }

    fn body_err(bytes: &[u8]) -> String {
        // DifficultyTable has no Debug impl, so unwrap_err is unavailable; match instead.
        match DifficultyTable::from_body_bytes(bytes, None) {
            Ok(_) => panic!("expected Err for {:?}", String::from_utf8_lossy(bytes)),
            Err(e) => e,
        }
    }

    #[test]
    fn from_body_bytes_malformed_json_is_err() {
        let err = body_err(b"not json");
        assert!(err.starts_with("body parse:"), "error is prefixed: {err}");
    }

    #[test]
    fn from_body_bytes_object_instead_of_array_is_err() {
        // body must be a JSON array; an object fails to deserialize into Vec<TableEntry>.
        let err = body_err(br#"{"md5":"a"}"#);
        assert!(err.starts_with("body parse:"));
    }

    #[test]
    fn from_body_bytes_entry_missing_md5_is_err() {
        // md5 is the only required field on TableEntry.
        let err = body_err(br#"[{"level":"1"}]"#);
        assert!(err.starts_with("body parse:"), "missing md5 rejected: {err}");
    }

    #[test]
    fn from_body_bytes_entry_only_md5_uses_serde_defaults() {
        let t = DifficultyTable::from_body_bytes(br#"[{"md5":"abc"}]"#, None).unwrap();
        assert_eq!(t.entries.len(), 1);
        let e = &t.entries[0];
        assert_eq!(e.md5, "abc");
        assert_eq!(e.level, "");
        assert_eq!(e.title, "");
        assert_eq!(e.artist, "");
        assert_eq!(e.url, "");
        assert_eq!(e.url_diff, "");
        // empty level still groups under "" via the derived order
        assert_eq!(t.level_order, vec![""]);
    }

    #[test]
    fn from_body_bytes_ignores_unknown_fields() {
        let t = DifficultyTable::from_body_bytes(br#"[{"md5":"x","extra":42,"sha256":"deadbeef"}]"#, None).unwrap();
        assert_eq!(t.entries.len(), 1);
        assert_eq!(t.entries[0].md5, "x");
    }

    #[test]
    fn from_body_bytes_empty_input_is_err() {
        assert!(DifficultyTable::from_body_bytes(b"", None).is_err());
    }

    #[test]
    fn from_body_bytes_roundtrips_through_serde() {
        let t = DifficultyTable::from_body_bytes(BODY, None).unwrap();
        let re = serde_json::to_vec(&t.entries).unwrap();
        let again = DifficultyTable::from_body_bytes(&re, None).unwrap();
        assert_eq!(again.entries.len(), t.entries.len());
        assert_eq!(again.level_order, t.level_order, "re-serialized body derives the same order");
    }

    #[test]
    fn from_body_bytes_is_deterministic() {
        let a = DifficultyTable::from_body_bytes(BODY, None).unwrap();
        let b = DifficultyTable::from_body_bytes(BODY, None).unwrap();
        assert_eq!(a.level_order, b.level_order);
        assert_eq!(a.by_level(), b.by_level());
    }

    // ----- TableHeader / TableEntry serde --------------------------------

    #[test]
    fn header_deserializes_from_empty_object_with_all_none() {
        let h: TableHeader = serde_json::from_str("{}").unwrap();
        assert!(h.name.is_none());
        assert!(h.symbol.is_none());
        assert!(h.data_url.is_none());
        assert!(h.level_order.is_none());
    }

    #[test]
    fn header_deserializes_partial_fields() {
        let h: TableHeader = serde_json::from_str(r#"{"name":"N","data_url":"d.json"}"#).unwrap();
        assert_eq!(h.name.as_deref(), Some("N"));
        assert_eq!(h.data_url.as_deref(), Some("d.json"));
        assert!(h.symbol.is_none());
    }

    #[test]
    fn header_overrides_name_and_symbol_over_derived_defaults() {
        let header = TableHeader { name: Some("My Table".into()), symbol: Some("▼".into()), data_url: None, level_order: None };
        let t = DifficultyTable::from_body_bytes(br#"[{"md5":"a","level":"1"}]"#, Some(header)).unwrap();
        assert_eq!(t.name, "My Table");
        assert_eq!(t.symbol, "▼");
    }

    // ----- join_url edge cases -------------------------------------------

    #[test]
    fn join_url_unparseable_base_passes_through_rel() {
        // base is not a valid absolute URL -> reqwest::Url::parse fails -> rel returned raw.
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
}
