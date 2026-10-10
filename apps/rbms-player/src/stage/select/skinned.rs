//! The browser a skin draws: what has become of the skin, the frame of input the reference's
//! browser runs, and the timers a skin's browser is animated on.
//!
//! The reference reads its browser's keys once a frame, after it has drawn
//! (`MusicSelectInputProcessor.input`): the option panel the held keys call up, the mouse wheel and
//! the keys that move the list, then the keys that open, close and start what is under the cursor
//! ([`SelectState::skin_input_frame`]). The two arrows of the keyboard that move the cursor are
//! read the same way, as held keys beside the turntable, so they slide the wheel, repeat while
//! they are held and let a press that lands during a slide go by, as the reference's do. Its mouse comes after that: a press on a bar of the wheel
//! ([`SelectState::press_bar`]). Everything here does what one of the browser's own keys already
//! does by calling what that key calls, so the two ways in cannot drift apart.
//!
//! The timers are the ones the reference's browser switches and no others
//! ([`SelectState::run_scene_timers`]): the start of input, the change of the bar under the cursor,
//! and the state of the ranking being fetched. The reference takes input from the first frame of
//! the scene -- the input time in a skin's header only starts a timer here, unlike on the decide
//! and result scenes -- and it never fades the browser out.
//!
//! None of this runs for a browser no skin draws.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rbms_render::skin_render::SkinAction;
use rbms_render::skin_render::frame::BarHold;
use rbms_skin::timer::{TimerId, timer_id};

use super::SelectState;
use super::keys::{KeyLayout, Panel, SelectKey, SelectKeys};
use crate::ir_ranking::RankingState;
use crate::keyconfig::key_index_of;
use crate::skin_host::Cluster;
use crate::skin_host::writers::{BrowserWrite, browser_write};
use crate::stage::result::SkinStatus;
use crate::stage::scene_life::{SceneTimes, advance};
use crate::stage::{FrameCtx, StageHandler, Transition};
use crate::{AppShared, KeyCode, SKIN_TYPE_MUSIC_SELECT, SelectItem, SelectTab, SelectView, SystemSound};

/// The length of a scene that never ends on its own. The browser reads the `input` time of a
/// skin's header and neither its `scene` nor its `fadeout` (`MusicSelector.render`).
const NO_SCENE_END_MS: i64 = i64::MAX;

/// The key that calls up the detail panel by itself (`ControlKeys.NUM5`).
pub(super) const DETAIL_PANEL_KEY: KeyCode = KeyCode::Digit5;

/// The arrows of the keyboard that hold the list towards the next bar and towards the one before,
/// beside the keys of the key table (`ControlKeys.DOWN` with `UP`, `ControlKeys.UP` with `DOWN`).
const LIST_NEXT_KEY: KeyCode = KeyCode::ArrowDown;
const LIST_PREVIOUS_KEY: KeyCode = KeyCode::ArrowUp;

/// The key whose change the reference forgets whenever a folder is closed from the keys, whichever
/// key closed it (`input.resetKeyChangedTime(1)`).
const FOLDER_CLOSE_FORGOTTEN_KEY: usize = 1;

/// The setting steps of an event run forwards and backwards.
const STEP_NEXT: i32 = 1;
const STEP_PREVIOUS: i32 = -1;

/// The bar under the cursor as the bar timer remembers it: the tab, how often the list on that tab
/// has been rebuilt, and the row.
pub(super) type BarMark = (SelectTab, u64, usize);

/// Where the wall clock stood at one moment of the frame clock, which is what dates every frame
/// after it. The wheel slides on the wall clock (`System.currentTimeMillis`), and a frame is handed
/// the frame clock; reading one off the other keeps a frame's two clocks from disagreeing.
#[derive(Debug, Clone, Copy)]
pub(super) struct WallClock {
    at: Instant,
    unix_ms: i64,
}

impl WallClock {
    /// The two clocks as they stand.
    pub(super) fn now() -> WallClock {
        let unix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        WallClock { at: Instant::now(), unix_ms: i64::try_from(unix.as_millis()).unwrap_or(i64::MAX) }
    }

    /// The wall clock's millisecond at the frame clock's `at`.
    pub(super) fn millis_at(&self, at: Instant) -> i64 {
        let since = i64::try_from(at.saturating_duration_since(self.at).as_millis()).unwrap_or(i64::MAX);
        self.unix_ms.saturating_add(since)
    }
}

/// What has become of the skin the browser is drawn with.
///
/// A skin still on its way after the scene has waited its limit out for it counts as not coming:
/// the built-in browser draws until it arrives, if it ever does.
pub(super) fn skin_status(shared: &AppShared) -> SkinStatus {
    let screen = SKIN_TYPE_MUSIC_SELECT;
    if shared.skin_screens.get(screen).is_some() && !shared.skins.is_waiting(screen) {
        return SkinStatus::Ready;
    }
    let chosen = shared.skins.document_path(&shared.config, screen).is_some();
    let readable = shared.skins.document(screen).is_some() || shared.skins.is_waiting(screen) || shared.skins.needs_reload_for(&shared.config, screen);
    if chosen && readable && !shared.skin_wait_is_spent(screen) { SkinStatus::Reading } else { SkinStatus::Gone }
}

/// What kind of bar the cursor is on, as far as the keys care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FocusedBar {
    /// A chart or a course (`SelectableBar`).
    Playable,
    /// Anything that opens (`DirectoryBar`).
    Folder,
    /// The list is empty.
    Nothing,
}

/// What a key asked of the bar under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    Play,
    Practice,
    Auto,
    Replay,
    Open,
}

/// The roles that start a chart or a course, in the order the reference asks about them: the first
/// that is pressed is the one that counts, and the ones after it are not asked.
const PLAYABLE_PICKS: [(SelectKey, Pick); 4] =
    [(SelectKey::Play, Pick::Play), (SelectKey::Practice, Pick::Practice), (SelectKey::Auto, Pick::Auto), (SelectKey::Replay, Pick::Replay)];

/// How the ranking of the chart under the cursor stands (`RankingData.ACCESS`, `FINISH`, `FAIL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RankingPhase {
    Access,
    Finish,
    Fail,
}

/// The timer that is on for as long as the ranking stands in each phase.
const RANKING_TIMERS: [(TimerId, RankingPhase); 3] =
    [(timer_id::IR_CONNECT_BEGIN, RankingPhase::Access), (timer_id::IR_CONNECT_SUCCESS, RankingPhase::Finish), (timer_id::IR_CONNECT_FAIL, RankingPhase::Fail)];

impl SelectState {
    /// Whether the keys are something else's right now: a box of the browser's being typed into or
    /// read, the ranking panel, the option overlay, or an editable text of the skin.
    pub(super) fn keys_are_elsewhere(&self, ctx: &FrameCtx<'_>) -> bool {
        self.holds_keys(ctx) || self.ranking_open || ctx.shared.options.is_open() || ctx.shared.skin_text_is_focused()
    }

    /// Notes that the key `index` went down on a browser a skin draws, for the frame that follows.
    /// Like any key it cancels an armed quit.
    pub(super) fn note_key_index(&mut self, ctx: &FrameCtx<'_>, index: usize) {
        if self.keys_are_elsewhere(ctx) {
            return;
        }
        self.esc_quit_at = None;
        self.keys.note_press(index);
    }

    /// One key going down on a browser a skin draws, before the browser's own keys see it. Answers
    /// whether the key was taken.
    ///
    /// A key bound to a lane is noted for the key table. With no option panel called up it still
    /// goes on to whatever else it is, so the browser's own keys keep working on the list. With one
    /// called up the key is the panel's and goes no further: the panel covers the list, and a key
    /// that steps one of its options must not also star the chart or open a screen underneath it.
    /// The panel counts as called up from the moment its keys are held, which is a frame before it
    /// is drawn.
    ///
    /// The keys the reference's browser has and this one had no use for do what the reference's do,
    /// through what the browser already does for its own: `1` steps the mode filter, `2` the order
    /// (as `F3` does), `3` the long note mode, `4` moves the replay slot on, `0` opens the search
    /// (as `/` does) and `F9` stars the chart (as `F` does). The two arrows that move the cursor
    /// are noted for the frame that follows, which reads them with the turntable
    /// ([`SelectState::list_hold`]). Every other key is left to do what it always has.
    pub(super) fn skinned_key(&mut self, ctx: &mut FrameCtx<'_>, code: KeyCode) -> bool {
        let lane = ctx.shared.lane_input_for(code).and_then(|(lane, dir)| key_index_of(ctx.shared.mode, lane, dir));
        if let Some(index) = lane {
            self.note_key_index(ctx, index);
        }
        match code {
            KeyCode::Digit1 => self.mode_event(ctx.shared, true),
            KeyCode::Digit2 => self.sort_event(ctx.shared, true),
            KeyCode::Digit3 => self.ln_mode_event(ctx.shared, true),
            KeyCode::Digit4 => self.next_replay(ctx.shared),
            KeyCode::Digit0 => self.start_search(ctx.shared),
            KeyCode::F9 => ctx.shared.toggle_focused_favorite(),
            KeyCode::ArrowDown | KeyCode::ArrowUp if !self.panel_is_called_up(ctx.shared) => {
                self.arrow_tap = if code == LIST_NEXT_KEY { BarHold::Next } else { BarHold::Previous };
            }
            _ => return lane.is_some() && self.panel_is_called_up(ctx.shared),
        }
        true
    }

    /// Whether an option panel is up or its keys are held, which is what puts it up on the frame
    /// that follows.
    pub(super) fn panel_is_called_up(&self, shared: &AppShared) -> bool {
        self.panel.is_open() || Panel::held(shared.start_pressed(), shared.select_pressed(), shared.keyconfig.held.is_down(DETAIL_PANEL_KEY)) != Panel::Closed
    }

    /// Which way the list is being held this frame: by the keys of the key table that move it, or by
    /// an arrow of the keyboard, the down arrow with the key for the next bar and the up arrow with
    /// the key for the one before, the next winning over the one before (`BarRenderer.input`).
    /// `tapped` is an arrow that went down since the last frame, which counts as held for this one
    /// so that a press shorter than a frame still moves the list.
    fn list_hold(&mut self, shared: &AppShared, layout: KeyLayout, tapped: BarHold) -> BarHold {
        let keys = self.keys.hold(layout);
        let held = &shared.keyconfig.held;
        if keys == BarHold::Next || tapped == BarHold::Next || held.is_down(LIST_NEXT_KEY) {
            BarHold::Next
        } else if keys == BarHold::Previous || tapped == BarHold::Previous || held.is_down(LIST_PREVIOUS_KEY) {
            BarHold::Previous
        } else {
            BarHold::None
        }
    }

    /// Move the cursor `moved` bars, towards the next bar when it is positive, with the sound of
    /// each (`BarManager.move`).
    fn step_bars(&mut self, shared: &mut AppShared, moved: i32) {
        for _ in 0..moved.unsigned_abs() {
            self.step_cursor(shared, moved > 0);
            if self.tab == SelectTab::Courses {
                shared.play_system_sound(SystemSound::Scratch);
            }
        }
    }

    /// How many whole notches the wheel has turned since it was last asked, towards the next bar
    /// when positive. What is left of a notch waits for the rest of it.
    pub(super) fn take_wheel(&mut self) -> i32 {
        let notches = self.wheel_lines.trunc();
        self.wheel_lines -= notches;
        notches as i32
    }

    /// One frame of input on a browser a skin draws (`MusicSelectInputProcessor.input`, then
    /// `Skin.mousePressed`).
    ///
    /// On the first frame the browser listens again, and for as long as the keys are something
    /// else's, the keys are only kept track of, so that what is held then is not a press later; a
    /// panel is still put up for the keys that are held on that first frame, and taken down while
    /// the keys are something else's. With an option panel called up the list stands still and the
    /// panel takes the presses of its own keys ([`super::panel`]). Otherwise the wheel and the held
    /// keys move the list and the keys under the cursor are read. A press on a bar is carried out
    /// whichever of these the frame was.
    pub(super) fn skin_input_frame(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let now_ms = self.clock.millis_at(ctx.now);
        let tapped = std::mem::take(&mut self.arrow_tap);
        let presses = ctx.shared.take_skin_bar_presses(SKIN_TYPE_MUSIC_SELECT);
        let elsewhere = self.keys_are_elsewhere(ctx);
        let shared = &mut *ctx.shared;
        let held = Panel::held(shared.start_pressed(), shared.select_pressed(), shared.keyconfig.held.is_down(DETAIL_PANEL_KEY));
        let panel = if elsewhere { Panel::Closed } else { held };
        self.show_panel(shared, panel);
        if !std::mem::replace(&mut self.listening, true) || elsewhere {
            self.keys = SelectKeys::settled(shared);
            self.wheel_lines = 0.0;
            self.scroller.reset_input(now_ms);
            return self.press_bars(shared, presses);
        }
        let layout = KeyLayout::of(shared.mode);
        self.keys.read(shared);
        if panel != Panel::Closed {
            self.scroller.reset_input(now_ms);
            self.panel_keys(shared, layout, panel, now_ms);
            return self.press_bars(shared, presses);
        }
        if self.keys.pressed(layout, SelectKey::NextReplay, true) {
            self.next_replay(shared);
        }
        let wheel = self.take_wheel();
        let hold = self.list_hold(shared, layout, tapped);
        let moved = self.scroller.input(wheel, hold, now_ms);
        self.step_bars(shared, moved);
        match self.list_keys(shared, layout) {
            Transition::Stay => self.press_bars(shared, presses),
            asked => asked,
        }
    }

    /// The keys of the list (`MusicSelectInputProcessor.input`, the branch with no panel up).
    ///
    /// On a chart or a course the first of play, practice, autoplay and replay that is pressed is
    /// the one that counts; on a folder a press of any key that opens one opens it. A key that
    /// closes a folder is asked about after either. What starts play is carried out last, as the
    /// reference leaves it for its next frame, and only if the cursor is still on something that
    /// plays.
    ///
    /// Practice opens the practice screen on a chart and plays a course, which has none. Autoplay
    /// plays a chart by itself once ([`SelectState::start_autoplay`]). The key for a replay plays
    /// the replay in the slot that is selected, and plays the chart when none is, which is a chart
    /// with no replay saved (`getSelectedReplay() >= 0`).
    fn list_keys(&mut self, shared: &mut AppShared, layout: KeyLayout) -> Transition {
        let pick = match self.focused_bar(shared) {
            FocusedBar::Playable => PLAYABLE_PICKS.iter().find(|(role, _)| self.keys.pressed(layout, *role, true)).map(|(_, pick)| *pick),
            FocusedBar::Folder => self.keys.pressed(layout, SelectKey::FolderOpen, true).then_some(Pick::Open),
            FocusedBar::Nothing => None,
        };
        if pick == Some(Pick::Open) {
            self.select_enter(shared);
        }
        if self.keys.pressed(layout, SelectKey::FolderClose, true) {
            self.keys.forget_change(FOLDER_CLOSE_FORGOTTEN_KEY);
            self.close_bar(shared);
        }
        if self.focused_bar(shared) != FocusedBar::Playable {
            return Transition::Stay;
        }
        match pick {
            Some(Pick::Replay) => match self.selected_replay {
                Some(slot) => self.play_replay_slot(shared, slot),
                None => self.select_enter(shared),
            },
            Some(Pick::Play) => self.select_enter(shared),
            Some(Pick::Practice) if self.tab == SelectTab::Courses => self.course_enter(shared),
            Some(Pick::Practice) => self.start_practice(shared),
            Some(Pick::Auto) => self.start_autoplay(shared),
            Some(Pick::Open) | None => Transition::Stay,
        }
    }

    /// What kind of bar the cursor is on.
    pub(super) fn focused_bar(&self, shared: &AppShared) -> FocusedBar {
        match self.tab {
            SelectTab::Courses if self.courses.focused().is_some() => FocusedBar::Playable,
            SelectTab::Courses => FocusedBar::Nothing,
            SelectTab::Songs => match shared.select_items.get(shared.sel) {
                Some(SelectItem::Song(_)) => FocusedBar::Playable,
                Some(SelectItem::Folder { .. }) => FocusedBar::Folder,
                None => FocusedBar::Nothing,
            },
        }
    }

    /// Go up one folder, or at the top of the list move the order on instead (`BarManager.close`).
    /// A search is left the way its own key leaves it, and the course tab has nothing to close.
    fn close_bar(&mut self, shared: &mut AppShared) {
        if self.tab != SelectTab::Songs {
            return;
        }
        if shared.searching {
            self.exit_search(shared);
        } else if shared.select_view == SelectView::Root {
            self.sort_event(shared, true);
        } else {
            self.select_back(shared);
        }
    }

    /// Carry out the presses on the bars of the skin's wheel, oldest first, until one of them
    /// leaves the browser. With the record modal up a press closes it instead, as a click that
    /// misses it always has.
    fn press_bars(&mut self, shared: &mut AppShared, presses: Vec<SkinAction>) -> Transition {
        if self.record_modal.is_some() {
            if !presses.is_empty() {
                self.record_modal = None;
            }
            return Transition::Stay;
        }
        for press in presses {
            let asked = self.press_bar(shared, press);
            if !matches!(asked, Transition::Stay) {
                return asked;
            }
        }
        Transition::Stay
    }

    /// One press on a bar of the skin's wheel.
    ///
    /// The left button opens the bar it landed on when that bar opens, and starts play otherwise --
    /// of the bar under the cursor, whichever bar was pressed, and of nothing when the cursor is on
    /// a bar that opens (`MusicSelector.select`, then `MusicSelector.render`). Any other button
    /// closes the folder that is open (`BarManager.close`).
    fn press_bar(&mut self, shared: &mut AppShared, press: SkinAction) -> Transition {
        match press {
            SkinAction::SelectBar { bar, .. } => {
                let opens = self.tab == SelectTab::Songs && matches!(shared.select_items.get(bar), Some(SelectItem::Folder { .. }));
                if opens {
                    self.enter_row(shared, bar)
                } else if self.focused_bar(shared) == FocusedBar::Playable {
                    self.select_enter(shared)
                } else {
                    Transition::Stay
                }
            }
            SkinAction::CloseBar => {
                self.close_bar(shared);
                Transition::Stay
            }
            SkinAction::Event { .. } | SkinAction::Write { .. } | SkinAction::FocusText { .. } => Transition::Stay,
        }
    }

    /// Step the order the list is in (`EventType.sort`): the browser's own sort key, with the sound
    /// the reference gives an option that changed.
    pub(super) fn sort_event(&mut self, shared: &mut AppShared, forward: bool) {
        if forward {
            self.cycle_sort(shared);
        } else {
            self.cycle_sort_back(shared);
        }
        shared.play_system_sound(SystemSound::OptionChange);
    }

    /// Step the play mode the list is filtered by (`EventType.mode`), which is the mode row of the
    /// browser's own filter panel.
    pub(super) fn mode_event(&mut self, shared: &mut AppShared, forward: bool) {
        self.filter.cycle_mode(forward);
        shared.sel = 0;
        self.rebuild(shared);
        shared.play_system_sound(SystemSound::OptionChange);
    }

    /// Step the long note mode (`EventType.lnmode`), which is the settings screen's own row. The
    /// reference builds its list again for it, so the bar timer starts over.
    pub(super) fn ln_mode_event(&mut self, shared: &mut AppShared, forward: bool) {
        rbms_config::adjust(&mut shared.config, rbms_config::SettingId::LnMode, if forward { STEP_NEXT } else { STEP_PREVIOUS });
        shared.save_settings();
        self.bar_changed = true;
        shared.play_system_sound(SystemSound::OptionChange);
    }

    /// Whether the skin's scrollbar was dragged since the last frame. The reference starts the bar
    /// timer over on every such write, whether or not it moved the cursor
    /// (`FloatPropertyFactory`, `musicselect_position`). The write itself is left where it is, for
    /// whatever carries it out.
    pub(super) fn scrollbar_was_dragged(shared: &mut AppShared) -> bool {
        let mut dragged = false;
        shared.skin_requests().take_if(Cluster::Select, |request| {
            dragged |= matches!(browser_write(request), Some(BrowserWrite::Position(_)));
            false
        });
        dragged
    }

    /// Keep the timers of a browser a skin draws (`MusicSelector.render` and the end of
    /// `MusicSelectInputProcessor.input`).
    ///
    /// The start of input goes on once the scene has run for longer than the skin's input time. The
    /// bar timer starts over whenever the bar under the cursor is not the one it was a frame ago --
    /// the cursor moved, or the list was built again -- and is otherwise only switched on if it was
    /// off, which is what lights it on a scene's first frame. The three ranking timers are each on
    /// for as long as the ranking of the chart under the cursor stands in its phase.
    ///
    /// The timers the reference has a name for and never switches are left alone: the three bar
    /// movement timers and the stop timer. The panel timers belong to whatever opens the panels.
    pub(super) fn run_scene_timers(&mut self, shared: &mut AppShared) {
        let now_us = shared.skin_now_us();
        let times = SceneTimes { scene_ms: NO_SCENE_END_MS, ..SceneTimes::of_skin(shared.skins.document(SKIN_TYPE_MUSIC_SELECT)) };
        advance(times, &mut shared.skin_timers, now_us);

        let mark: BarMark = match self.tab {
            SelectTab::Songs => (SelectTab::Songs, shared.select_gen, shared.sel),
            SelectTab::Courses => (SelectTab::Courses, self.courses_generation, self.courses.cursor()),
        };
        let moved = self.bar_seen.is_some_and(|seen| seen != mark);
        if std::mem::take(&mut self.bar_changed) || moved {
            shared.skin_timers.set_on(timer_id::SONGBAR_CHANGE, now_us);
        }
        self.bar_seen = Some(mark);
        shared.skin_timers.switch(timer_id::SONGBAR_CHANGE, true, now_us);

        let ranking = self.ranking_phase(shared);
        for (timer, phase) in RANKING_TIMERS {
            shared.skin_timers.switch(timer, ranking == Some(phase), now_us);
        }
    }

    /// How the ranking of the chart under the cursor stands, or `None` when there is no score
    /// server, no chart under the cursor, or nothing has been asked of the server about it yet.
    fn ranking_phase(&self, shared: &AppShared) -> Option<RankingPhase> {
        if self.tab != SelectTab::Songs || !shared.has_primary_ir_server() {
            return None;
        }
        let md5 = shared.focused_md5()?;
        Some(match shared.ranking_cache.peek(&md5)? {
            RankingState::Loading => RankingPhase::Access,
            RankingState::Ready(_) => RankingPhase::Finish,
            RankingState::Failed(_) => RankingPhase::Fail,
        })
    }
}
