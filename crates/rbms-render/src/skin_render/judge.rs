//! The judgement pop-up a play document draws for itself: the word for the judgement a region of the
//! field last took, and the combo that rides beside it (`SkinJudge`).
//!
//! The pop-up is one object made of other objects. The document nests a destination per judgement
//! under `judge.images` and another under `judge.numbers`, the loader assembles both lists into
//! tracks, and this pairs each track with the image or the digit strip its id names. Everything else
//! is the arrangement the reference makes, kept as it is:
//!
//! - `index` names the judgement region the pop-up follows. A region is a share of the lanes with a
//!   judgement, a combo and a timer of its own ([`JudgeFrame`](super::frame::JudgeFrame)), so the two
//!   pop-ups of a double screen show two different judgements. A pop-up whose region has judged
//!   nothing is not drawn, and neither is one that names a region the screen does not have.
//! - The word is the image at the judgement's place in `images`, best first. A perfect great taken
//!   on a full gauge shows the seventh image instead, and the first when the document has no seventh.
//! - The count is the number at the same place in `numbers`, but only for the three judgements that
//!   keep a combo. It counts the combo the run stood at when the region was last judged, whatever
//!   property the document's own `ref` names, and it is always centred over its places.
//! - The count is placed against the word's own corner rather than the screen's, and the loader has
//!   already pulled it left by half the places it reserves ([`pull_count_left`]).
//! - With `shift` the word slides left by half the width of the digits on show, measured before the
//!   slide, so the pair stays centred as the count grows.
//!
//! Nothing here decides how long the pop-up stays. A region's judgement is never cleared; the word
//! and the count each follow whatever timer their own destination names, which a document sets to the
//! region's judge timer and plays once.

use rbms_skin::dst::{DestinationTrack, DrawStateSource, Resolved, SkinRect, prepare};
use rbms_skin::loader::{LoadedSkin, StretchKind};
use rbms_skin::model::{Destination, ImageDef, ValueDef};
use rbms_skin::property::generated::FLOAT_GROOVEGAUGE_1P;

use super::draw::{ImageSelect, Placement};
use super::frame::JudgeHit;
use super::object::{Body, ImageBody, MAX_PLACES, SkinObject, Sprite, image_sprite};
use super::textures::Source;
use super::{SkinAssets, SkinFrame};
use crate::ctx::RenderCtx;
use crate::{BlendMode, Renderer};

/// How many words and counts one pop-up keeps: one for each of the six judgements, and a seventh
/// for a perfect great on a full gauge (`SkinJudge`'s two arrays). A document that lists more has
/// the rest ignored.
pub(crate) const JUDGE_SLOTS: usize = 7;

/// The slot a perfect great reads while the gauge is full.
const FULL_GAUGE_SLOT: usize = 6;

/// The judgement that has such a slot, which is also the slot it falls back to.
const PERFECT_GREAT: usize = 0;

/// How many judgements a region reports, best first.
const JUDGEMENTS: usize = 6;

/// How many of them keep a combo. The ones past these break it, so no count is drawn beside them.
const COMBO_JUDGEMENTS: usize = 3;

/// The whole numbers `SkinNumber.prepare` reads as "nothing to show".
const INTEGER_NO_VALUE: [i32; 2] = [i32::MIN, i32::MAX];

/// What a keyframe field reads as while the document has left it out (`JsonSkin.Animation`).
const FIELD_UNSET: i32 = i32::MIN;

/// The digits a strip is cut into when it holds nothing else.
const DIGIT_GLYPHS: u32 = 10;

/// The glyphs of a strip that carries an alternate zero after its ten digits.
const DIGIT_GLYPHS_WITH_ALTERNATE_ZERO: u32 = 11;

/// The glyph slot of that alternate zero, which fills the places a count leaves empty.
const GLYPH_ALTERNATE_ZERO: u32 = 10;

/// The base a count is written in.
const RADIX: u32 = 10;

/// The share of a full gauge a host reports for one that is full.
const FULL_GAUGE_SHARE: f32 = 1.0;

/// One pop-up, resolved from one of the document's `judge` objects.
#[derive(Debug)]
pub(crate) struct JudgeBody {
    /// The word each judgement shows, by slot. A slot the document left out, or whose image this
    /// build could not load, is `None`.
    pub(crate) words: Vec<Option<SkinObject>>,
    /// The count beside it, by the same slot.
    pub(crate) counts: Vec<Option<JudgeCount>>,
    pub(crate) shift: bool,
    /// The judgement region the pop-up follows: the document's `index`, as it wrote it.
    pub(crate) region: i32,
}

/// The count of one judgement: a destination, the digit strip it is cut from and how that strip is
/// read.
#[derive(Debug)]
pub(crate) struct JudgeCount {
    /// The destination the count is placed by, holding the strip as the image it is.
    pub(crate) part: SkinObject,
    /// How many glyphs one set of the strip holds: the ten digits, or those and an alternate zero.
    pub(crate) glyphs: u32,
    /// How many sets the strip holds, which its own timer and cycle step through.
    pub(crate) sets: u32,
    /// How many places the count reserves.
    pub(crate) places: u32,
    /// The gap between places, in document pixels.
    pub(crate) space: f32,
    /// Per-place nudges, as far as the document declared them, in screen pixels.
    pub(crate) offsets: Vec<(f32, f32, f32, f32)>,
}

impl JudgeCount {
    /// Whether an empty place is filled with the strip's alternate zero, which a strip that has one
    /// always does (`JsonPlaySkinObjectLoader`: `d > 10 ? 2 : 0`).
    fn fills_blanks(&self) -> bool {
        self.glyphs > DIGIT_GLYPHS
    }

    /// The glyph the place `index` places from the left shows for `combo`, or `None` for a place
    /// left empty (`SkinNumber.prepare`, the branch of a number with no negative strip).
    fn glyph(&self, combo: u32, index: u32) -> Option<u32> {
        let from_right = self.places - 1 - index;
        let remaining = RADIX.checked_pow(from_right).map_or(0, |scale| combo / scale);
        if remaining > 0 || from_right == 0 {
            Some(remaining % RADIX)
        } else if self.fills_blanks() {
            Some(GLYPH_ALTERNATE_ZERO)
        } else {
            None
        }
    }

    /// How many places are drawn for `combo`.
    fn drawn_places(&self, combo: u32) -> u32 {
        (0..self.places).filter(|index| self.glyph(combo, *index).is_some()).count() as u32
    }
}

/// The strip a part of the pop-up is cut from.
fn strip(part: &SkinObject) -> Option<&Sprite> {
    let Body::Image(image) = &part.body else {
        return None;
    };
    image.variants.first()?.as_ref().map(|(sprite, _, _)| sprite)
}

/// One part of the pop-up over the destination the loader assembled for it, holding `sprite` whole:
/// the reference hands a judge's image every cell of its source as one animation, whatever `len` or
/// `ref` the image declares.
fn part(track: DestinationTrack, sprite: Sprite) -> SkinObject {
    let body = Body::Image(ImageBody { variants: vec![Some((sprite, 0, sprite.cells()))], select: ImageSelect::First });
    SkinObject { stretch: StretchKind::from_id(track.stretch), track, body }
}

/// The image definition a value's strip is cut by, so a count's digits go through the same cutting
/// rules an image does.
fn strip_image(value: &ValueDef) -> ImageDef {
    ImageDef {
        src: value.src.clone(),
        x: value.x,
        y: value.y,
        w: value.w,
        h: value.h,
        divx: value.divx,
        divy: value.divy,
        timer: value.timer.clone(),
        cycle: value.cycle,
        ..ImageDef::default()
    }
}

/// Pulls a count left by half the places it reserves, keyframe by keyframe, exactly as the
/// reference's loader does before it reads the destination (`JsonPlaySkinObjectLoader`:
/// `ani.x -= ani.w * value.digit / 2`).
///
/// The arithmetic is the reference's whole-number arithmetic on the keyframes as the document wrote
/// them, where a field left out is the lowest whole number rather than absent. That is what makes a
/// keyframe that names neither `x` nor `w` still inherit the one before it when the count has an
/// even number of places, and what moves a keyframe that names an `x` but no `w` by nothing at all.
/// Both are how the reference behaves, so both are kept.
///
/// `track` holds the same keyframes filled in and sorted by time, so the new `x` values are filled
/// in and sorted the same way before they are written back. A slot the loader assembled no keyframe
/// for is left alone.
pub(crate) fn pull_count_left(track: &mut DestinationTrack, declared: &Destination, digit: i32) {
    let mut moved: Vec<(i64, i32)> = Vec::with_capacity(declared.dst.len());
    for frame in &declared.dst {
        let (previous_time, previous_x) = moved.last().copied().unwrap_or((0, 0));
        let raw = frame.x.unwrap_or(FIELD_UNSET).wrapping_sub(frame.w.unwrap_or(FIELD_UNSET).wrapping_mul(digit) / 2);
        moved.push((frame.time.unwrap_or(previous_time), if raw == FIELD_UNSET { previous_x } else { raw }));
    }
    moved.sort_by_key(|(time, _)| *time);
    if moved.len() != track.frames.len() {
        return;
    }
    for (frame, (_, x)) in track.frames.iter_mut().zip(moved) {
        frame.rect.x = x as f32;
    }
}

/// Everything resolving one pop-up's nested parts needs from the build.
struct PartBuilder<'a, 'b> {
    skin: &'a LoadedSkin,
    sources: Source<'a>,
    judge: &'a str,
    warnings: &'b mut Vec<String>,
}

impl PartBuilder<'_, '_> {
    /// The word one entry of `images` names, or `None` when its id is no image of the document or
    /// the image has no source this build could load. The reference looks among the document's
    /// images and nowhere else.
    fn word(&mut self, track: &DestinationTrack, id: &str) -> Option<SkinObject> {
        let Some(image) = self.skin.def.image.iter().find(|image| image.id == id) else {
            self.warnings.push(format!("judge {:?} names {id:?}, which is no image of the document", self.judge));
            return None;
        };
        let Some(sprite) = image_sprite(image, self.sources) else {
            self.warnings.push(format!("judge {:?} has no usable source {:?} for {id:?}", self.judge, image.src));
            return None;
        };
        Some(part(track.clone(), sprite))
    }

    /// The count one entry of `numbers` names, or `None` when its id is no value of the document,
    /// the value has no source this build could load, or its strip is too short to hold one set of
    /// digits.
    ///
    /// The strip is cut the way the reference cuts a judge's count and no other number: ten glyphs
    /// a set when its cells divide by ten and eleven otherwise, with no negative half however many
    /// cells there are. The places that are drawn stop at [`MAX_PLACES`], as every number's do; the
    /// pull to the left is by the places the document wrote.
    fn count(&mut self, track: &DestinationTrack, declared: &Destination) -> Option<JudgeCount> {
        let id = declared.id.as_str();
        let Some(value) = self.skin.def.value.iter().find(|value| value.id == id) else {
            self.warnings.push(format!("judge {:?} names {id:?}, which is no value of the document", self.judge));
            return None;
        };
        let Some(sprite) = image_sprite(&strip_image(value), self.sources) else {
            self.warnings.push(format!("judge {:?} has no usable source {:?} for {id:?}", self.judge, value.src));
            return None;
        };
        let cells = sprite.cells();
        let glyphs = if cells.is_multiple_of(DIGIT_GLYPHS) { DIGIT_GLYPHS } else { DIGIT_GLYPHS_WITH_ALTERNATE_ZERO };
        let sets = cells / glyphs;
        if sets == 0 {
            self.warnings.push(format!("judge {:?} cuts {id:?} into {cells} cells, which is not one set of digits", self.judge));
            return None;
        }
        let mut track = track.clone();
        pull_count_left(&mut track, declared, value.digit);
        Some(JudgeCount {
            part: part(track, sprite),
            glyphs,
            sets,
            places: value.digit.clamp(0, MAX_PLACES as i32) as u32,
            space: value.space as f32,
            offsets: value.offset.iter().map(|offset| (offset.x as f32, offset.y as f32, offset.w as f32, offset.h as f32)).collect(),
        })
    }
}

/// The pop-up behind `id`, or `None` when the document declares no `judge` by that name.
///
/// The slots are paired by position, as the reference pairs them: the count of a judgement is the
/// entry of `numbers` at the place its word has in `images`, and a count with no word at its place
/// is never read. A slot that cannot be resolved stays empty rather than costing the pop-up, which
/// then shows nothing for that judgement.
pub(crate) fn build_judge(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    _families: &[(String, String)],
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = skin.def.judge.iter().find(|judge| judge.id == id)?;
    let Some(tracks) = skin.nested.judge.get(id) else {
        warnings.push(format!("judge {id:?} has no assembled destinations, so its pop-up is dropped"));
        return None;
    };
    let mut builder = PartBuilder { skin, sources, judge: id, warnings };
    let slots = def.images.len().min(JUDGE_SLOTS);
    let mut words = Vec::with_capacity(slots);
    let mut counts = Vec::with_capacity(slots);
    for (slot, image) in def.images.iter().take(slots).enumerate() {
        words.push(tracks.images.get(slot).and_then(|named| builder.word(&named.track, &image.id)));
        counts.push(def.numbers.get(slot).zip(tracks.numbers.get(slot)).and_then(|(declared, named)| builder.count(&named.track, declared)));
    }
    Some(Body::Judge(JudgeBody { words, counts, shift: def.shift, region: def.index }))
}

/// What the region a pop-up follows last reported, or `None` while there is nothing to show for it.
fn judged(body: &JudgeBody, frame: &SkinFrame<'_>) -> Option<JudgeHit> {
    frame.data.judge.region(body.region).filter(|hit| hit.judgement < JUDGEMENTS)
}

/// Whether the region a pop-up follows has judged anything yet.
///
/// The reference asks this before anything else about the pop-up, its own draw conditions included,
/// so a pop-up with nothing to report calls none of the functions the skin gated it on.
pub(crate) fn has_judgement(body: &JudgeBody, frame: &SkinFrame<'_>) -> bool {
    judged(body, frame).is_some()
}

/// Whether the gauge on show is full, which is when a perfect great shows the seventh word
/// (`GrooveGauge.Gauge.isMax`).
fn gauge_is_full(frame: &SkinFrame<'_>) -> bool {
    let Some(gauge) = frame.data.gauge else {
        return false;
    };
    match (gauge.value, gauge.shown_scale()) {
        (Some(value), Some(scale)) => value == scale.max,
        (None, Some(_)) => frame.state.float(FLOAT_GROOVEGAUGE_1P) == FULL_GAUGE_SHARE,
        (_, None) => false,
    }
}

/// The word and the count one judgement reads (`SkinJudge.prepare`).
///
/// The seventh slot is looked for word and count apart, so a document that draws a full-gauge word
/// but no full-gauge count keeps the ordinary count beside it.
fn slots(body: &JudgeBody, judgement: usize, full_gauge: bool) -> (Option<&SkinObject>, Option<&JudgeCount>) {
    let word = |slot: usize| body.words.get(slot).and_then(Option::as_ref);
    let count = |slot: usize| body.counts.get(slot).and_then(Option::as_ref);
    if judgement == PERFECT_GREAT && full_gauge {
        return (word(FULL_GAUGE_SLOT).or_else(|| word(PERFECT_GREAT)), count(FULL_GAUGE_SLOT).or_else(|| count(PERFECT_GREAT)));
    }
    (word(judgement), if judgement < COMBO_JUDGEMENTS { count(judgement) } else { None })
}

/// The count as one frame shows it.
struct ShownCount<'a> {
    count: &'a JudgeCount,
    strip: &'a Sprite,
    /// Where its first place sits, the word's corner already added.
    at: Resolved,
    /// The combo, without its sign.
    combo: u32,
    /// Which set of the strip this frame animates to.
    set: u32,
}

impl ShownCount<'_> {
    /// The distance from one place to the next.
    fn step(&self) -> f32 {
        self.at.rect.w + self.count.space
    }

    /// How wide the digits on show are (`SkinNumber.getLength`).
    fn length(&self) -> f32 {
        self.step() * self.count.drawn_places(self.combo) as f32
    }
}

/// The pop-up as one frame shows it.
struct Shown<'a> {
    word: &'a SkinObject,
    strip: &'a Sprite,
    /// Where the word is drawn, slid left by half its count when the pop-up shifts.
    at: Resolved,
    count: Option<ShownCount<'a>>,
}

/// Resolves one part's destination for this frame, measured from `origin`.
fn place(part: &SkinObject, origin: (f32, f32), frame: &SkinFrame<'_>) -> Option<Resolved> {
    let state: &dyn DrawStateSource = frame.state;
    prepare(&part.track, frame.now_us, frame.timers, state, frame.script(), origin, frame.mouse)
}

/// Settles the count beside a word that is drawn at `corner` (`SkinNumber.prepare`).
///
/// A count that is not drawn -- a combo with nothing to show, a condition that does not hold, a
/// timer that is off -- reads no further, so the timer its strip animates by is asked only for a
/// count that is.
fn arrange_count<'a>(count: &'a JudgeCount, combo: i32, corner: (f32, f32), frame: &SkinFrame<'_>) -> Option<ShownCount<'a>> {
    if INTEGER_NO_VALUE.contains(&combo) {
        return None;
    }
    let strip = strip(&count.part)?;
    let at = place(&count.part, corner, frame)?;
    let set = strip.animation_index(count.sets, frame.now_us, frame.timers, frame.script());
    Some(ShownCount { count, strip, at, combo: combo.unsigned_abs(), set })
}

/// Settles what the pop-up shows this frame, or `None` when it shows nothing (`SkinJudge.prepare`).
///
/// Whatever the parts ask the skin's Lua -- a condition, a timer, the cell an animation is on -- is
/// asked here in the reference's order: the word's conditions and timer, then the timer its image
/// animates by, which is read whether or not the word is drawn, and only for a word that is drawn
/// the count's. Both stages of a frame come through here, so the draw stage reads back exactly the
/// answers the prepare stage was given.
fn arrange<'a>(body: &'a JudgeBody, frame: &SkinFrame<'_>) -> Option<Shown<'a>> {
    let hit = judged(body, frame)?;
    let (word, count) = slots(body, hit.judgement, gauge_is_full(frame));
    let word = word?;
    let placed = place(word, (0.0, 0.0), frame);
    let strip = strip(word)?;
    strip.prepare(frame);
    let mut at = placed?;
    let count = count.and_then(|count| arrange_count(count, hit.combo, (at.rect.x, at.rect.y), frame));
    if body.shift
        && let Some(count) = &count
    {
        at.rect.x -= count.length() / 2.0;
    }
    Some(Shown { word, strip, at, count })
}

/// Prepares the pop-up, answering whether it shows anything this frame.
///
/// The reference prepares the word and the count even when the pop-up's own conditions have already
/// left it out of the frame, so this is called whether or not the pop-up itself was placed.
pub(crate) fn prepare_judge(body: &JudgeBody, frame: &SkinFrame<'_>) -> bool {
    arrange(body, frame).is_some()
}

/// The same placement over one of the pop-up's own parts, tinted and turned the way that part's
/// destination asked.
fn part_placement<'a>(part: &'a SkinObject, resolved: &Resolved, place: &Placement<'a>) -> Placement<'a> {
    Placement {
        object: part,
        blend: BlendMode::from_skin_blend(part.track.blend),
        tint: resolved.color.into(),
        angle_deg: resolved.angle_deg,
        viewport: place.viewport,
    }
}

/// A length measured in screen pixels as the document length that lands on it.
fn document_length(screen: f32, scale: f32) -> f32 {
    if scale > 0.0 { screen / scale } else { 0.0 }
}

/// Draws the count, one place at a time, centred over the places it reserves (`SkinNumber.draw`
/// with the `align` of two a judge's count is always built with).
///
/// The per-place nudges are screen pixels, as they are for every number: the reference adds them to
/// a destination it has already scaled to the screen.
fn draw_count<R: Renderer>(r: &mut R, place: &Placement<'_>, shown: &ShownCount<'_>) -> bool {
    if shown.at.color.a == 0 {
        return false;
    }
    let count = shown.count;
    let part = part_placement(&count.part, &shown.at, place);
    let rect = shown.at.rect;
    let step = shown.step();
    let blanks = count.places - count.drawn_places(shown.combo);
    let centring = step * 0.5 * blanks as f32;
    let (scale_x, scale_y) = (place.viewport.scale_x(), place.viewport.scale_y());
    let mut drawn = false;
    for index in 0..count.places {
        let Some(glyph) = count.glyph(shown.combo, index) else {
            continue;
        };
        let (dx, dy, dw, dh) = count.offsets.get(index as usize).copied().unwrap_or_default();
        let at = SkinRect::new(
            rect.x + step * index as f32 - centring + document_length(dx, scale_x),
            rect.y + document_length(dy, scale_y),
            rect.w + document_length(dw, scale_x),
            rect.h + document_length(dh, scale_y),
        );
        drawn |= part.cell(r, shown.strip, shown.set * count.glyphs + glyph, at);
    }
    drawn
}

/// Draws the pop-up, answering whether anything reached the screen: the count first and the word
/// over it, each in its own destination's colour, blend and angle (`SkinJudge.draw`).
///
/// The pop-up's own rectangle places nothing. Its parts are placed by their own destinations, and a
/// part that has faded to nothing still takes its place in the arrangement, so a count that cannot
/// be seen slides the word all the same.
pub(crate) fn draw_judge<R: Renderer>(
    _ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &JudgeBody,
    _rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(shown) = arrange(body, frame) else {
        return false;
    };
    let mut drawn = shown.count.as_ref().is_some_and(|count| draw_count(r, place, count));
    if shown.at.color.a != 0 {
        let cell = shown.strip.animation_index(shown.strip.cells(), frame.now_us, frame.timers, frame.script());
        drawn |= part_placement(shown.word, &shown.at, place).cell(r, shown.strip, cell, shown.at.rect);
    }
    drawn
}
