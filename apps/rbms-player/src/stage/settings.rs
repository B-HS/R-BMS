//! The settings screen: the tabbed row list, the in-place text editor the NETWORK rows use, and
//! the inline rival list. The rows themselves are described in `rbms_config::SETTINGS`; this file
//! owns the screen's state and routes keys and clicks into them by name.
//!
//! The skin settings are not on this screen, and neither are the options of a sitting -- how fast the
//! notes scroll, what the lane is covered with, when a press counts and how a chart is dealt out.
//! They are the skin menu's ([`crate::stage::SkinMenuState`]) and the play menu's
//! ([`crate::stage::PlayMenuState`]), each of which lists its rows beside a preview of what they do.
//! This screen shows the tabs of [`SettingTab::SHOWN`] and a way to either menu: the key that opens it
//! from the song browser ([`Shortcut::PlayMenu`], [`Shortcut::SkinMenu`]), and its button at the end
//! of the tab strip. The key editor is reached the same two ways: the key that opens it from the
//! browser ([`Shortcut::KeyConfig`]), and the button above the other two. That button is not one of
//! the regions a frame registers ([`Hot`]): it is where the screen's layout says it is
//! ([`key_config_link`]), and a press is tested against that.
//!
//! A row that holds a number -- a volume, a judge width, a fade -- is set as such a row is on every
//! built-in screen ([`crate::number_row`]): an arrow moves it one step of the smallest change it
//! can hold, and ten with Shift held; its slider is dragged; and Enter, or a press on the number,
//! opens it to be typed into. A drag writes the settings file once, when the button comes back up.
#![allow(clippy::wildcard_imports)]

use crate::dialog::{DialogHandle, DialogState};
use crate::ir_panel::*;
use crate::ir_session::{AuthAction, GUEST_PLAYER_ID};
use crate::keyconfig::Shortcut;
use crate::number_row::{EDIT_HINT, EditKey, NumberInput, NumberValue, RowPart, hot_rect};
use crate::settings_ui::{SettingRow, audio_status_text, is_network_row, step_audio_device};
use crate::settings_view::{RivalsScene, SettingsLine, SettingsScene, key_config_link, number_part_at, number_row_hint, settings_hint};
use crate::stage::preview_menu::shift_after;
use crate::stage::{Canvas, FrameCtx, KeyConfigState, KeyInput, PlayMenuState, SkinMenuState, Stage, StageHandler, Transition, is_left_press};
use rbms_config::{AdjustOutcome, SettingId, SettingTab, adjust, step_skin};

use crate::*;

/// One step to the right on a row, which is what the right arrow, Enter and a click all ask for.
const ROW_STEP_FORWARD: i32 = 1;

/// One step to the left on a row.
const ROW_STEP_BACK: i32 = -1;

/// The settings screen's own state: the open tab and row, the rows it is showing, the in-place
/// editor, and the inline rival list.
///
/// `lines` holds the labels and values of the open tab, with what is said beside each, and is
/// rebuilt only when `dirty` says something moved, so a screen sitting still costs no formatting. `audio_devices` is the host's
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
    /// The IR profile list while it is open, and the row it is on. The list is drawn in the same
    /// inline panel the rivals use, so only one of the two is ever up.
    profiles_open: bool,
    profiles_sel: usize,
    audio_devices: Vec<String>,
    /// The system sound folder picker while it is up, on the same terms as the font picker below.
    sound_picker: Option<DialogHandle>,
    /// The font picker while it is up. It runs beside the frame loop, so the screen keeps drawing
    /// while the user is in front of it and the answer is collected in `update`.
    font_picker: Option<DialogHandle>,
    rows: Vec<SettingRow>,
    lines: Vec<SettingsLine>,
    dirty: bool,
    /// What the screen keeps about its rows that hold a number. Kept behind a pointer, because this
    /// state is moved with every screen change.
    numbers: Box<NumberRows>,
}

/// What the settings screen keeps about its rows that hold a number.
struct NumberRows {
    /// Which row is being typed into, and which is being dragged.
    input: NumberInput<SettingId>,
    /// Where a drag has carried a row whose number is stored when the drag ends
    /// ([`applies_when_released`]), until it does.
    carried: Option<(SettingId, i64)>,
    /// Whether Shift is held, which turns one step of a number into ten.
    shift_held: bool,
}

/// What the hint names as the key of a screen the player has left without one.
const NO_KEY: &str = "-";

/// Whether a row's number is stored when a drag of its slider ends rather than as the slider moves:
/// the count of voices reaches the output stream only by opening it again, which a slider must not
/// do at every place it passes.
fn applies_when_released(id: SettingId) -> bool {
    id == SettingId::AudioPolyphony
}

impl SettingsState {
    pub(crate) fn new() -> SettingsState {
        SettingsState::on_tab(SettingTab::SHOWN[0])
    }

    /// A settings screen opened straight onto one tab. A tab the screen does not show opens it on
    /// its first.
    pub(crate) fn on_tab(tab: SettingTab) -> SettingsState {
        SettingsState {
            tab: SettingTab::SHOWN.iter().position(|entry| *entry == tab).unwrap_or_default(),
            sel: 0,
            text_input: None,
            text_secret: false,
            text_edit_row: None,
            paste_held: false,
            rivals_open: false,
            rivals_sel: 0,
            profiles_open: false,
            profiles_sel: 0,
            audio_devices: Vec::new(),
            sound_picker: None,
            font_picker: None,
            rows: Vec::new(),
            lines: Vec::new(),
            dirty: true,
            numbers: Box::new(NumberRows { input: NumberInput::new(), carried: None, shift_held: false }),
        }
    }

    /// Whether one of the lists drawn over the rows, or an in-place editor, has the screen: the
    /// keys and the buttons that leave it for another screen stand down while one does.
    fn is_busy(&self) -> bool {
        self.rivals_open || self.profiles_open || self.text_input.is_some() || self.numbers.input.editing().is_some()
    }

    /// The open tab.
    pub(crate) fn current_tab(&self) -> SettingTab {
        SettingTab::SHOWN[self.tab.min(SettingTab::SHOWN.len() - 1)]
    }

    /// Re-read the open tab's rows and their values.
    ///
    /// A row a drag is carrying without storing it yet reads as where the drag has it.
    fn refresh(&mut self, shared: &AppShared) {
        self.rows = shared.settings_rows(self.current_tab());
        self.lines = self.rows.iter().map(|&row| shared.settings_scene_line(row)).collect();
        if let Some((id, steps)) = self.numbers.carried
            && let Some(line) = self.rows.iter().position(|row| *row == SettingRow::Fixed(id)).and_then(|at| self.lines.get_mut(at))
            && let Some(number) = line.number.as_mut()
        {
            number.steps = steps;
            line.value = number.spec.text(steps);
        }
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

    /// The descriptor-table row the cursor is on.
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

    /// The number one row holds as the screen shows it: what is stored, or where a drag has carried
    /// a row whose number is stored when the drag ends.
    fn number_of(&self, shared: &AppShared, id: SettingId) -> Option<NumberValue> {
        let stored = shared.setting_number(id)?;
        let steps = self.numbers.carried.filter(|(carried, _)| *carried == id).map_or(stored.steps, |(_, steps)| steps);
        Some(NumberValue { steps, ..stored })
    }

    /// The row under the cursor and the number it holds, when it holds one.
    fn focused_number(&self, shared: &AppShared) -> Option<(SettingId, NumberValue)> {
        let id = self.focused_fixed()?;
        self.number_of(shared, id).map(|value| (id, value))
    }

    /// Put one row that holds a number on the step `steps`. A volume reaches the output stream at
    /// once, so a slider dragged along one is heard as it moves.
    fn set_number(&mut self, ctx: &mut FrameCtx<'_>, id: SettingId, steps: i64) {
        if !ctx.shared.set_row_number(SettingRow::Fixed(id), steps) {
            return;
        }
        self.dirty = true;
        if matches!(id, SettingId::MasterVolume | SettingId::KeyVolume | SettingId::BgmVolume | SettingId::SystemVolume) {
            ctx.shared.apply_audio_gains();
        }
    }

    /// Store what was typed into the row being typed into, and close its editor. What is not a
    /// number is not stored, and the row says so for a while.
    fn commit_number_edit(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some((id, typed)) = self.numbers.input.take_edit() else {
            return;
        };
        let Some(value) = self.number_of(ctx.shared, id) else {
            return;
        };
        if let Some(steps) = self.numbers.input.typed_steps(id, &value.spec, &typed, ctx.now) {
            self.set_number(ctx, id, steps);
        }
    }

    /// Carry the row being dragged to where the pointer is along its slider. The number is stored
    /// as it moves, except for a row whose number is stored when the drag ends.
    fn drag_to(&mut self, ctx: &mut FrameCtx<'_>, x: f32) {
        let Some((id, _)) = self.numbers.input.dragged() else {
            return;
        };
        let Some(steps) = self.number_of(ctx.shared, id).and_then(|value| self.numbers.input.dragged_to(&value, x)) else {
            return;
        };
        if applies_when_released(id) {
            self.numbers.carried = Some((id, steps));
            self.dirty = true;
        } else {
            self.set_number(ctx, id, steps);
        }
    }

    /// End a drag: store the number of a row that was only being carried, and write the settings
    /// file, once for the whole drag.
    fn end_drag(&mut self, ctx: &mut FrameCtx<'_>) {
        if !self.numbers.input.end_drag() {
            return;
        }
        if let Some((id, steps)) = self.numbers.carried.take() {
            self.dirty = true;
            self.set_number(ctx, id, steps);
        }
        ctx.shared.save_settings();
    }

    /// A press of the left button at `at` on `row`, the row at `index` of the open tab. On a row
    /// that holds a number, a press on the slider begins a drag -- the row is put where the press
    /// landed, unless it landed on the knob, which is taken hold of where it stands -- and one on
    /// the number opens the row to be typed into. Any other press steps the row once.
    fn press_row(&mut self, ctx: &mut FrameCtx<'_>, row: SettingRow, index: usize, at: (f32, f32)) -> Transition {
        let SettingRow::Fixed(id) = row else {
            return Transition::Stay;
        };
        let part = hot_rect(ctx.shared, Hot::SettingRow(index)).map_or(RowPart::Rest, |drawn| number_part_at(drawn, at));
        match (self.number_of(ctx.shared, id), part) {
            (Some(value), RowPart::Slider(area)) => {
                if !self.numbers.input.begin_drag(id, area, &value, at.0) {
                    self.drag_to(ctx, at.0);
                }
            }
            (Some(value), RowPart::Number) => self.numbers.input.begin_edit(id, &value),
            (Some(_), RowPart::Rest) | (None, _) => return self.step(ctx, row, ROW_STEP_FORWARD, false),
        }
        Transition::Stay
    }

    /// A press of the right button at `at`: a row that holds a number goes one step back, so a step
    /// taken by mistake with the mouse can be taken back with it. Any other row, and every row under
    /// a list or an editor, is left alone.
    fn step_back_under(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) {
        if self.rivals_open || self.profiles_open || self.text_input.is_some() {
            return;
        }
        let Some(Hot::SettingRow(index)) = ctx.shared.hit_test(at) else {
            return;
        };
        let Some(SettingRow::Fixed(id)) = self.rows.get(index).copied() else {
            return;
        };
        self.commit_number_edit(ctx);
        if let Some(value) = self.number_of(ctx.shared, id) {
            self.sel = index;
            self.set_number(ctx, id, value.stepped(ROW_STEP_BACK, false));
        }
    }

    /// Apply one left/right step to a row. A row that holds a number moves one step of the smallest
    /// change it can hold, and ten when `wide` asks. A row the configuration document cannot step on
    /// its own comes back as an action this screen runs. A row a skin document declares is the skin
    /// menu's, and no tab of this screen lists one.
    fn step(&mut self, ctx: &mut FrameCtx<'_>, row: SettingRow, delta: i32, wide: bool) -> Transition {
        let SettingRow::Fixed(id) = row else {
            return Transition::Stay;
        };
        if let Some(value) = self.number_of(ctx.shared, id) {
            self.set_number(ctx, id, value.stepped(delta, wide));
            return Transition::Stay;
        }
        let outcome = adjust(&mut ctx.shared.config, id, delta);
        if let AdjustOutcome::Action(action) = outcome {
            return self.run_action(ctx, action, delta);
        }
        if outcome == AdjustOutcome::Changed {
            self.dirty = true;
        }
        match id {
            SettingId::MasterVolume | SettingId::KeyVolume | SettingId::BgmVolume | SettingId::SystemVolume => ctx.shared.apply_audio_gains(),
            SettingId::SyncSettings | SettingId::AutoUploadReplay => ctx.shared.save_settings(),
            _ => {}
        }
        Transition::Stay
    }

    /// A row that opens a screen or a picker, steps something only this process can enumerate, or
    /// reaches the score server. Stepping left runs nothing but the rows that have a meaningful
    /// other direction: the font and the sound folder, which reset, and the device, which cycles
    /// either way.
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
                ctx.shared.drop_launch_layout();
                step_skin(&mut ctx.shared.config);
            }
            SettingId::AudioDevice => {
                step_audio_device(&mut ctx.shared.config.audio, delta, &self.audio_devices);
            }
            SettingId::SoundFolder => {
                if delta < 0 {
                    ctx.shared.config.audio.sound_folder = None;
                    ctx.shared.reload_system_sounds();
                } else if self.sound_picker.is_none() {
                    self.sound_picker = Some(crate::dialog::pick_sound_folder());
                }
            }
            SettingId::GuideSe => {
                ctx.shared.syssound.set_guide_enabled(ctx.shared.config.audio.guide_se);
            }
            _ => {
                if delta > 0 {
                    self.enter_network_row(ctx.shared, id);
                }
            }
        }
        Transition::Stay
    }

    /// Collect the sound folder picker's answer, and read the set it names.
    fn poll_sound_picker(&mut self, shared: &mut AppShared) {
        let Some(picker) = self.sound_picker.as_ref() else {
            return;
        };
        match picker.poll() {
            DialogState::Open => return,
            DialogState::Picked(path) => {
                self.sound_picker = None;
                shared.config.audio.sound_folder = Some(path.to_string_lossy().into_owned());
                shared.reload_system_sounds();
            }
            DialogState::Dismissed => self.sound_picker = None,
        }
        self.dirty = true;
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
            SettingId::IrProfiles => {
                self.profiles_open = true;
                self.profiles_sel = 0;
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

    /// Keys inside the inline IR profile list. A profile is added by typing its base URL; Enter on
    /// an existing row turns it on or off, and D removes it.
    fn profiles_input(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) {
        if self.text_input.is_some() {
            match key.code {
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let value = self.text_input.take().map(|mut edit| edit.take()).unwrap_or_default();
                    if let Some(profile) = new_ir_profile(&value, &shared.config.network.ir_profiles) {
                        shared.config.network.ir_profiles.push(profile);
                        self.profiles_sel = shared.config.network.ir_profiles.len().saturating_sub(1);
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
        let rows = ir_profile_rows(&shared.config.network.ir_profiles).len();
        let on_add = self.profiles_sel + 1 >= rows;
        match key.code {
            KeyCode::Escape => {
                self.profiles_open = false;
                shared.save_settings();
                shared.rebuild_server();
            }
            KeyCode::ArrowUp => self.profiles_sel = self.profiles_sel.saturating_sub(1),
            KeyCode::ArrowDown => self.profiles_sel = (self.profiles_sel + 1).min(rows.saturating_sub(1)),
            KeyCode::Enter | KeyCode::NumpadEnter if on_add => {
                self.text_input = Some(TextEdit::new());
                self.text_secret = false;
                self.text_edit_row = None;
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if let Some(profile) = shared.config.network.ir_profiles.get_mut(self.profiles_sel) {
                    profile.enabled = !profile.enabled;
                }
            }
            KeyCode::KeyD if !on_add && self.profiles_sel < shared.config.network.ir_profiles.len() => {
                shared.config.network.ir_profiles.remove(self.profiles_sel);
                self.profiles_sel = self.profiles_sel.min(ir_profile_rows(&shared.config.network.ir_profiles).len().saturating_sub(1));
            }
            _ => {}
        }
    }

    /// Click inside the inline IR profile list, on the same terms as the rival list.
    fn profiles_click(&mut self, shared: &AppShared, index: usize) {
        let rows = ir_profile_rows(&shared.config.network.ir_profiles).len();
        self.profiles_sel = index.min(rows.saturating_sub(1));
        if self.profiles_sel + 1 >= rows {
            self.text_input = Some(TextEdit::new());
            self.text_secret = false;
            self.text_edit_row = None;
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
    /// chart, are visible. `now` is what a row that refused a typed figure stops saying so against.
    fn scene(&self, shared: &AppShared, now: Instant) -> SettingsScene {
        let list_open = self.rivals_open || self.profiles_open;
        let editor = match (&self.text_input, list_open) {
            (Some(edit), false) => Some(editor_display(edit, self.text_secret)),
            _ => None,
        };
        let inline_editor = self.text_input.as_ref().map(|edit| editor_display(edit, false));
        let rivals = if self.profiles_open {
            Some(RivalsScene { rows: ir_profile_rows(&shared.config.network.ir_profiles), sel: self.profiles_sel, editor: inline_editor })
        } else {
            self.rivals_open.then(|| RivalsScene { rows: rival_rows(&shared.config.network.rivals), sel: self.rivals_sel, editor: inline_editor })
        };
        SettingsScene {
            tabs: SettingTab::SHOWN.iter().map(|tab| tab.label()).collect(),
            tab: self.tab.min(SettingTab::SHOWN.len() - 1),
            rows: self.lines.clone(),
            sel: self.sel.min(self.lines.len().saturating_sub(1)),
            hint: self.hint(shared),
            editor,
            status: self.audio_status_line(shared).unwrap_or_else(|| shared.net_status.clone()),
            rivals,
            typed: self.numbers.input.editing().and_then(|id| Some((self.row_at(id)?, self.numbers.input.edit_of(id)?.clone()))),
            dragged: self.numbers.input.dragged().and_then(|(id, _)| self.row_at(id)),
            refused: self.rows.iter().position(|row| matches!(row, SettingRow::Fixed(id) if self.numbers.input.is_refused(*id, now))),
        }
    }

    /// Where a row of the settings table is in the open tab.
    fn row_at(&self, id: SettingId) -> Option<usize> {
        self.rows.iter().position(|row| *row == SettingRow::Fixed(id))
    }

    /// The keys of the screen as the hint under the title names them: what a number being typed
    /// takes, or the keys of the row under the cursor -- what sets its number when it holds one,
    /// and the keys of the screen when it does not -- and the three that open the other screens.
    fn hint(&self, shared: &AppShared) -> String {
        if self.numbers.input.editing().is_some() {
            return EDIT_HINT.to_string();
        }
        let key_of = |shortcut| shared.shortcut_key(shortcut).map_or(NO_KEY, key_name);
        let (play_menu, skin_menu, key_config) = (key_of(Shortcut::PlayMenu), key_of(Shortcut::SkinMenu), key_of(Shortcut::KeyConfig));
        if !self.is_busy() && self.focused_number(shared).is_some() {
            return number_row_hint(play_menu, skin_menu, key_config);
        }
        settings_hint(play_menu, skin_menu, key_config)
    }

    /// Keys on the row list itself, once the editor and the rival list have had their turn.
    fn row_key(&mut self, ctx: &mut FrameCtx<'_>, key: &KeyInput<'_>) -> Transition {
        let focused = self.focused();
        let number = self.focused_number(ctx.shared);
        match key.code {
            KeyCode::Escape => {
                ctx.shared.save_settings();
                return Transition::Back;
            }
            KeyCode::Tab => {
                self.tab = (self.tab + 1) % SettingTab::SHOWN.len();
                self.sel = 0;
                self.dirty = true;
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let Some(row) = focused else {
                    return Transition::Stay;
                };
                let Some(id) = self.focused_fixed() else {
                    return self.step(ctx, row, ROW_STEP_FORWARD, false);
                };
                match (id, number) {
                    (_, Some((_, value))) => self.numbers.input.begin_edit(id, &value),
                    (SettingId::KeyConfig, None) => return Transition::Open(Stage::KeyConfig(KeyConfigState::new())),
                    (SettingId::Font, None) => return self.run_action(ctx, SettingId::Font, ROW_STEP_FORWARD),
                    _ if is_network_row(id) => return self.step(ctx, row, ROW_STEP_FORWARD, false),
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
                    return self.step(ctx, row, ROW_STEP_BACK, self.numbers.shift_held);
                }
            }
            KeyCode::ArrowRight => {
                if let Some(row) = focused {
                    return self.step(ctx, row, ROW_STEP_FORWARD, self.numbers.shift_held);
                }
            }
            KeyCode::Digit0 | KeyCode::Numpad0 if number.is_some() => {
                if let Some((id, value)) = number {
                    self.set_number(ctx, id, value.shipped);
                }
            }
            code if ctx.shared.is_shortcut(Shortcut::PlayMenu, code) => return Transition::Open(PlayMenuState::stage()),
            code if ctx.shared.is_shortcut(Shortcut::SkinMenu, code) => return Transition::Open(SkinMenuState::stage()),
            code if ctx.shared.is_shortcut(Shortcut::KeyConfig, code) => return Transition::Open(Stage::KeyConfig(KeyConfigState::new())),
            _ => {}
        }
        Transition::Stay
    }
}

impl StageHandler for SettingsState {
    /// Re-read the host's output device list on the way in, so the AUDIO DEVICE row cycles through
    /// what is plugged in now, and build the rows the open tab shows. Coming back from the skin menu
    /// or the play menu is an entry too, so what is said beside a row follows what was chosen there.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        self.audio_devices = rbms_audio::output_device_names();
        self.refresh(ctx.shared);
    }

    /// Keep the rival cursor inside the list, which a background refresh can shorten.
    ///
    /// The ACCOUNT row is the one value a worker thread can change while the screen sits still, so
    /// the tab holding it rebuilds its rows every frame; every other tab keeps the rows it has
    /// until a key or a click moves one.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.poll_font_picker(ctx.shared);
        self.poll_sound_picker(ctx.shared);
        let rows = rival_rows(&ctx.shared.config.network.rivals).len();
        self.rivals_sel = self.rivals_sel.min(rows.saturating_sub(1));
        if self.rows.contains(&SettingRow::Fixed(SettingId::Account)) {
            self.dirty = true;
        }
        Transition::Stay
    }

    /// A key pressed while a slider is being dragged ends the drag first, which stores where the
    /// drag had the row: the key that leaves the screen must not leave what the slider showed
    /// behind, and neither must a drag whose button was never seen coming back up.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if matches!(key.code, KeyCode::ControlLeft | KeyCode::ControlRight | KeyCode::SuperLeft | KeyCode::SuperRight) {
            self.paste_held = key.pressed;
            return Transition::Stay;
        }
        if let Some(held) = shift_after(self.numbers.shift_held, &key) {
            self.numbers.shift_held = held;
            return Transition::Stay;
        }
        if !key.pressed {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.end_drag(ctx);
        if self.rivals_open {
            self.rivals_input(ctx.shared, &key);
            self.dirty = true;
            return Transition::Stay;
        }
        if self.profiles_open {
            self.profiles_input(ctx.shared, &key);
            self.dirty = true;
            return Transition::Stay;
        }
        if self.text_input.is_some() {
            self.text_input_key(ctx.shared, &key);
            self.dirty = true;
            return Transition::Stay;
        }
        match self.numbers.input.edit_key(&key) {
            Some(EditKey::Taken) => return Transition::Stay,
            Some(EditKey::Cancel) => {
                self.numbers.input.cancel_edit();
                return Transition::Stay;
            }
            Some(EditKey::Commit) => {
                self.commit_number_edit(ctx);
                return Transition::Stay;
            }
            Some(EditKey::CommitThenPass) => self.commit_number_edit(ctx),
            None => {}
        }
        self.row_key(ctx, &key)
    }

    /// A press of the left button acts on what it lands on, after storing a number that was being
    /// typed. On a row that holds a number it takes hold of the slider, opens the number to be typed
    /// into, or moves the row one step, by where on the row it lands; a press of the right button on
    /// such a row moves it one step back. The left button coming back up ends a drag.
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32), button: MouseButton, pressed: bool) -> Transition {
        if !pressed {
            if button == MouseButton::Left {
                self.end_drag(ctx);
            }
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.end_drag(ctx);
        if button == MouseButton::Right {
            self.step_back_under(ctx, at);
            return Transition::Stay;
        }
        if !is_left_press(button, pressed) {
            return Transition::Stay;
        }
        self.commit_number_edit(ctx);
        let link = key_config_link();
        if link.x <= at.0 && at.0 < link.x + link.w && link.y <= at.1 && at.1 < link.y + link.h {
            return if self.is_busy() { Transition::Stay } else { Transition::Open(Stage::KeyConfig(KeyConfigState::new())) };
        }
        match ctx.shared.hit_test(at) {
            Some(Hot::SettingTab(_) | Hot::SettingRow(_) | Hot::PlayMenuLink | Hot::SkinMenuLink) if self.rivals_open || self.profiles_open => {}
            Some(Hot::SettingTab(ti)) => {
                self.cancel_text_edit();
                self.tab = ti.min(SettingTab::SHOWN.len() - 1);
                self.sel = 0;
                self.dirty = true;
            }
            Some(Hot::PlayMenuLink | Hot::SkinMenuLink) if self.text_input.is_some() => {}
            Some(Hot::PlayMenuLink) => return Transition::Open(PlayMenuState::stage()),
            Some(Hot::SkinMenuLink) => return Transition::Open(SkinMenuState::stage()),
            Some(Hot::SettingRow(i)) => {
                if let Some(row) = self.rows.get(i).copied() {
                    self.cancel_text_edit();
                    self.sel = i;
                    return self.press_row(ctx, row, i, at);
                }
            }
            Some(Hot::RivalRow(i)) if self.profiles_open => self.profiles_click(ctx.shared, i),
            Some(Hot::RivalRow(i)) => self.rivals_click(ctx.shared, i),
            _ => {}
        }
        Transition::Stay
    }

    /// The cursor moving with a button held carries the row being dragged along its slider, wherever
    /// on the screen the pointer is.
    fn handle_mouse_drag(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition {
        self.drag_to(ctx, at.0);
        Transition::Stay
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        self.ensure_rows(ctx.shared);
        let scene = self.scene(ctx.shared, ctx.now);
        canvas.clear_bga();
        let hot = render_settings(canvas, &scene);
        ctx.shared.hot.extend(hot.into_iter().filter_map(|(rect, h)| {
            let mapped = match h {
                SettingsHot::Tab(i) => Hot::SettingTab(i),
                SettingsHot::PlayMenuLink => Hot::PlayMenuLink,
                SettingsHot::SkinMenuLink => Hot::SkinMenuLink,
                SettingsHot::KeyConfigLink => return None,
                SettingsHot::Row(i) => Hot::SettingRow(i),
                SettingsHot::RivalRow(i) => Hot::RivalRow(i),
            };
            Some((rect, mapped))
        }));
    }
}

#[cfg(test)]
mod note_tests;
#[cfg(test)]
mod number_tests;
#[cfg(test)]
mod play_tests;
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

    /// Names the folder the frames of the settings screen are saved in. Nothing is saved without it.
    const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

    /// The two sizes the screen is drawn at: the one it is laid out for, and a full HD window.
    const DRAWN_SIZES: [(u32, u32); 2] = [(CW, CH), (1920, 1080)];

    /// How far below the button a press is clear of it and of the tab strip under it, and how far
    /// inside the button's corner a pixel is read, clear of its edge.
    const BELOW_THE_BUTTON: f32 = 400.0;
    const INSIDE_THE_BUTTON: f32 = 2.0;

    fn ctx(app: &mut App) -> FrameCtx<'_> {
        FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
    }

    /// The middle of the button to the key editor.
    fn on_the_key_config_button() -> (f32, f32) {
        let link = key_config_link();
        (link.x + link.w * 0.5, link.y + link.h * 0.5)
    }

    /// The key editor opens from the button above the one to the skin menu and from the key that
    /// opens it from the song browser, and from neither while a list or an editor has the screen.
    #[test]
    fn the_key_editor_opens_from_its_button_and_from_its_key() {
        let mut app = app();
        let mut state = SettingsState::new();
        let at = on_the_key_config_button();
        assert!(matches!(state.handle_mouse(&mut ctx(&mut app), at, MouseButton::Left, true), Transition::Open(Stage::KeyConfig(_))), "the button");
        assert!(matches!(state.handle_mouse(&mut ctx(&mut app), at, MouseButton::Right, true), Transition::Stay), "another button of the mouse opened it");
        assert!(
            matches!(state.handle_mouse(&mut ctx(&mut app), (at.0, at.1 + BELOW_THE_BUTTON), MouseButton::Left, true), Transition::Stay),
            "a press off the button"
        );
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(Shortcut::KeyConfig.default_key())), Transition::Open(Stage::KeyConfig(_))), "the key");

        state.rivals_open = true;
        assert!(matches!(state.handle_mouse(&mut ctx(&mut app), at, MouseButton::Left, true), Transition::Stay), "the button under the rival list");
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(Shortcut::KeyConfig.default_key())), Transition::Stay), "the key under the rival list");
        state.rivals_open = false;
        state.text_input = Some(TextEdit::new());
        assert!(matches!(state.handle_mouse(&mut ctx(&mut app), at, MouseButton::Left, true), Transition::Stay), "the button under an editor");
    }

    /// The keys that open the play menu, the skin menu and the key editor from this screen are the
    /// ones the key configuration holds, and the hint under the title names them as they are now.
    #[test]
    fn the_keys_to_the_other_screens_are_read_from_the_key_configuration() {
        use crate::keyconfig::Bindable;
        let mut app = app();
        let mut state = SettingsState::new();
        let mode = app.shared.mode;
        assert!(state.scene(&app.shared, Instant::now()).hint.contains("F8 PLAY   F12 SKINS   F5 KEYS"), "{}", state.scene(&app.shared, Instant::now()).hint);
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F8)), Transition::Open(Stage::PlayMenu(_))), "the key the play menu ships on");
        app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::KeyConfig), Some(KeyCode::F6));
        app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::SkinMenu), Some(KeyCode::F7));
        app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::PlayMenu), Some(KeyCode::F10));
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F5)), Transition::Stay), "the key the editor shipped on still opens it");
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F12)), Transition::Stay), "the key the menu shipped on still opens it");
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F8)), Transition::Stay), "the key the play menu shipped on still opens it");
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F6)), Transition::Open(Stage::KeyConfig(_))));
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F7)), Transition::Open(Stage::SkinMenu(_))));
        assert!(matches!(state.handle_key(&mut ctx(&mut app), press(KeyCode::F10)), Transition::Open(Stage::PlayMenu(_))));
        assert!(state.scene(&app.shared, Instant::now()).hint.contains("F10 PLAY   F7 SKINS   F6 KEYS"), "{}", state.scene(&app.shared, Instant::now()).hint);
        app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::PlayMenu), None);
        assert!(
            state.scene(&app.shared, Instant::now()).hint.contains("- PLAY"),
            "a menu left without a key is not named as one: {}",
            state.scene(&app.shared, Instant::now()).hint
        );
    }

    /// The screen draws the button to the key editor where a press is tested for it, at the size the
    /// screen is laid out for and at full HD.
    #[test]
    fn the_button_to_the_key_editor_is_drawn_where_it_is_pressed() {
        use crate::stage::{Canvas, HeadlessCanvas};
        use rbms_render::Renderer;
        for size in DRAWN_SIZES {
            let mut app = app();
            let mut state = SettingsState::new();
            let mut pixels = HeadlessCanvas::new(size.0, size.1);
            state.draw(&mut ctx(&mut app), &mut Canvas::Headless(&mut pixels));
            if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
                let folder = std::path::Path::new(&folder);
                std::fs::create_dir_all(folder).expect("the capture folder can be created");
                let file = folder.join(format!("settings-key-config-button-{}x{}.png", size.0, size.1));
                image::save_buffer(file, pixels.rgba(), size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
            }
            let link = key_config_link();
            let scale = size.0 as f32 / CW as f32;
            let corner = pixels.pixel_at(((link.x + INSIDE_THE_BUTTON) * scale) as u32, ((link.y + INSIDE_THE_BUTTON) * scale) as u32);
            assert_eq!(corner, rbms_render::theme().panel_hi, "the button is not drawn where a press is tested for it");
            assert_eq!(pixels.size(), size);
        }
    }
}
