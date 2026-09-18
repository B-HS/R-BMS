//! One gate per screen: draw the selected document, or say so and let the built-in layout draw.
//!
//! Every screen keeps the layout it always had. These functions are the only place that decides
//! between the two, so "no document selected" is one branch rather than five, and a screen with no
//! document produces exactly the pixels it produced before this module existed.
//!
//! The screen-specific part is not the branch, though: it is which timers a screen owns. A document
//! animates against the reference's timer ids, and something has to switch those on at the moments
//! the reference switches them. [`PlayTimers`] and [`SelectTimers`] do that from the same snapshots
//! the built-in screens are drawn from.

use rbms_skin::dst::OffsetSource;
use rbms_skin::model::SkinLayer;
use rbms_skin::property::SkinStateSource;
use rbms_skin::timer::{TimerId, TimerState, timer_id};

use super::state::{DecideViewState, FrameExtra, KeyConfigViewState, PlayViewState, ResultViewState, SelectViewState};
use super::{SkinExprEval, SkinFrame, SkinScreen};
use crate::ctx::RenderCtx;
use crate::hud::HudView;
use crate::result::{ResultView, TargetView};
use crate::select::SelectView;
use crate::{Renderer, TextureId};

/// A full gauge, in the percent the HUD and result views carry it as.
const GAUGE_FULL: f32 = 100.0;

/// Everything a screen needs to draw the document it has, beyond the game state itself.
pub struct SkinDraw<'a> {
    pub screen: &'a SkinScreen,
    pub timers: &'a TimerState,
    /// The clock this frame is drawn against.
    pub now_ms: i64,
    pub lua: Option<&'a dyn SkinExprEval>,
    pub mouse: Option<(f32, f32)>,
    pub background: Option<TextureId>,
    /// The player's nudges for this document, when any have been made.
    pub offsets: Option<&'a dyn OffsetSource>,
    /// The screen-shaped state the document's own field, wheel and graphs read, or
    /// [`FrameExtra::None`] from a screen that draws only scalar objects.
    pub extra: FrameExtra<'a>,
}

impl std::fmt::Debug for SkinDraw<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SkinDraw").field("now_ms", &self.now_ms).field("objects", &self.screen.object_count()).finish_non_exhaustive()
    }
}

impl SkinDraw<'_> {
    /// Draws the document with `state` answering its property reads, and answers how many objects
    /// reached the screen.
    pub fn draw<R: Renderer>(&self, ctx: &mut RenderCtx<'_>, r: &mut R, state: &dyn SkinStateSource) -> usize {
        let frame =
            SkinFrame { now_ms: self.now_ms, timers: self.timers, state, lua: self.lua, mouse: self.mouse, background: self.background, extra: self.extra };
        self.screen.draw(ctx, r, &frame)
    }

    /// Draws one document phase with the frame state used for a full draw.
    pub fn draw_layer<R: Renderer>(&self, ctx: &mut RenderCtx<'_>, r: &mut R, state: &dyn SkinStateSource, layer: SkinLayer) -> usize {
        let frame =
            SkinFrame { now_ms: self.now_ms, timers: self.timers, state, lua: self.lua, mouse: self.mouse, background: self.background, extra: self.extra };
        self.screen.draw_layer(ctx, r, &frame, layer)
    }
}

/// The play screen. Answers false when no document is selected, which is the caller's cue to draw
/// its own field, HUD and covers.
pub fn render_play_screen<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, document: Option<&SkinDraw<'_>>, state: &PlayViewState<'_>) -> bool {
    draw_with(ctx, r, document, state)
}

/// The song browser.
///
/// The option panel's rows travel with the frame rather than with the view, because they are the
/// application's own configuration rows and not something the browser measured; they are lifted back
/// out here so a document reads them through the same state source as everything else.
pub fn render_select_screen<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, document: Option<&SkinDraw<'_>>, view: &SelectView) -> bool {
    let Some(document) = document else {
        return false;
    };
    let options = document.extra.select().and_then(|list| list.options);
    let state = SelectViewState::new(view, document.now_ms, document.offsets, options);
    draw_with(ctx, r, Some(document), &state)
}

/// The score screen.
pub fn render_result_screen<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    document: Option<&SkinDraw<'_>>,
    view: &ResultView,
    target: Option<&TargetView>,
    cleared: bool,
) -> bool {
    let Some(document) = document else {
        return false;
    };
    let state = ResultViewState::new(view, target, cleared, document.now_ms, document.offsets);
    draw_with(ctx, r, Some(document), &state)
}

/// The loading screen, which stands in for the reference's decide screen.
///
/// The caller hands the state in rather than the few values it is made of, because what the screen
/// can say about the chart it is starting -- its genre, its subtitle, its artist, its level -- comes
/// from the library rather than from the load, and rebuilding the state here would answer every one
/// of those with nothing.
pub fn render_decide_screen<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, document: Option<&SkinDraw<'_>>, state: &DecideViewState<'_>) -> bool {
    draw_with(ctx, r, document, state)
}

/// The key configuration screen.
pub fn render_keyconfig_screen<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, document: Option<&SkinDraw<'_>>, keys: &[String]) -> bool {
    let Some(document) = document else {
        return false;
    };
    let state = KeyConfigViewState { keys, now_ms: document.now_ms, offsets: document.offsets };
    draw_with(ctx, r, Some(document), &state)
}

/// The branch every screen shares: draw the document, or report that there is none.
fn draw_with<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, document: Option<&SkinDraw<'_>>, state: &dyn SkinStateSource) -> bool {
    match document {
        Some(document) => {
            document.draw(ctx, r, state);
            true
        }
        None => false,
    }
}

/// The first key a side numbers from, and the key its high band numbers from.
const FIRST_KEY: u8 = 1;

/// The key the reference moves into its second, wider band of lane timer ids.
const TENTH_KEY: u8 = 10;

/// The highest key either band names, beyond which the reference publishes nothing.
const LAST_KEY: u8 = 99;

/// How many lanes a bitmask of the previous frame's lane states holds.
const LANES_REMEMBERED: usize = u128::BITS as usize;

/// One kind of lane event, as the reference publishes it: a turntable id and two runs of key ids per
/// side.
///
/// The reference numbers keys one to nine straight after the turntable and then jumps to a second
/// band for keys ten and up, which is why one band cannot be a single base plus an offset.
struct LaneTimerBand {
    /// The turntable's id on each side.
    scratch: [i32; 2],
    /// The id each side's first key takes, with keys two to nine following it.
    key: [i32; 2],
    /// The id each side's tenth key takes, with the rest up to [`LAST_KEY`] following it.
    high_key: [i32; 2],
}

impl LaneTimerBand {
    /// The id one lane of one side publishes in this band, or `None` for a key past [`LAST_KEY`].
    fn at(&self, side: usize, key: u8) -> Option<TimerId> {
        let side = usize::from(side > 0);
        Some(match key {
            0 => TimerId(self.scratch[side]),
            key if key < TENTH_KEY => TimerId(self.key[side] + i32::from(key - FIRST_KEY)),
            key if key <= LAST_KEY => TimerId(self.high_key[side] + i32::from(key - TENTH_KEY)),
            _ => return None,
        })
    }
}

/// Where a key bomb burns.
const BOMB_BAND: LaneTimerBand = LaneTimerBand {
    scratch: [timer_id::BOMB_1P_SCRATCH.get(), timer_id::BOMB_2P_SCRATCH.get()],
    key: [timer_id::BOMB_1P_KEY1.get(), timer_id::BOMB_2P_KEY1.get()],
    high_key: [timer_id::BOMB_1P_KEY10.get(), timer_id::BOMB_2P_KEY10.get()],
};

/// Where a long note is being held down.
const HOLD_BAND: LaneTimerBand = LaneTimerBand {
    scratch: [timer_id::HOLD_1P_SCRATCH.get(), timer_id::HOLD_2P_SCRATCH.get()],
    key: [timer_id::HOLD_1P_KEY1.get(), timer_id::HOLD_2P_KEY1.get()],
    high_key: [timer_id::HOLD_1P_KEY10.get(), timer_id::HOLD_2P_KEY10.get()],
};

/// Where a lane went down.
const KEYON_BAND: LaneTimerBand = LaneTimerBand {
    scratch: [timer_id::KEYON_1P_SCRATCH.get(), timer_id::KEYON_2P_SCRATCH.get()],
    key: [timer_id::KEYON_1P_KEY1.get(), timer_id::KEYON_2P_KEY1.get()],
    high_key: [timer_id::KEYON_1P_KEY10.get(), timer_id::KEYON_2P_KEY10.get()],
};

/// Where a lane came back up.
const KEYOFF_BAND: LaneTimerBand = LaneTimerBand {
    scratch: [timer_id::KEYOFF_1P_SCRATCH.get(), timer_id::KEYOFF_2P_SCRATCH.get()],
    key: [timer_id::KEYOFF_1P_KEY1.get(), timer_id::KEYOFF_2P_KEY1.get()],
    high_key: [timer_id::KEYOFF_1P_KEY10.get(), timer_id::KEYOFF_2P_KEY10.get()],
};

/// The judgement pop-up timer of each side.
const JUDGE_TIMERS: [TimerId; 2] = [timer_id::JUDGE_1P, timer_id::JUDGE_2P];

/// The neutral pose of each side's character, which a run starts in.
const CHARA_NEUTRAL_TIMERS: [TimerId; 2] = [timer_id::PM_CHARA_1P_NEUTRAL, timer_id::PM_CHARA_2P_NEUTRAL];

/// Every character timer the end of a chart switches off, so the win and loss poses the music-end
/// timer drives have the screen to themselves (`BMSPlayer`, where the run reaches its finished
/// state).
const CHARA_TIMERS: [TimerId; 9] = [
    timer_id::PM_CHARA_1P_NEUTRAL,
    timer_id::PM_CHARA_1P_FEVER,
    timer_id::PM_CHARA_1P_GREAT,
    timer_id::PM_CHARA_1P_GOOD,
    timer_id::PM_CHARA_1P_BAD,
    timer_id::PM_CHARA_2P_NEUTRAL,
    timer_id::PM_CHARA_2P_GREAT,
    timer_id::PM_CHARA_2P_BAD,
    timer_id::PM_CHARA_DANCE,
];

/// How full the gauge has to be before the first player's character celebrates rather than merely
/// approves. The reference asks its gauge whether it sits at its own maximum, which is the full bar
/// the HUD reports.
const CHARA_FEVER_GAUGE: f32 = GAUGE_FULL;

/// The last judgement index the character treats as a good hit, and the one it treats as a fair
/// one, in the order `HudView::last_judge` reports them.
const CHARA_JUDGE_GREAT_LAST: u8 = 1;
const CHARA_JUDGE_GOOD: u8 = 2;

/// Which character timer one judgement restarts on `side`.
///
/// The first player's character reacts to the run directly and celebrates on a full gauge. The
/// second player's is the opponent, so the mapping is the reference's inverted one: the run going
/// well is what makes it look bad.
fn chara_reaction(side: usize, judge: u8, gauge: f32) -> TimerId {
    if side > 0 {
        return if judge <= CHARA_JUDGE_GOOD { timer_id::PM_CHARA_2P_BAD } else { timer_id::PM_CHARA_2P_GREAT };
    }
    match judge {
        judge if judge <= CHARA_JUDGE_GREAT_LAST && gauge >= CHARA_FEVER_GAUGE => timer_id::PM_CHARA_1P_FEVER,
        judge if judge <= CHARA_JUDGE_GREAT_LAST => timer_id::PM_CHARA_1P_GREAT,
        CHARA_JUDGE_GOOD => timer_id::PM_CHARA_1P_GOOD,
        _ => timer_id::PM_CHARA_1P_BAD,
    }
}

/// The combo timer of each side.
const COMBO_TIMERS: [TimerId; 2] = [timer_id::COMBO_1P, timer_id::COMBO_2P];

/// One lane as the play timers read it.
///
/// `side` and `key` are where the lane sits on the screen rather than where it sits in the chart: a
/// turntable is key zero of its own side however the chart numbers it, and the keys of each field
/// are numbered from one going right. That is the numbering the reference's timer ids carry, so a
/// document written against `KEYON_2P_KEY1` lights the first key of the right-hand field.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LaneTimerState {
    /// `0` for the left-hand field, `1` for the right-hand one.
    pub side: u8,
    /// `0` for a turntable, `1` and up for the keys of its own field.
    pub key: u8,
    /// Whether the lane is being held down.
    pub down: bool,
    /// Whether a long note is being held in it.
    pub hold: bool,
    /// Whether its key bomb is still burning.
    pub bomb: bool,
}

/// Where the run's lanes and its last judgement stand this frame.
pub struct PlayLanes<'a> {
    /// One entry per lane of the chart, in the chart's own lane order.
    pub lanes: &'a [LaneTimerState],
    /// Which field the last judgement landed in. A single-field run always answers the left one, so
    /// its document never sees the right-hand band switch on.
    pub judged_side: usize,
}

/// What the play screen's timers were last switched against.
///
/// A document animates a judgement pop-up, a combo flash and a gauge flash by measuring from the
/// moment each timer switched on, so each one has to be restarted at the frame the thing it reports
/// happened. That is an edge, and an edge needs the previous frame; this holds it.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlayTimers {
    /// How many inputs had been judged.
    judged: u32,
    combo: u32,
    gauge: f32,
    /// Which lanes were down, one bit each, so a press and a release are edges rather than states.
    /// Lanes past [`LANES_REMEMBERED`] are not remembered, which no mode reaches.
    down: u128,
    /// Which lanes had a bomb burning, the same way.
    bombs: u128,
    /// Whether a frame has been seen at all, so the first one does not read as a change from zero.
    seen: bool,
}

impl PlayTimers {
    /// A memory with nothing seen yet.
    pub fn new() -> PlayTimers {
        PlayTimers::default()
    }

    /// Switches the timers that mark the run starting: play on, ready off.
    pub fn start(&mut self, timers: &mut TimerState, now_ms: i64) {
        timers.set_off(timer_id::READY);
        timers.set_on(timer_id::PLAY, now_ms);
        timers.set_off(timer_id::MUSIC_END);
        for neutral in CHARA_NEUTRAL_TIMERS {
            timers.set_on(neutral, now_ms);
        }
    }

    /// Switches the timer that marks the run being failed.
    pub fn fail(&mut self, timers: &mut TimerState, now_ms: i64) {
        timers.set_on(timer_id::FAILED, now_ms);
    }

    /// Switches this frame's timers from what changed since the last one.
    ///
    /// `total_notes` is what a full combo is measured against; a run whose chart has none never
    /// reports one. The gauge and the full combo belong to the run rather than to a field, so they
    /// stay on the left-hand band however many fields the run has.
    pub fn update(&mut self, timers: &mut TimerState, hud: &HudView<'_>, total_notes: u32, now_ms: i64, play: &PlayLanes<'_>) {
        let side = usize::from(play.judged_side > 0);
        let judged: u32 = hud.counts.iter().sum();
        if self.seen && judged > self.judged {
            timers.set_on(JUDGE_TIMERS[side], now_ms);
            if let Some(judge) = hud.last_judge {
                timers.set_on(chara_reaction(side, judge, hud.gauge), now_ms);
            }
        }
        if hud.combo > self.combo {
            timers.set_on(COMBO_TIMERS[side], now_ms);
        } else if hud.combo == 0 {
            for combo in COMBO_TIMERS {
                timers.set_off(combo);
            }
        }
        timers.switch(timer_id::FULLCOMBO_1P, total_notes > 0 && hud.combo >= total_notes, now_ms);
        if self.seen && hud.gauge > self.gauge {
            timers.set_on(timer_id::GAUGE_INCLEASE_1P, now_ms);
        }
        timers.switch(timer_id::GAUGE_MAX_1P, hud.gauge >= GAUGE_FULL, now_ms);
        self.update_lanes(timers, play.lanes, now_ms);
        update_chara_band(timers, total_notes > 0 && judged >= total_notes, now_ms);

        self.judged = judged;
        self.combo = hud.combo;
        self.gauge = hud.gauge;
        self.seen = true;
    }

    /// Switches the per-lane timers a document lights its keys, its bombs and its held long notes
    /// from.
    ///
    /// A press and a release are edges, so each restarts its own timer and switches the other off,
    /// exactly as the reference does. A held long note is a state and keeps the moment the hold
    /// began. A bomb is an edge too, taken from whether one is burning: two hits in the same lane
    /// inside one bomb's own window read as the one bomb, which at the length a bomb burns for is a
    /// frame or two of a repeated flash rather than a restarted one.
    fn update_lanes(&mut self, timers: &mut TimerState, lanes: &[LaneTimerState], now_ms: i64) {
        let mut down = 0u128;
        let mut bombs = 0u128;
        for (lane, state) in lanes.iter().enumerate().take(LANES_REMEMBERED) {
            let bit = 1u128 << lane;
            down |= u128::from(state.down) << lane;
            bombs |= u128::from(state.bomb) << lane;
            let side = usize::from(state.side);
            if let Some(hold) = HOLD_BAND.at(side, state.key) {
                timers.switch(hold, state.hold, now_ms);
            }
            if let Some(bomb) = BOMB_BAND.at(side, state.key) {
                match (state.bomb, self.bombs & bit != 0) {
                    (true, false) => timers.set_on(bomb, now_ms),
                    (false, true) => timers.set_off(bomb),
                    _ => {}
                }
            }
            let (Some(on), Some(off)) = (KEYON_BAND.at(side, state.key), KEYOFF_BAND.at(side, state.key)) else {
                continue;
            };
            match (state.down, self.down & bit != 0) {
                (true, false) => {
                    timers.set_off(off);
                    timers.set_on(on, now_ms);
                }
                (false, true) => {
                    timers.set_off(on);
                    timers.set_on(off, now_ms);
                }
                _ => {}
            }
        }
        self.down = down;
        self.bombs = bombs;
    }
}

/// Switches the character band from whether the chart still has notes to judge.
///
/// While it does, both sides rest in their neutral pose: the pose is a state rather than an edge, so
/// a run that was already going when the screen was entered still has a character on screen. A
/// reaction does not switch the neutral timer off here -- the renderer holds the pose back for as
/// long as the reaction it interrupted is still playing, because how long that is comes from the
/// definition file rather than from the run.
///
/// The reference marks the end of a chart by the play clock passing the last note and clears the
/// whole band there; every note having been judged is the same moment reported by the state the play
/// screen already hands over, and it is what the win and loss poses are measured from.
fn update_chara_band(timers: &mut TimerState, finished: bool, now_ms: i64) {
    timers.switch(timer_id::MUSIC_END, finished, now_ms);
    for neutral in CHARA_NEUTRAL_TIMERS {
        timers.switch(neutral, !finished, now_ms);
    }
    if !finished {
        return;
    }
    for timer in CHARA_TIMERS {
        timers.set_off(timer);
    }
}

/// What the browser's timers were last switched against.
#[derive(Debug, Default, Clone, Copy)]
pub struct SelectTimers {
    row: usize,
    seen: bool,
}

impl SelectTimers {
    /// A memory with nothing seen yet.
    pub fn new() -> SelectTimers {
        SelectTimers::default()
    }

    /// Switches the bar-movement timers from which row is focused now.
    ///
    /// The reference restarts the shared movement timer on every change and the directional one for
    /// the way the list went, which is what lets a document animate the wheel turning.
    pub fn update(&mut self, timers: &mut TimerState, row: usize, now_ms: i64) {
        if self.seen && row != self.row {
            timers.set_on(timer_id::SONGBAR_MOVE, now_ms);
            timers.set_on(timer_id::SONGBAR_CHANGE, now_ms);
            let moved: TimerId = if row > self.row { timer_id::SONGBAR_MOVE_DOWN } else { timer_id::SONGBAR_MOVE_UP };
            timers.set_on(moved, now_ms);
        }
        self.row = row;
        self.seen = true;
    }
}

/// How long after the score screen is entered the graph timer that marks the end of the trend
/// animation switches on. The reference runs the gauge trend in over about a second, and a document
/// measures the end of that run from this timer.
const RESULT_GRAPH_MS: i64 = 1_000;

/// What the score screen's timers were last switched against.
///
/// A document draws the gauge trend growing from the moment the screen opened and marks the score it
/// reports when that score is a new best, so the screen has to say when it opened and whether the
/// run it is reporting beat what was there before.
///
/// The run is measured once, when the screen is entered, and the summary it reports never moves
/// afterwards: rbms has no rank reveal for a key to skip and no second submission that rewrites the
/// score on screen. So the score timer is settled on entry too, rather than watched for a change
/// across frames that cannot happen.
#[derive(Debug, Default, Clone, Copy)]
pub struct ResultTimers;

impl ResultTimers {
    /// A memory with nothing seen yet.
    pub fn new() -> ResultTimers {
        ResultTimers
    }

    /// Switches the timers that mark the screen opening: the trend begins now, has not ended yet,
    /// and the score timer is on exactly when the run set a new best. A chart with no score behind
    /// it counts as a new best, because every run on it is the best there has been.
    pub fn enter(&mut self, timers: &mut TimerState, view: &ResultView, now_ms: i64) {
        timers.set_on(timer_id::RESULTGRAPH_BEGIN, now_ms);
        timers.set_off(timer_id::RESULTGRAPH_END);
        let record = view.prev_best_ex.is_none_or(|best| view.ex_score > best);
        timers.switch(timer_id::RESULT_UPDATESCORE, record, now_ms);
    }

    /// Switches this frame's timers: the trend ends a second after the screen opened.
    pub fn update(&mut self, timers: &mut TimerState, now_ms: i64) {
        let trend_over = timers.elapsed(timer_id::RESULTGRAPH_BEGIN, now_ms).is_some_and(|open_for| open_for >= RESULT_GRAPH_MS);
        timers.switch(timer_id::RESULTGRAPH_END, trend_over, now_ms);
    }
}
