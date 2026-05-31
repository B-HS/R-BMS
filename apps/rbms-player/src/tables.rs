use std::path::Path;

use serde::{Deserialize, Serialize};

/// A user-added difficulty table: a display name and a `location` (an http(s) URL to a
/// header/data json, or a local file path). Persisted in `tables.ron`.
#[derive(Clone, Serialize, Deserialize)]
pub struct TableSource {
    pub name: String,
    pub location: String,
}

/// The persisted list of difficulty-table sources.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TableList {
    pub tables: Vec<TableSource>,
}

impl TableList {
    pub fn load(path: &Path) -> TableList {
        match std::fs::read_to_string(path) {
            Ok(s) => ron::from_str(&s).unwrap_or_else(|e| {
                eprintln!("tables parse failed ({e}); using empty list");
                TableList::default()
            }),
            Err(_) => TableList::default(),
        }
    }

    pub fn save(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                eprintln!("tables dir create failed ({}): {e}", dir.display());
            }
        }
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => {
                if let Err(e) = std::fs::write(path, &s) {
                    eprintln!("tables write failed ({}): {e}", path.display());
                }
            }
            Err(e) => eprintln!("tables save failed: {e}"),
        }
    }
}
