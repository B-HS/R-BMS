//! The LOADING screen: the frame that is shown while something the player asked for is being read
//! off disk or off the network.
//!
//! Everything slow the app waits for passes through here — the folder scan, and a chart's keysounds
//! and background images — so that none of it happens on the frame loop with the window frozen.
//! Fetching a difficulty table is the one slow thing that does not, because nothing waits for it: a
//! table that has no copy yet, one that is being registered and one that is fetched again all run
//! beside whichever screen is up ([`crate::tablesrc::TableFetches`]), and the scan here reads only
//! the copies and the table files that are on this machine. Each task says what step it is on and
//! how far through it is, and each carries a cancel the Escape key sets, so leaving does not leave
//! a worker behind.
//!
//! The screen is always the built-in one. A chart picked in the browser is shown by the decide
//! scene while its files decode ([`crate::stage::decide`]) when a skin draws that scene, and comes
//! here only for whatever is still outstanding when the scene ends; [`ChartAssets`] is the load
//! the two screens hand to each other.
//!
//! A chart whose play screen a skin draws does not wait here at all. The reference's play screen
//! has a loading state of its own (`BMSPlayer.STATE_PRELOAD`), which the skin draws, so the chart
//! goes to the play screen as soon as it is parsed and the decodes go with it
//! ([`ChartAssets::enter_preloading`], [`ChartLoads`]). Only the folder scan and a chart with no
//! play skin are waited for on this screen.
#![allow(clippy::wildcard_imports)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::app_play::{LoadedChart, PendingChart};
use crate::stage::{Canvas, FrameCtx, KeyInput, PlayState, Stage, StageHandler, Transition};
use crate::tablesrc::{TableStore, read_and_match_all};
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

/// How far the table-matching half of a background scan has got. The chart walk keeps its own
/// counters inside the library scan it is driving, so only what happens after it lives here.
#[derive(Default)]
pub(crate) struct ScanProgress {
    /// Difficulty tables matched against the library so far.
    pub(crate) tables: AtomicUsize,
}

/// Which half of a background scan is running: the chart walk into the song database, or the
/// difficulty-table match against the library that walk produced.
///
/// They are two steps rather than one worker because the screen names them separately, and
/// because only the first is worth stopping: the match reads the copies of the tables and the table
/// files off this machine and asks no address anything, so it is over in the time it takes to read
/// them. `tables` is where the match reads the copies from.
pub(crate) enum ScanStep {
    Walking { scan: crate::library::LibraryScan, sources: Vec<TableSource>, tables: TableStore },
    Matching { rx: Receiver<ScanOutcome> },
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
    pub(crate) rx: Receiver<(i32, crate::DecodedImage)>,
    pub(crate) progress: Arc<AtomicUsize>,
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) total: usize,
}

/// The decodes of a parsed chart that are still running: its keysounds, its background images, or
/// both. Either is `None` once it has finished, or when the chart named nothing of that kind.
///
/// Whichever screen is up while they run owns this: the LOADING screen, the decide scene, or the
/// play screen itself while a skin draws its loading state.
#[derive(Default)]
pub(crate) struct ChartLoads {
    bga: Option<BgaLoad>,
    keysounds: Option<KeysoundLoad>,
}

impl ChartLoads {
    /// The decodes a chart load started.
    pub(crate) fn new(bga: Option<BgaLoad>, keysounds: Option<KeysoundLoad>) -> ChartLoads {
        ChartLoads { bga, keysounds }
    }

    /// Take everything that has arrived since the last frame: the keysounds into the audio bank and
    /// the images into `images`. `true` once nothing is outstanding.
    pub(crate) fn poll(&mut self, shared: &mut AppShared, images: &mut std::collections::HashMap<i32, crate::DecodedImage>) -> bool {
        if let Some(load) = self.keysounds.take()
            && !ChartLoads::poll_keysounds(shared, &load)
        {
            self.keysounds = Some(load);
        }
        if let Some(load) = self.bga.take()
            && !ChartLoads::poll_images(images, &load)
        {
            self.bga = Some(load);
        }
        self.is_done()
    }

    /// Whether nothing is outstanding.
    pub(crate) fn is_done(&self) -> bool {
        self.keysounds.is_none() && self.bga.is_none()
    }

    /// Whether keysounds are still decoding.
    fn keysounds_pending(&self) -> bool {
        self.keysounds.is_some()
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
    fn poll_images(images: &mut std::collections::HashMap<i32, crate::DecodedImage>, load: &BgaLoad) -> bool {
        while let Ok((id, image)) = load.rx.try_recv() {
            images.insert(id, image);
        }
        if load.progress.load(Ordering::Relaxed) < load.total {
            return false;
        }
        while let Ok((id, image)) = load.rx.try_recv() {
            images.insert(id, image);
        }
        println!("loaded {} BGA images", images.len());
        true
    }

    /// How many of the chart's files have been dealt with, and how many it named. Both counters
    /// only ever rise, so the bar drawn from them cannot go backwards.
    pub(crate) fn progress(&self) -> (usize, usize) {
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
    pub(crate) fn stop(&self) {
        for cancel in [self.keysounds.as_ref().map(|load| &load.cancel), self.bga.as_ref().map(|load| &load.cancel)].into_iter().flatten() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// The heading and the line under it while these decodes are what is waited for: the course
    /// and its stage on a course, and otherwise the step still outstanding.
    fn heading(&self, shared: &AppShared) -> (&'static str, String) {
        course_heading(shared).unwrap_or_else(|| ("LOADING", self.step()))
    }

    /// The built-in LOADING screen for these decodes: the heading, the step under it, and the bar
    /// while there is anything to count. The LOADING screen draws a chart's files coming in with
    /// this, and so does a play screen that waits for them itself when no skin draws its wait.
    pub(crate) fn draw(&self, shared: &AppShared, canvas: &mut Canvas<'_>) {
        let (heading, sub) = self.heading(shared);
        let (bx, by) = draw_heading(canvas, shared.frame_count, heading, &sub);
        let (done, total) = self.progress();
        if total > 0 {
            LoadingState::draw_determinate(canvas, bx, by, done, total, "files");
        }
    }
}

/// The heading of a chart that is loading as a stage of a course, or `None` outside one.
fn course_heading(shared: &AppShared) -> Option<(&'static str, String)> {
    shared.course_run.as_ref().map(|run| ("COURSE", format!("{} - {}", run.course.name, stage_label(run))))
}

/// Paints the screen's ground, the heading with its ellipsis and the line under it, and answers
/// where the progress bar under them begins.
fn draw_heading(canvas: &mut Canvas<'_>, frame_count: u64, heading: &str, sub: &str) -> (f32, f32) {
    let th = rbms_render::theme();
    canvas.clear(th.bg);
    let cx = CW as f32 * 0.5;
    let cy = CH as f32 * 0.5;
    let dots = ".".repeat((frame_count / DOTS_PERIOD_FRAMES % DOTS_MAX) as usize);
    draw_text_centered(canvas, cx, cy - 36.0, 3.0, th.text, &format!("{heading}{dots}"));
    if !sub.is_empty() {
        draw_text_centered(canvas, cx, cy + 14.0, 1.6, th.text_dim, sub);
    }
    (cx - BAR_W * 0.5, cy + 56.0)
}

/// A parsed chart waiting for the files it named, and what has arrived so far.
pub(crate) struct ChartAssets {
    chart: PendingChart,
    images: std::collections::HashMap<i32, crate::DecodedImage>,
    loads: ChartLoads,
    /// Whether the decide cue has been heard for this chart already, by a decide scene that showed
    /// the chart while this was decoding. It is heard once a chart, whichever screen plays it.
    announced: bool,
}

impl ChartAssets {
    /// The load of a chart that has just been parsed, with nothing of it in yet.
    pub(crate) fn of(loaded: LoadedChart) -> ChartAssets {
        ChartAssets { chart: loaded.chart, images: std::collections::HashMap::new(), loads: ChartLoads::new(loaded.bga, loaded.keysounds), announced: false }
    }

    /// Note that the decide cue has been played for this chart, so the screen that starts the run
    /// does not play it again.
    pub(crate) fn mark_announced(&mut self) {
        self.announced = true;
    }

    /// Take everything that has arrived since the last frame. `true` once nothing is outstanding.
    pub(crate) fn poll(&mut self, shared: &mut AppShared) -> bool {
        self.loads.poll(shared, &mut self.images)
    }

    /// How many of the chart's files have been dealt with, and how many it named.
    pub(crate) fn progress(&self) -> (usize, usize) {
        self.loads.progress()
    }

    /// Stop both decodes.
    pub(crate) fn stop(&self) {
        self.loads.stop();
    }

    /// The screen that plays this chart, now that everything it named is in.
    fn into_play(self) -> PlayState {
        self.chart.into_play(self.images)
    }

    fn take_images(&mut self) -> std::collections::HashMap<i32, crate::DecodedImage> {
        std::mem::take(&mut self.images)
    }

    /// Whether the play screen takes this chart before its files are in: a skin draws the play
    /// screen, so the screen's own loading state waits for them, as the reference's does
    /// (`BMSPlayer.STATE_PRELOAD`). A chart asked for practice still waits where it always did,
    /// because the practice panel is built from the decoded chart.
    pub(crate) fn preloads_on_the_play_screen(shared: &AppShared) -> bool {
        shared.has_play_scene() && !shared.has_practice_request()
    }

    /// The play screen for this chart with whatever is still decoding handed to it, which is where
    /// the chart goes when a skin draws that screen. The song clock is not started: the play screen
    /// starts it when the run does.
    pub(crate) fn enter_preloading(self, shared: &mut AppShared) -> Stage {
        if !self.announced {
            shared.play_system_sound(SystemSound::Decide);
        }
        let ChartAssets { chart, images, loads, .. } = self;
        let mut play = chart.into_play(images);
        play.wait_for(loads);
        Stage::Play(Box::new(play))
    }

    /// The screen this chart goes to now that everything it named is in: the practice panel when
    /// the browser asked for one, and otherwise the run, started here with the decide cue unless a
    /// decide scene has already played it.
    pub(crate) fn enter(mut self, shared: &mut AppShared) -> Stage {
        if shared.has_practice_request() {
            let mut images = self.take_images();
            if let Some(stage) = shared.practice_stage_if_requested(&mut images) {
                return stage;
            }
            self.images = images;
        }
        if ChartAssets::preloads_on_the_play_screen(shared) {
            return self.enter_preloading(shared);
        }
        if !self.announced {
            shared.play_system_sound(SystemSound::Decide);
        }
        shared.start_play();
        Stage::Play(Box::new(self.into_play()))
    }
}

/// What this LOADING frame is waiting to do.
///
/// `Song` reads and parses a chart; `Assets` waits for the files that chart named; `Scan`
/// (re)scans the song folders and re-matches the difficulty tables that can be read off this
/// machine. Only `Song` still blocks, so the LOADING frame is presented before it starts.
pub(crate) enum LoadingTask {
    /// Load the library chart at this index and enter Play.
    Song(usize),
    /// Rescan every library folder and re-match the tables, then Select.
    Scan { step: ScanStep, progress: Arc<ScanProgress> },
    /// Decode what a parsed chart named, then enter Play.
    Assets(Box<ChartAssets>),
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
        LoadingState::waiting_on(Box::new(ChartAssets::of(loaded)))
    }

    /// Carry on waiting for a load another screen started, from wherever it has got to.
    pub(crate) fn waiting_on(assets: Box<ChartAssets>) -> LoadingState {
        LoadingState { task: LoadingTask::Assets(assets), drawn: false }
    }

    /// Rescan every library folder off-thread into the song database (then re-match tables),
    /// landing back on Select. Used at startup and after the folder list changes.
    ///
    /// The song database is kept beside the settings file, and so are the copies of the tables, so
    /// where the one is says where the others are.
    pub(crate) fn scan(dirs: Vec<String>, sources: Vec<TableSource>, db_path: PathBuf, full: bool) -> LoadingState {
        let tables = TableStore::beside(&db_path);
        let scan = crate::library::spawn_scan(db_path, dirs, full);
        let step = ScanStep::Walking { scan, sources, tables };
        LoadingState { task: LoadingTask::Scan { step, progress: Arc::new(ScanProgress::default()) }, drawn: false }
    }

    /// Rescan the folder list as it stands now.
    pub(crate) fn rescan(shared: &AppShared) -> LoadingState {
        let full = shared.song_db.as_ref().is_none_or(|db| db.needs_full_rescan().unwrap_or(false));
        LoadingState::scan(shared.config.library.folders.clone(), shared.config.library.tables.clone(), songdb_path(&shared.settings_path), full)
    }

    /// Whether keysounds are still decoding, which is what holds an audio-settings reopen back.
    pub(crate) fn keysounds_pending(&self) -> bool {
        matches!(&self.task, LoadingTask::Assets(assets) if assets.loads.keysounds_pending())
    }

    /// Whether this screen plays the decide cue when the run it is waiting for starts, for the tests
    /// that hold the cue to once a chart.
    #[cfg(test)]
    pub(crate) fn plays_the_decide_cue(&self) -> bool {
        matches!(&self.task, LoadingTask::Assets(assets) if !assets.announced)
    }

    /// Apply a finished background scan: swap in the merged library/tables and return to the
    /// (rebuilt) select screen.
    ///
    /// A table at an address that has no copy was not read and has no folder yet. Those are due
    /// their first fetch from here on, which the song list starts once it is up
    /// ([`crate::tablesrc::start_first_fetches`]); the scan itself waited for none of them. The
    /// player's own folders are matched against the new library from what was kept of them, asking
    /// the network nothing ([`crate::user_folders::rematch`]).
    fn apply_scan(shared: &mut AppShared, out: ScanOutcome) -> Transition {
        shared.library = out.library;
        shared.table_names = out.names;
        shared.table_levels = out.levels;
        shared.table_fetches.first_fetches_are_due();
        crate::user_folders::rematch(shared);
        shared.select_view = SelectView::Root;
        shared.sel = 0;
        shared.rebuild_select_items();
        notify(Level::Info, format!("scanned {} charts", shared.library.len()));
        shared.print_selection();
        Transition::Back
    }

    /// A background scan is running; move it on the frame its current step finishes, otherwise keep
    /// animating the LOADING screen.
    fn poll_scan(shared: &mut AppShared, step: &mut ScanStep, progress: &Arc<ScanProgress>) -> Transition {
        match step {
            ScanStep::Walking { scan, sources, tables } => {
                let Some(report) = scan.poll() else {
                    return Transition::Stay;
                };
                if let Some(failure) = &report.failure {
                    notify(Level::Error, format!("library scan failed: {failure}"));
                }
                if report.cancelled {
                    notify(Level::Info, format!("library scan stopped early ({} charts read)", report.upserted));
                } else if report.upserted > 0 || report.removed > 0 {
                    notify(Level::Info, format!("library: {} chart(s) read, {} dropped", report.upserted, report.removed));
                }
                *step =
                    ScanStep::Matching { rx: LoadingState::spawn_table_match(report.library, std::mem::take(sources), tables.clone(), Arc::clone(progress)) };
                Transition::Stay
            }
            ScanStep::Matching { rx } => match rx.try_recv() {
                Ok(out) => LoadingState::apply_scan(shared, out),
                Err(TryRecvError::Disconnected) => Transition::Back,
                Err(TryRecvError::Empty) => Transition::Stay,
            },
        }
    }

    /// Match every configured difficulty table against the library the walk just produced, off the
    /// frame loop because a table is read and parsed for it: a table file, or the copy kept of a
    /// table at an address. No address is asked anything, so this waits on no server; a table that
    /// has no copy keeps its slot with no levels until it has been fetched beside the song list.
    fn spawn_table_match(library: Library, sources: Vec<TableSource>, tables: TableStore, progress: Arc<ScanProgress>) -> Receiver<ScanOutcome> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (names, levels) = read_and_match_all(&tables, &sources, &library, &progress.tables);
            let _ = tx.send(ScanOutcome { library, names, levels });
        });
        rx
    }

    /// Perform the queued chart load (one frame after the LOADING screen is shown) then wait on
    /// what it named — `load()` starts the decodes and hands back the parsed chart.
    ///
    /// A course stage whose chart cannot be read ends the course there, as leaving it does.
    fn finish_song(shared: &mut AppShared, index: usize) -> Transition {
        let Some(entry) = shared.library.songs().get(index) else {
            return Self::give_up_on_the_chart(shared);
        };
        shared.chart_path = entry.path.to_string_lossy().to_string();
        match shared.load() {
            Some(loaded) => Transition::To(shared.enter_loaded_chart(loaded)),
            None => Self::give_up_on_the_chart(shared),
        }
    }

    /// Go back without the chart this screen was loading. A course standing on it ends, keeping what
    /// was played of it ([`crate::app_result::abandon_course`]): left standing, it would take the next
    /// chart the player picks as its next stage.
    fn give_up_on_the_chart(shared: &mut AppShared) -> Transition {
        if shared.course_run.is_some() {
            return crate::app_result::abandon_course(shared, crate::app_result::Sending::Now);
        }
        Transition::Back
    }

    /// Cancel back to Select rather than exit: during the initial background scan `songs` is still
    /// empty, and routing through the usual exit would quit on the empty library — surprising
    /// mid-load. Any half-loaded play state is dropped with this screen, and every worker this
    /// screen started is told to stop. A course loading its next stage ends with it.
    ///
    /// The table match is not something to cancel. The walk it follows is finished, the match
    /// itself reads a few files and is over in a moment, and leaving in the middle of it would throw
    /// away the library that was just read and every table with it for the rest of the run. Escape
    /// there is let pass, and the screen hands on to the song list by itself.
    fn cancel(&mut self, shared: &mut AppShared) -> Transition {
        match &self.task {
            LoadingTask::Assets(assets) => assets.stop(),
            LoadingTask::Scan { step: ScanStep::Walking { scan, .. }, .. } => scan.cancel(),
            LoadingTask::Scan { step: ScanStep::Matching { .. }, .. } => return Transition::Stay,
            LoadingTask::Song(_) => {}
        }
        shared.release_play_audio();
        shared.select_view = SelectView::Root;
        shared.sel = 0;
        shared.rebuild_select_items();
        Self::give_up_on_the_chart(shared)
    }

    /// Whether the chart queued here is about to be shown by a skin's play screen, whose own loading
    /// state is what the player is meant to see: the one frame this screen is up for before the
    /// chart is parsed is then left black rather than drawn as the built-in LOADING screen.
    fn hands_on_to_a_play_skin(&self, shared: &AppShared) -> bool {
        let LoadingTask::Song(index) = &self.task else {
            return false;
        };
        !shared.has_practice_request() && shared.library.songs().get(*index).is_some_and(|entry| shared.has_play_scene_for(entry.mode))
    }

    /// The heading and the line under it: what is being waited for, and which one of it.
    fn heading(&self, shared: &AppShared) -> (&'static str, String) {
        match &self.task {
            LoadingTask::Song(i) => course_heading(shared)
                .unwrap_or_else(|| ("LOADING", shared.library.songs().get(*i).map(|e| e.title.chars().take(TITLE_CHARS).collect()).unwrap_or_default())),
            LoadingTask::Scan { step: ScanStep::Matching { .. }, progress } => {
                ("MATCHING TABLES", format!("{} / {}", progress.tables.load(Ordering::Relaxed), shared.config.library.tables.len()))
            }
            LoadingTask::Scan { step: ScanStep::Walking { scan, .. }, .. } => match scan.counts().found {
                0 => ("SCANNING", format!("{} folder(s)", shared.config.library.folders.len())),
                found => ("SCANNING", format!("{found} charts found")),
            },
            LoadingTask::Assets(assets) => assets.loads.heading(shared),
        }
    }

    /// A filled bar with a count under it, for a task that knows how much there is to do.
    fn draw_determinate(canvas: &mut Canvas<'_>, x: f32, y: f32, done: usize, total: usize, unit: &str) {
        let th = rbms_render::theme();
        let frac = if total > 0 { done as f32 / total as f32 } else { 1.0 };
        canvas.fill_rect(Rect::new(x, y, BAR_W, BAR_H), th.panel_hi);
        canvas.fill_rect(Rect::new(x, y, BAR_W * frac, BAR_H), th.good);
        draw_text_centered(canvas, x + BAR_W * 0.5, y + 16.0, 1.2, th.text_dim, &format!("{done} / {total} {unit}"));
    }

    /// A bar that sweeps back and forth, for a task with no total to count toward. Without it a
    /// scan of a large library reads as a frozen window.
    fn draw_sweep(canvas: &mut Canvas<'_>, x: f32, y: f32, frame: u64) {
        let th = rbms_render::theme();
        canvas.fill_rect(Rect::new(x, y, BAR_W, BAR_H), th.panel_hi);
        let phase = (frame % SWEEP_PERIOD_FRAMES) as f32 / SWEEP_HALF_FRAMES;
        let t = if phase <= 1.0 { phase } else { 2.0 - phase };
        canvas.fill_rect(Rect::new(x + t * (BAR_W - SWEEP_W), y, SWEEP_W, BAR_H), th.good);
    }
}

impl StageHandler for LoadingState {
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let ready = match &mut self.task {
            LoadingTask::Scan { step, progress } => return LoadingState::poll_scan(ctx.shared, step, progress),
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
        Transition::To(assets.enter(ctx.shared))
    }

    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if key.pressed && key.code == KeyCode::Escape { self.cancel(ctx.shared) } else { Transition::Stay }
    }

    /// The heading, the step under it and the progress bar: determinate for the decodes and the
    /// table match, which know how many there are, and a ping-ponging sweep for the folder scan,
    /// which has no total to count toward.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        canvas.clear_bga();
        if self.hands_on_to_a_play_skin(ctx.shared) {
            canvas.native().clear(Color::BLACK);
            self.drawn = true;
            return;
        }
        let (heading, sub) = self.heading(ctx.shared);
        let (bx, by) = draw_heading(canvas, ctx.shared.frame_count, heading, &sub);
        match &self.task {
            LoadingTask::Assets(assets) => {
                let (done, total) = assets.progress();
                LoadingState::draw_determinate(canvas, bx, by, done, total, "files");
            }
            LoadingTask::Scan { step: ScanStep::Matching { .. }, progress } => {
                let done = progress.tables.load(Ordering::Relaxed);
                LoadingState::draw_determinate(canvas, bx, by, done, ctx.shared.config.library.tables.len(), "tables");
            }
            LoadingTask::Scan { .. } => LoadingState::draw_sweep(canvas, bx, by, ctx.shared.frame_count),
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
        ChartAssets {
            chart: crate::app_play::pending_chart_for_tests(),
            images: std::collections::HashMap::new(),
            loads: ChartLoads::new(bga, sounds),
            announced: false,
        }
    }

    fn decoded_images() -> std::collections::HashMap<i32, crate::DecodedImage> {
        [(1, crate::DecodedImage::for_test(vec![1, 2, 3, 4], 1, 1)), (2, crate::DecodedImage::for_test(vec![5, 6, 7, 8], 1, 1))].into_iter().collect()
    }

    /// One bar covers both decodes, so it cannot jump backwards when one of them finishes.
    #[test]
    fn the_bar_counts_every_file_the_chart_named() {
        let assets = assets(Some(keysounds(300)), Some(images(4)));
        assert_eq!(assets.progress(), (0, 304));
        assets.loads.keysounds.as_ref().expect("keysounds").progress.store(120, Ordering::Relaxed);
        assets.loads.bga.as_ref().expect("images").progress.store(2, Ordering::Relaxed);
        assert_eq!(assets.progress(), (122, 304));
    }

    /// A worker counts jobs attempted and can be asked for one more than there are if a file is
    /// reported twice by a racing read; the bar may not run past its end.
    #[test]
    fn a_worker_that_over_reports_cannot_push_the_bar_past_full() {
        let assets = assets(Some(keysounds(10)), None);
        assets.loads.keysounds.as_ref().expect("keysounds").progress.store(99, Ordering::Relaxed);
        assert_eq!(assets.progress(), (10, 10));
    }

    #[test]
    fn a_chart_with_no_images_still_reports_its_keysounds() {
        let assets = assets(Some(keysounds(8)), None);
        assert_eq!(assets.progress(), (0, 8));
        assert!(assets.loads.step().contains("keysounds"));
        assert!(!assets.loads.step().contains("images"));
    }

    #[test]
    fn the_step_names_both_decodes_while_both_are_running() {
        let assets = assets(Some(keysounds(2)), Some(images(3)));
        let step = assets.loads.step();
        assert!(step.contains("0 / 2 keysounds"), "got {step}");
        assert!(step.contains("0 / 3 images"), "got {step}");
    }

    #[test]
    fn taking_images_for_practice_consumes_the_loading_map() {
        let mut assets = assets(None, None);
        assets.images = decoded_images();

        let images = assets.take_images();

        assert_eq!(images.len(), 2);
        assert!(assets.images.is_empty());
    }

    #[test]
    fn normal_play_moves_the_loaded_images_into_the_play_state() {
        let mut assets = assets(None, None);
        assets.images = decoded_images();

        let play = assets.into_play();

        assert_eq!(play.bga_count(), 2);
    }

    /// Leaving mid-load has to stop every worker the screen started, or an abandoned chart keeps
    /// decoding hundreds of samples nobody is going to hear.
    #[test]
    fn leaving_stops_both_decodes() {
        let assets = assets(Some(keysounds(4)), Some(images(4)));
        assets.stop();
        assert!(assets.loads.keysounds.as_ref().expect("keysounds").cancel.load(Ordering::Relaxed));
        assert!(assets.loads.bga.as_ref().expect("images").cancel.load(Ordering::Relaxed));
    }

    /// Leaving mid-scan has to stop the walk, or an abandoned scan keeps reading a library nobody
    /// is waiting for. The walk is what owns the flag, so what is checked is that the screen reaches
    /// it rather than that a scan of its own is stopped.
    #[test]
    fn leaving_a_scan_stops_the_walk() {
        let mut app = crate::stage::render_tests::app();
        let db = std::env::temp_dir().join(format!("rbms-cancel-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&db);
        let mut state = LoadingState::scan(Vec::new(), Vec::new(), db.clone(), false);
        state.cancel(&mut app.shared);
        let LoadingTask::Scan { step: ScanStep::Walking { scan, .. }, .. } = &state.task else {
            panic!("the task is a walking scan");
        };
        assert_eq!(scan.counts().found, 0, "an empty root walks nothing");
        let _ = std::fs::remove_file(&db);
    }

    /// Run a scan of no folders over `sources`, started the way the app starts one, and stay on
    /// the LOADING screen until it hands back to the song list.
    fn scanned(app: &mut crate::App, sources: Vec<TableSource>) {
        let settings = app.shared.settings_path.clone();
        std::fs::create_dir_all(settings.parent().expect("the settings file is in a folder")).expect("the settings folder");
        let mut state = LoadingState::scan(Vec::new(), sources, songdb_path(&settings), false);
        loop {
            match state.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }) {
                Transition::Stay => std::thread::yield_now(),
                Transition::Back => return,
                _ => panic!("a scan hands back to the screen it was opened over"),
            }
        }
    }

    /// Run the song list's frames until nothing is being fetched any more.
    fn landed(app: &mut crate::App, select: &mut SelectState) {
        while !app.shared.table_fetches.is_idle() {
            select.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
            std::thread::yield_now();
        }
    }

    /// Starting up, and reading the library again: the table step reads the copies and asks no
    /// address anything, whether a table has a copy, has none, or is at an address that will never
    /// answer. The song list is what fetches the ones with no copy, once, after the scan has handed
    /// back; the one that answers is kept and is a folder, the one that does not says why once and
    /// is left alone when the library is read again.
    #[test]
    fn the_table_step_asks_no_address_anything_and_the_song_list_fetches_what_has_no_copy() {
        const BODY: &[u8] = br#"[{"md5":"1111","level":"1"}]"#;
        crate::notify::exclusive(|| {
            let (kept, new, dead) = ("https://tables.test/scan/kept/", "https://tables.test/scan/new/", "https://tables.test/scan/dead/");
            let mut app = crate::stage::render_tests::app();
            let tables = crate::tablesrc::test_outside::FakeTables::default();
            let _installed = tables.install();
            let store = TableStore::beside(&app.shared.settings_path);
            [kept, new, dead].into_iter().for_each(|address| store.forget(address));
            store.keep_for_tests(kept, "Kept", BODY, Some(1));
            tables.serve(new, "New", BODY);
            let sources: Vec<TableSource> = [kept, new, dead].map(|address| TableSource { name: String::new(), location: address.to_string() }).to_vec();
            app.shared.config.library.tables = sources.clone();

            scanned(&mut app, sources.clone());
            assert_eq!(tables.asked(), 0, "the table step of a scan asked an address for its table");
            assert_eq!(app.shared.table_names, vec!["KEPT".to_string(), "TABLE".to_string(), "TABLE".to_string()]);
            assert_eq!(app.shared.table_levels.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 0, 0], "only the table that has a copy has levels yet");
            assert!(app.shared.table_fetches.is_idle(), "the scan started a fetch of its own");
            for address in [new, dead] {
                assert_eq!(crate::tablesrc::table_status(address), Some(crate::tablesrc::TableStatus::NotFetched), "{address}");
            }

            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            let mut select = SelectState::new();
            select.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
            landed(&mut app, &mut select);
            assert_eq!(tables.asked(), 2, "each table that has no copy is fetched once, and the one that has is not fetched");
            assert_eq!(app.shared.table_names[1], "NEW");
            assert_eq!(app.shared.table_levels.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1, 0]);
            assert!(store.has_copy_file(new) && !store.has_copy_file(dead), "the table that arrived is kept, the other has nothing to keep");
            assert!(crate::tablesrc::table_load_failed(dead), "the table that did not answer is not shown as failed");
            crate::notify::drain(&mut said);
            let count = |said: &[(Level, String)], start: &str| said.iter().filter(|(_, message)| message.starts_with(start)).count();
            assert_eq!(count(&said, "fetching 2 difficulty table(s) for the first time"), 1, "{said:?}");
            assert_eq!(count(&said, &format!("table fetch failed ({dead}): ")), 1, "{said:?}");
            assert_eq!(count(&said, "difficulty tables fetched for the first time: 1 added, 1 failed"), 1, "{said:?}");

            said.clear();
            scanned(&mut app, sources);
            select.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
            landed(&mut app, &mut select);
            assert_eq!(tables.asked(), 2, "reading the library again fetched a table: the one that failed, or the one that has a copy now");
            assert_eq!(app.shared.table_names[1], "NEW", "the table that was fetched is not read from its copy");
            assert!(crate::tablesrc::table_load_failed(dead), "reading the library again forgot that the table had failed");
            crate::notify::drain(&mut said);
            assert_eq!(count(&said, "table fetch failed") + count(&said, "difficulty tables fetched") + count(&said, "fetching"), 0, "{said:?}");

            for address in [kept, new, dead] {
                crate::tablesrc::forget_table_status(address);
                store.forget(address);
            }
        });
    }

    /// A table named on the command line is a source like any the player registered: the scan the
    /// app starts with reads it without asking its address anything, and the song list fetches it
    /// when it has no copy.
    #[test]
    fn a_table_named_on_the_command_line_is_read_and_fetched_like_any_other() {
        let address = "https://tables.test/scan/command-line/";
        let tables = crate::tablesrc::test_outside::FakeTables::default();
        let _installed = tables.install();
        tables.serve(address, "Named", br#"[{"md5":"1111","level":"1"}]"#);
        let home = std::env::temp_dir().join(format!("rbms-loading-command-line-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("songs")).expect("the test's folder is made");
        let mut config = Config::default();
        config.library.folders = vec![home.join("songs").to_string_lossy().into_owned()];
        let launch = LaunchOptions { table_url: Some(address.to_string()), ..LaunchOptions::default() };
        rbms_render::font::use_embedded_fonts_only();
        let mut app = App::new(String::new(), config, launch, home.join("settings.ron"));

        assert!(matches!(app.stage, Stage::Loading(_)), "an app with a folder to read starts by reading it");
        loop {
            match app.stage.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }) {
                Transition::Stay => std::thread::yield_now(),
                Transition::Back => break,
                _ => panic!("the scan the app starts with hands on to the song list"),
            }
        }
        assert_eq!(tables.asked(), 0, "the scan at startup asked the address named on the command line");
        assert_eq!(crate::tablesrc::table_status(address), Some(crate::tablesrc::TableStatus::NotFetched));

        let mut select = SelectState::new();
        select.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
        landed(&mut app, &mut select);
        assert_eq!((tables.asked(), app.shared.table_names.clone()), (1, vec!["NAMED".to_string()]));

        crate::tablesrc::forget_table_status(address);
        drop(app);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// Escape during the table step throws nothing away: the step reads a few files and is over
    /// by itself, and what it read -- the library that was just walked, and every table that has a
    /// copy -- is what the song list opens on. Leaving there used to cost the run its tables.
    #[test]
    fn escape_during_the_table_step_keeps_what_the_scan_read() {
        let mut app = crate::stage::render_tests::app();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut state = LoadingState { task: LoadingTask::Scan { step: ScanStep::Matching { rx }, progress: Arc::new(ScanProgress::default()) }, drawn: false };
        let escape = KeyInput { code: KeyCode::Escape, pressed: true, released: false, text: None };
        let mut ctx = FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 };
        assert!(matches!(state.handle_key(&mut ctx, escape), Transition::Stay), "escape left the table step, and its tables behind");
        assert!(matches!(state.update(&mut ctx), Transition::Stay));

        tx.send(ScanOutcome { library: Library::default(), names: vec!["READ".to_string()], levels: vec![vec![("1".to_string(), Vec::new())]] })
            .expect("the screen is still waiting for the step");
        assert!(matches!(state.update(&mut ctx), Transition::Back), "the step did not hand on to the song list when it finished");
        assert_eq!(app.shared.table_names, vec!["READ".to_string()], "the tables the step read were thrown away");
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
        let db = std::env::temp_dir().join(format!("rbms-step-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&db);
        let walking = LoadingState::scan(Vec::new(), Vec::new(), db.clone(), false);
        assert_eq!(walking.heading(&app.shared).0, "SCANNING");
        assert_eq!(walking.heading(&app.shared).1, "0 folder(s)");

        let progress = Arc::new(ScanProgress::default());
        let matching = LoadingState {
            task: LoadingTask::Scan { step: ScanStep::Matching { rx: std::sync::mpsc::channel().1 }, progress: progress.clone() },
            drawn: false,
        };
        let (heading, sub) = matching.heading(&app.shared);
        assert_eq!(heading, "MATCHING TABLES");
        assert_eq!(sub, "0 / 1");
        progress.tables.store(1, Ordering::Relaxed);
        assert_eq!(matching.heading(&app.shared).1, "1 / 1");
        let _ = std::fs::remove_file(&db);
    }
}
