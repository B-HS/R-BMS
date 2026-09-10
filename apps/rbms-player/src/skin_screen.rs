//! What a loaded skin document needs from the player before it can be drawn, and the compiled
//! screens the documents turn into.
//!
//! [`rbms_render::SkinScreen`] draws a document but decodes nothing and runs nothing: it asks the
//! host for pixels and for compiled expressions. This module answers both. Images are decoded with
//! the same `image` crate the chart loader already links, and expressions are compiled into the
//! sandbox [`rbms_skin`] built for that document, so a skin's Lua is confined to the same instance
//! that was created inside the skin root with the loader's own budget.
//!
//! [`SkinScreens`] is the other half: one compiled screen per screen type, rebuilt when the document
//! behind it is read again. A rebuild registers a fresh set of textures, so the previous screen is
//! always released first.
//!
//! A document owns the whole screen it replaces, so the frame is wiped before it draws: the built-in
//! screens each wipe their own, and a document that leaves a corner unpainted would otherwise show
//! the frame before it rather than the background behind it.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rbms_render::font::with_text_context;
use rbms_render::result::{ResultView, TargetView};
use rbms_render::skin_render::state::{DecideViewState, KeyConfigViewState, PlayViewState, ResultViewState, SelectViewState};
use rbms_render::{
    Color, Renderer, SkinAssets, SkinDraw, SkinExprEval, SkinImage, SkinScreen, TextContext, TextureId, render_decide_screen, render_keyconfig_screen,
    render_play_screen, render_result_screen, render_select_screen, with_render_ctx,
};
use rbms_skin::dst::{LuaDrawEval, LuaExprId, OffsetSource};
use rbms_skin::loader::{LoadedSkin, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_RESULT};
use rbms_skin::lua::{LuaFrame, LuaSandbox};
use rbms_skin::property::SkinStateSource;

use crate::assets::{DecodePool, SkinAsset, SkinAssetJob, SkinAssetKind, spawn_skin_asset_decode};
use crate::notify::{Level, notify};
use crate::skin_select::SkinLibrary;
use crate::stage::Canvas;
use crate::{AppShared, SelectScene};

/// Hands one document's already-decoded files to [`SkinScreen::build`], and compiles the
/// expressions it carries.
///
/// Nothing here touches the disk. A published skin names dozens of source images and a font or two,
/// and decoding them is seconds of work; that happens on the worker pool before the screen is built
/// at all ([`PendingScreen`]), so the frame the document first draws on costs a texture upload and
/// nothing else. Held only for the length of one build: everything it produces is owned by the
/// screen afterwards, and the sandbox it compiles into belongs to the document.
pub(crate) struct PlayerSkinAssets<'a> {
    sandbox: Option<&'a LuaSandbox>,
    prepared: BTreeMap<SkinAssetJob, SkinAsset>,
}

impl<'a> PlayerSkinAssets<'a> {
    /// The assets one document is compiled with: its own sandbox, and the files a worker read.
    pub(crate) fn new(skin: &'a LoadedSkin, prepared: BTreeMap<SkinAssetJob, SkinAsset>) -> PlayerSkinAssets<'a> {
        PlayerSkinAssets { sandbox: skin.lua(), prepared }
    }

    /// The same, with no sandbox, for the tests that only care what it does with files.
    #[cfg(test)]
    pub(crate) fn prepared_only(prepared: BTreeMap<SkinAssetJob, SkinAsset>) -> PlayerSkinAssets<'a> {
        PlayerSkinAssets { sandbox: None, prepared }
    }
}

impl SkinAssets for PlayerSkinAssets<'_> {
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

    fn expression(&mut self, source: &str) -> Option<LuaExprId> {
        self.sandbox?.compile(source).ok()
    }
}

/// One frame's evaluator: the document's sandbox bound to the state the frame is drawn from.
///
/// Every read goes through [`LuaFrame`] -- draw gating and the `value`, `floatvalue` and `text`
/// fields alike -- because that is what holds the per-frame ceilings. An expression that loops
/// cannot stall a frame however many objects point at it, and a frame that spends its whole Lua
/// allowance leaves the rest of its objects on their defaults rather than running over.
pub(crate) struct SkinSandboxFrame<'a> {
    frame: LuaFrame<'a>,
}

impl<'a> SkinSandboxFrame<'a> {
    /// Binds `sandbox` to the state this frame answers property reads from.
    pub(crate) fn new(sandbox: &'a LuaSandbox, state: &'a dyn SkinStateSource) -> SkinSandboxFrame<'a> {
        SkinSandboxFrame { frame: sandbox.frame(state) }
    }

    /// How many expressions this frame has evaluated, which is what the budget counts.
    #[cfg(test)]
    pub(crate) fn calls(&self) -> u32 {
        self.frame.calls()
    }
}

impl LuaDrawEval for SkinSandboxFrame<'_> {
    fn eval_draw(&self, expr: LuaExprId) -> Option<bool> {
        self.frame.eval_draw(expr)
    }
}

impl SkinExprEval for SkinSandboxFrame<'_> {
    fn eval_integer(&self, expr: LuaExprId) -> Option<i32> {
        self.frame.eval_int(expr)
    }

    fn eval_float(&self, expr: LuaExprId) -> Option<f32> {
        self.frame.eval_float(expr)
    }

    fn eval_text(&self, expr: LuaExprId) -> Option<String> {
        self.frame.eval_string(expr)
    }
}

/// One compiled screen and the read of its document it was built from.
struct BuiltScreen {
    screen: SkinScreen,
    build: u64,
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
        let mut assets = PlayerSkinAssets::new(document, pending.ready);
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
}

/// Where the pointer is in a document's own coordinates: scaled onto the size it was authored at,
/// and measured up from the bottom the way a document measures everything.
fn document_cursor(cursor: (f32, f32), screen: (u32, u32), authored: (f32, f32)) -> Option<(f32, f32)> {
    let (screen_w, screen_h) = (screen.0 as f32, screen.1 as f32);
    if screen_w <= 0.0 || screen_h <= 0.0 {
        return None;
    }
    let (authored_w, authored_h) = authored;
    Some((cursor.0 / screen_w * authored_w, authored_h - cursor.1 / screen_h * authored_h))
}

impl AppShared {
    /// The clock every document is animated against, shared by its timers and its `skin.time()`.
    pub(crate) fn skin_now_ms(&self) -> i64 {
        self.clock.elapsed().as_millis() as i64
    }

    /// Read and compile the document one screen is drawn with, so the gate below has something to
    /// hand over. Cheap on every frame but the one after a document is chosen or reloaded.
    pub(crate) fn prepare_skin(&mut self, canvas: &mut Canvas<'_>, screen: i32) {
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
        self.skin_screens.get(screen).is_some() || self.skin_screens.is_pending(screen)
    }

    /// Whether one screen's document has finished being read and compiled, as against merely being
    /// on its way.
    #[cfg(test)]
    pub(crate) fn has_compiled_skin(&self, screen: i32) -> bool {
        self.skin_screens.get(screen).is_some()
    }

    /// The nudges the player made in the document one screen is drawn with.
    fn skin_offsets(&self, screen: i32) -> Option<&dyn OffsetSource> {
        let path = self.config.skin.document(screen)?;
        self.config.skin.customisation(path).map(|stored| stored as &dyn OffsetSource)
    }

    /// Everything one frame of a document needs, with `state` answering its property reads.
    ///
    /// `None` when no document is compiled for `screen`, which is the caller's cue to draw its own
    /// layout exactly as it always did.
    fn skin_frame<'a>(
        &'a self,
        canvas: &Canvas<'_>,
        screen: i32,
        state: &'a dyn SkinStateSource,
        lua: &'a mut Option<SkinSandboxFrame<'a>>,
        background: Option<TextureId>,
    ) -> Option<SkinDraw<'a>> {
        let compiled = self.skin_screens.get(screen)?;
        *lua = self.skins.document(screen).and_then(LoadedSkin::lua).map(|sandbox| SkinSandboxFrame::new(sandbox, state));
        Some(SkinDraw {
            screen: compiled,
            timers: &self.skin_timers,
            now_ms: state.now_ms(),
            lua: lua.as_ref().map(|frame| frame as &dyn SkinExprEval),
            mouse: document_cursor(self.cursor, canvas.size(), compiled.authored_size()),
            background,
            offsets: self.skin_offsets(screen),
        })
    }

    /// Draw the play screen's document. `true` when it drew, and the built-in field stands aside.
    pub(crate) fn draw_play_skin(&self, canvas: &mut Canvas<'_>, screen: i32, state: &PlayViewState<'_>, background: Option<TextureId>) -> bool {
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, screen, state, &mut lua, background) else {
            return false;
        };
        canvas.clear(Color::BLACK);
        with_render_ctx(|ctx| render_play_screen(ctx, canvas, Some(&document), state))
    }

    /// Draw the song browser's document.
    pub(crate) fn draw_select_skin(&self, canvas: &mut Canvas<'_>, view: &SelectScene, background: Option<TextureId>) -> bool {
        let state = SelectViewState { view, now_ms: self.skin_now_ms(), offsets: self.skin_offsets(SKIN_TYPE_MUSIC_SELECT) };
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, SKIN_TYPE_MUSIC_SELECT, &state, &mut lua, background) else {
            return false;
        };
        canvas.clear(Color::BLACK);
        with_render_ctx(|ctx| render_select_screen(ctx, canvas, Some(&document), view))
    }

    /// Draw the score screen's document.
    pub(crate) fn draw_result_skin(&self, canvas: &mut Canvas<'_>, view: &ResultView, target: Option<&TargetView>, cleared: bool) -> bool {
        let state = ResultViewState { view, target, cleared, now_ms: self.skin_now_ms(), offsets: self.skin_offsets(SKIN_TYPE_RESULT) };
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, SKIN_TYPE_RESULT, &state, &mut lua, None) else {
            return false;
        };
        canvas.clear(Color::BLACK);
        with_render_ctx(|ctx| render_result_screen(ctx, canvas, Some(&document), view, target, cleared))
    }

    /// Draw the loading screen's document, which stands in for the reference's decide screen.
    pub(crate) fn draw_decide_skin(&self, canvas: &mut Canvas<'_>, progress: f32, done: bool, title: &str) -> bool {
        let state = DecideViewState { progress, done, title, now_ms: self.skin_now_ms(), offsets: self.skin_offsets(SKIN_TYPE_DECIDE) };
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, SKIN_TYPE_DECIDE, &state, &mut lua, None) else {
            return false;
        };
        canvas.clear(Color::BLACK);
        with_render_ctx(|ctx| render_decide_screen(ctx, canvas, Some(&document), progress, done, title))
    }

    /// Draw the key configuration screen's document.
    pub(crate) fn draw_keyconfig_skin(&self, canvas: &mut Canvas<'_>, keys: &[String]) -> bool {
        let state = KeyConfigViewState { keys, now_ms: self.skin_now_ms(), offsets: self.skin_offsets(SKIN_TYPE_KEY_CONFIG) };
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, SKIN_TYPE_KEY_CONFIG, &state, &mut lua, None) else {
            return false;
        };
        canvas.clear(Color::BLACK);
        with_render_ctx(|ctx| render_keyconfig_screen(ctx, canvas, Some(&document), keys))
    }
}

#[cfg(test)]
mod tests;
