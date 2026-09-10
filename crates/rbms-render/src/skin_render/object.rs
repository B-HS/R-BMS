//! A document's objects, resolved once at load into a flat draw list.
//!
//! Every entry pairs the destination track that animates it with the body that says what to draw:
//! which texture, which cell of it, how many digits, which font. Resolving all of that at load is
//! what lets a frame be a single pass with no lookups by name and no file access.
//!
//! The shapes follow the reference implementation's own object classes -- `SkinImage`,
//! `SkinNumber`, `SkinFloat`, `SkinSlider`, `SkinGraph` -- including how a strip of digits is cut
//! into sets and which glyph slot the sign and the decimal point occupy.

use std::borrow::Cow;

use rbms_skin::dst::{DestinationTrack, LuaExprId};
use rbms_skin::loader::{LoadedSkin, StretchKind};
use rbms_skin::model::{FloatValueDef, GraphDef, ImageDef, PropertyRef, SliderDef, TextDef, ValueDef};
use rbms_skin::property::SkinStateSource;
use rbms_skin::timer::TimerId;

use super::{SkinAssets, SkinExprEval};
use crate::{TextureId, UvRect};

/// Cells a division count of zero or less stands for: the whole image, undivided.
const SINGLE_DIVISION: u32 = 1;

/// Glyph slot holding the alternate zero a document asks for with `zeropadding` 2.
const GLYPH_REVERSE_ZERO: u32 = 10;

/// Glyph slot holding the decimal point of a fractional number.
const GLYPH_DECIMAL_POINT: u32 = 11;

/// Glyph slot holding the sign of a fractional number.
const GLYPH_SIGN: u32 = 12;

/// Glyph slot an integer strip keeps its sign in, one past the ten digits and the alternate zero.
const GLYPH_INTEGER_SIGN: u32 = 11;

/// Most digits a fractional number is drawn with, whole and fractional parts together
/// (`FloatFormatter.KETAMAX`).
const MAX_FRACTION_DIGITS: i32 = 8;

/// The most digit places one number object draws.
///
/// A whole number property is an `i32`, which is ten digits and a sign, and a fractional one is
/// held to [`MAX_FRACTION_DIGITS`] plus a point and a sign; sixteen covers both with room to spare.
/// The reference sizes its place array from the document's `digit` with no ceiling at all, so a
/// mistyped field there is a mistyped allocation -- here it is a slice that stops.
pub(crate) const MAX_PLACES: usize = 16;

/// Which kind of object a draw-list entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SkinObjectKind {
    Image,
    Number,
    Float,
    Text,
    Slider,
    Graph,
    Background,
}

/// One entry of the draw list.
#[derive(Debug)]
pub(crate) struct SkinObject {
    pub(crate) id: String,
    pub(crate) track: DestinationTrack,
    pub(crate) stretch: StretchKind,
    pub(crate) body: Body,
}

impl SkinObject {
    pub(crate) fn kind(&self) -> SkinObjectKind {
        match self.body {
            Body::Image(_) => SkinObjectKind::Image,
            Body::Number(_) => SkinObjectKind::Number,
            Body::Float(_) => SkinObjectKind::Float,
            Body::Text(_) => SkinObjectKind::Text,
            Body::Slider(_) => SkinObjectKind::Slider,
            Body::Graph(_) => SkinObjectKind::Graph,
            Body::Background => SkinObjectKind::Background,
        }
    }
}

/// What an object draws, once its source has been resolved.
#[derive(Debug)]
pub(crate) enum Body {
    Image(ImageBody),
    Number(NumberBody),
    Float(FloatBody),
    Text(TextBody),
    Slider(SliderBody),
    Graph(GraphBody),
    /// The background image slot, which the frame fills rather than the document.
    Background,
}

/// One registered texture cut into a grid of animation cells.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Sprite {
    pub(crate) tex: TextureId,
    /// The whole texture, in pixels.
    pub(crate) size: (u32, u32),
    /// Top left of the grid inside the texture, in pixels.
    pub(crate) origin: (u32, u32),
    /// One cell, in pixels.
    pub(crate) cell: (u32, u32),
    pub(crate) columns: u32,
    pub(crate) rows: u32,
    /// The timer the cell animation is measured from, or the frame clock when the document names
    /// none.
    pub(crate) timer: Option<TimerId>,
    /// Milliseconds one pass over the cells takes. Zero holds cell zero.
    pub(crate) cycle: i32,
}

impl Sprite {
    /// Cuts a texture into `divx` x `divy` cells over the `(x, y, w, h)` region a document named.
    ///
    /// A region with no stated extent covers the whole texture. The reference writes that as `-1`
    /// and throws on anything else non-positive; a zero is just as meaningless, so both are read
    /// the same lenient way here.
    fn new(tex: TextureId, size: (u32, u32), region: (i32, i32, i32, i32), divisions: (i32, i32), timer: Option<TimerId>, cycle: i32) -> Sprite {
        let (x, y, w, h) = region;
        let (columns, rows) = (division(divisions.0), division(divisions.1));
        let width = if w > 0 { w as u32 } else { size.0 };
        let height = if h > 0 { h as u32 } else { size.1 };
        Sprite { tex, size, origin: (x.max(0) as u32, y.max(0) as u32), cell: (width / columns, height / rows), columns, rows, timer, cycle }
    }

    /// How many cells the grid holds.
    pub(crate) fn cells(&self) -> u32 {
        self.columns * self.rows
    }

    /// One cell's size in texture pixels, which is what a stretch mode measures against.
    pub(crate) fn cell_size(&self) -> (f32, f32) {
        (self.cell.0 as f32, self.cell.1 as f32)
    }

    /// Where cell `index` sits in the texture, as normalised coordinates.
    pub(crate) fn uv(&self, index: u32) -> UvRect {
        let index = index.min(self.cells().saturating_sub(1));
        let (column, row) = (index % self.columns, index / self.columns);
        UvRect::from_pixels(self.origin.0 + column * self.cell.0, self.origin.1 + row * self.cell.1, self.cell.0, self.cell.1, self.size.0, self.size.1)
    }

    /// Which of `count` cells the animation is on, following `SkinSourceImage.getImageIndex`: a
    /// cycle of zero, a timer that is off, and a moment before the timer started all hold cell zero.
    pub(crate) fn animation_index(&self, count: u32, now_ms: i64, timers: &rbms_skin::timer::TimerState) -> u32 {
        if self.cycle <= 0 || count == 0 {
            return 0;
        }
        let mut time = now_ms;
        if let Some(timer) = self.timer {
            let Some(started) = timers.get(timer) else {
                return 0;
            };
            time -= started;
        }
        if time < 0 {
            return 0;
        }
        ((time.saturating_mul(i64::from(count)) / i64::from(self.cycle)) % i64::from(count)) as u32
    }
}

/// A division count of zero or less means the image is not divided.
fn division(value: i32) -> u32 {
    if value > 0 { value as u32 } else { SINGLE_DIVISION }
}

/// The glyph slots one number's places take, most significant first, with `None` for a place that
/// is left blank.
///
/// Inline storage rather than a `Vec`, because this is built afresh for every number object on
/// every frame and a document may hold hundreds of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Places {
    slots: [Option<u32>; MAX_PLACES],
    len: usize,
}

impl Places {
    /// `len` blank places, held to [`MAX_PLACES`].
    fn blank(len: usize) -> Places {
        Places { slots: [None; MAX_PLACES], len: len.min(MAX_PLACES) }
    }

    /// The places, in the order they are drawn.
    pub(crate) fn as_slice(&self) -> &[Option<u32>] {
        &self.slots[..self.len]
    }

    /// Whether there is nothing to draw at all.
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Puts `glyph` in one place, ignoring a place past the end.
    fn set(&mut self, index: usize, glyph: u32) {
        if index < self.len {
            self.slots[index] = Some(glyph);
        }
    }

    /// The glyph in one place, or `None` when it is blank or past the end.
    fn get(&self, index: usize) -> Option<u32> {
        self.slots.get(index).copied().flatten()
    }
}

/// Where an object reads one value from.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum ValueSource {
    /// Nothing was named, so the object draws its own default.
    #[default]
    None,
    /// A property id.
    Id(i32),
    /// A compiled expression.
    Expr(LuaExprId),
}

impl ValueSource {
    /// Reads a document field that is either a property id, an expression, or absent, compiling the
    /// expression now so a frame never has to.
    fn new(property: Option<&PropertyRef>, fallback: i32, assets: &mut dyn SkinAssets) -> ValueSource {
        match property {
            Some(PropertyRef::Id(id)) => ValueSource::Id(*id),
            Some(PropertyRef::Expr(source)) => assets.expression(source).map_or(ValueSource::None, ValueSource::Expr),
            None if fallback != 0 => ValueSource::Id(fallback),
            None => ValueSource::None,
        }
    }

    /// The integer this frame, or zero when nothing was named.
    pub(crate) fn integer(&self, state: &dyn SkinStateSource, lua: Option<&dyn SkinExprEval>) -> i32 {
        match self {
            ValueSource::None => 0,
            ValueSource::Id(id) => state.integer(*id),
            ValueSource::Expr(expr) => lua.and_then(|lua| lua.eval_integer(*expr)).unwrap_or_default(),
        }
    }

    /// The number this frame.
    ///
    /// Nothing is narrowed here: a rate id already answers between zero and one, and a `FLOAT_*` id
    /// carries a plain measurement -- a hi-speed multiplier, an average timing in milliseconds --
    /// that a drawn `floatvalue` shows as it is. What reads a *share* clamps at its own call site.
    pub(crate) fn float(&self, state: &dyn SkinStateSource, lua: Option<&dyn SkinExprEval>) -> f32 {
        let raw = match self {
            ValueSource::None => 0.0,
            ValueSource::Id(id) => state.float(*id),
            ValueSource::Expr(expr) => lua.and_then(|lua| lua.eval_float(*expr)).unwrap_or_default(),
        };
        rbms_skin::property::sanitize_float(raw)
    }

    /// The text this frame, borrowed from the state when it comes from a property so that a line
    /// which has not changed costs no allocation.
    pub(crate) fn text<'a>(&self, state: &'a dyn SkinStateSource, lua: Option<&dyn SkinExprEval>) -> Cow<'a, str> {
        match self {
            ValueSource::None => Cow::Borrowed(""),
            ValueSource::Id(id) => Cow::Borrowed(state.string(*id)),
            ValueSource::Expr(expr) => Cow::Owned(lua.and_then(|lua| lua.eval_text(*expr)).unwrap_or_default()),
        }
    }

    /// Whether the document named anything at all.
    pub(crate) fn is_named(&self) -> bool {
        !matches!(self, ValueSource::None)
    }
}

/// A still or animated image, or a set of them one property picks between.
#[derive(Debug)]
pub(crate) struct ImageBody {
    /// One entry per selectable variant, each a sprite and the cell range it animates over.
    pub(crate) variants: Vec<(Sprite, u32, u32)>,
    /// The integer that picks a variant.
    pub(crate) select: ValueSource,
}

/// Where each of a twelve-slot fractional set's glyphs sits in a strip that only carries eleven
/// cells per sign (`JsonSkinObjectLoader`, the `% 22` and `% 11` branches).
///
/// Such a strip has no alternate zero of its own, so the slot for one shares cell zero with the
/// ordinary zero, and the decimal point follows the ten digits rather than the eleven slots.
const SHARED_ZERO_FRACTION_CELLS: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 10];

/// The ten digits every strip starts with, which is also the smallest an integer strip can be.
const DIGITS_PER_SET: u32 = 10;

/// Cells one sign takes in the two fractional strips that share their zero.
const SHARED_ZERO_CELLS_PER_SIGN: u32 = 11;

/// How many glyph slots the one fractional layout that carries a sign holds.
const SIGNED_FRACTION_GLYPHS: u32 = 13;

/// How many glyph slots every other fractional layout holds: ten digits, an alternate zero and a
/// decimal point.
const FRACTION_GLYPHS: u32 = 12;

/// The `zeropadding` an eleven-cell integer strip is forced to, because its eleventh cell is the
/// alternate zero and there is nothing else it could be for (`JsonSkinObjectLoader`'s `d > 10`).
const FORCED_ALTERNATE_ZERO: i32 = 2;

/// How a digit strip is cut up: how many glyph slots one set answers for, where the negative half
/// of a set starts, and which cell each slot actually reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DigitLayout {
    /// Glyph slots one set answers for. A slot past this is left blank rather than reading into the
    /// next set.
    pub(crate) glyphs: u32,
    pub(crate) sets: u32,
    /// Whether negative values read from a half of their own.
    pub(crate) negative: bool,
    /// Cells one whole set takes in the strip, both halves together.
    stride: u32,
    /// Where the negative half starts inside a set.
    negative_offset: u32,
    /// Slot to cell offset inside one half, when the strip does not store its glyphs in slot order.
    remap: Option<&'static [u32]>,
}

impl DigitLayout {
    /// A layout whose cells are stored in slot order, one half per sign when `negative`.
    const fn packed(glyphs: u32, sets: u32, negative: bool) -> DigitLayout {
        let stride = if negative { glyphs * 2 } else { glyphs };
        DigitLayout { glyphs, sets, negative, stride, negative_offset: glyphs, remap: None }
    }

    /// Which cell of the strip slot `slot` of `set` reads, or `None` when the slot is past the end
    /// of a set and nothing is drawn for it.
    pub(crate) fn cell(&self, set: u32, negative: bool, slot: u32) -> Option<u32> {
        let within = match self.remap {
            Some(table) => *table.get(slot as usize)?,
            None if slot < self.glyphs => slot,
            None => return None,
        };
        let half = if negative && self.negative { self.negative_offset } else { 0 };
        Some(set * self.stride + half + within)
    }

    /// How an integer strip is cut (`JsonSkinObjectLoader`): a multiple of twenty-four carries a
    /// negative half of twelve glyphs, and anything else is ten or eleven glyphs with no negatives.
    pub(crate) fn integer(cells: u32) -> DigitLayout {
        if cells > 0 && cells.is_multiple_of(24) {
            return DigitLayout::packed(FRACTION_GLYPHS, cells / 24, true);
        }
        let glyphs = if cells > 0 && cells.is_multiple_of(DIGITS_PER_SET) { DIGITS_PER_SET } else { SHARED_ZERO_CELLS_PER_SIGN };
        DigitLayout::packed(glyphs, (cells / glyphs).max(1), false)
    }

    /// How a fractional strip is cut. The reference lists five shapes and treats everything else as
    /// twelve glyphs per set.
    ///
    /// Two of the five store eleven cells per sign and still answer twelve slots, because the
    /// alternate zero shares a cell with the ordinary one; those get [`SHARED_ZERO_FRACTION_CELLS`]
    /// rather than reading slot for cell.
    pub(crate) fn fraction(cells: u32) -> DigitLayout {
        if cells > 0 && cells.is_multiple_of(26) {
            return DigitLayout::packed(SIGNED_FRACTION_GLYPHS, cells / 26, true);
        }
        if cells > 0 && cells.is_multiple_of(24) {
            return DigitLayout::packed(FRACTION_GLYPHS, cells / 24, true);
        }
        if cells > 0 && cells.is_multiple_of(SHARED_ZERO_CELLS_PER_SIGN * 2) {
            return DigitLayout {
                glyphs: FRACTION_GLYPHS,
                sets: cells / (SHARED_ZERO_CELLS_PER_SIGN * 2),
                negative: true,
                stride: SHARED_ZERO_CELLS_PER_SIGN * 2,
                negative_offset: SHARED_ZERO_CELLS_PER_SIGN,
                remap: Some(SHARED_ZERO_FRACTION_CELLS),
            };
        }
        if cells > 0 && cells.is_multiple_of(12) {
            return DigitLayout::packed(FRACTION_GLYPHS, cells / 12, false);
        }
        if cells > 0 && cells.is_multiple_of(SHARED_ZERO_CELLS_PER_SIGN) {
            return DigitLayout {
                glyphs: FRACTION_GLYPHS,
                sets: cells / SHARED_ZERO_CELLS_PER_SIGN,
                negative: false,
                stride: SHARED_ZERO_CELLS_PER_SIGN,
                negative_offset: 0,
                remap: Some(SHARED_ZERO_FRACTION_CELLS),
            };
        }
        DigitLayout::packed(FRACTION_GLYPHS, (cells / FRACTION_GLYPHS).max(1), false)
    }
}

/// A whole number drawn from a digit strip.
#[derive(Debug)]
pub(crate) struct NumberBody {
    pub(crate) sprite: Sprite,
    pub(crate) layout: DigitLayout,
    /// How many digit places are drawn.
    pub(crate) digits: u32,
    /// 0 leaves leading places blank, 1 fills them with zeroes, 2 with the alternate zero.
    pub(crate) zero_padding: i32,
    /// Gap between places, in document pixels.
    pub(crate) space: f32,
    /// 0 leaves the places where they fall, 1 pulls them over the blanks, anything else centres.
    pub(crate) align: i32,
    pub(crate) value: ValueSource,
    /// Per-place nudges, as far as the document declared them.
    pub(crate) offsets: Vec<(f32, f32, f32, f32)>,
}

/// A fractional number drawn from a digit strip.
#[derive(Debug)]
pub(crate) struct FloatBody {
    pub(crate) sprite: Sprite,
    pub(crate) layout: DigitLayout,
    /// Whole-number places.
    pub(crate) integer_digits: i32,
    /// Fractional places.
    pub(crate) fraction_digits: i32,
    /// Whether a place is kept for the sign, which only the twenty-six-cell strip has a glyph for:
    /// every other branch of `JsonSkinObjectLoader` hands `SkinFloat` a literal false however the
    /// document set `isSignvisible`.
    pub(crate) sign: bool,
    pub(crate) zero_padding: i32,
    pub(crate) space: f32,
    /// Read the same way as a whole number's.
    pub(crate) align: i32,
    /// What the property is multiplied by before it is drawn.
    pub(crate) gain: f32,
    pub(crate) value: ValueSource,
    pub(crate) offsets: Vec<(f32, f32, f32, f32)>,
}

/// A run of text.
///
/// The document's `size` is not kept: it names the size the reference loads the font at, and the
/// drawn height is the destination's own, so the ratio the reference computes is already the
/// destination height here.
#[derive(Debug)]
pub(crate) struct TextBody {
    /// The family the text engine registered this document's font under.
    pub(crate) family: Option<String>,
    /// 0 left, 1 centred, 2 right.
    pub(crate) align: i32,
    pub(crate) value: ValueSource,
    /// Text the document wrote out rather than reading from a property.
    pub(crate) constant: Option<String>,
}

/// A handle that slides along its track.
#[derive(Debug)]
pub(crate) struct SliderBody {
    pub(crate) sprite: Sprite,
    /// 0 up, 1 right, 2 down, 3 left, in the document's y-up space.
    pub(crate) direction: i32,
    /// How far, in document pixels, a full-scale value moves the handle.
    pub(crate) range: f32,
    pub(crate) value: ValueSource,
    /// The whole-number property and its endpoints, when the document scales one itself.
    pub(crate) ref_num: Option<(i32, i32)>,
}

/// A bar that grows with its value.
#[derive(Debug)]
pub(crate) struct GraphBody {
    pub(crate) sprite: Sprite,
    /// 1 grows upwards, anything else rightwards.
    pub(crate) direction: i32,
    pub(crate) value: ValueSource,
    pub(crate) ref_num: Option<(i32, i32)>,
}

/// Everything the draw list needs about one image source.
type Source<'a> = &'a [(String, TextureId, (u32, u32))];

/// Looks an image source up by the id a document gave it.
fn source_of<'a>(sources: Source<'a>, id: &str) -> Option<&'a (String, TextureId, (u32, u32))> {
    sources.iter().find(|(source, _, _)| source == id)
}

/// The timer an object animates its cells with, when the document named one as a plain id.
fn cell_timer(property: Option<&PropertyRef>) -> Option<TimerId> {
    property.and_then(PropertyRef::id).map(TimerId)
}

/// The sprite an image definition cuts out of its source.
fn image_sprite(def: &ImageDef, sources: Source<'_>) -> Option<Sprite> {
    let (_, tex, size) = source_of(sources, &def.src)?;
    Some(Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle))
}

/// Builds every object the document's top-level destinations name, in document order.
///
/// A destination whose id names nothing this build draws is dropped with a warning rather than
/// failing the skin: a document written for a screen with more object kinds still shows everything
/// this build does understand.
pub(crate) fn build_objects(
    skin: &LoadedSkin,
    sources: Source<'_>,
    families: &[(String, String)],
    assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Vec<SkinObject> {
    let mut objects = Vec::with_capacity(skin.destinations.len());
    for named in &skin.destinations {
        let stretch = StretchKind::from_id(named.track.stretch);
        if !stretch.is_supported() {
            warnings.push(format!("object {:?} asks for stretch {:?}, which is drawn stretched instead", named.id, stretch));
        }
        let Some(body) = build_body(skin, &named.id, sources, families, assets, warnings) else {
            continue;
        };
        objects.push(SkinObject { id: named.id.clone(), track: named.track.clone(), stretch, body });
    }
    objects
}

/// The body behind one destination id, or `None` when nothing declares it.
fn build_body(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    families: &[(String, String)],
    assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = &skin.def;
    if let Some(image) = def.image.iter().find(|image| image.id == id) {
        return image_body(image, sources, assets, warnings).map(Body::Image);
    }
    if let Some(set) = def.imageset.iter().find(|set| set.id == id) {
        let variants: Vec<(Sprite, u32, u32)> = set
            .images
            .iter()
            .filter_map(|name| def.image.iter().find(|image| &image.id == name))
            .filter_map(|image| image_sprite(image, sources))
            .map(|sprite| (sprite, 0, sprite.cells()))
            .collect();
        if variants.is_empty() {
            warnings.push(format!("image set {id:?} names no image this build could load"));
            return None;
        }
        let select = ValueSource::new(set.value.as_ref(), set.reference, assets);
        return Some(Body::Image(ImageBody { variants, select }));
    }
    if let Some(value) = def.value.iter().find(|value| value.id == id) {
        return number_body(value, sources, assets, warnings).map(Body::Number);
    }
    if let Some(value) = def.floatvalue.iter().find(|value| value.id == id) {
        return float_body(value, sources, assets, warnings).map(Body::Float);
    }
    if let Some(text) = def.text.iter().find(|text| text.id == id) {
        return Some(Body::Text(text_body(text, families, assets)));
    }
    if let Some(slider) = def.slider.iter().find(|slider| slider.id == id) {
        return slider_body(slider, sources, assets, warnings).map(Body::Slider);
    }
    if let Some(graph) = def.graph.iter().find(|graph| graph.id == id) {
        return graph_body(graph, sources, assets, warnings).map(Body::Graph);
    }
    if def.bga.as_ref().is_some_and(|bga| bga.id == id) {
        return Some(Body::Background);
    }
    warnings.push(format!("object {id:?} is not a kind this build draws"));
    None
}

/// One image, animated over its cells and optionally split into variants a property picks between.
fn image_body(def: &ImageDef, sources: Source<'_>, assets: &mut dyn SkinAssets, warnings: &mut Vec<String>) -> Option<ImageBody> {
    let Some(sprite) = image_sprite(def, sources) else {
        warnings.push(format!("image {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let cells = sprite.cells();
    let groups = if def.len > 1 { (def.len as u32).min(cells.max(1)) } else { 1 };
    let per_group = (cells / groups).max(1);
    let variants = (0..groups).map(|group| (sprite, group * per_group, per_group)).collect();
    let select = if groups > 1 { ValueSource::new(None, def.reference, assets) } else { ValueSource::None };
    Some(ImageBody { variants, select })
}

/// A whole number and the strip it is drawn from.
fn number_body(def: &ValueDef, sources: Source<'_>, assets: &mut dyn SkinAssets, warnings: &mut Vec<String>) -> Option<NumberBody> {
    let Some((_, tex, size)) = source_of(sources, &def.src) else {
        warnings.push(format!("value {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let sprite = Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle);
    let layout = DigitLayout::integer(sprite.cells());
    Some(NumberBody {
        sprite,
        layout,
        digits: def.digit.clamp(1, MAX_PLACES as i32) as u32,
        zero_padding: integer_padding(&layout, def),
        space: def.space as f32,
        align: def.align,
        value: ValueSource::new(def.value.as_ref(), def.reference, assets),
        offsets: digit_offsets(&def.offset),
    })
}

/// Whether a fractional strip keeps a place for the sign.
///
/// Only the twenty-six-cell strip has a glyph for one, and it is the only branch of
/// `JsonSkinObjectLoader` that passes the document's `isSignvisible` on: every other branch hands
/// `SkinFloat` a literal false. Honouring the flag on a strip with no sign glyph would widen the
/// number by a place and leave that place blank.
pub(crate) fn fraction_sign(is_sign_visible: bool, layout: &DigitLayout) -> bool {
    is_sign_visible && layout.glyphs == SIGNED_FRACTION_GLYPHS
}

/// Which of a value definition's two padding fields a strip reads (`JsonSkinObjectLoader`).
///
/// Only the twenty-four-cell strip, the one with a negative half, reads `zeropadding`. The others
/// read `padding`, and an eleven-cell strip is forced to the alternate zero because its eleventh
/// cell is that glyph and nothing else.
pub(crate) fn integer_padding(layout: &DigitLayout, def: &ValueDef) -> i32 {
    if layout.negative {
        return def.zeropadding;
    }
    if layout.glyphs > DIGITS_PER_SET { FORCED_ALTERNATE_ZERO } else { def.padding }
}

/// A fractional number and the strip it is drawn from.
fn float_body(def: &FloatValueDef, sources: Source<'_>, assets: &mut dyn SkinAssets, warnings: &mut Vec<String>) -> Option<FloatBody> {
    let Some((_, tex, size)) = source_of(sources, &def.src) else {
        warnings.push(format!("float value {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let sprite = Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle);
    let layout = DigitLayout::fraction(sprite.cells());
    let (integer_digits, fraction_digits) = fraction_places(def.iketa, def.fketa);
    Some(FloatBody {
        sprite,
        layout,
        integer_digits,
        fraction_digits,
        sign: fraction_sign(def.is_sign_visible, &layout),
        zero_padding: def.zeropadding,
        space: def.space as f32,
        align: def.align,
        gain: def.gain,
        value: ValueSource::new(def.value.as_ref(), def.reference, assets),
        offsets: digit_offsets(&def.offset),
    })
}

/// How many whole and fractional places a document actually gets, with the fractional part keeping
/// its full width when the two together ask for more than the strip can show
/// (`FloatFormatter::new`).
fn fraction_places(iketa: i32, fketa: i32) -> (i32, i32) {
    let whole = iketa.max(0);
    let fraction = fketa.max(0);
    if whole >= MAX_FRACTION_DIGITS || fraction >= MAX_FRACTION_DIGITS || whole + fraction >= MAX_FRACTION_DIGITS {
        let fraction = fraction.min(MAX_FRACTION_DIGITS);
        return (MAX_FRACTION_DIGITS - fraction, fraction);
    }
    (whole, fraction)
}

/// The per-place nudges a number definition carries.
fn digit_offsets(offsets: &[ValueDef]) -> Vec<(f32, f32, f32, f32)> {
    offsets.iter().map(|offset| (offset.x as f32, offset.y as f32, offset.w as f32, offset.h as f32)).collect()
}

/// A text run and the font it is drawn with.
fn text_body(def: &TextDef, families: &[(String, String)], assets: &mut dyn SkinAssets) -> TextBody {
    TextBody {
        family: families.iter().find(|(id, _)| *id == def.font).map(|(_, family)| family.clone()),
        align: def.align,
        value: ValueSource::new(def.value.as_ref(), def.reference, assets),
        constant: def.constant_text.clone(),
    }
}

/// A slider and the track it moves along.
fn slider_body(def: &SliderDef, sources: Source<'_>, assets: &mut dyn SkinAssets, warnings: &mut Vec<String>) -> Option<SliderBody> {
    let Some((_, tex, size)) = source_of(sources, &def.src) else {
        warnings.push(format!("slider {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let sprite = Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle);
    let value = ValueSource::new(def.value.as_ref(), def.slider_type, assets);
    Some(SliderBody {
        sprite,
        direction: def.angle,
        range: def.range as f32,
        value,
        ref_num: (def.is_ref_num && def.max > def.min).then_some((def.min, def.max)),
    })
}

/// A bar graph and the direction it grows in.
fn graph_body(def: &GraphDef, sources: Source<'_>, assets: &mut dyn SkinAssets, warnings: &mut Vec<String>) -> Option<GraphBody> {
    let Some((_, tex, size)) = source_of(sources, &def.src) else {
        warnings.push(format!("graph {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let sprite = Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle);
    Some(GraphBody {
        sprite,
        direction: def.angle,
        value: ValueSource::new(def.value.as_ref(), def.graph_type, assets),
        ref_num: (def.is_ref_num && def.max > def.min).then_some((def.min, def.max)),
    })
}

/// The glyph slots a whole number's places take, most significant first, with `None` for a place
/// that is left blank (`SkinNumber.prepare`).
pub(crate) fn integer_glyphs(body: &NumberBody, value: i32) -> Places {
    let mut slots = Places::blank(body.digits as usize);
    let places = slots.len;
    if places == 0 {
        return slots;
    }
    let signed = body.layout.negative;
    let mut remaining = value.unsigned_abs();
    for place in (0..places).rev() {
        let last = place == places - 1;
        let glyph = if signed && body.zero_padding > 0 {
            if place == 0 {
                Some(GLYPH_INTEGER_SIGN)
            } else if remaining > 0 || last {
                Some(remaining % 10)
            } else {
                Some(if body.zero_padding == 2 { GLYPH_REVERSE_ZERO } else { 0 })
            }
        } else if remaining > 0 || last {
            Some(remaining % 10)
        } else if body.zero_padding == 2 {
            Some(GLYPH_REVERSE_ZERO)
        } else if body.zero_padding == 1 {
            Some(0)
        } else if signed && slots.get(place + 1).is_some_and(|next| next != GLYPH_INTEGER_SIGN) {
            Some(GLYPH_INTEGER_SIGN)
        } else {
            None
        };
        if let Some(glyph) = glyph {
            slots.set(place, glyph);
        }
        remaining /= 10;
    }
    slots
}

/// The glyph slots a fractional number's places take (`FloatFormatter.calcuateAndGetDigits`).
pub(crate) fn fraction_glyphs(body: &FloatBody, value: f64) -> Places {
    let sign = i32::from(body.sign);
    let point = i32::from(body.fraction_digits != 0);
    let length = sign + body.integer_digits + body.fraction_digits + point;
    let mut slots = Places::blank(length.max(0) as usize);
    if slots.is_empty() {
        return slots;
    }
    if body.integer_digits == 0 && body.fraction_digits == 0 && sign == 1 {
        slots.set(0, GLYPH_SIGN);
        return slots;
    }

    let scale = 10f64.powi(body.integer_digits);
    let show_sign = sign == 1 && value < scale;
    let mut base = sign + body.integer_digits;
    if body.zero_padding == 0 {
        let whole = value as i64;
        let width = (if whole != 0 { whole.abs() as f64 } else { 1.0 }).log10() as i32 + 1;
        base = body.integer_digits.min(width) + sign;
    }
    let mut scaled = (value * 10f64.powi(body.fraction_digits)) as i64;
    let mut place = if body.integer_digits == 0 { body.fraction_digits + sign + 1 } else { base + body.fraction_digits + point };
    let mut fraction_left = body.fraction_digits;
    while place > sign {
        let index = (place - 1).max(0) as usize;
        slots.set(index, if fraction_left <= -1 && scaled == 0 && body.zero_padding == 2 { GLYPH_REVERSE_ZERO } else { (scaled % 10).unsigned_abs() as u32 });
        fraction_left -= 1;
        if fraction_left == 0 {
            place -= 1;
            slots.set((place - 1).max(0) as usize, GLYPH_DECIMAL_POINT);
        }
        scaled /= 10;
        place -= 1;
    }
    if place == 1 {
        slots.set(0, if show_sign { GLYPH_SIGN } else { (scaled % 10).unsigned_abs() as u32 });
    }
    if body.integer_digits == 0 && sign == 1 {
        slots.set(0, GLYPH_SIGN);
    }
    slots
}
