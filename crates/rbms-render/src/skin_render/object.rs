//! A document's objects, resolved once at load into a flat draw list.
//!
//! Every entry pairs the destination track that animates it with the body that says what to draw:
//! which texture, which cell of it, how many digits, which font. Resolving all of that at load is
//! what lets a frame be a single pass with no lookups by name and no file access.
//!
//! The shapes follow the reference implementation's own object classes -- `SkinImage`,
//! `SkinNumber`, `SkinFloat`, `SkinSlider`, `SkinGraph` -- including how a strip of digits is cut
//! into sets and which glyph slot the sign and the decimal point occupy.
//!
//! So does the order each kind is prepared in ([`SkinObject::prepare`]). A skin's Lua is called
//! while an object is prepared and at no other time, and which of an object's functions run on a
//! frame it is hidden on differs from class to class; that order is the reference's, kind by kind.

use std::borrow::Cow;

use rbms_skin::dst::{DestinationTrack, DrawStateSource, Keyframe, LuaDrawEval, LuaFnId, Resolved, SkinColor, SkinRect, TimerRef, prepare};
use rbms_skin::loader::{LoadedSkin, StretchKind};
use rbms_skin::model::{FloatValueDef, GraphDef, ImageDef, PropertyRef, SliderDef, ValueDef};
use rbms_skin::property::SkinHost;
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerState};

use super::draw::{ImageSelect, float_value, number_value, share};
use super::textures::{Source, source_of};
use super::{SkinAssets, SkinFrame, bga, covers, gauge, graphs, judge, notes, refs, songlist, text, text_input};
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

/// The keyframe the reference gives a note field, a judgement pop-up and a song wheel as they are
/// constructed, before the document's own destination is read: at time zero, no extent, white and
/// fully transparent (`SkinNote`, `SkinJudge` and `SkinBar` each end their constructor with
/// `setDestination(0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 0, 0, 0, 0, 0, 0, new int[0])`).
const SELF_PLACED_KEYFRAME: Keyframe = Keyframe {
    time_ms: 0,
    rect: SkinRect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
    clip: None,
    color: SkinColor { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: 0 },
    angle_deg: 0.0,
};

/// The colour the parts of a self-placed object are drawn through. The reference never reads such
/// an object's own colour: its renderer sets the batch to white or draws each part in the colour of
/// that part's own destination (`LaneRenderer.drawLane`, `BarRenderer.render`, `SkinJudge.draw`).
const SELF_PLACED_COLOR: SkinColor = SkinColor { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: u8::MAX };

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
    Note,
    Gauge,
    Judge,
    SongList,
    HiddenCover,
    LiftCover,
    GaugeGraph,
    JudgeGraph,
    BpmGraph,
    TimingDistribution,
    TimingVisualizer,
    HitError,
    /// An image the document refers to by a negative destination id rather than ships.
    Reference,
}

/// One entry of the draw list.
#[derive(Debug)]
pub(crate) struct SkinObject {
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
            Body::Text(_) | Body::TextInput(_) => SkinObjectKind::Text,
            Body::Slider(_) => SkinObjectKind::Slider,
            Body::Graph(_) => SkinObjectKind::Graph,
            Body::Bga(_) => SkinObjectKind::Background,
            Body::Reference(_) => SkinObjectKind::Reference,
            Body::Note(_) => SkinObjectKind::Note,
            Body::Gauge(_) => SkinObjectKind::Gauge,
            Body::Judge(_) => SkinObjectKind::Judge,
            Body::SongList(_) => SkinObjectKind::SongList,
            Body::HiddenCover(_) => SkinObjectKind::HiddenCover,
            Body::LiftCover(_) => SkinObjectKind::LiftCover,
            Body::GaugeGraph(_) => SkinObjectKind::GaugeGraph,
            Body::JudgeGraph(_) => SkinObjectKind::JudgeGraph,
            Body::BpmGraph(_) => SkinObjectKind::BpmGraph,
            Body::TimingDistribution(_) => SkinObjectKind::TimingDistribution,
            Body::TimingVisualizer(_) => SkinObjectKind::TimingVisualizer,
            Body::HitError(_) => SkinObjectKind::HitError,
        }
    }
}

impl SkinObject {
    /// Hands back the textures this object made for itself while it was drawn. The ones it draws
    /// from belong to the screen, which hands those back on its own.
    pub(crate) fn release<R: crate::Renderer>(&self, r: &mut R) {
        graphs::release(&self.body, r);
        text::release(&self.body, r);
        songlist::release(&self.body, r);
    }

    /// The entry one top-level destination becomes.
    ///
    /// An object that places itself starts from the keyframe the reference constructs it with, so
    /// the document's own destination only adds to it ([`self_placed_track`]).
    fn new(declared: &DestinationTrack, body: Body) -> SkinObject {
        let track = if body.places_itself() { self_placed_track(declared) } else { declared.clone() };
        SkinObject { stretch: StretchKind::from_id(track.stretch), track, body }
    }

    /// Everything this object settles before a frame is drawn, or `None` when it is not drawn this
    /// frame (`SkinObject.prepare` and each subclass's override of it).
    ///
    /// The steps every kind shares are [`Self::place`]: the draw conditions in the order the
    /// document declared them, stopping at the first that fails, then the timer, then where the
    /// keyframes put the object. What differs by kind is when the value is read and whether a
    /// hidden object reads it at all, and a skin's Lua sees that difference because a value written
    /// as a function is called exactly then:
    ///
    /// | kind | order | a hidden object still reads |
    /// | --- | --- | --- |
    /// | image, image set | value, shared steps, source timer | the value and the source timer |
    /// | number, float | value, shared steps, source timer | the value |
    /// | text | shared steps, value | the value |
    /// | slider, graph | shared steps, source timer, value | nothing |
    ///
    /// A number whose value is one of the "no value" sentinels returns before the shared steps, so
    /// not even its conditions are evaluated (`SkinNumber.prepare`, `SkinFloat.prepare`). So does an
    /// image whose selecting value is negative (`SkinImage.prepare`); one that selects a set the
    /// document could not supply goes through the shared steps first and is left out after them.
    ///
    /// `frame.lua` is what every one of those reads goes through, for every kind: a condition
    /// through [`place_part`], a destination's timer and a source's timer through
    /// [`TimerRef::value_us`], and a value through [`ValueSource`]. Drawing reads the same values
    /// again, and by then they are answers kept from here.
    ///
    /// How often a function is called differs by what it stands for. A condition and a value are
    /// called once for each object that asks, on every frame the table above has it asked. A timer
    /// is not: the reference calls a timer function twice for each read (`TimerProperty.isOff`, then
    /// `get`), and here the frame's evaluator calls it the first time anything reads it and hands
    /// that answer to every later read of the frame, another object's included. Which frames a
    /// timer function runs on is unchanged -- it runs when the first object that reaches its timer
    /// does -- so a skin that does work inside one still has it done on the frame it expects.
    pub(crate) fn prepare(&self, frame: &SkinFrame<'_>) -> Option<Resolved> {
        match &self.body {
            Body::Image(body) => {
                let slot = body.select.slot(body.variants.len(), frame)?;
                let placed = self.place(frame);
                let (sprite, _, _) = body.variants.get(slot)?.as_ref()?;
                sprite.prepare(frame);
                placed
            }
            Body::Number(body) => {
                number_value(&body.value, frame)?;
                let placed = self.place(frame)?;
                body.sprite.prepare(frame);
                Some(placed)
            }
            Body::Float(body) => {
                float_value(body, frame)?;
                let placed = self.place(frame)?;
                body.sprite.prepare(frame);
                Some(placed)
            }
            Body::Text(body) => {
                let placed = self.place(frame);
                prepare_text(body, frame);
                placed
            }
            Body::TextInput(body) => {
                let placed = self.place(frame);
                prepare_text(body.shown(), frame);
                placed
            }
            Body::Slider(body) => {
                let placed = self.place(frame)?;
                body.sprite.prepare(frame);
                share(&body.value, body.ref_num, frame);
                Some(placed)
            }
            Body::Graph(body) => {
                let placed = self.place(frame)?;
                body.sprite.prepare(frame);
                share(&body.value, body.ref_num, frame);
                Some(placed)
            }
            Body::Note(body) => {
                let placed = self.place(frame)?;
                for lane in &body.lanes {
                    let sprites = [lane.note, lane.ln_end, lane.ln_start, lane.ln_body_active, lane.ln_body, lane.mine, lane.hidden];
                    sprites.iter().flatten().for_each(|sprite| sprite.prepare(frame));
                }
                for bar in &body.bars {
                    place_part(&bar.track, frame);
                    bar.sprite.prepare(frame);
                }
                Some(placed)
            }
            Body::Judge(body) => {
                let placed = self.place(frame)?;
                judge::prepare_judge(body, frame);
                Some(placed)
            }
            Body::SongList(body) => {
                let placed = self.place(frame)?;
                songlist::prepare_songlist(body, frame);
                Some(placed)
            }
            Body::Gauge(body) => {
                let placed = self.place(frame)?;
                body.nodes.iter().for_each(|sprite| sprite.prepare(frame));
                Some(placed)
            }
            Body::HiddenCover(body) | Body::LiftCover(body) => {
                let placed = self.place(frame)?;
                body.sprite.prepare(frame);
                Some(placed)
            }
            Body::Bga(_)
            | Body::Reference(_)
            | Body::GaugeGraph(_)
            | Body::JudgeGraph(_)
            | Body::BpmGraph(_)
            | Body::TimingDistribution(_)
            | Body::TimingVisualizer(_)
            | Body::HitError(_) => self.place(frame),
        }
    }

    /// The steps of [`Self::prepare`] every kind shares: the draw conditions, the timer, and where
    /// the keyframes put the object on this frame (`SkinObject.prepare`).
    ///
    /// An object that places itself answers [`SELF_PLACED_COLOR`] whatever its keyframes say, so
    /// its parts are drawn in their own colours and the transparent keyframe it starts from does
    /// not hide them.
    fn place(&self, frame: &SkinFrame<'_>) -> Option<Resolved> {
        let mut placed = place_part(&self.track, frame)?;
        if self.body.places_itself() {
            placed.color = SELF_PLACED_COLOR;
        }
        Some(placed)
    }
}

/// Where one destination sits this frame, or `None` when its conditions or its timer leave it out.
fn place_part(track: &DestinationTrack, frame: &SkinFrame<'_>) -> Option<Resolved> {
    let state: &dyn DrawStateSource = frame.state;
    prepare(track, frame.now_us, frame.timers, state, frame.script(), (0.0, 0.0), frame.mouse)
}

/// The track a self-placed object animates on: the keyframe the reference constructs it with, and
/// then whatever the document's destination adds.
///
/// A destination with no `dst` adds almost nothing. The reference's loader applies a destination's
/// timer, loop, blend, filter, centre, conditions and pointer rectangle once per keyframe it reads
/// (`JSONSkinLoader.setDestination`), so with no keyframe to read none of them are applied at all;
/// only the offsets and the stretch, which it sets after that loop, reach the object. That is how
/// `{ id = "notes", offset = 30 }` draws a note field: the object is always there, and the lists
/// nested under it say where everything goes.
///
/// A destination that does carry keyframes has them added after the constructed one, which stays
/// first among those at time zero because the reference inserts a keyframe ahead of the first later
/// one and no earlier.
fn self_placed_track(declared: &DestinationTrack) -> DestinationTrack {
    if declared.frames.is_empty() {
        return DestinationTrack {
            offsets: declared.offsets.clone(),
            relative: declared.relative,
            stretch: declared.stretch,
            frames: vec![SELF_PLACED_KEYFRAME],
            ..DestinationTrack::default()
        };
    }
    let mut track = declared.clone();
    let at = track.frames.partition_point(|frame| frame.time_ms < SELF_PLACED_KEYFRAME.time_ms);
    track.frames.insert(at, SELF_PLACED_KEYFRAME);
    track
}

/// Reads the text a text object shows, when the document named a property to read it from
/// (`SkinText.prepare`, which reads it whether or not the object is drawn).
fn prepare_text(body: &text::TextBody, frame: &SkinFrame<'_>) {
    if body.value.is_named() {
        body.value.text(frame.state, frame.lua);
    }
}

/// What an object draws, once its source has been resolved.
#[derive(Debug)]
pub(crate) enum Body {
    Image(ImageBody),
    Number(NumberBody),
    Float(FloatBody),
    Text(text::TextBody),
    /// A text object the document marked editable.
    TextInput(text_input::TextInputBody),
    Slider(SliderBody),
    Graph(GraphBody),
    /// The background image slot, which the frame fills rather than the document.
    Bga(bga::BgaBody),
    /// An image the frame supplies, named by a negative destination id.
    Reference(refs::ReferenceBody),
    Note(notes::NoteBody),
    Gauge(gauge::GaugeBody),
    Judge(judge::JudgeBody),
    SongList(songlist::SongListBody),
    HiddenCover(covers::CoverBody),
    LiftCover(covers::CoverBody),
    GaugeGraph(graphs::GaugeGraphBody),
    JudgeGraph(graphs::JudgeGraphBody),
    BpmGraph(graphs::BpmGraphBody),
    TimingDistribution(graphs::TimingDistributionBody),
    TimingVisualizer(graphs::TimingVisualizerBody),
    HitError(graphs::HitErrorBody),
}

impl Body {
    /// Whether the reference constructs this object with a destination of its own, so it is placed
    /// by the lists nested under it rather than by the document's destination: the note field, the
    /// judgement pop-up and the song wheel.
    pub(crate) fn places_itself(&self) -> bool {
        matches!(self, Body::Note(_) | Body::Judge(_) | Body::SongList(_))
    }
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
    pub(crate) timer: Option<TimerRef>,
    /// Milliseconds one pass over the cells takes. Zero holds cell zero.
    pub(crate) cycle: i32,
}

impl Sprite {
    /// Cuts a texture into `divx` x `divy` cells over the `(x, y, w, h)` region a document named.
    ///
    /// A region with no stated extent covers the whole texture. The reference writes that as `-1`
    /// and throws on anything else non-positive; a zero is just as meaningless, so both are read
    /// the same lenient way here.
    fn new(tex: TextureId, size: (u32, u32), region: (i32, i32, i32, i32), divisions: (i32, i32), timer: Option<TimerRef>, cycle: i32) -> Sprite {
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

    /// Where cell `index` sits in the texture, in pixels: the region a stretch mode measures against
    /// and, for the trimming modes, cuts down.
    pub(crate) fn region(&self, index: u32) -> SkinRect {
        let index = index.min(self.cells().saturating_sub(1));
        let (column, row) = (index % self.columns, index / self.columns);
        SkinRect::new((self.origin.0 + column * self.cell.0) as f32, (self.origin.1 + row * self.cell.1) as f32, self.cell.0 as f32, self.cell.1 as f32)
    }

    /// A pixel region of this sprite's texture as normalised coordinates.
    pub(crate) fn region_uv(&self, region: SkinRect) -> UvRect {
        let (width, height) = (self.size.0.max(1) as f32, self.size.1.max(1) as f32);
        UvRect::new(region.x / width, region.y / height, (region.x + region.w) / width, (region.y + region.h) / height)
    }

    /// Where cell `index` sits in the texture, as normalised coordinates.
    pub(crate) fn uv(&self, index: u32) -> UvRect {
        self.region_uv(self.region(index))
    }

    /// Reads the timer the cell animation is measured from, as the prepare stage reads it when it
    /// picks the cell an object shows (`SkinSource.getImage`). A sprite that does not animate reads
    /// nothing.
    pub(crate) fn prepare(&self, frame: &SkinFrame<'_>) {
        self.animation_index(self.cells(), frame.now_us, frame.timers, frame.script());
    }

    /// Which of `count` cells the animation is on, following `SkinSourceImage.getImageIndex`: a
    /// cycle of zero, a timer that is off, and a moment before the timer started all hold cell zero.
    ///
    /// `now_us` is the frame clock in microseconds. The cycle is in milliseconds, and the clock and
    /// the timer are each truncated to one before they are subtracted, as `TimerProperty.get` has it.
    /// `lua` is what a timer the skin computes with a function is asked through; without it such a
    /// timer is off.
    pub(crate) fn animation_index(&self, count: u32, now_us: i64, timers: &TimerState, lua: Option<&dyn LuaDrawEval>) -> u32 {
        if self.cycle <= 0 || count == 0 {
            return 0;
        }
        let mut time = now_us / MICROS_PER_MILLI;
        if let Some(timer) = self.timer {
            let started_us = timer.value_us(timers, lua);
            if started_us == TIMER_OFF {
                return 0;
            }
            time -= started_us / MICROS_PER_MILLI;
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
#[derive(Debug, Clone, Default)]
pub(crate) enum ValueSource {
    /// Nothing was named, so the object draws its own default.
    #[default]
    None,
    /// A property id.
    Id(i32),
    /// A Lua function the skin handed over, called on every frame the object is drawn.
    Function(LuaFnId),
    /// A property the skin named rather than numbered.
    Name(String),
}

impl ValueSource {
    /// Reads a skin field that is a property id, a function, a name, or absent.
    ///
    /// A script a document wrote as a string is a function by the time a skin is loaded. Source the
    /// loader never compiled is no reference at all, and reads as though the field had named
    /// nothing.
    pub(crate) fn new(property: Option<&PropertyRef>, fallback: i32) -> ValueSource {
        match property {
            Some(PropertyRef::Id(id)) => ValueSource::Id(*id),
            Some(PropertyRef::Func(function)) => ValueSource::Function(*function),
            Some(PropertyRef::Name(name)) => ValueSource::Name(name.clone()),
            Some(PropertyRef::Expr(_)) => ValueSource::None,
            None if fallback != 0 => ValueSource::Id(fallback),
            None => ValueSource::None,
        }
    }

    /// The integer this frame, or zero when nothing was named.
    ///
    /// A function or a name with no evaluator to ask reads as the reference reads a function that
    /// raised: zero here, and the zero or empty value of its type in the readers below.
    pub(crate) fn integer(&self, state: &dyn SkinHost, lua: Option<&dyn LuaDrawEval>) -> i32 {
        match self {
            ValueSource::None => 0,
            ValueSource::Id(id) => state.integer(*id),
            ValueSource::Function(function) => lua.map(|lua| lua.call_integer(*function)).unwrap_or_default(),
            ValueSource::Name(name) => lua.map(|lua| lua.named_integer(name)).unwrap_or_default(),
        }
    }

    /// The number this frame, exactly as its source gave it.
    ///
    /// Nothing is narrowed or tidied here. A `FLOAT_*` id carries a plain measurement -- a hi-speed
    /// multiplier, an average timing in milliseconds -- that a drawn `floatvalue` shows as it is,
    /// and the reference tells a number it cannot show from one it can by looking at the raw value
    /// ([`float_value`]). A slider and a graph read an id from the rate space instead and come here
    /// only for a function or a name ([`share`]).
    pub(crate) fn float(&self, state: &dyn SkinHost, lua: Option<&dyn LuaDrawEval>) -> f32 {
        match self {
            ValueSource::None => 0.0,
            ValueSource::Id(id) => state.float(*id),
            ValueSource::Function(function) => lua.map(|lua| lua.call_float(*function)).unwrap_or_default(),
            ValueSource::Name(name) => lua.map(|lua| lua.named_float(name)).unwrap_or_default(),
        }
    }

    /// The text this frame, borrowed from the state when it comes from a property so that a line
    /// which has not changed costs no allocation.
    pub(crate) fn text<'a>(&self, state: &'a dyn SkinHost, lua: Option<&dyn LuaDrawEval>) -> Cow<'a, str> {
        match self {
            ValueSource::None => Cow::Borrowed(""),
            ValueSource::Id(id) => state.text(*id),
            ValueSource::Function(function) => Cow::Owned(lua.map(|lua| lua.call_text(*function)).unwrap_or_default()),
            ValueSource::Name(name) => Cow::Owned(lua.map(|lua| lua.named_text(name)).unwrap_or_default()),
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
    /// One entry per selectable variant, each a sprite and the cell range it animates over. An
    /// image set keeps a `None` where it names an image the document could not supply, so the
    /// variants after it keep their index (`JsonSkinObjectLoader` leaves that source null).
    pub(crate) variants: Vec<Option<(Sprite, u32, u32)>>,
    /// What picks a variant.
    pub(crate) select: ImageSelect,
}

impl ImageBody {
    /// The variant this frame shows, or `None` when the image is not drawn: the selecting value is
    /// negative, or it names a variant the document could not supply ([`ImageSelect::slot`]).
    pub(crate) fn chosen(&self, frame: &SkinFrame<'_>) -> Option<&(Sprite, u32, u32)> {
        self.variants.get(self.select.slot(self.variants.len(), frame)?)?.as_ref()
    }
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

/// The `zeropadding` that fills empty places with the strip's alternate zero. A fractional number
/// reads anything above it as this and anything below zero as none (`FloatFormatter`).
const ALTERNATE_ZERO_PADDING: i32 = 2;

/// The `zeropadding` an eleven-cell integer strip is forced to, because its eleventh cell is the
/// alternate zero and there is nothing else it could be for (`JsonSkinObjectLoader`'s `d > 10`).
const FORCED_ALTERNATE_ZERO: i32 = ALTERNATE_ZERO_PADDING;

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

/// The timer an object animates its cells with, when the skin named one by id or handed over a
/// function that computes it.
fn cell_timer(property: Option<&PropertyRef>) -> Option<TimerRef> {
    property.and_then(PropertyRef::timer)
}

/// The sprite an image definition cuts out of its source.
pub(crate) fn image_sprite(def: &ImageDef, sources: Source<'_>) -> Option<Sprite> {
    let (_, tex, size) = source_of(sources, &def.src)?;
    Some(Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle))
}

/// Builds every object the document's top-level destinations name, in document order.
///
/// A destination whose id names nothing this build draws is dropped with a warning rather than
/// failing the skin: a document written for a screen with more object kinds still shows everything
/// this build does understand.
///
/// So is a destination with no keyframe, unless what it names places itself. The reference removes
/// such an object before the first frame (`SkinObject.validate`, checked by `Skin.prepare`), so its
/// conditions are never asked; keeping it would call whatever functions the skin gated it on, on
/// every frame, for an object that can never be drawn.
///
/// `kept` is given, for each object built, which of the document's destinations it was built from.
pub(crate) fn build_objects(
    skin: &LoadedSkin,
    sources: Source<'_>,
    families: &[(String, String)],
    assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
    kept: &mut Vec<usize>,
) -> Vec<SkinObject> {
    let mut objects = Vec::with_capacity(skin.destinations.len());
    for (destination, named) in skin.destinations.iter().enumerate() {
        let Some(body) = build_body(skin, &named.id, sources, families, assets, warnings) else {
            continue;
        };
        if named.track.frames.is_empty() && !body.places_itself() {
            warnings.push(format!("object {:?} has no destination keyframe, so it is never drawn", named.id));
            continue;
        }
        objects.push(SkinObject::new(&named.track, body));
        kept.push(destination);
    }
    objects
}

/// The body behind one destination id, or `None` when nothing declares it.
///
/// A reference image is looked for before anything the document declared, which is where the
/// reference's loader looks for it (`JSONSkinLoader`, ahead of its object loader).
pub(crate) fn build_body(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    families: &[(String, String)],
    assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = &skin.def;
    if let Some(body) = refs::build_reference(id) {
        return Some(body);
    }
    if let Some(image) = def.image.iter().find(|image| image.id == id) {
        return image_body(image, sources, warnings).map(Body::Image);
    }
    if let Some(set) = def.imageset.iter().find(|set| set.id == id) {
        let variants: Vec<Option<(Sprite, u32, u32)>> = set
            .images
            .iter()
            .map(|name| {
                def.image.iter().find(|image| &image.id == name).and_then(|image| image_sprite(image, sources)).map(|sprite| (sprite, 0, sprite.cells()))
            })
            .collect();
        if variants.iter().all(Option::is_none) {
            warnings.push(format!("image set {id:?} names no image this build could load"));
            return None;
        }
        return Some(Body::Image(ImageBody { variants, select: ImageSelect::of_set(set.value.as_ref(), set.reference) }));
    }
    if let Some(value) = def.value.iter().find(|value| value.id == id) {
        return number_body(value, sources, warnings).map(Body::Number);
    }
    if let Some(value) = def.floatvalue.iter().find(|value| value.id == id) {
        return float_body(value, sources, warnings).map(Body::Float);
    }
    if let Some(text) = def.text.iter().find(|text| text.id == id) {
        return Some(if text_input::is_editable(text) {
            Body::TextInput(text_input::text_input_body(text, families))
        } else {
            Body::Text(text::text_body(text, families))
        });
    }
    if let Some(slider) = def.slider.iter().find(|slider| slider.id == id) {
        return slider_body(slider, sources, warnings).map(Body::Slider);
    }
    if let Some(graph) = def.graph.iter().find(|graph| graph.id == id) {
        return graph_body(graph, sources, warnings).map(Body::Graph);
    }
    if def.bga.as_ref().is_some_and(|bga| bga.id == id) {
        return Some(Body::Bga(bga::BgaBody));
    }
    if let Some(body) = notes::build_note(skin, id, sources, families, assets, warnings) {
        return Some(body);
    }
    if let Some(body) = gauge::build_gauge(skin, id, sources, families, assets, warnings) {
        return Some(body);
    }
    if let Some(body) = judge::build_judge(skin, id, sources, families, assets, warnings) {
        return Some(body);
    }
    if let Some(body) = songlist::build_songlist(skin, id, sources, families, assets, warnings) {
        return Some(body);
    }
    if let Some(body) = covers::build_cover(skin, id, sources, families, assets, warnings) {
        return Some(body);
    }
    if let Some(body) = graphs::build_graph(skin, id, sources, families, assets, warnings) {
        return Some(body);
    }
    warnings.push(format!("object {id:?} is not a kind this build draws"));
    None
}

/// One image, animated over its cells and optionally split into variants a property picks between.
fn image_body(def: &ImageDef, sources: Source<'_>, warnings: &mut Vec<String>) -> Option<ImageBody> {
    let Some(sprite) = image_sprite(def, sources) else {
        warnings.push(format!("image {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let cells = sprite.cells();
    let groups = if def.len > 1 { (def.len as u32).min(cells.max(1)) } else { 1 };
    let per_group = (cells / groups).max(1);
    let variants = (0..groups).map(|group| Some((sprite, group * per_group, per_group))).collect();
    let select = if groups > 1 { ImageSelect::of_index(def.reference) } else { ImageSelect::First };
    Some(ImageBody { variants, select })
}

/// A whole number and the strip it is drawn from.
fn number_body(def: &ValueDef, sources: Source<'_>, warnings: &mut Vec<String>) -> Option<NumberBody> {
    let Some((_, tex, size)) = source_of(sources, &def.src) else {
        warnings.push(format!("value {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let sprite = Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle);
    let layout = DigitLayout::integer(sprite.cells());
    Some(NumberBody {
        sprite,
        layout,
        digits: def.digit.clamp(0, MAX_PLACES as i32) as u32,
        zero_padding: integer_padding(&layout, def),
        space: def.space as f32,
        align: def.align,
        value: ValueSource::new(def.value.as_ref(), def.reference),
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
fn float_body(def: &FloatValueDef, sources: Source<'_>, warnings: &mut Vec<String>) -> Option<FloatBody> {
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
        zero_padding: def.zeropadding.clamp(0, ALTERNATE_ZERO_PADDING),
        space: def.space as f32,
        align: def.align,
        gain: def.gain,
        value: ValueSource::new(def.value.as_ref(), def.reference),
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

/// A slider and the track it moves along.
fn slider_body(def: &SliderDef, sources: Source<'_>, warnings: &mut Vec<String>) -> Option<SliderBody> {
    let Some((_, tex, size)) = source_of(sources, &def.src) else {
        warnings.push(format!("slider {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let sprite = Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle);
    let value = ValueSource::new(def.value.as_ref(), def.slider_type);
    Some(SliderBody {
        sprite,
        direction: def.angle,
        range: def.range as f32,
        value,
        ref_num: (def.value.is_none() && def.is_ref_num).then_some((def.min, def.max)),
    })
}

/// A bar graph and the direction it grows in.
fn graph_body(def: &GraphDef, sources: Source<'_>, warnings: &mut Vec<String>) -> Option<GraphBody> {
    let Some((_, tex, size)) = source_of(sources, &def.src) else {
        warnings.push(format!("graph {:?} has no usable source {:?}", def.id, def.src));
        return None;
    };
    let sprite = Sprite::new(*tex, *size, (def.x, def.y, def.w, def.h), (def.divx, def.divy), cell_timer(def.timer.as_ref()), def.cycle);
    Some(GraphBody {
        sprite,
        direction: def.angle,
        value: ValueSource::new(def.value.as_ref(), def.graph_type),
        ref_num: (def.value.is_none() && def.is_ref_num).then_some((def.min, def.max)),
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
