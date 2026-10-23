//! The song browser: the list and its detail panel, the hover preview, the record modal, the IR
//! ranking panel, and every way out of it (a chart, the managers, the settings screen, or quit).
//!
//! The list itself — which folder is open, the filtered rows and the cursor — lives in
//! [`AppShared`], because every other screen returns here and the debug overlay reports it from
//! anywhere. What this screen owns is what only makes sense while it is up.
//!
//! The largest pieces of that live in their own files: [`preview`] drives the hover preview's
//! loading and playback, [`list`] builds the list and its model, [`scene`] converts that model into
//! what each renderer draws, and [`images`] reads the pictures a skin's frame refers to.
//!
//! A browser a skin draws is also driven the way the reference's is: [`keys`] is its key table and
//! [`skinned`] is the frame of input and the timers that go with it, [`panel`] is the three option
//! panels the held keys call up, and [`events`] is what the skin's buttons and those panels' keys
//! run. The keys the browser has always had keep doing what they did there, and a browser no skin
//! draws is not touched by any of them.
//!
//! What is the application's rather than a skin's is drawn over the skin ([`overlay`]): the hint on
//! an empty list, the ranking panel, the record modal, the filter panel, the search box and the key
//! guide ([`guide`]), each of which takes the keys and the mouse before the skin does.
//!
//! The search is a state of its own ([`search`]): a word being typed takes the keys that are text
//! and no others, and the list a word left on show is driven like any other list.
#![allow(clippy::wildcard_imports)]

use std::sync::mpsc::TryRecvError;
use std::time::{SystemTime, UNIX_EPOCH};

use rbms_render::skin_render::frame::{BarHold, BarScroller, SCROLL_DURATION_HIGH_MS, SCROLL_DURATION_LOW_MS};
use rbms_render::{BgaFrame, FrameData, ReferenceImages, SongBars};
use rbms_skin::property::generated::{BUTTON_KEYCONFIG, BUTTON_SKINSELECT};

use crate::ir_ext::PrimaryProfileDirection;
use crate::ir_ranking_view::{PanelAction, PanelLine, offline_lines, panel_action, panel_lines, profile_line};
use crate::ir_replay::from_ir_replay;
use crate::keyconfig::{SCRATCH_BACKWARD_INDEX, SCRATCH_FORWARD_INDEX, Shortcut, ShortcutGroup, ShortcutScreen, key_index_of};
use crate::select_tree::{BarIdentity, OpenedFolder, SelectFolder};
use crate::skin_host::chart::ChartState;
use crate::skin_host::select::{BrowserLent, CourseShown};
use crate::skin_screen::SelectDraw;
use crate::stage::result::SkinStatus;
use crate::stage::{
    Canvas, FoldersState, FrameCtx, KeyConfigState, KeyInput, PlayMenuState, SettingsState, SkinMenuState, Stage, StageHandler, TablesState, Transition,
    is_left_press,
};
use crate::textedit::ImeEdit;
use crate::*;

#[cfg(test)]
mod bars_tests;
mod events;
#[cfg(test)]
mod events_tests;
mod filter;
mod guide;
mod images;
mod keys;
mod lent;
#[cfg(test)]
mod lent_tests;
mod list;
#[cfg(test)]
mod missing_tests;
mod overlay;
#[cfg(test)]
mod overlay_tests;
mod panel;
#[cfg(test)]
mod panel_tests;
mod preview;
#[cfg(test)]
pub(crate) mod recorded_tests;
mod scene;
mod search;
#[cfg(test)]
mod search_tests;
pub(crate) mod skin_preview;
mod skin_writes;
mod skinned;
#[cfg(test)]
mod skinned_tests;
#[cfg(test)]
mod table_tests;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
pub(super) mod tests;
#[cfg(test)]
mod tree_tests;
#[cfg(test)]
mod user_folder_tests;

use filter::{FilterKey, FilterPanel, render_filter_panel};
use guide::GUIDE_KEY;
use images::{SongImages, Wanted};
use keys::{KeyLayout, SelectKeys};
use lent::FocusedRecords;
pub(crate) use list::SelectFilter;
use list::{ChartFacts, chart_under_cursor, missing_chart_meta};
use panel::PanelState;
use preview::PreviewState;
use scene::BarList;
use skinned::{BarMark, WallClock, skin_status};

/// The key table the built-in browser reads its controller keys by, whatever mode is up: the
/// reference's table for a layout with a turntable ([`keys`]). The keys that open the focused row
/// are the ones that carry `FOLDER_OPEN` there, which on a chart are `PLAY`, `PRACTICE`, `AUTO` and
/// `REPLAY` -- the built-in browser opens the row for all of them -- and the keys that go up one
/// folder are the ones that carry `FOLDER_CLOSE`. A browser a skin draws reads the whole table, by
/// the mode that is up.
const BUILT_IN_KEY_LAYOUT: KeyLayout = KeyLayout::Beat;

/// The key the skin menu ships on, on the built-in browser and on one a skin draws alike
/// ([`Shortcut::SkinMenu`]). It is the key the reference's browser opens its skin configuration
/// with (`MusicSelector.input`), which on a skin's browser runs the same event the skin's own button
/// does. The key that is read is the one the key configuration holds; this is where it starts.
pub(crate) const SKIN_MENU_KEY: KeyCode = KeyCode::F12;

/// The key the play menu ships on, on the built-in browser and on one a skin draws alike
/// ([`Shortcut::PlayMenu`]). The reference has no screen of the kind and so no key for it; this is
/// a function key the browser read nothing from, beside the ones that open its other screens. The
/// key that is read is the one the key configuration holds; this is where it starts.
pub(crate) const PLAY_MENU_KEY: KeyCode = KeyCode::F8;

/// The key the ranking panel closes on by itself, beside the keys that go back
/// ([`crate::ir_ranking_view::panel_action`]): the one its shortcut ships on. Once the shortcut has
/// been moved the panel closes on the key it was moved to, and this one is whatever else it is.
const RANKING_PANEL_SHIPPED_KEY: KeyCode = KeyCode::KeyI;

/// How long a turntable direction is held before the list starts to repeat, and how often it moves
/// after that: the two times a skin's wheel slides by (`scrolldurationlow` and `scrolldurationhigh`
/// in `Config.java`, which `BarRenderer.input` applies to a held scratch).
const SCRATCH_REPEAT_DELAY: Duration = Duration::from_millis(SCROLL_DURATION_LOW_MS as u64);
const SCRATCH_REPEAT_INTERVAL: Duration = Duration::from_millis(SCROLL_DURATION_HIGH_MS as u64);

/// What the browser says when a chart a table lists that the library does not hold is asked for.
const NOT_IN_THE_LIBRARY: &str = "not in the library";

/// A turntable direction that is being held, so the list keeps moving while it is.
#[derive(Clone, Copy)]
struct ScratchHold {
    /// [`SCRATCH_FORWARD_INDEX`] or [`SCRATCH_BACKWARD_INDEX`].
    index: usize,
    next_step_at: Instant,
}

/// The browser's own state: the record modal, the quit confirmation, the ranking panel, the
/// lazily computed detail of the focused chart, the assembled scene cache and the hover preview.
pub(crate) struct SelectState {
    /// When the records modal is open, the index into the focused chart's record list (newest
    /// first) currently shown in detail.
    record_modal: Option<usize>,
    /// When the first Esc at the select root was pressed: a second press within
    /// [`crate::ROOT_ESC_CONFIRM`] quits, anything else cancels.
    esc_quit_at: Option<Instant>,
    /// IR ranking panel: whether it is open (which also gives it the arrow keys) and the focused row.
    ranking_open: bool,
    ranking_sel: usize,
    /// Lazily computed detail (notes/LN/length/BPM range) for the currently focused song, with the
    /// song index it was computed for — recomputed only when the focus moves to a different song.
    focused_detail: Option<ChartDetail>,
    focused_detail_si: Option<usize>,
    /// Focus-settle debounce for the heavy detail: the row the focus is currently on and when it
    /// arrived there. The detail is computed only once it has rested for [`FOCUS_DETAIL_DEBOUNCE`].
    focus_settle_si: Option<usize>,
    focus_settle_at: Instant,
    /// The focused chart's cover art, at the resolution the file itself was decoded at, and the
    /// song index it was decoded for.
    cover_image: Option<crate::DecodedImage>,
    cover_si: Option<usize>,
    cached_scene: Option<SelectScene>,
    cached_key: Option<SelectKey>,
    /// Whether a shift key is down, which is what turns the sort key around, and whether a control
    /// key is, which is what turns V into a paste. Tracked here rather than read from the event
    /// because a key event only names the key that moved.
    shift_held: bool,
    ctrl_held: bool,
    /// The search word as the search box edits it, and where the browser goes back to when the
    /// search is left, which is also what says a search is on ([`search`]). It is kept as a place
    /// that outlasts a difficulty table arriving while the search is on.
    search_edit: ImeEdit,
    search_resume: Option<crate::select_tree::ListPlace>,
    filter: FilterPanel,
    /// The list generation, the filter and the ordering the rows on screen were built with. Every
    /// other screen rebuilds the list unfiltered, and the settings screen can move the favourites
    /// switch and the SORT row behind the browser's back, so this is what notices any of them and
    /// rebuilds.
    applied: Option<(u64, SelectFilter, SortMode)>,
    preview: PreviewState,
    /// Which of the two lists the browser is showing.
    tab: SelectTab,
    /// The course list, resolved against the library it was built for.
    courses: CourseList,
    /// The turntable direction a controller is holding, which repeats the list move.
    scratch_hold: Option<ScratchHold>,
    /// The list on show as bars, which is what both the built-in rows and a skin's wheel are made
    /// from.
    bars: BarList,
    /// What the song database adds to the library's charts, read for a skin's wheel.
    facts: ChartFacts,
    /// How often [`SelectState::facts`] and the course list have been read, which is what tells the
    /// bars built from them that they are out of date.
    facts_generation: u64,
    courses_generation: u64,
    /// The pictures a skin's frame refers to: those of the chart under the cursor and the back
    /// image of the chart loaded last.
    images: SongImages,
    /// The back image of the chart loaded last, with the chart and the reading of the facts it was
    /// looked up for.
    loaded_backbmp: Option<(String, u64, Option<PathBuf>)>,
    /// The reference's key table as a browser a skin draws last read it, and whether it is being
    /// read at all: a browser that has just come up, or has just had its keys back from something
    /// drawn over it, takes what is held as held rather than as pressed.
    keys: SelectKeys,
    listening: bool,
    /// The slide of a skin's wheel and the repeat of a key held on it.
    scroller: BarScroller,
    /// How far the mouse wheel has turned since the list last moved by it, in lines.
    wheel_lines: f32,
    /// The bar the cursor was on when a skin's bar timer was last looked after, and whether
    /// something has asked for that timer to start over since.
    bar_seen: Option<BarMark>,
    bar_changed: bool,
    /// What had become of the skin on the frame drawn last.
    skin_seen: Option<SkinStatus>,
    /// What dates the frames of a skin's wheel.
    clock: WallClock,
    /// The option panel the held keys call up on a browser a skin draws.
    panel: PanelState,
    /// Whether the key guide is up over a browser a skin draws.
    guide_open: bool,
    /// Whether an event of the skin changed a setting that has not been written out yet.
    options_dirty: bool,
    /// What the player's records hold of the chart under the cursor, read when the cursor comes to
    /// a chart rather than on every frame a skin asks about it.
    focused_records: FocusedRecords,
    /// The replay slot a replay of the bar under the cursor would be played from, or `None` when it
    /// has no replay (`MusicSelector.selectedreplay`).
    selected_replay: Option<usize>,
    /// The course under the cursor as a skin reads it, with the reading of the course list and the
    /// row it was made for.
    course_shown: Option<(u64, usize, CourseShown)>,
    /// The way an arrow key that went down since the last frame of a skin's browser moves the
    /// wheel, kept so that a press let go before that frame is still one.
    arrow_tap: BarHold,
}

impl Default for SelectState {
    fn default() -> SelectState {
        SelectState {
            record_modal: None,
            esc_quit_at: None,
            ranking_open: false,
            ranking_sel: 0,
            focused_detail: None,
            focused_detail_si: None,
            focus_settle_si: None,
            focus_settle_at: Instant::now(),
            cover_image: None,
            cover_si: None,
            cached_scene: None,
            cached_key: None,
            shift_held: false,
            ctrl_held: false,
            search_edit: ImeEdit::default(),
            search_resume: None,
            filter: FilterPanel::default(),
            applied: None,
            preview: PreviewState::default(),
            tab: SelectTab::default(),
            courses: CourseList::default(),
            scratch_hold: None,
            bars: BarList::default(),
            facts: ChartFacts::default(),
            facts_generation: 0,
            courses_generation: 0,
            images: SongImages::default(),
            loaded_backbmp: None,
            keys: SelectKeys::default(),
            listening: false,
            scroller: BarScroller::default(),
            wheel_lines: 0.0,
            bar_seen: None,
            bar_changed: false,
            skin_seen: None,
            clock: WallClock::now(),
            panel: PanelState::default(),
            guide_open: false,
            options_dirty: false,
            focused_records: FocusedRecords::default(),
            selected_replay: None,
            course_shown: None,
            arrow_tap: BarHold::None,
        }
    }
}

impl SelectState {
    pub(crate) fn new() -> SelectState {
        SelectState::default()
    }

    /// A browser on its course tab, holding `courses` resolved against `library`, for the tests and
    /// the capture harness that show a course list without a settings folder to read one from.
    #[cfg(test)]
    pub(crate) fn on_courses(courses: Vec<rbms_course::Course>, library: &rbms_library::Library) -> SelectState {
        let mut courses = CourseList::from_courses(courses);
        courses.resolve_in_library(library);
        SelectState { tab: SelectTab::Courses, courses, ..SelectState::default() }
    }

    /// Whether every picture the browser asked for has been read, for the harness that waits for
    /// them before it saves a frame.
    #[cfg(test)]
    pub(crate) fn pictures_are_in(&self) -> bool {
        self.images.is_settled()
    }

    /// Whether the wheel of a skin's browser is not sliding, for the harness that waits a slide out
    /// before it saves a frame.
    #[cfg(test)]
    pub(crate) fn wheel_is_at_rest(&self) -> bool {
        let slide = self.scroller.at(self.clock.millis_at(Instant::now()));
        slide.duration_ms == 0 || slide.duration_ms <= slide.now_ms
    }

    /// Whether a second Esc/Left right now would quit: the arming press must still be inside
    /// [`crate::ROOT_ESC_CONFIRM`]. Read by both the guide line and the scene cache key, so the
    /// "press again" prompt disappears exactly when the window closes.
    fn esc_quit_armed(&self) -> bool {
        esc_confirms_quit(self.esc_quit_at, Instant::now())
    }

    fn select_key(&self, shared: &AppShared) -> SelectKey {
        let cursor = match self.tab {
            SelectTab::Songs => shared.sel,
            SelectTab::Courses => self.courses.cursor().wrapping_add(self.courses.len()),
        };
        (
            shared.select_gen,
            cursor,
            self.record_modal,
            shared.scores.records().len(),
            shared.config.display.score_graph,
            self.esc_quit_armed(),
            self.tab,
            self.is_skinned(shared),
        )
    }

    /// Whether a skin draws the browser, or is on its way to: the scene is worded for the screen
    /// that draws it.
    fn is_skinned(&self, shared: &AppShared) -> bool {
        shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT)
    }

    /// Move the focused row one step down the list (`forward`) or up it, on whichever tab is up.
    ///
    /// The course list wraps. The song list wraps when a skin draws the browser, whose wheel goes
    /// round the ends of its list as the reference's does (`BarManager.move`); the built-in
    /// browser's list stops at its ends, as it always has.
    fn step_cursor(&mut self, shared: &mut AppShared, forward: bool) {
        if self.tab == SelectTab::Courses {
            self.courses.move_cursor(if forward { 1 } else { -1 });
            return;
        }
        let rows = shared.select_items.len();
        if shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT) {
            if rows > 0 {
                shared.sel = (shared.sel.min(rows - 1) + if forward { 1 } else { rows - 1 }) % rows;
            }
        } else if forward {
            if shared.sel + 1 < rows {
                shared.sel += 1;
            }
        } else {
            shared.sel = shared.sel.saturating_sub(1);
        }
        shared.print_selection();
    }

    /// Whether the list may be moved with the keys of a controller right now: nothing is being
    /// typed into or read over it, and neither START nor SELECT is held, because with one of them
    /// down the same keys belong to the option panels.
    fn takes_pad_keys(&self, ctx: &FrameCtx<'_>) -> bool {
        !(self.holds_keys(ctx) || self.ranking_open || ctx.shared.options.is_open() || ctx.shared.start_pressed() || ctx.shared.select_pressed())
    }

    /// Go up one folder from a controller. Unlike Esc this never quits: at the root, and on the
    /// course tab, there is no folder to leave.
    fn close_folder(&mut self, shared: &mut AppShared) -> Transition {
        if self.tab != SelectTab::Songs || shared.is_at_root() {
            return Transition::Stay;
        }
        self.select_back(shared)
    }

    /// Keep the list moving while a turntable direction stays down: one step per
    /// [`SCRATCH_REPEAT_INTERVAL`] once it has been held for [`SCRATCH_REPEAT_DELAY`]. The hold is
    /// dropped the moment the key is up or the list stops taking controller keys.
    fn repeat_scratch(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some(hold) = self.scratch_hold else {
            return;
        };
        if !self.takes_pad_keys(ctx) || !ctx.shared.key_index_pressed(hold.index) {
            self.scratch_hold = None;
            return;
        }
        if ctx.now < hold.next_step_at {
            return;
        }
        self.step_cursor(ctx.shared, hold.index == SCRATCH_FORWARD_INDEX);
        self.scratch_hold = Some(ScratchHold { next_step_at: ctx.now + SCRATCH_REPEAT_INTERVAL, ..hold });
    }

    /// Enter the focused select item: descend into a folder, or start a chart.
    ///
    /// A chart goes to the decide scene when a skin draws one and to the LOADING screen otherwise
    /// ([`AppShared::decided_song_stage`]); either way the browser stays suspended underneath.
    ///
    /// Any replay download still in flight is abandoned when a chart starts: its result would
    /// otherwise land mid-load and swap the chart out from under the run that is starting.
    fn select_enter(&mut self, shared: &mut AppShared) -> Transition {
        self.enter_row(shared, shared.sel)
    }

    /// Enter the select item on `row` of the song list, which is the focused one for every way in
    /// but a press on a folder a skin's wheel shows off the cursor. The course tab has no rows of
    /// that list on show: there it is the focused course that starts.
    fn enter_row(&mut self, shared: &mut AppShared, row: usize) -> Transition {
        if self.tab == SelectTab::Courses {
            return self.course_enter(shared);
        }
        match shared.select_items.get(row) {
            Some(SelectItem::Song(i)) => {
                let i = *i;
                self.record_modal = None;
                shared.replay_download_rx = None;
                shared.replay_download_target = None;
                shared.play_system_sound(SystemSound::Select);
                Transition::Open(shared.decided_song_stage(i))
            }
            Some(SelectItem::Folder { target, .. }) => {
                let target = target.clone();
                shared.play_system_sound(SystemSound::FolderOpen);
                self.enter_folder(shared, target);
                Transition::Stay
            }
            Some(SelectItem::Missing(_)) => {
                crate::notify::notify(crate::notify::Level::Info, NOT_IN_THE_LIBRARY);
                Transition::Stay
            }
            None => Transition::Stay,
        }
    }

    /// Open `folder`, remembering the bar the cursor is on as the one to come back to
    /// (`BarManager.updateBar`: `prevbar`, kept in `sourcebars`). A folder opened by a press on a bar
    /// off the cursor comes back to the cursor's bar, as the reference's does.
    fn enter_folder(&mut self, shared: &mut AppShared, folder: SelectFolder) {
        let came_from = shared.select_items.get(shared.sel).and_then(|item| shared.identity_of(item));
        shared.select_path.push(OpenedFolder { folder, came_from });
        shared.sel = 0;
        self.rebuild(shared);
    }

    /// Close the folder open innermost and put the cursor on the bar it was opened from: the
    /// folder's own bar when it is the top of the library that comes back, else the bar the cursor
    /// was on when it was opened, and the first bar when that is no longer there (`BarManager.close`
    /// and `updateBar`).
    fn leave_folder(&mut self, shared: &mut AppShared) {
        let Some(left) = shared.select_path.pop() else {
            return;
        };
        self.rebuild(shared);
        let back_to = if shared.is_at_root() { Some(BarIdentity::Opens(left.folder)) } else { left.came_from };
        shared.sel = back_to.and_then(|identity| shared.row_of(&identity)).unwrap_or(0);
    }

    /// Start the focused course, unless one of its stages is not in the library — the reference
    /// will not begin a course it cannot supply every chart of.
    fn course_enter(&mut self, shared: &mut AppShared) -> Transition {
        let Some(entry) = self.courses.focused() else {
            return Transition::Stay;
        };
        if !entry.is_playable() {
            notify(Level::Warn, format!("{} stage(s) missing from the library", entry.missing.len()));
            return Transition::Stay;
        }
        let entry = entry.clone();
        shared.play_system_sound(SystemSound::Select);
        crate::start_course(shared, &entry)
    }

    /// Load the course list, or re-resolve the one already loaded against the library as it stands
    /// now — a scan that has since found a course's charts clears its missing-stage mark.
    fn refresh_courses(&mut self, shared: &AppShared) {
        if self.courses.is_empty() {
            self.courses = CourseList::load(&courses_dir(&crate::config_dir()), &shared.library);
        } else {
            self.courses.resolve_in_library(&shared.library);
        }
        self.courses_generation = self.courses_generation.wrapping_add(1);
    }

    /// Open the practice panel on the focused chart. The panel needs the parsed chart, so the load
    /// path is the one a run takes and the flag is what makes it land on the panel instead.
    fn start_practice(&mut self, shared: &mut AppShared) -> Transition {
        let Some(index) = shared.focused_song_index() else {
            return Transition::Stay;
        };
        self.record_modal = None;
        shared.practice_requested = true;
        Transition::Open(shared.decided_song_stage(index))
    }

    /// Rebuild the row list through the filter panel, and remember what it was built from so the
    /// browser can tell when something behind its back has changed either.
    fn rebuild(&mut self, shared: &mut AppShared) {
        let filter = self.filter.filter(&shared.config);
        shared.rebuild_select_items_with(filter);
        self.applied = Some((shared.select_gen, filter, shared.config.library.sort));
    }

    /// Cycle the song-list sort order (F3), or step back through it (Shift+F3).
    fn cycle_sort(&mut self, shared: &mut AppShared) {
        self.set_sort(shared, shared.config.library.sort.next());
    }

    /// Step back through the sort orders, so a list overshot by one press is one press away again.
    fn cycle_sort_back(&mut self, shared: &mut AppShared) {
        self.set_sort(shared, shared.config.library.sort.prev());
    }

    /// Take a new ordering, which is the configuration's own SORT row: the settings screen edits the
    /// same field, so an ordering chosen either way is the one the list is built with and the one
    /// that is still there on the next launch.
    fn set_sort(&mut self, shared: &mut AppShared, sort: SortMode) {
        if shared.config.library.sort == sort {
            return;
        }
        shared.config.library.sort = sort;
        shared.sel = 0;
        self.rebuild(shared);
        shared.save_settings();
    }

    /// Show or hide the filter panel (`F2`).
    fn toggle_filter_panel(&mut self) {
        self.filter.toggle();
    }

    /// Keys while the filter panel is up. Returns whether the panel consumed the key.
    ///
    /// The panel closes on the key that opened it, which is the key the key configuration gives the
    /// filter. The key the filter ships on is one the panel closes on by itself, so once the filter
    /// has been moved off it that key is taken and does nothing, like any key the panel has no use
    /// for.
    fn filter_panel_input(&mut self, shared: &mut AppShared, code: KeyCode) -> bool {
        if self.filter.is_open() {
            if shared.is_shortcut(Shortcut::Filter, code) {
                self.toggle_filter_panel();
                return true;
            }
            if code == Shortcut::Filter.default_key() {
                return true;
            }
        }
        match self.filter.handle_key(shared, code) {
            FilterKey::Ignored => false,
            FilterKey::Consumed => true,
            FilterKey::Moved => {
                shared.sel = 0;
                self.rebuild(shared);
                true
            }
        }
    }

    /// (Re)compute the focused chart's heavy detail (notes/LN/length/BPM range) only when the focus
    /// moves to a different song — so scrolling the list parses at most one chart per moved row.
    ///
    /// The cover is decoded on the same schedule ([`SelectState::refresh_cover`]).
    fn refresh_focused_detail(&mut self, shared: &AppShared, now: Instant) {
        let si = shared.focused_song_index();
        if si != self.focus_settle_si {
            self.focus_settle_si = si;
            self.focus_settle_at = now;
        }
        if self.focus_settle_at.elapsed() < FOCUS_DETAIL_DEBOUNCE {
            return;
        }
        self.refresh_cover(shared, si);
        if si == self.focused_detail_si {
            return;
        }
        self.focused_detail_si = si;
        self.focused_detail = si.and_then(|i| shared.library.songs().get(i)).and_then(|e| match &shared.song_db {
            Some(db) => crate::library::chart_detail(db, &e.path, e.mode),
            None => compute_chart_detail(&e.path, e.mode),
        });
    }

    /// Decode the cover of the chart the focus has settled on (`#STAGEFILE`, then `#BANNER`), which
    /// is uploaded into the single BGA slot when the built-in browser draws, so it costs one decode
    /// per moved row.
    ///
    /// A browser a skin draws does not decode it here, on the frame loop: its pictures are read by a
    /// worker ([`SelectState::refresh_song_images`]) and the cover is taken from those. The cover is
    /// forgotten while a skin draws, so a skin that goes away -- one that could not be read, or one
    /// the player switched off -- leaves the built-in browser to decode its cover again.
    fn refresh_cover(&mut self, shared: &AppShared, si: Option<usize>) {
        if shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT) {
            self.cover_image = None;
            self.cover_si = None;
            return;
        }
        if si == self.cover_si {
            return;
        }
        self.cover_si = si;
        self.cover_image = si.and_then(|i| shared.library.songs().get(i)).and_then(|e| {
            let dir = e.path.parent()?;
            [&e.stagefile, &e.banner].into_iter().filter(|n| !n.trim().is_empty()).find_map(|n| decode_bga_image(dir, n))
        });
    }

    /// Ask for the pictures a skin's frame refers to, and take in the ones that have been read.
    ///
    /// The stage file and the banner are those of the chart under the cursor, changed the moment
    /// the cursor moves, as the reference changes them (`MusicSelector.selectedBarMoved`); any
    /// other kind of bar has none. The back image is the one of the chart that was loaded last,
    /// which is the only back image the reference's browser ever has
    /// (`BMSResource.setBMSFile`). A browser no skin draws asks for nothing.
    fn refresh_song_images(&mut self, shared: &AppShared) {
        if !shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT) {
            self.images.want(Wanted::default());
            return;
        }
        if self.loaded_backbmp.as_ref().is_none_or(|(chart, facts, _)| *chart != shared.chart_path || *facts != self.facts_generation) {
            self.loaded_backbmp = Some((shared.chart_path.clone(), self.facts_generation, shared.loaded_chart_backbmp(&self.facts)));
        }
        let focused = self.bars.bars().get(self.cursor(shared)).and_then(|bar| bar.chart.as_ref()).map(|chart| &chart.images);
        self.images.want(Wanted {
            stagefile: focused.and_then(|images| images.stagefile.as_deref()),
            banner: focused.and_then(|images| images.banner.as_deref()),
            backbmp: self.loaded_backbmp.as_ref().and_then(|(_, _, backbmp)| backbmp.as_deref()),
        });
        self.images.poll();
    }

    /// Open the record-detail modal for the focused chart (newest record first), if it has any.
    fn open_record_modal(&mut self, shared: &AppShared) {
        if let Some(md5) = shared.focused_md5()
            && !shared.scores.for_md5(&md5).is_empty()
        {
            self.record_modal = Some(0);
        }
    }

    /// Move the record-detail modal selection within the focused chart's records (clamped).
    fn record_modal_nav(&mut self, shared: &AppShared, d: i32) {
        let Some(ri) = self.record_modal else { return };
        let n = shared.focused_md5().map(|m| shared.scores.for_md5(&m).len()).unwrap_or(0);
        if n == 0 {
            self.record_modal = None;
            return;
        }
        self.record_modal = Some((ri as i32 + d).clamp(0, n as i32 - 1) as usize);
    }

    /// Load and start the replay attached to the record currently shown in the modal.
    fn play_record_replay(&mut self, shared: &mut AppShared) -> Transition {
        let Some(ri) = self.record_modal else { return Transition::Stay };
        let Some(md5) = shared.focused_md5() else {
            return Transition::Stay;
        };
        let file = shared.scores.for_md5(&md5).get(ri).and_then(|r| r.replay_file.clone());
        let Some(file) = file else { return Transition::Stay };
        self.play_replay_file(shared, &file)
    }

    /// Load the saved replay `file` and start it, which also closes the record modal: a replay that
    /// cannot be read leaves the browser where it is with a message.
    fn play_replay_file(&mut self, shared: &mut AppShared, file: &str) -> Transition {
        let dir = shared.settings_path.parent().map(|d| d.join("replays")).unwrap_or_else(|| PathBuf::from("replays"));
        match Replay::load(&dir.join(file)) {
            Ok(rp) => {
                self.record_modal = None;
                shared.chart_path = rp.chart_path.clone();
                shared.replay = Some(rp);
                self.stop_preview(shared);
                match shared.load() {
                    Some(loaded) => Transition::Open(shared.decided_chart_stage(loaded)),
                    None => Transition::Stay,
                }
            }
            Err(e) => {
                notify(Level::Error, format!("replay load failed: {e}"));
                self.record_modal = None;
                Transition::Stay
            }
        }
    }

    /// Show or hide the ranking panel. While it is open it also takes the arrow keys, so one key
    /// both opens it and gives it focus.
    fn toggle_ranking_panel(&mut self, shared: &mut AppShared) {
        self.ranking_open = !self.ranking_open;
        self.ranking_sel = self.ranking_selectable_start(shared);
        if self.ranking_open {
            shared.ranking_requested = None;
        }
    }

    fn ranking_selectable_start(&self, shared: &AppShared) -> usize {
        usize::from(shared.can_switch_primary_ir())
    }

    fn clamp_ranking_selection(&mut self, shared: &AppShared, lines: &[PanelLine]) {
        let Some(last) = lines.len().checked_sub(1) else {
            self.ranking_sel = 0;
            return;
        };
        self.ranking_sel = self.ranking_sel.clamp(self.ranking_selectable_start(shared).min(last), last);
    }

    /// The lines the panel is showing for the focused chart.
    fn ranking_lines(&self, shared: &AppShared) -> Vec<PanelLine> {
        if !shared.has_primary_ir_server() {
            return offline_lines();
        }
        let mut lines = match shared.focused_md5() {
            Some(md5) => panel_lines(shared.ranking_cache.peek(&md5)),
            None => panel_lines(None),
        };
        if shared.can_switch_primary_ir()
            && let Some((label, position, count)) = shared.primary_ir_profile()
        {
            lines.insert(0, profile_line(&label, position, count));
        }
        lines
    }

    /// Keys while the ranking panel is open. Returns whether the panel consumed the key.
    fn ranking_panel_input(&mut self, shared: &mut AppShared, code: KeyCode) -> bool {
        if !self.ranking_open {
            return false;
        }
        let own = shared.is_shortcut(Shortcut::Ranking, code);
        let action = if own { Some(PanelAction::Close) } else { panel_action(code).filter(|_| code != RANKING_PANEL_SHIPPED_KEY) };
        let Some(action) = action else {
            return false;
        };
        let lines = self.ranking_lines(shared);
        self.clamp_ranking_selection(shared, &lines);
        let start = self.ranking_selectable_start(shared).min(lines.len().saturating_sub(1));
        match action {
            PanelAction::Close => self.toggle_ranking_panel(shared),
            PanelAction::Up => self.ranking_sel = self.ranking_sel.saturating_sub(1).max(start),
            PanelAction::Down => self.ranking_sel = (self.ranking_sel + 1).min(lines.len().saturating_sub(1)),
            PanelAction::PlayReplay => self.play_ranking_replay(shared),
            PanelAction::PreviousProfile => {
                if shared.shift_primary_ir(PrimaryProfileDirection::Previous) {
                    self.ranking_sel = self.ranking_selectable_start(shared);
                }
            }
            PanelAction::NextProfile => {
                if shared.shift_primary_ir(PrimaryProfileDirection::Next) {
                    self.ranking_sel = self.ranking_selectable_start(shared);
                }
            }
        }
        true
    }

    /// Click on a panel row: focus it, or start its replay when it is already focused.
    fn ranking_click(&mut self, shared: &mut AppShared, index: usize) {
        let lines = self.ranking_lines(shared);
        if lines.is_empty() {
            return;
        }
        let index = index.min(lines.len() - 1);
        if index < self.ranking_selectable_start(shared) {
            return;
        }
        if self.ranking_sel == index {
            self.play_ranking_replay(shared);
        } else {
            self.ranking_sel = index;
        }
    }

    /// Download the replay behind the focused panel row; the frame loop starts it once it lands.
    fn play_ranking_replay(&mut self, shared: &mut AppShared) {
        let Some(line) = self.ranking_lines(shared).into_iter().nth(self.ranking_sel) else {
            return;
        };
        let Some(replay_id) = line.replay_id else {
            shared.net_status = "no replay for that row".to_string();
            return;
        };
        let Some(index) = shared.focused_song_index() else {
            return;
        };
        let Some(entry) = shared.library.songs().get(index) else {
            return;
        };
        if shared.replay_download_rx.is_some() {
            return;
        }
        shared.replay_download_target = Some((entry.path.to_string_lossy().to_string(), entry.md5.clone()));
        shared.net_status = format!("downloading replay {replay_id}...");
        shared.replay_download_rx = Some(rbms_ir::spawn_query(shared.primary_ir_server(), move |server| server.download_replay(&replay_id)));
    }

    /// Start a ranking fetch when the focus has settled on a new chart and the panel has no cached
    /// answer for it. Called once per frame while the select screen is up.
    ///
    /// Only one fetch is ever in flight: a second one would drop the first receiver, stranding that
    /// chart's `Loading` placeholder in the cache and leaving the panel on LOADING forever. While a
    /// fetch runs the request for the newly focused chart is simply retried next frame.
    fn update_ranking(&mut self, shared: &mut AppShared) {
        if !self.ranking_open || !shared.has_primary_ir_server() || shared.ranking_rx.is_some() {
            return;
        }
        let Some(chart) = shared.focused_chart_id() else {
            return;
        };
        if self.focus_settle_at.elapsed() < FOCUS_DETAIL_DEBOUNCE {
            return;
        }
        if shared.ranking_requested.as_deref() == Some(chart.md5.as_str()) {
            return;
        }
        self.ranking_sel = self.ranking_selectable_start(shared);
        shared.start_ranking_fetch(chart);
    }

    /// Take a downloaded replay and enter playback for it, the same path a local record replay takes.
    fn poll_replay_download(&mut self, shared: &mut AppShared) -> Transition {
        let Some(rx) = shared.replay_download_rx.take() else {
            return Transition::Stay;
        };
        match rx.try_recv() {
            Ok(Ok(data)) => {
                let Some((chart_path, md5)) = shared.replay_download_target.take() else {
                    return Transition::Stay;
                };
                let replay = from_ir_replay(&data, &chart_path, &md5);
                if replay.events.is_empty() {
                    shared.net_status = "downloaded replay has no inputs".to_string();
                    return Transition::Stay;
                }
                shared.net_status = "replay downloaded".to_string();
                self.ranking_open = false;
                self.record_modal = None;
                shared.chart_path = chart_path;
                shared.replay = Some(replay);
                self.stop_preview(shared);
                match shared.load() {
                    Some(loaded) => Transition::Open(shared.decided_chart_stage(loaded)),
                    None => Transition::Stay,
                }
            }
            Ok(Err(error)) => {
                shared.replay_download_target = None;
                shared.net_status = format!("replay download: {}", crate::ir_outcome::short_error(&error));
                Transition::Stay
            }
            Err(TryRecvError::Empty) => {
                shared.replay_download_rx = Some(rx);
                Transition::Stay
            }
            Err(TryRecvError::Disconnected) => {
                shared.replay_download_target = None;
                Transition::Stay
            }
        }
    }

    /// Esc in the select screen: one level up, or — at the root — arm the quit confirmation and only
    /// exit on a second Esc within [`ROOT_ESC_CONFIRM`]. An empty library behaves the same, so a
    /// first-run window can't be closed by a stray keypress.
    fn select_escape(&mut self, shared: &mut AppShared) -> Transition {
        if !shared.is_at_root() {
            self.esc_quit_at = None;
            return self.select_back(shared);
        }
        if self.esc_quit_armed() {
            return Transition::Quit;
        }
        self.esc_quit_at = Some(Instant::now());
        Transition::Stay
    }

    /// Go up one select level; at the root, quit. A search is left before any folder is: the list
    /// it runs over is the flat song list, and going back from a search goes to where the search
    /// was made from rather than to the top of the library.
    fn select_back(&mut self, shared: &mut AppShared) -> Transition {
        if self.search_is_on(shared) {
            shared.play_system_sound(SystemSound::FolderClose);
            self.exit_search(shared);
            return Transition::Stay;
        }
        if shared.is_at_root() {
            return Transition::Quit;
        }
        shared.play_system_sound(SystemSound::FolderClose);
        self.leave_folder(shared);
        Transition::Stay
    }

    /// Keys while the record modal is open: it takes every key so the list underneath cannot move.
    fn modal_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) -> Transition {
        match key.code {
            KeyCode::Escape => self.record_modal = None,
            KeyCode::ArrowUp => self.record_modal_nav(shared, -1),
            KeyCode::ArrowDown => self.record_modal_nav(shared, 1),
            KeyCode::Enter | KeyCode::NumpadEnter => return self.play_record_replay(shared),
            _ => {}
        }
        Transition::Stay
    }

    /// One key going down on a browser a skin draws, once the key table has noted it, for the two
    /// things only such a browser has: the screens the reference's browser opens from its own keys,
    /// and the option panels. Answers what the key asked for, or `None` when it is not one of these.
    ///
    /// While an option panel is called up the keys that move through the list and start what is
    /// under the cursor are the panel's and do nothing to the list: the reference reads none of
    /// them with a panel up, and the two arrows that move the cursor scroll the first panel's target
    /// instead ([`panel`]). The screens the reference's browser opens from its own keys are
    /// shortcuts like any other ([`SelectState::run_shortcut`]).
    fn skin_screen_key(&mut self, shared: &mut AppShared, code: KeyCode) -> Option<Transition> {
        match code {
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Enter | KeyCode::NumpadEnter
                if self.panel_is_called_up(shared) =>
            {
                Some(Transition::Stay)
            }
            _ => None,
        }
    }

    /// Carry out one shortcut of the browser, whichever key or controller element it was read from.
    ///
    /// Each does what the key it ships on always did. The second set -- the reference's number keys
    /// -- is read on a browser a skin draws and nowhere else, as it always was: `1` steps the mode
    /// filter, `2` the order, `3` the long note mode, `4` moves the replay slot on, `6` opens the key
    /// configuration, `0` opens the search and `F9` stars the chart (`MusicSelectInputProcessor.java:71-110`,
    /// `MusicSelector.java:296`). On such a browser the skin menu and the key configuration are
    /// opened by running the events the skin's own buttons run (13 and 14). The play menu is opened
    /// the same way on either browser: the reference has no event that opens a screen of its kind,
    /// so there is none for a skin's button to run. The option overlay's two keys and the key held
    /// for the detail panel are read where those are, not here.
    ///
    /// The reload of the difficulty tables only starts here, and with it a fetch of the player's
    /// own folders when they are signed in: both are fetched beside the browser, which goes on being
    /// used, and what comes back is taken a frame at a time ([`crate::tablesrc::apply_finished_fetches`],
    /// [`crate::user_folders::apply_finished_fetch`]).
    fn run_shortcut(&mut self, shared: &mut AppShared, shortcut: Shortcut) -> Transition {
        let skinned = skin_status(shared) == SkinStatus::Ready;
        if shortcut.screen() != ShortcutScreen::Browser || (shortcut.group() == ShortcutGroup::SkinBrowser && !skinned) {
            return Transition::Stay;
        }
        match shortcut {
            Shortcut::Search | Shortcut::RefSearch => self.start_search(shared),
            Shortcut::Filter => self.toggle_filter_panel(),
            Shortcut::Sort if self.shift_held => self.cycle_sort_back(shared),
            Shortcut::Sort => self.cycle_sort(shared),
            Shortcut::RefSort => self.sort_event(shared, true),
            Shortcut::Practice => return self.start_practice(shared),
            Shortcut::Favourite | Shortcut::RefFavourite => shared.toggle_focused_favorite(),
            Shortcut::Records => self.open_record_modal(shared),
            Shortcut::Ranking => self.toggle_ranking_panel(shared),
            Shortcut::ReloadTables => {
                crate::tablesrc::reload_tables(shared, None);
                crate::user_folders::start_fetch(shared, true);
            }
            Shortcut::Settings if self.shift_held => {
                self.tab = self.tab.next();
                self.refresh_courses(shared);
            }
            Shortcut::Settings => return Transition::Open(Stage::Settings(SettingsState::new())),
            Shortcut::SkinMenu if skinned => return self.run_event(shared, BUTTON_SKINSELECT, 0),
            Shortcut::SkinMenu => return Transition::Open(SkinMenuState::stage()),
            Shortcut::PlayMenu => return Transition::Open(PlayMenuState::stage()),
            Shortcut::KeyConfig => return Transition::Open(Stage::KeyConfig(KeyConfigState::new())),
            Shortcut::RefKeyConfig => return self.run_event(shared, BUTTON_KEYCONFIG, 0),
            Shortcut::Folders => return Transition::Open(Stage::Folders(FoldersState::new())),
            Shortcut::Tables => return Transition::Open(Stage::Tables(TablesState::new())),
            Shortcut::RefMode => self.mode_event(shared, true),
            Shortcut::RefLnMode => self.ln_mode_event(shared, true),
            Shortcut::RefReplay => self.next_replay(shared),
            Shortcut::OptionsPin
            | Shortcut::OptionsHold
            | Shortcut::RefDetail
            | Shortcut::Retry
            | Shortcut::NextSong
            | Shortcut::ReplayPause
            | Shortcut::ReplayFaster
            | Shortcut::ReplaySlower
            | Shortcut::ReplayForward
            | Shortcut::ReplayBack => {}
        }
        Transition::Stay
    }

    /// One shortcut from a controller. It does what the key does, on the terms a key would: nothing
    /// while the skin is still being read or while something that takes every key is over the list,
    /// except that the filter panel closes on its own shortcut as it does on its key.
    fn pad_shortcut(&mut self, ctx: &mut FrameCtx<'_>, shortcut: Shortcut) -> Transition {
        if skin_status(ctx.shared) == SkinStatus::Reading {
            return Transition::Stay;
        }
        if self.filter.is_open() && shortcut == Shortcut::Filter {
            self.toggle_filter_panel();
            return Transition::Stay;
        }
        if self.holds_keys(ctx) || ctx.shared.options.is_open() || ctx.shared.skin_text_is_focused() {
            return Transition::Stay;
        }
        self.esc_quit_at = None;
        self.run_shortcut(ctx.shared, shortcut)
    }
}

impl StageHandler for SelectState {
    /// The browser keeps the keys from the option overlay while something on it is being typed into
    /// or read: the search box, the record modal, the filter panel, the key guide. The option
    /// overlay opens on a shift key, and a shift key held to type a capital letter belongs to the
    /// search box rather than to it. A list a search left on show is not one of these: it is a list.
    fn holds_keys(&self, ctx: &FrameCtx<'_>) -> bool {
        ctx.shared.searching || self.record_modal.is_some() || self.filter.is_open() || self.guide_open
    }

    /// Arriving back on the browser is when the course list is re-resolved: a scan that has just
    /// finished may have supplied the very charts a course was missing. A key still held from the
    /// screen that was left is not a press here.
    ///
    /// A screen that moved the browser off the list a search was on took the search with it, so the
    /// search is dropped too. A scan and the table manager leave the browser in the folders it was
    /// in ([`crate::select_tree::show_the_list_again`]), the search's own list among them.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        SelectState::end_autoplay_run(ctx.shared);
        self.drop_a_search_left_behind(ctx.shared);
        self.refresh_courses(ctx.shared);
        self.listening = false;
        self.guide_open = false;
    }

    /// One frame of the browser. What moves the list depends on what draws it: a skin's browser
    /// reads the reference's key table once a frame ([`SelectState::skin_input_frame`]) and keeps
    /// the skin's timers ([`SelectState::run_scene_timers`]), a browser waiting for its skin takes
    /// no input at all, and the built-in browser repeats a held turntable as it always has.
    ///
    /// The difficulty tables are fetched beside the browser and never ahead of it. The first frame
    /// after the library has been read is where the tables that have no copy yet start being
    /// fetched ([`crate::tablesrc::start_first_fetches`]), and every frame takes what any fetch has
    /// come back with ([`crate::tablesrc::apply_finished_fetches`]): a table that arrives is a
    /// folder from that frame, with the cursor left on the bar it was on. The player's own folders
    /// land the same way ([`crate::user_folders::apply_finished_fetch`]).
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let started = self.poll_replay_download(ctx.shared);
        if !matches!(started, Transition::Stay) {
            return started;
        }
        crate::tablesrc::start_first_fetches(ctx.shared);
        crate::tablesrc::apply_finished_fetches(ctx.shared);
        crate::user_folders::apply_finished_fetch(ctx.shared);
        let skin = skin_status(ctx.shared);
        if ctx.shared.skin_text_is_focused() {
            self.confirm_search(ctx.shared);
        }
        self.refresh_lent(ctx.shared);
        if skin == SkinStatus::Ready {
            self.follow_a_press_on_the_skins_search_text(ctx.shared);
        }
        let asked = match skin {
            SkinStatus::Ready => match self.skin_input_frame(ctx) {
                Transition::Stay => self.carry_out_skin_events(ctx.shared),
                asked => asked,
            },
            SkinStatus::Reading => {
                self.listening = false;
                self.guide_open = false;
                self.panel = PanelState::default();
                Transition::Stay
            }
            SkinStatus::Gone => {
                self.listening = false;
                self.guide_open = false;
                self.panel = PanelState::default();
                self.repeat_scratch(ctx);
                Transition::Stay
            }
        };
        self.settle_option_changes(ctx.shared);
        if !matches!(asked, Transition::Stay) {
            return asked;
        }
        self.bar_changed |= skin == SkinStatus::Ready && SelectState::scrollbar_was_dragged(ctx.shared);
        self.carry_out_skin_writes(ctx.shared);
        if self.applied != Some((ctx.shared.select_gen, self.filter.filter(&ctx.shared.config), ctx.shared.config.library.sort)) {
            self.rebuild(ctx.shared);
        }
        self.refresh_focused_detail(ctx.shared, ctx.now);
        self.refresh_bars(ctx.shared);
        if skin == SkinStatus::Ready {
            self.run_scene_timers(ctx.shared);
        }
        self.refresh_song_images(ctx.shared);
        self.update_preview(ctx.shared, ctx.now);
        self.update_ranking(ctx.shared);
        Transition::Stay
    }

    /// The preview owns the shared output stream's preview namespace, so it is torn down before any
    /// other screen can touch the engine. A search word being typed is confirmed on the way out, so
    /// the browser that is come back to is a list and not a box waiting for letters.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        self.confirm_search(ctx.shared);
        self.write_option_changes(ctx.shared);
        if self.preview_active() {
            self.stop_preview(ctx.shared);
        }
        SelectState::end_select_bgm(ctx.shared);
    }

    /// Keys on the browser. While a search word is typed the keys that are text are the word's, and
    /// the rest go on to do what they do on the list ([`SelectState::search_key`]).
    ///
    /// The keys that move through the list, open what is under the cursor and go back are the
    /// browser's own and fixed. Every other key is looked up in the key configuration
    /// ([`Shortcut`]), so a shortcut the player moved is read from where they moved it.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if matches!(key.code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            if key.pressed {
                self.shift_held = true;
            } else if key.released {
                self.shift_held = false;
            }
        }
        if matches!(key.code, KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::SuperLeft | KeyCode::SuperRight) {
            if key.pressed {
                self.ctrl_held = true;
            } else if key.released {
                self.ctrl_held = false;
            }
        }
        if !key.pressed {
            return Transition::Stay;
        }
        let skin = skin_status(ctx.shared);
        if skin == SkinStatus::Reading {
            return Transition::Stay;
        }
        if self.guide_open {
            self.guide_key(key.code);
            return Transition::Stay;
        }
        if self.record_modal.is_some() {
            return self.modal_key(ctx.shared, &key);
        }
        if self.filter_panel_input(ctx.shared, key.code) {
            return Transition::Stay;
        }
        if ctx.shared.searching
            && let Some(asked) = self.search_key(ctx, &key)
        {
            return asked;
        }
        if self.ranking_panel_input(ctx.shared, key.code) {
            return Transition::Stay;
        }
        if !matches!(key.code, KeyCode::Escape | KeyCode::ArrowLeft) {
            self.esc_quit_at = None;
        }
        if skin == SkinStatus::Ready {
            if key.code == GUIDE_KEY && !self.ctrl_held && !self.ranking_open {
                self.guide_open = true;
                return Transition::Stay;
            }
            if self.skinned_key(ctx, key.code) {
                return Transition::Stay;
            }
            if let Some(asked) = self.skin_screen_key(ctx.shared, key.code) {
                return asked;
            }
        }
        match key.code {
            KeyCode::Escape | KeyCode::ArrowLeft => return self.select_escape(ctx.shared),
            KeyCode::ArrowUp => self.step_cursor(ctx.shared, false),
            KeyCode::ArrowDown => self.step_cursor(ctx.shared, true),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::ArrowRight => return self.select_enter(ctx.shared),
            code => {
                if let Some(shortcut) = ctx.shared.shortcut_for(ShortcutScreen::Browser, code) {
                    return self.run_shortcut(ctx.shared, shortcut);
                }
            }
        }
        Transition::Stay
    }

    /// The input method composes into the search word while one is typed, and nowhere else.
    fn handle_ime(&mut self, ctx: &mut FrameCtx<'_>, ime: &winit::event::Ime) {
        self.search_ime(ctx.shared, ime);
    }

    /// A controller on the browser, with the panels closed: the turntable moves down and up the
    /// list, a white key opens the focused row and a black key goes up a folder — the first rows
    /// of the reference implementation's select key table (`b4-screens.md` section 3.5).
    ///
    /// Only a lane going down counts. The key is named by its index in the mode that is up
    /// ([`key_index_of`]), so the second side of a double-play layout does the same as the first.
    ///
    /// That is the built-in browser. A browser a skin draws reads the whole table off the keys'
    /// state once a frame, so there a key going down is only noted for the frame that follows, and
    /// a browser waiting for its skin takes nothing.
    ///
    /// A key of a controller is no letter: one going down while a search word is typed ends the
    /// typing, with what was typed left on show, before it does what it does.
    fn handle_pad(&mut self, ctx: &mut FrameCtx<'_>, event: PadEvent) -> Transition {
        if matches!(event, PadEvent::Lane { press: true, .. } | PadEvent::Control(_) | PadEvent::Shortcut(_)) {
            self.confirm_search(ctx.shared);
        }
        if let PadEvent::Shortcut(shortcut) = event {
            return self.pad_shortcut(ctx, shortcut);
        }
        let PadEvent::Lane { lane, dir, press: true } = event else {
            return Transition::Stay;
        };
        let Some(index) = key_index_of(ctx.shared.mode, lane, dir) else {
            return Transition::Stay;
        };
        match skin_status(ctx.shared) {
            SkinStatus::Reading => return Transition::Stay,
            SkinStatus::Ready => {
                self.note_key_index(ctx, index);
                return Transition::Stay;
            }
            SkinStatus::Gone => {}
        }
        if !self.takes_pad_keys(ctx) {
            return Transition::Stay;
        }
        self.esc_quit_at = None;
        match index {
            SCRATCH_FORWARD_INDEX | SCRATCH_BACKWARD_INDEX => {
                self.step_cursor(ctx.shared, index == SCRATCH_FORWARD_INDEX);
                self.scratch_hold = Some(ScratchHold { index, next_step_at: ctx.now + SCRATCH_REPEAT_DELAY });
            }
            _ if BUILT_IN_KEY_LAYOUT.carries(index, keys::SelectKey::FolderOpen) => return self.select_enter(ctx.shared),
            _ if BUILT_IN_KEY_LAYOUT.carries(index, keys::SelectKey::FolderClose) => return self.close_folder(ctx.shared),
            _ => {}
        }
        Transition::Stay
    }

    /// Clicks on the browser. The bottom navigation buttons are the clickable equivalents of the
    /// keyboard shortcuts, and a click that misses every region closes an open modal.
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32), button: MouseButton, pressed: bool) -> Transition {
        if !is_left_press(button, pressed) {
            return Transition::Stay;
        }
        match ctx.shared.hit_test(at) {
            Some(Hot::SelectRow(idx)) if self.tab == SelectTab::Courses => {
                if self.courses.cursor() == idx {
                    return self.select_enter(ctx.shared);
                }
                self.courses.focus(idx);
            }
            Some(Hot::SelectRow(idx)) => {
                if ctx.shared.sel == idx {
                    return self.select_enter(ctx.shared);
                }
                ctx.shared.sel = idx;
                self.record_modal = None;
                ctx.shared.print_selection();
            }
            Some(Hot::RecordRow(ri)) => self.record_modal = Some(ri),
            Some(Hot::RankingRow(i)) => self.ranking_click(ctx.shared, i),
            Some(Hot::ModalReplay) => return self.play_record_replay(ctx.shared),
            Some(Hot::ModalClose) => self.record_modal = None,
            Some(Hot::GuideClose) => self.guide_open = false,
            Some(Hot::OverlayPanel) => {}
            Some(Hot::NavSearch) => {
                if self.search_is_on(ctx.shared) {
                    self.exit_search(ctx.shared);
                } else {
                    self.start_search(ctx.shared);
                }
            }
            Some(Hot::NavSort) => self.cycle_sort(ctx.shared),
            Some(Hot::NavFolders) => return Transition::Open(Stage::Folders(FoldersState::new())),
            Some(Hot::NavTables) => return Transition::Open(Stage::Tables(TablesState::new())),
            Some(Hot::NavRecords) => self.open_record_modal(ctx.shared),
            Some(Hot::NavSettings) => return Transition::Open(Stage::Settings(SettingsState::new())),
            None => self.record_modal = None,
            _ => {}
        }
        Transition::Stay
    }

    /// The wheel of the mouse turns a skin's list, and waits for the frame that follows like the
    /// keys do (`BarRenderer.input`). The built-in browser has no use for it. A search word being
    /// typed does not take the wheel: it turns the list under the box.
    fn handle_scroll(&mut self, ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        if skin_status(ctx.shared) == SkinStatus::Ready && !self.list_is_covered(ctx) {
            self.wheel_lines += lines;
        }
        Transition::Stay
    }

    /// Paints the browser. The focused chart's cover goes into the single BGA slot, drawn behind the
    /// panel quads: the renderer leaves the cover square unfilled so the texture shows through.
    ///
    /// A skin that is on its way is waited for in the dark rather than behind the built-in browser:
    /// the reference stands still on the frame it left while it reads a skin, which after a result
    /// has faded out is black. A skin that is not coming -- it could not be read, or it has been
    /// waited for too long -- leaves the built-in browser to draw.
    ///
    /// The frame a skin can first be drawn on, after this browser has drawn frames without it, is
    /// the first moment of its scene: every timer off and the scene clock at zero. A skin that was
    /// there on the browser's first frame is carrying a scene on -- the one that was begun when the
    /// screen was entered, or the one a screen opened over the browser was closed back onto -- and
    /// is left to.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        self.refresh_scene_cache(ctx.shared);
        self.refresh_lent(ctx.shared);
        let ranking_lines = if self.ranking_open { self.ranking_lines(ctx.shared) } else { Vec::new() };
        self.clamp_ranking_selection(ctx.shared, &ranking_lines);
        let Some(view) = self.cached_scene.as_ref() else {
            return;
        };
        ctx.shared.prepare_skin(canvas, SKIN_TYPE_MUSIC_SELECT);
        let skin = skin_status(ctx.shared);
        if skin == SkinStatus::Ready && self.skin_seen.is_some_and(|seen| seen != SkinStatus::Ready) {
            ctx.shared.start_skin_scene_clock();
        }
        self.skin_seen = Some(skin);
        let has_document = ctx.shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT);
        match (self.cover_image.as_ref().or_else(|| self.images.cover()), &view.detail) {
            (Some(cover), SelectDetail::Song(_)) if !has_document => {
                canvas.set_background(cover.generation, &cover.rgba, cover.width, cover.height, cover_rect());
            }
            _ => canvas.clear_bga(),
        }
        let wall_clock = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let cursor = self.cursor(ctx.shared);
        let images = if has_document { self.images.textures(canvas) } else { ReferenceImages::default() };
        let bars = if has_document { self.bars.wheel(i64::try_from(wall_clock.as_secs()).unwrap_or(i64::MAX)) } else { &[] };
        let wheel = SongBars {
            ln_mode: i32::from(ctx.shared.config.judge.ln_mode.id()),
            scroll: self.scroller.at(self.clock.millis_at(ctx.now)),
            ..SongBars::new(bars, cursor)
        };
        let entry = SelectState::search_entry(&self.search_edit, ctx.shared);
        let data = FrameData { bars: Some(&wheel), images, bga: BgaFrame::of(images.stagefile.or(images.banner)), entry, ..FrameData::default() };
        let lent = BrowserLent {
            courses: &view.header,
            panel: self.panel.number(),
            mode_filter: self.filter.filter(&ctx.shared.config).mode,
            course: self.course_shown.as_ref().map(|(_, _, course)| course),
            selected_replay: self.selected_replay,
            best: self.focused_records.best(),
            replays_stored: self.focused_records.replays_stored(),
        };
        let focused = if self.tab == SelectTab::Songs { ctx.shared.focused_song_index() } else { None };
        let measured = self.focused_detail.as_ref().filter(|_| self.focused_detail_si == focused);
        let missing = match ctx.shared.select_items.get(ctx.shared.sel) {
            Some(SelectItem::Missing(missing)) if self.tab == SelectTab::Songs => ctx.shared.missing_chart(missing),
            _ => None,
        };
        let meta = focused
            .and_then(|index| Some(chart_under_cursor(ctx.shared.library.songs().get(index)?, self.facts.of(index), measured, &images)))
            .or_else(|| missing.map(missing_chart_meta));
        let chart = meta.as_ref().map_or(ChartState::Empty, ChartState::Chart);
        if ctx.shared.draw_select_skin(canvas, &SelectDraw { data, chart, lent }) {
            self.draw_skin_overlays(canvas, ctx.shared, view, &ranking_lines);
            return;
        }
        if skin == SkinStatus::Reading {
            canvas.native().clear(Color::BLACK);
            return;
        }
        let hot = render_select(canvas, view);
        ctx.shared.hot.extend(hot.into_iter().map(|(rect, h)| {
            let mapped = match h {
                SelectHot::Row(i) => Hot::SelectRow(i),
                SelectHot::Record(i) => Hot::RecordRow(i),
                SelectHot::ModalReplay => Hot::ModalReplay,
                SelectHot::ModalClose => Hot::ModalClose,
                SelectHot::Search => Hot::NavSearch,
                SelectHot::Sort => Hot::NavSort,
                SelectHot::Folders => Hot::NavFolders,
                SelectHot::Tables => Hot::NavTables,
                SelectHot::Records => Hot::NavRecords,
                SelectHot::Settings => Hot::NavSettings,
            };
            (rect, mapped)
        }));
        if self.ranking_open {
            let hot = render_ranking_panel(canvas, &ranking_lines, self.ranking_sel, true, ctx.shared.can_switch_primary_ir());
            ctx.shared.hot.extend(hot.into_iter().map(|(rect, index)| (rect, Hot::RankingRow(index))));
        }
        if self.filter.is_open() {
            render_filter_panel(canvas, &self.filter, &ctx.shared.config);
        }
    }
}
