//! The LOADING screen: the one frame that is shown before a blocking chart load, the background
//! folder rescan, and the keysound decode that fills the audio bank before play begins.
#![allow(clippy::wildcard_imports)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::stage::{Canvas, FrameCtx, KeyInput, PlayState, Stage, StageHandler, Transition};
use crate::*;

/// Width and height of the progress bar under the heading.
const BAR_W: f32 = 360.0;
const BAR_H: f32 = 8.0;

/// Width of the sweeping segment of the indeterminate scan bar.
const SWEEP_W: f32 = 96.0;

/// Frames one full ping-pong of the indeterminate bar takes.
const SWEEP_PERIOD_FRAMES: u64 = 120;
const SWEEP_HALF_FRAMES: f32 = 60.0;

/// Frames between steps of the animated ellipsis after the heading.
const DOTS_PERIOD_FRAMES: u64 = 12;
const DOTS_MAX: u64 = 4;

/// How much of a chart title the LOADING line shows.
const TITLE_CHARS: usize = 30;

/// Result of a background folder scan (run off-thread so the LOADING screen keeps animating instead
/// of freezing): the rescanned library and the per-table matched levels.
pub(crate) struct ScanOutcome {
    pub(crate) library: Library,
    pub(crate) names: Vec<String>,
    pub(crate) levels: Vec<TableLevels>,
}

/// The in-flight keysound decode of a loaded chart: the decoded samples arriving from the worker
/// pool, how many have been reported, the cooperative cancel, and how many there are in total.
pub(crate) struct KeysoundLoad {
    pub(crate) rx: Receiver<(u32, rbms_audio::DecodedAudio)>,
    pub(crate) progress: Arc<AtomicUsize>,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) total: usize,
}

/// What this LOADING frame is waiting to do.
///
/// `Song` loads a chart and enters Play; `Scan` (re)scans the song folders and re-matches the
/// difficulty tables before returning to Select; `Keysounds` fills the audio bank of a chart that
/// has already been parsed. `Song` and `Scan` both block, so the LOADING frame is presented first
/// (folder descent within the already-scanned library is instant and skips this).
pub(crate) enum LoadingTask {
    /// Load the library chart at this index and enter Play.
    Song(usize),
    /// Rescan every library folder and re-match the tables, then Select.
    Scan { rx: Receiver<ScanOutcome>, count: Arc<AtomicUsize> },
    /// Decode the loaded chart's keysounds, then enter Play.
    Keysounds { play: Box<PlayState>, load: KeysoundLoad },
}

/// The LOADING screen's own state: what it is waiting for, and whether it has been on screen for a
/// frame yet (a blocking load only starts once the user can see that something is happening).
pub(crate) struct LoadingState {
    task: LoadingTask,
    drawn: bool,
}

impl LoadingState {
    /// Queue a chart load; the actual (blocking) work happens one frame later, so a LOADING frame
    /// is presented first.
    pub(crate) fn song(index: usize) -> LoadingState {
        LoadingState { task: LoadingTask::Song(index), drawn: false }
    }

    /// Continue a chart whose keysounds are still decoding on the worker pool.
    pub(crate) fn keysounds(play: PlayState, load: KeysoundLoad) -> LoadingState {
        LoadingState { task: LoadingTask::Keysounds { play: Box::new(play), load }, drawn: false }
    }

    /// Rescan every library folder off-thread and merge into one song list (then re-match tables),
    /// landing back on Select. Used at startup and after the folder list changes.
    pub(crate) fn scan(dirs: Vec<String>, sources: Vec<TableSource>) -> LoadingState {
        let count = Arc::new(AtomicUsize::new(0));
        let progress = count.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let library = Library::from_songs(scan_folders(&dirs, &progress));
            let (names, levels) = fetch_and_match(&sources, &library);
            let _ = tx.send(ScanOutcome { library, names, levels });
        });
        LoadingState { task: LoadingTask::Scan { rx, count }, drawn: false }
    }

    /// Rescan the folder list as it stands now.
    pub(crate) fn rescan(shared: &AppShared) -> LoadingState {
        LoadingState::scan(shared.config.library.folders.clone(), shared.config.library.tables.clone())
    }

    /// Whether keysounds are still decoding, which is what holds an audio-settings reopen back.
    pub(crate) fn keysounds_pending(&self) -> bool {
        matches!(self.task, LoadingTask::Keysounds { .. })
    }

    /// Apply a finished background scan: swap in the merged library/tables and return to the
    /// (rebuilt) select screen.
    fn apply_scan(shared: &mut AppShared, out: ScanOutcome) -> Transition {
        shared.library = out.library;
        shared.table_names = out.names;
        shared.table_levels = out.levels;
        shared.select_view = SelectView::Root;
        shared.sel = 0;
        shared.rebuild_select_items();
        println!("scanned {} charts", shared.library.len());
        shared.print_selection();
        Transition::Back
    }

    /// A background folder scan is running; apply it the frame it finishes, otherwise keep
    /// animating the LOADING screen.
    fn poll_scan(shared: &mut AppShared, rx: &Receiver<ScanOutcome>) -> Transition {
        match rx.try_recv() {
            Ok(out) => LoadingState::apply_scan(shared, out),
            Err(TryRecvError::Disconnected) => Transition::Back,
            Err(TryRecvError::Empty) => Transition::Stay,
        }
    }

    /// Perform the queued chart load (one frame after the LOADING screen is shown) then enter Play —
    /// `load()` (re)bases the song clock at its end, so playback starts at 0 only after everything is
    /// loaded. Folder scans don't reach here (they run on a background thread, polled above).
    fn finish_song(shared: &mut AppShared, index: usize) -> Transition {
        let Some(entry) = shared.library.songs().get(index) else {
            return Transition::Back;
        };
        shared.chart_path = entry.path.to_string_lossy().to_string();
        match shared.load() {
            Some(loaded) => Transition::To(shared.enter_loaded_chart(loaded)),
            None => Transition::Back,
        }
    }

    /// Drain decoded keysounds into the audio bank; once every worker has reported in, start play.
    /// Drives the determinate loading bar.
    ///
    /// A sample can be sent just before its progress tick, so the last-worker check is followed by
    /// one more drain before the receiver is dropped.
    fn poll_keysounds(shared: &mut AppShared, load: &KeysoundLoad) -> bool {
        if let Some(audio) = shared.audio.as_mut() {
            while let Ok((id, dec)) = load.rx.try_recv() {
                audio.insert_decoded(id, dec);
            }
        }
        if load.progress.load(Ordering::Relaxed) < load.total {
            return false;
        }
        if let Some(audio) = shared.audio.as_mut() {
            while let Ok((id, dec)) = load.rx.try_recv() {
                audio.insert_decoded(id, dec);
            }
        }
        println!("keysounds ready ({} decoded)", shared.audio.as_ref().map(|a| a.loaded()).unwrap_or(0));
        true
    }

    /// Cancel back to Select rather than exit: during the initial background scan `songs` is still
    /// empty, and routing through the usual exit would quit on the empty library — surprising
    /// mid-load. Any half-loaded play state is dropped with this screen.
    fn cancel(&mut self, shared: &mut AppShared) -> Transition {
        if let LoadingTask::Keysounds { load, .. } = &self.task {
            load.cancel.store(true, Ordering::Relaxed);
        }
        shared.release_play_audio();
        shared.select_view = SelectView::Root;
        shared.sel = 0;
        shared.rebuild_select_items();
        Transition::Back
    }
}

impl StageHandler for LoadingState {
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        match &self.task {
            LoadingTask::Scan { rx, .. } => LoadingState::poll_scan(ctx.shared, rx),
            LoadingTask::Keysounds { load, .. } => {
                if !LoadingState::poll_keysounds(ctx.shared, load) {
                    return Transition::Stay;
                }
                let LoadingTask::Keysounds { play, .. } = std::mem::replace(&mut self.task, LoadingTask::Song(0)) else {
                    return Transition::Stay;
                };
                ctx.shared.start_play();
                Transition::To(Stage::Play(play))
            }
            LoadingTask::Song(index) if self.drawn => LoadingState::finish_song(ctx.shared, *index),
            LoadingTask::Song(_) => Transition::Stay,
        }
    }

    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if key.pressed && key.code == KeyCode::Escape { self.cancel(ctx.shared) } else { Transition::Stay }
    }

    /// The progress bar under the heading: determinate for a keysound decode, which knows how many
    /// samples there are, and a ping-ponging sweep plus a live chart count for a folder scan, which
    /// has no total to count toward and would otherwise read as frozen.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        canvas.clear(th.bg);
        let (heading, sub): (&str, String) = match &self.task {
            LoadingTask::Song(i) => ("LOADING", ctx.shared.library.songs().get(*i).map(|e| e.title.chars().take(TITLE_CHARS).collect()).unwrap_or_default()),
            LoadingTask::Scan { .. } => ("SCANNING", format!("{} folder(s)", ctx.shared.config.library.folders.len())),
            LoadingTask::Keysounds { .. } => ("LOADING", String::new()),
        };
        let cx = CW as f32 * 0.5;
        let cy = CH as f32 * 0.5;
        let dots = ".".repeat((ctx.shared.frame_count / DOTS_PERIOD_FRAMES % DOTS_MAX) as usize);
        draw_text_centered(canvas, cx, cy - 36.0, 3.0, th.text, &format!("{heading}{dots}"));
        if !sub.is_empty() {
            draw_text_centered(canvas, cx, cy + 14.0, 1.6, th.text_dim, &sub);
        }
        let bx = cx - BAR_W * 0.5;
        let by = cy + 56.0;
        match &self.task {
            LoadingTask::Keysounds { load, .. } => {
                let done = load.progress.load(Ordering::Relaxed).min(load.total);
                let frac = if load.total > 0 { done as f32 / load.total as f32 } else { 1.0 };
                canvas.fill_rect(Rect::new(bx, by, BAR_W, BAR_H), Color::rgb(28, 28, 40));
                canvas.fill_rect(Rect::new(bx, by, BAR_W * frac, BAR_H), th.good);
                draw_text_centered(canvas, cx, by + 16.0, 1.2, th.text_dim, &format!("{done} / {} keysounds", load.total));
            }
            LoadingTask::Scan { count, .. } => {
                canvas.fill_rect(Rect::new(bx, by, BAR_W, BAR_H), Color::rgb(28, 28, 40));
                let phase = (ctx.shared.frame_count % SWEEP_PERIOD_FRAMES) as f32 / SWEEP_HALF_FRAMES;
                let t = if phase <= 1.0 { phase } else { 2.0 - phase };
                canvas.fill_rect(Rect::new(bx + t * (BAR_W - SWEEP_W), by, SWEEP_W, BAR_H), th.good);
                let found = count.load(Ordering::Relaxed);
                draw_text_centered(canvas, cx, by + 16.0, 1.2, th.text_dim, &format!("{found} charts found"));
            }
            LoadingTask::Song(_) => {}
        }
        self.drawn = true;
    }
}
