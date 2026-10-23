use rbms_model::translations::Catalog;

use crate::ctx::{RenderCtx, with_render_ctx};
use crate::result::{RANK_BANDS, dj_rank, draw_rank_bar};
use crate::text::select::SelectText;
use crate::{Color, Rect, Renderer};

/// Clickable region kinds the song-select screen reports back to the app so it can map them onto its
/// own input handling without the renderer knowing the app's `Hot` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectHot {
    Row(usize),
    Record(usize),
    ModalReplay,
    /// The record modal's button that plays the chart laid out as the shown record's replay was.
    ModalLayout,
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
    /// DJ rank of the chart's best run (`AAA`, `A` …), or `None` when nothing has been recorded on
    /// it or the app is not showing ranks.
    pub dj_level: Option<&'static str>,
    /// The chart has been starred, which the row marks next to its clear lamp.
    pub favorite: bool,
    /// The row is a chart a difficulty table lists that the library does not hold: drawn dimmed,
    /// with no lamp and no mode or level.
    pub missing: bool,
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
    Folder {
        label: String,
        count: usize,
    },
    /// A chart a difficulty table lists that the library does not hold, as the table names it.
    Missing {
        title: String,
        artist: String,
    },
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
    /// Where the play stands with the score server, and the colour it is said in; `None` when no
    /// score server is set up.
    pub ir_line: Option<(String, Color)>,
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
    /// The query as the search box draws it while it is open, caret and all (`None` when not
    /// searching). The app composes it, so where the caret sits is the app's to decide.
    pub search: Option<String>,
    /// Current sort-order label (e.g. `"DEFAULT"`, `"TITLE"`), shown top-right.
    pub sort: &'static str,
    /// What the browser's filter is taking out (e.g. `"LV 10–12  FAVOURITES"`), shown next to the
    /// sort order, or `None` when it is taking nothing out. A filter with its panel closed would
    /// otherwise be invisible, and a filtered library looks like a lost one.
    pub filter: Option<String>,
    /// Shown centered when the list is empty: `(title, body)` onboarding/empty hint (e.g. a first-run
    /// "add a music folder" call to action). Ignored when rows exist.
    pub empty_hint: Option<(&'static str, &'static str)>,
}

/// Width of the marker a starred chart carries, drawn in the gap between the clear lamp and the
/// mode badge so it costs the title no room.
const FAVORITE_W: f32 = 4.0;

/// Room kept on the right of a row for its DJ rank, so a long title is trimmed to fit rather than
/// drawn under it.
const DJ_LEVEL_W: f32 = 48.0;

/// Where a chart row's title starts, past its mode and level badges, and the room it leaves on the
/// row's right.
const ROW_TITLE_X: f32 = 116.0;
const ROW_TITLE_RIGHT_ROOM: f32 = 24.0;

/// The detail panel over a chart the library does not hold: the strip across its head, how far its
/// text sits in from the panel's edges, and the size and line height of its title, its artist and
/// its notice.
const MISSING_DETAIL_STRIP_H: f32 = 6.0;
const MISSING_DETAIL_INSET: f32 = 40.0;
const MISSING_DETAIL_TITLE_SCALE: f32 = 2.0;
const MISSING_DETAIL_TITLE_LINE: f32 = 30.0;
const MISSING_DETAIL_ARTIST_SCALE: f32 = 1.3;
const MISSING_DETAIL_ARTIST_LINE: f32 = 24.0;
const MISSING_DETAIL_NOTICE_SCALE: f32 = 1.2;

/// The cover (`#STAGEFILE`) square in the detail panel. The GPU backend uploads the decoded image
/// here; the renderer frames it / draws a placeholder. Kept as a function so both sides share it.
pub fn cover_rect() -> Rect {
    crate::select_layout().cover_rect
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
    r.fill_rect(rect, select_panel_color(if active { th.button_active } else { th.panel_hi }, th.select_panel_alpha));
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

fn select_panel_color(color: Color, alpha: u8) -> Color {
    Color { a: alpha, ..color }
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
    r.clear(th.bg);
    let mut hot = Vec::new();
    render_top_bar(ctx, r, v);
    if v.modal.is_none() {
        render_nav_bar(ctx, r, v, &mut hot);
    }
    render_list(ctx, r, v, &mut hot);
    match &v.detail {
        SelectDetail::Song(d) => render_song_detail(ctx, r, v, d, &mut hot),
        SelectDetail::Folder { label, count } => render_folder_detail(ctx, r, label, *count),
        SelectDetail::Missing { title, artist } => render_missing_detail(ctx, r, title, artist),
        SelectDetail::Empty => {}
    }
    if let Some(modal) = &v.modal {
        render_modal(ctx, r, modal, &mut hot);
    }
    hot
}

/// Draw only the record-detail modal, over whatever the target already shows, for a screen that
/// draws the list and the detail panel itself (a skin) and still needs the application's own
/// modal on top of it. Returns the modal's clickable regions, which are the same as
/// [`render_select`] reports while the modal is up.
pub fn render_select_modal<R: Renderer>(r: &mut R, modal: &SelectModal) -> Vec<(Rect, SelectHot)> {
    with_render_ctx(|ctx| {
        let mut hot = Vec::new();
        render_modal(ctx, r, modal, &mut hot);
        hot
    })
}

/// The strip across the head of the screen: the title, the search box or the open folder, and the
/// ordering with the cursor's place in the list.
fn render_top_bar<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, v: &SelectView) {
    let th = ctx.theme;
    let language = ctx.language;
    let list = th.select_layout.list_rect;
    r.fill_rect(Rect::new(0.0, 0.0, 1280.0, 52.0), select_panel_color(th.topbar, th.select_panel_alpha));
    ctx.draw_text(r, list.x, 12.0, 2.6, th.text, SelectText::Title.get(language));
    let m = v.rows.len();
    if let Some(q) = &v.search {
        let bx = list.x + list.w - 326.0;
        let bw = 240.0;
        r.fill_rect(Rect::new(bx, 10.0, bw, 32.0), select_panel_color(th.button_active, th.select_panel_alpha));
        outline(r, Rect::new(bx, 10.0, bw, 32.0), 1.5, th.focus);
        let fitted = ctx.fit_text(&format!("\u{1F50D} {q}"), 1.4, bw - 20.0);
        ctx.draw_text(r, bx + 10.0, 18.0, 1.4, th.text, &fitted);
        ctx.draw_text(r, bx + bw + 12.0, 20.0, 1.2, th.text_dim, &SelectText::SearchHits.fill(language, &[&m]));
    } else if !v.header.is_empty() {
        let header_x = list.x + 268.0;
        let fitted = ctx.fit_text(&v.header, 1.3, list.x + list.w - header_x - 80.0);
        ctx.draw_text(r, header_x, 22.0, 1.3, th.text_dim, &fitted);
    }
    let sort_line = match &v.filter {
        Some(filter) => SelectText::SortAndFilter.fill(language, &[&v.sort, filter]),
        None => SelectText::SortLine.fill(language, &[&v.sort]),
    };
    ctx.draw_text_right(r, list.x + list.w, 6.0, 1.0, th.accent, &sort_line);
    ctx.draw_text_right(r, list.x + list.w, 24.0, 1.4, th.text_dim, &format!("{}/{}", (v.sel + 1).min(m.max(1)), m));
}

/// The row of buttons under the list, each the clickable equivalent of one of the browser's keys.
fn render_nav_bar<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, v: &SelectView, hot: &mut Vec<(Rect, SelectHot)>) {
    let th = ctx.theme;
    let language = ctx.language;
    let list = th.select_layout.list_rect;
    let by = list.y + list.h + 8.0;
    let sort_btn = SelectText::SortLine.fill(language, &[&v.sort]);
    let search = SelectText::Search.get(language);
    let search_open = v.search.is_some().then(|| format!("\u{2715} {search}"));
    let buttons: [(&str, &str, SelectHot); 6] = [
        (search_open.as_deref().unwrap_or(search), "/", SelectHot::Search),
        (&sort_btn, "F3", SelectHot::Sort),
        (SelectText::Folders.get(language), "O", SelectHot::Folders),
        (SelectText::Tables.get(language), "T", SelectHot::Tables),
        (SelectText::Records.get(language), "R", SelectHot::Records),
        (SelectText::Settings.get(language), "TAB", SelectHot::Settings),
    ];
    let mut bx = list.x.min(th.select_layout.detail_rect.x);
    for (label, key, h) in buttons {
        let active = matches!(h, SelectHot::Search) && v.search.is_some();
        let w = nav_button(ctx, r, bx, by, label, key, active);
        hot.push((Rect::new(bx, by, w, 30.0), h));
        bx += w + 8.0;
    }
    ctx.draw_text_right(r, 1280.0 - 20.0, by + 9.0, 1.0, th.text_muted, v.guide);
}

fn render_list<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, v: &SelectView, hot: &mut Vec<(Rect, SelectHot)>) {
    let th = ctx.theme;
    let language = ctx.language;
    let layout = th.select_layout;
    let list = layout.list_rect;
    let m = v.rows.len();
    if m == 0 {
        let cx = list.x + list.w * 0.5;
        match v.empty_hint {
            Some((title, body)) => {
                ctx.draw_text_centered(r, cx, list.y + 80.0, 2.2, th.text, title);
                ctx.draw_text_centered(r, cx, list.y + 124.0, 1.3, th.text_dim, body);
            }
            None => ctx.draw_text(r, list.x + 12.0, list.y + 40.0, 1.5, th.text_dim, SelectText::NoCharts.get(language)),
        }
        return;
    }
    let visible = (list.h / (layout.row_height + layout.row_gap)).floor() as usize;
    let start = v.sel.saturating_sub(visible / 2).min(m.saturating_sub(visible.min(m)));
    let modal_open = v.modal.is_some();
    let mut y = list.y;
    for idx in start..(start + visible).min(m) {
        let row = &v.rows[idx];
        let focused = idx == v.sel;
        let h = layout.row_height;
        let bg = if focused {
            select_panel_color(th.row_focus, th.select_panel_alpha)
        } else if row.folder {
            select_panel_color(th.row_folder, th.select_panel_alpha)
        } else if idx % 2 == 0 {
            select_panel_color(th.row_even, th.select_panel_alpha)
        } else {
            select_panel_color(th.row_odd, th.select_panel_alpha)
        };
        r.fill_rect(Rect::new(list.x, y, list.w, h), bg);
        if focused {
            r.fill_rect(Rect::new(list.x, y, 2.0, h), th.focus);
            r.fill_rect(Rect::new(list.x + list.w - 2.0, y, 2.0, h), th.focus);
        }
        if !row.missing {
            r.fill_rect(Rect::new(list.x, y, 6.0, h), row.lamp);
            r.fill_rect(Rect::new(list.x + list.w - 10.0, y, 8.0, h), row.lamp);
        }
        if row.favorite {
            r.fill_rect(Rect::new(list.x + 6.0, y, FAVORITE_W, h), th.accent);
        }
        let cy = y + h * 0.5;
        if row.folder {
            let fitted = ctx.fit_text(&format!("\u{25B8} {}", row.title), 1.6, list.w - 120.0);
            ctx.draw_text(r, list.x + 16.0, cy - 9.0, 1.6, th.text, &fitted);
            if let Some(n) = row.folder_count {
                ctx.draw_text_right(r, list.x + list.w - 20.0, cy - 8.0, 1.3, th.text_dim, &SelectText::ChartCount.fill(language, &[&n]));
            }
        } else if row.missing {
            let title_scale = if focused { 1.7 } else { 1.5 };
            let fitted = ctx.fit_text(&row.title, title_scale, list.w - ROW_TITLE_X - ROW_TITLE_RIGHT_ROOM);
            ctx.draw_text(r, list.x + ROW_TITLE_X, cy - title_scale * 7.0, title_scale, th.text_muted, &fitted);
        } else {
            let mode_chip = Badge {
                rect: Rect::new(list.x + 12.0, cy - 12.0, 46.0, 24.0),
                fill: darken(row.mode_color, 0.45),
                border: row.mode_color,
                scale: 1.2,
                text_color: Color::WHITE,
            };
            badge(ctx, r, &mode_chip, row.mode_short);
            let level_chip = Badge {
                rect: Rect::new(list.x + 64.0, cy - 12.0, 42.0, 24.0),
                fill: darken(row.difficulty_color, 0.4),
                border: row.difficulty_color,
                scale: 1.4,
                text_color: Color::WHITE,
            };
            badge(ctx, r, &level_chip, &row.level);
            let title_scale = if focused { 1.7 } else { 1.5 };
            let title_color = if focused { th.title_focus } else { th.title_dim };
            let rank_room = if row.dj_level.is_some() { DJ_LEVEL_W } else { 0.0 };
            let fitted = ctx.fit_text(&row.title, title_scale, list.w - 116.0 - 24.0 - rank_room);
            ctx.draw_text(r, list.x + 116.0, cy - title_scale * 7.0, title_scale, title_color, &fitted);
            if let Some(rank) = row.dj_level {
                ctx.draw_text_right(r, list.x + list.w - 16.0, cy - 8.0, 1.3, th.text_dim, rank);
            }
        }
        if !modal_open {
            hot.push((Rect::new(list.x, y, list.w, h), SelectHot::Row(idx)));
        }
        y += h + layout.row_gap;
    }
}

fn render_song_detail<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, v: &SelectView, d: &DetailView, hot: &mut Vec<(Rect, SelectHot)>) {
    let th = ctx.theme;
    let language = ctx.language;
    let detail = th.select_layout.detail_rect;
    outline(r, detail, 1.0, th.divider);
    r.fill_rect(Rect::new(detail.x, detail.y, detail.w, 6.0), d.accent);

    let cr = cover_rect();
    match d.cover {
        CoverState::Present => {}
        CoverState::Loading => {
            r.fill_rect(cr, th.panel);
            ctx.draw_text_centered(r, cr.x + cr.w * 0.5, cr.y + cr.h * 0.5 - 7.0, 1.2, th.text_dim, SelectText::Loading.get(language));
        }
        CoverState::None => {
            r.fill_rect(cr, th.panel);
            ctx.draw_text_centered(r, cr.x + cr.w * 0.5, cr.y + cr.h * 0.5 - 7.0, 1.2, th.text_muted, SelectText::NoImage.get(language));
        }
    }
    outline(r, cr, 2.0, th.divider);

    let tx = detail.x + 192.0;
    let tw = detail.x + detail.w - 16.0 - tx;
    let (l1, l2) = wrap_two(ctx, &d.title, 2.0, tw);
    let mut ty = detail.y + 26.0;
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
    let mode_chip = Badge {
        rect: Rect::new(tx, detail.y + 136.0, 56.0, 26.0),
        fill: darken(d.mode_color, 0.45),
        border: d.mode_color,
        scale: 1.4,
        text_color: Color::WHITE,
    };
    badge(ctx, r, &mode_chip, d.mode_short);
    let level_chip = Badge {
        rect: Rect::new(tx + 64.0, detail.y + 136.0, 64.0, 26.0),
        fill: darken(d.difficulty_color, 0.4),
        border: d.difficulty_color,
        scale: 1.4,
        text_color: Color::WHITE,
    };
    badge(ctx, r, &level_chip, &format!("Lv{}", d.level));
    ctx.draw_text(r, tx + 138.0, detail.y + 141.0, 1.3, d.difficulty_color, d.difficulty_name);

    let inner_x = detail.x + 16.0;
    let inner_w = detail.w - 32.0;
    r.fill_rect(Rect::new(inner_x, detail.y + 194.0, inner_w, 2.0), th.divider);
    let col_w = inner_w / 3.0;
    for (i, cell) in d.stats.iter().take(6).enumerate() {
        let cx = inner_x + 6.0 + (i % 3) as f32 * col_w;
        let cy = detail.y + 208.0 + (i / 3) as f32 * 34.0;
        ctx.draw_text(r, cx, cy, 1.0, th.text_dim, cell.label);
        let fitted = ctx.fit_text(&cell.value, 1.4, col_w - 12.0);
        ctx.draw_text(r, cx, cy + 14.0, 1.4, th.text, &fitted);
    }

    r.fill_rect(Rect::new(inner_x, detail.y + 288.0, inner_w, 2.0), th.divider);
    ctx.draw_text(r, inner_x, detail.y + 302.0, 1.3, th.text_dim, SelectText::Density.get(language));
    let box_y = detail.y + 320.0;
    let box_h = 74.0;
    let box_rect = Rect::new(inner_x, box_y, inner_w, box_h);
    r.fill_rect(box_rect, th.bg);
    match &d.density {
        Some(dens) if !dens.bins.is_empty() && dens.peak > 0.0 => {
            ctx.draw_text_right(
                r,
                detail.x + detail.w - 16.0,
                detail.y + 304.0,
                1.0,
                th.text_dim,
                &SelectText::DensityLine.fill(language, &[&(dens.peak.round() as i32), &(dens.avg.round() as i32), &(dens.end.round() as i32)]),
            );
            let n = dens.bins.len();
            let step = inner_w / n as f32;
            for (i, &b) in dens.bins.iter().enumerate() {
                let bx = inner_x + i as f32 * step;
                if i % 10 == 0 {
                    r.fill_rect(Rect::new(bx, box_y, 1.0, box_h), th.panel);
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
    let language = ctx.language;
    let detail = th.select_layout.detail_rect;
    let inner_x = detail.x + 16.0;
    let inner_w = detail.w - 32.0;
    ctx.draw_text(r, inner_x, detail.y + 408.0, 1.4, th.text, SelectText::Records.get(language));
    ctx.draw_text_right(
        r,
        detail.x + detail.w - 16.0,
        detail.y + 412.0,
        1.2,
        th.text_dim,
        &SelectText::ClearAndPlays.fill(language, &[&rec.clears, &rec.plays]),
    );
    let Some(best) = &rec.best else {
        ctx.draw_text(r, inner_x, detail.y + 448.0, 1.3, th.text_dim, SelectText::NoPlayYet.get(language));
        return;
    };
    r.fill_rect(Rect::new(inner_x, detail.y + 432.0, inner_w, 28.0), best.lamp);
    ctx.draw_text(r, inner_x + 10.0, detail.y + 438.0, 1.4, Color::BLACK, best.lamp_label);
    ctx.draw_text_right(r, inner_x + inner_w - 10.0, detail.y + 438.0, 1.4, Color::BLACK, &format!("EX {}", best.ex));
    let mut list_top = detail.y + 472.0;
    if let Some((ex, max_ex)) = rec.rank_bar {
        let (rank, rcol) = RANK_BANDS[dj_rank(ex, max_ex)];
        ctx.draw_text(r, inner_x, detail.y + 470.0, 1.3, rcol, &SelectText::RankLine.fill(language, &[&rank]));
        draw_rank_bar(r, inner_x + 92.0, detail.y + 474.0, inner_w - 92.0, 12.0, ex, max_ex);
        list_top = detail.y + 498.0;
    }
    let rh = 32.0;
    let max_rows = ((detail.y + detail.h - list_top) / rh).floor() as usize;
    let modal_open = v.modal.is_some();
    for (ri, rec_row) in rec.recent.iter().take(max_rows).enumerate() {
        let y = list_top + ri as f32 * rh;
        r.fill_rect(Rect::new(inner_x, y, inner_w, rh - 4.0), select_panel_color(th.panel, th.select_panel_alpha));
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
    let language = ctx.language;
    let detail = th.select_layout.detail_rect;
    outline(r, detail, 1.0, th.divider);
    r.fill_rect(Rect::new(detail.x, detail.y, detail.w, 6.0), th.text_muted);
    r.fill_rect(Rect::new(detail.x + 16.0, detail.y + 36.0, detail.w - 32.0, 120.0), select_panel_color(th.panel_hi, th.select_panel_alpha));
    ctx.draw_text(r, detail.x + 40.0, detail.y + 60.0, 2.4, th.text, SelectText::FolderTitle.get(language));
    let fitted = ctx.fit_text(label, 1.6, detail.w - 80.0);
    ctx.draw_text(r, detail.x + 40.0, detail.y + 108.0, 1.6, th.text, &fitted);
    if count > 0 {
        ctx.draw_text(r, detail.x + 40.0, detail.y + 180.0, 1.4, th.text_dim, &SelectText::ChartCount.fill(language, &[&count]));
    }
}

/// The detail panel over a chart the library does not hold: what its table calls it and who it is
/// by, and that it is not here to be played. It is laid out as the folder panel is, dimmed.
fn render_missing_detail<R: Renderer>(ctx: &mut RenderCtx<'_>, r: &mut R, title: &str, artist: &str) {
    let th = ctx.theme;
    let detail = th.select_layout.detail_rect;
    outline(r, detail, 1.0, th.divider);
    r.fill_rect(Rect::new(detail.x, detail.y, detail.w, MISSING_DETAIL_STRIP_H), th.text_muted);
    let x = detail.x + MISSING_DETAIL_INSET;
    let width = detail.w - MISSING_DETAIL_INSET * 2.0;
    let (first, second) = wrap_two(ctx, title, MISSING_DETAIL_TITLE_SCALE, width);
    let mut y = detail.y + MISSING_DETAIL_INSET;
    for line in std::iter::once(first).chain(second) {
        ctx.draw_text(r, x, y, MISSING_DETAIL_TITLE_SCALE, th.text_muted, &line);
        y += MISSING_DETAIL_TITLE_LINE;
    }
    if !artist.is_empty() {
        let fitted = ctx.fit_text(artist, MISSING_DETAIL_ARTIST_SCALE, width);
        ctx.draw_text(r, x, y, MISSING_DETAIL_ARTIST_SCALE, th.text_dim, &fitted);
        y += MISSING_DETAIL_ARTIST_LINE;
    }
    let fitted = ctx.fit_text(SelectText::MissingChartNotice.get(ctx.language), MISSING_DETAIL_NOTICE_SCALE, width);
    ctx.draw_text(r, x, y + MISSING_DETAIL_ARTIST_LINE, MISSING_DETAIL_NOTICE_SCALE, th.accent, &fitted);
}

/// Where the record modal says how the play stands with the score server: under the judgements and
/// above the key guide, across the modal's width.
const MODAL_IR_LINE_Y: f32 = 340.0;
const MODAL_IR_LINE_INSET: f32 = 20.0;
const MODAL_IR_LINE_SCALE: f32 = 1.3;

/// Where the record modal's layout button stands: its left edge beside the replay button's right
/// edge and how far from it, how wide and tall it is, and how its label is set in it. The modal's
/// key hint for the button stands on the line of the modal's other keys, at its right edge.
const MODAL_REPLAY_BUTTON_RIGHT: f32 = 240.0;
const MODAL_LAYOUT_BUTTON_GAP: f32 = 12.0;
const MODAL_LAYOUT_BUTTON_W: f32 = 260.0;
const MODAL_BUTTON_H: f32 = 36.0;
const MODAL_BUTTON_LABEL_SCALE: f32 = 1.6;
const MODAL_BUTTON_LABEL_DROP: f32 = 9.0;
const MODAL_LAYOUT_LABEL_INSET: f32 = 16.0;
const MODAL_KEYS_LINE_FROM_BOTTOM: f32 = 92.0;
const MODAL_KEYS_LINE_INSET: f32 = 20.0;
const MODAL_KEYS_LINE_SCALE: f32 = 1.1;

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
    ctx.draw_text(r, rx, my + 258.0, 1.7, modal.clear_color, &SelectText::ModalGauge.fill(ctx.language, &[&modal.gauge_value]));
    ctx.draw_text(r, rx, my + 288.0, 1.3, th.text_dim, &format!("{} / {}", modal.index + 1, modal.total));
    if let Some((line, color)) = &modal.ir_line {
        let fitted = ctx.fit_text(line, MODAL_IR_LINE_SCALE, mw - MODAL_IR_LINE_INSET * 2.0);
        ctx.draw_text(r, mx + MODAL_IR_LINE_INSET, my + MODAL_IR_LINE_Y, MODAL_IR_LINE_SCALE, *color, &fitted);
    }

    let by = my + mh - 52.0;
    if modal.has_replay {
        let rect = Rect::new(mx + 20.0, by, 220.0, 36.0);
        r.fill_rect(rect, th.button);
        ctx.draw_text(r, mx + 36.0, by + 9.0, 1.6, th.text, SelectText::PlayReplay.get(ctx.language));
        hot.push((rect, SelectHot::ModalReplay));
        let layout = Rect::new(mx + MODAL_REPLAY_BUTTON_RIGHT + MODAL_LAYOUT_BUTTON_GAP, by, MODAL_LAYOUT_BUTTON_W, MODAL_BUTTON_H);
        r.fill_rect(layout, th.button);
        let label = ctx.fit_text(SelectText::PlayThisLayout.get(ctx.language), MODAL_BUTTON_LABEL_SCALE, layout.w - MODAL_LAYOUT_LABEL_INSET * 2.0);
        ctx.draw_text(r, layout.x + MODAL_LAYOUT_LABEL_INSET, by + MODAL_BUTTON_LABEL_DROP, MODAL_BUTTON_LABEL_SCALE, th.text, &label);
        hot.push((layout, SelectHot::ModalLayout));
    } else {
        ctx.draw_text(r, mx + 24.0, by + 9.0, 1.3, th.text_dim, SelectText::NoReplaySaved.get(ctx.language));
    }
    ctx.draw_text(r, mx + 20.0, my + mh - 92.0, 1.1, th.text_dim, SelectText::ModalKeys.get(ctx.language));
    if modal.has_replay {
        let keys = SelectText::ModalLayoutKey.get(ctx.language);
        ctx.draw_text_right(r, mx + mw - MODAL_KEYS_LINE_INSET, my + mh - MODAL_KEYS_LINE_FROM_BOTTOM, MODAL_KEYS_LINE_SCALE, th.text_dim, keys);
    }
    let close = Rect::new(mx + mw - 120.0, by, 100.0, 36.0);
    r.fill_rect(close, th.button_active);
    ctx.draw_text(r, mx + mw - 96.0, by + 9.0, 1.6, th.text, SelectText::Close.get(ctx.language));
    hot.push((close, SelectHot::ModalClose));
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;

    use super::*;
    use crate::text::testing::{differs_within, drawn_in};
    use crate::{CpuCanvas, Theme, ThemeConfig, set_theme};

    fn view() -> SelectView {
        SelectView {
            rows: vec![SelectRow {
                folder: true,
                title: "Folder".to_string(),
                mode_short: "",
                mode_color: Color::BLACK,
                level: String::new(),
                difficulty_color: Color::BLACK,
                lamp: Color::GRAY,
                folder_count: Some(1),
                dj_level: None,
                favorite: false,
                missing: false,
            }],
            sel: 0,
            header: String::new(),
            guide: "",
            detail: SelectDetail::Folder { label: "Folder".to_string(), count: 1 },
            modal: None,
            score_graph: false,
            search: None,
            sort: "DEFAULT",
            filter: None,
            empty_hint: None,
        }
    }

    /// The size of the screen the modal is drawn on.
    const SCREEN_W: u32 = 1280;
    const SCREEN_H: u32 = 720;

    fn modal(has_replay: bool) -> SelectModal {
        SelectModal {
            title: "Chart".to_string(),
            clear_label: "HARD",
            clear_color: Color::WHITE,
            when: "2026-05-31 01:02".to_string(),
            sub: "7K   RANDOM   GAUGE HARD".to_string(),
            counts: [10, 4, 3, 2, 1, 0],
            ex: 24,
            max_ex: 40,
            rank: Some("AA"),
            rank_color: Color::WHITE,
            max_combo: 12,
            total_notes: 20,
            bp: 3,
            empty_poor: 0,
            gauge_value: 80,
            index: 0,
            total: 1,
            has_replay,
            ir_line: None,
        }
    }

    fn region_of(hot: &[(Rect, SelectHot)], wanted: SelectHot) -> Option<Rect> {
        hot.iter().find(|(_, kind)| *kind == wanted).map(|(rect, _)| *rect)
    }

    #[test]
    fn the_modal_of_a_record_with_a_replay_has_a_layout_button_between_the_replay_and_close_buttons() {
        let mut canvas = CpuCanvas::new(SCREEN_W, SCREEN_H);
        let hot = render_select_modal(&mut canvas, &modal(true));
        let (replay, layout, close) = (
            region_of(&hot, SelectHot::ModalReplay).expect("the replay button"),
            region_of(&hot, SelectHot::ModalLayout).expect("the layout button"),
            region_of(&hot, SelectHot::ModalClose).expect("the close button"),
        );
        assert!(replay.x + replay.w < layout.x && layout.x + layout.w < close.x, "the buttons are not in a row: {replay:?} {layout:?} {close:?}");
        assert_eq!((layout.y, layout.h), (replay.y, replay.h), "the layout button is on another line than the replay button");
    }

    #[test]
    fn the_modal_of_a_record_with_no_replay_offers_no_layout() {
        let mut canvas = CpuCanvas::new(SCREEN_W, SCREEN_H);
        let hot = render_select_modal(&mut canvas, &modal(false));
        assert!(region_of(&hot, SelectHot::ModalLayout).is_none() && region_of(&hot, SelectHot::ModalReplay).is_none());
        assert!(region_of(&hot, SelectHot::ModalClose).is_some());
    }

    #[test]
    fn the_layout_button_carries_its_label_and_is_drawn_in_every_language() {
        crate::font::use_embedded_fonts_only();
        for language in rbms_model::Language::ALL {
            crate::text::set_language(language);
            let mut canvas = CpuCanvas::new(SCREEN_W, SCREEN_H);
            let hot = render_select_modal(&mut canvas, &modal(true));
            let layout = region_of(&hot, SelectHot::ModalLayout).expect("the layout button");
            let ground = canvas.pixel_at(layout.x as u32 + 1, layout.y as u32 + 1);
            let lettered = (layout.y as u32..(layout.y + layout.h) as u32)
                .flat_map(|y| (layout.x as u32..(layout.x + layout.w) as u32).map(move |x| (x, y)))
                .filter(|(x, y)| canvas.pixel_at(*x, *y) != ground)
                .count();
            if language == rbms_model::Language::English {
                assert!(lettered > 0, "the button has no label on it");
            }
        }
        crate::text::set_language(rbms_model::Language::English);
    }

    #[test]
    fn select_hot_regions_and_cover_follow_the_configured_layout() {
        let theme = ThemeConfig::parse(
            "(select_layout: Some((list_rect: Some((664.0, 60.0, 584.0, 600.0)), detail_rect: Some((32.0, 60.0, 616.0, 600.0)), cover_rect: Some((48.0, 78.0, 160.0, 160.0)))) )",
        )
        .resolve();
        set_theme(theme);
        let mut canvas = CpuCanvas::new(1280, 720);
        let hot = render_select(&mut canvas, &view());
        let row = hot.iter().find(|(_, kind)| matches!(kind, SelectHot::Row(0))).map(|(rect, _)| *rect);
        assert_eq!(row, Some(Rect::new(664.0, 60.0, 584.0, 36.0)));
        assert!(hot.iter().all(|(rect, _)| rect.x + rect.w <= 1280.0));
        assert_eq!(cover_rect(), Rect::new(48.0, 78.0, 160.0, 160.0));
        set_theme(crate::Theme::default());
    }

    fn translated() -> impl Iterator<Item = Language> {
        Language::ALL.into_iter().filter(|language| *language != Language::English)
    }

    fn in_language(language: Language, view: &SelectView) -> CpuCanvas {
        drawn_in(language, SCREEN_W, SCREEN_H, |ctx, canvas| {
            render_select_ctx(ctx, canvas, view);
        })
    }

    fn modal_in(language: Language, modal: &SelectModal) -> (CpuCanvas, Vec<(Rect, SelectHot)>) {
        let mut hot = Vec::new();
        let canvas = drawn_in(language, SCREEN_W, SCREEN_H, |ctx, canvas| render_modal(ctx, canvas, modal, &mut hot));
        (canvas, hot)
    }

    fn record_row() -> RecordRowView {
        RecordRowView { when: "2026-05-31 01:02".to_string(), lamp: Color::WHITE, lamp_label: "HARD", ex: 1602, max_ex: 1848, bp: 12, trend: None }
    }

    fn song_detail(best: bool) -> SelectDetail {
        SelectDetail::Song(Box::new(DetailView {
            accent: Color::BLUE,
            title: "Chart".to_string(),
            subtitle: String::new(),
            artist: "Artist".to_string(),
            genre_maker: String::new(),
            mode_short: "7K",
            mode_color: Color::BLUE,
            level: "12".to_string(),
            difficulty_color: Color::RED,
            difficulty_name: "ANOTHER",
            cover: CoverState::Loading,
            stats: Vec::new(),
            density: Some(DensityView { bins: vec![1, 3, 2, 5], peak: 5.0, avg: 3.0, end: 2.0 }),
            records: RecordsView {
                plays: 3,
                clears: 2,
                best: best.then(record_row),
                rank_bar: best.then_some((1602, 1848)),
                recent: if best { vec![record_row()] } else { Vec::new() },
            },
        }))
    }

    /// How tall the strip across the head of the screen is, and the row of buttons under the list.
    const HEAD_H: f32 = 52.0;
    const NAV_ROW_GAP: f32 = 8.0;
    const NAV_ROW_H: f32 = 30.0;

    #[test]
    fn the_title_the_buttons_and_the_list_are_written_in_the_language_asked_for() {
        let layout = Theme::default().select_layout;
        let head = Rect::new(0.0, 0.0, SCREEN_W as f32, HEAD_H);
        let buttons = Rect::new(0.0, layout.list_rect.y + layout.list_rect.h + NAV_ROW_GAP, SCREEN_W as f32, NAV_ROW_H);
        let english = in_language(Language::English, &view());
        for language in translated() {
            let drawn = in_language(language, &view());
            assert!(differs_within(&english, &drawn, head), "{language:?}: the title is still English");
            assert!(differs_within(&english, &drawn, buttons), "{language:?}: the buttons are still English");
            assert!(differs_within(&english, &drawn, layout.list_rect), "{language:?}: a folder row's chart count is still English");
        }
    }

    #[test]
    fn the_sort_line_the_search_hits_and_an_empty_list_are_written_in_the_language_asked_for() {
        let layout = Theme::default().select_layout;
        let list = layout.list_rect;
        let sort_line = Rect::new(list.x, 0.0, list.w, 20.0);
        let hits = Rect::new(list.x + list.w - 80.0, 20.0, 80.0, 14.0);
        let empty = SelectView { rows: Vec::new(), ..view() };
        let searching = SelectView { search: Some("abc".to_string()), filter: Some("LV 10".to_string()), ..view() };
        let (english_empty, english_searching) = (in_language(Language::English, &empty), in_language(Language::English, &searching));
        for language in translated() {
            assert!(differs_within(&english_empty, &in_language(language, &empty), list), "{language:?}: the empty list is still English");
            let drawn = in_language(language, &searching);
            assert!(differs_within(&english_searching, &drawn, sort_line), "{language:?}: the sort and filter line is still English");
            assert!(differs_within(&english_searching, &drawn, hits), "{language:?}: the number of hits is still English");
        }
    }

    /// Where the lines of a chart's detail panel stand, from the panel's top: the density heading and
    /// its figures, the records heading and its counts, the note on a chart nobody has played and the
    /// rank of the best record; and how far in from the panel's edge, how wide and how tall the left
    /// and right ends of a line are.
    const DENSITY_HEADING_Y: f32 = 302.0;
    const DENSITY_FIGURES_Y: f32 = 304.0;
    const RECORDS_HEADING_Y: f32 = 408.0;
    const RECORDS_COUNTS_Y: f32 = 412.0;
    const NO_PLAY_Y: f32 = 448.0;
    const BEST_RANK_Y: f32 = 470.0;
    const DETAIL_INSET: f32 = 16.0;
    const DETAIL_HEADING_W: f32 = 90.0;
    const DETAIL_FIGURES_W: f32 = 200.0;
    const DETAIL_LINE_H: f32 = 14.0;

    #[test]
    fn the_detail_panel_of_a_chart_is_written_in_the_language_asked_for() {
        let layout = Theme::default().select_layout;
        let detail = layout.detail_rect;
        let heading = |from: f32| Rect::new(detail.x + DETAIL_INSET, detail.y + from, DETAIL_HEADING_W, DETAIL_LINE_H);
        let figures = |from: f32| Rect::new(detail.x + detail.w - DETAIL_INSET - DETAIL_FIGURES_W, detail.y + from, DETAIL_FIGURES_W, DETAIL_LINE_H);
        let (played, unplayed) = (SelectView { detail: song_detail(true), ..view() }, SelectView { detail: song_detail(false), ..view() });
        let (english_played, english_unplayed) = (in_language(Language::English, &played), in_language(Language::English, &unplayed));
        for language in translated() {
            let drawn = in_language(language, &played);
            assert!(differs_within(&english_played, &drawn, layout.cover_rect), "{language:?}: the cover's note is still English");
            assert!(differs_within(&english_played, &drawn, heading(DENSITY_HEADING_Y)), "{language:?}: the density heading is still English");
            assert!(differs_within(&english_played, &drawn, figures(DENSITY_FIGURES_Y)), "{language:?}: the density figures are still English");
            assert!(differs_within(&english_played, &drawn, heading(RECORDS_HEADING_Y)), "{language:?}: the records heading is still English");
            assert!(differs_within(&english_played, &drawn, figures(RECORDS_COUNTS_Y)), "{language:?}: the clear and play counts are still English");
            assert!(differs_within(&english_played, &drawn, heading(BEST_RANK_Y)), "{language:?}: the rank of the best record is still English");
            let unplayed_drawn = in_language(language, &unplayed);
            assert!(differs_within(&english_unplayed, &unplayed_drawn, heading(NO_PLAY_Y)), "{language:?}: the note on an unplayed chart is still English");
        }
    }

    #[test]
    fn the_detail_panel_of_a_folder_or_a_missing_chart_is_written_in_the_language_asked_for() {
        let detail = Theme::default().select_layout.detail_rect;
        let folder = SelectView { detail: SelectDetail::Folder { label: "Folder".to_string(), count: 12 }, ..view() };
        let missing = SelectView { detail: SelectDetail::Missing { title: "Absent".to_string(), artist: String::new() }, ..view() };
        let (english_folder, english_missing) = (in_language(Language::English, &folder), in_language(Language::English, &missing));
        for language in translated() {
            assert!(differs_within(&english_folder, &in_language(language, &folder), detail), "{language:?}: the folder panel is still English");
            assert!(differs_within(&english_missing, &in_language(language, &missing), detail), "{language:?}: the missing chart's notice is still English");
        }
    }

    /// Where the modal's lines stand relative to its close button: the left end of the row of buttons
    /// and of the line of keys, how far above it the line of keys and the gauge line are, how far
    /// left of it the gauge line starts, and how tall a line is.
    const MODAL_BUTTONS_LEFT_OF_CLOSE: f32 = 580.0;
    const MODAL_KEYS_ABOVE_CLOSE: f32 = 40.0;
    const MODAL_GAUGE_ABOVE_CLOSE: f32 = 160.0;
    const MODAL_GAUGE_LEFT_OF_CLOSE: f32 = 220.0;
    const MODAL_LINE_H: f32 = 22.0;

    #[test]
    fn the_record_modal_is_written_in_the_language_asked_for() {
        for has_replay in [true, false] {
            let (english, english_hot) = modal_in(Language::English, &modal(has_replay));
            let close = region_of(&english_hot, SelectHot::ModalClose).expect("the close button");
            let bottom = Rect::new(close.x - MODAL_BUTTONS_LEFT_OF_CLOSE, close.y, MODAL_BUTTONS_LEFT_OF_CLOSE, close.h);
            let keys = Rect::new(close.x - MODAL_BUTTONS_LEFT_OF_CLOSE, close.y - MODAL_KEYS_ABOVE_CLOSE, MODAL_BUTTONS_LEFT_OF_CLOSE + close.w, MODAL_LINE_H);
            let gauge = Rect::new(close.x - MODAL_GAUGE_LEFT_OF_CLOSE, close.y - MODAL_GAUGE_ABOVE_CLOSE, MODAL_GAUGE_LEFT_OF_CLOSE + close.w, MODAL_LINE_H);
            for language in translated() {
                let (drawn, _) = modal_in(language, &modal(has_replay));
                assert!(differs_within(&english, &drawn, close), "{language:?}: the close button is still English");
                assert!(differs_within(&english, &drawn, bottom), "{language:?}: the buttons of a record with replay {has_replay} are still English");
                assert!(differs_within(&english, &drawn, keys), "{language:?}: the line of keys is still English");
                assert!(differs_within(&english, &drawn, gauge), "{language:?}: the gauge line is still English");
            }
        }
    }
}
