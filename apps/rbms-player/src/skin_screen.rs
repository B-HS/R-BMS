//! What a loaded skin document needs from the player before it can be drawn, and the compiled
//! screens the documents turn into.
//!
//! [`rbms_render::SkinScreen`] draws a document but decodes nothing and runs nothing: it asks the
//! host for pixels, and a frame for whatever the skin's Lua answers. This module supplies both.
//! Images are decoded with the same `image` crate the chart loader already links. The Lua -- a Lua
//! skin's functions, or the scripts a document wrote as strings -- is in the interpreter
//! [`rbms_skin`] loaded the skin into, confined to the skin root and held to the loader's budget;
//! each frame binds the state it is drawn from to that interpreter once.
//!
//! [`SkinScreens`] is the other half: one compiled screen per screen type, rebuilt when the document
//! behind it is read again. A rebuild registers a fresh set of textures, so the previous screen is
//! always released first.
//!
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rbms_config::SkinCustomisation;
use rbms_render::font::with_text_context;
use rbms_render::result::{ResultExtras, ResultView};
use rbms_render::skin_render::state::{DecideChart, DecideViewState, KeyConfigViewState, PlayViewState, ResultViewState, SelectViewState};
use rbms_render::{
    Color, FrameExtra, PlayTimers, Renderer, ResultTimers, SelectListState, SelectTimers, SkinAssets, SkinDraw, SkinImage, SkinScreen, TextContext, TextureId,
    render_decide_screen, render_keyconfig_screen, render_play_screen, render_result_screen, render_select_screen, with_render_ctx,
};
use rbms_skin::dst::{LuaDrawEval, OffsetSource, SkinOffset};
use rbms_skin::loader::{LoadedSkin, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_RESULT};
use rbms_skin::property::SkinHost;
use rbms_skin::timer::TimerState;

use crate::assets::{DecodePool, SkinAsset, SkinAssetJob, SkinAssetKind, spawn_skin_asset_decode};
use crate::notify::{Level, notify};
use crate::skin_select::SkinLibrary;
use crate::stage::Canvas;
use crate::stage::canvas::UI_SIZE;
use crate::{AppShared, SelectScene};

/// Hands one document's already-decoded files to [`SkinScreen::build`].
///
/// Nothing here touches the disk. A published skin names dozens of source images and a font or two,
/// and decoding them is seconds of work; that happens on the worker pool before the screen is built
/// at all ([`PendingScreen`]), so the frame the document first draws on costs a texture upload and
/// nothing else. Held only for the length of one build: everything it produces is owned by the
/// screen afterwards.
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
/// the art behind it, the nudges it is drawn with, and the screen-shaped state no property id can
/// hold.
///
/// One value rather than three arguments because every screen passes all three and two of them are
/// borrowed from the caller's own frame: keeping them together is what lets the borrow be written
/// once.
struct FrameInputs<'a> {
    background: Option<TextureId>,
    offsets: &'a DocumentOffsets<'a>,
    extra: FrameExtra<'a>,
}

/// The browser's frame state: the rows the browser measured, with whether the option panel is open
/// joined on.
///
/// The panel is the application's rather than something the browser measured, so its state is
/// attached here -- beside the offsets and the timers, which are the application's too -- rather
/// than travelling through the browser's own view.
fn select_list(view: &SelectScene, options_open: bool) -> SelectListState<'_> {
    SelectListState { rows: &view.rows, sel: view.sel, options_open }
}

/// One document whose files are still being read off the frame loop.
///
/// The screen it belongs to keeps drawing its built-in layout until every file has arrived, which
/// is the whole point: a published skin is dozens of source images, some of them two thousand
/// pixels square, and decoding those between two frames is a visible stall at the exact moment a
/// chart starts.
struct PendingScreen {
    build: u64,
    pool: DecodePool<SkinAssetJob, SkinAsset>,
    ready: BTreeMap<SkinAssetJob, SkinAsset>,
    cancel: Arc<AtomicBool>,
}

impl PendingScreen {
    /// Starts reading every file `document` names.
    fn start(build: u64, document: &LoadedSkin) -> PendingScreen {
        let images = document.sources.values().map(|path| (SkinAssetKind::Image, path.clone()));
        let fonts = document.fonts.values().map(|path| (SkinAssetKind::Font, path.clone()));
        let jobs: Vec<SkinAssetJob> = images.chain(fonts).collect();
        let cancel = Arc::new(AtomicBool::new(false));
        let pool = spawn_skin_asset_decode(jobs, Arc::clone(&cancel));
        PendingScreen { build, pool, ready: BTreeMap::new(), cancel }
    }

    /// Collects whatever has arrived, answering whether every job is now accounted for.
    ///
    /// The counter is read before the channel is drained: a worker sends its result and only then
    /// counts the job, so a count that has reached the total means everything sent is already
    /// waiting to be taken.
    fn poll(&mut self) -> bool {
        let (received, progress, total) = &self.pool;
        let done = progress.load(Ordering::Relaxed) >= *total;
        while let Ok((job, asset)) = received.try_recv() {
            self.ready.insert(job, asset);
        }
        done
    }

    /// Stops the workers, for a document nobody is waiting for any more.
    fn abandon(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// The documents that have been compiled into screens, one per screen type.
///
/// A screen holds registered textures, so this outlives the stage that draws with it: the browser's
/// document stays compiled while a chart is played and is drawn again on the way back, rather than
/// being decoded twice.
#[derive(Default)]
pub(crate) struct SkinScreens {
    built: BTreeMap<i32, BuiltScreen>,
    pending: BTreeMap<i32, PendingScreen>,
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
    /// frame ever waits on a file.
    ///
    /// The previous screen is released before the new one is built, because each build claims a
    /// texture namespace of its own: skipping the release would leave the old set uploaded for the
    /// rest of the session.
    pub(crate) fn sync<R: Renderer>(&mut self, r: &mut R, text: &mut TextContext, screen: i32, skins: &SkinLibrary) {
        let wanted = skins.build_of(screen);
        let held = self.built.get(&screen).map(|entry| entry.build).or_else(|| self.pending.get(&screen).map(|entry| entry.build));
        if wanted != held {
            if let Some(mut stale) = self.built.remove(&screen) {
                stale.screen.release(r);
            }
            if let Some(stale) = self.pending.remove(&screen) {
                stale.abandon();
            }
            let (Some(build), Some(document)) = (wanted, skins.document(screen)) else {
                return;
            };
            self.pending.insert(screen, PendingScreen::start(build, document));
        }

        let Some(pending) = self.pending.get_mut(&screen) else {
            return;
        };
        if !pending.poll() {
            return;
        }
        let Some(pending) = self.pending.remove(&screen) else {
            return;
        };
        let Some(document) = skins.document(screen) else {
            return;
        };
        let mut assets = PlayerSkinAssets::new(pending.ready);
        let compiled = SkinScreen::build(r, text, document, &mut assets);
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
}

impl AppShared {
    /// The clock every document is animated against, in microseconds since the scene began, shared
    /// by its timers and its scripts' `time()`. It is not the song clock: a run starting does not move it.
    pub(crate) fn skin_now_us(&self) -> i64 {
        self.scene_started.elapsed().as_micros() as i64
    }

    /// Start a scene: every timer off, every timer driver forgetting what it saw, and the scene
    /// clock back at zero (`TimerManager.setMainState`).
    pub(crate) fn begin_skin_scene(&mut self) {
        self.skin_timers.clear();
        self.skin_play_timers = PlayTimers::new();
        self.skin_select_timers = SelectTimers::new();
        self.skin_result_timers = ResultTimers::new();
        self.scene_started = Instant::now();
    }

    /// Take the running scene out to be parked, and begin a fresh one for the screen about to be
    /// opened over it.
    pub(crate) fn suspend_skin_scene(&mut self) -> SkinScene {
        let elapsed = self.scene_started.elapsed();
        let scene = SkinScene {
            timers: std::mem::take(&mut self.skin_timers),
            play: self.skin_play_timers,
            select: self.skin_select_timers,
            result: self.skin_result_timers,
            elapsed,
        };
        self.begin_skin_scene();
        scene
    }

    /// Put a parked scene back. The clock picks up at the moment it was parked, because it stood
    /// still for as long as the screen opened over it was up.
    pub(crate) fn resume_skin_scene(&mut self, scene: SkinScene) {
        let SkinScene { timers, play, select, result, elapsed } = scene;
        self.skin_timers = timers;
        self.skin_play_timers = play;
        self.skin_select_timers = select;
        self.skin_result_timers = result;
        let now = Instant::now();
        self.scene_started = now.checked_sub(elapsed).unwrap_or(now);
    }

    /// Make the running scene look as old as `by` more, for the tests that need a clock with
    /// something on it.
    #[cfg(test)]
    pub(crate) fn age_skin_scene(&mut self, by: Duration) {
        self.scene_started = self.scene_started.checked_sub(by).expect("the process has been up for longer than the age asked for");
    }

    /// Read and compile the document one screen is drawn with, so the gate below has something to
    /// hand over. Cheap on every frame but the one after a document is chosen or reloaded.
    pub(crate) fn prepare_skin(&mut self, canvas: &mut Canvas<'_>, screen: i32) {
        if !self.skin_document_is_enabled(screen) {
            return;
        }
        if self.skins.needs_reload_for(&self.config, screen) {
            self.skins.reload_for(&self.config, screen);
        }
        let skins = &self.skins;
        let screens = &mut self.skin_screens;
        with_text_context(|text| screens.sync(canvas, text, screen, skins));
    }

    /// Whether a document is compiled for one screen, which is what decides whether the built-in
    /// layout's own background slot should be filled at all: a document owns the whole screen, and
    /// an image placed by the built-in layout would sit wherever that layout put it.
    ///
    /// A document whose files are still being read counts as present, so the background does not
    /// move into the built-in slot for the handful of frames before the document takes over.
    pub(crate) fn has_skin_document(&self, screen: i32) -> bool {
        self.skin_document_is_enabled(screen) && (self.skin_screens.get(screen).is_some() || self.skin_screens.is_pending(screen))
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
        DocumentOffsets { document: self.config.skin.document(screen).and_then(|path| self.config.skin.customisation(path)) }
    }

    /// Draws one frame of a document: builds everything the frame needs, with `state` answering its
    /// property reads and `extra` carrying the screen-shaped state no property id can hold, and
    /// hands it to `draw`.
    ///
    /// `None` when no document is compiled for `screen`, which is the caller's cue to draw its own
    /// layout exactly as it always did. `draw` is not called then.
    ///
    /// A skin's Lua -- the functions of a Lua skin, the scripts of a document -- lives in the
    /// interpreter the skin was loaded into, and can only be called while a host is bound to it. So
    /// the whole frame is drawn inside one binding, with `state` as the host, and every call the
    /// frame makes is held to that interpreter's frame budget. A binding that cannot be made leaves
    /// the document undrawn for the frame, like a document that is not there.
    ///
    /// The nudges travel in `inputs` rather than being read here because they are borrowed from the
    /// configuration and handed on as a trait object: the caller holds them for the whole frame.
    ///
    /// The pointer is kept in the [`UI_SIZE`] space the window's events are mapped onto, which is
    /// the space it is read back out of here.
    fn with_skin_frame<R>(&self, screen: i32, state: &dyn SkinHost, inputs: FrameInputs<'_>, draw: impl FnOnce(&SkinDraw<'_>) -> R) -> Option<R> {
        if !self.skin_document_is_enabled(screen) {
            return None;
        }
        let compiled = self.skin_screens.get(screen)?;
        let frame = |lua: Option<&dyn LuaDrawEval>| {
            draw(&SkinDraw {
                screen: compiled,
                timers: &self.skin_timers,
                now_us: state.now_us(),
                lua,
                mouse: document_cursor(self.cursor, UI_SIZE, compiled.authored_size()),
                background: inputs.background,
                offsets: Some(inputs.offsets as &dyn OffsetSource),
                extra: inputs.extra,
            })
        };
        match self.skins.document(screen).and_then(LoadedSkin::runtime) {
            Some(runtime) => runtime.frame(state, |bound| frame(Some(bound as &dyn LuaDrawEval))).ok(),
            None => Some(frame(None)),
        }
    }

    /// Whether the player chose a document for one screen, which is all it takes for that document
    /// to draw the screen alone.
    fn skin_document_is_enabled(&self, screen: i32) -> bool {
        self.config.skin.document(screen).is_some()
    }

    /// Draw the play screen's document. `true` when it drew, and the built-in field stands aside.
    ///
    /// The one document still drawn on the [`UI_SIZE`] screen rather than on the target's own
    /// pixels: its note field and covers take their rows from the built-in field's geometry, which
    /// is measured in that space, so the rest of the document has to be laid out in it too.
    pub(crate) fn draw_play_skin(
        &self,
        canvas: &mut Canvas<'_>,
        screen: i32,
        state: &PlayViewState<'_>,
        background: Option<TextureId>,
        extra: FrameExtra<'_>,
    ) -> bool {
        let offsets = self.skin_offsets(screen);
        self.with_skin_frame(screen, state, FrameInputs { background, offsets: &offsets, extra }, |document| {
            canvas.clear(Color::BLACK);
            with_render_ctx(|ctx| render_play_screen(ctx, canvas, Some(document), state))
        })
        .unwrap_or(false)
    }

    /// Draw the song browser's document, with the browser's rows joined on so a document draws its
    /// wheel without the stage assembling it.
    ///
    /// This and the screens below it draw on the target's own pixels ([`Canvas::native`]), so a
    /// document's rectangles are scaled once, from the size it was authored at straight onto the
    /// viewport, and whatever it compares with an image's own size is compared in real pixels.
    pub(crate) fn draw_select_skin(&self, canvas: &mut Canvas<'_>, view: &SelectScene, background: Option<TextureId>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_MUSIC_SELECT);
        let options_open = self.options.is_open();
        let own = select_list(view, options_open);
        let state = SelectViewState::new(view, self.skin_now_us(), Some(&offsets), options_open);
        let inputs = FrameInputs { background, offsets: &offsets, extra: FrameExtra::Select(&own) };
        self.with_skin_frame(SKIN_TYPE_MUSIC_SELECT, &state, inputs, |document| {
            let mut target = canvas.native();
            target.clear(Color::BLACK);
            with_render_ctx(|ctx| render_select_screen(ctx, &mut target, Some(document), view))
        })
        .unwrap_or(false)
    }

    /// Draw the score screen's document.
    pub(crate) fn draw_result_skin(&self, canvas: &mut Canvas<'_>, view: &ResultView, extras: &ResultExtras, cleared: bool, extra: FrameExtra<'_>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_RESULT);
        let state = ResultViewState::new(view, extras.target.as_ref(), cleared, self.skin_now_us(), Some(&offsets));
        self.with_skin_frame(SKIN_TYPE_RESULT, &state, FrameInputs { background: None, offsets: &offsets, extra }, |document| {
            let mut target = canvas.native();
            target.clear(Color::BLACK);
            with_render_ctx(|ctx| render_result_screen(ctx, &mut target, Some(document), view, extras.target.as_ref(), cleared))
        })
        .unwrap_or(false)
    }

    /// Draw the loading screen's document, which stands in for the reference's decide screen.
    pub(crate) fn draw_decide_skin(&self, canvas: &mut Canvas<'_>, progress: f32, done: bool, title: &str, chart: DecideChart<'_>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_DECIDE);
        let state = DecideViewState { progress, done, title, chart, now_us: self.skin_now_us(), offsets: Some(&offsets) };
        self.with_skin_frame(SKIN_TYPE_DECIDE, &state, FrameInputs { background: None, offsets: &offsets, extra: FrameExtra::None }, |document| {
            let mut target = canvas.native();
            target.clear(Color::BLACK);
            with_render_ctx(|ctx| render_decide_screen(ctx, &mut target, Some(document), &state))
        })
        .unwrap_or(false)
    }

    /// Draw the key configuration screen's document.
    pub(crate) fn draw_keyconfig_skin(&self, canvas: &mut Canvas<'_>, keys: &[String]) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_KEY_CONFIG);
        let state = KeyConfigViewState { keys, now_us: self.skin_now_us(), offsets: Some(&offsets) };
        self.with_skin_frame(SKIN_TYPE_KEY_CONFIG, &state, FrameInputs { background: None, offsets: &offsets, extra: FrameExtra::None }, |document| {
            let mut target = canvas.native();
            target.clear(Color::BLACK);
            with_render_ctx(|ctx| render_keyconfig_screen(ctx, &mut target, Some(document), keys))
        })
        .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests;
