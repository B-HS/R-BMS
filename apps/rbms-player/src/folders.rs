use std::path::Path;

use serde::{Deserialize, Serialize};

/// The persisted list of song-library folders. Every folder is scanned and the results merged into
/// one song list, so the library can span multiple directories. Stored in `folders.ron`.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FolderList {
    pub folders: Vec<String>,
}

impl FolderList {
    pub fn load(path: &Path) -> FolderList {
        match std::fs::read_to_string(path) {
            Ok(s) => ron::from_str(&s).unwrap_or_else(|e| {
                // Back up a corrupt file before falling back to defaults, so the next save() does not
                // silently overwrite a recoverable list (matches keyconfig/settings/scores).
                let backup = path.with_extension("ron.bak");
                let _ = std::fs::rename(path, &backup);
                eprintln!("folders parse failed ({e}); backed up to {} and using empty list", backup.display());
                FolderList::default()
            }),
            Err(_) => FolderList::default(),
        }
    }

    pub fn save(&self, path: &Path) {
        match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(s) => {
                if let Err(e) = crate::write_atomic(path, &s) {
                    eprintln!("folders write failed ({}): {e}", path.display());
                }
            }
            Err(e) => eprintln!("folders save failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        assert!(FolderList::default().folders.is_empty());
    }

    #[test]
    fn ron_round_trip_preserves_folders_and_order() {
        let l = FolderList { folders: vec!["/a".into(), "/b/c".into(), "D:\\songs".into()] };
        let s = ron::ser::to_string_pretty(&l, ron::ser::PrettyConfig::default()).unwrap();
        let back: FolderList = ron::from_str(&s).unwrap();
        assert_eq!(back.folders, vec!["/a".to_string(), "/b/c".into(), "D:\\songs".into()]);
    }

    #[test]
    fn empty_unit_ron_parses_to_empty_list() {
        let l: FolderList = ron::from_str("()").expect("serde(default) empty unit parses");
        assert!(l.folders.is_empty());
    }

    #[test]
    fn load_missing_file_returns_empty_list() {
        let path = std::env::temp_dir().join(format!("rbms_folders_missing_{}.ron", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let l = FolderList::load(&path);
        assert!(l.folders.is_empty());
        assert!(!path.exists(), "load does not create the file");
    }

    #[test]
    fn load_malformed_file_returns_empty_list() {
        let dir = std::env::temp_dir().join(format!("rbms_folders_bad_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("folders.ron");
        std::fs::write(&path, "@@@ not ron @@@").unwrap();
        let l = FolderList::load(&path);
        assert!(l.folders.is_empty(), "malformed => empty list");
        // The corrupt file is backed up to .ron.bak (and moved aside) before defaults are used.
        assert!(path.with_extension("ron.bak").exists(), "malformed folders file backed up to .bak");
        assert!(!path.exists(), "the corrupt file is renamed away");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_then_load_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!("rbms_folders_io_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested/folders.ron");
        FolderList { folders: vec!["/songs".into()] }.save(&path);
        assert!(path.exists());
        let back = FolderList::load(&path);
        assert_eq!(back.folders, vec!["/songs".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
