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
    /// Rasterized, run-length-merged glyph pixels keyed by `(glyph cache key, packed RGB)`. Built
    /// once per glyph+colour and replayed every frame, so the hot draw path neither re-runs
    /// `swash.with_pixels` nor emits one quad per pixel. Cleared with `cache` on a family change.
    runs: HashMap<(CacheKey, u32), Vec<GlyphRun>>,
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
        TextEngine { fs, swash: SwashCache::new(), default_family: family.clone(), family, cache: HashMap::new(), runs: HashMap::new() }
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
        let rgb = ((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32;
        let (ox, oy) = (x.round() as i32, y.round() as i32);
        // Disjoint-field reborrows so `laid` (borrowing self.cache) coexists with building the run
        // cache (self.fs/self.swash/self.runs). Each glyph is rasterized + horizontally run-merged
        // once per (glyph, colour); later frames just replay the cached runs into `fill_rect`.
        let (fs, swash, runs_cache) = (&mut self.fs, &mut self.swash, &mut self.runs);
        for &(gx, gy, ck) in &laid.glyphs {
            let key = (ck, rgb);
            if let std::collections::hash_map::Entry::Vacant(slot) = runs_cache.entry(key) {
                let mut pixels: Vec<(i32, i32, u8, u8, u8, u8)> = Vec::new();
                swash.with_pixels(fs, ck, base, |dx, dy, col| {
                    let ca = col.a();
                    if ca != 0 {
                        pixels.push((dx, dy, col.r(), col.g(), col.b(), ca));
                    }
                });
                slot.insert(merge_runs(pixels));
            }
            for run in &runs_cache[&key] {
                let a = ((run.ca as u16 * color.a as u16) / 255) as u8;
                if a == 0 {
                    continue;
                }
                r.fill_rect(Rect::new((ox + gx + run.dx) as f32, (oy + gy + run.dy) as f32, run.w as f32, 1.0), Color { r: run.r, g: run.g, b: run.b, a });
            }
        }
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
            e.runs.clear();
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
            e.runs.clear();
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
    fn merge_runs_coalesces_only_identical_contiguous_pixels() {
        let c = (200u8, 100u8, 50u8); // a fixed glyph colour
        // Row 0: three contiguous pixels at one coverage, then a gap, then a different coverage.
        let pixels = vec![
            (0, 0, c.0, c.1, c.2, 180),
            (2, 0, c.0, c.1, c.2, 180),
            (1, 0, c.0, c.1, c.2, 180), // out of order on purpose — merge sorts first
            (3, 0, c.0, c.1, c.2, 90),  // coverage change -> new run
            (5, 0, c.0, c.1, c.2, 90),  // gap at x=4 -> new run
            (0, 1, c.0, c.1, c.2, 180), // next row -> new run even though same x-start/colour
        ];
        let total: usize = pixels.len();
        let runs = merge_runs(pixels);
        // 0..=2 merge (w=3); x=3 alone (w=1); x=5 alone (w=1); row 1 x=0 alone (w=1) => 4 runs.
        assert_eq!(runs.len(), 4, "runs split on gap, coverage change and row change");
        let merged = runs.iter().find(|r| r.dy == 0 && r.dx == 0).unwrap();
        assert_eq!(merged.w, 3, "the three contiguous equal pixels coalesce into one width-3 run");
        // Every original lit pixel is still covered exactly once (no loss, no double-count).
        assert_eq!(runs.iter().map(|r| r.w as usize).sum::<usize>(), total);
    }

    #[test]
    fn merge_runs_empty_input_yields_no_runs() {
        let runs = merge_runs(Vec::new());
        assert!(runs.is_empty(), "no pixels -> no runs");
    }

    #[test]
    fn merge_runs_full_contiguous_row_coalesces_to_one_run() {
        // A whole row of identical contiguous pixels merges into a single width-N run.
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
        // Adjacent pixels with distinct colours each become their own run despite contiguity.
        let pixels = vec![
            (0, 0, 1u8, 0u8, 0u8, 255u8),
            (1, 0, 2u8, 0u8, 0u8, 255u8),
            (2, 0, 3u8, 0u8, 0u8, 255u8),
        ];
        let n = pixels.len();
        let runs = merge_runs(pixels);
        assert_eq!(runs.len(), n, "no merging when colour changes every pixel");
        assert!(runs.iter().all(|r| r.w == 1));
    }

    #[test]
    fn merge_runs_preserves_total_pixel_count() {
        // Whatever the gaps/colours, the runs must cover exactly the input pixel count.
        let pixels = vec![
            (5, 2, 9u8, 9u8, 9u8, 200u8),
            (6, 2, 9u8, 9u8, 9u8, 200u8),
            (8, 2, 9u8, 9u8, 9u8, 200u8), // gap at x=7
            (0, 0, 9u8, 9u8, 9u8, 200u8),
            (1, 0, 9u8, 9u8, 9u8, 100u8), // coverage change
        ];
        let n = pixels.len();
        let runs = merge_runs(pixels);
        assert_eq!(runs.iter().map(|r| r.w as usize).sum::<usize>(), n, "count preserved");
    }

    #[test]
    fn merge_runs_sorts_rows_then_columns() {
        // Out-of-order input is row-major sorted; verify runs come out ordered and contiguous merge.
        let pixels = vec![
            (1, 1, 7u8, 7u8, 7u8, 50u8),
            (0, 0, 7u8, 7u8, 7u8, 50u8),
            (1, 0, 7u8, 7u8, 7u8, 50u8),
            (0, 1, 7u8, 7u8, 7u8, 50u8),
        ];
        let runs = merge_runs(pixels);
        // Row 0 (0,1) merges to one width-2 run, row 1 (0,1) merges to one width-2 run.
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
        // Appending characters never narrows the line (proportional but monotone in this font).
        let base = text_width("AB", 2.0);
        let more = text_width("ABCDEF", 2.0);
        assert!(more >= base, "longer string is at least as wide ({base} vs {more})");
        // Repeating the same glyph scales roughly linearly upward.
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
        // One glyph wider than the whole tiny budget -> just the ellipsis, no overflow, no infinite loop.
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
        // Positive but smaller than any glyph: loop breaks immediately, output is the lone ellipsis.
        let out = fit_text("LONG STRING HERE", 1.4, 0.01);
        assert_eq!(out, "…");
        assert!(out.ends_with('…'));
    }

    #[test]
    fn draw_text_centered_and_right_position_relative_to_width() {
        // Centered/right helpers reduce to draw_text at a shifted origin; check they paint within bounds.
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
            // Text should sit in the right region: its left edge is near right_x - width.
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
