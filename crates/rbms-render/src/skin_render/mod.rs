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

mod draw;
mod object;
pub mod screen;
pub mod state;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use rbms_skin::dst::{LuaDrawEval, LuaExprId, SkinColor, SkinRect};
use rbms_skin::loader::LoadedSkin;
use rbms_skin::property::SkinStateSource;
use rbms_skin::timer::TimerState;

use crate::font::TextContext;
use crate::{Color, Rect, Renderer, TextureId};

pub use object::SkinObjectKind;
pub use screen::{
    PlayTimers, SelectTimers, SkinDraw, render_decide_screen, render_keyconfig_screen, render_play_screen, render_result_screen, render_select_screen,
};

/// Pixel height one unit of the text engine's legacy scale draws at.
///
/// A skin sizes its text in pixels while [`TextContext`] takes that scale, so the two are related by
/// this factor. It tracks `font::px_for`; the ratio itself is pinned by test rather than shared,
/// because the text engine's scale is its own private unit.
const TEXT_PIXELS_PER_SCALE: f32 = 8.5;

/// The smallest text scale worth asking the engine for, below which it clamps anyway.
const MIN_TEXT_SCALE: f32 = 0.1;

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

/// What the host supplies that this crate cannot: an image decoder, and a Lua compiler.
///
/// Both are the host's because both are outside a renderer's business. Decoding lives with whatever
/// image library the application already links, and compiling belongs to the sandbox that will
/// evaluate the result, which is `rbms_skin`'s and is reached through [`SkinExprEval`] at draw time.
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

    /// Compile one Lua expression, returning the handle a frame evaluates it by. A host without a
    /// sandbox returns `None`, which drops the one object that needed it.
    fn expression(&mut self, source: &str) -> Option<LuaExprId>;
}

/// Evaluates the compiled expressions a document wrote in place of property ids.
///
/// The draw gating half comes from the supertrait, so one implementation serves both this and
/// [`rbms_skin::dst::prepare`].
pub trait SkinExprEval: LuaDrawEval {
    /// The expression's integer value this frame, or `None` when it raised or ran out of budget.
    fn eval_integer(&self, expr: LuaExprId) -> Option<i32>;
    /// The expression's number value this frame.
    fn eval_float(&self, expr: LuaExprId) -> Option<f32>;
    /// The expression's text value this frame.
    fn eval_text(&self, expr: LuaExprId) -> Option<String>;
}

/// A host with no Lua sandbox: nothing compiles and nothing evaluates.
///
/// A document that carries no expression draws identically with this in place, so a build that has
/// not wired a sandbox up yet is not blocked from rendering ordinary skins.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoExpressions;

impl LuaDrawEval for NoExpressions {
    fn eval_draw(&self, _expr: LuaExprId) -> Option<bool> {
        None
    }
}

impl SkinExprEval for NoExpressions {
    fn eval_integer(&self, _expr: LuaExprId) -> Option<i32> {
        None
    }

    fn eval_float(&self, _expr: LuaExprId) -> Option<f32> {
        None
    }

    fn eval_text(&self, _expr: LuaExprId) -> Option<String> {
        None
    }
}

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
}

/// Everything one frame of a skin needs from the running game.
pub struct SkinFrame<'a> {
    /// The clock the frame is drawn against, the same one every timer is measured on.
    pub now_ms: i64,
    pub timers: &'a TimerState,
    pub state: &'a dyn SkinStateSource,
    /// The compiled expressions, when the host has a sandbox.
    pub lua: Option<&'a dyn SkinExprEval>,
    /// Where the pointer is in document coordinates, for the objects a document gated on it.
    pub mouse: Option<(f32, f32)>,
    /// The background image this frame, already registered with the renderer.
    pub background: Option<TextureId>,
}

impl std::fmt::Debug for SkinFrame<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SkinFrame").field("now_ms", &self.now_ms).field("mouse", &self.mouse).finish_non_exhaustive()
    }
}

/// A loaded document, compiled into something a screen draws every frame.
///
/// Building it registers every image source the document names and resolves every object it draws;
/// after that a frame costs one pass over the object list and no file access at all.
#[derive(Debug)]
pub struct SkinScreen {
    /// The size the document was authored at, which every coordinate in it is relative to.
    authored: (f32, f32),
    objects: Vec<object::SkinObject>,
    /// Every texture this screen registered, so it can hand them all back.
    textures: Vec<TextureId>,
    /// The font family each font id resolved to inside the text engine.
    families: Vec<(String, String)>,
    warnings: Vec<String>,
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
        let serial = NEXT_SCREEN_SERIAL.fetch_add(1, Ordering::Relaxed);
        let mut warnings: Vec<String> = Vec::new();
        let mut textures: Vec<TextureId> = Vec::new();

        let mut sources: Vec<(String, TextureId, (u32, u32))> = Vec::new();
        for (id, path) in &skin.sources {
            match assets.image(path) {
                Some(image) => {
                    let key = format!("rbms.skin.{serial}.source.{id}");
                    let tex = r.register_texture(&key, &image.rgba, image.width, image.height);
                    textures.push(tex);
                    sources.push((id.clone(), tex, (image.width, image.height)));
                }
                None => warnings.push(format!("image source {id:?} could not be decoded from {}", path.display())),
            }
        }

        let mut families: Vec<(String, String)> = Vec::new();
        for (id, path) in &skin.fonts {
            match assets.font(path).and_then(|data| text.load_font(data)) {
                Some(family) => families.push((id.clone(), family)),
                None => warnings.push(format!("font {id:?} could not be loaded from {}", path.display())),
            }
        }

        let objects = object::build_objects(skin, &sources, &families, assets, &mut warnings);
        let authored = (skin.def.w.max(1) as f32, skin.def.h.max(1) as f32);
        SkinScreen { authored, objects, textures, families, warnings }
    }

    /// The size the document was authored at.
    pub fn authored_size(&self) -> (f32, f32) {
        self.authored
    }

    /// How many objects this screen draws.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// The document's own id for each object, in draw order.
    pub fn object_ids(&self) -> Vec<&str> {
        self.objects.iter().map(|object| object.id.as_str()).collect()
    }

    /// How many of this screen's objects are of one kind.
    pub fn count_of(&self, kind: SkinObjectKind) -> usize {
        self.objects.iter().filter(|object| object.kind() == kind).count()
    }

    /// How many textures this screen holds.
    pub fn texture_count(&self) -> usize {
        self.textures.len()
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
        for tex in self.textures.drain(..) {
            r.release_texture(tex);
        }
        self.objects.clear();
    }

    /// Draws one frame, in the document's own object order, and answers how many objects were drawn.
    ///
    /// Order is z-order: an object is submitted exactly where the document put it, so a backend that
    /// merges adjacent draws still lands them in the same place.
    pub fn draw<R: Renderer>(&self, ctx: &mut crate::ctx::RenderCtx<'_>, r: &mut R, frame: &SkinFrame<'_>) -> usize {
        let (screen_w, screen_h) = r.size();
        let viewport = SkinViewport::new(self.authored, (screen_w as f32, screen_h as f32));
        let mut drawn = 0;
        for object in &self.objects {
            if draw::draw_object(ctx, r, object, &viewport, frame) {
                drawn += 1;
            }
        }
        ctx.text.reset_family();
        drawn
    }
}
