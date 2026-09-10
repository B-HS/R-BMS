//! The charts the player has starred, kept next to the scores in the config directory.
//!
//! A favourite is a chart md5, the same key the score book and the difficulty tables are indexed
//! by, so a starred chart is still starred after the library is rescanned or the file is moved.
//! The set is ordered, so the file it is written to reads the same twice running.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::notify::{Level, notify};
use crate::write_atomic;

/// Name of the file the favourites are kept in, next to `scores.ron`.
pub(crate) const FAVORITES_FILE: &str = "favorites.ron";

/// The starred charts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Favorites {
    /// Chart md5s, ordered so the file is stable.
    md5s: BTreeSet<String>,
}

impl Favorites {
    /// Read the favourites at `path`. A file that is not there yet is an empty set — the normal
    /// state of a fresh install — and one that cannot be read is reported and treated the same, so
    /// a corrupt file never stops the browser from opening.
    pub(crate) fn load(path: &Path) -> Favorites {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Favorites::default();
        };
        match ron::from_str::<Favorites>(&text) {
            Ok(favorites) => favorites,
            Err(e) => {
                notify(Level::Warn, format!("favorites not loaded ({e}); starting with none"));
                Favorites::default()
            }
        }
    }

    /// Write the favourites to `path`, reporting a failed write rather than losing it silently.
    pub(crate) fn save(&self, path: &Path) {
        let text = match ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            Ok(text) => text,
            Err(e) => {
                notify(Level::Error, format!("favorites save failed: {e}"));
                return;
            }
        };
        if let Err(e) = write_atomic(path, &text) {
            notify(Level::Error, format!("favorites save failed ({}): {e}", path.display()));
        }
    }

    /// Whether this chart is starred, which is what draws the star on its row and what the
    /// favourites-only filter keeps a chart for.
    pub(crate) fn contains(&self, md5: &str) -> bool {
        self.md5s.contains(md5)
    }

    /// Star or unstar this chart, reporting whether it is starred afterwards. An empty md5 — a
    /// folder row rather than a chart — is left alone.
    pub(crate) fn toggle(&mut self, md5: &str) -> bool {
        if md5.trim().is_empty() {
            return false;
        }
        if self.md5s.remove(md5) {
            return false;
        }
        self.md5s.insert(md5.to_string());
        true
    }

    /// How many charts are starred.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.md5s.len()
    }

    /// Every starred chart's md5, in the order the file holds them.
    pub(crate) fn md5s(&self) -> impl Iterator<Item = &str> {
        self.md5s.iter().map(String::as_str)
    }
}

impl crate::AppShared {
    /// Star or unstar the chart the browser's cursor is on, and write the change out at once so it
    /// survives a crash the way a saved score does. A folder row is not a chart, so it is left
    /// alone.
    pub(crate) fn toggle_focused_favorite(&mut self) {
        let Some(md5) = self.focused_md5() else {
            return;
        };
        self.favorites.toggle(&md5);
        self.favorites.save(&self.favorites_path);
        self.select_gen = self.select_gen.wrapping_add(1);
    }
}

/// Where the favourites live for a given settings file: next to it, like the score book.
pub(crate) fn favorites_path(settings_path: &Path) -> PathBuf {
    settings_path.parent().map(|dir| dir.join(FAVORITES_FILE)).unwrap_or_else(|| PathBuf::from(FAVORITES_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rbms-favorites-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp dir");
        dir
    }

    #[test]
    fn a_fresh_set_has_nothing_starred() {
        let favorites = Favorites::default();
        assert_eq!(favorites.len(), 0);
        assert!(!favorites.contains("abc"));
    }

    #[test]
    fn starring_a_chart_and_starring_it_again_undoes_it() {
        let mut favorites = Favorites::default();
        assert!(favorites.toggle("abc"), "the first press stars it");
        assert!(favorites.contains("abc"));
        assert_eq!(favorites.len(), 1);
        assert!(!favorites.toggle("abc"), "the second press unstars it");
        assert!(!favorites.contains("abc"));
        assert_eq!(favorites.len(), 0);
    }

    /// A folder row has no chart behind it, so pressing the key on one must not star an empty key
    /// that then matches every chart without an md5.
    #[test]
    fn a_row_with_no_chart_behind_it_cannot_be_starred() {
        let mut favorites = Favorites::default();
        assert!(!favorites.toggle(""));
        assert!(!favorites.toggle("   "));
        assert_eq!(favorites.len(), 0);
    }

    #[test]
    fn the_set_survives_a_round_trip_through_its_file() {
        let dir = temp_dir("round-trip");
        let path = dir.join(FAVORITES_FILE);
        let mut favorites = Favorites::default();
        favorites.toggle("bbb");
        favorites.toggle("aaa");
        favorites.save(&path);
        let back = Favorites::load(&path);
        assert_eq!(back, favorites);
        assert!(back.contains("aaa"));
        assert!(back.contains("bbb"));
    }

    #[test]
    fn a_file_that_is_not_there_yet_reads_as_nothing_starred() {
        let dir = temp_dir("missing");
        assert_eq!(Favorites::load(&dir.join(FAVORITES_FILE)), Favorites::default());
    }

    /// A hand-edited or truncated file must not stop the browser from opening.
    #[test]
    fn a_file_that_cannot_be_read_falls_back_to_nothing_starred() {
        let dir = temp_dir("corrupt");
        let path = dir.join(FAVORITES_FILE);
        std::fs::write(&path, b"this is not ron").expect("a file is written");
        assert_eq!(Favorites::load(&path), Favorites::default());
    }

    #[test]
    fn the_file_sits_next_to_the_settings_it_belongs_to() {
        let dir = temp_dir("path");
        assert_eq!(favorites_path(&dir.join("settings.ron")), dir.join(FAVORITES_FILE));
        assert_eq!(favorites_path(Path::new("settings.ron")), PathBuf::from(FAVORITES_FILE), "a bare filename still resolves");
    }
}
