//! Telling whether running a skin left its folder as it found it.
//!
//! A skin never writes into its own folder: its writes go to the overlay. The dump proves it for the
//! run it just did by listing the folder before and after.
//!
//! A file is told apart by its size and its modification time, so a file rewritten at the same
//! length shows as well as one that grew. The contents themselves are not read: a pack is hundreds
//! of megabytes, and a write that restores both the length and the time is not something a skin
//! has the means to do.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::SystemTime;

use super::report::PackCheckReport;

/// What one file is told apart by between two listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStamp {
    size: u64,
    /// When the file was last written, or `None` on a file system that does not say.
    modified: Option<SystemTime>,
}

/// Every file under a folder, as its path from the folder and what it is told apart by.
pub type Listing = BTreeMap<String, FileStamp>;

/// Lists a folder and everything under it. Symbolic links are listed as themselves and not followed.
pub fn list(root: &Path) -> Listing {
    let mut found = Listing::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else { continue };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else { continue };
            let path = entry.path();
            if metadata.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_string_lossy().replace('\\', "/"), FileStamp { size: metadata.len(), modified: metadata.modified().ok() });
            }
        }
    }
    found
}

/// Compares the folder now with the listing taken before.
pub fn compare(root: &Path, before: &Listing) -> PackCheckReport {
    let after = list(root);
    let mut changed: Vec<String> = after.iter().filter(|(path, stamp)| before.get(*path) != Some(stamp)).map(|(path, _)| path.clone()).collect();
    changed.extend(before.keys().filter(|path| !after.contains_key(*path)).cloned());
    changed.sort();
    PackCheckReport { folder: root.display().to_string(), files_before: before.len(), files_after: after.len(), changed }
}
