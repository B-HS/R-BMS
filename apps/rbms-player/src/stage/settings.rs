//! The settings screen: the rail of groups, the rows of the open group under the headings of its
//! sections, the panel that says what the row under the cursor is for, the in-place text editor the
//! NETWORK rows use, and the inline rival list. The rows themselves are described in
//! `rbms_config::SETTINGS`; this file owns the screen's state and routes keys and clicks into them
//! by name. How the screen is drawn is [`crate::settings_view`]'s, what it says of its groups and
//! rows is [`catalog`]'s, and what it says of the row under the cursor is [`info`]'s.
//!
//! The skin settings are not on this screen, and neither are the options of a sitting -- how fast the
//! notes scroll, what the lane is covered with, when a press counts and how a chart is dealt out.
//! They are the skin menu's ([`crate::stage::SkinMenuState`]) and the play menu's
//! ([`crate::stage::PlayMenuState`]), each of which lists its rows beside a preview of what they do.
//! This screen shows the groups of [`SettingTab::SHOWN`] and a way to either menu: the key that opens
//! it from the song browser ([`Shortcut::PlayMenu`], [`Shortcut::SkinMenu`]), and its button at the
//! foot of the rail. The key editor is reached the same two ways: the key that opens it from the
//! browser ([`Shortcut::KeyConfig`]), and the button under the other two. That button, the one that
//! puts a group back and the two of the question it asks are not among the regions a frame registers
//! ([`Hot`]): each is where the screen's layout says it is, and a press is tested against that.
//!
//! A row is set by what it holds. An arrow turns a switch over and steps a choice, and so does
//! Enter, which takes a choice to its next value as the skin and play menus do; Escape is the key
//! that saves and leaves. Enter on a row that opens something opens it: the folder of the system
//! sounds and the font are picked, the key editor is opened. A press on a switch turns it over, a
//! press on a value of a choice takes that value, and a press on an arrow of a longer choice steps
//! it. A row that holds a number -- a volume, a judge width, a fade -- is
//! set as such a row is on every built-in screen ([`crate::number_row`]): an arrow moves it one
//! step of the smallest change it can hold, and ten with Shift held; its slider is dragged; and
//! Enter, or a press on the number, opens it to be typed into. A drag writes the settings file
//! once, when the button comes back up. A press anywhere else on a row moves the cursor there and
//! changes nothing, and a press of the right button steps a row back.
//!
//! The zero key puts the row under the cursor back to what it ships as, and with Shift held asks
//! whether every row of the open group should be put back ([`Listing::confirming`]). Text --
//! the address of the score server, the id scores are sent under -- is not put back by either.
#![allow(clippy::wildcard_imports)]

use crate::dialog::{DialogHandle, DialogState};
use crate::ir_panel::*;
use crate::ir_session::{AuthAction, GUEST_PLAYER_ID};
use crate::keyconfig::Shortcut;
use crate::number_row::{EDIT_HINT, EditKey, NumberInput, NumberValue, hot_rect};
use crate::settings_ui::{SettingRow, audio_status_text, is_resettable, shipped_value, step_audio_device};
use crate::settings_view::{
    CONFIRM_HINT, ConfirmScene, ControlPart, InfoScene, ListItem, RIVALS_HINT, RailGroup, RivalsScene, RowControl, SettingsLine, SettingsScene,
    confirm_yes_rect, control_part_at, key_config_link, list_rect, reset_group_rect, visible_rows,
};
use crate::stage::preview_menu::{NEXT_TAB, PREVIOUS_TAB, contains, revealed_top_of, shift_after, stepped_tab, wheel_scrolled_over};
use crate::stage::{Canvas, FrameCtx, KeyConfigState, KeyInput, PlayMenuState, SkinMenuState, Stage, StageHandler, Transition, is_left_press};
use rbms_config::{AdjustOutcome, SettingId, SettingTab, adjust, cycle_values, step_skin};

use crate::*;

pub(crate) mod catalog;
mod info;

/// One step to the right on a row, which is what the right arrow, Enter and a click all ask for.
const ROW_STEP_FORWARD: i32 = 1;

/// One step to the left on a row.
const ROW_STEP_BACK: i32 = -1;

/// How many values a switch has, which is the most steps that bring one back to what it ships as.
const SWITCH_VALUES: usize = 2;

/// What the question about a group adds in a group that holds lines of text, which it does not
/// put back.
const KEPT_TEXT_NOTE: &str = " TEXT IS KEPT.";

/// The settings screen's own state: the open group and row, the rows it is showing, the in-place
/// editor, and the inline rival list.
///
/// `rows` is the rows of the open group in the order its sections list them, and `lines` holds their
/// labels and values, with what is said beside each. Both, and how the group is listed
/// ([`Listing`]), are rebuilt only when `dirty` says something moved, so a screen sitting still
/// formats no row.
/// `audio_devices` is the host's output device list, re-read on every entry so the AUDIO DEVICE row
/// cycles through what is plugged in now without enumerating on every keystroke.
pub(crate) struct SettingsState {
    tab: usize,
    /// The row under the cursor, by its place in `rows`.
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
    /// How the open group is listed and what the rail says of every group. Kept behind a pointer,
    /// because this state is moved with every screen change.
    list: Box<Listing>,
    /// What the screen keeps about its rows that hold a number. Kept behind a pointer, because this
    /// state is moved with every screen change.
    numbers: Box<NumberRows>,
}

/// How the settings screen lists the open group, beyond the rows themselves.
#[derive(Default)]
struct Listing {
    /// The rows of the open group with the heading of each section before its first.
    entries: Vec<ListItem>,
    /// What the rail says of every group.
    groups: Vec<RailGroup>,
    /// The first entry of the list on show: the cursor is brought back onto the screen by every key
    /// that moves it, and the wheel moves the list alone.
    top: usize,
    /// The part of a line the wheel has turned that has not moved the list yet.
    wheel_lines: f32,
    /// Whether the screen is asking whether every row of the open group should be put back.
    confirming: bool,
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
            list: Box::default(),
            numbers: Box::new(NumberRows { input: NumberInput::new(), carried: None, shift_held: false }),
        }
    }

    /// Whether one of the lists drawn over the rows, an in-place editor or the question about a
    /// group has the screen: the keys and the buttons that leave it for another screen stand down
    /// while one does.
    fn is_busy(&self) -> bool {
        self.rivals_open || self.profiles_open || self.list.confirming || self.text_input.is_some() || self.numbers.input.editing().is_some()
    }

    /// The open tab.
    pub(crate) fn current_tab(&self) -> SettingTab {
        SettingTab::SHOWN[self.tab.min(SettingTab::SHOWN.len() - 1)]
    }

    /// Re-read the open group's rows and their values, and what the rail says of every group.
    ///
    /// The rows are listed under the headings of the group's sections ([`catalog::arranged`]). A row
    /// a drag is carrying without storing it yet reads as where the drag has it.
    fn refresh(&mut self, shared: &AppShared) {
        let tab = self.current_tab();
        self.rows.clear();
        self.list.entries.clear();
        for (heading, rows) in catalog::arranged(tab, &shared.settings_rows(tab)) {
            self.list.entries.push(ListItem::Heading(heading));
            for row in rows {
                self.list.entries.push(ListItem::Row(self.rows.len()));
                self.rows.push(row);
            }
        }
        self.lines = self.rows.iter().map(|&row| shared.settings_scene_line(row)).collect();
        self.list.groups = SettingTab::SHOWN
            .iter()
            .map(|group| RailGroup { name: group.label(), rows: shared.settings_rows(*group).len(), changed: shared.changed_rows(*group) })
            .collect();
        if let Some((id, steps)) = self.numbers.carried
            && let Some(line) = self.rows.iter().position(|row| *row == SettingRow::Fixed(id)).and_then(|at| self.lines.get_mut(at))
            && let Some(number) = line.number.as_mut()
        {
            number.steps = steps;
            line.value = number.spec.text(steps);
        }
        self.sel = self.sel.min(self.rows.len().saturating_sub(1));
        self.list.top = self.list.top.min(self.list.entries.len().saturating_sub(visible_rows()));
        self.dirty = false;
    }

    /// Scroll the list just far enough to have the cursor's row on show, with the heading of its
    /// section when the row is the first under it.
    fn reveal_cursor(&mut self) {
        let Some(at) = self.list.entries.iter().position(|entry| *entry == ListItem::Row(self.sel)) else {
            return;
        };
        let above = at.saturating_sub(1);
        let from = if matches!(self.list.entries.get(above), Some(ListItem::Heading(_))) { above } else { at };
        self.list.top = revealed_top_of(from, self.list.top, self.list.entries.len(), visible_rows());
        self.list.top = revealed_top_of(at, self.list.top, self.list.entries.len(), visible_rows());
    }

    /// Move the cursor to the row at `at` of the open group, held to its two ends, and bring it
    /// onto the screen.
    fn focus(&mut self, at: usize) {
        self.sel = at.min(self.rows.len().saturating_sub(1));
        self.reveal_cursor();
    }

    /// Move the cursor to the first row of the section before the one it is in, or after it: the
    /// first row of its own section when it is further down that section and the way is back.
    fn focus_section(&mut self, delta: i32) {
        let entries = &self.list.entries;
        let is_first = |pair: &[ListItem]| match pair {
            [ListItem::Heading(_), ListItem::Row(row)] => Some(*row),
            _ => None,
        };
        let firsts: Vec<usize> = entries.windows(2).filter_map(is_first).collect();
        let target = if delta < 0 { firsts.iter().rev().find(|first| **first < self.sel) } else { firsts.iter().find(|first| **first > self.sel) };
        if let Some(first) = target {
            self.focus(*first);
        }
    }

    /// Open the group one step along the rail from the open one, round its ends, with the cursor on
    /// its first row.
    fn step_group(&mut self, delta: i32) {
        self.open_group(stepped_tab(Some(self.tab), SettingTab::SHOWN.len(), delta));
    }

    /// Open the group at one place of the rail, with the cursor on its first row.
    fn open_group(&mut self, place: usize) {
        self.tab = place.min(SettingTab::SHOWN.len() - 1);
        self.sel = 0;
        self.list.top = 0;
        self.dirty = true;
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

    /// A press of the left button at `at` on `row`, the row at `index` of the open group, by the
    /// part of the row it landed on ([`control_part_at`]). On a row that holds a number, a press on
    /// the slider begins a drag -- the row is put where the press landed, unless it landed on the
    /// knob, which is taken hold of where it stands -- and one on the number opens the row to be
    /// typed into. A press on a switch turns it over, one on a value of a choice takes that value,
    /// one on an arrow steps the choice, and one on a field, a path or an action opens or runs it.
    /// A press anywhere else on the row has moved the cursor there and does nothing more.
    fn press_row(&mut self, ctx: &mut FrameCtx<'_>, row: SettingRow, index: usize, at: (f32, f32)) -> Transition {
        let SettingRow::Fixed(id) = row else {
            return Transition::Stay;
        };
        let Some(control) = self.lines.get(index).map(|line| line.control) else {
            return Transition::Stay;
        };
        let part = hot_rect(ctx.shared, Hot::SettingRow(index)).map_or(ControlPart::Label, |drawn| control_part_at(drawn, control, at));
        match (part, self.number_of(ctx.shared, id)) {
            (ControlPart::Slider(area), Some(value)) => {
                if !self.numbers.input.begin_drag(id, area, &value, at.0) {
                    self.drag_to(ctx, at.0);
                }
            }
            (ControlPart::Number, Some(value)) => self.numbers.input.begin_edit(id, &value),
            (ControlPart::Segment(place), _) => self.choose(ctx, id, place),
            (ControlPart::Back, _) => return self.step(ctx, row, ROW_STEP_BACK, false),
            (ControlPart::Switch | ControlPart::Forward | ControlPart::Open, _) => return self.step(ctx, row, ROW_STEP_FORWARD, false),
            (ControlPart::Slider(_) | ControlPart::Number | ControlPart::Label, _) => {}
        }
        Transition::Stay
    }

    /// Put a choice drawn with all its values on show on the value at `place` of them.
    ///
    /// The layout of the built-in note field is stored by name and may be standing on a file the run
    /// was started with, which is none of its values, so it is named outright; any other choice is
    /// stepped forward to the value, round the end of its list.
    fn choose(&mut self, ctx: &mut FrameCtx<'_>, id: SettingId, place: usize) {
        let values = cycle_values(id);
        let Some(wanted) = values.get(place) else {
            return;
        };
        if id == SettingId::Skin {
            ctx.shared.drop_launch_layout();
            ctx.shared.config.display.skin = (*wanted).to_string();
            self.dirty = true;
            return;
        }
        let held = ctx.shared.setting_line(id).1;
        let Some(standing) = values.iter().position(|value| *value == held) else {
            return;
        };
        for _ in 0..(place + values.len() - standing) % values.len() {
            self.step(ctx, SettingRow::Fixed(id), ROW_STEP_FORWARD, false);
        }
    }

    /// A press of the right button at `at`: a row that holds a value goes one step back, so a step
    /// taken by mistake with the mouse can be taken back with it, wherever on the row the press
    /// lands. A row that is run or opened, a line of text and a path are left alone, and so is every
    /// row under a list, an editor or the question about a group.
    fn step_back_under(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32)) {
        if self.rivals_open || self.profiles_open || self.text_input.is_some() {
            return;
        }
        let Some(Hot::SettingRow(index)) = ctx.shared.hit_test(at) else {
            return;
        };
        let Some(row) = self.rows.get(index).copied() else {
            return;
        };
        self.commit_number_edit(ctx);
        let steps_back = matches!(
            self.lines.get(index).map(|line| line.control),
            Some(RowControl::Slider | RowControl::Switch { .. } | RowControl::Segments { .. } | RowControl::Stepper)
        );
        if steps_back {
            self.sel = index;
            self.step(ctx, row, ROW_STEP_BACK, false);
        }
    }

    /// Put one row back to what it ships as, and report whether it moved. A row the screen does not
    /// put back ([`is_resettable`]) and a row that is already what it ships as are left alone.
    ///
    /// A number is put on the step it ships on. The font, the folder of the system sounds, the
    /// output device and the layout of the built-in note field are named outright, since each is
    /// stepped by something only the running program can do. A switch and any other choice are
    /// stepped forward until they read what a fresh configuration reads, which runs whatever a step
    /// of that row runs.
    fn reset_row(&mut self, ctx: &mut FrameCtx<'_>, id: SettingId) -> bool {
        if !is_resettable(id) {
            return false;
        }
        if let Some(value) = self.number_of(ctx.shared, id) {
            if value.steps == value.shipped {
                return false;
            }
            self.set_number(ctx, id, value.shipped);
            return true;
        }
        let shipped = shipped_value(id);
        if ctx.shared.setting_line(id).1 == shipped {
            return false;
        }
        self.dirty = true;
        match id {
            SettingId::Font => ctx.shared.reset_font(),
            SettingId::SoundFolder => {
                ctx.shared.config.audio.sound_folder = None;
                ctx.shared.reload_system_sounds();
            }
            SettingId::AudioDevice => {
                ctx.shared.config.audio.device = None;
                ctx.shared.config.audio.mark_reopen_pending();
            }
            SettingId::Skin => {
                ctx.shared.drop_launch_layout();
                ctx.shared.config.display.skin = shipped;
            }
            _ => {
                for _ in 0..cycle_values(id).len().max(SWITCH_VALUES) {
                    if ctx.shared.setting_line(id).1 == shipped {
                        break;
                    }
                    self.step(ctx, SettingRow::Fixed(id), ROW_STEP_FORWARD, false);
                }
            }
        }
        true
    }

    /// Put every row of the open group back to what it ships as, and write the settings file.
    fn reset_group(&mut self, ctx: &mut FrameCtx<'_>) {
        for row in self.rows.clone() {
            if let SettingRow::Fixed(id) = row {
                self.reset_row(ctx, id);
            }
        }
        ctx.shared.save_settings();
        self.dirty = true;
    }

    /// How many rows of the open group are not what they ship as.
    fn changed_here(&self) -> usize {
        self.lines.iter().filter(|line| line.changed).count()
    }

    /// Ask whether every row of the open group should be put back, when any of them would move.
    fn ask_to_reset_group(&mut self) {
        self.list.confirming = self.changed_here() > 0;
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
            SettingId::GuideSe => ctx.shared.syssound.set_guide_enabled(ctx.shared.config.audio.guide_se),
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

    /// Everything the settings screen draws this frame. The AUDIO group owns the status line while
    /// it is open: it is the only place the engine's open report, and a reopen held back by a running
    /// chart, are visible. `now` is what a row that refused a typed figure stops saying so against.
    fn scene(&self, shared: &AppShared, now: Instant) -> SettingsScene {
        let list_open = self.rivals_open || self.profiles_open;
        let editor = match (&self.text_input, list_open) {
            (Some(edit), false) => self.text_edit_row.and_then(|id| self.row_at(id)).map(|at| (at, editor_display(edit, self.text_secret))),
            _ => None,
        };
        let inline_editor = self.text_input.as_ref().map(|edit| editor_display(edit, false));
        let rivals = if self.profiles_open {
            Some(RivalsScene { rows: ir_profile_rows(&shared.config.network.ir_profiles), sel: self.profiles_sel, editor: inline_editor })
        } else {
            self.rivals_open.then(|| RivalsScene { rows: rival_rows(&shared.config.network.rivals), sel: self.rivals_sel, editor: inline_editor })
        };
        let tab = self.current_tab();
        let sel = self.sel.min(self.lines.len().saturating_sub(1));
        let confirm = self.list.confirming.then(|| {
            let changed = self.changed_here();
            let go = if changed == 1 { "ROW GOES" } else { "ROWS GO" };
            let holds_text = self.lines.iter().any(|line| matches!(line.control, RowControl::Field { .. }));
            let kept = if holds_text { KEPT_TEXT_NOTE } else { "" };
            ConfirmScene { title: format!("RESET {}?", tab.label()), body: format!("{changed} CHANGED {go} BACK TO DEFAULT.{kept}") }
        });
        SettingsScene {
            groups: self.list.groups.clone(),
            group: self.tab.min(SettingTab::SHOWN.len() - 1),
            summary: catalog::group_summary(tab).to_string(),
            entries: self.list.entries.clone(),
            rows: self.lines.clone(),
            sel,
            top: self.list.top,
            info: self.focused_fixed().zip(self.lines.get(sel)).map(|(id, line)| self.busy_info(info::info(id, line, &self.audio_devices))),
            hint: self.hint(shared),
            link_keys: self.link_keys(shared),
            editor,
            status: self.audio_status_line(shared).unwrap_or_else(|| shared.net_status.clone()),
            rivals,
            confirm,
            typed: self.numbers.input.editing().and_then(|id| Some((self.row_at(id)?, self.numbers.input.edit_of(id)?.clone()))),
            dragged: self.numbers.input.dragged().and_then(|(id, _)| self.row_at(id)),
            refused: self.rows.iter().position(|row| matches!(row, SettingRow::Fixed(id) if self.numbers.input.is_refused(*id, now))),
        }
    }

    /// The panel of the row under the cursor as the mode that has the keys leaves it, in the order
    /// [`SettingsState::hint`] asks: the question about the group, a number being typed into the
    /// row, a list drawn over the rows, a line of text being typed into the row. With none of them
    /// up it is the row's own panel.
    fn busy_info(&self, info: InfoScene) -> InfoScene {
        let focused = self.focused_fixed();
        if self.list.confirming {
            return info::while_busy(info, &info::CONFIRM_KEYS, None);
        }
        if let Some(id) = self.numbers.input.editing() {
            let typed = self.numbers.input.edit_of(id).filter(|_| Some(id) == focused).map(|edit| edit.text().to_string());
            return info::while_busy(info, &info::NUMBER_EDIT_KEYS, typed);
        }
        if self.rivals_open || self.profiles_open {
            return info::while_busy(info, &info::LIST_KEYS, None);
        }
        if let Some(edit) = &self.text_input {
            let typed = self.text_edit_row.filter(|id| Some(*id) == focused).map(|_| editor_display(edit, self.text_secret));
            return info::while_busy(info, &info::TEXT_EDIT_KEYS, typed);
        }
        info
    }

    /// Where a row of the settings table is in the open group.
    fn row_at(&self, id: SettingId) -> Option<usize> {
        self.rows.iter().position(|row| *row == SettingRow::Fixed(id))
    }

    /// The keys that open the play menu, the skin menu and the key editor, as the key configuration
    /// names them now.
    fn link_keys(&self, shared: &AppShared) -> [String; 3] {
        [Shortcut::PlayMenu, Shortcut::SkinMenu, Shortcut::KeyConfig].map(|shortcut| shared.shortcut_key(shortcut).map_or(NO_KEY, key_name).to_string())
    }

    /// The keys of the screen as the line at its foot names them: what the question about a group,
    /// a number being typed, a line of text being typed or a list drawn over the rows takes while
    /// one of them has the screen, and otherwise the keys the row under the cursor hears, followed by
    /// the keys of the screen and the three that open the other screens.
    fn hint(&self, shared: &AppShared) -> String {
        if self.list.confirming {
            return CONFIRM_HINT.to_string();
        }
        if self.numbers.input.editing().is_some() {
            return EDIT_HINT.to_string();
        }
        if self.rivals_open || self.profiles_open {
            return RIVALS_HINT.to_string();
        }
        if self.text_input.is_some() {
            return info::TEXT_EDIT_HINT.to_string();
        }
        let row = self.focused_fixed().zip(self.lines.get(self.sel)).map_or_else(Vec::new, |(id, line)| info::row_keys(id, line.control));
        info::hint(&row, &self.link_keys(shared))
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
            KeyCode::Tab => self.step_group(if self.numbers.shift_held { PREVIOUS_TAB } else { NEXT_TAB }),
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
                    _ => return self.step(ctx, row, ROW_STEP_FORWARD, false),
                }
            }
            KeyCode::ArrowUp => self.focus(self.sel.saturating_sub(1)),
            KeyCode::ArrowDown => self.focus(self.sel + 1),
            KeyCode::PageUp => self.focus_section(ROW_STEP_BACK),
            KeyCode::PageDown => self.focus_section(ROW_STEP_FORWARD),
            KeyCode::Home => self.focus(0),
            KeyCode::End => self.focus(self.rows.len()),
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
            KeyCode::Digit0 | KeyCode::Numpad0 if self.numbers.shift_held => self.ask_to_reset_group(),
            KeyCode::Digit0 | KeyCode::Numpad0 if self.focused_fixed().is_some_and(is_resettable) => {
                if let Some(id) = self.focused_fixed() {
                    self.reset_row(ctx, id);
                }
            }
            code if ctx.shared.is_shortcut(Shortcut::PlayMenu, code) => return Transition::Open(PlayMenuState::stage()),
            code if ctx.shared.is_shortcut(Shortcut::SkinMenu, code) => return Transition::Open(SkinMenuState::stage()),
            code if ctx.shared.is_shortcut(Shortcut::KeyConfig, code) => return Transition::Open(Stage::KeyConfig(KeyConfigState::new())),
            _ => {}
        }
        Transition::Stay
    }

    /// A key pressed while the screen asks whether a group should be put back: Enter puts it back,
    /// Escape leaves it as it is, and either takes the question down. Every other key is the
    /// question's and does nothing.
    fn confirm_key(&mut self, ctx: &mut FrameCtx<'_>, code: KeyCode) {
        match code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                self.list.confirming = false;
                self.reset_group(ctx);
            }
            KeyCode::Escape => self.list.confirming = false,
            _ => {}
        }
    }
}

impl StageHandler for SettingsState {
    /// Re-read the host's output device list on the way in, so the AUDIO DEVICE row cycles through
    /// what is plugged in now, and build the rows the open group shows. Coming back from the skin
    /// menu or the play menu is an entry too, so what is said beside a row follows what was chosen
    /// there.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        self.audio_devices = rbms_audio::output_device_names();
        self.refresh(ctx.shared);
    }

    /// Keep the rival cursor inside the list, which a background refresh can shorten.
    ///
    /// The ACCOUNT row is the one value a worker thread can change while the screen sits still, so
    /// the group holding it rebuilds its rows every frame; every other group keeps the rows it has
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
        if self.list.confirming {
            self.confirm_key(ctx, key.code);
            return Transition::Stay;
        }
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
    /// typed: a group of the rail opens, a button to another screen opens that screen, the button
    /// under the groups asks whether the open group should be put back, and a row takes the cursor
    /// and is set by the part of it the press landed on ([`SettingsState::press_row`]). A press of
    /// the right button on a row that holds a value moves it one step back. The left button coming
    /// back up ends a drag. While the question about a group is up, a press on its first button puts
    /// the group back and any press takes the question down.
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32), button: MouseButton, pressed: bool) -> Transition {
        if !pressed {
            if button == MouseButton::Left {
                self.end_drag(ctx);
            }
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.end_drag(ctx);
        if self.list.confirming {
            self.list.confirming = false;
            if button == MouseButton::Left && contains(confirm_yes_rect(), at) {
                self.reset_group(ctx);
            }
            return Transition::Stay;
        }
        if button == MouseButton::Right {
            self.step_back_under(ctx, at);
            return Transition::Stay;
        }
        if !is_left_press(button, pressed) {
            return Transition::Stay;
        }
        self.commit_number_edit(ctx);
        if contains(key_config_link(), at) {
            return if self.is_busy() { Transition::Stay } else { Transition::Open(Stage::KeyConfig(KeyConfigState::new())) };
        }
        if contains(reset_group_rect(SettingTab::SHOWN.len()), at) {
            if !self.is_busy() {
                self.ask_to_reset_group();
            }
            return Transition::Stay;
        }
        match ctx.shared.hit_test(at) {
            Some(Hot::SettingTab(_) | Hot::SettingRow(_) | Hot::PlayMenuLink | Hot::SkinMenuLink) if self.rivals_open || self.profiles_open => {}
            Some(Hot::SettingTab(place)) => {
                self.cancel_text_edit();
                self.open_group(place);
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

    /// The wheel scrolls the rows while the pointer is over them, and does nothing anywhere else --
    /// over a slider too, which a turn of the wheel never moves -- or while a list, an editor or the
    /// question about a group has the screen. The cursor stays on its row, which the next key that
    /// moves it brings back onto the screen.
    fn handle_scroll(&mut self, ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        if !lines.is_finite() || self.is_busy() || !contains(list_rect(), ctx.shared.cursor) {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.list.top = wheel_scrolled_over(&mut self.list.wheel_lines, self.list.top, self.list.entries.len(), lines, visible_rows());
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
mod layout_tests;
#[cfg(test)]
mod note_tests;
#[cfg(test)]
mod number_tests;
#[cfg(test)]
mod play_tests;
#[cfg(test)]
mod skin_tests;
#[cfg(test)]
mod tests;
