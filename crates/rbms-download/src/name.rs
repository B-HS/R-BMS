use std::fs;
use std::path::{Path, PathBuf};

use crate::error::DownloadError;

const FORBIDDEN_NAME_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const REPLACEMENT_CHAR: char = '_';
const FOLDER_TITLE_MAX_CHARS: usize = 64;
const FOLDER_TITLE_MAX_BYTES: usize = 200;
const FOLDER_ID_CHARS: usize = 8;
const FALLBACK_FOLDER_NAME: &str = "chart";
const SETTLE_MAX_NUMBER: usize = 1000;

fn is_reserved_device_name(stem: &str) -> bool {
    let upper = stem.trim_end().to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper.strip_prefix("COM").or_else(|| upper.strip_prefix("LPT")).is_some_and(|number| matches!(number.as_bytes(), [b'1'..=b'9']))
}

fn has_reserved_stem(name: &str) -> bool {
    is_reserved_device_name(name.split('.').next().unwrap_or_default())
}

/// Whether `part` can be one piece of a path on every platform: not empty, not `.` or `..`, no
/// control character, no character Windows refuses (a drive or stream mark among them), no end in a
/// space or a dot, and not the name of a Windows device.
pub(crate) fn is_safe_component(part: &str) -> bool {
    !part.is_empty() && !part.ends_with([' ', '.']) && !part.chars().any(|c| c.is_control() || FORBIDDEN_NAME_CHARS.contains(&c)) && !has_reserved_stem(part)
}

/// The path an archive entry called `name` is unpacked to, relative to the folder unpacked into;
/// `None` when any piece of it is not [`is_safe_component`]. Both `/` and `\` separate pieces.
pub(crate) fn entry_path(name: &str) -> Option<PathBuf> {
    let mut path = PathBuf::new();
    for part in name.split(['/', '\\']) {
        if !is_safe_component(part) {
            return None;
        }
        path.push(part);
    }
    Some(path)
}

fn replace_forbidden(c: char) -> char {
    if c.is_control() || FORBIDDEN_NAME_CHARS.contains(&c) { REPLACEMENT_CHAR } else { c }
}

fn is_edge_junk(c: char) -> bool {
    c.is_whitespace() || c == '.'
}

fn truncated(title: &str) -> &str {
    let mut end = 0;
    for (index, c) in title.char_indices().take(FOLDER_TITLE_MAX_CHARS) {
        let next = index + c.len_utf8();
        if next > FOLDER_TITLE_MAX_BYTES {
            break;
        }
        end = next;
    }
    &title[..end]
}

/// A folder name made from a chart's title that every platform accepts: the title with what a file
/// name may not hold replaced, cut to a length, followed by the first characters of `md5` so that
/// two charts with one title do not share a folder. A title that leaves nothing, or that names a
/// Windows device, gives the md5 part alone.
pub fn safe_folder_name(title: &str, md5: &str) -> String {
    let id: String = md5.chars().take(FOLDER_ID_CHARS).map(replace_forbidden).collect();
    let cleaned: String = title.chars().map(replace_forbidden).collect();
    let cut = truncated(cleaned.trim_matches(is_edge_junk)).trim_end_matches(is_edge_junk);
    match (cut.is_empty() || has_reserved_stem(cut), id.is_empty()) {
        (true, true) => FALLBACK_FOLDER_NAME.to_string(),
        (true, false) => id,
        (false, true) => cut.to_string(),
        (false, false) => format!("{cut} [{id}]"),
    }
}

/// Move `staging` to a new folder under `parent` named `name`, or `name (2)`, `name (3)` ... when
/// that is taken. Nothing that is there is ever overwritten. Returns the folder it ended up at.
pub fn settle(staging: &Path, parent: &Path, name: &str) -> Result<PathBuf, DownloadError> {
    if !is_safe_component(name) {
        return Err(DownloadError::UnsafeName(name.to_string()));
    }
    fs::create_dir_all(parent)?;
    let is_taken = |path: &Path| path.symlink_metadata().is_ok();
    for number in 1..=SETTLE_MAX_NUMBER {
        let target = parent.join(if number == 1 { name.to_string() } else { format!("{name} ({number})") });
        if is_taken(&target) {
            continue;
        }
        match fs::rename(staging, &target) {
            Ok(()) => return Ok(target),
            Err(_) if is_taken(&target) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(DownloadError::NameTaken { name: name.to_string() })
}
