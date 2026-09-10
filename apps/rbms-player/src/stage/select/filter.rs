//! The browser's filter panel: the axes it edits, the keys that move them, and the overlay it is
//! drawn as.
//!
//! The panel is a view onto one [`SelectFilter`]; the list is rebuilt through it, so what is on
//! screen and what the rows were chosen by are the same value. Only the favourites axis is
//! remembered between runs — it has a settings row of its own — so the level, mode and lamp axes
//! reset with the browser, which is what stops a forgotten filter from looking like a lost library.
#![allow(clippy::wildcard_imports)]

use rbms_render::{Rect, Renderer, draw_text, draw_text_right, theme};

use crate::stage::select::list::{CLEAR_FILTERS, ClearFilter, SelectFilter};
use crate::*;

/// Lowest level either bound can be set to.
const LEVEL_MIN: i32 = 1;

/// Highest level either bound can be set to. Insane tables label charts well past the twelve a
/// `#PLAYLEVEL` usually states, so the bound reaches far enough to pick one of those out.
const LEVEL_MAX: i32 = 25;

/// The value an unset level bound sits at, one step below the lowest real level.
const LEVEL_ANY: i32 = 0;

const PANEL_X: f32 = 32.0;
const PANEL_Y: f32 = 60.0;
const PANEL_W: f32 = 380.0;
const ROW_H: f32 = 30.0;
const ROW_PITCH: f32 = 32.0;
const ROWS_Y: f32 = 56.0;
const LABEL_X: f32 = 14.0;
const VALUE_RIGHT: f32 = 14.0;
const TITLE_SCALE: f32 = 1.6;
const HINT_SCALE: f32 = 1.0;
const ROW_SCALE: f32 = 1.3;
const TEXT_DROP: f32 = 6.0;

/// One editable axis of the filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilterRow {
    LevelFrom,
    LevelTo,
    Mode,
    Clear,
    Favorites,
}

/// The axes in the order the panel lists them.
const FILTER_ROWS: [FilterRow; 5] = [FilterRow::LevelFrom, FilterRow::LevelTo, FilterRow::Mode, FilterRow::Clear, FilterRow::Favorites];

impl FilterRow {
    fn label(self) -> &'static str {
        match self {
            FilterRow::LevelFrom => "LEVEL FROM",
            FilterRow::LevelTo => "LEVEL TO",
            FilterRow::Mode => "MODE",
            FilterRow::Clear => "CLEAR",
            FilterRow::Favorites => "FAVOURITES",
        }
    }

    /// What this axis is currently set to, as the panel shows it.
    fn value(self, filter: &SelectFilter) -> String {
        match self {
            FilterRow::LevelFrom => level_label(filter.level_from),
            FilterRow::LevelTo => level_label(filter.level_to),
            FilterRow::Mode => filter.mode.map_or_else(|| "ALL".to_string(), |mode| mode_short(mode).to_string()),
            FilterRow::Clear => filter.clear.label().to_string(),
            FilterRow::Favorites => if filter.favorite_only { "STARRED ONLY" } else { "ALL" }.to_string(),
        }
    }

    /// Whether this axis is filtering anything out, which is what highlights the row.
    fn is_set(self, filter: &SelectFilter) -> bool {
        match self {
            FilterRow::LevelFrom => filter.level_from.is_some(),
            FilterRow::LevelTo => filter.level_to.is_some(),
            FilterRow::Mode => filter.mode.is_some(),
            FilterRow::Clear => filter.clear != ClearFilter::Any,
            FilterRow::Favorites => filter.favorite_only,
        }
    }

    /// Step this axis by `delta`. The two level bounds carry each other so the range can never
    /// close on nothing: raising the lower bound past the upper one takes the upper one with it.
    fn adjust(self, filter: &mut SelectFilter, delta: i32) {
        match self {
            FilterRow::LevelFrom => {
                filter.level_from = step_level(filter.level_from, delta);
                if let (Some(from), Some(to)) = (filter.level_from, filter.level_to)
                    && from > to
                {
                    filter.level_to = Some(from);
                }
            }
            FilterRow::LevelTo => {
                filter.level_to = step_level(filter.level_to, delta);
                if let (Some(from), Some(to)) = (filter.level_from, filter.level_to)
                    && to < from
                {
                    filter.level_from = Some(to);
                }
            }
            FilterRow::Mode => filter.mode = step_mode(filter.mode, delta),
            FilterRow::Clear => filter.clear = step_in(&CLEAR_FILTERS, filter.clear, delta),
            FilterRow::Favorites => filter.favorite_only = !filter.favorite_only,
        }
    }
}

fn level_label(level: Option<i32>) -> String {
    level.map_or_else(|| "ANY".to_string(), |n| n.to_string())
}

/// Move a level bound one step, where the step below the lowest level is "unset".
fn step_level(level: Option<i32>, delta: i32) -> Option<i32> {
    let next = (level.unwrap_or(LEVEL_ANY) + delta).clamp(LEVEL_ANY, LEVEL_MAX);
    (next >= LEVEL_MIN).then_some(next)
}

/// Move the mode axis one step through "every mode" followed by each play mode.
fn step_mode(mode: Option<Mode>, delta: i32) -> Option<Mode> {
    let at = mode.and_then(|m| Mode::ALL.iter().position(|candidate| *candidate == m)).map_or(0, |i| i as i32 + 1);
    let next = (at + delta).clamp(0, Mode::ALL.len() as i32);
    (next > 0).then(|| Mode::ALL[next as usize - 1])
}

/// Move one step through a fixed list of values, wrapping at neither end so a held key settles.
fn step_in<T: Copy + PartialEq>(values: &[T], current: T, delta: i32) -> T {
    let at = values.iter().position(|candidate| *candidate == current).unwrap_or(0) as i32;
    values[(at + delta).clamp(0, values.len() as i32 - 1) as usize]
}

/// What the panel did with a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FilterKey {
    /// The panel is closed or does not use this key, so the browser gets it.
    Ignored,
    /// The panel used the key and the list is unchanged.
    Consumed,
    /// The panel used the key and the list has to be rebuilt through the new filter.
    Moved,
}

/// The filter panel: whether it is up, which axis has the cursor, and the axes it is editing.
#[derive(Debug, Default)]
pub(super) struct FilterPanel {
    open: bool,
    sel: usize,
    /// The axes this panel owns. The favourites switch is overwritten from the settings file every
    /// time the filter is read, so the settings row and this row can never disagree.
    filter: SelectFilter,
}

impl FilterPanel {
    pub(super) fn is_open(&self) -> bool {
        self.open
    }

    /// Show or hide the panel.
    pub(super) fn toggle(&mut self) {
        self.open = !self.open;
        self.sel = 0;
    }

    /// The filter the list is built through: this panel's axes, plus the favourites switch as the
    /// settings file has it.
    pub(super) fn filter(&self, config: &Config) -> SelectFilter {
        SelectFilter { favorite_only: config.library.favorite_only, ..self.filter }
    }

    /// What the filter is taking out, short enough for the browser's top bar, or `None` when it is
    /// taking nothing out. A filter with the panel closed is otherwise invisible, which is what
    /// makes a filtered library look like a lost one.
    pub(super) fn summary(&self, config: &Config) -> Option<String> {
        let filter = self.filter(config);
        if !filter.is_active() {
            return None;
        }
        let mut parts: Vec<String> = Vec::new();
        match (filter.level_from, filter.level_to) {
            (None, None) => {}
            (Some(from), None) => parts.push(format!("LV {from}+")),
            (None, Some(to)) => parts.push(format!("LV \u{2264}{to}")),
            (Some(from), Some(to)) if from == to => parts.push(format!("LV {from}")),
            (Some(from), Some(to)) => parts.push(format!("LV {from}\u{2013}{to}")),
        }
        if let Some(mode) = filter.mode {
            parts.push(mode_short(mode).to_string());
        }
        if filter.clear != ClearFilter::Any {
            parts.push(filter.clear.label().to_string());
        }
        if filter.favorite_only {
            parts.push("FAVOURITES".to_string());
        }
        Some(parts.join("  "))
    }

    /// One key while the panel is up. The panel takes every key it recognises — including Escape
    /// and Enter — so the list underneath cannot move and a chart cannot start behind it.
    pub(super) fn handle_key(&mut self, shared: &mut AppShared, code: KeyCode) -> FilterKey {
        if !self.open {
            return FilterKey::Ignored;
        }
        self.filter.favorite_only = shared.config.library.favorite_only;
        match code {
            KeyCode::F2 | KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter => {
                self.open = false;
                FilterKey::Consumed
            }
            KeyCode::ArrowUp => {
                self.sel = self.sel.saturating_sub(1);
                FilterKey::Consumed
            }
            KeyCode::ArrowDown => {
                self.sel = (self.sel + 1).min(FILTER_ROWS.len() - 1);
                FilterKey::Consumed
            }
            KeyCode::ArrowLeft => self.adjust(shared, -1),
            KeyCode::ArrowRight => self.adjust(shared, 1),
            KeyCode::Backspace => {
                let cleared = SelectFilter::default();
                if self.filter == cleared {
                    return FilterKey::Consumed;
                }
                self.filter = cleared;
                self.store_favorites(shared);
                FilterKey::Moved
            }
            _ => FilterKey::Consumed,
        }
    }

    /// Step the focused axis, reporting whether anything moved.
    fn adjust(&mut self, shared: &mut AppShared, delta: i32) -> FilterKey {
        let before = self.filter;
        FILTER_ROWS[self.sel.min(FILTER_ROWS.len() - 1)].adjust(&mut self.filter, delta);
        if self.filter == before {
            return FilterKey::Consumed;
        }
        self.store_favorites(shared);
        FilterKey::Moved
    }

    /// Write the favourites switch back to the settings file, which is the one axis that outlives
    /// the browser.
    fn store_favorites(&self, shared: &mut AppShared) {
        if shared.config.library.favorite_only == self.filter.favorite_only {
            return;
        }
        shared.config.library.favorite_only = self.filter.favorite_only;
        shared.save_settings();
    }
}

/// Draw the panel over the top of the row list.
///
/// It is keyboard-only: the browser's clickable regions name rows and buttons of the list itself,
/// so a panel row has nothing to report a click as.
pub(super) fn render_filter_panel<R: Renderer>(r: &mut R, panel: &FilterPanel, config: &Config) {
    let th = theme();
    let filter = panel.filter(config);
    let height = ROWS_Y + FILTER_ROWS.len() as f32 * ROW_PITCH + 8.0;
    r.fill_rect(Rect::new(PANEL_X, PANEL_Y, PANEL_W, height), th.panel_hi);
    r.fill_rect(Rect::new(PANEL_X, PANEL_Y, PANEL_W, 2.0), th.focus);
    draw_text(r, PANEL_X + LABEL_X, PANEL_Y + 12.0, TITLE_SCALE, th.accent, "FILTER");
    draw_text(r, PANEL_X + LABEL_X, PANEL_Y + 36.0, HINT_SCALE, th.text_muted, "\u{2191}\u{2193} AXIS   \u{2190}\u{2192} VALUE   BACKSPACE RESET   F2 CLOSE");
    for (index, row) in FILTER_ROWS.iter().enumerate() {
        let y = PANEL_Y + ROWS_Y + index as f32 * ROW_PITCH;
        let rect = Rect::new(PANEL_X + 4.0, y, PANEL_W - 8.0, ROW_H);
        if index == panel.sel {
            r.fill_rect(rect, th.row_focus);
        }
        let value_color = if row.is_set(&filter) { th.accent } else { th.text_dim };
        draw_text(r, PANEL_X + LABEL_X, y + TEXT_DROP, ROW_SCALE, th.text, row.label());
        draw_text_right(r, PANEL_X + PANEL_W - VALUE_RIGHT, y + TEXT_DROP, ROW_SCALE, value_color, &row.value(&filter));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> FilterPanel {
        FilterPanel::default()
    }

    #[test]
    fn a_fresh_panel_is_closed_and_filters_nothing() {
        let panel = panel();
        assert!(!panel.is_open());
        assert!(!panel.filter(&Config::default()).is_active());
    }

    #[test]
    fn every_axis_has_its_own_name() {
        let mut labels: Vec<&str> = FILTER_ROWS.iter().map(|row| row.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), FILTER_ROWS.len(), "two axes share a name");
    }

    /// The step below the lowest level is "no bound", so one key both sets and unsets a bound.
    #[test]
    fn a_level_bound_steps_between_unset_and_the_levels() {
        assert_eq!(step_level(None, 1), Some(LEVEL_MIN));
        assert_eq!(step_level(Some(LEVEL_MIN), -1), None);
        assert_eq!(step_level(None, -1), None, "stepping below unset is not an error");
        assert_eq!(step_level(Some(LEVEL_MAX), 1), Some(LEVEL_MAX), "the bound stops at the highest level");
    }

    /// A range that closed on nothing would empty the list with no way to see why, so the bounds
    /// push each other rather than crossing.
    #[test]
    fn the_level_bounds_carry_each_other_rather_than_crossing() {
        let mut filter = SelectFilter { level_from: Some(5), level_to: Some(6), ..SelectFilter::default() };
        for _ in 0..3 {
            FilterRow::LevelFrom.adjust(&mut filter, 1);
        }
        assert_eq!(filter.level_from, Some(8));
        assert_eq!(filter.level_to, Some(8), "raising the lower bound took the upper one with it");
        for _ in 0..4 {
            FilterRow::LevelTo.adjust(&mut filter, -1);
        }
        assert_eq!(filter.level_to, Some(4));
        assert_eq!(filter.level_from, Some(4), "lowering the upper bound took the lower one with it");
    }

    #[test]
    fn the_mode_axis_steps_through_every_mode_and_back_to_all() {
        let mut seen = vec![step_mode(None, -1)];
        let mut mode = None;
        for _ in 0..Mode::ALL.len() {
            mode = step_mode(mode, 1);
            seen.push(mode);
        }
        assert_eq!(seen[0], None, "stepping below ALL stays at ALL");
        assert_eq!(seen[1..].iter().flatten().copied().collect::<Vec<Mode>>(), Mode::ALL.to_vec());
        assert_eq!(step_mode(mode, 1), mode, "the axis stops at the last mode");
        assert_eq!(step_mode(Mode::ALL.first().copied(), -1), None, "stepping back off the first mode unsets the axis");
    }

    #[test]
    fn the_clear_axis_steps_through_every_lamp_filter() {
        let mut clear = ClearFilter::Any;
        let mut seen = vec![clear];
        for _ in 1..CLEAR_FILTERS.len() {
            clear = step_in(&CLEAR_FILTERS, clear, 1);
            seen.push(clear);
        }
        assert_eq!(seen, CLEAR_FILTERS.to_vec());
        assert_eq!(step_in(&CLEAR_FILTERS, clear, 1), clear, "the axis stops at the last filter");
        assert_eq!(step_in(&CLEAR_FILTERS, ClearFilter::Any, -1), ClearFilter::Any);
    }

    #[test]
    fn a_closed_panel_leaves_every_key_to_the_browser() {
        let mut panel = panel();
        let mut app = crate::stage::select::tests::app();
        for code in [KeyCode::ArrowDown, KeyCode::Escape, KeyCode::Enter, KeyCode::F2] {
            assert_eq!(panel.handle_key(&mut app.shared, code), FilterKey::Ignored, "{code:?}");
        }
    }

    /// An open panel has to take Escape and Enter, or closing it would also leave the folder and
    /// confirming an axis would start the chart behind it.
    #[test]
    fn an_open_panel_takes_the_keys_that_would_otherwise_leave_or_start_a_chart() {
        let mut app = crate::stage::select::tests::app();
        for code in [KeyCode::Escape, KeyCode::Enter, KeyCode::F2] {
            let mut panel = panel();
            panel.toggle();
            assert_eq!(panel.handle_key(&mut app.shared, code), FilterKey::Consumed, "{code:?}");
            assert!(!panel.is_open(), "{code:?} did not close the panel");
        }
    }

    #[test]
    fn moving_an_axis_reports_that_the_list_has_to_be_rebuilt() {
        let mut app = crate::stage::select::tests::app();
        let mut panel = panel();
        panel.toggle();
        assert_eq!(panel.handle_key(&mut app.shared, KeyCode::ArrowRight), FilterKey::Moved);
        assert_eq!(panel.filter(&app.shared.config).level_from, Some(LEVEL_MIN));
        assert_eq!(panel.handle_key(&mut app.shared, KeyCode::ArrowLeft), FilterKey::Moved);
        assert_eq!(panel.handle_key(&mut app.shared, KeyCode::ArrowLeft), FilterKey::Consumed, "an axis already at its end did not move");
    }

    #[test]
    fn the_cursor_stays_inside_the_axis_list() {
        let mut app = crate::stage::select::tests::app();
        let mut panel = panel();
        panel.toggle();
        for _ in 0..FILTER_ROWS.len() * 2 {
            panel.handle_key(&mut app.shared, KeyCode::ArrowDown);
        }
        assert_eq!(panel.sel, FILTER_ROWS.len() - 1);
        for _ in 0..FILTER_ROWS.len() * 2 {
            panel.handle_key(&mut app.shared, KeyCode::ArrowUp);
        }
        assert_eq!(panel.sel, 0);
    }

    /// The favourites axis is the one the settings file remembers, so moving it there has to write
    /// the file rather than only change the panel.
    #[test]
    fn the_favourites_axis_is_written_back_to_the_settings_file() {
        let mut app = crate::stage::select::tests::app();
        let mut panel = panel();
        panel.toggle();
        for _ in 0..FILTER_ROWS.len() {
            panel.handle_key(&mut app.shared, KeyCode::ArrowDown);
        }
        assert_eq!(panel.handle_key(&mut app.shared, KeyCode::ArrowRight), FilterKey::Moved);
        assert!(app.shared.config.library.favorite_only, "the settings file was not told");
        assert!(panel.filter(&app.shared.config).favorite_only);
    }

    /// The settings screen edits the same switch, so a change made there has to show in the panel
    /// rather than being overwritten by whatever the panel last held.
    #[test]
    fn the_favourites_axis_follows_the_settings_row() {
        let mut config = Config::default();
        config.library.favorite_only = true;
        assert!(panel().filter(&config).favorite_only);
    }

    #[test]
    fn resetting_clears_every_axis_at_once() {
        let mut app = crate::stage::select::tests::app();
        let mut panel = panel();
        panel.toggle();
        panel.handle_key(&mut app.shared, KeyCode::ArrowRight);
        panel.handle_key(&mut app.shared, KeyCode::ArrowDown);
        panel.handle_key(&mut app.shared, KeyCode::ArrowDown);
        panel.handle_key(&mut app.shared, KeyCode::ArrowRight);
        assert!(panel.filter(&app.shared.config).is_active());
        assert_eq!(panel.handle_key(&mut app.shared, KeyCode::Backspace), FilterKey::Moved);
        assert!(!panel.filter(&app.shared.config).is_active(), "reset left an axis set");
        assert_eq!(panel.handle_key(&mut app.shared, KeyCode::Backspace), FilterKey::Consumed, "resetting an unset filter rebuilds nothing");
    }
}
