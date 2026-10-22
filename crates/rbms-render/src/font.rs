mod block;

#[cfg(test)]
mod block_tests;

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hasher;
use std::sync::Arc;

use cosmic_text::{
    Attrs, Buffer, CacheKey, Color as CtColor, Family, FontSystem, LayoutGlyph, Metrics, Shaping, Stretch, Style, SwashCache, SwashContent, SwashImage, Weight,
    fontdb,
};

use crate::glyph_atlas::{GlyphAtlas, GlyphAtlasBinding, glyph_bitmap, packed_rgb};
use crate::{BYTES_PER_PIXEL, BlendMode, Color, QuadParams, Rect, Renderer};

pub use block::{BLOCK_MAX_DIM, BLOCK_MAX_EM, BLOCK_MAX_PIXELS, BlockAlign, BlockFit, BlockSpec, TextBlock};
pub(crate) use block::{CAP_CHARS, CLEAR_PIXEL, block_extent, slack};

/// Bundled default UI font (Inter, SIL OFL 1.1). cosmic-text falls back to installed system
/// fonts for scripts Inter lacks (CJK, Thai, Arabic, …), so any language renders.
///
/// A static holding the bytes themselves rather than a constant or a reference to them, so a binary
/// that names these bytes again -- the player ships the same file inside its default skin pack --
/// points at this one copy instead of carrying its own.
pub static BUNDLED_FONT: [u8; include_bytes!("../../../assets/fonts/Inter-Regular.ttf").len()] = *include_bytes!("../../../assets/fonts/Inter-Regular.ttf");

/// Family requested when a font database somehow exposes no face at all; the generic name lets
/// cosmic-text pick whatever it has rather than panicking.
const FALLBACK_FAMILY: &str = "sans-serif";

/// Locale pinned by [`use_embedded_fonts_only`]. cosmic-text derives font matching and per-script
/// fallback order from the locale, so a fixed one removes the last host-dependent input.
const DETERMINISTIC_LOCALE: &str = "en-US";

/// The locale whose font draws an ideograph no loaded font has, on a host that is not set to
/// Chinese.
///
/// One code point is drawn differently in Japan, Korea and the Chinese-speaking regions, so the
/// engine picks the fallback font for ideographs by locale, and it knows the bare language codes
/// only: a host set to `ko-KR` or `en-US` is answered with the Simplified Chinese font. A chart's
/// ideographs are almost always Japanese, and they stand beside kana that the Japanese font draws
/// whatever the locale, so the Japanese font is asked for instead. Hangul is matched by script and
/// goes to the Korean font either way.
const HAN_LOCALE: &str = "ja";

/// The language code of the locales whose ideographs are left to the engine, which tells the
/// Chinese-speaking regions apart itself.
const CHINESE_LANGUAGE: &str = "zh";

/// The locale the engine matches ideographs under on a host whose locale is `system`.
fn han_locale(system: &str) -> &str {
    let language = system.split(['-', '_']).next().unwrap_or_default();
    if language.eq_ignore_ascii_case(CHINESE_LANGUAGE) { system } else { HAN_LOCALE }
}

/// Marker appended to a string [`TextContext::fit_text`] had to clip.
const ELLIPSIS: &str = "…";

/// Legacy `scale` argument (originally one 5×7 glyph cell = `7*scale` px tall) → font pixel
/// size, picked so the existing call sites keep a comparable visual size.
fn px_for(scale: f32) -> f32 {
    (scale * 8.5).round().max(8.0)
}

/// One face of a font family: what the font database is asked for when a line is shaped.
///
/// A family name alone does not name a face. Two files of one typographic family -- a medium and a
/// black cut, say -- report the same family and differ only in weight, so asking for the family
/// with the default weight would hand back whichever of them sits nearer to regular for both.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Face {
    family: String,
    weight: Weight,
    style: Style,
    stretch: Stretch,
    /// The database entry this face was loaded as, when it came through [`TextContext::load_font`].
    id: Option<fontdb::ID>,
}

impl Face {
    /// The face a bare family name asks for: its regular, upright, unstretched cut.
    fn of_family(family: &str) -> Face {
        Face { family: family.to_string(), weight: Weight::NORMAL, style: Style::Normal, stretch: Stretch::Normal, id: None }
    }

    /// Whether two faces are the same cut of the same family, whichever file each came from.
    fn same_cut(&self, other: &Face) -> bool {
        self.family == other.family && self.weight == other.weight && self.style == other.style && self.stretch == other.stretch
    }

    /// What a shaping run asks the font database for.
    fn attrs(&self) -> Attrs<'_> {
        Attrs::new().family(Family::Name(&self.family)).weight(self.weight).style(self.style).stretch(self.stretch)
    }
}

/// A fingerprint of a font file's bytes, so the same file loaded again is recognised.
fn content_digest(data: &[u8]) -> (usize, u64) {
    let mut hasher = std::hash::DefaultHasher::new();
    hasher.write(data);
    (data.len(), hasher.finish())
}

/// The weight of the type a line of UI text is set in, on the usual 1..=1000 scale.
///
/// The bundled face is a variable font with a weight axis, so any of these is drawn from it
/// without another file: shaping and rasterization both follow the weight asked for. A face that
/// has no such axis is matched to the nearest cut its family has.
///
/// A heavier weight sets wider glyphs, so a line drawn at one weight has to be measured at the same
/// one, which is why it travels with the size in a [`TextStyle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const REGULAR: FontWeight = FontWeight(Weight::NORMAL.0);
    pub const MEDIUM: FontWeight = FontWeight(Weight::MEDIUM.0);
    pub const SEMIBOLD: FontWeight = FontWeight(Weight::SEMIBOLD.0);
    pub const BOLD: FontWeight = FontWeight(Weight::BOLD.0);
}

/// How a line of UI text is set: its legacy `scale` (see [`px_for`]) and the weight of its type.
///
/// The `*_styled` calls take one of these where the plain ones take a bare `scale`. The plain ones
/// set the line in the face's own weight, which for the bundled face is [`FontWeight::REGULAR`],
/// so a regular style draws the same pixels as a plain call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub scale: f32,
    pub weight: FontWeight,
}

impl TextStyle {
    pub const fn new(scale: f32, weight: FontWeight) -> TextStyle {
        TextStyle { scale, weight }
    }

    /// The regular weight at `scale`.
    pub const fn regular(scale: f32) -> TextStyle {
        TextStyle::new(scale, FontWeight::REGULAR)
    }
}

/// The layout cache's weight level for a line that asked for no weight and so is set in the face's
/// own. No real weight is zero.
const OWN_WEIGHT_KEY: u16 = 0;

/// Where a line is kept in the layout cache under its family: its pixel size and the weight it was
/// set in ([`OWN_WEIGHT_KEY`] for the face's own).
type LayoutKey = (u32, u16);

fn layout_key(px: f32, weight: Option<FontWeight>) -> LayoutKey {
    (px as u32, weight.map_or(OWN_WEIGHT_KEY, |w| w.0))
}

/// How far rounding a length to whole target pixels can move it, in pixels.
const ROUNDING_ERROR_PX: f32 = 0.5;

/// The one factor glyphs are rasterized larger by on `r`, or `None` when text is drawn in `r`'s own
/// units: a renderer that is its own target, or one whose axes are scaled unlike each other.
///
/// The smaller of the two axes is taken, so a line rasterized at the target's resolution never
/// outgrows the box it was measured into on either axis.
///
/// The two axes are told apart from a target of the screen's own shape by more than rounding can
/// account for. Each edge of such a target is rounded to whole pixels on its own, as a window's
/// logical size on a scaled display is, so the longer edge is off by up to half a pixel and the
/// shorter by as much, which measured along the longer edge is `longer / shorter` times as many.
/// The scales may therefore differ by that much, over the longer edge, and still be the screen's
/// shape; a window stretched out of shape differs by more and keeps its text drawn the old way,
/// stretched with the rest of the screen.
fn device_text_scale<R: Renderer>(r: &R) -> Option<f32> {
    let (sx, sy) = r.device_scale();
    if sx == 1.0 && sy == 1.0 {
        return None;
    }
    let (w, h) = r.size();
    let (longer, shorter) = (w.max(h).max(1) as f32, w.min(h).max(1) as f32);
    let rounding_slack_px = ROUNDING_ERROR_PX * (1.0 + longer / shorter);
    let uniform = (sx - sy).abs() * longer <= rounding_slack_px;
    (uniform && sx.is_finite() && sy.is_finite() && sx > 0.0 && sy > 0.0).then_some(sx.min(sy))
}

/// A shaped, laid-out single line: total width plus each glyph's pen position and rasterizer
/// cache key (relative to the draw origin), and each glyph as shaped with the baseline of its
/// line, from which its key at any other size is worked out. Cached per `(text, px, weight)` so a
/// string is shaped once.
struct Laid {
    width: f32,
    glyphs: Vec<(i32, i32, CacheKey)>,
    shaped: Vec<(LayoutGlyph, f32)>,
    used: u64,
}

/// One horizontal run of identical-coverage pixels inside a rasterized glyph, relative to the
/// glyph pen origin: `w` contiguous pixels on row `dy` starting at `dx`, all sharing colour
/// `(r,g,b)` and coverage alpha `ca`. Replacing N adjacent 1×1 fills with one `w`×1 fill is
/// pixel-identical (each covered pixel still source-over-blends the same colour at the same alpha),
/// so it just trims the quad count the glyph emits.
struct GlyphRun {
    dx: i32,
    dy: i32,
    w: u16,
    r: u8,
    g: u8,
    b: u8,
    ca: u8,
}

/// A cached glyph rasterization plus the access stamp used to evict the least recently used entry.
struct RunEntry {
    runs: Vec<GlyphRun>,
    used: u64,
}

/// Maximum number of shaped lines (`(text, px)` pairs) kept in the layout cache. The play HUD and
/// the select screen build fresh strings every frame (score, combo, timers), so without a bound the
/// cache grows for the whole session; a few hundred entries covers a frame's live strings many times
/// over while keeping the cache flat.
pub const LAYOUT_CACHE_LIMIT: usize = 512;

/// Maximum number of rasterized `(glyph, colour)` entries kept in the glyph-run cache. Larger than
/// the layout bound because one line contributes one layout entry but many glyph entries.
pub const RUN_CACHE_LIMIT: usize = 4096;

/// Percentage of a cache that survives an eviction sweep. Evicting in one batch down to this share
/// (instead of a single entry per insert) keeps the O(n) sweep amortized to O(1) per insert.
const CACHE_RETAIN_PERCENT: usize = 75;

/// One text context for the (single-threaded) UI: a font database with fallback, a glyph
/// rasterization cache, the resolved default family, and the per-string layout cache.
///
/// The layout cache is nested `family -> (px, weight) -> text -> Laid` so a cache hit (the common per-frame
/// case) is looked up by `&str` with no key allocation (`String: Borrow<str>`), unlike a composite
/// key. Keeping one branch per family lets a skin alternate between fonts without losing either
/// font's shaped lines; the glyph-run and atlas caches need no family level because a
/// [`CacheKey`] already carries the font it was shaped with.
///
/// Owning one of these and passing it to a renderer (via [`crate::RenderCtx`]) is the injected
/// alternative to the thread-local engine behind the free functions in this module.
pub struct TextContext {
    fs: FontSystem,
    swash: SwashCache,
    family: String,
    default_family: String,
    cache: HashMap<String, HashMap<LayoutKey, HashMap<String, Laid>>>,
    /// Rasterized, run-length-merged glyph pixels keyed by `(glyph cache key, packed RGB)`. Built
    /// once per glyph+colour and replayed every frame, so the hot draw path neither re-rasterizes
    /// nor emits one quad per pixel. Survives family changes.
    ///
    /// This is the only copy of a glyph's pixels the draw path keeps, and it is swept to a bound:
    /// the rasterizer's own image cache is never swept and a window drag asks it for a glyph at a
    /// new fractional size every frame, so the draw path rasterizes past it
    /// ([`SwashCache::get_image_uncached`]).
    runs: HashMap<(CacheKey, u32), RunEntry>,
    /// The same rasterizations packed into one texture page, for the textured-quad text path.
    /// Filled lazily by [`TextContext::draw_text_atlas`] and left empty by the default path.
    atlas: GlyphAtlas<(CacheKey, u32)>,
    /// Monotonic access counter stamped onto every cache entry on use; the eviction sweep keeps the
    /// entries with the highest stamps.
    tick: u64,
    /// The face each name [`TextContext::load_font`] handed out stands for.
    faces: HashMap<String, Face>,
    /// The name each loaded font file was given, by the fingerprint of its bytes, so a file loaded
    /// again adds nothing to the database.
    loaded: HashMap<(usize, u64), String>,
    /// What [`block`] measured of a face at one design size, by `(name, size bits)`.
    design: HashMap<(String, u32), block::DesignMetrics>,
    /// The blend the last skin object that set one left the batch in ([`TextContext::inherited_blend`]).
    left_blend: BlendMode,
}

impl Default for TextContext {
    fn default() -> Self {
        TextContext::new()
    }
}

impl TextContext {
    /// Adopt a font system whose database already ends with the family this engine should treat as
    /// its default (the last-loaded face wins, matching how [`load_font`] reports a family).
    fn with_font_system(fs: FontSystem) -> Self {
        let family = fs.db().faces().last().and_then(|f| f.families.first().map(|(n, _)| n.clone())).unwrap_or_else(|| FALLBACK_FAMILY.to_string());
        TextContext {
            fs,
            swash: SwashCache::new(),
            default_family: family.clone(),
            family,
            cache: HashMap::new(),
            runs: HashMap::new(),
            atlas: GlyphAtlas::new(),
            tick: 0,
            faces: HashMap::new(),
            loaded: HashMap::new(),
            design: HashMap::new(),
            left_blend: BlendMode::Alpha,
        }
    }

    /// The face `name` stands for: the one loaded under it, or the regular cut of the family it
    /// spells when nothing was.
    fn face(&self, name: &str) -> Face {
        self.faces.get(name).cloned().unwrap_or_else(|| Face::of_family(name))
    }

    /// The name a freshly loaded face is handed out under.
    ///
    /// The family's own name is the regular cut's, whichever cut of the family is loaded first: it
    /// is also what the built-in screens are set in when nothing else was asked for, so it must
    /// never come to mean a heavier cut because of the order a skin lists its fonts in. Any other
    /// cut is told apart by its weight, and by its style and stretch as well when even that is
    /// taken.
    fn name_for(&self, face: &Face) -> String {
        let taken = |name: &str| self.faces.get(name).is_some_and(|known| !known.same_cut(face));
        let by_weight = format!("{} {}", face.family, face.weight.0);
        if face.same_cut(&Face::of_family(&face.family)) && !taken(&face.family) {
            face.family.clone()
        } else if !taken(&by_weight) {
            by_weight
        } else {
            format!("{by_weight} {:?} {:?}", face.style, face.stretch)
        }
    }

    /// A context over the bundled font plus the host's installed fonts (the app default).
    ///
    /// A character the font in use lacks is drawn from an installed one. Which one draws an
    /// ideograph is decided by [`han_locale`] rather than by the host's locale as it stands.
    pub fn new() -> Self {
        let (locale, mut db) = FontSystem::new().into_locale_and_db();
        db.load_font_data(BUNDLED_FONT.to_vec());
        Self::with_font_system(FontSystem::new_with_locale_and_db(han_locale(&locale).to_string(), db))
    }

    /// Build a context whose database holds nothing but the bundled font, on a pinned locale, so
    /// shaping, font matching and fallback have no host-specific input left. See
    /// [`use_embedded_fonts_only`].
    pub fn embedded_only() -> Self {
        let mut db = fontdb::Database::new();
        db.load_font_data(BUNDLED_FONT.to_vec());
        Self::with_font_system(FontSystem::new_with_locale_and_db(DETERMINISTIC_LOCALE.to_string(), db))
    }

    fn layout_len(&self) -> usize {
        self.cache.values().flat_map(|by_px| by_px.values()).map(|m| m.len()).sum()
    }

    /// Drop the least recently used layout entries until only `CACHE_RETAIN_PERCENT` of the bound
    /// remains, so the next inserts are free again.
    fn evict_layouts(&mut self) {
        let retain = LAYOUT_CACHE_LIMIT * CACHE_RETAIN_PERCENT / 100;
        let mut ages: Vec<(u64, String, LayoutKey, String)> = self
            .cache
            .iter()
            .flat_map(|(family, by_key)| by_key.iter().flat_map(move |(key, m)| m.iter().map(move |(t, l)| (l.used, family.clone(), *key, t.clone()))))
            .collect();
        ages.sort_unstable_by_key(|e| std::cmp::Reverse(e.0));
        for (_, family, key, text) in ages.into_iter().skip(retain) {
            if let Some(m) = self.cache.get_mut(&family).and_then(|by_key| by_key.get_mut(&key)) {
                m.remove(&text);
            }
        }
        for by_px in self.cache.values_mut() {
            by_px.retain(|_, m| !m.is_empty());
        }
        self.cache.retain(|_, by_px| !by_px.is_empty());
    }

    fn ensure(&mut self, text: &str, px: f32, weight: Option<FontWeight>) {
        let key = layout_key(px, weight);
        self.tick += 1;
        let tick = self.tick;
        if let Some(entry) = self.cache.get_mut(self.family.as_str()).and_then(|by_key| by_key.get_mut(&key)).and_then(|m| m.get_mut(text)) {
            entry.used = tick;
            return;
        }
        let face = self.face(&self.family);
        let attrs = match weight {
            Some(weight) => face.attrs().weight(Weight(weight.0)),
            None => face.attrs(),
        };
        let mut buf = Buffer::new(&mut self.fs, Metrics::new(px, px * 1.2));
        buf.set_size(None, None);
        buf.set_text(text, &attrs, Shaping::Advanced, None);
        buf.shape_until_scroll(&mut self.fs, false);
        let mut width = 0f32;
        let mut glyphs = Vec::new();
        let mut shaped = Vec::new();
        for run in buf.layout_runs() {
            width = width.max(run.line_w);
            for g in run.glyphs {
                let p = g.physical((0.0, run.line_y), 1.0);
                glyphs.push((p.x, p.y, p.cache_key));
                shaped.push((g.clone(), run.line_y));
            }
        }
        self.cache.entry(self.family.clone()).or_default().entry(key).or_default().insert(text.to_string(), Laid { width, glyphs, shaped, used: tick });
        if self.layout_len() > LAYOUT_CACHE_LIMIT {
            self.evict_layouts();
        }
    }

    fn laid(&self, text: &str, px: f32, weight: Option<FontWeight>) -> Option<&Laid> {
        self.cache.get(self.family.as_str()).and_then(|by_key| by_key.get(&layout_key(px, weight))).and_then(|m| m.get(text))
    }

    fn width(&mut self, text: &str, px: f32, weight: Option<FontWeight>) -> f32 {
        self.ensure(text, px, weight);
        self.laid(text, px, weight).map(|l| l.width).unwrap_or(0.0)
    }

    /// Draw one shaped line with its top left at `(x, y)` in `r`'s units.
    ///
    /// On a renderer that is its own target, or one stretched out of shape, every glyph is
    /// rasterized at `px` and filled in `r`'s units, which is the only way text was ever drawn.
    /// On one that enlarges both axes alike ([`device_text_scale`]) the same layout is rasterized
    /// at `px` times that factor and filled on the target's own pixels, with the line's origin put
    /// on the nearest of them: the glyphs are as sharp as the target allows, and the line keeps the
    /// width it was measured at. The baseline is the one the small rasterization puts on a whole
    /// pixel, scaled, so a line sits as high in its row as the screen was laid out to show it.
    fn draw<R: Renderer>(&mut self, r: &mut R, at: (f32, f32), color: Color, text: &str, px: f32, weight: Option<FontWeight>) {
        self.ensure(text, px, weight);
        let tick = self.tick;
        let Some(laid) = self.cache.get(self.family.as_str()).and_then(|by_key| by_key.get(&layout_key(px, weight))).and_then(|m| m.get(text)) else {
            return;
        };
        let mut painter = GlyphPainter { fs: &mut self.fs, swash: &mut self.swash, runs: &mut self.runs, tick };
        match device_text_scale(r) {
            None => {
                let origin = (at.0.round() as i32, at.1.round() as i32);
                painter.paint(color, origin, laid.glyphs.iter().copied(), |rect, ink| r.fill_rect(rect, ink));
            }
            Some(scale) => {
                let (sx, sy) = r.device_scale();
                let origin = ((at.0 * sx).round() as i32, (at.1 * sy).round() as i32);
                let glyphs = laid.shaped.iter().map(|(g, line_y)| {
                    let p = g.physical((0.0, line_y.trunc() * scale), scale);
                    (p.x, p.y, p.cache_key)
                });
                painter.paint(color, origin, glyphs, |rect, ink| r.fill_rect_device(rect, ink));
            }
        }
        if self.runs.len() > RUN_CACHE_LIMIT {
            evict_runs(&mut self.runs);
        }
    }

    /// Width in pixels the string occupies at the given legacy `scale` (see [`px_for`]).
    pub fn text_width(&mut self, text: &str, scale: f32) -> f32 {
        self.width(text, px_for(scale), None)
    }

    /// Width in pixels the string occupies in `style`.
    pub fn text_width_styled(&mut self, text: &str, style: TextStyle) -> f32 {
        self.width(text, px_for(style.scale), Some(style.weight))
    }

    /// Draw a left-aligned string with its top-left near `(x, y)`.
    pub fn draw_text<R: Renderer>(&mut self, r: &mut R, x: f32, y: f32, scale: f32, color: Color, text: &str) {
        self.draw(r, (x, y), color, text, px_for(scale), None);
    }

    /// Draw a left-aligned string in `style` with its top-left near `(x, y)`.
    pub fn draw_text_styled<R: Renderer>(&mut self, r: &mut R, x: f32, y: f32, style: TextStyle, color: Color, text: &str) {
        self.draw(r, (x, y), color, text, px_for(style.scale), Some(style.weight));
    }

    /// Draw a string horizontally centred on `center_x`.
    pub fn draw_text_centered<R: Renderer>(&mut self, r: &mut R, center_x: f32, y: f32, scale: f32, color: Color, text: &str) {
        let w = self.text_width(text, scale);
        self.draw_text(r, center_x - w * 0.5, y, scale, color, text);
    }

    /// Draw a string in `style` horizontally centred on `center_x`.
    pub fn draw_text_centered_styled<R: Renderer>(&mut self, r: &mut R, center_x: f32, y: f32, style: TextStyle, color: Color, text: &str) {
        let w = self.text_width_styled(text, style);
        self.draw_text_styled(r, center_x - w * 0.5, y, style, color, text);
    }

    /// Draw a string whose right edge lands on `right_x`.
    pub fn draw_text_right<R: Renderer>(&mut self, r: &mut R, right_x: f32, y: f32, scale: f32, color: Color, text: &str) {
        let w = self.text_width(text, scale);
        self.draw_text(r, right_x - w, y, scale, color, text);
    }

    /// Draw a string in `style` whose right edge lands on `right_x`.
    pub fn draw_text_right_styled<R: Renderer>(&mut self, r: &mut R, right_x: f32, y: f32, style: TextStyle, color: Color, text: &str) {
        let w = self.text_width_styled(text, style);
        self.draw_text_styled(r, right_x - w, y, style, color, text);
    }

    /// Truncate `text` so it fits within `max_width` px at `scale`, appending an ellipsis when
    /// clipped; returns the original when it already fits.
    pub fn fit_text(&mut self, text: &str, scale: f32, max_width: f32) -> String {
        self.fit(text, px_for(scale), None, max_width)
    }

    /// [`TextContext::fit_text`] for a string in `style`, measured in it.
    pub fn fit_text_styled(&mut self, text: &str, style: TextStyle, max_width: f32) -> String {
        self.fit(text, px_for(style.scale), Some(style.weight), max_width)
    }

    fn fit(&mut self, text: &str, px: f32, weight: Option<FontWeight>, max_width: f32) -> String {
        if max_width <= 0.0 || self.width(text, px, weight) <= max_width {
            return text.to_string();
        }
        let ell_w = self.width(ELLIPSIS, px, weight);
        let mut out = String::new();
        let mut acc = 0.0;
        let mut buf = [0u8; 4];
        for ch in text.chars() {
            let w = self.width(ch.encode_utf8(&mut buf), px, weight);
            if acc + w + ell_w > max_width {
                break;
            }
            out.push(ch);
            acc += w;
        }
        out.push_str(ELLIPSIS);
        out
    }

    /// Register an extra font face, returning the name to pass to [`TextContext::set_family`].
    ///
    /// The name is the face's family name when it is the family's regular cut. Any other cut
    /// carries its weight too, so each cut can be asked for on its own, in whichever order they
    /// are loaded.
    ///
    /// Success means the database gained at least one face from `data`; bytes that are not a font
    /// add nothing and yield `None`, leaving the database and every cache untouched. Lines already
    /// shaped under the returned name are dropped so they are laid out with the new face. A file
    /// that was loaded before is recognised by its bytes and answers the name it was given then,
    /// without adding a second copy of it to the database.
    pub fn load_font(&mut self, data: Vec<u8>) -> Option<String> {
        let digest = content_digest(&data);
        if let Some(name) = self.loaded.get(&digest) {
            return Some(name.clone());
        }
        let db = self.fs.db_mut();
        let loaded = db.load_font_source(fontdb::Source::Binary(Arc::new(data)));
        let info = loaded.last().and_then(|id| db.face(*id))?;
        let family = info.families.first().map(|(name, _)| name.clone())?;
        let face = Face { family, weight: info.weight, style: info.style, stretch: info.stretch, id: Some(info.id) };
        let name = self.name_for(&face);
        self.cache.remove(&name);
        self.design.retain(|(known, _), _| *known != name);
        self.faces.insert(name.clone(), face);
        self.loaded.insert(digest, name.clone());
        Some(name)
    }

    /// The blend a skin's text is drawn with, which is whatever the object drawn before it left
    /// behind.
    ///
    /// The reference keeps one blend setting on the batch every skin object draws through, and a
    /// text object is the one kind that never sets it (`SkinTextFont.draw` calls `setType` and
    /// nothing else), so a line of text is blended the way the last image, number or bar was. It is
    /// kept here because this is the one piece of drawing state that outlives a frame, as the
    /// reference's batch does.
    pub fn inherited_blend(&self) -> BlendMode {
        self.left_blend
    }

    /// Records the blend an object that set one has just drawn with ([`TextContext::inherited_blend`]).
    pub fn leave_blend(&mut self, blend: BlendMode) {
        self.left_blend = blend;
    }

    /// Make `name` the preferred family for subsequent text. Lines shaped under other families stay
    /// cached, so alternating between families does not re-shape or re-rasterize anything.
    pub fn set_family(&mut self, name: &str) {
        if self.family != name {
            self.family = name.to_string();
        }
    }

    /// The family a line is drawn in when nothing else was asked for: the bundled face, which the
    /// engine's fallback fills in for any character it lacks.
    pub fn default_family(&self) -> &str {
        &self.default_family
    }

    /// Restore the bundled default family.
    pub fn reset_family(&mut self) {
        if self.family != self.default_family {
            self.family = self.default_family.clone();
        }
    }

    /// Live occupancy as `(layout entries, glyph-run entries)`.
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.layout_len(), self.runs.len())
    }

    /// How many glyphs are packed into the atlas page, and how big that page currently is.
    pub fn glyph_atlas_stats(&self) -> (usize, (u32, u32)) {
        (self.atlas.len(), self.atlas.size())
    }

    /// Empties the glyph page and starts it over at its initial height.
    ///
    /// The engine is a process-wide singleton, so a test that cares which page a line landed on has
    /// to start from a known one; nothing in the player calls this.
    pub fn reset_glyph_atlas(&mut self) {
        self.atlas.clear();
    }

    /// Draw a left-aligned string as one textured quad per glyph, reading from the shared glyph
    /// atlas instead of filling each run of lit pixels.
    ///
    /// `at` is the top left of the line, the same anchor [`TextContext::draw_text`] takes.
    /// `binding` holds the caller's uploaded copy of the page and must belong to `r`. Every glyph
    /// of the line is packed before any of it is drawn, so a page that fills up and restarts
    /// part-way through cannot leave the earlier quads pointing at pixels that have moved.
    ///
    /// Across lines the same protection comes from the binding: a later line that grows or empties
    /// the page is given a page of its own, and the one this line measured against stays uploaded
    /// until [`GlyphAtlasBinding::end_frame`], which a backend that submits a whole frame at the end
    /// has to call once it has.
    ///
    /// The result is pixel-identical to [`TextContext::draw_text`] on an opaque target: the atlas
    /// stores exactly what the rasterizer produced, and tinting by the text's own alpha reproduces
    /// the coverage arithmetic the fill path does by hand.
    pub fn draw_text_atlas<R: Renderer>(&mut self, r: &mut R, binding: &mut GlyphAtlasBinding, at: (f32, f32), scale: f32, color: Color, text: &str) {
        let px = px_for(scale);
        self.ensure(text, px, None);
        let Some(glyphs) = self.laid(text, px, None).map(|l| l.glyphs.clone()) else {
            return;
        };
        let base = CtColor::rgb(color.r, color.g, color.b);
        let rgb = packed_rgb(color);
        for &(_, _, ck) in &glyphs {
            if self.atlas.get(&(ck, rgb)).is_none() {
                let glyph = glyph_bitmap(&mut self.fs, &mut self.swash, ck, base);
                self.atlas.insert((ck, rgb), glyph.width, glyph.height, glyph.left, glyph.top, &glyph.rgba);
            }
        }
        let tex = binding.sync(r, &self.atlas);
        let (page_w, page_h) = self.atlas.size();
        let (ox, oy) = (at.0.round() as i32, at.1.round() as i32);
        let tint = Color { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: color.a };
        for (gx, gy, ck) in glyphs {
            let Some(entry) = self.atlas.get(&(ck, rgb)) else {
                continue;
            };
            if entry.is_empty() {
                continue;
            }
            let dst = Rect::new((ox + gx + entry.left) as f32, (oy + gy + entry.top) as f32, entry.width as f32, entry.height as f32);
            let mut params = QuadParams::new(dst);
            params.src = entry.uv(page_w, page_h);
            params.tint = tint;
            r.draw_textured_quad(tex, params);
        }
    }
}

/// Drop the least recently used glyph-run entries until only `CACHE_RETAIN_PERCENT` of the bound
/// remains. Free function so it can run while `self.cache` is immutably borrowed by the draw loop.
///
/// Callers sweep once a whole line is drawn, never between its glyphs: every glyph inserted by one
/// [`Engine::draw`] shares a single `used` stamp and this sort is unstable, so a mid-line sweep
/// could drop an entry the same call was still about to replay, silently leaving a hole in the text.
fn evict_runs(runs: &mut HashMap<(CacheKey, u32), RunEntry>) {
    let retain = RUN_CACHE_LIMIT * CACHE_RETAIN_PERCENT / 100;
    let mut ages: Vec<(u64, (CacheKey, u32))> = runs.iter().map(|(k, v)| (v.used, *k)).collect();
    ages.sort_unstable_by_key(|e| std::cmp::Reverse(e.0));
    for (_, k) in ages.into_iter().skip(retain) {
        runs.remove(&k);
    }
}

/// What drawing a line's glyphs borrows from a [`TextContext`]: the rasterizer and the glyph-run
/// cache, with the access stamp the line's entries are marked with.
struct GlyphPainter<'a> {
    fs: &'a mut FontSystem,
    swash: &'a mut SwashCache,
    runs: &'a mut HashMap<(CacheKey, u32), RunEntry>,
    tick: u64,
}

impl GlyphPainter<'_> {
    /// Rasterize each of `glyphs` -- a pen position relative to `origin` and the key it is
    /// rasterized under -- through the glyph-run cache, and hand every run of lit pixels to `fill`
    /// as a one pixel tall rectangle in the same units as `origin`, in `color` with the run's
    /// coverage folded into its alpha.
    ///
    /// The cache is not swept here: a sweep in the middle of a line could drop a glyph the same
    /// line still has to draw ([`evict_runs`]).
    fn paint(&mut self, color: Color, origin: (i32, i32), glyphs: impl Iterator<Item = (i32, i32, CacheKey)>, mut fill: impl FnMut(Rect, Color)) {
        let base = CtColor::rgb(color.r, color.g, color.b);
        let rgb = packed_rgb(color);
        let (ox, oy) = origin;
        for (gx, gy, ck) in glyphs {
            let key = (ck, rgb);
            match self.runs.entry(key) {
                std::collections::hash_map::Entry::Occupied(mut slot) => slot.get_mut().used = self.tick,
                std::collections::hash_map::Entry::Vacant(slot) => {
                    let pixels = self.swash.get_image_uncached(self.fs, ck).map(|image| lit_pixels(&image, base)).unwrap_or_default();
                    slot.insert(RunEntry { runs: merge_runs(pixels), used: self.tick });
                }
            }
            let Some(entry) = self.runs.get(&key) else {
                continue;
            };
            for run in &entry.runs {
                let a = ((run.ca as u16 * color.a as u16) / 255) as u8;
                if a == 0 {
                    continue;
                }
                fill(Rect::new((ox + gx + run.dx) as f32, (oy + gy + run.dy) as f32, run.w as f32, 1.0), Color { r: run.r, g: run.g, b: run.b, a });
            }
        }
    }
}

/// The lit pixels `(dx, dy, r, g, b, ca)` of a rasterized glyph, relative to its pen origin: a
/// coverage mask is inked in `base`, a colour image keeps its own colours, and a pixel with no
/// coverage is left out.
fn lit_pixels(image: &SwashImage, base: CtColor) -> Vec<(i32, i32, u8, u8, u8, u8)> {
    let (left, top) = (image.placement.left, -image.placement.top);
    let width = image.placement.width.max(1) as usize;
    let place = |index: usize| (left + (index % width) as i32, top + (index / width) as i32);
    match image.content {
        SwashContent::Mask => image
            .data
            .iter()
            .enumerate()
            .filter(|(_, coverage)| **coverage != 0)
            .map(|(index, coverage)| {
                let (dx, dy) = place(index);
                (dx, dy, base.r(), base.g(), base.b(), *coverage)
            })
            .collect(),
        SwashContent::Color => image
            .data
            .chunks_exact(BYTES_PER_PIXEL)
            .enumerate()
            .filter(|(_, pixel)| pixel[3] != 0)
            .map(|(index, pixel)| {
                let (dx, dy) = place(index);
                (dx, dy, pixel[0], pixel[1], pixel[2], pixel[3])
            })
            .collect(),
        SwashContent::SubpixelMask => Vec::new(),
    }
}

/// Merge a glyph's lit pixels `(dx, dy, r, g, b, ca)` into horizontal runs of identical colour and
/// coverage. Pixels are row-major sorted first, then adjacent cells (same row, contiguous `dx`,
/// identical `(r,g,b,ca)`) coalesce. A gap, a colour change, or a coverage change starts a new run,
/// so the union of runs covers exactly the input pixels once each — the merge is pixel-identical to
/// emitting every pixel on its own.
fn merge_runs(mut pixels: Vec<(i32, i32, u8, u8, u8, u8)>) -> Vec<GlyphRun> {
    pixels.sort_unstable_by_key(|p| (p.1, p.0));
    let mut runs: Vec<GlyphRun> = Vec::new();
    for (dx, dy, r, g, b, ca) in pixels {
        if let Some(last) = runs.last_mut()
            && last.dy == dy
            && last.dx + last.w as i32 == dx
            && last.r == r
            && last.g == g
            && last.b == b
            && last.ca == ca
        {
            last.w += 1;
            continue;
        }
        runs.push(GlyphRun { dx, dy, w: 1, r, g, b, ca });
    }
    runs
}

thread_local! {
    static ENGINE: RefCell<TextContext> = RefCell::new(TextContext::new());
}

/// Run `f` against this thread's shared [`TextContext`] — the bridge every global helper below and
/// [`crate::with_render_ctx`] go through. Do not call a global helper from inside `f`: the context is
/// borrowed for the whole closure.
pub fn with_text_context<T>(f: impl FnOnce(&mut TextContext) -> T) -> T {
    ENGINE.with(|c| f(&mut c.borrow_mut()))
}

/// Width in pixels the string will occupy at the given scale (proportional, shaping-aware).
pub fn text_width(text: &str, scale: f32) -> f32 {
    with_text_context(|e| e.text_width(text, scale))
}

/// Live occupancy of the two text caches as `(layout entries, glyph-run entries)`, bounded by
/// [`LAYOUT_CACHE_LIMIT`] and [`RUN_CACHE_LIMIT`]. Always available (not test-only) so an app debug
/// overlay can show cache pressure next to its frame stats.
pub fn cache_stats() -> (usize, usize) {
    with_text_context(|e| e.cache_stats())
}

/// Draw a left-aligned string with its top-left near `(x, y)`. `scale` keeps its legacy meaning
/// (see [`px_for`]). Any Unicode script the bundled or a system font covers is rendered.
pub fn draw_text<R: Renderer>(r: &mut R, x: f32, y: f32, scale: f32, color: Color, text: &str) {
    with_text_context(|e| e.draw_text(r, x, y, scale, color, text));
}

pub fn draw_text_centered<R: Renderer>(r: &mut R, center_x: f32, y: f32, scale: f32, color: Color, text: &str) {
    with_text_context(|e| e.draw_text_centered(r, center_x, y, scale, color, text));
}

pub fn draw_text_right<R: Renderer>(r: &mut R, right_x: f32, y: f32, scale: f32, color: Color, text: &str) {
    with_text_context(|e| e.draw_text_right(r, right_x, y, scale, color, text));
}

/// Truncate `text` so it fits within `max_width` px at `scale`, appending an ellipsis when clipped.
/// Keeps long titles inside a panel instead of overflowing. Returns the original when it already fits.
pub fn fit_text(text: &str, scale: f32, max_width: f32) -> String {
    with_text_context(|e| e.fit_text(text, scale, max_width))
}

/// Width in pixels the string occupies in `style`, measured at its weight.
pub fn text_width_styled(text: &str, style: TextStyle) -> f32 {
    with_text_context(|e| e.text_width_styled(text, style))
}

/// [`draw_text`] in `style`.
pub fn draw_text_styled<R: Renderer>(r: &mut R, x: f32, y: f32, style: TextStyle, color: Color, text: &str) {
    with_text_context(|e| e.draw_text_styled(r, x, y, style, color, text));
}

/// [`draw_text_centered`] in `style`.
pub fn draw_text_centered_styled<R: Renderer>(r: &mut R, center_x: f32, y: f32, style: TextStyle, color: Color, text: &str) {
    with_text_context(|e| e.draw_text_centered_styled(r, center_x, y, style, color, text));
}

/// [`draw_text_right`] in `style`.
pub fn draw_text_right_styled<R: Renderer>(r: &mut R, right_x: f32, y: f32, style: TextStyle, color: Color, text: &str) {
    with_text_context(|e| e.draw_text_right_styled(r, right_x, y, style, color, text));
}

/// [`fit_text`] in `style`, measured at its weight.
pub fn fit_text_styled(text: &str, style: TextStyle, max_width: f32) -> String {
    with_text_context(|e| e.fit_text_styled(text, style, max_width))
}

/// Register an extra font (e.g. a user-chosen TTF/OTF read from disk or fetched as bytes) with
/// the UI font system, returning its family name to pass to [`set_ui_family`]. Returns `None`
/// if the data adds no face to the font database (for example bytes that are not a font).
pub fn load_font(data: Vec<u8>) -> Option<String> {
    with_text_context(|e| e.load_font(data))
}

/// Make `name` the preferred UI family for subsequent text (missing glyphs still fall back to
/// system fonts). Lines shaped under other families stay cached.
pub fn set_ui_family(name: &str) {
    with_text_context(|e| e.set_family(name));
}

/// Rebuild the UI text engine so it can only ever use the bundled font: a fresh, empty font
/// database with just that face and a pinned locale, instead of [`FontSystem::new`]'s system-font
/// scan.
///
/// The app keeps the default engine, where cosmic-text falls back to installed fonts for scripts
/// the bundled face lacks. Pixel-comparison tests call this instead, because that fallback resolves
/// to whichever fonts the host happens to have: the same string then rasterizes differently on
/// different machines. With only the bundled face present, an uncovered codepoint resolves to the
/// same notdef box everywhere, which is what determinism needs.
///
/// Affects the calling thread's engine only (the engine is thread-local) and drops every cached
/// layout and glyph run with the old font system.
pub fn use_embedded_fonts_only() {
    ENGINE.with(|c| *c.borrow_mut() = TextContext::embedded_only());
}

/// Restore the bundled default UI family.
pub fn reset_ui_family() {
    with_text_context(TextContext::reset_family);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CpuCanvas, Renderer};

    /// Empty both thread-local caches. Tests share an engine per test thread, so a test that asserts
    /// absolute entry counts starts from a known state instead of whatever ran before it.
    fn clear_caches() {
        use_embedded_fonts_only();
        assert_eq!(cache_stats(), (0, 0), "both caches are empty on a fresh engine");
    }

    #[test]
    fn layout_cache_stays_under_limit_and_keeps_recently_used_entries() {
        clear_caches();
        let overflow = LAYOUT_CACHE_LIMIT + 1000;
        let mut c = CpuCanvas::new(64, 16);
        let name = |i: usize| format!("S{i}");
        for i in 0..overflow {
            draw_text(&mut c, 0.0, 0.0, 1.0, Color::WHITE, &name(i));
        }
        let after_flood = cache_stats().0;
        assert!(after_flood <= LAYOUT_CACHE_LIMIT, "{overflow} distinct strings leave at most {LAYOUT_CACHE_LIMIT} entries (got {after_flood})");

        draw_text(&mut c, 0.0, 0.0, 1.0, Color::WHITE, &name(overflow - 1));
        assert_eq!(cache_stats().0, after_flood, "the newest string is still cached (hit adds no entry)");

        draw_text(&mut c, 0.0, 0.0, 1.0, Color::WHITE, &name(0));
        assert_eq!(cache_stats().0, after_flood + 1, "the oldest string was evicted (miss adds one entry)");
    }

    /// Draw one glyph once per colour until the run cache has overflowed its bound, and report the
    /// colour index used last. The run cache is keyed by (glyph, colour), so every call is a miss.
    fn flood_run_cache(c: &mut CpuCanvas) -> usize {
        let overflow = RUN_CACHE_LIMIT + 200;
        let shade = |i: usize| Color::rgb((i & 0xff) as u8, ((i >> 8) & 0xff) as u8, 7);
        for i in 0..overflow {
            draw_text(c, 0.0, 0.0, 1.0, shade(i), "M");
        }
        overflow - 1
    }

    /// The sweep keeps `CACHE_RETAIN_PERCENT` of the bound and drops the least recently used
    /// entries, the same contract the layout cache test pins.
    #[test]
    fn glyph_run_cache_evicts_down_to_the_retained_share_and_keeps_the_newest_colour() {
        clear_caches();
        let mut c = CpuCanvas::new(32, 24);
        let newest = flood_run_cache(&mut c);
        let entries = cache_stats().1;
        let floor = RUN_CACHE_LIMIT * CACHE_RETAIN_PERCENT / 100;
        assert!(entries <= RUN_CACHE_LIMIT, "an overflowing flood leaves at most {RUN_CACHE_LIMIT} run entries (got {entries})");
        assert!(entries >= floor, "a sweep retains {CACHE_RETAIN_PERCENT}% of the bound, so at least {floor} entries survive (got {entries})");

        let shade = |i: usize| Color::rgb((i & 0xff) as u8, ((i >> 8) & 0xff) as u8, 7);
        draw_text(&mut c, 0.0, 0.0, 1.0, shade(newest), "M");
        assert_eq!(cache_stats().1, entries, "the most recently drawn colour is still cached (hit adds no entry)");
        draw_text(&mut c, 0.0, 0.0, 1.0, shade(0), "M");
        assert_eq!(cache_stats().1, entries + 1, "the oldest colour was evicted (miss adds one entry)");
    }

    /// Eviction runs once the line is finished, so a sweep can never drop a glyph the same call is
    /// still about to paint: text drawn against a full cache is pixel-identical to text drawn cold.
    #[test]
    fn a_draw_under_run_cache_pressure_still_paints_every_glyph() {
        let text = "GOLDEN 0123";
        let mut cold = CpuCanvas::new(160, 24);
        draw_text(&mut cold, 2.0, 2.0, 1.4, Color::WHITE, text);

        let mut scratch = CpuCanvas::new(32, 24);
        flood_run_cache(&mut scratch);

        let mut hot = CpuCanvas::new(160, 24);
        draw_text(&mut hot, 2.0, 2.0, 1.4, Color::WHITE, text);
        assert_eq!(cold.pixels(), hot.pixels(), "a full run cache does not silently drop glyphs from the line");
    }

    #[test]
    fn switching_families_back_and_forth_keeps_every_familys_cache() {
        clear_caches();
        let unloaded = "a family that is definitely not loaded";
        let mut c = CpuCanvas::new(64, 16);
        draw_text(&mut c, 0.0, 0.0, 1.0, Color::WHITE, "CACHED");
        let after_default = cache_stats();
        assert!(after_default.0 > 0 && after_default.1 > 0, "drawing populates both caches ({after_default:?})");

        set_ui_family(unloaded);
        assert_eq!(cache_stats(), after_default, "a family switch drops nothing");
        draw_text(&mut c, 0.0, 0.0, 1.0, Color::WHITE, "CACHED");
        let after_second = cache_stats();
        assert_eq!(after_second.0, after_default.0 + 1, "the second family shapes the string once under its own branch");

        reset_ui_family();
        draw_text(&mut c, 0.0, 0.0, 1.0, Color::WHITE, "CACHED");
        assert_eq!(cache_stats(), after_second, "returning to the first family hits its cache (no new layout, no new glyph run)");

        set_ui_family(unloaded);
        draw_text(&mut c, 0.0, 0.0, 1.0, Color::WHITE, "CACHED");
        assert_eq!(cache_stats(), after_second, "and the second family still hits as well");
        reset_ui_family();
    }

    #[test]
    fn load_font_rejects_bytes_that_are_not_a_font_without_disturbing_later_loads() {
        clear_caches();
        let faces_before = with_text_context(|e| e.fs.db().len());
        assert_eq!(load_font(b"this is definitely not a font file".to_vec()), None);
        assert_eq!(load_font(Vec::new()), None);
        assert_eq!(with_text_context(|e| e.fs.db().len()), faces_before, "rejected data adds no face");

        let family = load_font(BUNDLED_FONT.to_vec()).expect("a real font still loads after a rejected one");
        assert_eq!(with_text_context(|e| e.fs.db().len()), faces_before + 1, "the real font adds exactly one face");
        set_ui_family(&family);
        assert!(text_width("OK", 2.0) > 0.0);
        reset_ui_family();
    }

    #[test]
    fn fit_text_truncates_with_ellipsis_when_too_wide() {
        let full = "A VERY LONG SONG TITLE THAT OVERFLOWS THE PANEL";
        let wide = text_width(full, 1.4);
        assert_eq!(fit_text(full, 1.4, wide + 10.0), full, "fits unchanged when within width");
        let clipped = fit_text(full, 1.4, wide * 0.4);
        assert!(clipped.ends_with('…'), "clipped text ends with an ellipsis");
        assert!(text_width(&clipped, 1.4) <= wide * 0.4, "clipped text fits the budget");
        assert!(clipped.chars().count() < full.chars().count(), "clipped is shorter");
        assert_eq!(fit_text("東", 1.4, 1.0), "…");
        assert_eq!(fit_text("anything", 1.4, 0.0), "anything", "non-positive width is a no-op");
    }

    #[test]
    fn merge_runs_coalesces_only_identical_contiguous_pixels() {
        let c = (200u8, 100u8, 50u8);
        let pixels = vec![
            (0, 0, c.0, c.1, c.2, 180),
            (2, 0, c.0, c.1, c.2, 180),
            (1, 0, c.0, c.1, c.2, 180),
            (3, 0, c.0, c.1, c.2, 90),
            (5, 0, c.0, c.1, c.2, 90),
            (0, 1, c.0, c.1, c.2, 180),
        ];
        let total: usize = pixels.len();
        let runs = merge_runs(pixels);
        assert_eq!(runs.len(), 4, "runs split on gap, coverage change and row change");
        let merged = runs.iter().find(|r| r.dy == 0 && r.dx == 0).unwrap();
        assert_eq!(merged.w, 3, "the three contiguous equal pixels coalesce into one width-3 run");
        assert_eq!(runs.iter().map(|r| r.w as usize).sum::<usize>(), total);
    }

    /// A glyph image of `width` x `height` pixels whose top left corner lies `left` right of the pen
    /// and `top` above the baseline.
    fn glyph_image(content: SwashContent, (left, top): (i32, i32), (width, height): (u32, u32), data: Vec<u8>) -> SwashImage {
        SwashImage { content, placement: cosmic_text::Placement { left, top, width, height }, data, ..SwashImage::default() }
    }

    #[test]
    fn lit_pixels_ink_a_mask_in_the_base_colour_and_keep_a_colour_image_as_it_is() {
        let ink = CtColor::rgb(10, 20, 30);
        let mask = glyph_image(SwashContent::Mask, (-1, 5), (3, 2), vec![0, 10, 0, 255, 0, 20]);
        assert_eq!(
            lit_pixels(&mask, ink),
            vec![(0, -5, 10, 20, 30, 10), (-1, -4, 10, 20, 30, 255), (1, -4, 10, 20, 30, 20)],
            "a mask is inked and its empty cells left out"
        );

        let colour = glyph_image(SwashContent::Color, (2, 3), (2, 1), vec![1, 2, 3, 255, 9, 9, 9, 0]);
        assert_eq!(lit_pixels(&colour, ink), vec![(2, -3, 1, 2, 3, 255)], "a colour image keeps its own colours and its clear pixel is left out");

        let subpixel = glyph_image(SwashContent::SubpixelMask, (0, 0), (1, 1), vec![255, 255, 255]);
        assert!(lit_pixels(&subpixel, ink).is_empty(), "a subpixel mask is not drawn");
        assert!(lit_pixels(&glyph_image(SwashContent::Mask, (0, 0), (0, 0), Vec::new()), ink).is_empty(), "an empty image lights nothing");
    }

    #[test]
    fn merge_runs_empty_input_yields_no_runs() {
        let runs = merge_runs(Vec::new());
        assert!(runs.is_empty(), "no pixels -> no runs");
    }

    #[test]
    fn merge_runs_full_contiguous_row_coalesces_to_one_run() {
        let c = (10u8, 20u8, 30u8);
        let pixels: Vec<_> = (0..8).map(|x| (x, 0i32, c.0, c.1, c.2, 255u8)).collect();
        let runs = merge_runs(pixels);
        assert_eq!(runs.len(), 1, "one contiguous run for a full row");
        assert_eq!(runs[0].w, 8);
        assert_eq!((runs[0].dx, runs[0].dy), (0, 0));
        assert_eq!(runs[0].ca, 255);
    }

    #[test]
    fn merge_runs_all_different_colors_never_coalesce() {
        let pixels = vec![(0, 0, 1u8, 0u8, 0u8, 255u8), (1, 0, 2u8, 0u8, 0u8, 255u8), (2, 0, 3u8, 0u8, 0u8, 255u8)];
        let n = pixels.len();
        let runs = merge_runs(pixels);
        assert_eq!(runs.len(), n, "no merging when colour changes every pixel");
        assert!(runs.iter().all(|r| r.w == 1));
    }

    #[test]
    fn merge_runs_preserves_total_pixel_count() {
        let pixels = vec![
            (5, 2, 9u8, 9u8, 9u8, 200u8),
            (6, 2, 9u8, 9u8, 9u8, 200u8),
            (8, 2, 9u8, 9u8, 9u8, 200u8),
            (0, 0, 9u8, 9u8, 9u8, 200u8),
            (1, 0, 9u8, 9u8, 9u8, 100u8),
        ];
        let n = pixels.len();
        let runs = merge_runs(pixels);
        assert_eq!(runs.iter().map(|r| r.w as usize).sum::<usize>(), n, "count preserved");
    }

    #[test]
    fn merge_runs_sorts_rows_then_columns() {
        let pixels = vec![(1, 1, 7u8, 7u8, 7u8, 50u8), (0, 0, 7u8, 7u8, 7u8, 50u8), (1, 0, 7u8, 7u8, 7u8, 50u8), (0, 1, 7u8, 7u8, 7u8, 50u8)];
        let runs = merge_runs(pixels);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].dy, 0);
        assert_eq!(runs[0].w, 2);
        assert_eq!(runs[1].dy, 1);
        assert_eq!(runs[1].w, 2);
    }

    #[test]
    fn text_width_empty_is_zero_nonempty_is_positive() {
        assert_eq!(text_width("", 2.0), 0.0);
        assert!(text_width("X", 2.0) > 0.0);
        assert!(text_width(" ", 2.0) > 0.0, "a space still advances the pen");
    }

    #[test]
    fn text_width_grows_with_scale() {
        let small = text_width("HELLO", 1.0);
        let large = text_width("HELLO", 3.0);
        assert!(large > small, "larger scale -> wider ({small} vs {large})");
    }

    #[test]
    fn text_width_longer_string_is_at_least_as_wide() {
        let base = text_width("AB", 2.0);
        let more = text_width("ABCDEF", 2.0);
        assert!(more >= base, "longer string is at least as wide ({base} vs {more})");
        let one = text_width("M", 2.0);
        let four = text_width("MMMM", 2.0);
        assert!(four > one, "four glyphs wider than one ({one} vs {four})");
    }

    #[test]
    fn text_width_is_deterministic_across_calls() {
        let a = text_width("DETERMINISM", 2.2);
        let b = text_width("DETERMINISM", 2.2);
        assert_eq!(a, b, "same input -> same width (cache + pure shaping)");
    }

    #[test]
    fn fit_text_returns_original_when_it_fits_exactly() {
        let s = "FITS";
        let w = text_width(s, 1.5);
        assert_eq!(fit_text(s, 1.5, w), s, "width exactly equal to budget keeps the string (<=)");
        assert_eq!(fit_text(s, 1.5, w + 0.5), s, "slack budget keeps the string");
    }

    #[test]
    fn fit_text_zero_and_negative_width_is_noop() {
        assert_eq!(fit_text("abc", 1.4, 0.0), "abc", "zero width returns original unchanged");
        assert_eq!(fit_text("abc", 1.4, -10.0), "abc", "negative width returns original unchanged");
    }

    #[test]
    fn fit_text_single_overwide_glyph_collapses_to_ellipsis() {
        let out = fit_text("W", 2.0, 0.5);
        assert_eq!(out, "…");
    }

    #[test]
    fn fit_text_clipped_result_fits_budget_and_ends_with_ellipsis() {
        let full = "THE QUICK BROWN FOX JUMPS OVER";
        let w = text_width(full, 1.4);
        let budget = w * 0.5;
        let clipped = fit_text(full, 1.4, budget);
        assert!(clipped.ends_with('…'), "clipped ends with ellipsis");
        assert!(clipped.chars().count() < full.chars().count(), "clipped is shorter");
        assert!(text_width(&clipped, 1.4) <= budget, "clipped fits within budget");
    }

    #[test]
    fn fit_text_tiny_positive_budget_yields_at_least_an_ellipsis() {
        let out = fit_text("LONG STRING HERE", 1.4, 0.01);
        assert_eq!(out, "…");
        assert!(out.ends_with('…'));
    }

    #[test]
    fn draw_text_centered_and_right_position_relative_to_width() {
        let mut c = CpuCanvas::new(300, 40);
        c.clear(Color::rgb(0, 0, 0));
        draw_text_centered(&mut c, 150.0, 12.0, 2.0, Color::WHITE, "MID");
        let any_left = (0..150).any(|x| (0..40).any(|y| c.pixel_at(x, y).r > 30));
        let any_right = (150..300).any(|x| (0..40).any(|y| c.pixel_at(x, y).r > 30));
        assert!(any_left && any_right, "centered text straddles the center x");

        let mut c2 = CpuCanvas::new(300, 40);
        c2.clear(Color::rgb(0, 0, 0));
        draw_text_right(&mut c2, 299.0, 12.0, 2.0, Color::WHITE, "END");
        let w = text_width("END", 2.0);
        let leftmost = (0..300).find(|&x| (0..40).any(|y| c2.pixel_at(x, y).r > 30));
        if let Some(lx) = leftmost {
            assert!((lx as f32) > 299.0 - w - 6.0, "right-aligned text hugs the right edge (lx={lx})");
        }
    }

    #[test]
    fn draw_text_empty_string_paints_nothing() {
        let mut c = CpuCanvas::new(60, 20);
        c.clear(Color::rgb(0, 0, 0));
        draw_text(&mut c, 5.0, 5.0, 2.0, Color::WHITE, "");
        let lit = (0..60 * 20).any(|i| c.pixel_at((i % 60) as u32, (i / 60) as u32).r > 5);
        assert!(!lit, "empty string emits no glyph pixels");
    }

    /// The whole point of the deterministic mode: nothing but the bundled face is reachable, so no
    /// host-installed font can influence shaping or rasterization.
    #[test]
    fn the_embedded_only_engine_exposes_a_single_face() {
        use_embedded_fonts_only();
        let (faces, family, locale) = ENGINE.with(|c| {
            let e = c.borrow();
            (e.fs.db().len(), e.family.clone(), e.fs.locale().to_string())
        });
        assert_eq!(faces, 1, "only the bundled font is loaded");
        assert_eq!(family, ENGINE.with(|c| c.borrow().default_family.clone()), "the bundled family is the default");
        assert_eq!(locale, DETERMINISTIC_LOCALE, "the locale is pinned, not read from the host");
    }

    /// A codepoint the bundled font lacks (the song-select folder marker) has nowhere to fall back
    /// to in this mode, so it must still paint — as the font's own notdef box, identically on every
    /// host — rather than silently vanish.
    #[test]
    fn embedded_only_paints_a_codepoint_the_bundled_font_lacks() {
        use_embedded_fonts_only();
        let marker = "\u{25B8}";
        assert!(text_width(marker, 2.0) > 0.0, "the missing glyph still advances the pen");
        let mut c = CpuCanvas::new(48, 40);
        c.clear(Color::rgb(0, 0, 0));
        draw_text(&mut c, 4.0, 4.0, 2.4, Color::WHITE, marker);
        let lit = (0..48 * 40).any(|i| c.pixel_at((i % 48) as u32, (i / 48) as u32).r > 30);
        assert!(lit, "an uncovered codepoint rasterizes as a visible notdef");
    }

    #[test]
    fn ideographs_are_matched_as_japanese_unless_the_host_is_chinese() {
        for system in ["ko-KR", "ko", "en-US", "en-KR", "ja-JP", "ja", "de_DE", ""] {
            assert_eq!(han_locale(system), "ja", "{system:?}");
        }
        for system in ["zh-TW", "zh-HK", "zh-CN", "zh-Hant-TW", "zh", "ZH-tw"] {
            assert_eq!(han_locale(system), system, "a Chinese host keeps the font of its own region");
        }
    }

    /// Lines of a busy built-in menu: a title, tabs, a column of names and a column of values, and
    /// hints, in the sizes and colours the built-in screens use.
    const MENU: &[(f32, &str)] = &[
        (2.4, "PLAY OPTIONS"),
        (1.5, "PLAY  JUDGE  DISPLAY  INPUT  AUDIO  ONLINE"),
        (1.3, "HI-SPEED"),
        (1.4, "FIX 1.50x / GREEN 300"),
        (1.3, "LANE COVER"),
        (1.4, "SUDDEN+ 245 / LIFT 0"),
        (1.3, "GAUGE"),
        (1.4, "ASSIST EASY / EASY / NORMAL / HARD / EX-HARD"),
        (1.3, "RANDOM"),
        (1.4, "OFF / MIRROR / RANDOM / R-RANDOM / S-RANDOM"),
        (1.3, "SCRATCH AUTO"),
        (1.4, "ON"),
        (1.3, "JUDGE ALGORITHM"),
        (1.4, "COMBO / DURATION / LOWEST"),
        (1.3, "TARGET SCORE"),
        (1.4, "AAA 0123456789 +-%"),
        (1.2, "SCROLL / ARROWS: MOVE  ENTER: CHANGE  ESC: BACK"),
        (1.2, "abcdefghijklmnopqrstuvwxyz"),
        (1.1, "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        (1.6, "Status: saved"),
        (1.8, "SETTINGS"),
        (1.0, "v0.1.6"),
    ];

    /// The colours the menu's lines take turns in.
    const MENU_INKS: [Color; 3] = [Color::WHITE, Color::GRAY, Color::CYAN];

    /// The size the menu is laid out for.
    const MENU_SPACE: (u32, u32) = (1280, 720);

    /// Draw [`MENU`] onto `target` through the built-in screens' adapter.
    fn draw_menu(target: &mut CpuCanvas) {
        let row = MENU_SPACE.1 as f32 / MENU.len() as f32;
        let mut ui = crate::ScaledRenderer::new(target, MENU_SPACE);
        for (i, (scale, line)) in MENU.iter().enumerate() {
            draw_text(&mut ui, 8.0, i as f32 * row, *scale, MENU_INKS[i % MENU_INKS.len()], line);
        }
    }

    /// Draw [`MENU`] on a fresh engine through the built-in screens' adapter onto a target `factor`
    /// times [`MENU_SPACE`], and answer the occupancy of both caches.
    fn menu_cache_use(factor: u32) -> (usize, usize) {
        use_embedded_fonts_only();
        draw_menu(&mut CpuCanvas::new(MENU_SPACE.0 * factor, MENU_SPACE.1 * factor));
        cache_stats()
    }

    /// Text drawn at the target's resolution is laid out once in the screen's own units, so it adds
    /// no layout entries, and it rasterizes each glyph once per colour and pen position just as the
    /// small rasterization does, so a whole menu at three times the size still leaves most of the
    /// glyph-run bound free: the per-frame strings of a screen are not swept out of the cache by
    /// their own frame.
    #[test]
    fn an_enlarged_menu_fits_both_caches_as_the_small_one_does() {
        let small = menu_cache_use(1);
        for factor in [2, 3] {
            let large = menu_cache_use(factor);
            assert_eq!(large.0, small.0, "at {factor}x the menu shaped {} lines, at 1x {}", large.0, small.0);
            assert!(large.1 * 4 <= RUN_CACHE_LIMIT, "at {factor}x the menu holds {} glyph runs of {RUN_CACHE_LIMIT}", large.1);
            assert!(large.1 <= small.1 * 2, "at {factor}x the menu holds {} glyph runs, at 1x {}", large.1, small.1);
        }
    }

    /// How many frames of a window drag the test below draws, each at a width the one before it did
    /// not have.
    const DRAG_FRAMES: u32 = 48;

    /// A letterboxed viewport's height for a given width: the screen's shape, to the nearest pixel.
    fn letterboxed_height(width: u32) -> u32 {
        (width as f32 * MENU_SPACE.1 as f32 / MENU_SPACE.0 as f32).round() as u32
    }

    /// Dragging a window edge draws every frame at a size the frame before did not have, so every
    /// frame rasterizes its glyphs at a new fractional size. Each of those is kept by the glyph-run
    /// cache, which is swept down to its bound, and must not also be kept by the rasterizer, whose
    /// image cache is never swept.
    #[test]
    fn a_window_drag_rasterizes_past_the_rasterizers_unbounded_image_cache() {
        use_embedded_fonts_only();
        for frame in 0..DRAG_FRAMES {
            let width = MENU_SPACE.0 + 1 + frame;
            draw_menu(&mut CpuCanvas::new(width, letterboxed_height(width)));
        }
        let (images, runs) = with_text_context(|e| (e.swash.image_cache.len(), e.runs.len()));
        assert!(runs <= RUN_CACHE_LIMIT, "{DRAG_FRAMES} drag frames leave {runs} glyph runs of {RUN_CACHE_LIMIT}");
        assert_eq!(images, 0, "{DRAG_FRAMES} drag frames leave {images} rasterized images beside {runs} glyph runs");
    }

    /// The factor [`device_text_scale`] answers on a target of `size` drawn through the built-in
    /// screens' adapter.
    fn text_scale_on(size: (u32, u32)) -> Option<f32> {
        let mut target = CpuCanvas::new(size.0, size.1);
        device_text_scale(&crate::ScaledRenderer::new(&mut target, MENU_SPACE))
    }

    /// The target a window of `factor` times the screen opens as when each axis is rounded to whole
    /// pixels on its own, as a logical size on a scaled display is.
    fn rounded_window(factor: f32) -> (u32, u32) {
        ((MENU_SPACE.0 as f32 * factor).round() as u32, (MENU_SPACE.1 as f32 * factor).round() as u32)
    }

    /// Representative viewports, each a letterboxed or stretched window of the screen's own shape to
    /// the nearest pixel: its two axes agree, so text is rasterized at the target's resolution.
    #[test]
    fn a_viewport_of_the_screens_shape_is_rasterized_at_the_targets_resolution() {
        const VIEWPORTS: [(u32, u32); 6] = [(1366, 768), (1600, 900), (1702, 958), (2560, 1440), (2880, 1620), (3456, 1944)];
        for size in VIEWPORTS {
            let scale = text_scale_on(size).unwrap_or_else(|| panic!("{size:?} was taken for a stretched window"));
            let (sx, sy) = (size.0 as f32 / MENU_SPACE.0 as f32, size.1 as f32 / MENU_SPACE.1 as f32);
            assert_eq!(scale, sx.min(sy), "{size:?} rasterizes at the smaller of its two axes");
        }
        assert_eq!(text_scale_on(MENU_SPACE), None, "a target of the screen's own size has nothing to enlarge");
    }

    /// Every window of one to three times the screen, in hundredths, with each axis rounded alone:
    /// the two axes then differ by up to a pixel and a half over the longer edge, which is rounding
    /// and not a stretch.
    #[test]
    fn a_window_whose_axes_were_rounded_apart_is_still_the_screens_shape() {
        const HUNDREDTHS: std::ops::RangeInclusive<u32> = 101..=300;
        const HUNDREDTHS_PER_UNIT: f32 = 100.0;
        for hundredths in HUNDREDTHS {
            let factor = hundredths as f32 / HUNDREDTHS_PER_UNIT;
            let size = rounded_window(factor);
            assert!(text_scale_on(size).is_some(), "{size:?} (x{factor}) was taken for a stretched window");
        }
    }

    /// A window pulled out of shape is still drawn the old way, stretched with the rest of the screen.
    /// The last of them is a single pixel taller than rounding either edge could explain.
    #[test]
    fn a_window_stretched_out_of_shape_keeps_its_text_drawn_in_the_screens_units() {
        const STRETCHED: [(u32, u32); 5] = [(1920, 720), (2560, 720), (1280, 1080), (1920, 1090), (1920, 1081)];
        for size in STRETCHED {
            assert_eq!(text_scale_on(size), None, "{size:?} was taken for the screen's shape");
        }
    }

    const DEFAULT_PACK_LINE: &str = "HI-SPEED FIX 1.50 / GREEN 300";

    /// A plain line on a fresh engine that has loaded `fonts` as skin fonts, in that order: its
    /// pixels and its width.
    fn plain_line_after_loading(fonts: &[&[u8]]) -> (Vec<u8>, f32) {
        use_embedded_fonts_only();
        for font in fonts {
            load_font(font.to_vec()).expect("a skin font loads");
        }
        let mut canvas = CpuCanvas::new(260, 40);
        canvas.clear(Color::BLACK);
        draw_text(&mut canvas, 4.0, 6.0, 1.8, Color::WHITE, DEFAULT_PACK_LINE);
        (canvas.pixels().to_vec(), text_width(DEFAULT_PACK_LINE, 1.8))
    }

    /// The default skin pack ships Inter in two files, a regular one and a bold one, under the family
    /// the built-in screens are set in. Which of them the pack lists first is nothing the built-in
    /// screens may depend on: they stay in the regular cut either way.
    #[test]
    fn the_built_in_screens_stay_regular_whichever_cut_of_their_family_a_skin_loads_first() {
        let bold: &[u8] = include_bytes!("../../../assets/skins/rbms-default/font/Inter-Bold.ttf");
        let (alone, alone_width) = plain_line_after_loading(&[]);
        let (regular_first, regular_first_width) = plain_line_after_loading(&[&BUNDLED_FONT, bold]);
        let (bold_first, bold_first_width) = plain_line_after_loading(&[bold, &BUNDLED_FONT]);
        let (bold_only, bold_only_width) = plain_line_after_loading(&[bold]);

        assert_eq!(regular_first_width, alone_width, "the regular cut loaded first leaves the line its width");
        assert_eq!(bold_first_width, alone_width, "the bold cut loaded first leaves the line its width");
        assert_eq!(bold_only_width, alone_width, "the bold cut loaded alone leaves the line its width");
        assert!(regular_first == alone && bold_first == alone && bold_only == alone, "and its pixels");

        use_embedded_fonts_only();
        let heavy = load_font(bold.to_vec()).expect("the bold cut loads");
        assert_ne!(heavy, with_text_context(|e| e.default_family().to_string()), "the bold cut does not take the family's own name");
    }

    /// A regular style is the plain call: the same layout, the same pixels.
    #[test]
    fn a_regular_style_draws_what_the_plain_call_draws() {
        use_embedded_fonts_only();
        let mut plain = CpuCanvas::new(200, 32);
        plain.clear(Color::BLACK);
        draw_text(&mut plain, 4.0, 6.0, 1.4, Color::WHITE, "HI-SPEED 1.50");
        let mut styled = CpuCanvas::new(200, 32);
        styled.clear(Color::BLACK);
        draw_text_styled(&mut styled, 4.0, 6.0, TextStyle::regular(1.4), Color::WHITE, "HI-SPEED 1.50");
        assert_eq!(styled.pixels(), plain.pixels());
        assert_eq!(text_width_styled("HI-SPEED 1.50", TextStyle::regular(1.4)), text_width("HI-SPEED 1.50", 1.4));
    }

    /// The sum of the red channel over a canvas: how much ink a white line on black put down.
    fn ink(canvas: &CpuCanvas) -> u64 {
        canvas.pixels().chunks_exact(crate::BYTES_PER_PIXEL).map(|pixel| u64::from(pixel[0])).sum()
    }

    /// The bundled face's weight axis is reached: a bold line is wider and puts down more ink than a
    /// regular one, and each weight keeps a layout of its own, so measuring one never answers with
    /// the other's width.
    #[test]
    fn a_heavier_weight_sets_a_wider_and_darker_line_with_a_layout_of_its_own() {
        clear_caches();
        let text = "HI-SPEED FIX";
        let regular = TextStyle::regular(2.4);
        let bold = TextStyle::new(2.4, FontWeight::BOLD);
        let paint = |style: TextStyle| {
            let mut c = CpuCanvas::new(240, 40);
            c.clear(Color::BLACK);
            draw_text_styled(&mut c, 4.0, 6.0, style, Color::WHITE, text);
            c
        };
        let (thin, heavy) = (paint(regular), paint(bold));
        assert_eq!(cache_stats().0, 2, "each weight shaped the line once under its own key");
        assert!(text_width_styled(text, bold) > text_width_styled(text, regular), "bold advances further");
        assert!(ink(&heavy) * 4 > ink(&thin) * 5, "bold puts down at least a quarter more ink ({} against {})", ink(&heavy), ink(&thin));

        let fitted = fit_text_styled(text, bold, text_width_styled(text, regular));
        assert!(fitted.ends_with(ELLIPSIS), "a bold line is measured bold, so the regular width does not hold it");
    }

    #[test]
    fn shapes_and_rasterizes_multilingual_text() {
        assert!(text_width("東方 한국어 ABC", 2.0) > 0.0);
        assert_eq!(text_width("", 2.0), 0.0);
        assert!(load_font(BUNDLED_FONT.to_vec()).is_some());
        let mut c = CpuCanvas::new(220, 48);
        c.clear(Color::rgb(0, 0, 0));
        draw_text(&mut c, 4.0, 4.0, 2.4, Color::WHITE, "A가");
        let lit = (0..220 * 48).any(|i| c.pixel_at((i % 220) as u32, (i / 220) as u32).r > 30);
        assert!(lit, "text must rasterize visible pixels");
    }
}
