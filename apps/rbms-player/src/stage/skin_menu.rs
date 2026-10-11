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
//! screen says where it goes ([`PREVIEW_RECT`]), fills the rectangle when no frame of a skin is in
//! it -- with what the preview says of it, or with nothing at all for a wait the preview asks to be
//! left dark -- and tells the preview when a setting moved. The preview takes no press of the mouse:
//! a button of a previewed skin does nothing.
//!
//! The screen is laid out in the fixed space every built-in screen is laid out in, and is opened
//! over the screen it was asked for from, which Escape goes back to.

use rbms_config::{AdjustOutcome, SettingId, SettingTab, adjust, descriptor};
use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_centered, draw_text_right, fit_text, text_width, theme};
use rbms_skin::loader::{
    SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS,
    SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_RESULT,
};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::dialog::{DialogHandle, DialogState, pick_skin_pack_folder};
use crate::settings_ui::SettingRow;
use crate::settings_view::SettingsLine;
use crate::skin_preview::region::with_region;
use crate::skin_preview::{PreviewStatus, SkinPreview};
use crate::skin_select::{OffsetAxis, SkinRow};
use crate::stage::{Canvas, FrameCtx, KeyInput, Stage, StageHandler, Transition, is_left_press};
use crate::{AppShared, CW, Hot};

/// The room kept between the screen's edges and everything on it.
const MARGIN: f32 = 24.0;

/// The gap between the list on the left and the preview on the right.
const COLUMN_GAP: f32 = 16.0;

/// Where the title is written, and how large.
const TITLE_Y: f32 = 16.0;
const TITLE_SCALE: f32 = 2.4;

/// Where the name of the screen being configured is written, beside the title.
const SCREEN_NAME_X: f32 = 132.0;
const SCREEN_NAME_Y: f32 = 26.0;
const SCREEN_NAME_SCALE: f32 = 1.6;

/// The top of the two columns, under the head.
const COLUMNS_TOP: f32 = 84.0;

/// The preview: as large as the screen leaves room for at the shape of a screen, against the right
/// edge. Sixteen by nine, on whole pixels at the size the screen is laid out for.
const PREVIEW_W: f32 = 832.0;
const PREVIEW_H: f32 = 468.0;

/// Where the preview goes, in the space this screen is laid out in.
pub(crate) const PREVIEW_RECT: Rect = Rect { x: CW as f32 - MARGIN - PREVIEW_W, y: COLUMNS_TOP, w: PREVIEW_W, h: PREVIEW_H };

/// The list of settings: from the left margin to the gap before the preview, and down to the hint.
pub(crate) const LIST_RECT: Rect = Rect { x: MARGIN, y: COLUMNS_TOP, w: PREVIEW_RECT.x - COLUMN_GAP - MARGIN, h: HINT_Y - HINT_GAP - COLUMNS_TOP };

/// What is said under the preview: about the document being drawn and the row under the cursor.
pub(crate) const INFO_RECT: Rect =
    Rect { x: PREVIEW_RECT.x, y: COLUMNS_TOP + PREVIEW_H + INFO_GAP, w: PREVIEW_W, h: HINT_Y - HINT_GAP - (COLUMNS_TOP + PREVIEW_H + INFO_GAP) };

/// The gap between the preview and what is said under it.
const INFO_GAP: f32 = 12.0;

/// Where the line of keys is written at the foot of the screen, how large, and the room above it.
const HINT_Y: f32 = 688.0;
const HINT_SCALE: f32 = 1.2;
const HINT_GAP: f32 = 12.0;

/// How thick the line round the preview is.
const PREVIEW_BORDER: f32 = 2.0;

/// The status written in the preview's place: how large its heading and what follows it are, and
/// how far the heading is above the first line of what follows.
const STATUS_SCALE: f32 = 1.8;
const STATUS_DETAIL_SCALE: f32 = 1.2;
const STATUS_LINE_GAP: f32 = 36.0;

/// How many lines what follows the heading is written over -- a reason a skin did not load is a
/// sentence with a file's name in it -- and how far one of them is from the next.
const STATUS_DETAIL_LINES: usize = 3;
const STATUS_DETAIL_PITCH: f32 = 18.0;

/// How far the detail's text is kept from the preview's own edges.
const STATUS_PAD: f32 = 24.0;

/// The strip of screens above the preview: where it is, how tall, and the gap between two tabs. The
/// tabs share its width evenly.
const KIND_STRIP_Y: f32 = 24.0;
const KIND_TAB_H: f32 = 32.0;
const KIND_TAB_GAP: f32 = 6.0;
const KIND_TAB_W: f32 = (PREVIEW_W - KIND_TAB_GAP * (SCREEN_KINDS.len() - 1) as f32) / SCREEN_KINDS.len() as f32;

/// How large a tab's name is written, and how far below the tab's top.
const KIND_SCALE: f32 = 1.5;
const KIND_TEXT_Y: f32 = 8.0;

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

/// The rows of the list: how far they are kept from the list's own edges, how far one is from the
/// next, how tall one is, and how many are on show at once.
const LIST_PAD: f32 = 4.0;
const ROW_PITCH: f32 = 33.0;
const ROW_H: f32 = 29.0;
const VISIBLE_ROWS: usize = 17;

/// How far a row's label and value are kept from the row's own edges, and how far below its top
/// each is written.
const ROW_PAD: f32 = 10.0;
const LABEL_TEXT_Y: f32 = 9.0;
const VALUE_TEXT_Y: f32 = 8.0;

/// How large a row's label and its value are written.
const LABEL_SCALE: f32 = 1.3;
const VALUE_SCALE: f32 = 1.4;

/// The widest a row's value is drawn, as a share of the row, and the room kept between the label and
/// it. The label has what the value leaves.
const VALUE_MAX_SHARE: f32 = 0.55;
const LABEL_GAP: f32 = 12.0;

/// The most of the room a path is given that what is written before it may take and still be kept
/// when the path is too long: that it is the default pack, that it is missing.
const PATH_LEAD_MAX_SHARE: f32 = 0.6;

/// What a path is cut at, on any platform.
const PATH_SEPARATORS: [char; 2] = ['/', '\\'];

/// What stands for the part of a path that was left out.
const PATH_GAP: &str = "\u{2026}";

/// The most bytes one character is written in, which is the buffer a character is measured from.
const UTF8_MAX_BYTES: usize = 4;

/// The scroll bar along the list's right edge: how wide it is, how far inside the edge, and the
/// shortest its thumb is drawn.
const SCROLL_BAR_W: f32 = 5.0;
const SCROLL_BAR_INSET: f32 = 3.0;
const SCROLL_THUMB_MIN_H: f32 = 16.0;

/// The room the scroll bar takes from the rows.
const SCROLL_BAR_ROOM: f32 = SCROLL_BAR_W + SCROLL_BAR_INSET * 2.0;

/// Where the cursor's place in the list is written, measured up from the list's foot, and how large.
const COUNTER_RISE: f32 = 20.0;
const COUNTER_SCALE: f32 = 1.2;

/// What is said under the preview: how far it is kept from the panel's edges, how large it is
/// written, how far one line is from the next, and where what loaded and the row under the cursor
/// begin, measured down from the panel's top.
const INFO_PAD: f32 = 16.0;
const INFO_SCALE: f32 = 1.3;
const INFO_LINE_PITCH: f32 = 17.0;
const INFO_LOADED_Y: f32 = 8.0;
const INFO_ROW_Y: f32 = 46.0;

/// How many lines the LOADED value is written over under the preview, and how many the value of the
/// row under the cursor is when it does not fit beside its label.
const INFO_LOADED_LINES: usize = 2;
const INFO_VALUE_LINES: usize = 2;

/// The room kept between the heading of a line under the preview and what follows it.
const INFO_HEADING_GAP: f32 = 12.0;

/// One step to the right on a row, which is what the right arrow, Enter and a press of the left
/// button all ask for, and one step to the left, which the left arrow and a press of the right
/// button ask for.
const ROW_STEP_FORWARD: i32 = 1;
const ROW_STEP_BACK: i32 = -1;

/// How many steps one press of an arrow moves an offset row by, and by how many with Shift held: an
/// offset runs over thousands of steps, and a preview is where it is set by eye.
const ONE_STEP: usize = 1;
const SHIFT_STEPS: usize = 10;

/// One step along the strip of screens: forwards, and back.
const NEXT_SCREEN: i32 = 1;
const PREVIOUS_SCREEN: i32 = -1;

/// The most rows one turn of the wheel moves the list by, whatever the device reports.
const WHEEL_ROWS_MAX: f32 = 64.0;

/// The screen's title.
const TITLE: &str = "SKIN";

/// The keys of the screen, written along its foot.
const HINT: &str =
    "UP DOWN MOVE   LEFT RIGHT CHANGE   SHIFT OFFSET x10   ENTER RUN   0 RESET OFFSET   TAB SCREEN   WHEEL SCROLL   RIGHT CLICK STEP BACK   ESC SAVE/BACK";

/// What is written under the preview in place of the row under the cursor while the folder picker is
/// up.
const PICKER_NOTE: &str = "CHOOSE THE PACK FOLDER IN THE WINDOW THAT OPENED";

/// What is said of a row the chosen document declares, by the kind of row it is: a row of the
/// settings table says what it is for itself (`rbms_config::SettingDescriptor::help`).
///
/// An offset is said by its axis. A place and a size are counted in pixels of the screen, except for
/// the one offset that moves a whole play screen, which is a share of it; a row is not told which
/// offset it is of, so both are said.
const PROPERTY_HELP: &str = "An option this skin declares";
const FILE_HELP: &str = "A file slot this skin declares: which of its files is used";
const OFFSET_PLACE_HELP: &str = "An offset of this skin, in pixels - in percent when it is of the whole screen. 0 puts it back";
const OFFSET_ANGLE_HELP: &str = "An offset of this skin: a turn, in degrees. 0 puts it back";
const OFFSET_ALPHA_HELP: &str = "An offset of this skin: added to the opacity, which runs to 255. 0 puts it back";

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
        self.top = self.top.min(self.last_top());
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

    /// The furthest the list can be scrolled: the last screenful.
    fn last_top(&self) -> usize {
        self.rows.len().saturating_sub(VISIBLE_ROWS)
    }

    /// Scroll the list just far enough to have the cursor's row on show.
    fn reveal_cursor(&mut self) {
        if self.sel < self.top {
            self.top = self.sel;
        } else if self.sel >= self.top + VISIBLE_ROWS {
            self.top = self.sel + 1 - VISIBLE_ROWS;
        }
        self.top = self.top.min(self.last_top());
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

    /// Apply one left/right step to a row, and tell the preview when it moved something. An offset
    /// row is stepped `steps` times over; any other row once, whatever is asked.
    fn step(&mut self, ctx: &mut FrameCtx<'_>, row: SettingRow, delta: i32, steps: usize) {
        let moved = match row {
            SettingRow::Skin(row @ SkinRow::Offset(_, _)) => (0..steps).filter(|_| ctx.shared.step_skin_row(row, delta)).count() > 0,
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

    /// Put the focused offset row back to what the document's author chose, which is the one row
    /// kind with a value a key can zero rather than cycle.
    fn reset_focused_row(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some(SettingRow::Skin(row)) = self.focused() else {
            return;
        };
        if ctx.shared.reset_skin_row(row) {
            self.settings_moved(ctx);
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
        let count = SCREEN_KINDS.len() as i32;
        let next = kind_index(ctx.shared.config.skin.screen).map_or(0, |at| (at as i32 + delta).rem_euclid(count) as usize);
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
            KeyCode::ArrowLeft => self.step_focused(ctx, ROW_STEP_BACK, self.arrow_steps()),
            KeyCode::ArrowRight => self.step_focused(ctx, ROW_STEP_FORWARD, self.arrow_steps()),
            KeyCode::Enter | KeyCode::NumpadEnter => self.step_focused(ctx, ROW_STEP_FORWARD, ONE_STEP),
            KeyCode::Digit0 | KeyCode::Numpad0 => {
                self.reveal_cursor();
                self.reset_focused_row(ctx);
            }
            _ => {}
        }
        Transition::Stay
    }

    /// How many steps a press of an arrow asks for: more with Shift held.
    fn arrow_steps(&self) -> usize {
        if self.shift_held { SHIFT_STEPS } else { ONE_STEP }
    }

    /// Step the row under the cursor, with the row on show.
    fn step_focused(&mut self, ctx: &mut FrameCtx<'_>, delta: i32, steps: usize) {
        self.reveal_cursor();
        if let Some(row) = self.focused() {
            self.step(ctx, row, delta, steps);
        }
    }

    /// Draw the head: the title, and the name of the screen being configured beside it.
    fn draw_head(shared: &AppShared, canvas: &mut Canvas<'_>) {
        let colors = theme();
        draw_text(canvas, MARGIN, TITLE_Y, TITLE_SCALE, colors.text, TITLE);
        let screen_name = shared.setting_line(SettingId::SkinScreen).1;
        let room = LIST_RECT.x + LIST_RECT.w - SCREEN_NAME_X;
        draw_text(canvas, SCREEN_NAME_X, SCREEN_NAME_Y, SCREEN_NAME_SCALE, Color::YELLOW, &fit_text(&screen_name, SCREEN_NAME_SCALE, room));
    }

    /// Draw the strip of screens over the preview, the one being configured lit, and record each tab
    /// as clickable.
    fn draw_kinds(shared: &mut AppShared, canvas: &mut Canvas<'_>) {
        let colors = theme();
        let chosen = kind_index(shared.config.skin.screen);
        for (at, (_, name)) in SCREEN_KINDS.iter().enumerate() {
            let on = chosen == Some(at);
            let rect = kind_rect(at);
            canvas.fill_rect(rect, if on { colors.button } else { colors.panel });
            draw_text_centered(canvas, rect.x + rect.w * 0.5, rect.y + KIND_TEXT_Y, KIND_SCALE, if on { Color::YELLOW } else { colors.text_dim }, name);
            shared.hot.push((rect, Hot::SkinMenuKind(at)));
        }
    }

    /// Draw the rows on show, the cursor's lit, with the scroll bar and the cursor's place in the
    /// list when there are more rows than fit, and record each drawn row as clickable. A value too
    /// long for its row is cut short at its end, except the pack folder's, which is a path: it keeps
    /// its end ([`fit_path`]) and has all the room its row's short label leaves.
    fn draw_list(&self, shared: &mut AppShared, canvas: &mut Canvas<'_>) {
        let colors = theme();
        canvas.fill_rect(LIST_RECT, colors.panel);
        let shown = self.top..(self.top + VISIBLE_ROWS).min(self.lines.len());
        for (slot, index) in shown.enumerate() {
            let SettingsLine { label, value, .. } = &self.lines[index];
            let on = index == self.sel;
            let rect = row_rect(slot);
            if on {
                canvas.fill_rect(rect, colors.button);
            }
            let room = rect.w - ROW_PAD * 2.0;
            let value = match self.rows.get(index) {
                Some(SettingRow::Fixed(SettingId::SkinPack)) => fit_path(value, VALUE_SCALE, pack_value_room(label)),
                _ => fit_text(value, VALUE_SCALE, room * VALUE_MAX_SHARE),
            };
            let label = fit_text(label, LABEL_SCALE, room - text_width(&value, VALUE_SCALE) - LABEL_GAP);
            draw_text(canvas, rect.x + ROW_PAD, rect.y + LABEL_TEXT_Y, LABEL_SCALE, if on { colors.text } else { colors.text_dim }, &label);
            draw_text_right(canvas, rect.x + rect.w - ROW_PAD, rect.y + VALUE_TEXT_Y, VALUE_SCALE, if on { Color::YELLOW } else { colors.text }, &value);
            shared.hot.push((rect, Hot::SkinMenuRow(index)));
        }
        if self.lines.len() <= VISIBLE_ROWS {
            return;
        }
        canvas.fill_rect(scroll_track(), colors.panel_hi);
        canvas.fill_rect(scroll_thumb(self.top, self.lines.len()), colors.text_muted);
        let counter = format!("{} / {}", self.sel + 1, self.lines.len());
        draw_text_right(canvas, LIST_RECT.x + LIST_RECT.w - ROW_PAD, LIST_RECT.y + LIST_RECT.h - COUNTER_RISE, COUNTER_SCALE, colors.text_muted, &counter);
    }

    /// Draw what is said under the preview: what loaded, in full, and the row under the cursor with
    /// its label and value in full and what the row is for. A value that does not fit beside its
    /// label -- a folder's path -- is written under it, over as many lines as it is given.
    fn draw_info(&self, canvas: &mut Canvas<'_>) {
        let colors = theme();
        canvas.fill_rect(INFO_RECT, colors.panel);
        let left = INFO_RECT.x + INFO_PAD;
        let room = INFO_RECT.w - INFO_PAD * 2.0;

        if let Some(loaded) = self.loaded_line() {
            draw_text(canvas, left, INFO_RECT.y + INFO_LOADED_Y, INFO_SCALE, colors.text_muted, &loaded.label);
            let indent = text_width(&loaded.label, INFO_SCALE) + INFO_HEADING_GAP;
            for (at, line) in wrapped(&loaded.value, INFO_SCALE, room - indent, INFO_LOADED_LINES).iter().enumerate() {
                draw_text(canvas, left + indent, INFO_RECT.y + INFO_LOADED_Y + at as f32 * INFO_LINE_PITCH, INFO_SCALE, colors.text_dim, line);
            }
        }

        if self.pack_picker.is_some() {
            draw_text(canvas, left, INFO_RECT.y + INFO_ROW_Y, INFO_SCALE, colors.accent, PICKER_NOTE);
            return;
        }
        let (Some(row), Some(line)) = (self.focused(), self.lines.get(self.sel)) else {
            return;
        };
        let label = fit_text(&line.label, INFO_SCALE, room);
        let mut y = INFO_RECT.y + INFO_ROW_Y;
        draw_text(canvas, left, y, INFO_SCALE, colors.text, &label);
        if text_width(&label, INFO_SCALE) + INFO_HEADING_GAP + text_width(&line.value, INFO_SCALE) <= room {
            draw_text_right(canvas, left + room, y, INFO_SCALE, Color::YELLOW, &line.value);
            y += INFO_LINE_PITCH;
        } else {
            y += INFO_LINE_PITCH;
            for part in wrapped(&line.value, INFO_SCALE, room, INFO_VALUE_LINES) {
                draw_text(canvas, left, y, INFO_SCALE, Color::YELLOW, &part);
                y += INFO_LINE_PITCH;
            }
        }
        draw_text(canvas, left, y, INFO_SCALE, colors.text_muted, &fit_text(row_help(row), INFO_SCALE, room));
    }

    /// Fill the preview's rectangle and write `status` in it, for a frame the preview drew nothing
    /// on: a heading, and under it what follows from it or why, over as many lines as it is given.
    ///
    /// The fill goes through the region a frame of a skin would have been drawn in, so it covers the
    /// very pixels that frame covers: the preview is put on whole pixels of the target, which at a
    /// window that is not a whole multiple of this screen's size is not where the rectangle's own
    /// edges fall.
    fn draw_status(canvas: &mut Canvas<'_>, status: &PreviewStatus) {
        let colors = theme();
        with_region(canvas, PREVIEW_RECT, |region| region.clear(colors.panel));
        let middle = (PREVIEW_RECT.x + PREVIEW_RECT.w * 0.5, PREVIEW_RECT.y + PREVIEW_RECT.h * 0.5);
        if let Some(message) = status.message() {
            draw_text_centered(canvas, middle.0, middle.1 - STATUS_LINE_GAP, STATUS_SCALE, colors.text, message);
        }
        for (at, line) in status_detail_lines(status).iter().enumerate() {
            draw_text_centered(canvas, middle.0, middle.1 + at as f32 * STATUS_DETAIL_PITCH, STATUS_DETAIL_SCALE, colors.text_muted, line);
        }
    }

    /// Leave the preview's rectangle dark, for a wait the preview asks not to be written over
    /// ([`SkinPreview::waits_in_the_dark`]). The fill covers the pixels a frame of a skin covers, as
    /// the status panel's does.
    fn draw_dark(canvas: &mut Canvas<'_>) {
        with_region(canvas, PREVIEW_RECT, |region| region.clear(Color::BLACK));
    }
}

/// Where a screen is on the strip of screens, or `None` for one the strip does not offer.
fn kind_index(screen: i32) -> Option<usize> {
    SCREEN_KINDS.iter().position(|(kind, _)| *kind == screen)
}

/// Where the tab at one place of the strip of screens is drawn.
fn kind_rect(at: usize) -> Rect {
    Rect::new(PREVIEW_RECT.x + at as f32 * (KIND_TAB_W + KIND_TAB_GAP), KIND_STRIP_Y, KIND_TAB_W, KIND_TAB_H)
}

/// The room the pack folder's value is written in: all of its row but its label, written whole.
fn pack_value_room(label: &str) -> f32 {
    row_rect(0).w - ROW_PAD * 2.0 - text_width(label, LABEL_SCALE) - LABEL_GAP
}

/// Where the row in one slot of the list is drawn, the first slot being the top one on show.
fn row_rect(slot: usize) -> Rect {
    Rect::new(LIST_RECT.x + LIST_PAD, LIST_RECT.y + LIST_PAD + slot as f32 * ROW_PITCH, LIST_RECT.w - LIST_PAD - SCROLL_BAR_ROOM, ROW_H)
}

/// The whole length the scroll bar's thumb travels, which is drawn under it so the thumb's own
/// length reads against something.
fn scroll_track() -> Rect {
    Rect::new(LIST_RECT.x + LIST_RECT.w - SCROLL_BAR_INSET - SCROLL_BAR_W, LIST_RECT.y + LIST_PAD, SCROLL_BAR_W, VISIBLE_ROWS as f32 * ROW_PITCH)
}

/// Where the scroll bar's thumb is for a list of `len` rows whose first row on show is `top`: as
/// much of the bar as the rows on show are of the list, and as far down it as the list is scrolled.
fn scroll_thumb(top: usize, len: usize) -> Rect {
    let track = scroll_track();
    let rows = len.max(1) as f32;
    let height = (track.h * VISIBLE_ROWS as f32 / rows).clamp(SCROLL_THUMB_MIN_H.min(track.h), track.h);
    let travel = len.saturating_sub(VISIBLE_ROWS).max(1) as f32;
    Rect::new(track.x, track.y + (track.h - height) * (top as f32 / travel).min(1.0), track.w, height)
}

/// Whether a point of the screen is inside a rectangle of it.
fn contains(rect: Rect, at: (f32, f32)) -> bool {
    at.0 >= rect.x && at.0 < rect.x + rect.w && at.1 >= rect.y && at.1 < rect.y + rect.h
}

/// What is said of one row under the preview.
fn row_help(row: SettingRow) -> &'static str {
    match row {
        SettingRow::Fixed(id) => descriptor(id).help,
        SettingRow::Skin(SkinRow::Property(_)) => PROPERTY_HELP,
        SettingRow::Skin(SkinRow::File(_)) => FILE_HELP,
        SettingRow::Skin(SkinRow::Offset(_, OffsetAxis::X | OffsetAxis::Y | OffsetAxis::W | OffsetAxis::H)) => OFFSET_PLACE_HELP,
        SettingRow::Skin(SkinRow::Offset(_, OffsetAxis::R)) => OFFSET_ANGLE_HELP,
        SettingRow::Skin(SkinRow::Offset(_, OffsetAxis::A)) => OFFSET_ALPHA_HELP,
    }
}

/// What is written under a status's heading in the preview's place, broken over the lines it is
/// given.
fn status_detail_lines(status: &PreviewStatus) -> Vec<String> {
    status.detail().map_or_else(Vec::new, |detail| wrapped(detail, STATUS_DETAIL_SCALE, PREVIEW_RECT.w - STATUS_PAD * 2.0, STATUS_DETAIL_LINES))
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

/// `text` broken into at most `lines` lines no wider than `width` at `scale`: at a space where a line
/// has one, and anywhere where it has none. What does not fit the last line is cut short.
///
/// A break `text` brings with it is not one of them: a line feed, a tab or any other control
/// character is read as a space, so what is written never runs to more lines than it was given.
fn wrapped(text: &str, scale: f32, width: f32, lines: usize) -> Vec<String> {
    let text: String = text.chars().map(|ch| if ch.is_control() { ' ' } else { ch }).collect();
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() && out.len() < lines {
        let cut = if out.len() + 1 == lines { rest.len() } else { line_break(rest, scale, width) };
        if cut == 0 || cut >= rest.len() {
            out.push(fit_text(rest, scale, width));
            break;
        }
        out.push(rest[..cut].trim_end().to_string());
        rest = rest[cut..].trim_start();
    }
    out
}

/// Where the first line of `text` ends when it is broken to `width` at `scale`, as an offset into
/// it: the last space before it runs past the width, the character it runs past the width on when
/// there was no space, and its end when all of it fits.
///
/// The width is the sum of the characters' own, as `fit_text` measures a line it cuts short.
fn line_break(text: &str, scale: f32, width: f32) -> usize {
    let mut used = 0.0;
    let mut space = None;
    let mut buffer = [0u8; UTF8_MAX_BYTES];
    for (at, ch) in text.char_indices() {
        used += text_width(ch.encode_utf8(&mut buffer), scale);
        if used > width {
            return space.unwrap_or(at);
        }
        if ch == ' ' {
            space = Some(at);
        }
    }
    text.len()
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
    /// must not act on what is typed there.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if matches!(key.code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            self.shift_held = key.pressed || (self.shift_held && !key.released);
            return Transition::Stay;
        }
        if !key.pressed || self.pack_picker.is_some() {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.row_key(ctx, key.code)
    }

    /// A press on a row moves the cursor there and steps the row once: forwards with the left
    /// button, and back with the right, so a step taken by mistake can be taken back with the mouse.
    /// A press of the left button on a tab of the strip configures that screen. A press anywhere
    /// else -- the preview included -- does nothing.
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32), button: MouseButton, pressed: bool) -> Transition {
        let delta = match button {
            MouseButton::Left => ROW_STEP_FORWARD,
            MouseButton::Right => ROW_STEP_BACK,
            _ => return Transition::Stay,
        };
        if !pressed || self.pack_picker.is_some() {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        match ctx.shared.hit_test(at) {
            Some(Hot::SkinMenuRow(index)) => {
                if let Some(row) = self.rows.get(index).copied() {
                    self.sel = index;
                    self.step(ctx, row, delta, ONE_STEP);
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

    /// The wheel scrolls the list while the pointer is over it, and does nothing anywhere else. The
    /// cursor stays on its row, which the next key that moves it brings back onto the screen.
    fn handle_scroll(&mut self, ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        if !lines.is_finite() || !contains(LIST_RECT, ctx.shared.cursor) {
            return Transition::Stay;
        }
        self.ensure_rows(ctx.shared);
        self.wheel_lines = (self.wheel_lines + lines).clamp(-WHEEL_ROWS_MAX, WHEEL_ROWS_MAX);
        let whole = self.wheel_lines.trunc();
        self.wheel_lines -= whole;
        let last = self.last_top();
        self.top = if whole < 0.0 { self.top.saturating_sub((-whole) as usize) } else { (self.top + whole as usize).min(last) };
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
        let colors = theme();
        canvas.clear_bga();
        canvas.clear(colors.bg);
        SkinMenuState::draw_head(ctx.shared, canvas);
        SkinMenuState::draw_kinds(ctx.shared, canvas);
        self.draw_list(ctx.shared, canvas);
        self.draw_info(canvas);

        let border = Rect::new(
            PREVIEW_RECT.x - PREVIEW_BORDER,
            PREVIEW_RECT.y - PREVIEW_BORDER,
            PREVIEW_RECT.w + PREVIEW_BORDER * 2.0,
            PREVIEW_RECT.h + PREVIEW_BORDER * 2.0,
        );
        canvas.fill_rect(border, colors.divider);
        self.preview.draw(ctx, canvas, PREVIEW_RECT);
        if self.preview.waits_in_the_dark() {
            SkinMenuState::draw_dark(canvas);
        } else if !self.preview.status().is_drawn() {
            SkinMenuState::draw_status(canvas, self.preview.status());
        }

        draw_text(canvas, MARGIN, HINT_Y, HINT_SCALE, colors.text_muted, HINT);
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
