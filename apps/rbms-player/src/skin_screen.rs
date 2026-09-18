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
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rbms_config::SkinCustomisation;
use rbms_render::font::with_text_context;
use rbms_render::result::{ResultContent, ResultExtras, ResultView};
use rbms_render::skin_render::events::DocumentEvents;
use rbms_render::skin_render::state::{
    DecideChart, DecideViewState, KeyConfigViewState, OPTION_ROW_FOCUSED_FIRST, OPTION_ROW_LABEL_FIRST, OPTION_ROW_VALUE_FIRST, PlayViewState, PracticeRows,
    PracticeViewState, ResultViewState, SelectViewState,
};
use rbms_render::theme::OPTIONS_ROW_COUNT;
use rbms_render::{
    Color, CpuCanvas, FrameExtra, OptionsRows, PlayContent, Rect, Renderer, ScreenContent, SelectContent, SelectListState, SkinAssets, SkinDraw, SkinExprEval,
    SkinFrame, SkinHotAction, SkinHotspot, SkinImage, SkinObjectKind, SkinScreen, TextContext, TextureId, render_decide_screen, render_keyconfig_screen,
    render_play_screen, render_result_screen, render_select_screen, with_render_ctx,
};
use rbms_skin::dst::{DrawStateSource, LuaDrawEval, LuaExprId, OffsetSource, SkinOffset};
use rbms_skin::loader::{LoadedSkin, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_RESULT, skin_type_mode};
use rbms_skin::lua::{LuaFrame, LuaSandbox};
use rbms_skin::model::{SkinComposition, SkinDef, SkinLayer};
use rbms_skin::property::SkinStateSource;
use rbms_skin::property::generated::OPTION_PANEL1;
use rbms_skin::property::{UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER, UNMAPPED_STRING};
use rbms_skin::timer::{TimerRequest, TimerState};

mod events;

use crate::assets::{DecodePool, SkinAsset, SkinAssetJob, SkinAssetKind, skin_document_is_enabled, spawn_skin_asset_decode};
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

    fn eval_timer(&self, expr: LuaExprId) -> Option<i64> {
        self.frame.eval_timer(expr)
    }

    fn run_action(&self, expr: LuaExprId) -> Vec<TimerRequest> {
        self.frame.run_action(expr)
    }
}

/// One compiled screen and the read of its document it was built from.
struct BuiltScreen {
    screen: SkinScreen,
    build: u64,
    /// Which blocks of native output this document both named and compiled the objects for.
    ///
    /// Settled here rather than asked each frame: a block's demand is a list of object ids the
    /// document has to have declared, and answering that means walking every object the document
    /// draws, once per block, for every screen drawn. Neither side of the question moves while a
    /// screen is built, so it is answered once and read from here.
    content: ScreenContent,
    /// Whether this document placed the chart's art itself, so the built-in slot must leave it
    /// alone. Read off the document rather than the compiled screen, for the reason
    /// [`document_places_chart_art`] gives.
    places_chart_art: bool,
    /// The document's own timers, events and clickable objects, compiled beside the screen it draws
    /// and from the same sandbox.
    events: DocumentEvents,
}

/// Which of the three screens a skin type belongs to, for the screens that let a document stand in
/// for named pieces of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenKind {
    Play,
    Select,
    Result,
}

/// The screen one skin type draws, or `None` for the screens that have no replacement blocks of
/// their own: a document there either draws the whole screen or sits over it.
fn screen_kind(screen: i32) -> Option<ScreenKind> {
    match screen {
        SKIN_TYPE_MUSIC_SELECT => Some(ScreenKind::Select),
        SKIN_TYPE_RESULT => Some(ScreenKind::Result),
        _ => skin_type_mode(screen).map(|_| ScreenKind::Play),
    }
}

/// What a document has to have compiled before one named block of native output steps aside.
///
/// Three kinds of demand, because that is what the blocks themselves are made of. Some are a set of
/// named objects, so the document is asked for ids; some are whatever the document chose to call its
/// note field or its wheel, so it is asked for kinds; and the browser's top bar is a row of buttons
/// that have to lead somewhere, so it is asked for hotspot actions. A block that demands nothing is
/// replaced on the strength of being named at all, which is what the play frame is.
#[derive(Debug, Clone, Copy)]
struct Replacement {
    /// Object ids the document must have declared, every one of them.
    ids: &'static [&'static str],
    /// Object kinds it must draw at least one of, every one of them.
    kinds: &'static [SkinObjectKind],
    /// Kinds any one of which is enough, for the block the specification offers a choice in.
    either: &'static [SkinObjectKind],
    /// Actions its `hotspot` table must name, every one of them.
    actions: &'static [SkinHotAction],
}

impl Replacement {
    /// A block that asks only to be named.
    const NOTHING: Replacement = Replacement { ids: &[], kinds: &[], either: &[], actions: &[] };

    const fn ids(ids: &'static [&'static str]) -> Replacement {
        Replacement { ids, ..Replacement::NOTHING }
    }

    const fn kinds(kinds: &'static [SkinObjectKind]) -> Replacement {
        Replacement { kinds, ..Replacement::NOTHING }
    }

    const fn either(either: &'static [SkinObjectKind]) -> Replacement {
        Replacement { either, ..Replacement::NOTHING }
    }

    const fn actions(actions: &'static [SkinHotAction]) -> Replacement {
        Replacement { actions, ..Replacement::NOTHING }
    }

    /// The same demand with named objects added, for a block that asks for both a kind and a reading
    /// beside it.
    const fn with_ids(self, ids: &'static [&'static str]) -> Replacement {
        Replacement { ids, ..self }
    }

    /// Whether `screen` compiled everything this block needs.
    fn met_by(&self, screen: &SkinScreen) -> bool {
        let available = screen.object_ids();
        self.ids.iter().all(|id| available.contains(id))
            && self.kinds.iter().all(|kind| screen.count_of(*kind) > 0)
            && (self.either.is_empty() || self.either.iter().any(|kind| screen.count_of(*kind) > 0))
            && self.actions.iter().all(|action| screen.declares_hotspot(*action))
    }
}

/// Every block of native output a document may stand in for, by the screen it belongs to and the
/// name the document writes in its `replace` list.
///
/// `None` is a name this build has no block for, which is worth a warning of its own: a document
/// that misspells a block would otherwise quietly keep the native content it meant to replace.
fn replacement(screen: i32, name: &str) -> Option<Replacement> {
    match screen_kind(screen)? {
        ScreenKind::Play => match name {
            "field" => Some(Replacement::kinds(&[SkinObjectKind::Note])),
            "gauge" => Some(Replacement::kinds(&[SkinObjectKind::Gauge]).with_ids(&["play-gauge-value"])),
            "judge" => Some(Replacement::kinds(&[SkinObjectKind::Judge])),
            "score" => Some(Replacement::ids(&["play-ex", "play-best", "play-green"])),
            "counts" => Some(Replacement::ids(&[
                "play-count-pg",
                "play-count-gr",
                "play-count-gd",
                "play-count-bd",
                "play-count-pr",
                "play-count-ms",
                "play-fast",
                "play-slow",
            ])),
            "graph" => Some(Replacement::ids(&["play-graph-ex", "play-graph-best", "play-graph-target"])),
            "cover" => Some(Replacement::either(&[SkinObjectKind::HiddenCover, SkinObjectKind::LiftCover])),
            "frame" => Some(Replacement::NOTHING),
            _ => None,
        },
        ScreenKind::Select => match name {
            "list" => Some(Replacement::kinds(&[SkinObjectKind::SongList])),
            "detail" => Some(Replacement::ids(&[
                "select-title",
                "select-artist",
                "select-genre",
                "select-level",
                "select-stat-0",
                "select-stat-1",
                "select-stat-2",
                "select-stat-3",
                "select-stat-4",
                "select-stat-5",
                "select-density",
                "select-record",
            ])),
            "topbar" => Some(Replacement::actions(&[
                SkinHotAction::Search,
                SkinHotAction::Sort,
                SkinHotAction::Folders,
                SkinHotAction::Tables,
                SkinHotAction::Records,
                SkinHotAction::Settings,
            ])),
            "options" => Some(Replacement::ids(OPTION_PANEL_IDS)),
            _ => None,
        },
        ScreenKind::Result => match name {
            "score" => {
                Some(Replacement::ids(&["result-score-label", "result-score", "result-combo-label", "result-combo", "result-notes-label", "result-notes"]))
            }
            "clear" => Some(Replacement::ids(&["result-clear"])),
            "judgment" => Some(Replacement::ids(&[
                "result-judge-perfect",
                "result-judge-great",
                "result-judge-good",
                "result-judge-bad",
                "result-judge-poor",
                "result-judge-miss",
            ])),
            "target" => Some(Replacement::ids(&["result-target"])),
            "grade" => Some(Replacement::ids(&["result-rank", "result-rate", "result-rankbar"])),
            "graphs" => Some(Replacement::kinds(&[SkinObjectKind::GaugeGraph, SkinObjectKind::JudgeGraph, SkinObjectKind::TimingDistribution])),
            "title" => Some(Replacement::ids(&["result-title"])),
            "hint" => Some(Replacement::ids(&["result-hint"])),
            "ir" => Some(Replacement::ids(&["result-ir"])),
            _ => None,
        },
    }
}

/// The objects the browser's option overlay is replaced by: a label and a value for each of the
/// panel's rows, and the panel behind them.
const OPTION_PANEL_IDS: &[&str] = &[
    "options-panel",
    "option-row-0-label",
    "option-row-0-value",
    "option-row-1-label",
    "option-row-1-value",
    "option-row-2-label",
    "option-row-2-value",
    "option-row-3-label",
    "option-row-3-value",
    "option-row-4-label",
    "option-row-4-value",
    "option-row-5-label",
    "option-row-5-value",
    "option-row-6-label",
    "option-row-6-value",
    "option-row-7-label",
    "option-row-7-value",
    "option-row-8-label",
    "option-row-8-value",
    "option-row-9-label",
    "option-row-9-value",
    "option-row-10-label",
    "option-row-10-value",
];

/// Whether one block of native output has both been named by the document and had everything it
/// needs compiled.
fn block_replaced(compiled: &SkinScreen, names: &std::collections::BTreeSet<String>, screen: i32, name: &str) -> bool {
    names.contains(name) && replacement(screen, name).is_some_and(|need| need.met_by(compiled))
}

/// Which blocks of one screen's native output a freshly compiled document has taken over.
///
/// A block the document named but did not compile the objects for stays native, which is the
/// contract the score screen had first and every screen now shares.
fn screen_content_of(screen: i32, compiled: &SkinScreen, names: &std::collections::BTreeSet<String>) -> ScreenContent {
    let mut content = ScreenContent::default();
    let on = |name: &str| block_replaced(compiled, names, screen, name);
    match screen_kind(screen) {
        Some(ScreenKind::Play) => {
            content.play = PlayContent {
                field: on("field"),
                gauge: on("gauge"),
                judge: on("judge"),
                score: on("score"),
                counts: on("counts"),
                graph: on("graph"),
                cover: on("cover"),
                frame: on("frame"),
            };
        }
        Some(ScreenKind::Select) => {
            content.select = SelectContent { list: on("list"), detail: on("detail"), topbar: on("topbar"), options: on("options") };
        }
        Some(ScreenKind::Result) => {
            content.result = ResultContent {
                score: on("score"),
                clear: on("clear"),
                judgment: on("judgment"),
                target: on("target"),
                grade: on("grade"),
                graphs: on("graphs"),
                title: on("title"),
                hint: on("hint"),
                ir: on("ir"),
            };
        }
        None => {}
    }
    content
}

/// Whether a document places the chart's own art, which is a matter of declaring a `bga` object
/// rather than of drawing one this frame.
///
/// Declaring is the whole test. A document may gate every destination of its `bga` object off --
/// which is how the default bundle spells BGA SIZE OFF -- and it has still taken the art over: a
/// built-in quad painted underneath would put back exactly the picture the player turned off, and
/// the document would have to keep a fully transparent destination alive purely to hold its claim.
/// A document with no `bga` object at all has claimed nothing, and the built-in slot paints as it
/// always did.
fn document_places_chart_art(def: &SkinDef) -> bool {
    def.bga.is_some()
}

/// The nudges one document is drawn with: what the player chose for the whole bundle, with what was
/// chosen inside this document laid over it.
///
/// Two borrows rather than one merged map, because a frame reads a handful of ids and rebuilding a
/// map every frame to answer them would cost more than the second lookup it saves.
/// Between the score server's status lines when a document shows them as one.
const IR_STATUS_SEPARATOR: &str = "   ";

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MergedOffsets<'a> {
    document: Option<&'a SkinCustomisation>,
    bundle: Option<&'a SkinCustomisation>,
}

impl OffsetSource for MergedOffsets<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.document.and_then(|stored| stored.offset(id)).or_else(|| self.bundle.and_then(|stored| stored.offset(id)))
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
    offsets: &'a MergedOffsets<'a>,
    extra: FrameExtra<'a>,
}

/// The browser's frame state: the rows and detail the browser measured, with the option panel's rows
/// joined on.
///
/// The panel's rows are read out of the configuration rather than measured by the browser, so they
/// are attached here -- beside the offsets and the timers, which are the application's too -- rather
/// than travelling through the browser's own view.
fn select_list<'a>(view: &'a SelectScene, options: &'a OptionsRows<'a>) -> SelectListState<'a> {
    SelectListState { rows: &view.rows, sel: view.sel, detail: &view.detail, options: Some(options) }
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
    /// The one live preview, of whichever document the skin settings are on. One rather than one per
    /// screen because only one is ever shown: the settings screen's own thumbnail and a document's
    /// `skinpreview` pane are both looking at the row the player is editing.
    preview: Option<PreviewCache>,
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
        let mut event_warnings: Vec<String> = Vec::new();
        let events = DocumentEvents::build(&document.def, &mut assets, &mut event_warnings);
        let warnings: Vec<&String> = compiled.warnings().iter().chain(event_warnings.iter()).collect();
        if let Some(first) = warnings.first() {
            let rest = warnings.len() - 1;
            let more = if rest > 0 { format!(" (and {rest} more)") } else { String::new() };
            notify(Level::Warn, format!("skin: {first}{more}"));
        }
        for name in document.replace_names() {
            if !replacement(screen, name).is_some_and(|need| need.met_by(&compiled)) {
                notify(Level::Warn, format!("skin: the {name} replacement is incomplete; native content remains visible"));
            }
        }
        let content = screen_content_of(screen, &compiled, document.replace_names());
        let places_chart_art = document_places_chart_art(&document.def);
        self.built.insert(screen, BuiltScreen { screen: compiled, build: pending.build, content, places_chart_art, events });
    }

    /// Whether the document compiled for `screen` places the chart's art itself.
    fn places_chart_art(&self, screen: i32) -> bool {
        self.built.get(&screen).is_some_and(|entry| entry.places_chart_art)
    }

    /// The compiled timers, events and clickable objects of one screen's document.
    fn events(&self, screen: i32) -> Option<&DocumentEvents> {
        self.built.get(&screen).map(|entry| &entry.events)
    }

    /// Which blocks of native output the compiled document for `screen` has taken over, or none at
    /// all while the built-in screen draws.
    fn content(&self, screen: i32) -> ScreenContent {
        self.built.get(&screen).map_or_else(ScreenContent::default, |entry| entry.content)
    }

    /// The compiled screen one screen type is drawn with, or `None` while the built-in screen draws.
    pub(crate) fn get(&self, screen: i32) -> Option<&SkinScreen> {
        self.built.get(&screen).map(|entry| &entry.screen)
    }

    /// Whether one screen's document is still having its files read.
    pub(crate) fn is_pending(&self, screen: i32) -> bool {
        self.pending.contains_key(&screen)
    }

    /// Everything one screen's document reported while it compiled, for the tests that hold a
    /// shipped document to compiling clean.
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
        if !self.skin_document_is_enabled(screen) {
            return;
        }
        if self.skins.needs_reload_for(&self.config, screen) {
            self.skins.reload_for(&self.config, screen);
        }
        let skins = &self.skins;
        let screens = &mut self.skin_screens;
        with_text_context(|text| screens.sync(canvas, text, screen, skins));
        self.refresh_skin_preview(canvas, screen);
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

    pub(crate) fn skin_uses_overlay(&self, screen: i32) -> bool {
        let supports_overlay = matches!(screen, SKIN_TYPE_MUSIC_SELECT | SKIN_TYPE_RESULT) || skin_type_mode(screen).is_some();
        supports_overlay
            && self.skin_document_is_enabled(screen)
            && self.skins.document(screen).is_some_and(|document| document.def.composition == SkinComposition::Overlay)
    }

    /// Whether the document shares its screen with the native layout.
    pub(crate) fn skin_uses_native_layout(&self, screen: i32) -> bool {
        let supports_native_layout = matches!(screen, SKIN_TYPE_MUSIC_SELECT | SKIN_TYPE_RESULT) || skin_type_mode(screen).is_some();
        supports_native_layout
            && self.skin_document_is_enabled(screen)
            && self.skins.document(screen).is_some_and(|document| matches!(document.def.composition, SkinComposition::Overlay | SkinComposition::Layered))
    }

    /// Whether a native-layout document has separate background and foreground destinations.
    pub(crate) fn skin_uses_layered_layout(&self, screen: i32) -> bool {
        self.skin_document_is_enabled(screen) && self.skins.document(screen).is_some_and(|document| document.def.composition == SkinComposition::Layered)
    }

    /// Which blocks of one screen's native output the document has taken over.
    ///
    /// Only a layered document replaces anything: one that draws the whole screen leaves nothing to
    /// replace, and one that sits over the screen is drawn on top of what is already there. Which
    /// blocks the document itself covers was settled when it compiled ([`BuiltScreen::content`]);
    /// what is read here every frame is how the player chose to compose it.
    pub(crate) fn screen_content(&self, screen: i32) -> ScreenContent {
        if !self.skin_uses_layered_layout(screen) {
            return ScreenContent::default();
        }
        self.skin_screens.content(screen)
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

    /// The nudges the player made in the document one screen is drawn with, over the ones made for
    /// its whole bundle.
    ///
    /// A bundle-scope row is chosen once and read by every screen shipped beside it, so the two
    /// stores are consulted in that order rather than one replacing the other. The result is held by
    /// the caller for the length of a frame, which is what keeps it two borrows instead of a map.
    /// The score server's status, as one line a document can show where it likes.
    fn ir_status_text(&self) -> String {
        self.ir_status.lines().iter().map(|(text, _)| text.as_str()).collect::<Vec<_>>().join(IR_STATUS_SEPARATOR)
    }

    /// Whether the document for `screen` places the chart's art itself, so the built-in slot must
    /// not paint it a second time underneath.
    pub(crate) fn skin_draws_background(&self, screen: Option<i32>) -> bool {
        screen.is_some_and(|screen| self.skin_screens.places_chart_art(screen))
    }

    pub(crate) fn skin_offsets(&self, screen: i32) -> MergedOffsets<'_> {
        let Some(path) = self.config.skin.document(screen) else {
            return MergedOffsets::default();
        };
        MergedOffsets {
            document: self.config.skin.customisation(path),
            bundle: self.config.skin.bundle_key(path).and_then(|bundle| self.config.skin.shared_customisation(&bundle)),
        }
    }

    /// Everything one frame of a document needs, with `state` answering its property reads and
    /// `extra` carrying the screen-shaped state no property id can hold.
    ///
    /// `None` when no document is compiled for `screen`, which is the caller's cue to draw its own
    /// layout exactly as it always did.
    ///
    /// The nudges travel in `inputs` rather than being read here because they are borrowed from the
    /// configuration and handed on as a trait object: the caller holds them for the whole frame.
    fn skin_frame<'a>(
        &'a self,
        canvas: &Canvas<'_>,
        screen: i32,
        state: &'a dyn SkinStateSource,
        lua: &'a mut Option<SkinSandboxFrame<'a>>,
        inputs: FrameInputs<'a>,
    ) -> Option<SkinDraw<'a>> {
        if !self.skin_document_is_enabled(screen) {
            return None;
        }
        let compiled = self.skin_screens.get(screen)?;
        *lua = self.skins.document(screen).and_then(LoadedSkin::lua).map(|sandbox| SkinSandboxFrame::new(sandbox, state));
        Some(SkinDraw {
            screen: compiled,
            timers: &self.skin_timers,
            now_ms: state.now_ms(),
            lua: lua.as_ref().map(|frame| frame as &dyn SkinExprEval),
            mouse: document_cursor(self.cursor, canvas.size(), compiled.authored_size()),
            background: inputs.background,
            offsets: Some(inputs.offsets as &dyn OffsetSource),
            extra: inputs.extra,
        })
    }

    /// Every rectangle the document drawn for `screen` offers a click on, already placed on the
    /// canvas.
    ///
    /// Empty for a screen with no compiled document, which is the caller's cue that its own layout
    /// still owns every hit rectangle on the screen. The frame is assembled exactly as a drawn one
    /// is, because a slot is only clickable where it was actually drawn: the same gates, the same
    /// clock and the same pointer.
    /// `stands_in` is whether the document has taken over a block of native output whose own
    /// rectangles it must now answer for. Its own `act` objects are answered either way: those are
    /// the document's alone, so offering them lays nothing over a native rectangle.
    pub(crate) fn skin_hotspots(
        &self,
        canvas: &Canvas<'_>,
        screen: i32,
        state: &dyn SkinStateSource,
        extra: FrameExtra<'_>,
        stands_in: bool,
    ) -> Vec<SkinHotspot> {
        if !self.skin_document_is_enabled(screen) {
            return Vec::new();
        }
        let Some(compiled) = self.skin_screens.get(screen) else {
            return Vec::new();
        };
        let lua = self.skins.document(screen).and_then(LoadedSkin::lua).map(|sandbox| SkinSandboxFrame::new(sandbox, state));
        let frame = SkinFrame {
            now_ms: state.now_ms(),
            timers: &self.skin_timers,
            state,
            lua: lua.as_ref().map(|frame| frame as &dyn SkinExprEval),
            mouse: document_cursor(self.cursor, canvas.size(), compiled.authored_size()),
            background: None,
            extra,
        };
        let mut spots = if stands_in { compiled.hotspots_on_screen(&frame, canvas.size()) } else { Vec::new() };
        if let Some(events) = self.skin_screens.events(screen) {
            spots.extend(compiled.event_hotspots_on_screen(&frame, canvas.size(), events));
        }
        spots
    }

    fn skin_document_is_enabled(&self, screen: i32) -> bool {
        skin_document_is_enabled(&self.settings_path, &self.config, screen)
    }

    /// Draw the play screen's document. `true` when it drew, and the built-in field stands aside.
    pub(crate) fn draw_play_skin(
        &self,
        canvas: &mut Canvas<'_>,
        screen: i32,
        state: &PlayViewState<'_>,
        background: Option<TextureId>,
        extra: FrameExtra<'_>,
    ) -> bool {
        let offsets = self.skin_offsets(screen);
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, screen, state, &mut lua, FrameInputs { background, offsets: &offsets, extra }) else {
            return false;
        };
        if !self.skin_uses_overlay(screen) {
            canvas.clear(Color::BLACK);
        }
        with_render_ctx(|ctx| render_play_screen(ctx, canvas, Some(&document), state))
    }

    /// Draws one phase of a layered play document without clearing the native frame.
    pub(crate) fn draw_play_skin_layer(
        &self,
        canvas: &mut Canvas<'_>,
        screen: i32,
        state: &PlayViewState<'_>,
        background: Option<TextureId>,
        extra: FrameExtra<'_>,
        layer: SkinLayer,
    ) -> bool {
        let offsets = self.skin_offsets(screen);
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, screen, state, &mut lua, FrameInputs { background, offsets: &offsets, extra }) else {
            return false;
        };
        with_render_ctx(|ctx| {
            document.draw_layer(ctx, canvas, state, layer);
        });
        true
    }

    /// Draw the song browser's document.
    ///
    /// A caller with nothing of its own to add gets the browser's rows and the option panel joined
    /// on here, so a document draws its wheel and its panel without the stage assembling either.
    pub(crate) fn draw_select_skin(&self, canvas: &mut Canvas<'_>, view: &SelectScene, background: Option<TextureId>, extra: FrameExtra<'_>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_MUSIC_SELECT);
        let options = crate::app_options::options_rows(self);
        let own = select_list(view, &options);
        let extra = match extra {
            FrameExtra::None => FrameExtra::Select(&own),
            given => given,
        };
        let state = SelectViewState::new(view, self.skin_now_ms(), Some(&offsets), extra.select().and_then(|list| list.options));
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, SKIN_TYPE_MUSIC_SELECT, &state, &mut lua, FrameInputs { background, offsets: &offsets, extra }) else {
            return false;
        };
        if !self.skin_uses_overlay(SKIN_TYPE_MUSIC_SELECT) {
            canvas.clear(Color::BLACK);
        }
        with_render_ctx(|ctx| render_select_screen(ctx, canvas, Some(&document), view))
    }

    /// Draws one phase of a layered browser document without clearing the native frame.
    pub(crate) fn draw_select_skin_layer(
        &self,
        canvas: &mut Canvas<'_>,
        view: &SelectScene,
        background: Option<TextureId>,
        extra: FrameExtra<'_>,
        layer: SkinLayer,
    ) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_MUSIC_SELECT);
        let options = crate::app_options::options_rows(self);
        let own = select_list(view, &options);
        let extra = match extra {
            FrameExtra::None => FrameExtra::Select(&own),
            given => given,
        };
        let state = SelectViewState::new(view, self.skin_now_ms(), Some(&offsets), extra.select().and_then(|list| list.options));
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, SKIN_TYPE_MUSIC_SELECT, &state, &mut lua, FrameInputs { background, offsets: &offsets, extra }) else {
            return false;
        };
        with_render_ctx(|ctx| {
            document.draw_layer(ctx, canvas, &state, layer);
        });
        true
    }

    /// The rectangles the browser's document offers a click on, already placed on the canvas, or an
    /// empty list while the browser's own layout still owns them.
    ///
    /// Nothing is answered until the document has actually taken the row list or the top bar over:
    /// a document that draws neither has nothing to be clicked on that the built-in browser has not
    /// already put there, and answering anyway would lay a second set of rectangles over the first.
    pub(crate) fn select_hotspots(&self, canvas: &Canvas<'_>, view: &SelectScene) -> Vec<SkinHotspot> {
        let content = self.screen_content(SKIN_TYPE_MUSIC_SELECT).select;
        let stands_in = content.list || content.topbar;
        let acts = self.skin_screens.events(SKIN_TYPE_MUSIC_SELECT).is_some_and(|events| events.click_count() > 0);
        if !stands_in && !acts {
            return Vec::new();
        }
        let offsets = self.skin_offsets(SKIN_TYPE_MUSIC_SELECT);
        let options = crate::app_options::options_rows(self);
        let own = select_list(view, &options);
        let state = SelectViewState::new(view, self.skin_now_ms(), Some(&offsets), Some(&options));
        self.skin_hotspots(canvas, SKIN_TYPE_MUSIC_SELECT, &state, FrameExtra::Select(&own), stands_in)
    }

    /// Draw the score screen's document.
    pub(crate) fn draw_result_skin(&self, canvas: &mut Canvas<'_>, view: &ResultView, extras: &ResultExtras, cleared: bool, extra: FrameExtra<'_>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_RESULT);
        let state = ResultViewState::new(view, extras.target.as_ref(), cleared, self.skin_now_ms(), Some(&offsets))
            .with_hint(extras.run_again)
            .with_ir_status(self.ir_status_text());
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, SKIN_TYPE_RESULT, &state, &mut lua, FrameInputs { background: None, offsets: &offsets, extra }) else {
            return false;
        };
        if !self.skin_uses_overlay(SKIN_TYPE_RESULT) {
            canvas.clear(Color::BLACK);
        }
        with_render_ctx(|ctx| render_result_screen(ctx, canvas, Some(&document), view, extras.target.as_ref(), cleared))
    }

    pub(crate) fn draw_result_skin_layer(
        &self,
        canvas: &mut Canvas<'_>,
        view: &ResultView,
        extras: &ResultExtras,
        cleared: bool,
        extra: FrameExtra<'_>,
        layer: SkinLayer,
    ) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_RESULT);
        let state = ResultViewState::new(view, extras.target.as_ref(), cleared, self.skin_now_ms(), Some(&offsets))
            .with_hint(extras.run_again)
            .with_ir_status(self.ir_status_text());
        let mut lua = None;
        let Some(document) = self.skin_frame(canvas, SKIN_TYPE_RESULT, &state, &mut lua, FrameInputs { background: None, offsets: &offsets, extra }) else {
            return false;
        };
        with_render_ctx(|ctx| {
            document.draw_layer(ctx, canvas, &state, layer);
        });
        true
    }

    /// Draw the loading screen's document, which stands in for the reference's decide screen.
    pub(crate) fn draw_decide_skin(&self, canvas: &mut Canvas<'_>, progress: f32, done: bool, title: &str, chart: DecideChart<'_>) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_DECIDE);
        let state = DecideViewState { progress, done, title, chart, now_ms: self.skin_now_ms(), offsets: Some(&offsets) };
        let mut lua = None;
        let Some(document) =
            self.skin_frame(canvas, SKIN_TYPE_DECIDE, &state, &mut lua, FrameInputs { background: None, offsets: &offsets, extra: FrameExtra::None })
        else {
            return false;
        };
        canvas.clear(Color::BLACK);
        with_render_ctx(|ctx| render_decide_screen(ctx, canvas, Some(&document), &state))
    }

    /// Draw the key configuration screen's document.
    pub(crate) fn draw_keyconfig_skin(&self, canvas: &mut Canvas<'_>, keys: &[String]) -> bool {
        let offsets = self.skin_offsets(SKIN_TYPE_KEY_CONFIG);
        let state = KeyConfigViewState { keys, now_ms: self.skin_now_ms(), offsets: Some(&offsets) };
        let mut lua = None;
        let Some(document) =
            self.skin_frame(canvas, SKIN_TYPE_KEY_CONFIG, &state, &mut lua, FrameInputs { background: None, offsets: &offsets, extra: FrameExtra::None })
        else {
            return false;
        };
        canvas.clear(Color::BLACK);
        with_render_ctx(|ctx| render_keyconfig_screen(ctx, canvas, Some(&document), keys))
    }
}

/// How large the live preview of another document is drawn, in the pixels it is rendered at.
///
/// Sixteen by nine, so a document authored at any of the sizes the loader accepts keeps its shape,
/// and small enough that rendering one costs a fraction of a frame.
const PREVIEW_SIZE: (u32, u32) = (256, 144);

/// The registry key the one preview texture is held under, so a refreshed preview replaces the
/// pixels it had rather than leaving a second texture uploaded every time the settings move.
const PREVIEW_TEXTURE_KEY: &str = "rbms.skin.preview";

/// The preview of one document, rendered once and held until the document behind it is read again.
///
/// The pixels and the texture are kept apart because they are made at different moments: the pixels
/// come from an offscreen pass that needs no renderer, and the texture is registered with whichever
/// renderer the frame is being drawn on.
struct PreviewCache {
    screen: i32,
    build: u64,
    size: (u32, u32),
    /// `None` for a document that would not render, which is remembered so the attempt is not made
    /// again on the next frame.
    image: Option<SkinImage>,
    tex: Option<TextureId>,
}

/// Everything a document reads while it is being previewed: nothing at all.
///
/// A preview is a still of the document rather than of a run, so every property is unmapped, every
/// timer unset and the clock is at zero. What survives that is the document's own art and the
/// choices its customisation rows baked in when it was read, which is what a preview is for.
#[derive(Debug, Default, Clone, Copy)]
struct PreviewIdleState;

impl OffsetSource for PreviewIdleState {
    fn offset(&self, _id: i32) -> Option<SkinOffset> {
        None
    }
}

impl DrawStateSource for PreviewIdleState {
    fn boolean(&self, id: i32) -> bool {
        if id < 0 { !UNMAPPED_BOOLEAN } else { UNMAPPED_BOOLEAN }
    }
}

impl SkinStateSource for PreviewIdleState {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, _id: i32) -> f32 {
        UNMAPPED_FLOAT
    }

    fn string(&self, _id: i32) -> &str {
        UNMAPPED_STRING
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        0
    }
}

/// Reads and decodes every file one document names, on the calling thread.
///
/// The worker pool does the decoding, exactly as a screen build does; what is different is that the
/// caller waits for it. A preview is asked for when the settings screen opens or the SKIN rows move,
/// and the answer is held until that document is read again -- so the wait is paid once per document
/// rather than once per frame, and no frame loop has to poll for it.
fn read_document_files(document: &LoadedSkin) -> BTreeMap<SkinAssetJob, SkinAsset> {
    let images = document.sources.values().map(|path| (SkinAssetKind::Image, path.clone()));
    let fonts = document.fonts.values().map(|path| (SkinAssetKind::Font, path.clone()));
    let jobs: Vec<SkinAssetJob> = images.chain(fonts).collect();
    let (received, _progress, _total) = spawn_skin_asset_decode(jobs, Arc::new(AtomicBool::new(false)));
    received.into_iter().collect()
}

/// Draws one document onto a canvas of its own and answers the pixels.
///
/// Must not be called from inside a render context or a text context: it opens both itself, and
/// each is one borrow of a thread-local deep.
fn render_document_offscreen(document: &LoadedSkin, size: (u32, u32)) -> Option<SkinImage> {
    let mut canvas = CpuCanvas::new(size.0, size.1);
    let mut assets = PlayerSkinAssets::new(document, read_document_files(document));
    let mut compiled = with_text_context(|text| SkinScreen::build(&mut canvas, text, document, &mut assets));
    let timers = TimerState::default();
    let state = PreviewIdleState;
    canvas.clear(Color::BLACK);
    let frame = SkinFrame { now_ms: 0, timers: &timers, state: &state, lua: None, mouse: None, background: None, extra: FrameExtra::None };
    with_render_ctx(|ctx| compiled.draw(ctx, &mut canvas, &frame));
    compiled.release(&mut canvas);
    SkinImage::new(size.0, size.1, canvas.pixels().to_vec())
}

impl SkinScreens {
    /// The compiled screen one screen type is drawn with, for the caller that has something to set
    /// on it.
    fn get_mut(&mut self, screen: i32) -> Option<&mut SkinScreen> {
        self.built.get_mut(&screen).map(|entry| &mut entry.screen)
    }

    /// The live preview of the document chosen for `screen`, rendered at `size`.
    ///
    /// Rendered once per read of that document and held afterwards, so asking every frame costs a
    /// comparison. `None` while no document is chosen for that screen, or when the document would
    /// not render -- and one that would not render is remembered as such rather than retried.
    pub(crate) fn render_preview(&mut self, skins: &SkinLibrary, screen: i32, size: (u32, u32)) -> Option<&SkinImage> {
        let build = skins.build_of(screen)?;
        let held = self.preview.as_ref().is_some_and(|cache| cache.screen == screen && cache.build == build && cache.size == size);
        if !held {
            let image = skins.document(screen).and_then(|document| render_document_offscreen(document, size));
            self.preview = Some(PreviewCache { screen, build, size, image, tex: None });
        }
        self.preview.as_ref()?.image.as_ref()
    }

    /// The same preview as a texture registered with `r`, for the screens that draw it.
    ///
    /// Registered once per preview under one key, so a frame that asks again is handed the handle it
    /// already had rather than uploading the picture a second time.
    pub(crate) fn preview_texture<R: Renderer>(&mut self, r: &mut R, skins: &SkinLibrary, screen: i32, size: (u32, u32)) -> Option<TextureId> {
        self.render_preview(skins, screen, size)?;
        let cache = self.preview.as_mut()?;
        if cache.tex.is_none() {
            let image = cache.image.as_ref()?;
            cache.tex = Some(r.register_texture(PREVIEW_TEXTURE_KEY, &image.rgba, image.width, image.height));
        }
        cache.tex
    }
}

/// The option panel on its own, for resolving where a document put its rows.
///
/// Only the panel's own ids are answered, because what is wanted is the rectangle each row landed
/// on, and a row's rectangle is gated on the panel rather than on the chart underneath it.
struct OptionPanelState<'a> {
    rows: &'a OptionsRows<'a>,
    now_ms: i64,
    offsets: &'a MergedOffsets<'a>,
}

impl OffsetSource for OptionPanelState<'_> {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.offsets.offset(id)
    }
}

impl DrawStateSource for OptionPanelState<'_> {
    fn boolean(&self, id: i32) -> bool {
        let asked = id.abs();
        let answer = if asked == OPTION_PANEL1 {
            self.rows.open
        } else {
            match option_row_of(asked, OPTION_ROW_FOCUSED_FIRST) {
                Some(row) => self.rows.focused == row,
                None => UNMAPPED_BOOLEAN,
            }
        };
        if id < 0 { !answer } else { answer }
    }
}

impl SkinStateSource for OptionPanelState<'_> {
    fn integer(&self, _id: i32) -> i32 {
        UNMAPPED_INTEGER
    }

    fn float(&self, _id: i32) -> f32 {
        UNMAPPED_FLOAT
    }

    fn string(&self, id: i32) -> &str {
        if let Some(row) = option_row_of(id, OPTION_ROW_LABEL_FIRST) {
            return self.rows.labels[row];
        }
        if let Some(row) = option_row_of(id, OPTION_ROW_VALUE_FIRST) {
            return self.rows.values[row].as_str();
        }
        UNMAPPED_STRING
    }

    fn timer(&self, _id: i32) -> Option<i64> {
        None
    }

    fn now_ms(&self) -> i64 {
        self.now_ms
    }
}

/// Which option row one of the panel's private id bands names, counted from `first`.
fn option_row_of(id: i32, first: i32) -> Option<usize> {
    (id >= first && id < first + OPTIONS_ROW_COUNT as i32).then(|| (id - first) as usize)
}

impl AppShared {
    /// The preview of whichever document the SKIN tab is configuring, as a texture on `canvas`.
    ///
    /// `None` while the built-in screen is chosen for that row, which is the caller's cue to leave
    /// the preview area empty.
    pub(crate) fn skin_preview_texture(&mut self, canvas: &mut Canvas<'_>, size: (u32, u32)) -> Option<TextureId> {
        let screen = self.config.skin.screen;
        let skins = &self.skins;
        self.skin_screens.preview_texture(canvas, skins, screen, size)
    }

    /// Points the document drawn for `screen` at the preview it should show, or at nothing.
    ///
    /// The recursion guard is here: a document previewing the very screen it is drawn on would have
    /// to render itself to draw itself, so that case is handed no texture and the pane stays empty.
    /// Nothing is rendered at all for a document that declares no preview pane, which is every
    /// document the bundle ships.
    pub(crate) fn refresh_skin_preview(&mut self, canvas: &mut Canvas<'_>, screen: i32) {
        if self.skin_screens.get(screen).is_none_or(|compiled| compiled.count_of(SkinObjectKind::SkinPreview) == 0) {
            return;
        }
        let previewed = self.config.skin.screen;
        let tex = if previewed == screen {
            None
        } else {
            let skins = &self.skins;
            self.skin_screens.preview_texture(canvas, skins, previewed, PREVIEW_SIZE)
        };
        if let Some(compiled) = self.skin_screens.get_mut(screen) {
            compiled.set_preview_texture(tex);
        }
    }

    /// The rectangle each row of the browser's document-drawn option panel occupies this frame, as
    /// `(row, rectangle)` pairs placed on the canvas.
    ///
    /// Empty unless the document has actually taken the panel over, because until then the panel the
    /// player clicks is the one the overlay draws itself.
    ///
    /// The rows are resolved against the panel alone ([`OptionPanelState`]) rather than against the
    /// browser's whole frame, so a document that gated one of its rows on something else -- the
    /// focused chart, say -- would have that row answered here as though the gate were shut.
    pub(crate) fn option_row_rects(&self, canvas: &Canvas<'_>) -> Vec<(usize, Rect)> {
        if !self.screen_content(SKIN_TYPE_MUSIC_SELECT).select.options {
            return Vec::new();
        }
        let Some(compiled) = self.skin_screens.get(SKIN_TYPE_MUSIC_SELECT) else {
            return Vec::new();
        };
        let offsets = self.skin_offsets(SKIN_TYPE_MUSIC_SELECT);
        let rows = crate::app_options::options_rows(self);
        let state = OptionPanelState { rows: &rows, now_ms: self.skin_now_ms(), offsets: &offsets };
        let frame = SkinFrame {
            now_ms: state.now_ms,
            timers: &self.skin_timers,
            state: &state,
            lua: None,
            mouse: document_cursor(self.cursor, canvas.size(), compiled.authored_size()),
            background: None,
            extra: FrameExtra::None,
        };
        (0..OPTIONS_ROW_COUNT)
            .filter_map(|row| compiled.object_rect_on_screen(&frame, canvas.size(), &format!("option-row-{row}-value")).map(|rect| (row, rect)))
            .collect()
    }

    /// Which play document the practice panel borrows its rows from, and how many rows it asks for.
    ///
    /// `None` when the chart's mode has no document, or the document declares no practice pane, and
    /// then the panel keeps drawing its own rows.
    pub(crate) fn practice_document(&self) -> Option<(i32, usize)> {
        let screen = rbms_skin::loader::mode_skin_type(self.mode)?;
        if !self.skin_document_is_enabled(screen) {
            return None;
        }
        let visible = self.skin_screens.get(screen)?.practice_visible_items()?;
        Some((screen, visible.max(0) as usize))
    }

    /// Draws the practice panel with the play document of the chart's mode: its background layer,
    /// then its foreground layer, which is where the rows it bound to the practice ids are.
    ///
    /// `true` when it drew, and the panel's own rows stand aside. The rows are ordinary objects of
    /// the document, so the two layer passes are what puts them on screen; what makes them practice
    /// rows is the state behind them, which answers those ids and nothing else.
    pub(crate) fn draw_practice_skin(&self, canvas: &mut Canvas<'_>, screen: i32, rows: &PracticeRows<'_>) -> bool {
        let offsets = self.skin_offsets(screen);
        let state = PracticeViewState { rows, now_ms: self.skin_now_ms(), offsets: Some(&offsets) };
        let mut lua = None;
        let inputs = FrameInputs { background: None, offsets: &offsets, extra: FrameExtra::Practice(rows) };
        let Some(document) = self.skin_frame(canvas, screen, &state, &mut lua, inputs) else {
            return false;
        };
        canvas.clear(Color::BLACK);
        with_render_ctx(|ctx| {
            document.draw_layer(ctx, canvas, &state, SkinLayer::Background);
            document.draw_layer(ctx, canvas, &state, SkinLayer::Foreground);
        });
        true
    }
}

#[cfg(test)]
mod tests;
