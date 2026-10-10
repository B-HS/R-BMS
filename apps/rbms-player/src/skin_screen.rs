//! What a loaded skin document needs from the player before it can be drawn, and the compiled
//! screens the documents turn into.
//!
//! [`rbms_render::SkinScreen`] draws a document but decodes nothing and runs nothing: it asks the
//! host for pixels, and a frame for whatever the skin's Lua answers. This module supplies both.
//! Images are decoded with the same `image` crate the chart loader already links. The Lua -- a Lua
//! skin's functions, or the scripts a document wrote as strings -- is in the interpreter
//! [`rbms_skin`] loaded the skin into, confined to the skin root and held to the loader's budget.
//!
//! A frame is made in the reference's two stages. The state the screen is drawn from is gathered
//! into one [`ScreenHost`], that host is bound to the interpreter once, and every object of the
//! document is prepared inside that one binding -- which is the only time the skin's Lua runs. The
//! binding ends, and the frame is drawn from what was prepared.
//!
//! [`SkinScreens`] is the other half: one compiled screen per screen type, rebuilt when the document
//! behind it is read again, and the textures those screens draw from.
//!
//! A Lua skin is read on the first frame of the screen it draws rather than when it is chosen,
//! because it builds its screen out of that screen's state: the frame hands the state it is drawn
//! from to the library, which runs the skin against it, and the next frame takes the result in. A
//! skin that cannot be read says why once and leaves the screen to its built-in layout.
//!
//! # Textures
//!
//! A screen takes the image sources its assembled destinations draw from and no others
//! ([`referenced_source_files`]): a published skin declares a sheet for every customisation it
//! offers, and decoding the ones this load never shows costs gigabytes. Those files are decoded on
//! the worker pool, never on the frame loop.
//!
//! The reference reads a skin's images where it reads the skin, and the screen stands still until
//! it has. Here the frame loop keeps running, so two rules stand in for that:
//!
//! - **Nothing of a document is drawn before its files are in.** The screen is compiled on the
//!   frame the last of its files has been read, so an object whose source is still being decoded is
//!   not drawn on that frame or any before it; until then the document draws nothing at all and its
//!   screen's built-in layout stands in.
//! - **A scene does not begin before its document can be drawn.** While the document of the screen
//!   being drawn is on its way -- waiting to be read, or read and having its files decoded -- the
//!   scene clock stands still where it was when the wait began, which is at the scene's first
//!   moment for a screen that was just entered ([`AppShared::skin_is_loading`]). The clock moves
//!   again on the frame the document is compiled, so no part of a timer's animation plays out
//!   before there is a texture to show it with. A wait longer than [`SCENE_HOLD_LIMIT`] is given up
//!   on and the scene runs without its document for as long as that takes; a screen that is nothing
//!   but its document asks whether that has happened ([`AppShared::skin_wait_is_spent`]) and moves
//!   on.
//!
//! Textures are shared by file and counted ([`SkinTexturePool`]). A screen that is left --
//! [`Transition::To`](crate::stage::Transition) or a return from it -- gives its textures up at the
//! end of the first frame of the screen that follows, and they are freed once that screen has said
//! which files it shares with it, so a file two screens in a row draw from is uploaded once. A
//! screen with another opened over it keeps its textures while it is parked and nothing but opened
//! screens stand over it -- the settings and back. The first scene begun over it by
//! [`Transition::To`](crate::stage::Transition) ends that: a chart picked in the browser leaves the
//! browser parked under the decide scene, and the run that follows is a screen change like any
//! other, as it is in the reference, where every change of screen disposes of the skin it leaves.
//! So the browser's textures are gone while the chart is played and are read again on the way back.
//! A file is decoded again only when it was written since it was uploaded, which a worker finds out
//! with one stat.
//!
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rbms_config::SkinCustomisation;
use rbms_render::font::with_text_context;
use rbms_render::result::{ResultExtras, ResultView};
use rbms_render::skin_render::PreparedFrame;
use rbms_render::skin_render::state::{DecideChart, DecideViewState, KeyConfigViewState, PlayViewState, ResultViewState, SelectViewState};
use rbms_render::skin_render::textures::{SkinTexturePool, TextureStats, referenced_source_files};
use rbms_render::{
    BgaFrame, Color, FrameData, PlayTimers, Renderer, ResultTimers, SelectTimers, SkinAssets, SkinFrame, SkinImage, SkinScreen, SongBars, TextContext,
    TextureId, with_render_ctx,
};
use rbms_skin::dst::{LuaDrawEval, OffsetSource, SkinOffset};
use rbms_skin::loader::{LoadedSkin, SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_RESULT};
use rbms_skin::property::{SkinHost, StaticScreen};
use rbms_skin::timer::TimerState;

use crate::assets::{DecodePool, FileStamp, SkinAsset, SkinAssetJob, SkinAssetKind, SkinAssetRead, SkinAssetRequest, spawn_skin_asset_decode};
use crate::notify::{Level, notify};
use crate::skin_host::ScreenHost;
use crate::skin_host::chart::{ChartMeta, ChartState};
use crate::skin_host::loading::{LoadingScreen, LoadingState};
use crate::skin_host::system::{CourseStage, SystemState, Volumes};
use crate::skin_select::{SkinLibrary, SkinRead};
use crate::stage::Canvas;
use crate::stage::canvas::UI_SIZE;
use crate::{AppShared, SelectScene};

/// Hands one document's already-decoded files to [`SkinScreen::build`].
///
/// Nothing here touches the disk. A published skin draws from dozens of source images and a font or
/// two, and decoding them is seconds of work; that happens on the worker pool before the screen is
/// built at all ([`PendingScreen`]), so the frame the document first draws on costs a texture upload
/// and nothing else. An image that is not here is one the screen draws from the texture already
/// uploaded for its file, or goes without. Held only for the length of one build: everything it
/// produces is owned by the screen or the texture pool afterwards.
pub(crate) struct PlayerSkinAssets {
    prepared: BTreeMap<SkinAssetJob, SkinAsset>,
}

impl PlayerSkinAssets {
    /// The assets one document is compiled with: the files a worker read.
    pub(crate) fn new(prepared: BTreeMap<SkinAssetJob, SkinAsset>) -> PlayerSkinAssets {
        PlayerSkinAssets { prepared }
    }
}

impl SkinAssets for PlayerSkinAssets {
    fn image(&mut self, path: &Path) -> Option<SkinImage> {
        match self.prepared.remove(&(SkinAssetKind::Image, path.to_path_buf())) {
            Some(SkinAsset::Image(image)) => Some(image),
            _ => None,
        }
    }

    fn font(&mut self, path: &Path) -> Option<Vec<u8>> {
        match self.prepared.remove(&(SkinAssetKind::Font, path.to_path_buf())) {
            Some(SkinAsset::Font(bytes)) => Some(bytes),
            _ => None,
        }
    }
}

/// One compiled screen and the read of its document it was built from.
struct BuiltScreen {
    screen: SkinScreen,
    build: u64,
}

/// What follows the reason a document could not be read, in the one message that says so.
const SKIN_FALLBACK_NOTE: &str = " - drawing the built-in screen";

/// The nudges one document is drawn with: what the player chose inside it, or nothing at all for a
/// document nobody has customised.
///
/// Borrowed from the configuration rather than copied, because a frame reads a handful of ids and
/// the stored choices already answer them.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DocumentOffsets<'a> {
    document: Option<&'a SkinCustomisation>,
}

impl OffsetSource for DocumentOffsets<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.document.and_then(|stored| stored.offset(id))
    }
}

/// What a caller brings to one frame of a document beyond the state its properties are read from:
/// the nudges it is drawn with, the state no property id can hold, the size of what it is drawn on,
/// and the clusters of properties only that screen can fill in.
///
/// One value rather than five arguments because every screen passes them all and they are borrowed
/// from the caller's own frame: keeping them together is what lets the borrow be written once.
struct FrameInputs<'a> {
    offsets: &'a DocumentOffsets<'a>,
    data: FrameData<'a>,
    /// The target's size in its own pixels, which is what a skin's Lua is told the window measures.
    window: (u32, u32),
    /// The chart in hand, for the screens that have one to say.
    chart: ChartState<'a>,
    /// The load in progress, for the screens that connect it.
    loading: LoadingState,
}

impl<'a> FrameInputs<'a> {
    /// The inputs of a screen that connects neither a chart nor a load.
    fn new(offsets: &'a DocumentOffsets<'a>, data: FrameData<'a>, window: (u32, u32)) -> FrameInputs<'a> {
        FrameInputs { offsets, data, window, chart: ChartState::default(), loading: LoadingState::default() }
    }
}

/// What the decide screen brings to one frame of its document.
pub(crate) struct DecideDraw<'a> {
    /// The chart that was picked.
    pub(crate) chart: &'a ChartMeta<'a>,
    /// How much of the chart's files are in, from nothing (0) to everything (1).
    pub(crate) progress: f32,
    /// What no property id carries: the chart's stage image and the series its graphs plot.
    pub(crate) data: FrameData<'a>,
}

/// The version the skin's version text shows.
const SKIN_VERSION: &str = concat!("R-BMS ", env!("CARGO_PKG_VERSION"));

/// One frame of a document with its first stage behind it: every object prepared, the skin's Lua
/// asked everything this frame will ask of it, and nothing drawn yet.
struct PreparedDocument<'a> {
    screen: &'a SkinScreen,
    /// The frame the objects were prepared against, with no interpreter in it: drawing reads what
    /// the skin's Lua answered out of `prepared`.
    frame: SkinFrame<'a>,
    prepared: PreparedFrame,
}

impl PreparedDocument<'_> {
    /// The second stage: draws what was prepared onto `r`.
    fn draw<R: Renderer>(&self, r: &mut R) {
        with_render_ctx(|ctx| self.screen.draw_prepared(ctx, r, &self.frame, &self.prepared));
    }

    /// The second stage on the target's own pixels ([`Canvas::native`]), over a cleared frame, so
    /// a document's rectangles are scaled once, from the size it was authored at straight onto the
    /// viewport, and whatever it compares with an image's own size is compared in real pixels.
    fn draw_native(&self, canvas: &mut Canvas<'_>) {
        let mut target = canvas.native();
        target.clear(Color::BLACK);
        self.draw(&mut target);
    }
}

/// Which options the reference settles once on one screen rather than on every frame.
fn static_screen_of(screen: i32) -> StaticScreen {
    match screen {
        SKIN_TYPE_MUSIC_SELECT => StaticScreen::Select,
        SKIN_TYPE_RESULT | SKIN_TYPE_COURSE_RESULT => StaticScreen::Result,
        _ => StaticScreen::Other,
    }
}

/// The window size a skin's Lua is told, which the host contract carries as signed pixels.
fn window_size(size: (u32, u32)) -> (i32, i32) {
    (i32::try_from(size.0).unwrap_or(i32::MAX), i32::try_from(size.1).unwrap_or(i32::MAX))
}

/// The browser's frame state: the rows the browser measured, with whether the option panel is open
/// joined on.
///
/// The panel is the application's rather than something the browser measured, so its state is
/// attached here -- beside the offsets and the timers, which are the application's too -- rather
/// than travelling through the browser's own view.
fn select_list(view: &SelectScene, options_open: bool) -> SongBars<'_> {
    SongBars { rows: &view.rows, sel: view.sel, options_open }
}

/// How long a scene is held still for a document that is on its way before it is given up on and
/// runs without it. Far longer than the largest screen of a published skin takes to decode, so it is
/// only ever reached by a read that has gone wrong.
const SCENE_HOLD_LIMIT: Duration = Duration::from_secs(10);

/// How far past its limit a test makes a wait look.
#[cfg(test)]
const WAIT_OVERRUN: Duration = Duration::from_secs(1);

/// One document whose files are still being read off the frame loop.
///
/// The screen it belongs to keeps drawing its built-in layout until every file has arrived, which
/// is the whole point: a published skin is dozens of source images, some of them several thousand
/// pixels square, and decoding those between two frames is a visible stall at the exact moment a
/// chart starts.
struct PendingScreen {
    build: u64,
    pool: DecodePool<SkinAssetJob, SkinAssetRead>,
    ready: BTreeMap<SkinAssetJob, SkinAsset>,
    /// The stamp each image file had when a worker looked at it.
    stamps: BTreeMap<PathBuf, FileStamp>,
    /// The files this document draws from that were uploaded already when it was asked for, each
    /// held so that letting go of the screen that had it does not free it before this one is built.
    pins: Vec<PathBuf>,
    cancel: Arc<AtomicBool>,
}

impl PendingScreen {
    /// Starts reading every file `document` draws from: the fonts it names, and the image sources
    /// its objects reference -- not the ones it only declares.
    ///
    /// A file that is uploaded already is held where it is, and a worker is only asked whether it
    /// has been written since ([`SkinAsset::Unchanged`]).
    fn start(build: u64, document: &LoadedSkin, textures: &mut SkinTexturePool, stamps: &BTreeMap<PathBuf, FileStamp>) -> PendingScreen {
        let mut pins = Vec::new();
        let mut requests: Vec<SkinAssetRequest> = Vec::new();
        for path in referenced_source_files(document) {
            let uploaded = textures.hold(path);
            if uploaded {
                pins.push(path.to_path_buf());
            }
            let known = if uploaded || textures.is_refused(path) { stamps.get(path).copied() } else { None };
            requests.push(((SkinAssetKind::Image, path.to_path_buf()), known));
        }
        requests.extend(document.fonts.values().map(|path| ((SkinAssetKind::Font, path.clone()), None)));
        let cancel = Arc::new(AtomicBool::new(false));
        let pool = spawn_skin_asset_decode(requests, Arc::clone(&cancel));
        PendingScreen { build, pool, ready: BTreeMap::new(), stamps: BTreeMap::new(), pins, cancel }
    }

    /// Collects whatever has arrived, answering whether every job is now accounted for.
    ///
    /// The counter is read before the channel is drained: a worker sends its result and only then
    /// counts the job, so a count that has reached the total means everything sent is already
    /// waiting to be taken.
    ///
    /// A pool whose workers have all gone is accounted for whatever it counted. A worker that died
    /// on a file never counts it or the files queued behind it, and nothing more is coming: the
    /// document is compiled without them, each with a line in its warnings, rather than waited for
    /// for ever.
    fn poll(&mut self) -> bool {
        let (received, progress, total) = &self.pool;
        let counted = progress.load(Ordering::Relaxed) >= *total;
        loop {
            match received.try_recv() {
                Ok((job, read)) => {
                    if let Some(stamp) = read.stamp {
                        self.stamps.insert(job.1.clone(), stamp);
                    }
                    if !matches!(read.asset, SkinAsset::Unchanged) {
                        self.ready.insert(job, read.asset);
                    }
                }
                Err(TryRecvError::Empty) => return counted,
                Err(TryRecvError::Disconnected) => return true,
            }
        }
    }

    /// Gives back the hold on every file that was pinned when the document was asked for.
    fn unpin(&mut self, textures: &mut SkinTexturePool) {
        for path in self.pins.drain(..) {
            textures.release(&path);
        }
    }

    /// Stops the workers and lets go of what was pinned, for a document nobody is waiting for any
    /// more.
    fn abandon(mut self, textures: &mut SkinTexturePool) {
        self.cancel.store(true, Ordering::Relaxed);
        self.unpin(textures);
    }
}

/// A scene standing still while the document it is drawn with is on its way.
#[derive(Debug, Clone, Copy)]
struct SceneHold {
    /// The screen whose document is being waited for.
    screen: i32,
    /// What the scene clock read when the wait began, which is what it reads until the wait ends.
    at: Duration,
    /// When the wait began.
    since: Instant,
}

/// The documents that have been compiled into screens, one per screen type, and the textures they
/// draw from.
///
/// A screen is compiled for as long as it is being drawn or is parked under nothing but screens
/// opened over it. The frame after its scene is left, or after a scene is begun over it while it is
/// parked, it is let go of, and its textures are freed once the screen that follows has taken hold
/// of the files the two share (the module's own notes say how).
#[derive(Default)]
pub(crate) struct SkinScreens {
    built: BTreeMap<i32, BuiltScreen>,
    pending: BTreeMap<i32, PendingScreen>,
    /// The texture of every image file a compiled screen draws from, one per file.
    textures: SkinTexturePool,
    /// The stamp of the version of each file that is uploaded, which is what a worker compares the
    /// file against before decoding it again.
    stamps: BTreeMap<PathBuf, FileStamp>,
    /// The screens asked for since a frame last ended.
    prepared: BTreeSet<i32>,
    /// The screens the frame before that asked for: what the scene being left was drawn with.
    last_prepared: BTreeSet<i32>,
    /// The screens parked under another that was opened over them, each for as long as the scene it
    /// was parked with exists and no scene has been begun over it.
    parked: Vec<(i32, Weak<()>)>,
    /// Whether a scene began, was parked or was put back since the screens nobody draws any more
    /// were last let go of.
    scene_moved: bool,
    /// The wait the running scene is standing still for.
    hold: Option<SceneHold>,
    /// Whether the running scene already waited its limit out, and is not held again.
    hold_spent: bool,
}

impl SkinScreens {
    /// Nothing compiled yet.
    pub(crate) fn new() -> SkinScreens {
        SkinScreens::default()
    }

    /// Move the document one screen is drawn with one step towards being drawable.
    ///
    /// The first call for a read starts its files decoding on the worker pool and returns; later
    /// calls collect what has arrived, and the one that finds everything present compiles the
    /// screen. Until then [`SkinScreens::get`] answers `None` and the built-in layout draws, so no
    /// frame ever waits on a file and no object is drawn before the image it draws from is in.
    ///
    /// The screen compiled from an earlier read is let go of first, but its textures are not freed
    /// until the new read has taken hold of the files it shares with it: a skin read again for a
    /// new scene decodes nothing it already had uploaded.
    pub(crate) fn sync<R: Renderer>(&mut self, r: &mut R, text: &mut TextContext, screen: i32, skins: &SkinLibrary) {
        let wanted = skins.build_of(screen);
        let held = self.built.get(&screen).map(|entry| entry.build).or_else(|| self.pending.get(&screen).map(|entry| entry.build));
        if wanted != held {
            self.let_go(r, screen);
            let (Some(build), Some(document)) = (wanted, skins.document(screen)) else {
                return;
            };
            self.pending.insert(screen, PendingScreen::start(build, document, &mut self.textures, &self.stamps));
            self.sweep(r);
        }

        let Some(pending) = self.pending.get_mut(&screen) else {
            return;
        };
        if !pending.poll() {
            return;
        }
        let Some(mut pending) = self.pending.remove(&screen) else {
            return;
        };
        let Some(document) = skins.document(screen) else {
            pending.abandon(&mut self.textures);
            return;
        };
        let mut assets = PlayerSkinAssets::new(std::mem::take(&mut pending.ready));
        let compiled = SkinScreen::build_shared(r, text, document, &mut assets, &mut self.textures);
        self.stamps.append(&mut pending.stamps);
        pending.unpin(&mut self.textures);
        self.sweep(r);
        if let Some(first) = compiled.warnings().first() {
            let rest = compiled.warnings().len() - 1;
            let more = if rest > 0 { format!(" (and {rest} more)") } else { String::new() };
            notify(Level::Warn, format!("skin: {first}{more}"));
        }
        self.built.insert(screen, BuiltScreen { screen: compiled, build: pending.build });
    }

    /// The compiled screen one screen type is drawn with, or `None` while the built-in screen draws.
    pub(crate) fn get(&self, screen: i32) -> Option<&SkinScreen> {
        self.built.get(&screen).map(|entry| &entry.screen)
    }

    /// Let go of one screen's compiled document and of whatever was still being read for it. The
    /// textures it held stay uploaded until the next [`SkinScreens::sweep`].
    fn let_go<R: Renderer>(&mut self, r: &mut R, screen: i32) {
        if let Some(mut stale) = self.built.remove(&screen) {
            stale.screen.release_shared(r, &mut self.textures);
        }
        if let Some(stale) = self.pending.remove(&screen) {
            stale.abandon(&mut self.textures);
        }
    }

    /// Free every texture no screen, and no document on its way to being one, holds any more.
    fn sweep<R: Renderer>(&mut self, r: &mut R) {
        self.textures.sweep(r);
        let textures = &self.textures;
        self.stamps.retain(|path, _| textures.contains(path) || textures.is_refused(path));
    }

    /// Let go of every screen compiled from a document the library no longer holds a read of: the
    /// textures of the ones that were built, and the workers of the ones still being read.
    ///
    /// [`SkinScreens::sync`] does this for the screen being drawn. This reaches the others, so a
    /// document the player has left behind does not stay uploaded until its screen is next opened.
    pub(crate) fn release_dropped<R: Renderer>(&mut self, r: &mut R, skins: &SkinLibrary) {
        let dropped: BTreeSet<i32> = self.built.keys().chain(self.pending.keys()).copied().filter(|screen| skins.build_of(*screen).is_none()).collect();
        for screen in dropped {
            self.let_go(r, screen);
        }
    }

    /// Note that a frame asked for one screen's document, which is what keeps it compiled when the
    /// frame ends.
    fn mark_prepared(&mut self, screen: i32) {
        self.prepared.insert(screen);
    }

    /// Note that the running scene was left, parked or put back, so the frame that follows lets go
    /// of whatever it no longer draws.
    fn scene_moved(&mut self) {
        self.scene_moved = true;
        self.hold = None;
        self.hold_spent = false;
    }

    /// Park the screens the running scene was drawn with for as long as `scene` exists.
    fn park(&mut self, scene: &Arc<()>) {
        let drawn: Vec<i32> = self.last_prepared.union(&self.prepared).copied().collect();
        self.parked.extend(drawn.into_iter().map(|screen| (screen, Arc::downgrade(scene))));
    }

    /// Stop keeping every parked screen compiled: a scene was begun over them, which is a change of
    /// screen and not a screen opened to be closed again. The frame that follows lets go of them
    /// with whatever else it does not draw, and their scenes read them again when they are put back.
    fn unpark(&mut self) {
        self.parked.clear();
    }

    /// End a frame: once after a scene has moved, let go of every screen the frame did not ask for
    /// and that is not parked under another; then free the textures nobody holds.
    ///
    /// The textures are left where they are while a screen the frame asked for is still waiting for
    /// its Lua skin to be read, because that skin has not yet said which of them it draws from too.
    pub(crate) fn finish_frame<R: Renderer>(&mut self, r: &mut R, skins: &SkinLibrary) {
        let prepared = std::mem::take(&mut self.prepared);
        if self.scene_moved {
            self.scene_moved = false;
            self.parked.retain(|(_, scene)| scene.strong_count() > 0);
            let parked = &self.parked;
            let left: BTreeSet<i32> = self
                .built
                .keys()
                .chain(self.pending.keys())
                .copied()
                .filter(|screen| !prepared.contains(screen) && !parked.iter().any(|(kept, _)| kept == screen))
                .collect();
            for screen in left {
                self.let_go(r, screen);
            }
        }
        if !prepared.iter().any(|screen| skins.is_waiting(*screen)) {
            self.sweep(r);
        }
        self.last_prepared = prepared;
    }

    /// How many textures the compiled screens have uploaded between them, and their RGBA bytes.
    pub(crate) fn texture_stats(&self) -> TextureStats {
        self.textures.stats()
    }

    /// Whether one screen's document is still having its files read.
    pub(crate) fn is_pending(&self, screen: i32) -> bool {
        self.pending.contains_key(&screen)
    }

    /// Everything one screen's document reported while it compiled, for the tests that say why a
    /// fixture document never finished compiling.
    #[cfg(test)]
    fn warnings(&self, screen: i32) -> &[String] {
        match self.get(screen) {
            Some(screen) => screen.warnings(),
            None => &[],
        }
    }
}

/// Where the pointer is in a document's own coordinates: scaled onto the size it was authored at,
/// and measured up from the bottom the way a document measures everything.
///
/// `cursor` is a position in a `screen`-sized space laid over the viewport the frame lands in. Only
/// the share of that space it has crossed matters, so the answer is the same whichever space the
/// position was measured in, and however many pixels the document is drawn on.
fn document_cursor(cursor: (f32, f32), screen: (u32, u32), authored: (f32, f32)) -> Option<(f32, f32)> {
    let (screen_w, screen_h) = (screen.0 as f32, screen.1 as f32);
    if screen_w <= 0.0 || screen_h <= 0.0 {
        return None;
    }
    let (authored_w, authored_h) = authored;
    Some((cursor.0 / screen_w * authored_w, authored_h - cursor.1 / screen_h * authored_h))
}

/// Everything a scene's skin remembers from one frame to the next: which timers are on and since
/// when, what each screen's timer driver last saw, and how long the scene had been running.
///
/// A scene is one stay on one screen. Opening another screen over it parks this value with the
/// screen, and going back puts it back, so the scene carries on from where it was left rather than
/// starting over.
pub(crate) struct SkinScene {
    timers: TimerState,
    play: PlayTimers,
    select: SelectTimers,
    result: ResultTimers,
    elapsed: Duration,
    /// What keeps the screens this scene was drawn with compiled while it is parked: they are held
    /// for as long as this exists, so a parked scene that is dropped rather than put back lets go of
    /// them all the same.
    parked: Arc<()>,
}

impl AppShared {
    /// The clock every document is animated against, in microseconds since the scene began, shared
    /// by its timers and its scripts' `time()`. It is not the song clock: a run starting does not move it.
    ///
    /// It stands still while the document of the screen being drawn is on its way
    /// ([`AppShared::skin_is_loading`]), so a scene's first moment is the first one its skin can be
    /// drawn at.
    pub(crate) fn skin_now_us(&self) -> i64 {
        self.scene_elapsed().as_micros() as i64
    }

    /// How long the running scene has run, not counting any time it was held still for its document.
    fn scene_elapsed(&self) -> Duration {
        self.skin_screens.hold.map_or_else(|| self.scene_started.elapsed(), |hold| hold.at)
    }

    /// Start a scene: every timer off, every timer driver forgetting what it saw, and the scene
    /// clock back at zero (`TimerManager.setMainState`). Every Lua skin read for an earlier scene is
    /// read again the next time its screen is drawn, because it was built from that scene's state.
    ///
    /// The screens parked under the one being left stop being kept compiled. The reference disposes
    /// of a screen's skin on every change of screen (`MainController.changeState`), and a screen
    /// parked here is one the player has left for as long as a run and its result take: its
    /// textures do not stay uploaded beside theirs. Its scene is still put back as it was parked,
    /// and reads its document again then.
    pub(crate) fn begin_skin_scene(&mut self) {
        self.reset_skin_scene();
        self.skins.expire_scripted();
        self.skin_screens.unpark();
    }

    /// Put the timers and the scene clock where a scene starts, and note that the scene moved: the
    /// frame that follows lets go of the screens it no longer draws
    /// ([`AppShared::finish_skin_frame`]).
    fn reset_skin_scene(&mut self) {
        self.skin_timers.clear();
        self.skin_play_timers = PlayTimers::new();
        self.skin_select_timers = SelectTimers::new();
        self.skin_result_timers = ResultTimers::new();
        self.skin_screens.scene_moved();
        self.scene_started = Instant::now();
    }

    /// Take the running scene out to be parked, and begin a fresh one for the screen about to be
    /// opened over it.
    ///
    /// The parked screen keeps the skin it was read with and the textures it was compiled with: it
    /// is drawn again, from the same Lua state, when the screen opened over it is left -- unless a
    /// scene is begun over it in between ([`AppShared::begin_skin_scene`]), which lets go of both.
    pub(crate) fn suspend_skin_scene(&mut self) -> SkinScene {
        let elapsed = self.scene_elapsed();
        let parked = Arc::new(());
        self.skin_screens.park(&parked);
        let scene = SkinScene {
            timers: std::mem::take(&mut self.skin_timers),
            play: self.skin_play_timers,
            select: self.skin_select_timers,
            result: self.skin_result_timers,
            elapsed,
            parked,
        };
        self.reset_skin_scene();
        scene
    }

    /// Put a parked scene back. The clock picks up at the moment it was parked, because it stood
    /// still for as long as the screen opened over it was up. Its screens stop being parked and are
    /// kept by being drawn again -- or read and compiled again, when a scene begun over them let go
    /// of them; the screen that was opened over them is let go of by the frame that follows.
    pub(crate) fn resume_skin_scene(&mut self, scene: SkinScene) {
        let SkinScene { timers, play, select, result, elapsed, parked } = scene;
        self.skin_timers = timers;
        self.skin_play_timers = play;
        self.skin_select_timers = select;
        self.skin_result_timers = result;
        self.skin_screens.scene_moved();
        drop(parked);
        let now = Instant::now();
        self.scene_started = now.checked_sub(elapsed).unwrap_or(now);
    }

    /// Make the running scene look as old as `by` more, for the tests that need a clock with
    /// something on it.
    #[cfg(test)]
    pub(crate) fn age_skin_scene(&mut self, by: Duration) {
        self.scene_started = self.scene_started.checked_sub(by).expect("the process has been up for longer than the age asked for");
    }

    /// Make the wait the running scene is held for look as though it has outlasted its limit, for
    /// the tests of what a screen does once its document has been given up on.
    #[cfg(test)]
    pub(crate) fn outlast_skin_wait(&mut self) {
        let hold = self.skin_screens.hold.as_mut().expect("the scene is held for its document");
        hold.since = Instant::now().checked_sub(SCENE_HOLD_LIMIT + WAIT_OVERRUN).expect("the process has been up for longer than the limit");
    }

    /// Read and compile the document one screen is drawn with, so the gate below has something to
    /// hand over. Cheap on every frame but the one after a document is chosen or reloaded.
    ///
    /// A document no screen is drawn with any more is let go of first, whichever screen it drew: the
    /// pack was changed or taken away, or a settings file arrived that chooses differently.
    ///
    /// A document that is only data is read here. A Lua skin is asked for here and read by the frame
    /// that follows, which is the first place the screen's state exists to read it against
    /// ([`AppShared::with_skin_frame`]); what that frame read is taken in on the next call. A
    /// document that could not be read is reported once and not read again until the player moves a
    /// choice or asks for a reload, so the screen falls back to its built-in layout without stalling.
    ///
    /// Asking is also what keeps the screen compiled: one that a scene's first frame does not ask
    /// for is let go of when that frame ends. And it is where the scene is held still for a
    /// document that is on its way, and set going again on the frame the document is compiled.
    pub(crate) fn prepare_skin(&mut self, canvas: &mut Canvas<'_>, screen: i32) {
        self.skin_screens.mark_prepared(screen);
        self.read_and_compile_skin(canvas, screen);
        self.hold_scene_while_loading(screen);
    }

    /// The reading and compiling half of [`AppShared::prepare_skin`].
    fn read_and_compile_skin(&mut self, canvas: &mut Canvas<'_>, screen: i32) {
        if self.skins.pack_moved(&self.config) {
            self.rescan_skins();
        }
        self.skins.drop_moved(&self.config);
        self.skin_screens.release_dropped(canvas, &self.skins);
        if !self.skin_document_is_enabled(screen) {
            return;
        }
        if self.skins.needs_reload_for(&self.config, screen) {
            self.skins.request_for(&self.config, screen);
        }
        self.skins.adopt(screen);
        if let Some(reason) = self.skins.take_failure(screen) {
            notify(Level::Warn, format!("skin: {reason}{SKIN_FALLBACK_NOTE}"));
        }
        let skins = &self.skins;
        let screens = &mut self.skin_screens;
        with_text_context(|text| screens.sync(canvas, text, screen, skins));
    }

    /// Whether one screen's document is on its way but cannot be drawn yet: a Lua skin waiting for
    /// the screen's first frame to be read on, or a document that was read and whose images and
    /// fonts are still being decoded.
    ///
    /// A screen that waits for its skin before it starts -- the reference waits for every skin --
    /// asks this. A document that failed to read is not loading: it is not coming.
    pub(crate) fn skin_is_loading(&self, screen: i32) -> bool {
        self.skin_document_is_enabled(screen)
            && self.skin_screens.get(screen).is_none()
            && (self.skin_screens.is_pending(screen) || self.skins.is_waiting(screen))
    }

    /// Whether the running scene waited its limit out for `screen`'s document, which is still not
    /// drawable: the scene runs without it, and a screen that has nothing else to draw moves on.
    pub(crate) fn skin_wait_is_spent(&self, screen: i32) -> bool {
        self.skin_screens.hold_spent && self.skin_is_loading(screen)
    }

    /// Hold the scene clock still while `screen`'s document is on its way, and set it going again
    /// once the document is compiled, is known not to be coming, or has been waited for longer
    /// than [`SCENE_HOLD_LIMIT`].
    ///
    /// The clock picks up where it stood, so a timer switched on before the wait began is as old
    /// afterwards as it was before, and a scene that was only just entered begins at its first
    /// moment with its document on screen.
    fn hold_scene_while_loading(&mut self, screen: i32) {
        let loading = self.skin_is_loading(screen);
        match self.skin_screens.hold {
            None if loading && !self.skin_screens.hold_spent => {
                self.skin_screens.hold = Some(SceneHold { screen, at: self.scene_started.elapsed(), since: Instant::now() });
            }
            Some(hold) if hold.screen == screen && (!loading || hold.since.elapsed() > SCENE_HOLD_LIMIT) => {
                self.skin_screens.hold_spent = loading;
                self.release_scene_hold();
            }
            _ => {}
        }
    }

    /// Set a held scene going again from where it stood.
    fn release_scene_hold(&mut self) {
        if let Some(hold) = self.skin_screens.hold.take() {
            let now = Instant::now();
            self.scene_started = now.checked_sub(hold.at).unwrap_or(now);
        }
    }

    /// End a frame of whichever screen is up, after it has drawn.
    ///
    /// The first frame after a scene was left, parked or put back lets go of every compiled screen
    /// it did not ask for and that is not parked under another, and the textures nobody holds any
    /// more are freed -- which is how a screen's textures leave the GPU when the player leaves the
    /// screen, without the screen that follows decoding again what the two share. A scene held for
    /// a screen the frame did not ask for is set going again, so a clock is never left standing for
    /// a document nobody is waiting on.
    pub(crate) fn finish_skin_frame(&mut self, canvas: &mut Canvas<'_>) {
        if self.skin_screens.hold.is_some_and(|hold| !self.skin_screens.prepared.contains(&hold.screen)) {
            self.release_scene_hold();
        }
        self.skin_screens.finish_frame(canvas, &self.skins);
    }

    /// The debug panel's line about skin textures: how many are uploaded and what they come to.
    pub(crate) fn debug_skin_texture_line(&self) -> String {
        let stats = self.skin_screens.texture_stats();
        format!("SKIN TEX {}  {:.1} MB", stats.count, stats.mebibytes())
    }

    /// Whether a document is compiled for one screen, which is what decides whether the built-in
    /// layout's own background slot should be filled at all: a document owns the whole screen, and
    /// an image placed by the built-in layout would sit wherever that layout put it.
    ///
    /// A document whose files are still being read counts as present, so the background does not
    /// move into the built-in slot for the handful of frames before the document takes over. So does
    /// a Lua skin waiting for this screen's frame to be read on, which is also what gets that frame
    /// drawn through the document's path at all.
    pub(crate) fn has_skin_document(&self, screen: i32) -> bool {
        self.skin_document_is_enabled(screen)
            && (self.skin_screens.get(screen).is_some() || self.skin_screens.is_pending(screen) || self.skins.is_waiting(screen))
    }

    /// Whether one screen's document has finished being read and compiled, as against merely being
    /// on its way.
    #[cfg(test)]
    pub(crate) fn has_compiled_skin(&self, screen: i32) -> bool {
        self.skin_screens.get(screen).is_some()
    }

    /// Everything one screen's document reported while it compiled.
    #[cfg(test)]
    pub(crate) fn skin_warnings(&self, screen: i32) -> &[String] {
        self.skin_screens.warnings(screen)
    }

    /// The nudges the player made in the document one screen is drawn with, held by the caller for
    /// the length of a frame.
    pub(crate) fn skin_offsets(&self, screen: i32) -> DocumentOffsets<'_> {
        DocumentOffsets { document: self.skins.document_path(&self.config, screen).and_then(|path| self.config.skin.customisation(path)) }
    }

    /// Makes one frame of a document: gathers the state it is drawn from into one host, prepares
    /// every object against it, and hands the prepared frame to `draw`.
    ///
    /// `None` when no document is compiled for `screen`, which is the caller's cue to draw its own
    /// layout exactly as it always did. `draw` is not called then.
    ///
    /// The host is a [`ScreenHost`]. `adapter` is the state this screen was drawn from before the
    /// host's clusters existed, and it still answers every id no cluster knows, so a document shows
    /// what it showed while the clusters are being filled in. The scene's timers are the host's own:
    /// a script that asks for a timer reads the one its destinations are animated against.
    ///
    /// A skin's Lua -- the functions of a Lua skin, the scripts of a document -- lives in the
    /// interpreter the skin was loaded into, and can only be called while a host is bound to it. The
    /// host is bound exactly once a frame, every object is prepared inside that binding, and the
    /// binding has ended by the time `draw` runs: drawing reads what the skin's Lua answered and
    /// calls nothing. Every call the frame makes is held to that interpreter's frame budget. A
    /// binding that cannot be made leaves the document undrawn for the frame, like a document that
    /// is not there.
    ///
    /// What the skin told the game while it was prepared -- an event, a sound, a value written back
    /// -- stays recorded in the host and goes with it. Nothing carries those out yet.
    ///
    /// The nudges travel in `inputs` rather than being read here because they are borrowed from the
    /// configuration and handed on as a trait object: the caller holds them for the whole frame.
    ///
    /// The pointer is kept in the [`UI_SIZE`] space the window's events are mapped onto, which is
    /// the space it is read back out of here.
    ///
    /// This is also where a Lua skin waiting for this screen is read: the host is the screen's own
    /// state, settled for the frame, which is what the skin's Lua reads while it builds its tables.
    /// The frame that reads the skin draws nothing with it -- its files are not decoded yet.
    fn with_skin_frame<R>(&self, screen: i32, adapter: &dyn SkinHost, inputs: FrameInputs<'_>, draw: impl FnOnce(&PreparedDocument<'_>) -> R) -> Option<R> {
        if !self.skin_document_is_enabled(screen) {
            return None;
        }
        let mut host = ScreenHost::new(adapter.now_us(), &self.skin_timers);
        host.system = self.skin_system_state();
        host.chart = inputs.chart;
        host.loading = inputs.loading;
        host.offsets = Some(inputs.offsets as &dyn OffsetSource);
        host.static_screen = Some(static_screen_of(screen));
        host.window = Some(window_size(inputs.window));
        host.fallback = Some(adapter);
        if self.skins.is_waiting(screen) {
            self.skins.read_waiting(&self.config, screen, SkinRead { host: &host, seed: self.skins.seed() });
            return None;
        }
        let compiled = self.skin_screens.get(screen)?;
        let frame = SkinFrame {
            now_us: host.now_us,
            timers: &self.skin_timers,
            state: &host,
            lua: None,
            mouse: document_cursor(self.cursor, UI_SIZE, compiled.authored_size()),
            data: inputs.data,
        };
        let prepared = match self.skins.document(screen).and_then(LoadedSkin::runtime) {
            Some(runtime) => runtime.frame(&host, |bound| compiled.prepare(&SkinFrame { lua: Some(bound as &dyn LuaDrawEval), ..frame })).ok()?,
            None => compiled.prepare(&frame),
        };
        Some(draw(&PreparedDocument { screen: compiled, frame, prepared }))
    }

    /// What the machine and the player's totals say: the clock, the volumes, the recorded runs and the
    /// course in progress, which every screen's document can read.
    fn skin_system_state(&self) -> SystemState<'_> {
        let unix_seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX));
        let audio = &self.config.audio;
        SystemState {
            clock: rbms_skin::lua::local_time(unix_seconds),
            uptime_ms: Some(u64::try_from(self.booted.elapsed().as_millis()).unwrap_or(u64::MAX)),
            fps: Some(self.fps),
            player_name: Some(&self.config.network.player_id),
            version: Some(SKIN_VERSION),
            history: Some(self.scores.records()),
            volumes: Some(Volumes { system: audio.system, key: audio.key, background: audio.bg }),
            course: self.course_run.as_ref().map(|run| CourseStage { index: run.index, count: run.course.stage_count() }),
        }
    }

    /// Whether one screen has a document to be drawn with -- the player chose one, or the skin pack
    /// holds one for it -- which is all it takes for that document to draw the screen alone.
    fn skin_document_is_enabled(&self, screen: i32) -> bool {
        self.skins.document_path(&self.config, screen).is_some()
    }

    /// Why one screen's document could not be read, for the tests and the capture harness that say
    /// what became of a skin.
    #[cfg(test)]
    pub(crate) fn skin_failure(&self, screen: i32) -> Option<&str> {
        self.skins.failure(screen)
    }

    /// Draw the play screen's document. `true` when it drew, and the built-in field stands aside.
    ///
    /// The one document still drawn on the [`UI_SIZE`] screen rather than on the target's own
    /// pixels: its note field and covers take their rows from the built-in field's geometry, which
    /// is measured in that space, so the rest of the document has to be laid out in it too.
    pub(crate) fn draw_play_skin(&self, canvas: &mut Canvas<'_>, screen: i32, state: &PlayViewState<'_>, data: FrameData<'_>) -> bool {
        let offsets = self.skin_offsets(screen);
        let window = canvas.native().size();
        self.with_skin_frame(screen, state, FrameInputs::new(&offsets, data, window), |document| {
            canvas.clear(Color::BLACK);
            document.draw(canvas);
        })
        .is_some()
    }

    /// Draw the song browser's document, with the browser's rows joined on so a document draws its
    /// wheel without the stage assembling it.
    ///
    /// This and the screens below it draw on the target's own pixels
    /// ([`PreparedDocument::draw_native`]).
    pub(crate) fn draw_select_skin(&self, canvas: &mut Canvas<'_>, view: &SelectScene, background: Option<TextureId>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_MUSIC_SELECT);
        let options_open = self.options.is_open();
        let own = select_list(view, options_open);
        let state = SelectViewState::new(view, self.skin_now_us(), Some(&offsets), options_open);
        let data = FrameData { bars: Some(&own), bga: BgaFrame::of(background), ..FrameData::default() };
        let inputs = FrameInputs::new(&offsets, data, canvas.native().size());
        self.with_skin_frame(SKIN_TYPE_MUSIC_SELECT, &state, inputs, |document| document.draw_native(canvas)).is_some()
    }

    /// Draw the score screen's document.
    pub(crate) fn draw_result_skin(&self, canvas: &mut Canvas<'_>, view: &ResultView, extras: &ResultExtras, cleared: bool, data: FrameData<'_>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_RESULT);
        let state = ResultViewState::new(view, extras.target.as_ref(), cleared, self.skin_now_us(), Some(&offsets));
        let inputs = FrameInputs::new(&offsets, data, canvas.native().size());
        self.with_skin_frame(SKIN_TYPE_RESULT, &state, inputs, |document| document.draw_native(canvas)).is_some()
    }

    /// Draw the decide screen's document.
    ///
    /// The chart cluster answers for the chart and the loading cluster for how much of it is in. The
    /// decide screen is not the player screen, so neither loading option is on, as on the reference's
    /// decide screen. The state the screen was drawn from before the clusters existed still answers
    /// the ids no cluster knows.
    pub(crate) fn draw_decide_skin(&self, canvas: &mut Canvas<'_>, scene: &DecideDraw<'_>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_DECIDE);
        let chart = scene.chart;
        let older = DecideChart { subtitle: chart.subtitle, artist: chart.artist, genre: chart.genre, level: chart.level, difficulty: chart.difficulty };
        let state =
            DecideViewState { progress: scene.progress, done: false, title: chart.title, chart: older, now_us: self.skin_now_us(), offsets: Some(&offsets) };
        let loading = LoadingState { screen: LoadingScreen::Elsewhere, progress: scene.progress };
        let inputs = FrameInputs { chart: ChartState::Chart(chart), loading, ..FrameInputs::new(&offsets, scene.data, canvas.native().size()) };
        self.with_skin_frame(SKIN_TYPE_DECIDE, &state, inputs, |document| document.draw_native(canvas)).is_some()
    }

    /// Draw the key configuration screen's document.
    pub(crate) fn draw_keyconfig_skin(&self, canvas: &mut Canvas<'_>, keys: &[String]) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_KEY_CONFIG);
        let state = KeyConfigViewState { keys, now_us: self.skin_now_us(), offsets: Some(&offsets) };
        let inputs = FrameInputs::new(&offsets, FrameData::default(), canvas.native().size());
        self.with_skin_frame(SKIN_TYPE_KEY_CONFIG, &state, inputs, |document| document.draw_native(canvas)).is_some()
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod texture_tests;
