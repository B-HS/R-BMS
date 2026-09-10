//! The three measurement panels along the bottom of the result screen: how the gauge moved through
//! the run, where its inputs landed in time, and what share of them took each judgement.
//!
//! The run's own report is a table of numbers; these are the shapes behind them, drawn only when the
//! RESULT GRAPHS setting asks for them.

use crate::ctx::RenderCtx;
use crate::{Color, Rect, Renderer};

use super::{CONTENT_RIGHT, CONTENT_X, ResultPalette, ResultView};

/// Top of the strip of graph panels. The left column above it belongs to the score server's report,
/// which is drawn over this screen by the player, so the strip starts below the tallest that gets.
const GRAPH_Y: f32 = 556.0;

/// Height of a graph panel.
const GRAPH_H: f32 = 120.0;

/// Gap between two graph panels.
const GRAPH_GAP: f32 = 32.0;

/// How many panels the graph strip holds.
const GRAPH_COUNT: usize = 3;

/// Inset from a panel's edge to its plot area.
const GRAPH_PAD: f32 = 8.0;

/// Height the title line takes off the top of a panel.
const GRAPH_TITLE_H: f32 = 22.0;

/// Height of a label strip inside a plot.
const GRAPH_LABEL_H: f32 = 16.0;

/// Text scale of a graph panel's title, note and axis labels.
const GRAPH_LABEL_SCALE: f32 = 1.1;

/// Thickness of a hairline drawn as a plot's axis.
const AXIS_W: f32 = 1.0;

/// Highest value the gauge trend's axis shows: the gauge is a percentage of its own maximum.
const GAUGE_PERCENT_MAX: f32 = 100.0;

/// Thickness of one plotted gauge sample.
const GAUGE_TREND_DOT: f32 = 2.0;

/// Narrowest a histogram bar is drawn, so a full histogram still shows every bucket.
const HIST_BAR_MIN_W: f32 = 1.0;

/// Gap between two histogram bars, taken off the bar rather than added to the pitch.
const HIST_BAR_GAP: f32 = 1.0;

/// Height of the judge distribution's single stacked bar.
const JUDGE_BAR_H: f32 = 18.0;

/// Gap under the stacked bar, before its legend.
const JUDGE_BAR_GAP: f32 = 8.0;

/// How many legend entries the judge distribution puts in one column.
const JUDGE_LEGEND_ROWS: usize = 2;

/// Height of one judge legend row.
const JUDGE_LEGEND_ROW_H: f32 = 22.0;

/// Shown in a panel whose measurement is empty, so a run nothing was recorded for still reads as a
/// finished panel rather than a drawing bug.
const NO_DATA: &str = "NO DATA";

/// Where the graph panel at `at` sits. The three share the content column evenly.
fn graph_panel_rect(at: usize) -> Rect {
    let w = (CONTENT_RIGHT - CONTENT_X - GRAPH_GAP * (GRAPH_COUNT as f32 - 1.0)) / GRAPH_COUNT as f32;
    Rect::new(CONTENT_X + (w + GRAPH_GAP) * at as f32, GRAPH_Y, w, GRAPH_H)
}

/// Draw one graph panel's ground, its title and the note on its right, and return the plot area
/// left inside it.
fn draw_graph_frame<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, panel: Rect, title: &str, note: &str) -> Rect {
    let th = ctx.theme;
    r.fill_rect(panel, th.panel);
    ctx.draw_text(r, panel.x + GRAPH_PAD, panel.y + GRAPH_PAD, GRAPH_LABEL_SCALE, th.text_dim, title);
    if !note.is_empty() {
        ctx.draw_text_right(r, panel.x + panel.w - GRAPH_PAD, panel.y + GRAPH_PAD, GRAPH_LABEL_SCALE, th.text_muted, note);
    }
    Rect::new(panel.x + GRAPH_PAD, panel.y + GRAPH_TITLE_H, panel.w - GRAPH_PAD * 2.0, panel.h - GRAPH_TITLE_H - GRAPH_PAD)
}

/// Say that a panel's measurement is empty, in the middle of where its plot would have been.
fn draw_no_data<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, plot: Rect) {
    let th = ctx.theme;
    ctx.draw_text_centered(r, plot.x + plot.w * 0.5, plot.y + plot.h * 0.5, GRAPH_LABEL_SCALE, th.text_muted, NO_DATA);
}

/// Plot the gauge once a second through the run, oldest on the left, against a full 0..100 axis so
/// two runs of different lengths are read on the same scale.
///
/// One column of the plot is one sample of the series, picked by position: a long run has more
/// samples than the panel has columns, and a short one has fewer.
fn draw_gauge_trend<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, plot: Rect, series: &[f32], color: Color) {
    let th = ctx.theme;
    r.fill_rect(Rect::new(plot.x, plot.y, plot.w, AXIS_W), th.divider);
    r.fill_rect(Rect::new(plot.x, plot.y + plot.h - AXIS_W, plot.w, AXIS_W), th.divider);
    let Some(last) = series.len().checked_sub(1) else {
        draw_no_data(ctx, r, plot);
        return;
    };
    let columns = (plot.w as usize).max(1);
    let span = columns.saturating_sub(1).max(1);
    for column in 0..columns {
        let at = column * last / span;
        let level = (series[at] / GAUGE_PERCENT_MAX).clamp(0.0, 1.0);
        let top = plot.y + (plot.h - GAUGE_TREND_DOT) * (1.0 - level);
        r.fill_rect(Rect::new(plot.x + column as f32, top, AXIS_W, GAUGE_TREND_DOT), color);
    }
}

/// Plot how many judged inputs landed in each timing bucket, earliest on the left, with the bucket
/// a note hit exactly on marked. Bars are scaled to the tallest bucket, since what the shape says is
/// where the run sat rather than how many notes it had.
fn draw_timing_histogram<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, plot: Rect, hist: &[u32], color: Color) {
    let th = ctx.theme;
    let bars = Rect::new(plot.x, plot.y, plot.w, plot.h - GRAPH_LABEL_H);
    r.fill_rect(Rect::new(bars.x, bars.y + bars.h - AXIS_W, bars.w, AXIS_W), th.divider);
    let peak = hist.iter().copied().max().unwrap_or(0);
    if peak == 0 {
        draw_no_data(ctx, r, bars);
        return;
    }
    let pitch = bars.w / hist.len() as f32;
    for (at, count) in hist.iter().enumerate() {
        let height = (bars.h - AXIS_W) * *count as f32 / peak as f32;
        let x = bars.x + pitch * at as f32;
        r.fill_rect(Rect::new(x, bars.y + bars.h - AXIS_W - height, (pitch - HIST_BAR_GAP).max(HIST_BAR_MIN_W), height), color);
    }
    r.fill_rect(Rect::new(bars.x + pitch * (hist.len() / 2) as f32, bars.y, AXIS_W, bars.h), th.divider);
    let label_y = plot.y + plot.h - GRAPH_LABEL_H;
    ctx.draw_text(r, plot.x, label_y, GRAPH_LABEL_SCALE, th.text_muted, "EARLY");
    ctx.draw_text_right(r, plot.x + plot.w, label_y, GRAPH_LABEL_SCALE, th.text_muted, "LATE");
}

/// Show what share of the judged inputs took each judgement: one stacked bar in the judge colours,
/// then the same six shares written out. The counts themselves are already in the report above, so
/// what this adds is the proportion.
fn draw_judge_distribution<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, plot: Rect, dist: &[u32; 6], palette: &ResultPalette) {
    let th = ctx.theme;
    let total: u32 = dist.iter().sum();
    if total == 0 {
        draw_no_data(ctx, r, plot);
        return;
    }
    let mut x = plot.x;
    for (at, count) in dist.iter().enumerate() {
        let w = plot.w * *count as f32 / total as f32;
        r.fill_rect(Rect::new(x, plot.y, w, JUDGE_BAR_H), palette.judge_colors[at]);
        x += w;
    }
    let column_w = plot.w / (dist.len() / JUDGE_LEGEND_ROWS) as f32;
    let legend_y = plot.y + JUDGE_BAR_H + JUDGE_BAR_GAP;
    for (at, count) in dist.iter().enumerate() {
        let left = plot.x + column_w * (at / JUDGE_LEGEND_ROWS) as f32;
        let y = legend_y + JUDGE_LEGEND_ROW_H * (at % JUDGE_LEGEND_ROWS) as f32;
        ctx.draw_text(r, left, y, GRAPH_LABEL_SCALE, palette.judge_colors[at], &palette.judge_labels[at]);
        let share = *count as f32 * GAUGE_PERCENT_MAX / total as f32;
        ctx.draw_text_right(r, left + column_w - GRAPH_PAD, y, GRAPH_LABEL_SCALE, th.text, &format!("{share:.1}%"));
    }
}

/// Draw the three measurement panels along the bottom: the gauge through the run, where its inputs
/// landed in time, and what share of them took each judgement.
pub(super) fn draw_result_graphs<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, view: &ResultView, palette: &ResultPalette) {
    let measured: u32 = view.timing_hist.iter().sum();
    let gauge_note = view.gauge_series.last().map(|v| format!("{}%", v.round() as i32)).unwrap_or_default();
    let plot = draw_graph_frame(ctx, r, graph_panel_rect(0), "GAUGE", &gauge_note);
    draw_gauge_trend(ctx, r, plot, &view.gauge_series, view.clear_color);
    let plot = draw_graph_frame(ctx, r, graph_panel_rect(1), "TIMING", &measured.to_string());
    draw_timing_histogram(ctx, r, plot, &view.timing_hist, ctx.theme.accent);
    let plot = draw_graph_frame(ctx, r, graph_panel_rect(2), "JUDGE", "");
    draw_judge_distribution(ctx, r, plot, &view.judge_dist, palette);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bottom of the screen the strip has to stay clear of, so the key hint under it stays readable.
    const HINT_Y: f32 = 692.0;

    #[test]
    fn the_panels_share_the_content_column_evenly_and_do_not_overlap() {
        let panels: Vec<Rect> = (0..GRAPH_COUNT).map(graph_panel_rect).collect();
        assert_eq!(panels[0].x, CONTENT_X, "the strip does not start at the content column");
        let last = panels[GRAPH_COUNT - 1];
        assert!((last.x + last.w - CONTENT_RIGHT).abs() < 0.01, "the strip does not end at the content column");
        for pair in panels.windows(2) {
            assert!((pair[0].w - pair[1].w).abs() < 0.01, "the panels are not the same width");
            assert!((pair[1].x - (pair[0].x + pair[0].w) - GRAPH_GAP).abs() < 0.01, "the panels do not sit one gap apart");
        }
    }

    /// The strip is drawn under a left column the player writes the score server's report into, and
    /// over the key hint, so it has to fit between the two.
    #[test]
    fn the_strip_fits_between_the_report_above_it_and_the_hint_below() {
        let panel = graph_panel_rect(0);
        assert!(panel.y + panel.h < HINT_Y, "the strip runs into the key hint");
    }

    /// Every panel has room for what is drawn in it: a title line, and a plot tall enough for the
    /// judge legend, which is the tallest of the three.
    #[test]
    fn a_panel_has_room_for_its_title_and_its_plot() {
        let panel = graph_panel_rect(0);
        let plot_h = panel.h - GRAPH_TITLE_H - GRAPH_PAD;
        let judge_h = JUDGE_BAR_H + JUDGE_BAR_GAP + JUDGE_LEGEND_ROW_H * JUDGE_LEGEND_ROWS as f32;
        assert!(plot_h > 0.0, "a panel has no plot area");
        assert!(judge_h <= plot_h, "the judge legend is taller than the plot it is drawn in");
        assert!(GRAPH_LABEL_H < plot_h, "the timing labels take the whole plot");
    }

    /// The judge legend lays six entries out in whole columns, so a distribution never loses one off
    /// the end of the grid.
    #[test]
    fn the_judge_legend_grid_holds_every_judgement() {
        let entries = 6usize;
        assert_eq!(entries % JUDGE_LEGEND_ROWS, 0, "the legend grid does not divide evenly");
        assert!(entries / JUDGE_LEGEND_ROWS > 0, "the legend has no columns");
    }
}
