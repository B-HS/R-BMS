//! The difficulty-table manager: the list of table sources, the URL/file pickers that add one, and
//! the removal that drops its matched levels with it.
//!
//! Neither way of adding one stops the app: the file picker is a native panel the window keeps
//! drawing behind, and the table itself — fetched from a URL or read off disk — is parsed and
//! matched against the library on the LOADING screen rather than on the frame loop.
#![allow(clippy::wildcard_imports)]

use crate::dialog::{DialogHandle, DialogState};
use crate::stage::{Canvas, FrameCtx, KeyInput, LoadingState, Stage, StageHandler, Transition};
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
            shared.config.library.tables.remove(idx);
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
        for i in 0..tables.len() + 2 {
            let y = ROW_TOP + i as f32 * ROW_PITCH;
            let on = i == self.sel;
            let label = if i < tables.len() {
                let s = &tables[i];
                let nm = if s.name.is_ascii() && !s.name.trim().is_empty() { s.name.clone() } else { String::new() };
                let loc: String = s.location.chars().take(LOCATION_CHARS).collect();
                if nm.is_empty() { loc } else { format!("{nm}  -  {loc}") }
            } else if i == add_url {
                "+ ADD TABLE (URL)".to_string()
            } else {
                "+ ADD TABLE (FILE)".to_string()
            };
            canvas.fill_rect(Rect::new(x0, y, PANEL_W, ROW_H), if on { th.button } else { th.panel });
            let col = if i >= tables.len() {
                Color::GREEN
            } else if on {
                th.text
            } else {
                th.text_dim
            };
            draw_text(canvas, x0 + 16.0, y + 10.0, 1.5, col, &label);
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
