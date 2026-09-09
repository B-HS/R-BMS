//! The difficulty-table manager: the list of table sources, the URL/file pickers that add one, and
//! the removal that drops its matched levels with it.
#![allow(clippy::wildcard_imports)]

use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::*;

/// Width of the table list panel.
const PANEL_W: f32 = 900.0;

/// How much of a source location the row shows before it is cut.
const LOCATION_CHARS: usize = 64;

/// How much of the typed URL the entry box shows, newest characters first.
const URL_ENTRY_CHARS: usize = 70;

/// Vertical layout of the table rows.
const ROW_TOP: f32 = 124.0;
const ROW_PITCH: f32 = 44.0;
const ROW_H: f32 = 36.0;

/// The table manager's own state: which row is focused, and the URL being typed when the "+ ADD
/// TABLE (URL)" row has been entered.
#[derive(Default)]
pub(crate) struct TablesState {
    sel: usize,
    text_input: Option<String>,
}

impl TablesState {
    pub(crate) fn new() -> TablesState {
        TablesState::default()
    }

    /// Rows in the table manager: each source, then the two add actions.
    fn row_count(shared: &AppShared) -> usize {
        shared.config.library.tables.len() + 2
    }

    fn add_table_file(&mut self, shared: &mut AppShared) {
        if let Some(path) = rfd::FileDialog::new().set_title("Select table json").add_filter("json", &["json"]).pick_file() {
            let location = path.to_string_lossy().to_string();
            if shared.config.library.tables.iter().any(|t| t.location == location) {
                eprintln!("table already added: {location}");
                return;
            }
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("table").to_string();
            shared.add_table_source(TableSource { name, location });
        }
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

    /// Typing the URL of a new table. Enter adds it, Esc abandons the entry.
    fn url_entry_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) {
        match key.code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let url = self.text_input.take().unwrap_or_default().trim().to_string();
                if !url.is_empty() {
                    if shared.config.library.tables.iter().any(|t| t.location == url) {
                        eprintln!("table already added: {url}");
                    } else {
                        shared.add_table_source(TableSource { name: String::new(), location: url });
                    }
                }
            }
            KeyCode::Escape => self.text_input = None,
            KeyCode::Backspace => {
                if let Some(b) = self.text_input.as_mut() {
                    b.pop();
                }
            }
            _ => {
                if let (Some(b), Some(t)) = (self.text_input.as_mut(), key.text) {
                    b.extend(t.chars().filter(|c| !c.is_control()));
                }
            }
        }
    }
}

impl StageHandler for TablesState {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    /// Table-manager input. In URL-text mode, type the URL (Enter adds, Esc cancels); otherwise
    /// navigate, add (URL/file), remove (D), or leave (Esc — rebuilds the browse list).
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if !key.pressed {
            return Transition::Stay;
        }
        if self.text_input.is_some() {
            self.url_entry_key(ctx.shared, &key);
            return Transition::Stay;
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
                    self.text_input = Some(String::new());
                } else if self.sel == add_file {
                    self.add_table_file(ctx.shared);
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
        if let Some(buf) = self.text_input.as_ref() {
            draw_text(canvas, x0, 84.0, 1.4, Color::YELLOW, "TYPE TABLE URL  -  ENTER ADD  ESC CANCEL");
            canvas.fill_rect(Rect::new(x0, 116.0, PANEL_W, 40.0), th.button_active);
            let shown: String = buf.chars().rev().take(URL_ENTRY_CHARS).collect::<Vec<_>>().into_iter().rev().collect();
            draw_text(canvas, x0 + 14.0, 128.0, 1.6, th.text, &format!("{shown}_"));
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
