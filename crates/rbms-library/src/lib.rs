//! The local song library: walking the configured folders, the per-chart summary the select
//! screen browses, and the heavier per-chart detail it computes for the focused row only.
//!
//! Scanning is a headless, cancellable operation over the filesystem — no app state, no rendering —
//! so the same code backs the player's background scan and the `rbms-cli scan` smoke check.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rbms_chart::to_model;
use rbms_model::Mode;

pub mod scan;
pub mod songdb;

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

/// Whether a path carries one of the chart extensions the scanner reads: the BMS family, and the
/// JSON bmson format the parser decodes on its own path.
pub fn is_chart(p: &Path) -> bool {
    matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("bms" | "bme" | "bml" | "pms" | "bmson"))
}

/// Whether a path is a bmson chart, which is decoded from JSON instead of the BMS line grammar.
pub fn is_bmson(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case(rbms_parser::bmson::EXTENSION)).unwrap_or(false)
}

/// Parse + time-integrate a single chart to derive its playable-note count, long-note count, length
/// and BPM range for the select detail panel. `None` if the file can't be read.
pub fn compute_chart_detail(path: &Path, mode: Mode) -> Option<ChartDetail> {
    let bytes = std::fs::read(path).ok()?;
    let (model, total_value) = if is_bmson(path) {
        let chart = rbms_parser::bmson::parse(&bytes).ok()?;
        let model = chart.to_model_in_mode(mode);
        let total = model.meta.total;
        (model, total)
    } else {
        let src = rbms_parser::parse(&bytes);
        (to_model(&src, mode), src.headers.total.unwrap_or(0.0))
    };
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
