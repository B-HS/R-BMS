//! The ranking mode: the IR ranking panel ([`crate::ir_ranking_view`]) opened, driven and closed
//! from the keys of a controller as well as from the keyboard.
//!
//! A ranking key held on a chart for as long as a skin's wheel takes for its long slide
//! ([`super::SCRATCH_REPEAT_DELAY`], `SCROLL_DURATION_LOW_MS`) opens the panel over the browser. Let
//! go sooner, the key does what it always did on the list, at the moment it is let go rather than
//! when it went down: the result screen's blue keys confirm on the release for the same reason. A
//! ranking key with no role on the list opens the panel the moment it goes down. On a bar that is
//! no chart, with no score server, with something over the list or an option panel called up, and
//! with the RANKING KEYS row off, a ranking key does at once what it always did.
//!
//! The ranking keys are the blue keys ([`crate::stage::result::blue_keys`]): keys two, four and six
//! of the seven-key modes, on either side of a double-play layout, and keys two and four of the
//! five-key ones. The nine-button layout's blue keys move the list, so its ranking keys are buttons
//! two and eight, which have no role on the list. Key six of the keyboard is the favourite shortcut's
//! as shipped, so the keyboard opens the panel with keys two and four only. A key the table in
//! force gives a role on the list other than going up a folder is no ranking key: the built-in
//! browser reads the nine buttons by the seven-key table, whose button eight moves the list. The
//! built-in browser reads the lanes of a controller and no lane of the keyboard, as it always has.
//!
//! While the panel is open the keys of the list drive it: the keys that move the list move the
//! selection, repeating as they do on the list, the keys that open the bar under the cursor play
//! the selected row's replay, a ranking key closes it, START asks for the page after the ones held
//! and SELECT goes to the local player's row. The wheel of the mouse moves the selection. Escape,
//! the ranking shortcut and the left arrow close it, as they always have.
//!
//! What is held when the panel opens or closes, or while something else has the keys, is held and
//! no press: a key that went down for an option panel does not open the panel once the option panel
//! has gone, and the key that opened the panel does not close it when it is let go.

use std::time::{Duration, Instant};

use rbms_model::Mode;

use super::keys::{KeyLayout, Panel, SelectKey};
use super::{SCRATCH_REPEAT_DELAY, SCRATCH_REPEAT_INTERVAL, SelectState};
use crate::ir_ranking::{RankingBoard, RankingState};
use crate::ir_ranking_view::PanelAction;
use crate::keyconfig::{KEY_INDEX_COUNT, Shortcut};
use crate::stage::result::blue_keys;
use crate::{AppShared, SelectItem, SelectTab};

/// How long the cursor rests on a chart before its ranking is fetched while the panel is closed, as
/// the reference fetches the ranking of the bar under the cursor (`rankingDuration`), so a skin that
/// draws the ranking itself has it to draw.
pub(super) const RANKING_AUTO_FETCH_AFTER: Duration = Duration::from_secs(5);

/// The ranking keys of the nine-button layout: buttons two and eight.
const POPN_RANKING_KEYS: &[usize] = &[1, 7];

/// The key of the seven-key modes whose keyboard key is the favourite shortcut's as shipped, which
/// is why the keyboard does not open the panel with it.
const KEYBOARD_FAVOURITE_KEY: usize = 5;

/// The roles on the list a ranking key may not carry: all of them but going up a folder.
const OTHER_LIST_ROLES: [SelectKey; 7] =
    [SelectKey::Play, SelectKey::Auto, SelectKey::Replay, SelectKey::Up, SelectKey::Down, SelectKey::FolderOpen, SelectKey::Practice];

/// The key indices that open and close the ranking mode in `mode`, before the table in force is
/// asked whether they are free for it ([`is_ranking_key`]).
pub(super) fn ranking_keys_of(mode: Mode) -> &'static [usize] {
    if mode == Mode::POPN_9K { POPN_RANKING_KEYS } else { blue_keys(mode) }
}

/// Whether the key `index` opens and closes the ranking mode in `mode`, read by `layout`.
pub(super) fn is_ranking_key(mode: Mode, layout: KeyLayout, index: usize) -> bool {
    ranking_keys_of(mode).contains(&index) && !OTHER_LIST_ROLES.iter().any(|role| layout.carries(index, *role))
}

/// Whether a key of the keyboard on the key `index` counts in `mode`: every one does but the
/// favourite shortcut's, where that key is a ranking key.
pub(super) fn keyboard_counts(mode: Mode, index: usize) -> bool {
    index != KEYBOARD_FAVOURITE_KEY || !ranking_keys_of(mode).contains(&index)
}

/// Whether the key `index` is down: on the keyboard or a controller when `keyboard` says the
/// keyboard counts, on a controller alone otherwise.
fn key_held(shared: &AppShared, index: usize, keyboard: bool) -> bool {
    shared.key_index_pressed(index) && (keyboard || !shared.keyboard_index_pressed(index))
}

/// What the list's move of the key `index` is in the panel, if it has one: the key for the next bar
/// selects the row below and the key for the one before the row above.
fn panel_move(layout: KeyLayout, index: usize) -> Option<PanelAction> {
    if layout.carries(index, SelectKey::Up) {
        Some(PanelAction::Down)
    } else if layout.carries(index, SelectKey::Down) {
        Some(PanelAction::Up)
    } else {
        None
    }
}

/// Where the local player's line is among the lines of `board` ([`crate::ir_ranking_view::panel_lines`]):
/// their row of the leaderboard when the board holds it, and otherwise the line that names them
/// after the leaderboard, which follows the line saying there are no scores when there are none.
pub(super) fn your_line(board: &RankingBoard) -> usize {
    board.rows.iter().position(|row| row.is_you).unwrap_or_else(|| board.rows.len() + usize::from(board.rows.is_empty()))
}

/// What the ranking keys come to on one frame ([`RankingKeys::act`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RankingKeyAct {
    Nothing,
    /// The key went down and waits to be held long enough or let go: what it does on the list is
    /// held back.
    Wait(usize),
    /// The key that waited was let go before the mode opened: it does now what it does on the list.
    Let(usize),
    Open,
    Close,
}

/// What the browser hands the ranking keys on one frame.
#[derive(Debug, Clone, Copy)]
pub(super) struct RankingKeyFrame {
    /// Whether nothing else has the keys: no option panel, nothing over the list, no word typed.
    pub(super) free: bool,
    /// Whether the mode may be opened on the bar under the cursor.
    pub(super) can_open: bool,
    /// Whether the mode is open.
    pub(super) open: bool,
    /// The ranking keys, none while the RANKING KEYS row is off.
    pub(super) ranking: [bool; KEY_INDEX_COUNT],
    /// The keys that do something on the list, so wait to be let go before they do it.
    pub(super) waits: [bool; KEY_INDEX_COUNT],
}

/// What the browser does on one frame of the ranking keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RankingFrame {
    Unchanged,
    /// The mode was opened or closed: the list reads no key this frame.
    Toggled,
    /// The key `index` was let go before the mode opened: the list does now what the key does on it.
    Let(usize),
}

/// A key of the list held in the open mode, which moves the selection again once it is due.
#[derive(Debug, Clone, Copy)]
struct Moving {
    index: usize,
    action: PanelAction,
    next_at: Instant,
}

/// The nine keys as the ranking mode reads them, which ranking key waits, and the buttons and the
/// wheel of the open mode.
#[derive(Debug, Default, Clone)]
pub(super) struct RankingKeys {
    down: [bool; KEY_INDEX_COUNT],
    /// The keys that went down since the last reading, whether or not they are still down.
    tapped: [bool; KEY_INDEX_COUNT],
    /// The keys that are down and have already done what they do: opened the mode, closed it, or
    /// gone down while something else had the keys.
    spent: [bool; KEY_INDEX_COUNT],
    /// The ranking key that waits, and when it went down.
    waiting: Option<(usize, Instant)>,
    moving: Option<Moving>,
    start_down: bool,
    select_down: bool,
    /// How far the mouse wheel has turned in the open mode since it last moved the selection.
    wheel: f32,
}

impl RankingKeys {
    /// Notes that the key `index` went down, for the reading that follows.
    pub(super) fn note_press(&mut self, index: usize) {
        if let Some(tapped) = self.tapped.get_mut(index) {
            *tapped = true;
        }
    }

    /// Reads the keys for one frame, `held` saying which are down, and answers which went down since
    /// the last reading. A key that went down and came up between two readings is down for this one.
    pub(super) fn read(&mut self, held: [bool; KEY_INDEX_COUNT]) -> [bool; KEY_INDEX_COUNT] {
        std::array::from_fn(|index| {
            let tapped = std::mem::take(&mut self.tapped[index]);
            let down = tapped || held[index];
            let fresh = down && (tapped || !self.down[index]);
            self.down[index] = down;
            fresh
        })
    }

    /// Reads START and SELECT for one frame and answers which of them went down since the last.
    pub(super) fn buttons(&mut self, start: bool, select: bool) -> (bool, bool) {
        let fresh = (start && !self.start_down, select && !self.select_down);
        self.start_down = start;
        self.select_down = select;
        fresh
    }

    /// What the ranking keys come to on a frame whose fresh presses are `fresh`.
    pub(super) fn act(&mut self, fresh: [bool; KEY_INDEX_COUNT], frame: &RankingKeyFrame, now: Instant) -> RankingKeyAct {
        for index in 0..KEY_INDEX_COUNT {
            self.spent[index] &= self.down[index];
        }
        if !frame.free {
            self.waiting = None;
            self.spent = self.down;
            return RankingKeyAct::Nothing;
        }
        let pressed = (0..KEY_INDEX_COUNT).find(|index| fresh[*index] && frame.ranking[*index] && !self.spent[*index]);
        if frame.open {
            self.waiting = None;
            let Some(index) = pressed else {
                return RankingKeyAct::Nothing;
            };
            self.spent[index] = true;
            return RankingKeyAct::Close;
        }
        if let Some((index, since)) = self.waiting {
            if !self.down[index] {
                self.waiting = None;
                return RankingKeyAct::Let(index);
            }
            if frame.can_open && now.saturating_duration_since(since) >= SCRATCH_REPEAT_DELAY {
                self.waiting = None;
                self.spent[index] = true;
                return RankingKeyAct::Open;
            }
            return RankingKeyAct::Nothing;
        }
        let Some(index) = pressed.filter(|_| frame.can_open) else {
            return RankingKeyAct::Nothing;
        };
        if frame.waits[index] {
            self.waiting = Some((index, now));
            return RankingKeyAct::Wait(index);
        }
        self.spent[index] = true;
        RankingKeyAct::Open
    }

    /// The move of a key of the list held in the open mode, once it is due again: a key let go stops
    /// moving the selection.
    fn repeat(&mut self, now: Instant) -> Option<PanelAction> {
        let moving = self.moving?;
        if !self.down[moving.index] {
            self.moving = None;
            return None;
        }
        if now < moving.next_at {
            return None;
        }
        self.moving = Some(Moving { next_at: now + SCRATCH_REPEAT_INTERVAL, ..moving });
        Some(moving.action)
    }

    /// The whole notches the wheel has turned in the open mode with `lines` added, towards the row
    /// below when positive. What is left of a notch waits for the rest of it.
    fn take_wheel(&mut self, lines: f32) -> i32 {
        self.wheel += lines;
        let notches = self.wheel.trunc();
        self.wheel -= notches;
        notches as i32
    }

    /// Forget what only the open mode keeps: the move of a held key and the wheel.
    pub(super) fn leave_the_mode(&mut self) {
        self.moving = None;
        self.wheel = 0.0;
    }
}

impl SelectState {
    /// Whether the ranking mode may be opened from the keys on the bar under the cursor: a chart of
    /// the song list, with a score server to ask about it.
    pub(super) fn ranking_can_open(&self, shared: &AppShared) -> bool {
        self.tab == SelectTab::Songs && matches!(shared.select_items.get(shared.sel), Some(SelectItem::Song(_))) && shared.has_primary_ir_server()
    }

    /// Whether something the ranking panel does not own is over the browser and takes its keys.
    pub(super) fn over_the_ranking(&self, shared: &AppShared) -> bool {
        self.record_modal.is_some()
            || self.download_modal.is_some()
            || self.filter.is_open()
            || self.guide_open
            || shared.options.is_open()
            || shared.skin_text_is_focused()
    }

    /// Whether the ranking keys may read the keys this frame: nothing else is over the browser, no
    /// word is being typed, and either the mode is open, whose keys START and SELECT are, or no
    /// option panel is called up.
    pub(super) fn ranking_keys_are_free(&self, shared: &AppShared) -> bool {
        let panel = Panel::held(shared.start_pressed(), shared.select_pressed(), shared.shortcut_held(Shortcut::RefDetail));
        !self.over_the_ranking(shared) && !shared.searching && (self.ranking_open || panel == Panel::Closed)
    }

    /// Note that the key `index` went down between two frames, from the keyboard or from a controller.
    pub(super) fn note_ranking_press(&mut self, mode: Mode, index: usize, from_keyboard: bool) {
        if !from_keyboard || keyboard_counts(mode, index) {
            self.ranking_keys.note_press(index);
        }
    }

    /// Whether a press of the key `index` on the built-in browser waits to be let go rather than
    /// going up a folder at once: it is a ranking key on a chart that the mode may open on.
    pub(super) fn ranking_key_waits(&self, shared: &AppShared, layout: KeyLayout, index: usize) -> bool {
        shared.config.library.ranking_keys
            && is_ranking_key(shared.mode, layout, index)
            && layout.carries(index, SelectKey::FolderClose)
            && self.ranking_can_open(shared)
    }

    /// One frame of the ranking keys, read by `layout`, the lanes of the keyboard counting when
    /// `keyboard` says so. `free` says whether the keys are the ranking mode's to read
    /// ([`SelectState::ranking_keys_are_free`]).
    ///
    /// A ranking key that waits takes its press away from the key table of a skin's browser, so the
    /// list does not go up a folder when it goes down; what it does on the list is the browser's to
    /// carry out when it is let go ([`RankingFrame::Let`]).
    pub(super) fn ranking_frame(&mut self, shared: &mut AppShared, layout: KeyLayout, keyboard: bool, free: bool, now: Instant) -> RankingFrame {
        let mode = shared.mode;
        let held = std::array::from_fn(|index| key_held(shared, index, keyboard && keyboard_counts(mode, index)));
        let fresh = self.ranking_keys.read(held);
        let buttons = self.ranking_keys.buttons(shared.start_pressed(), shared.select_pressed());
        let on = shared.config.library.ranking_keys;
        let frame = RankingKeyFrame {
            free,
            can_open: self.ranking_can_open(shared),
            open: self.ranking_open,
            ranking: std::array::from_fn(|index| on && is_ranking_key(mode, layout, index)),
            waits: std::array::from_fn(|index| layout.carries(index, SelectKey::FolderClose)),
        };
        match self.ranking_keys.act(fresh, &frame, now) {
            RankingKeyAct::Nothing => {}
            RankingKeyAct::Wait(index) => {
                self.keys.forget_change(index);
            }
            RankingKeyAct::Let(index) => return RankingFrame::Let(index),
            RankingKeyAct::Open | RankingKeyAct::Close => {
                self.toggle_ranking_panel(shared);
                return RankingFrame::Toggled;
            }
        }
        if self.ranking_open && free {
            self.ranking_mode_keys(shared, layout, fresh, buttons, now);
        }
        RankingFrame::Unchanged
    }

    /// The keys of the open mode on one frame: the keys that move the list move the selection and
    /// repeat while they are held, the keys that open a bar play the selected row's replay, START asks
    /// for the next page and SELECT goes to the local player's row.
    fn ranking_mode_keys(&mut self, shared: &mut AppShared, layout: KeyLayout, fresh: [bool; KEY_INDEX_COUNT], buttons: (bool, bool), now: Instant) {
        for index in (0..KEY_INDEX_COUNT).filter(|index| fresh[*index]) {
            if let Some(action) = panel_move(layout, index) {
                self.run_ranking_action(shared, action);
                self.ranking_keys.moving = Some(Moving { index, action, next_at: now + SCRATCH_REPEAT_DELAY });
            } else if layout.carries(index, SelectKey::FolderOpen) || layout.carries(index, SelectKey::Play) {
                self.run_ranking_action(shared, PanelAction::PlayReplay);
            }
        }
        if let Some(action) = self.ranking_keys.repeat(now) {
            self.run_ranking_action(shared, action);
        }
        let (start, select) = buttons;
        if start {
            self.run_ranking_action(shared, PanelAction::NextPage);
        }
        if select {
            self.run_ranking_action(shared, PanelAction::YourRow);
        }
    }

    /// The wheel of the mouse turned by `lines` over the open mode: a row a notch, down the panel for
    /// a turn towards the next bar.
    pub(super) fn ranking_wheel(&mut self, shared: &mut AppShared, lines: f32) {
        let notches = self.ranking_keys.take_wheel(lines);
        let action = if notches > 0 { PanelAction::Down } else { PanelAction::Up };
        for _ in 0..notches.unsigned_abs() {
            self.run_ranking_action(shared, action);
        }
    }

    /// Put the selection on the local player's line, and ask for the page their row is on when the
    /// board holds neither it nor the page it is on.
    pub(super) fn ranking_your_row(&mut self, shared: &mut AppShared) {
        let Some(chart) = shared.focused_chart_id() else {
            return;
        };
        let Some(RankingState::Ready(board)) = shared.ranking_cache.peek(&chart.md5) else {
            return;
        };
        let (line, page) = (your_line(board), board.page_to_find_you());
        self.ranking_sel = self.ranking_selectable_start(shared) + line;
        if let Some(page) = page {
            shared.start_ranking_page(chart, page);
        }
    }

    /// Ask for the page of the leaderboard after the ones the board of the chart under the cursor
    /// holds. Nothing is asked while another fetch is on its way or the server has no more.
    pub(super) fn ranking_next_page(&mut self, shared: &mut AppShared) {
        let Some(chart) = shared.focused_chart_id() else {
            return;
        };
        let Some(RankingState::Ready(board)) = shared.ranking_cache.peek(&chart.md5) else {
            return;
        };
        let next = board.pages + 1;
        shared.start_ranking_page(chart, next);
    }
}
