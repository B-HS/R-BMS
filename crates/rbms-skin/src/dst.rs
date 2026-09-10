//! Destination tracks: what a skin object looks like at a given moment, and whether it is drawn at
//! all.
//!
//! A destination track is a timer plus a list of keyframes. [`resolve`] answers "where, what colour
//! and at what angle is this object now", following `SkinObject.prepareRegion` and its `getRate`
//! helper step for step. [`prepare`] wraps it with the draw gating of `SkinObject.prepare`: the
//! option and Lua conditions first, then the region, then the pointer test.
//!
//! Nothing here reaches for a renderer or for Lua itself. Game state arrives through
//! [`DrawStateSource`], compiled Lua expressions through [`LuaDrawEval`], so this module is
//! testable with plain fakes and compiles without the `lua` feature.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::timer::{TimerId, TimerState};

/// `loop` value that plays the animation once and then stops drawing it, rather than repeating.
pub const LOOP_ONCE: i64 = -1;

/// Where the reference parks a finished one-shot animation (`SkinObject.java:361-364`). Every keyframe
/// time a document can express is at or above zero, so the `starttime > time` test that follows
/// drops the object.
const ONCE_EXPIRED_TIME: i64 = -1;

/// `stretch` value a document leaves unset, which the renderer reads as plain stretch-to-fit.
pub const STRETCH_UNSPECIFIED: i32 = -1;

/// A rectangle in skin coordinates: the document's own pixel space, before any screen scaling.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkinRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl SkinRect {
    /// A rectangle from its position and size.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

/// A straight 8-bit colour. The reference carries these as floats and only quantises when it hands
/// them to the GPU, so an interpolated channel here is that float rounded to nearest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkinColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl SkinColor {
    /// A colour from its four channels.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// A live nudge to a destination, addressed by offset id (`SkinObject.SkinOffset`).
///
/// `x`/`y`/`w`/`h` move and resize the region, `r` turns it and `a` shifts its alpha in the same
/// 0..=255 units as [`SkinColor::a`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SkinOffset {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub r: f32,
    pub a: f32,
}

/// Where a destination reads its live offsets from.
pub trait OffsetSource {
    /// The offset registered under `id`, or `None` when the game sets none.
    fn offset(&self, id: i32) -> Option<SkinOffset>;
}

/// The game state a draw condition consults.
///
/// The property registry implements this over the whole skin state; the negation rule for a
/// negative id lives in [`Self::boolean`], not in this module.
pub trait DrawStateSource: OffsetSource {
    /// The option under `id`. A negative id reads `abs(id)` and negates the answer
    /// (`BooleanPropertyFactory.getBooleanProperty`), and an id the build does not implement reads
    /// as `false`.
    fn boolean(&self, id: i32) -> bool;
}

/// A handle to a Lua expression the loader compiled once at load time.
///
/// Opaque on purpose: this module never parses or runs Lua, it only asks [`LuaDrawEval`] for the
/// answer, so the interpolator stands on its own without the `lua` feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LuaExprId(pub u32);

/// Evaluates the compiled Lua expressions a document used in place of option ids.
pub trait LuaDrawEval {
    /// The expression's value for this frame, or `None` when it raised or ran past its budget. A
    /// failed expression hides its object rather than failing the skin.
    fn eval_draw(&self, expr: LuaExprId) -> Option<bool>;
}

/// How a keyframe pair is interpolated (`SkinObject.getRate`, `SkinObject.java:545-575`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Acc {
    /// Constant rate.
    #[default]
    Linear,
    /// Ease in, `r * r`.
    Accelerate,
    /// Ease out, `1 - (r - 1)^2`.
    Decelerate,
    /// No interpolation: every value holds at the keyframe until the next one.
    Step,
}

impl Acc {
    /// Reads the document's integer. Anything outside 1..=3 interpolates linearly, as the
    /// reference's `switch` does by falling through.
    pub const fn from_id(value: i32) -> Self {
        match value {
            1 => Self::Accelerate,
            2 => Self::Decelerate,
            3 => Self::Step,
            _ => Self::Linear,
        }
    }

    /// Shapes a 0..=1 progress. [`Acc::Step`] shapes it linearly like the reference, which drops
    /// the rate later rather than in `getRate`.
    pub fn apply(self, rate: f32) -> f32 {
        match self {
            Self::Accelerate => rate * rate,
            Self::Decelerate => 1.0 - (rate - 1.0) * (rate - 1.0),
            Self::Linear | Self::Step => rate,
        }
    }

    /// Whether values hold at their keyframe instead of being interpolated.
    pub const fn is_step(self) -> bool {
        matches!(self, Self::Step)
    }
}

/// One keyframe of a destination track.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe {
    /// Milliseconds from the moment the track's timer switched on.
    pub time_ms: i64,
    pub rect: SkinRect,
    /// The scissor rectangle in force at this keyframe, if the document set one.
    pub clip: Option<SkinRect>,
    pub acc: Acc,
    pub color: SkinColor,
    pub angle_deg: f32,
}

/// The pointer rectangle a destination is only drawn under, relative to its own region
/// (`SkinObject.java:603-607`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl MouseRect {
    /// Whether a region-relative point is inside, edges included, as libGDX's `Rectangle.contains`
    /// has it.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.x <= x && self.x + self.w >= x && self.y <= y && self.y + self.h >= y
    }
}

/// One condition an object must meet to be drawn at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawCondition {
    /// An option id, sign included. The negation of a negative id belongs to
    /// [`DrawStateSource::boolean`]; this carries the id exactly as the document wrote it.
    Option(i32),
    /// A compiled Lua expression.
    Lua(LuaExprId),
}

/// A skin object's animation: which timer it follows, how it loops, and what it looks like along
/// the way.
#[derive(Debug, Clone)]
pub struct DestinationTrack {
    /// The timer keyframe times are measured from. `None` measures from the caller's clock.
    pub timer: Option<TimerId>,
    /// [`LOOP_ONCE`] plays once and stops; any other value repeats from that millisecond to the
    /// last keyframe.
    pub loop_ms: i64,
    pub blend: i32,
    pub filter: i32,
    pub center: i32,
    /// Offset ids applied to every resolved value, in order.
    pub offsets: Vec<i32>,
    /// Not a document field. The play screen's loader sets it on judge-count objects alone, and it
    /// keeps their offsets from moving the region's origin (`JsonPlaySkinObjectLoader.java:260`).
    pub relative: bool,
    /// Keyframes in ascending time order, which the loader guarantees.
    pub frames: Vec<Keyframe>,
    /// Draw conditions. An empty list always draws.
    pub draw_conditions: Vec<DrawCondition>,
    /// The pointer rectangle the object is drawn under, if the document set one.
    pub mouse_rect: Option<MouseRect>,
    /// The document's `stretch`, or [`STRETCH_UNSPECIFIED`].
    pub stretch: i32,
}

impl Default for DestinationTrack {
    /// An empty track with the defaults a document that names none carries: no timer, a loop point
    /// at zero that repeats the whole animation, and an unspecified stretch.
    fn default() -> Self {
        Self {
            timer: None,
            loop_ms: 0,
            blend: 0,
            filter: 0,
            center: 0,
            offsets: Vec::new(),
            relative: false,
            frames: Vec::new(),
            draw_conditions: Vec::new(),
            mouse_rect: None,
            stretch: STRETCH_UNSPECIFIED,
        }
    }
}

impl DestinationTrack {
    /// The acceleration the whole track interpolates with.
    ///
    /// The reference keeps one `acc` per object and the first non-zero one declared wins
    /// (`SkinObject.java:218-220`), so a track whose keyframes disagree follows its first shaped
    /// keyframe. Keyframes carry their own value here because that is how a document writes them.
    pub fn effective_acc(&self) -> Acc {
        self.frames.iter().map(|frame| frame.acc).find(|acc| *acc != Acc::Linear).unwrap_or(Acc::Linear)
    }
}

/// What a destination looks like on one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    pub rect: SkinRect,
    /// The scissor rectangle, already offset and known to have a positive size.
    pub clip: Option<SkinRect>,
    pub color: SkinColor,
    pub angle_deg: f32,
}

/// A latch that lets a warning fire once for the life of the process, so a per-frame fault does not
/// fill the log.
#[derive(Debug, Default)]
pub struct WarnOnce(AtomicBool);

impl WarnOnce {
    /// A latch that has not fired.
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    /// Claims the one warning. True exactly once per latch.
    pub fn should_warn(&self) -> bool {
        !self.0.swap(true, Ordering::Relaxed)
    }
}

/// Fires once when a document carries Lua draw conditions but the caller passed no evaluator.
static MISSING_LUA_EVALUATOR: WarnOnce = WarnOnce::new();

/// Turns a document's integer `op` list into draw conditions, the way `SkinObject.setDrawCondition`
/// does.
///
/// A zero op is "no condition" and is dropped, a repeated op is dropped, and an op no property
/// implements is dropped rather than read as false, which is what keeps an object the build does
/// not understand visible instead of silently hiding it. `is_known` is asked about the positive id.
pub fn draw_conditions_from_ops<F: FnMut(i32) -> bool>(ops: &[i32], mut is_known: F) -> Vec<DrawCondition> {
    let mut seen: Vec<i32> = Vec::with_capacity(ops.len());
    let mut conditions = Vec::with_capacity(ops.len());
    for &op in ops {
        if op == 0 || seen.contains(&op) {
            continue;
        }
        seen.push(op);
        if is_known(op.saturating_abs()) {
            conditions.push(DrawCondition::Option(op));
        }
    }
    conditions
}

/// The keyframe an instant sits on and how far it has travelled towards the next one
/// (`SkinObject.getRate`).
fn frame_index_and_rate(frames: &[Keyframe], now_ms: i64, acc: Acc) -> (usize, f32) {
    let last = frames.len() - 1;
    let mut later = frames[last].time_ms;
    if now_ms == later {
        return (last, 0.0);
    }
    for index in (0..last).rev() {
        let earlier = frames[index].time_ms;
        if earlier <= now_ms && later > now_ms {
            let span = later - earlier;
            let rate = (now_ms - earlier) as f32 / span as f32;
            return (index, acc.apply(rate));
        }
        later = earlier;
    }
    (0, 0.0)
}

/// Linear blend of two rectangles.
fn lerp_rect(from: SkinRect, to: SkinRect, rate: f32) -> SkinRect {
    SkinRect { x: from.x + (to.x - from.x) * rate, y: from.y + (to.y - from.y) * rate, w: from.w + (to.w - from.w) * rate, h: from.h + (to.h - from.h) * rate }
}

/// Linear blend of one colour channel, rounded back to 8 bits.
fn lerp_channel(from: u8, to: u8, rate: f32) -> u8 {
    let value = f32::from(from) + (f32::from(to) - f32::from(from)) * rate;
    value.clamp(0.0, f32::from(u8::MAX)).round() as u8
}

/// Linear blend of two colours.
fn lerp_color(from: SkinColor, to: SkinColor, rate: f32) -> SkinColor {
    SkinColor {
        r: lerp_channel(from.r, to.r, rate),
        g: lerp_channel(from.g, to.g, rate),
        b: lerp_channel(from.b, to.b, rate),
        a: lerp_channel(from.a, to.a, rate),
    }
}

/// Applies one offset to a region or clip rectangle.
///
/// A relative track takes the size change alone: its position is already stated relative to
/// something the offset moved.
fn apply_offset(rect: &mut SkinRect, offset: &SkinOffset, relative: bool) {
    if !relative {
        rect.x += offset.x - offset.w / 2.0;
        rect.y += offset.y - offset.h / 2.0;
    }
    rect.w += offset.w;
    rect.h += offset.h;
}

/// The colour every keyframe shares, if they all share one.
///
/// The reference caches that case and it is the only fixed-value cache with an observable effect:
/// it applies the offsets' alpha where the interpolating path drops them
/// (`SkinObject.java:480-518`).
fn uniform_color(frames: &[Keyframe]) -> Option<SkinColor> {
    let first = frames.first()?.color;
    frames.iter().all(|frame| frame.color == first).then_some(first)
}

/// Shifts an alpha by an offset's, in 0..=255 units, clamped like the reference's 0..=1 clamp.
fn offset_alpha(alpha: u8, shift: f32) -> u8 {
    (f32::from(alpha) + shift).clamp(0.0, f32::from(u8::MAX)).round() as u8
}

/// Where a destination is on this frame, or `None` when its timer is off or its animation has not
/// started (`SkinObject.java:348-433`, with `prepareClip`, `prepareColor` and `prepareAngle` at 435, 480 and 526).
///
/// This looks at the timer and the keyframes alone. Whether the object is drawn at all is
/// [`prepare`]'s question.
pub fn resolve(track: &DestinationTrack, now_ms: i64, timers: &TimerState, offsets: &dyn OffsetSource) -> Option<Resolved> {
    let last = track.frames.len().checked_sub(1)?;
    let mut time = now_ms;
    if let Some(timer) = track.timer {
        time -= timers.get(timer)?;
    }

    let end = track.frames[last].time_ms;
    if track.loop_ms == LOOP_ONCE {
        if time > end {
            time = ONCE_EXPIRED_TIME;
        }
    } else if end > 0 && time > track.loop_ms {
        time = if end == track.loop_ms { track.loop_ms } else { (time - track.loop_ms) % (end - track.loop_ms) + track.loop_ms };
    }
    if track.frames[0].time_ms > time {
        return None;
    }

    let applied: Vec<SkinOffset> = if track.offsets.is_empty() { Vec::new() } else { track.offsets.iter().filter_map(|id| offsets.offset(*id)).collect() };

    let acc = track.effective_acc();
    let (index, rate) = frame_index_and_rate(&track.frames, time, acc);
    let frame = &track.frames[index];
    let interpolated = rate != 0.0 && !acc.is_step() && index < last;

    let mut rect = if interpolated { lerp_rect(frame.rect, track.frames[index + 1].rect, rate) } else { frame.rect };
    for offset in &applied {
        apply_offset(&mut rect, offset, track.relative);
    }

    let clip = frame.clip.map(|clip| {
        let mut clip = match interpolated.then(|| track.frames[index + 1].clip) {
            Some(Some(next)) => lerp_rect(clip, next, rate),
            _ => clip,
        };
        for offset in &applied {
            apply_offset(&mut clip, offset, track.relative);
        }
        clip
    });
    let clip = clip.filter(|clip| clip.w > 0.0 && clip.h > 0.0);

    let uniform = uniform_color(&track.frames);
    let mut color = match uniform {
        Some(color) => color,
        None if interpolated => lerp_color(frame.color, track.frames[index + 1].color, rate),
        None => frame.color,
    };
    if uniform.is_some() || rate == 0.0 {
        for offset in &applied {
            color.a = offset_alpha(color.a, offset.a);
        }
    }

    let mut angle_deg = if interpolated { (frame.angle_deg + (track.frames[index + 1].angle_deg - frame.angle_deg) * rate).trunc() } else { frame.angle_deg };
    for offset in &applied {
        angle_deg = (angle_deg + offset.r).trunc();
    }

    Some(Resolved { rect, clip, color, angle_deg })
}

/// Whether one draw condition holds this frame.
fn condition_holds(condition: DrawCondition, state: &dyn DrawStateSource, lua: Option<&dyn LuaDrawEval>) -> bool {
    match condition {
        DrawCondition::Option(id) => state.boolean(id),
        DrawCondition::Lua(expr) => match lua {
            Some(lua) => lua.eval_draw(expr).unwrap_or(false),
            None => {
                if MISSING_LUA_EVALUATOR.should_warn() {
                    eprintln!("skin draw conditions need Lua but no evaluator was supplied; those objects stay hidden");
                }
                false
            }
        },
    }
}

/// Everything a skin object needs before it is drawn, or `None` when it is not drawn this frame
/// (`SkinObject.prepare`, `SkinObject.java:591-611`).
///
/// The order is the reference's: every draw condition first, so a hidden object never resolves its
/// region; then the region, moved by `offset_xy`; then the pointer test against the moved region.
/// A track with a pointer rectangle and no pointer position is not drawn.
///
/// `lua` may be `None` while no evaluator exists yet; Lua conditions then read as false and warn
/// once.
pub fn prepare(
    track: &DestinationTrack,
    now_ms: i64,
    timers: &TimerState,
    state: &dyn DrawStateSource,
    lua: Option<&dyn LuaDrawEval>,
    offset_xy: (f32, f32),
    mouse: Option<(f32, f32)>,
) -> Option<Resolved> {
    for condition in &track.draw_conditions {
        if !condition_holds(*condition, state, lua) {
            return None;
        }
    }

    let mut resolved = resolve(track, now_ms, timers, state)?;
    let (offset_x, offset_y) = offset_xy;
    resolved.rect.x += offset_x;
    resolved.rect.y += offset_y;
    if let Some(clip) = resolved.clip.as_mut() {
        clip.x += offset_x;
        clip.y += offset_y;
    }

    if let Some(rect) = track.mouse_rect {
        let (x, y) = mouse?;
        if !rect.contains(x - resolved.rect.x, y - resolved.rect.y) {
            return None;
        }
    }

    Some(resolved)
}

#[cfg(test)]
mod tests;
