//! The commands only a play document carries: the lanes and their note images, the judgement
//! pop-ups, the covers, the background animation and the three charts drawn beside the field.
//!
//! The note object is the one place the format and this build's model differ in shape. A body
//! declares its notes one lane at a time and its lane rectangles one `#DST_NOTE` at a time, while
//! the model carries a single note set holding a list per family; the lists are therefore collected
//! here and spread into the set once every line has been read.

use std::collections::BTreeMap;

use crate::loader::skin_type_mode;
use crate::model::{Animation, BgaDef, JudgeDef, NoteSet, ValueDef};

use super::body::{Builder, add_hidden_cover, plain_destination};
use super::convert::{FIELD_COUNT, Rect, parse_fields};
use super::{Outcome, graphs};

/// The nudge every object anchored to the judgement line follows, so a lifted field carries them.
const OFFSET_LIFT: i32 = 3;

/// The nudge the note field itself follows.
const OFFSET_NOTES: i32 = 30;

/// The nudge a judgement pop-up follows. The reference numbers both sides the same.
const OFFSET_JUDGE: i32 = 32;

/// How many judgements a pop-up carries a word for.
const JUDGEMENTS: usize = 6;

/// How many sides a document may declare a pop-up for.
const JUDGE_SIDES: usize = 3;

/// The lane number that names a side's scratch rather than one of its keys.
const SCRATCH_STEP: i32 = 10;

/// A number object's `align` that centres it, which is what makes a judgement count sit on its own
/// middle rather than start there.
const ALIGN_CENTER: i32 = 2;

/// Halves a centred count's width, which is how far left its anchor moves.
const CENTER_HALVES: i32 = 2;

/// One judgement pop-up being collected.
#[derive(Debug, Default)]
struct JudgeState {
    /// Where the pop-up's own record sits in the document's `judge` list.
    def: usize,
    /// Judgement slot to the top-level destination the word was parked in.
    images: BTreeMap<usize, (usize, String)>,
    /// Judgement slot to the top-level destination the count was parked in.
    numbers: BTreeMap<usize, (usize, String)>,
}

/// Everything a play body collects before its note set can be assembled.
#[derive(Debug, Default)]
pub(crate) struct PlayState {
    lanes: BTreeMap<usize, Rect>,
    families: BTreeMap<Family, BTreeMap<usize, String>>,
    note_offsets: Vec<i32>,
    expansion: Option<(i32, i32)>,
    note_two: Option<i32>,
    lines: Vec<String>,
    line_current: BTreeMap<i32, usize>,
    judge: BTreeMap<usize, JudgeState>,
    bga: Option<usize>,
    judgeline: Option<usize>,
    cover: Option<usize>,
}

/// One list of per-lane note images the model carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Family {
    Note,
    LnStart,
    LnEnd,
    LnBody,
    LnBodyActive,
    HcnStart,
    HcnEnd,
    HcnBody,
    HcnBodyActive,
    HcnDamage,
    HcnReactive,
    Mine,
}

/// The families one `#SRC_*` note command fills. `#SRC_LN_BODY` fills both halves of the long-note
/// body, which is what makes the `_INACTIVE` and `_ACTIVE` commands overrides rather than
/// requirements.
fn note_families(name: &str) -> Option<&'static [Family]> {
    Some(match name {
        "SRC_NOTE" => &[Family::Note],
        "SRC_LN_START" => &[Family::LnStart],
        "SRC_LN_END" => &[Family::LnEnd],
        "SRC_LN_BODY" => &[Family::LnBody, Family::LnBodyActive],
        "SRC_LN_BODY_INACTIVE" => &[Family::LnBody],
        "SRC_LN_BODY_ACTIVE" => &[Family::LnBodyActive],
        "SRC_HCN_START" => &[Family::HcnStart],
        "SRC_HCN_END" => &[Family::HcnEnd],
        "SRC_HCN_BODY" => &[Family::HcnBody, Family::HcnBodyActive],
        "SRC_HCN_BODY_INACTIVE" => &[Family::HcnBody],
        "SRC_HCN_BODY_ACTIVE" => &[Family::HcnBodyActive],
        "SRC_HCN_DAMAGE" => &[Family::HcnDamage],
        "SRC_HCN_REACTIVE" => &[Family::HcnReactive],
        "SRC_MINE" => &[Family::Mine],
        _ => return None,
    })
}

/// Which lane of this build's note lists a body's lane number means, or `None` when the mode has no
/// such lane.
///
/// A number that is a multiple of ten names a side's scratch; anything else is that side's key,
/// counted from one. A number past the keys of the mode names nothing.
fn lane_of(mode: rbms_model::Mode, lane: i32) -> Option<usize> {
    let sides = mode.player.max(1) as i32;
    if lane % SCRATCH_STEP == 0 {
        return mode.scratch.get((lane / SCRATCH_STEP) as usize).copied();
    }
    let keys_per_side = (mode.key as i32 - mode.scratch.len() as i32) / sides;
    let within = if lane > SCRATCH_STEP { lane - SCRATCH_STEP - 1 } else { lane - 1 };
    if within < 0 || within >= keys_per_side {
        return None;
    }
    let side = lane / SCRATCH_STEP;
    usize::try_from(within + side * (mode.key as i32 / sides)).ok()
}

/// The side a `#*_NOWJUDGE_nP` or `#*_NOWCOMBO_nP` command names, counted from zero.
fn judge_side(name: &str) -> Option<usize> {
    let side = name.strip_suffix('P')?.chars().next_back()?.to_digit(10)? as usize;
    (1..=JUDGE_SIDES).contains(&side).then(|| side - 1)
}

/// Runs one play command, or answers `None` when the name is not one of them.
pub(crate) fn execute(builder: &mut Builder, name: &str, fields: &[&str], warnings: &mut Vec<String>) -> Option<Outcome> {
    skin_type_mode(builder.skin_type)?;
    let values = parse_fields(fields);
    if let Some(families) = note_families(name) {
        source_note(builder, families, &values);
        return Some(Outcome::Handled);
    }
    match name {
        "SRC_BGA" => {
            let id = builder.next_id("bga");
            builder.def.bga = Some(BgaDef { id: id.clone() });
            builder.play.bga = Some(builder.add_destination(id));
        }
        "DST_BGA" => plain_destination(builder, builder.play.bga, &values, fields),
        "SRC_LINE" => source_line(builder, &values),
        "DST_LINE" => destination_line(builder, &values, fields),
        "DST_NOTE" => destination_note(builder, &values, fields),
        "DST_NOTE2" => builder.play.note_two = Some(crate::model::DEFAULT_SKIN_HEIGHT - builder.geometry.scale_y(values[1])),
        "DST_NOTE_EXPANSION_RATE" => builder.play.expansion = Some((values[1], values[2])),
        "SRC_JUDGELINE" => {
            builder.play.judgeline = builder.add_image(&values, "judgeline").map(|id| builder.add_destination(id));
        }
        "DST_JUDGELINE" => {
            if let Some(index) = builder.play.judgeline {
                let rect = builder.geometry.dst_rect(&values, true);
                builder.keyframe(index, rect, &values, fields, &[OFFSET_LIFT]);
            }
        }
        "SRC_HIDDEN" | "SRC_LIFT" => builder.play.cover = add_hidden_cover(builder, &values, name == "SRC_LIFT"),
        "DST_HIDDEN" | "DST_LIFT" => {
            if let Some(index) = builder.play.cover {
                let rect = builder.geometry.dst_rect(&values, false);
                builder.keyframe(index, rect, &values, fields, &[OFFSET_LIFT]);
            }
        }
        _ if name.starts_with("SRC_NOWJUDGE_") => drop(source_now_judge(builder, name, &values)),
        _ if name.starts_with("DST_NOWJUDGE_") => drop(destination_now_judge(builder, name, &values, fields)),
        _ if name.starts_with("SRC_NOWCOMBO_") => drop(source_now_combo(builder, name, &values)),
        _ if name.starts_with("DST_NOWCOMBO_") => drop(destination_now_combo(builder, name, &values, fields)),
        _ => return graphs::execute(builder, name, fields, warnings),
    }
    Some(Outcome::Handled)
}

/// One `#SRC_*` note command: the image of one lane in one or two families.
fn source_note(builder: &mut Builder, families: &[Family], values: &[i32; FIELD_COUNT]) {
    let Some(mode) = skin_type_mode(builder.skin_type) else { return };
    let Some(lane) = lane_of(mode, values[1]) else { return };
    let Some(id) = builder.add_image(values, "note") else { return };
    for family in families {
        builder.play.families.entry(*family).or_default().insert(lane, id.clone());
    }
}

/// `#SRC_LINE,index,...`, the measure line drawn across the field.
fn source_line(builder: &mut Builder, values: &[i32; FIELD_COUNT]) {
    let Some(id) = builder.add_image(values, "line") else { return };
    let index = builder.add_destination(id.clone());
    builder.play.lines.push(id);
    builder.play.line_current.insert(values[1], index);
}

/// `#DST_LINE,index,...`, which follows the lift nudge so it rides with the judgement line.
fn destination_line(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let Some(index) = builder.play.line_current.get(&values[1]).copied() else { return };
    let rect = builder.geometry.dst_rect(values, true);
    builder.keyframe(index, rect, values, fields, &[OFFSET_LIFT]);
}

/// `#DST_NOTE,lane,...`, which is a lane rectangle rather than an animation: the note object itself
/// is assembled from every lane once the body has been read.
fn destination_note(builder: &mut Builder, values: &[i32; FIELD_COUNT], fields: &[&str]) {
    let Some(mode) = skin_type_mode(builder.skin_type) else { return };
    let Some(lane) = lane_of(mode, values[1]) else { return };
    let rect = builder.geometry.rect(values[3], values[4], values[5], values[6]);
    builder.play.lanes.entry(lane).or_insert(rect);
    if builder.play.note_offsets.is_empty() {
        builder.play.note_offsets = super::convert::read_offsets(fields, &[OFFSET_NOTES]);
    }
}

/// `#SRC_NOWJUDGE_nP,judgement,...`, one word of one side's pop-up.
fn source_now_judge(builder: &mut Builder, name: &str, values: &[i32; FIELD_COUNT]) -> Option<()> {
    let side = judge_side(name)?;
    let slot = usize::try_from(values[1]).ok()?;
    if slot >= JUDGEMENTS {
        return Some(());
    }
    let shift = values[11] != 1;
    if !builder.play.judge.contains_key(&side) {
        let id = builder.next_id("judge");
        builder.def.judge.push(JudgeDef { id: id.clone(), index: side as i32, shift, ..JudgeDef::default() });
        let at = builder.def.judge.len() - 1;
        builder.play.judge.insert(side, JudgeState { def: at, ..JudgeState::default() });
        builder.add_destination(id);
    }
    let id = builder.add_image(values, "judge-word")?;
    let index = builder.add_destination(id.clone());
    builder.play.judge.get_mut(&side)?.images.insert(slot, (index, id));
    Some(())
}

/// `#DST_NOWJUDGE_nP,judgement,...`.
fn destination_now_judge(builder: &mut Builder, name: &str, values: &[i32; FIELD_COUNT], fields: &[&str]) -> Option<()> {
    let side = judge_side(name)?;
    let slot = usize::try_from(values[1]).ok()?;
    let index = builder.play.judge.get(&side)?.images.get(&slot)?.0;
    let rect = builder.geometry.dst_rect(values, true);
    builder.keyframe(index, rect, values, fields, &[OFFSET_JUDGE, OFFSET_LIFT]);
    Some(())
}

/// `#SRC_NOWCOMBO_nP,judgement,...`, the count drawn beside one word.
fn source_now_combo(builder: &mut Builder, name: &str, values: &[i32; FIELD_COUNT]) -> Option<()> {
    let side = judge_side(name)?;
    let slot = usize::try_from(values[1]).ok()?;
    let src = builder.source_id(values[2])?;
    let id = builder.next_id("judge-count");
    let (divx, divy) = (values[7].max(1), values[8].max(1));
    builder.def.value.push(ValueDef {
        id: id.clone(),
        src,
        x: values[3],
        y: values[4],
        w: values[5],
        h: values[6],
        divx,
        divy,
        timer: super::body::timer_of(values[10]),
        cycle: values[9],
        align: if values[12] == 1 { ALIGN_CENTER } else { values[12] },
        digit: values[13],
        space: values[15],
        reference: values[11],
        ..ValueDef::default()
    });
    let index = builder.add_destination(id.clone());
    builder.play.judge.get_mut(&side)?.numbers.insert(slot, (index, id));
    Some(())
}

/// `#DST_NOWCOMBO_nP,judgement,...`, placed against its own pop-up rather than the screen.
fn destination_now_combo(builder: &mut Builder, name: &str, values: &[i32; FIELD_COUNT], fields: &[&str]) -> Option<()> {
    let side = judge_side(name)?;
    let slot = usize::try_from(values[1]).ok()?;
    let (index, id) = builder.play.judge.get(&side)?.numbers.get(&slot)?.clone();
    let centred = builder.def.value.iter().find(|value| value.id == id).map(|value| (value.align, value.digit));
    let mut x = values[3];
    if let Some((align, digits)) = centred
        && align == ALIGN_CENTER
    {
        x -= digits * values[5] / CENTER_HALVES;
    }
    let rect = Rect {
        x: builder.geometry.scale_x(x),
        y: -builder.geometry.scale_y(values[4]),
        w: builder.geometry.scale_x(values[5]),
        h: builder.geometry.scale_y(values[6]),
    };
    builder.keyframe(index, rect, values, fields, &[OFFSET_JUDGE, OFFSET_LIFT]);
    Some(())
}

/// Assembles the note set, the pop-ups and the measure lines once the whole body has been read.
pub(crate) fn finish(builder: &mut Builder) {
    finish_judges(builder);
    if builder.play.lanes.is_empty() {
        return;
    }
    let count = builder.play.lanes.keys().copied().max().unwrap_or_default() + 1;
    let lanes: Vec<Rect> = (0..count).map(|lane| builder.play.lanes.get(&lane).copied().unwrap_or(Rect { x: 0, y: 0, w: 0, h: 0 })).collect();
    let mut note = NoteSet { id: builder.next_id("note-field"), ..NoteSet::default() };
    let families = std::mem::take(&mut builder.play.families);
    for (family, images) in &families {
        let list: Vec<String> = (0..count).map(|lane| images.get(&lane).cloned().unwrap_or_default()).collect();
        *list_of(&mut note, *family) = list;
    }
    note.dst = lanes.iter().map(|rect| Animation { x: Some(rect.x), y: Some(rect.y), w: Some(rect.w), h: Some(rect.h), ..Animation::default() }).collect();
    note.dst2 = builder.play.note_two;
    if let Some((w, h)) = builder.play.expansion {
        note.expansionrate = vec![w, h];
    }
    let lines = std::mem::take(&mut builder.play.lines);
    note.group = super::detach(&mut builder.def, &lines);
    note.group.retain(|destination| !destination.dst.is_empty());

    let id = note.id.clone();
    let offsets = std::mem::take(&mut builder.play.note_offsets);
    builder.def.note = Some(note);
    let index = builder.add_destination(id);
    let bounds = field_bounds(&lanes);
    if let Some(destination) = builder.def.destination.get_mut(index) {
        destination.offsets = offsets;
        destination.dst.push(Animation { time: Some(0), x: Some(bounds.x), y: Some(bounds.y), w: Some(bounds.w), h: Some(bounds.h), ..Animation::default() });
    }
}

/// The rectangle that holds every lane, which is the note object's own destination.
fn field_bounds(lanes: &[Rect]) -> Rect {
    let used: Vec<&Rect> = lanes.iter().filter(|rect| rect.w > 0 && rect.h > 0).collect();
    let Some(first) = used.first() else { return Rect { x: 0, y: 0, w: 0, h: 0 } };
    let left = used.iter().map(|rect| rect.x).min().unwrap_or(first.x);
    let bottom = used.iter().map(|rect| rect.y).min().unwrap_or(first.y);
    let right = used.iter().map(|rect| rect.x + rect.w).max().unwrap_or(first.x + first.w);
    let top = used.iter().map(|rect| rect.y + rect.h).max().unwrap_or(first.y + first.h);
    Rect { x: left, y: bottom, w: right - left, h: top - bottom }
}

/// Moves each pop-up's parked destinations into the record they belong to.
fn finish_judges(builder: &mut Builder) {
    let states: Vec<(usize, JudgeState)> = std::mem::take(&mut builder.play.judge).into_values().map(|state| (state.def, state)).collect();
    for (at, state) in states {
        let images = slot_ids(&state.images);
        let numbers = slot_ids(&state.numbers);
        let images = super::detach(&mut builder.def, &images);
        let numbers = super::detach(&mut builder.def, &numbers);
        if let Some(judge) = builder.def.judge.get_mut(at) {
            judge.images = images;
            judge.numbers = numbers;
        }
    }
}

/// The parked destination id of each judgement slot, up to the highest one declared.
fn slot_ids(slots: &BTreeMap<usize, (usize, String)>) -> Vec<String> {
    let count = slots.keys().copied().max().map_or(0, |highest| highest + 1);
    (0..count).map(|slot| slots.get(&slot).map(|(_, id)| id.clone()).unwrap_or_default()).collect()
}

/// The note set list one family fills.
fn list_of(note: &mut NoteSet, family: Family) -> &mut Vec<String> {
    match family {
        Family::Note => &mut note.note,
        Family::LnStart => &mut note.lnstart,
        Family::LnEnd => &mut note.lnend,
        Family::LnBody => &mut note.lnbody,
        Family::LnBodyActive => &mut note.lnbody_active,
        Family::HcnStart => &mut note.hcnstart,
        Family::HcnEnd => &mut note.hcnend,
        Family::HcnBody => &mut note.hcnbody,
        Family::HcnBodyActive => &mut note.hcnbody_active,
        Family::HcnDamage => &mut note.hcndamage,
        Family::HcnReactive => &mut note.hcnreactive,
        Family::Mine => &mut note.mine,
    }
}
