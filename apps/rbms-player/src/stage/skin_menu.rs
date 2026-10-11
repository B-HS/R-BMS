//! The skin menu: the skin settings on the left, and on the right what they do -- the skin of the
//! screen being configured, drawn with sample data as the settings are changed.
//!
//! The settings are the ones the settings file has always kept in its skin group: the pack folder,
//! the screen being configured, the document chosen for it, what loaded, and the options, the file
//! slots and the offsets the chosen document declares. This screen is where they are changed; how
//! they are stored and which calls change them is unchanged, so a settings file written before this
//! screen existed reads as it did.
//!
//! The rows are the ones `rbms_config::SETTINGS` groups under [`SettingTab::Skin`], with the chosen
//! document's own between them ([`crate::AppShared::settings_rows`]). They are listed in one column
//! that scrolls, with the row under the cursor written out in full under the preview, because a
//! skin's own names for its options are longer than a column this narrow.
//!
//! The right half is not this screen's. [`SkinPreview`] draws it and says what is there, and this
//! screen says where it goes ([`crate::stage::preview_menu::PREVIEW_RECT`]), fills the rectangle when no frame of a skin is in
//! it -- with what the preview says of it, or with nothing at all for a wait the preview asks to be
//! left dark -- and tells the preview when a setting moved. The preview takes no press of the mouse:
//! a button of a previewed skin does nothing.
//!
//! A row that holds a number -- an offset of the chosen document, or how opaque one of its parts is
//! -- is set as such a row is on every built-in screen ([`crate::number_row`]): an arrow moves it one
//! unit, and ten with Shift held; its slider is dragged, and the preview follows it frame by frame,
//! because an offset is read where it is stored; and Enter, or a press on the number, opens it to be
//! typed into. A drag writes the settings file once, when the button comes back up.
//!
//! The screen is laid out in the fixed space every built-in screen is laid out in, and is opened
//! over the screen it was asked for from, which Escape goes back to.

use rbms_config::{AdjustOutcome, SettingId, SettingTab, adjust, descriptor};
use rbms_render::{Renderer, text_width, theme};
use rbms_skin::loader::{
    SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS,
    SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_RESULT,
};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::dialog::{DialogHandle, DialogState, pick_skin_pack_folder};
use crate::number_row::{EDIT_HINT, EditKey, NumberInput, RowPart};
use crate::settings_ui::SettingRow;
use crate::settings_view::SettingsLine;
use crate::skin_preview::SkinPreview;
use crate::skin_select::{OffsetAxis, SkinRow, SkinRowKind};
use crate::stage::preview_menu::{
    LABEL_GAP, LABEL_SCALE, LIST_RECT, ListEntry, ListWindow, NEXT_TAB as NEXT_SCREEN, NumberCell, PREVIOUS_TAB as PREVIOUS_SCREEN, ROW_STEP_BACK,
    ROW_STEP_FORWARD, RowLook, UTF8_MAX_BYTES, VALUE_SCALE, VISIBLE_ROWS, click_step, contains, draw_head, draw_hint, draw_info_lead, draw_info_note,
    draw_info_panel, draw_info_row, draw_list, draw_preview, draw_tabs, fitted_value, last_top, number_part_at, revealed_top, row_room, shift_after,
    stepped_tab, wheel_scrolled,
};
use crate::stage::{Canvas, FrameCtx, KeyInput, Stage, StageHandler, Transition, is_left_press};
use crate::{AppShared, Hot};

/// The screens the strip offers, left to right, each with the short name on its tab: every screen
/// this build draws with a skin, the play screens first. A screen a document can declare and nothing
/// draws is reached with the SCREEN row alone.
const SCREEN_KINDS: [(i32, &str); 10] = [
    (SKIN_TYPE_PLAY_7KEYS, "7K"),
    (SKIN_TYPE_PLAY_5KEYS, "5K"),
    (SKIN_TYPE_PLAY_14KEYS, "14K"),
    (SKIN_TYPE_PLAY_10KEYS, "10K"),
    (SKIN_TYPE_PLAY_9KEYS, "9K"),
    (SKIN_TYPE_MUSIC_SELECT, "SELECT"),
    (SKIN_TYPE_DECIDE, "DECIDE"),
    (SKIN_TYPE_RESULT, "RESULT"),
    (SKIN_TYPE_COURSE_RESULT, "COURSE"),
    (SKIN_TYPE_KEY_CONFIG, "KEYS"),
];

/// The most of the room a path is given that what is written before it may take and still be kept
/// when the path is too long: that it is the default pack, that it is missing.
const PATH_LEAD_MAX_SHARE: f32 = 0.6;

/// What a path is cut at, on any platform.
const PATH_SEPARATORS: [char; 2] = ['/', '\\'];

/// What stands for the part of a path that was left out.
const PATH_GAP: &str = "\u{2026}";

/// The screen's title.
const TITLE: &str = "SKIN";

/// The keys of the screen, written along its foot.
const HINT: &str =
    "UP DOWN MOVE   LEFT RIGHT CHANGE   SHIFT x10   ENTER RUN OR TYPE   0 RESET   DRAG A SLIDER   TAB SCREEN   WHEEL SCROLL   RIGHT CLICK BACK   ESC SAVE";

/// What is written under the preview in place of the row under the cursor while the folder picker is
/// up.
const PICKER_NOTE: &str = "CHOOSE THE PACK FOLDER IN THE WINDOW THAT OPENED";

/// What is said of a row the chosen document declares, by the kind of row it is: a row of the
/// settings table says what it is for itself (`rbms_config::SettingDescriptor::help`).
///
/// An offset is said by its axis. A place and a size are counted in pixels of the screen, except for
/// the one offset that moves a whole play screen, which is a share of it; a row is not told which
/// offset it is of, so both are said. An offset the player may move along its alpha alone is an
/// opacity, and is said as one: it reads in percent, a hundred being no offset at all. Its slider
/// and its arrows end there, so its help says what goes further: a typed figure, for the part a
/// skin draws with no alpha of its own and lights by adding to it.
///
/// A place or a size runs over thousands of pixels, more than its slider has places for, so its
/// help says what sets it to the pixel.
const PROPERTY_HELP: &str = "An option this skin declares";
const FILE_HELP: &str = "A file slot this skin declares: which of its files is used";
const OFFSET_PLACE_HELP: &str =
    "An offset of this skin, in pixels - percent for the whole screen. The slider is coarse: arrows or ENTER set it exactly. 0 key resets";
const OFFSET_ANGLE_HELP: &str = "An offset of this skin: a turn, in degrees. 0 key resets";
const OFFSET_ALPHA_HELP: &str = "An offset of this skin: added to the opacity, from -255 (gone) to +255. 0 key resets";
const OPACITY_HELP: &str = "How opaque this part is: 0% hides it, 100% is the skin's own. Typing past 100% (to 200%) adds alpha. 0 key resets";

/// What is said of a row of an offset the game adds to every play skin, which no document declares:
/// a row of that name in a skin's own list would be read as the skin's.
const AUTOMATIC_PLACE_HELP: &str =
    "An offset the game adds to every play skin, in pixels - percent for the whole screen. Arrows or ENTER set it exactly. 0 key resets";
const AUTOMATIC_ALPHA_HELP: &str = "An offset the game adds to every play skin: added to the opacity, from -255 (gone) to +255. 0 key resets";

/// The skin menu's own state: the rows it lists and where the cursor and the list are, the folder
/// picker while it is up, the preview on its right, and the modifier that turns the key for the next
/// screen into the key for the one before.
///
/// `lines` holds the labels and values of `rows`, and both are rebuilt when `dirty` says something
/// moved. `top` is the first row on show: the cursor is brought back onto the screen by every key
/// that moves it or the row under it, and the wheel moves the list alone.
pub(crate) struct SkinMenuState {
    preview: SkinPreview,
    shift_held: bool,
    sel: usize,
    top: usize,
    rows: Vec<SettingRow>,
    lines: Vec<SettingsLine>,
    dirty: bool,
    /// The skin pack folder picker while it is up. It runs beside the frame loop, so the screen
    /// keeps drawing while the user is in front of it and the answer is collected in `update`.
    pack_picker: Option<DialogHandle>,
    /// What puts the folder picker up. The native one, except in a test, which has nobody to
    /// answer it.
    open_pack_picker: fn() -> DialogHandle,
    /// The part of a line the wheel has turned that has not moved the list yet.
    wheel_lines: f32,
    /// Which row that holds a number is being typed into, and which is being dragged.
    numbers: NumberInput<SettingRow>,
}

impl SkinMenuState {
    pub(crate) fn new() -> SkinMenuState {
        SkinMenuState {
            preview: SkinPreview::new(),
            shift_held: false,
            sel: 0,
            top: 0,
            rows: Vec::new(),
            lines: Vec::new(),
            dirty: true,
            pack_picker: None,
            open_pack_picker: pick_skin_pack_folder,
            wheel_lines: 0.0,
            numbers: NumberInput::new(),
        }
    }

    /// The skin menu as a screen to open over another.
    pub(crate) fn stage() -> Stage {
        Stage::SkinMenu(Box::new(SkinMenuState::new()))
    }

    /// Re-read the rows and their values, keeping the cursor on the row it was on.
    ///
    /// A row of the settings table is followed to wherever it now is, since the chosen document's own
    /// rows come and go above the actions; a row of a document is held by its place, because the row
    /// at that place of another document is the nearest thing to it.
    fn refresh(&mut self, shared: &AppShared) {
        let focused = self.focused();
        self.rows = shared.settings_rows(SettingTab::Skin);
        self.lines = self.rows.iter().map(|&row| shared.settings_scene_line(row)).collect();
        if let Some(fixed @ SettingRow::Fixed(_)) = focused
            && let Some(at) = self.rows.iter().position(|row| *row == fixed)
        {
            self.sel = at;
        }
        self.sel = self.sel.min(self.rows.len().saturating_sub(1));
        self.top = self.top.min(last_top(self.rows.len()));
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

    /// The LOADED row as the list shows it: what the chosen document is, or why it is not drawn.
    fn loaded_line(&self) -> Option<&SettingsLine> {
        let at = self.rows.iter().position(|row| *row == SettingRow::Fixed(SettingId::SkinInfo))?;
        self.lines.get(at)
    }

    /// Scroll the list just far enough to have the cursor's row on show.
    fn reveal_cursor(&mut self) {
        self.top = revealed_top(self.sel, self.top, self.rows.len());
    }

    /// Move the cursor to a row and bring it onto the screen.
    fn focus(&mut self, at: usize) {
        self.sel = at.min(self.rows.len().saturating_sub(1));
        self.reveal_cursor();
    }

    /// Take in that a setting moved: the rows are read again, the cursor is kept on show, and the
    /// preview is told. Whatever the change needed of the library has been asked for by now.
    fn settings_moved(&mut self, ctx: &mut FrameCtx<'_>) {
        self.refresh(ctx.shared);
        self.reveal_cursor();
        self.preview.settings_changed(ctx.shared);
    }

    /// Put one row that holds a number on the step `steps`, and tell the preview when that moved
    /// it. An offset is read where it is stored by every frame, so the frame after shows it.
    fn set_number(&mut self, ctx: &mut FrameCtx<'_>, row: SettingRow, steps: i64) {
        if ctx.shared.set_row_number(row, steps) {
            self.settings_moved(ctx);
        }
    }

    /// Store what was typed into the row being typed into, and close its editor. What is not a
    /// number is not stored, and the row says so for a while.
    fn commit_edit(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some((row, typed)) = self.numbers.take_edit() else {
            return;
        };
        let Some(value) = ctx.shared.row_number(row) else {
            return;
        };
        if let Some(steps) = self.numbers.typed_steps(row, &value.spec, &typed, ctx.now) {
            self.set_number(ctx, row, steps);
        }
    }

    /// Carry the row being dragged to where the pointer is along its slider.
    fn drag_to(&mut self, ctx: &mut FrameCtx<'_>, x: f32) {
        let Some((row, _)) = self.numbers.dragged() else {
            return;
        };
        if let Some(steps) = ctx.shared.row_number(row).and_then(|value| self.numbers.dragged_to(&value, x)) {
            self.set_number(ctx, row, steps);
        }
    }

    /// End a drag, and write the settings file: once for the whole drag.
    fn end_drag(&mut self, ctx: &mut FrameCtx<'_>) {
        if self.numbers.end_drag() {
            ctx.shared.save_settings();
        }
    }

    /// A press of a button on `row`, which the last frame drew as `hot`. On a row that holds a
    /// number, a press of the left button on the slider begins a drag -- the row is put where the
    /// press landed, unless it landed on the knob, which is taken hold of where it stands -- and
    /// one on the number opens the row to be typed into. Any other press steps the row once,
    /// forwards or back as `delta` says.
    fn press_row(&mut self, ctx: &mut FrameCtx<'_>, row: SettingRow, hot: Hot, at: (f32, f32), button: MouseButton, delta: i32) {
        let number = ctx.shared.row_number(row).filter(|_| button == MouseButton::Left);
        match number.map(|value| (value, number_part_at(ctx.shared, hot, at))) {
            Some((value, RowPart::Slider(area))) => {
                if !self.numbers.begin_drag(row, area, &value, at.0) {
                    self.drag_to(ctx, at.0);
                }
            }
            Some((value, RowPart::Number)) => self.numbers.begin_edit(row, &value),
            Some((_, RowPart::Rest)) | None => self.step(ctx, row, delta, false),
        }
    }

    /// Apply one left/right step to a row, and tell the preview when it moved something. A row that
    /// holds a number moves one step of the smallest change it can hold, and ten when `wide` asks;
    /// any other row is stepped once, whatever is asked.
    fn step(&mut self, ctx: &mut FrameCtx<'_>, row: SettingRow, delta: i32, wide: bool) {
        if let Some(value) = ctx.shared.row_number(row) {
            self.set_number(ctx, row, value.stepped(delta, wide));
            return;
        }
        let moved = match row {
            SettingRow::Skin(row) => SkinMenuState::step_document_row(ctx.shared, row, delta),
            SettingRow::Fixed(id) => match adjust(&mut ctx.shared.config, id, delta) {
                AdjustOutcome::Action(action) => self.run_action(ctx.shared, action, delta),
                AdjustOutcome::Changed => {
                    if id == SettingId::SkinScreen {
                        ctx.shared.reload_skin();
                    }
                    true
                }
                AdjustOutcome::Unchanged => false,
            },
        };
        if moved {
            self.settings_moved(ctx);
        }
    }

    /// Step a property or a file slot of the chosen document, and report whether it moved.
    ///
    /// Either changes what is drawn, so the document is read again. An offset is read live out of the
    /// stored choices and needs no reload, which is why it is not stepped here.
    fn step_document_row(shared: &mut AppShared, row: SkinRow, delta: i32) -> bool {
        if !shared.step_skin_row(row, delta) {
            return false;
        }
        if shared.skin_reload_pending() {
            shared.reload_skin();
        }
        true
    }

    /// A row the configuration document cannot step on its own, and report whether a setting moved.
    ///
    /// The pack folder opens a picker to the right and names no pack to the left; the document row
    /// cycles either way; a reload and a reset run to the right alone. Each writes the settings file
    /// where the settings screen's row of the same name always did.
    fn run_action(&mut self, shared: &mut AppShared, id: SettingId, delta: i32) -> bool {
        match id {
            SettingId::SkinPack if delta < 0 => {
                shared.set_skin_pack(None);
                shared.save_settings();
                true
            }
            SettingId::SkinPack => {
                if self.pack_picker.is_none() {
                    self.pack_picker = Some((self.open_pack_picker)());
                }
                false
            }
            SettingId::SkinDocument => {
                let moved = shared.cycle_skin_document(delta);
                if moved {
                    shared.reload_skin();
                }
                moved
            }
            SettingId::SkinReload if delta > 0 => {
                shared.rescan_skins_afresh();
                shared.reload_skin();
                shared.save_settings();
                true
            }
            SettingId::SkinReset if delta > 0 => {
                shared.reset_skin_document();
                shared.save_settings();
                true
            }
            _ => false,
        }
    }

    /// Put the focused row back to what the document's author chose, when it is a row that holds a
    /// number: the one row kind with a value a key can put back rather than cycle. An offset goes
    /// back to none, which for an opacity is the whole of it.
    fn reset_focused_row(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some(row) = self.focused() else {
            return;
        };
        if let Some(value) = ctx.shared.row_number(row) {
            self.set_number(ctx, row, value.shipped);
        }
    }

    /// Collect the skin pack picker's answer: the folder becomes the pack every screen takes its
    /// document from, and is written out so the next run starts with it.
    fn poll_pack_picker(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some(picker) = self.pack_picker.as_ref() else {
            return;
        };
        match picker.poll() {
            DialogState::Open => {}
            DialogState::Picked(path) => {
                self.pack_picker = None;
                ctx.shared.set_skin_pack(Some(path.to_string_lossy().into_owned()));
                ctx.shared.save_settings();
                self.settings_moved(ctx);
            }
            DialogState::Dismissed => self.pack_picker = None,
        }
    }

    /// Configure another screen: read the document chosen for it, and tell the preview.
    fn choose_screen(&mut self, ctx: &mut FrameCtx<'_>, screen: i32) {
        if ctx.shared.config.skin.screen == screen {
            return;
        }
        ctx.shared.config.skin.screen = screen;
        ctx.shared.reload_skin();
        self.settings_moved(ctx);
    }

    /// Move the screen being configured one along the strip of screens, round its ends. From a
    /// screen the strip does not offer, either direction lands on the strip's first.
    fn step_screen(&mut self, ctx: &mut FrameCtx<'_>, delta: i32) {
        let next = stepped_tab(kind_index(ctx.shared.config.skin.screen), SCREEN_KINDS.len(), delta);
        self.choose_screen(ctx, SCREEN_KINDS[next].0);
    }

    /// Keys on the row list, once the modifier and the picker have had their say.
    fn row_key(&mut self, ctx: &mut FrameCtx<'_>, code: KeyCode) -> Transition {
        match code {
            KeyCode::Escape => {
                ctx.shared.save_settings();
                return Transition::Back;
            }
            KeyCode::Tab => self.step_screen(ctx, if self.shift_held { PREVIOUS_SCREEN } else { NEXT_SCREEN }),
            KeyCode::ArrowUp => self.focus(self.sel.saturating_sub(1)),
            KeyCode::ArrowDown => self.focus(self.sel + 1),
            KeyCode::PageUp => self.focus(self.sel.saturating_sub(VISIBLE_ROWS)),
            KeyCode::PageDown => self.focus(self.sel + VISIBLE_ROWS),
            KeyCode::ArrowLeft => self.step_focused(ctx, ROW_STEP_BACK, self.shift_held),
            KeyCode::ArrowRight => self.step_focused(ctx, ROW_STEP_FORWARD, self.shift_held),
            KeyCode::Enter | KeyCode::NumpadEnter => self.enter_focused(ctx),
            KeyCode::Digit0 | KeyCode::Numpad0 => {
                self.reveal_cursor();
                self.reset_focused_row(ctx);
            }
            _ => {}
        }
        Transition::Stay
    }

    /// Enter on the row under the cursor: a row that holds a number is opened to be typed into, and
    /// any other row is stepped forwards, which runs the ones that are run.
    fn enter_focused(&mut self, ctx: &mut FrameCtx<'_>) {
        self.reveal_cursor();
        let Some(row) = self.focused() else {
            return;
        };
        match ctx.shared.row_number(row) {
            Some(value) => self.numbers.begin_edit(row, &value),
            None => self.step(ctx, row, ROW_STEP_FORWARD, false),
        }
    }

    /// Step the row under the cursor, with the row on show. A press of an arrow with Shift held
    /// asks for a wide step: an offset runs over thousands of steps, and a preview is where it is
    /// set by eye.
    fn step_focused(&mut self, ctx: &mut FrameCtx<'_>, delta: i32, wide: bool) {
        self.reveal_cursor();
        if let Some(row) = self.focused() {
            self.step(ctx, row, delta, wide);
        }
    }

    /// Draw the rows on show ([`draw_list`]). A value too long for its row is cut short at its end,
    /// except the pack folder's, which is a path: it keeps its end ([`fit_path`]) and has all the
    /// room its row's short label leaves. A row that holds a number is drawn with its slider and
    /// reads as its number alone, in the room a number has -- what an opacity past all of it adds
    /// is said under the preview -- or with the figure being typed into it. `now` is what a row that
    /// refused a typed figure stops saying so against.
    fn draw_rows(&self, shared: &mut AppShared, canvas: &mut Canvas<'_>, now: std::time::Instant) {
        let list = ListWindow { len: self.lines.len(), sel: self.sel, top: self.top };
        let entry = |index: usize| {
            let SettingsLine { label, value, number, .. } = &self.lines[index];
            let row = self.rows.get(index).copied();
            let value = match (row, number) {
                (Some(SettingRow::Fixed(SettingId::SkinPack)), _) => fit_path(value, VALUE_SCALE, pack_value_room(label)),
                (_, Some(number)) => number.spec.text(number.steps),
                _ => fitted_value(value),
            };
            let number = (*number).zip(row).map(|(number, row)| NumberCell {
                fraction: number.fraction(),
                dragged: self.numbers.is_dragging(row),
                edit: self.numbers.edit_of(row),
                refused: self.numbers.is_refused(row, now),
            });
            ListEntry { label, value, look: RowLook::Plain, number }
        };
        draw_list(shared, canvas, list, entry, Hot::SkinMenuRow);
    }

    /// Draw what is said under the preview: what loaded, in full, and the row under the cursor with
    /// its label and value in full and what the row is for.
    fn draw_info(&self, shared: &AppShared, canvas: &mut Canvas<'_>) {
        draw_info_panel(canvas);
        if let Some(loaded) = self.loaded_line() {
            draw_info_lead(canvas, &loaded.label, &loaded.value);
        }
        if self.pack_picker.is_some() {
            draw_info_note(canvas, PICKER_NOTE);
            return;
        }
        if let (Some(row), Some(line)) = (self.focused(), self.lines.get(self.sel)) {
            draw_info_row(canvas, &line.label, &line.value, row_help(shared, row));
        }
    }
}

/// Where a screen is on the strip of screens, or `None` for one the strip does not offer.
fn kind_index(screen: i32) -> Option<usize> {
    SCREEN_KINDS.iter().position(|(kind, _)| *kind == screen)
}

/// The room the pack folder's value is written in: all of its row but its label, written whole.
fn pack_value_room(label: &str) -> f32 {
    row_room() - text_width(label, LABEL_SCALE) - LABEL_GAP
}

/// What is said of one row under the preview. A row of the chosen document is said by the kind of
/// row the document makes it, and a row of an offset the game adds to every play skin as that; one
/// whose declaration is no longer there is said by what it was listed as.
fn row_help(shared: &AppShared, row: SettingRow) -> &'static str {
    let listed = match row {
        SettingRow::Fixed(id) => return descriptor(id).help,
        SettingRow::Skin(listed) => listed,
    };
    let kind = shared.skins.kind(&shared.config, listed).unwrap_or(match listed {
        SkinRow::Property(_) => SkinRowKind::Property,
        SkinRow::File(_) => SkinRowKind::File,
        SkinRow::Offset(_, axis) => SkinRowKind::Offset(axis),
    });
    match kind {
        SkinRowKind::Offset(axis) if shared.skins.is_automatic_for(&shared.config, shared.config.skin.screen, listed) => automatic_offset_help(axis),
        _ => skin_row_help(kind),
    }
}

/// What is said of one axis of an offset the game adds to every play skin.
pub(crate) fn automatic_offset_help(axis: OffsetAxis) -> &'static str {
    match axis {
        OffsetAxis::X | OffsetAxis::Y | OffsetAxis::W | OffsetAxis::H => AUTOMATIC_PLACE_HELP,
        OffsetAxis::R => OFFSET_ANGLE_HELP,
        OffsetAxis::A => AUTOMATIC_ALPHA_HELP,
    }
}

/// What is said of a row a skin document declares, by the kind of row it is.
pub(crate) fn skin_row_help(kind: SkinRowKind) -> &'static str {
    match kind {
        SkinRowKind::Property => PROPERTY_HELP,
        SkinRowKind::File => FILE_HELP,
        SkinRowKind::Offset(OffsetAxis::X | OffsetAxis::Y | OffsetAxis::W | OffsetAxis::H) => OFFSET_PLACE_HELP,
        SkinRowKind::Offset(OffsetAxis::R) => OFFSET_ANGLE_HELP,
        SkinRowKind::Offset(OffsetAxis::A) => OFFSET_ALPHA_HELP,
        SkinRowKind::Opacity => OPACITY_HELP,
    }
}

/// A value that holds a path, cut to `width` at `scale` by leaving out the beginning of the path
/// rather than the end of the value: a folder's own name is the last thing in its path and the
/// first thing looked for. What is written before the path -- that it is the default pack, that it
/// is missing -- is kept where it is, unless it would leave the path too little. A value that fits
/// is written whole.
fn fit_path(value: &str, scale: f32, width: f32) -> String {
    if text_width(value, scale) <= width {
        return value.to_string();
    }
    let gap = text_width(PATH_GAP, scale);
    let lead = value.find(PATH_SEPARATORS).map_or("", |path_at| &value[..path_at]);
    let lead = if text_width(lead, scale) + gap <= width * PATH_LEAD_MAX_SHARE { lead } else { "" };
    let room = width - text_width(lead, scale) - gap;
    let mut buffer = [0u8; UTF8_MAX_BYTES];
    let (mut tail, mut used) = (value.len(), 0.0);
    for (at, ch) in value.char_indices().rev() {
        used += text_width(ch.encode_utf8(&mut buffer), scale);
        if used > room || at < lead.len() {
            break;
        }
        tail = at;
    }
    format!("{lead}{PATH_GAP}{}", &value[tail..])
}

impl StageHandler for SkinMenuState {
    /// Walk the skin folder on the way in, so a document dropped into it while the game was running
    /// is offered, read the document of the screen being configured if it is not the one that was
    /// read, and build the rows.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        ctx.shared.rescan_skins();
        if ctx.shared.skin_reload_pending() {
            ctx.shared.reload_skin();
        }
        self.refresh(ctx.shared);
        self.reveal_cursor();
    }

    /// Collect the folder picker's answer, and bring the preview up to this frame.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.poll_pack_picker(ctx);
        self.preview.update(ctx);
        Transition::Stay
    }

    /// Escape writes the settings out and goes back to the screen the menu was opened over. Keys are
    /// ignored while the native folder picker is up: it has the keyboard, and the screen behind it
    /// must not act on what is typed there. A key pressed while a slider is being dragged ends the
    /// drag first: the key may move the cursor or the screen on, and the pointer must not go on
    /// carrying whatever row is then in that place.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if let Some(held) = shift_after(self.shift_held, &key) {
            self.shift_held = held;
            return Transition::Stay;
        }
        if !key.pressed || self.pack_picker.is_some() {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.end_drag(ctx);
        match self.numbers.edit_key(&key) {
            Some(EditKey::Taken) => return Transition::Stay,
            Some(EditKey::Cancel) => {
                self.numbers.cancel_edit();
                return Transition::Stay;
            }
            Some(EditKey::Commit) => {
                self.commit_edit(ctx);
                return Transition::Stay;
            }
            Some(EditKey::CommitThenPass) => self.commit_edit(ctx),
            None => {}
        }
        self.row_key(ctx, key.code)
    }

    /// A press on a row moves the cursor there and steps the row once: forwards with the left
    /// button, and back with the right, so a step taken by mistake can be taken back with the mouse.
    /// On a row that holds a number, a press of the left button on the slider puts the row where the
    /// press landed and begins a drag, and one on the number opens the row to be typed into; the
    /// left button coming back up ends a drag. A press of the left button on a tab of the strip
    /// configures that screen. A press anywhere else -- the preview included -- does nothing, but
    /// any press stores a number that was being typed.
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32), button: MouseButton, pressed: bool) -> Transition {
        if !pressed {
            if button == MouseButton::Left {
                self.end_drag(ctx);
            }
            return Transition::Stay;
        }
        let Some(delta) = click_step(button) else {
            return Transition::Stay;
        };
        if self.pack_picker.is_some() {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.end_drag(ctx);
        self.commit_edit(ctx);
        match ctx.shared.hit_test(at) {
            Some(hot @ Hot::SkinMenuRow(index)) => {
                if let Some(row) = self.rows.get(index).copied() {
                    self.sel = index;
                    self.press_row(ctx, row, hot, at, button, delta);
                }
            }
            Some(Hot::SkinMenuKind(index)) if is_left_press(button, pressed) => {
                if let Some((screen, _)) = SCREEN_KINDS.get(index) {
                    self.choose_screen(ctx, *screen);
                }
            }
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

    /// The wheel scrolls the list while the pointer is over it, and does nothing anywhere else --
    /// over a slider too, which a turn of the wheel never moves. The cursor stays on its row, which
    /// the next key that moves it brings back onto the screen.
    fn handle_scroll(&mut self, ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        if !lines.is_finite() || !contains(LIST_RECT, ctx.shared.cursor) {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.top = wheel_scrolled(&mut self.wheel_lines, self.top, self.rows.len(), lines);
        Transition::Stay
    }

    /// Whatever the preview compiled or was playing goes with the screen.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        self.preview.release(ctx.shared);
    }

    /// The menu's own half first, then the preview into its rectangle, then the status in its place
    /// when no frame of a skin is there -- or nothing but the dark, for a wait the preview asks to be
    /// left unsaid.
    ///
    /// What loaded is the one value that moves with nobody touching a row -- the preview reads the
    /// document a frame at a time -- so the rows are read again whenever it is not what they show.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        if self.loaded_line().map(|line| line.value.as_str()) != Some(ctx.shared.setting_line(SettingId::SkinInfo).1.as_str()) {
            self.dirty = true;
        }
        self.ensure_rows(ctx.shared);
        canvas.clear_bga();
        canvas.clear(theme().bg);
        draw_head(canvas, TITLE, &ctx.shared.setting_line(SettingId::SkinScreen).1);
        let chosen = kind_index(ctx.shared.config.skin.screen);
        draw_tabs(ctx.shared, canvas, SCREEN_KINDS.iter().map(|(_, name)| *name), chosen, Hot::SkinMenuKind);
        self.draw_rows(ctx.shared, canvas, ctx.now);
        self.draw_info(ctx.shared, canvas);
        draw_preview(&mut self.preview, ctx, canvas);
        draw_hint(canvas, if self.numbers.editing().is_some() { EDIT_HINT } else { HINT });
    }

    /// The browser's lines, and what the preview last left in its rectangle.
    fn debug_lines(&self, ctx: &FrameCtx<'_>) -> Vec<String> {
        let mut lines = ctx.shared.browser_debug_lines();
        lines.push(format!("SKIN PREVIEW {:?}", self.preview.status()));
        lines
    }
}

#[cfg(test)]
mod tests;
