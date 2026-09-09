use crate::ctx::{RenderCtx, with_render_ctx};
use crate::result::{RANK_BANDS, dj_rank, draw_rank_bar};
use crate::{Color, Rect, Renderer};

/// Clickable region kinds the song-select screen reports back to the app so it can map them onto its
/// own input handling without the renderer knowing the app's `Hot` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectHot {
    Row(usize),
    Record(usize),
    ModalReplay,
    ModalClose,
    Search,
    Sort,
    Folders,
    Tables,
    Records,
    Settings,
}

/// One row in the song bar list — a folder or a chart. Colours/labels are resolved by the app so the
/// renderer stays backend- and domain-agnostic.
pub struct SelectRow {
    pub folder: bool,
    pub title: String,
    pub mode_short: &'static str,
    pub mode_color: Color,
    pub level: String,
    pub difficulty_color: Color,
    pub lamp: Color,
    pub folder_count: Option<usize>,
}

/// Per-second note-density readout for the detail panel histogram (`bins`) plus the three scalar
/// densities (notes/sec) shown reference-style.
pub struct DensityView {
    pub bins: Vec<u32>,
    pub peak: f64,
    pub avg: f64,
    pub end: f64,
}

pub struct StatCell {
    pub label: &'static str,
    pub value: String,
}

/// A single local-record line in the detail panel (and the source for the best/summary line).
pub struct RecordRowView {
    pub when: String,
    pub lamp: Color,
    pub lamp_label: &'static str,
    pub ex: u32,
    pub max_ex: u32,
    pub bp: u32,
    /// Optional EX-trend tag versus the next-older play, e.g. `("+45", GREEN)`.
    pub trend: Option<(String, Color)>,
}

pub struct RecordsView {
    pub plays: usize,
    pub clears: usize,
    pub best: Option<RecordRowView>,
    /// `(ex, max_ex)` for the DJ-rank bar of the best record (SCORE GRAPH option), else `None`.
    pub rank_bar: Option<(u32, u32)>,
    pub recent: Vec<RecordRowView>,
}

/// Cover (`#STAGEFILE`/`#BANNER`) state. The texture itself is drawn by the GPU backend into
/// `cover_rect()`; the renderer only frames it or shows a placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverState {
    None,
    Loading,
    Present,
}

pub struct DetailView {
    pub accent: Color,
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    pub genre_maker: String,
    pub mode_short: &'static str,
    pub mode_color: Color,
    pub level: String,
    pub difficulty_color: Color,
    pub difficulty_name: &'static str,
    pub cover: CoverState,
    pub stats: Vec<StatCell>,
    pub density: Option<DensityView>,
    pub records: RecordsView,
}

pub enum SelectDetail {
    Song(Box<DetailView>),
    Folder { label: String, count: usize },
    Empty,
}

/// Full-screen record-detail modal data (opened from a record row or `R`).
pub struct SelectModal {
    pub title: String,
    pub clear_label: &'static str,
    pub clear_color: Color,
    pub when: String,
    pub sub: String,
    pub counts: [u32; 6],
    pub ex: u32,
    pub max_ex: u32,
    pub rank: Option<&'static str>,
    pub rank_color: Color,
    pub max_combo: u32,
    pub total_notes: u32,
    pub bp: u32,
    pub empty_poor: u32,
    pub gauge_value: i32,
    pub index: usize,
    pub total: usize,
    pub has_replay: bool,
}

/// Backend-agnostic song-select snapshot. The app assembles it each frame from its state; the GPU
/// backend additionally uploads the cover texture into [`cover_rect`].
pub struct SelectView {
    pub rows: Vec<SelectRow>,
    pub sel: usize,
    pub header: String,
    pub guide: &'static str,
    pub detail: SelectDetail,
    pub modal: Option<SelectModal>,
    pub score_graph: bool,
    /// Live search query while the search box is open (`None` when not searching). Filters the list.
    pub search: Option<String>,
    /// Current sort-order label (e.g. `"DEFAULT"`, `"TITLE"`), shown top-right.
    pub sort: &'static str,
    /// Shown centered when the list is empty: `(title, body)` onboarding/empty hint (e.g. a first-run
    /// "add a music folder" call to action). Ignored when rows exist.
    pub empty_hint: Option<(&'static str, &'static str)>,
}

const LIST_X: f32 = 32.0;
const LIST_W: f32 = 584.0;
const DETAIL_X: f32 = 632.0;
const DETAIL_W: f32 = 616.0;
const TOP: f32 = 60.0;
const BOTTOM: f32 = 660.0;
const ROW_H: f32 = 36.0;
const ROW_GAP: f32 = 4.0;

/// The cover (`#STAGEFILE`) square in the detail panel. The GPU backend uploads the decoded image
/// here; the renderer frames it / draws a placeholder. Kept as a function so both sides share it.
pub fn cover_rect() -> Rect {
    Rect::new(DETAIL_X + 16.0, 78.0, 160.0, 160.0)
}

fn darken(c: Color, f: f32) -> Color {
    Color { r: (c.r as f32 * f) as u8, g: (c.g as f32 * f) as u8, b: (c.b as f32 * f) as u8, a: 255 }
}

/// Split `text` into at most two lines that each fit `width` at `scale` (character wrap, since CJK
/// titles have no spaces). The second line is ellipsis-truncated if the remainder still overflows.
fn wrap_two(ctx: &mut RenderCtx<'_>, text: &str, scale: f32, width: f32) -> (String, Option<String>) {
    if width <= 0.0 || ctx.text_width(text, scale) <= width {
        return (text.to_string(), None);
    }
    let mut split = 0;
    for (i, ch) in text.char_indices() {
        if ctx.text_width(&text[..i + ch.len_utf8()], scale) > width {
            break;
        }
        split = i + ch.len_utf8();
    }
    let split = split.max(text.chars().next().map(char::len_utf8).unwrap_or(0));
    (text[..split].to_string(), Some(ctx.fit_text(&text[split..], scale, width)))
}

fn outline<R: Renderer>(r: &mut R, rect: Rect, t: f32, c: Color) {
    r.fill_rect(Rect::new(rect.x, rect.y, rect.w, t), c);
    r.fill_rect(Rect::new(rect.x, rect.y + rect.h - t, rect.w, t), c);
    r.fill_rect(Rect::new(rect.x, rect.y, t, rect.h), c);
    r.fill_rect(Rect::new(rect.x + rect.w - t, rect.y, t, rect.h), c);
}

/// A bottom-bar navigation button: a chip with a `label` and a dim `key` hint. Returns its width so
/// the caller can lay the next one out and record the matching hot rect. `active` tints it (e.g. an
/// open search box).
fn nav_button<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, x: f32, y: f32, label: &str, key: &str, active: bool) -> f32 {
    let th = ctx.theme;
    let lw = ctx.text_width(label, 1.2);
    let kw = ctx.text_width(key, 1.0);
    let w = lw + kw + 28.0;
    let rect = Rect::new(x, y, w, 30.0);
    r.fill_rect(rect, if active { th.button_active } else { th.panel_hi });
    outline(r, rect, 1.0, if active { th.focus } else { th.divider });
    ctx.draw_text(r, x + 10.0, y + 9.0, 1.2, th.text, label);
    ctx.draw_text(r, x + 10.0 + lw + 8.0, y + 11.0, 1.0, th.accent, key);
    w
}

/// The geometry and colours of a label chip; its text is passed separately since it is the one part
/// that changes per call.
struct Badge {
    rect: Rect,
    fill: Color,
    border: Color,
    scale: f32,
    text_color: Color,
}

/// A small filled label chip (badge) with a coloured border and centred text.
fn badge<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, chip: &Badge, text: &str) {
    let Badge { rect, fill, border, scale, text_color } = *chip;
    r.fill_rect(rect, fill);
    outline(r, rect, 1.5, border);
    ctx.draw_text_centered(r, rect.x + rect.w * 0.5, rect.y + (rect.h - scale * 14.0) * 0.5, scale, text_color, text);
}

/// IIDX/LR2-style song select: left bar list (KEY/level badges, clear-lamp LED), right detail panel
/// (cover, metadata, stat grid, note-density histogram, local records). Returns the clickable regions
/// (row / record / modal buttons) for the app to hit-test; row/record hits are suppressed while the
/// record modal is open.
pub fn render_select<R: Renderer>(r: &mut R, v: &SelectView) -> Vec<(Rect, SelectHot)> {
    with_render_ctx(|ctx| render_select_ctx(ctx, r, v))
}

/// [`render_select`] against a caller-supplied context.
pub fn render_select_ctx<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, v: &SelectView) -> Vec<(Rect, SelectHot)> {
    let th = ctx.theme;
    let mut hot = Vec::new();
    r.clear(th.bg);
    r.fill_rect(Rect::new(0.0, 0.0, 1280.0, 52.0), th.topbar);
    ctx.draw_text(r, LIST_X, 12.0, 2.6, th.text, "MUSIC SELECT");
    if let Some(q) = &v.search {
        let bx = 290.0;
        let bw = 240.0;
        r.fill_rect(Rect::new(bx, 10.0, bw, 32.0), th.button_active);
        outline(r, Rect::new(bx, 10.0, bw, 32.0), 1.5, th.focus);
        let fitted = ctx.fit_text(&format!("\u{1F50D} {q}_"), 1.4, bw - 20.0);
        ctx.draw_text(r, bx + 10.0, 18.0, 1.4, th.text, &fitted);
    } else if !v.header.is_empty() {
        let fitted = ctx.fit_text(&v.header, 1.3, LIST_X + LIST_W - 300.0 - 80.0);
        ctx.draw_text(r, 300.0, 22.0, 1.3, th.text_dim, &fitted);
    }
    let m = v.rows.len();
    ctx.draw_text_right(r, LIST_X + LIST_W, 6.0, 1.0, th.accent, &format!("SORT: {}", v.sort));
    ctx.draw_text_right(r, LIST_X + LIST_W, 24.0, 1.4, th.text_dim, &format!("{}/{}", (v.sel + 1).min(m.max(1)), m));

    if v.modal.is_none() {
        let by = BOTTOM + 8.0;
        let sort_btn = format!("SORT: {}", v.sort);
        let search_btn = if v.search.is_some() { "\u{2715} SEARCH" } else { "SEARCH" };
        let buttons: [(&str, &str, SelectHot); 6] = [
            (search_btn, "/", SelectHot::Search),
            (&sort_btn, "F3", SelectHot::Sort),
            ("FOLDERS", "O", SelectHot::Folders),
            ("TABLES", "T", SelectHot::Tables),
            ("RECORDS", "R", SelectHot::Records),
            ("SETTINGS", "TAB", SelectHot::Settings),
        ];
        let mut bx = LIST_X;
        for (label, key, h) in buttons {
            let active = matches!(h, SelectHot::Search) && v.search.is_some();
            let w = nav_button(ctx, r, bx, by, label, key, active);
            hot.push((Rect::new(bx, by, w, 30.0), h));
            bx += w + 8.0;
        }
        ctx.draw_text_right(r, 1280.0 - 20.0, by + 9.0, 1.0, th.text_muted, v.guide);
    }

    render_list(ctx, r, v, &mut hot);
    match &v.detail {
        SelectDetail::Song(d) => render_song_detail(ctx, r, v, d, &mut hot),
        SelectDetail::Folder { label, count } => render_folder_detail(ctx, r, label, *count),
        SelectDetail::Empty => {}
    }
    if let Some(modal) = &v.modal {
        render_modal(ctx, r, modal, &mut hot);
    }
    hot
}

fn render_list<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, v: &SelectView, hot: &mut Vec<(Rect, SelectHot)>) {
    let th = ctx.theme;
    let m = v.rows.len();
    if m == 0 {
        let cx = LIST_X + LIST_W * 0.5;
        match v.empty_hint {
            Some((title, body)) => {
                ctx.draw_text_centered(r, cx, TOP + 80.0, 2.2, th.text, title);
                ctx.draw_text_centered(r, cx, TOP + 124.0, 1.3, th.text_dim, body);
            }
            None => ctx.draw_text(r, LIST_X + 12.0, TOP + 40.0, 1.5, th.text_dim, "NO CHARTS"),
        }
        return;
    }
    let visible = ((BOTTOM - TOP) / (ROW_H + ROW_GAP)).floor() as usize;
    let start = v.sel.saturating_sub(visible / 2).min(m.saturating_sub(visible.min(m)));
    let modal_open = v.modal.is_some();
    let mut y = TOP;
    for idx in start..(start + visible).min(m) {
        let row = &v.rows[idx];
        let focused = idx == v.sel;
        let h = ROW_H;
        let bg = if focused {
            th.row_focus
        } else if row.folder {
            th.row_folder
        } else if idx % 2 == 0 {
            th.row_even
        } else {
            th.row_odd
        };
        r.fill_rect(Rect::new(LIST_X, y, LIST_W, h), bg);
        if focused {
            r.fill_rect(Rect::new(LIST_X, y, 2.0, h), th.focus);
            r.fill_rect(Rect::new(LIST_X + LIST_W - 2.0, y, 2.0, h), th.focus);
        }
        r.fill_rect(Rect::new(LIST_X, y, 6.0, h), row.lamp);
        r.fill_rect(Rect::new(LIST_X + LIST_W - 10.0, y, 8.0, h), row.lamp);
        let cy = y + h * 0.5;
        if row.folder {
            let fitted = ctx.fit_text(&format!("\u{25B8} {}", row.title), 1.6, LIST_W - 120.0);
            ctx.draw_text(r, LIST_X + 16.0, cy - 9.0, 1.6, Color::rgb(220, 220, 150), &fitted);
            if let Some(n) = row.folder_count {
                ctx.draw_text_right(r, LIST_X + LIST_W - 20.0, cy - 8.0, 1.3, th.text_dim, &format!("{n} CHARTS"));
            }
        } else {
            let mode_chip = Badge {
                rect: Rect::new(LIST_X + 12.0, cy - 12.0, 46.0, 24.0),
                fill: darken(row.mode_color, 0.45),
                border: row.mode_color,
                scale: 1.2,
                text_color: Color::WHITE,
            };
            badge(ctx, r, &mode_chip, row.mode_short);
            let level_chip = Badge {
                rect: Rect::new(LIST_X + 64.0, cy - 12.0, 42.0, 24.0),
                fill: darken(row.difficulty_color, 0.4),
                border: row.difficulty_color,
                scale: 1.4,
                text_color: Color::WHITE,
            };
            badge(ctx, r, &level_chip, &row.level);
            let title_scale = if focused { 1.7 } else { 1.5 };
            let title_color = if focused { th.title_focus } else { th.title_dim };
            let fitted = ctx.fit_text(&row.title, title_scale, LIST_W - 116.0 - 24.0);
            ctx.draw_text(r, LIST_X + 116.0, cy - title_scale * 7.0, title_scale, title_color, &fitted);
        }
        if !modal_open {
            hot.push((Rect::new(LIST_X, y, LIST_W, h), SelectHot::Row(idx)));
        }
        y += h + ROW_GAP;
    }
}

fn render_song_detail<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, v: &SelectView, d: &DetailView, hot: &mut Vec<(Rect, SelectHot)>) {
    let th = ctx.theme;
    outline(r, Rect::new(DETAIL_X, TOP, DETAIL_W, BOTTOM - TOP), 1.0, th.divider);
    r.fill_rect(Rect::new(DETAIL_X, TOP, DETAIL_W, 6.0), d.accent);

    let cr = cover_rect();
    match d.cover {
        CoverState::Present => {}
        CoverState::Loading => {
            r.fill_rect(cr, Color::rgb(16, 18, 28));
            ctx.draw_text_centered(r, cr.x + cr.w * 0.5, cr.y + cr.h * 0.5 - 7.0, 1.2, th.text_dim, "LOADING");
        }
        CoverState::None => {
            r.fill_rect(cr, Color::rgb(16, 18, 28));
            ctx.draw_text_centered(r, cr.x + cr.w * 0.5, cr.y + cr.h * 0.5 - 7.0, 1.2, Color::rgb(70, 72, 88), "NO IMAGE");
        }
    }
    outline(r, cr, 2.0, Color::rgb(90, 96, 120));

    let tx = DETAIL_X + 192.0;
    let tw = DETAIL_X + DETAIL_W - 16.0 - tx;
    let (l1, l2) = wrap_two(ctx, &d.title, 2.0, tw);
    let mut ty = 86.0;
    ctx.draw_text(r, tx, ty, 2.0, th.text, &l1);
    ty += 30.0;
    if let Some(l2) = &l2 {
        ctx.draw_text(r, tx, ty, 2.0, th.text, l2);
        ty += 30.0;
    } else if !d.subtitle.is_empty() {
        let fitted = ctx.fit_text(&d.subtitle, 1.2, tw);
        ctx.draw_text(r, tx, ty, 1.2, th.text_dim, &fitted);
        ty += 24.0;
    }
    if !d.artist.is_empty() {
        let fitted = ctx.fit_text(&d.artist, 1.3, tw);
        ctx.draw_text(r, tx, ty, 1.3, th.accent, &fitted);
        ty += 24.0;
    }
    if !d.genre_maker.is_empty() {
        let fitted = ctx.fit_text(&d.genre_maker, 1.1, tw);
        ctx.draw_text(r, tx, ty, 1.1, th.text_dim, &fitted);
    }
    let mode_chip =
        Badge { rect: Rect::new(tx, 196.0, 56.0, 26.0), fill: darken(d.mode_color, 0.45), border: d.mode_color, scale: 1.4, text_color: Color::WHITE };
    badge(ctx, r, &mode_chip, d.mode_short);
    let level_chip = Badge {
        rect: Rect::new(tx + 64.0, 196.0, 64.0, 26.0),
        fill: darken(d.difficulty_color, 0.4),
        border: d.difficulty_color,
        scale: 1.4,
        text_color: Color::WHITE,
    };
    badge(ctx, r, &level_chip, &format!("Lv{}", d.level));
    ctx.draw_text(r, tx + 138.0, 201.0, 1.3, d.difficulty_color, d.difficulty_name);

    let inner_x = DETAIL_X + 16.0;
    let inner_w = DETAIL_W - 32.0;
    r.fill_rect(Rect::new(inner_x, 254.0, inner_w, 2.0), th.divider);
    let col_w = inner_w / 3.0;
    for (i, cell) in d.stats.iter().take(6).enumerate() {
        let cx = inner_x + 6.0 + (i % 3) as f32 * col_w;
        let cy = 268.0 + (i / 3) as f32 * 34.0;
        ctx.draw_text(r, cx, cy, 1.0, th.text_dim, cell.label);
        let fitted = ctx.fit_text(&cell.value, 1.4, col_w - 12.0);
        ctx.draw_text(r, cx, cy + 14.0, 1.4, th.text, &fitted);
    }

    r.fill_rect(Rect::new(inner_x, 348.0, inner_w, 2.0), th.divider);
    ctx.draw_text(r, inner_x, 362.0, 1.3, th.text_dim, "DENSITY");
    let box_y = 380.0;
    let box_h = 74.0;
    let box_rect = Rect::new(inner_x, box_y, inner_w, box_h);
    r.fill_rect(box_rect, Color::rgb(10, 11, 17));
    match &d.density {
        Some(dens) if !dens.bins.is_empty() && dens.peak > 0.0 => {
            ctx.draw_text_right(
                r,
                DETAIL_X + DETAIL_W - 16.0,
                364.0,
                1.0,
                th.text_dim,
                &format!("PEAK {}  AVG {}  END {}  /s", dens.peak.round() as i32, dens.avg.round() as i32, dens.end.round() as i32),
            );
            let n = dens.bins.len();
            let step = inner_w / n as f32;
            for (i, &b) in dens.bins.iter().enumerate() {
                let bx = inner_x + i as f32 * step;
                if i % 10 == 0 {
                    r.fill_rect(Rect::new(bx, box_y, 1.0, box_h), Color::rgb(26, 28, 38));
                }
                if b == 0 {
                    continue;
                }
                let t = (b as f64 / dens.peak).clamp(0.0, 1.0) as f32;
                let bh = (box_h * t).max(1.0);
                let bw = (step - 0.3).max(1.0).min(inner_x + inner_w - bx);
                let ht = t.min(0.85);
                let col = Color { r: (60.0 + 180.0 * ht) as u8, g: (140.0 + 40.0 * ht) as u8, b: (200.0 - 110.0 * ht) as u8, a: 255 };
                r.fill_rect(Rect::new(bx, box_y + box_h - bh, bw, bh), col);
            }
        }
        _ => ctx.draw_text_centered(r, inner_x + inner_w * 0.5, box_y + box_h * 0.5 - 7.0, 1.2, th.text_dim, "…"),
    }
    outline(r, box_rect, 1.0, th.divider);

    render_records(ctx, r, v, &d.records, hot);
}

fn render_records<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, v: &SelectView, rec: &RecordsView, hot: &mut Vec<(Rect, SelectHot)>) {
    let th = ctx.theme;
    let inner_x = DETAIL_X + 16.0;
    let inner_w = DETAIL_W - 32.0;
    ctx.draw_text(r, inner_x, 468.0, 1.4, th.text, "RECORDS");
    ctx.draw_text_right(r, DETAIL_X + DETAIL_W - 16.0, 472.0, 1.2, th.text_dim, &format!("{} CLEAR / {} PLAYS", rec.clears, rec.plays));
    let Some(best) = &rec.best else {
        ctx.draw_text(r, inner_x, 508.0, 1.3, th.text_dim, "NO PLAY YET");
        return;
    };
    r.fill_rect(Rect::new(inner_x, 492.0, inner_w, 28.0), best.lamp);
    ctx.draw_text(r, inner_x + 10.0, 498.0, 1.4, Color::BLACK, best.lamp_label);
    ctx.draw_text_right(r, inner_x + inner_w - 10.0, 498.0, 1.4, Color::BLACK, &format!("EX {}", best.ex));
    let mut list_top = 532.0;
    if let Some((ex, max_ex)) = rec.rank_bar {
        let (rank, rcol) = RANK_BANDS[dj_rank(ex, max_ex)];
        ctx.draw_text(r, inner_x, 530.0, 1.3, rcol, &format!("RANK {rank}"));
        draw_rank_bar(r, inner_x + 92.0, 534.0, inner_w - 92.0, 12.0, ex, max_ex);
        list_top = 558.0;
    }
    let rh = 32.0;
    let max_rows = ((BOTTOM - list_top) / rh).floor() as usize;
    let modal_open = v.modal.is_some();
    for (ri, rec_row) in rec.recent.iter().take(max_rows).enumerate() {
        let y = list_top + ri as f32 * rh;
        r.fill_rect(Rect::new(inner_x, y, inner_w, rh - 4.0), th.panel);
        r.fill_rect(Rect::new(inner_x, y, 6.0, rh - 4.0), rec_row.lamp);
        ctx.draw_text(r, inner_x + 14.0, y + 3.0, 1.1, th.text_dim, &rec_row.when);
        ctx.draw_text(r, inner_x + 14.0, y + 16.0, 1.2, rec_row.lamp, rec_row.lamp_label);
        let ex_txt =
            if v.score_graph { format!("{}  EX {}", RANK_BANDS[dj_rank(rec_row.ex, rec_row.max_ex)].0, rec_row.ex) } else { format!("EX {}", rec_row.ex) };
        ctx.draw_text_right(r, inner_x + inner_w - 12.0, y + 3.0, 1.2, th.text, &ex_txt);
        match &rec_row.trend {
            Some((t, c)) => {
                ctx.draw_text_right(r, inner_x + inner_w - 80.0, y + 17.0, 1.0, *c, t);
                ctx.draw_text_right(r, inner_x + inner_w - 12.0, y + 17.0, 1.0, th.text_dim, &format!("BP {}", rec_row.bp));
            }
            None => ctx.draw_text_right(r, inner_x + inner_w - 12.0, y + 17.0, 1.0, th.text_dim, &format!("BP {}", rec_row.bp)),
        }
        if !modal_open {
            hot.push((Rect::new(inner_x, y, inner_w, rh - 4.0), SelectHot::Record(ri)));
        }
    }
}

fn render_folder_detail<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, label: &str, count: usize) {
    let th = ctx.theme;
    outline(r, Rect::new(DETAIL_X, TOP, DETAIL_W, BOTTOM - TOP), 1.0, th.divider);
    r.fill_rect(Rect::new(DETAIL_X, TOP, DETAIL_W, 6.0), Color::rgb(120, 130, 90));
    r.fill_rect(Rect::new(DETAIL_X + 16.0, 96.0, DETAIL_W - 32.0, 120.0), th.panel_hi);
    ctx.draw_text(r, DETAIL_X + 40.0, 120.0, 2.4, th.text, "FOLDER");
    let fitted = ctx.fit_text(label, 1.6, DETAIL_W - 80.0);
    ctx.draw_text(r, DETAIL_X + 40.0, 168.0, 1.6, Color::rgb(210, 210, 150), &fitted);
    if count > 0 {
        ctx.draw_text(r, DETAIL_X + 40.0, 240.0, 1.4, th.text_dim, &format!("{count} CHARTS"));
    }
}

fn render_modal<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, modal: &SelectModal, hot: &mut Vec<(Rect, SelectHot)>) {
    let th = ctx.theme;
    r.fill_rect(Rect::new(0.0, 0.0, 1280.0, 720.0), Color { r: 0, g: 0, b: 0, a: 180 });
    let (mw, mh) = (720.0, 470.0);
    let mx = (1280.0 - mw) * 0.5;
    let my = (720.0 - mh) * 0.5;
    r.fill_rect(Rect::new(mx, my, mw, mh), th.panel_hi);
    r.fill_rect(Rect::new(mx, my, mw, 56.0), modal.clear_color);
    ctx.draw_text(r, mx + 20.0, my + 16.0, 2.6, Color::BLACK, modal.clear_label);
    ctx.draw_text_right(r, mx + mw - 20.0, my + 20.0, 1.4, Color::BLACK, &modal.when);
    let fitted = ctx.fit_text(&modal.title, 1.5, mw - 40.0);
    ctx.draw_text(r, mx + 20.0, my + 72.0, 1.5, th.text_dim, &fitted);
    ctx.draw_text(r, mx + 20.0, my + 100.0, 1.3, th.text_dim, &modal.sub);

    const NAMES: [&str; 6] = ["PGREAT", "GREAT", "GOOD", "BAD", "POOR", "MISS"];
    const COLS: [Color; 6] = [Color::GREEN, Color::BLUE, Color::YELLOW, Color::ORANGE, Color::RED, Color::rgb(150, 60, 60)];
    for k in 0..6 {
        let y = my + 138.0 + k as f32 * 30.0;
        ctx.draw_text(r, mx + 30.0, y, 1.7, COLS[k], NAMES[k]);
        ctx.draw_text_right(r, mx + 330.0, y, 1.7, th.text, &modal.counts[k].to_string());
    }
    let rx = mx + 380.0;
    ctx.draw_text(r, rx, my + 138.0, 1.7, th.text, &format!("EX  {} / {}", modal.ex, modal.max_ex));
    if let Some(rank) = modal.rank {
        ctx.draw_text_right(r, mx + mw - 24.0, my + 138.0, 1.7, modal.rank_color, rank);
    }
    ctx.draw_text(r, rx, my + 168.0, 1.7, Color::GREEN, &format!("COMBO  {} / {}", modal.max_combo, modal.total_notes));
    ctx.draw_text(r, rx, my + 198.0, 1.7, Color::ORANGE, &format!("BP  {}", modal.bp));
    ctx.draw_text(r, rx, my + 228.0, 1.7, th.text_dim, &format!("EMPTY POOR  {}", modal.empty_poor));
    ctx.draw_text(r, rx, my + 258.0, 1.7, modal.clear_color, &format!("GAUGE  {}%", modal.gauge_value));
    ctx.draw_text(r, rx, my + 288.0, 1.3, th.text_dim, &format!("{} / {}", modal.index + 1, modal.total));

    let by = my + mh - 52.0;
    if modal.has_replay {
        let rect = Rect::new(mx + 20.0, by, 220.0, 36.0);
        r.fill_rect(rect, Color::rgb(70, 120, 80));
        ctx.draw_text(r, mx + 36.0, by + 9.0, 1.6, th.text, "PLAY REPLAY");
        hot.push((rect, SelectHot::ModalReplay));
    } else {
        ctx.draw_text(r, mx + 24.0, by + 9.0, 1.3, th.text_dim, "NO REPLAY SAVED");
    }
    ctx.draw_text(r, mx + 20.0, my + mh - 92.0, 1.1, th.text_dim, "UP DOWN PREV/NEXT   ENTER REPLAY   ESC CLOSE");
    let close = Rect::new(mx + mw - 120.0, by, 100.0, 36.0);
    r.fill_rect(close, Color::rgb(90, 70, 80));
    ctx.draw_text(r, mx + mw - 96.0, by + 9.0, 1.6, th.text, "CLOSE");
    hot.push((close, SelectHot::ModalClose));
}
