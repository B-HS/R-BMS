//! The local song library: walking the configured folders, the per-chart summary the select
//! screen browses, and the heavier per-chart detail it computes for the focused row only.
//!
//! Scanning is a headless, cancellable operation over the filesystem — no app state, no rendering —
//! so the same code backs the player's background scan and the `rbms-cli scan` smoke check.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rbms_chart::to_model;
use rbms_model::Mode;

#[cfg(test)]
mod tests;

/// One chart in the library, as read from its header block alone (cheap enough to do for every
/// file in every folder). Anything needing full timing integration lives in [`ChartDetail`].
#[derive(Clone, Debug)]
pub struct SongEntry {
    pub path: PathBuf,
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub genre: String,
    pub maker: String,
    pub level: String,
    pub difficulty: i32,
    pub init_bpm: f64,
    pub rank: i32,
    pub total: f64,
    pub mode: Mode,
    pub md5: String,
    pub stagefile: String,
    pub banner: String,
    /// `#PREVIEW` audio path, played on a settled focus by the select screen's preview engine.
    pub preview: String,
}

/// Per-chart details that need full timing integration (`to_model`), computed lazily for the focused
/// song only (not every chart in the library) and cached by the select screen.
#[derive(Clone, Debug)]
pub struct ChartDetail {
    pub notes: usize,
    pub long_notes: usize,
    pub duration_us: i64,
    pub bpm_min: f64,
    pub bpm_max: f64,
    pub density: Vec<u32>,
    pub peak_density: f64,
    pub avg_density: f64,
    pub end_density: f64,
}

/// Whether a path carries one of the BMS chart extensions the scanner reads.
pub fn is_chart(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("bms" | "bme" | "bml" | "pms"))
}

/// Scan every configured library folder and merge the results into one song list (the library is
/// the union of all folders). Missing/unreadable folders contribute nothing. `count` is bumped per
/// chart read so a progress screen can follow along; setting `cancel` stops the walk early.
pub fn scan_folders(folders: &[String], count: &AtomicUsize, cancel: &AtomicBool) -> Vec<SongEntry> {
    let mut out = Vec::new();
    for f in folders {
        out.extend(scan_folder(Path::new(f), count, cancel));
    }
    out
}

/// Scan one folder tree, returning its charts sorted by lowercased title.
pub fn scan_folder(root: &Path, count: &AtomicUsize, cancel: &AtomicBool) -> Vec<SongEntry> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_chart(&p)
                && let Ok(bytes) = std::fs::read(&p)
            {
                let src = rbms_parser::parse(&bytes);
                let mode = rbms_chart::detect_mode(&src, p.to_str().unwrap_or(""));
                let h = &src.headers;
                let title = if h.title.is_empty() { p.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string() } else { h.title.clone() };
                out.push(SongEntry {
                    path: p,
                    title,
                    subtitle: h.subtitle.clone(),
                    artist: h.artist.clone(),
                    genre: h.genre.clone(),
                    maker: h.maker.clone(),
                    level: h.play_level.clone(),
                    difficulty: h.difficulty,
                    init_bpm: h.init_bpm,
                    rank: h.rank,
                    total: h.total.unwrap_or(0.0),
                    mode,
                    md5: src.md5.clone(),
                    stagefile: h.stagefile.clone(),
                    banner: h.banner.clone(),
                    preview: h.preview.clone(),
                });
                count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    out.sort_by_key(|s| s.title.to_lowercase());
    out
}

/// Parse + time-integrate a single chart to derive its playable-note count, long-note count, length
/// and BPM range for the select detail panel. `None` if the file can't be read.
pub fn compute_chart_detail(path: &Path, mode: Mode) -> Option<ChartDetail> {
    let bytes = std::fs::read(path).ok()?;
    let src = rbms_parser::parse(&bytes);
    let total_value = src.headers.total.unwrap_or(0.0);
    let model = to_model(&src, mode);
    let long_notes =
        model.timelines.iter().flat_map(|tl| tl.notes.iter().flatten()).filter(|n| matches!(n.kind, rbms_model::NoteKind::LongStart { .. })).count();
    let duration_us = model.timelines.last().map(|t| t.time_us).unwrap_or(0);
    let mut bpm_min = f64::MAX;
    let mut bpm_max = f64::MIN;
    for tl in &model.timelines {
        if tl.bpm > 0.0 {
            bpm_min = bpm_min.min(tl.bpm);
            bpm_max = bpm_max.max(tl.bpm);
        }
    }
    if bpm_min > bpm_max {
        bpm_min = model.init_bpm;
        bpm_max = model.init_bpm;
    }
    let dens = rbms_chart::note_density(&model, total_value);
    Some(ChartDetail {
        notes: rbms_chart::count_playable_notes(&model),
        long_notes,
        duration_us,
        bpm_min,
        bpm_max,
        density: dens.bins,
        peak_density: dens.peak,
        avg_density: dens.avg,
        end_density: dens.end,
    })
}

/// A scanned song list plus the md5 index every chart-keyed lookup (scores, difficulty tables,
/// IR rankings) goes through, so those stay hash lookups instead of scans of the whole library.
#[derive(Clone, Debug, Default)]
pub struct Library {
    songs: Vec<SongEntry>,
    by_md5: HashMap<String, Vec<usize>>,
}

impl Library {
    /// Index a scanned song list. Entry order is preserved, so existing indices stay valid.
    pub fn from_songs(songs: Vec<SongEntry>) -> Library {
        let mut by_md5: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, s) in songs.iter().enumerate() {
            by_md5.entry(s.md5.to_ascii_lowercase()).or_default().push(i);
        }
        Library { songs, by_md5 }
    }

    pub fn songs(&self) -> &[SongEntry] {
        &self.songs
    }

    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.songs.len()
    }

    /// Positions in [`Library::songs`] of every chart with this md5 (case-insensitive).
    pub fn indices_for_md5(&self, md5: &str) -> &[usize] {
        self.by_md5.get(&md5.to_ascii_lowercase()).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The library's md5s in entry order — what `DifficultyTable::match_levels` matches against.
    pub fn md5s(&self) -> impl Iterator<Item = &str> {
        self.songs.iter().map(|s| s.md5.as_str())
    }
}
