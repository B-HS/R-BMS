use serde::{Deserialize, Serialize};

/// One chart in a BMS difficulty-table body (`data.json`). Charts are matched to a local
/// library by `md5` (lowercase hex). `level` is the table's own level string (e.g. "16").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntry {
    pub md5: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub url_diff: String,
}

/// A BMS difficulty-table header (`header.json`): display metadata plus the location of the
/// body. `level_order` fixes how levels are ordered in the UI; absent fields fall back to
/// defaults derived from the body.
#[derive(Debug, Clone, Deserialize)]
pub struct TableHeader {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub data_url: Option<String>,
    #[serde(default)]
    pub level_order: Option<Vec<String>>,
}
