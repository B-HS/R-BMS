//! The song browser: the list and its detail panel, the hover preview, the record modal, the IR
//! ranking panel, and every way out of it (a chart, the managers, the settings screen, or quit).
//!
//! The list itself — which folder is open, the filtered rows and the cursor — lives in
//! [`AppShared`], because every other screen returns here and the debug overlay reports it from
//! anywhere. What this screen owns is what only makes sense while it is up.
//!
//! The two largest pieces of that live in their own files: [`preview`] drives the hover preview's
//! loading and playback, and [`scene`] assembles what the renderer draws.
#![allow(clippy::wildcard_imports)]

use std::sync::mpsc::TryRecvError;

use crate::ir_ranking_view::{PanelAction, PanelLine, offline_lines, panel_action, panel_lines};
use crate::ir_replay::from_ir_replay;
use crate::stage::{Canvas, FoldersState, FrameCtx, KeyInput, LoadingState, SettingsState, Stage, StageHandler, TablesState, Transition};
use crate::*;

mod filter;
mod list;
mod preview;
mod scene;
#[cfg(test)]
pub(super) mod tests;

use filter::{FilterKey, FilterPanel, render_filter_panel};
use list::SelectFilter;
use preview::PreviewState;

/// What the search box draws where the next character will go.
const SEARCH_CARET: &str = "_";

/// Roughly how many characters the search box shows at once, so a query too long for it scrolls
/// under the caret instead of hiding the end being edited. The renderer trims to the exact pixel
/// width on top of this; this only decides which stretch of the query it is handed.
const SEARCH_VISIBLE_CHARS: usize = 24;

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
    /// The focused chart's cover art, at the resolution the file itself was decoded at.
    cover_image: Option<crate::DecodedImage>,
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
            cached_scene: None,
            cached_key: None,
            shift_held: false,
            ctrl_held: false,
            search_edit: TextEdit::new(),
            search_resume: None,
            filter: FilterPanel::default(),
            applied: None,
            preview: PreviewState::default(),
        }
    }
}

impl SelectState {
    pub(crate) fn new() -> SelectState {
        SelectState::default()
    }

    /// Whether a second Esc/Left right now would quit: the arming press must still be inside
    /// [`crate::ROOT_ESC_CONFIRM`]. Read by both the guide line and the scene cache key, so the
    /// "press again" prompt disappears exactly when the window closes.
    fn esc_quit_armed(&self) -> bool {
        esc_confirms_quit(self.esc_quit_at, Instant::now())
    }

    fn select_key(&self, shared: &AppShared) -> SelectKey {
        (shared.select_gen, shared.sel, self.record_modal, shared.scores.records().len(), shared.config.display.score_graph, self.esc_quit_armed())
    }

    /// Enter the focused select item: descend into a folder, or start a chart.
    ///
    /// Any replay download still in flight is abandoned when a chart starts: its result would
    /// otherwise land mid-load and swap the chart out from under the run that is starting.
    fn select_enter(&mut self, shared: &mut AppShared) -> Transition {
        match shared.select_items.get(shared.sel) {
            Some(SelectItem::Song(i)) => {
                let i = *i;
                self.record_modal = None;
                shared.replay_download_rx = None;
                shared.replay_download_target = None;
                Transition::Open(Stage::Loading(LoadingState::song(i)))
            }
            Some(SelectItem::Folder { target, .. }) => {
                shared.select_view = *target;
                shared.sel = 0;
                self.rebuild(shared);
                Transition::Stay
            }
            None => Transition::Stay,
        }
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
    /// The cover (`#STAGEFILE`, then `#BANNER`) is decoded on the same schedule and uploaded into
    /// the single BGA slot when the browser draws, so it costs one decode per moved row too.
    fn refresh_focused_detail(&mut self, shared: &AppShared, now: Instant) {
        let si = shared.focused_song_index();
        if si != self.focus_settle_si {
            self.focus_settle_si = si;
            self.focus_settle_at = now;
        }
        if si == self.focused_detail_si || self.focus_settle_at.elapsed() < FOCUS_DETAIL_DEBOUNCE {
            return;
        }
        self.focused_detail_si = si;
        self.focused_detail = si.and_then(|i| shared.library.songs().get(i)).and_then(|e| compute_chart_detail(&e.path, e.mode));
        self.cover_image = si.and_then(|i| shared.library.songs().get(i)).and_then(|e| {
            let dir = e.path.parent()?;
            [&e.stagefile, &e.banner].into_iter().filter(|n| !n.trim().is_empty()).find_map(|n| decode_bga_image(dir, n))
        });
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
        let dir = shared.settings_path.parent().map(|d| d.join("replays")).unwrap_or_else(|| PathBuf::from("replays"));
        match Replay::load(&dir.join(&file)) {
            Ok(rp) => {
                self.record_modal = None;
                shared.chart_path = rp.chart_path.clone();
                shared.replay = Some(rp);
                self.stop_preview(shared);
                match shared.load() {
                    Some(loaded) => Transition::Open(shared.enter_loaded_chart(loaded)),
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
        self.ranking_sel = 0;
        if self.ranking_open {
            shared.ranking_requested = None;
        }
    }

    /// The lines the panel is showing for the focused chart.
    fn ranking_lines(&self, shared: &AppShared) -> Vec<PanelLine> {
        if shared.config.network.server_url.is_none() {
            return offline_lines();
        }
        match shared.focused_md5() {
            Some(md5) => panel_lines(shared.ranking_cache.peek(&md5)),
            None => panel_lines(None),
        }
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
        match action {
            PanelAction::Close => self.toggle_ranking_panel(shared),
            PanelAction::Up => self.ranking_sel = self.ranking_sel.saturating_sub(1),
            PanelAction::Down => self.ranking_sel = (self.ranking_sel + 1).min(lines.len().saturating_sub(1)),
            PanelAction::PlayReplay => self.play_ranking_replay(shared),
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
        shared.replay_download_rx = Some(rbms_ir::spawn_query(shared.server.clone(), move |server| server.download_replay(&replay_id)));
    }

    /// Start a ranking fetch when the focus has settled on a new chart and the panel has no cached
    /// answer for it. Called once per frame while the select screen is up.
    ///
    /// Only one fetch is ever in flight: a second one would drop the first receiver, stranding that
    /// chart's `Loading` placeholder in the cache and leaving the panel on LOADING forever. While a
    /// fetch runs the request for the newly focused chart is simply retried next frame.
    fn update_ranking(&mut self, shared: &mut AppShared) {
        if !self.ranking_open || shared.config.network.server_url.is_none() || shared.ranking_rx.is_some() {
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
        self.ranking_sel = 0;
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
                    Some(loaded) => Transition::Open(shared.enter_loaded_chart(loaded)),
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
        ctx.shared.searching || self.record_modal.is_some() || self.filter.is_open()
    }

    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let started = self.poll_replay_download(ctx.shared);
        if !matches!(started, Transition::Stay) {
            return started;
        }
        if self.applied != Some((ctx.shared.select_gen, self.filter.filter(&ctx.shared.config), ctx.shared.config.library.sort)) {
            self.rebuild(ctx.shared);
        }
        self.refresh_focused_detail(ctx.shared, ctx.now);
        self.update_preview(ctx.shared, ctx.now);
        self.update_ranking(ctx.shared);
        Transition::Stay
    }

    /// The preview owns the shared output stream's preview namespace, so it is torn down before any
    /// other screen can touch the engine.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        if self.preview_active() {
            self.stop_preview(ctx.shared);
        }
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
        match key.code {
            KeyCode::Escape | KeyCode::ArrowLeft => return self.select_escape(ctx.shared),
            KeyCode::Slash => self.start_search(ctx.shared),
            KeyCode::F3 if self.shift_held => self.cycle_sort_back(ctx.shared),
            KeyCode::F3 => self.cycle_sort(ctx.shared),
            KeyCode::F2 => self.toggle_filter_panel(),
            KeyCode::KeyF => ctx.shared.toggle_focused_favorite(),
            KeyCode::Tab => return Transition::Open(Stage::Settings(SettingsState::new())),
            KeyCode::KeyO => return Transition::Open(Stage::Folders(FoldersState::new())),
            KeyCode::KeyT => return Transition::Open(Stage::Tables(TablesState::new())),
            KeyCode::KeyR => self.open_record_modal(ctx.shared),
            KeyCode::KeyI => self.toggle_ranking_panel(ctx.shared),
            KeyCode::ArrowUp => {
                ctx.shared.sel = ctx.shared.sel.saturating_sub(1);
                ctx.shared.print_selection();
            }
            KeyCode::ArrowDown => {
                if ctx.shared.sel + 1 < ctx.shared.select_items.len() {
                    ctx.shared.sel += 1;
                }
                ctx.shared.print_selection();
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::ArrowRight => return self.select_enter(ctx.shared),
            _ => {}
        }
        Transition::Stay
    }

    /// Clicks on the browser. The bottom navigation buttons are the clickable equivalents of the
    /// keyboard shortcuts, and a click that misses every region closes an open modal.
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition {
        match ctx.shared.hit_test(at) {
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

    /// Paints the browser. The focused chart's cover goes into the single BGA slot, drawn behind the
    /// panel quads: the renderer leaves the cover square unfilled so the texture shows through.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        self.refresh_scene_cache(ctx.shared);
        let ranking_lines = if self.ranking_open { self.ranking_lines(ctx.shared) } else { Vec::new() };
        let Some(view) = self.cached_scene.as_ref() else {
            return;
        };
        ctx.shared.prepare_skin(canvas, SKIN_TYPE_MUSIC_SELECT);
        let mut document_background = None;
        match (&self.cover_image, &view.detail) {
            (Some(cover), SelectDetail::Song(_)) if !ctx.shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT) => {
                canvas.set_background(cover.generation, &cover.rgba, cover.width, cover.height, cover_rect());
            }
            (Some(cover), SelectDetail::Song(_)) => {
                canvas.clear_bga();
                document_background = canvas.background_texture(cover.generation, &cover.rgba, cover.width, cover.height);
            }
            _ => canvas.clear_bga(),
        }
        let now_ms = ctx.shared.skin_now_ms();
        let row = ctx.shared.sel;
        ctx.shared.skin_select_timers.update(&mut ctx.shared.skin_timers, row, now_ms);
        if ctx.shared.draw_select_skin(canvas, view, document_background) {
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
            let hot = render_ranking_panel(canvas, &ranking_lines, self.ranking_sel, true);
            ctx.shared.hot.extend(hot.into_iter().map(|(rect, index)| (rect, Hot::RankingRow(index))));
        }
        if self.filter.is_open() {
            render_filter_panel(canvas, &self.filter, &ctx.shared.config);
        }
    }
}
