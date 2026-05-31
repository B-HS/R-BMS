use std::cell::RefCell;
use std::collections::HashMap;

use cosmic_text::{Attrs, Buffer, CacheKey, Color as CtColor, Family, FontSystem, Metrics, Shaping, SwashCache};

use crate::{Color, Rect, Renderer};

/// Bundled default UI font (Inter, SIL OFL 1.1). cosmic-text falls back to installed system
/// fonts for scripts Inter lacks (CJK, Thai, Arabic, …), so any language renders.
const BUNDLED_FONT: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");

/// Legacy `scale` argument (originally one 5×7 glyph cell = `7*scale` px tall) → font pixel
/// size, picked so the existing call sites keep a comparable visual size.
fn px_for(scale: f32) -> f32 {
    (scale * 8.5).round().max(8.0)
}

/// A shaped, laid-out single line: total width plus each glyph's pen position and rasterizer
/// cache key (relative to the draw origin). Cached per `(text, px)` so a string is shaped once.
struct Laid {
    width: f32,
    glyphs: Vec<(i32, i32, CacheKey)>,
}

/// One text context for the (single-threaded) UI: a font database with fallback, a glyph
/// rasterization cache, the resolved default family, and the per-string layout cache.
///
/// The layout cache is nested `px -> text -> Laid` so a cache hit (the common per-frame case) is
/// looked up by `&str` with no key allocation (`String: Borrow<str>`), unlike a `(String, u32)` key.
struct TextEngine {
    fs: FontSystem,
    swash: SwashCache,
    family: String,
    default_family: String,
    cache: HashMap<u32, HashMap<String, Laid>>,
}

impl TextEngine {
    fn new() -> Self {
        let mut fs = FontSystem::new();
        fs.db_mut().load_font_data(BUNDLED_FONT.to_vec());
        let family = fs
            .db()
            .faces()
            .last()
            .and_then(|f| f.families.first().map(|(n, _)| n.clone()))
            .unwrap_or_else(|| "sans-serif".to_string());
        TextEngine { fs, swash: SwashCache::new(), default_family: family.clone(), family, cache: HashMap::new() }
    }

    fn ensure(&mut self, text: &str, px: f32) {
        let pxu = px as u32;
        if self.cache.get(&pxu).is_some_and(|m| m.contains_key(text)) {
            return;
        }
        let mut buf = Buffer::new(&mut self.fs, Metrics::new(px, px * 1.2));
        buf.set_size(None, None);
        let attrs = Attrs::new().family(Family::Name(&self.family));
        buf.set_text(text, &attrs, Shaping::Advanced, None);
        buf.shape_until_scroll(&mut self.fs, false);
        let mut width = 0f32;
        let mut glyphs = Vec::new();
        for run in buf.layout_runs() {
            width = width.max(run.line_w);
            for g in run.glyphs {
                let p = g.physical((0.0, run.line_y), 1.0);
                glyphs.push((p.x, p.y, p.cache_key));
            }
        }
        self.cache.entry(pxu).or_default().insert(text.to_string(), Laid { width, glyphs });
    }

    fn laid(&self, text: &str, px: f32) -> Option<&Laid> {
        self.cache.get(&(px as u32)).and_then(|m| m.get(text))
    }

    fn width(&mut self, text: &str, px: f32) -> f32 {
        self.ensure(text, px);
        self.laid(text, px).map(|l| l.width).unwrap_or(0.0)
    }

    fn draw<R: Renderer>(&mut self, r: &mut R, x: f32, y: f32, color: Color, text: &str, px: f32) {
        self.ensure(text, px);
        let Some(laid) = self.cache.get(&(px as u32)).and_then(|m| m.get(text)) else { return };
        let base = CtColor::rgb(color.r, color.g, color.b);
        let (ox, oy) = (x.round() as i32, y.round() as i32);
        // `laid` borrows self.cache; the loop borrows self.swash/self.fs — disjoint fields. (A
        // horizontal run-length merge here was profiled at only ~11% fewer quads — anti-aliased
        // glyphs have per-pixel varying coverage so same-alpha runs are mostly length 1 — so it is
        // deferred; the real win is a glyph texture atlas, out of P3 scope. See docs/font-cjk-support.)
        for &(gx, gy, ck) in &laid.glyphs {
            self.swash.with_pixels(&mut self.fs, ck, base, |dx, dy, col| {
                let ca = col.a();
                if ca == 0 {
                    return;
                }
                let a = ((ca as u16 * color.a as u16) / 255) as u8;
                r.fill_rect(Rect::new((ox + gx + dx) as f32, (oy + gy + dy) as f32, 1.0, 1.0), Color { r: col.r(), g: col.g(), b: col.b(), a });
            });
        }
    }
}

thread_local! {
    static ENGINE: RefCell<TextEngine> = RefCell::new(TextEngine::new());
}

/// Width in pixels the string will occupy at the given scale (proportional, shaping-aware).
pub fn text_width(text: &str, scale: f32) -> f32 {
    let px = px_for(scale);
    ENGINE.with(|c| c.borrow_mut().width(text, px))
}

/// Draw a left-aligned string with its top-left near `(x, y)`. `scale` keeps its legacy meaning
/// (see [`px_for`]). Any Unicode script the bundled or a system font covers is rendered.
pub fn draw_text<R: Renderer>(r: &mut R, x: f32, y: f32, scale: f32, color: Color, text: &str) {
    let px = px_for(scale);
    ENGINE.with(|c| c.borrow_mut().draw(r, x, y, color, text, px));
}

pub fn draw_text_centered<R: Renderer>(r: &mut R, center_x: f32, y: f32, scale: f32, color: Color, text: &str) {
    draw_text(r, center_x - text_width(text, scale) * 0.5, y, scale, color, text);
}

pub fn draw_text_right<R: Renderer>(r: &mut R, right_x: f32, y: f32, scale: f32, color: Color, text: &str) {
    draw_text(r, right_x - text_width(text, scale), y, scale, color, text);
}

/// Truncate `text` so it fits within `max_width` px at `scale`, appending an ellipsis when clipped.
/// Keeps long titles inside a panel instead of overflowing. Returns the original when it already fits.
pub fn fit_text(text: &str, scale: f32, max_width: f32) -> String {
    if max_width <= 0.0 || text_width(text, scale) <= max_width {
        return text.to_string();
    }
    let ellipsis = '…';
    let ell_w = text_width("…", scale);
    let mut out = String::new();
    let mut acc = 0.0;
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        let w = text_width(ch.encode_utf8(&mut buf), scale);
        if acc + w + ell_w > max_width {
            break;
        }
        out.push(ch);
        acc += w;
    }
    out.push(ellipsis);
    out
}

/// Register an extra font (e.g. a user-chosen TTF/OTF read from disk or fetched as bytes) with
/// the UI font system, returning its family name to pass to [`set_ui_family`]. Returns `None`
/// if the data has no usable face.
pub fn load_font(data: Vec<u8>) -> Option<String> {
    ENGINE.with(|c| {
        let e = &mut *c.borrow_mut();
        e.fs.db_mut().load_font_data(data);
        e.fs.db().faces().last().and_then(|f| f.families.first().map(|(n, _)| n.clone()))
    })
}

/// Make `name` the preferred UI family for subsequent text (missing glyphs still fall back to
/// system fonts). Clears the shaped-glyph cache so the change takes effect.
pub fn set_ui_family(name: &str) {
    ENGINE.with(|c| {
        let e = &mut *c.borrow_mut();
        if e.family != name {
            e.family = name.to_string();
            e.cache.clear();
        }
    });
}

/// Restore the bundled default UI family.
pub fn reset_ui_family() {
    ENGINE.with(|c| {
        let e = &mut *c.borrow_mut();
        if e.family != e.default_family {
            e.family = e.default_family.clone();
            e.cache.clear();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CpuCanvas, Renderer};

    #[test]
    fn fit_text_truncates_with_ellipsis_when_too_wide() {
        let full = "A VERY LONG SONG TITLE THAT OVERFLOWS THE PANEL";
        let wide = text_width(full, 1.4);
        assert_eq!(fit_text(full, 1.4, wide + 10.0), full, "fits unchanged when within width");
        let clipped = fit_text(full, 1.4, wide * 0.4);
        assert!(clipped.ends_with('…'), "clipped text ends with an ellipsis");
        assert!(text_width(&clipped, 1.4) <= wide * 0.4, "clipped text fits the budget");
        assert!(clipped.chars().count() < full.chars().count(), "clipped is shorter");
        // A single glyph wider than the whole budget collapses to just the ellipsis (no overflow, no loop).
        assert_eq!(fit_text("東", 1.4, 1.0), "…");
        assert_eq!(fit_text("anything", 1.4, 0.0), "anything", "non-positive width is a no-op");
    }

    #[test]
    fn shapes_and_rasterizes_multilingual_text() {
        // Proportional, shaping-aware width: positive for shaped scripts, zero for empty.
        assert!(text_width("東方 한국어 ABC", 2.0) > 0.0);
        assert_eq!(text_width("", 2.0), 0.0);
        // The bundled font is loadable and reports a family.
        assert!(load_font(BUNDLED_FONT.to_vec()).is_some());
        // Drawing actually emits visible glyph pixels (full path through fill_rect).
        let mut c = CpuCanvas::new(220, 48);
        c.clear(Color::rgb(0, 0, 0));
        draw_text(&mut c, 4.0, 4.0, 2.4, Color::WHITE, "A가");
        let lit = (0..220 * 48).any(|i| c.pixel_at((i % 220) as u32, (i / 220) as u32).r > 30);
        assert!(lit, "text must rasterize visible pixels");
    }
}
