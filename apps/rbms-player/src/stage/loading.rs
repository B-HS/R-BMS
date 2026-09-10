//! The LOADING screen: the frame that is shown while something the player asked for is being read
//! off disk or off the network.
//!
//! Everything slow the app does passes through here — the folder scan, a chart's keysounds and
//! background images, and fetching a difficulty table — so that none of it happens on the frame
//! loop with the window frozen. Each task says what step it is on and how far through it is, and
//! each carries a cancel the Escape key sets, so leaving does not leave a worker behind.
#![allow(clippy::wildcard_imports)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::app_play::{LoadedChart, PendingChart};
use crate::stage::{Canvas, FrameCtx, KeyInput, PlayState, Stage, StageHandler, Transition};
use crate::tablesrc::load_and_match_md5s;
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

/// How much of a table's location the FETCHING line shows.
const LOCATION_CHARS: usize = 56;

/// How far a background folder scan has got. Read by the screen while the worker fills it in.
#[derive(Default)]
pub(crate) struct ScanProgress {
    /// Charts found so far.
    pub(crate) charts: AtomicUsize,
    /// Difficulty tables matched against the library so far.
    pub(crate) tables: AtomicUsize,
    /// Set once the folder walk is over and the difficulty tables are being matched, which is the
    /// step the screen names and the point its bar stops sweeping and starts filling.
    pub(crate) matching: AtomicBool,
    /// Set when the screen is left, so the walk stops instead of finishing work nobody wants.
    pub(crate) cancel: AtomicBool,
}

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

/// The in-flight background-image decode of a loaded chart, in the same shape as the keysounds.
pub(crate) struct BgaLoad {
    pub(crate) rx: Receiver<(i32, Vec<u8>)>,
    pub(crate) progress: Arc<AtomicUsize>,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) total: usize,
}

/// A parsed chart waiting for the files it named, and what has arrived so far.
pub(crate) struct ChartAssets {
    chart: PendingChart,
    images: std::collections::HashMap<i32, Vec<u8>>,
    bga: Option<BgaLoad>,
    keysounds: Option<KeysoundLoad>,
}

impl ChartAssets {
    /// Take everything that has arrived since the last frame. `true` once nothing is outstanding.
    fn poll(&mut self, shared: &mut AppShared) -> bool {
        if let Some(load) = self.keysounds.take()
            && !ChartAssets::poll_keysounds(shared, &load)
        {
            self.keysounds = Some(load);
        }
        if let Some(load) = self.bga.take()
            && !ChartAssets::poll_images(&mut self.images, &load)
        {
            self.bga = Some(load);
        }
        self.keysounds.is_none() && self.bga.is_none()
    }

    /// Drain decoded keysounds into the audio bank; `true` once every worker has reported in.
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

    /// Drain decoded background images into the chart's own map, on the same last-drain rule.
    fn poll_images(images: &mut std::collections::HashMap<i32, Vec<u8>>, load: &BgaLoad) -> bool {
        while let Ok((id, rgba)) = load.rx.try_recv() {
            images.insert(id, rgba);
        }
        if load.progress.load(Ordering::Relaxed) < load.total {
            return false;
        }
        while let Ok((id, rgba)) = load.rx.try_recv() {
            images.insert(id, rgba);
        }
        println!("loaded {} BGA images", images.len());
        true
    }

    /// How many of the chart's files have been dealt with, and how many it named. Both counters
    /// only ever rise, so the bar drawn from them cannot go backwards.
    fn progress(&self) -> (usize, usize) {
        let sounds = self.keysounds.as_ref().map(|load| (load.progress.load(Ordering::Relaxed).min(load.total), load.total)).unwrap_or_default();
        let images = self.bga.as_ref().map(|load| (load.progress.load(Ordering::Relaxed).min(load.total), load.total)).unwrap_or_default();
        (sounds.0 + images.0, sounds.1 + images.1)
    }

    /// What the screen says it is doing: the step still outstanding, with its own count.
    fn step(&self) -> String {
        let mut steps = Vec::new();
        if let Some(load) = self.keysounds.as_ref() {
            steps.push(format!("{} / {} keysounds", load.progress.load(Ordering::Relaxed).min(load.total), load.total));
        }
        if let Some(load) = self.bga.as_ref() {
            steps.push(format!("{} / {} images", load.progress.load(Ordering::Relaxed).min(load.total), load.total));
        }
        steps.join("   ")
    }

    /// Stop both decodes. The workers check between files, so an abandoned chart stops promptly
    /// rather than decoding hundreds of samples nobody is going to hear.
    fn stop(&self) {
        for cancel in [self.keysounds.as_ref().map(|load| &load.cancel), self.bga.as_ref().map(|load| &load.cancel)].into_iter().flatten() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// The screen that plays this chart, now that everything it named is in.
    fn into_play(self) -> PlayState {
        self.chart.into_play(self.images)
    }
}

/// A difficulty table being fetched before it joins the library.
///
/// The fetch is an http request, which can take seconds or hang on an unreachable host, so it does
/// not happen on the frame loop. `cancel` is what Escape sets: the request itself cannot be taken
/// back, but its result is dropped rather than pushed into a library the user has moved on from.
pub(crate) struct TableAdd {
    source: TableSource,
    rx: Receiver<(String, TableLevels)>,
    cancel: Arc<AtomicBool>,
}

/// What this LOADING frame is waiting to do.
///
/// `Song` reads and parses a chart; `Assets` waits for the files that chart named; `Scan`
/// (re)scans the song folders and re-matches the difficulty tables; `Table` fetches one table that
/// is being added. Only `Song` still blocks, so the LOADING frame is presented before it starts.
pub(crate) enum LoadingTask {
    /// Load the library chart at this index and enter Play.
    Song(usize),
    /// Rescan every library folder and re-match the tables, then Select.
    Scan { rx: Receiver<ScanOutcome>, progress: Arc<ScanProgress> },
    /// Decode what a parsed chart named, then enter Play.
    Assets(Box<ChartAssets>),
    /// Fetch one difficulty table, then return to the table manager.
    Table(Box<TableAdd>),
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

    /// Wait for the files a parsed chart named — its keysounds, its background images, or both.
    pub(crate) fn assets(loaded: LoadedChart) -> LoadingState {
        let assets = ChartAssets { chart: loaded.chart, images: std::collections::HashMap::new(), bga: loaded.bga, keysounds: loaded.keysounds };
        LoadingState { task: LoadingTask::Assets(Box::new(assets)), drawn: false }
    }

    /// Rescan every library folder off-thread and merge into one song list (then re-match tables),
    /// landing back on Select. Used at startup and after the folder list changes.
    pub(crate) fn scan(dirs: Vec<String>, sources: Vec<TableSource>) -> LoadingState {
        let progress = Arc::new(ScanProgress::default());
        let worker = progress.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let library = Library::from_songs(scan_folders(&dirs, &worker.charts, &worker.cancel));
            worker.matching.store(true, Ordering::Relaxed);
            let (names, levels) = fetch_and_match(&sources, &library, &worker.tables);
            let _ = tx.send(ScanOutcome { library, names, levels });
        });
        LoadingState { task: LoadingTask::Scan { rx, progress }, drawn: false }
    }

    /// Rescan the folder list as it stands now.
    pub(crate) fn rescan(shared: &AppShared) -> LoadingState {
        LoadingState::scan(shared.config.library.folders.clone(), shared.config.library.tables.clone())
    }

    /// Fetch one difficulty table off-thread and match it against the library that is loaded now.
    ///
    /// The md5s are copied out here rather than shared, so the worker does not hold the library the
    /// browser is reading from.
    pub(crate) fn table(shared: &AppShared, source: TableSource) -> LoadingState {
        let md5s: Vec<String> = shared.library.md5s().map(str::to_string).collect();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        let job = source.clone();
        let abandoned = cancel.clone();
        std::thread::spawn(move || {
            let matched = load_and_match_md5s(&job, &md5s);
            if !abandoned.load(Ordering::Relaxed) {
                let _ = tx.send(matched);
            }
        });
        LoadingState { task: LoadingTask::Table(Box::new(TableAdd { source, rx, cancel })), drawn: false }
    }

    /// Whether keysounds are still decoding, which is what holds an audio-settings reopen back.
    pub(crate) fn keysounds_pending(&self) -> bool {
        matches!(&self.task, LoadingTask::Assets(assets) if assets.keysounds.is_some())
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
        notify(Level::Info, format!("scanned {} charts", shared.library.len()));
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

    /// A table fetch is running; add it to the library the frame it lands. A source whose table
    /// could not be fetched still takes its slot, with the failure already reported by the loader,
    /// so the row is there to look at and remove rather than silently missing.
    fn poll_table(shared: &mut AppShared, add: &TableAdd) -> Transition {
        match add.rx.try_recv() {
            Ok((name, levels)) => {
                let owned: usize = levels.iter().map(|(_, charts)| charts.len()).sum();
                shared.config.library.tables.push(add.source.clone());
                shared.table_names.push(name.clone());
                shared.table_levels.push(levels);
                shared.save_settings();
                notify(Level::Info, format!("table added: {name} ({owned} local charts)"));
                Transition::Back
            }
            Err(TryRecvError::Disconnected) => Transition::Back,
            Err(TryRecvError::Empty) => Transition::Stay,
        }
    }

    /// Perform the queued chart load (one frame after the LOADING screen is shown) then wait on
    /// what it named — `load()` starts the decodes and hands back the parsed chart.
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

    /// Cancel back to Select rather than exit: during the initial background scan `songs` is still
    /// empty, and routing through the usual exit would quit on the empty library — surprising
    /// mid-load. Any half-loaded play state is dropped with this screen, and every worker this
    /// screen started is told to stop.
    fn cancel(&mut self, shared: &mut AppShared) -> Transition {
        match &self.task {
            LoadingTask::Assets(assets) => assets.stop(),
            LoadingTask::Scan { progress, .. } => progress.cancel.store(true, Ordering::Relaxed),
            LoadingTask::Table(add) => add.cancel.store(true, Ordering::Relaxed),
            LoadingTask::Song(_) => {}
        }
        shared.release_play_audio();
        shared.select_view = SelectView::Root;
        shared.sel = 0;
        shared.rebuild_select_items();
        Transition::Back
    }

    /// The heading and the line under it: what is being waited for, and which one of it.
    fn heading(&self, shared: &AppShared) -> (&'static str, String) {
        match &self.task {
            LoadingTask::Song(i) => ("LOADING", shared.library.songs().get(*i).map(|e| e.title.chars().take(TITLE_CHARS).collect()).unwrap_or_default()),
            LoadingTask::Scan { progress, .. } if progress.matching.load(Ordering::Relaxed) => {
                ("MATCHING TABLES", format!("{} / {}", progress.tables.load(Ordering::Relaxed), shared.config.library.tables.len()))
            }
            LoadingTask::Scan { progress, .. } => match progress.charts.load(Ordering::Relaxed) {
                0 => ("SCANNING", format!("{} folder(s)", shared.config.library.folders.len())),
                found => ("SCANNING", format!("{found} charts found")),
            },
            LoadingTask::Assets(assets) => ("LOADING", assets.step()),
            LoadingTask::Table(add) => ("FETCHING TABLE", add.source.location.chars().take(LOCATION_CHARS).collect()),
        }
    }

    /// A filled bar with a count under it, for a task that knows how much there is to do.
    fn draw_determinate(canvas: &mut Canvas<'_>, x: f32, y: f32, done: usize, total: usize, unit: &str) {
        let th = rbms_render::theme();
        let frac = if total > 0 { done as f32 / total as f32 } else { 1.0 };
        canvas.fill_rect(Rect::new(x, y, BAR_W, BAR_H), Color::rgb(28, 28, 40));
        canvas.fill_rect(Rect::new(x, y, BAR_W * frac, BAR_H), th.good);
        draw_text_centered(canvas, x + BAR_W * 0.5, y + 16.0, 1.2, th.text_dim, &format!("{done} / {total} {unit}"));
    }

    /// A bar that sweeps back and forth, for a task with no total to count toward. Without it a
    /// scan of a large library reads as a frozen window.
    fn draw_sweep(canvas: &mut Canvas<'_>, x: f32, y: f32, frame: u64) {
        let th = rbms_render::theme();
        canvas.fill_rect(Rect::new(x, y, BAR_W, BAR_H), Color::rgb(28, 28, 40));
        let phase = (frame % SWEEP_PERIOD_FRAMES) as f32 / SWEEP_HALF_FRAMES;
        let t = if phase <= 1.0 { phase } else { 2.0 - phase };
        canvas.fill_rect(Rect::new(x + t * (BAR_W - SWEEP_W), y, SWEEP_W, BAR_H), th.good);
    }
}

impl StageHandler for LoadingState {
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let ready = match &mut self.task {
            LoadingTask::Scan { rx, .. } => return LoadingState::poll_scan(ctx.shared, rx),
            LoadingTask::Table(add) => return LoadingState::poll_table(ctx.shared, add),
            LoadingTask::Song(index) => {
                let index = *index;
                return if self.drawn { LoadingState::finish_song(ctx.shared, index) } else { Transition::Stay };
            }
            LoadingTask::Assets(assets) => assets.poll(ctx.shared),
        };
        if !ready {
            return Transition::Stay;
        }
        let LoadingTask::Assets(assets) = std::mem::replace(&mut self.task, LoadingTask::Song(0)) else {
            return Transition::Stay;
        };
        ctx.shared.start_play();
        Transition::To(Stage::Play(Box::new(assets.into_play())))
    }

    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if key.pressed && key.code == KeyCode::Escape { self.cancel(ctx.shared) } else { Transition::Stay }
    }

    /// The heading, the step under it and the progress bar: determinate for the decodes and the
    /// table match, which know how many there are, and a ping-ponging sweep for the folder scan and
    /// the http fetch, which have no total to count toward.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        canvas.clear(th.bg);
        let (heading, sub) = self.heading(ctx.shared);
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
            LoadingTask::Assets(assets) => {
                let (done, total) = assets.progress();
                LoadingState::draw_determinate(canvas, bx, by, done, total, "files");
            }
            LoadingTask::Scan { progress, .. } if progress.matching.load(Ordering::Relaxed) => {
                let done = progress.tables.load(Ordering::Relaxed);
                LoadingState::draw_determinate(canvas, bx, by, done, ctx.shared.config.library.tables.len(), "tables");
            }
            LoadingTask::Scan { .. } => LoadingState::draw_sweep(canvas, bx, by, ctx.shared.frame_count),
            LoadingTask::Table(_) => LoadingState::draw_sweep(canvas, bx, by, ctx.shared.frame_count),
            LoadingTask::Song(_) => {}
        }
        self.drawn = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keysounds(total: usize) -> KeysoundLoad {
        let (_tx, rx) = std::sync::mpsc::channel();
        KeysoundLoad { rx, progress: Arc::new(AtomicUsize::new(0)), cancel: Arc::new(AtomicBool::new(false)), total }
    }

    fn images(total: usize) -> BgaLoad {
        let (_tx, rx) = std::sync::mpsc::channel();
        BgaLoad { rx, progress: Arc::new(AtomicUsize::new(0)), cancel: Arc::new(AtomicBool::new(false)), total }
    }

    fn assets(sounds: Option<KeysoundLoad>, bga: Option<BgaLoad>) -> ChartAssets {
        ChartAssets { chart: crate::app_play::pending_chart_for_tests(), images: std::collections::HashMap::new(), bga, keysounds: sounds }
    }

    /// One bar covers both decodes, so it cannot jump backwards when one of them finishes.
    #[test]
    fn the_bar_counts_every_file_the_chart_named() {
        let assets = assets(Some(keysounds(300)), Some(images(4)));
        assert_eq!(assets.progress(), (0, 304));
        assets.keysounds.as_ref().expect("keysounds").progress.store(120, Ordering::Relaxed);
        assets.bga.as_ref().expect("images").progress.store(2, Ordering::Relaxed);
        assert_eq!(assets.progress(), (122, 304));
    }

    /// A worker counts jobs attempted and can be asked for one more than there are if a file is
    /// reported twice by a racing read; the bar may not run past its end.
    #[test]
    fn a_worker_that_over_reports_cannot_push_the_bar_past_full() {
        let assets = assets(Some(keysounds(10)), None);
        assets.keysounds.as_ref().expect("keysounds").progress.store(99, Ordering::Relaxed);
        assert_eq!(assets.progress(), (10, 10));
    }

    #[test]
    fn a_chart_with_no_images_still_reports_its_keysounds() {
        let assets = assets(Some(keysounds(8)), None);
        assert_eq!(assets.progress(), (0, 8));
        assert!(assets.step().contains("keysounds"));
        assert!(!assets.step().contains("images"));
    }

    #[test]
    fn the_step_names_both_decodes_while_both_are_running() {
        let assets = assets(Some(keysounds(2)), Some(images(3)));
        let step = assets.step();
        assert!(step.contains("0 / 2 keysounds"), "got {step}");
        assert!(step.contains("0 / 3 images"), "got {step}");
    }

    /// Leaving mid-load has to stop every worker the screen started, or an abandoned chart keeps
    /// decoding hundreds of samples nobody is going to hear.
    #[test]
    fn leaving_stops_both_decodes() {
        let assets = assets(Some(keysounds(4)), Some(images(4)));
        assets.stop();
        assert!(assets.keysounds.as_ref().expect("keysounds").cancel.load(Ordering::Relaxed));
        assert!(assets.bga.as_ref().expect("images").cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn leaving_a_scan_stops_the_walk() {
        let state = LoadingState { task: LoadingTask::Scan { rx: std::sync::mpsc::channel().1, progress: Arc::new(ScanProgress::default()) }, drawn: false };
        let LoadingTask::Scan { progress, .. } = &state.task else {
            panic!("the task is a scan");
        };
        assert!(!progress.cancel.load(Ordering::Relaxed));
        progress.cancel.store(true, Ordering::Relaxed);
        assert!(progress.cancel.load(Ordering::Relaxed), "the walk reads this between folders");
    }

    /// The audio settings may not reopen the stream under a chart that is still filling its bank,
    /// so this has to keep answering for the decode rather than for the screen.
    #[test]
    fn the_screen_reports_keysounds_as_pending_only_while_they_are() {
        let waiting = LoadingState { task: LoadingTask::Assets(Box::new(assets(Some(keysounds(4)), None))), drawn: false };
        assert!(waiting.keysounds_pending());
        let images_only = LoadingState { task: LoadingTask::Assets(Box::new(assets(None, Some(images(4))))), drawn: false };
        assert!(!images_only.keysounds_pending(), "images do not hold the audio stream");
        assert!(!LoadingState::song(0).keysounds_pending());
    }

    #[test]
    fn a_chart_with_nothing_outstanding_is_ready_on_the_first_poll() {
        let mut app = crate::stage::render_tests::app();
        assert!(assets(None, None).poll(&mut app.shared), "a chart that named no files has nothing to wait for");
        assert!(!assets(Some(keysounds(4)), None).poll(&mut app.shared), "one that did has to wait for them");
    }

    /// The scan says which of its two steps it is on, so a library big enough to spend a minute
    /// matching tables does not look like a scan that stopped.
    #[test]
    fn the_scan_names_the_step_it_is_on() {
        let mut app = crate::stage::render_tests::app();
        app.shared.config.library.tables = vec![TableSource { name: "one".into(), location: "/one.json".into() }];
        let progress = Arc::new(ScanProgress::default());
        let state = LoadingState { task: LoadingTask::Scan { rx: std::sync::mpsc::channel().1, progress: progress.clone() }, drawn: false };
        assert_eq!(state.heading(&app.shared).0, "SCANNING");
        progress.charts.store(42, Ordering::Relaxed);
        assert_eq!(state.heading(&app.shared).1, "42 charts found");
        progress.matching.store(true, Ordering::Relaxed);
        let (heading, sub) = state.heading(&app.shared);
        assert_eq!(heading, "MATCHING TABLES");
        assert_eq!(sub, "0 / 1");
    }
}
