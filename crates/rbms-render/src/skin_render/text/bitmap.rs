//! A `text` object whose font is a bitmap font: glyphs cut out of the font's page images.
//!
//! This is the reference's `SkinTextBitmap`. What every such text shares:
//!
//! - The glyphs are scaled by `size * (screen width / skin width) / the size the font was made at`,
//!   the same across and down. The destination's height plays no part in it; only its top edge
//!   does, which is the capital line of the first line of text.
//! - The destination's `x` is the same anchor a TrueType text has, and its width the same room.
//!   What `overflow` and `wrapping` do with a line too long for that room is what the layout in
//!   [`crate::bitmap_font`] does, which is the reference's layout carried over.
//! - A glyph is drawn where the layout puts it, on no pixel grid.
//! - Like every text, it is blended the way the object before it was.
//!
//! A font of type 0, the plain kind, holds its glyphs' coverage. Its glyphs are filtered bilinearly
//! whatever the destination's `filter` says (`SkinObjectRenderer.TYPE_BILINEAR`), and a shadow is
//! the same glyphs drawn first at half the brightness, moved right and down by the document's offset
//! in screen pixels. `outlineColor`, `outlineWidth`, `shadowColor` and `shadowSmoothness` are not
//! read.
//!
//! A font of type 1 or 2 is a distance field font: its pages hold, in their alpha, how far each
//! texel is from the glyph's outline. The reference draws the two types through one path -- the
//! type only sets the fonts in a fallback chain apart, and this has no such chain -- with a shader
//! of its own (`SkinObjectRenderer.TYPE_DISTANCE_FIELD`, [`crate::distance_field_fragment`]):
//!
//! - The glyphs are drawn once. There is no second pass for a shadow; the shader draws the shadow
//!   and the outline inside each glyph's own quad, which is why such a font is made with padding.
//! - What the shader is told comes from the text ([`FieldInk`]): the outline's colour and width, and
//!   the shadow's colour, smoothness and offset.
//! - The shadow's offset is in pixels of the font's page, not of the screen, so it grows and shrinks
//!   with the glyphs. The page's size is the one the font file states.
//!
//! Two things differ from the reference on purpose.
//!
//! A character the font does not have is not drawn as an empty box. It is drawn from the text
//! engine's own fonts, on the same line and at the size the bitmap font was made for: it takes the
//! room of its advance there, so the glyphs after it move over as they would for any other glyph.
//! Those characters are composed into one texture per text object, drawn over the bitmap glyphs.
//! Beside a distance field font they are drawn plainly in the text's colour, with no outline and no
//! shadow, the way the reference draws the glyphs it borrows from a plain fallback font.
//!
//! The pages are not all loaded when the skin is. A published font spreads eleven thousand glyphs
//! over as many as thirty pages of sixteen megabytes each, and a line of text touches one or two of
//! them, so a page is asked for the first time a glyph on it is to be drawn ([`PageTable`]), or
//! when the screen is built for a text whose string the document wrote out. A glyph is drawn from
//! the frame its page is answered for with a texture, and the rest of its line does not wait for
//! it: a line whose string changes while the screen is up -- a song wheel being turned -- shows at
//! once every glyph whose page is already there, and the others a moment later.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use rbms_skin::model::TextDef;

use super::{SHADOW_DIVISOR, TextBody, anchor_x};
use crate::bitmap_font::{BitmapFont, Piece};
use crate::ctx::RenderCtx;
use crate::font::{BlockAlign, BlockFit, BlockSpec, CLEAR_PIXEL, TextBlock, TextContext};
use crate::skin_render::color::{DIGITS_PER_CHANNEL, HEX_RADIX};
use crate::skin_render::draw::Placement;
use crate::skin_render::textures::SizedTexture;
use crate::{
    BYTES_PER_PIXEL, BlendMode, Color, DISTANCE_FIELD_EDGE, DistanceFieldParams, DistanceFieldStyle, QuadParams, Rect, Renderer, TextureFilter, TextureId,
    UvRect, locked,
};

#[cfg(test)]
mod field_tests;
#[cfg(test)]
mod tests;

/// The `type` of a font the reference draws with its distance field shader
/// (`SkinTextBitmapSource.TYPE_DISTANCE_FIELD`).
const FONT_TYPE_DISTANCE_FIELD: i32 = 1;

/// The `type` of a coloured distance field font, which the reference draws exactly like the plain
/// one (`SkinTextBitmapSource.TYPE_COLORED_DISTANCE_FIELD`).
const FONT_TYPE_COLORED_DISTANCE_FIELD: i32 = 2;

/// The least distance a glyph's outline is ever drawn out to, however wide the text asks for it
/// (`SkinTextBitmap.setDistanceFieldUniforms`: `Math.max(0.1f, ...)`).
const MIN_OUTLINE_DISTANCE: f32 = 0.1;

/// What a text's `outlineWidth` and `shadowSmoothness` are divided by on their way to the shader
/// (`SkinTextBitmap.setDistanceFieldUniforms`).
const FIELD_HALVING: f32 = 2.0;

/// The colour a text's outline or shadow has when what the document wrote is not a colour
/// (`JsonSkinObjectLoader.parseHexColor(..., Color.WHITE)`): opaque white, which is not the
/// transparent white a text that wrote nothing has.
const UNREADABLE_COLOR: Color = Color { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: u8::MAX };

/// How many hexadecimal digits a written colour with an alpha of its own has.
const DIGITS_WITH_ALPHA: usize = 8;

/// How many of a colour's channels are always written: red, green and blue.
const WRITTEN_CHANNELS: usize = 3;

/// A colour as the reference reads one a text wrote (`Color.valueOf` of the libGDX it ships,
/// 1.9.9): a `#` is skipped, the first six digits are red, green and blue, and the next two are the
/// alpha only when the text is exactly eight digits long -- any other length is opaque. Text that
/// is too short or holds something other than digits there is [`UNREADABLE_COLOR`].
fn written_color(text: &str) -> Color {
    let digits = text.strip_prefix('#').unwrap_or(text).as_bytes();
    let channel = |index: usize| {
        let pair = digits.get(index * DIGITS_PER_CHANNEL..(index + 1) * DIGITS_PER_CHANNEL)?;
        let pair = std::str::from_utf8(pair).ok().filter(|pair| pair.bytes().all(|digit| digit.is_ascii_hexdigit()))?;
        u8::from_str_radix(pair, HEX_RADIX).ok()
    };
    let read = || {
        let alpha = if digits.len() == DIGITS_WITH_ALPHA { channel(WRITTEN_CHANNELS)? } else { u8::MAX };
        Some(Color { r: channel(0)?, g: channel(1)?, b: channel(2)?, a: alpha })
    };
    read().unwrap_or(UNREADABLE_COLOR)
}

/// The outline and the shadow a text asks a distance field font for, as the document wrote them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FieldInk {
    outline_color: Color,
    outline_width: f32,
    shadow_color: Color,
    shadow_smoothness: f32,
}

impl FieldInk {
    /// What `def` asks for (`JsonSkinObjectLoader.createText`).
    pub(crate) fn of(def: &TextDef) -> FieldInk {
        FieldInk {
            outline_color: written_color(&def.outline_color),
            outline_width: def.outline_width,
            shadow_color: written_color(&def.shadow_color),
            shadow_smoothness: def.shadow_smoothness,
        }
    }

    /// What the shader is told for a glyph on a page `page` pixels large, with the shadow moved by
    /// `shadow` of those pixels (`SkinTextBitmap.setDistanceFieldUniforms`).
    fn style(&self, shadow: (f32, f32), page: (f32, f32)) -> DistanceFieldStyle {
        DistanceFieldStyle {
            outline_distance: (DISTANCE_FIELD_EDGE - self.outline_width / FIELD_HALVING).max(MIN_OUTLINE_DISTANCE),
            outline_color: self.outline_color,
            shadow_color: self.shadow_color,
            shadow_smoothing: self.shadow_smoothness / FIELD_HALVING,
            shadow_offset: (shadow.0 / page.0, shadow.1 / page.1),
        }
    }
}

/// How many characters a face remembers the stand-in glyph of before it forgets them all and
/// starts over.
const FALLBACK_CACHE_LIMIT: usize = 2048;

/// The channel of an RGBA pixel that holds its opacity.
const ALPHA_CHANNEL: usize = 3;

/// Tells one text object's stand-in texture from another's in the renderer's registry.
static NEXT_OVERLAY_SERIAL: AtomicU32 = AtomicU32::new(0);

/// How a bitmap font's pages are meant to be read (`font.type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BitmapKind {
    /// The pages hold the glyphs' coverage.
    Standard,
    /// The pages hold each glyph's distance field.
    DistanceField,
}

impl BitmapKind {
    /// The kind a document's `type` asks for. Anything but the two distance field values is the
    /// plain kind (`SkinTextBitmap.draw`).
    pub(crate) fn of(font_type: i32) -> BitmapKind {
        match font_type {
            FONT_TYPE_DISTANCE_FIELD | FONT_TYPE_COLORED_DISTANCE_FIELD => BitmapKind::DistanceField,
            _ => BitmapKind::Standard,
        }
    }
}

/// Where one page image is on its way to being a texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageSlot {
    /// A glyph on the page was to be drawn, and nobody has answered for the page yet.
    Wanted,
    Loaded(TextureId, (u32, u32)),
    /// The page will not be a texture: it would not decode, or the screen had no room for it.
    Refused,
}

/// The page images one screen's bitmap fonts have asked for, by file.
///
/// A page is entered the first time a glyph on it is to be drawn, or when the screen is built for a
/// glyph of a string the document wrote out, and stays until the screen is released. The screen
/// that owns the table answers each wanted page once
/// ([`SkinScreen::settle_font_page`](crate::skin_render::SkinScreen::settle_font_page)).
#[derive(Debug, Default)]
pub(crate) struct PageTable {
    slots: BTreeMap<PathBuf, PageSlot>,
}

impl PageTable {
    /// The pages nobody has answered for yet.
    pub(crate) fn wanted(&self) -> Vec<PathBuf> {
        self.slots.iter().filter(|(_, slot)| **slot == PageSlot::Wanted).map(|(path, _)| path.clone()).collect()
    }

    /// Whether `path` is a page nobody has answered for yet.
    pub(crate) fn is_wanted(&self, path: &Path) -> bool {
        self.slots.get(path) == Some(&PageSlot::Wanted)
    }

    /// Answers for the page at `path`: the texture it became, or nothing when it became none.
    pub(crate) fn settle(&mut self, path: &Path, texture: Option<SizedTexture>) {
        let slot = texture.map_or(PageSlot::Refused, |(tex, size)| PageSlot::Loaded(tex, size));
        self.slots.insert(path.to_path_buf(), slot);
    }

    /// How many pages are textures.
    pub(crate) fn loaded(&self) -> usize {
        self.slots.values().filter(|slot| matches!(slot, PageSlot::Loaded(..))).count()
    }

    /// Forgets every page, for a screen whose textures have gone back to the renderer.
    pub(crate) fn clear(&mut self) {
        self.slots.clear();
    }
}

/// One screen's page table, shared by every bitmap font the screen draws with.
pub(crate) type SharedPages = Arc<Mutex<PageTable>>;

/// A character the font lacks, as the text engine draws it at one size.
#[derive(Debug, Clone)]
struct FallbackGlyph {
    /// How far the pen moves over it, in screen pixels.
    advance: f32,
    /// How far right of the pen its ink starts.
    bearing: f32,
    /// Its pixels, or `None` for a character with no ink.
    block: Option<Arc<TextBlock>>,
}

/// A bitmap font as one screen draws with it: the font, how its pages are read, the screen's page
/// table and the stand-ins for the characters it lacks.
#[derive(Debug)]
pub(crate) struct ScreenFace {
    font: Arc<BitmapFont>,
    kind: BitmapKind,
    pages: SharedPages,
    fallbacks: Mutex<HashMap<(char, u32), FallbackGlyph>>,
}

impl ScreenFace {
    pub(crate) fn new(font: Arc<BitmapFont>, kind: BitmapKind, pages: SharedPages) -> ScreenFace {
        ScreenFace { font, kind, pages, fallbacks: Mutex::new(HashMap::new()) }
    }

    /// How the font's pages are meant to be read.
    pub(crate) fn kind(&self) -> BitmapKind {
        self.kind
    }

    /// The stand-in for `ch` at an em of `em` pixels, measured once and remembered.
    ///
    /// The text engine lays a line out from the ink of its first glyph, so what it reports of a
    /// single character is its ink and how far past the ink's left edge the pen ends up. The space
    /// left of the ink is not reported and is taken to equal the space right of it, which is what
    /// the glyphs of the scripts a bitmap font tends to lack -- Hangul, kana, ideographs -- have.
    fn fallback(&self, text: &mut TextContext, ch: char, em: f32, max_dim: u32) -> FallbackGlyph {
        let key = (ch, em.to_bits());
        let mut known = locked(&self.fallbacks);
        if let Some(glyph) = known.get(&key) {
            return glyph.clone();
        }
        if known.len() >= FALLBACK_CACHE_LIMIT {
            known.clear();
        }
        let mut utf8 = [0u8; 4];
        let family = text.default_family().to_string();
        let spec = BlockSpec {
            text: ch.encode_utf8(&mut utf8),
            family: &family,
            em_px: em,
            design_px: em,
            width: em,
            align: BlockAlign::Left,
            fit: BlockFit::Overflow,
            max_dim,
        };
        let reach = text.caret_x(&spec, 1).unwrap_or_default();
        let block = text.compose_block(&spec);
        let bearing = block.as_ref().map_or(0.0, |block| (reach - block.layout_width).max(0.0));
        let glyph = FallbackGlyph { advance: reach + bearing, bearing, block: block.map(Arc::new) };
        known.insert(key, glyph.clone());
        glyph
    }

    /// What `ch` is drawn with: the font's own glyph where it has one, the text engine's otherwise.
    /// A control character the font has no glyph for is not drawn and takes no room.
    fn piece(&self, text: &mut TextContext, ch: char, em: f32, scale: f32, max_dim: u32) -> Option<Piece> {
        if let Some(piece) = self.font.piece(ch) {
            return Some(piece);
        }
        if ch.is_control() {
            return None;
        }
        let glyph = self.fallback(text, ch, em, max_dim);
        Some(Piece::Fallback { ch, advance: glyph.advance / scale })
    }

    /// The texture of each page in `pages`, by page number, and whether every one of them has been
    /// answered for. One that has not is asked for here, if nobody has, and has no texture yet. A
    /// page that was refused, and a page the font names no file for, has no texture and is not
    /// waited for.
    fn page_textures(&self, pages: &[usize]) -> (Vec<(usize, Option<SizedTexture>)>, bool) {
        let mut table = locked(&self.pages);
        let mut settled = true;
        let mut textures = Vec::with_capacity(pages.len());
        for page in pages {
            let texture = match self.font.page(*page) {
                None => None,
                Some(path) => match table.slots.get(path) {
                    Some(PageSlot::Loaded(tex, size)) => Some((*tex, *size)),
                    Some(PageSlot::Refused) => None,
                    Some(PageSlot::Wanted) => {
                        settled = false;
                        None
                    }
                    None => {
                        table.slots.insert(path.to_path_buf(), PageSlot::Wanted);
                        settled = false;
                        None
                    }
                },
            };
            textures.push((*page, texture));
        }
        (textures, settled)
    }

    /// Asks for the pages the glyphs of `line` are on, ahead of the frame that first draws them: a
    /// string the document wrote out is known when the screen is built, so its pages need not wait
    /// for the object to come on screen.
    pub(crate) fn want_pages_of(&self, line: &str) {
        let mut table = locked(&self.pages);
        let pages = line.chars().filter_map(|ch| self.font.piece(ch)).filter_map(|piece| inked_page(&piece));
        for path in pages.filter_map(|page| self.font.page(page)) {
            table.slots.entry(path.to_path_buf()).or_insert(PageSlot::Wanted);
        }
    }
}

/// The page a piece's pixels are on, for a piece that is a glyph of the font with pixels to draw.
fn inked_page(piece: &Piece) -> Option<usize> {
    match piece {
        Piece::Glyph { glyph, .. } if glyph.has_ink() => Some(glyph.page),
        _ => None,
    }
}

/// One glyph of a laid out line: which pixels of which page, and where they go.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GlyphQuad {
    page: usize,
    /// The glyph's rectangle on its page, as `(x, y, width, height)` in pixels.
    src: (u32, u32, u32, u32),
    /// Where it lands, measured from the line's anchor and the destination's top edge.
    at: Rect,
}

/// Where the texture of a line's stand-in glyphs lands, measured like a [`GlyphQuad`].
#[derive(Debug, Clone, Copy, PartialEq)]
struct OverlayStamp {
    tex: TextureId,
    size: (u32, u32),
    at: Rect,
}

/// A line as it was last laid out.
#[derive(Debug)]
struct LaidOut {
    text: String,
    /// The scale and the width the line was laid out at, as their bit patterns.
    sized: (u32, u32),
    quads: Vec<GlyphQuad>,
    /// The pages the quads read from, each once.
    pages: Vec<usize>,
    overlay: Option<OverlayStamp>,
    /// The texture of each page in `pages`, kept once every one of them has been answered for.
    /// Until then the pages are looked up on every frame the line is drawn.
    textures: Option<Vec<(usize, Option<SizedTexture>)>>,
}

impl LaidOut {
    /// Whether this is `line` laid out at `sized`, with its stand-in texture still known to `r` at
    /// the size it was made -- which it is not on another renderer than the one it was uploaded to.
    fn holds<R: Renderer>(&self, r: &R, line: &str, sized: (u32, u32)) -> bool {
        self.text == line && self.sized == sized && self.overlay.is_none_or(|overlay| r.texture_size(overlay.tex) == Some(overlay.size))
    }
}

/// How one line is laid out.
#[derive(Debug, Clone, Copy)]
struct LineSpec {
    scale: f32,
    width: f32,
    align: BlockAlign,
    fit: BlockFit,
}

/// One stand-in glyph and where its top left corner lands among the others.
struct Stamp {
    x: i32,
    y: i32,
    block: Arc<TextBlock>,
}

/// The stand-in glyphs of one line composed into one image, with where its top left corner lands.
struct Overlay {
    rgba: Vec<u8>,
    size: (u32, u32),
    origin: (i32, i32),
}

/// Composes `stamps` into one image no longer on an edge than `max_dim`, or `None` when there is
/// nothing to compose. Where two glyphs overlap the more opaque pixel is kept.
///
/// Where a stamp lands is as far off as the font's advances put it, and a font may give any, so
/// the distances between stamps are worked out in numbers wide enough for any two of them.
fn compose_overlay(stamps: &[Stamp], max_dim: u32) -> Option<Overlay> {
    let far = |near: i32, extent: u32| i64::from(near) + i64::from(extent);
    let left = stamps.iter().map(|stamp| stamp.x).min()?;
    let top = stamps.iter().map(|stamp| stamp.y).min()?;
    let right = stamps.iter().map(|stamp| far(stamp.x, stamp.block.width)).max()?;
    let bottom = stamps.iter().map(|stamp| far(stamp.y, stamp.block.height)).max()?;
    let width = (right - i64::from(left)).clamp(0, i64::from(max_dim)) as u32;
    let height = (bottom - i64::from(top)).clamp(0, i64::from(max_dim)) as u32;
    if width == 0 || height == 0 {
        return None;
    }
    let mut rgba = CLEAR_PIXEL.repeat(width as usize * height as usize);
    for stamp in stamps {
        let across = stamp.block.width as usize;
        let (first_column, first_row) = (i64::from(stamp.x) - i64::from(left), i64::from(stamp.y) - i64::from(top));
        for (row, pixels) in stamp.block.rgba.chunks_exact(across * BYTES_PER_PIXEL).enumerate() {
            let y = first_row + row as i64;
            if y < 0 || y >= i64::from(height) {
                continue;
            }
            for (column, pixel) in pixels.chunks_exact(BYTES_PER_PIXEL).enumerate() {
                let x = first_column + column as i64;
                if x < 0 || x >= i64::from(width) {
                    continue;
                }
                let at = (y as usize * width as usize + x as usize) * BYTES_PER_PIXEL;
                if pixel[ALPHA_CHANNEL] > rgba[at + ALPHA_CHANNEL] {
                    rgba[at..at + BYTES_PER_PIXEL].copy_from_slice(pixel);
                }
            }
        }
    }
    Some(Overlay { rgba, size: (width, height), origin: (left, top) })
}

/// What a text object keeps of the bitmap font it draws with: the face, and the line it laid out
/// last.
#[derive(Debug)]
pub(crate) struct BitmapText {
    face: Arc<ScreenFace>,
    /// The registry key of this object's stand-in texture.
    key: String,
    /// The stand-in texture, kept apart from the layout so a line laid out again replaces its
    /// pixels under the same handle.
    handle: Option<TextureId>,
    laid: Option<LaidOut>,
}

impl BitmapText {
    pub(crate) fn new(face: Arc<ScreenFace>) -> BitmapText {
        let key = format!("rbms.skin.text.fallback.{}", NEXT_OVERLAY_SERIAL.fetch_add(1, Ordering::Relaxed));
        BitmapText { face, key, handle: None, laid: None }
    }

    /// The face this text draws with.
    #[cfg(test)]
    pub(crate) fn face(&self) -> &Arc<ScreenFace> {
        &self.face
    }

    /// Lays `line` out and keeps the result: a quad for every glyph the font has, and one texture
    /// for the characters it lacks.
    ///
    /// The quads are kept page by page, in the order of the line within a page, which is the order
    /// the reference draws them in (`BitmapFontCache.draw` goes through its pages one at a time).
    /// Where the quads of two neighbours overlap, as they do in a font made with padding, that
    /// order is which of the two is on top -- an outlined glyph's outline lies over the neighbour
    /// drawn before it.
    fn lay_out<R: Renderer>(&mut self, text: &mut TextContext, r: &mut R, line: &str, spec: LineSpec) {
        let face = &self.face;
        let font = &face.font;
        let scale = spec.scale;
        let em = (font.original_size() * scale).round().max(1.0);
        let max_dim = r.max_texture_size();
        let (laid, scale_x) = font.layout_fitted(line, &mut |ch| face.piece(text, ch, em, scale, max_dim), spec.width, spec.align, spec.fit, scale);
        let squeeze = scale_x / scale;

        let mut quads: Vec<GlyphQuad> = Vec::new();
        let mut pages: BTreeSet<usize> = BTreeSet::new();
        let mut stamps: Vec<Stamp> = Vec::new();
        for run in &laid.runs {
            let top = run.row as f32 * font.line_height() * scale;
            for (pen, piece) in run.placed() {
                match piece {
                    Piece::Glyph { glyph, .. } if inked_page(piece).is_some() => {
                        let at = Rect::new(
                            run.x + pen + glyph.xoffset as f32 * scale_x,
                            top + (glyph.yoffset as f32 - font.ascent()) * scale,
                            glyph.width as f32 * scale_x,
                            glyph.height as f32 * scale,
                        );
                        let src = (glyph.x.max(0) as u32, glyph.y.max(0) as u32, glyph.width as u32, glyph.height as u32);
                        quads.push(GlyphQuad { page: glyph.page, src, at });
                        pages.insert(glyph.page);
                    }
                    Piece::Glyph { .. } => {}
                    Piece::Fallback { ch, .. } => {
                        let glyph = face.fallback(text, *ch, em, max_dim);
                        if let Some(block) = glyph.block {
                            let x = ((run.x + pen) / squeeze + glyph.bearing + block.left).round() as i32;
                            stamps.push(Stamp { x, y: (top + block.top).round() as i32, block });
                        }
                    }
                }
            }
        }

        quads.sort_by_key(|quad| quad.page);

        let overlay = compose_overlay(&stamps, max_dim).and_then(|overlay| {
            let (width, height) = overlay.size;
            let tex = r.register_texture(&self.key, &overlay.rgba, width, height);
            self.handle = Some(tex);
            let at = Rect::new(overlay.origin.0 as f32 * squeeze, overlay.origin.1 as f32, width as f32 * squeeze, height as f32);
            r.texture_size(tex).map(|size| OverlayStamp { tex, size, at })
        });
        self.laid = Some(LaidOut {
            text: line.to_string(),
            sized: (scale.to_bits(), spec.width.to_bits()),
            quads,
            pages: pages.into_iter().collect(),
            overlay,
            textures: None,
        });
    }

    /// Hands the stand-in texture back to the renderer and forgets the line.
    pub(crate) fn release<R: Renderer>(&mut self, r: &mut R) {
        if let Some(handle) = self.handle.take() {
            r.release_texture(handle);
        }
        self.laid = None;
    }
}

/// One pass over a line's glyphs: how far it is moved and what it is tinted.
#[derive(Debug, Clone, Copy)]
struct Pass {
    moved: (f32, f32),
    tint: Color,
}

/// Draws `line` in the bitmap font of `body` into `dst`, a destination on screen, answering whether
/// anything reached the screen. A glyph whose page is still on its way is not drawn; the rest of
/// its line is.
pub(super) fn draw<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &TextBody,
    bitmap: &mut BitmapText,
    dst: Rect,
    line: &str,
) -> bool {
    let scale = body.size as f32 * place.viewport.scale_x() / bitmap.face.font.original_size();
    if !(scale.is_finite() && scale > 0.0 && dst.w.is_finite()) {
        return false;
    }
    let sized = (scale.to_bits(), dst.w.to_bits());
    if !bitmap.laid.as_ref().is_some_and(|laid| laid.holds(r, line, sized)) {
        bitmap.lay_out(ctx.text, r, line, LineSpec { scale, width: dst.w, align: body.block_align(), fit: body.block_fit() });
    }
    let face = &bitmap.face;
    let Some(laid) = bitmap.laid.as_mut() else {
        return false;
    };
    let (textures, settled) = match laid.textures.take() {
        Some(kept) => (kept, true),
        None => face.page_textures(&laid.pages),
    };

    let anchor = anchor_x(body.block_align(), dst);
    let blend = ctx.text.inherited_blend();
    let ink = place.tint;
    let drawn = if face.kind() == BitmapKind::DistanceField {
        let field = Field { ink: body.field, shadow: body.shadow, page: face.font.page_size() };
        draw_field(r, laid, &textures, (anchor, dst.y), blend, ink, field)
    } else {
        let shade = Color { r: ink.r / SHADOW_DIVISOR, g: ink.g / SHADOW_DIVISOR, b: ink.b / SHADOW_DIVISOR, a: ink.a };
        let shadow = (body.shadow != (0.0, 0.0)).then_some(Pass { moved: body.shadow, tint: shade });
        let passes = shadow.into_iter().chain([Pass { moved: (0.0, 0.0), tint: ink }]);
        let mut drawn = false;
        for pass in passes {
            drawn |= draw_pass(r, laid, &textures, (anchor, dst.y), blend, pass);
        }
        drawn
    };
    if settled {
        laid.textures = Some(textures);
    }
    drawn
}

/// What a distance field font draws a line with besides its colour.
#[derive(Debug, Clone, Copy)]
struct Field {
    ink: FieldInk,
    /// How far the shadow is moved right and down, in pixels of the font's page.
    shadow: (f32, f32),
    /// The size the font file says its pages are, or nothing where it does not say.
    page: (f32, f32),
}

/// Draws every glyph of `laid` through the distance field shader, in `tint` and with the outline
/// and the shadow `field` asks for, answering whether any was drawn. `origin` is the line's anchor
/// and the destination's top edge.
///
/// The shadow's offset is measured against the page size the font file states; a file that states
/// none is measured against the page the glyph is on, as the reference falls back to the size of
/// the page it loaded. The stand-ins for the characters the font lacks are not distances, and are
/// drawn over the glyphs as the plain texture they are.
fn draw_field<R: Renderer>(
    r: &mut R,
    laid: &LaidOut,
    textures: &[(usize, Option<SizedTexture>)],
    origin: (f32, f32),
    blend: BlendMode,
    tint: Color,
    field: Field,
) -> bool {
    let mut drawn = false;
    for quad in &laid.quads {
        let Some((tex, size)) = textures.iter().find(|(page, _)| *page == quad.page).and_then(|(_, texture)| *texture) else {
            continue;
        };
        let stated = |stated: f32, loaded: u32| if stated > 0.0 { stated } else { loaded as f32 };
        let style = field.ink.style(field.shadow, (stated(field.page.0, size.0), stated(field.page.1, size.1)));
        let (x, y, width, height) = quad.src;
        let dst = Rect::new(origin.0 + quad.at.x, origin.1 + quad.at.y, quad.at.w, quad.at.h);
        let src = UvRect::from_pixels(x, y, width, height, size.0, size.1);
        r.draw_distance_field_quad(tex, DistanceFieldParams { dst, src, tint, blend, style });
        drawn = true;
    }
    if let Some(overlay) = laid.overlay {
        draw_overlay(r, overlay, origin, blend, Pass { moved: (0.0, 0.0), tint });
        drawn = true;
    }
    drawn
}

/// Draws the texture of a line's stand-in glyphs, moved and tinted as `pass` says.
fn draw_overlay<R: Renderer>(r: &mut R, overlay: OverlayStamp, origin: (f32, f32), blend: BlendMode, pass: Pass) {
    let dst = Rect::new((origin.0 + overlay.at.x).round() + pass.moved.0, (origin.1 + overlay.at.y).round() + pass.moved.1, overlay.at.w, overlay.at.h);
    let filter = if overlay.at.w == overlay.size.0 as f32 { TextureFilter::Nearest } else { TextureFilter::Linear };
    r.draw_textured_quad(overlay.tex, QuadParams { tint: pass.tint, blend, filter, ..QuadParams::new(dst) });
}

/// Draws every glyph of `laid` once, moved and tinted as `pass` says, answering whether any was
/// drawn. `origin` is the line's anchor and the destination's top edge.
fn draw_pass<R: Renderer>(r: &mut R, laid: &LaidOut, textures: &[(usize, Option<SizedTexture>)], origin: (f32, f32), blend: BlendMode, pass: Pass) -> bool {
    let (left, top) = (origin.0 + pass.moved.0, origin.1 + pass.moved.1);
    let mut drawn = false;
    for quad in &laid.quads {
        let Some((tex, size)) = textures.iter().find(|(page, _)| *page == quad.page).and_then(|(_, texture)| *texture) else {
            continue;
        };
        let (x, y, width, height) = quad.src;
        let dst = Rect::new(left + quad.at.x, top + quad.at.y, quad.at.w, quad.at.h);
        let src = UvRect::from_pixels(x, y, width, height, size.0, size.1);
        r.draw_textured_quad(tex, QuadParams { src, tint: pass.tint, blend, filter: TextureFilter::Linear, ..QuadParams::new(dst) });
        drawn = true;
    }
    if let Some(overlay) = laid.overlay {
        draw_overlay(r, overlay, origin, blend, pass);
        drawn = true;
    }
    drawn
}
