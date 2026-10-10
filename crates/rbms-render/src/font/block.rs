//! A skin's text laid out the way the reference lays it out, composed into one bitmap.
//!
//! The reference draws a skin's TrueType text through libGDX: `SkinTextFont` generates the font at
//! the size the document wrote, lays the string out with `GlyphLayout`, and draws it scaled by
//! `destination height / size`. Three things about that layout decide where a line lands, and none
//! of them is what a general text engine does by default, so they are redone here on top of the
//! shaping the engine already has:
//!
//! - A line starts at the ink of its first glyph and ends at the ink of its last, not at their
//!   advances (`BitmapFontData.getGlyphs`). That tight width is what alignment, shrinking and
//!   cutting all measure.
//! - Every advance is a whole number of pixels at the size the font was generated at, and is only
//!   then scaled.
//! - The height a line is hung from is the capital height: the pixel height of the first capital
//!   the font has, again at the generated size (`FreeTypeFontGenerator.generateData`). The
//!   reference reads it off a glyph its hinter has fitted to the pixel grid, so it is a whole
//!   number that depends on that hinter; [`DesignMetrics`] follows the two steps of it that move a
//!   capital's top.
//!
//! What differs on purpose is the raster. The reference scales a bitmap made at the generated size;
//! this renders each glyph at the size it reaches the screen at, so a line is as sharp at 1080 rows
//! as at 720. A character the font does not have is drawn from whichever other font has it, where
//! the reference draws an empty box.

use cosmic_text::{Buffer, CacheKey, Family, FeatureTag, FontFeatures, LayoutGlyph, Metrics, Shaping, SwashContent, SwashImage, Wrap, fontdb};

use super::{Face, TextContext};
use crate::BYTES_PER_PIXEL;

/// The longest edge a composed block is allowed, in pixels. A line that would be longer is cut off
/// there rather than asked of the renderer as one enormous texture.
pub const BLOCK_MAX_DIM: u32 = 8192;

/// The capitals a font's capital height is read from, in the order they are tried
/// (`BitmapFont.BitmapFontData.capChars`).
const CAP_CHARS: [char; 26] =
    ['M', 'N', 'B', 'D', 'C', 'E', 'F', 'K', 'A', 'G', 'H', 'I', 'J', 'L', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'];

/// The small letter whose rounded top the reference's hinter fits to the pixel grid. FreeType reads
/// that height off several small letters and takes their middle; one of them stands in for it here.
const SMALL_ROUND: char = 'o';

/// The size outlines are measured at, large enough that the pixel a measurement is rounded to is a
/// small part of any size a font is generated at.
const MEASURE_PX: f32 = 1024.0;

/// How far past a whole pixel a small letter's top has to reach before the hinter fits it up to the
/// next one rather than down (`af_latin_metrics_scale_dim`: `( scaled + 40 ) & ~63`, in 64ths).
const SMALL_FIT_BIAS: f32 = 40.0 / 64.0;

/// How far that fit may move the top of the em, in pixels, before the hinter gives it up (the same
/// function: `dist &= ~127`).
const SMALL_FIT_LIMIT_PX: f32 = 2.0;

/// The glyph every font keeps for a character it does not have.
const NOTDEF_GLYPH: u16 = 0;

/// The features that merge or swap glyphs by context. The reference maps one character to one
/// glyph, so a line laid out to match it must as well.
const CONTEXTUAL_FEATURES: [FeatureTag; 4] =
    [FeatureTag::STANDARD_LIGATURES, FeatureTag::CONTEXTUAL_LIGATURES, FeatureTag::DISCRETIONARY_LIGATURES, FeatureTag::CONTEXTUAL_ALTERNATES];

/// The table the reference reads kerning from, and the only one: FreeType answers a pair's kerning
/// from a font's `kern` table and knows nothing of the positioning table newer fonts keep theirs in
/// (`FreeTypeFontGenerator.generateData`: `parameter.kerning & face.hasKerning()`).
const KERN_TABLE: u32 = u32::from_be_bytes(*b"kern");

/// A pixel of nothing: white, so a filtered sample at a glyph's edge does not darken towards it,
/// and fully transparent.
const CLEAR_PIXEL: [u8; BYTES_PER_PIXEL] = [u8::MAX, u8::MAX, u8::MAX, 0];

/// The largest value one channel holds, as the wide integer the compositing arithmetic runs in.
const CHANNEL_MAX: u32 = u8::MAX as u32;

/// Which edge of the width it was given a line is pushed against (`SkinText.ALIGN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockAlign {
    Left,
    Center,
    Right,
}

/// What happens to a line longer than the width it was given (`SkinText.OVERFLOW_*` and
/// `wrapping`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFit {
    /// Nothing: the line runs past the width.
    Overflow,
    /// The whole block is squeezed sideways until its longest line is exactly the width.
    Shrink,
    /// The line stops after the last glyph that fits, with nothing put in place of the rest.
    Truncate,
    /// The line is broken into as many as it takes.
    Wrap,
}

/// A block of text to compose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockSpec<'a> {
    pub text: &'a str,
    /// The name [`TextContext::load_font`] handed out for the font.
    pub family: &'a str,
    /// The em size the glyphs are drawn at, in pixels of the target.
    pub em_px: f32,
    /// The size the document says the font is generated at. Advances and the two heights a block
    /// is hung from are whole pixels at this size.
    pub design_px: f32,
    /// The width the block is laid out in, in pixels of the target.
    pub width: f32,
    pub align: BlockAlign,
    pub fit: BlockFit,
    /// The longest edge the caller can hold as a texture.
    pub max_dim: u32,
}

/// A composed block: its pixels and where they sit.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    pub width: u32,
    pub height: u32,
    /// RGBA8, not premultiplied. Glyphs are white, so the block takes its colour from a tint.
    pub rgba: Vec<u8>,
    /// Where the bitmap's left edge lands, measured from the left edge of the width the block was
    /// laid out in, after any squeeze.
    pub left: f32,
    /// Where the bitmap's top edge lands, measured down from the capital line of the first line.
    pub top: f32,
    /// How wide the bitmap is drawn, which is narrower than it is when the block was squeezed.
    pub drawn_width: f32,
    /// The width of the longest line as the reference measures it, after any squeeze.
    pub layout_width: f32,
    /// How many lines the block came to.
    pub lines: usize,
}

/// What a face measures at one generated size, in whole pixels of that size.
///
/// The capital height is the one the reference's font generator reads, which is the height of a
/// capital after FreeType's automatic hinter has fitted it (libGDX generates with
/// `Hinting.AutoMedium`). That hinter does two things to it: it first rescales the whole font
/// vertically so the top of the small letters lands on a pixel row, and then moves the capital's
/// own top edge to the nearest row. Both are followed here from the outline; the rest of the hinter
/// is not, so the result can be a pixel off the reference's at some sizes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DesignMetrics {
    /// Height of the first capital the face has.
    pub(super) cap: f32,
    /// Distance from one baseline to the next.
    pub(super) line: f32,
}

/// How much the reference's hinter stretches a font vertically at `design_px`, given how far above
/// the baseline its small letters reach at that size.
pub(super) fn small_letter_fit(small_top: f32, design_px: f32) -> f32 {
    let fitted = (small_top + SMALL_FIT_BIAS).floor();
    if small_top <= 0.0 || fitted <= 0.0 {
        return 1.0;
    }
    let factor = fitted / small_top;
    if ((factor - 1.0) * design_px).abs() < SMALL_FIT_LIMIT_PX { factor } else { 1.0 }
}

/// One glyph of a line, with its place along it.
struct Placed {
    /// The glyph's pixels, or `None` for one with no ink.
    image: Option<SwashImage>,
    /// Where the glyph's own origin sits along the line.
    origin: f32,
    /// How far above the baseline the glyph's origin sits.
    lift: f32,
    /// Where the pen stands before this glyph.
    start: f32,
    /// Where the pen stands after it.
    end: f32,
    /// Where the glyph's ink ends.
    ink_right: f32,
}

/// One laid-out line.
struct Line {
    glyphs: Vec<Placed>,
    /// From the ink of the first glyph to the ink of the last.
    width: f32,
}

impl Line {
    /// The width of a line that ends at its last glyph's ink, or nothing for an empty one.
    fn measure(&mut self) {
        self.width = self.glyphs.last().map_or(0.0, |last| last.ink_right);
    }

    /// Whether any glyph after the first reaches past `width`, which is what makes the reference
    /// cut a line (`GlyphLayout.setText`: the first glyph is never tested).
    fn overflows(&self, width: f32) -> bool {
        self.glyphs.iter().skip(1).any(|glyph| glyph.ink_right > width)
    }

    /// Cuts the line to the glyphs that fit in `width` (`GlyphLayout.truncate` with an empty
    /// replacement string). A glyph fits when the pen is still inside the width after it; the last
    /// glyph of the line is measured to its ink instead, because that is where a line ends.
    fn truncate(&mut self, width: f32) {
        let last = self.glyphs.len().saturating_sub(1);
        let starts_inside = self.glyphs.first().is_some_and(|first| first.start <= width);
        let fits = |(index, glyph): &(usize, &Placed)| if *index == last { glyph.ink_right <= width } else { glyph.end <= width };
        let kept = if starts_inside { self.glyphs.iter().enumerate().take_while(fits).count() } else { 0 };
        self.glyphs.truncate(kept);
        self.measure();
    }
}

/// How far a line is moved right inside the width it was laid out in.
fn slack(align: BlockAlign, width: f32, line: f32) -> f32 {
    match align {
        BlockAlign::Left => 0.0,
        BlockAlign::Center => (width - line) / 2.0,
        BlockAlign::Right => width - line,
    }
}

/// The shaping features a block is laid out with: everything on but the contextual substitutions,
/// and kerning only for a font the reference would have kerned.
fn plain_features(kerned: bool) -> FontFeatures {
    let mut features = FontFeatures::new();
    for tag in CONTEXTUAL_FEATURES {
        features.disable(tag);
    }
    if !kerned {
        features.disable(FeatureTag::KERNING);
    }
    features
}

/// Composites one straight-alpha pixel over another in place.
fn over(dst: &mut [u8], src: [u8; BYTES_PER_PIXEL]) {
    let alpha = u32::from(src[3]);
    if alpha == 0 {
        return;
    }
    let under = u32::from(dst[3]) * (CHANNEL_MAX - alpha) / CHANNEL_MAX;
    let out = alpha + under;
    for channel in 0..3 {
        let mixed = (u32::from(src[channel]) * alpha + u32::from(dst[channel]) * under) / out;
        dst[channel] = mixed.min(CHANNEL_MAX) as u8;
    }
    dst[3] = out.min(CHANNEL_MAX) as u8;
}

/// The pixel at `index` of a glyph image, as straight-alpha RGBA. A coverage mask is white ink.
fn glyph_pixel(image: &SwashImage, index: usize) -> Option<[u8; BYTES_PER_PIXEL]> {
    match image.content {
        SwashContent::Mask => image.data.get(index).map(|coverage| [u8::MAX, u8::MAX, u8::MAX, *coverage]),
        SwashContent::Color => {
            let at = index * BYTES_PER_PIXEL;
            image.data.get(at..at + BYTES_PER_PIXEL).map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        }
        SwashContent::SubpixelMask => None,
    }
}

/// A glyph image and where its top left corner lands in the block.
struct Stamp<'a> {
    image: &'a SwashImage,
    x: i32,
    y: i32,
}

impl TextContext {
    /// Shapes `text` in `face` at `px`, answering the glyphs of each visual line in order. With a
    /// `wrap` width a line is broken at a space where it has one and inside a word where it has
    /// none.
    pub(super) fn shape_block(&mut self, face: &Face, text: &str, px: f32, wrap: Option<f32>) -> Vec<Vec<LayoutGlyph>> {
        let features = plain_features(self.kerned(face));
        let mut buffer = Buffer::new(&mut self.fs, Metrics::new(px, px));
        buffer.set_wrap(if wrap.is_some() { Wrap::WordOrGlyph } else { Wrap::None });
        buffer.set_size(wrap, None);
        buffer.set_text(text, &face.attrs().font_features(features), Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.fs, false);
        buffer.layout_runs().map(|run| run.glyphs.to_vec()).collect()
    }

    /// Whether the reference kerns `face`, which it does only from a `kern` table. A face the
    /// database cannot find is not kerned.
    pub(super) fn kerned(&mut self, face: &Face) -> bool {
        let query = fontdb::Query { families: &[Family::Name(&face.family)], weight: face.weight, stretch: face.stretch, style: face.style };
        let Some(id) = face.id.or_else(|| self.fs.db().query(&query)) else {
            return false;
        };
        self.fs.get_font(id, face.weight).is_some_and(|font| font.as_swash().table(KERN_TABLE).is_some())
    }

    /// One glyph's pixels at `px`, on the pixel grid.
    ///
    /// Nothing is kept: the block the glyph goes into is what the caller holds on to, and a glyph
    /// this large kept per size would outgrow every other cache the engine has.
    fn raster(&mut self, glyph: &LayoutGlyph, px: f32) -> Option<SwashImage> {
        let (key, ..) = CacheKey::new(glyph.font_id, glyph.glyph_id, px, (0.0, 0.0), glyph.font_weight, glyph.cache_key_flags);
        self.swash.get_image_uncached(&mut self.fs, key)
    }

    /// The glyph `face` draws one letter with, or `None` when it has none for it. With `own`, a
    /// letter another font had to supply does not count, which is how the reference measures a
    /// font: from the font itself.
    fn letter(&mut self, face: &Face, letter: char, own: bool) -> Option<LayoutGlyph> {
        let mut text = [0u8; 4];
        let lines = self.shape_block(face, letter.encode_utf8(&mut text), MEASURE_PX, None);
        let glyph = lines.into_iter().flatten().next()?;
        let from_face = face.id.is_none_or(|id| id == glyph.font_id);
        (glyph.glyph_id != NOTDEF_GLYPH && (from_face || !own)).then_some(glyph)
    }

    /// How far above the baseline a glyph's outline reaches when the font is `px` tall.
    fn rise(&mut self, glyph: &LayoutGlyph, px: f32) -> Option<f32> {
        let top = self.raster(glyph, MEASURE_PX).map(|image| image.placement.top as f32).filter(|top| *top > 0.0)?;
        Some(top * px / MEASURE_PX)
    }

    /// What `face` measures at `design_px`, or `None` when it has no capital to measure.
    pub(super) fn design_metrics(&mut self, name: &str, face: &Face, design_px: f32) -> Option<DesignMetrics> {
        let key = (name.to_string(), design_px.to_bits());
        if let Some(known) = self.design.get(&key) {
            return Some(*known);
        }
        let mut glyph = None;
        for own in [true, false] {
            for capital in CAP_CHARS {
                glyph = glyph.or_else(|| self.letter(face, capital, own));
            }
        }
        let glyph = glyph?;
        let small = self.letter(face, SMALL_ROUND, true).and_then(|small| self.rise(&small, design_px));
        let fit = small.map_or(1.0, |small_top| small_letter_fit(small_top, design_px));
        let cap = (self.rise(&glyph, design_px)? * fit).round().max(1.0);
        let font = self.fs.get_font(glyph.font_id, glyph.font_weight)?;
        let metrics = font.as_swash().metrics(&[]);
        let units = f32::from(metrics.units_per_em.max(1));
        let line = ((metrics.ascent.abs() + metrics.descent.abs() + metrics.leading) * design_px / units).round();
        let measured = DesignMetrics { cap, line };
        self.design.insert(key, measured);
        Some(measured)
    }

    /// Gives every glyph of one shaped line its place along it.
    ///
    /// `scale` is how much larger the glyphs are drawn than the font is generated: an advance is
    /// rounded to a whole pixel at the generated size and then multiplied out, which is what
    /// scaling a font of whole-pixel advances comes to.
    fn place_line(&mut self, glyphs: &[LayoutGlyph], em_px: f32, scale: f32) -> Line {
        let mut pen = 0.0;
        let mut placed: Vec<Placed> = Vec::with_capacity(glyphs.len());
        for glyph in glyphs {
            let image = self.raster(glyph, em_px).filter(|image| image.placement.width > 0 && image.placement.height > 0);
            let ink = image.as_ref().map_or((0.0, 0.0), |image| {
                let left = image.placement.left as f32;
                (left, left + image.placement.width as f32)
            });
            if placed.is_empty() {
                pen = -ink.0;
            }
            let advance = (glyph.w / scale).round() * scale;
            placed.push(Placed {
                image,
                origin: pen + glyph.font_size * glyph.x_offset,
                lift: glyph.font_size * glyph.y_offset,
                start: pen,
                end: pen + advance,
                ink_right: pen + ink.1,
            });
            pen += advance;
        }
        let mut line = Line { glyphs: placed, width: 0.0 };
        line.measure();
        line
    }

    /// Lays `spec` out and composes it, or `None` when nothing of it would be drawn: an empty
    /// string, a size or a width that is not a number, a font with no capital to hang a line from,
    /// or a block that was squeezed or cut down to nothing.
    pub fn compose_block(&mut self, spec: &BlockSpec<'_>) -> Option<TextBlock> {
        let sized = spec.em_px.is_finite() && spec.em_px > 0.0 && spec.design_px.is_finite() && spec.design_px > 0.0;
        if !sized || !spec.width.is_finite() || spec.text.is_empty() {
            return None;
        }
        let face = self.face(spec.family);
        let design = self.design_metrics(spec.family, &face, spec.design_px)?;
        let scale = spec.em_px / spec.design_px;

        let wrap = (spec.fit == BlockFit::Wrap).then_some(spec.width.max(0.0));
        let shaped = self.shape_block(&face, spec.text, spec.em_px, wrap);
        let mut lines: Vec<Line> = Vec::with_capacity(shaped.len());
        for glyphs in &shaped {
            lines.push(self.place_line(glyphs, spec.em_px, scale));
        }
        if spec.fit == BlockFit::Truncate
            && let Some(cut) = lines.iter().position(|line| line.overflows(spec.width))
        {
            lines.truncate(cut + 1);
            lines[cut].truncate(spec.width);
        }

        let widest = lines.iter().map(|line| line.width).fold(0.0, f32::max);
        let squeeze = if spec.fit == BlockFit::Shrink && widest > spec.width { spec.width / widest } else { 1.0 };
        if squeeze <= 0.0 {
            return None;
        }

        let mut stamps: Vec<Stamp<'_>> = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let origin = slack(spec.align, spec.width, line.width * squeeze) / squeeze;
            let baseline = ((design.cap + index as f32 * design.line) * scale).round();
            for glyph in &line.glyphs {
                if let Some(image) = &glyph.image {
                    let x = (origin + glyph.origin).round() as i32 + image.placement.left;
                    let y = (baseline - glyph.lift).round() as i32 - image.placement.top;
                    stamps.push(Stamp { image, x, y });
                }
            }
        }

        let left = stamps.iter().map(|stamp| stamp.x).min()?;
        let top = stamps.iter().map(|stamp| stamp.y).min()?;
        let right = stamps.iter().map(|stamp| stamp.x + stamp.image.placement.width as i32).max()?;
        let bottom = stamps.iter().map(|stamp| stamp.y + stamp.image.placement.height as i32).max()?;
        let limit = spec.max_dim.min(BLOCK_MAX_DIM);
        let width = ((right - left).max(0) as u32).min(limit);
        let height = ((bottom - top).max(0) as u32).min(limit);
        if width == 0 || height == 0 {
            return None;
        }

        let mut rgba = CLEAR_PIXEL.repeat(width as usize * height as usize);
        for stamp in &stamps {
            let across = stamp.image.placement.width as usize;
            for row in 0..stamp.image.placement.height as usize {
                let y = stamp.y - top + row as i32;
                if y < 0 || y >= height as i32 {
                    continue;
                }
                for column in 0..across {
                    let x = stamp.x - left + column as i32;
                    if x < 0 || x >= width as i32 {
                        continue;
                    }
                    let Some(pixel) = glyph_pixel(stamp.image, row * across + column) else {
                        continue;
                    };
                    let at = (y as usize * width as usize + x as usize) * BYTES_PER_PIXEL;
                    over(&mut rgba[at..at + BYTES_PER_PIXEL], pixel);
                }
            }
        }

        Some(TextBlock {
            width,
            height,
            rgba,
            left: left as f32 * squeeze,
            top: top as f32,
            drawn_width: width as f32 * squeeze,
            layout_width: widest * squeeze,
            lines: lines.len(),
        })
    }

    /// Where the caret of a line stands after its first `chars` characters, measured from the left
    /// edge of the width the line is laid out in, in pixels of the target. This is where
    /// [`TextContext::compose_block`] puts the pen at that point of the first line, with the same
    /// alignment and squeeze applied, so a caret drawn there sits between the glyphs that were
    /// composed.
    ///
    /// A character the pen has not reached yet puts the caret at the end of the line. A line with
    /// nothing in it puts it where an empty line is aligned to. `None` for a size or a width that is
    /// not a number, or for a font with no capital to hang a line from, which is where a block of
    /// that spec is not composed either.
    pub fn caret_x(&mut self, spec: &BlockSpec<'_>, chars: usize) -> Option<f32> {
        let sized = spec.em_px.is_finite() && spec.em_px > 0.0 && spec.design_px.is_finite() && spec.design_px > 0.0;
        if !sized || !spec.width.is_finite() {
            return None;
        }
        let face = self.face(spec.family);
        self.design_metrics(spec.family, &face, spec.design_px)?;
        let scale = spec.em_px / spec.design_px;

        let shaped = if spec.text.is_empty() { Vec::new() } else { self.shape_block(&face, spec.text, spec.em_px, None) };
        let Some(glyphs) = shaped.first() else {
            return Some(slack(spec.align, spec.width, 0.0));
        };
        let line = self.place_line(glyphs, spec.em_px, scale);
        let squeeze = if spec.fit == BlockFit::Shrink && line.width > spec.width { spec.width / line.width } else { 1.0 };

        let byte = spec.text.char_indices().nth(chars).map_or(spec.text.len(), |(at, _)| at);
        let pen = glyphs
            .iter()
            .zip(&line.glyphs)
            .find(|(glyph, _)| glyph.start >= byte)
            .map_or_else(|| line.glyphs.last().map_or(0.0, |last| last.end), |(_, placed)| placed.start);
        Some(slack(spec.align, spec.width, line.width * squeeze) + pen.max(0.0) * squeeze)
    }
}
