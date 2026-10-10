//! The `text` object: a run of text in the font the document registered for it.
//!
//! A text whose font loaded is drawn the way the reference draws a TrueType text
//! (`SkinText` and `SkinTextFont`):
//!
//! - The destination's height is the em size. The document's `size` is only the size the reference
//!   generates the font at, so glyph advances and the capital height are whole pixels at that size
//!   and are then scaled by `height / size`; a `size` of nothing draws nothing.
//! - The destination's `x` is an anchor, not a corner: the left end of a left-aligned line, the
//!   middle of a centred one and the right end of a right-aligned one. Its width is the room the
//!   line is laid out in, which is what `overflow` and `wrapping` measure against.
//! - The destination's top edge is the capital line of the first line of text. The baseline hangs a
//!   capital height below it and descenders fall further still, so the box is not what the glyphs
//!   fill.
//! - A shadow is the same line drawn first, at half the brightness and the same opacity, moved by
//!   the document's offset: right for a positive `shadowOffsetX` and down for a positive
//!   `shadowOffsetY`. The offset is in screen pixels and is not scaled with the skin.
//! - `outlineColor`, `outlineWidth`, `shadowColor` and `shadowSmoothness` are read by the
//!   reference's distance-field bitmap fonts only ([`bitmap::FieldInk`]). A TrueType text ignores
//!   all four, and so does this.
//! - A text sets no blend of its own. It is drawn with whatever the object before it left on the
//!   batch ([`carry_blend`]), and it is never turned or stretched.
//!
//! The line is composed once into a texture of its own at the size it reaches the screen at
//! ([`TextContext::compose_block`](crate::font::TextContext::compose_block)), and composed again
//! only when the string, the size or the width changes.
//!
//! A text whose font is a bitmap `.fnt` shares the anchor, the top edge, the shadow and the blend
//! above and none of the rest: its size comes from the document's `size` rather than from the
//! destination's height, and its glyphs are cut out of the font's pages rather than composed
//! ([`bitmap`]).
//!
//! A text whose font did not load -- a file that would not read, or an id the document never
//! declared -- keeps the stand-in it has always had: the default face at the destination's height,
//! anchored the same way and with none of the above.

use std::borrow::Cow;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use rbms_skin::dst::SkinRect;
use rbms_skin::model::TextDef;
use rbms_skin::property::{NameSpace, reference_implements};

use super::SkinFrame;
use super::draw::Placement;
use super::object::{Body, ValueSource};
use crate::ctx::RenderCtx;
use crate::font::{BlockAlign, BlockFit, BlockSpec, TextContext};
use crate::{BlendMode, Color, QuadParams, Rect, Renderer, TextureFilter, TextureId};

pub(crate) mod bitmap;

#[cfg(test)]
mod tests;

/// Pixel height one unit of the text engine's legacy scale draws at.
///
/// A skin sizes its text in pixels while [`crate::font::TextContext`] takes that scale, so the two
/// are related by this factor. It tracks `font::px_for`; the ratio itself is pinned by test rather
/// than shared, because the text engine's scale is its own private unit.
pub(crate) const TEXT_PIXELS_PER_SCALE: f32 = 8.5;

/// The smallest text scale worth asking the engine for, below which it clamps anyway.
pub(crate) const MIN_TEXT_SCALE: f32 = 0.1;

/// A text object's `align` value that centres the line on its destination's anchor
/// (`SkinText.ALIGN_CENTER`). Text numbers its alignments differently from numbers, which is why
/// the two sets are named apart.
const TEXT_ALIGN_CENTER: i32 = 1;

/// A text object's `align` value that ends the line at its destination's anchor
/// (`SkinText.ALIGN_RIGHT`). Every other value starts it there.
const TEXT_ALIGN_RIGHT: i32 = 2;

/// A text object's `overflow` value that squeezes a line too long for its destination
/// (`SkinText.OVERFLOW_SHRINK`).
const TEXT_OVERFLOW_SHRINK: i32 = 1;

/// A text object's `overflow` value that cuts such a line short (`SkinText.OVERFLOW_TRUNCATE`).
/// Every other value lets it run on.
const TEXT_OVERFLOW_TRUNCATE: i32 = 2;

/// How much of a text's brightness its shadow keeps (`SkinTextFont.draw`: `color.r / 2`).
const SHADOW_DIVISOR: u8 = 2;

/// Tells one text object's texture from another's in the renderer's registry.
static NEXT_TEXT_SERIAL: AtomicU32 = AtomicU32::new(0);

/// What a text object's texture currently holds.
#[derive(Debug, Clone, PartialEq)]
struct Composed {
    text: String,
    /// The em size and the width the line was laid out at, as their bit patterns.
    laid_out: (u32, u32),
    /// Where the texture sits and how large, or `None` when the line came to no pixels at all.
    block: Option<Stamp>,
}

/// Where a composed line sits relative to its anchor, and how large it is.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Stamp {
    left: f32,
    top: f32,
    drawn_width: f32,
    size: (u32, u32),
}

/// The texture one text object keeps its line in.
#[derive(Debug)]
struct Line {
    key: String,
    handle: Option<TextureId>,
    composed: Option<Composed>,
}

impl Line {
    fn new() -> Line {
        Line { key: format!("rbms.skin.text.{}", NEXT_TEXT_SERIAL.fetch_add(1, Ordering::Relaxed)), handle: None, composed: None }
    }

    /// The texture holding `spec` composed, and where it sits, or `None` when the line draws
    /// nothing or the renderer would not hold it.
    ///
    /// The line is composed again when the string, the size or the width is not what the texture
    /// was made from, and also when the renderer does not know the texture at that size, which is
    /// what drawing onto another renderer than the one it was uploaded to looks like.
    fn texture<R: Renderer>(&mut self, text: &mut TextContext, r: &mut R, spec: &BlockSpec<'_>) -> Option<(TextureId, Stamp)> {
        let laid_out = (spec.em_px.to_bits(), spec.width.to_bits());
        let current = self.composed.as_ref().filter(|composed| composed.text == spec.text && composed.laid_out == laid_out);
        let held = match current.map(|composed| composed.block) {
            Some(None) => return None,
            Some(Some(stamp)) => self.handle.filter(|handle| r.texture_size(*handle) == Some(stamp.size)).map(|handle| (handle, stamp)),
            None => None,
        };
        if held.is_some() {
            return held;
        }

        let block = text.compose_block(spec);
        let stamp = block.as_ref().map(|block| Stamp { left: block.left, top: block.top, drawn_width: block.drawn_width, size: (block.width, block.height) });
        self.composed = Some(Composed { text: spec.text.to_string(), laid_out, block: stamp });
        let block = block?;
        let handle = r.register_texture(&self.key, &block.rgba, block.width, block.height);
        self.handle = Some(handle);
        r.texture_size(handle).and(stamp).map(|stamp| (handle, stamp))
    }

    fn release<R: Renderer>(&mut self, r: &mut R) {
        if let Some(handle) = self.handle.take() {
            r.release_texture(handle);
        }
        self.composed = None;
    }
}

/// A run of text.
#[derive(Debug)]
pub(crate) struct TextBody {
    /// The name the text engine registered this document's font under, or `None` when the font
    /// did not load.
    pub(crate) family: Option<String>,
    /// 0 left, 1 centred, 2 right.
    pub(crate) align: i32,
    pub(crate) value: ValueSource,
    /// Text the document wrote out, shown only when it names no property to read instead.
    pub(crate) constant: Option<String>,
    /// The size the document says the font is generated at.
    size: i32,
    wrapping: bool,
    overflow: i32,
    /// How far the shadow is moved right and down, in screen pixels. No shadow when both are zero.
    shadow: (f32, f32),
    line: RefCell<Line>,
    /// The bitmap font this text is drawn with and the line it laid out last, when its font is one.
    /// Such a text has no `family`.
    bitmap: Option<RefCell<bitmap::BitmapText>>,
    /// The outline and the shadow a distance field font draws this text with. No other font reads
    /// them.
    field: bitmap::FieldInk,
}

impl TextBody {
    /// Whether the document names a property the reference would read the text from
    /// (`JsonSkinObjectLoader.createText`): any function or name it handed over, or an id the
    /// reference has a string under. An id it has none under is no property at all.
    fn reads_property(&self) -> bool {
        match &self.value {
            ValueSource::None => false,
            ValueSource::Id(id) => reference_implements(NameSpace::Text, *id),
            ValueSource::Function(_) | ValueSource::Name(_) => true,
        }
    }

    /// The string shown this frame (`SkinText.prepare`): what the property reads, and the written
    /// out text only for an object with no property. A property that reads empty shows nothing,
    /// whatever the document wrote out beside it.
    pub(crate) fn shown<'a>(&'a self, frame: &SkinFrame<'a>) -> Cow<'a, str> {
        if self.reads_property() {
            return self.value.text(frame.state, frame.lua);
        }
        Cow::Borrowed(self.constant.as_deref().unwrap_or_default())
    }

    pub(crate) fn block_align(&self) -> BlockAlign {
        match self.align {
            TEXT_ALIGN_CENTER => BlockAlign::Center,
            TEXT_ALIGN_RIGHT => BlockAlign::Right,
            _ => BlockAlign::Left,
        }
    }

    /// How `line` is laid out when this text is drawn into `dst`, a destination on screen, or `None`
    /// for a text whose font did not load, which is drawn in the stand-in face instead.
    pub(crate) fn layout<'a>(&'a self, line: &'a str, dst: Rect, max_dim: u32) -> Option<BlockSpec<'a>> {
        let family = self.family.as_deref()?;
        Some(BlockSpec {
            text: line,
            family,
            em_px: dst.h,
            design_px: self.size as f32,
            width: dst.w,
            align: self.block_align(),
            fit: self.block_fit(),
            max_dim,
        })
    }

    /// What becomes of a line too long for its destination (`SkinTextFont.setLayout`,
    /// `SkinTextBitmap.setLayout`). Wrapping wins over every `overflow`.
    fn block_fit(&self) -> BlockFit {
        match self.overflow {
            _ if self.wrapping => BlockFit::Wrap,
            TEXT_OVERFLOW_SHRINK => BlockFit::Shrink,
            TEXT_OVERFLOW_TRUNCATE => BlockFit::Truncate,
            _ => BlockFit::Overflow,
        }
    }

    /// Hands the texture the line was composed into back to the renderer.
    pub(crate) fn release<R: Renderer>(&self, r: &mut R) {
        self.line.borrow_mut().release(r);
        if let Some(bitmap) = &self.bitmap {
            bitmap.borrow_mut().release(r);
        }
    }
}

/// Where a line's layout starts: the destination's `x` is the left end of a left-aligned line, the
/// middle of a centred one and the right end of a right-aligned one, and the line is aligned again
/// inside the destination's width from there (`SkinTextFont.draw`, `SkinTextBitmap.draw`).
fn anchor_x(align: BlockAlign, dst: Rect) -> f32 {
    match align {
        BlockAlign::Left => dst.x,
        BlockAlign::Center => dst.x - dst.w / 2.0,
        BlockAlign::Right => dst.x - dst.w,
    }
}

/// What a font a document declared was loaded as.
#[derive(Debug, Clone)]
pub(crate) enum FontRef {
    /// A TrueType font, by the name the text engine registered it under.
    Family(String),
    /// A bitmap font, as the screen that loaded it draws with it.
    Bitmap(Arc<bitmap::ScreenFace>),
}

impl FontRef {
    /// The text engine's name for the font, when it is one of the engine's.
    pub(crate) fn family(&self) -> Option<&str> {
        match self {
            FontRef::Family(family) => Some(family),
            FontRef::Bitmap(_) => None,
        }
    }

    fn face(&self) -> Option<&Arc<bitmap::ScreenFace>> {
        match self {
            FontRef::Family(_) => None,
            FontRef::Bitmap(face) => Some(face),
        }
    }
}

/// The fonts a screen's text objects are built from: each font id of the document that loaded, and
/// what it loaded as.
pub(crate) type Fonts = [(String, FontRef)];

/// A text run and the font it is drawn with.
///
/// A text in a bitmap font whose string the document wrote out has the pages of that string asked
/// for here, when the screen is built, rather than on the frame the object first comes on screen.
pub(crate) fn text_body(def: &TextDef, fonts: &Fonts) -> TextBody {
    let font = fonts.iter().find(|(id, _)| *id == def.font).map(|(_, font)| font);
    let body = TextBody {
        family: font.and_then(FontRef::family).map(str::to_string),
        align: def.align,
        value: ValueSource::new(def.value.as_ref(), def.reference),
        constant: def.constant_text.clone(),
        size: def.size,
        wrapping: def.wrapping,
        overflow: def.overflow,
        shadow: (def.shadow_offset_x, def.shadow_offset_y),
        line: RefCell::new(Line::new()),
        bitmap: font.and_then(FontRef::face).map(|face| RefCell::new(bitmap::BitmapText::new(Arc::clone(face)))),
        field: bitmap::FieldInk::of(def),
    };
    if let (Some(face), Some(constant), false) = (font.and_then(FontRef::face), &body.constant, body.reads_property()) {
        face.want_pages_of(constant);
    }
    body
}

/// Hands back the texture a text object in `body` holds. Every other kind of object holds none of
/// this kind.
pub(crate) fn release<R: Renderer>(body: &Body, r: &mut R) {
    match body {
        Body::Text(body) => body.release(r),
        Body::TextInput(body) => body.shown().release(r),
        _ => {}
    }
}

/// Records what the object in `body`, which has just drawn with `blend`, leaves on the batch for a
/// text that follows it.
///
/// A text leaves what it found, because it sets nothing. A note field leaves plain alpha blending
/// whatever its destination asked for (`LaneRenderer.drawLane` sets blend 0). Every other object
/// leaves its own destination's blend (`SkinObject.draw`).
pub(crate) fn carry_blend(ctx: &mut RenderCtx<'_>, body: &Body, blend: BlendMode) {
    match body {
        Body::Text(_) | Body::TextInput(_) => {}
        Body::Note(_) => ctx.text.leave_blend(BlendMode::Alpha),
        _ => ctx.text.leave_blend(blend),
    }
}

/// A run of text, in the font the document registered for it.
pub(crate) fn draw_text<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &TextBody,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let line = body.shown(frame);
    draw_line(ctx, r, place, body, rect, &line)
}

/// Draws `line` the way the text object in `body` draws its own string at `rect`, answering whether
/// anything was drawn. An empty line draws nothing (`SkinText.prepare`).
///
/// This is the whole of a text's drawing apart from choosing the string, so an object that supplies
/// its strings itself -- a song wheel naming its rows -- draws them through here. Such an object
/// keeps one body per line it shows at a time, because a body holds the one line it composed last.
pub(crate) fn draw_line<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, place: &Placement<'_>, body: &TextBody, rect: SkinRect, line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let dst = place.viewport.place(rect);
    if let Some(bitmap) = &body.bitmap {
        return bitmap::draw(ctx, r, place, body, &mut bitmap.borrow_mut(), dst, line);
    }
    match body.layout(line, dst, r.max_texture_size()) {
        Some(spec) => draw_composed(ctx, r, place, body, &spec, dst),
        None => draw_stand_in(ctx, r, place, body, dst, line),
    }
}

/// Draws a line in the document's own font, the way the reference places it.
fn draw_composed<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, place: &Placement<'_>, body: &TextBody, spec: &BlockSpec<'_>, dst: Rect) -> bool {
    let align = spec.align;
    let Some((tex, stamp)) = body.line.borrow_mut().texture(ctx.text, r, spec) else {
        return false;
    };

    let anchor = anchor_x(align, dst);
    let (width, height) = (stamp.size.0 as f32, stamp.size.1 as f32);
    let at = Rect::new((anchor + stamp.left).round(), (dst.y + stamp.top).round(), stamp.drawn_width, height);
    let filter = if stamp.drawn_width == width { TextureFilter::Nearest } else { TextureFilter::Linear };
    let blend = ctx.text.inherited_blend();
    let quad = |dst: Rect, tint: Color| QuadParams { tint, blend, filter, ..QuadParams::new(dst) };

    if body.shadow != (0.0, 0.0) {
        let ink = place.tint;
        let shade = Color { r: ink.r / SHADOW_DIVISOR, g: ink.g / SHADOW_DIVISOR, b: ink.b / SHADOW_DIVISOR, a: ink.a };
        r.draw_textured_quad(tex, quad(Rect::new(at.x + body.shadow.0, at.y + body.shadow.1, at.w, at.h), shade));
    }
    r.draw_textured_quad(tex, quad(at, place.tint));
    true
}

/// Draws a line whose font did not load, in the default face at the destination's height.
fn draw_stand_in<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, place: &Placement<'_>, body: &TextBody, dst: Rect, line: &str) -> bool {
    ctx.text.reset_family();
    let scale = (dst.h / TEXT_PIXELS_PER_SCALE).max(MIN_TEXT_SCALE);
    match body.block_align() {
        BlockAlign::Left => ctx.draw_text(r, dst.x, dst.y, scale, place.tint, line),
        BlockAlign::Center => ctx.draw_text_centered(r, dst.x, dst.y, scale, place.tint, line),
        BlockAlign::Right => ctx.draw_text_right(r, dst.x, dst.y, scale, place.tint, line),
    }
    true
}
