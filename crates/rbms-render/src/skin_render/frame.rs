//! What one frame of a skin is drawn from.
//!
//! A document addresses most of the running game by property id, and [`SkinFrame::state`] answers
//! those. What an id cannot carry travels in [`FrameData`]: the note field, the song bars, the
//! series a graph plots and the images a document only refers to. Each of those is a capability a
//! frame may or may not have rather than a kind of screen, so every part is filled on its own and a
//! part left empty leaves the objects that read it undrawn. That is what lets a gauge be drawn on a
//! score screen and a tempo graph on the browser: the screen fills the part, and the object asks
//! for nothing else.
//!
//! The shape of each part belongs to the module that draws from it -- [`LaneNotes`] to the note
//! field, [`SongBars`] to the wheel, each series to its graph -- and this module only gathers them.
//!
//! A frame is made in two stages, as the reference makes one (`Skin.drawAllObjects`). The first
//! prepares every object in the order the document declared them: its draw conditions, its timer,
//! where it sits, and the values it shows. Only when the last object has been prepared does the
//! second stage draw the ones the first left standing, in the same order. A skin's Lua is asked in
//! the first stage alone, which is what lets a skin hang per-frame work on a `draw` function and
//! rely on every such function having run before anything reaches the screen. What it answered is
//! kept in a [`PreparedFrame`] and read back from there while drawing, so the second stage needs no
//! interpreter at all and can run after the host has been unbound from it.

use std::cell::{Cell, RefCell};
use std::ops::Range;

use rbms_skin::dst::{LuaDrawEval, LuaFnId, Resolved, WarnOnce};
use rbms_skin::property::SkinHost;
use rbms_skin::timer::{TIMER_OFF, TimerState};

pub use super::bga::{
    BgaEvent, BgaExpand, BgaFrame, BgaPick, BgaPicture, BgaPlayhead, BgaShow, BgaTextures, DEFAULT_MISS_LAYER_DURATION_MS, MISS_LAYER_NONE, SMALL_PICTURE_EDGE,
    key_out_black, on_small_canvas,
};
pub use super::gauge::{GAUGE_TYPES, GaugeFrame, GaugeScale};
use super::graphs::{BpmTimeline, GaugeHistory, NoteDistribution, RecentHits, TimingHistogram};
pub use super::notes::{
    ConstantScroll, JUDGE_AREA_WINDOWS, JudgeArea, LaneLong, LaneNotes, LaneOffsets, NOTE_UNJUDGED, NoteDisplay, NoteStates, Unplayed, current_duration_ms,
    first_lane_rect, fixed_hispeed, lane_offsets,
};
use super::object::SkinObject;
use super::refs::ReferenceImages;
use super::songlist::SongBars;
pub use super::songlist::{
    BarDistribution, BarHold, BarKind, BarScroll, BarScroller, BarTrophy, LAMP_KINDS, RANK_KINDS, SCROLL_DURATION_HIGH_MS, SCROLL_DURATION_LOW_MS, SongBar,
};
use super::text_input::TextEntry;
use super::{SkinViewport, draw};
use crate::Renderer;
use crate::ctx::RenderCtx;

/// The series a frame carries for the objects that plot a run or a chart rather than one number.
///
/// Every one is optional and independent of the others, and none belongs to a screen: whichever
/// screen knows a series fills it.
#[derive(Debug, Default, Clone, Copy)]
pub struct FrameSeries<'a> {
    /// How a run's gauge moved, for the gauge graph: the samples of every gauge the run could have
    /// been played on, half a second apart. Which of them is plotted, and where it clears, is the
    /// frame's [`FrameData::gauge`], so a gauge graph needs both.
    pub gauge_history: Option<GaugeHistory<'a>>,
    /// How a run's hits were spread around their notes, a millisecond to a count, with the run's
    /// mean, its deviation and its judgement windows, for the timing distribution graph.
    pub timing: Option<TimingHistogram<'a>>,
    /// Where a chart's tempo changes, for the tempo graph.
    pub bpm: Option<BpmTimeline<'a>>,
    /// How a chart's notes are spread second by second, for the judgement graph: by kind of note,
    /// by the judgement each took, and by that judgement with early and late apart.
    pub notes: Option<NoteDistribution<'a>>,
    /// The hits a run has taken most recently, for the two visualisers.
    pub recent_hits: Option<RecentHits<'a>>,
}

/// How many judgement regions a play screen can have: one for each player the reference keeps a
/// judge timer for (`JudgeManager.JUDGE_TIMER`, timers 46, 47 and 247).
pub const JUDGE_REGIONS: usize = 3;

/// What one judgement region last reported (`JudgeManager.judgenow` and `judgecombo`).
///
/// A region is a share of the lanes: a play screen with one judge object has one region that every
/// lane reports to, and a double screen with two has its left field report to region zero and its
/// right field to region one (`lane / (lanes / regions)`). Each keeps the last judgement given in it
/// and the combo the run stood at then, and nothing ever clears either: whether the pop-up is still
/// on show is left to the timers its own destinations follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgeHit {
    /// Which judgement it was, best first: 0 a perfect great, 1 a great, 2 a good, 3 a bad, 4 a
    /// poor and 5 a miss.
    pub judgement: usize,
    /// The combo the run stood at when it landed, carried across the stages of a course
    /// (`JudgeManager.getCourseCombo`). This is what the pop-up counts, not the live combo.
    pub combo: i32,
    /// When it landed, on the frame clock in microseconds: the moment the region's judge timer and
    /// its combo timer are switched on at. The pop-up is timed by those timers and not by this, so
    /// this is for whoever keeps them.
    pub at_us: i64,
}

/// The judgement regions of a play frame, by region number, for the judge object.
///
/// [`JudgeFrame::default`] has judged nothing anywhere, which is what every screen but a play
/// screen passes and what a play screen passes until its first note is judged.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JudgeFrame {
    pub regions: [Option<JudgeHit>; JUDGE_REGIONS],
}

impl JudgeFrame {
    /// What the region a judge object follows last reported, or `None` when nothing has been judged
    /// there or the object names a region no play screen has (`JudgeManager.getNowJudge`).
    pub fn region(&self, index: i32) -> Option<JudgeHit> {
        usize::try_from(index).ok().and_then(|index| self.regions.get(index).copied().flatten())
    }

    /// The same regions with `hit` as the last report of region `index`. A region past the last one
    /// is ignored.
    pub fn with_region(mut self, index: usize, hit: JudgeHit) -> JudgeFrame {
        if let Some(region) = self.regions.get_mut(index) {
            *region = Some(hit);
        }
        self
    }
}

/// Everything a frame carries beside its scalar property source.
///
/// [`FrameData::default`] carries nothing at all, which is what a screen that draws only scalar
/// objects passes; a screen with more to say fills the parts it has and leaves the rest.
#[derive(Default, Clone, Copy)]
pub struct FrameData<'a> {
    /// The chart under the play head, for the note object: the timelines, where the play head is,
    /// what the run has done to each note and how the field scrolls. A frame that leaves it out
    /// draws no note field.
    pub notes: Option<&'a LaneNotes<'a>>,
    /// The gauge a screen shows -- which one, how full, and the limits of every gauge of the run --
    /// for the gauge object and the gauge graph. A play screen fills it for the run in progress and
    /// a score screen for the one that ended.
    pub gauge: Option<GaugeFrame>,
    /// The browser's bars, for the song wheel: the list on show, where the cursor is in it and how
    /// far the wheel is through sliding. A frame that leaves it out draws no wheel, and so does one
    /// whose list is empty.
    pub bars: Option<&'a SongBars<'a>>,
    pub series: FrameSeries<'a>,
    /// The images a document refers to rather than ships.
    pub images: ReferenceImages,
    /// What the `bga` object shows this frame.
    pub bga: BgaFrame,
    /// The editable text being typed into, which draws what is typed in place of what it shows.
    pub entry: Option<TextEntry<'a>>,
    /// What each judgement region last reported, for the judge object. Only a play screen has any.
    pub judge: JudgeFrame,
}

impl std::fmt::Debug for FrameData<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FrameData")
            .field("notes", &self.notes)
            .field("gauge", &self.gauge)
            .field("bars", &self.bars.is_some())
            .field("series", &self.series)
            .field("images", &self.images)
            .field("bga", &self.bga)
            .field("entry", &self.entry)
            .field("judge", &self.judge)
            .finish()
    }
}

/// Everything one frame of a skin needs from the running game.
pub struct SkinFrame<'a> {
    /// The clock the frame is drawn against, in microseconds, the same one every timer is measured
    /// on.
    pub now_us: i64,
    pub timers: &'a TimerState,
    pub state: &'a dyn SkinHost,
    /// The skin's Lua bound to this frame, when the skin has an interpreter. It is asked while the
    /// frame is prepared and never while it is drawn, so a frame handed to the draw stage alone may
    /// leave it out.
    pub lua: Option<&'a dyn LuaDrawEval>,
    /// Where the pointer is in document coordinates, for the objects a document gated on it.
    pub mouse: Option<(f32, f32)>,
    /// The state a property id cannot carry: bars, series, lane geometry and images. A screen with
    /// nothing of the kind to say passes [`FrameData::default`].
    pub data: FrameData<'a>,
}

impl<'a> SkinFrame<'a> {
    /// The evaluator as destination gating and timers ask for it.
    pub(crate) fn script(&self) -> Option<&'a dyn LuaDrawEval> {
        self.lua
    }

    /// The same frame with `lua` answering in place of its own evaluator, which is how each stage is
    /// handed the evaluator that belongs to it.
    fn staged<'b>(&self, lua: Option<&'b dyn LuaDrawEval>) -> SkinFrame<'b>
    where
        'a: 'b,
    {
        SkinFrame { now_us: self.now_us, timers: self.timers, state: self.state, lua, mouse: self.mouse, data: self.data }
    }
}

impl std::fmt::Debug for SkinFrame<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SkinFrame").field("now_us", &self.now_us).field("mouse", &self.mouse).finish_non_exhaustive()
    }
}

/// What an object asked the skin's Lua for while it was prepared.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Asked {
    /// A function value the skin handed over.
    Function(LuaFnId),
    /// A property the skin named rather than numbered.
    Name(String),
}

/// What the skin's Lua answered, in the type it was asked for.
#[derive(Debug, Clone, PartialEq)]
enum Answer {
    Boolean(bool),
    Integer(i32),
    Float(f32),
    Text(String),
    /// The microsecond a timer switched on, or [`TIMER_OFF`].
    Timer(i64),
}

/// One question the prepare stage put to the skin's Lua and the answer it got.
#[derive(Debug, Clone, PartialEq)]
struct Kept {
    asked: Asked,
    answer: Answer,
}

/// The evaluator of the prepare stage: every question goes to the interpreter, and the answer is
/// kept so the draw stage can read it back without asking again.
///
/// Nothing is shared between two questions here: each goes to the interpreter's frame and each
/// answer is kept for the object that asked. A condition or a value two objects name is called once
/// for each, as the reference calls it, because a skin may count on being called that often. A timer
/// two objects name is the one thing the interpreter's frame calls only once (`BoundFrame::timer` in
/// `rbms_skin::lua`); both objects are still given its answer.
struct Recorder<'a> {
    live: &'a dyn LuaDrawEval,
    kept: RefCell<Vec<Kept>>,
}

impl<'a> Recorder<'a> {
    fn new(live: &'a dyn LuaDrawEval) -> Recorder<'a> {
        Recorder { live, kept: RefCell::new(Vec::new()) }
    }

    /// How many answers have been kept so far, which is where the next object's answers begin.
    fn mark(&self) -> usize {
        self.kept.borrow().len()
    }

    fn keep(&self, asked: Asked, answer: Answer) {
        self.kept.borrow_mut().push(Kept { asked, answer });
    }

    /// Every answer kept, in the order the questions were asked.
    fn finish(self) -> Vec<Kept> {
        self.kept.into_inner()
    }
}

impl LuaDrawEval for Recorder<'_> {
    fn call_boolean(&self, function: LuaFnId) -> bool {
        let answer = self.live.call_boolean(function);
        self.keep(Asked::Function(function), Answer::Boolean(answer));
        answer
    }

    fn call_integer(&self, function: LuaFnId) -> i32 {
        let answer = self.live.call_integer(function);
        self.keep(Asked::Function(function), Answer::Integer(answer));
        answer
    }

    fn call_float(&self, function: LuaFnId) -> f32 {
        let answer = self.live.call_float(function);
        self.keep(Asked::Function(function), Answer::Float(answer));
        answer
    }

    fn call_text(&self, function: LuaFnId) -> String {
        let answer = self.live.call_text(function);
        self.keep(Asked::Function(function), Answer::Text(answer.clone()));
        answer
    }

    fn call_timer(&self, function: LuaFnId) -> i64 {
        let answer = self.live.call_timer(function);
        self.keep(Asked::Function(function), Answer::Timer(answer));
        answer
    }

    fn named_boolean(&self, name: &str) -> bool {
        let answer = self.live.named_boolean(name);
        self.keep(Asked::Name(name.to_owned()), Answer::Boolean(answer));
        answer
    }

    fn named_integer(&self, name: &str) -> i32 {
        let answer = self.live.named_integer(name);
        self.keep(Asked::Name(name.to_owned()), Answer::Integer(answer));
        answer
    }

    fn named_float(&self, name: &str) -> f32 {
        let answer = self.live.named_float(name);
        self.keep(Asked::Name(name.to_owned()), Answer::Float(answer));
        answer
    }

    fn named_text(&self, name: &str) -> String {
        let answer = self.live.named_text(name);
        self.keep(Asked::Name(name.to_owned()), Answer::Text(answer.clone()));
        answer
    }
}

/// What the prepare stage settled about one frame of one screen: which of its objects are drawn and
/// where, and everything the skin's Lua answered along the way.
///
/// It belongs to the screen and the frame it was prepared from. Drawing another screen with it
/// draws nothing that screen did not prepare.
#[derive(Debug, Clone, Default)]
pub struct PreparedFrame {
    /// One entry per object of the screen, in document order: where it is drawn, or `None` when its
    /// conditions, its timer or its own value left it out of the frame.
    placed: Vec<Option<Resolved>>,
    /// Which of [`Self::kept`] each object asked for, by the same index.
    asked_by: Vec<Range<usize>>,
    kept: Vec<Kept>,
    /// Whether the frame was prepared with an interpreter to ask.
    scripted: bool,
}

impl PreparedFrame {
    /// How many objects were prepared, which is every object of the screen.
    pub fn object_count(&self) -> usize {
        self.placed.len()
    }

    /// How many objects the prepare stage left to be drawn. An object counted here may still put
    /// nothing on screen: one with no source for this frame, or one faded to nothing.
    pub fn visible_count(&self) -> usize {
        self.placed.iter().filter(|placed| placed.is_some()).count()
    }

    /// How many answers the skin's Lua gave while the frame was prepared.
    pub fn answer_count(&self) -> usize {
        self.kept.len()
    }

    /// Where each object of the screen was left, in document order: its region when it is drawn
    /// this frame, which is what a pointer event is judged against.
    pub(crate) fn placed(&self) -> &[Option<Resolved>] {
        &self.placed
    }
}

/// Fires once when the draw stage asks for something the prepare stage never did.
static UNPREPARED_READ: WarnOnce = WarnOnce::new();

/// The evaluator of the draw stage: it answers from what the prepare stage kept and never reaches
/// the interpreter.
///
/// An object is answered from its own questions only. When it asked the same one more than once the
/// last answer stands, which is the state the reference's object is left in when its `prepare`
/// returns. A question the object never asked while it was prepared reads as the fallback of its
/// type and is counted, because it means an object's prepare and its draw have drifted apart.
struct Replay<'a> {
    prepared: &'a PreparedFrame,
    /// The object being drawn.
    object: Cell<usize>,
    unprepared: Cell<usize>,
}

impl<'a> Replay<'a> {
    fn new(prepared: &'a PreparedFrame) -> Replay<'a> {
        Replay { prepared, object: Cell::new(0), unprepared: Cell::new(0) }
    }

    /// Moves on to the object at `index`, whose answers the next questions are read from.
    fn enter(&self, index: usize) {
        self.object.set(index);
    }

    /// How many questions had no answer kept for them.
    fn unprepared(&self) -> usize {
        self.unprepared.get()
    }

    /// The last answer the object being drawn was given to a question `matches` recognises.
    fn recall<T>(&self, matches: impl Fn(&Kept) -> Option<T>) -> Option<T> {
        let own = self.prepared.asked_by.get(self.object.get()).cloned().unwrap_or_default();
        let found = self.prepared.kept.get(own).and_then(|kept| kept.iter().rev().find_map(matches));
        if found.is_none() {
            self.unprepared.set(self.unprepared.get() + 1);
        }
        found
    }

    /// The last answer `function` gave the object being drawn, read with `read`.
    fn function<T>(&self, function: LuaFnId, read: impl Fn(&Answer) -> Option<T>) -> Option<T> {
        self.recall(|kept| if kept.asked == Asked::Function(function) { read(&kept.answer) } else { None })
    }

    /// The last answer the property called `name` gave the object being drawn, read with `read`.
    fn name<T>(&self, name: &str, read: impl Fn(&Answer) -> Option<T>) -> Option<T> {
        self.recall(|kept| match &kept.asked {
            Asked::Name(asked) if asked == name => read(&kept.answer),
            _ => None,
        })
    }
}

/// An answer read as a condition, when it was asked as one.
fn as_boolean(answer: &Answer) -> Option<bool> {
    if let Answer::Boolean(value) = answer { Some(*value) } else { None }
}

/// An answer read as a whole number, when it was asked as one.
fn as_integer(answer: &Answer) -> Option<i32> {
    if let Answer::Integer(value) = answer { Some(*value) } else { None }
}

/// An answer read as a number, when it was asked as one.
fn as_float(answer: &Answer) -> Option<f32> {
    if let Answer::Float(value) = answer { Some(*value) } else { None }
}

/// An answer read as text, when it was asked as text.
fn as_text(answer: &Answer) -> Option<String> {
    if let Answer::Text(value) = answer { Some(value.clone()) } else { None }
}

/// An answer read as a timer, when it was asked as one.
fn as_timer(answer: &Answer) -> Option<i64> {
    if let Answer::Timer(value) = answer { Some(*value) } else { None }
}

impl LuaDrawEval for Replay<'_> {
    fn call_boolean(&self, function: LuaFnId) -> bool {
        self.function(function, as_boolean).unwrap_or_default()
    }

    fn call_integer(&self, function: LuaFnId) -> i32 {
        self.function(function, as_integer).unwrap_or_default()
    }

    fn call_float(&self, function: LuaFnId) -> f32 {
        self.function(function, as_float).unwrap_or_default()
    }

    fn call_text(&self, function: LuaFnId) -> String {
        self.function(function, as_text).unwrap_or_default()
    }

    fn call_timer(&self, function: LuaFnId) -> i64 {
        self.function(function, as_timer).unwrap_or(TIMER_OFF)
    }

    fn named_boolean(&self, name: &str) -> bool {
        self.name(name, as_boolean).unwrap_or_default()
    }

    fn named_integer(&self, name: &str) -> i32 {
        self.name(name, as_integer).unwrap_or_default()
    }

    fn named_float(&self, name: &str) -> f32 {
        self.name(name, as_float).unwrap_or_default()
    }

    fn named_text(&self, name: &str) -> String {
        self.name(name, as_text).unwrap_or_default()
    }
}

/// The first stage of a frame: prepares every object, in document order, before anything is drawn.
///
/// `frame.lua` is the interpreter the skin was loaded into, bound to the host for this frame. It is
/// asked here and nowhere after; with none, a function value reads as its fallback, exactly as it
/// did when a frame was a single pass.
pub(crate) fn prepare_objects(objects: &[SkinObject], frame: &SkinFrame<'_>) -> PreparedFrame {
    let recorder = frame.lua.map(Recorder::new);
    let mark = || recorder.as_ref().map_or(0, Recorder::mark);
    let staged = frame.staged(recorder.as_ref().map(|recorder| recorder as &dyn LuaDrawEval));

    let mut placed = Vec::with_capacity(objects.len());
    let mut asked_by = Vec::with_capacity(objects.len());
    for object in objects {
        let first = mark();
        placed.push(object.prepare(&staged));
        asked_by.push(first..mark());
    }
    let scripted = recorder.is_some();
    PreparedFrame { placed, asked_by, kept: recorder.map(Recorder::finish).unwrap_or_default(), scripted }
}

/// The second stage of a frame: draws what [`prepare_objects`] left standing, in the same order, and
/// answers how many objects reached the screen.
///
/// `frame.lua` is not consulted. Whatever an object reads from the skin's Lua while it draws comes
/// out of `prepared`, so this may run with the interpreter no longer bound to anything.
pub(crate) fn draw_objects<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    objects: &[SkinObject],
    viewport: &SkinViewport,
    frame: &SkinFrame<'_>,
    prepared: &PreparedFrame,
) -> usize {
    let replay = Replay::new(prepared);
    let staged = frame.staged(prepared.scripted.then_some(&replay as &dyn LuaDrawEval));

    let mut drawn = 0;
    for (index, (object, placed)) in objects.iter().zip(&prepared.placed).enumerate() {
        let Some(resolved) = placed else {
            continue;
        };
        replay.enter(index);
        if draw::draw_resolved(ctx, r, object, index, viewport, &staged, resolved) {
            drawn += 1;
        }
    }
    if replay.unprepared() > 0 && UNPREPARED_READ.should_warn() {
        eprintln!("a skin object read {} values while drawing that it had not read while being prepared; those read as their fallbacks", replay.unprepared());
    }
    drawn
}
