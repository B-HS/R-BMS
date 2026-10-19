//! Finding the charts of a library folder.

use std::path::{Path, PathBuf};

use super::Failure;

/// The chart files under a folder and the folders that could not be listed.
pub struct Walked {
    /// Every chart file, in path order so that the first of two files with one md5 is the same on
    /// every run.
    pub charts: Vec<PathBuf>,
    pub failures: Vec<Failure>,
}

/// Walks `root` depth-first and collects the files [`rbms_library::is_chart`] reads. Symbolic links
/// are not followed, as the library scan does not follow them, so a link pointing back up its own
/// tree cannot make the walk run forever.
pub fn charts_under(root: &Path) -> Walked {
    let mut charts = Vec::new();
    let mut failures = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                failures.push(Failure { path: dir, reason: format!("folder not listed: {e}") });
                continue;
            }
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                stack.push(path);
            } else if rbms_library::is_chart(&path) {
                charts.push(path);
            }
        }
    }
    charts.sort();
    failures.sort_by(|a, b| a.path.cmp(&b.path));
    Walked { charts, failures }
}
