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
}
