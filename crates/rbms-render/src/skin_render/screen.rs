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
use rbms_skin::property::SkinStateSource;
use rbms_skin::timer::{TimerId, TimerState, timer_id};

use super::state::{DecideViewState, KeyConfigViewState, PlayViewState, ResultViewState, SelectViewState};
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
        let frame = SkinFrame { now_ms: self.now_ms, timers: self.timers, state, lua: self.lua, mouse: self.mouse, background: self.background };
        self.screen.draw(ctx, r, &frame)
    }
}

/// The play screen. Answers false when no document is selected, which is the caller's cue to draw
/// its own field, HUD and covers.
pub fn render_play_screen<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, document: Option<&SkinDraw<'_>>, state: &PlayViewState<'_>) -> bool {
    draw_with(ctx, r, document, state)
}

/// The song browser.
pub fn render_select_screen<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, document: Option<&SkinDraw<'_>>, view: &SelectView) -> bool {
    let Some(document) = document else {
        return false;
    };
    let state = SelectViewState { view, now_ms: document.now_ms, offsets: document.offsets };
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
    let state = ResultViewState { view, target, cleared, now_ms: document.now_ms, offsets: document.offsets };
    draw_with(ctx, r, Some(document), &state)
}

/// The loading screen, which stands in for the reference's decide screen.
pub fn render_decide_screen<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, document: Option<&SkinDraw<'_>>, progress: f32, done: bool, title: &str) -> bool {
    let Some(document) = document else {
        return false;
    };
    let state = DecideViewState { progress, done, title, now_ms: document.now_ms, offsets: document.offsets };
    draw_with(ctx, r, Some(document), &state)
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
    }

    /// Switches the timer that marks the run being failed.
    pub fn fail(&mut self, timers: &mut TimerState, now_ms: i64) {
        timers.set_on(timer_id::FAILED, now_ms);
    }

    /// Switches this frame's timers from what changed since the last one.
    ///
    /// `total_notes` is what a full combo is measured against; a run whose chart has none never
    /// reports one.
    pub fn update(&mut self, timers: &mut TimerState, hud: &HudView<'_>, total_notes: u32, now_ms: i64) {
        let judged: u32 = hud.counts.iter().sum();
        if self.seen && judged > self.judged {
            timers.set_on(timer_id::JUDGE_1P, now_ms);
        }
        if hud.combo > self.combo {
            timers.set_on(timer_id::COMBO_1P, now_ms);
        } else if hud.combo == 0 {
            timers.set_off(timer_id::COMBO_1P);
        }
        timers.switch(timer_id::FULLCOMBO_1P, total_notes > 0 && hud.combo >= total_notes, now_ms);
        if self.seen && hud.gauge > self.gauge {
            timers.set_on(timer_id::GAUGE_INCLEASE_1P, now_ms);
        }
        timers.switch(timer_id::GAUGE_MAX_1P, hud.gauge >= GAUGE_FULL, now_ms);

        self.judged = judged;
        self.combo = hud.combo;
        self.gauge = hud.gauge;
        self.seen = true;
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
