//! The difficulty-table manager: the list of table sources, the URL/file pickers that add one, and
//! the removal that drops its matched levels with it.
//!
//! Neither way of adding one stops the app: the file picker is a native panel the window keeps
//! drawing behind, and the table itself — fetched from a URL or read off disk — is parsed and
//! matched against the library on the LOADING screen rather than on the frame loop.
//!
//! Each source says what became of its table: how many of its charts the library has, or that it
//! could not be loaded and why. A table that failed is not a folder in the song list, so this is
//! the screen that has to account for it.
#![allow(clippy::wildcard_imports)]

use crate::dialog::{DialogHandle, DialogState};
use crate::stage::{Canvas, FrameCtx, KeyInput, LoadingState, Stage, StageHandler, Transition};
use crate::tablesrc::{TableStatus, forget_table_status, table_status};
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
        TableStatus::Loaded { entries, owned, stale: Some(_), .. } => (format!("SAVED COPY  -  {owned} OF {entries} CHARTS OWNED"), true),
        TableStatus::Failed { .. } => ("LOAD FAILED".to_string(), true),
    }
}

/// What there is to say about a source beyond its row, when there is anything: why it failed, why
/// it was not refreshed, that it lists nothing, or that none of its charts are in the library.
fn status_detail(status: &TableStatus) -> Option<String> {
    match status {
        TableStatus::Failed { reason } => Some(format!("LOAD FAILED: {reason}")),
        TableStatus::Loaded { stale: Some(reason), .. } => Some(format!("NOT REFRESHED: {reason}")),
        TableStatus::Loaded { entries: 0, .. } => Some("THE TABLE WAS READ BUT LISTS NO CHARTS - IT HAS NO FOLDER IN THE SONG LIST".to_string()),
        TableStatus::Loaded { owned: 0, .. } => Some("NONE OF THIS TABLE'S CHARTS ARE IN THE LIBRARY YET - ITS FOLDER IS EMPTY".to_string()),
        TableStatus::Loaded { .. } => None,
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

    /// Rows in the table manager: each source, then the two add actions.
    fn row_count(shared: &AppShared) -> usize {
        shared.config.library.tables.len() + 2
    }

    /// Add the table the file picker came back with, on the same screen of its own the typed URL
    /// uses.
    ///
    /// A table file is not always small — an insane-level table is tens of thousands of entries, and
    /// each is matched against every chart in the library — so it is parsed and matched on a worker
    /// rather than in the middle of a frame.
    fn add_picked_file(&mut self, shared: &AppShared, path: &std::path::Path) -> Transition {
        let location = path.to_string_lossy().to_string();
        if shared.config.library.tables.iter().any(|t| t.location == location) {
            notify(Level::Warn, format!("table already added: {location}"));
            return Transition::Stay;
        }
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or(DEFAULT_TABLE_NAME).to_string();
        Transition::Open(Stage::Loading(LoadingState::table(shared, TableSource { name, location })))
    }

    fn remove_table_source(&mut self, shared: &mut AppShared, idx: usize) {
        if idx < shared.config.library.tables.len() {
            let removed = shared.config.library.tables.remove(idx);
            forget_table_status(&removed.location);
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

    /// Start fetching the typed URL, on a screen of its own so the window keeps drawing while the
    /// request is out.
    fn add_typed_url(&mut self, shared: &AppShared) -> Transition {
        let url = self.text_input.take().map(|mut edit| edit.take()).unwrap_or_default().trim().to_string();
        if url.is_empty() {
            return Transition::Stay;
        }
        if shared.config.library.tables.iter().any(|t| t.location == url) {
            notify(Level::Warn, format!("table already added: {url}"));
            return Transition::Stay;
        }
        Transition::Open(Stage::Loading(LoadingState::table(shared, TableSource { name: String::new(), location: url })))
    }

    /// Typing the URL of a new table. Enter adds it, Esc abandons the entry, and the caret keys
    /// edit what has been typed so far rather than only its end.
    fn url_entry_key(&mut self, shared: &AppShared, key: &KeyInput<'_>) -> Transition {
        if key.code == KeyCode::Enter || key.code == KeyCode::NumpadEnter {
            return self.add_typed_url(shared);
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
        draw_text(canvas, x0, 84.0, 1.4, Color::YELLOW, "TYPE TABLE URL  -  ENTER ADD  ESC CANCEL  CTRL+V PASTE");
        canvas.fill_rect(Rect::new(x0, 116.0, PANEL_W, 40.0), th.button_active);
        let (shown, caret) = edit.window(URL_ENTRY_CHARS);
        let text_x = x0 + 14.0;
        draw_text(canvas, text_x, 128.0, URL_ENTRY_SCALE, th.text, &shown);
        let before: String = shown.chars().take(caret).collect();
        canvas.fill_rect(Rect::new(text_x + text_width(&before, URL_ENTRY_SCALE), 126.0, CARET_W, CARET_H), Color::YELLOW);
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
    /// Collect the file picker's answer once the user has given one. The picker runs beside the
    /// frame loop, so this is where an added table actually lands.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let Some(picker) = self.picker.as_ref() else {
            return Transition::Stay;
        };
        match picker.poll() {
            DialogState::Open => return Transition::Stay,
            DialogState::Picked(path) => {
                self.picker = None;
                return self.add_picked_file(ctx.shared, &path);
            }
            DialogState::Dismissed => self.picker = None,
        }
        Transition::Stay
    }

    /// Table-manager input. In URL-text mode, type the URL (Enter adds, Esc cancels); otherwise
    /// navigate, add (URL/file), remove (D), or leave (Esc — rebuilds the browse list). Keys are
    /// ignored while the native picker is up, which owns the keyboard itself.
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
            draw_text(canvas, x0, 84.0, 1.4, Color::YELLOW, "PICK A TABLE FILE IN THE OPEN DIALOG");
            return;
        }
        draw_text(canvas, x0, 84.0, 1.3, th.text_muted, "UP DOWN MOVE   ENTER SELECT   D REMOVE   ESC BACK");
        let tables = &ctx.shared.config.library.tables;
        let add_url = tables.len();
        let row_count = tables.len() + 2;
        for i in 0..row_count {
            let y = ROW_TOP + i as f32 * ROW_PITCH;
            let on = i == self.sel;
            let status = tables.get(i).and_then(|source| table_status(&source.location));
            let label = match tables.get(i) {
                Some(source) => source_label(source, status.as_ref()),
                None if i == add_url => "+ ADD TABLE (URL)".to_string(),
                None => "+ ADD TABLE (FILE)".to_string(),
            };
            canvas.fill_rect(Rect::new(x0, y, PANEL_W, ROW_H), if on { th.button } else { th.panel });
            let col = if i >= tables.len() {
                Color::GREEN
            } else if on {
                th.text
            } else {
                th.text_dim
            };
            let mut label_w = PANEL_W - ROW_PAD_X * 2.0;
            if let Some((summary, is_problem)) = status.as_ref().map(status_summary) {
                label_w -= text_width(&summary, STATUS_SCALE) + STATUS_GAP;
                let status_col = match (is_problem, on) {
                    (true, _) => th.warn,
                    (false, true) => th.text,
                    (false, false) => th.text_muted,
                };
                draw_text_right(canvas, x0 + PANEL_W - ROW_PAD_X, y + STATUS_DY, STATUS_SCALE, status_col, &summary);
            }
            draw_text(canvas, x0 + ROW_PAD_X, y + ROW_TEXT_DY, ROW_TEXT_SCALE, col, &rbms_render::fit_text(&label, ROW_TEXT_SCALE, label_w));
        }
        let focused = tables.get(self.sel).and_then(|source| table_status(&source.location));
        if let Some(detail) = focused.as_ref().and_then(status_detail) {
            let is_problem = !matches!(focused, Some(TableStatus::Loaded { stale: None, .. }));
            let detail: Vec<char> = detail.chars().collect();
            let top = ROW_TOP + row_count as f32 * ROW_PITCH + DETAIL_GAP;
            for (line, chars) in detail.chunks(DETAIL_LINE_CHARS).take(DETAIL_MAX_LINES).enumerate() {
                let text: String = chars.iter().collect();
                let fitted = rbms_render::fit_text(&text, DETAIL_SCALE, PANEL_W);
                draw_text(canvas, x0, top + line as f32 * DETAIL_PITCH, DETAIL_SCALE, if is_problem { th.warn } else { th.text_muted }, &fitted);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::render_tests::app;

    fn press(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text: None }
    }

    fn typed(text: &str) -> KeyInput<'_> {
        KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) }
    }

    /// A URL is fetched on a screen of its own, so the window keeps drawing while the request is
    /// out rather than freezing on the table manager.
    #[test]
    fn adding_a_url_leaves_for_the_loading_screen_rather_than_fetching_here() {
        let app = app();
        let mut state = TablesState::new();
        state.text_input = Some(TextEdit::from_text("https://example.test/table.json"));
        let moved = matches!(state.url_entry_key(&app.shared, &press(KeyCode::Enter)), Transition::Open(Stage::Loading(_)));
        assert!(moved, "adding a table has to leave the frame loop free");
        assert!(state.text_input.is_none(), "the entry is finished with");
        assert!(app.shared.config.library.tables.is_empty(), "nothing is added until the fetch comes back");
    }

    #[test]
    fn an_empty_or_repeated_url_is_refused_without_leaving_the_screen() {
        let mut app = app();
        app.shared.config.library.tables.push(TableSource { name: "have".into(), location: "https://example.test/have.json".into() });
        let mut state = TablesState::new();

        state.text_input = Some(TextEdit::from_text("   "));
        assert!(matches!(state.url_entry_key(&app.shared, &press(KeyCode::Enter)), Transition::Stay));

        state.text_input = Some(TextEdit::from_text("https://example.test/have.json"));
        assert!(matches!(state.url_entry_key(&app.shared, &press(KeyCode::Enter)), Transition::Stay));
        assert_eq!(app.shared.config.library.tables.len(), 1, "the same table is not added twice");
    }

    #[test]
    fn escape_abandons_what_was_typed_without_adding_it() {
        let app = app();
        let mut state = TablesState::new();
        state.text_input = Some(TextEdit::from_text("https://example.test/table.json"));
        assert!(matches!(state.url_entry_key(&app.shared, &press(KeyCode::Escape)), Transition::Stay));
        assert!(state.text_input.is_none());
        assert!(app.shared.config.library.tables.is_empty());
    }

    /// A URL is the one value in the app long enough to need editing rather than retyping.
    #[test]
    fn the_caret_keys_edit_the_typed_url_in_place() {
        let app = app();
        let mut state = TablesState::new();
        state.text_input = Some(TextEdit::from_text("http//x"));
        for key in [KeyCode::Home, KeyCode::ArrowRight, KeyCode::ArrowRight, KeyCode::ArrowRight, KeyCode::ArrowRight] {
            state.url_entry_key(&app.shared, &press(key));
        }
        state.url_entry_key(&app.shared, &typed("s:"));
        assert_eq!(state.text_input.as_ref().expect("still typing").text(), "https://x");

        state.url_entry_key(&app.shared, &press(KeyCode::End));
        state.url_entry_key(&app.shared, &press(KeyCode::Backspace));
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
        state.url_entry_key(&app.shared, &KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some("a") });
        assert_eq!(state.text_input.as_ref().expect("still typing").text(), "", "a shortcut is not typed into the line");

        state.handle_key(&mut ctx(&mut app), KeyInput { code: KeyCode::ControlLeft, pressed: false, released: true, text: None });
        assert!(!state.paste_held);
        state.url_entry_key(&app.shared, &typed("a"));
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
    /// library, so a picked file goes to the LOADING screen the same way a typed URL does rather
    /// than being parsed in the middle of a frame.
    #[test]
    fn a_picked_file_is_parsed_on_the_loading_screen_rather_than_in_a_frame() {
        let mut app = app();
        let mut state = TablesState::new();
        let (tx, handle) = crate::dialog::handle_for_tests();
        state.picker = Some(handle);
        tx.send(Some(std::path::PathBuf::from("/tables/insane.json"))).expect("the handle is still held");
        let moved = state.update(&mut ctx(&mut app));
        assert!(matches!(moved, Transition::Open(Stage::Loading(_))), "a picked file was not sent to the loading screen");
        assert!(app.shared.config.library.tables.is_empty(), "the source is added only once its table has been read");
    }

    /// A source already in the list is not added twice, and saying so does not open a screen.
    #[test]
    fn a_file_already_in_the_list_is_not_added_again() {
        let mut app = app();
        let mut state = TablesState::new();
        app.shared.config.library.tables.push(TableSource { name: "one".into(), location: "/tables/one.json".into() });
        let moved = state.add_picked_file(&app.shared, std::path::Path::new("/tables/one.json"));
        assert!(matches!(moved, Transition::Stay));
        assert_eq!(app.shared.config.library.tables.len(), 1);
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

    /// Register `file` the way the picker does and stay on the LOADING screen it opens until the
    /// table has been read and the screen hands back.
    fn register(app: &mut crate::App, file: &TableFile) {
        let mut state = TablesState::new();
        let Transition::Open(Stage::Loading(mut loading)) = state.add_picked_file(&app.shared, &file.0) else {
            panic!("a picked file was not sent to the loading screen");
        };
        loop {
            match loading.update(&mut ctx(app)) {
                Transition::Stay => std::thread::yield_now(),
                Transition::Back => return,
                _ => panic!("the loading screen left for somewhere other than the table manager"),
            }
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
        let loaded = TableStatus::Loaded { name: "\u{767a}\u{72c2}".to_string(), entries: 9, levels: 3, owned: 4, stale: None };
        let unnamed = TableSource { name: String::new(), location: "https://tables.test/t/".into() };
        assert_eq!(source_label(&unnamed, None), "https://tables.test/t/");
        assert_eq!(source_label(&unnamed, Some(&loaded)), "\u{767a}\u{72c2}  -  https://tables.test/t/");
        assert_eq!(source_label(&unnamed, Some(&TableStatus::Failed { reason: "x".into() })), "https://tables.test/t/");
        let named = TableSource { name: "\u{7b2c}2".into(), location: "/tables/second.json".into() };
        assert_eq!(source_label(&named, Some(&loaded)), "\u{7b2c}2  -  /tables/second.json", "a name in any script is a name");
    }

    #[test]
    fn each_status_reads_differently_at_a_glance_and_in_full() {
        let fresh = TableStatus::Loaded { name: "T".into(), entries: 1035, levels: 26, owned: 1035, stale: None };
        assert_eq!(status_summary(&fresh), ("1035 OF 1035 CHARTS OWNED  -  26 LEVELS".to_string(), false));
        assert_eq!(status_detail(&fresh), None, "a table that loaded and is owned needs no more said");

        let stale = TableStatus::Loaded { name: "T".into(), entries: 10, levels: 2, owned: 3, stale: Some("address: HTTP 503 Service Unavailable".into()) };
        assert_eq!(status_summary(&stale), ("SAVED COPY  -  3 OF 10 CHARTS OWNED".to_string(), true));
        assert_eq!(status_detail(&stale).as_deref(), Some("NOT REFRESHED: address: HTTP 503 Service Unavailable"));

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

        crate::tablesrc::load_and_match_md5s(&app.shared.config.library.tables[0], &[]);
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
}
