//! Turning the table a Lua skin returns into the document model.
//!
//! A JSON skin reaches [`SkinDef`] through serde. A Lua skin cannot use that reader as it stands:
//! the reference copies a Lua table into the same record field by field with its own coercions
//! (`LuaSkinLoader.fromLuaValue`), and those are looser than any deserialiser -- a fractional number
//! in an integer field, the number `1` where a string id belongs, a function where a property id
//! belongs. This module is that copy, and it is made in two parts.
//!
//! **Plain fields** are read by the model's own serde definitions, driven by a deserialiser over Lua
//! values (`Reader`). The list of fields, their names and their defaults therefore stay the model's
//! and are written nowhere else; a field added to a record is read from a Lua skin the day it is
//! added. The reader applies the reference's rules:
//!
//! - **Records.** Start from the record's default. For a table, assign every string key that equals
//!   a field name exactly; ignore every other key. Anything that is not a table leaves the default
//!   record, so a top-level value that is not a table yields a document with no `type`. A field that
//!   holds one optional record is set whenever its key holds anything at all.
//! - **Lists.** For a table, convert every value whatever its key, so holes close up and a stray
//!   string key contributes an element: first the values at positive integer keys in ascending
//!   order, then the rest in the order the interpreter walks them. Anything that is not a table is
//!   an empty list. The reference's order is its own hash table's wherever a key is not part of a
//!   plain sequence, which no other interpreter reproduces; for a sequence, holes or not, the two
//!   agree.
//! - **Scalars.** Integers of every width use [`coerce::to_int`], floats [`coerce::to_float`],
//!   booleans [`coerce::to_boolean`] (the number zero is true) and strings [`coerce::to_jstring`]
//!   (the number `1` becomes `"1"`). An optional scalar is set whenever its key holds anything, which
//!   keeps "absent" distinct from every number an animation field can hold.
//!
//! **Reference fields** -- every place the model holds a [`PropertyRef`], an [`EventRef`], a writer or
//! a [`DestinationOption`] -- cannot come out of the model's reader, which knows the two forms a JSON
//! document spells and has no way to produce a function. They are read by hand in a second walk over
//! the same tables (`References`), and the reader only counts the ones it passes. The two counts are
//! compared at the end, so a reference field the model gains and the second walk does not know is an
//! error the first time a skin uses it, not a value silently dropped. The second walk also reads
//! font fallbacks, the one record the reference reads by a rule of its own. Its rules:
//!
//! - **References.** In this order: a function is registered with [`SkinLua::register`] under the
//!   [`LuaFnKind`] of the field and stored as a function reference; a number, or a string that
//!   spells one, is an id where the field's type has ids; any other string, and any number where
//!   there are no ids, is looked up as a name and, failing that, compiled with [`SkinLua::compile`];
//!   anything else leaves the field unset. A timer field has no names and takes no negative id; a
//!   text writer has no ids. A source that does not compile is pushed onto [`FromLua::warnings`] and
//!   leaves the field unset.
//! - **Conditions.** An `op` element that is a number, or a string that spells one, is an option id,
//!   sign included; a function or any other string is a boolean property; whatever is left, a Lua
//!   boolean included, is no condition at all.
//! - **Font fallbacks.** A string or a number is a path of type zero; a table carries `path`, kept
//!   only when it is a string or a number, and `type`.
//! - `if`, `value`, `values` and `include` mean nothing here: a Lua skin branches in Lua.
//!
//! Every table is read raw, so no metamethod runs and nothing a skin wrote can make the conversion
//! fail. Compiling a timer script does call it once, which is why the loader converts inside
//! [`SkinLua::with_host`].
//!
//! Compiled only with the `lua` feature.

use std::cell::Cell;
use std::path::Path;
use std::vec::IntoIter;

use mlua::Value;
use serde::Deserialize;
use serde::de::value::{Error as ReadError, StrDeserializer};
use serde::de::{DeserializeSeed, Deserializer, Error as _, MapAccess, SeqAccess, Visitor};

use crate::SkinError;
use crate::dst::LuaFnId;
use crate::lua::{LuaFnKind, SkinLua, coerce, error_message};
use crate::model::{
    CustomEvent, Destination, DestinationOption, EventRef, FloatValueDef, FloatWriterRef, FontDef, FontFallback, GraphDef, ImageDef, ImageSet, JudgeDef,
    NoteSet, PropertyRef, SkinDef, SliderDef, SongList, StringWriterRef, TextDef, ValueDef,
};

/// The id the reader leaves in a reference field until the second walk reads the field properly.
const STAND_IN_ID: i64 = 0;

/// The first key of a sequence.
const FIRST_SEQUENCE_KEY: i64 = 1;

/// What a boolean property's name is prefixed with to negate it, any number of times.
const NEGATION_PREFIX: char = '!';

/// The type of a font fallback that names none.
const DEFAULT_FALLBACK_TYPE: i32 = 0;

/// What the reader reports when the model asks for a shape no Lua value is read as.
const UNREADABLE_SHAPE: &str = "the skin model holds a shape the Lua converter does not read";

/// What the reader reports when a record's value is asked for before its key.
const VALUE_BEFORE_KEY: &str = "a record value was read before its key";

/// What the conversion reports when the reader passed a reference field the second walk never read.
const UNREAD_REFERENCE: &str = "the skin model holds a reference field the Lua converter does not read";

/// What a conversion works with besides the value itself.
#[derive(Debug)]
pub struct FromLua<'a> {
    /// The interpreter the value came from, which is where its functions are registered.
    pub lua: &'a SkinLua,
    /// The entry file, for messages.
    pub path: &'a Path,
    /// Where a recoverable problem is recorded.
    pub warnings: &'a mut Vec<String>,
    /// Whether the reference knows a property of this kind by this name, which decides whether a
    /// string in a reference field is kept as a name or compiled as a script.
    ///
    /// The kind selects the table: [`LuaFnKind::Boolean`] the boolean names, [`LuaFnKind::Integer`]
    /// the number names, [`LuaFnKind::Float`] the rate names, [`LuaFnKind::Text`] the string names,
    /// [`LuaFnKind::Event`] the event names, [`LuaFnKind::FloatWriter`] the rate writers and
    /// [`LuaFnKind::TextWriter`] the string writers. It is never asked about a timer, which has no
    /// names, and a boolean name arrives with its negation prefixes already taken off.
    pub known_name: fn(LuaFnKind, &str) -> bool,
}

/// Converts the value one pass of a skin's entry file returned.
///
/// The loader calls this once per pass. The header pass's result supplies the type, the name and
/// the customisation rows; the body pass's result supplies everything that is drawn.
///
/// Nothing a skin put in the value fails this. The error is for the interpreter running out of
/// memory while a table is walked, and for a model this converter has fallen behind.
pub fn skin_def_from_lua(value: &Value, context: &mut FromLua<'_>) -> Result<SkinDef, SkinError> {
    let path = context.path.to_string_lossy().into_owned();
    let failed = |message: String| SkinError::LuaLoad { path: path.clone(), message };

    let stand_ins = Cell::new(0);
    let mut skin = SkinDef::deserialize(Reader { value, stand_ins: &stand_ins }).map_err(|error| failed(error.to_string()))?;

    let mut references = References { context, read: 0 };
    references.skin(&mut skin, value).map_err(|error| failed(error_message(&error)))?;
    if references.read != stand_ins.get() {
        return Err(failed(UNREAD_REFERENCE.to_owned()));
    }
    Ok(skin)
}

/// The value `owner` holds at the string key `key`, or `nil` when it is not a table.
fn field(owner: &Value, key: &str) -> mlua::Result<Value> {
    match owner {
        Value::Table(table) => table.raw_get(key),
        _ => Ok(Value::Nil),
    }
}

/// The elements of a list field: every value of a table, sequence keys first and in order.
fn elements(value: &Value) -> mlua::Result<Vec<Value>> {
    let Value::Table(table) = value else {
        return Ok(Vec::new());
    };
    let mut sequence: Vec<(i64, Value)> = Vec::new();
    let mut rest: Vec<Value> = Vec::new();
    table.for_each(|key: Value, element: Value| {
        match key {
            Value::Integer(index) if index >= FIRST_SEQUENCE_KEY => sequence.push((index, element)),
            _ => rest.push(element),
        }
        Ok(())
    })?;
    sequence.sort_by_key(|(index, _)| *index);
    Ok(sequence.into_iter().map(|(_, element)| element).chain(rest).collect())
}

/// The fields of a record a table spells: each string key that is one of `fields`, with its value.
///
/// Only a string key can name a field. The reference compares the text of every key, but the text of
/// a number or a boolean is never an identifier.
fn named_fields(value: &Value, fields: &'static [&'static str]) -> mlua::Result<Vec<(&'static str, Value)>> {
    let Value::Table(table) = value else {
        return Ok(Vec::new());
    };
    let mut named = Vec::new();
    table.for_each(|key: Value, held: Value| {
        if let Value::String(key) = &key
            && let Some(name) = fields.iter().find(|name| *key.as_bytes() == *name.as_bytes())
        {
            named.push((*name, held));
        }
        Ok(())
    })?;
    Ok(named)
}

/// Reports a table that could not be walked as a reader error.
fn unwalkable(error: mlua::Error) -> ReadError {
    ReadError::custom(error_message(&error))
}

/// Reports a shape the model asked for that no Lua value is read as.
fn unreadable<T>(shape: &str) -> Result<T, ReadError> {
    Err(ReadError::custom(format!("{UNREADABLE_SHAPE}: {shape}")))
}

/// One Lua value, read as whatever the model's field asks for.
///
/// A field never fails to read: a value of the wrong type is coerced as the reference coerces it.
/// The one question the reader does not answer is the open one a reference field asks
/// (`deserialize_any`); there it counts the field and hands back a stand-in id for the second walk
/// to replace.
struct Reader<'v> {
    value: &'v Value,
    stand_ins: &'v Cell<usize>,
}

/// Writes the reader methods that coerce a scalar and hand it to the visitor.
macro_rules! read_scalars {
    ($($method:ident => $visit:ident($coerce:path)),* $(,)?) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, ReadError> {
                visitor.$visit($coerce(self.value))
            }
        )*
    };
}

impl<'de> Deserializer<'de> for Reader<'_> {
    type Error = ReadError;

    read_scalars! {
        deserialize_bool => visit_bool(coerce::to_boolean),
        deserialize_i8 => visit_i32(coerce::to_int),
        deserialize_i16 => visit_i32(coerce::to_int),
        deserialize_i32 => visit_i32(coerce::to_int),
        deserialize_i64 => visit_i32(coerce::to_int),
        deserialize_u8 => visit_i32(coerce::to_int),
        deserialize_u16 => visit_i32(coerce::to_int),
        deserialize_u32 => visit_i32(coerce::to_int),
        deserialize_u64 => visit_i32(coerce::to_int),
        deserialize_f32 => visit_f32(coerce::to_float),
        deserialize_f64 => visit_f32(coerce::to_float),
        deserialize_char => visit_string(coerce::to_jstring),
        deserialize_str => visit_string(coerce::to_jstring),
        deserialize_string => visit_string(coerce::to_jstring),
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, ReadError> {
        self.stand_ins.set(self.stand_ins.get() + 1);
        visitor.visit_i64(STAND_IN_ID)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, ReadError> {
        visitor.visit_some(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, ReadError> {
        visitor.visit_seq(Elements { rest: elements(self.value).map_err(unwalkable)?.into_iter(), stand_ins: self.stand_ins })
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, ReadError> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value, ReadError> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_struct<V: Visitor<'de>>(self, _name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, ReadError> {
        visitor.visit_map(Fields { rest: named_fields(self.value, fields).map_err(unwalkable)?.into_iter(), value: None, stand_ins: self.stand_ins })
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, ReadError> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, ReadError> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, ReadError> {
        visitor.visit_unit()
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, ReadError> {
        visitor.visit_unit()
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, ReadError> {
        unreadable("bytes")
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, ReadError> {
        unreadable("bytes")
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, ReadError> {
        unreadable("a map")
    }

    fn deserialize_enum<V: Visitor<'de>>(self, _name: &'static str, _variants: &'static [&'static str], _visitor: V) -> Result<V::Value, ReadError> {
        unreadable("an enum")
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, ReadError> {
        unreadable("an identifier")
    }
}

/// The elements of a list field, handed to the model one at a time.
struct Elements<'v> {
    rest: IntoIter<Value>,
    stand_ins: &'v Cell<usize>,
}

impl<'de> SeqAccess<'de> for Elements<'_> {
    type Error = ReadError;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, ReadError> {
        let Some(element) = self.rest.next() else {
            return Ok(None);
        };
        seed.deserialize(Reader { value: &element, stand_ins: self.stand_ins }).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.rest.len())
    }
}

/// The fields a table spells for one record, handed to the model one at a time.
struct Fields<'v> {
    rest: IntoIter<(&'static str, Value)>,
    value: Option<Value>,
    stand_ins: &'v Cell<usize>,
}

impl<'de> MapAccess<'de> for Fields<'_> {
    type Error = ReadError;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, ReadError> {
        let Some((name, value)) = self.rest.next() else {
            return Ok(None);
        };
        self.value = Some(value);
        seed.deserialize(StrDeserializer::new(name)).map(Some)
    }

    fn next_value_seed<S: DeserializeSeed<'de>>(&mut self, seed: S) -> Result<S::Value, ReadError> {
        let value = self.value.take().ok_or_else(|| ReadError::custom(VALUE_BEFORE_KEY))?;
        seed.deserialize(Reader { value: &value, stand_ins: self.stand_ins })
    }
}

/// What a reference field settles on, before it is fitted to the field's own type.
enum Reference {
    Id(i32),
    Function(LuaFnId),
    Name(String),
}

impl Reference {
    fn property(self) -> PropertyRef {
        match self {
            Self::Id(id) => PropertyRef::Id(id),
            Self::Function(function) => PropertyRef::Func(function),
            Self::Name(name) => PropertyRef::Name(name),
        }
    }

    fn event(self) -> EventRef {
        match self {
            Self::Id(id) => EventRef::Id(id),
            Self::Function(function) => EventRef::Lua(function),
            Self::Name(name) => EventRef::Name(name),
        }
    }

    fn float_writer(self) -> FloatWriterRef {
        match self {
            Self::Id(id) => FloatWriterRef::Id(id),
            Self::Function(function) => FloatWriterRef::Lua(function),
            Self::Name(name) => FloatWriterRef::Name(name),
        }
    }

    /// A text writer has no id form, so an id names nothing.
    fn text_writer(self) -> Option<StringWriterRef> {
        match self {
            Self::Id(_) => None,
            Self::Function(function) => Some(StringWriterRef::Lua(function)),
            Self::Name(name) => Some(StringWriterRef::Name(name)),
        }
    }
}

/// The second walk: every reference field of the model, read from the table its record came from.
///
/// Lists are walked with [`elements`], the same order the reader built them in, so the n-th record
/// meets the n-th table. `read` counts the fields and conditions that held a value, which is what
/// the reader counted from its side.
struct References<'c, 'a> {
    context: &'c mut FromLua<'a>,
    read: usize,
}

impl References<'_, '_> {
    fn skin(&mut self, skin: &mut SkinDef, root: &Value) -> mlua::Result<()> {
        self.each(&mut skin.font, root, "font", Self::font)?;
        self.each(&mut skin.image, root, "image", Self::image)?;
        self.each(&mut skin.imageset, root, "imageset", Self::image_set)?;
        self.each(&mut skin.value, root, "value", Self::number)?;
        self.each(&mut skin.floatvalue, root, "floatvalue", Self::float_number)?;
        self.each(&mut skin.text, root, "text", Self::text)?;
        self.each(&mut skin.slider, root, "slider", Self::slider)?;
        self.each(&mut skin.graph, root, "graph", Self::graph)?;
        self.each(&mut skin.hidden_cover, root, "hiddenCover", |this, cover, source| {
            cover.timer = this.timer(source)?;
            Ok(())
        })?;
        self.each(&mut skin.lift_cover, root, "liftCover", |this, cover, source| {
            cover.timer = this.timer(source)?;
            Ok(())
        })?;
        self.each(&mut skin.judge, root, "judge", Self::judge)?;
        self.each(&mut skin.custom_events, root, "customEvents", Self::custom_event)?;
        self.each(&mut skin.custom_timers, root, "customTimers", |this, timer, source| {
            timer.timer = this.timer(source)?;
            Ok(())
        })?;
        self.each(&mut skin.destination, root, "destination", Self::destination)?;
        if let Some(note) = &mut skin.note {
            self.note(note, &field(root, "note")?)?;
        }
        if let Some(list) = &mut skin.songlist {
            self.song_list(list, &field(root, "songlist")?)?;
        }
        Ok(())
    }

    /// Runs `read` on each record of `list` with the element of `owner[key]` it was made from.
    fn each<T>(&mut self, list: &mut [T], owner: &Value, key: &str, read: impl Fn(&mut Self, &mut T, &Value) -> mlua::Result<()>) -> mlua::Result<()> {
        for (record, source) in list.iter_mut().zip(elements(&field(owner, key)?)?) {
            read(self, record, &source)?;
        }
        Ok(())
    }

    fn font(&mut self, font: &mut FontDef, source: &Value) -> mlua::Result<()> {
        font.fallback = elements(&field(source, "fallback")?)?.iter().map(font_fallback).collect::<mlua::Result<_>>()?;
        Ok(())
    }

    fn image(&mut self, image: &mut ImageDef, source: &Value) -> mlua::Result<()> {
        image.timer = self.timer(source)?;
        image.act = self.act(source)?;
        Ok(())
    }

    fn image_set(&mut self, set: &mut ImageSet, source: &Value) -> mlua::Result<()> {
        set.value = self.value(source, LuaFnKind::Integer)?;
        set.act = self.act(source)?;
        Ok(())
    }

    fn number(&mut self, number: &mut ValueDef, source: &Value) -> mlua::Result<()> {
        number.timer = self.timer(source)?;
        number.value = self.value(source, LuaFnKind::Integer)?;
        self.each(&mut number.offset, source, "offset", Self::number)
    }

    fn float_number(&mut self, number: &mut FloatValueDef, source: &Value) -> mlua::Result<()> {
        number.timer = self.timer(source)?;
        number.value = self.value(source, LuaFnKind::Float)?;
        self.each(&mut number.offset, source, "offset", Self::number)
    }

    fn text(&mut self, text: &mut TextDef, source: &Value) -> mlua::Result<()> {
        text.value = self.value(source, LuaFnKind::Text)?;
        text.event = self.reference(source, "event", LuaFnKind::TextWriter)?.and_then(Reference::text_writer);
        Ok(())
    }

    fn slider(&mut self, slider: &mut SliderDef, source: &Value) -> mlua::Result<()> {
        slider.timer = self.timer(source)?;
        slider.value = self.value(source, LuaFnKind::Float)?;
        slider.event = self.reference(source, "event", LuaFnKind::FloatWriter)?.map(Reference::float_writer);
        Ok(())
    }

    fn graph(&mut self, graph: &mut GraphDef, source: &Value) -> mlua::Result<()> {
        graph.timer = self.timer(source)?;
        graph.value = self.value(source, LuaFnKind::Float)?;
        Ok(())
    }

    fn judge(&mut self, judge: &mut JudgeDef, source: &Value) -> mlua::Result<()> {
        self.each(&mut judge.images, source, "images", Self::destination)?;
        self.each(&mut judge.numbers, source, "numbers", Self::destination)
    }

    fn custom_event(&mut self, event: &mut CustomEvent, source: &Value) -> mlua::Result<()> {
        event.action = self.reference(source, "action", LuaFnKind::Event)?.map(Reference::event);
        event.condition = self.reference(source, "condition", LuaFnKind::Boolean)?.map(Reference::property);
        Ok(())
    }

    fn note(&mut self, note: &mut NoteSet, source: &Value) -> mlua::Result<()> {
        self.each(&mut note.group, source, "group", Self::destination)?;
        self.each(&mut note.bpm, source, "bpm", Self::destination)?;
        self.each(&mut note.stop, source, "stop", Self::destination)?;
        self.each(&mut note.time, source, "time", Self::destination)
    }

    fn song_list(&mut self, list: &mut SongList, source: &Value) -> mlua::Result<()> {
        self.each(&mut list.listoff, source, "listoff", Self::destination)?;
        self.each(&mut list.liston, source, "liston", Self::destination)?;
        self.each(&mut list.text, source, "text", Self::destination)?;
        self.each(&mut list.level, source, "level", Self::destination)?;
        self.each(&mut list.lamp, source, "lamp", Self::destination)?;
        self.each(&mut list.playerlamp, source, "playerlamp", Self::destination)?;
        self.each(&mut list.rivallamp, source, "rivallamp", Self::destination)?;
        self.each(&mut list.trophy, source, "trophy", Self::destination)?;
        self.each(&mut list.label, source, "label", Self::destination)?;
        if let Some(graph) = &mut list.graph {
            self.destination(graph, &field(source, "graph")?)?;
        }
        Ok(())
    }

    fn destination(&mut self, destination: &mut Destination, source: &Value) -> mlua::Result<()> {
        destination.timer = self.timer(source)?;
        destination.draw = self.reference(source, "draw", LuaFnKind::Boolean)?.map(Reference::property);
        destination.op = elements(&field(source, "op")?)?.iter().map(|element| self.condition(element)).collect();
        Ok(())
    }

    fn timer(&mut self, source: &Value) -> mlua::Result<Option<PropertyRef>> {
        Ok(self.reference(source, "timer", LuaFnKind::Timer)?.map(Reference::property))
    }

    fn value(&mut self, source: &Value, kind: LuaFnKind) -> mlua::Result<Option<PropertyRef>> {
        Ok(self.reference(source, "value", kind)?.map(Reference::property))
    }

    fn act(&mut self, source: &Value) -> mlua::Result<Option<EventRef>> {
        Ok(self.reference(source, "act", LuaFnKind::Event)?.map(Reference::event))
    }

    /// The reference `source[key]` holds, read as a field of `kind`.
    fn reference(&mut self, source: &Value, key: &str, kind: LuaFnKind) -> mlua::Result<Option<Reference>> {
        let value = field(source, key)?;
        if value.is_nil() {
            return Ok(None);
        }
        self.read += 1;
        Ok(self.resolve(&value, kind))
    }

    /// One element of an `op` list (`LuaSkinLoader`'s `DestinationOption` reader).
    fn condition(&mut self, element: &Value) -> DestinationOption {
        self.read += 1;
        if coerce::to_number(element).is_some() {
            return DestinationOption { id: coerce::to_int(element), property: None };
        }
        match self.resolve(element, LuaFnKind::Boolean) {
            Some(reference) => DestinationOption { id: 0, property: Some(reference.property()) },
            None => DestinationOption::UNCONDITIONAL,
        }
    }

    /// Settles one value on a form, in the reference's order (`LuaSkinLoader.serializeLuaScript`).
    ///
    /// The reference asks `isnumber` before `isstring`, and both are true of a number and of a
    /// string that spells one. So `"41"` is the id 41 wherever there are ids, and where there are
    /// none the number `41` is the text `"41"`, to be looked up as a name and then compiled.
    fn resolve(&mut self, value: &Value, kind: LuaFnKind) -> Option<Reference> {
        if let Value::Function(function) = value {
            return Some(Reference::Function(self.context.lua.register(function.clone(), kind)));
        }
        if kind != LuaFnKind::TextWriter && coerce::to_number(value).is_some() {
            let id = coerce::to_int(value);
            return (kind != LuaFnKind::Timer || id >= 0).then_some(Reference::Id(id));
        }
        if !matches!(value, Value::String(_) | Value::Integer(_) | Value::Number(_)) {
            return None;
        }

        let text = coerce::to_jstring(value);
        if self.is_name(kind, &text) {
            return Some(Reference::Name(text));
        }
        match self.context.lua.compile(&text, kind) {
            Ok(function) => Some(Reference::Function(function)),
            Err(error) => {
                self.context.warnings.push(format!("{}: {error}", self.context.path.display()));
                None
            }
        }
    }

    /// Whether `text` is a name the reference knows for a field of `kind`.
    fn is_name(&self, kind: LuaFnKind, text: &str) -> bool {
        match kind {
            LuaFnKind::Timer => false,
            LuaFnKind::Boolean => (self.context.known_name)(kind, text.trim_start_matches(NEGATION_PREFIX)),
            LuaFnKind::Integer | LuaFnKind::Float | LuaFnKind::Text | LuaFnKind::Event | LuaFnKind::FloatWriter | LuaFnKind::TextWriter => {
                (self.context.known_name)(kind, text)
            }
        }
    }
}

/// One element of a font's `fallback` list (`LuaSkinLoader`'s `FontFallback` reader).
fn font_fallback(element: &Value) -> mlua::Result<FontFallback> {
    let is_text = |value: &Value| matches!(value, Value::String(_) | Value::Integer(_) | Value::Number(_));
    if is_text(element) {
        return Ok(FontFallback { path: coerce::to_jstring(element), font_type: DEFAULT_FALLBACK_TYPE });
    }
    let path = field(element, "path")?;
    Ok(FontFallback { path: if is_text(&path) { coerce::to_jstring(&path) } else { String::new() }, font_type: coerce::to_int(&field(element, "type")?) })
}
