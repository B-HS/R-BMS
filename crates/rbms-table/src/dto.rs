use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// One chart in a BMS difficulty-table body (`data.json`). Charts are matched to a local
/// library by `md5` (lowercase hex). `level` is the table's own level string (e.g. "16").
///
/// Tables in the wild are written by hand and by many tools, so every field is read leniently:
/// a level written as a number is the same level as one written as text, and an entry that
/// carries only a `sha256` still parses rather than failing the whole body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntry {
    #[serde(default, deserialize_with = "lenient_text")]
    pub md5: String,
    #[serde(default, deserialize_with = "lenient_text", skip_serializing_if = "String::is_empty")]
    pub sha256: String,
    #[serde(default, deserialize_with = "lenient_text")]
    pub level: String,
    #[serde(default, deserialize_with = "lenient_text")]
    pub title: String,
    #[serde(default, deserialize_with = "lenient_text")]
    pub artist: String,
    #[serde(default, deserialize_with = "lenient_text")]
    pub url: String,
    #[serde(default, deserialize_with = "lenient_text")]
    pub url_diff: String,
}

impl TableEntry {
    /// Whether the entry names a chart at all: one with neither hash cannot be matched to anything.
    pub fn names_a_chart(&self) -> bool {
        !self.md5.trim().is_empty() || !self.sha256.trim().is_empty()
    }
}

/// A BMS difficulty-table header (`header.json`): display metadata plus the location of the
/// body. `level_order` fixes how levels are ordered in the UI; absent fields fall back to
/// defaults derived from the body.
///
/// `data_url` is every body the header names: the format allows one address or a list of them,
/// and a table split over several files is the sum of them.
#[derive(Debug, Clone, Deserialize)]
pub struct TableHeader {
    #[serde(default, deserialize_with = "lenient_optional_text")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "lenient_optional_text")]
    pub symbol: Option<String>,
    #[serde(default, deserialize_with = "lenient_text_list")]
    pub data_url: Vec<String>,
    #[serde(default, deserialize_with = "lenient_optional_text_list")]
    pub level_order: Option<Vec<String>>,
    /// The courses the table lists, as it wrote them. They are kept rather than read: nothing plays
    /// a table's courses yet, and the copy saved of a table has to be the whole of it.
    #[serde(default)]
    pub course: Option<Value>,
}

/// A JSON scalar as the text a table means by it: `12` and `"12"` are the same level.
fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

/// Every scalar of a JSON list as text, with whatever is not a scalar left out.
fn texts_of(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::Array(items) => Some(items.iter().filter_map(text_of).collect()),
        _ => None,
    }
}

fn lenient_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(text_of(&Value::deserialize(deserializer)?).unwrap_or_default())
}

fn lenient_optional_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Ok(text_of(&Value::deserialize(deserializer)?))
}

/// One address or a list of them, as a list.
fn lenient_text_list<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let value = Value::deserialize(deserializer)?;
    Ok(texts_of(&value).unwrap_or_else(|| text_of(&value).into_iter().collect()))
}

fn lenient_optional_text_list<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Vec<String>>, D::Error> {
    Ok(texts_of(&Value::deserialize(deserializer)?))
}
