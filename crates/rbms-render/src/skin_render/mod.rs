//! Drawing a loaded JSON skin through the renderer's primitives.
//!
//! [`rbms_skin`] answers *what* a document says: which objects it declares, where each one sits at a
//! given millisecond, and whether it is drawn at all. This module answers *how* that reaches the
//! screen: it turns the document's image sources into registered textures, its objects into a flat
//! draw list, and each resolved destination into a [`QuadParams`].
//!
//! Two coordinate changes happen here and nowhere else. A document is authored y-up with the origin
//! at the bottom left of its own `w` x `h` space, while the renderer is y-down over the logical
//! screen, so [`SkinViewport`] flips and scales every rectangle. The rotation sign was already
//! turned around by the loader, so an angle arrives screen-clockwise and is passed straight through.
//!
//! Nothing here replaces the built-in screens. A screen with no document selected draws exactly what
//! it drew before; [`SkinScreen`] is what a screen reaches for only once a document has loaded.
//!
//! Nothing here knows the built-in screens either. A document is answered by a
//! [`SkinHost`](rbms_skin::property::SkinHost) and drawn from a [`FrameData`]: the application
//! implements the one and fills the other, and which timers a screen switches on, and when, is the
//! application's to decide. A play document takes its lanes and its judgement line from itself, and
//! is drawn under the one offset that moves a whole screen (`whole`).

mod bga;
mod color;
mod covers;
mod draw;
pub mod frame;
mod gauge;
pub mod graphs;
pub mod input;
mod judge;
mod notes;
mod object;
pub mod refs;
mod songlist;
mod text;
mod text_input;
pub mod textures;
mod whole;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_list_graphs;
#[cfg(test)]
mod tests_play_objects;

use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use rbms_skin::dst::{LuaDrawEval, SkinColor, SkinRect};
use rbms_skin::loader::LoadedSkin;
use rbms_skin::property::generated::OFFSET_ALL;

use crate::font::TextContext;
use crate::{Color, Rect, Renderer};

pub use bga::BgaFrame;
pub use color::parse_hex_color;
pub use frame::{FrameData, FrameSeries, PreparedFrame, SkinFrame};
pub use gauge::GaugeFrame;
pub use graphs::{BpmTimeline, GaugeHistory, NoteDistribution, RecentHits, TimingHistogram};
pub use input::{SkinAction, SkinEvent, SkinInputMap, SkinPointer, SkinPointerButton, SkinWriter};
pub use object::SkinObjectKind;
pub use refs::{ReferenceImage, ReferenceImages};
pub use songlist::SongBars;
pub use text_input::{Composition, SkinTextWriter, TextEntry, TextEntryStart};

/// Distinguishes one loaded document's textures from another's, so a play screen and a select screen
/// can each hold a document without their image sources colliding in the registry.
static NEXT_SCREEN_SERIAL: AtomicU32 = AtomicU32::new(0);

/// A rectangle in document space becomes one in screen space only through [`SkinViewport`]; this
/// conversion is the plain field copy the two share, and does not flip or scale anything.
impl From<SkinRect> for Rect {
    fn from(rect: SkinRect) -> Rect {
        Rect { x: rect.x, y: rect.y, w: rect.w, h: rect.h }
    }
}

impl From<SkinColor> for Color {
    fn from(color: SkinColor) -> Color {
        Color { r: color.r, g: color.g, b: color.b, a: color.a }
    }
}

/// Decoded RGBA8 pixels for one of a document's image sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl SkinImage {
    /// An image from its size and pixels, or `None` when the buffer does not hold exactly that many
    /// RGBA8 pixels.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Option<SkinImage> {
        let expected = (width as usize) * (height as usize) * crate::BYTES_PER_PIXEL;
        (width > 0 && height > 0 && rgba.len() == expected).then_some(SkinImage { width, height, rgba })
    }
}

/// What the host supplies that this crate cannot: decoded images and the bytes of fonts.
///
/// Decoding is the host's because it is outside a renderer's business: it lives with whatever image
/// library the application already links. Nothing is compiled here either. A skin's Lua -- the
/// functions a Lua skin hands over and the scripts a document wrote as strings -- is in the
/// interpreter `rbms_skin` loaded the skin into, and a frame reaches it through
/// [`LuaDrawEval`].
pub trait SkinAssets {
    /// Decode the image file at `path` into RGBA8 pixels, or `None` when it cannot be read.
    fn image(&mut self, path: &Path) -> Option<SkinImage>;

    /// The bytes of the font file at `path`, or `None` when it cannot be read.
    ///
    /// The default reads the file, which is what a host with nothing prepared wants. A host that
    /// keeps its frame loop off the disk overrides this with bytes it read on a worker, the same
    /// way it does for images.
    fn font(&mut self, path: &Path) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }
}

/// A frame with no interpreter behind it: nothing evaluates.
///
/// A skin that carries no Lua draws identically with this in place. A function value or a property
/// name reads as its type's fallback: false, zero, empty text, and a timer that is off.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoExpressions;

impl LuaDrawEval for NoExpressions {}

/// Maps the document's own coordinate space onto the screen being drawn.
///
/// A document is authored y-up with the origin at the bottom left of a `w` x `h` space it chose, and
/// the renderer is y-down over the logical screen, so a rectangle's top edge is the document's
/// distance from the bottom subtracted from the document's height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkinViewport {
    scale_x: f32,
    scale_y: f32,
    skin_height: f32,
}

impl SkinViewport {
    /// The mapping from a document authored at `skin` onto a screen of `screen` logical pixels. A
    /// document or screen with no extent maps one to one rather than dividing by zero.
    pub fn new(skin: (f32, f32), screen: (f32, f32)) -> SkinViewport {
        let (skin_w, skin_h) = skin;
        let (screen_w, screen_h) = screen;
        let scale_x = if skin_w > 0.0 { screen_w / skin_w } else { 1.0 };
        let scale_y = if skin_h > 0.0 { screen_h / skin_h } else { 1.0 };
        SkinViewport { scale_x, scale_y, skin_height: skin_h }
    }

    /// Where a document rectangle lands on screen.
    pub fn place(&self, rect: SkinRect) -> Rect {
        Rect { x: rect.x * self.scale_x, y: (self.skin_height - (rect.y + rect.h)) * self.scale_y, w: rect.w * self.scale_x, h: rect.h * self.scale_y }
    }

    /// How much a document pixel is stretched horizontally, which is what a text size and the gap
    /// between digits scale by.
    pub fn scale_x(&self) -> f32 {
        self.scale_x
    }

    /// How much a document pixel is stretched vertically.
    pub fn scale_y(&self) -> f32 {
        self.scale_y
    }

    /// How far above the document's own foot a screen row falls.
    ///
    /// The inverse of the vertical half of [`SkinViewport::place`], for the objects that have to
    /// compare a rectangle the document authored with a line the running game measured in screen
    /// pixels. A screen of no height puts every row at the document's ceiling rather than dividing
    /// by zero.
    pub fn document_y(&self, screen_y: f32) -> f32 {
        if self.scale_y <= 0.0 { self.skin_height } else { self.skin_height - screen_y / self.scale_y }
    }
}

/// A loaded document, compiled into something a screen draws every frame.
///
/// Building it registers every image source the document's objects draw from and resolves every
/// object it draws; after that a frame costs two passes over the object list, one to prepare and one
/// to draw, and no file access at all.
#[derive(Debug)]
pub struct SkinScreen {
    /// The size the document was authored at, which every coordinate in it is relative to.
    authored: (f32, f32),
    objects: Vec<object::SkinObject>,
    /// What each object does with the pointer, by the same index as `objects`.
    interactions: Vec<input::Interaction>,
    /// Every texture this screen registered, so it can hand them all back.
    textures: textures::SkinTextures,
    /// The font family each font id resolved to inside the text engine.
    families: Vec<(String, String)>,
    warnings: Vec<String>,
    /// The offset the whole screen is drawn under, settled by the first frame that is drawn and
    /// kept for as long as the screen is (`Skin.ensureRenderer`, which sets the transform when it
    /// makes the renderer and never again). `None` for a document that is not of a play type, which
    /// is never moved.
    whole: Option<OnceLock<whole::WholeOffset>>,
}

impl SkinScreen {
    /// Compiles a loaded document against `r`, registering its images as textures and its fonts with
    /// `text`.
    ///
    /// Nothing here fails the whole screen: an image that will not decode, a font that will not
    /// load, an object whose type this build does not draw and an expression that will not compile
    /// each drop the one object involved and leave a line in [`SkinScreen::warnings`].
    ///
    /// Each build claims a registry namespace of its own, so two screens may hold documents that
    /// name the same source id without colliding. That also means a reload registers a fresh set of
    /// textures: [`SkinScreen::release`] the old screen first, or the old set stays uploaded.
    pub fn build<R: Renderer>(r: &mut R, text: &mut TextContext, skin: &LoadedSkin, assets: &mut dyn SkinAssets) -> SkinScreen {
        SkinScreen::compile(r, text, skin, assets, None)
    }

    /// [`SkinScreen::build`], with the screen's image sources held in `pool` rather than registered
    /// for this screen alone.
    ///
    /// A file the pool already holds is drawn from the texture that is there, and `assets` is only
    /// asked for it in case the host decoded it again. The screen has to be handed back with
    /// [`SkinScreen::release_shared`] and the same pool.
    pub fn build_shared<R: Renderer>(
        r: &mut R,
        text: &mut TextContext,
        skin: &LoadedSkin,
        assets: &mut dyn SkinAssets,
        pool: &mut textures::SkinTexturePool,
    ) -> SkinScreen {
        SkinScreen::compile(r, text, skin, assets, Some(pool))
    }

    /// Compiles a document, taking its textures from `pool` when there is one.
    fn compile<R: Renderer>(
        r: &mut R,
        text: &mut TextContext,
        skin: &LoadedSkin,
        assets: &mut dyn SkinAssets,
        pool: Option<&mut textures::SkinTexturePool>,
    ) -> SkinScreen {
        let serial = NEXT_SCREEN_SERIAL.fetch_add(1, Ordering::Relaxed);
        let mut warnings: Vec<String> = Vec::new();
        let textures = textures::SkinTextures::register(r, skin, assets, serial, pool, &mut warnings);

        let mut families: Vec<(String, String)> = Vec::new();
        for (id, path) in &skin.fonts {
            match assets.font(path).and_then(|data| text.load_font(data)) {
                Some(family) => families.push((id.clone(), family)),
                None => warnings.push(format!("font {id:?} could not be loaded from {}", path.display())),
            }
        }

        let mut kept = Vec::new();
        let objects = object::build_objects(skin, textures.sources(), &families, assets, &mut warnings, &mut kept);
        let interactions = input::interactions(skin, &kept, &objects);
        let authored = (skin.def.w.max(1) as f32, skin.def.h.max(1) as f32);
        let whole = whole::applies_to(skin.def.skin_type).then(OnceLock::new);
        SkinScreen { authored, objects, interactions, textures, families, warnings, whole }
    }

    /// The size the document was authored at.
    pub fn authored_size(&self) -> (f32, f32) {
        self.authored
    }

    /// How many objects this screen draws.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// How many of this screen's objects are of one kind.
    pub fn count_of(&self, kind: SkinObjectKind) -> usize {
        self.objects.iter().filter(|object| object.kind() == kind).count()
    }

    /// How many textures this screen holds.
    pub fn texture_count(&self) -> usize {
        self.textures.stats().count
    }

    /// How many image-source textures this screen holds and the RGBA bytes they come to.
    pub fn texture_stats(&self) -> textures::TextureStats {
        self.textures.stats()
    }

    /// The font families this screen registered, as `(document font id, family name)`.
    pub fn families(&self) -> &[(String, String)] {
        &self.families
    }

    /// Everything that was dropped while building, one line each.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Hands every texture back to `r`. A screen is unusable afterwards and must be rebuilt, which
    /// is what a skin reload does.
    pub fn release<R: Renderer>(&mut self, r: &mut R) {
        self.release_textures(r, None);
    }

    /// [`SkinScreen::release`] for a screen made by [`SkinScreen::build_shared`]: its hold on each
    /// pooled texture goes back to `pool`, where the texture waits for the pool's next sweep.
    pub fn release_shared<R: Renderer>(&mut self, r: &mut R, pool: &mut textures::SkinTexturePool) {
        self.release_textures(r, Some(pool));
    }

    /// Lets go of everything this screen registered or holds.
    fn release_textures<R: Renderer>(&mut self, r: &mut R, pool: Option<&mut textures::SkinTexturePool>) {
        self.textures.release(r, pool);
        for object in &self.objects {
            object.release(r);
        }
        self.objects.clear();
        self.interactions.clear();
    }

    /// The first stage of a frame: prepares every object in the document's own order -- draw
    /// conditions, then the timer, then where it sits, then the values it shows -- and draws nothing
    /// (`Skin.drawAllObjects`, the `prepare` loop).
    ///
    /// This is the only stage that asks the skin's Lua anything, so it is the one that has to run
    /// while `frame.lua` is bound to a host. Bind once, prepare inside the binding, and the frame
    /// can be drawn after the binding has ended.
    pub fn prepare(&self, frame: &SkinFrame<'_>) -> PreparedFrame {
        frame::prepare_objects(&self.objects, frame)
    }

    /// The second stage of a frame: draws what `prepared` left standing, in the document's own
    /// object order, and answers how many objects were drawn.
    ///
    /// Order is z-order: an object is submitted exactly where the document put it, so a backend that
    /// merges adjacent draws still lands them in the same place. `frame` is the frame `prepared` was
    /// made from; its `lua` is not consulted, because everything the skin's Lua had to say was said
    /// while the frame was prepared.
    ///
    /// A play document is drawn under the whole-screen offset the host answers `OFFSET_ALL` with:
    /// every object, and every clip one is drawn under, is moved by that share of the target and
    /// stretched away from its bottom left corner. The offset is read on the first frame drawn and
    /// that reading is kept, as the reference keeps the transform it set when it made its renderer.
    /// Where the pointer is taken to be is not moved with it, in the reference or here.
    pub fn draw_prepared<R: Renderer>(&self, ctx: &mut crate::ctx::RenderCtx<'_>, r: &mut R, frame: &SkinFrame<'_>, prepared: &PreparedFrame) -> usize {
        let (screen_w, screen_h) = r.size();
        let viewport = SkinViewport::new(self.authored, (screen_w as f32, screen_h as f32));
        let whole = self.whole.as_ref().map(|settled| *settled.get_or_init(|| whole::WholeOffset::of(frame.state.offset(OFFSET_ALL))));
        let drawn = match whole.filter(|whole| !whole.is_identity()) {
            Some(whole) => frame::draw_objects(ctx, &mut whole.over(r), &self.objects, &viewport, frame, prepared),
            None => frame::draw_objects(ctx, r, &self.objects, &viewport, frame, prepared),
        };
        ctx.text.reset_family();
        drawn
    }

    /// Makes one whole frame, both stages back to back, and answers how many objects were drawn.
    ///
    /// Every object is prepared before the first one is drawn, as [`SkinScreen::prepare`] and
    /// [`SkinScreen::draw_prepared`] called in turn would have it.
    pub fn draw<R: Renderer>(&self, ctx: &mut crate::ctx::RenderCtx<'_>, r: &mut R, frame: &SkinFrame<'_>) -> usize {
        let prepared = self.prepare(frame);
        self.draw_prepared(ctx, r, frame, &prepared)
    }
}
