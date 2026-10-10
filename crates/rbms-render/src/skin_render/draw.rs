//! Turning one resolved object into primitive calls.
//!
//! Each function here takes an object whose destination has already answered "where, what colour, at
//! what angle" and turns that into quads. The order of the three coordinate steps is fixed and
//! matters: the viewport first flips and scales the rectangle onto the screen, the stretch mode
//! then reshapes it there and the filter rule compares it with the source there, and the rotation
//! anchor is finally measured against the same screen rectangle, because that is the box the
//! backend turns. Everything after the viewport is in screen pixels, which is the space the
//! reference makes all three of those decisions in.

use rbms_skin::dst::{Resolved, SkinRect};
use rbms_skin::loader::{Filtering, filtering_for, stretch_rect};
use rbms_skin::model::PropertyRef;
use rbms_skin::property::{FLOAT_ABSENT, INTEGER_ABSENT, NameSpace, reference_implements};

use super::object::{
    Body, DigitLayout, FloatBody, GraphBody, ImageBody, MovieBody, NumberBody, SkinObject, SliderBody, Sprite, ValueSource, fraction_glyphs, integer_glyphs,
};
use super::{SkinFrame, SkinViewport, bga, covers, gauge, graphs, judge, notes, refs, songlist, text, text_input};
use crate::ctx::RenderCtx;
use crate::{BlendMode, Color, QuadParams, Rect, Renderer, TextureFilter, TextureId, UvRect, skin_center_offset};

/// The `align` value that leaves a run of digit places where they fall. A whole number's blank
/// places lead it, so this is flush right for one (`SkinNumber.prepare`); a fractional number's
/// trail it, so the same value is flush left there (`SkinFloat.prepare`).
const ALIGN_IN_PLACE: i32 = 0;

/// The `align` value that moves a run a whole slot for every blank place: over the leading blanks
/// of a whole number, which makes it flush left, and over the trailing ones of a fractional number,
/// which makes it flush right. Any other value moves the run half as far and centres it.
const ALIGN_OVER_BLANKS: i32 = 1;

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

/// The two numbers `SkinFloat.prepare` reads as "no value": Java's `Float.MIN_VALUE`, which is what
/// a host answers for a number it does not carry, and `Float.MAX_VALUE`.
const FLOAT_NO_VALUE: [f32; 2] = [FLOAT_ABSENT, f32::MAX];

/// What a slider or a graph reads when its value is not a number at all. The reference multiplies a
/// NaN straight into a size; here it is no travel and no bar.
const SHARE_OF_NOTHING: f32 = 0.0;

/// The share a whole number scaled between two endpoints reads past its far end
/// (`SkinObject.RateProperty`).
const SHARE_FULL: f32 = 1.0;

/// The share it reads before its near end.
const SHARE_EMPTY: f32 = 0.0;

/// The alignment shift a whole number applies, which pulls its digits over the blank places to
/// their left (`SkinNumber.draw`).
const SHIFT_TOWARDS_START: f32 = -1.0;

/// The alignment shift a fractional number applies, which pushes its digits the other way
/// (`SkinFloat.draw`).
const SHIFT_TOWARDS_END: f32 = 1.0;

/// The fixed parts of one object's draw: what is being drawn, how it is tinted and turned, and where
/// the document's space lands on screen.
///
/// Open to the whole renderer rather than this file because the objects that draw a field, a wheel
/// or a graph live in modules of their own and place their pieces against the same three decisions.
pub(crate) struct Placement<'a> {
    pub(crate) object: &'a SkinObject,
    pub(crate) blend: BlendMode,
    pub(crate) tint: Color,
    pub(crate) angle_deg: f32,
    pub(crate) viewport: &'a SkinViewport,
}

/// One axis of a quad that may have been asked for with a negative extent.
struct Span {
    /// Where the quad starts on screen, which is its lower coordinate.
    origin: f32,
    /// How far it reaches from there, never negative.
    extent: f32,
    /// The texture coordinate at the start.
    near: f32,
    /// The texture coordinate at the end.
    far: f32,
    /// Where the rotation anchor sits, measured from the start.
    pivot: f32,
}

impl Span {
    /// The axis as the backend wants it. A negative extent reaches backwards from its origin, so
    /// the quad starts that much earlier, reads its texture the other way round and keeps its
    /// anchor on the same screen point: that is `SpriteBatch.draw` given a negative width, whose
    /// vertices run from `x` to `x + width` with the region's coordinates in their usual order.
    fn of(origin: f32, extent: f32, near: f32, far: f32, pivot: f32) -> Span {
        if extent < 0.0 {
            Span { origin: origin + extent, extent: -extent, near: far, far: near, pivot: pivot - extent }
        } else {
            Span { origin, extent, near, far, pivot }
        }
    }
}

/// Whether a size is one a quad can be drawn at: a number, and not nothing. A negative one is drawn
/// mirrored.
fn has_extent(size: f32) -> bool {
    size.is_finite() && size != 0.0
}

/// A pixel region of a texture `size` pixels large as normalised coordinates. A region of negative
/// width or height reads backwards, as a `TextureRegion` set that way does.
fn pixel_uv(region: SkinRect, size: (u32, u32)) -> UvRect {
    let (width, height) = (size.0.max(1) as f32, size.1.max(1) as f32);
    UvRect::new(region.x / width, region.y / height, (region.x + region.w) / width, (region.y + region.h) / height)
}

impl Placement<'_> {
    /// A quad in screen space, with the rotation anchor measured against it.
    ///
    /// `dst` may have a negative width or height. It is then drawn mirrored on that axis, reaching
    /// back from its own corner, and the anchor is measured the way the reference measures it:
    /// `centerx * width` from that corner, sign and all (`Skin.SkinObjectRenderer.draw`).
    pub(crate) fn quad(&self, dst: Rect, src: UvRect, filter: TextureFilter) -> QuadParams {
        let pivot = skin_center_offset(self.object.track.center, dst.w, dst.h);
        let across = Span::of(dst.x, dst.w, src.u0, src.u1, pivot.0);
        let down = Span::of(dst.y, dst.h, src.v0, src.v1, pivot.1);
        QuadParams {
            dst: Rect { x: across.origin, y: down.origin, w: across.extent, h: down.extent },
            src: UvRect::new(across.near, down.near, across.far, down.far),
            tint: self.tint,
            blend: self.blend,
            filter,
            angle_deg: self.angle_deg,
            center: (across.pivot, down.pivot),
        }
    }

    /// Places one sprite cell, fitting it to the destination the way the object's stretch mode asks.
    pub(crate) fn cell<R: Renderer>(&self, r: &mut R, sprite: &Sprite, cell: u32, rect: SkinRect) -> bool {
        self.region(r, sprite, sprite.region(cell), rect)
    }

    /// Places one pixel region of a sprite's texture ([`Self::texture`]).
    pub(crate) fn region<R: Renderer>(&self, r: &mut R, sprite: &Sprite, region: SkinRect, rect: SkinRect) -> bool {
        self.texture(r, sprite.tex, sprite.size, region, rect)
    }

    /// Places one pixel region of a texture `size` pixels large, fitting it to the destination the
    /// way the object's stretch mode asks and reading the part of the region that mode leaves
    /// (`SkinObject.draw`). This is the one path every image, digit, handle, bar and reference image
    /// reaches the screen by.
    ///
    /// Both the fit and the filter are decided in screen pixels, after the viewport has scaled the
    /// rectangle, because that is the space the reference decides them in: `Skin.setDestination`
    /// multiplies a destination into screen coordinates before `SkinObject.draw` ever compares it
    /// with the source region. A document authored at half the screen's size would otherwise draw
    /// its unresized objects at half size and pick the wrong filter for everything.
    ///
    /// The rectangle keeps the sign of its width and height through all of that, as it does in the
    /// reference: the stretch modes do their arithmetic on the signed sizes, a mirrored rectangle is
    /// never the size of its source and so never draws unfiltered once the document asked for
    /// filtering, and only the quad itself is turned the right way round ([`Self::quad`]). A
    /// rectangle with no area, or with a size that is not a number, draws nothing.
    pub(crate) fn texture<R: Renderer>(&self, r: &mut R, tex: TextureId, size: (u32, u32), region: SkinRect, rect: SkinRect) -> bool {
        self.sampled(r, tex, size, region, rect, None)
    }

    /// [`Self::texture`], read through `filter` whatever the object's own filter and size would
    /// have chosen. `None` leaves the choice to them.
    fn sampled<R: Renderer>(&self, r: &mut R, tex: TextureId, size: (u32, u32), region: SkinRect, rect: SkinRect, filter: Option<TextureFilter>) -> bool {
        let placed = self.viewport.place(rect);
        let (fitted, source) = stretch_rect(self.object.stretch, SkinRect::new(placed.x, placed.y, placed.w, placed.h), region);
        if !(has_extent(fitted.w) && has_extent(fitted.h) && fitted.x.is_finite() && fitted.y.is_finite()) {
            return false;
        }
        let filter = filter.unwrap_or_else(|| texture_filter(filtering_for(self.object.track.filter, fitted, (source.w, source.h))));
        let dst = Rect { x: fitted.x, y: fitted.y, w: fitted.w, h: fitted.h };
        r.draw_textured_quad(tex, self.quad(dst, pixel_uv(source, size), filter));
        true
    }
}

/// The renderer's filter for the one the skin rules chose.
fn texture_filter(filtering: Filtering) -> TextureFilter {
    match filtering {
        Filtering::Nearest => TextureFilter::Nearest,
        Filtering::Linear => TextureFilter::Linear,
    }
}

/// Prepares and draws one object on its own, answering whether anything reached the screen.
///
/// A screen never does this: it prepares every object before it draws the first. It is what a test
/// of one object's drawing reaches for.
#[cfg(test)]
pub(crate) fn draw_object<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, object: &SkinObject, viewport: &SkinViewport, frame: &SkinFrame<'_>) -> bool {
    object.prepare(frame).is_some_and(|resolved| draw_resolved(ctx, r, object, 0, viewport, frame, &resolved))
}

/// Draws one object where the prepare stage put it, answering whether anything reached the screen.
///
/// A fully transparent object leaves before it draws anything, which is where `SkinObject.draw`
/// leaves too. Under alpha and additive blending that only saves work, but multiply and
/// invert-destination read no source alpha at all, so an object fading out under one of those would
/// otherwise keep darkening the screen at `a: 0`.
///
/// `index` is the object's place among its screen's objects, which is how an editable text finds out
/// that it is the one being typed into.
pub(crate) fn draw_resolved<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    object: &SkinObject,
    index: usize,
    viewport: &SkinViewport,
    frame: &SkinFrame<'_>,
    resolved: &Resolved,
) -> bool {
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
        Body::Movie(body) => draw_movie(r, &place, body, rect),
        Body::Number(body) => draw_number(r, &place, body, rect, frame),
        Body::Float(body) => draw_float(r, &place, body, rect, frame),
        Body::Text(body) => text::draw_text(ctx, r, &place, body, rect, frame),
        Body::TextInput(body) => text_input::draw_text_input(ctx, r, &place, body, index, rect, frame),
        Body::Slider(body) => draw_slider(r, &place, body, rect, frame),
        Body::Graph(body) => draw_graph(r, &place, body, rect, frame),
        Body::Bga(body) => bga::draw_bga(r, &place, body, rect, frame),
        Body::Reference(body) => refs::draw_reference(r, &place, body, rect, frame),
        Body::Note(body) => notes::draw_note(ctx, r, &place, body, rect, frame),
        Body::Gauge(body) => gauge::draw_gauge(ctx, r, &place, body, rect, frame),
        Body::Judge(body) => judge::draw_judge(ctx, r, &place, body, rect, frame),
        Body::SongList(body) => songlist::draw_songlist(ctx, r, &place, body, rect, frame),
        Body::HiddenCover(body) | Body::LiftCover(body) => covers::draw_cover(ctx, r, &place, body, rect, frame),
        Body::GaugeGraph(body) => graphs::draw_gauge_graph(ctx, r, &place, body, rect, frame),
        Body::JudgeGraph(body) => graphs::draw_judge_graph(ctx, r, &place, body, rect, frame),
        Body::BpmGraph(body) => graphs::draw_bpm_graph(ctx, r, &place, body, rect, frame),
        Body::TimingDistribution(body) => graphs::draw_timing_distribution(ctx, r, &place, body, rect, frame),
        Body::TimingVisualizer(body) => graphs::draw_timing_visualizer(ctx, r, &place, body, rect, frame),
        Body::HitError(body) => graphs::draw_hit_error(ctx, r, &place, body, rect, frame),
    };

    if clipped {
        r.pop_clip();
    }
    if drawn {
        text::carry_blend(ctx, &object.body, place.blend);
    }
    drawn
}

/// What picks the set an image shows (`SkinImage.ref`).
#[derive(Debug, Clone, Default)]
pub(crate) enum ImageSelect {
    /// Nothing does: the image always shows its first set.
    #[default]
    First,
    /// An id in the image index space, which is not the space a number reads
    /// (`IntegerPropertyFactory.getImageIndexProperty`).
    Index(i32),
    /// A value the document supplied itself, read as a whole number: an image set's `value`.
    Value(ValueSource),
}

impl ImageSelect {
    /// The selection a `ref` field names.
    ///
    /// An id the reference has no image index under selects nothing, and the image then always shows
    /// its first set: the reference's lookup answers null there, and `SkinImage.prepare` reads a
    /// null as zero. That is every image whose `ref` was left out, since id zero is such an id.
    pub(crate) fn of_index(id: i32) -> ImageSelect {
        if reference_implements(NameSpace::ImageIndex, id) { ImageSelect::Index(id) } else { ImageSelect::First }
    }

    /// The selection an image set reads: its `value` when the document gave one, its `ref` otherwise
    /// (`JsonSkinObjectLoader`).
    ///
    /// A `value` written as an id is a number, not an image index, because the reference resolves
    /// that field through its whole-number lookup. One the reference has no number under resolves to
    /// nothing there, which leaves the set reading its `ref` as though no `value` had been written;
    /// so does source the loader never compiled.
    pub(crate) fn of_set(value: Option<&PropertyRef>, reference: i32) -> ImageSelect {
        match value {
            Some(PropertyRef::Id(id)) if !reference_implements(NameSpace::Integer, *id) => ImageSelect::of_index(reference),
            Some(PropertyRef::Expr(_)) | None => ImageSelect::of_index(reference),
            Some(property) => ImageSelect::Value(ValueSource::new(Some(property), 0)),
        }
    }

    /// Which of `sets` sets this frame shows, or `None` when the image is not drawn at all
    /// (`SkinImage.prepare`): a negative index hides the image before anything else about it is
    /// looked at, and one past the last set shows the first.
    pub(crate) fn slot(&self, sets: usize, frame: &SkinFrame<'_>) -> Option<usize> {
        let index = match self {
            ImageSelect::First => 0,
            ImageSelect::Index(id) => frame.state.image_index(*id),
            ImageSelect::Value(value) => value.integer(frame.state, frame.lua),
        };
        let index = usize::try_from(index).ok()?;
        Some(if index >= sets { 0 } else { index })
    }
}

/// A still or animated image, showing the set a property picked.
fn draw_image<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &ImageBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let Some((sprite, first, count)) = body.chosen(frame) else {
        return false;
    };
    let cell = first + sprite.animation_index(*count, frame.now_us, frame.timers, frame.script());
    place.cell(r, sprite, cell, rect)
}

/// An image whose source is a movie: the whole of the frame that is on show, or nothing at all
/// while there is none (`SkinImage.prepare`, which leaves the object undrawn when its source has no
/// frame for it yet).
///
/// The frame is fitted like any image, by the object's stretch mode, and always read through the
/// linear filter: the reference draws a movie with its own renderer type, which sets that filter
/// whatever the destination's `filter` says (`SkinImage.draw`, `Skin.SkinObjectRenderer.setFilter`).
/// A movie's frames are rarely the size of the rectangle they land in, and read point by point a
/// playing movie shimmers.
fn draw_movie<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &MovieBody, rect: SkinRect) -> bool {
    let Some((tex, size)) = body.movie.frame() else {
        return false;
    };
    let whole = SkinRect::new(0.0, 0.0, size.0 as f32, size.1 as f32);
    place.sampled(r, tex, size, whole, rect, Some(TextureFilter::Linear))
}

/// How far a run of digit places is nudged to keep its alignment.
///
/// A blank place still takes a slot, so `blanks` is what the run is shifted by: an `align` of zero
/// leaves the places where they fall, one moves the digits a whole slot per blank, and anything else
/// half of one. Which way that is, and so which of the first two is left and which is right, is the
/// caller's ([`DigitRun::shift_sign`]).
fn digit_shift(step: f32, blanks: usize, align: i32) -> f32 {
    match align {
        ALIGN_IN_PLACE => 0.0,
        ALIGN_OVER_BLANKS => step * blanks as f32,
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

/// A length measured in screen pixels as the document length that lands on it. A viewport that
/// scales by nothing has no such length, and the nudge is dropped.
fn document_length(screen: f32, scale: f32) -> f32 {
    if scale > 0.0 { screen / scale } else { 0.0 }
}

/// Draws one run of digit places.
///
/// The gap between places is a document length and scales with the document. The per-place nudges
/// are not: the reference adds them to a destination that has already been scaled to the screen
/// (`SkinNumber.draw`), so they are screen pixels whatever size the document was authored at.
fn draw_places<R: Renderer>(r: &mut R, place: &Placement<'_>, sprite: &Sprite, rect: SkinRect, places: &[Option<u32>], run: DigitRun<'_>) -> bool {
    let step = rect.w + run.space;
    let blanks = places.iter().filter(|slot| slot.is_none()).count();
    let shift = digit_shift(step, blanks, run.align) * run.shift_sign;
    let (scale_x, scale_y) = (place.viewport.scale_x(), place.viewport.scale_y());
    let mut drawn = false;
    for (index, slot) in places.iter().enumerate() {
        let Some(cell) = slot.and_then(|glyph| run.layout.cell(run.set, run.negative, glyph)) else {
            continue;
        };
        let (dx, dy, dw, dh) = run.offsets.get(index).copied().unwrap_or((0.0, 0.0, 0.0, 0.0));
        let at = SkinRect::new(
            rect.x + step * index as f32 + shift + document_length(dx, scale_x),
            rect.y + document_length(dy, scale_y),
            rect.w + document_length(dw, scale_x),
            rect.h + document_length(dh, scale_y),
        );
        drawn |= place.cell(r, sprite, cell, at);
    }
    drawn
}

/// The whole number a number object shows this frame, or `None` when it shows nothing
/// (`SkinNumber.prepare`).
///
/// A property with nothing to report answers one of the two sentinels rather than a number, and the
/// object then draws no places at all. So does an object that names no property: the reference reads
/// a missing one as the lower sentinel, not as zero.
pub(crate) fn number_value(value: &ValueSource, frame: &SkinFrame<'_>) -> Option<i32> {
    let read = if value.is_named() { value.integer(frame.state, frame.lua) } else { INTEGER_ABSENT };
    (!INTEGER_NO_VALUE.contains(&read)).then_some(read)
}

/// A whole number.
fn draw_number<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &NumberBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let Some(value) = number_value(&body.value, frame) else {
        return false;
    };
    let places = integer_glyphs(body, value);
    let set = body.sprite.animation_index(body.layout.sets, frame.now_us, frame.timers, frame.script());
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

/// How many places a fractional object draws: a sign, the whole places, the fractional places and
/// the point between them (`FloatFormatter`'s `length`).
fn fraction_places(body: &FloatBody) -> i32 {
    i32::from(body.sign) + body.integer_digits + body.fraction_digits + i32::from(body.fraction_digits != 0)
}

/// The number a fractional object shows this frame, its `gain` applied, or `None` when it shows
/// nothing (`SkinFloat.prepare`).
///
/// Nothing is shown for an object that names no property, for either of the reference's two "no
/// value" sentinels before or after the gain, for a product that is not a finite number, and for an
/// object the document gave no places at all.
pub(crate) fn float_value(body: &FloatBody, frame: &SkinFrame<'_>) -> Option<f32> {
    if !body.value.is_named() {
        return None;
    }
    let raw = body.value.float(frame.state, frame.lua);
    let value = raw * body.gain;
    let unshowable = FLOAT_NO_VALUE.contains(&raw) || !value.is_finite() || FLOAT_NO_VALUE.contains(&value) || fraction_places(body) == 0;
    (!unshowable).then_some(value)
}

/// A fractional number.
fn draw_float<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &FloatBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let Some(value) = float_value(body, frame) else {
        return false;
    };
    let places = fraction_glyphs(body, f64::from(value.abs()));
    let set = body.sprite.animation_index(body.layout.sets, frame.now_us, frame.timers, frame.script());
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

/// Java's `(int)` cast of a float: towards zero, saturating at the type's ends, and zero for a NaN.
fn java_int(value: f32) -> f32 {
    value as i32 as f32
}

/// The share a whole number is of the way from `min` to `max` (`SkinObject.RateProperty`).
///
/// The endpoints may be written either way round. Past the far one the share is full and before the
/// near one it is empty, and in between it is the distance from `min` as a share of the span, with
/// the reference's own operand order. Two endpoints that are the same number divide nothing by
/// nothing when the value sits on them.
fn ranged_share(value: i32, min: i32, max: i32) -> f32 {
    let (past_far, before_near) = if min < max { (value > max, value < min) } else { (value < max, value > min) };
    if past_far {
        SHARE_FULL
    } else if before_near {
        SHARE_EMPTY
    } else {
        ((value as f32 - min as f32) / max.wrapping_sub(min) as f32).abs()
    }
}

/// The share a slider or a graph is at this frame: a rate it reads, or a whole number it scales
/// between endpoints of its own.
///
/// The share is not held between zero and one, because the reference does not hold it either
/// (`SkinSlider.prepare`, `SkinGraph.prepare`): a handle travels past the end of its track and a bar
/// grows past its destination when the value says so. A rate is read from the rate space alone, and
/// one the host does not carry is zero. Only a value that is not a number is replaced
/// ([`SHARE_OF_NOTHING`]).
pub(crate) fn share(value: &ValueSource, ref_num: Option<(i32, i32)>, frame: &SkinFrame<'_>) -> f32 {
    let read = match (ref_num, value) {
        (Some((min, max)), value) => ranged_share(value.integer(frame.state, frame.lua), min, max),
        (None, ValueSource::Id(id)) => frame.state.rate(*id).unwrap_or(SHARE_EMPTY),
        (None, value) => value.float(frame.state, frame.lua),
    };
    if read.is_nan() { SHARE_OF_NOTHING } else { read }
}

/// How far a slider's handle travels at a full share, in document pixels.
///
/// The reference scales `range` to the screen once, when the skin loads, and keeps the result as a
/// whole number of screen pixels (`JsonSkinObjectLoader`: `(int) (scale * range)`, with the
/// horizontal scale for a handle that moves sideways and the vertical one otherwise). The travel is
/// that whole number, read back into the document's space.
fn slider_travel(body: &SliderBody, viewport: &SkinViewport) -> f32 {
    let sideways = body.direction == DIRECTION_RIGHT || body.direction == DIRECTION_LEFT;
    let scale = if sideways { viewport.scale_x() } else { viewport.scale_y() };
    document_length(java_int(scale * body.range), scale)
}

/// A handle moved along its track by its value (`SkinSlider.draw`).
fn draw_slider<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &SliderBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let travel = share(&body.value, body.ref_num, frame) * slider_travel(body, place.viewport);
    let mut at = rect;
    match body.direction {
        DIRECTION_UP => at.y += travel,
        DIRECTION_RIGHT => at.x += travel,
        DIRECTION_DOWN => at.y -= travel,
        DIRECTION_LEFT => at.x -= travel,
        _ => {}
    }
    let cell = body.sprite.animation_index(body.sprite.cells(), frame.now_us, frame.timers, frame.script());
    place.cell(r, &body.sprite, cell, at)
}

/// A bar cropped to its value, in the source as well as the destination so it is revealed rather
/// than squashed (`SkinGraph.draw`).
///
/// The source is cropped to a whole number of texels, towards zero, while the destination keeps the
/// exact share. An upward bar keeps the foot of its cell and of its destination; any other keeps the
/// left edge of both. A share past one reads past the cell and draws past the destination, and a
/// negative one does both backwards.
///
/// The crop comes first and the stretch mode then fits the cropped region, which is the order
/// `SkinGraph.draw` hands its region to `SkinObject.draw` in.
fn draw_graph<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &GraphBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    let value = share(&body.value, body.ref_num, frame);
    let cell = body.sprite.animation_index(body.sprite.cells(), frame.now_us, frame.timers, frame.script());
    let whole = body.sprite.region(cell);
    let (region, at) = if body.direction == GRAPH_VERTICAL {
        let shown = java_int(whole.h * value);
        (SkinRect { y: whole.y + whole.h - shown, h: shown, ..whole }, SkinRect { h: rect.h * value, ..rect })
    } else {
        (SkinRect { w: java_int(whole.w * value), ..whole }, SkinRect { w: rect.w * value, ..rect })
    };
    place.region(r, &body.sprite, region, at)
}
