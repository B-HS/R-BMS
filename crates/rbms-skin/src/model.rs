//! The serde mirror of a skin document: every object list it declares, the destination records that
//! animate them, and the sentinel-inheritance rule the loader resolves before anything else sees a
//! keyframe.
//!
//! This is a shape, not a policy. Field names, defaults and nesting follow the reference
//! implementation's `JsonSkin.java` so an existing third-party document parses unchanged; unknown
//! keys are ignored the way its reader ignores them. Every interpretation of these values --
//! inheritance, file resolution, Lua compilation -- lives in [`crate::loader`].
//!
//! A JSON document reaches this shape through serde, and there a reference to game state is a number
//! or a string. A Lua skin reaches it through the loader's table converter, which may also put a
//! function value where a JSON document puts an id; [`PropertyRef`], [`EventRef`],
//! [`FloatWriterRef`] and [`StringWriterRef`] hold every form either kind of skin can write.

mod graphs;
mod objects;

pub use graphs::{BpmGraph, GaugeGraph, HitErrorVisualizer, JudgeGraph, TimingDistributionGraph, TimingVisualizer};
pub use objects::{
    BgaDef, FloatValueDef, GaugeDef, GraphDef, HiddenCover, ImageDef, ImageSet, JudgeDef, LiftCover, NoteSet, PmChara, Practice, SkinConfigurationProperty,
    SkinPreview, SliderDef, SongList, TextDef, ValueDef,
};

use std::fmt;

use serde::Deserialize;
use serde::de::{self, Visitor};

use crate::dst::{LuaFnId, TimerRef};
use crate::timer::TimerId;

/// The width a document is authored against when it names none.
pub const DEFAULT_SKIN_WIDTH: i32 = 1280;

/// The height a document is authored against when it names none.
pub const DEFAULT_SKIN_HEIGHT: i32 = 720;

/// The `type` value of a document that declares no screen. The loader refuses such a document
/// rather than guessing from its file name.
pub const SKIN_TYPE_UNSET: i32 = -1;

/// A single undivided cell: one column, one row.
const SINGLE_DIVISION: i32 = 1;

/// The `stretch` value of a destination that names none.
pub const STRETCH_UNSET: i32 = -1;

/// The sentinel the reference writes into an unset animation field (`Integer.MIN_VALUE`). A
/// document never spells it out; the mirror reads a missing field as `None` instead.
pub const ANIMATION_UNSET: i32 = i32::MIN;

/// A reference to game state, in any of the forms a skin may write one.
///
/// The reference accepts every form in the same field and settles on one in a fixed order
/// (`LuaSkinLoader.serializeLuaScript`, and `JsonSkinSerializer`'s `LuaScriptSerializer` for the two
/// forms JSON can spell): a function value is called, a number is a property id, a string is looked
/// up as a property name, and only a string no name table knows is Lua source, compiled as
/// `return <source>`. So `"timer": 41` and `"timer": "main_state == 4"` are equally valid in a JSON
/// document, and `draw = function() ... end` is as valid as either in a Lua one.
///
/// Whatever else a Lua skin puts in such a field -- `nil`, a boolean, a table -- is no reference at
/// all. Every field of this type is an `Option`, and that case is its `None`: the field reads as
/// though the skin had left it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyRef {
    /// A property id, sign included. Negation of a negative id is the registry's rule.
    Id(i32),
    /// A Lua function: either a function value a Lua skin handed over, or source the loader
    /// compiled. The function stays in the interpreter the skin was loaded into and is called by
    /// this handle on every frame it is read.
    Func(LuaFnId),
    /// A property the skin named rather than numbered, spelled exactly as it wrote it. For a boolean
    /// a leading `!` negates. The loader only leaves a name here once a name table knows it; one it
    /// does not know is source, and becomes a [`Self::Func`] or an [`Self::Expr`] instead.
    Name(String),
    /// Lua source as a JSON document wrote it, which the loader compiles once at load time.
    Expr(String),
}

impl PropertyRef {
    /// The id, when the skin wrote one.
    pub fn id(&self) -> Option<i32> {
        match self {
            Self::Id(id) => Some(*id),
            Self::Func(_) | Self::Name(_) | Self::Expr(_) => None,
        }
    }

    /// The function handle, when the skin handed a function over.
    pub fn function(&self) -> Option<LuaFnId> {
        match self {
            Self::Func(function) => Some(*function),
            Self::Id(_) | Self::Name(_) | Self::Expr(_) => None,
        }
    }

    /// The property name, when the skin named one.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Name(name) => Some(name),
            Self::Id(_) | Self::Func(_) | Self::Expr(_) => None,
        }
    }

    /// The expression source, when the document wrote one.
    pub fn expr(&self) -> Option<&str> {
        match self {
            Self::Expr(source) => Some(source),
            Self::Id(_) | Self::Func(_) | Self::Name(_) => None,
        }
    }

    /// The timer this names, when it is one an animation can follow: a timer id or a function.
    ///
    /// A negative id names no timer at all (`TimerPropertyFactory.getTimerProperty`), so whatever
    /// carries one animates on the scene clock, exactly as though the field had been left out. Zero
    /// is an id like any other.
    ///
    /// The reference has no timer names (`LuaSkinLoader` passes no name lookup for
    /// `TimerProperty`), so text in a timer field is source. Source the loader has not compiled into
    /// a function names no timer here.
    pub fn timer(&self) -> Option<TimerRef> {
        match self {
            Self::Id(id) => (*id >= 0).then_some(TimerRef::Id(TimerId(*id))),
            Self::Func(function) => Some(TimerRef::Lua(*function)),
            Self::Name(_) | Self::Expr(_) => None,
        }
    }
}

/// Reads the two forms of [`PropertyRef`] a JSON document can spell: a number is an id and a string
/// is source. A function never arrives this way, and neither does a name, which only the loader can
/// tell from source.
///
/// Numbers arrive as `i64`, `u64` or `f64` depending on which parser is in play, so all three are
/// accepted and narrowed to the `int` field the reference declares.
struct PropertyRefVisitor;

impl Visitor<'_> for PropertyRefVisitor {
    type Value = PropertyRef;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a property id or a Lua expression")
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        i32::try_from(value).map(PropertyRef::Id).map_err(|_| E::custom(format!("property id {value} does not fit a 32-bit integer")))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        i32::try_from(value).map(PropertyRef::Id).map_err(|_| E::custom(format!("property id {value} does not fit a 32-bit integer")))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        let truncated = value.trunc();
        if truncated < f64::from(i32::MIN) || truncated > f64::from(i32::MAX) {
            return Err(E::custom(format!("property id {value} does not fit a 32-bit integer")));
        }
        Ok(PropertyRef::Id(truncated as i32))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(PropertyRef::Expr(value.to_owned()))
    }
}

impl<'de> Deserialize<'de> for PropertyRef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(PropertyRefVisitor)
    }
}

/// What runs when an image is clicked or a custom event fires (`Event`).
///
/// The forms and their order are [`PropertyRef`]'s. What differs is what is done with the result:
/// an event is run for its effect, and a function is always called with exactly one integer
/// argument, whatever it declares (`SkinLuaAccessor.loadEvent`, where `narg()` is always one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventRef {
    /// An event id, built in or declared by the skin's own `customEvents`.
    Id(i32),
    /// A Lua function, called by this handle each time the event runs.
    Lua(LuaFnId),
    /// An event the skin named rather than numbered.
    Name(String),
    /// Lua source as a JSON document wrote it. Unlike a property's, it is compiled as it stands,
    /// with no `return` put in front of it.
    Script(String),
}

impl EventRef {
    /// The same reference a JSON document's number or string spells for a property.
    fn from_document(value: PropertyRef) -> Self {
        match value {
            PropertyRef::Id(id) => Self::Id(id),
            PropertyRef::Func(function) => Self::Lua(function),
            PropertyRef::Name(name) => Self::Name(name),
            PropertyRef::Expr(source) => Self::Script(source),
        }
    }
}

impl<'de> Deserialize<'de> for EventRef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        PropertyRef::deserialize(deserializer).map(Self::from_document)
    }
}

/// Where a slider writes the value it is dragged to (`FloatWriter`).
///
/// A function is called with the new value as its one argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloatWriterRef {
    /// A rate id, written through the same id space a slider reads its value from.
    Id(i32),
    /// A Lua function, called by this handle with each value written.
    Lua(LuaFnId),
    /// A rate the skin named rather than numbered.
    Name(String),
    /// Lua source as a JSON document wrote it, compiled as it stands.
    Script(String),
}

impl FloatWriterRef {
    /// The same reference a JSON document's number or string spells for a property.
    fn from_document(value: PropertyRef) -> Self {
        match value {
            PropertyRef::Id(id) => Self::Id(id),
            PropertyRef::Func(function) => Self::Lua(function),
            PropertyRef::Name(name) => Self::Name(name),
            PropertyRef::Expr(source) => Self::Script(source),
        }
    }
}

impl<'de> Deserialize<'de> for FloatWriterRef {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        PropertyRef::deserialize(deserializer).map(Self::from_document)
    }
}

/// Where an editable text writes what was typed into it (`StringWriter`).
///
/// A function is called with the new text as its one argument. There is no id form: the reference
/// gives a string writer no id lookup at all, so a number in this field names nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringWriterRef {
    /// A Lua function, called by this handle with each text written.
    Lua(LuaFnId),
    /// A string property the skin named, written through its writer.
    Name(String),
    /// Lua source as a JSON document wrote it, compiled as it stands.
    Script(String),
}

impl StringWriterRef {
    /// Reads a text's `event` field, where a number names nothing and leaves the field unset
    /// (`JsonSkinSerializer` registers the `StringWriter` reader with no id lookup).
    pub(crate) fn field<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Self>, D::Error> {
        let value: Option<PropertyRef> = Option::deserialize(deserializer)?;
        Ok(value.and_then(|value| match value {
            PropertyRef::Id(_) => None,
            PropertyRef::Func(function) => Some(Self::Lua(function)),
            PropertyRef::Name(name) => Some(Self::Name(name)),
            PropertyRef::Expr(source) => Some(Self::Script(source)),
        }))
    }
}

/// A named group of customisation rows the configuration screen shows together.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Category {
    pub name: String,
    pub item: Vec<String>,
}

/// One customisation choice the document offers, with the option ids each item enables.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PropertyDef {
    pub category: String,
    pub name: String,
    pub item: Vec<PropertyItem>,
    pub def: Option<String>,
}

/// One item of a [`PropertyDef`], naming the option id it turns on.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PropertyItem {
    pub name: String,
    pub op: i32,
}

/// One file slot the player picks from, whose `path` is a wildcard pattern.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Filepath {
    pub category: String,
    pub name: String,
    pub path: String,
    pub def: Option<String>,
}

/// One nudge slot the player adjusts, and which of its axes the document allows.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct OffsetDef {
    pub category: String,
    pub name: String,
    pub id: i32,
    pub x: bool,
    pub y: bool,
    pub w: bool,
    pub h: bool,
    pub r: bool,
    pub a: bool,
}

/// An image file the document's objects draw from.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Source {
    pub id: String,
    pub path: String,
}

/// A font file, with the fallbacks used for glyphs it lacks.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FontDef {
    pub id: String,
    pub path: String,
    pub fallback: Vec<FontFallback>,
    #[serde(rename = "type")]
    pub font_type: i32,
}

/// One fallback font file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FontFallback {
    pub path: String,
    #[serde(rename = "type")]
    pub font_type: i32,
}

/// An action the document fires when its condition turns true.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CustomEvent {
    pub id: i32,
    pub action: Option<EventRef>,
    pub condition: Option<PropertyRef>,
    #[serde(rename = "minInterval")]
    pub min_interval: i32,
}

/// A timer the document defines itself, driven by an expression rather than by the player.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CustomTimer {
    pub id: i32,
    pub timer: Option<PropertyRef>,
}

/// A rectangle written in document coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct RectDef {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// One draw condition: an option id, or a boolean property in any form but an id.
///
/// A number in an `op` list is always an option id, sign included, and never a property; a function
/// or a string is a property (`LuaSkinLoader`'s `DestinationOption` reader).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DestinationOption {
    pub id: i32,
    pub property: Option<PropertyRef>,
}

impl DestinationOption {
    /// The entry that gates nothing: no id and no property.
    ///
    /// This is what a Lua skin's `op = { true }` or `op = { false }` becomes. A Lua boolean is
    /// neither a number nor something that resolves to a property, so the reference keeps an entry
    /// with id zero and no property, which neither of its two condition lists picks up
    /// (`JsonSkin.Destination.getOptionIds`, `getDrawConditions`). The object draws either way.
    pub const UNCONDITIONAL: Self = Self { id: 0, property: None };

    /// Whether this entry gates nothing.
    pub fn is_unconditional(&self) -> bool {
        *self == Self::UNCONDITIONAL
    }

    /// The shorthand a document actually writes: a bare id, or a bare property in another form.
    fn from_shorthand(value: PropertyRef) -> Self {
        match value {
            PropertyRef::Id(id) => Self { id, property: None },
            property => Self { id: 0, property: Some(property) },
        }
    }
}

/// Reads a draw condition in either the shorthand or the spelled-out object form.
struct DestinationOptionVisitor;

impl<'de> Visitor<'de> for DestinationOptionVisitor {
    type Value = DestinationOption;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an option id, a Lua expression, or an object holding one of them")
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        PropertyRefVisitor.visit_i64(value).map(DestinationOption::from_shorthand)
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        PropertyRefVisitor.visit_u64(value).map(DestinationOption::from_shorthand)
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        PropertyRefVisitor.visit_f64(value).map(DestinationOption::from_shorthand)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(DestinationOption::from_shorthand(PropertyRef::Expr(value.to_owned())))
    }

    fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        DestinationOptionFields::deserialize(de::value::MapAccessDeserializer::new(map))
            .map(|fields| DestinationOption { id: fields.id, property: fields.property })
    }
}

/// The spelled-out object form of a draw condition.
#[derive(Default, Deserialize)]
#[serde(default)]
struct DestinationOptionFields {
    id: i32,
    property: Option<PropertyRef>,
}

impl<'de> Deserialize<'de> for DestinationOption {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DestinationOptionVisitor)
    }
}

/// An animation attached to an object: which timer drives it, how it loops, and its keyframes.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Destination {
    pub id: String,
    pub blend: i32,
    pub filter: i32,
    pub timer: Option<PropertyRef>,
    #[serde(rename = "loop")]
    pub loop_ms: i32,
    pub center: i32,
    pub offset: i32,
    pub offsets: Vec<i32>,
    pub stretch: i32,
    pub op: Vec<DestinationOption>,
    pub draw: Option<PropertyRef>,
    pub dst: Vec<Animation>,
    #[serde(rename = "mouseRect")]
    pub mouse_rect: Option<RectDef>,
}

impl Default for Destination {
    fn default() -> Self {
        Self {
            id: String::new(),
            blend: 0,
            filter: 0,
            timer: None,
            loop_ms: 0,
            center: 0,
            offset: 0,
            offsets: Vec::new(),
            stretch: STRETCH_UNSET,
            op: Vec::new(),
            draw: None,
            dst: Vec::new(),
            mouse_rect: None,
        }
    }
}

/// One keyframe. Every field a document leaves out reads as `None` and is filled by the loader,
/// from the type default on the first keyframe and from the previous keyframe after that.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Animation {
    pub time: Option<i64>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub w: Option<i32>,
    pub h: Option<i32>,
    pub clip_x: Option<i32>,
    pub clip_y: Option<i32>,
    pub clip_w: Option<i32>,
    pub clip_h: Option<i32>,
    pub acc: Option<i32>,
    pub a: Option<i32>,
    pub r: Option<i32>,
    pub g: Option<i32>,
    pub b: Option<i32>,
    pub angle: Option<i32>,
}

/// A whole skin document.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SkinDef {
    #[serde(rename = "type")]
    pub skin_type: i32,
    pub name: String,
    pub author: String,
    pub w: i32,
    pub h: i32,
    pub fadeout: i32,
    pub input: i32,
    pub scene: i32,
    pub close: i32,
    pub loadend: i32,
    pub playstart: i32,
    pub judgetimer: i32,
    pub finishmargin: i32,
    pub category: Vec<Category>,
    pub property: Vec<PropertyDef>,
    pub filepath: Vec<Filepath>,
    pub offset: Vec<OffsetDef>,
    pub source: Vec<Source>,
    pub font: Vec<FontDef>,
    pub image: Vec<ImageDef>,
    pub imageset: Vec<ImageSet>,
    pub value: Vec<ValueDef>,
    pub floatvalue: Vec<FloatValueDef>,
    pub text: Vec<TextDef>,
    pub slider: Vec<SliderDef>,
    pub graph: Vec<GraphDef>,
    pub gaugegraph: Vec<GaugeGraph>,
    pub judgegraph: Vec<JudgeGraph>,
    pub bpmgraph: Vec<BpmGraph>,
    pub hiterrorvisualizer: Vec<HitErrorVisualizer>,
    pub timingvisualizer: Vec<TimingVisualizer>,
    pub timingdistributiongraph: Vec<TimingDistributionGraph>,
    pub note: Option<NoteSet>,
    pub gauge: Option<GaugeDef>,
    #[serde(rename = "hiddenCover")]
    pub hidden_cover: Vec<HiddenCover>,
    #[serde(rename = "liftCover")]
    pub lift_cover: Vec<LiftCover>,
    pub bga: Option<BgaDef>,
    pub skinpreview: Option<SkinPreview>,
    pub practice: Option<Practice>,
    pub judge: Vec<JudgeDef>,
    pub songlist: Option<SongList>,
    pub pmchara: Vec<PmChara>,
    #[serde(rename = "skinSelect")]
    pub skin_select: Option<SkinConfigurationProperty>,
    #[serde(rename = "customEvents")]
    pub custom_events: Vec<CustomEvent>,
    #[serde(rename = "customTimers")]
    pub custom_timers: Vec<CustomTimer>,
    pub destination: Vec<Destination>,
}

impl Default for SkinDef {
    fn default() -> Self {
        Self {
            skin_type: SKIN_TYPE_UNSET,
            name: String::new(),
            author: String::new(),
            w: DEFAULT_SKIN_WIDTH,
            h: DEFAULT_SKIN_HEIGHT,
            fadeout: 0,
            input: 0,
            scene: 0,
            close: 0,
            loadend: 0,
            playstart: 0,
            judgetimer: SINGLE_DIVISION,
            finishmargin: 0,
            category: Vec::new(),
            property: Vec::new(),
            filepath: Vec::new(),
            offset: Vec::new(),
            source: Vec::new(),
            font: Vec::new(),
            image: Vec::new(),
            imageset: Vec::new(),
            value: Vec::new(),
            floatvalue: Vec::new(),
            text: Vec::new(),
            slider: Vec::new(),
            graph: Vec::new(),
            gaugegraph: Vec::new(),
            judgegraph: Vec::new(),
            bpmgraph: Vec::new(),
            hiterrorvisualizer: Vec::new(),
            timingvisualizer: Vec::new(),
            timingdistributiongraph: Vec::new(),
            note: None,
            gauge: None,
            hidden_cover: Vec::new(),
            lift_cover: Vec::new(),
            bga: None,
            skinpreview: None,
            practice: None,
            judge: Vec::new(),
            songlist: None,
            pmchara: Vec::new(),
            skin_select: None,
            custom_events: Vec::new(),
            custom_timers: Vec::new(),
            destination: Vec::new(),
        }
    }
}
