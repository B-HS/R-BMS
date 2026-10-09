//! Settling the scripts a JSON document wrote as strings.
//!
//! Wherever a document may write a property id it may write a string instead, and the reference
//! decides what the string is while it reads the field (`JsonSkinSerializer`'s
//! `LuaScriptSerializer` and `DestinationOptionSerializer`): a name one of the property tables knows
//! is that property, and anything else is Lua source, compiled on the spot into the interpreter the
//! document was given. Serde cannot do either half -- it has no name table and no interpreter -- so
//! the mirror in [`crate::model`] keeps the string as it was written, and this module makes the
//! reference's decision for every such string once the document has been read.
//!
//! The rules, field by field:
//!
//! - A boolean, integer, float or text property is looked up by name first. A boolean name may be
//!   negated with a leading `!`. A float field names a rate. What is not a name is compiled as
//!   `return <source>`.
//! - A timer has no names. Its source is compiled as `return <source>` and called once; when that
//!   yields a function the function is the timer, and otherwise the chunk is
//!   (`SkinLuaAccessor.loadTimerProperty`).
//! - An event and the two writers have no names in a JSON document either, and their source is
//!   compiled as it stands, with no `return` in front.
//! - An `op` element is a boolean property.
//!
//! Source that does not compile costs the one field that carried it: the field is left unset, which
//! for an `op` element means the element gates nothing, and the reason is recorded as a warning.
//! That is the reference's outcome too, where the failed property is `null`.
//!
//! After this pass a loaded document holds no [`PropertyRef::Expr`] and no `Script` variant, so
//! nothing downstream of the loader ever compiles anything. A build without the `lua` feature has no
//! interpreter to compile into and refuses a document that carries a script with
//! [`SkinError::LuaUnavailable`], rather than drawing it as though the script were not there.

use crate::SkinError;
use crate::model::{
    CustomEvent, Destination, DestinationOption, EventRef, FloatValueDef, FloatWriterRef, GraphDef, ImageDef, ImageSet, JudgeDef, NoteSet, PropertyRef,
    SkinDef, SliderDef, SongList, StringWriterRef, TextDef, ValueDef,
};

/// One place in a document where a reference to game state is written, by what the reference reads
/// it as.
pub(crate) enum Slot<'a> {
    Boolean(&'a mut Option<PropertyRef>),
    Integer(&'a mut Option<PropertyRef>),
    Float(&'a mut Option<PropertyRef>),
    Text(&'a mut Option<PropertyRef>),
    Timer(&'a mut Option<PropertyRef>),
    Event(&'a mut Option<EventRef>),
    FloatWriter(&'a mut Option<FloatWriterRef>),
    TextWriter(&'a mut Option<StringWriterRef>),
    Condition(&'a mut DestinationOption),
}

impl Slot<'_> {
    /// The source this slot still holds, when a document wrote a string here and nothing has
    /// settled it yet.
    fn source(&self) -> Option<&str> {
        match self {
            Self::Boolean(property) | Self::Integer(property) | Self::Float(property) | Self::Text(property) | Self::Timer(property) => {
                property.as_ref().and_then(PropertyRef::expr)
            }
            Self::Event(Some(EventRef::Script(source))) => Some(source),
            Self::FloatWriter(Some(FloatWriterRef::Script(source))) => Some(source),
            Self::TextWriter(Some(StringWriterRef::Script(source))) => Some(source),
            Self::Condition(option) => option.property.as_ref().and_then(PropertyRef::expr),
            Self::Event(_) | Self::FloatWriter(_) | Self::TextWriter(_) => None,
        }
    }
}

/// What is done with each slot of a walk.
type Visit<'v> = &'v mut dyn FnMut(Slot<'_>) -> Result<(), SkinError>;

/// Visits every slot of one destination: its timer, each `op` element in order, then `draw`.
pub(crate) fn each_destination_slot(destination: &mut Destination, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Timer(&mut destination.timer))?;
    for option in &mut destination.op {
        visit(Slot::Condition(option))?;
    }
    visit(Slot::Boolean(&mut destination.draw))
}

/// Visits every slot of a list of destinations.
fn each_destination(list: &mut [Destination], visit: Visit<'_>) -> Result<(), SkinError> {
    list.iter_mut().try_for_each(|destination| each_destination_slot(destination, visit))
}

fn image(image: &mut ImageDef, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Timer(&mut image.timer))?;
    visit(Slot::Event(&mut image.act))
}

fn image_set(set: &mut ImageSet, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Integer(&mut set.value))?;
    visit(Slot::Event(&mut set.act))
}

fn number(number: &mut ValueDef, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Timer(&mut number.timer))?;
    visit(Slot::Integer(&mut number.value))?;
    number.offset.iter_mut().try_for_each(|offset| self::number(offset, visit))
}

fn float_number(number: &mut FloatValueDef, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Timer(&mut number.timer))?;
    visit(Slot::Float(&mut number.value))?;
    number.offset.iter_mut().try_for_each(|offset| self::number(offset, visit))
}

fn text(text: &mut TextDef, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Text(&mut text.value))?;
    visit(Slot::TextWriter(&mut text.event))
}

fn slider(slider: &mut SliderDef, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Timer(&mut slider.timer))?;
    visit(Slot::Float(&mut slider.value))?;
    visit(Slot::FloatWriter(&mut slider.event))
}

fn graph(graph: &mut GraphDef, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Timer(&mut graph.timer))?;
    visit(Slot::Float(&mut graph.value))
}

fn judge(judge: &mut JudgeDef, visit: Visit<'_>) -> Result<(), SkinError> {
    each_destination(&mut judge.images, visit)?;
    each_destination(&mut judge.numbers, visit)
}

fn custom_event(event: &mut CustomEvent, visit: Visit<'_>) -> Result<(), SkinError> {
    visit(Slot::Event(&mut event.action))?;
    visit(Slot::Boolean(&mut event.condition))
}

fn note(note: &mut NoteSet, visit: Visit<'_>) -> Result<(), SkinError> {
    each_destination(&mut note.group, visit)?;
    each_destination(&mut note.bpm, visit)?;
    each_destination(&mut note.stop, visit)?;
    each_destination(&mut note.time, visit)
}

fn song_list(list: &mut SongList, visit: Visit<'_>) -> Result<(), SkinError> {
    each_destination(&mut list.listoff, visit)?;
    each_destination(&mut list.liston, visit)?;
    each_destination(&mut list.text, visit)?;
    each_destination(&mut list.level, visit)?;
    each_destination(&mut list.lamp, visit)?;
    each_destination(&mut list.playerlamp, visit)?;
    each_destination(&mut list.rivallamp, visit)?;
    each_destination(&mut list.trophy, visit)?;
    each_destination(&mut list.label, visit)?;
    list.graph.as_mut().map_or(Ok(()), |graph| each_destination_slot(graph, visit))
}

/// Visits every slot of a document, list by list in the order the mirror declares them.
///
/// The list of fields is the one the Lua table converter's second walk reads
/// (`from_lua::References`): a reference field added to the mirror belongs in both.
pub(crate) fn each_slot(def: &mut SkinDef, visit: Visit<'_>) -> Result<(), SkinError> {
    def.image.iter_mut().try_for_each(|record| image(record, visit))?;
    def.imageset.iter_mut().try_for_each(|record| image_set(record, visit))?;
    def.value.iter_mut().try_for_each(|record| number(record, visit))?;
    def.floatvalue.iter_mut().try_for_each(|record| float_number(record, visit))?;
    def.text.iter_mut().try_for_each(|record| text(record, visit))?;
    def.slider.iter_mut().try_for_each(|record| slider(record, visit))?;
    def.graph.iter_mut().try_for_each(|record| graph(record, visit))?;
    def.hidden_cover.iter_mut().try_for_each(|cover| visit(Slot::Timer(&mut cover.timer)))?;
    def.lift_cover.iter_mut().try_for_each(|cover| visit(Slot::Timer(&mut cover.timer)))?;
    def.judge.iter_mut().try_for_each(|record| judge(record, visit))?;
    def.custom_events.iter_mut().try_for_each(|record| custom_event(record, visit))?;
    def.custom_timers.iter_mut().try_for_each(|timer| visit(Slot::Timer(&mut timer.timer)))?;
    each_destination(&mut def.destination, visit)?;
    def.note.as_mut().map_or(Ok(()), |record| note(record, visit))?;
    def.songlist.as_mut().map_or(Ok(()), |record| song_list(record, visit))
}

/// Refuses a slot that holds a script, where there is no interpreter to compile it into.
pub(crate) fn refuse(slot: Slot<'_>) -> Result<(), SkinError> {
    match slot.source() {
        Some(_) => Err(SkinError::LuaUnavailable),
        None => Ok(()),
    }
}

/// Compiles the scripts of a document into one interpreter.
#[cfg(feature = "lua")]
pub(crate) struct Settler<'a> {
    /// The interpreter the document's scripts live in.
    pub lua: &'a crate::lua::SkinLua,
    /// The document, for messages.
    pub path: &'a str,
    /// Where a script that will not compile is recorded.
    pub warnings: &'a mut Vec<String>,
}

#[cfg(feature = "lua")]
impl Settler<'_> {
    /// Makes the reference's decision for one slot. A slot that holds no string is left alone.
    ///
    /// A timer script is called once as it is compiled, so this wants a host bound
    /// ([`SkinLua::with_host`](crate::lua::SkinLua::with_host)) whenever a document may carry one.
    pub(crate) fn settle(&mut self, slot: Slot<'_>) -> Result<(), SkinError> {
        use crate::lua::LuaFnKind;

        let Some(source) = slot.source().map(str::to_owned) else {
            return Ok(());
        };
        match slot {
            Slot::Boolean(property) => *property = self.property(&source, LuaFnKind::Boolean),
            Slot::Integer(property) => *property = self.property(&source, LuaFnKind::Integer),
            Slot::Float(property) => *property = self.property(&source, LuaFnKind::Float),
            Slot::Text(property) => *property = self.property(&source, LuaFnKind::Text),
            Slot::Timer(property) => *property = self.compiled(&source, LuaFnKind::Timer).map(PropertyRef::Func),
            Slot::Event(event) => *event = self.compiled(&source, LuaFnKind::Event).map(EventRef::Lua),
            Slot::FloatWriter(writer) => *writer = self.compiled(&source, LuaFnKind::FloatWriter).map(FloatWriterRef::Lua),
            Slot::TextWriter(writer) => *writer = self.compiled(&source, LuaFnKind::TextWriter).map(StringWriterRef::Lua),
            Slot::Condition(option) => option.property = self.property(&source, LuaFnKind::Boolean),
        }
        Ok(())
    }

    /// A string in a property field: the property it names, or the function it compiles into.
    fn property(&mut self, source: &str, kind: crate::lua::LuaFnKind) -> Option<PropertyRef> {
        let name = match kind {
            crate::lua::LuaFnKind::Boolean => source.trim_start_matches(crate::property::NEGATION_MARK),
            _ => source,
        };
        if crate::lua::main_state::is_property_name(kind, name) {
            return Some(PropertyRef::Name(source.to_owned()));
        }
        self.compiled(source, kind).map(PropertyRef::Func)
    }

    /// The function `source` compiles into as a field of `kind`, or `None` and a warning.
    fn compiled(&mut self, source: &str, kind: crate::lua::LuaFnKind) -> Option<crate::dst::LuaFnId> {
        match self.lua.compile(source, kind) {
            Ok(function) => Some(function),
            Err(error) => {
                self.warnings.push(format!("{}: {error}", self.path));
                None
            }
        }
    }
}
