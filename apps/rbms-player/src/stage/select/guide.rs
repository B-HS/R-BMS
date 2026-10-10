//! The key guide of a browser a skin draws.
//!
//! A skin owns the whole screen, so the built-in browser's row of buttons and its guide line are not
//! there, and the keys that open the managers, the search, the filter and the rest have nothing on
//! screen to name them. The guide is the application's own panel over the skin that does: one key
//! ([`GUIDE_KEY`]) puts it up and the same key, Escape or Enter takes it down, and while it is up it
//! takes every key and every press of the mouse.
//!
//! What it lists is read rather than written out. The browser's own keys are the table below, each
//! entry holding the keys themselves so the text drawn is the key that is bound; START, SELECT and
//! the keys of the reference's key table are read from the key configuration and from that table
//! ([`KeyLayout::indices_of`]), so a key that was rebound is the key the guide names.

use rbms_play::ScratchDir;
use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_right, fit_text, theme};
use winit::keyboard::KeyCode;

use super::SelectState;
use super::keys::{KeyLayout, SelectKey};
use super::skinned::DETAIL_PANEL_KEY;
use crate::app_options::{HOLD_KEY, TOGGLE_KEY};
use crate::keyconfig::{ControlAction, key_index_of, key_name};
use crate::{AppShared, CH, CW, Hot};

/// The key that puts the guide up and takes it down.
pub(super) const GUIDE_KEY: KeyCode = KeyCode::KeyH;

/// What a key that has no name in the key configuration's own vocabulary is called on the guide.
const UNBOUND: &str = "-";

/// One of the browser's own keys, as the guide lists it: the keys that are named, and what they do.
pub(super) struct Shortcut {
    pub(super) keys: &'static [KeyCode],
    pub(super) does: &'static str,
}

/// The browser's own keys, in the order the guide lists them. A key a skin's browser takes for
/// something else -- the number keys, which are the reference's -- is not here.
pub(super) const BROWSER_SHORTCUTS: [Shortcut; 15] = [
    Shortcut { keys: &[KeyCode::ArrowUp, KeyCode::ArrowDown], does: "MOVE THE LIST" },
    Shortcut { keys: &[KeyCode::Enter], does: "OPEN A FOLDER OR PLAY THE CHART" },
    Shortcut { keys: &[KeyCode::Escape], does: "BACK ONE FOLDER - TWICE AT THE TOP QUITS" },
    Shortcut { keys: &[KeyCode::KeyO], does: "MUSIC FOLDERS" },
    Shortcut { keys: &[KeyCode::KeyT], does: "DIFFICULTY TABLES" },
    Shortcut { keys: &[KeyCode::KeyR], does: "RECORDS OF THE CHART" },
    Shortcut { keys: &[KeyCode::Slash], does: "SEARCH" },
    Shortcut { keys: &[KeyCode::F2], does: "FILTER" },
    Shortcut { keys: &[KeyCode::F3], does: "SORT ORDER - WITH SHIFT BACKWARDS" },
    Shortcut { keys: &[KeyCode::F4], does: "PRACTICE" },
    Shortcut { keys: &[KeyCode::Tab], does: "SETTINGS - WITH SHIFT THE COURSE LIST" },
    Shortcut { keys: &[KeyCode::KeyI], does: "IR RANKING" },
    Shortcut { keys: &[KeyCode::KeyF], does: "FAVOURITE" },
    Shortcut { keys: &[TOGGLE_KEY, HOLD_KEY], does: "OPTIONS OVERLAY" },
    Shortcut { keys: &[GUIDE_KEY], does: "THIS GUIDE" },
];

/// What each role of the reference's key table is called on the guide, in the order they are listed.
const KEY_ROLES: [(SelectKey, &str); 7] = [
    (SelectKey::Play, "PLAY THE CHART"),
    (SelectKey::Practice, "PRACTICE"),
    (SelectKey::Auto, "PLAY IT BY ITSELF"),
    (SelectKey::FolderOpen, "OPEN A FOLDER"),
    (SelectKey::FolderClose, "BACK ONE FOLDER"),
    (SelectKey::Up, "NEXT BAR"),
    (SelectKey::Down, "PREVIOUS BAR"),
];

/// The panel's width and the room it keeps round what it lists.
const GUIDE_W: f32 = 880.0;
const PAD_X: f32 = 28.0;
const TITLE_Y: f32 = 16.0;
const FIRST_SECTION_Y: f32 = 64.0;
const SECTION_H: f32 = 32.0;
const ROW_PITCH: f32 = 26.0;
const BOTTOM_PAD: f32 = 44.0;
const COLUMN_GAP: f32 = 32.0;
const KEY_COLUMN_W: f32 = 132.0;
const KEY_COLUMN_GAP: f32 = 8.0;

/// The rule under a column's heading and the one along the top of the panel: how far above the first
/// line the first stands, and how thick either is.
const HEADING_RULE_RISE: f32 = 8.0;
const RULE_THICKNESS: f32 = 2.0;

/// How far the hint at the right of the title is let down, so it sits on the title's baseline.
const HINT_DROP: f32 = 8.0;

/// The text sizes of the panel.
const TITLE_SCALE: f32 = 2.0;
const SECTION_SCALE: f32 = 1.3;
const ROW_SCALE: f32 = 1.2;
const HINT_SCALE: f32 = 1.0;

/// How opaque what is behind the panel is dimmed to.
const BACKDROP_ALPHA: u8 = 150;

const GUIDE_TITLE: &str = "R-BMS KEYS";
const BROWSER_SECTION: &str = "BROWSER";
const PANELS_SECTION: &str = "OPTION PANELS";
const KEYS_SECTION: &str = "KEYS 1-7 AND THE TURNTABLE";

/// What a key is called on the guide: the name the key configuration gives it where it has one, and
/// the name printed on the key for the ones it has not.
pub(super) fn shortcut_label(code: KeyCode) -> &'static str {
    match code {
        KeyCode::F1 => "F1",
        KeyCode::F2 => "F2",
        KeyCode::F3 => "F3",
        KeyCode::F4 => "F4",
        KeyCode::Tab => "TAB",
        KeyCode::Slash => "/",
        KeyCode::Enter => "ENTER",
        KeyCode::Escape => "ESC",
        other => key_name(other),
    }
}

/// The keys of one entry, as one string.
fn keys_label(keys: &[KeyCode]) -> String {
    keys.iter().map(|code| shortcut_label(*code)).collect::<Vec<_>>().join(" ")
}

/// One line of the guide: the keys, and what they do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuideRow {
    pub(super) keys: String,
    pub(super) does: String,
}

impl GuideRow {
    fn new(keys: impl Into<String>, does: impl Into<String>) -> GuideRow {
        GuideRow { keys: keys.into(), does: does.into() }
    }
}

/// The key of the keyboard bound to the key index `index` of the reference's table in the mode that
/// is up, or [`UNBOUND`] for an index no key of the keyboard is on.
fn index_label(shared: &AppShared, index: usize) -> &'static str {
    let forward = shared.active_keys.iter().find(|(_, lane)| key_index_of(shared.mode, *lane, ScratchDir::Forward) == Some(index)).map(|(code, _)| *code);
    let backward =
        shared.active_reverse_keys.iter().find(|(_, lane)| key_index_of(shared.mode, *lane, ScratchDir::Backward) == Some(index)).map(|(code, _)| *code);
    forward.or(backward).map_or(UNBOUND, shortcut_label)
}

/// The key a control is read from right now, or [`UNBOUND`].
fn control_label(shared: &AppShared, action: ControlAction) -> &'static str {
    shared.control_key_in_force(action).map_or(UNBOUND, shortcut_label)
}

/// The lines of the browser's own keys.
pub(super) fn browser_rows() -> Vec<GuideRow> {
    BROWSER_SHORTCUTS.iter().map(|shortcut| GuideRow::new(keys_label(shortcut.keys), shortcut.does)).collect()
}

/// The lines of the three option panels: the keys held for each, as they are bound now.
pub(super) fn panel_rows(shared: &AppShared) -> Vec<GuideRow> {
    let start = control_label(shared, ControlAction::Start);
    let select = control_label(shared, ControlAction::Select);
    vec![
        GuideRow::new(start, "HOLD FOR THE PLAY OPTIONS"),
        GuideRow::new(select, "HOLD FOR THE ASSIST OPTIONS"),
        GuideRow::new(format!("{start} + {select} / {}", shortcut_label(DETAIL_PANEL_KEY)), "HOLD FOR THE DETAIL OPTIONS"),
    ]
}

/// The lines of the keys of the reference's table: for each role, the keyboard keys that are on the
/// indices that carry it in the mode that is up.
pub(super) fn key_rows(shared: &AppShared) -> Vec<GuideRow> {
    let layout = KeyLayout::of(shared.mode);
    KEY_ROLES
        .iter()
        .map(|(role, does)| {
            let keys = layout.indices_of(*role).into_iter().map(|index| index_label(shared, index)).collect::<Vec<_>>().join(" ");
            GuideRow::new(keys, *does)
        })
        .collect()
}

/// How far the right column reaches below the first heading: the option panels, a gap, and the keys
/// under a heading of their own.
fn right_column_extent(options: usize, keys: usize) -> f32 {
    options as f32 * ROW_PITCH + ROW_PITCH * 0.5 + SECTION_H + keys as f32 * ROW_PITCH
}

/// Where the panel is: centred on the screen the application lays itself out for, and tall enough
/// for the longer of its two columns.
fn guide_rect(browser_rows: usize, options: usize, keys: usize) -> Rect {
    let left = browser_rows as f32 * ROW_PITCH;
    let height = FIRST_SECTION_Y + SECTION_H + left.max(right_column_extent(options, keys)) + BOTTOM_PAD;
    Rect::new((CW as f32 - GUIDE_W) * 0.5, (CH as f32 - height) * 0.5, GUIDE_W, height)
}

/// Draws one column: its heading, then a line for each row, the keys in a column of their own.
fn draw_column<R: Renderer>(r: &mut R, x: f32, y: f32, width: f32, heading: &str, rows: &[GuideRow]) {
    let th = theme();
    draw_text(r, x, y, SECTION_SCALE, th.accent, heading);
    r.fill_rect(Rect::new(x, y + SECTION_H - HEADING_RULE_RISE, width, RULE_THICKNESS), th.divider);
    let does_width = width - KEY_COLUMN_W;
    for (index, row) in rows.iter().enumerate() {
        let row_y = y + SECTION_H + index as f32 * ROW_PITCH;
        draw_text(r, x, row_y, ROW_SCALE, th.focus, &fit_text(&row.keys, ROW_SCALE, KEY_COLUMN_W - KEY_COLUMN_GAP));
        draw_text(r, x + KEY_COLUMN_W, row_y, ROW_SCALE, th.text, &fit_text(&row.does, ROW_SCALE, does_width));
    }
}

/// Draw the guide over the screen, and answer what a press on it hits: everything on the screen is
/// the guide's to close it, and the panel itself takes a press without closing it.
pub(super) fn render_guide<R: Renderer>(r: &mut R, shared: &AppShared) -> Vec<(Rect, Hot)> {
    let th = theme();
    let browser = browser_rows();
    let options = panel_rows(shared);
    let keys = key_rows(shared);
    let screen = Rect::new(0.0, 0.0, CW as f32, CH as f32);
    let panel = guide_rect(browser.len(), options.len(), keys.len());
    r.fill_rect(screen, Color { r: 0, g: 0, b: 0, a: BACKDROP_ALPHA });
    r.fill_rect(panel, th.panel_hi);
    r.fill_rect(Rect::new(panel.x, panel.y, panel.w, RULE_THICKNESS), th.focus);
    draw_text(r, panel.x + PAD_X, panel.y + TITLE_Y, TITLE_SCALE, th.text, GUIDE_TITLE);
    let hint = format!("{} OR ESC CLOSES", shortcut_label(GUIDE_KEY));
    draw_text_right(r, panel.x + panel.w - PAD_X, panel.y + TITLE_Y + HINT_DROP, HINT_SCALE, th.text_muted, &hint);

    let column_w = (panel.w - PAD_X * 2.0 - COLUMN_GAP) * 0.5;
    let top = panel.y + FIRST_SECTION_Y;
    draw_column(r, panel.x + PAD_X, top, column_w, BROWSER_SECTION, &browser);
    let right_x = panel.x + PAD_X + column_w + COLUMN_GAP;
    draw_column(r, right_x, top, column_w, PANELS_SECTION, &options);
    let keys_top = top + SECTION_H + options.len() as f32 * ROW_PITCH + ROW_PITCH * 0.5;
    draw_column(r, right_x, keys_top, column_w, KEYS_SECTION, &keys);
    vec![(screen, Hot::GuideClose), (panel, Hot::OverlayPanel)]
}

impl SelectState {
    /// One key while the guide is up: it closes on its own key, Escape and Enter, and takes every
    /// other key without doing anything with it, so the list under it cannot move.
    pub(super) fn guide_key(&mut self, code: KeyCode) {
        if matches!(code, KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter) || code == GUIDE_KEY {
            self.guide_open = false;
        }
    }
}
