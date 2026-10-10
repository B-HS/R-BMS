//! Resolving a document's `songlist` into a wheel, the way the reference's loader does
//! (`JsonSelectSkinObjectLoader`).
//!
//! Each of the wheel's lists looks its ids up in exactly one kind of definition: a bar in the image
//! sets, a lamp, a trophy and a label in the images, a text in the texts, a level in the values and
//! the graph in the graphs. An id of another kind is no part at all, and neither is a destination
//! with no keyframe, which the reference removes before the first frame (`SkinBar.validate`).

use std::cell::{Cell, RefCell};

use rbms_skin::dst::DestinationTrack;
use rbms_skin::loader::{LoadedSkin, NamedTrack, StretchKind};
use rbms_skin::model::{GraphDef, ImageDef, PropertyRef, SkinDef, ValueDef};
use rbms_skin::property::{NameSpace, reference_implements};

use super::bars::{BAR_LABELS, BAR_LAMPS, BAR_LEVELS, BAR_SLOTS, BAR_TEXTS, BAR_TROPHIES, LAMP_KINDS, RANK_KINDS};
use super::{BarArea, BarPart, GraphPart, ImagePart, Kept, LevelPart, PADDING_ALTERNATE_ZERO, Slot, SongListBody, TextPart};
use crate::skin_render::SkinAssets;
use crate::skin_render::draw::ImageSelect;
use crate::skin_render::object::{Body, ImageBody, MAX_PLACES, SkinObject, Sprite, image_sprite};
use crate::skin_render::text::{Fonts, text_body};
use crate::skin_render::textures::Source;

/// The cycle the reference starts a bar set's search with, which the first image found replaces
/// (`int cycle = -1`).
const CYCLE_UNSET: i32 = -1;

/// The ten digits every level strip holds. A strip that is not a whole number of tens holds an
/// alternate zero as well and is cut in elevens.
const DIGITS: u32 = 10;

/// The graph `type` that counts a folder's charts by clear lamp. Any other negative type counts
/// them by score rank.
const GRAPH_BY_LAMP: i32 = -1;

/// Adds `line` unless the same one is already there, so a wheel whose every slot names the same
/// missing part reports it once rather than once per slot.
fn warn_once(warnings: &mut Vec<String>, line: String) {
    if !warnings.contains(&line) {
        warnings.push(line);
    }
}

/// A destination as an object: the track, the stretch it asks for, and what it draws.
fn object_of(track: &DestinationTrack, body: Body) -> SkinObject {
    SkinObject { track: track.clone(), stretch: StretchKind::from_id(track.stretch), body }
}

/// A still or animated image as the body of an object, with nothing to pick between.
fn image_body(sprite: Sprite) -> Body {
    Body::Image(ImageBody { variants: vec![Some((sprite, 0, sprite.cells()))], select: ImageSelect::First })
}

/// Everything resolving a wheel's parts needs from the build.
struct Builder<'a, 'b> {
    def: &'a SkinDef,
    sources: Source<'a>,
    fonts: &'a Fonts,
    warnings: &'b mut Vec<String>,
}

impl Builder<'_, '_> {
    /// The images of the set `id`, one per kind of bar, or `None` when the document declares no
    /// such set or none of its images could be loaded.
    ///
    /// The whole set animates on the timer of the first image that names one and on the cycle of
    /// the first image found, whatever the others say: the reference builds one image object over
    /// all of them.
    fn bar_set(&mut self, id: &str) -> Option<Vec<Option<Sprite>>> {
        let Some(set) = self.def.imageset.iter().find(|set| set.id == id) else {
            warn_once(self.warnings, format!("song bar {id:?} is not an image set, so the slots that name it are not drawn"));
            return None;
        };
        let mut variants: Vec<Option<Sprite>> =
            set.images.iter().map(|name| self.def.image.iter().find(|image| image.id == *name).and_then(|image| image_sprite(image, self.sources))).collect();
        let (mut timer, mut cycle) = (None, CYCLE_UNSET);
        for sprite in variants.iter().flatten() {
            timer = timer.or(sprite.timer);
            if cycle == CYCLE_UNSET {
                cycle = sprite.cycle;
            }
        }
        for sprite in variants.iter_mut().flatten() {
            (sprite.timer, sprite.cycle) = (timer, cycle);
        }
        if variants.iter().all(Option::is_none) {
            warn_once(self.warnings, format!("song bar {id:?} names no image this build could load, so the slots that name it are not drawn"));
            return None;
        }
        Some(variants)
    }

    /// One slot's bar in one of its two states.
    fn bar(&self, named: &NamedTrack, variants: &[Option<Sprite>]) -> Option<BarPart> {
        if named.track.frames.is_empty() {
            return None;
        }
        let body = Body::Image(ImageBody {
            variants: variants.iter().map(|sprite| sprite.map(|sprite| (sprite, 0, sprite.cells()))).collect(),
            select: ImageSelect::First,
        });
        Some(BarPart { object: object_of(&named.track, body), variants: variants.to_vec(), shown: Cell::new(None) })
    }

    /// The first `limit` entries of one of the wheel's lists, each resolved by `resolve` and left as
    /// a hole when it resolves to nothing, so the entries after it keep their number.
    fn list<T>(&mut self, tracks: &[NamedTrack], limit: usize, mut resolve: impl FnMut(&mut Self, &NamedTrack) -> Option<T>) -> Vec<Option<T>> {
        tracks.iter().take(limit).map(|named| if named.track.frames.is_empty() { None } else { resolve(self, named) }).collect()
    }

    /// A lamp, a trophy or a label: the image `named` names.
    fn image(&mut self, named: &NamedTrack) -> Option<ImagePart> {
        let Some(sprite) = self.def.image.iter().find(|image| image.id == named.id).and_then(|image| image_sprite(image, self.sources)) else {
            warn_once(self.warnings, format!("song bar part {:?} names no image this build could load, so it is not drawn", named.id));
            return None;
        };
        Some(ImagePart { object: object_of(&named.track, image_body(sprite)), sprite, kept: Cell::new(Kept::NEVER), cell: Cell::new(0) })
    }

    /// One of the wheel's texts, with a line for each of the `slots` bars it may name at once.
    fn text(&mut self, named: &NamedTrack, slots: usize) -> Option<TextPart> {
        let Some(def) = self.def.text.iter().find(|text| text.id == named.id) else {
            warn_once(self.warnings, format!("song bar text {:?} names no text, so it is not drawn", named.id));
            return None;
        };
        Some(TextPart {
            object: object_of(&named.track, Body::Text(text_body(def, self.fonts))),
            lines: (0..slots).map(|_| text_body(def, self.fonts)).collect(),
            owners: (0..slots).map(|_| Cell::new(None)).collect(),
            kept: Cell::new(Kept::NEVER),
        })
    }

    /// The sprite a definition that is not an image cuts out of its source.
    fn strip(&self, src: &str, region: (i32, i32, i32, i32), divisions: (i32, i32), timer: Option<&PropertyRef>, cycle: i32) -> Option<Sprite> {
        let (x, y, w, h) = region;
        let cut = ImageDef { src: src.to_owned(), x, y, w, h, divx: divisions.0, divy: divisions.1, timer: timer.cloned(), cycle, ..ImageDef::default() };
        image_sprite(&cut, self.sources)
    }

    /// One of the wheel's level numbers: the value `named` names, cut in tens or in elevens.
    ///
    /// The strip never has a negative half and its padding is not the document's to choose: an
    /// eleven-glyph strip pads with its alternate zero and a ten-glyph one leaves its leading places
    /// blank.
    fn level(&mut self, named: &NamedTrack) -> Option<LevelPart> {
        let def: Option<&ValueDef> = self.def.value.iter().find(|value| value.id == named.id);
        let sprite = def.and_then(|def| self.strip(&def.src, (def.x, def.y, def.w, def.h), (def.divx, def.divy), def.timer.as_ref(), def.cycle));
        let (Some(def), Some(sprite)) = (def, sprite) else {
            warn_once(self.warnings, format!("song bar level {:?} names no value this build could load, so it is not drawn", named.id));
            return None;
        };
        let per_set = if sprite.cells().is_multiple_of(DIGITS) { DIGITS } else { DIGITS + 1 };
        let sets = sprite.cells() / per_set;
        if sets == 0 {
            warn_once(self.warnings, format!("song bar level {:?} is cut into fewer cells than the ten digits, so it is not drawn", named.id));
            return None;
        }
        Some(LevelPart {
            object: object_of(&named.track, image_body(sprite)),
            sprite,
            per_set,
            sets,
            digits: usize::try_from(def.digit).unwrap_or_default().min(MAX_PLACES),
            zero_padding: if per_set > DIGITS { PADDING_ALTERNATE_ZERO } else { 0 },
            space: def.space as f32,
            align: def.align,
            own_value: reference_implements(NameSpace::Integer, def.reference).then_some(def.reference),
        })
    }

    /// The wheel's graph: the last graph definition `named` names whose `type` is negative.
    fn graph(&mut self, named: &NamedTrack) -> Option<GraphPart> {
        let def: &GraphDef = self.def.graph.iter().rfind(|graph| graph.id == named.id && graph.graph_type < 0)?;
        let ranks = def.graph_type != GRAPH_BY_LAMP;
        let kinds = if ranks { RANK_KINDS as u32 } else { LAMP_KINDS as u32 };
        let sprite = self.strip(&def.src, (def.x, def.y, def.w, def.h), (def.divx, def.divy), def.timer.as_ref(), def.cycle);
        let Some(sprite) = sprite.filter(|sprite| sprite.cells() >= kinds && !named.track.frames.is_empty()) else {
            warn_once(self.warnings, format!("song bar graph {:?} has no destination or no sheet of {kinds} columns, so it is not drawn", named.id));
            return None;
        };
        Some(GraphPart { track: named.track.clone(), sprite, kinds, frames: sprite.cells() / kinds, ranks, shown: Cell::new(None) })
    }
}

/// The wheel behind `id`, or `None` when the document declares no `songlist` by that name -- or
/// declares one none of whose slots can be drawn.
///
/// A wheel of no slots is refused rather than built empty, because a body of any kind is what a
/// screen counts when it decides whether the document has taken the bar list over: an empty one
/// would hide the built-in browser and then draw nothing in its place.
///
/// A slot is the pair of a `liston` and a `listoff` destination at the same place in their lists,
/// and both are cut from the image set the `liston` one names; the id of the `listoff` one is not
/// read. Slots past the shorter list, and past the sixty the reference draws, are not slots.
pub(crate) fn build_songlist(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    fonts: &Fonts,
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let wheel = skin.def.songlist.as_ref().filter(|list| list.id == id)?;
    let Some(tracks) = skin.nested.songlist.as_ref() else {
        warnings.push(format!("song wheel {id:?} has no assembled slots, so its bars are left to the built-in browser"));
        return None;
    };
    let count = tracks.liston.len().min(tracks.listoff.len()).min(BAR_SLOTS);
    if count == 0 {
        warnings.push(format!("song wheel {id:?} declares no slots, so its bars are left to the built-in browser"));
        return None;
    }
    if tracks.liston.len() != tracks.listoff.len() {
        warnings.push(format!(
            "song wheel {id:?} declares {} selected bars and {} others, so only the first {count} slots are drawn",
            tracks.liston.len(),
            tracks.listoff.len()
        ));
    }

    let mut builder = Builder { def: &skin.def, sources, fonts, warnings };
    let mut slots: Vec<Slot> = Vec::with_capacity(count);
    for (on, off) in tracks.liston.iter().zip(&tracks.listoff).take(count) {
        slots.push(match builder.bar_set(&on.id) {
            Some(variants) => Slot { on: builder.bar(on, &variants), off: builder.bar(off, &variants) },
            None => Slot::default(),
        });
    }
    if slots.iter().all(|slot| slot.on.is_none() && slot.off.is_none()) {
        builder.warnings.push(format!("song wheel {id:?} has no slot it can draw a bar on, so its bars are left to the built-in browser"));
        return None;
    }

    let body = SongListBody {
        center: wheel.center,
        clickable: wheel.clickable.iter().filter_map(|slot| usize::try_from(*slot).ok()).filter(|slot| *slot < count).collect(),
        text: builder.list(&tracks.text, BAR_TEXTS, |builder, named| builder.text(named, count)),
        level: builder.list(&tracks.level, BAR_LEVELS, Builder::level),
        lamp: builder.list(&tracks.lamp, BAR_LAMPS, Builder::image),
        player_lamp: builder.list(&tracks.playerlamp, BAR_LAMPS, Builder::image),
        rival_lamp: builder.list(&tracks.rivallamp, BAR_LAMPS, Builder::image),
        trophy: builder.list(&tracks.trophy, BAR_TROPHIES, Builder::image),
        label: builder.list(&tracks.label, BAR_LABELS, Builder::image),
        graph: tracks.graph.as_ref().and_then(|named| builder.graph(named)),
        areas: RefCell::new(vec![BarArea::EMPTY; count]),
        slots,
    };
    Some(Body::SongList(body))
}
