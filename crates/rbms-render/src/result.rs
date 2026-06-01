use crate::font::{draw_text, draw_text_centered, draw_text_right};
use crate::{Color, Rect, Renderer};

/// Backend-agnostic result snapshot.
pub struct ResultView {
    pub title: String,
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub max_score: u32,
    pub max_combo: u32,
    pub total_notes: u32,
    pub fast: u32,
    pub slow: u32,
    pub gauge: f32,
    pub clear_label: &'static str,
    pub clear_color: Color,
    /// Best EX on this chart before this play (`None` = first play); enables the "vs BEST" delta.
    pub prev_best_ex: Option<u32>,
    /// EX of the immediately previous play (`None` = first play); enables the "vs PREV" delta.
    pub prev_ex: Option<u32>,
    /// Whether to draw the DJ-LEVEL rank bar + score deltas (the SCORE GRAPH option).
    pub show_graph: bool,
}

const JUDGE_COLORS: [Color; 6] = [Color::GREEN, Color::BLUE, Color::YELLOW, Color::ORANGE, Color::RED, Color::rgb(120, 30, 30)];
const JUDGE_NAMES: [&str; 6] = ["PGREAT", "GREAT", "GOOD", "BAD", "POOR", "MISS"];

/// IIDX DJ-LEVEL bands (`F`..`AAA`) with their display colours, low rank first. The band index
/// is the return of `dj_rank`. Rank boundaries are ninths of the maximum EX (the BMS/IIDX standard).
pub const RANK_BANDS: [(&str, Color); 8] = [
    ("F", Color::rgb(150, 80, 80)),
    ("E", Color::rgb(200, 90, 70)),
    ("D", Color::rgb(230, 150, 60)),
    ("C", Color::rgb(230, 210, 70)),
    ("B", Color::rgb(70, 220, 120)),
    ("A", Color::rgb(90, 200, 230)),
    ("AA", Color::rgb(210, 210, 235)),
    ("AAA", Color::rgb(255, 215, 0)),
];

/// Lower rate boundary of each band in `RANK_BANDS` (`F`=0, then `2/9..8/9`), plus a trailing `1.0`.
/// `RANK_BOUNDS[b]..RANK_BOUNDS[b+1]` is band `b`'s EX-rate span — used to size the rank-bar segments.
const RANK_BOUNDS: [f32; 9] = [0.0, 2.0 / 9.0, 3.0 / 9.0, 4.0 / 9.0, 5.0 / 9.0, 6.0 / 9.0, 7.0 / 9.0, 8.0 / 9.0, 1.0];

/// IIDX DJ LEVEL of an EX score as an index into `RANK_BANDS` (0=`F` .. 7=`AAA`). The rate is
/// `ex/max_ex`; bands sit at ninths (`AAA`≥8/9, `AA`≥7/9, … `C`≥4/9, … `F`<2/9).
pub fn dj_rank(ex: u32, max_ex: u32) -> usize {
    if max_ex == 0 {
        return 0;
    }
    let rate = ex as f32 / max_ex as f32;
    RANK_BOUNDS[1..8].iter().rposition(|&b| rate >= b).map(|i| i + 1).unwrap_or(0)
}

/// Draw the DJ-LEVEL rank bar: eight band segments (`F`..`AAA`, widths proportional to each band's
/// EX-rate span) with passed bands bright and future bands dim, plus a white marker at the current
/// EX rate. Backend-agnostic, so both the result screen and the select panel can draw it.
pub fn draw_rank_bar<R: Renderer>(r: &mut R, x: f32, y: f32, w: f32, h: f32, ex: u32, max_ex: u32) {
    let band = dj_rank(ex, max_ex);
    for b in 0..8 {
        let sx = x + RANK_BOUNDS[b] * w;
        let sw = (RANK_BOUNDS[b + 1] - RANK_BOUNDS[b]) * w;
        let (_, col) = RANK_BANDS[b];
        let c = if b <= band { col } else { Color { a: 50, ..col } };
        r.fill_rect(Rect::new(sx, y, (sw - 1.0).max(1.0), h), c);
    }
    let rate = if max_ex > 0 { (ex as f32 / max_ex as f32).clamp(0.0, 1.0) } else { 0.0 };
    r.fill_rect(Rect::new(x + rate * w - 1.5, y - 3.0, 3.0, h + 6.0), Color::WHITE);
}

/// A signed EX delta as `(text, colour)`: green for a gain, red for a loss, grey for no change.
pub fn ex_delta_label(delta: i64) -> (String, Color) {
    if delta > 0 {
        (format!("+{delta}"), Color::GREEN)
    } else if delta < 0 {
        (format!("{delta}"), Color::rgb(230, 90, 70))
    } else {
        ("±0".into(), Color::GRAY)
    }
}

const PGREAT_PINK: Color = Color::rgb(255, 40, 150);

/// IIDX music-result layout: a big DJ-LEVEL rank + clear lamp + rank bar on the left, a full score
/// report (EX / combo / per-judge counts with PGREAT in hot pink / FAST-SLOW / gauge) on the right.
pub fn render_result<R: Renderer>(r: &mut R, view: &ResultView) {
    let th = crate::theme::theme();
    let w = r.size().0 as f32;
    r.clear(th.bg);
    r.fill_rect(Rect::new(0.0, 0.0, w, 52.0), th.topbar);
    if !view.title.is_empty() {
        draw_text_centered(r, w * 0.5, 14.0, 2.0, th.text, &view.title);
    }

    // --- LEFT: DJ LEVEL + clear lamp + rank bar ---
    let lcx = 232.0;
    let (rank, rcol) = RANK_BANDS[dj_rank(view.ex_score, view.max_score)];
    let rate = if view.max_score > 0 { view.ex_score as f32 / view.max_score as f32 * 100.0 } else { 0.0 };
    draw_text_centered(r, lcx, 96.0, 7.0, rcol, rank);
    draw_text_centered(r, lcx, 214.0, 2.4, rcol, &format!("{rate:.2}%"));
    r.fill_rect(Rect::new(44.0, 258.0, 376.0, 48.0), view.clear_color);
    draw_text_centered(r, lcx, 270.0, 2.6, Color::BLACK, view.clear_label);
    if view.show_graph {
        draw_rank_bar(r, 44.0, 340.0, 376.0, 18.0, view.ex_score, view.max_score);
        let mut dy = 384.0;
        match view.prev_best_ex {
            Some(pb) => {
                let (t, c) = ex_delta_label(view.ex_score as i64 - pb as i64);
                draw_text(r, 44.0, dy, 1.8, c, &format!("{t} vs BEST"));
                if view.ex_score > pb {
                    draw_text_right(r, 420.0, dy, 1.8, Color::YELLOW, "NEW RECORD");
                }
            }
            None => draw_text(r, 44.0, dy, 1.8, th.text_muted, "FIRST PLAY"),
        }
        if let Some(pp) = view.prev_ex {
            dy += 28.0;
            let (t, c) = ex_delta_label(view.ex_score as i64 - pp as i64);
            draw_text(r, 44.0, dy, 1.8, c, &format!("{t} vs PREV"));
        }
    }

    // --- RIGHT: score report ---
    let (rx, rr) = (480.0, 1236.0);
    let mut y = 80.0;
    draw_text(r, rx, y, 2.4, th.text, "EX SCORE");
    draw_text_right(r, rr, y, 2.4, th.text, &format!("{} / {}", view.ex_score, view.max_score));
    y += 42.0;
    draw_text(r, rx, y, 2.0, th.text_dim, "MAX COMBO");
    draw_text_right(r, rr, y, 2.0, Color::GREEN, &format!("{} / {}", view.max_combo, view.total_notes));
    y += 30.0;
    draw_text(r, rx, y, 2.0, th.text_dim, "TOTAL NOTES");
    draw_text_right(r, rr, y, 2.0, th.text, &view.total_notes.to_string());
    y += 30.0;
    r.fill_rect(Rect::new(rx, y, rr - rx, 2.0), th.divider);
    y += 16.0;

    let denom = view.total_notes.max(1) as f32;
    for i in 0..6 {
        let col = if i == 0 { PGREAT_PINK } else { JUDGE_COLORS[i] };
        draw_text(r, rx, y, 2.0, col, JUDGE_NAMES[i]);
        draw_text_right(r, rr, y, 2.0, th.text, &view.counts[i].to_string());
        let frac = (view.counts[i] as f32 / denom).min(1.0);
        r.fill_rect(Rect::new(rx, y + 23.0, (rr - rx) * frac, 4.0), col);
        y += 34.0;
    }
    y += 10.0;
    draw_text(r, rx, y, 1.8, th.accent, &format!("FAST {}", view.fast));
    draw_text(r, rx + 170.0, y, 1.8, Color::ORANGE, &format!("SLOW {}", view.slow));
    draw_text_right(r, rr, y, 2.0, view.clear_color, &format!("GAUGE {}%", view.gauge.round() as i32));

    draw_text_centered(r, w * 0.5, 692.0, 1.2, th.text_muted, "ENTER / ESC  SELECT");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dj_rank_bands_match_ninths() {
        // max_ex = 1800 (900 notes) so k/9 lands on whole numbers: AAA=1600, AA=1400, ... C=800.
        let max = 1800;
        assert_eq!(RANK_BANDS[dj_rank(1800, max)].0, "AAA", "100% is AAA");
        assert_eq!(RANK_BANDS[dj_rank(1600, max)].0, "AAA", "exactly 8/9 is AAA");
        assert_eq!(RANK_BANDS[dj_rank(1599, max)].0, "AA", "just under 8/9 is AA");
        assert_eq!(RANK_BANDS[dj_rank(1400, max)].0, "AA", "exactly 7/9 is AA");
        assert_eq!(RANK_BANDS[dj_rank(800, max)].0, "C", "exactly 4/9 is C");
        assert_eq!(RANK_BANDS[dj_rank(799, max)].0, "D", "just under 4/9 is D");
        assert_eq!(RANK_BANDS[dj_rank(0, max)].0, "F", "0% is F");
    }

    #[test]
    fn dj_rank_zero_max_is_lowest() {
        assert_eq!(dj_rank(0, 0), 0, "no notes -> F, no divide-by-zero");
    }

    #[test]
    fn ex_delta_label_signs() {
        assert_eq!(ex_delta_label(12).0, "+12");
        assert_eq!(ex_delta_label(-7).0, "-7");
        assert_eq!(ex_delta_label(0).0, "±0");
    }

    #[test]
    fn ex_delta_label_colors_by_sign() {
        assert_eq!(ex_delta_label(1).1, Color::GREEN, "gain is green");
        assert_eq!(ex_delta_label(-1).1, Color::rgb(230, 90, 70), "loss is red-ish");
        assert_eq!(ex_delta_label(0).1, Color::GRAY, "no change is grey");
    }

    #[test]
    fn ex_delta_label_large_magnitudes() {
        assert_eq!(ex_delta_label(123456).0, "+123456");
        assert_eq!(ex_delta_label(-987654).0, "-987654");
        // i64 extremes do not panic and format with the expected sign.
        assert!(ex_delta_label(i64::MAX).0.starts_with('+'));
        assert!(ex_delta_label(i64::MIN).0.starts_with('-'));
    }

    #[test]
    fn dj_rank_returns_index_into_bands_in_range() {
        // Every possible rate maps to a valid 0..=7 band index.
        let max = 900u32;
        for ex in 0..=max {
            let b = dj_rank(ex, max);
            assert!(b < RANK_BANDS.len(), "band index {b} in range for ex={ex}");
        }
    }

    #[test]
    fn dj_rank_is_monotonic_non_decreasing_in_ex() {
        let max = 1800u32;
        let mut prev = 0usize;
        for ex in 0..=max {
            let b = dj_rank(ex, max);
            assert!(b >= prev, "rank never decreases as ex grows (ex={ex} band={b} prev={prev})");
            prev = b;
        }
        assert_eq!(prev, 7, "full EX reaches the top band");
    }

    #[test]
    fn dj_rank_every_ninth_boundary_is_exact() {
        // max=1800 so each k/9 is a whole number; verify all eight band names at their lower bound.
        let max = 1800u32;
        let expect = [
            (0, "F"),
            (400, "E"),   // 2/9
            (600, "D"),   // 3/9
            (800, "C"),   // 4/9
            (1000, "B"),  // 5/9
            (1200, "A"),  // 6/9
            (1400, "AA"), // 7/9
            (1600, "AAA"),// 8/9
        ];
        for (ex, name) in expect {
            assert_eq!(RANK_BANDS[dj_rank(ex, max)].0, name, "ex {ex}/{max} -> {name}");
            if ex > 0 {
                assert_ne!(RANK_BANDS[dj_rank(ex - 1, max)].0, name, "one below the boundary is a lower band");
            }
        }
    }

    #[test]
    fn dj_rank_ex_above_max_clamps_to_top_band() {
        // ex > max gives rate > 1.0 which still satisfies rate >= 8/9 -> AAA (no overflow).
        assert_eq!(RANK_BANDS[dj_rank(5000, 1800)].0, "AAA");
    }

    #[test]
    fn dj_rank_zero_max_ignores_ex() {
        // max_ex == 0 short-circuits to F regardless of ex (no divide-by-zero).
        assert_eq!(dj_rank(0, 0), 0);
        assert_eq!(dj_rank(999, 0), 0);
    }

    #[test]
    fn rank_bounds_are_sorted_and_span_unit_interval() {
        assert_eq!(RANK_BOUNDS[0], 0.0);
        assert_eq!(RANK_BOUNDS[8], 1.0);
        for w in RANK_BOUNDS.windows(2) {
            assert!(w[1] > w[0], "bounds strictly increasing ({} -> {})", w[0], w[1]);
        }
        assert_eq!(RANK_BOUNDS.len(), RANK_BANDS.len() + 1, "one more bound than bands");
    }

    #[test]
    fn draw_rank_bar_runs_without_panic_for_edge_inputs() {
        use crate::CpuCanvas;
        let mut c = CpuCanvas::new(400, 40);
        // zero max (rate path guarded), full score, and partial score all paint inside bounds.
        draw_rank_bar(&mut c, 10.0, 10.0, 380.0, 18.0, 0, 0);
        draw_rank_bar(&mut c, 10.0, 10.0, 380.0, 18.0, 1800, 1800);
        draw_rank_bar(&mut c, 10.0, 10.0, 380.0, 18.0, 900, 1800);
        // The bar drew something visible somewhere on the canvas.
        let lit = (0..400 * 40).any(|i| c.pixel_at((i % 400) as u32, (i / 400) as u32).a == 255);
        assert!(lit, "rank bar paints pixels");
    }

    #[test]
    fn render_result_smoke_first_play_and_with_deltas() {
        use crate::CpuCanvas;
        // First play (no prev) with the graph shown.
        let view = ResultView {
            title: "TEST SONG".into(),
            counts: [100, 20, 5, 2, 1, 3],
            ex_score: 220,
            max_score: 262,
            max_combo: 120,
            total_notes: 131,
            fast: 7,
            slow: 9,
            gauge: 88.4,
            clear_label: "CLEAR",
            clear_color: Color::GREEN,
            prev_best_ex: None,
            prev_ex: None,
            show_graph: true,
        };
        let mut c = CpuCanvas::new(1280, 720);
        render_result(&mut c, &view);
        let lit = (0..1280 * 720).any(|i| c.pixel_at((i % 1280) as u32, (i / 1280) as u32).r > 40);
        assert!(lit, "result screen renders visible content");

        // With both deltas (a new record) and an empty title — must not panic.
        let view2 = ResultView { title: String::new(), prev_best_ex: Some(200), prev_ex: Some(215), ..view };
        let mut c2 = CpuCanvas::new(1280, 720);
        render_result(&mut c2, &view2);
    }

    #[test]
    fn render_result_handles_zero_notes_without_divide_by_zero() {
        use crate::CpuCanvas;
        // total_notes==0 exercises the denom.max(1) guard and max_score==0 rate guard.
        let view = ResultView {
            title: "EMPTY".into(),
            counts: [0; 6],
            ex_score: 0,
            max_score: 0,
            max_combo: 0,
            total_notes: 0,
            fast: 0,
            slow: 0,
            gauge: 0.0,
            clear_label: "FAILED",
            clear_color: Color::RED,
            prev_best_ex: None,
            prev_ex: None,
            show_graph: true,
        };
        let mut c = CpuCanvas::new(1280, 720);
        render_result(&mut c, &view);
    }
}
