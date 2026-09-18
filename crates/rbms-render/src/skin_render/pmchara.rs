//! The character a play document's `pmchara` object draws.
//!
//! The object names a `.chp` definition rather than one of the document's own image sources, so its
//! sheets are read and registered apart from them ([`load_library`]) and its frames are cut from the
//! definition's own rectangle table rather than from a grid.
//!
//! Two kinds come out of one definition. A still type shows one rectangle in the destination the
//! document gave it, exactly like an image object. A motion type is an animation per row of the
//! definition, each bound either to one of the built-in character timers -- which is how the running
//! game tells the character what just happened -- or, for a document that asks for one motion by
//! name, to the object's own timer. Every frame of a motion carries its own rectangle inside the
//! definition's [`CharaDef::size`] box, which is mapped into the destination anchored at its top
//! left with the y axis flipped, the way `PomyuCharaLoader` maps it.

use rbms_skin::chp::{self, CharaDef, CharaRect, CharaSheet, IMAGE_SLOT_COUNT, MOTION_COUNT, NO_LOOP, SLOT_CHAR_BMP, SLOT_CHAR_FACE, SLOT_SELECT_CG};
use rbms_skin::dst::{DestinationTrack, SkinRect};
use rbms_skin::loader::LoadedSkin;
use rbms_skin::model::PmChara;
use rbms_skin::timer::{TimerId, TimerState, timer_id};

use super::draw::Placement;
use super::object::Body;
use super::{SkinAssets, SkinFrame};
use crate::{Color, Renderer, TextureId, UvRect};

/// The `type` of an object that plays the whole character: every motion the definition has, each on
/// its own built-in timer.
const TYPE_PLAY: i32 = 0;

/// The first and last `type` that shows one still rectangle.
const TYPE_STILL_FIRST: i32 = 1;
const TYPE_STILL_LAST: i32 = 5;

/// The first and last `type` that plays one named motion on the document's own timer.
const TYPE_MOTION_FIRST: i32 = 6;
const TYPE_MOTION_LAST: i32 = 15;

/// The still types, in the order `PomyuCharaLoader` numbers them.
const TYPE_BACKGROUND: i32 = 1;
const TYPE_NAME: i32 = 2;
const TYPE_FACE_UPPER: i32 = 3;
const TYPE_FACE_ALL: i32 = 4;
const TYPE_SELECT_CG: i32 = 5;

/// Which `#xx` rectangle the background plate and the name plate are cut from.
const RECT_NAME: usize = 0;
const RECT_BACKGROUND: usize = 1;

/// The `color` and `side` a document writes for the second player. Everything else is the first.
const SECOND_PLAYER: i32 = 2;

/// Fully opaque, in the eight-bit units a frame's own alpha and a destination's tint share.
const OPAQUE: u16 = 255;

/// The motion a document's `type` asks for, in the numbering the definition's rows carry.
///
/// `PomyuCharaLoader.getMotionForType`, which has no entry for the play type because a play object
/// takes every motion at once.
const fn motion_for_type(chara_type: i32) -> Option<i32> {
    Some(match chara_type {
        6 => 1,
        7 => 6,
        8 => 7,
        9 => 8,
        10 => 10,
        11 => 17,
        12 => 15,
        13 => 16,
        14 => 3,
        15 => 14,
        _ => return None,
    })
}

/// The motion the definition's rows number the neutral pose, which every side has.
const MOTION_NEUTRAL: i32 = 1;

/// The motions the music-end timer drives: a win, a loss and a win on a full gauge.
const MOTION_WIN: i32 = 15;
const MOTION_LOSE: i32 = 16;
const MOTION_FEVER_WIN: i32 = 17;

/// The motions the first player's own reaction timers drive.
const MOTION_FEVER: i32 = 6;
const MOTION_GREAT: i32 = 7;
const MOTION_GOOD: i32 = 8;
const MOTION_BAD: i32 = 10;

/// How many draw conditions one motion carries, which is how many option ids the reference copies
/// out of a destination.
const MOTION_OPTIONS: usize = 3;

/// The option id that stands for "no condition".
const NO_OPTION: i32 = 0;

/// The timer and the draw conditions one motion of a play object takes, or `None` for a motion the
/// side has no timer for.
///
/// The table is `PomyuCharaLoader.load`'s own: the first player's character reacts to the run
/// directly, and the second player's is the opponent, so the borderline conditions on its win and
/// loss motions are the other way round.
pub(crate) fn play_binding(second_player: bool, motion: i32) -> Option<(TimerId, [i32; MOTION_OPTIONS])> {
    let border = rbms_skin::property::generated::OPTION_1P_BORDER_OR_MORE;
    let full = rbms_skin::property::generated::OPTION_1P_100;
    let plain = [NO_OPTION; MOTION_OPTIONS];
    if second_player {
        return Some(match motion {
            MOTION_NEUTRAL => (timer_id::PM_CHARA_2P_NEUTRAL, plain),
            MOTION_GREAT => (timer_id::PM_CHARA_2P_GREAT, plain),
            MOTION_BAD => (timer_id::PM_CHARA_2P_BAD, plain),
            MOTION_WIN => (timer_id::MUSIC_END, [-border, NO_OPTION, NO_OPTION]),
            MOTION_LOSE => (timer_id::MUSIC_END, [border, NO_OPTION, NO_OPTION]),
            _ => return None,
        });
    }
    Some(match motion {
        MOTION_NEUTRAL => (timer_id::PM_CHARA_1P_NEUTRAL, plain),
        MOTION_FEVER => (timer_id::PM_CHARA_1P_FEVER, plain),
        MOTION_GREAT => (timer_id::PM_CHARA_1P_GREAT, plain),
        MOTION_GOOD => (timer_id::PM_CHARA_1P_GOOD, plain),
        MOTION_BAD => (timer_id::PM_CHARA_1P_BAD, plain),
        MOTION_WIN => (timer_id::MUSIC_END, [border, -full, NO_OPTION]),
        MOTION_LOSE => (timer_id::MUSIC_END, [-border, NO_OPTION, NO_OPTION]),
        MOTION_FEVER_WIN => (timer_id::MUSIC_END, [full, NO_OPTION, NO_OPTION]),
        _ => return None,
    })
}

/// Which side a reaction timer belongs to, or `None` for a timer that is not one.
///
/// A reaction plays once and then gives the screen back to the neutral pose. The reference does that
/// by switching the neutral timer off while a reaction runs and back on when it ends; the timers
/// here stay as the play screen set them and the same rule is applied when the frame is chosen,
/// which keeps the play screen from needing to know how long a definition's motions are.
fn reaction_side(timer: TimerId) -> Option<usize> {
    match timer {
        timer_id::PM_CHARA_1P_FEVER | timer_id::PM_CHARA_1P_GREAT | timer_id::PM_CHARA_1P_GOOD | timer_id::PM_CHARA_1P_BAD => Some(0),
        timer_id::PM_CHARA_2P_GREAT | timer_id::PM_CHARA_2P_BAD => Some(1),
        _ => None,
    }
}

/// Which side a neutral timer belongs to, or `None` for a timer that is not one.
fn neutral_side(timer: TimerId) -> Option<usize> {
    match timer {
        timer_id::PM_CHARA_1P_NEUTRAL => Some(0),
        timer_id::PM_CHARA_2P_NEUTRAL => Some(1),
        _ => None,
    }
}

/// One definition, with the sheets it names already registered.
#[derive(Debug)]
pub(crate) struct LoadedChara {
    def: CharaDef,
    images: [Option<(TextureId, (u32, u32))>; IMAGE_SLOT_COUNT],
}

/// Every definition one document's `pmchara` objects read, keyed by the `src` that named it.
#[derive(Debug, Default)]
pub(crate) struct CharaLibrary {
    entries: Vec<(String, LoadedChara)>,
}

impl CharaLibrary {
    /// The definition one `src` resolved to.
    fn get(&self, src: &str) -> Option<&LoadedChara> {
        self.entries.iter().find(|(name, _)| name == src).map(|(_, chara)| chara)
    }
}

/// Reads every definition the document's `pmchara` objects name and registers their sheets.
///
/// `serial` distinguishes one loaded document's textures from another's, the same way the
/// document's own image sources are distinguished, and every texture registered here is pushed onto
/// `textures` so the screen hands it back when it is released.
pub(crate) fn load_library<R: Renderer>(
    r: &mut R,
    skin: &LoadedSkin,
    assets: &mut dyn SkinAssets,
    serial: u32,
    textures: &mut Vec<TextureId>,
    warnings: &mut Vec<String>,
) -> CharaLibrary {
    let mut library = CharaLibrary::default();
    for object in &skin.def.pmchara {
        if library.get(&object.src).is_some() {
            continue;
        }
        let Some(path) = chp::chara_source_path(skin, &object.src) else {
            warnings.push(format!("character {:?} names the unreadable source {:?}", object.id, object.src));
            continue;
        };
        let def = match chp::read_chara(&skin.root, &path) {
            Ok(def) => def,
            Err(error) => {
                warnings.push(format!("character {:?} was skipped: {error}", object.id));
                continue;
            }
        };
        let index = library.entries.len();
        let mut images: [Option<(TextureId, (u32, u32))>; IMAGE_SLOT_COUNT] = Default::default();
        for (slot, file) in def.images.iter().enumerate() {
            let Some(file) = file else {
                continue;
            };
            let Some(image) = assets.image(file) else {
                warnings.push(format!("character {:?} could not decode {}", object.id, file.display()));
                continue;
            };
            let key = format!("rbms.skin.{serial}.chara.{index}.{slot}");
            let tex = r.register_texture(&key, &image.rgba, image.width, image.height);
            textures.push(tex);
            images[slot] = Some((tex, (image.width, image.height)));
        }
        library.entries.push((object.src.clone(), LoadedChara { def, images }));
    }
    library
}

/// One frame of one motion, resolved against the sheet it is cut from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CharaMotionFrame {
    /// The sheet rectangle, or `None` for a frame the definition sized at nothing, which draws
    /// nothing at all.
    pub(crate) src: Option<UvRect>,
    pub(crate) source: (f32, f32),
    pub(crate) destination: CharaRect,
    pub(crate) alpha: u8,
    pub(crate) angle_deg: f32,
}

/// One animation row, bound to the timer that plays it.
#[derive(Debug)]
pub(crate) struct CharaMotion {
    pub(crate) tex: TextureId,
    /// The timer the animation is measured from, or `None` to measure from the frame clock.
    pub(crate) timer: Option<TimerId>,
    pub(crate) options: [i32; MOTION_OPTIONS],
    pub(crate) frames: Vec<CharaMotionFrame>,
    pub(crate) frame_ms: i64,
    /// How many frames play once before the rest repeat.
    pub(crate) once: usize,
    /// The box every frame's destination is stated in.
    pub(crate) size: (f32, f32),
}

impl CharaMotion {
    /// Milliseconds one pass over every frame takes.
    fn cycle_ms(&self) -> i64 {
        self.frame_ms * self.frames.len() as i64
    }

    /// Which frame the animation is on `elapsed` milliseconds after its timer switched on.
    ///
    /// The frames up to the loop point play once and the rest repeat, which is the two-part split
    /// `PomyuCharaLoader` emits as two objects.
    pub(crate) fn frame_at(&self, elapsed: i64) -> usize {
        let count = self.frames.len();
        if count == 0 || self.frame_ms <= 0 {
            return 0;
        }
        let once = self.once.min(count);
        let played_once = self.frame_ms * once as i64;
        if elapsed < played_once {
            return (elapsed / self.frame_ms) as usize;
        }
        if count == once {
            return count - 1;
        }
        once + (((elapsed - played_once) / self.frame_ms) as usize % (count - once))
    }
}

/// What a character object draws.
#[derive(Debug)]
pub(crate) enum CharaKind {
    /// One rectangle of one sheet, in the destination the document gave the object.
    Still { tex: TextureId, src: UvRect, source: (f32, f32) },
    /// Every animation row the object plays, in the definition's own draw order.
    Motion(Vec<CharaMotion>),
}

/// A character object's body.
#[derive(Debug)]
pub(crate) struct PmCharaBody {
    pub(crate) kind: CharaKind,
}

/// The sheet rectangle of one still type, or `None` when the definition has no sheet for it.
fn still_frame(chara: &LoadedChara, chara_type: i32, second_player: bool) -> Option<(TextureId, (u32, u32), CharaRect)> {
    let coloured = |base: usize| chara.images[base + usize::from(second_player)].or(chara.images[base]);
    let (tex, size) = match chara_type {
        TYPE_BACKGROUND | TYPE_NAME => coloured(SLOT_CHAR_BMP)?,
        TYPE_FACE_UPPER | TYPE_FACE_ALL => coloured(SLOT_CHAR_FACE)?,
        TYPE_SELECT_CG => coloured(SLOT_SELECT_CG)?,
        _ => return None,
    };
    let rect = match chara_type {
        TYPE_BACKGROUND => chara.def.rect(RECT_BACKGROUND),
        TYPE_NAME => chara.def.rect(RECT_NAME),
        TYPE_FACE_UPPER => chara.def.face_upper,
        TYPE_FACE_ALL => chara.def.face_all,
        _ => CharaRect { x: 0, y: 0, w: size.0 as i32, h: size.1 as i32 },
    };
    Some((tex, size, rect))
}

/// The texture region one sheet rectangle names, or `None` when it has no extent.
fn region_of(rect: CharaRect, size: (u32, u32)) -> Option<UvRect> {
    (rect.w > 0 && rect.h > 0).then(|| UvRect::from_pixels(rect.x.max(0) as u32, rect.y.max(0) as u32, rect.w as u32, rect.h as u32, size.0, size.1))
}

/// Builds one animation row into a motion.
fn build_motion(
    chara: &LoadedChara,
    sheet: CharaSheet,
    motion: i32,
    frames: &[chp::CharaFrame],
    second_player: bool,
    timer: Option<TimerId>,
    options: [i32; MOTION_OPTIONS],
) -> Option<CharaMotion> {
    let slot = sheet.slot(second_player);
    let (tex, size) = chara.images[slot].or(chara.images[sheet.slot(false)])?;
    let count = frames.len();
    let index = usize::try_from(motion).ok().filter(|motion| *motion < MOTION_COUNT)?;
    let mut loop_frame = chara.def.loop_frame[index];
    let last = count as i32 - 1;
    if loop_frame >= last {
        loop_frame = last - 1;
    } else if loop_frame < NO_LOOP {
        loop_frame = NO_LOOP;
    }
    let built = frames
        .iter()
        .map(|frame| {
            let rect = chara.def.rect(frame.source);
            CharaMotionFrame {
                src: region_of(rect, size),
                source: (rect.w.max(0) as f32, rect.h.max(0) as f32),
                destination: frame.destination,
                alpha: frame.alpha,
                angle_deg: frame.angle_deg as f32,
            }
        })
        .collect();
    Some(CharaMotion {
        tex,
        timer,
        options,
        frames: built,
        frame_ms: i64::from(chara.def.frame_ms[index]),
        once: (loop_frame + 1).max(0) as usize,
        size: (chara.def.size.0 as f32, chara.def.size.1 as f32),
    })
}

/// The body one `pmchara` object draws, or `None` when the id names no character.
pub(crate) fn build_pmchara(skin: &LoadedSkin, id: &str, charas: &CharaLibrary, track: &DestinationTrack, warnings: &mut Vec<String>) -> Option<Body> {
    let object: &PmChara = skin.def.pmchara.iter().find(|object| object.id == id)?;
    let Some(chara) = charas.get(&object.src) else {
        warnings.push(format!("character {id:?} has no definition at {:?}", object.src));
        return None;
    };
    let Some(chara_type) = object.chara_type else {
        warnings.push(format!("character {id:?} names no type"));
        return None;
    };
    let second_player = chara.def.second_player_colour(object.color == SECOND_PLAYER);
    if (TYPE_STILL_FIRST..=TYPE_STILL_LAST).contains(&chara_type) {
        let Some((tex, size, rect)) = still_frame(chara, chara_type, second_player) else {
            warnings.push(format!("character {id:?} has no sheet for type {chara_type}"));
            return None;
        };
        let Some(src) = region_of(rect, size) else {
            warnings.push(format!("character {id:?} has an empty rectangle for type {chara_type}"));
            return None;
        };
        return Some(Body::PmChara(PmCharaBody { kind: CharaKind::Still { tex, src, source: (rect.w as f32, rect.h as f32) } }));
    }
    if chara_type != TYPE_PLAY && !(TYPE_MOTION_FIRST..=TYPE_MOTION_LAST).contains(&chara_type) {
        warnings.push(format!("character {id:?} names the unknown type {chara_type}"));
        return None;
    }
    let side = object.side == SECOND_PLAYER;
    let wanted = motion_for_type(chara_type);
    let motions: Vec<CharaMotion> = chara
        .def
        .rows
        .iter()
        .filter_map(|row| {
            let (timer, options) = match wanted {
                Some(wanted) if wanted == row.motion => (track.timer, [NO_OPTION; MOTION_OPTIONS]),
                Some(_) => return None,
                None => play_binding(side, row.motion).map(|(timer, options)| (Some(timer), options))?,
            };
            build_motion(chara, row.sheet, row.motion, &row.frames, second_player, timer, options)
        })
        .collect();
    if motions.is_empty() {
        warnings.push(format!("character {id:?} has no animation row for type {chara_type}"));
        return None;
    }
    Some(Body::PmChara(PmCharaBody { kind: CharaKind::Motion(motions) }))
}

/// Which sides have a reaction motion running this frame, so their neutral pose stands aside.
fn reacting(motions: &[CharaMotion], now_ms: i64, timers: &TimerState) -> [bool; 2] {
    let mut sides = [false; 2];
    for motion in motions {
        let Some((timer, side)) = motion.timer.and_then(|timer| Some((timer, reaction_side(timer)?))) else {
            continue;
        };
        if timers.elapsed(timer, now_ms).is_some_and(|elapsed| (0..motion.cycle_ms()).contains(&elapsed)) {
            sides[side] = true;
        }
    }
    sides
}

/// Whether one motion draws this frame, and how far into it the animation is.
fn motion_elapsed(motion: &CharaMotion, frame: &SkinFrame<'_>, reacting: [bool; 2]) -> Option<i64> {
    if motion.options.iter().any(|option| *option != NO_OPTION && !frame.state.boolean(*option)) {
        return None;
    }
    let Some(timer) = motion.timer else {
        return Some(frame.now_ms);
    };
    let elapsed = frame.timers.elapsed(timer, frame.now_ms).filter(|elapsed| *elapsed >= 0)?;
    if reaction_side(timer).is_some() && elapsed >= motion.cycle_ms() {
        return None;
    }
    if neutral_side(timer).is_some_and(|side| reacting[side]) {
        return None;
    }
    Some(elapsed)
}

/// Where one frame of a motion lands, in the document's own coordinates.
///
/// The definition states a frame inside its own size box, measured down from the box's head; the
/// destination is measured up from the document's foot. So the box is anchored at the destination's
/// top left and the y axis is turned over, which is `PomyuCharaLoader`'s own mapping.
fn placed(frame: &CharaMotionFrame, motion: &CharaMotion, rect: SkinRect) -> Option<SkinRect> {
    let (size_w, size_h) = motion.size;
    if size_w <= 0.0 || size_h <= 0.0 {
        return None;
    }
    let destination = frame.destination;
    Some(SkinRect::new(
        rect.x + destination.x as f32 * rect.w / size_w,
        rect.y + rect.h - (destination.y + destination.h) as f32 * rect.h / size_h,
        destination.w as f32 * rect.w / size_w,
        destination.h as f32 * rect.h / size_h,
    ))
}

/// Draws one character, answering whether anything reached the screen.
pub(crate) fn draw_pmchara<R: Renderer>(r: &mut R, place: &Placement<'_>, body: &PmCharaBody, rect: SkinRect, frame: &SkinFrame<'_>) -> bool {
    match &body.kind {
        CharaKind::Still { tex, src, source } => place.region(r, *tex, *src, *source, rect),
        CharaKind::Motion(motions) => {
            let reacting = reacting(motions, frame.now_ms, frame.timers);
            let mut drawn = false;
            for motion in motions {
                let Some(elapsed) = motion_elapsed(motion, frame, reacting) else {
                    continue;
                };
                let Some(cell) = motion.frames.get(motion.frame_at(elapsed)) else {
                    continue;
                };
                let (Some(src), Some(at)) = (cell.src, placed(cell, motion, rect)) else {
                    continue;
                };
                let tint = Color { a: (u16::from(place.tint.a) * u16::from(cell.alpha) / OPAQUE) as u8, ..place.tint };
                let framed =
                    Placement { object: place.object, blend: place.blend, tint, angle_deg: place.angle_deg + cell.angle_deg, viewport: place.viewport };
                drawn |= framed.region(r, motion.tex, src, cell.source, at);
            }
            drawn
        }
    }
}
