//! The list of captured layouts on LAYOUT PICK: the captures of the chart on show first, or every
//! capture, a cursor on one of them, and a removal waiting for the second press that confirms it.

use rbms_model::Language;
use rbms_model::translations::Catalog;
use rbms_render::{Rect, Renderer, draw_text, fit_text, theme};

use crate::CW;
use crate::captured_layouts::{CapturedLayout, CapturedLayouts};
use crate::format::fmt_datetime;
use crate::stage::Canvas;
use crate::stage::preview_menu::wrapped;
use crate::text::menus::layout_pick::LayoutPickText;

/// Where the list is drawn: right of the field, from the top of its lanes.
const LIST_X: f32 = 928.0;
const LIST_RIGHT_INSET: f32 = 24.0;
const LIST_TOP: f32 = 184.0;

/// How large the heading and the two lines of each row are drawn, and how far apart.
const HEADING_SCALE: f32 = 1.3;
const ROW_SCALE: f32 = 1.2;
const DETAIL_SCALE: f32 = 1.0;
const FIRST_ROW_DROP: f32 = 28.0;
const ROW_HEIGHT: f32 = 40.0;
const DETAIL_DROP: f32 = 18.0;

/// How far the band under the row with the cursor reaches past its text.
const BAND_PAD: f32 = 3.0;

/// How many rows are in view at once.
const VISIBLE_ROWS: usize = 10;

/// How many lines what an empty list says may take.
const NOTHING_CAPTURED_LINES: usize = 2;

/// What the list is drawn for: the chart on show, whether the list has the keys, the key or the two
/// keys a result screen captures a layout on -- `None` while it has none -- and the language on show.
pub(crate) struct ListView<'a> {
    pub(crate) md5: &'a str,
    pub(crate) focused: bool,
    pub(crate) capture_key: Option<&'a str>,
    pub(crate) language: Language,
}

impl ListView<'_> {
    /// What the list says while it is empty: the key a result screen captures a layout on, or, while
    /// that has no key, that one is given in the key editor.
    pub(crate) fn nothing_captured(&self) -> String {
        match self.capture_key {
            Some(key) => LayoutPickText::CapturedNone.fill(self.language, &[&key]),
            None => LayoutPickText::CapturedNoneWithoutKey.get(self.language).to_string(),
        }
    }
}

/// Which captures the list holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Scope {
    /// The captures of the chart on show.
    #[default]
    Chart,
    /// Every capture, of every chart.
    All,
}

/// The list as the screen holds it.
#[derive(Debug, Clone, Default)]
pub(crate) struct CapturedList {
    pub(crate) scope: Scope,
    cursor: usize,
    /// The capture the first press of the removal key was on, until another key is pressed.
    armed: Option<CapturedLayout>,
}

impl CapturedList {
    /// The captures in the list, newest first, for the chart `md5`.
    pub(crate) fn entries(&self, kept: &CapturedLayouts, md5: &str) -> Vec<CapturedLayout> {
        match self.scope {
            Scope::Chart => kept.of_chart(md5),
            Scope::All => kept.all().to_vec(),
        }
    }

    /// The capture under the cursor, kept within the list.
    pub(crate) fn selected(&self, kept: &CapturedLayouts, md5: &str) -> Option<CapturedLayout> {
        let entries = self.entries(kept, md5);
        let at = self.cursor.min(entries.len().saturating_sub(1));
        entries.get(at).cloned()
    }

    /// Where the cursor is, kept within a list of `len`.
    pub(crate) fn cursor(&self, len: usize) -> usize {
        self.cursor.min(len.saturating_sub(1))
    }

    /// Move the cursor `by` rows, stopping at either end of a list of `len`.
    pub(crate) fn step(&mut self, by: i64, len: usize) {
        let at = i64::try_from(self.cursor(len)).unwrap_or(i64::MAX).saturating_add(by).max(0);
        self.cursor = usize::try_from(at).unwrap_or(usize::MAX).min(len.saturating_sub(1));
    }

    /// Turn to the other scope, the cursor kept on the capture it was on when the other list has it.
    pub(crate) fn turn_scope(&mut self, kept: &CapturedLayouts, md5: &str) {
        let on = self.selected(kept, md5);
        self.scope = match self.scope {
            Scope::Chart => Scope::All,
            Scope::All => Scope::Chart,
        };
        let entries = self.entries(kept, md5);
        self.cursor = on.and_then(|on| entries.iter().position(|entry| entry.same_layout(&on))).unwrap_or_default();
    }

    /// Put the cursor on `capture` in the list for the chart `md5`, where it is in it.
    pub(crate) fn point_at(&mut self, kept: &CapturedLayouts, md5: &str, capture: &CapturedLayout) {
        let entries = self.entries(kept, md5);
        self.cursor = entries.iter().position(|entry| entry.same_layout(capture)).unwrap_or_default();
    }

    /// Take a press of the removal key on `selected`: the first press arms it and answers `None`,
    /// a second on the same capture answers it to be removed.
    pub(crate) fn press_remove(&mut self, selected: CapturedLayout) -> Option<CapturedLayout> {
        match self.armed.take() {
            Some(armed) if armed.same_layout(&selected) => Some(selected),
            _ => {
                self.armed = Some(selected);
                None
            }
        }
    }

    /// Let go of a removal waiting for its second press.
    pub(crate) fn disarm(&mut self) {
        self.armed = None;
    }

    /// Whether a removal waits for its second press.
    #[cfg(test)]
    pub(crate) fn is_armed(&self) -> bool {
        self.armed.is_some()
    }

    /// Draw the list: its heading and how many it holds, then each capture in view on two lines --
    /// its arrangement and lane order, and under it what the run came to and when, or for a capture
    /// of another chart that chart's title -- the cursor's row on a band while the list has the keys.
    pub(crate) fn draw(&self, canvas: &mut Canvas<'_>, kept: &CapturedLayouts, view: &ListView<'_>) {
        let th = theme();
        let entries = self.entries(kept, view.md5);
        let width = CW as f32 - LIST_RIGHT_INSET - LIST_X;
        let heading = match self.scope {
            Scope::Chart => LayoutPickText::CapturedOfChart,
            Scope::All => LayoutPickText::CapturedOfAll,
        };
        let heading_colour = if view.focused { th.accent } else { th.text_dim };
        draw_text(canvas, LIST_X, LIST_TOP, HEADING_SCALE, heading_colour, &fit_text(&heading.fill(view.language, &[&entries.len()]), HEADING_SCALE, width));
        let first_row = LIST_TOP + FIRST_ROW_DROP;
        if entries.is_empty() {
            for (row, line) in wrapped(&view.nothing_captured(), DETAIL_SCALE, width, NOTHING_CAPTURED_LINES).iter().enumerate() {
                draw_text(canvas, LIST_X, first_row + row as f32 * DETAIL_DROP, DETAIL_SCALE, th.text_muted, line);
            }
            return;
        }
        let cursor = self.cursor(entries.len());
        let first = cursor.saturating_sub(VISIBLE_ROWS - 1);
        for (row, (at, entry)) in entries.iter().enumerate().skip(first).take(VISIBLE_ROWS).enumerate() {
            let y = first_row + row as f32 * ROW_HEIGHT;
            let on = at == cursor && view.focused;
            if on {
                canvas.fill_rect(Rect::new(LIST_X - BAND_PAD, y - BAND_PAD, width + BAND_PAD * 2.0, ROW_HEIGHT - BAND_PAD), th.panel_hi);
            }
            let colour = if on { th.accent } else { th.text };
            let layout = format!("{} {}", entry.random_label, entry.layout_text());
            draw_text(canvas, LIST_X, y, ROW_SCALE, colour, &fit_text(&layout, ROW_SCALE, width));
            let detail = match entry.md5.eq_ignore_ascii_case(view.md5) {
                true => format!("EX {}  {}  {}", entry.ex_score, entry.lamp, fmt_datetime(entry.captured_at)),
                false => entry.title.clone(),
            };
            draw_text(canvas, LIST_X, y + DETAIL_DROP, DETAIL_SCALE, th.text_dim, &fit_text(&detail, DETAIL_SCALE, width));
        }
        let elsewhere = entries.get(cursor).filter(|entry| view.focused && !entry.md5.eq_ignore_ascii_case(view.md5));
        if elsewhere.is_some() {
            let shown = entries.len().min(VISIBLE_ROWS) as f32;
            draw_text(canvas, LIST_X, first_row + shown * ROW_HEIGHT, DETAIL_SCALE, th.accent, LayoutPickText::CapturedElsewhere.get(view.language));
        }
    }
}
