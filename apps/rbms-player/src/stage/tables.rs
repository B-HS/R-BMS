//! The difficulty-table manager: the list of table sources, the URL/file pickers that add one, and
//! the removal that drops its matched levels with it.
//!
//! Neither way of adding one stops the app, or leaves this screen: the file picker is a native
//! panel the window keeps drawing behind, and the table itself — fetched from a URL or read off
//! disk — is fetched, parsed and matched against the library on a worker beside the frame loop
//! ([`register_table`]). The source is a row from the moment it is added, which says the table is
//! on its way until it has landed.
//!
//! Each source says what became of its table: how many of its charts the library has, that it has
//! not been fetched yet, that it is being fetched, or that it could not be loaded and why. A table
//! that has not arrived or that failed is not a folder in the song list, so this is the screen
//! that has to account for it.
//!
//! A table at an address is fetched once and read from the copy kept of it after that, so the row
//! also says how long ago that copy was fetched, and points its age out once it is old. Nothing
//! fetches a table again by itself: R does, for the table under the cursor, and A for every table.
//! Those run beside the screen too, which goes on being used while the row says the table is on
//! its way. A fetch goes on after the screen is left, and lands on the song list.
//!
//! The last row is the signed-in player's own folders ([`crate::user_folders`]), which come from
//! their account rather than from an address typed here: it says what became of them, R fetches
//! them again, A fetches them with every table, and it cannot be removed, the folders being edited
//! on the web site.
#![allow(clippy::wildcard_imports)]

use crate::dialog::{DialogHandle, DialogState};
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::tablesrc::{
    COPY_OLD_AFTER_DAYS, TableOrigin, TableStatus, apply_finished_fetches, copy_age, copy_age_days, forget_table, is_address, register_table, reload_tables,
    table_is_fetching, table_status, unix_now,
};
use crate::user_folders::{FolderProblem, FolderStatus, USER_FOLDERS_LABEL};
use crate::*;

/// Width of the table list panel.
const PANEL_W: f32 = 900.0;

/// How much of a source location the row shows before it is cut.
const LOCATION_CHARS: usize = 64;

/// How much of the typed URL the entry box shows around the caret.
const URL_ENTRY_CHARS: usize = 70;

/// Text scale of the URL entry box, which the caret is placed by measuring.
const URL_ENTRY_SCALE: f32 = 1.6;

/// Width of the caret drawn in the URL entry box.
const CARET_W: f32 = 2.0;

/// Height of the caret drawn in the URL entry box.
const CARET_H: f32 = 20.0;

/// What a picked file is called when its name is not usable as one.
const DEFAULT_TABLE_NAME: &str = "table";

/// Vertical layout of the table rows.
const ROW_TOP: f32 = 124.0;
const ROW_PITCH: f32 = 44.0;
const ROW_H: f32 = 36.0;

/// Inset of a row's text from the panel's edges, and how far below the row's top it sits.
const ROW_PAD_X: f32 = 16.0;
const ROW_TEXT_DY: f32 = 10.0;
const ROW_TEXT_SCALE: f32 = 1.5;

/// The status at the right edge of a source's row: its text scale, how far below the row's top it
/// sits, and the room kept between it and the source's name.
const STATUS_SCALE: f32 = 1.2;
const STATUS_DY: f32 = 12.0;
const STATUS_GAP: f32 = 24.0;

/// The room between the age of a copy and the rest of the status it stands at the end of.
const SAVED_GAP: f32 = 18.0;

/// The keys of the list, under the title.
const LIST_KEYS: &str = "UP DOWN MOVE   ENTER SELECT   R RELOAD   A RELOAD ALL   D REMOVE   ESC BACK";

/// What the row of the player's own folders says while nobody is signed in, and under the list.
const FOLDERS_SIGNED_OUT: &str = "LOG IN TO LOAD YOUR FOLDERS";
const FOLDERS_SIGNED_OUT_DETAIL: &str = "LOG IN ON THE NETWORK TAB OF THE SETTINGS - YOUR FOLDERS ARE MADE AND EDITED ON THE WEB SITE";

/// What the row of the player's own folders says when the server has none to give.
const FOLDERS_NONE: &str = "THE SERVER HAS NO FOLDERS";

/// What is said under the list of the player's own folders when there is nothing wrong with them,
/// and what D says of them: they are not this screen's to remove.
const FOLDERS_DETAIL: &str = "YOUR FOLDERS ARE MADE AND EDITED ON THE WEB SITE - R FETCHES THEM AGAIN";
const FOLDERS_NOT_REMOVED: &str = "your folders are edited on the web site and cannot be removed here";

/// What the row of a table that is being fetched again says in place of its status, and what is
/// said under the list of the table under the cursor while it is: whatever its last reload came
/// to is not what is happening to it now.
const RELOADING: &str = "RELOADING...";
const RELOADING_DETAIL: &str = "FETCHING THE TABLE AGAIN - IT STAYS AS IT IS UNTIL THE NEW ONE ARRIVES";

/// The same two for a table that is on its way and is not in the song list yet: one at an address
/// that has never arrived, and one that is a file being read.
const FETCHING: &str = "FETCHING...";
const FETCHING_DETAIL: &str = "FETCHING THE TABLE - ITS FOLDER APPEARS IN THE SONG LIST WHEN IT ARRIVES";
const READING: &str = "READING...";
const READING_DETAIL: &str = "READING THE TABLE FILE - ITS FOLDER APPEARS IN THE SONG LIST WHEN IT HAS BEEN READ";

/// What the row of a table at an address says when nothing has been fetched of it and nothing is
/// fetching it, and what is said of it under the list.
const NOT_FETCHED: &str = "NOT FETCHED YET";
const NOT_FETCHED_DETAIL: &str = "NOT FETCHED YET - THE SONG LIST FETCHES IT WHEN IT IS NEXT UP, AND R FETCHES IT NOW";

/// What is said of a copy before how long ago it was fetched, and all that is said of one that
/// does not record when that was.
const SAVED_UNDATED: &str = "SAVED";

/// What reloading is, along the bottom of the screen: where the lines end above the bottom edge,
/// their pitch and their scale.
const RELOAD_NOTE: [&str; 3] = [
    "A TABLE AT AN ADDRESS IS FETCHED ONCE AND READ FROM ITS SAVED COPY AFTER THAT - THE ROW SHOWS HOW LONG AGO THE COPY WAS FETCHED.",
    "A TABLE IS FETCHED WHILE YOU GO ON USING THE PLAYER, HERE OR IN THE SONG LIST - ITS FOLDER APPEARS WHEN IT ARRIVES.",
    "R FETCHES THE TABLE UNDER THE CURSOR AGAIN, A EVERY TABLE. A TABLE REGISTERED AS A FILE IS READ FROM THE FILE AGAIN.",
];
const NOTE_BOTTOM: f32 = 40.0;
const NOTE_PITCH: f32 = 20.0;
const NOTE_SCALE: f32 = 1.1;

/// The lines under the list that spell out the focused source's status: where they start below the
/// last row, their pitch and scale, and how much of a long reason they carry.
const DETAIL_GAP: f32 = 12.0;
const DETAIL_PITCH: f32 = 22.0;
const DETAIL_SCALE: f32 = 1.2;
const DETAIL_LINE_CHARS: usize = 96;
const DETAIL_MAX_LINES: usize = 3;

/// What a source's row shows of it: the name the user gave it or, failing that, the name its table
/// gave itself, then where it is loaded from.
fn source_label(source: &TableSource, status: Option<&TableStatus>) -> String {
    let own = source.name.trim();
    let name = match status {
        Some(TableStatus::Loaded { name, .. }) if own.is_empty() => name.as_str(),
        _ => own,
    };
    let location: String = source.location.chars().take(LOCATION_CHARS).collect();
    if name.is_empty() { location } else { format!("{name}  -  {location}") }
}

/// The few words at the right edge of a source's row, and whether they report a problem.
fn status_summary(status: &TableStatus) -> (String, bool) {
    match status {
        TableStatus::Loaded { entries: 0, stale: None, .. } => ("NO CHARTS LISTED".to_string(), true),
        TableStatus::Loaded { entries, levels, owned, stale: None, .. } => (format!("{owned} OF {entries} CHARTS OWNED  -  {levels} LEVELS"), false),
        TableStatus::Loaded { entries, owned, stale: Some(_), .. } => (format!("RELOAD FAILED  -  {owned} OF {entries} CHARTS OWNED"), true),
        TableStatus::Failed { .. } => ("LOAD FAILED".to_string(), true),
        TableStatus::NotFetched => (NOT_FETCHED.to_string(), false),
    }
}

/// What there is to say about a source beyond its row, when there is anything: why it failed, why
/// its reload left it as it was, that it lists nothing, or that none of its charts are in the
/// library.
fn status_detail(status: &TableStatus) -> Option<String> {
    match status {
        TableStatus::Failed { reason } => Some(format!("LOAD FAILED: {reason}")),
        TableStatus::Loaded { stale: Some(reason), .. } => Some(format!("RELOAD FAILED, THE TABLE IS AS IT WAS: {reason}")),
        TableStatus::Loaded { entries: 0, .. } => Some("THE TABLE WAS READ BUT LISTS NO CHARTS - IT HAS NO FOLDER IN THE SONG LIST".to_string()),
        TableStatus::Loaded { owned: 0, .. } => Some("NONE OF THIS TABLE'S CHARTS ARE IN THE LIBRARY YET - ITS FOLDER IS EMPTY".to_string()),
        TableStatus::Loaded { .. } => None,
        TableStatus::NotFetched => Some(NOT_FETCHED_DETAIL.to_string()),
    }
}

/// How long ago the copy of a table was fetched, as the end of its row says it, and whether the
/// copy is old enough for that to be pointed out. `None` for a table that has no copy: one read
/// from a file, or one that did not load.
fn saved_note(status: &TableStatus, now: u64) -> Option<(String, bool)> {
    match status {
        TableStatus::Loaded { origin: TableOrigin::Address { fetched_at: Some(at) }, .. } => {
            Some((format!("{SAVED_UNDATED} {}", copy_age(*at, now)), copy_age_days(*at, now) > COPY_OLD_AFTER_DAYS))
        }
        TableStatus::Loaded { origin: TableOrigin::Address { fetched_at: None }, .. } => Some((SAVED_UNDATED.to_string(), false)),
        TableStatus::Loaded { origin: TableOrigin::File, .. } | TableStatus::Failed { .. } | TableStatus::NotFetched => None,
    }
}

/// What a table that is on its way says: the few words of its row, and the line under the list.
///
/// A table that is in the song list is being fetched again and stays as it is meanwhile. One that
/// is not -- never fetched, or failed the last time -- has no folder until this fetch lands, and
/// one registered as a file is being read rather than fetched.
fn on_its_way(source: &TableSource, status: Option<&TableStatus>) -> (&'static str, &'static str) {
    match status {
        Some(TableStatus::Loaded { .. }) => (RELOADING, RELOADING_DETAIL),
        _ if is_address(&source.location) => (FETCHING, FETCHING_DETAIL),
        _ => (READING, READING_DETAIL),
    }
}

/// What is said under the list of the source under the cursor, and whether it reports a problem:
/// that the table is on its way while it is (`arriving`, the line [`on_its_way`] gives it), and
/// otherwise what there is to say of what became of it ([`status_detail`]). A fetch that is out
/// says nothing of the one that failed before it, which is not what is happening to the table now.
fn focused_detail(status: Option<&TableStatus>, arriving: Option<&str>) -> Option<(String, bool)> {
    if let Some(arriving) = arriving {
        return Some((arriving.to_string(), false));
    }
    let status = status?;
    let is_problem = !matches!(status, TableStatus::Loaded { stale: None, .. } | TableStatus::NotFetched);
    status_detail(status).map(|detail| (detail, is_problem))
}

/// The line that says how old an old copy is and how to replace it, for the source under the
/// cursor.
fn old_copy_note(status: &TableStatus, now: u64) -> Option<String> {
    let TableStatus::Loaded { origin: TableOrigin::Address { fetched_at: Some(at) }, .. } = status else {
        return None;
    };
    let days = copy_age_days(*at, now);
    (days > COPY_OLD_AFTER_DAYS).then(|| format!("THE SAVED COPY IS {days} DAYS OLD - R FETCHES THE TABLE AGAIN"))
}

/// The few words at the right edge of the row of the player's own folders, and whether they report
/// a problem.
fn folders_summary(status: &FolderStatus) -> (String, bool) {
    match status {
        FolderStatus::SignedOut => (FOLDERS_SIGNED_OUT.to_string(), false),
        FolderStatus::Fetching => (FETCHING.to_string(), false),
        FolderStatus::Shown { folders, listed, owned, problem: None, .. } => (format!("{owned} OF {listed} CHARTS OWNED  -  {folders} FOLDERS"), false),
        FolderStatus::Shown { listed, owned, problem: Some(_), .. } => (format!("RELOAD FAILED  -  {owned} OF {listed} CHARTS OWNED"), true),
        FolderStatus::NotFetched { problem: None } => (NOT_FETCHED.to_string(), false),
        FolderStatus::NotFetched { problem: Some(FolderProblem::NoFolders) } => (FOLDERS_NONE.to_string(), false),
        FolderStatus::NotFetched { problem: Some(FolderProblem::Failed(_)) } => ("LOAD FAILED".to_string(), true),
    }
}

/// What is said under the list of the player's own folders, and whether it reports a problem.
fn folders_detail(status: &FolderStatus) -> (String, bool) {
    match status {
        FolderStatus::SignedOut => (FOLDERS_SIGNED_OUT_DETAIL.to_string(), false),
        FolderStatus::Shown { problem: Some(FolderProblem::Failed(reason)), .. } => (format!("RELOAD FAILED, THE FOLDERS ARE AS THEY WERE: {reason}"), true),
        FolderStatus::NotFetched { problem: Some(FolderProblem::Failed(reason)) } => (format!("LOAD FAILED: {reason}"), true),
        FolderStatus::Fetching | FolderStatus::Shown { .. } | FolderStatus::NotFetched { .. } => (FOLDERS_DETAIL.to_string(), false),
    }
}

/// How long ago the player's own folders were fetched, as the end of their row says it.
fn folders_saved_note(status: &FolderStatus, now: u64) -> Option<String> {
    match status {
        FolderStatus::Shown { fetched_at: Some(at), .. } => Some(format!("{SAVED_UNDATED} {}", copy_age(*at, now))),
        FolderStatus::Shown { fetched_at: None, .. } => Some(SAVED_UNDATED.to_string()),
        FolderStatus::SignedOut | FolderStatus::Fetching | FolderStatus::NotFetched { .. } => None,
    }
}

/// The table manager's own state: which row is focused, the URL being typed when the "+ ADD TABLE
/// (URL)" row has been entered, the file picker while it is up, and whether a paste modifier is
/// held.
#[derive(Default)]
pub(crate) struct TablesState {
    sel: usize,
    text_input: Option<TextEdit>,
    picker: Option<DialogHandle>,
    /// Control or Command held, which is what turns V into a paste rather than a typed letter.
    paste_held: bool,
}

impl TablesState {
    pub(crate) fn new() -> TablesState {
        TablesState::default()
    }

    /// Rows in the table manager: each source, the two add actions, then the player's own folders.
    fn row_count(shared: &AppShared) -> usize {
        shared.config.library.tables.len() + 3
    }

    /// The row of the player's own folders, the last one.
    fn folders_row(shared: &AppShared) -> usize {
        shared.config.library.tables.len() + 2
    }

    /// Add `source` to the list and start fetching its table, without leaving the screen: the row
    /// is there at once with the cursor on it, and says the table is on its way until it has
    /// landed. A source already in the list is not added twice, whether its table has arrived or is
    /// still out.
    fn add_source(&mut self, shared: &mut AppShared, source: TableSource) {
        if shared.config.library.tables.iter().any(|t| t.location == source.location) {
            notify(Level::Warn, format!("table already added: {}", source.location));
            return;
        }
        self.sel = shared.config.library.tables.len();
        register_table(shared, source);
    }

    /// Add the table the file picker came back with.
    ///
    /// A table file is not always small — an insane-level table is tens of thousands of entries, and
    /// each is matched against every chart in the library — so it is parsed and matched on a worker
    /// rather than in the middle of a frame, the same worker a table at an address is fetched on.
    fn add_picked_file(&mut self, shared: &mut AppShared, path: &std::path::Path) {
        let location = path.to_string_lossy().to_string();
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or(DEFAULT_TABLE_NAME).to_string();
        self.add_source(shared, TableSource { name, location });
    }

    /// Take a source off the list, with everything kept of its table. One whose table is still on
    /// its way is removed like any other: what arrives for it afterwards is kept nowhere.
    fn remove_table_source(&mut self, shared: &mut AppShared, idx: usize) {
        if idx < shared.config.library.tables.len() {
            let removed = shared.config.library.tables.remove(idx);
            forget_table(shared, &removed.location);
            if idx < shared.table_names.len() {
                shared.table_names.remove(idx);
            }
            if idx < shared.table_levels.len() {
                shared.table_levels.remove(idx);
            }
            shared.save_settings();
            self.sel = self.sel.min(TablesState::row_count(shared).saturating_sub(1));
        }
    }

    /// Add the typed URL as a source and start fetching it. The request is out beside the screen,
    /// which shows the list again with the new row in it.
    fn add_typed_url(&mut self, shared: &mut AppShared) {
        let url = self.text_input.take().map(|mut edit| edit.take()).unwrap_or_default().trim().to_string();
        if !url.is_empty() {
            self.add_source(shared, TableSource { name: String::new(), location: url });
        }
    }

    /// Typing the URL of a new table. Enter adds it, Esc abandons the entry, and the caret keys
    /// edit what has been typed so far rather than only its end. None of them leaves the screen.
    fn url_entry_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) -> Transition {
        if key.code == KeyCode::Enter || key.code == KeyCode::NumpadEnter {
            self.add_typed_url(shared);
            return Transition::Stay;
        }
        if key.code == KeyCode::Escape {
            self.text_input = None;
            return Transition::Stay;
        }
        let Some(edit) = self.text_input.as_mut() else {
            return Transition::Stay;
        };
        edit_key(edit, key, self.paste_held);
        Transition::Stay
    }

    /// The URL entry box, with the stretch of a long URL the caret is in and the caret itself.
    fn draw_url_entry(&self, canvas: &mut Canvas<'_>, edit: &TextEdit, x0: f32) {
        let th = rbms_render::theme();
        draw_text(canvas, x0, 84.0, 1.4, th.accent, "TYPE TABLE URL  -  ENTER ADD  ESC CANCEL  CTRL+V PASTE");
        canvas.fill_rect(Rect::new(x0, 116.0, PANEL_W, 40.0), th.button_active);
        let (shown, caret) = edit.window(URL_ENTRY_CHARS);
        let text_x = x0 + 14.0;
        draw_text(canvas, text_x, 128.0, URL_ENTRY_SCALE, th.text, &shown);
        let before: String = shown.chars().take(caret).collect();
        canvas.fill_rect(Rect::new(text_x + text_width(&before, URL_ENTRY_SCALE), 126.0, CARET_W, CARET_H), th.focus);
    }
}

#[cfg(test)]
impl TablesState {
    /// Open the URL entry on `url`, so a render snapshot can be taken of it being typed.
    pub(super) fn begin_url_entry_for_tests(&mut self, url: &str) {
        self.sel = 0;
        self.text_input = Some(TextEdit::from_text(url));
    }

    /// Put the caret at the start of what is being typed.
    pub(super) fn move_caret_home_for_tests(&mut self) {
        if let Some(edit) = self.text_input.as_mut() {
            edit.home();
        }
    }
}

impl StageHandler for TablesState {
    /// Take the tables a fetch has come back with, and collect the file picker's answer once the
    /// user has given one. Both run beside the frame loop, so this is where a reloaded table and an
    /// added one actually land.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        apply_finished_fetches(ctx.shared);
        crate::user_folders::apply_finished_fetch(ctx.shared);
        let Some(picker) = self.picker.as_ref() else {
            return Transition::Stay;
        };
        match picker.poll() {
            DialogState::Open => {}
            DialogState::Picked(path) => {
                self.picker = None;
                self.add_picked_file(ctx.shared, &path);
            }
            DialogState::Dismissed => self.picker = None,
        }
        Transition::Stay
    }

    /// Table-manager input. In URL-text mode, type the URL (Enter adds, Esc cancels); otherwise
    /// navigate, add (URL/file), reload (R the table under the cursor, A every table), remove (D),
    /// or leave (Esc — rebuilds the browse list). Keys are ignored while the native picker is up,
    /// which owns the keyboard itself. A fetch goes on after the screen is left.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if matches!(key.code, KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::SuperLeft | KeyCode::SuperRight) {
            self.paste_held = key.pressed;
            return Transition::Stay;
        }
        if !key.pressed || self.picker.is_some() {
            return Transition::Stay;
        }
        if self.text_input.is_some() {
            return self.url_entry_key(ctx.shared, &key);
        }
        let n = TablesState::row_count(ctx.shared);
        let add_url = ctx.shared.config.library.tables.len();
        let add_file = add_url + 1;
        match key.code {
            KeyCode::Escape => {
                ctx.shared.select_view = SelectView::Root;
                ctx.shared.sel = 0;
                ctx.shared.rebuild_select_items();
                return Transition::Back;
            }
            KeyCode::ArrowUp => self.sel = self.sel.saturating_sub(1),
            KeyCode::ArrowDown => self.sel = (self.sel + 1).min(n.saturating_sub(1)),
            KeyCode::KeyD | KeyCode::Delete if self.sel < ctx.shared.config.library.tables.len() => self.remove_table_source(ctx.shared, self.sel),
            KeyCode::KeyD | KeyCode::Delete if self.sel == TablesState::folders_row(ctx.shared) => notify(Level::Info, FOLDERS_NOT_REMOVED),
            KeyCode::KeyR if self.sel < ctx.shared.config.library.tables.len() => reload_tables(ctx.shared, Some(self.sel)),
            KeyCode::KeyR if self.sel == TablesState::folders_row(ctx.shared) => {
                crate::user_folders::start_fetch(ctx.shared, true);
            }
            KeyCode::KeyA => {
                reload_tables(ctx.shared, None);
                crate::user_folders::start_fetch(ctx.shared, true);
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if self.sel == add_url {
                    self.text_input = Some(TextEdit::new());
                } else if self.sel == add_file {
                    self.picker = Some(crate::dialog::pick_table_file());
                }
            }
            _ => {}
        }
        Transition::Stay
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        canvas.clear(th.bg);
        let x0 = (CW as f32 - PANEL_W) * 0.5;
        draw_text(canvas, x0, 40.0, 3.0, th.text, "DIFFICULTY TABLES");
        if let Some(edit) = self.text_input.as_ref() {
            self.draw_url_entry(canvas, edit, x0);
            return;
        }
        if self.picker.is_some() {
            draw_text(canvas, x0, 84.0, 1.4, th.accent, "PICK A TABLE FILE IN THE OPEN DIALOG");
            return;
        }
        draw_text(canvas, x0, 84.0, 1.3, th.text_muted, LIST_KEYS);
        let now = unix_now();
        let shared = &*ctx.shared;
        let tables = &shared.config.library.tables;
        let add_url = tables.len();
        let folders_row = TablesState::folders_row(shared);
        let folders = shared.user_folders.status();
        let row_count = TablesState::row_count(shared);
        for i in 0..row_count {
            let y = ROW_TOP + i as f32 * ROW_PITCH;
            let on = i == self.sel;
            let status = tables.get(i).and_then(|source| table_status(&source.location));
            let label = match tables.get(i) {
                Some(source) => source_label(source, status.as_ref()),
                None if i == add_url => "+ ADD TABLE (URL)".to_string(),
                None if i == folders_row => USER_FOLDERS_LABEL.to_string(),
                None => "+ ADD TABLE (FILE)".to_string(),
            };
            canvas.fill_rect(Rect::new(x0, y, PANEL_W, ROW_H), if on { th.button } else { th.panel });
            let col = if i >= tables.len() && i != folders_row {
                th.accent
            } else if on {
                th.text
            } else {
                th.text_dim
            };
            let mut status_right = x0 + PANEL_W - ROW_PAD_X;
            if i == folders_row {
                if let Some(saved) = folders_saved_note(&folders, now) {
                    draw_text_right(canvas, status_right, y + STATUS_DY, STATUS_SCALE, if on { th.text } else { th.text_muted }, &saved);
                    status_right -= text_width(&saved, STATUS_SCALE) + SAVED_GAP;
                }
                let (summary, is_problem) = folders_summary(&folders);
                let status_col = match (folders == FolderStatus::Fetching, is_problem, on) {
                    (true, ..) => th.accent,
                    (false, true, _) => th.warn,
                    (false, false, true) => th.text,
                    (false, false, false) => th.text_muted,
                };
                draw_text_right(canvas, status_right, y + STATUS_DY, STATUS_SCALE, status_col, &summary);
                status_right -= text_width(&summary, STATUS_SCALE) + STATUS_GAP;
            }
            if let Some((saved, is_old)) = status.as_ref().and_then(|status| saved_note(status, now)) {
                let saved_col = match (is_old, on) {
                    (true, _) => th.warn,
                    (false, true) => th.text,
                    (false, false) => th.text_muted,
                };
                draw_text_right(canvas, status_right, y + STATUS_DY, STATUS_SCALE, saved_col, &saved);
                status_right -= text_width(&saved, STATUS_SCALE) + SAVED_GAP;
            }
            let arriving = tables.get(i).filter(|source| table_is_fetching(shared, &source.location)).map(|source| on_its_way(source, status.as_ref()).0);
            let summary = arriving.map(|arriving| (arriving.to_string(), false)).or_else(|| status.as_ref().map(status_summary));
            if let Some((summary, is_problem)) = summary {
                let status_col = match (arriving.is_some(), is_problem, on) {
                    (true, ..) => th.accent,
                    (false, true, _) => th.warn,
                    (false, false, true) => th.text,
                    (false, false, false) => th.text_muted,
                };
                draw_text_right(canvas, status_right, y + STATUS_DY, STATUS_SCALE, status_col, &summary);
                status_right -= text_width(&summary, STATUS_SCALE) + STATUS_GAP;
            }
            let label_w = status_right - (x0 + ROW_PAD_X);
            draw_text(canvas, x0 + ROW_PAD_X, y + ROW_TEXT_DY, ROW_TEXT_SCALE, col, &rbms_render::fit_text(&label, ROW_TEXT_SCALE, label_w));
        }
        let focused = tables.get(self.sel).and_then(|source| table_status(&source.location));
        let focused_arriving =
            tables.get(self.sel).filter(|source| table_is_fetching(shared, &source.location)).map(|source| on_its_way(source, focused.as_ref()).1);
        let mut line_top = ROW_TOP + row_count as f32 * ROW_PITCH + DETAIL_GAP;
        let detail = if self.sel == folders_row { Some(folders_detail(&folders)) } else { focused_detail(focused.as_ref(), focused_arriving) };
        if let Some((detail, is_problem)) = detail {
            let detail: Vec<char> = detail.chars().collect();
            for chars in detail.chunks(DETAIL_LINE_CHARS).take(DETAIL_MAX_LINES) {
                let text: String = chars.iter().collect();
                let fitted = rbms_render::fit_text(&text, DETAIL_SCALE, PANEL_W);
                draw_text(canvas, x0, line_top, DETAIL_SCALE, if is_problem { th.warn } else { th.text_muted }, &fitted);
                line_top += DETAIL_PITCH;
            }
        }
        if let Some(note) = focused.as_ref().filter(|_| focused_arriving.is_none()).and_then(|status| old_copy_note(status, now)) {
            draw_text(canvas, x0, line_top, DETAIL_SCALE, th.warn, &rbms_render::fit_text(&note, DETAIL_SCALE, PANEL_W));
        }
        let note_top = CH as f32 - NOTE_BOTTOM - NOTE_PITCH * (RELOAD_NOTE.len() - 1) as f32;
        for (line, note) in RELOAD_NOTE.iter().enumerate() {
            draw_text(canvas, x0, note_top + line as f32 * NOTE_PITCH, NOTE_SCALE, th.text_muted, &rbms_render::fit_text(note, NOTE_SCALE, PANEL_W));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::render_tests::app;
    use crate::tablesrc::{TableStore, forget_table_status, read_and_match};

    fn press(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text: None }
    }

    fn typed(text: &str) -> KeyInput<'_> {
        KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) }
    }

    /// A typed URL is a row of the list from the moment Enter is pressed, with the cursor on it,
    /// and the table is fetched beside the screen: nothing leaves for another screen, and nothing
    /// waits for the request. The source is in the settings whatever becomes of its table.
    #[test]
    fn adding_a_url_puts_its_row_in_the_list_at_once_and_fetches_beside_the_screen() {
        crate::notify::exclusive(|| {
            let address = "https://tables.test/manager/typed/";
            let mut app = app();
            let mut addresses = Addresses::of(&app);
            addresses.tables.serve(address, "Typed", TWO_LEVELS);
            addresses.registered.push(address.to_string());
            let mut state = TablesState::new();
            state.sel = TablesState::row_count(&app.shared) - 1;
            state.text_input = Some(TextEdit::from_text(address));

            let open = addresses.tables.hold();
            assert!(matches!(state.url_entry_key(&mut app.shared, &press(KeyCode::Enter)), Transition::Stay), "adding a table left the screen");
            assert!(state.text_input.is_none(), "the entry is finished with");
            assert_eq!(app.shared.config.library.tables.len(), 1, "the row is not there until the table has arrived");
            assert_eq!(state.sel, 0, "the cursor is not on the row that was added");
            assert!(table_is_fetching(&app.shared, address), "nothing is fetching the table that was added");
            assert_eq!(table_status(address), Some(TableStatus::NotFetched));
            assert_eq!((app.shared.table_names.len(), app.shared.table_levels.len()), (1, 1), "the row has no slot in the song list's tables");
            assert_eq!(folders(&mut app, SelectView::Root).len(), 1, "a table that has not arrived is a folder already");
            let kept = rbms_config::load(&app.shared.settings_path).expect("the settings were saved");
            assert_eq!(kept.config.library.tables.iter().map(|source| source.location.as_str()).collect::<Vec<_>>(), vec![address]);
            assert!(matches!(state.update(&mut ctx(&mut app)), Transition::Stay), "the screen stopped while the table was out");

            drop(open);
            addresses.wait_for_fetches(&mut app, &mut state);
            assert_eq!(addresses.tables.asked(), 1);
            assert_eq!(table_status(address).as_ref().map(status_summary), Some(("0 OF 2 CHARTS OWNED  -  2 LEVELS".to_string(), false)));
            assert!(addresses.store().has_copy_file(address), "a table that was registered was not kept");
            assert_eq!(folders(&mut app, SelectView::Root).len(), 2, "the table that arrived is not a folder");
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.iter().any(|(level, message)| *level == Level::Info && message == "table added: TYPED (0 local charts)"), "{said:?}");
        });
    }

    /// A table whose address does not answer keeps its row, which says why, and nothing announces
    /// it as added. It is not fetched again by itself, and R fetches it again.
    #[test]
    fn a_registered_address_that_does_not_answer_keeps_its_row_and_says_why() {
        crate::notify::exclusive(|| {
            let address = "https://tables.test/manager/unanswered/";
            let mut app = app();
            let mut addresses = Addresses::of(&app);
            addresses.register(&mut app, address);

            assert_eq!(app.shared.config.library.tables.len(), 1, "the row is there to look at, fetch again and remove");
            let status = table_status(address).expect("the failure is remembered");
            assert_eq!(status_summary(&status), ("LOAD FAILED".to_string(), true));
            assert!(status_detail(&status).is_some_and(|detail| detail.starts_with("LOAD FAILED: ")), "the row does not say why");
            assert!(!addresses.store().has_copy_file(address));
            assert_eq!(folders(&mut app, SelectView::Root).len(), 1, "a table that was not fetched is a folder");
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.iter().all(|(_, message)| !message.contains("table added")), "{said:?}");
            assert_eq!(
                said.iter().filter(|(level, message)| *level == Level::Warn && message.starts_with(&format!("table load failed ({address}): "))).count(),
                1,
                "{said:?}"
            );

            let mut state = TablesState::new();
            for _ in 0..3 {
                state.update(&mut ctx(&mut app));
            }
            assert_eq!(addresses.tables.asked(), 1, "a table whose fetch failed was fetched again by itself");

            addresses.tables.serve(address, "Answered", ONE_LEVEL);
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
            addresses.wait_for_fetches(&mut app, &mut state);
            assert_eq!(addresses.tables.asked(), 2);
            assert_eq!(table_status(address).as_ref().map(status_summary), Some(("0 OF 1 CHARTS OWNED  -  1 LEVELS".to_string(), false)));
            assert!(addresses.store().has_copy_file(address));
        });
    }

    /// While a table that was registered is still out: registering its address again is refused,
    /// and the keys that fetch tables again do not fetch it a second time.
    #[test]
    fn a_table_on_its_way_is_not_registered_or_fetched_a_second_time() {
        crate::notify::exclusive(|| {
            let address = "https://tables.test/manager/once/";
            let mut app = app();
            let mut addresses = Addresses::of(&app);
            addresses.tables.serve(address, "Once", ONE_LEVEL);
            addresses.registered.push(address.to_string());
            let mut state = TablesState::new();

            let open = addresses.tables.hold();
            state.text_input = Some(TextEdit::from_text(address));
            state.url_entry_key(&mut app.shared, &press(KeyCode::Enter));
            state.text_input = Some(TextEdit::from_text(address));
            state.url_entry_key(&mut app.shared, &press(KeyCode::Enter));
            assert_eq!(app.shared.config.library.tables.len(), 1, "the address of a table on its way was registered again");
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyA));
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.iter().any(|(level, message)| *level == Level::Warn && message == &format!("table already added: {address}")), "{said:?}");
            assert_eq!(said.iter().filter(|(_, message)| message == "the tables are being fetched already").count(), 2, "{said:?}");

            drop(open);
            addresses.wait_for_fetches(&mut app, &mut state);
            assert_eq!(addresses.tables.asked(), 1, "a table on its way was fetched a second time");
            assert_eq!(app.shared.table_names, vec!["ONCE".to_string()]);
        });
    }

    /// A table removed while it is still on its way from being registered leaves nothing: no row,
    /// no copy, and no word that it was added. Registered again before that fetch is back, it is
    /// fetched for itself and is there when it lands.
    #[test]
    fn a_table_removed_while_it_is_being_registered_leaves_nothing_and_can_be_registered_again() {
        crate::notify::exclusive(|| {
            let address = "https://tables.test/manager/removed-while-registering/";
            let mut app = app();
            let mut addresses = Addresses::of(&app);
            addresses.tables.serve(address, "Going", ONE_LEVEL);
            addresses.registered.push(address.to_string());
            let mut state = TablesState::new();

            let open = addresses.tables.hold();
            state.text_input = Some(TextEdit::from_text(address));
            state.url_entry_key(&mut app.shared, &press(KeyCode::Enter));
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyD));
            assert!(app.shared.config.library.tables.is_empty() && app.shared.table_names.is_empty() && app.shared.table_levels.is_empty());
            assert_eq!(table_status(address), None);
            drop(open);
            addresses.wait_for_fetches(&mut app, &mut state);
            assert!(!addresses.store().has_copy_file(address), "the fetch left a copy of a table that is no longer registered");
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.iter().all(|(_, message)| !message.contains("table added")), "{said:?}");

            let open = addresses.tables.hold();
            state.text_input = Some(TextEdit::from_text(address));
            state.url_entry_key(&mut app.shared, &press(KeyCode::Enter));
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyD));
            state.text_input = Some(TextEdit::from_text(address));
            state.url_entry_key(&mut app.shared, &press(KeyCode::Enter));
            drop(open);
            addresses.wait_for_fetches(&mut app, &mut state);
            assert_eq!(app.shared.table_names, vec!["GOING".to_string()], "the table registered again never arrived");
            assert!(addresses.store().has_copy_file(address), "the table registered again has no copy");
            assert_eq!(addresses.tables.asked(), 3, "each registration is one fetch of its own");
            crate::notify::drain(&mut said);
            assert_eq!(said.iter().filter(|(_, message)| message.starts_with("table added:")).count(), 1, "{said:?}");
        });
    }

    #[test]
    fn an_empty_or_repeated_url_is_refused_without_leaving_the_screen() {
        let mut app = app();
        app.shared.config.library.tables.push(TableSource { name: "have".into(), location: "https://example.test/have.json".into() });
        let mut state = TablesState::new();

        state.text_input = Some(TextEdit::from_text("   "));
        assert!(matches!(state.url_entry_key(&mut app.shared, &press(KeyCode::Enter)), Transition::Stay));
        assert_eq!(app.shared.config.library.tables.len(), 1, "an empty entry was added as a table");

        state.text_input = Some(TextEdit::from_text("https://example.test/have.json"));
        assert!(matches!(state.url_entry_key(&mut app.shared, &press(KeyCode::Enter)), Transition::Stay));
        assert_eq!(app.shared.config.library.tables.len(), 1, "the same table is not added twice");
        assert!(app.shared.table_fetches.is_idle(), "a table that was refused is being fetched");
    }

    #[test]
    fn escape_abandons_what_was_typed_without_adding_it() {
        let mut app = app();
        let mut state = TablesState::new();
        state.text_input = Some(TextEdit::from_text("https://example.test/table.json"));
        assert!(matches!(state.url_entry_key(&mut app.shared, &press(KeyCode::Escape)), Transition::Stay));
        assert!(state.text_input.is_none());
        assert!(app.shared.config.library.tables.is_empty());
    }

    /// A URL is the one value in the app long enough to need editing rather than retyping.
    #[test]
    fn the_caret_keys_edit_the_typed_url_in_place() {
        let mut app = app();
        let mut state = TablesState::new();
        state.text_input = Some(TextEdit::from_text("http//x"));
        for key in [KeyCode::Home, KeyCode::ArrowRight, KeyCode::ArrowRight, KeyCode::ArrowRight, KeyCode::ArrowRight] {
            state.url_entry_key(&mut app.shared, &press(key));
        }
        state.url_entry_key(&mut app.shared, &typed("s:"));
        assert_eq!(state.text_input.as_ref().expect("still typing").text(), "https://x");

        state.url_entry_key(&mut app.shared, &press(KeyCode::End));
        state.url_entry_key(&mut app.shared, &press(KeyCode::Backspace));
        assert_eq!(state.text_input.as_ref().expect("still typing").text(), "https://");
    }

    /// The paste modifier is held down while V is pressed, so V must not also be typed as a letter.
    #[test]
    fn a_held_paste_modifier_stops_its_letters_being_typed() {
        let mut app = app();
        let mut state = TablesState::new();
        state.text_input = Some(TextEdit::new());
        state.handle_key(&mut ctx(&mut app), KeyInput { code: KeyCode::ControlLeft, pressed: true, released: false, text: None });
        assert!(state.paste_held);
        state.url_entry_key(&mut app.shared, &KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some("a") });
        assert_eq!(state.text_input.as_ref().expect("still typing").text(), "", "a shortcut is not typed into the line");

        state.handle_key(&mut ctx(&mut app), KeyInput { code: KeyCode::ControlLeft, pressed: false, released: true, text: None });
        assert!(!state.paste_held);
        state.url_entry_key(&mut app.shared, &typed("a"));
        assert_eq!(state.text_input.as_ref().expect("still typing").text(), "a");
    }

    fn ctx(app: &mut crate::App) -> FrameCtx<'_> {
        FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
    }

    /// The picker is a native window with the keyboard; the screen behind it must not act on keys
    /// that were meant for it.
    #[test]
    fn keys_are_ignored_while_the_file_picker_is_up() {
        let mut app = app();
        let mut state = TablesState::new();
        let (_tx, handle) = crate::dialog::handle_for_tests();
        state.picker = Some(handle);
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::Escape)), Transition::Stay), "escape belonged to the dialog");
    }

    /// A table file can be tens of thousands of entries, each matched against every chart in the
    /// library, so a picked file is read on the worker a typed URL is fetched on rather than in the
    /// middle of a frame. Its row is there at once, on this screen, and says the file is being read.
    #[test]
    fn a_picked_file_is_a_row_at_once_and_is_read_beside_the_screen() {
        let file = TableFile::written("picked", br#"[{"md5":"1111","level":"1"}]"#);
        let mut app = app();
        let mut state = TablesState::new();
        state.sel = TablesState::row_count(&app.shared) - 1;
        let (tx, handle) = crate::dialog::handle_for_tests();
        state.picker = Some(handle);
        tx.send(Some(file.0.clone())).expect("the handle is still held");

        assert!(matches!(state.update(&mut ctx(&mut app)), Transition::Stay), "a picked file left the screen");
        assert!(state.picker.is_none());
        assert_eq!(app.shared.config.library.tables.len(), 1, "the row is not there until the file has been read");
        assert_eq!(state.sel, 0, "the cursor is not on the row that was added");
        assert_eq!(on_its_way(&app.shared.config.library.tables[0], table_status(&file.location()).as_ref()), (READING, READING_DETAIL));
        while !app.shared.table_fetches.is_idle() {
            state.update(&mut ctx(&mut app));
            std::thread::yield_now();
        }
        assert_eq!(table_status(&file.location()).as_ref().map(status_summary), Some(("0 OF 1 CHARTS OWNED  -  1 LEVELS".to_string(), false)));
        assert_eq!(app.shared.table_names.len(), 1);
    }

    /// A source already in the list is not added twice, and nothing is fetched for it.
    #[test]
    fn a_file_already_in_the_list_is_not_added_again() {
        let mut app = app();
        let mut state = TablesState::new();
        app.shared.config.library.tables.push(TableSource { name: "one".into(), location: "/tables/one.json".into() });
        state.add_picked_file(&mut app.shared, std::path::Path::new("/tables/one.json"));
        assert_eq!(app.shared.config.library.tables.len(), 1);
        assert!(app.shared.table_fetches.is_idle());
    }

    /// A table file written for one test, removed when the test is over.
    struct TableFile(std::path::PathBuf);

    impl TableFile {
        fn written(tag: &str, body: &[u8]) -> TableFile {
            let path = std::env::temp_dir().join(format!("rbms-tables-{tag}-{}.json", std::process::id()));
            std::fs::write(&path, body).expect("write the fixture");
            TableFile(path)
        }

        fn location(&self) -> String {
            self.0.to_string_lossy().to_string()
        }
    }

    impl Drop for TableFile {
        fn drop(&mut self) {
            forget_table_status(&self.location());
            let _ = std::fs::remove_file(&self.0);
        }
    }

    /// Register `file` the way the picker does, and run the table manager's frames until the file
    /// has been read and its table has landed.
    fn register(app: &mut crate::App, file: &TableFile) {
        let mut state = TablesState::new();
        state.add_picked_file(&mut app.shared, &file.0);
        while !app.shared.table_fetches.is_idle() {
            assert!(matches!(state.update(&mut ctx(app)), Transition::Stay), "the table manager left for another screen");
            std::thread::yield_now();
        }
    }

    /// The folders of the song list as it stands, by label.
    fn folders(app: &mut crate::App, view: SelectView) -> Vec<String> {
        app.shared.select_view = view;
        app.shared.rebuild_select_items();
        app.shared
            .select_items
            .iter()
            .map(|item| match item {
                SelectItem::Folder { label, .. } => label.clone(),
                SelectItem::Song(index) => format!("song {index}"),
            })
            .collect()
    }

    /// The report this was written for: a table was registered and no folder for it appeared. A
    /// table that loads is a folder in the song list whether or not any of its charts are owned,
    /// with its levels inside it in the table's order.
    #[test]
    fn a_registered_table_none_of_whose_charts_are_owned_is_still_a_folder_with_its_levels() {
        let file = TableFile::written("unowned", br#"[{"md5":"1111","level":"2"},{"md5":"2222","level":"1"},{"md5":"3333","level":"2"}]"#);
        let mut app = app();
        register(&mut app, &file);

        assert_eq!(app.shared.config.library.tables.len(), 1, "the source is in the settings");
        let root = folders(&mut app, SelectView::Root);
        assert_eq!(root.len(), 2, "the song list has the table's folder beside ALL SONGS: {root:?}");
        assert!(root[1].ends_with(" (0)"), "the folder says nothing in it is owned: {}", root[1]);
        assert_eq!(folders(&mut app, SelectView::TableLevels(0)), vec!["LV 1 (0)", "LV 2 (0)"], "the table's levels, in its order");
        assert!(folders(&mut app, SelectView::TableLevel(0, 0)).is_empty(), "an unowned level opens onto an empty list");

        let status = table_status(&file.location()).expect("the load is remembered");
        assert_eq!(status_summary(&status), ("0 OF 3 CHARTS OWNED  -  2 LEVELS".to_string(), false));
        assert!(status_detail(&status).is_some_and(|detail| detail.contains("NONE OF THIS TABLE'S CHARTS")), "the manager says why the folder is empty");
    }

    /// A table that could not be read is not a folder, so the table manager has to be where it is
    /// accounted for — and nothing may announce it as added, which is what made the missing folder
    /// read as a fault of the song list.
    #[test]
    fn a_table_that_could_not_be_read_keeps_its_row_says_why_and_is_not_announced_as_added() {
        crate::notify::exclusive(|| {
            let file = TableFile::written("unreadable", b"<!doctype html><html><head><title>not a table</title></head></html>");
            let mut app = app();
            register(&mut app, &file);

            assert_eq!(app.shared.config.library.tables.len(), 1, "the row is there to look at and remove");
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.iter().all(|(_, message)| !message.contains("table added")), "a table that was not read was announced as added: {said:?}");
            assert_eq!(folders(&mut app, SelectView::Root).len(), 1, "a table that was not read is not a folder");

            let status = table_status(&file.location()).expect("the failure is remembered");
            assert_eq!(status_summary(&status), ("LOAD FAILED".to_string(), true));
            assert!(status_detail(&status).is_some_and(|detail| detail.starts_with("LOAD FAILED: body parse:")), "the manager says what failed");

            let mut state = TablesState::new();
            state.remove_table_source(&mut app.shared, 0);
            assert_eq!(table_status(&file.location()), None, "removing the source forgets what became of it");
        });
    }

    /// A table that was read and lists nothing has no levels, so it is no folder either. That is the
    /// shape of the first report again — registered, and nothing in the song list — so it is said
    /// where the table was registered and in place of the announcement that it was added.
    #[test]
    fn a_table_that_lists_no_charts_says_so_instead_of_being_announced_as_added() {
        crate::notify::exclusive(|| {
            let file = TableFile::written("empty", b"[]");
            let mut app = app();
            register(&mut app, &file);

            assert_eq!(app.shared.config.library.tables.len(), 1, "the row is there to look at and remove");
            assert_eq!(folders(&mut app, SelectView::Root).len(), 1, "a table with no levels is not a folder");
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.iter().all(|(_, message)| !message.starts_with("table added:")), "an empty table was announced like one with a folder: {said:?}");
            assert!(
                said.iter().any(|(level, message)| *level == crate::notify::Level::Warn && message.contains("lists no charts")),
                "nothing said the table is empty: {said:?}"
            );

            let status = table_status(&file.location()).expect("the load is remembered");
            assert_eq!(status_summary(&status), ("NO CHARTS LISTED".to_string(), true));
            assert!(status_detail(&status).is_some_and(|detail| detail.contains("NO FOLDER")), "the manager says why there is no folder");
        });
    }

    #[test]
    fn a_source_is_labelled_by_the_users_name_then_the_tables_own_then_only_its_location() {
        let loaded = TableStatus::Loaded { name: "\u{767a}\u{72c2}".to_string(), entries: 9, levels: 3, owned: 4, origin: TableOrigin::File, stale: None };
        let unnamed = TableSource { name: String::new(), location: "https://tables.test/t/".into() };
        assert_eq!(source_label(&unnamed, None), "https://tables.test/t/");
        assert_eq!(source_label(&unnamed, Some(&loaded)), "\u{767a}\u{72c2}  -  https://tables.test/t/");
        assert_eq!(source_label(&unnamed, Some(&TableStatus::Failed { reason: "x".into() })), "https://tables.test/t/");
        let named = TableSource { name: "\u{7b2c}2".into(), location: "/tables/second.json".into() };
        assert_eq!(source_label(&named, Some(&loaded)), "\u{7b2c}2  -  /tables/second.json", "a name in any script is a name");
    }

    #[test]
    fn each_status_reads_differently_at_a_glance_and_in_full() {
        let fresh = TableStatus::Loaded { name: "T".into(), entries: 1035, levels: 26, owned: 1035, origin: TableOrigin::File, stale: None };
        assert_eq!(status_summary(&fresh), ("1035 OF 1035 CHARTS OWNED  -  26 LEVELS".to_string(), false));
        assert_eq!(status_detail(&fresh), None, "a table that loaded and is owned needs no more said");

        let stale = TableStatus::Loaded {
            name: "T".into(),
            entries: 10,
            levels: 2,
            owned: 3,
            origin: TableOrigin::Address { fetched_at: Some(0) },
            stale: Some("address: HTTP 503 Service Unavailable".into()),
        };
        assert_eq!(status_summary(&stale), ("RELOAD FAILED  -  3 OF 10 CHARTS OWNED".to_string(), true));
        assert_eq!(status_detail(&stale).as_deref(), Some("RELOAD FAILED, THE TABLE IS AS IT WAS: address: HTTP 503 Service Unavailable"));

        let failed = TableStatus::Failed { reason: "header https://tables.test/t/header.json: HTTP 404 Not Found".into() };
        assert_eq!(status_detail(&failed).as_deref(), Some("LOAD FAILED: header https://tables.test/t/header.json: HTTP 404 Not Found"));
    }

    /// What became of a table is on the screen, not only in the functions that word it.
    #[test]
    fn a_source_whose_table_failed_is_drawn_differently_from_one_nothing_is_known_of() {
        let file = TableFile::written("drawn", b"not a table");
        let mut app = app();
        app.shared.config.library.tables.push(TableSource { name: "one".into(), location: file.location() });
        let unknown = crate::stage::render_tests::render(&mut app, Stage::Tables(TablesState::new())).pixel_checksum();

        let nothing_owned = rbms_table::Md5Index::new(std::iter::empty());
        let store = TableStore::beside(&app.shared.settings_path);
        read_and_match(&store, &app.shared.config.library.tables[0], &nothing_owned);
        let failed = crate::stage::render_tests::render(&mut app, Stage::Tables(TablesState::new())).pixel_checksum();
        assert_ne!(unknown, failed, "the failure is not on the screen");
    }

    #[test]
    fn a_dismissed_picker_is_let_go_of_and_adds_nothing() {
        let mut app = app();
        let mut state = TablesState::new();
        let (tx, handle) = crate::dialog::handle_for_tests();
        state.picker = Some(handle);
        tx.send(None).expect("the handle is still held");
        state.update(&mut ctx(&mut app));
        assert!(state.picker.is_none(), "the screen stops waiting once it has an answer");
        assert!(app.shared.config.library.tables.is_empty());
    }

    /// The addresses of one test, and the copies they leave beside its app's settings, which are
    /// dropped when the test is over. A table at an address is fetched from here and nowhere else.
    struct Addresses {
        tables: crate::tablesrc::test_outside::FakeTables,
        settings: std::path::PathBuf,
        registered: Vec<String>,
        _installed: crate::tablesrc::test_outside::Installed,
    }

    impl Addresses {
        fn of(app: &crate::App) -> Addresses {
            let tables = crate::tablesrc::test_outside::FakeTables::default();
            let installed = tables.install();
            Addresses { tables, settings: app.shared.settings_path.clone(), registered: Vec::new(), _installed: installed }
        }

        fn store(&self) -> TableStore {
            TableStore::beside(&self.settings)
        }

        /// Register the table at `address` the way typing its URL does, and run the table
        /// manager's frames until its fetch has landed.
        fn register(&mut self, app: &mut crate::App, address: &str) {
            self.registered.push(address.to_string());
            let mut state = TablesState::new();
            state.text_input = Some(TextEdit::from_text(address));
            assert!(matches!(state.url_entry_key(&mut app.shared, &press(KeyCode::Enter)), Transition::Stay), "a typed address left the table manager");
            self.wait_for_fetches(app, &mut state);
        }

        /// Run the table manager's frames until nothing of this app's is being fetched any more.
        fn wait_for_fetches(&self, app: &mut crate::App, state: &mut TablesState) {
            while !app.shared.table_fetches.is_idle() {
                assert!(matches!(state.update(&mut ctx(app)), Transition::Stay), "the table manager left for another screen");
                std::thread::yield_now();
            }
        }
    }

    impl Drop for Addresses {
        fn drop(&mut self) {
            let store = self.store();
            for address in &self.registered {
                forget_table_status(address);
                store.forget(address);
            }
        }
    }

    const ONE_LEVEL: &[u8] = br#"[{"md5":"1111","level":"1"}]"#;
    const TWO_LEVELS: &[u8] = br#"[{"md5":"1111","level":"1"},{"md5":"2222","level":"2"}]"#;

    /// A table registered by its address is fetched once and kept: the row says which day the copy
    /// is from, and removing the table drops the copy with it.
    #[test]
    fn a_table_registered_by_address_is_kept_and_its_copy_goes_when_it_is_removed() {
        let address = "https://tables.test/manager/kept/";
        let mut app = app();
        let mut addresses = Addresses::of(&app);
        addresses.tables.serve(address, "Kept", TWO_LEVELS);
        addresses.register(&mut app, address);

        assert_eq!(addresses.tables.asked(), 1);
        assert!(addresses.store().has_copy_file(address), "a fetched table was not kept");
        let status = table_status(address).expect("the load is remembered");
        let now = unix_now();
        let (saved, is_old) = saved_note(&status, now).expect("a table at an address says how long ago its copy was fetched");
        assert_eq!((saved, is_old), ("SAVED JUST NOW".to_string(), false), "a copy fetched just now is not said to be, or is old");
        assert_eq!(old_copy_note(&status, now), None);
        let written = ron::ser::to_string(&app.shared.config.library.tables).expect("the sources are written");
        assert_eq!(written, format!(r#"[(name:"",location:"{address}")]"#), "what the settings file holds of a table is its name and its address, as before");
        let earlier: Vec<TableSource> =
            ron::from_str(r#"[(name: "mine", location: "https://tables.test/manager/earlier/")]"#).expect("an earlier settings entry reads");
        assert_eq!((earlier[0].name.as_str(), earlier[0].location.as_str()), ("mine", "https://tables.test/manager/earlier/"));

        let mut state = TablesState::new();
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyD));
        assert!(app.shared.config.library.tables.is_empty());
        assert!(!addresses.store().has_copy_file(address), "the copy of a removed table was left behind");
    }

    /// R fetches the table under the cursor again without leaving the screen. While it is out the
    /// row says so; when it lands the copy, the row and the folder in the song list are the new
    /// table's, and one line says how the reload went.
    #[test]
    fn r_fetches_the_table_under_the_cursor_again_and_the_song_list_follows() {
        crate::notify::exclusive(|| {
            let address = "https://tables.test/manager/reloaded/";
            let mut app = app();
            let mut addresses = Addresses::of(&app);
            addresses.tables.serve(address, "Before", ONE_LEVEL);
            addresses.register(&mut app, address);
            assert_eq!(folders(&mut app, SelectView::TableLevels(0)), vec!["LV 1 (0)"]);

            addresses.tables.serve(address, "After", TWO_LEVELS);
            let mut state = TablesState::new();
            let open = addresses.tables.hold();
            assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR)), Transition::Stay), "a reload left the screen");
            assert!(table_is_fetching(&app.shared, address), "the row has nothing to say the table is on its way");
            assert!(matches!(state.update(&mut ctx(&mut app)), Transition::Stay), "the screen stopped while the table was out");
            assert_eq!(app.shared.table_names, vec!["BEFORE".to_string()], "the table changed before the reload came back");
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
            drop(open);
            addresses.wait_for_fetches(&mut app, &mut state);

            assert_eq!(addresses.tables.asked(), 2, "one registration and one reload; a second R while it was out is not a second fetch");
            assert_eq!(app.shared.table_names, vec!["AFTER".to_string()]);
            assert_eq!(folders(&mut app, SelectView::TableLevels(0)), vec!["LV 1 (0)", "LV 2 (0)"], "the folder is the reloaded table's");
            assert_eq!(addresses.store().copy_name(address).as_deref(), Some("After"), "the copy was not replaced");
            assert_eq!(table_status(address).as_ref().map(status_summary), Some(("0 OF 2 CHARTS OWNED  -  2 LEVELS".to_string(), false)));
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.iter().any(|(_, message)| message == "difficulty tables reloaded: 1 updated, 0 failed"), "nothing said how the reload went: {said:?}");
        });
    }

    /// A reload that fails changes nothing but what the row says: the table, its folder and its
    /// copy are the ones from before, and the reason is on the row and in a notification.
    #[test]
    fn a_reload_that_fails_keeps_the_table_and_says_why() {
        crate::notify::exclusive(|| {
            let address = "https://tables.test/manager/down/";
            let mut app = app();
            let mut addresses = Addresses::of(&app);
            addresses.tables.serve(address, "Kept", TWO_LEVELS);
            addresses.register(&mut app, address);
            let before = table_status(address);

            addresses.tables.take_down(address);
            let mut state = TablesState::new();
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
            addresses.wait_for_fetches(&mut app, &mut state);

            assert_eq!(app.shared.table_names, vec!["KEPT".to_string()]);
            assert_eq!(folders(&mut app, SelectView::TableLevels(0)), vec!["LV 1 (0)", "LV 2 (0)"], "the folder of a table whose reload failed changed");
            assert_eq!(addresses.store().copy_name(address).as_deref(), Some("Kept"));
            let status = table_status(address).expect("the table is still loaded");
            assert_eq!(status_summary(&status), ("RELOAD FAILED  -  0 OF 2 CHARTS OWNED".to_string(), true));
            assert!(status_detail(&status).is_some_and(|detail| detail.starts_with("RELOAD FAILED, THE TABLE IS AS IT WAS: ")), "the row does not say why");
            let now = unix_now();
            assert_eq!(saved_note(&status, now), before.as_ref().and_then(|before| saved_note(before, now)), "the copy's age changed");
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(
                said.iter().any(|(level, message)| *level == Level::Warn && message.starts_with(&format!("table reload failed ({address}): "))),
                "{said:?}"
            );
            assert!(said.iter().any(|(level, message)| *level == Level::Warn && message == "difficulty tables reloaded: 0 updated, 1 failed"), "{said:?}");

            addresses.tables.serve(address, "Back", ONE_LEVEL);
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
            addresses.wait_for_fetches(&mut app, &mut state);
            assert_eq!(
                table_status(address).as_ref().map(status_summary),
                Some(("0 OF 1 CHARTS OWNED  -  1 LEVELS".to_string(), false)),
                "a later reload clears it"
            );
        });
    }

    /// A fetches every table again: the ones at an address from their addresses and the one that is
    /// a file from its file, and the tally counts each.
    #[test]
    fn a_fetches_every_table_again_and_reads_a_table_file_again() {
        crate::notify::exclusive(|| {
            let (up, down) = ("https://tables.test/manager/all/up/", "https://tables.test/manager/all/down/");
            let file = TableFile::written("reload-all", ONE_LEVEL);
            let mut app = app();
            let mut addresses = Addresses::of(&app);
            addresses.tables.serve(up, "Up", ONE_LEVEL);
            addresses.tables.serve(down, "Down", ONE_LEVEL);
            addresses.register(&mut app, up);
            addresses.register(&mut app, down);
            register(&mut app, &file);
            assert_eq!(addresses.tables.asked(), 2);

            addresses.tables.serve(up, "Up", TWO_LEVELS);
            addresses.tables.take_down(down);
            std::fs::write(&file.0, TWO_LEVELS).expect("rewrite the table file");
            let mut state = TablesState::new();
            state.sel = TablesState::row_count(&app.shared) - 1;
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyA));
            addresses.wait_for_fetches(&mut app, &mut state);

            assert_eq!(addresses.tables.asked(), 4, "each table at an address is fetched once more, and the file is not fetched at all");
            let levels: Vec<usize> = app.shared.table_levels.iter().map(Vec::len).collect();
            assert_eq!(levels, vec![2, 1, 2], "the table that answered and the file are new, the one that did not answer is as it was");
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            assert!(said.iter().any(|(_, message)| message == "difficulty tables reloaded: 2 updated, 1 failed"), "{said:?}");

            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
            assert_eq!(addresses.tables.asked(), 4, "R on a row that is not a table reloaded something");
        });
    }

    /// A table removed while its reload is out is not brought back by the reload landing, the copy
    /// the reload wrote of it is dropped, and nothing counts it as a table that was updated.
    #[test]
    fn a_table_removed_while_its_reload_is_out_stays_removed() {
        crate::notify::exclusive(|| {
            let address = "https://tables.test/manager/removed-meanwhile/";
            let mut app = app();
            let mut addresses = Addresses::of(&app);
            addresses.tables.serve(address, "Going", ONE_LEVEL);
            addresses.register(&mut app, address);

            let mut state = TablesState::new();
            let open = addresses.tables.hold();
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
            state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyD));
            let mut said = Vec::new();
            crate::notify::drain(&mut said);
            drop(open);
            addresses.wait_for_fetches(&mut app, &mut state);

            assert!(app.shared.config.library.tables.is_empty() && app.shared.table_names.is_empty() && app.shared.table_levels.is_empty());
            assert!(!addresses.store().has_copy_file(address), "the reload left a copy of a table that is no longer registered");
            crate::notify::drain(&mut said);
            assert!(
                said.iter().all(|(_, message)| !message.starts_with("difficulty tables reloaded:")),
                "a table that was removed was counted in how its reload went: {said:?}"
            );
        });
    }

    /// A settings file written by hand can list one address twice. Both rows are the same table,
    /// so a reload that fetched it once brings both up to date.
    #[test]
    fn a_reload_updates_every_row_that_names_the_address_it_fetched() {
        let address = "https://tables.test/manager/twice/";
        let mut app = app();
        let mut addresses = Addresses::of(&app);
        addresses.tables.serve(address, "Twice", ONE_LEVEL);
        addresses.register(&mut app, address);
        app.shared.config.library.tables.push(TableSource { name: "again".into(), location: address.to_string() });
        app.shared.table_names.push("AGAIN".to_string());
        let first = app.shared.table_levels[0].clone();
        app.shared.table_levels.push(first);

        addresses.tables.serve(address, "Twice", TWO_LEVELS);
        let mut state = TablesState::new();
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyA));
        addresses.wait_for_fetches(&mut app, &mut state);

        assert_eq!(addresses.tables.asked(), 2, "one registration and one reload: an address listed twice is fetched once");
        let levels: Vec<usize> = app.shared.table_levels.iter().map(Vec::len).collect();
        assert_eq!(levels, vec![2, 2], "a row that names the reloaded address was left as it was");
        assert_eq!(app.shared.table_names, vec!["TWICE".to_string(), "AGAIN".to_string()], "each row keeps the name its source gives it");
        app.shared.config.library.tables.clear();
    }

    /// The song list looking into a level of a table follows that level by its name when a reload
    /// moves it, and steps out to the table's levels when the reloaded table no longer has it.
    #[test]
    fn a_level_being_looked_at_is_followed_by_name_through_a_reload() {
        const LEVELS_1_2_3: &[u8] = br#"[{"md5":"1111","level":"1"},{"md5":"2222","level":"2"},{"md5":"3333","level":"3"}]"#;
        const LEVELS_0_1_2: &[u8] = br#"[{"md5":"0000","level":"0"},{"md5":"1111","level":"1"},{"md5":"2222","level":"2"}]"#;
        const LEVELS_0_1: &[u8] = br#"[{"md5":"0000","level":"0"},{"md5":"1111","level":"1"}]"#;
        let address = "https://tables.test/manager/followed/";
        let mut app = app();
        let mut addresses = Addresses::of(&app);
        addresses.tables.serve(address, "Followed", LEVELS_1_2_3);
        addresses.register(&mut app, address);
        let names = |app: &crate::App| app.shared.table_levels[0].iter().map(|(name, _)| name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&app), vec!["1", "2", "3"]);

        app.shared.select_view = SelectView::TableLevel(0, 1);
        app.shared.rebuild_select_items();
        addresses.tables.serve(address, "Followed", LEVELS_0_1_2);
        let mut state = TablesState::new();
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
        addresses.wait_for_fetches(&mut app, &mut state);
        assert_eq!(names(&app), vec!["0", "1", "2"]);
        assert!(app.shared.select_view == SelectView::TableLevel(0, 2), "the list is no longer looking into level 2, which moved one place down");

        addresses.tables.serve(address, "Followed", LEVELS_0_1);
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
        addresses.wait_for_fetches(&mut app, &mut state);
        assert_eq!(names(&app), vec!["0", "1"]);
        assert!(app.shared.select_view == SelectView::TableLevels(0), "a level the table no longer has is still being looked into");

        app.shared.select_view = SelectView::TableLevel(0, 0);
        addresses.tables.serve(address, "Followed", LEVELS_1_2_3);
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
        addresses.wait_for_fetches(&mut app, &mut state);
        assert!(
            app.shared.select_view == SelectView::TableLevels(0),
            "the list went on looking into the first level, which is another level now that level 0 is gone"
        );
    }

    /// The age of a copy is pointed out once the copy is old, and not before; a table that is a
    /// file has no copy to age.
    #[test]
    fn the_age_of_a_copy_is_pointed_out_once_the_copy_is_old() {
        const DAY: u64 = 86_400;
        let now = 1_760_227_200 + 3600;
        let loaded = |origin| TableStatus::Loaded { name: "T".into(), entries: 2, levels: 1, owned: 1, origin, stale: None };
        let fresh = loaded(TableOrigin::Address { fetched_at: Some(now - COPY_OLD_AFTER_DAYS * DAY) });
        assert_eq!(saved_note(&fresh, now), Some(("SAVED 30 DAYS AGO".to_string(), false)), "a copy exactly at the limit is not old yet");
        assert_eq!(old_copy_note(&fresh, now), None);

        let old = loaded(TableOrigin::Address { fetched_at: Some(now - 45 * DAY) });
        assert_eq!(saved_note(&old, now), Some(("SAVED 45 DAYS AGO".to_string(), true)));
        let hours_old = loaded(TableOrigin::Address { fetched_at: Some(now - 3 * 3600) });
        assert_eq!(saved_note(&hours_old, now), Some(("SAVED 3 HOURS AGO".to_string(), false)), "an age is counted in the zone of whoever reads it");
        assert_eq!(old_copy_note(&old, now).as_deref(), Some("THE SAVED COPY IS 45 DAYS OLD - R FETCHES THE TABLE AGAIN"));

        let undated = loaded(TableOrigin::Address { fetched_at: None });
        assert_eq!(saved_note(&undated, now), Some(("SAVED".to_string(), false)), "a copy that does not say when it is from is not called old");
        assert_eq!(saved_note(&loaded(TableOrigin::File), now), None);
        assert_eq!(saved_note(&TableStatus::Failed { reason: "x".into() }, now), None);
    }

    /// Where the frames below are saved for a person to open, when this names a folder: the
    /// variable the capture harness reads.
    const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

    /// A full HD window, which the captures are also drawn for beside the size the screen is laid
    /// out for.
    const LARGE: (u32, u32) = (1920, 1080);

    /// Draw the table manager with the cursor on the row `sel`, and answer what was drawn at the
    /// size the screen is laid out for. With a capture folder named, the frame is saved there as
    /// `<name>-<size>.png` at that size and at full HD.
    fn drawn(app: &mut crate::App, sel: usize, name: &str) -> u64 {
        let focused = || Stage::Tables(TablesState { sel, ..TablesState::default() });
        let pixels = crate::stage::render_tests::render(app, focused());
        if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
            let folder = std::path::PathBuf::from(folder);
            std::fs::create_dir_all(&folder).expect("the capture folder can be created");
            let mut large = crate::stage::HeadlessCanvas::new(LARGE.0, LARGE.1);
            crate::stage::render_tests::render_into(app, focused(), &mut large);
            for (frame, size) in [(&pixels, (CW, CH)), (&large, LARGE)] {
                let file = folder.join(format!("{name}-{}x{}.png", size.0, size.1));
                image::save_buffer(file, frame.rgba(), size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
            }
        }
        pixels.pixel_checksum()
    }

    /// What is said under the list of a table that is being fetched again is that it is, whatever
    /// its last reload came to: a reason it failed before is not what is happening to it now.
    #[test]
    fn a_table_on_its_way_says_so_under_the_list_and_not_why_it_failed_before() {
        let stale = TableStatus::Loaded {
            name: "T".into(),
            entries: 10,
            levels: 2,
            owned: 3,
            origin: TableOrigin::Address { fetched_at: Some(0) },
            stale: Some("the address lists no charts now".into()),
        };
        let failed = TableStatus::Failed { reason: "address: HTTP 404 Not Found".into() };
        for status in [Some(&stale), Some(&failed), Some(&TableStatus::NotFetched), None] {
            assert_eq!(focused_detail(status, Some(RELOADING_DETAIL)), Some((RELOADING_DETAIL.to_string(), false)), "{status:?}");
        }
        assert_eq!(focused_detail(Some(&stale), None), Some(("RELOAD FAILED, THE TABLE IS AS IT WAS: the address lists no charts now".to_string(), true)));
        assert_eq!(focused_detail(Some(&failed), None), Some(("LOAD FAILED: address: HTTP 404 Not Found".to_string(), true)));
        let unowned = TableStatus::Loaded { name: "T".into(), entries: 3, levels: 2, owned: 0, origin: TableOrigin::File, stale: None };
        assert!(focused_detail(Some(&unowned), None).is_some_and(|(detail, is_problem)| detail.contains("NONE OF THIS TABLE'S CHARTS") && !is_problem));
        assert_eq!(focused_detail(Some(&TableStatus::NotFetched), None), Some((NOT_FETCHED_DETAIL.to_string(), false)), "a table not fetched yet is a problem");
        assert_eq!(focused_detail(None, None), None);
        rbms_render::font::use_embedded_fonts_only();
        for detail in [RELOADING_DETAIL, FETCHING_DETAIL, READING_DETAIL, NOT_FETCHED_DETAIL] {
            assert_eq!(rbms_render::fit_text(detail, DETAIL_SCALE, PANEL_W), detail, "what is said under the list is cut short");
            assert!(detail.chars().count() <= DETAIL_LINE_CHARS, "what is said of a table on its way is broken over two lines: {detail}");
        }
        for note in RELOAD_NOTE {
            assert_eq!(rbms_render::fit_text(note, NOTE_SCALE, PANEL_W), note, "a line of the note along the bottom is cut short");
        }
    }

    /// A table that is not in the song list is one of three things, and its row says which: it has
    /// not been fetched yet, it is on its way, or its fetch failed. Each draws a row of its own and
    /// a line of its own under the list.
    #[test]
    fn a_table_never_fetched_one_on_its_way_and_one_that_failed_draw_rows_of_their_own() {
        const BODY: &[u8] = br#"[{"md5":"1111","level":"1"},{"md5":"2222","level":"2"},{"md5":"3333","level":"2"}]"#;
        let (kept, dead) = ("https://tables.test/states/insane1/", "https://tables.test/states/unreachable/");
        let (waiting, arriving) = ("https://tables.test/states/satellite/", "https://tables.test/states/stella/");
        let mut app = app();
        let mut addresses = Addresses::of(&app);
        addresses.tables.serve(kept, "Insane BMS", BODY);
        addresses.tables.serve(arriving, "Stella", BODY);
        addresses.register(&mut app, kept);
        crate::notify::exclusive(|| addresses.register(&mut app, dead));
        addresses.registered.extend([waiting, arriving].map(str::to_string));
        app.shared.config.library.tables.push(TableSource { name: String::new(), location: waiting.to_string() });
        read_and_match(&addresses.store(), &app.shared.config.library.tables[2], &rbms_table::Md5Index::new(std::iter::empty()));

        let open = addresses.tables.hold();
        let mut state = TablesState::new();
        state.text_input = Some(TextEdit::from_text(arriving));
        state.url_entry_key(&mut app.shared, &press(KeyCode::Enter));
        assert_eq!(state.sel, 3);

        let tables = app.shared.config.library.tables.clone();
        let said_of = |app: &crate::App, row: usize| {
            let status = table_status(&tables[row].location);
            let arriving = table_is_fetching(&app.shared, &tables[row].location).then(|| on_its_way(&tables[row], status.as_ref()));
            let summary = arriving.map_or_else(|| status.as_ref().map(status_summary).expect("the row has a status").0, |(summary, _)| summary.to_string());
            (summary, focused_detail(status.as_ref(), arriving.map(|(_, detail)| detail)).map(|(detail, _)| detail))
        };
        assert_eq!(said_of(&app, 1).0, "LOAD FAILED");
        assert!(said_of(&app, 1).1.is_some_and(|detail| detail.starts_with("LOAD FAILED: ")));
        assert_eq!(said_of(&app, 2), (NOT_FETCHED.to_string(), Some(NOT_FETCHED_DETAIL.to_string())));
        assert_eq!(said_of(&app, 3), (FETCHING.to_string(), Some(FETCHING_DETAIL.to_string())));

        let loaded = drawn(&mut app, 0, "tables-state-loaded");
        let failed = drawn(&mut app, 1, "tables-state-failed");
        let not_fetched = drawn(&mut app, 2, "tables-state-not-fetched-yet");
        let fetching = drawn(&mut app, 3, "tables-state-fetching");
        let frames = [loaded, failed, not_fetched, fetching];
        for (at, frame) in frames.iter().enumerate() {
            assert!(frames[at + 1..].iter().all(|other| other != frame), "two of the states draw the same frame: {frames:?}");
        }

        drop(open);
        addresses.wait_for_fetches(&mut app, &mut state);
        assert_eq!(said_of(&app, 3).0, "0 OF 3 CHARTS OWNED  -  2 LEVELS");
        drawn(&mut app, 3, "tables-state-arrived");
        app.shared.config.library.tables.clear();
    }

    /// What a row says is on the screen: how long ago its copy was fetched, that the table is on
    /// its way, that its reload failed, and that its copy is old each draw a frame of their own.
    #[test]
    fn the_rows_of_the_table_manager_draw_what_became_of_each_table() {
        const DAY: u64 = 86_400;
        const BODY: &[u8] = br#"[{"md5":"1111","level":"1"},{"md5":"2222","level":"2"},{"md5":"3333","level":"2"}]"#;
        let (insane, satellite, old) = ("https://tables.test/drawn/insane1/", "https://tables.test/drawn/satellite/", "https://tables.test/drawn/old/");
        let file = TableFile::written("drawn-rows", BODY);
        let mut app = app();
        let mut addresses = Addresses::of(&app);
        addresses.tables.serve(insane, "Insane BMS", BODY);
        addresses.tables.serve(satellite, "Satellite", BODY);
        addresses.registered.push(old.to_string());
        addresses.store().keep_for_tests(old, "Stella", BODY, Some(unix_now() - 45 * DAY));
        addresses.register(&mut app, insane);
        addresses.register(&mut app, satellite);
        register(&mut app, &file);
        app.shared.config.library.tables.push(TableSource { name: String::new(), location: old.to_string() });
        let nothing_owned = rbms_table::Md5Index::new(std::iter::empty());
        read_and_match(&addresses.store(), &app.shared.config.library.tables[3], &nothing_owned);
        assert_eq!(addresses.tables.asked(), 2, "the table with an old copy was read from the copy");

        let loaded = drawn(&mut app, 0, "tables-loaded");
        let old_focused = drawn(&mut app, 3, "tables-old-copy");
        assert_ne!(loaded, old_focused);

        let open = addresses.tables.hold();
        let mut state = TablesState { sel: 1, ..TablesState::default() };
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyA));
        let reloading = drawn(&mut app, 1, "tables-reloading");
        addresses.tables.take_down(satellite);
        drop(open);
        addresses.wait_for_fetches(&mut app, &mut state);
        let failed = drawn(&mut app, 1, "tables-reload-failed");
        assert_ne!(reloading, failed, "a reload that is out and one that failed draw the same row");
        assert_ne!(loaded, failed);

        let open = addresses.tables.hold();
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
        let on_its_way_again = drawn(&mut app, 1, "tables-reloading-after-a-failure");
        assert_ne!(on_its_way_again, failed, "a table on its way again is drawn as one whose reload failed");
        drop(open);
        addresses.wait_for_fetches(&mut app, &mut state);
        app.shared.config.library.tables.clear();
    }
}

#[cfg(test)]
mod folder_row_tests {
    use super::*;
    use crate::ir_session::AccountSession;
    use crate::stage::render_tests::app;
    use crate::user_folders::tests::{PLAYER, SERVER, TOKEN, two_folders};

    fn press(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text: None }
    }

    fn ctx(app: &mut crate::App) -> FrameCtx<'_> {
        FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
    }

    /// The row of the player's own folders says what became of them in each state they can be in.
    #[test]
    fn the_folders_row_says_what_became_of_the_folders() {
        assert_eq!(folders_summary(&FolderStatus::SignedOut), (FOLDERS_SIGNED_OUT.to_string(), false));
        assert_eq!(folders_summary(&FolderStatus::Fetching), (FETCHING.to_string(), false));
        let shown = FolderStatus::Shown { folders: 5, listed: 120, owned: 87, fetched_at: Some(0), problem: None };
        assert_eq!(folders_summary(&shown), ("87 OF 120 CHARTS OWNED  -  5 FOLDERS".to_string(), false));
        assert_eq!(folders_saved_note(&shown, 3_600).as_deref(), Some("SAVED 1 HOUR AGO"));
        let stale = FolderStatus::Shown { folders: 5, listed: 120, owned: 87, fetched_at: None, problem: Some(FolderProblem::Failed("timed out".into())) };
        assert_eq!(folders_summary(&stale), ("RELOAD FAILED  -  87 OF 120 CHARTS OWNED".to_string(), true));
        assert_eq!(folders_detail(&stale), ("RELOAD FAILED, THE FOLDERS ARE AS THEY WERE: timed out".to_string(), true));
        assert_eq!(folders_summary(&FolderStatus::NotFetched { problem: None }), (NOT_FETCHED.to_string(), false));
        assert_eq!(folders_summary(&FolderStatus::NotFetched { problem: Some(FolderProblem::NoFolders) }), (FOLDERS_NONE.to_string(), false));
        let failed = FolderStatus::NotFetched { problem: Some(FolderProblem::Failed("refused".into())) };
        assert_eq!(folders_summary(&failed), ("LOAD FAILED".to_string(), true));
        assert_eq!(folders_detail(&failed), ("LOAD FAILED: refused".to_string(), true));
    }

    /// The row is the last one and cannot be removed; R fetches the folders again when someone is
    /// signed in, and does nothing for nobody.
    #[test]
    fn the_folders_row_fetches_again_and_is_never_removed() {
        let mut app = app();
        let mut state = TablesState::new();
        for _ in 0..TablesState::row_count(&app.shared) {
            state.handle_key(&mut ctx(&mut app), press(KeyCode::ArrowDown));
        }
        assert_eq!(state.sel, TablesState::folders_row(&app.shared), "the folders are the last row");
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
        assert_eq!(app.shared.user_folders.status(), FolderStatus::SignedOut, "nobody is signed in, and nothing was fetched");

        app.shared.config.network.server_url = Some(SERVER.into());
        app.shared.server = two_folders();
        app.shared.session = AccountSession::restored(Some(TOKEN.into()), Some(PLAYER.into()));
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyR));
        assert_eq!(app.shared.user_folders.status(), FolderStatus::Fetching);
        state.handle_key(&mut ctx(&mut app), press(KeyCode::KeyD));
        assert_eq!(TablesState::row_count(&app.shared), 3, "the folders row was removed");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.shared.user_folders.is_fetching() {
            state.update(&mut ctx(&mut app));
            assert!(std::time::Instant::now() < deadline, "the folders never came back");
            std::thread::yield_now();
        }
        assert!(matches!(app.shared.user_folders.status(), FolderStatus::Shown { folders: 2, listed: 4, .. }));
        if let Some(folder) = app.shared.settings_path.parent() {
            let _ = std::fs::remove_dir_all(folder.join("user-folders"));
        }
    }
}
