//! The timers and the events a document defines for itself, and the clicks its own objects answer.
//!
//! Three things a document may declare that nothing else in this module draws. `customTimers` are
//! timers the document computes rather than the player switching them, `customEvents` are actions
//! it fires when a condition of its own turns true, and an `image` or `imageset` with an `act` is a
//! rectangle a click runs one of those events from.
//!
//! The split of work follows the reference implementation's. A document answers everything it can
//! by itself -- its own timers, its own expressions -- and what is left is a numbered event the
//! player has to carry out, which leaves here as a [`SkinEventRequest`] rather than as a call into
//! any screen. That is what keeps this module free of the browser, the settings and the stages.

use std::cell::Cell;

use rbms_skin::dst::{LuaExprId, WarnOnce};
use rbms_skin::model::{PropertyRef, SkinDef};
use rbms_skin::property::SkinStateSource;
use rbms_skin::timer::{TimerId, TimerState};

use super::state::{SkinHotAction, SkinHotspot};
use super::{SkinAssets, SkinExprEval};
use crate::Rect;

/// One step forward, which is what the first mouse button carries (`SkinObject.mousePressed`'s
/// `buttonEvents`).
const STEP_FORWARD: i32 = 1;

/// One step back, which is the half of a split rectangle the reference numbers first.
const STEP_BACK: i32 = -1;

/// The step an event fired by its own condition carries (`Event.exec(state)`, which fills both
/// arguments with zero).
const CONDITION_STEP: i32 = 0;

/// `click`: the whole rectangle steps forward.
const CLICK_FORWARD: i32 = 0;

/// `click`: the whole rectangle steps back.
const CLICK_BACK: i32 = 1;

/// `click`: the left half steps back and the right half forward.
const CLICK_HORIZONTAL: i32 = 2;

/// `click`: the lower half steps back and the upper half forward, measured in the document's own
/// space, where a rectangle is measured upwards from its foot.
const CLICK_VERTICAL: i32 = 3;

/// How many custom events one fired event may chain through before the player stops following it.
///
/// A document may name another of its own events as an action, and nothing stops it naming itself.
/// The reference follows that chain with the call stack and falls over; this counts instead.
const MAX_EVENT_DEPTH: usize = 8;

/// The lowest key assignment event the reference numbers, and the count of them in its first band
/// (`EventFactory`'s `keyassign<N>`).
const KEYASSIGN_LOW: std::ops::RangeInclusive<i32> = 101..=139;

/// Its second band of key assignments, which the reference moves to after the first runs out.
const KEYASSIGN_HIGH: std::ops::RangeInclusive<i32> = 150..=164;

/// The practice panel's own rows, which the reference dispatches through the state like any other
/// numbered event (`SkinProperty.BUTTON_PRACTICE_ITEM1` and the fifteen after it).
const PRACTICE_ITEMS: std::ops::RangeInclusive<i32> = 370..=385;

/// Every event the reference's own table names, in its numbering (`EventFactory.EventType`).
///
/// A document may declare a `customEvents` entry with one of these numbers; the reference still
/// answers with the built-in one, because `EventFactory.getEvent` reaches its table before it
/// reaches the state's custom map. The list is what decides which side of that a number falls on,
/// not what this build can carry out: an event named here that no screen answers is warned about
/// where it is dispatched, so a document is told the difference between a number nobody knows and
/// one this player has no action for.
const BUILTIN_EVENTS: &[i32] = &[
    10, 11, 12, 13, 14, 15, 16, 17, 19, 40, 42, 43, 54, 55, 57, 59, 72, 73, 74, 75, 77, 78, 79, 89, 90, 210, 211, 212, 213, 308, 312, 315, 316, 317, 318, 321,
    322, 323, 324, 330, 331, 332, 340, 341, 342, 343, 344, 350, 351, 352, 353, 360, 361,
];

/// Whether a number is one of the reference's own events rather than one a document defines.
pub fn is_builtin_event(id: i32) -> bool {
    BUILTIN_EVENTS.contains(&id) || KEYASSIGN_LOW.contains(&id) || KEYASSIGN_HIGH.contains(&id) || PRACTICE_ITEMS.contains(&id)
}

/// Fires once when a document names an event nothing in it declares.
static UNKNOWN_EVENT: WarnOnce = WarnOnce::new();

/// Fires once when a document's own events chain deeper than [`MAX_EVENT_DEPTH`].
static EVENT_CHAIN: WarnOnce = WarnOnce::new();

/// A number a document wrote where the player expected either a state id or an expression.
///
/// The same two forms every expression-typed field of a document takes. What the number means is
/// the field's own business: a timer id, an option id, or an event number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinRef {
    /// The number the document wrote.
    Id(i32),
    /// An expression it wrote instead, compiled once when the screen was built.
    Lua(LuaExprId),
}

impl SkinRef {
    /// Compiles one field, or `None` when the document left it out or its expression would not
    /// compile.
    fn compile(field: Option<&PropertyRef>, assets: &mut dyn SkinAssets) -> Option<SkinRef> {
        match field? {
            PropertyRef::Id(id) => Some(SkinRef::Id(*id)),
            PropertyRef::Expr(source) => assets.expression(source).map(SkinRef::Lua),
        }
    }
}

/// What a click on one of a document's own rectangles asks for.
///
/// The step is settled when the rectangle is offered rather than when it is clicked: a `click` of
/// two or three splits the rectangle in half and each half carries its own step, which is the same
/// answer the reference reaches by comparing the click with the middle of the rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinEventClick {
    /// The event the object named, as the document wrote it.
    pub act: SkinRef,
    /// The step this half of the rectangle carries, [`STEP_FORWARD`] or [`STEP_BACK`].
    pub step: i32,
}

/// One numbered event a document fired that only the player can carry out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinEventRequest {
    /// The reference's own number for the event.
    pub id: i32,
    /// The step it was fired with: `1` or `-1` from a click, `0` from a condition.
    pub step: i32,
}

/// What one frame of the document's own timers and events is run against.
pub struct SkinEventFrame<'a> {
    /// The clock the frame is drawn on, which is the clock a timer's moment is measured against.
    pub now_ms: i64,
    pub state: &'a dyn SkinStateSource,
    /// The compiled expressions, when the host has a sandbox.
    pub lua: Option<&'a dyn SkinExprEval>,
}

impl std::fmt::Debug for SkinEventFrame<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SkinEventFrame").field("now_ms", &self.now_ms).finish_non_exhaustive()
    }
}

/// One object a click runs an event from.
#[derive(Debug)]
struct ClickTarget {
    /// The object id, which is what ties the rectangle drawn this frame back to this entry.
    id: String,
    act: SkinRef,
    click: i32,
}

impl ClickTarget {
    /// The rectangles this object offers, given where it was drawn this frame.
    fn spots(&self, rect: Rect) -> Vec<SkinHotspot> {
        let whole = |step: i32| vec![spot(rect, self.act, step)];
        match self.click {
            CLICK_FORWARD => whole(STEP_FORWARD),
            CLICK_BACK => whole(STEP_BACK),
            CLICK_HORIZONTAL => {
                let half = rect.w / 2.0;
                vec![
                    spot(Rect::new(rect.x, rect.y, half, rect.h), self.act, STEP_BACK),
                    spot(Rect::new(rect.x + half, rect.y, rect.w - half, rect.h), self.act, STEP_FORWARD),
                ]
            }
            CLICK_VERTICAL => {
                let half = rect.h / 2.0;
                vec![
                    spot(Rect::new(rect.x, rect.y, rect.w, half), self.act, STEP_BACK),
                    spot(Rect::new(rect.x, rect.y + half, rect.w, rect.h - half), self.act, STEP_FORWARD),
                ]
            }
            _ => Vec::new(),
        }
    }
}

/// One clickable rectangle, with the step that half of it carries.
fn spot(rect: Rect, act: SkinRef, step: i32) -> SkinHotspot {
    SkinHotspot { rect, action: SkinHotAction::Event(SkinEventClick { act, step }) }
}

/// One timer a document computes for itself (`CustomTimer`).
#[derive(Debug)]
struct CustomTimer {
    id: TimerId,
    /// What answers when the timer switched on. A timer with none is passive: only an action moves
    /// it, and this never writes over what one set.
    value: Option<SkinRef>,
}

/// One event a document defines for itself (`CustomEvent`).
#[derive(Debug)]
struct CustomEvent {
    id: i32,
    action: Option<SkinRef>,
    /// What makes the event fire by itself. An event with none only ever fires when something names
    /// it: a click, or another event's action.
    condition: Option<SkinRef>,
    min_interval_ms: i64,
    /// When it last fired, so the interval is measured from that rather than from the screen
    /// opening. A never-fired event has no last moment and always fires the first time its
    /// condition holds, which is the reference's `Long.MIN_VALUE` test.
    fired_at: Cell<Option<i64>>,
}

impl CustomEvent {
    /// Whether enough time has passed since this last fired.
    fn may_fire(&self, now_ms: i64) -> bool {
        self.fired_at.get().is_none_or(|last| now_ms - last >= self.min_interval_ms)
    }
}

/// Everything one document declares that is fired rather than drawn.
#[derive(Debug, Default)]
pub struct DocumentEvents {
    clicks: Vec<ClickTarget>,
    timers: Vec<CustomTimer>,
    events: Vec<CustomEvent>,
}

impl DocumentEvents {
    /// Reads one document's `customTimers`, `customEvents` and clickable objects, compiling every
    /// expression they carry into `assets`.
    ///
    /// Nothing here fails the screen: an expression that will not compile, a timer that would
    /// shadow a built-in one and a `click` value this build has no meaning for each drop the one
    /// entry involved and leave a line in `warnings`.
    pub fn build(def: &SkinDef, assets: &mut dyn SkinAssets, warnings: &mut Vec<String>) -> DocumentEvents {
        let mut events = DocumentEvents::default();
        for timer in &def.custom_timers {
            let id = TimerId(timer.id);
            if id.is_builtin() {
                warnings.push(format!("custom timer {} is numbered as a built-in timer and is never read", timer.id));
                continue;
            }
            let value = match &timer.timer {
                Some(field) => match SkinRef::compile(Some(field), assets) {
                    Some(value) => Some(value),
                    None => {
                        warnings.push(format!("custom timer {} has an expression this build could not compile", timer.id));
                        continue;
                    }
                },
                None => None,
            };
            events.timers.push(CustomTimer { id, value });
        }

        for event in &def.custom_events {
            if is_builtin_event(event.id) {
                warnings.push(format!("custom event {} is numbered as a built-in event and is never reached", event.id));
                continue;
            }
            events.events.push(CustomEvent {
                id: event.id,
                action: SkinRef::compile(event.action.as_ref(), assets),
                condition: SkinRef::compile(event.condition.as_ref(), assets),
                min_interval_ms: i64::from(event.min_interval),
                fired_at: Cell::new(None),
            });
        }

        let images = def.image.iter().map(|image| (&image.id, image.act.as_ref(), image.click));
        let sets = def.imageset.iter().map(|set| (&set.id, set.act.as_ref(), set.click));
        for (id, act, click) in images.chain(sets) {
            let Some(act) = SkinRef::compile(act, assets) else {
                continue;
            };
            if !matches!(click, CLICK_FORWARD | CLICK_BACK | CLICK_HORIZONTAL | CLICK_VERTICAL) {
                warnings.push(format!("object {id:?} asks for click {click}, which is not a kind this build answers"));
                continue;
            }
            events.clicks.push(ClickTarget { id: id.clone(), act, click });
        }
        events
    }

    /// Whether the document declared none of the three.
    pub fn is_empty(&self) -> bool {
        self.clicks.is_empty() && self.timers.is_empty() && self.events.is_empty()
    }

    /// How many objects offer a click.
    pub fn click_count(&self) -> usize {
        self.clicks.len()
    }

    /// The rectangles this frame offers a click on, in the order the objects were drawn.
    ///
    /// `drawn` is every object that reached the screen this frame, in draw order, so an object the
    /// document gated off or faded out is not in it and is not clickable either -- the same rule
    /// the reference keeps by testing `obj.draw`. The caller hit-tests the last match first, which
    /// is the topmost object.
    pub fn hotspots(&self, drawn: &[(&str, Rect)]) -> Vec<SkinHotspot> {
        let mut spots = Vec::new();
        for (id, rect) in drawn {
            for target in self.clicks.iter().filter(|target| target.id == *id) {
                spots.extend(target.spots(*rect));
            }
        }
        spots
    }

    /// Runs the document's own timers and then the events whose condition holds, in the order the
    /// reference updates them (`Skin.updateCustomObjects`).
    ///
    /// Answers the numbered events that were fired and that only the player can carry out.
    pub fn update(&self, timers: &mut TimerState, frame: &SkinEventFrame<'_>) -> Vec<SkinEventRequest> {
        for timer in &self.timers {
            let Some(value) = timer.value else {
                continue;
            };
            timers.set_at(timer.id, self.moment(value, frame));
        }

        let mut requests = Vec::new();
        for event in &self.events {
            let Some(condition) = event.condition else {
                continue;
            };
            if !self.holds(condition, frame) || !event.may_fire(frame.now_ms) {
                continue;
            }
            event.fired_at.set(Some(frame.now_ms));
            self.run(event.action, CONDITION_STEP, timers, frame, &mut requests, 0);
        }
        requests
    }

    /// Answers one click that landed on a rectangle [`DocumentEvents::hotspots`] offered.
    pub fn click(&self, click: SkinEventClick, timers: &mut TimerState, frame: &SkinEventFrame<'_>) -> Vec<SkinEventRequest> {
        let mut requests = Vec::new();
        self.run(Some(click.act), click.step, timers, frame, &mut requests, 0);
        requests
    }

    /// Carries out one action: an expression for its effects, a built-in number for the player, or
    /// another of the document's own events.
    fn run(&self, action: Option<SkinRef>, step: i32, timers: &mut TimerState, frame: &SkinEventFrame<'_>, out: &mut Vec<SkinEventRequest>, depth: usize) {
        let Some(action) = action else {
            return;
        };
        if depth >= MAX_EVENT_DEPTH {
            if EVENT_CHAIN.should_warn() {
                eprintln!("skin events chained past {MAX_EVENT_DEPTH} of the document's own; the rest of the chain is not followed");
            }
            return;
        }
        match action {
            SkinRef::Lua(expr) => {
                let Some(lua) = frame.lua else {
                    return;
                };
                for request in lua.run_action(expr) {
                    timers.apply(request, frame.now_ms);
                }
            }
            SkinRef::Id(id) if is_builtin_event(id) => out.push(SkinEventRequest { id, step }),
            SkinRef::Id(id) => {
                let Some(event) = self.events.iter().find(|event| event.id == id) else {
                    if UNKNOWN_EVENT.should_warn() {
                        eprintln!("skin event {id} is neither a built-in one nor declared by the document; nothing happens");
                    }
                    return;
                };
                event.fired_at.set(Some(frame.now_ms));
                self.run(event.action, step, timers, frame, out, depth + 1);
            }
        }
    }

    /// When the timer a field stands for switched on, or `None` while it is off.
    fn moment(&self, value: SkinRef, frame: &SkinEventFrame<'_>) -> Option<i64> {
        match value {
            SkinRef::Id(id) => frame.state.timer(id),
            SkinRef::Lua(expr) => frame.lua?.eval_timer(expr),
        }
    }

    /// Whether one condition holds this frame. A condition needing an evaluator the host does not
    /// have reads as false, the same way a draw condition does.
    fn holds(&self, condition: SkinRef, frame: &SkinEventFrame<'_>) -> bool {
        match condition {
            SkinRef::Id(id) => frame.state.boolean(id),
            SkinRef::Lua(expr) => frame.lua.and_then(|lua| lua.eval_draw(expr)).unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests;
