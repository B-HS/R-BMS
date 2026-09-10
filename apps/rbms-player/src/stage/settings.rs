//! The settings screen: the tabbed row list, the in-place text editor the NETWORK rows use, and
//! the inline rival list. The rows themselves are described in `rbms_config::SETTINGS`; this file
//! owns the screen's state and routes keys and clicks into them by name.
#![allow(clippy::wildcard_imports)]

use crate::dialog::{DialogHandle, DialogState};
use crate::ir_panel::*;
use crate::ir_session::{AuthAction, GUEST_PLAYER_ID};
use crate::settings_ui::{SettingRow, audio_status_text, is_network_row, step_audio_device};
use crate::settings_view::{RivalsScene, SettingsScene};
use crate::skin_select::SkinRow;
use crate::stage::{Canvas, FrameCtx, KeyConfigState, KeyInput, Stage, StageHandler, Transition};
use rbms_config::{AdjustOutcome, SettingId, SettingTab, adjust, step_skin};

use crate::*;

/// One step to the right on a row, which is what the right arrow, Enter and a click all ask for.
const ROW_STEP_FORWARD: i32 = 1;

/// One step to the left on a row.
const ROW_STEP_BACK: i32 = -1;

/// The settings screen's own state: the open tab and row, the rows it is showing, the in-place
/// editor, and the inline rival list.
///
/// `lines` holds the labels and values of the open tab and is rebuilt only when `dirty` says
/// something moved, so a screen sitting still costs no formatting. `audio_devices` is the host's
/// output device list, re-read on every entry so the AUDIO DEVICE row cycles through what is
/// plugged in now without enumerating on every keystroke.
pub(crate) struct SettingsState {
    tab: usize,
    sel: usize,
    text_input: Option<TextEdit>,
    /// Whether the in-place editor must hide what is being typed.
    text_secret: bool,
    /// Settings row the open in-place editor belongs to. The commit writes this row's field, so a
    /// selection that moves while the editor is open (a mouse click) cannot redirect the value —
    /// which would otherwise persist a typed password into another row.
    text_edit_row: Option<SettingId>,
    /// Whether a paste modifier is down, which is what turns V into a clipboard paste rather than
    /// a typed character.
    paste_held: bool,
    rivals_open: bool,
    rivals_sel: usize,
    audio_devices: Vec<String>,
    /// The font picker while it is up. It runs beside the frame loop, so the screen keeps drawing
    /// while the user is in front of it and the answer is collected in `update`.
    font_picker: Option<DialogHandle>,
    rows: Vec<SettingRow>,
    lines: Vec<(String, String)>,
    dirty: bool,
}

impl SettingsState {
    pub(crate) fn new() -> SettingsState {
        SettingsState::on_tab(SettingTab::ALL[0])
    }

    /// A settings screen opened straight onto one tab.
    pub(crate) fn on_tab(tab: SettingTab) -> SettingsState {
        SettingsState {
            tab: SettingTab::ALL.iter().position(|entry| *entry == tab).unwrap_or_default(),
            sel: 0,
            text_input: None,
            text_secret: false,
            text_edit_row: None,
            paste_held: false,
            rivals_open: false,
            rivals_sel: 0,
            audio_devices: Vec::new(),
            font_picker: None,
            rows: Vec::new(),
            lines: Vec::new(),
            dirty: true,
        }
    }

    /// The open tab.
    fn current_tab(&self) -> SettingTab {
        SettingTab::ALL[self.tab.min(SettingTab::ALL.len() - 1)]
    }

    /// Re-read the open tab's rows and their values.
    fn refresh(&mut self, shared: &AppShared) {
        self.rows = shared.settings_rows(self.current_tab());
        self.lines = self.rows.iter().map(|&row| shared.settings_line(row)).collect();
        self.sel = self.sel.min(self.rows.len().saturating_sub(1));
        self.dirty = false;
    }

    /// Re-read the rows if anything has moved since they were last built.
    fn ensure_rows(&mut self, shared: &AppShared) {
        if self.dirty {
            self.refresh(shared);
        }
    }

    /// The row the cursor is on.
    fn focused(&self) -> Option<SettingRow> {
        self.rows.get(self.sel).copied()
    }

    /// The descriptor-table row the cursor is on, or `None` when it is on a skin document's own.
    fn focused_fixed(&self) -> Option<SettingId> {
        match self.focused() {
            Some(SettingRow::Fixed(id)) => Some(id),
            _ => None,
        }
    }

    /// The AUDIO tab's status line: why the stream is not exactly what the rows ask for.
    fn audio_status_line(&self, shared: &AppShared) -> Option<String> {
        if self.current_tab() != SettingTab::Audio {
            return None;
        }
        Some(audio_status_text(shared.audio_report.as_ref(), shared.audio_failed))
    }

    /// Apply one left/right step to a row. A row the configuration document cannot step on its own
    /// comes back as an action this screen runs.
    fn step(&mut self, ctx: &mut FrameCtx<'_>, row: SettingRow, delta: i32) -> Transition {
        let id = match row {
            SettingRow::Fixed(id) => id,
            SettingRow::Skin(row) => return self.step_skin(ctx, row, delta),
        };
        let outcome = adjust(&mut ctx.shared.config, id, delta);
        if let AdjustOutcome::Action(action) = outcome {
            return self.run_action(ctx, action, delta);
        }
        if outcome == AdjustOutcome::Changed {
            self.dirty = true;
            if id == SettingId::SkinScreen {
                ctx.shared.reload_skin();
            }
        }
        match id {
            SettingId::MasterVolume | SettingId::KeyVolume | SettingId::BgmVolume | SettingId::SystemVolume => ctx.shared.apply_audio_gains(),
            SettingId::SyncSettings | SettingId::AutoUploadReplay => ctx.shared.save_settings(),
            _ => {}
        }
        Transition::Stay
    }

    /// A row that opens a screen or a picker, steps something only this process can enumerate, or
    /// reaches the score server. Stepping left runs nothing but the two rows that have a meaningful
    /// other direction: the font, which resets, and the skin, which cycles either way.
    fn run_action(&mut self, ctx: &mut FrameCtx<'_>, id: SettingId, delta: i32) -> Transition {
        self.dirty = true;
        match id {
            SettingId::KeyConfig => {
                if delta > 0 {
                    return Transition::Open(Stage::KeyConfig(KeyConfigState::new()));
                }
            }
            SettingId::Font => {
                if delta < 0 {
                    ctx.shared.reset_font();
                } else if self.font_picker.is_none() {
                    self.font_picker = Some(crate::dialog::pick_font_file());
                }
            }
            SettingId::Skin => {
                ctx.shared.launch.skin_path = None;
                step_skin(&mut ctx.shared.config);
            }
            SettingId::AudioDevice => {
                step_audio_device(&mut ctx.shared.config.audio, delta, &self.audio_devices);
            }
            SettingId::SkinDocument => {
                if ctx.shared.cycle_skin_document(delta) {
                    ctx.shared.reload_skin();
                }
            }
            SettingId::SkinReload => {
                if delta > 0 {
                    ctx.shared.rescan_skins();
                    ctx.shared.reload_skin();
                    ctx.shared.save_settings();
                }
            }
            SettingId::SkinReset => {
                if delta > 0 {
                    ctx.shared.reset_skin_document();
                    ctx.shared.save_settings();
                }
            }
            _ => {
                if delta > 0 {
                    self.enter_network_row(ctx.shared, id);
                }
            }
        }
        Transition::Stay
    }

    /// Step one of the chosen document's own customisation rows.
    ///
    /// A property or a file changes what is drawn, so the document is read again; an offset is read
    /// live out of the stored choices and needs no reload.
    fn step_skin(&mut self, ctx: &mut FrameCtx<'_>, row: SkinRow, delta: i32) -> Transition {
        if ctx.shared.step_skin_row(row, delta) {
            self.dirty = true;
            if ctx.shared.skin_reload_pending() {
                ctx.shared.reload_skin();
            }
        }
        Transition::Stay
    }

    /// Put the focused offset row back to what the document's author chose, which is the one row
    /// kind with a value a key can zero rather than cycle.
    fn reset_focused_row(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some(SettingRow::Skin(row)) = self.focused() else {
            return;
        };
        if ctx.shared.reset_skin_row(row) {
            self.dirty = true;
        }
    }

    /// Collect the font picker's answer once the user has given one, and load what they chose.
    ///
    /// The picker runs beside the frame loop, so this is where a chosen font actually lands. A
    /// dismissed picker leaves the font alone.
    fn poll_font_picker(&mut self, shared: &mut AppShared) {
        let Some(picker) = self.font_picker.as_ref() else {
            return;
        };
        match picker.poll() {
            DialogState::Open => return,
            DialogState::Picked(path) => {
                self.font_picker = None;
                shared.apply_font(&path);
            }
            DialogState::Dismissed => self.font_picker = None,
        }
        self.dirty = true;
    }

    /// Enter (or a step to the right) on a NETWORK row: open its editor, or run what it names.
    fn enter_network_row(&mut self, shared: &mut AppShared, id: SettingId) {
        if is_text_row(id) {
            self.begin_network_edit(shared, id);
            return;
        }
        match id {
            SettingId::Login => shared.start_auth(AuthAction::Login),
            SettingId::Register => shared.start_auth(AuthAction::Register),
            SettingId::Logout => shared.finish_logout(),
            SettingId::UploadSettings => shared.start_settings_upload(),
            SettingId::DownloadSettings => shared.start_settings_download(),
            SettingId::Rivals => {
                self.rivals_open = true;
                self.rivals_sel = 0;
                self.cancel_text_edit();
            }
            SettingId::Account => shared.net_status = shared.session.status_text(),
            _ => {}
        }
    }

    /// Open the in-place editor for a NETWORK text row, pre-filled with the current value. The
    /// password row starts empty and never shows what is already held.
    ///
    /// The row is remembered in `text_edit_row` so the commit writes the field the editor was
    /// opened on even if the selection has since moved (a mouse click can move it while an editor
    /// is open); committing a typed password into another row's field would persist it in clear.
    fn begin_network_edit(&mut self, shared: &AppShared, id: SettingId) {
        self.text_secret = is_secret_row(id);
        self.text_edit_row = Some(id);
        self.text_input = Some(TextEdit::from_text(match id {
            SettingId::PlayerId => shared.config.network.player_id.clone(),
            SettingId::ServerUrl => shared.config.network.server_url.clone().unwrap_or_default(),
            SettingId::Email => shared.config.network.ir_email.clone().unwrap_or_default(),
            _ => String::new(),
        }));
    }

    /// Close any open in-place editor, dropping what was typed. Called whenever the focus leaves
    /// the row the editor belongs to, so a half-typed secret never outlives its row.
    fn cancel_text_edit(&mut self) {
        self.text_input = None;
        self.text_secret = false;
        self.text_edit_row = None;
    }

    /// Commit an edited NETWORK text row. A blank SERVER URL means offline, a blank PLAYER ID means
    /// guest, and the password is held in memory only until the next LOGIN/REGISTER consumes it.
    /// A row that holds no text writes nothing — there is no catch-all field.
    fn commit_network_edit(&mut self, shared: &mut AppShared, id: SettingId, value: String) {
        let Some(field) = network_text_field(id) else {
            return;
        };
        match field {
            NetworkTextField::ServerUrl => {
                shared.config.network.server_url = (!value.is_empty()).then_some(value);
                shared.rebuild_server();
            }
            NetworkTextField::PlayerId => shared.config.network.player_id = if value.is_empty() { GUEST_PLAYER_ID.to_string() } else { value },
            NetworkTextField::Email => shared.config.network.ir_email = (!value.is_empty()).then_some(value),
            NetworkTextField::Password => {
                shared.password = value;
                return;
            }
        }
        shared.save_settings();
    }

    /// Edit the active NETWORK text field. Enter commits, Esc cancels. Everything but a secret row
    /// is trimmed; a password keeps the exact characters typed. The row the editor was **opened
    /// on** decides which field is written, not the row that happens to be focused at commit time.
    fn text_input_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) {
        match key.code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let raw = self.text_input.take().map(|mut edit| edit.take()).unwrap_or_default();
                let value = if self.text_secret { raw } else { raw.trim().to_string() };
                let edited = self.text_edit_row.take();
                self.text_secret = false;
                if let Some(id) = edited {
                    self.commit_network_edit(shared, id, value);
                }
            }
            KeyCode::Escape => self.cancel_text_edit(),
            _ => {
                if let Some(edit) = self.text_input.as_mut() {
                    edit_key(edit, key, self.paste_held);
                }
            }
        }
    }

    /// Keys inside the inline rival list. Typing an id runs through the same buffer as the settings
    /// rows, so only one editor is ever open.
    fn rivals_input(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) {
        if self.text_input.is_some() {
            match key.code {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let value = self.text_input.take().map(|mut edit| edit.take()).unwrap_or_default();
                    if let Some(id) = normalise_rival(&value, &shared.config.network.rivals) {
                        shared.config.network.rivals.push(id);
                        self.rivals_sel = shared.config.network.rivals.len().saturating_sub(1);
                    }
                }
                KeyCode::Escape => self.text_input = None,
                _ => {
                    if let Some(edit) = self.text_input.as_mut() {
                        edit_key(edit, key, self.paste_held);
                    }
                }
            }
            return;
        }
        let rows = rival_rows(&shared.config.network.rivals).len();
        let on_add = self.rivals_sel + 1 >= rows;
        match key.code {
            KeyCode::Escape => {
                self.rivals_open = false;
                shared.push_rivals();
            }
            KeyCode::ArrowUp => self.rivals_sel = self.rivals_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.rivals_sel = (self.rivals_sel + 1).min(rows.saturating_sub(1)),
            KeyCode::Enter | KeyCode::NumpadEnter if on_add => {
                self.text_input = Some(TextEdit::new());
                self.text_secret = false;
                self.text_edit_row = None;
            }
            KeyCode::KeyD if !on_add && self.rivals_sel < shared.config.network.rivals.len() => {
                shared.config.network.rivals.remove(self.rivals_sel);
                self.rivals_sel = self.rivals_sel.min(rival_rows(&shared.config.network.rivals).len().saturating_sub(1));
            }
            _ => {}
        }
    }

    /// Click inside the inline rival list: focus a row, or open the editor on the add row.
    fn rivals_click(&mut self, shared: &AppShared, index: usize) {
        let rows = rival_rows(&shared.config.network.rivals).len();
        self.rivals_sel = index.min(rows.saturating_sub(1));
        if self.rivals_sel + 1 >= rows {
            self.text_input = Some(TextEdit::new());
            self.text_secret = false;
            self.text_edit_row = None;
        }
    }

    /// Everything the settings screen draws this frame. The AUDIO tab owns the status line while it
    /// is open: it is the only place the engine's open report, and a reopen held back by a running
    /// chart, are visible.
    fn scene(&self, shared: &AppShared) -> SettingsScene {
        let editor = match (&self.text_input, self.rivals_open) {
            (Some(edit), false) => Some(editor_display(edit, self.text_secret)),
            _ => None,
        };
        let rivals = self.rivals_open.then(|| RivalsScene {
            rows: rival_rows(&shared.config.network.rivals),
            sel: self.rivals_sel,
            editor: self.text_input.as_ref().map(|edit| editor_display(edit, false)),
        });
        SettingsScene {
            tabs: SettingTab::ALL.iter().map(|tab| tab.label()).collect(),
            tab: self.tab.min(SettingTab::ALL.len() - 1),
            rows: self.lines.clone(),
            sel: self.sel.min(self.lines.len().saturating_sub(1)),
            editor,
            status: self.audio_status_line(shared).unwrap_or_else(|| shared.net_status.clone()),
            rivals,
        }
    }

    /// Keys on the row list itself, once the editor and the rival list have had their turn.
    fn row_key(&mut self, ctx: &mut FrameCtx<'_>, key: &KeyInput<'_>) -> Transition {
        let focused = self.focused();
        match key.code {
            KeyCode::Escape => {
                ctx.shared.save_settings();
                return Transition::Back;
            }
            KeyCode::Tab => {
                self.tab = (self.tab + 1) % SettingTab::ALL.len();
                self.sel = 0;
                self.dirty = true;
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let Some(row) = focused else {
                    return Transition::Stay;
                };
                let Some(id) = self.focused_fixed() else {
                    return self.step(ctx, row, ROW_STEP_FORWARD);
                };
                match id {
                    SettingId::KeyConfig => return Transition::Open(Stage::KeyConfig(KeyConfigState::new())),
                    SettingId::Font => return self.run_action(ctx, SettingId::Font, ROW_STEP_FORWARD),
                    _ if is_network_row(id) || is_skin_action_row(id) => return self.step(ctx, row, ROW_STEP_FORWARD),
                    _ => {
                        ctx.shared.save_settings();
                        return Transition::Back;
                    }
                }
            }
            KeyCode::ArrowUp => self.sel = self.sel.saturating_sub(1),
            KeyCode::ArrowDown => self.sel = (self.sel + 1).min(self.rows.len().saturating_sub(1)),
            KeyCode::ArrowLeft => {
                if let Some(row) = focused {
                    return self.step(ctx, row, ROW_STEP_BACK);
                }
            }
            KeyCode::ArrowRight => {
                if let Some(row) = focused {
                    return self.step(ctx, row, ROW_STEP_FORWARD);
                }
            }
            KeyCode::Digit0 | KeyCode::Numpad0 => self.reset_focused_row(ctx),
            _ => {}
        }
        Transition::Stay
    }
}

/// Whether Enter on this row runs something rather than leaving the screen: the two SKIN actions
/// and the document row, which Enter steps the way the right arrow does.
fn is_skin_action_row(id: SettingId) -> bool {
    matches!(id, SettingId::SkinDocument | SettingId::SkinReload | SettingId::SkinReset)
}

impl StageHandler for SettingsState {
    /// Re-read the host's output device list on the way in, so the AUDIO DEVICE row cycles through
    /// what is plugged in now; walk the skin folder, so a document dropped into it while the game
    /// was running is offered; and build the rows the first tab shows.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        self.audio_devices = rbms_audio::output_device_names();
        ctx.shared.rescan_skins();
        if ctx.shared.skin_reload_pending() {
            ctx.shared.reload_skin();
        }
        self.refresh(ctx.shared);
    }

    /// Keep the rival cursor inside the list, which a background refresh can shorten.
    ///
    /// The ACCOUNT row is the one value a worker thread can change while the screen sits still, so
    /// the tab holding it rebuilds its rows every frame; every other tab keeps the rows it has
    /// until a key or a click moves one.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.poll_font_picker(ctx.shared);
        let rows = rival_rows(&ctx.shared.config.network.rivals).len();
        self.rivals_sel = self.rivals_sel.min(rows.saturating_sub(1));
        if self.rows.contains(&SettingRow::Fixed(SettingId::Account)) {
            self.dirty = true;
        }
        Transition::Stay
    }

    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if matches!(key.code, KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::SuperLeft | KeyCode::SuperRight) {
            self.paste_held = key.pressed;
            return Transition::Stay;
        }
        if !key.pressed {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        if self.rivals_open {
            self.rivals_input(ctx.shared, &key);
            self.dirty = true;
            return Transition::Stay;
        }
        if self.text_input.is_some() {
            self.text_input_key(ctx.shared, &key);
            self.dirty = true;
            return Transition::Stay;
        }
        self.row_key(ctx, &key)
    }

    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition {
        self.ensure_rows(ctx.shared);
        match ctx.shared.hit_test(at) {
            Some(Hot::SettingTab(_) | Hot::SettingRow(_)) if self.rivals_open => {}
            Some(Hot::SettingTab(ti)) => {
                self.cancel_text_edit();
                self.tab = ti.min(SettingTab::ALL.len() - 1);
                self.sel = 0;
                self.dirty = true;
            }
            Some(Hot::SettingRow(i)) => {
                if let Some(row) = self.rows.get(i).copied() {
                    self.cancel_text_edit();
                    self.sel = i;
                    return self.step(ctx, row, ROW_STEP_FORWARD);
                }
            }
            Some(Hot::RivalRow(i)) => self.rivals_click(ctx.shared, i),
            _ => {}
        }
        Transition::Stay
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        self.ensure_rows(ctx.shared);
        let scene = self.scene(ctx.shared);
        canvas.clear_bga();
        let hot = render_settings(canvas, &scene);
        ctx.shared.hot.extend(hot.into_iter().map(|(rect, h)| {
            let mapped = match h {
                SettingsHot::Tab(i) => Hot::SettingTab(i),
                SettingsHot::Row(i) => Hot::SettingRow(i),
                SettingsHot::RivalRow(i) => Hot::RivalRow(i),
            };
            (rect, mapped)
        }));
    }
}

#[cfg(test)]
mod skin_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::render_tests::app;

    pub(super) fn press(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text: None }
    }

    fn typed(text: &str) -> KeyInput<'_> {
        KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) }
    }

    /// The in-place editor is the same editor the table manager's URL entry is, so a value typed
    /// into a settings row can be corrected in the middle rather than only from the end.
    #[test]
    fn a_settings_row_can_be_edited_anywhere_in_the_line() {
        let mut app = app();
        let mut state = SettingsState::on_tab(SettingTab::Network);
        state.text_input = Some(TextEdit::from_text("dj@exampletest"));

        state.text_input_key(&mut app.shared, &press(KeyCode::Home));
        for _ in 0..10 {
            state.text_input_key(&mut app.shared, &press(KeyCode::ArrowRight));
        }
        state.text_input_key(&mut app.shared, &typed("."));
        assert_eq!(state.text_input.as_ref().expect("the editor is still open").text(), "dj@example.test");

        state.text_input_key(&mut app.shared, &press(KeyCode::End));
        state.text_input_key(&mut app.shared, &press(KeyCode::Backspace));
        assert_eq!(state.text_input.as_ref().expect("the editor is still open").text(), "dj@example.tes");
    }

    /// A password on screen carries neither the password nor, once it is long, its length.
    #[test]
    fn a_secret_row_shows_a_capped_mask_and_never_the_value() {
        let secret = TextEdit::from_text("a-very-long-passphrase-indeed");
        let shown = editor_display(&secret, true);
        assert!(!shown.contains("passphrase"), "the value reached the screen");
        assert!(shown.chars().filter(|c| *c == PASSWORD_MASK_CHAR).count() <= PASSWORD_MASK_MAX, "the mask reports the length");
    }

    /// A value too long for the column scrolls under the caret rather than showing only its start.
    #[test]
    fn a_long_value_shows_the_stretch_the_caret_is_in() {
        let long = TextEdit::from_text("https://ir.example.test/a/very/long/endpoint/path/that/does/not/fit");
        let at_end = editor_display(&long, false);
        let mut at_start = long.clone();
        at_start.home();
        assert_ne!(at_end, editor_display(&at_start, false), "the window does not follow the caret");
        assert!(at_end.ends_with('_'), "the caret is at the end of the line it is at the end of");
    }

    /// A native picker can sit in front of the user for as long as they like, so the font row opens
    /// it beside the frame loop and collects the answer in `update` — the window keeps drawing while
    /// it is up, and the font lands when they choose one.
    #[test]
    fn the_font_row_waits_for_its_picker_off_the_frame_loop() {
        let mut app = app();
        let mut state = SettingsState::on_tab(SettingTab::Display);
        let (tx, handle) = crate::dialog::handle_for_tests();
        state.font_picker = Some(handle);
        state.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
        assert!(state.font_picker.is_some(), "the screen gave up on a picker nobody has answered");

        tx.send(None).expect("the handle is still held");
        state.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
        assert!(state.font_picker.is_none(), "the screen kept waiting after the picker was dismissed");
        assert_eq!(app.shared.config.display.font_path, None, "a dismissed picker changed the font");
    }

    /// A font the user picked is loaded and written out, so it is the face the next launch starts
    /// with. A file that is not a font is reported rather than stored.
    #[test]
    fn a_picked_font_that_cannot_be_read_is_not_stored() {
        let mut app = app();
        let mut state = SettingsState::on_tab(SettingTab::Display);
        let (tx, handle) = crate::dialog::handle_for_tests();
        state.font_picker = Some(handle);
        tx.send(Some(std::path::PathBuf::from("/fonts/no-such-face.ttf"))).expect("the handle is still held");
        state.update(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
        assert!(state.font_picker.is_none());
        assert_eq!(app.shared.config.display.font_path, None, "a font that could not be read was stored anyway");
    }
}
