//! What the person who uploads a pack should look at before they do: files that speak of
//! redistribution, names that may not survive a change of platform, and charts that name files
//! outside their folder. None of it stops a pack from being written.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::folders::{PackEntry, SongFolder};

/// Words, in lower case, that a text file says when it speaks of passing the files on: "redistribution" and "transfer" in Japanese.
const NOTICE_KEYWORDS: [&str; 4] = ["再配布", "二次配布", "転載", "redistribut"];

/// The most bytes of a text file that are looked through.
const MAX_NOTICE_SCAN_BYTES: u64 = 1024 * 1024;

const TEXT_EXTENSION: &str = "txt";

/// What a pack is worth a second look for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notices {
    /// Every text file of the pack, by its name in the archive.
    pub text_files: Vec<String>,
    /// The text files that speak of redistribution.
    pub notice_files: Vec<String>,
    /// Files whose names have a combining mark in them, as a name made on macOS may and a chart may not name the same way on another platform.
    pub nfd_names: usize,
    /// The files that charts of the pack name from outside the pack: a path that climbs out of the folder, or an absolute one.
    pub escapes_root: usize,
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{3099}' | '\u{309A}' | '\u{0300}'..='\u{036F}')
}

fn has_extension(name: &str, wanted: &str) -> bool {
    Path::new(name).extension().and_then(|extension| extension.to_str()).is_some_and(|extension| extension.eq_ignore_ascii_case(wanted))
}

/// The text of a file's bytes: UTF-8 when they are, otherwise the Shift_JIS that Japanese authors
/// write in. A file cut short in the middle of a character is still UTF-8.
fn text_of(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(e) if e.error_len().is_none() => String::from_utf8_lossy(&bytes[..e.valid_up_to()]).into_owned(),
        Err(_) => encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes).0.into_owned(),
    }
}

fn mentions_redistribution(path: &Path) -> bool {
    let mut bytes = Vec::new();
    if File::open(path).and_then(|file| file.take(MAX_NOTICE_SCAN_BYTES).read_to_end(&mut bytes)).is_err() {
        return false;
    }
    let text = text_of(&bytes).to_lowercase();
    NOTICE_KEYWORDS.iter().any(|keyword| text.contains(keyword))
}

fn has_drive_prefix(path: &str) -> bool {
    matches!(path.as_bytes(), [letter, b':', ..] if letter.is_ascii_alphabetic())
}

/// Whether a path a chart gives for a file leaves the folder the chart is in: it is absolute, or it
/// climbs above the folder at some point.
fn leaves_the_folder(reference: &str) -> bool {
    let unified = reference.trim().replace('\\', "/");
    if unified.starts_with('/') || has_drive_prefix(&unified) {
        return true;
    }
    let mut depth = 0usize;
    for part in unified.split('/') {
        match part {
            "" | "." => {}
            ".." if depth == 0 => return true,
            ".." => depth -= 1,
            _ => depth += 1,
        }
    }
    false
}

/// The files the BMS charts of `folder` name that are outside it, each counted once.
fn files_outside_the_folder(folder: &SongFolder) -> usize {
    let mut outside = BTreeSet::new();
    for chart in folder.charts.iter().filter(|chart| !rbms_library::is_bmson(&chart.path)) {
        let Ok(bytes) = std::fs::read(&chart.path) else {
            continue;
        };
        let source = rbms_parser::parse(&bytes);
        outside.extend(source.wav.values().chain(source.bmp.values()).filter(|reference| leaves_the_folder(reference)).cloned());
    }
    outside.len()
}

/// Looks through a song folder and what it packs. The charts are read again here, and the text
/// files up to a size, but no other file is opened.
pub fn scan_notices(folder: &SongFolder, entries: &[PackEntry]) -> Notices {
    let below_top = |entry: &PackEntry| entry.name.get(folder.top.len() + 1..).unwrap_or(&entry.name).to_string();
    let texts: Vec<&PackEntry> = entries.iter().filter(|entry| has_extension(&entry.name, TEXT_EXTENSION)).collect();
    Notices {
        text_files: texts.iter().map(|entry| entry.name.clone()).collect(),
        notice_files: texts.iter().filter(|entry| mentions_redistribution(&entry.source)).map(|entry| entry.name.clone()).collect(),
        nfd_names: entries.iter().filter(|entry| below_top(entry).chars().any(is_combining_mark)).count(),
        escapes_root: files_outside_the_folder(folder),
    }
}
