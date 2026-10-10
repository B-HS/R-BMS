//! An editable `text` object: a run of text the player can type into.
//!
//! The reference keeps one class for both (`SkinText`, whose `editable` flag is what makes
//! `Skin.mousePressed` open an input over it). It is an object of its own here because it is the
//! only text that takes input.
//!
//! # What makes a text editable
//!
//! `JsonSkinObjectLoader.createText` gives a text a writer -- its `event`, or the writer the
//! reference has under its `ref` -- and makes it editable when the document says so *or* when it
//! names no `event` of its own and the reference can write its `ref`
//! ([`is_editable`]). That second clause is what makes the search box of a skin that only wrote
//! `ref = 30` a box one can type into. A text with an `event` and no `editable` is not editable.
//!
//! # What it does with the pointer
//!
//! A press inside its input bounds ([`input_bounds`]) is reported as
//! [`SkinAction::FocusText`](super::input::SkinAction::FocusText) and stops the walk, writer or no
//! writer. Whether there is anything to type into is then asked of the screen
//! ([`SkinScreen::text_entry_start`]): a text with no writer does not take focus
//! (`SkinTextInput.focus`).
//!
//! # What it draws while it is typed into
//!
//! The reference lays a scene2d `TextField` over the text, in the system font. Here the typed
//! line is drawn by the text object itself, in its own font and its own place, instead of what the
//! text object shows ([`TextEntry`]), so what is typed is where and as the skin lays out its text.
//! The caret is a bar the height of the destination, as wide as the reference's two pixel cursor,
//! standing between the glyphs the line was composed with ([`crate::font::TextContext::caret_x`]). Text an input
//! method is still composing is drawn at the caret and underlined, and hides the caret when the input
//! method asks for none.

use rbms_skin::dst::{LuaFnId, SkinRect};
use rbms_skin::model::{StringWriterRef, TextDef};
use rbms_skin::property::{NameSpace, id_of_name, reference_writes};

use super::draw::Placement;
use super::object::Body;
use super::text::{MIN_TEXT_SCALE, TEXT_PIXELS_PER_SCALE, TextBody, draw_line, draw_text, text_body};
use super::{SkinFrame, SkinScreen};
use crate::ctx::RenderCtx;
use crate::font::BlockAlign;
use crate::{Rect, Renderer};

#[cfg(test)]
mod tests;

/// A text's `align` that puts its region's x at the middle of the text, and the one that puts it at
/// the right end (`SkinText.getInputBounds`). Anything else is the left end.
const ALIGN_CENTER: i32 = 1;
const ALIGN_RIGHT: i32 = 2;

/// How wide the caret is, in pixels of the document: the reference's cursor is a texture two pixels
/// wide (`SkinTextInput.ensureTextField`).
const CARET_WIDTH: f32 = 2.0;

/// How thick the line under composing text is, in pixels of the document.
const UNDERLINE_THICKNESS: f32 = 2.0;

/// The narrowest a bar is drawn, in screen pixels, whatever the document is scaled down to.
const MIN_BAR_PX: f32 = 1.0;

/// Where an editable text writes what was typed into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinTextWriter {
    /// A string id, for the host's [`write_text`](rbms_skin::property::SkinHost::write_text).
    Id(i32),
    /// A function in the skin's interpreter, called with the text.
    Function(LuaFnId),
}

/// What an editable text starts being typed into from, and where it writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEntryStart {
    pub writer: SkinTextWriter,
    /// What the text shows when it is pressed, which is what the typing begins with
    /// (`SkinTextInput.focus`: `textField.setText(target.getCurrentText())`).
    pub shown: String,
}

/// Text an input method is composing at the caret, which the line being typed does not hold yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Composition<'a> {
    pub text: &'a str,
    /// Where the input method's own caret stands inside `text`, in characters, or `None` when it
    /// asks for none to be shown.
    pub caret: Option<usize>,
}

/// The editable text that is being typed into, as a frame draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextEntry<'a> {
    /// Which of the screen's objects it is.
    pub object: usize,
    /// What has been typed.
    pub typed: &'a str,
    /// How many characters of it are before the caret.
    pub caret: usize,
    pub composing: Option<Composition<'a>>,
}

/// The writer a text declares in its `event`, or `None` when it names none the reference can build
/// (`StringWriter` is read by name only: a number names nothing).
fn event_writer(def: &TextDef) -> Option<SkinTextWriter> {
    match def.event.as_ref()? {
        StringWriterRef::Lua(function) => Some(SkinTextWriter::Function(*function)),
        StringWriterRef::Name(name) => id_of_name(NameSpace::Text, name).filter(|id| reference_writes(NameSpace::Text, *id)).map(SkinTextWriter::Id),
        StringWriterRef::Script(_) => None,
    }
}

/// Where a text writes what is typed into it: its `event`, or else the writer the reference has under
/// its `ref` (`text.event != null ? text.event : getStringWriter(text.ref)`).
pub(crate) fn text_writer(def: &TextDef) -> Option<SkinTextWriter> {
    event_writer(def).or_else(|| reference_writes(NameSpace::Text, def.reference).then_some(SkinTextWriter::Id(def.reference)))
}

/// Whether a text is one the player can type into (`text.editable || (text.event == null && writer
/// != null)`).
pub(crate) fn is_editable(def: &TextDef) -> bool {
    def.editable || (event_writer(def).is_none() && text_writer(def).is_some())
}

/// Where a press is taken as a press on an editable text whose region is `region`: the region itself,
/// moved left by its whole width for a right-aligned text and by half of it for a centred one
/// (`SkinText.getInputBounds`).
pub(crate) fn input_bounds(region: SkinRect, align: i32) -> SkinRect {
    let left = match align {
        ALIGN_RIGHT => region.x - region.w,
        ALIGN_CENTER => region.x - region.w / 2.0,
        _ => region.x,
    };
    SkinRect::new(left, region.y, region.w, region.h)
}

/// A text object the document marked `editable`, or that the reference makes editable by itself.
#[derive(Debug)]
pub(crate) struct TextInputBody {
    /// What is shown while nothing is being typed, which is the text object the document declared.
    text: TextBody,
    /// Where confirming the typing writes, or `None` for a text with nothing to write to.
    writer: Option<SkinTextWriter>,
}

impl TextInputBody {
    /// The text object this was declared as, which is what the prepare stage reads the value of.
    pub(crate) fn shown(&self) -> &TextBody {
        &self.text
    }
}

/// The editable text a text record declares.
pub(crate) fn text_input_body(def: &TextDef, families: &[(String, String)]) -> TextInputBody {
    TextInputBody { text: text_body(def, families), writer: text_writer(def) }
}

impl SkinScreen {
    /// What typing into the editable text that is this screen's object number `object` starts from,
    /// read against `frame`, or `None` when it is no editable text or has no writer to confirm to
    /// (`SkinTextInput.focus` returns without taking focus then).
    pub fn text_entry_start(&self, object: usize, frame: &SkinFrame<'_>) -> Option<TextEntryStart> {
        let Body::TextInput(input) = &self.objects.get(object)?.body else {
            return None;
        };
        Some(TextEntryStart { writer: input.writer?, shown: input.text.shown(frame).into_owned() })
    }
}

/// Draws an editable text: what it shows while nothing is being typed into it, and the line being
/// typed, with its caret, while `frame` says it is the object `index` that is.
pub(crate) fn draw_text_input<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &TextInputBody,
    index: usize,
    rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    match frame.data.entry.filter(|entry| entry.object == index) {
        Some(entry) => draw_entry(ctx, r, place, &body.text, rect, &entry),
        None => draw_text(ctx, r, place, &body.text, rect, frame),
    }
}

/// The left edge a line is laid out from, for an alignment: the destination's x is the anchor, and a
/// line is placed against it by its alignment.
fn anchor(align: BlockAlign, dst: Rect) -> f32 {
    match align {
        BlockAlign::Left => dst.x,
        BlockAlign::Center => dst.x - dst.w / 2.0,
        BlockAlign::Right => dst.x - dst.w,
    }
}

/// Where the caret of `line` stands after its first `chars` characters, in screen pixels, the way the
/// text object lays the line out.
fn caret_x<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &R, body: &TextBody, dst: Rect, line: &str, chars: usize) -> f32 {
    let align = body.block_align();
    if let Some(spec) = body.layout(line, dst, r.max_texture_size()) {
        return anchor(align, dst) + ctx.text.caret_x(&spec, chars).unwrap_or_default();
    }
    ctx.text.reset_family();
    let scale = (dst.h / TEXT_PIXELS_PER_SCALE).max(MIN_TEXT_SCALE);
    let before: String = line.chars().take(chars).collect();
    let before = ctx.text.text_width(&before, scale);
    let whole = ctx.text.text_width(line, scale);
    match align {
        BlockAlign::Left => dst.x + before,
        BlockAlign::Center => dst.x - whole / 2.0 + before,
        BlockAlign::Right => dst.x - whole + before,
    }
}

/// Draws the line being typed in the place and the font of the text object, with a caret.
fn draw_entry<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, place: &Placement<'_>, body: &TextBody, rect: SkinRect, entry: &TextEntry<'_>) -> bool {
    let caret = entry.caret.min(entry.typed.chars().count());
    let before: String = entry.typed.chars().take(caret).collect();
    let after: String = entry.typed.chars().skip(caret).collect();
    let composing = entry.composing.map_or("", |composing| composing.text);
    let line = format!("{before}{composing}{after}");
    draw_line(ctx, r, place, body, rect, &line);

    let dst = place.viewport.place(rect);
    let bar = |across: f32| (across * place.viewport.scale_x()).max(MIN_BAR_PX);
    let composed_from = caret;
    let composed_to = caret + composing.chars().count();

    if let Some(composition) = entry.composing {
        let from = caret_x(ctx, r, body, dst, &line, composed_from);
        let to = caret_x(ctx, r, body, dst, &line, composed_to);
        let thick = bar(UNDERLINE_THICKNESS);
        r.fill_rect(Rect::new(from, dst.y + dst.h - thick, (to - from).max(MIN_BAR_PX), thick), place.tint);
        if composition.caret.is_none() {
            return true;
        }
    }
    let inside = entry.composing.and_then(|composition| composition.caret).unwrap_or_default().min(composed_to - composed_from);
    let at = caret_x(ctx, r, body, dst, &line, composed_from + inside);
    r.fill_rect(Rect::new(at, dst.y, bar(CARET_WIDTH), dst.h), place.tint);
    true
}
