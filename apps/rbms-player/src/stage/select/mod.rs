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
#![allow(clippy::wildcard_imports)]

use std::sync::mpsc::TryRecvError;
use std::time::{SystemTime, UNIX_EPOCH};

use rbms_render::skin_render::frame::{BarHold, BarScroller, SCROLL_DURATION_HIGH_MS, SCROLL_DURATION_LOW_MS};
use rbms_render::{BgaFrame, FrameData, ReferenceImages, SongBars};
use rbms_skin::property::generated::{BUTTON_KEYCONFIG, BUTTON_SKINSELECT};

use crate::ir_ext::PrimaryProfileDirection;
use crate::ir_ranking_view::{PanelAction, PanelLine, offline_lines, panel_action, panel_lines, profile_line};
use crate::ir_replay::from_ir_replay;
use crate::keyconfig::{SCRATCH_BACKWARD_INDEX, SCRATCH_FORWARD_INDEX, key_index_of};
use crate::skin_host::chart::ChartState;
use crate::skin_host::select::{BrowserLent, CourseShown};
use crate::skin_screen::SelectDraw;
use crate::stage::result::SkinStatus;
use crate::stage::{Canvas, FoldersState, FrameCtx, KeyInput, SettingsState, Stage, StageHandler, TablesState, Transition, is_left_press};
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
mod overlay;
#[cfg(test)]
mod overlay_tests;
mod panel;
#[cfg(test)]
mod panel_tests;
mod preview;
mod scene;
mod skin_writes;
mod skinned;
#[cfg(test)]
mod skinned_tests;
#[cfg(test)]
pub(super) mod tests;

use filter::{FilterKey, FilterPanel, render_filter_panel};
use guide::GUIDE_KEY;
use images::{SongImages, Wanted};
use keys::{KeyLayout, SelectKeys};
use lent::FocusedRecords;
use list::{ChartFacts, SelectFilter, chart_under_cursor};
use panel::PanelState;
use preview::PreviewState;
use scene::BarList;
use skinned::{BarMark, WallClock, skin_status};

/// What the search box draws where the next character will go.
const SEARCH_CARET: &str = "_";

/// Roughly how many characters the search box shows at once, so a query too long for it scrolls
/// under the caret instead of hiding the end being edited. The renderer trims to the exact pixel
/// width on top of this; this only decides which stretch of the query it is handed.
const SEARCH_VISIBLE_CHARS: usize = 24;

/// The key table the built-in browser reads its controller keys by, whatever mode is up: the
/// reference's table for a layout with a turntable ([`keys`]). The keys that open the focused row
/// are the ones that carry `FOLDER_OPEN` there, which on a chart are `PLAY`, `PRACTICE`, `AUTO` and
/// `REPLAY` -- the built-in browser opens the row for all of them -- and the keys that go up one
/// folder are the ones that carry `FOLDER_CLOSE`. A browser a skin draws reads the whole table, by
/// the mode that is up.
const BUILT_IN_KEY_LAYOUT: KeyLayout = KeyLayout::Beat;

/// How long a turntable direction is held before the list starts to repeat, and how often it moves
/// after that: the two times a skin's wheel slides by (`scrolldurationlow` and `scrolldurationhigh`
/// in `Config.java`, which `BarRenderer.input` applies to a held scratch).
const SCRATCH_REPEAT_DELAY: Duration = Duration::from_millis(SCROLL_DURATION_LOW_MS as u64);
const SCRATCH_REPEAT_INTERVAL: Duration = Duration::from_millis(SCROLL_DURATION_HIGH_MS as u64);

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
    /// The search query being typed, and where the browser goes back to when the box closes.
    search_edit: TextEdit,
    search_resume: Option<(SelectView, usize)>,
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
            search_edit: TextEdit::new(),
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
        if self.tab != SelectTab::Songs || shared.select_view == SelectView::Root {
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
                let target = *target;
                shared.play_system_sound(SystemSound::FolderOpen);
                shared.select_view = target;
                shared.sel = 0;
                self.rebuild(shared);
                Transition::Stay
            }
            None => Transition::Stay,
        }
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

    /// Open the search box (`/`). Search spans the whole library, so switch to the flat song list,
    /// remembering the folder and the row it was opened from.
    fn start_search(&mut self, shared: &mut AppShared) {
        if !shared.searching {
            self.search_resume = Some((shared.select_view, shared.sel));
        }
        shared.searching = true;
        self.search_edit = TextEdit::new();
        shared.search.clear();
        shared.select_view = SelectView::AllSongs;
        shared.sel = 0;
        self.rebuild(shared);
    }

    /// Close the search box (Esc), clear the query and go back to the folder and the row the search
    /// was opened from — a search is a detour, not a way of leaving the folder you were in.
    fn exit_search(&mut self, shared: &mut AppShared) {
        shared.searching = false;
        self.search_edit = TextEdit::new();
        shared.search.clear();
        let (view, sel) = self.search_resume.take().unwrap_or((SelectView::AllSongs, 0));
        shared.select_view = view;
        shared.sel = 0;
        self.rebuild(shared);
        shared.sel = sel.min(shared.select_items.len().saturating_sub(1));
    }

    /// Take what has been typed into the query the list is filtered by.
    fn apply_search(&mut self, shared: &mut AppShared) {
        shared.search = self.search_edit.text().to_string();
        shared.sel = 0;
        self.rebuild(shared);
    }

    /// The stretch of the query the search box draws, with a caret standing where the next
    /// character will go. A query longer than the box scrolls with the caret, so editing the start
    /// of a long one is not done blind.
    pub(super) fn search_display(&self) -> String {
        let (shown, at) = self.search_edit.window(SEARCH_VISIBLE_CHARS);
        let split = shown.char_indices().nth(at).map_or(shown.len(), |(byte, _)| byte);
        format!("{}{SEARCH_CARET}{}", &shown[..split], &shown[split..])
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
    fn filter_panel_input(&mut self, shared: &mut AppShared, code: KeyCode) -> bool {
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
        let Some(action) = panel_action(code) else {
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
        if shared.select_view != SelectView::Root {
            self.esc_quit_at = None;
            return self.select_back(shared);
        }
        if self.esc_quit_armed() {
            return Transition::Quit;
        }
        self.esc_quit_at = Some(Instant::now());
        Transition::Stay
    }

    /// Go up one select level; at the root, quit.
    fn select_back(&mut self, shared: &mut AppShared) -> Transition {
        match shared.select_view {
            SelectView::Root => return Transition::Quit,
            _ => shared.play_system_sound(SystemSound::FolderClose),
        }
        match shared.select_view {
            SelectView::Root => return Transition::Quit,
            SelectView::AllSongs | SelectView::TableLevels(_) => {
                shared.select_view = SelectView::Root;
                shared.sel = 0;
                self.rebuild(shared);
            }
            SelectView::TableLevel(ti, _) => {
                shared.select_view = SelectView::TableLevels(ti);
                shared.sel = 0;
                self.rebuild(shared);
            }
        }
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
    /// `6` opens the key configuration and `F12` the skin settings, which are the events 13 and 14
    /// (`MusicSelector.input`). While an option panel is called up the keys that move through the
    /// list and start what is under the cursor are the panel's and do nothing to the list: the
    /// reference reads none of them with a panel up, and the two arrows that move the cursor scroll
    /// the first panel's target instead ([`panel`]).
    fn skin_screen_key(&mut self, shared: &mut AppShared, code: KeyCode) -> Option<Transition> {
        match code {
            KeyCode::Digit6 => Some(self.run_event(shared, BUTTON_KEYCONFIG, 0)),
            KeyCode::F12 => Some(self.run_event(shared, BUTTON_SKINSELECT, 0)),
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Enter | KeyCode::NumpadEnter
                if self.panel_is_called_up(shared) =>
            {
                Some(Transition::Stay)
            }
            _ => None,
        }
    }

    /// Keys while the search box is open: everything that is not a control key types into the query.
    fn search_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) -> Transition {
        match key.code {
            KeyCode::Escape => self.exit_search(shared),
            KeyCode::Backspace => {
                self.search_edit.backspace();
                self.apply_search(shared);
            }
            KeyCode::Delete => {
                self.search_edit.delete();
                self.apply_search(shared);
            }
            KeyCode::ArrowLeft => self.search_edit.left(),
            KeyCode::ArrowRight => self.search_edit.right(),
            KeyCode::Home => self.search_edit.home(),
            KeyCode::End => self.search_edit.end(),
            KeyCode::KeyV if self.ctrl_held => {
                self.search_edit.paste();
                self.apply_search(shared);
            }
            KeyCode::F3 => self.cycle_sort(shared),
            KeyCode::ArrowUp => {
                shared.sel = shared.sel.saturating_sub(1);
                shared.print_selection();
            }
            KeyCode::ArrowDown => {
                if shared.sel + 1 < shared.select_items.len() {
                    shared.sel += 1;
                }
                shared.print_selection();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => return self.select_enter(shared),
            _ => {
                if let Some(t) = key.text {
                    self.search_edit.insert(t);
                    if self.search_edit.text() != shared.search {
                        self.apply_search(shared);
                    }
                }
            }
        }
        Transition::Stay
    }
}

impl StageHandler for SelectState {
    /// The browser takes every key itself while something on it is being typed into or read: the
    /// search box, the record modal, the filter panel. The option overlay opens on a shift key, and
    /// a shift key held to type a capital letter belongs to the search box rather than to it.
    fn holds_keys(&self, ctx: &FrameCtx<'_>) -> bool {
        ctx.shared.searching || self.record_modal.is_some() || self.filter.is_open() || self.guide_open
    }

    /// Arriving back on the browser is when the course list is re-resolved: a scan that has just
    /// finished may have supplied the very charts a course was missing. A key still held from the
    /// screen that was left is not a press here.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        SelectState::end_autoplay_run(ctx.shared);
        self.refresh_courses(ctx.shared);
        self.listening = false;
        self.guide_open = false;
    }

    /// One frame of the browser. What moves the list depends on what draws it: a skin's browser
    /// reads the reference's key table once a frame ([`SelectState::skin_input_frame`]) and keeps
    /// the skin's timers ([`SelectState::run_scene_timers`]), a browser waiting for its skin takes
    /// no input at all, and the built-in browser repeats a held turntable as it always has.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let started = self.poll_replay_download(ctx.shared);
        if !matches!(started, Transition::Stay) {
            return started;
        }
        let skin = skin_status(ctx.shared);
        self.refresh_lent(ctx.shared);
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
    /// other screen can touch the engine.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        self.write_option_changes(ctx.shared);
        if self.preview_active() {
            self.stop_preview(ctx.shared);
        }
        SelectState::end_select_bgm(ctx.shared);
    }

    /// Keys on the browser. While the search box is open, anything that is not a named shortcut —
    /// letters, digits, space — types into the query.
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
        if ctx.shared.searching {
            return self.search_key(ctx.shared, &key);
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
            KeyCode::Slash => self.start_search(ctx.shared),
            KeyCode::F3 if self.shift_held => self.cycle_sort_back(ctx.shared),
            KeyCode::F3 => self.cycle_sort(ctx.shared),
            KeyCode::F2 => self.toggle_filter_panel(),
            KeyCode::F4 => return self.start_practice(ctx.shared),
            KeyCode::Tab if self.shift_held => {
                self.tab = self.tab.next();
                self.refresh_courses(ctx.shared);
            }
            KeyCode::KeyF => ctx.shared.toggle_focused_favorite(),
            KeyCode::Tab => return Transition::Open(Stage::Settings(SettingsState::new())),
            KeyCode::KeyO => return Transition::Open(Stage::Folders(FoldersState::new())),
            KeyCode::KeyT => return Transition::Open(Stage::Tables(TablesState::new())),
            KeyCode::KeyR => self.open_record_modal(ctx.shared),
            KeyCode::KeyI => self.toggle_ranking_panel(ctx.shared),
            KeyCode::ArrowUp => self.step_cursor(ctx.shared, false),
            KeyCode::ArrowDown => self.step_cursor(ctx.shared, true),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::ArrowRight => return self.select_enter(ctx.shared),
            _ => {}
        }
        Transition::Stay
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
    fn handle_pad(&mut self, ctx: &mut FrameCtx<'_>, event: PadEvent) -> Transition {
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
                if ctx.shared.searching {
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
    /// keys do (`BarRenderer.input`). The built-in browser has no use for it.
    fn handle_scroll(&mut self, ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        if skin_status(ctx.shared) == SkinStatus::Ready && !self.keys_are_elsewhere(ctx) {
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
        let data = FrameData { bars: Some(&wheel), images, bga: BgaFrame::of(images.stagefile.or(images.banner)), ..FrameData::default() };
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
        let meta = focused.and_then(|index| Some(chart_under_cursor(ctx.shared.library.songs().get(index)?, self.facts.of(index), measured)));
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
