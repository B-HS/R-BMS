//! The song wheel a browser body declares, and the pieces each of its rows draws.
//!
//! The two formats disagree about what a wheel is made of. A comma-separated body declares one bar
//! image per *kind* of row and one rectangle per *slot*, and places every other piece -- the level,
//! the lamps, the label -- once, relative to whichever bar it lands on. This build's model instead
//! carries one entry per slot in every list. The relative pieces are therefore collected while the
//! body is read and spread across the slots here, each one placed against the bar of its own slot.

use std::collections::BTreeMap;

use crate::model::{Animation, Destination, ImageSet, SongList, TextDef};

use super::body::Builder;
use super::convert::{FIELD_COUNT, Rect, parse_fields, push_keyframe};
use super::{Outcome, graphs};

/// The pieces a row draws beside its bar, in the order the model lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Level,
    Lamp,
    MyLamp,
    RivalLamp,
    Trophy,
    Label,
    Graph,
    Title,
}

/// The piece a `#SRC_BAR_*` or `#DST_BAR_*` command names.
fn part_of(name: &str) -> Option<Part> {
    Some(match name.trim_start_matches("SRC_BAR_").trim_start_matches("DST_BAR_") {
        "LEVEL" => Part::Level,
        "LAMP" => Part::Lamp,
        "MY_LAMP" => Part::MyLamp,
        "RIVAL_LAMP" => Part::RivalLamp,
        "TROPHY" => Part::Trophy,
        "LABEL" => Part::Label,
        "GRAPH" => Part::Graph,
        "TITLE" => Part::Title,
        _ => return None,
    })
}

/// One piece as the body declared it: which object draws it, and where it sits on a bar.
#[derive(Debug, Default)]
struct PartState {
    object: Option<String>,
    values: Option<[i32; FIELD_COUNT]>,
    fields: Vec<String>,
}

/// Everything a browser body collects before its wheel can be assembled.
#[derive(Debug, Default)]
pub(crate) struct SelectState {
    bars: BTreeMap<usize, String>,
    off: BTreeMap<usize, usize>,
    on: BTreeMap<usize, usize>,
    rects: BTreeMap<usize, Rect>,
    center: i32,
    clickable: Vec<i32>,
    parts: BTreeMap<Part, PartState>,
    declared: bool,
}

/// The id every slot's bar names once the wheel is assembled, which is the one image set the bar
/// images of every row kind were gathered into.
const BAR_SET_ID: &str = "bar-body";

/// The id a slot's bar is parked under while the body is read, which has to be its own so the slot
/// it belongs to is not guessed at when the wheel is assembled.
fn slot_id(slot: usize, focused: bool) -> String {
    let half = if focused { "on" } else { "off" };
    format!("bar-{half}-{slot}")
}

/// Runs one browser command, or answers `None` when the name is not one of them.
pub(crate) fn execute(builder: &mut Builder, name: &str, fields: &[&str], warnings: &mut Vec<String>) -> Option<Outcome> {
    let values = parse_fields(fields);
    match name {
        "SRC_BAR_BODY" => source_bar(builder, &values),
        "DST_BAR_BODY_OFF" => destination_bar(builder, &values, fields, false),
        "DST_BAR_BODY_ON" => destination_bar(builder, &values, fields, true),
        "BAR_CENTER" => builder.select.center = values[1],
        "BAR_AVAILABLE" => builder.select.clickable = (values[1]..=values[2]).collect(),
        "SRC_BAR_FLASH" | "DST_BAR_FLASH" | "SRC_README" | "DST_README" | "SRC_BAR_RANK" | "DST_BAR_RANK" => {}
        _ if name.starts_with("SRC_BAR_") => source_part(builder, name, &values)?,
        _ if name.starts_with("DST_BAR_") => destination_part(builder, name, &values, fields)?,
        _ => return graphs::execute(builder, name, fields, warnings),
    }
    Some(Outcome::Handled)
}

/// `#SRC_BAR_BODY,kind,slot,x,y,w,h,divx,divy,cycle,timer`, one row kind's bar image.
fn source_bar(builder: &mut Builder, values: &[i32; FIELD_COUNT]) {
    let Some(id) = builder.add_image(values, "bar") else { return };
    let Ok(kind) = usize::try_from(values[1]) else { return };
    builder.select.bars.insert(kind, id);
    builder.select.declared = true;
}

/// `#DST_BAR_BODY_OFF,slot,...` and its `_ON` twin, one rectangle per slot of the wheel.
fn destination_bar(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str], focused: bool) {
    let Ok(slot) = usize::try_from(values[1]) else { return };
    builder.select.declared = true;
    let held = if focused { builder.select.on.get(&slot) } else { builder.select.off.get(&slot) };
    let index = match held {
        Some(index) => *index,
        None => {
            let index = builder.add_destination(slot_id(slot, focused));
            if focused {
                builder.select.on.insert(slot, index);
            } else {
                builder.select.off.insert(slot, index);
            }
            index
        }
    };
    let rect = builder.geometry.dst_rect(values, true);
    builder.keyframe(index, rect, values, fields, &[]);
    if !focused {
        builder.select.rects.entry(slot).or_insert(rect);
    }
}

/// `#SRC_BAR_*,kind,...`, the object one of a row's pieces is drawn with. The first kind a body
/// declares stands for every kind, because this build draws a row's level, lamps and label from the
/// row itself rather than from the strip the kind indexes.
fn source_part(builder: &mut Builder, name: &str, values: &[i32; FIELD_COUNT]) -> Option<()> {
    let part = part_of(name)?;
    builder.select.declared = true;
    if builder.select.parts.get(&part).is_some_and(|state| state.object.is_some()) {
        return Some(());
    }
    let id = match part {
        Part::Title | Part::Level => {
            let id = builder.next_id("bar-text");
            builder.def.text.push(TextDef { id: id.clone(), align: values[12], ..TextDef::default() });
            id
        }
        _ => builder.add_image(values, "bar-part")?,
    };
    builder.select.parts.entry(part).or_default().object = Some(id);
    Some(())
}

/// `#DST_BAR_*,kind,...`, where the piece sits on whichever bar its row landed on.
fn destination_part(builder: &mut Builder, name: &str, values: &[i32; FIELD_COUNT], fields: &[&str]) -> Option<()> {
    let part = part_of(name)?;
    let state = builder.select.parts.entry(part).or_default();
    if state.values.is_some() {
        return Some(());
    }
    state.values = Some(*values);
    state.fields = fields.iter().map(|field| (*field).to_owned()).collect();
    Some(())
}

/// Assembles the wheel once the whole body has been read.
pub(crate) fn finish(builder: &mut Builder) {
    if !builder.select.declared {
        return;
    }
    let slots = [builder.select.off.keys().copied().max(), builder.select.on.keys().copied().max()].into_iter().flatten().max();
    let Some(count) = slots.map(|highest| highest + 1) else { return };

    let images: Vec<String> = builder.select.bars.values().cloned().collect();
    if !images.is_empty() {
        builder.def.imageset.push(ImageSet { id: BAR_SET_ID.to_owned(), images, ..ImageSet::default() });
    }
    let off_ids: Vec<String> = (0..count).map(|slot| slot_id(slot, false)).collect();
    let on_ids: Vec<String> = (0..count).map(|slot| slot_id(slot, true)).collect();
    let listoff = named_slots(super::detach(&mut builder.def, &off_ids));
    let liston = named_slots(super::detach(&mut builder.def, &on_ids));

    let parts = std::mem::take(&mut builder.select.parts);
    let rects: Vec<Rect> = (0..count).map(|slot| builder.select.rects.get(&slot).copied().unwrap_or(Rect { x: 0, y: 0, w: 0, h: 0 })).collect();
    let spread = |part: Part| -> Vec<Destination> { parts.get(&part).map(|state| spread_part(builder, state, &rects)).unwrap_or_default() };
    let (text, level, lamp) = (spread(Part::Title), spread(Part::Level), spread(Part::Lamp));
    let (playerlamp, rivallamp, trophy) = (spread(Part::MyLamp), spread(Part::RivalLamp), spread(Part::Trophy));
    let (label, graph) = (spread(Part::Label), spread(Part::Graph).into_iter().next());

    let wheel = SongList {
        id: builder.next_id("songlist"),
        center: builder.select.center,
        clickable: std::mem::take(&mut builder.select.clickable),
        listoff,
        liston,
        text,
        level,
        lamp,
        playerlamp,
        rivallamp,
        trophy,
        label,
        graph,
    };
    let id = wheel.id.clone();
    builder.def.songlist = Some(wheel);
    let index = builder.add_destination(id);
    let bounds = wheel_bounds(&rects);
    if let Some(destination) = builder.def.destination.get_mut(index) {
        destination.dst.push(Animation { time: Some(0), x: Some(bounds.x), y: Some(bounds.y), w: Some(bounds.w), h: Some(bounds.h), ..Animation::default() });
    }
}

/// Renames every slot's bar to the image set the bar images were gathered into, now that each one
/// has been lifted out of the place it was parked in.
fn named_slots(slots: Vec<Destination>) -> Vec<Destination> {
    slots.into_iter().map(|destination| Destination { id: BAR_SET_ID.to_owned(), ..destination }).collect()
}

/// One piece placed on every slot, each against its own bar.
///
/// The body writes the piece's rectangle relative to the bar's top-left corner with the vertical
/// axis pointing down, which is the corner the format measures everything from.
fn spread_part(builder: &Builder, state: &PartState, rects: &[Rect]) -> Vec<Destination> {
    let (Some(id), Some(values)) = (state.object.as_ref(), state.values.as_ref()) else {
        return Vec::new();
    };
    let fields: Vec<&str> = state.fields.iter().map(String::as_str).collect();
    rects
        .iter()
        .map(|bar| {
            let mut destination = Destination { id: id.clone(), ..Destination::default() };
            let rect = Rect {
                x: bar.x + builder.geometry.scale_x(values[3]),
                y: bar.y + bar.h - builder.geometry.scale_y(values[4] + values[6]),
                w: builder.geometry.scale_x(values[5]),
                h: builder.geometry.scale_y(values[6]),
            };
            push_keyframe(&mut destination, rect, values, &fields, &[]);
            destination
        })
        .collect()
}

/// The rectangle that holds every slot, which is the wheel's own destination.
fn wheel_bounds(rects: &[Rect]) -> Rect {
    let used: Vec<&Rect> = rects.iter().filter(|rect| rect.w > 0 && rect.h > 0).collect();
    if used.is_empty() {
        return Rect { x: 0, y: 0, w: 0, h: 0 };
    }
    let left = used.iter().map(|rect| rect.x).min().unwrap_or_default();
    let bottom = used.iter().map(|rect| rect.y).min().unwrap_or_default();
    let right = used.iter().map(|rect| rect.x + rect.w).max().unwrap_or_default();
    let top = used.iter().map(|rect| rect.y + rect.h).max().unwrap_or_default();
    Rect { x: left, y: bottom, w: right - left, h: top - bottom }
}
