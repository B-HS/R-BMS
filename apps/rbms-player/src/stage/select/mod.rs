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

mod preview;
mod scene;

use preview::PreviewState;

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
    cover_rgba: Option<Vec<u8>>,
    cached_scene: Option<SelectScene>,
    cached_key: Option<SelectKey>,
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
            cover_rgba: None,
            cached_scene: None,
            cached_key: None,
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
                shared.rebuild_select_items();
                Transition::Stay
            }
            None => Transition::Stay,
        }
    }

    /// Open the search box (`/`). Search spans the whole library, so switch to the flat song list.
    fn start_search(&mut self, shared: &mut AppShared) {
        shared.searching = true;
        shared.search.clear();
        shared.select_view = SelectView::AllSongs;
        shared.sel = 0;
        shared.rebuild_select_items();
    }

    /// Close the search box (Esc) and clear the filter.
    fn exit_search(&mut self, shared: &mut AppShared) {
        shared.searching = false;
        shared.search.clear();
        shared.sel = 0;
        shared.rebuild_select_items();
    }

    /// Cycle the song-list sort order (F3).
    fn cycle_sort(&mut self, shared: &mut AppShared) {
        shared.sort = shared.sort.next();
        shared.sel = 0;
        shared.rebuild_select_items();
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
        self.cover_rgba = si.and_then(|i| shared.library.songs().get(i)).and_then(|e| {
            let dir = e.path.parent()?;
            [&e.stagefile, &e.banner].into_iter().filter(|n| !n.trim().is_empty()).find_map(|n| decode_bga_256(dir, n))
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
                eprintln!("replay load failed: {e}");
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
                shared.rebuild_select_items();
            }
            SelectView::TableLevel(ti, _) => {
                shared.select_view = SelectView::TableLevels(ti);
                shared.sel = 0;
                shared.rebuild_select_items();
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
                shared.search.pop();
                shared.sel = 0;
                shared.rebuild_select_items();
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
                    let add: String = t.chars().filter(|c| !c.is_control()).collect();
                    if !add.is_empty() {
                        shared.search.push_str(&add);
                        shared.sel = 0;
                        shared.rebuild_select_items();
                    }
                }
            }
        }
        Transition::Stay
    }
}

impl StageHandler for SelectState {
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let started = self.poll_replay_download(ctx.shared);
        if !matches!(started, Transition::Stay) {
            return started;
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
        if !key.pressed {
            return Transition::Stay;
        }
        if self.record_modal.is_some() {
            return self.modal_key(ctx.shared, &key);
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
            KeyCode::F3 => self.cycle_sort(ctx.shared),
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
        match (&self.cover_rgba, &view.detail) {
            (Some(rgba), SelectDetail::Song(_)) => canvas.set_bga(rgba, cover_rect()),
            _ => canvas.clear_bga(),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_preview_clip_sits_at_the_base_of_the_preview_namespace() {
        assert_eq!(PREVIEW_ID, IdNamespace::PREVIEW.base);
        assert!(IdNamespace::PREVIEW.contains(PREVIEW_ID));
        assert!(!IdNamespace::PLAY.contains(PREVIEW_ID));
    }

    #[test]
    fn preview_keysounds_land_inside_the_preview_namespace() {
        for wav in [0, 1, 1_295, IdNamespace::PREVIEW.len - 1] {
            let id = SelectState::preview_sample_id(wav);
            assert!(IdNamespace::PREVIEW.contains(id), "wav {wav} escaped the preview namespace");
            assert!(!IdNamespace::PLAY.contains(id));
            assert_eq!(id, IdNamespace::PREVIEW.base + wav);
        }
    }

    #[test]
    fn an_out_of_range_wav_index_is_clamped_into_the_preview_namespace() {
        let clamped = SelectState::preview_sample_id(u32::MAX);
        assert!(IdNamespace::PREVIEW.contains(clamped));
        assert_eq!(clamped, IdNamespace::PREVIEW.base + IdNamespace::PREVIEW.len - 1);
    }

    #[test]
    fn a_chart_keysound_and_the_preview_copy_of_it_never_share_an_id() {
        for wav in [1u32, 36, 1_295] {
            assert_ne!(wav, SelectState::preview_sample_id(wav));
            assert!(IdNamespace::PLAY.contains(wav));
        }
    }
}
