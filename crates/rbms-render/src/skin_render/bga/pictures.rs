//! Putting a chart's decoded background pictures onto the renderer the way the reference draws them.
//!
//! The reference does three things to a picture between the file and the screen, and each is a
//! property of the pixels rather than of where they land, so they are done here, once, before a
//! picture is uploaded:
//!
//! - A picture no longer than 256 pixels on its longer side is put on a 256 x 256 canvas of
//!   transparent black, centred across and flush with the top (`BGImageProcessor.convert`). Fitted
//!   into a rectangle afterwards, it keeps that square's shape and the empty space around it. A
//!   picture exactly 256 x 256 is that canvas already, to the byte, so it is not copied onto one.
//! - A layer is drawn through a shader that turns every pixel whose red, green and blue are all
//!   zero into a transparent one (`layer.frag`). Doing that to the texels is the same thing as long
//!   as the layer is sampled unfiltered, which it is.
//! - The chart's own picture and the miss layer are drawn as they are.
//!
//! A layer and a picture of the same number are different textures for that reason: a picture has
//! two looks ([`BgaLook`]), and each of the three roles keeps a texture of its own.
//!
//! A chart's background can change pictures many times a second, and painting a large picture and
//! handing it to the renderer costs most of a millisecond, which a frame that judges keys cannot
//! spend. So that work is done before the chart plays, as far as memory allows:
//!
//! - Before the chart plays, [`BgaTextures::preload`] uploads every picture the chart shows, in each
//!   look it is shown in, as a texture of its own, until [`RESIDENT_BYTES`] are held. A frame that
//!   shows one of those hands the renderer nothing.
//! - The looks past that allowance have the pixels they hand the renderer painted when the picture
//!   is decoded, on the decode's own worker ([`BgaPrepared`]), so the frame that shows one only
//!   uploads it.
//!
//! The two are one plan, made before any picture is decoded from the pictures' sizes as their files
//! give them ([`uploaded_ahead`]): a look that is to be uploaded ahead is not painted on the worker,
//! since the texture shows it from then on and holding its pixels twice is memory spent on nothing,
//! and the room painting has goes to the looks that are not. The upload ahead paints those looks
//! itself, before the chart plays.
//!
//! The plan is the whole chart's. A part of it played on its own, such as a practice slice, may
//! show only looks the plan left out because the chart's earlier ones filled the allowance, so its
//! upload ahead takes its own order ([`BgaPreloadOrder::Part`]): the looks the plan names first,
//! since nothing was painted for them, then the rest while there is room.
//!
//! A picture past both allowances is painted and uploaded the frame it is first shown in, into the
//! canvas and the texture its role keeps, which asks the allocator for nothing. The pixels are the
//! same whichever way they got there.

use std::collections::HashMap;

use super::{BgaExpand, BgaFrame, BgaPick, BgaPlayhead, BgaShow};
use crate::{Renderer, TextureId};

/// The longest side, in pixels, of a picture that is put on a canvas of its own
/// (`BGImageProcessor`'s `bgasize <= 256`), and the canvas's side.
pub const SMALL_PICTURE_EDGE: u32 = 256;

/// Bytes in a mebibyte, the unit the allowances are given in.
const MEBIBYTE: usize = 1024 * 1024;

/// Most bytes of texture the pictures uploaded ahead of play hold between them: 64 pictures of
/// 1024 x 1024, or a thousand of 256 x 256, which is more than a chart's background is in all but
/// the rarest charts. A picture past it is uploaded each time it is shown, as every picture once
/// was.
pub const RESIDENT_BYTES: usize = 256 * MEBIBYTE;

/// The registry key of the chart's own picture.
const BASE_KEY: &str = "rbms.skin.bga.base";

/// The registry key of the layer over it.
const LAYER_KEY: &str = "rbms.skin.bga.layer";

/// The registry key of the miss layer.
const MISS_KEY: &str = "rbms.skin.bga.miss";

/// The start of the registry key of a picture uploaded ahead of play. Its look and the number of
/// its decode follow, which makes the key its own.
const RESIDENT_KEY: &str = "rbms.skin.bga.resident";

/// Bytes in a pixel of RGBA8.
const RGBA_BYTES: usize = 4;

/// The byte of a pixel that is its alpha.
const ALPHA_INDEX: usize = 3;

/// How a role shows a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BgaLook {
    /// As it is: the chart's own picture and the miss layer.
    Plain,
    /// With every pure black pixel turned transparent: the layer (`layer.frag`).
    Keyed,
}

/// One decoded picture as the game holds it.
#[derive(Debug, Clone, Copy)]
pub struct BgaPicture<'a> {
    /// This decode's own number: two pictures with the same one are the same pixels, which is what
    /// lets a picture that is shown for many frames be uploaded once.
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    /// `width * height` pixels of RGBA8, not premultiplied.
    pub rgba: &'a [u8],
    /// The pixels each look hands the renderer, painted when the picture was decoded, or `None`
    /// for a picture nothing was painted ahead for.
    pub prepared: Option<&'a BgaPrepared>,
}

impl BgaPicture<'_> {
    /// Whether the pixels are as many as the size says.
    fn is_whole(&self) -> bool {
        self.width > 0 && self.height > 0 && self.rgba.len() == self.width as usize * self.height as usize * RGBA_BYTES
    }

    /// Whether the picture is put on a 256 x 256 canvas of its own.
    fn is_small(&self) -> bool {
        self.width.max(self.height) <= SMALL_PICTURE_EDGE
    }

    /// Whether the picture as it is takes painting to hand over: a small picture that is not already
    /// the size of its canvas. One exactly 256 x 256 lands across nothing and covers every row, so
    /// its canvas is its own pixels.
    fn needs_canvas(&self) -> bool {
        self.is_small() && (self.width, self.height) != (SMALL_PICTURE_EDGE, SMALL_PICTURE_EDGE)
    }

    /// The size of the texture either look of the picture is: the 256 x 256 canvas for a small
    /// picture, and the picture's own size otherwise.
    fn texture_size(&self) -> (u32, u32) {
        texture_size_of(self.width, self.height)
    }

    /// Bytes either look of the picture hands the renderer.
    pub fn texture_bytes(&self) -> usize {
        texture_bytes_of(self.width, self.height)
    }

    /// The pixels `look` hands the renderer when nothing has to be painted for them now: the ones
    /// painted ahead, or the picture's own when the look is the picture as it is. `None` when they
    /// still have to be painted.
    fn ready(&self, look: BgaLook) -> Option<&[u8]> {
        let painted = self.prepared.and_then(|prepared| match look {
            BgaLook::Plain => prepared.plain.as_deref(),
            BgaLook::Keyed => prepared.keyed.as_deref(),
        });
        if let Some(pixels) = painted.filter(|pixels| pixels.len() == self.texture_bytes()) {
            return Some(pixels);
        }
        (look == BgaLook::Plain && !self.needs_canvas()).then_some(self.rgba)
    }
}

/// The size of the texture either look of a `width` x `height` picture is: the 256 x 256 canvas for
/// a small picture, and the picture's own size otherwise.
fn texture_size_of(width: u32, height: u32) -> (u32, u32) {
    if width.max(height) <= SMALL_PICTURE_EDGE { (SMALL_PICTURE_EDGE, SMALL_PICTURE_EDGE) } else { (width, height) }
}

/// Bytes either look of a `width` x `height` picture hands the renderer, which is what holding it as
/// a texture costs: known from the picture's size alone, before its pixels are.
pub fn texture_bytes_of(width: u32, height: u32) -> usize {
    let (width, height) = texture_size_of(width, height);
    width as usize * height as usize * RGBA_BYTES
}

/// Which of a chart's looks [`BgaTextures::preload`] holds as textures, given what each costs
/// ([`texture_bytes_of`]) in the order the chart first shows them ([`BgaPlayhead::first_looks`]),
/// or `None` for one whose size is not known: each that fits in what `allowance` has left after
/// the ones before it. A look that does not fit leaves the room to the ones after it.
pub fn uploaded_ahead(costs: impl IntoIterator<Item = Option<usize>>, allowance: usize) -> Vec<bool> {
    let mut held = 0;
    costs
        .into_iter()
        .map(|cost| {
            let fits = cost.is_some_and(|cost| held + cost <= allowance);
            held += cost.filter(|_| fits).unwrap_or(0);
            fits
        })
        .collect()
}

/// A picture's pixels as each look hands them to the renderer, painted once, off the frame thread,
/// when the picture is decoded. A look that is the picture as it is, that is uploaded ahead of play,
/// or that nothing painted, is `None`, and whatever shows it paints it then.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BgaPrepared {
    /// The plain look of a small picture smaller than its canvas: the picture on its 256 x 256
    /// canvas.
    pub plain: Option<Vec<u8>>,
    /// The keyed look: on its canvas when the picture is small, with its black keyed out.
    pub keyed: Option<Vec<u8>>,
    /// The looks of the picture the chart's plan holds as textures from before the chart plays
    /// ([`uploaded_ahead`]), which were not painted: [`BgaTextures::preload`] uploads these and no
    /// other look of the picture for the chart the plan was made for, and these first for a part of
    /// it ([`BgaPreloadOrder::Part`]). `None` when no plan was made, and the upload ahead keeps to its
    /// allowance in the order the chart shows its pictures, as far as they are decoded.
    pub ahead: Option<Vec<BgaLook>>,
}

impl BgaPrepared {
    /// The pixels `look` hands the renderer for `picture`, painted now: `None` when they are the
    /// picture's own and nothing need be painted, or when the picture's pixels are not as many as
    /// its size says.
    pub fn paint(picture: &BgaPicture<'_>, look: BgaLook) -> Option<Vec<u8>> {
        if !picture.is_whole() || (look == BgaLook::Plain && !picture.needs_canvas()) {
            return None;
        }
        let mut canvas = Vec::new();
        paint(picture, look, &mut canvas);
        Some(canvas)
    }

    /// Bytes the painted pixels hold.
    pub fn bytes(&self) -> usize {
        [&self.plain, &self.keyed].into_iter().flatten().map(Vec::len).sum()
    }
}

/// Bytes a picture's 256 x 256 canvas holds.
const SMALL_CANVAS_BYTES: usize = (SMALL_PICTURE_EDGE * SMALL_PICTURE_EDGE) as usize * RGBA_BYTES;

/// The picture on its 256 x 256 canvas when it is small enough to be put on one, as the pixels and
/// the canvas's size; `None` for a picture that is drawn as it is (`BGImageProcessor.convert`).
///
/// The picture lands `(256 - width) / 2` pixels across, rounded down, and at the top. Everything
/// the picture does not cover is transparent black.
pub fn on_small_canvas(picture: &BgaPicture<'_>) -> Option<(Vec<u8>, (u32, u32))> {
    let mut canvas = Vec::new();
    let size = put_on_small_canvas(picture, &mut canvas)?;
    Some((canvas, size))
}

/// [`on_small_canvas`] painted into `canvas`, whatever it held, which keeps its allocation: the
/// canvas's size, or `None`, and `canvas` untouched, for a picture that is drawn as it is.
fn put_on_small_canvas(picture: &BgaPicture<'_>, canvas: &mut Vec<u8>) -> Option<(u32, u32)> {
    if !picture.is_whole() || !picture.is_small() {
        return None;
    }
    let side = SMALL_PICTURE_EDGE as usize;
    let (width, height) = (picture.width as usize, picture.height as usize);
    let across = (side - width) / 2;
    canvas.clear();
    canvas.resize(SMALL_CANVAS_BYTES, 0);
    for row in 0..height {
        let from = row * width * RGBA_BYTES;
        let to = (row * side + across) * RGBA_BYTES;
        canvas[to..to + width * RGBA_BYTES].copy_from_slice(&picture.rgba[from..from + width * RGBA_BYTES]);
    }
    Some((SMALL_PICTURE_EDGE, SMALL_PICTURE_EDGE))
}

/// Paints `look` of a whole `picture` into `canvas`, whatever it held, keeping its allocation: on its
/// 256 x 256 canvas when it is small, as it is otherwise, and keyed out for the keyed look.
fn paint(picture: &BgaPicture<'_>, look: BgaLook, canvas: &mut Vec<u8>) {
    if put_on_small_canvas(picture, canvas).is_none() {
        canvas.clear();
        canvas.extend_from_slice(picture.rgba);
    }
    if look == BgaLook::Keyed {
        key_out_black(canvas);
    }
}

/// Turns every pixel whose red, green and blue are all zero fully transparent (`layer.frag`),
/// whatever alpha it had. The colour stays zero.
pub fn key_out_black(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(RGBA_BYTES) {
        if pixel[..ALPHA_INDEX].iter().all(|channel| *channel == 0) {
            pixel[ALPHA_INDEX] = 0;
        }
    }
}

/// What a texture slot holds: whose pixels, at what size, under which handle.
#[derive(Debug, Clone, Copy)]
struct Held {
    generation: u64,
    size: (u32, u32),
    tex: TextureId,
}

/// The texture of one role, uploaded again only when the picture it shows changes.
#[derive(Debug)]
struct Slot {
    held: Option<Held>,
    /// The canvas the role paints a picture into before it is uploaded, when nothing painted it
    /// ahead: a small picture on its 256 x 256 canvas, or a layer with its black keyed out. Kept
    /// from one picture to the next.
    canvas: Vec<u8>,
}

impl Default for Slot {
    fn default() -> Slot {
        Slot { held: None, canvas: Vec::with_capacity(SMALL_CANVAS_BYTES) }
    }
}

impl Slot {
    /// The texture that shows `look` of a whole `picture`, uploading it first unless it is the one
    /// already there. `None` when the renderer cannot hold it.
    fn show<R: Renderer>(&mut self, r: &mut R, key: &str, picture: BgaPicture<'_>, look: BgaLook) -> Option<TextureId> {
        let size = (picture.width, picture.height);
        if let Some(held) = self.held.filter(|held| held.generation == picture.generation && held.size == size && r.texture_size(held.tex).is_some()) {
            return Some(held.tex);
        }
        let (width, height) = picture.texture_size();
        let rgba = match picture.ready(look) {
            Some(pixels) => pixels,
            None => {
                paint(&picture, look, &mut self.canvas);
                &self.canvas[..]
            }
        };
        let tex = r.register_texture(key, rgba, width, height);
        self.held = Some(Held { generation: picture.generation, size, tex });
        r.texture_size(tex).map(|_| tex)
    }

    fn release<R: Renderer>(&mut self, r: &mut R) {
        if let Some(held) = self.held.take() {
            r.release_texture(held.tex);
        }
    }
}

/// Which order [`BgaTextures::preload`] takes a chart's looks in, which is a question of whether the
/// playhead it is handed is the chart the decodes planned for ([`BgaPrepared::ahead`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BgaPreloadOrder {
    /// The plan's: the playhead is the chart the plan was made for, so the looks it names are
    /// uploaded and no others, whichever order the pictures are decoded in.
    #[default]
    Planned,
    /// The playhead's own: it shows a part of the chart the plan was made for, such as a practice
    /// slice, whose looks the plan may have left out only because others of the chart came before
    /// them. The looks of the part the plan names come first, since nothing was painted for them,
    /// and the rest follow in the order the part shows them while room is left.
    Part,
}

/// The textures a chart's background draws from: its own picture, the layer over it and the miss
/// layer, and the pictures uploaded ahead of play.
#[derive(Debug, Default)]
pub struct BgaTextures {
    base: Slot,
    layer: Slot,
    miss: Slot,
    /// The pictures uploaded ahead of play, by the number of their decode and the look they were
    /// uploaded in. A picture the renderer would not hold is here too, so it is not tried again.
    resident: HashMap<(u64, BgaLook), TextureId>,
    /// Bytes of texture the pictures in `resident` hold.
    resident_bytes: usize,
    /// What [`BgaTextures::preload`] has still to see to: each picture the chart shows, once for
    /// each look, that has not been uploaded ahead or found past the allowance. Made on the first
    /// call, so the frames after it walk the chart's pictures rather than all its events.
    pending: Option<Vec<(i32, BgaLook)>>,
    /// The order `pending` is made in.
    order: BgaPreloadOrder,
}

/// Whether the chart's plan, as `picture`'s decode carries it, names `look`: `None` for a decode
/// that carries no plan.
fn planned(picture: &BgaPicture<'_>, look: BgaLook) -> Option<bool> {
    picture.prepared.and_then(|prepared| prepared.ahead.as_ref()).map(|ahead| ahead.contains(&look))
}

/// The texture that shows `look` of `picture`: the one uploaded ahead of play when there is one, and
/// otherwise `slot`'s, uploaded now unless it already shows the picture.
fn shown<R: Renderer>(
    resident: &HashMap<(u64, BgaLook), TextureId>,
    slot: &mut Slot,
    r: &mut R,
    key: &str,
    picture: Option<BgaPicture<'_>>,
    look: BgaLook,
) -> Option<TextureId> {
    let picture = picture.filter(BgaPicture::is_whole)?;
    match resident.get(&(picture.generation, look)).copied().filter(|tex| r.texture_size(*tex).is_some()) {
        Some(tex) => Some(tex),
        None => slot.show(r, key, picture, look),
    }
}

impl BgaTextures {
    /// The frame that shows what `pick` says. `picture` finds the decoded picture of a picture
    /// number; a number it has none for shows as a picture the chart does not have.
    pub fn frame<'a, R: Renderer>(&mut self, r: &mut R, pick: BgaPick, expand: BgaExpand, picture: impl Fn(i32) -> Option<BgaPicture<'a>>) -> BgaFrame {
        let resident = &self.resident;
        let show = match pick {
            BgaPick::Blank => BgaShow::Blank,
            BgaPick::Miss(number) => BgaShow::Miss { image: shown(resident, &mut self.miss, r, MISS_KEY, number.and_then(&picture), BgaLook::Plain) },
            BgaPick::Playing { base, layer } => BgaShow::Playing {
                base: shown(resident, &mut self.base, r, BASE_KEY, base.and_then(&picture), BgaLook::Plain),
                layer: shown(resident, &mut self.layer, r, LAYER_KEY, layer.and_then(&picture), BgaLook::Keyed),
            },
        };
        BgaFrame { show, expand }
    }

    /// Uploads, ahead of play, the pictures `playhead`'s chart shows, each in every look the chart
    /// shows it in, as a texture of its own: a frame that later shows one of them hands the renderer
    /// nothing. Pictures are taken in the order the chart first shows them, until they hold
    /// [`RESIDENT_BYTES`] between them; one that would pass that is left to be uploaded when it is
    /// shown.
    ///
    /// A picture whose decode carries the chart's plan ([`BgaPrepared::ahead`]) is uploaded in the
    /// looks the plan names and no other, which keeps the textures to the looks nothing was painted
    /// for whichever order the pictures are decoded in. That holds for the chart the plan was made
    /// for; textures set to [`BgaPreloadOrder::Part`] take a part of it in its own order instead.
    ///
    /// One call uploads at most `bytes`, though always at least one picture when one is left, so
    /// the frames before play share the work between them. A picture `picture` does not have yet,
    /// because it is still being decoded, is taken by a later call.
    ///
    /// Answers the picture numbers and looks this call left held as textures, in the order it
    /// uploaded them.
    pub fn preload<'a, R: Renderer>(
        &mut self,
        r: &mut R,
        playhead: &BgaPlayhead,
        picture: impl Fn(i32) -> Option<BgaPicture<'a>>,
        bytes: usize,
    ) -> Vec<(i32, BgaLook)> {
        self.preload_within(r, playhead, picture, bytes, RESIDENT_BYTES)
    }

    /// [`BgaTextures::preload`] with the pictures held to `allowance` bytes between them.
    fn preload_within<'a, R: Renderer>(
        &mut self,
        r: &mut R,
        playhead: &BgaPlayhead,
        picture: impl Fn(i32) -> Option<BgaPicture<'a>>,
        bytes: usize,
        allowance: usize,
    ) -> Vec<(i32, BgaLook)> {
        let BgaTextures { resident, resident_bytes, pending, order, .. } = self;
        let follows_plan = *order == BgaPreloadOrder::Planned;
        let mut held = Vec::new();
        let pending = pending.get_or_insert_with(|| {
            let looks = playhead.first_looks();
            if follows_plan {
                return looks;
            }
            let (first, rest): (Vec<_>, Vec<_>) =
                looks.into_iter().partition(|&(number, look)| picture(number).and_then(|picture| planned(&picture, look)) == Some(true));
            first.into_iter().chain(rest).collect()
        });
        let mut uploaded = 0;
        let mut canvas = Vec::new();
        pending.retain(|&(number, look)| {
            if uploaded > 0 && uploaded >= bytes {
                return true;
            }
            let Some(picture) = picture(number).filter(BgaPicture::is_whole) else {
                return true;
            };
            let entry = (picture.generation, look);
            let cost = picture.texture_bytes();
            let unplanned = follows_plan && planned(&picture, look) == Some(false);
            if resident.contains_key(&entry) || unplanned || *resident_bytes + cost > allowance {
                return false;
            }
            let (width, height) = picture.texture_size();
            let rgba = match picture.ready(look) {
                Some(pixels) => pixels,
                None => {
                    paint(&picture, look, &mut canvas);
                    &canvas[..]
                }
            };
            let tex = r.register_texture(&format!("{RESIDENT_KEY}.{look:?}.{}", picture.generation), rgba, width, height);
            resident.insert(entry, tex);
            if r.texture_size(tex).is_some() {
                *resident_bytes += cost;
                held.push((number, look));
            }
            uploaded += cost;
            false
        });
        held
    }

    /// Sets the order [`BgaTextures::preload`] takes the chart's looks in from its next call, for a
    /// playhead that is or is not the chart the decodes planned for. What it already holds stays.
    pub fn set_order(&mut self, order: BgaPreloadOrder) {
        if self.order != order {
            self.order = order;
            self.pending = None;
        }
    }

    /// Hands every texture back to the renderer, for a screen that is done with its background.
    pub fn release<R: Renderer>(&mut self, r: &mut R) {
        self.base.release(r);
        self.layer.release(r);
        self.miss.release(r);
        for (_, tex) in self.resident.drain() {
            r.release_texture(tex);
        }
        self.resident_bytes = 0;
        self.pending = None;
    }

    /// Where each role's canvas is and how much it holds, the base's first, for a test that tells a
    /// canvas painted again from one made anew.
    #[cfg(test)]
    pub(super) fn canvases(&self) -> [(usize, usize); 3] {
        [&self.base, &self.layer, &self.miss].map(|slot| (slot.canvas.as_ptr() as usize, slot.canvas.capacity()))
    }

    /// Bytes of texture the pictures uploaded ahead of play hold, for a test of the allowance.
    #[cfg(test)]
    pub(super) fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }

    /// [`BgaTextures::preload`] under an allowance a test can reach without hundreds of megabytes of
    /// pictures.
    #[cfg(test)]
    pub(super) fn preload_under<'a, R: Renderer>(
        &mut self,
        r: &mut R,
        playhead: &BgaPlayhead,
        picture: impl Fn(i32) -> Option<BgaPicture<'a>>,
        bytes: usize,
        allowance: usize,
    ) -> Vec<(i32, BgaLook)> {
        self.preload_within(r, playhead, picture, bytes, allowance)
    }
}
