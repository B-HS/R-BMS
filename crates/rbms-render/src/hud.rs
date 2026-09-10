use crate::ctx::{RenderCtx, with_render_ctx};
use crate::result::{KEY_LANE_KIND, LANE_KIND_COUNT, SCRATCH_LANE_KIND, lane_kind_total};
use crate::skin::Skin;
use crate::{Color, Rect, Renderer};

const CYAN: Color = Color::rgb(90, 210, 230);

/// A judge-text position of this value means "wherever the skin puts it", so a player who has not
/// moved the row keeps the placement the skin was authored with.
const JUDGE_TEXT_FROM_SKIN: f32 = 0.0;

/// How far right of a FAST/SLOW total its key/scratch breakdown is drawn.
const FASTSLOW_SPLIT_X: f32 = 96.0;

/// Text scale of the key/scratch breakdown, smaller than the total it belongs to.
const FASTSLOW_SPLIT_SCALE: f32 = 1.2;

/// Width of the reference screen the HUD is laid out in, matching the skin's own coordinate space.
const CW_REFERENCE: f32 = 1280.0;

/// Distance from the right edge of the screen to the layout name. The result screen reports the
/// same layout in the same place, so the two screens share the placement.
pub(crate) const MODE_LABEL_MARGIN: f32 = 14.0;

/// Height the layout name sits at.
pub(crate) const MODE_LABEL_Y: f32 = 14.0;

/// Text scale of the layout name.
pub(crate) const MODE_LABEL_SCALE: f32 = 1.4;

/// Height the pacemaker line sits at, under the white number in the left-hand column.
const PACE_Y: f32 = 112.0;

/// Text scale of the pacemaker line, matching the numbers above it.
const PACE_SCALE: f32 = 1.4;

/// The target a run is being paced against, and how the run stands against it right now.
pub struct HudPace<'a> {
    /// What the target is called, as the TARGET row settled it.
    pub name: &'a str,
    /// Current EX minus the EX the target would hold at this point in the run, so a positive
    /// number is ahead of the pace and a negative one behind it.
    pub delta: i64,
}

/// Live play-HUD snapshot, filled from the judge engine each frame.
pub struct HudView<'a> {
    /// Short name of the layout the chart is being played on, so a five-key chart says so while it
    /// is running rather than only in the browser.
    pub mode_label: &'a str,
    pub combo: u32,
    pub last_judge: Option<u8>,
    pub last_fast: bool,
    /// Early and late hits, split into the keys and the turntable. A scratch is thrown rather than
    /// pressed and drifts in its own direction, so a run's key timing says nothing about it and the
    /// two are counted apart; the headline number stays the sum.
    pub fast: [u32; LANE_KIND_COUNT],
    pub slow: [u32; LANE_KIND_COUNT],
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub gauge: f32,
    /// IIDX-style green number (note travel time across the visible field, ms) for the current
    /// scroll speed; 0 hides it. Recomputed each frame so it tracks BPM/hi-speed/cover changes.
    pub green_number: f64,
    /// White number: how long a note spends under the lane cover before it appears, in ms. 0 hides
    /// it, which is what a run with no cover, or with the row switched off, asks for.
    pub white_number: f64,
    /// Where the judgment label sits, as a fraction of the field height above the judgment line.
    /// [`JUDGE_TEXT_FROM_SKIN`] keeps the skin's own placement.
    pub judge_text_y: f32,
    /// Max attainable EX (notes*2) — the live score-graph's full-scale top.
    pub max_ex: u32,
    /// Local best EX on this chart (for the graph's BEST bar + delta); `None` = no prior play.
    pub best_ex: Option<u32>,
    /// How the run stands against its target at this moment, or `None` when the run is not being
    /// paced against one.
    pub pace: Option<HudPace<'a>>,
}

/// How far above the judgment line the judgment label is drawn: the skin's own offset, or the
/// fraction of the field the JUDGE TEXT Y row asks for.
fn judge_text_offset(skin: &Skin, frac: f32) -> f32 {
    if frac <= JUDGE_TEXT_FROM_SKIN { skin.judge_text_y_offset } else { (skin.judge_y - skin.top_y) * frac }
}

/// The live pacemaker score graph's placement and scores.
struct ScoreGraph {
    rect: Rect,
    ex: u32,
    max_ex: u32,
    best: Option<u32>,
}

/// IIDX-style live "pacemaker" score graph: a vertical panel with A/AA/AAA rank-band lines, a CURRENT
/// (cyan) EX bar that grows toward the top, and a BEST (green) bar, plus a "vs BEST" delta. Drawn in
/// the gap between the field and the BGA.
fn draw_score_graph<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, graph: &ScoreGraph) {
    let ScoreGraph { rect: Rect { x, y, w, h }, ex, max_ex, best } = *graph;
    let th = ctx.theme;
    r.fill_rect(Rect::new(x, y, w, h), th.panel);
    let ratio = |v: u32| {
        if max_ex > 0 { (v as f32 / max_ex as f32).clamp(0.0, 1.0) } else { 0.0 }
    };
    for (band, label) in [(6.0f32 / 9.0, "A"), (7.0 / 9.0, "AA"), (8.0 / 9.0, "AAA")] {
        let ly = y + h * (1.0 - band);
        r.fill_rect(Rect::new(x, ly, w, 1.0), Color::rgb(74, 74, 92));
        ctx.draw_text(r, x + 4.0, ly + 2.0, 1.0, th.text_muted, label);
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
        ctx.draw_text_centered(r, x + w * 0.5, y - 18.0, 1.4, col, &format!("vs BEST {txt}"));
    }
    ctx.draw_text_centered(r, bx + bar_w * 0.5, y + h + 2.0, 1.0, CYAN, "NOW");
    if best.is_some() {
        ctx.draw_text_centered(r, bx + bar_w + gap + bar_w * 0.5, y + h + 2.0, 1.0, Color::GREEN, "BEST");
    }
}

/// [`render_hud_ctx`] against this thread's installed theme and shared text engine.
pub fn render_hud<R: Renderer>(r: &mut R, skin: &Skin, hud: &HudView<'_>) {
    with_render_ctx(|ctx| render_hud_ctx(ctx, r, skin, hud));
}

/// Draw the play HUD: gauge, score column, combo/judgment flash, pacemaker graph and judge counts.
pub fn render_hud_ctx<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, skin: &Skin, hud: &HudView<'_>) {
    let th = ctx.theme;
    let field_x0 = skin.x.iter().copied().fold(f32::MAX, f32::min);
    let field_right = skin.x.iter().zip(&skin.w).map(|(x, w)| x + w).fold(f32::MIN, f32::max);
    let field_w = field_right - field_x0;
    let cx = field_x0 + field_w * 0.5;
    let jy = skin.judge_y;
    let top = skin.top_y;

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
    ctx.draw_text_right(r, field_x0 + field_w - 4.0, gy + gh + 4.0, 1.6, gcol, &format!("{}", hud.gauge.round() as i32));
    ctx.draw_text(r, field_x0, gy + gh + 4.0, 1.2, th.text_muted, "0");

    ctx.draw_text_right(r, CW_REFERENCE - MODE_LABEL_MARGIN, MODE_LABEL_Y, MODE_LABEL_SCALE, th.text_muted, hud.mode_label);
    ctx.draw_text(r, 14.0, 14.0, 2.4, Color::WHITE, &format!("EX {}", hud.ex_score));
    if let Some(b) = hud.best_ex {
        ctx.draw_text(r, 14.0, 48.0, 1.4, Color::GREEN, &format!("BEST {b}"));
    }
    if hud.green_number > 0.0 {
        ctx.draw_text(r, 14.0, 72.0, 1.4, CYAN, &format!("GREEN {}", hud.green_number.round() as i32));
    }
    if hud.white_number > 0.0 {
        ctx.draw_text(r, 14.0, 92.0, 1.4, Color::WHITE, &format!("WHITE {}", hud.white_number.round() as i32));
    }
    if let Some(pace) = &hud.pace {
        let (delta, col) = crate::result::ex_delta_label(pace.delta);
        ctx.draw_text(r, 14.0, PACE_Y, PACE_SCALE, col, &format!("{} {delta}", pace.name));
    }

    if hud.combo > 0 {
        ctx.draw_text_centered(r, cx, (jy - skin.combo_y_offset).max(top), 5.0, Color::WHITE, &hud.combo.to_string());
    }

    if let Some(j) = hud.last_judge {
        let j = j as usize;
        ctx.draw_text_centered(r, cx, (jy - judge_text_offset(skin, hud.judge_text_y)).max(top), 3.0, skin.judge_colors[j], &skin.judge_labels[j]);
        if (1..=3).contains(&j) {
            let (txt, col) = if hud.last_fast { ("FAST", CYAN) } else { ("SLOW", Color::ORANGE) };
            ctx.draw_text_centered(r, cx, (jy - skin.fastslow_y_offset).max(top), 2.0, col, txt);
        }
    }

    let graph_x = field_right + 28.0;
    let graph_right = skin.bga.map(|b| b.x - 16.0).unwrap_or(graph_x + 150.0);
    let graph_w = (graph_right - graph_x).clamp(0.0, 170.0);
    if graph_w >= 80.0 {
        let rect = Rect::new(graph_x, top + 26.0, graph_w, (jy - top - 26.0).max(40.0));
        draw_score_graph(ctx, r, &ScoreGraph { rect, ex: hud.ex_score, max_ex: hud.max_ex, best: hud.best_ex });
    }

    let (tx, mut ty) = match skin.bga {
        Some(b) => (b.x.max(field_right + 20.0), top),
        None => (graph_x + graph_w + 20.0, top),
    };
    for i in 0..6 {
        ctx.draw_text(r, tx, ty, 1.6, skin.judge_colors[i], &skin.judge_labels_short[i]);
        ctx.draw_text(r, tx + 26.0, ty, 1.6, Color::WHITE, &hud.counts[i].to_string());
        ty += 16.0;
    }
    ty += 8.0;
    for (label, counts, color) in [("FAST", hud.fast, CYAN), ("SLOW", hud.slow, Color::ORANGE)] {
        ctx.draw_text(r, tx, ty, 1.6, color, &format!("{label} {}", lane_kind_total(counts)));
        if counts[SCRATCH_LANE_KIND] > 0 {
            let split = format!("{}/{}", counts[KEY_LANE_KIND], counts[SCRATCH_LANE_KIND]);
            ctx.draw_text(r, tx + FASTSLOW_SPLIT_X, ty, FASTSLOW_SPLIT_SCALE, th.text_muted, &split);
        }
        ty += 16.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CpuCanvas;
    use rbms_model::Mode;

    fn skin() -> Skin {
        Skin::default_for(Mode::BEAT_7K, 1280.0, 720.0)
    }

    fn hud() -> HudView<'static> {
        HudView {
            mode_label: "7K",
            pace: None,
            combo: 12,
            last_judge: Some(1),
            last_fast: true,
            fast: [8, 3],
            slow: [5, 2],
            counts: [10, 4, 2, 1, 0, 0],
            ex_score: 24,
            gauge: 74.0,
            green_number: 300.0,
            white_number: 0.0,
            judge_text_y: JUDGE_TEXT_FROM_SKIN,
            max_ex: 34,
            best_ex: Some(20),
        }
    }

    fn drawn(hud: &HudView<'_>) -> Vec<u8> {
        crate::font::use_embedded_fonts_only();
        let mut canvas = CpuCanvas::new(1280, 720);
        canvas.clear(Color::BLACK);
        render_hud(&mut canvas, &skin(), hud);
        canvas.pixels().to_vec()
    }

    /// A player who has not moved the row keeps the placement the skin was authored with, which is
    /// what makes the row an override rather than a replacement.
    #[test]
    fn a_judge_text_position_of_zero_keeps_the_skins_own_placement() {
        let skin = skin();
        assert_eq!(judge_text_offset(&skin, JUDGE_TEXT_FROM_SKIN), skin.judge_text_y_offset);
        assert_eq!(judge_text_offset(&skin, -1.0), skin.judge_text_y_offset, "a value below the range is not a position either");
    }

    /// The row is a fraction of the field, so half of it puts the label halfway up.
    #[test]
    fn a_judge_text_position_is_a_fraction_of_the_field_height() {
        let skin = skin();
        let height = skin.judge_y - skin.top_y;
        assert_eq!(judge_text_offset(&skin, 0.5), height * 0.5);
        assert_eq!(judge_text_offset(&skin, 1.0), height, "the whole field puts it at the top edge");
    }

    /// Moving the label has to reach the painted frame, or the row is wired to nothing.
    #[test]
    fn moving_the_judge_text_changes_the_painted_frame() {
        let base = drawn(&hud());
        let moved = drawn(&HudView { judge_text_y: 0.6, ..hud() });
        assert_ne!(base, moved, "the judge text position does not reach the screen");
    }

    /// The headline number is the sum of the two columns, so a run reads the same total it always
    /// did while the split is available underneath it.
    #[test]
    fn the_headline_fast_and_slow_numbers_are_the_sums_of_their_columns() {
        let hud = hud();
        assert_eq!(lane_kind_total(hud.fast), 11);
        assert_eq!(lane_kind_total(hud.slow), 7);
        assert_eq!(hud.fast[KEY_LANE_KIND] + hud.fast[SCRATCH_LANE_KIND], lane_kind_total(hud.fast));
    }

    /// Two runs with the same totals but different splits have to look different, or the breakdown
    /// is not being drawn.
    #[test]
    fn the_key_and_scratch_split_reaches_the_painted_frame() {
        let all_keys = drawn(&HudView { fast: [11, 0], slow: [7, 0], ..hud() });
        let split = drawn(&hud());
        assert_ne!(all_keys, split, "a run with scratch timing looks the same as one without");
    }

    /// The layout a chart is being played on is on the HUD, so a five-key run says so while it is
    /// running rather than only in the browser.
    /// A run being paced reports how it stands right now, and one that is not adds nothing to the
    /// column.
    #[test]
    fn the_pacemaker_line_reaches_the_painted_frame() {
        let quiet = drawn(&hud());
        let ahead = drawn(&HudView { pace: Some(HudPace { name: "RANK AAA", delta: 40 }), ..hud() });
        let behind = drawn(&HudView { pace: Some(HudPace { name: "RANK AAA", delta: -40 }), ..hud() });
        assert_ne!(quiet, ahead, "the pacemaker line is not drawn");
        assert_ne!(ahead, behind, "the sign of the pace does not reach the screen");
    }

    #[test]
    fn the_layout_name_reaches_the_painted_frame() {
        let seven = drawn(&hud());
        let five = drawn(&HudView { mode_label: "5K", ..hud() });
        assert_ne!(seven, five, "the layout name does not reach the screen");
    }

    /// The white number is the time a note spends under the cover, so a run with no cover has none
    /// and drawing it would put a stray zero on the HUD.
    #[test]
    fn the_white_number_is_only_drawn_when_there_is_one() {
        let none = drawn(&hud());
        let shown = drawn(&HudView { white_number: 120.0, ..hud() });
        assert_ne!(none, shown, "the white number does not reach the screen");
        assert_eq!(none, drawn(&HudView { white_number: 0.0, ..hud() }), "a run with no cover draws no white number");
    }
}
