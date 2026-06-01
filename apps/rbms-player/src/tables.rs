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
                // Back up a corrupt file before falling back to defaults, so the next save() does not
                // silently overwrite a recoverable list (matches keyconfig/settings/scores).
                let backup = path.with_extension("ron.bak");
                let _ = std::fs::rename(path, &backup);
                eprintln!("tables parse failed ({e}); backed up to {} and using empty list", backup.display());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> TableList {
        TableList {
            tables: vec![
                TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into() },
                TableSource { name: "".into(), location: "/local/table.json".into() },
            ],
        }
    }

    #[test]
    fn default_is_empty() {
        assert!(TableList::default().tables.is_empty());
    }

    #[test]
    fn ron_round_trip_preserves_sources_and_order() {
        let l = list();
        let s = ron::ser::to_string_pretty(&l, ron::ser::PrettyConfig::default()).unwrap();
        let back: TableList = ron::from_str(&s).unwrap();
        assert_eq!(back.tables.len(), 2, "count preserved");
        assert_eq!(back.tables[0].name, "Insane");
        assert_eq!(back.tables[0].location, "https://example.com/insane.json");
        assert_eq!(back.tables[1].name, "", "empty name preserved");
        assert_eq!(back.tables[1].location, "/local/table.json");
    }

    #[test]
    fn empty_unit_ron_parses_to_empty_list() {
        let l: TableList = ron::from_str("()").expect("serde(default) empty unit parses");
        assert!(l.tables.is_empty());
    }

    #[test]
    fn load_missing_file_returns_empty_list() {
        let path = std::env::temp_dir().join(format!("rbms_tables_missing_{}.ron", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let l = TableList::load(&path);
        assert!(l.tables.is_empty(), "missing file => empty list");
        assert!(!path.exists(), "load does not create the file");
    }

    #[test]
    fn load_malformed_file_returns_empty_list() {
        let dir = std::env::temp_dir().join(format!("rbms_tables_bad_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tables.ron");
        std::fs::write(&path, "not ron at all").unwrap();
        let l = TableList::load(&path);
        assert!(l.tables.is_empty(), "malformed => empty list");
        // A corrupt file is backed up to .ron.bak (and moved aside) before defaults are used, so the
        // next save() can't silently clobber a recoverable file — matching settings/scores/keyconfig.
        assert!(path.with_extension("ron.bak").exists(), "malformed tables file backed up to .bak");
        assert!(!path.exists(), "the corrupt file is renamed away");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_then_load_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!("rbms_tables_io_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested/tables.ron");
        list().save(&path);
        assert!(path.exists());
        let back = TableList::load(&path);
        assert_eq!(back.tables.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
