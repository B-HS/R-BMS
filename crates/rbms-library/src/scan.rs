//! Incremental library scanning: enumerate, diff against the stored stamps, parse in parallel and
//! commit in batches, cancellable at every step.
//!
//! A scan is three passes over one folder tree. The walk is single-threaded and does no parsing, so
//! it reaches the end of a large library quickly and hands on nothing but paths and file stamps.
//! The diff drops every chart whose modification time and size still match the stored row, which is
//! what turns the second run of a scan into a few file stats. What is left is parsed by a pool of
//! workers and committed in batches, so a browser reading the database sees the charts of batch one
//! while batch two is still being parsed.
//!
//! Cancelling is cooperative and checked in all three passes. A cancelled scan keeps the batches it
//! already committed and skips the deletion pass entirely: rows for charts the walk never reached
//! are not evidence that the files are gone.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;

use rbms_chart::{count_playable_notes, detect_mode, note_density, to_model};
use rbms_model::{Model, NoteKind};

use crate::songdb::rows::{
    CONTENT_BGA, CONTENT_NO_KEYSOUND, CONTENT_PREVIEW, CONTENT_TEXT, FEATURE_CHARGE_NOTE, FEATURE_HELL_CHARGE_NOTE, FEATURE_LONG_NOTE, FEATURE_MINE_NOTE,
    FEATURE_RANDOM, FEATURE_SCROLL, FEATURE_STOP_SEQUENCE, FEATURE_UNDEFINED_LN, NO_KEYSOUND_MIN_LENGTH_MS, NO_KEYSOUND_MS_PER_SAMPLE,
    NO_KEYSOUND_SAMPLE_SLACK, mode_id,
};
use crate::songdb::{DetailRow, PARSER_VERSION, SongDb, SongDbError, SongRow};

#[cfg(test)]
mod tests;

/// Charts committed in one transaction. Small enough that a browser opened mid-scan fills in
/// steadily, large enough that a full library is not thousands of transactions.
const COMMIT_BATCH: usize = 512;

/// Ceiling on parse workers, matching the keysound decode pool. Beyond this the scan is bound by
/// the disk rather than by the CPU.
const MAX_WORKERS: usize = 8;

/// Scroll speed a timeline has when the chart never changed it.
const DEFAULT_SCROLL: f64 = 1.0;

/// Microseconds in the millisecond the `length` column is stored in.
const US_PER_MS: i64 = 1_000;

/// Chart control-flow commands, whose presence is what the reference implementation's `FEATURE_RANDOM`
/// bit records.
const CONTROL_FLOW_COMMANDS: [&[u8]; 4] = [b"#RANDOM", b"#SETRANDOM", b"#SWITCH", b"#SETSWITCH"];

/// Extension of the text file whose presence next to a chart sets `CONTENT_TEXT`.
const TEXT_EXTENSION: &str = "txt";

/// How far a scan has got. The screen driving it reads these while the scan runs.
#[derive(Debug, Default)]
pub struct ScanProgress {
    /// Chart files the walk has reached.
    pub found: AtomicUsize,
    /// Charts actually read and parsed.
    pub parsed: AtomicUsize,
    /// Charts whose stored row still matched the file, so they were not read at all.
    pub skipped: AtomicUsize,
    /// Rows dropped because the file behind them is gone.
    pub removed: AtomicUsize,
}

impl ScanProgress {
    /// A consistent-enough snapshot for one frame of a progress screen.
    pub fn counts(&self) -> ScanCounts {
        ScanCounts {
            found: self.found.load(Ordering::Relaxed),
            parsed: self.parsed.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed),
            removed: self.removed.load(Ordering::Relaxed),
        }
    }
}

/// One read of [`ScanProgress`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanCounts {
    pub found: usize,
    pub parsed: usize,
    pub skipped: usize,
    pub removed: usize,
}

/// What to scan and how.
#[derive(Clone, Debug)]
pub struct ScanRequest {
    /// Library roots to walk, as configured.
    pub roots: Vec<String>,
    /// Re-parse every chart even when its file has not moved, which is what a parser change needs.
    pub full: bool,
    /// Set to stop the scan at the next check.
    pub cancel: Arc<AtomicBool>,
    /// Filled in as the scan runs.
    pub progress: Arc<ScanProgress>,
}

impl ScanRequest {
    /// A request over `roots` with its own cancel flag and progress counters.
    pub fn new(roots: Vec<String>, full: bool) -> ScanRequest {
        ScanRequest { roots, full, cancel: Arc::new(AtomicBool::new(false)), progress: Arc::new(ScanProgress::default()) }
    }
}

/// How a scan ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanOutcomeG {
    /// The whole tree was walked, parsed and reconciled.
    Completed { upserted: usize, removed: usize },
    /// The cancel flag went up. Whatever was committed before it stays; nothing was deleted.
    Cancelled,
}

/// Walk `req.roots`, refresh what changed and drop what is gone.
///
/// The parse generation is stamped only on a completed scan, so an interrupted upgrade rescan is
/// resumed rather than treated as done.
pub fn scan_into(db: &mut SongDb, req: &ScanRequest) -> Result<ScanOutcomeG, SongDbError> {
    let roots: Vec<String> = req.roots.iter().map(|root| normalize(Path::new(root))).collect();
    let found = enumerate(&req.roots, &req.progress, &req.cancel);
    if req.cancel.load(Ordering::Relaxed) {
        return Ok(ScanOutcomeG::Cancelled);
    }

    let stamps = db.stamp_index()?;
    let mut alive: HashSet<String> = HashSet::with_capacity(found.len());
    let mut jobs: Vec<Found> = Vec::new();
    for chart in found {
        alive.insert(chart.path.clone());
        if !req.full && stamps.get(&chart.path) == Some(&(chart.mtime, chart.size)) {
            req.progress.skipped.fetch_add(1, Ordering::Relaxed);
        } else {
            jobs.push(chart);
        }
    }

    let (upserted, stopped) = parse_and_commit(db, req, jobs)?;
    if stopped || req.cancel.load(Ordering::Relaxed) {
        return Ok(ScanOutcomeG::Cancelled);
    }

    let removed = db.delete_missing(&alive, &roots)?;
    req.progress.removed.store(removed, Ordering::Relaxed);
    db.set_parser_version(PARSER_VERSION)?;
    Ok(ScanOutcomeG::Completed { upserted, removed })
}

/// One chart file the walk reached, with everything the diff and the parse need about it.
#[derive(Clone, Debug)]
struct Found {
    source: PathBuf,
    path: String,
    folder: String,
    mtime: i64,
    size: i64,
    has_text: bool,
}

/// Walk every root depth-first, collecting chart files and their stamps. Symbolic links are not
/// followed, so a link pointing back up its own tree cannot make the walk run forever.
fn enumerate(roots: &[String], progress: &ScanProgress, cancel: &AtomicBool) -> Vec<Found> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = roots.iter().map(PathBuf::from).collect();
    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return out;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let folder = normalize(&dir);
        let mut charts: Vec<Found> = Vec::new();
        let mut has_text = false;
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
                continue;
            }
            if is_text(&path) {
                has_text = true;
                continue;
            }
            if !crate::is_chart(&path) {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            charts.push(Found {
                path: normalize(&path),
                source: path,
                folder: folder.clone(),
                mtime: modified_secs(&meta),
                size: meta.len() as i64,
                has_text: false,
            });
        }
        progress.found.fetch_add(charts.len(), Ordering::Relaxed);
        for mut chart in charts {
            chart.has_text = has_text;
            out.push(chart);
        }
    }
    out
}

/// Parse `jobs` across a worker pool and commit them in batches, returning how many rows were
/// written and whether the run stopped early.
fn parse_and_commit(db: &mut SongDb, req: &ScanRequest, jobs: Vec<Found>) -> Result<(usize, bool), SongDbError> {
    if jobs.is_empty() {
        return Ok((0, false));
    }
    let jobs = Arc::new(jobs);
    let cursor = Arc::new(AtomicUsize::new(0));
    let halt = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();
    for _ in 0..worker_count(jobs.len()) {
        let jobs = Arc::clone(&jobs);
        let cursor = Arc::clone(&cursor);
        let cancel = Arc::clone(&req.cancel);
        let halt = Arc::clone(&halt);
        let progress = Arc::clone(&req.progress);
        let tx = tx.clone();
        handles.push(std::thread::spawn(move || {
            while !cancel.load(Ordering::Relaxed) && !halt.load(Ordering::Relaxed) {
                let index = cursor.fetch_add(1, Ordering::Relaxed);
                let Some(job) = jobs.get(index) else {
                    break;
                };
                let Some(parsed) = parse_chart(job) else {
                    continue;
                };
                progress.parsed.fetch_add(1, Ordering::Relaxed);
                if tx.send(parsed).is_err() {
                    break;
                }
            }
        }));
    }
    drop(tx);

    let mut batch: Vec<(SongRow, Option<DetailRow>)> = Vec::with_capacity(COMMIT_BATCH);
    let mut upserted = 0usize;
    let mut failure: Option<SongDbError> = None;
    let mut stopped = false;
    while let Ok(parsed) = rx.recv() {
        batch.push(parsed);
        if batch.len() < COMMIT_BATCH {
            continue;
        }
        match db.upsert_batch_with_details(&batch) {
            Ok(()) => upserted += batch.len(),
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
        batch.clear();
        if req.cancel.load(Ordering::Relaxed) {
            stopped = true;
            break;
        }
    }
    if failure.is_none() && !stopped && !batch.is_empty() {
        match db.upsert_batch_with_details(&batch) {
            Ok(()) => upserted += batch.len(),
            Err(e) => failure = Some(e),
        }
    }

    halt.store(true, Ordering::Relaxed);
    for handle in handles {
        let _ = handle.join();
    }
    match failure {
        Some(e) => Err(e),
        None => Ok((upserted, stopped)),
    }
}

/// Workers to parse with: one per core, capped, and never more than there is work for.
fn worker_count(jobs: usize) -> usize {
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    cores.clamp(1, MAX_WORKERS).min(jobs.max(1))
}

/// Read and parse one chart into the row and the detail the database stores for it. A file that
/// cannot be read or is not a chart at all contributes nothing, exactly as the in-memory scan did.
fn parse_chart(job: &Found) -> Option<(SongRow, Option<DetailRow>)> {
    let bytes = std::fs::read(&job.source).ok()?;
    if crate::is_bmson(&job.source) {
        return parse_bmson_chart(job, &bytes);
    }
    let source = rbms_parser::parse(&bytes);
    let mode = detect_mode(&source, job.source.to_str().unwrap_or(""));
    let model = to_model(&source, mode);
    let headers = &source.headers;

    let duration_us = model.timelines.last().map(|tl| tl.time_us).unwrap_or(0);
    let length_ms = duration_us / US_PER_MS;
    let (bpm_min, bpm_max) = bpm_range(&model);
    let density = note_density(&model, headers.total.unwrap_or(0.0));
    let title = if headers.title.is_empty() { job.source.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string() } else { headers.title.clone() };

    let row = SongRow {
        path: job.path.clone(),
        md5: source.md5.clone(),
        sha256: source.sha256.clone(),
        title,
        subtitle: headers.subtitle.clone(),
        artist: headers.artist.clone(),
        subartist: headers.subartist.clone(),
        genre: headers.genre.clone(),
        maker: headers.maker.clone(),
        level: headers.play_level.clone(),
        difficulty: headers.difficulty,
        mode: mode_id(mode),
        judge: headers.rank,
        total: headers.total.unwrap_or(0.0),
        init_bpm: headers.init_bpm,
        min_bpm: bpm_min as i32,
        max_bpm: bpm_max as i32,
        length_ms,
        notes: count_playable_notes(&model) as i32,
        long_notes: long_note_count(&model) as i32,
        stagefile: headers.stagefile.clone(),
        banner: headers.banner.clone(),
        backbmp: String::new(),
        preview: headers.preview.clone(),
        folder: job.folder.clone(),
        favorite: 0,
        date: job.mtime,
        adddate: crate::songdb::now_secs(),
        size: job.size,
        feature: feature_bits(&model, uses_control_flow(&bytes)),
        content: content_bits(&model, job.has_text, !headers.preview.is_empty(), length_ms),
    };
    let detail = DetailRow { duration_us, peak_density: density.peak, avg_density: density.avg, end_density: density.end, density: density.bins };
    Some((row, Some(detail)))
}

/// Read one bmson chart into the row and the detail the database stores for it. bmson states its
/// own mode instead of leaving it to be detected, and its TOTAL is already absolute on the model.
fn parse_bmson_chart(job: &Found, bytes: &[u8]) -> Option<(SongRow, Option<DetailRow>)> {
    let chart = rbms_parser::bmson::parse(bytes).ok()?;
    let mode = chart.mode();
    let model = chart.to_model();

    let duration_us = model.timelines.last().map(|tl| tl.time_us).unwrap_or(0);
    let length_ms = duration_us / US_PER_MS;
    let (bpm_min, bpm_max) = bpm_range(&model);
    let density = note_density(&model, model.meta.total);
    let title = if model.meta.title.is_empty() {
        job.source.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string()
    } else {
        model.meta.title.clone()
    };

    let row = SongRow {
        path: job.path.clone(),
        md5: model.md5.clone(),
        sha256: model.sha256.clone(),
        title,
        subtitle: model.meta.subtitle.clone(),
        artist: model.meta.artist.clone(),
        subartist: model.meta.subartist.clone(),
        genre: model.meta.genre.clone(),
        maker: String::new(),
        level: model.meta.play_level.clone(),
        difficulty: model.meta.difficulty,
        mode: mode_id(mode),
        judge: model.meta.rank,
        total: model.meta.total,
        init_bpm: model.init_bpm,
        min_bpm: bpm_min as i32,
        max_bpm: bpm_max as i32,
        length_ms,
        notes: count_playable_notes(&model) as i32,
        long_notes: long_note_count(&model) as i32,
        stagefile: chart.info.eyecatch_image.clone(),
        banner: chart.info.banner_image.clone(),
        backbmp: chart.info.back_image.clone(),
        preview: chart.info.preview_music.clone(),
        folder: job.folder.clone(),
        favorite: 0,
        date: job.mtime,
        adddate: crate::songdb::now_secs(),
        size: job.size,
        feature: feature_bits(&model, false),
        content: content_bits(&model, job.has_text, !chart.info.preview_music.is_empty(), length_ms),
    };
    let detail = DetailRow { duration_us, peak_density: density.peak, avg_density: density.avg, end_density: density.end, density: density.bins };
    Some((row, Some(detail)))
}

/// The chart's BPM range, falling back to its initial BPM when no timeline states one — the same
/// derivation the focused-song detail panel does.
fn bpm_range(model: &Model) -> (f64, f64) {
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    for tl in &model.timelines {
        if tl.bpm > 0.0 {
            min = min.min(tl.bpm);
            max = max.max(tl.bpm);
        }
    }
    if min > max { (model.init_bpm, model.init_bpm) } else { (min, max) }
}

/// Long notes in the chart, counted by their heads.
fn long_note_count(model: &Model) -> usize {
    model.timelines.iter().flat_map(|tl| tl.notes.iter().flatten()).filter(|n| matches!(n.kind, NoteKind::LongStart { .. })).count()
}

/// The `feature` bits of a chart, following the reference implementation's `SongData` constructor:
/// what the timelines contain, plus whether the file is built with control flow.
fn feature_bits(model: &Model, control_flow: bool) -> i32 {
    let mut bits = if control_flow { FEATURE_RANDOM } else { 0 };
    for tl in &model.timelines {
        if tl.stop_us > 0 {
            bits |= FEATURE_STOP_SEQUENCE;
        }
        if tl.scroll != DEFAULT_SCROLL {
            bits |= FEATURE_SCROLL;
        }
        for note in tl.notes.iter().take(model.mode.key).flatten() {
            bits |= match note.kind {
                NoteKind::LongStart { ln } | NoteKind::LongEnd { ln } => long_note_feature(ln),
                NoteKind::Mine { .. } => FEATURE_MINE_NOTE,
                NoteKind::Normal => 0,
            };
        }
    }
    bits
}

/// The feature bit one long-note flavour sets.
fn long_note_feature(ln: rbms_model::LnKind) -> i32 {
    match ln {
        rbms_model::LnKind::Ln => FEATURE_LONG_NOTE,
        rbms_model::LnKind::Cn => FEATURE_CHARGE_NOTE,
        rbms_model::LnKind::Hcn => FEATURE_HELL_CHARGE_NOTE,
        rbms_model::LnKind::Undefined => FEATURE_UNDEFINED_LN,
    }
}

/// The `content` bits of a chart: what ships with it and whether it plays as one streamed track
/// rather than from keysounds, which is the reference implementation's length-against-sample-count test.
fn content_bits(model: &Model, has_text: bool, has_preview: bool, length_ms: i64) -> i32 {
    let mut bits = 0;
    if has_text {
        bits |= CONTENT_TEXT;
    }
    if model.bgamap.iter().any(|name| !name.is_empty()) {
        bits |= CONTENT_BGA;
    }
    if has_preview {
        bits |= CONTENT_PREVIEW;
    }
    let samples = model.wavmap.iter().filter(|name| !name.is_empty()).count() as i64;
    if length_ms >= NO_KEYSOUND_MIN_LENGTH_MS && samples <= length_ms / NO_KEYSOUND_MS_PER_SAMPLE + NO_KEYSOUND_SAMPLE_SLACK {
        bits |= CONTENT_NO_KEYSOUND;
    }
    bits
}

/// Whether the chart file carries a `#RANDOM`/`#SWITCH` family command. The parser resolves control
/// flow while reading, so the built model no longer says whether there was any; the file does.
fn uses_control_flow(bytes: &[u8]) -> bool {
    bytes.split(|b| *b == b'\n').any(|line| {
        let line = line.trim_ascii_start();
        CONTROL_FLOW_COMMANDS.iter().any(|cmd| line.len() >= cmd.len() && line[..cmd.len()].eq_ignore_ascii_case(cmd))
    })
}

/// Whether this is the text file whose presence next to a chart sets `CONTENT_TEXT`.
fn is_text(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case(TEXT_EXTENSION)).unwrap_or(false)
}

/// A file's modification time in whole seconds, `0` for a filesystem that does not keep one.
fn modified_secs(meta: &std::fs::Metadata) -> i64 {
    meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// The stored form of a path: absolute, with `/` separators, so the same chart is one row whatever
/// the working directory was and whichever platform wrote it.
///
/// Case is left alone. On a case-insensitive filesystem two spellings of one path are therefore two
/// rows; folding case here would instead merge two genuinely different charts on the case-sensitive
/// filesystems where that is legal.
pub fn normalize(path: &Path) -> String {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    absolute.to_string_lossy().replace('\\', "/")
}
