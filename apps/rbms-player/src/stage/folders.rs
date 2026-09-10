//! The song-folder manager: the list of picked library folders, the picker that adds one, and the
//! rescan that merges the result back into the library on the way out.
//!
//! The picker is a native panel the window keeps drawing behind rather than a call that stops the
//! app until the user answers it.
#![allow(clippy::wildcard_imports)]

use crate::dialog::{DialogHandle, DialogState};
use crate::stage::{Canvas, FrameCtx, KeyInput, LoadingState, Stage, StageHandler, Transition};
use crate::*;

/// Width of the folder list panel.
const PANEL_W: f32 = 980.0;

/// Path length past which the head of a folder path is elided so its name stays visible.
const PATH_ELIDE_CHARS: usize = 78;
const PATH_TAIL_CHARS: usize = 76;

/// Vertical layout of the folder rows.
const ROW_TOP: f32 = 124.0;
const ROW_PITCH: f32 = 44.0;
const ROW_H: f32 = 36.0;

/// The folder manager's own state: which row is focused, and the picker while it is up.
#[derive(Default)]
pub(crate) struct FoldersState {
    sel: usize,
    picker: Option<DialogHandle>,
}

impl FoldersState {
    pub(crate) fn new() -> FoldersState {
        FoldersState::default()
    }

    /// The folders plus a trailing "+ ADD FOLDER" row.
    fn row_count(shared: &AppShared) -> usize {
        shared.config.library.folders.len() + 1
    }

    /// Add the folder the picker came back with (deduped, persisted). The merged rescan happens
    /// when the user leaves the Folders screen, so several folders can be added in one visit.
    fn add_picked_folder(shared: &mut AppShared, dir: &std::path::Path) {
        let path = dir.to_string_lossy().to_string();
        if shared.config.library.folders.iter().any(|f| f == &path) {
            notify(Level::Warn, format!("folder already added: {path}"));
            return;
        }
        shared.config.library.folders.push(path);
        shared.save_settings();
    }

    fn remove_folder(&mut self, shared: &mut AppShared, idx: usize) {
        if idx < shared.config.library.folders.len() {
            shared.config.library.folders.remove(idx);
            shared.save_settings();
            self.sel = self.sel.min(FoldersState::row_count(shared).saturating_sub(1));
        }
    }
}

impl StageHandler for FoldersState {
    /// Collect the picker's answer once the user has given one, which is where an added folder
    /// actually lands.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        let Some(picker) = self.picker.as_ref() else {
            return Transition::Stay;
        };
        match picker.poll() {
            DialogState::Open => return Transition::Stay,
            DialogState::Picked(path) => {
                self.picker = None;
                FoldersState::add_picked_folder(ctx.shared, &path);
            }
            DialogState::Dismissed => self.picker = None,
        }
        Transition::Stay
    }

    /// Keys inside the folder manager. Leaving it rescans, so a folder added or removed here is
    /// merged back into the library on the way out. Keys are ignored while the native picker is up,
    /// which owns the keyboard itself.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if !key.pressed || self.picker.is_some() {
            return Transition::Stay;
        }
        let n = FoldersState::row_count(ctx.shared);
        let add_row = ctx.shared.config.library.folders.len();
        match key.code {
            KeyCode::Escape => return Transition::To(Stage::Loading(LoadingState::rescan(ctx.shared))),
            KeyCode::ArrowUp => self.sel = self.sel.saturating_sub(1),
            KeyCode::ArrowDown => self.sel = (self.sel + 1).min(n.saturating_sub(1)),
            KeyCode::KeyD | KeyCode::Delete if self.sel < ctx.shared.config.library.folders.len() => self.remove_folder(ctx.shared, self.sel),
            KeyCode::Enter | KeyCode::NumpadEnter if self.sel == add_row => self.picker = Some(crate::dialog::pick_song_folder()),
            _ => {}
        }
        Transition::Stay
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        canvas.clear(th.bg);
        let x0 = (CW as f32 - PANEL_W) * 0.5;
        draw_text(canvas, x0, 40.0, 3.0, th.text, "SONG FOLDERS");
        match self.picker.is_some() {
            true => draw_text(canvas, x0, 84.0, 1.4, Color::YELLOW, "PICK A SONG FOLDER IN THE OPEN DIALOG"),
            false => draw_text(canvas, x0, 84.0, 1.3, th.text_muted, "UP DOWN MOVE   ENTER ADD FOLDER   D REMOVE   ESC SAVE/RESCAN"),
        }
        let folders = &ctx.shared.config.library.folders;
        let add_row = folders.len();
        for i in 0..folders.len() + 1 {
            let y = ROW_TOP + i as f32 * ROW_PITCH;
            let on = i == self.sel;
            let label = if i < folders.len() {
                let f = &folders[i];
                if f.chars().count() > PATH_ELIDE_CHARS {
                    format!("…{}", f.chars().rev().take(PATH_TAIL_CHARS).collect::<Vec<_>>().into_iter().rev().collect::<String>())
                } else {
                    f.clone()
                }
            } else {
                "+ ADD FOLDER".to_string()
            };
            canvas.fill_rect(Rect::new(x0, y, PANEL_W, ROW_H), if on { th.button } else { th.panel });
            let col = if i == add_row {
                Color::GREEN
            } else if on {
                th.text
            } else {
                th.text_dim
            };
            draw_text(canvas, x0 + 16.0, y + 10.0, 1.4, col, &label);
        }
        if ctx.shared.config.library.folders.is_empty() {
            draw_text(canvas, x0, ROW_TOP + ROW_PITCH * 2.0, 1.2, th.text_muted, "No folders yet — ENTER on \"+ ADD FOLDER\" to pick one.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::render_tests::app;

    fn ctx(app: &mut crate::App) -> FrameCtx<'_> {
        FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
    }

    fn press(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text: None }
    }

    /// The picker is a native window with the keyboard; the screen behind it must not act on keys
    /// that were meant for it — least of all Escape, which here would leave and start a rescan.
    #[test]
    fn keys_are_ignored_while_the_picker_is_up() {
        let mut app = app();
        let mut state = FoldersState::new();
        let (_tx, handle) = crate::dialog::handle_for_tests();
        state.picker = Some(handle);
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::Escape)), Transition::Stay));
    }

    #[test]
    fn a_picked_folder_is_added_and_the_screen_stops_waiting() {
        let mut app = app();
        let mut state = FoldersState::new();
        let (tx, handle) = crate::dialog::handle_for_tests();
        state.picker = Some(handle);
        state.update(&mut ctx(&mut app));
        assert!(state.picker.is_some(), "an unanswered picker is still up");

        tx.send(Some(std::path::PathBuf::from("/songs/new"))).expect("the handle is still held");
        state.update(&mut ctx(&mut app));
        assert!(state.picker.is_none());
        assert_eq!(app.shared.config.library.folders, vec!["/songs/new".to_string()]);
    }

    #[test]
    fn the_same_folder_is_not_added_twice() {
        let mut app = app();
        app.shared.config.library.folders = vec!["/songs/new".to_string()];
        FoldersState::add_picked_folder(&mut app.shared, std::path::Path::new("/songs/new"));
        assert_eq!(app.shared.config.library.folders.len(), 1);
    }

    #[test]
    fn a_dismissed_picker_adds_nothing() {
        let mut app = app();
        let mut state = FoldersState::new();
        let (tx, handle) = crate::dialog::handle_for_tests();
        state.picker = Some(handle);
        tx.send(None).expect("the handle is still held");
        state.update(&mut ctx(&mut app));
        assert!(state.picker.is_none());
        assert!(app.shared.config.library.folders.is_empty());
    }
}
