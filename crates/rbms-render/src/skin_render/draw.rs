//! Turning one resolved object into primitive calls.
//!
//! Each function here takes an object whose destination has already answered "where, what colour, at
//! what angle" and turns that into quads. The order of the three coordinate steps is fixed and
//! matters: the viewport first flips and scales the rectangle onto the screen, the stretch mode
//! then reshapes it there and the filter rule compares it with the source there, and the rotation
//! anchor is finally measured against the same screen rectangle, because that is the box the
//! backend turns. Everything after the viewport is in screen pixels, which is the space the
//! reference makes all three of those decisions in.

use std::borrow::Cow;

use rbms_skin::dst::{DrawStateSource, LuaDrawEval, SkinRect, prepare};
use rbms_skin::loader::{Filtering, StretchKind, filtering_for, stretch_rect};

use super::object::{
    Body, DigitLayout, FloatBody, GraphBody, ImageBody, NumberBody, SkinObject, SliderBody, Sprite, TextBody, ValueSource, fraction_glyphs, integer_glyphs,
};
use super::{MIN_TEXT_SCALE, SkinFrame, SkinViewport, TEXT_PIXELS_PER_SCALE};
use crate::ctx::RenderCtx;
use crate::{BlendMode, Color, QuadParams, Rect, Renderer, TextureFilter, UvRect, skin_center_offset};

/// A number's `align` value that leaves its places where they fall, which is flush right because a
/// blank place is a leading one (`SkinNumber.prepare`).
const NUMBER_ALIGN_RIGHT: i32 = 0;

/// A number's `align` value that pulls its places over the blanks to their left.
const NUMBER_ALIGN_LEFT: i32 = 1;

/// A text object's `align` value that starts the line at its destination's anchor
/// (`SkinText.ALIGN_LEFT`). Text numbers its alignments differently from numbers, which is why the
/// two sets are named apart.
const TEXT_ALIGN_LEFT: i32 = 0;

/// A text object's `align` value that centres the line on its destination's anchor.
const TEXT_ALIGN_CENTER: i32 = 1;

/// The document's slider `angle` for a handle that travels up the screen.
const DIRECTION_UP: i32 = 0;

/// The document's slider `angle` for a handle that travels right.
const DIRECTION_RIGHT: i32 = 1;

/// The document's slider `angle` for a handle that travels down.
const DIRECTION_DOWN: i32 = 2;

/// The document's slider `angle` for a handle that travels left.
const DIRECTION_LEFT: i32 = 3;

/// The document's graph `angle` for a bar that grows upwards. Every other value grows rightwards.
const GRAPH_VERTICAL: i32 = 1;

/// The whole numbers a property answers with when it has no value to report
/// (`IntegerPropertyFactory`, read by `SkinNumber.prepare`). An object reading one is not drawn at
/// all, rather than drawing the number itself.
const INTEGER_NO_VALUE: [i32; 2] = [i32::MIN, i32::MAX];

/// The smallest positive number Java's `Float.MIN_VALUE` names, which `SkinFloat.prepare` reads as
/// "no value" alongside `Float.MAX_VALUE`.
const FLOAT_NO_VALUE_LOW: f32 = f32::from_bits(1);

/// The alignment shift a whole number applies, which pulls its digits over the blank places to
/// their left (`SkinNumber.draw`).
const SHIFT_TOWARDS_START: f32 = -1.0;

/// The alignment shift a fractional number applies, which pushes its digits the other way
/// (`SkinFloat.draw`).
const SHIFT_TOWARDS_END: f32 = 1.0;

/// The fixed parts of one object's draw: what is being drawn, how it is tinted and turned, and where
/// the document's space lands on screen.
struct Placement<'a> {
    object: &'a SkinObject,
    blend: BlendMode,
    tint: Color,
    angle_deg: f32,
    viewport: &'a SkinViewport,
}

impl Placement<'_> {
    /// A quad in screen space, with the rotation anchor measured against it.
    fn quad(&self, dst: Rect, src: UvRect, filter: TextureFilter) -> QuadParams {
        QuadParams {
            dst,
            src,
            tint: self.tint,
            blend: self.blend,
            filter,
            angle_deg: self.angle_deg,
            center: skin_center_offset(self.object.track.center, dst.w, dst.h),
        }
    }

    /// Places one sprite cell, fitting it to the destination the way the object's stretch mode asks.
    ///
    /// Both the fit and the filter are decided in screen pixels, after the viewport has scaled the
    /// rectangle, because that is the space the reference decides them in: `Skin.setDestination`
    /// multiplies a destination into screen coordinates before `SkinObject.draw` ever compares it
    /// with the source region. A document authored at half the screen's size would otherwise draw
    /// its unresized objects at half size and pick the wrong filter for everything.
    fn cell<R: Renderer>(&self, r: &mut R, sprite: &Sprite, cell: u32, rect: SkinRect) -> bool {
        let source = sprite.cell_size();
        let dst = fitted_screen_rect(self.viewport, self.object.stretch, rect, source);
        let filter = texture_filter(filtering_for(self.object.track.filter, screen_as_skin(dst), source));
        if dst.w <= 0.0 || dst.h <= 0.0 {
            return false;
        }
        r.draw_textured_quad(sprite.tex, self.quad(dst, sprite.uv(cell), filter));
        true
    }
}

/// A screen rectangle back in the shape the skin rules take, which measure sizes and never flip.
fn screen_as_skin(rect: Rect) -> SkinRect {
    SkinRect::new(rect.x, rect.y, rect.w, rect.h)
}

/// Where a document rectangle lands on screen once its stretch mode has fitted it there.
fn fitted_screen_rect(viewport: &SkinViewport, stretch: StretchKind, rect: SkinRect, source: (f32, f32)) -> Rect {
    let placed = viewport.place(rect);
    let fitted = stretch_rect(stretch, screen_as_skin(placed), source);
    Rect { x: fitted.x, y: fitted.y, w: fitted.w, h: fitted.h }
}

/// The renderer's filter for the one the skin rules chose.
fn texture_filter(filtering: Filtering) -> TextureFilter {
    match filtering {
        Filtering::Nearest => TextureFilter::Nearest,
        Filtering::Linear => TextureFilter::Linear,
    }
}

/// Draws one object, answering whether anything reached the screen.
///
/// A fully transparent object leaves before it draws anything, which is where `SkinObject.draw`
/// leaves too. Under alpha and additive blending that only saves work, but multiply and
/// invert-destination read no source alpha at all, so an object fading out under one of those would
/// otherwise keep darkening the screen at `a: 0`.
pub(crate) fn draw_object<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, object: &SkinObject, viewport: &SkinViewport, frame: &SkinFrame<'_>) -> bool {
    let state: &dyn DrawStateSource = frame.state;
    let gate: Option<&dyn LuaDrawEval> = frame.lua.map(|lua| lua as &dyn LuaDrawEval);
    let Some(resolved) = prepare(&object.track, frame.now_ms, frame.timers, state, gate, (0.0, 0.0), frame.mouse) else {
        return false;
    };

    if resolved.color.a == 0 {
        return false;
    }

    let place =
        Placement { object, blend: BlendMode::from_skin_blend(object.track.blend), tint: resolved.color.into(), angle_deg: resolved.angle_deg, viewport };
    let clipped = resolved.clip.is_some();
    if let Some(clip) = resolved.clip {
        r.push_clip(viewport.place(clip));
    }

    let rect = resolved.rect;
    let drawn = match &object.body {
        Body::Image(body) => draw_image(r, &place, body, rect, frame),
        Body::Number(body) => draw_number(r, &place, body, rect, frame),
        Body::Float(body) => draw_float(r, &place, body, rect, frame),
        Body::Text(body) => draw_text(ctx, r, &place, body, rect, frame),
        Body::Slider(body) => draw_slider(r, &place, body, rect, frame),
        Body::Graph(body) => draw_graph(r, &place, body, rect, frame),
        Body::Background => draw_background(r, &place, rect, frame),
    };

    if clipped {
        r.pop_clip();
    }
    drawn
}

/// A still or animated image, with the variant a property picked.
fn draw_image<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &ImageBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let chosen = if body.select.is_named() { body.select.integer(frame.state, frame.lua).max(0) as usize } else { 0 };
    let Some((sprite, first, count)) = body.variants.get(chosen).or_else(|| body.variants.first()) else {
        return false;
    };
    let cell = first + sprite.animation_index(*count, frame.now_ms, frame.timers);
    place.cell(r, sprite, cell, rect)
}

/// The background image the frame supplied, at the place the document put its `bga` object.
fn draw_background<R: Renderer>(r: &mut R, place: &Placement<'_>, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let Some(tex) = frame.background else {
        return false;
    };
    let dst = place.viewport.place(rect);
    if dst.w <= 0.0 || dst.h <= 0.0 {
        return false;
    }
    let filter = r.texture_size(tex).map_or(TextureFilter::Linear, |source| crate::background_filter(dst, source));
    r.draw_textured_quad(tex, place.quad(dst, UvRect::FULL, filter));
    true
}

/// How far a run of digit places is nudged to keep its alignment.
///
/// A blank place still takes a slot, so `blanks` is what the run is shifted by: right alignment
/// leaves the places where they fall, left alignment moves the digits over the blanks, and anything
/// else splits the difference.
fn digit_shift(step: f32, blanks: usize, align: i32) -> f32 {
    match align {
        NUMBER_ALIGN_RIGHT => 0.0,
        NUMBER_ALIGN_LEFT => step * blanks as f32,
        _ => step * 0.5 * blanks as f32,
    }
}

/// The parts of a digit run that do not come from the destination.
struct DigitRun<'a> {
    space: f32,
    align: i32,
    /// Which way the alignment shift is applied.
    shift_sign: f32,
    /// How the strip is cut, which is what turns a glyph slot into a cell.
    layout: &'a DigitLayout,
    /// Which set of the strip this frame animates to.
    set: u32,
    /// Whether the value is negative, so a strip with a half of its own reads from it.
    negative: bool,
    offsets: &'a [(f32, f32, f32, f32)],
}

/// Draws one run of digit places.
fn draw_places<R: Renderer>(r: &mut R, place: &Placement<'_>, sprite: &Sprite, rect: SkinRect, places: &[Option<u32>], run: DigitRun<'_>) -> bool {
    let step = rect.w + run.space;
    let blanks = places.iter().filter(|slot| slot.is_none()).count();
    let shift = digit_shift(step, blanks, run.align) * run.shift_sign;
    let mut drawn = false;
    for (index, slot) in places.iter().enumerate() {
        let Some(cell) = slot.and_then(|glyph| run.layout.cell(run.set, run.negative, glyph)) else {
            continue;
        };
        let (dx, dy, dw, dh) = run.offsets.get(index).copied().unwrap_or((0.0, 0.0, 0.0, 0.0));
        let at = SkinRect::new(rect.x + step * index as f32 + shift + dx, rect.y + dy, rect.w + dw, rect.h + dh);
        drawn |= place.cell(r, sprite, cell, at);
    }
    drawn
}

/// A whole number.
///
/// A property with nothing to report answers one of the two sentinels rather than a number, and the
/// object then draws no places at all (`SkinNumber.prepare`).
fn draw_number<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &NumberBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let value = body.value.integer(frame.state, frame.lua);
    if INTEGER_NO_VALUE.contains(&value) {
        return false;
    }
    let places = integer_glyphs(body, value);
    let set = body.sprite.animation_index(body.layout.sets, frame.now_ms, frame.timers);
    let run = DigitRun {
        space: body.space,
        align: body.align,
        shift_sign: SHIFT_TOWARDS_START,
        layout: &body.layout,
        set,
        negative: value < 0,
        offsets: &body.offsets,
    };
    draw_places(r, place, &body.sprite, rect, places.as_slice(), run)
}

/// A fractional number.
///
/// The same "no value" guard as a whole number, plus the reference's own two sentinels for a float
/// and the case where the document asked for no places at all (`SkinFloat.prepare`).
fn draw_float<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &FloatBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let raw = body.value.float(frame.state, frame.lua);
    if raw == FLOAT_NO_VALUE_LOW || raw == f32::MAX {
        return false;
    }
    let value = f64::from(raw * body.gain);
    if !value.is_finite() {
        return false;
    }
    let places = fraction_glyphs(body, value.abs());
    if places.is_empty() {
        return false;
    }
    let set = body.sprite.animation_index(body.layout.sets, frame.now_ms, frame.timers);
    let run = DigitRun {
        space: body.space,
        align: body.align,
        shift_sign: SHIFT_TOWARDS_END,
        layout: &body.layout,
        set,
        negative: value < 0.0,
        offsets: &body.offsets,
    };
    draw_places(r, place, &body.sprite, rect, places.as_slice(), run)
}

/// A run of text, in the font the document registered for it.
///
/// The line is anchored at the destination's left edge and grows in whichever direction its
/// alignment says, and its size comes from the destination's height rather than from the font's own
/// (`SkinTextFont.draw`, which scales by `region.height / parameter.size`).
fn draw_text<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, place: &Placement<'_>, body: &TextBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let line: Cow<'_, str> = match &body.constant {
        Some(constant) => Cow::Borrowed(constant.as_str()),
        None => body.value.text(frame.state, frame.lua),
    };
    if line.is_empty() {
        return false;
    }
    match &body.family {
        Some(family) => ctx.text.set_family(family),
        None => ctx.text.reset_family(),
    }
    let dst = place.viewport.place(rect);
    let scale = (dst.h / TEXT_PIXELS_PER_SCALE).max(MIN_TEXT_SCALE);
    match body.align {
        TEXT_ALIGN_LEFT => ctx.draw_text(r, dst.x, dst.y, scale, place.tint, &line),
        TEXT_ALIGN_CENTER => ctx.draw_text_centered(r, dst.x, dst.y, scale, place.tint, &line),
        _ => ctx.draw_text_right(r, dst.x, dst.y, scale, place.tint, &line),
    }
    true
}

/// The normalised value a slider or a graph is at, whether it reads a rate or scales a whole number
/// between endpoints of its own.
fn ratio(value: &ValueSource, ref_num: Option<(i32, i32)>, frame: &SkinFrame<'_>) -> f32 {
    match ref_num {
        Some((min, max)) => ((value.integer(frame.state, frame.lua) - min) as f32 / (max - min) as f32).clamp(0.0, 1.0),
        None => rbms_skin::property::clamp_float(value.float(frame.state, frame.lua)),
    }
}

/// A handle moved along its track by its value.
fn draw_slider<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &SliderBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let travel = ratio(&body.value, body.ref_num, frame) * body.range;
    let mut at = rect;
    match body.direction {
        DIRECTION_UP => at.y += travel,
        DIRECTION_RIGHT => at.x += travel,
        DIRECTION_DOWN => at.y -= travel,
        DIRECTION_LEFT => at.x -= travel,
        _ => {}
    }
    let cell = body.sprite.animation_index(body.sprite.cells(), frame.now_ms, frame.timers);
    place.cell(r, &body.sprite, cell, at)
}

/// A bar cropped to its value, in the source as well as the destination so it is revealed rather
/// than squashed.
fn draw_graph<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &GraphBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let value = ratio(&body.value, body.ref_num, frame);
    if value <= 0.0 {
        return false;
    }
    let cell = body.sprite.animation_index(body.sprite.cells(), frame.now_ms, frame.timers);
    let mut src = body.sprite.uv(cell);
    let mut at = rect;
    if body.direction == GRAPH_VERTICAL {
        at.h *= value;
        src.v0 = src.v1 - (src.v1 - src.v0) * value;
    } else {
        at.w *= value;
        src.u1 = src.u0 + (src.u1 - src.u0) * value;
    }
    let source = body.sprite.cell_size();
    let dst = fitted_screen_rect(place.viewport, place.object.stretch, at, source);
    let filter = texture_filter(filtering_for(place.object.track.filter, screen_as_skin(dst), source));
    if dst.w <= 0.0 || dst.h <= 0.0 {
        return false;
    }
    r.draw_textured_quad(body.sprite.tex, place.quad(dst, src, filter));
    true
}
