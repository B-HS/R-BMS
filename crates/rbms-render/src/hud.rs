use crate::font::{draw_text, draw_text_centered, draw_text_right};
use crate::skin::Skin;
use crate::{Color, Rect, Renderer};

const CYAN: Color = Color::rgb(90, 210, 230);

/// Live play-HUD snapshot, filled from the judge engine each frame.
pub struct HudView {
    pub combo: u32,
    pub last_judge: Option<u8>,
    pub last_fast: bool,
    pub fast: u32,
    pub slow: u32,
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub gauge: f32,
    /// IIDX-style green number (note travel time across the visible field, ms) for the current
    /// scroll speed; 0 hides it. Recomputed each frame so it tracks BPM/hi-speed/cover changes.
    pub green_number: f64,
    /// Max attainable EX (notes*2) — the live score-graph's full-scale top.
    pub max_ex: u32,
    /// Local best EX on this chart (for the graph's BEST bar + delta); `None` = no prior play.
    pub best_ex: Option<u32>,
}

/// IIDX-style live "pacemaker" score graph: a vertical panel with A/AA/AAA rank-band lines, a CURRENT
/// (cyan) EX bar that grows toward the top, and a BEST (green) bar, plus a "vs BEST" delta. Drawn in
/// the gap between the field and the BGA.
fn draw_score_graph<R: Renderer>(r: &mut R, x: f32, y: f32, w: f32, h: f32, ex: u32, max_ex: u32, best: Option<u32>) {
    let th = crate::theme::theme();
    r.fill_rect(Rect::new(x, y, w, h), th.panel);
    let ratio = |v: u32| {
        if max_ex > 0 { (v as f32 / max_ex as f32).clamp(0.0, 1.0) } else { 0.0 }
    };
    for (band, label) in [(6.0f32 / 9.0, "A"), (7.0 / 9.0, "AA"), (8.0 / 9.0, "AAA")] {
        let ly = y + h * (1.0 - band);
        r.fill_rect(Rect::new(x, ly, w, 1.0), Color::rgb(74, 74, 92));
        draw_text(r, x + 4.0, ly + 2.0, 1.0, th.text_muted, label);
    }
    let (bar_w, gap) = (26.0, 12.0);
    let bx = x + (w - (bar_w * 2.0 + gap)) * 0.5;
    let cur_top = y + h * (1.0 - ratio(ex));
    r.fill_rect(Rect::new(bx, cur_top, bar_w, (y + h) - cur_top), CYAN);
    if let Some(b) = best {
        let bt = y + h * (1.0 - ratio(b));
        r.fill_rect(Rect::new(bx + bar_w + gap, bt, bar_w, (y + h) - bt), Color::GREEN);
        let d = ex as i64 - b as i64;
        let (txt, col) = if d >= 0 { (format!("+{d}"), Color::GREEN) } else { (format!("{d}"), Color::rgb(230, 90, 70)) };
        draw_text_centered(r, x + w * 0.5, y - 18.0, 1.4, col, &format!("vs BEST {txt}"));
    }
    draw_text_centered(r, bx + bar_w * 0.5, y + h + 2.0, 1.0, CYAN, "NOW");
    if best.is_some() {
        draw_text_centered(r, bx + bar_w + gap + bar_w * 0.5, y + h + 2.0, 1.0, Color::GREEN, "BEST");
    }
}

pub fn render_hud<R: Renderer>(r: &mut R, skin: &Skin, hud: &HudView) {
    let th = crate::theme::theme();
    // Span the HUD across the full note area (both fields in DP), so the single gauge/combo sit over
    // the whole layout rather than one sub-field. `field_right` is the rightmost lane edge.
    let field_x0 = skin.x.iter().copied().fold(f32::MAX, f32::min);
    let field_right = skin.x.iter().zip(&skin.w).map(|(x, w)| x + w).fold(f32::MIN, f32::max);
    let field_w = field_right - field_x0;
    let cx = field_x0 + field_w * 0.5;
    let jy = skin.judge_y;
    let top = skin.top_y;

    // horizontal gauge below the field (0% left .. 100% right) with the clear-threshold tick,
    // IIDX-style. Height, thresholds and colours come from the skin (data-driven).
    let gy = jy + 10.0;
    let gh = skin.gauge_height;
    r.fill_rect(Rect::new(field_x0, gy, field_w, gh), Color::rgb(28, 28, 36));
    let v = (hud.gauge / 100.0).clamp(0.0, 1.0);
    let gcol = if hud.gauge >= skin.gauge_clear_threshold {
        skin.gauge_color_clear
    } else if hud.gauge >= skin.gauge_warn_threshold {
        skin.gauge_color_warn
    } else {
        skin.gauge_color_fail
    };
    r.fill_rect(Rect::new(field_x0, gy, field_w * v, gh), gcol);
    r.fill_rect(Rect::new(field_x0 + field_w * (skin.gauge_clear_threshold / 100.0).clamp(0.0, 1.0), gy - 2.0, 2.0, gh + 4.0), Color::WHITE);
    draw_text_right(r, field_x0 + field_w - 4.0, gy + gh + 4.0, 1.6, gcol, &format!("{}", hud.gauge.round() as i32));
    draw_text(r, field_x0, gy + gh + 4.0, 1.2, th.text_muted, "0");

    // Left info column (score + best + green number), in the left margin clear of the field.
    draw_text(r, 14.0, 14.0, 2.4, Color::WHITE, &format!("EX {}", hud.ex_score));
    if let Some(b) = hud.best_ex {
        draw_text(r, 14.0, 48.0, 1.4, Color::GREEN, &format!("BEST {b}"));
    }
    if hud.green_number > 0.0 {
        draw_text(r, 14.0, 72.0, 1.4, CYAN, &format!("GREEN {}", hud.green_number.round() as i32));
    }

    if hud.combo > 0 {
        draw_text_centered(r, cx, (jy - skin.combo_y_offset).max(top), 5.0, Color::WHITE, &hud.combo.to_string());
    }

    if let Some(j) = hud.last_judge {
        let j = j as usize;
        draw_text_centered(r, cx, (jy - skin.judge_text_y_offset).max(top), 3.0, skin.judge_colors[j], &skin.judge_labels[j]);
        if (1..=3).contains(&j) {
            let (txt, col) = if hud.last_fast { ("FAST", CYAN) } else { ("SLOW", Color::ORANGE) };
            draw_text_centered(r, cx, (jy - skin.fastslow_y_offset).max(top), 2.0, col, txt);
        }
    }

    // Live "pacemaker" score graph in the gap between the field and the BGA.
    let graph_x = field_right + 28.0;
    let graph_right = skin.bga.map(|b| b.x - 16.0).unwrap_or(graph_x + 150.0);
    let graph_w = (graph_right - graph_x).clamp(0.0, 170.0);
    if graph_w >= 80.0 {
        draw_score_graph(r, graph_x, top + 26.0, graph_w, (jy - top - 26.0).max(40.0), hud.ex_score, hud.max_ex, hud.best_ex);
    }

    // Judge counts: above the BGA box (or right of the graph when there is no BGA), always kept
    // right of the field so a wide DP layout can't overlap them.
    let (tx, mut ty) = match skin.bga {
        Some(b) => (b.x.max(field_right + 20.0), top),
        None => (graph_x + graph_w + 20.0, top),
    };
    for i in 0..6 {
        draw_text(r, tx, ty, 1.6, skin.judge_colors[i], &skin.judge_labels_short[i]);
        draw_text(r, tx + 26.0, ty, 1.6, Color::WHITE, &hud.counts[i].to_string());
        ty += 16.0;
    }
    ty += 8.0;
    draw_text(r, tx, ty, 1.6, CYAN, &format!("FAST {}", hud.fast));
    ty += 16.0;
    draw_text(r, tx, ty, 1.6, Color::ORANGE, &format!("SLOW {}", hud.slow));
}
