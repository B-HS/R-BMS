//! The judgement pop-up a play document draws for itself: the word for the judgement the last input
//! took, and the combo that rides beside it.
//!
//! The pop-up is one object made of other objects. The document nests a destination per judgement
//! under `judge.images` and another per judgement under `judge.numbers`, the loader assembles both
//! lists into tracks, and this resolves each of them to the very body an ordinary `image`, `text` or
//! `value` object would have had. What is left is the arrangement the reference makes: the combo is
//! placed against the pop-up's own corner rather than the screen's, and when the document sets
//! `shift` the pop-up slides left by half the combo so the pair stays centred as the count grows.
//!
//! rbms widens one thing and narrows another. The reference draws its judgement words from a bitmap,
//! so `judge.images` may only name an `image`; here it may name a `text` as well, which is what lets
//! a document show the words in a font rather than ship a strip of them. Against that, rbms judges a
//! double chart as one run over two fields rather than as two sides with a judgement each, so a
//! document's second pop-up (`index: 1`) reports the same judgement its first does -- see
//! [`super::state`], which answers both judgement bands from the one judgement the run carries.

use rbms_skin::dst::{DrawStateSource, LuaDrawEval, Resolved, SkinRect, prepare};
use rbms_skin::loader::{LoadedSkin, NamedTrack, StretchKind};
use rbms_skin::model::{Destination, JudgeDef};
use rbms_skin::property::generated::{NUMBER_COMBO, OPTION_1P_PERFECT, OPTION_2P_PERFECT};

use super::draw::Placement;
use super::object::{Body, NumberBody, Places, SkinObject, Source, ValueSource, build_body, integer_glyphs};
use super::{MIN_TEXT_SCALE, SkinAssets, SkinFrame, TEXT_PIXELS_PER_SCALE};
use crate::ctx::RenderCtx;
use crate::{BlendMode, Renderer};

/// How many judgements a run is counted in, best first.
const JUDGEMENTS: usize = 6;

/// How many of them carry a combo. The judgements past these break it, so the reference draws no
/// count beside them.
const COMBO_JUDGEMENTS: usize = 3;

/// The first judgement option of each player, from which the rest follow in order.
const JUDGE_OPTION_BASE: [i32; 2] = [OPTION_1P_PERFECT, OPTION_2P_PERFECT];

/// The whole numbers a property answers with when it has nothing to report, which draw no digits at
/// all (`SkinNumber.prepare`).
const INTEGER_NO_VALUE: [i32; 2] = [i32::MIN, i32::MAX];

/// A text object's `align` that starts the line at its destination's anchor.
const TEXT_ALIGN_LEFT: i32 = 0;

/// A text object's `align` that centres the line on it.
const TEXT_ALIGN_CENTER: i32 = 1;

/// A number's `align` that leaves its places where they fall, which is flush right.
const NUMBER_ALIGN_RIGHT: i32 = 0;

/// A number's `align` that pulls its places over the blanks to their left.
const NUMBER_ALIGN_LEFT: i32 = 1;

/// One pop-up, resolved from one of the document's `judge` objects.
#[derive(Debug)]
pub(crate) struct JudgeBody {
    /// The word each judgement shows, in the order the judgement options are numbered.
    pub(crate) images: Vec<Option<SkinObject>>,
    /// The combo beside it, for the judgements that keep one.
    pub(crate) numbers: Vec<Option<SkinObject>>,
    pub(crate) shift: bool,
    /// Which player's judgement the pop-up follows.
    pub(crate) player: usize,
}

/// Everything resolving one pop-up's nested parts needs from the build.
struct PartBuilder<'a, 'b> {
    skin: &'a LoadedSkin,
    sources: Source<'a>,
    families: &'a [(String, String)],
    assets: &'b mut dyn SkinAssets,
    warnings: &'b mut Vec<String>,
}

impl PartBuilder<'_, '_> {
    /// One draw-list entry per destination the document declared, matched to the track the loader
    /// assembled for it by id so a dropped entry cannot shift the rest out of step.
    fn parts(&mut self, declared: &[Destination], assembled: &[NamedTrack], combo: bool) -> Vec<Option<SkinObject>> {
        declared.iter().map(|entry| self.part(assembled.iter().find(|named| named.id == entry.id)?, combo)).collect()
    }

    /// One entry, or `None` when its id names nothing a pop-up is made of.
    ///
    /// Going through the ordinary resolver is what keeps a pop-up's word animating, tinting and
    /// reading its property exactly as the same object would outside one. The kinds are checked
    /// first because the resolver would otherwise recurse straight back here for a document whose
    /// judge names itself.
    fn part(&mut self, track: &NamedTrack, combo: bool) -> Option<SkinObject> {
        let def = &self.skin.def;
        let id = track.id.as_str();
        let known = if combo {
            def.value.iter().any(|value| value.id == id)
        } else {
            def.image.iter().any(|image| image.id == id) || def.imageset.iter().any(|set| set.id == id) || def.text.iter().any(|text| text.id == id)
        };
        if !known {
            self.warnings.push(format!("judge part {id:?} is not a kind a pop-up is made of"));
            return None;
        }
        let mut body = build_body(self.skin, id, self.sources, self.families, &mut *self.assets, self.warnings)?;
        if let Body::Number(number) = &mut body {
            number.value = ValueSource::Id(NUMBER_COMBO);
        }
        Some(SkinObject { id: track.id.clone(), layer: track.layer, track: track.track.clone(), stretch: StretchKind::from_id(track.track.stretch), body })
    }
}

/// The pop-up behind `id`, or `None` when the document declares no `judge` by that name.
///
/// A judge whose nested destinations the loader has not assembled is dropped with a warning rather
/// than drawn from the document's top-level objects: the nested lists are the only place the
/// per-judgement arrangement exists, and a pop-up without them would put every word in one place.
pub(crate) fn build_judge(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    families: &[(String, String)],
    assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = skin.def.judge.iter().find(|judge| judge.id == id)?;
    let Some(tracks) = skin.nested.judge.get(id) else {
        warnings.push(format!("judge {id:?} has no assembled destinations, so its pop-up is dropped"));
        return None;
    };
    let mut builder = PartBuilder { skin, sources, families, assets, warnings };
    let images = builder.parts(&def.images, &tracks.images, false);
    let numbers = builder.parts(&def.numbers, &tracks.numbers, true);
    if images.iter().all(Option::is_none) {
        warnings.push(format!("judge {id:?} names no word this build could draw"));
        return None;
    }
    let player = player_of(def, id, warnings);
    Some(Body::Judge(JudgeBody { images, numbers, shift: def.shift, player }))
}

/// Which player's judgement a pop-up follows, held to the two rbms answers options for.
///
/// A document that numbers a third side gets the second one with a line about it, because the run
/// has no third judgement to report and a pop-up that quietly never drew would look like a document
/// fault rather than a build limit.
fn player_of(def: &JudgeDef, id: &str, warnings: &mut Vec<String>) -> usize {
    let index = def.index.max(0) as usize;
    if index >= JUDGE_OPTION_BASE.len() {
        warnings.push(format!("judge {id:?} follows player {index}, which this build does not judge separately, so it follows the second side"));
    }
    index.min(JUDGE_OPTION_BASE.len() - 1)
}

/// Which judgement the last input took, or `None` while there has not been one.
fn current_judgement(player: usize, frame: &SkinFrame<'_>) -> Option<usize> {
    let base = JUDGE_OPTION_BASE[player.min(JUDGE_OPTION_BASE.len() - 1)];
    (0..JUDGEMENTS).find(|index| frame.state.boolean(base + *index as i32))
}

/// The same placement over one of the pop-up's own parts, tinted and turned the way that part's
/// destination asked.
fn part_placement<'a>(object: &'a SkinObject, resolved: &Resolved, place: &Placement<'a>) -> Placement<'a> {
    Placement {
        object,
        blend: BlendMode::from_skin_blend(object.track.blend),
        tint: resolved.color.into(),
        angle_deg: resolved.angle_deg,
        viewport: place.viewport,
    }
}

/// Resolves one part's destination for this frame, measured from `origin`.
fn resolve_part(object: &SkinObject, origin: (f32, f32), frame: &SkinFrame<'_>) -> Option<Resolved> {
    let state: &dyn DrawStateSource = frame.state;
    let gate: Option<&dyn LuaDrawEval> = frame.lua.map(|lua| lua as &dyn LuaDrawEval);
    prepare(&object.track, frame.now_ms, frame.timers, state, gate, origin, frame.mouse).filter(|resolved| resolved.color.a != 0)
}

/// Draws the pop-up, answering whether anything reached the screen.
pub(crate) fn draw_judge<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &JudgeBody,
    _rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(index) = current_judgement(body.player, frame) else {
        return false;
    };
    let Some(word) = body.images.get(index).and_then(Option::as_ref) else {
        return false;
    };
    let Some(resolved) = resolve_part(word, (0.0, 0.0), frame) else {
        return false;
    };

    let mut drawn = false;
    let mut slide = 0.0;
    if index < COMBO_JUDGEMENTS
        && let Some(count) = body.numbers.get(index).and_then(Option::as_ref)
        && let Body::Number(number) = &count.body
        && let Some(placed) = resolve_part(count, (resolved.rect.x, resolved.rect.y), frame)
        && let Some(run) = combo_run(number, frame)
    {
        let at = combo_rect(placed.rect, number.digits);
        slide = if body.shift { run.width(at.w + number.space) / 2.0 } else { 0.0 };
        drawn |= draw_combo(r, &part_placement(count, &placed, place), number, at, &run);
    }

    let at = SkinRect::new(resolved.rect.x - slide, resolved.rect.y, resolved.rect.w, resolved.rect.h);
    drawn |= draw_word(ctx, r, &part_placement(word, &resolved, place), &word.body, at, frame);
    drawn
}

/// The rectangle a pop-up's combo is drawn in, pulled left by half the places it reserves.
///
/// The reference makes this correction keyframe by keyframe as it loads the pop-up
/// (`JsonPlaySkinObjectLoader`), and because a keyframe's `x` and its `w` are interpolated at the
/// same rate, making it once to the rectangle they resolved to lands in the same place. What it
/// buys is a count that stays centred on the anchor the document gave it rather than growing off to
/// the right of it.
fn combo_rect(rect: SkinRect, digits: u32) -> SkinRect {
    SkinRect::new(rect.x - rect.w * digits as f32 / 2.0, rect.y, rect.w, rect.h)
}

/// The combo's digits for this frame, and which set of the strip they are read from.
struct ComboRun {
    places: Places,
    set: u32,
    negative: bool,
}

impl ComboRun {
    /// How wide the run is drawn, given the distance between one place and the next.
    fn width(&self, step: f32) -> f32 {
        self.places.as_slice().iter().filter(|slot| slot.is_some()).count() as f32 * step
    }
}

/// Reads the live combo and cuts it into digit places, or reports none when the count has nothing to
/// say.
fn combo_run(body: &NumberBody, frame: &SkinFrame<'_>) -> Option<ComboRun> {
    let value = body.value.integer(frame.state, frame.lua);
    if INTEGER_NO_VALUE.contains(&value) {
        return None;
    }
    let set = body.sprite.animation_index(body.layout.sets, frame.now_ms, frame.timers);
    Some(ComboRun { places: integer_glyphs(body, value), set, negative: value < 0 })
}

/// Draws the combo beside the pop-up, one place at a time.
fn draw_combo<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &NumberBody, rect: SkinRect, run: &ComboRun) -> bool {
    let slots = run.places.as_slice();
    let step = rect.w + body.space;
    let blanks = slots.iter().filter(|slot| slot.is_none()).count();
    let shift = -match body.align {
        NUMBER_ALIGN_RIGHT => 0.0,
        NUMBER_ALIGN_LEFT => step * blanks as f32,
        _ => step * 0.5 * blanks as f32,
    };
    let mut drawn = false;
    for (index, slot) in slots.iter().enumerate() {
        let Some(cell) = slot.and_then(|glyph| body.layout.cell(run.set, run.negative, glyph)) else {
            continue;
        };
        let (dx, dy, dw, dh) = body.offsets.get(index).copied().unwrap_or_default();
        let at = SkinRect::new(rect.x + step * index as f32 + shift + dx, rect.y + dy, rect.w + dw, rect.h + dh);
        drawn |= place.cell(r, &body.sprite, cell, at);
    }
    drawn
}

/// Draws the judgement's word, whether the document cut it from a strip or set it in a font.
fn draw_word<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, place: &Placement<'_>, body: &Body, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    match body {
        Body::Image(image) => {
            let chosen = if image.select.is_named() { image.select.integer(frame.state, frame.lua).max(0) as usize } else { 0 };
            let Some((sprite, first, count)) = image.variants.get(chosen).or_else(|| image.variants.first()) else {
                return false;
            };
            let cell = first + sprite.animation_index(*count, frame.now_ms, frame.timers);
            place.cell(r, sprite, cell, rect)
        }
        Body::Text(text) => {
            let line = match &text.constant {
                Some(constant) => std::borrow::Cow::Borrowed(constant.as_str()),
                None => text.value.text(frame.state, frame.lua),
            };
            if line.is_empty() {
                return false;
            }
            match &text.family {
                Some(family) => ctx.text.set_family(family),
                None => ctx.text.reset_family(),
            }
            let dst = place.viewport.place(rect);
            let scale = (dst.h / TEXT_PIXELS_PER_SCALE).max(MIN_TEXT_SCALE);
            match text.align {
                TEXT_ALIGN_LEFT => ctx.draw_text(r, dst.x, dst.y, scale, place.tint, &line),
                TEXT_ALIGN_CENTER => ctx.draw_text_centered(r, dst.x, dst.y, scale, place.tint, &line),
                _ => ctx.draw_text_right(r, dst.x, dst.y, scale, place.tint, &line),
            }
            true
        }
        _ => false,
    }
}
