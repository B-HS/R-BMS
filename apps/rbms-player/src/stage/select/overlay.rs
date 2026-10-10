//! What the application draws over a browser a skin draws.
//!
//! A skin owns the whole screen, but some of the browser is the application's rather than the
//! skin's, and a skin has nothing to say it with: the hint on a list with nothing in it, the IR
//! ranking panel, the record modal, the filter panel, the search box (the skin's own search text
//! shows nothing of what is typed on the keyboard), the course tab's mark and the key guide
//! ([`super::guide`]). They are laid out for the fixed screen the application's own screens use and
//! drawn scaled out to the skin's, on top of it.
//!
//! Every one of them is also something to click on. Each registers where it is among the frame's
//! clickable regions, in the order it is drawn, so a press that lands on one is the application's and
//! never reaches the skin's objects beneath it ([`crate::AppShared::skin_pointer`] stands aside for
//! anything the frame made clickable). The record modal and the guide are over the whole screen:
//! a press anywhere on them is theirs.
//!
//! None of this is drawn for a browser no skin draws, which has all of it already.

use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_centered, draw_text_right, fit_text, select_layout, text_width, theme};

use super::SelectState;
use super::filter::{filter_panel_rect, render_filter_panel};
use super::guide::render_guide;
use crate::ir_ranking_view::{PanelLine, render_ranking_panel_at};
use crate::{AppShared, CH, CW, Hot, SelectScene, SelectTab};

/// Where the strip of the browser's own marks starts: its left edge, and the line its lowest mark
/// stands on. It is in the part of ModernChic's browser nothing is drawn in, between the stage file
/// and the three windows along the bottom, and every mark is stacked upwards from there.
pub(super) const STRIP_X: f32 = 16.0;
pub(super) const STRIP_BOTTOM: f32 = 572.0;
const STRIP_GAP: f32 = 8.0;

/// The search box: how wide and tall it is, how its text is laid out in it, and what it says before
/// the query.
const SEARCH_W: f32 = 480.0;
pub(super) const SEARCH_H: f32 = 36.0;
const SEARCH_SCALE: f32 = 1.4;
const SEARCH_LABEL: &str = "SEARCH";
const SEARCH_PAD_X: f32 = 12.0;
const SEARCH_LABEL_W: f32 = 88.0;
const HITS_SCALE: f32 = 1.1;

/// The mark of the course tab.
pub(super) const TAB_MARK_H: f32 = 28.0;
const TAB_MARK_SCALE: f32 = 1.3;
const TAB_MARK_HINT: &str = "SHIFT+TAB FOR SONGS";
const TAB_MARK_HINT_SCALE: f32 = 1.0;
const TAB_MARK_PAD_X: f32 = 12.0;
const TAB_MARK_GAP: f32 = 16.0;

/// The hint on a list with nothing in it: the card it is on, centred on the screen, and how its two
/// lines are set.
pub(super) const HINT_Y: f32 = 252.0;
pub(super) const HINT_H: f32 = 128.0;
const HINT_PAD_X: f32 = 40.0;
const HINT_MARGIN_X: f32 = 32.0;
const HINT_TITLE_SCALE: f32 = 2.2;
const HINT_BODY_SCALE: f32 = 1.3;
const HINT_TITLE_Y: f32 = 26.0;
const HINT_BODY_Y: f32 = 80.0;

/// How thick the lines round the panels are: a hairline, and the heavier one the search box has.
const HAIRLINE: f32 = 1.0;
const SEARCH_OUTLINE: f32 = 1.5;

/// How far a line of text sits down in a box of a given height, in text scales: the height of a line
/// at scale one.
const TEXT_LINE: f32 = 14.0;

/// Where text goes in a box `height` tall so that it is in the middle of it.
fn centred_text_y(top: f32, height: f32, scale: f32) -> f32 {
    top + (height - scale * TEXT_LINE) * 0.5
}

/// A hairline round a rectangle.
fn outline<R: Renderer>(r: &mut R, rect: Rect, thickness: f32, color: Color) {
    r.fill_rect(Rect::new(rect.x, rect.y, rect.w, thickness), color);
    r.fill_rect(Rect::new(rect.x, rect.y + rect.h - thickness, rect.w, thickness), color);
    r.fill_rect(Rect::new(rect.x, rect.y, thickness, rect.h), color);
    r.fill_rect(Rect::new(rect.x + rect.w - thickness, rect.y, thickness, rect.h), color);
}

/// Where the IR ranking panel is over a skin's browser: where the built-in browser's own list is,
/// as tall and as wide as its detail column is, so the skin's list stays in view beside it.
pub(super) fn ranking_panel_rect() -> Rect {
    let layout = select_layout();
    Rect::new(layout.list_rect.x, layout.detail_rect.y, layout.detail_rect.w, layout.detail_rect.h)
}

/// The card a list with nothing in it puts its hint on.
fn draw_empty_hint<R: Renderer>(r: &mut R, title: &str, body: &str) {
    let th = theme();
    let widest = text_width(title, HINT_TITLE_SCALE).max(text_width(body, HINT_BODY_SCALE));
    let width = (widest + HINT_PAD_X * 2.0).min(CW as f32 - HINT_MARGIN_X * 2.0);
    let card = Rect::new((CW as f32 - width) * 0.5, HINT_Y, width, HINT_H);
    r.fill_rect(card, Color { a: th.select_panel_alpha, ..th.panel_hi });
    outline(r, card, HAIRLINE, th.divider);
    let centre = card.x + card.w * 0.5;
    draw_text_centered(r, centre, card.y + HINT_TITLE_Y, HINT_TITLE_SCALE, th.text, title);
    draw_text_centered(r, centre, card.y + HINT_BODY_Y, HINT_BODY_SCALE, th.text_dim, &fit_text(body, HINT_BODY_SCALE, card.w - HINT_PAD_X));
}

/// The search box, with the query as it is typed and how many charts it found. Returns the line it
/// stands on, so what is drawn above it stands clear.
fn draw_search_box<R: Renderer>(r: &mut R, bottom: f32, query: &str, hits: usize) -> f32 {
    let th = theme();
    let rect = Rect::new(STRIP_X, bottom - SEARCH_H, SEARCH_W, SEARCH_H);
    r.fill_rect(rect, th.button_active);
    outline(r, rect, SEARCH_OUTLINE, th.focus);
    let text_y = centred_text_y(rect.y, rect.h, SEARCH_SCALE);
    draw_text(r, rect.x + SEARCH_PAD_X, text_y, SEARCH_SCALE, th.text_dim, SEARCH_LABEL);
    let hits_label = format!("{hits} HITS");
    let room = rect.w - SEARCH_PAD_X * 2.0 - SEARCH_LABEL_W - text_width(&hits_label, HITS_SCALE) - SEARCH_PAD_X;
    draw_text(r, rect.x + SEARCH_PAD_X + SEARCH_LABEL_W, text_y, SEARCH_SCALE, th.text, &fit_text(query, SEARCH_SCALE, room));
    draw_text_right(r, rect.x + rect.w - SEARCH_PAD_X, centred_text_y(rect.y, rect.h, HITS_SCALE), HITS_SCALE, th.text_dim, &hits_label);
    rect.y - STRIP_GAP
}

/// The mark that says the list on show is the course list, with the key that goes back.
fn draw_course_mark<R: Renderer>(r: &mut R, bottom: f32) -> f32 {
    let th = theme();
    let label = SelectTab::Courses.label();
    let width = TAB_MARK_PAD_X * 2.0 + text_width(label, TAB_MARK_SCALE) + TAB_MARK_GAP + text_width(TAB_MARK_HINT, TAB_MARK_HINT_SCALE);
    let rect = Rect::new(STRIP_X, bottom - TAB_MARK_H, width, TAB_MARK_H);
    r.fill_rect(rect, th.button_active);
    outline(r, rect, HAIRLINE, th.focus);
    draw_text(r, rect.x + TAB_MARK_PAD_X, centred_text_y(rect.y, rect.h, TAB_MARK_SCALE), TAB_MARK_SCALE, th.accent, label);
    let hint_x = rect.x + TAB_MARK_PAD_X + text_width(label, TAB_MARK_SCALE) + TAB_MARK_GAP;
    draw_text(r, hint_x, centred_text_y(rect.y, rect.h, TAB_MARK_HINT_SCALE), TAB_MARK_HINT_SCALE, th.text_muted, TAB_MARK_HINT);
    rect.y - STRIP_GAP
}

impl SelectState {
    /// Draw the application's own panels over the skin that has just drawn the browser, and record
    /// where each of them takes a press.
    ///
    /// They are drawn from the lowest to the highest -- the hint, the strip of marks, the filter, the
    /// ranking, the guide, the record modal -- and registered in that order, which is what makes the
    /// one drawn on top the one a press lands on.
    ///
    /// The search box is drawn for every search that is open except one being typed into the skin's
    /// own search text, which draws what is typed itself. The skin's search text never shows the query
    /// typed on the keyboard -- the host answers it with nothing -- so a skin that has one is no
    /// reason to leave the box out: it is what the reference's search popup is.
    pub(super) fn draw_skin_overlays<R: Renderer>(&self, r: &mut R, shared: &mut AppShared, view: &SelectScene, ranking: &[PanelLine]) {
        let mut hot: Vec<(Rect, Hot)> = Vec::new();
        if let Some((title, body)) = view.empty_hint {
            draw_empty_hint(r, title, body);
        }
        let mut bottom = STRIP_BOTTOM;
        if shared.searching && !shared.skin_text_is_focused() {
            bottom = draw_search_box(r, bottom, &self.search_display(), view.rows.len());
        }
        if self.tab == SelectTab::Courses {
            draw_course_mark(r, bottom);
        }
        if self.filter.is_open() {
            render_filter_panel(r, &self.filter, &shared.config);
            hot.push((filter_panel_rect(), Hot::OverlayPanel));
        }
        if self.ranking_open {
            let panel = ranking_panel_rect();
            hot.push((panel, Hot::OverlayPanel));
            let rows = render_ranking_panel_at(r, panel, ranking, self.ranking_sel, true, shared.can_switch_primary_ir());
            hot.extend(rows.into_iter().map(|(rect, index)| (rect, Hot::RankingRow(index))));
        }
        if self.guide_open {
            hot.extend(render_guide(r, shared));
        }
        if let Some(modal) = &view.modal {
            hot.push((Rect::new(0.0, 0.0, CW as f32, CH as f32), Hot::ModalClose));
            let regions = rbms_render::render_select_modal(r, modal);
            hot.extend(regions.into_iter().filter_map(|(rect, region)| match region {
                rbms_render::SelectHot::ModalReplay => Some((rect, Hot::ModalReplay)),
                rbms_render::SelectHot::ModalClose => Some((rect, Hot::ModalClose)),
                _ => None,
            }));
        }
        shared.hot.extend(hot);
    }
}
