//! The play menu: the play options on the left, and on the right what they do -- a play screen of
//! the chosen mode, drawn by its skin on a sample chart as the options are changed.
//!
//! It is the skin menu's twin ([`crate::stage::SkinMenuState`]) and is made of the same parts
//! ([`crate::stage::preview_menu`]): the same title, strip of tabs, list, preview, panel under it
//! and line of keys, in the same places. What differs is what the rows are and what the preview is
//! of.
//!
//! The rows are the settings a player changes from one sitting to the next: how fast the notes
//! scroll and what the lane is covered with, when a press counts, and how a chart is dealt out.
//! They are the same fields the settings screen has always written, in the same settings file, and
//! with them the options and the offsets of the play skin of the previewed mode ([`rows`]): how
//! wide the lanes are, where the judgement is written, how opaque each part of the screen is.
//! Those are the document's own and are stored where the skin menu stores them, so either menu
//! reads what the other wrote.
//!
//! Two values of this screen are not settings and are stored nowhere: the mode on the strip of tabs
//! and the tempo of the sample chart. They choose what the options are shown on ([`PlayTarget`]).
//! A green number that is held is the same at every tempo and the scroll speed is not; a stored
//! scroll speed is the same at every tempo and the green number is not. Stepping the tempo is how
//! that is seen.
//!
//! The right half is not this screen's. [`SkinPreview`] draws it, and what the scroll comes to at
//! the target on show ([`PlayTarget::readout`]) is written under it, from the same functions the
//! play screen works its own speed out with. This screen writes the settings and tells the preview
//! that one moved; the mode and the tempo it hands over as what to show.
//!
//! A row that holds a number -- a speed, a cover, the judge offset, an offset of the play skin -- is
//! set as such a row is on every built-in screen ([`crate::number_row`]): an arrow moves it one step
//! of the smallest change it can hold, and ten with Shift held; its slider is dragged, and the
//! preview follows it frame by frame; and Enter, or a press on the number, opens it to be typed
//! into. A drag writes the settings file once, when the button comes back up. In the list such a row
//! reads as its number alone, in the room its slider leaves; what the number comes to -- the white
//! number of a cover, the travel time of a green number -- is said under the preview.
//!
//! A colour the play skin lets the player set is one row in the COLOR fold, read as its colour: the
//! arrows go round a list of colours keeping its alpha, Enter opens it to have a colour typed in hex,
//! and the zero key gives it back to the skin. Its red, green, blue and alpha are rows of their own
//! in the COLOR CHANNELS fold, set as any number is. A colour that was set already is read where it
//! is stored by every frame; one that comes to be set, or stops being set, has the document read
//! again once ([`crate::skin_select::SkinLibrary::set_color_for`]).
//!
//! The colour the notes SCRATCH AUTO plays are drawn in is set the same way, from the two rows under
//! SCRATCH AUTO: one read as the colour and a fold of its four channels. It is the player's own and
//! is drawn on every skin, so it is stored in the play options and not with any document, and
//! nothing is read again when it moves. The preview does not play SCRATCH AUTO, so the square of the
//! colour on its row is where it is seen.
//!
//! A row of the play skin that other screens' skins declare under the same name -- the five play
//! modes share most of theirs -- says so under the preview, and the key the skin menu has for it
//! ([`APPLY_TO_ALL_KEY`]) puts the row's value onto those screens here too.
//!
//! The screen is opened over the screen it was asked for from, which Escape goes back to after the
//! settings have been written out.

use rbms_config::{ScoreTarget, SettingId, skin_screen_label};
use rbms_render::{Color, Renderer, theme};
use rbms_skin::dst::SkinColor;
use rbms_skin::loader::{SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS, SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::app_play::scratch_auto_color;
use crate::notify::{Level, notify};
use crate::number_row::{EditKey, NumberInput, NumberValue, RowPart};
use crate::skin_preview::target::SpeedReadout;
use crate::skin_preview::{PlayTarget, SkinPreview};
use crate::skin_select::{CHANNEL_STEPS, ColorChannel, SkinRow, SkinRowKind, color_value, hex_color, preset_after};
use crate::stage::preview_menu::{
    ColorCell, INFO_ROW_UNDER_FIGURES_Y, InfoFigure, LIST_RECT, ListEntry, ListWindow, NEXT_TAB, NumberCell, PREVIOUS_TAB, ROW_STEP_BACK, ROW_STEP_FORWARD,
    RowLook, VISIBLE_ROWS, click_step, contains, draw_head, draw_hint, draw_info_figures, draw_info_panel, draw_info_row_noted, draw_list, draw_preview,
    draw_tabs, fitted_value, fold_value, last_top, number_part_at, revealed_top, shift_after, stepped_tab, wheel_scrolled,
};
use crate::stage::skin_menu::{APPLY_TO_ALL_KEY, automatic_offset_help, shared_row_note};
use crate::stage::{Canvas, FrameCtx, KeyInput, Stage, StageHandler, Transition, is_left_press};
use crate::{AppShared, Hot};

mod rows;

use rows::{
    BPM_UNIT, Fold, Folds, Line, PREVIEW_BPM_LABEL, Row, SCRATCH_AUTO_COLOR_LABEL, bpm_value, folded, help, hispeed_figure, is_read_only, listed,
    number_figure, number_of, reset_setting, setting_value, step_setting,
};

/// The screen's title.
const TITLE: &str = "PLAY";

/// The modes the strip offers, left to right, each with the short name on its tab: the play screens
/// a sample chart is written for, in the order the skin menu lists them.
pub(crate) const PREVIEW_MODES: [(i32, &str); 5] =
    [(SKIN_TYPE_PLAY_7KEYS, "7K"), (SKIN_TYPE_PLAY_5KEYS, "5K"), (SKIN_TYPE_PLAY_14KEYS, "14K"), (SKIN_TYPE_PLAY_10KEYS, "10K"), (SKIN_TYPE_PLAY_9KEYS, "9K")];

/// The tempos the sample chart can be written at, slowest first, and which of them the menu opens
/// on.
pub(crate) const PREVIEW_BPMS: [u32; 5] = [100, 120, 150, 180, 200];
const OPENING_BPM: usize = 2;

/// What each figure of the summary under the preview is called, left to right.
const FIGURE_BPM: &str = BPM_UNIT;
const FIGURE_HISPEED: &str = "HI-SPEED";
const FIGURE_TRAVEL: &str = "TRAVEL";
const FIGURE_GREEN: &str = "GREEN";
const FIGURE_WHITE: &str = "WHITE";

/// What is written after the tempo the summary is worked out at while the field scrolls at one
/// tempo of its own, which is why it is not the tempo of the sample chart.
const CONSTANT_MARK: &str = "(CONSTANT)";

/// What is written after the scroll speed while it is worked out from a held green number, and
/// while the speed that would hold the number is outside what a speed can be.
const HELD_MARK: &str = "(HELD)";
const LIMIT_MARK: &str = "(LIMIT)";

/// What the travel time is measured in, written after it.
const TRAVEL_UNIT: &str = "MS";

/// What is written after the white number while the lane cover it measures is switched off.
const COVER_OFF_MARK: &str = "(COVER OFF)";

/// What is said under the figures while the scroll speed stands at an end of its range.
const LIMIT_NOTE: &str = "HI-SPEED IS AT THE END OF ITS RANGE: NO SPEED HOLDS THIS GREEN NUMBER AT THIS TEMPO";

/// What is said under the TARGET RATE row while neither target names the rate it holds.
const TARGET_RATE_UNUSED_NOTE: &str = "NOT IN USE: NO TARGET IS RATE CUSTOM";

/// What is said under the SCRATCH AUTO row: the turntable notes it plays count for nothing, and a
/// run it plays one for is an assist, whose clear is named for the gauge it was cleared on, from
/// ASSIST EASY to ASSIST EXHARD.
const SCRATCH_AUTO_NOTE: &str = "WHILE ON: TURNTABLE NOT COUNTED, ASSIST LAMP BY GAUGE (E.G. ASSIST HARD)";

/// The keys of the screen, written along its foot.
const HINT: &str = "UP DOWN MOVE   LEFT RIGHT CHANGE   SHIFT x10   ENTER NEXT OR TYPE   0 DEFAULT   A ALL SCREENS   DRAG A SLIDER   TAB MODE   WHEEL SCROLL   RIGHT CLICK BACK   ESC SAVE";

/// The play menu's own state: the preview on its right, the mode and the tempo it is shown on, the
/// rows it lists and where the cursor and the list are, and the modifier that turns the key for the
/// next mode into the key for the one before and one step of a number into ten.
///
/// `rows` is rebuilt when `dirty` says the list may have changed, which is when the mode moved --
/// the options and the offsets of the play skin are another document's -- and when a fold was
/// opened or closed. What each row reads is worked out as it is drawn. `top` is the first entry on
/// show: the cursor is brought back onto the screen by every key that moves it or the row under
/// it, and the wheel moves the list alone.
pub(crate) struct PlayMenuState {
    preview: SkinPreview,
    shift_held: bool,
    /// Which of [`PREVIEW_MODES`] is previewed.
    mode: usize,
    /// Which of [`PREVIEW_BPMS`] the sample chart is written at.
    bpm: usize,
    sel: usize,
    top: usize,
    rows: Vec<Row>,
    dirty: bool,
    /// Which folds of the play skin's offsets have their rows on show.
    folds: Folds,
    /// The part of a line the wheel has turned that has not moved the list yet.
    wheel_lines: f32,
    /// Which row that holds a number is being typed into, and which is being dragged.
    numbers: NumberInput<Row>,
}

impl PlayMenuState {
    pub(crate) fn new() -> PlayMenuState {
        let (mode, bpm) = (0, OPENING_BPM);
        PlayMenuState {
            preview: SkinPreview::of_play(target_at(mode, bpm)),
            shift_held: false,
            mode,
            bpm,
            sel: 0,
            top: 0,
            rows: Vec::new(),
            dirty: true,
            folds: Folds::default(),
            wheel_lines: 0.0,
            numbers: NumberInput::new(),
        }
    }

    /// The play menu as a screen to open over another.
    pub(crate) fn stage() -> Stage {
        Stage::PlayMenu(Box::new(PlayMenuState::new()))
    }

    /// What the preview is of: the mode on the strip and the tempo of the sample chart.
    fn target(&self) -> PlayTarget {
        target_at(self.mode, self.bpm)
    }

    /// Preview another mode, or the same one at another tempo, and tell the preview. Another mode
    /// is drawn by another document, whose options the list is read again for.
    fn retarget(&mut self, mode: usize, bpm: usize) {
        let mode = mode.min(PREVIEW_MODES.len() - 1);
        self.dirty |= mode != self.mode;
        self.mode = mode;
        self.bpm = bpm.min(PREVIEW_BPMS.len() - 1);
        self.preview.show_play(self.target());
    }

    /// Move the mode one along the strip, round its ends.
    fn step_mode(&mut self, delta: i32) {
        self.retarget(stepped_tab(Some(self.mode), PREVIEW_MODES.len(), delta), self.bpm);
    }

    /// Move the tempo of the sample chart one along its list, round its ends.
    fn step_bpm(&mut self, delta: i32) {
        self.retarget(self.mode, stepped_tab(Some(self.bpm), PREVIEW_BPMS.len(), delta));
    }

    /// The name of the mode being previewed, with the tempo of the sample chart after it. The
    /// screen's own name begins with the word the title already is, which is not said twice.
    fn subtitle(&self) -> String {
        let target = self.target();
        let screen = skin_screen_label(target.screen).unwrap_or_default();
        format!("{}  {}", screen.strip_prefix(TITLE).map_or(screen, str::trim_start), bpm_value(target.bpm))
    }

    /// Read the list again, keeping the cursor on the row it was on.
    ///
    /// The tempo row, a row of the settings table, a fold and a row of the SCRATCH AUTO colour are followed to wherever they now are,
    /// since the rows of the play skin come and go above them; a row of a skin is held by its
    /// place, because the row at that place of another document is the nearest thing to it.
    fn refresh(&mut self, shared: &AppShared) {
        let focused = self.focused();
        self.rows = listed(shared, self.target().screen, self.folds);
        if let Some(row @ (Row::PreviewBpm | Row::Setting(_) | Row::Fold(_) | Row::AutoColor | Row::AutoChannel(_))) = focused
            && let Some(at) = self.rows.iter().position(|entry| *entry == row)
        {
            self.sel = at;
        }
        self.sel = self.row_from(self.sel, ROW_STEP_FORWARD);
        self.top = self.top.min(last_top(self.rows.len()));
        self.dirty = false;
    }

    /// Read the list again if it may have changed since it was last built.
    fn ensure_rows(&mut self, shared: &AppShared) {
        if self.dirty {
            self.refresh(shared);
        }
    }

    /// The row the cursor is on.
    fn focused(&self) -> Option<Row> {
        self.rows.get(self.sel).copied()
    }

    /// The row nearest to the entry at `at` that the cursor can stand on: that entry, or the first
    /// row past it in the direction `towards`, or the first the other way when there is none.
    fn row_from(&self, at: usize, towards: i32) -> usize {
        let at = at.min(self.rows.len().saturating_sub(1));
        let is_row = |index: &usize| self.rows.get(*index).is_some_and(|row| !row.is_heading());
        let ahead = || (at..self.rows.len()).find(is_row);
        let behind = || (0..=at).rev().find(is_row);
        let found = if towards < 0 { behind().or_else(ahead) } else { ahead().or_else(behind) };
        found.unwrap_or(at)
    }

    /// Scroll the list just far enough to have the cursor's row on show, with the name of its group
    /// when the row is the first under it.
    fn reveal_cursor(&mut self) {
        let above = self.sel.saturating_sub(1);
        let from = if self.rows.get(above).is_some_and(|row| row.is_heading()) { above } else { self.sel };
        self.top = revealed_top(from, self.top, self.rows.len());
        self.top = revealed_top(self.sel, self.top, self.rows.len());
    }

    /// Scroll the list just far enough to have the rows of a fold that was opened on show under
    /// it: all of them when there is room for them, and as many as fit under the fold on the first
    /// line when there is not. A fold that showed nothing new when it was opened would read as one
    /// that did nothing.
    fn reveal_fold(&mut self, fold: Fold) {
        let Some(at) = self.rows.iter().position(|row| *row == Row::Fold(fold)) else {
            return;
        };
        let held = self.rows[at + 1..].iter().take_while(|row| matches!(row, Row::Skin(_) | Row::AutoChannel(_))).count();
        self.top = revealed_top(at + held, self.top, self.rows.len());
        self.top = revealed_top(at, self.top, self.rows.len());
    }

    /// Move the cursor to the row at `at`, or past a group's name to the next row in the direction
    /// `towards`, and bring it onto the screen.
    fn focus(&mut self, at: usize, towards: i32) {
        self.sel = self.row_from(at, towards);
        self.reveal_cursor();
    }

    /// Have the document that draws the previewed mode read again, once an option of it moved.
    fn reread_skin(&self, shared: &mut AppShared) {
        let screen = self.target().screen;
        if shared.skins.needs_reload_for(&shared.config, screen) {
            shared.skins.request_for(&shared.config, screen);
        }
    }

    /// The number one row holds as the menu shows it, or `None` for a row that holds none.
    fn number_of(&self, shared: &AppShared, row: Row) -> Option<NumberValue> {
        let target = self.target();
        number_of(shared, &target.readout(&shared.config.play), target.screen, row)
    }

    /// Put one row that holds a number on the step `steps`, and tell the preview when that moved
    /// it. A row whose number is worked out from the others is left as it is. A channel of a colour
    /// that was not set has the document read again, as the colour comes to be set.
    fn set_number(&mut self, ctx: &mut FrameCtx<'_>, row: Row, steps: i64) {
        let moved = match row {
            _ if is_read_only(&ctx.shared.config.play, row) => false,
            Row::Setting(id) => ctx.shared.set_row_number(crate::settings_ui::SettingRow::Fixed(id), steps),
            Row::Skin(skin_row) => {
                let moved = ctx.shared.skins.set_number_for(&mut ctx.shared.config, self.target().screen, skin_row, steps);
                if moved {
                    self.reread_skin(ctx.shared);
                }
                moved
            }
            Row::AutoChannel(channel) => {
                let held = scratch_auto_color(&ctx.shared.config.play);
                if let Ok(value) = u8::try_from(CHANNEL_STEPS.clamp(steps))
                    && value != channel.get(held)
                {
                    self.put_scratch_auto_color(ctx, Some(channel.set(held, value)));
                }
                false
            }
            Row::Group(_) | Row::PreviewBpm | Row::Fold(_) | Row::AutoColor => false,
        };
        if moved {
            self.preview.settings_changed(ctx.shared);
        }
    }

    /// Set the colour row at `at` of the previewed play skin to `color`, or give it back to the skin
    /// with `None`, and tell the preview when that moved it. A colour that comes to be set, or stops
    /// being set, has the document read again.
    fn put_color(&mut self, ctx: &mut FrameCtx<'_>, at: usize, color: Option<SkinColor>) {
        let screen = self.target().screen;
        let config = &mut ctx.shared.config;
        let moved = match color {
            Some(color) => ctx.shared.skins.set_color_for(config, screen, at, color),
            None => ctx.shared.skins.reset_color_for(config, screen, at),
        };
        if moved {
            self.reread_skin(ctx.shared);
            self.preview.settings_changed(ctx.shared);
        }
    }

    /// Set the colour SCRATCH AUTO's notes are drawn in to `color`, or give back the green they ship
    /// in with `None`, and tell the preview when that moved it. It is an engine value read by every
    /// frame, so no document is read again.
    fn put_scratch_auto_color(&mut self, ctx: &mut FrameCtx<'_>, color: Option<SkinColor>) {
        if ctx.shared.config.play.scratch_auto_color != color {
            ctx.shared.config.play.scratch_auto_color = color;
            self.preview.settings_changed(ctx.shared);
        }
    }

    /// Store what was typed into the row being typed into, and close its editor. What is not a
    /// number -- or, typed into a colour row, not a colour -- is not stored, and the row says so for
    /// a while.
    fn commit_edit(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some((row, typed)) = self.numbers.take_edit() else {
            return;
        };
        if let Row::Skin(SkinRow::Color(at)) = row {
            if let Some(color) = self.numbers.typed_color(row, &typed, ctx.now) {
                self.put_color(ctx, at, Some(color));
            }
            return;
        }
        if row == Row::AutoColor {
            if let Some(color) = self.numbers.typed_color(row, &typed, ctx.now) {
                self.put_scratch_auto_color(ctx, Some(color));
            }
            return;
        }
        let Some(value) = self.number_of(ctx.shared, row) else {
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
        if let Some(steps) = self.number_of(ctx.shared, row).and_then(|value| self.numbers.dragged_to(&value, x)) {
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
    /// forwards or back as `delta` says. A row whose number is worked out from the others takes the
    /// cursor and nothing else, wherever on it the press lands.
    fn press_row(&mut self, ctx: &mut FrameCtx<'_>, row: Row, hot: Hot, at: (f32, f32), button: MouseButton, delta: i32) {
        let number = self.number_of(ctx.shared, row).filter(|value| button == MouseButton::Left && !value.locked);
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

    /// Enter on the row under the cursor: a row that holds a number is opened to be typed into, a
    /// colour row is opened to have a colour typed in hex, and any other row is stepped forwards.
    fn enter_focused(&mut self, ctx: &mut FrameCtx<'_>) {
        self.reveal_cursor();
        let Some(row) = self.focused() else {
            return;
        };
        if let Row::Skin(SkinRow::Color(at)) = row {
            if let Some((color, _)) = ctx.shared.skins.color_for(&ctx.shared.config, self.target().screen, at) {
                self.numbers.begin_hex_edit(row, &hex_color(color));
            }
            return;
        }
        if row == Row::AutoColor {
            self.numbers.begin_hex_edit(row, &hex_color(scratch_auto_color(&ctx.shared.config.play)));
            return;
        }
        match self.number_of(ctx.shared, row) {
            Some(value) => self.numbers.begin_edit(row, &value),
            None => self.step(ctx, row, ROW_STEP_FORWARD, false),
        }
    }

    /// Apply one left/right step to a row, and tell the preview when a setting moved. A row that
    /// holds a number moves one step of the smallest change it can hold, and ten when `wide` asks;
    /// any other row is stepped once. A row whose value is worked out from the others is left as it
    /// is.
    ///
    /// The tempo of the sample chart is no setting: the preview is shown another target, and is not
    /// told a setting moved. Neither is a fold, which shows or hides its rows whichever way it is
    /// stepped.
    fn step(&mut self, ctx: &mut FrameCtx<'_>, row: Row, delta: i32, wide: bool) {
        if is_read_only(&ctx.shared.config.play, row) {
            return;
        }
        let moved = match row {
            Row::Group(_) => false,
            Row::PreviewBpm => {
                self.step_bpm(delta);
                false
            }
            Row::Setting(id) => step_setting(&mut ctx.shared.config, id, delta, wide),
            Row::Skin(skin_row) => self.step_skin_row(ctx.shared, skin_row, delta, wide),
            Row::Fold(fold) => {
                self.folds.toggle(fold);
                self.refresh(ctx.shared);
                self.reveal_cursor();
                if self.folds.is_open(fold) {
                    self.reveal_fold(fold);
                }
                false
            }
            Row::AutoColor => {
                if let Some(next) = preset_after(scratch_auto_color(&ctx.shared.config.play), delta) {
                    self.put_scratch_auto_color(ctx, Some(next));
                }
                false
            }
            Row::AutoChannel(_) => {
                if let Some(value) = self.number_of(ctx.shared, row) {
                    self.set_number(ctx, row, value.stepped(delta, wide));
                }
                false
            }
        };
        if moved {
            self.preview.settings_changed(ctx.shared);
        }
    }

    /// Step a row of the previewed play skin, and report whether it moved. An option changes what
    /// is drawn, so its document is read again; an offset is read where it is stored by every
    /// frame, so the frame after the step shows it with nothing read again.
    fn step_skin_row(&self, shared: &mut AppShared, row: SkinRow, delta: i32, wide: bool) -> bool {
        let screen = self.target().screen;
        let moved = match shared.skins.number_for(&shared.config, screen, row) {
            Some(value) => shared.skins.set_number_for(&mut shared.config, screen, row, value.stepped(delta, wide)),
            None => shared.skins.step_for(&mut shared.config, screen, row, delta),
        };
        if moved {
            self.reread_skin(shared);
        }
        moved
    }

    /// Step the row under the cursor, with the row on show.
    fn step_focused(&mut self, ctx: &mut FrameCtx<'_>, delta: i32, wide: bool) {
        self.reveal_cursor();
        if let Some(row) = self.focused() {
            self.step(ctx, row, delta, wide);
        }
    }

    /// Put the row under the cursor back to what it ships as: a setting to what a fresh
    /// configuration holds, the tempo to the one the menu opens on, an option of the skin to the
    /// item its author chose, an offset of it to none -- which for an opacity is all of it -- a colour
    /// of it back to the skin's own, and a channel of a colour to where the colour ships. A row
    /// whose value is worked out from the others is left as it is, and so is a fold.
    fn reset_focused(&mut self, ctx: &mut FrameCtx<'_>) {
        self.reveal_cursor();
        let Some(row) = self.focused().filter(|row| !is_read_only(&ctx.shared.config.play, *row)) else {
            return;
        };
        let moved = match row {
            Row::PreviewBpm => {
                self.retarget(self.mode, OPENING_BPM);
                false
            }
            Row::Setting(id) => reset_setting(&mut ctx.shared.config, id),
            Row::Skin(SkinRow::Property(at)) => {
                let moved = ctx.shared.skins.reset_property_for(&mut ctx.shared.config, self.target().screen, at);
                if moved {
                    self.reread_skin(ctx.shared);
                }
                moved
            }
            Row::Skin(number @ (SkinRow::Offset(_, _) | SkinRow::ColorChannel(_, _))) => {
                let screen = self.target().screen;
                let shipped = ctx.shared.skins.number_for(&ctx.shared.config, screen, number).map(|value| value.shipped);
                let moved = shipped.is_some_and(|shipped| ctx.shared.skins.set_number_for(&mut ctx.shared.config, screen, number, shipped));
                if moved {
                    self.reread_skin(ctx.shared);
                }
                moved
            }
            Row::Skin(SkinRow::Color(at)) => {
                self.put_color(ctx, at, None);
                false
            }
            Row::AutoColor => {
                self.put_scratch_auto_color(ctx, None);
                false
            }
            Row::AutoChannel(_) => {
                if let Some(value) = self.number_of(ctx.shared, row) {
                    self.set_number(ctx, row, value.shipped);
                }
                false
            }
            Row::Group(_) | Row::Skin(SkinRow::File(_) | SkinRow::Export | SkinRow::ColorChannels) | Row::Fold(_) => false,
        };
        if moved {
            self.preview.settings_changed(ctx.shared);
        }
    }

    /// Put the value of the row under the cursor onto every other screen whose skin declares the
    /// same row -- the other play modes, which share most of their options, and any other screen
    /// that names one the same -- and say how many took it, as the skin menu does for the same key.
    /// A row that is not a skin's, or that no other screen declares, is left alone.
    fn apply_focused_to_all(&mut self, ctx: &mut FrameCtx<'_>) {
        self.reveal_cursor();
        let Some(Row::Skin(row)) = self.focused() else {
            return;
        };
        let Some(applied) = ctx.shared.skins.apply_to_peers(&mut ctx.shared.config, self.target().screen, row) else {
            return;
        };
        ctx.shared.save_settings();
        notify(Level::Info, applied.message());
        self.preview.settings_changed(ctx.shared);
    }

    /// What is said of a row of the previewed play skin that other screens' skins declare too, or
    /// `None` for any other row.
    fn peers_note(&self, shared: &AppShared, row: Row) -> Option<String> {
        let Row::Skin(row) = row else {
            return None;
        };
        shared_row_note(shared.skins.peers_for(&shared.config, self.target().screen, row).len())
    }

    /// What is said of the TARGET row beside what the settings table says of it: what is missing when
    /// it names a score the chart may not have, and what the run is paced against then. `None` for
    /// any other row and for a target that is always known.
    fn target_note(&self, shared: &AppShared, row: Row) -> Option<String> {
        let Row::Setting(SettingId::Target) = row else {
            return None;
        };
        shared.config.judge.target.missing_note(shared.config.judge.target_fallback, shared.config.judge.target_rate_percent)
    }

    /// What is said of a row beside what the settings table says of it: what the TARGET row stands
    /// on ([`PlayMenuState::target_note`]), that the TARGET RATE row is not in use while no target
    /// is RATE CUSTOM, and what SCRATCH AUTO costs a run. `None` for every other row.
    fn row_note(&self, shared: &AppShared, row: Row) -> Option<String> {
        let judge = &shared.config.judge;
        match row {
            Row::Setting(SettingId::Target) => self.target_note(shared, row),
            Row::Setting(SettingId::TargetRate) if ![judge.target, judge.target_fallback].contains(&ScoreTarget::RateCustom) => {
                Some(TARGET_RATE_UNUSED_NOTE.to_string())
            }
            Row::Setting(SettingId::ScratchAuto) => Some(SCRATCH_AUTO_NOTE.to_string()),
            _ => None,
        }
    }

    /// One entry of the list as it is written, in full.
    fn line(&self, shared: &AppShared, readout: &SpeedReadout, row: Row) -> Line {
        let look = if is_read_only(&shared.config.play, row) { RowLook::ReadOnly } else { RowLook::Plain };
        match row {
            Row::Group(name) => Line { label: name.to_string(), value: String::new(), look: RowLook::Heading },
            Row::PreviewBpm => Line { label: PREVIEW_BPM_LABEL.to_string(), value: bpm_value(self.target().bpm), look },
            Row::Setting(id) => Line { label: shared.setting_line(id).0.to_string(), value: setting_value(shared, readout, id), look },
            Row::Skin(option) => {
                let (label, value) = shared.skins.line_for(&shared.config, self.target().screen, option);
                Line { label, value, look }
            }
            Row::Fold(fold) => {
                let held = match fold {
                    Fold::AutoChannels => ColorChannel::ALL.len(),
                    _ => folded(shared, self.target().screen, fold).len(),
                };
                Line { label: fold.label().to_string(), value: fold_value(self.folds.is_open(fold), held), look }
            }
            Row::AutoColor => Line { label: SCRATCH_AUTO_COLOR_LABEL.to_string(), value: color_value(shared.config.play.scratch_auto_color), look },
            Row::AutoChannel(channel) => Line {
                label: format!("{SCRATCH_AUTO_COLOR_LABEL} {}", channel.label()),
                value: number_of(shared, readout, self.target().screen, row).map_or_else(String::new, |value| number_figure(row, &value)),
                look,
            },
        }
    }

    /// What is said of one row under the preview. An offset the game adds to every play skin is
    /// said as the skin menu says one.
    fn help_of(&self, shared: &AppShared, row: Row) -> &'static str {
        let screen = self.target().screen;
        let kind = match row {
            Row::Skin(skin_row) => shared.skins.kind_for(&shared.config, screen, skin_row),
            _ => None,
        };
        match (row, kind) {
            (Row::Skin(skin_row), Some(SkinRowKind::Offset(axis))) if shared.skins.is_automatic_for(&shared.config, screen, skin_row) => {
                automatic_offset_help(axis)
            }
            _ => help(&shared.config.play, row, kind),
        }
    }

    /// Keys on the list, once the modifier has had its say.
    fn row_key(&mut self, ctx: &mut FrameCtx<'_>, code: KeyCode) -> Transition {
        match code {
            KeyCode::Escape => {
                ctx.shared.save_settings();
                return Transition::Back;
            }
            KeyCode::Tab => {
                self.step_mode(if self.shift_held { PREVIOUS_TAB } else { NEXT_TAB });
                self.ensure_rows(ctx.shared);
                self.reveal_cursor();
            }
            KeyCode::ArrowUp => self.focus(self.sel.saturating_sub(1), ROW_STEP_BACK),
            KeyCode::ArrowDown => self.focus(self.sel + 1, ROW_STEP_FORWARD),
            KeyCode::PageUp => self.focus(self.sel.saturating_sub(VISIBLE_ROWS), ROW_STEP_BACK),
            KeyCode::PageDown => self.focus(self.sel + VISIBLE_ROWS, ROW_STEP_FORWARD),
            KeyCode::ArrowLeft => self.step_focused(ctx, ROW_STEP_BACK, self.shift_held),
            KeyCode::ArrowRight => self.step_focused(ctx, ROW_STEP_FORWARD, self.shift_held),
            KeyCode::Enter | KeyCode::NumpadEnter => self.enter_focused(ctx),
            KeyCode::Digit0 | KeyCode::Numpad0 => self.reset_focused(ctx),
            code if code == APPLY_TO_ALL_KEY => self.apply_focused_to_all(ctx),
            _ => {}
        }
        Transition::Stay
    }

    /// Draw the entries on show ([`draw_list`]), each value cut to the room a value has. A row that
    /// holds a number is drawn with its slider and reads as its number alone ([`number_figure`]),
    /// or with the figure being typed into it, and a colour row with a square of its colour or the
    /// colour being typed into it. `now` is what a row that refused a typed figure stops saying so
    /// against.
    fn draw_rows(&self, shared: &mut AppShared, canvas: &mut Canvas<'_>, readout: &SpeedReadout, now: std::time::Instant) {
        let list = ListWindow { len: self.rows.len(), sel: self.sel, top: self.top };
        let shown = self.top..(self.top + VISIBLE_ROWS).min(self.rows.len());
        let screen = self.target().screen;
        let color_of = |row: &Row| match row {
            Row::Skin(SkinRow::Color(at)) => shared.skins.color_for(&shared.config, screen, *at).map(|(color, _)| Color::from(color)),
            Row::AutoColor => Some(Color::from(scratch_auto_color(&shared.config.play))),
            _ => None,
        };
        let lines: Vec<(Line, Option<NumberValue>, Option<Color>)> =
            self.rows[shown].iter().map(|row| (self.line(shared, readout, *row), number_of(shared, readout, screen, *row), color_of(row))).collect();
        let entry = |index: usize| {
            let (Line { label, value, look }, number, color) = &lines[index - self.top];
            let row = self.rows[index];
            let Some(number) = number else {
                let color = color.map(|color| ColorCell { color, edit: self.numbers.edit_of(row), refused: self.numbers.is_refused(row, now) });
                return ListEntry { label, value: fitted_value(value), look: *look, number: None, color };
            };
            let cell = NumberCell {
                fraction: number.fraction(),
                dragged: self.numbers.is_dragging(row),
                edit: self.numbers.edit_of(row),
                refused: self.numbers.is_refused(row, now),
            };
            ListEntry { label, value: number_figure(row, number), look: *look, number: Some(cell), color: None }
        };
        draw_list(shared, canvas, list, entry, Hot::PlayMenuRow);
    }

    /// Draw what is said under the preview: what the scroll comes to at the target on show, as a
    /// row of figures, and the row under the cursor with its label and value in full, what the
    /// row is for, and for a row of the skin that other screens declare too, that they do.
    fn draw_info(&self, shared: &AppShared, canvas: &mut Canvas<'_>, readout: &SpeedReadout) {
        draw_info_panel(canvas);
        draw_info_figures(canvas, &summary(readout, shared.config.play.constant_speed), summary_note(readout));
        if let Some(row) = self.focused() {
            let line = self.line(shared, readout, row);
            let note = self.peers_note(shared, row).or_else(|| self.row_note(shared, row));
            draw_info_row_noted(canvas, INFO_ROW_UNDER_FIGURES_Y, &line.label, &line.value, self.help_of(shared, row), note.as_deref());
        }
    }
}

/// The target a mode and a tempo of the two lists make.
fn target_at(mode: usize, bpm: usize) -> PlayTarget {
    PlayTarget { screen: PREVIEW_MODES[mode.min(PREVIEW_MODES.len() - 1)].0, bpm: PREVIEW_BPMS[bpm.min(PREVIEW_BPMS.len() - 1)] }
}

/// What the scroll comes to, as the figures written under the preview, left to right: the tempo it
/// is worked out at, the scroll speed, how long a note is on show, the green number and the white
/// number.
///
/// What follows a figure says how to read it. The tempo is the field's own rather than the sample
/// chart's while CONSTANT is on. The speed is worked out from a held green number, and stands at an
/// end of its range when no speed holds that number. The white number is not in force while the
/// lane cover is switched off. The green number is written in the green a skin writes it in.
fn summary(readout: &SpeedReadout, constant: bool) -> [InfoFigure; 5] {
    let colors = theme();
    let figure = |name, value: String, tag| InfoFigure { name, value, color: colors.text, tag, tag_color: colors.accent };
    let speed_tag = match (readout.held, readout.limited) {
        (_, true) => LIMIT_MARK,
        (true, false) => HELD_MARK,
        (false, false) => "",
    };
    [
        figure(FIGURE_BPM, (readout.bpm.round() as i64).to_string(), if constant { CONSTANT_MARK } else { "" }),
        InfoFigure {
            tag_color: if readout.limited { colors.warn } else { colors.accent },
            ..figure(FIGURE_HISPEED, hispeed_figure(readout.hispeed), speed_tag)
        },
        InfoFigure { tag_color: colors.text_muted, ..figure(FIGURE_TRAVEL, readout.duration_ms.to_string(), TRAVEL_UNIT) },
        InfoFigure { color: Color::GREEN, ..figure(FIGURE_GREEN, readout.green_number.to_string(), "") },
        figure(FIGURE_WHITE, readout.white_number.to_string(), if readout.cover_on { "" } else { COVER_OFF_MARK }),
    ]
}

/// What is said under the figures: that the scroll speed stands at an end of its range, when it
/// does, so the green number on show is not the one asked for.
fn summary_note(readout: &SpeedReadout) -> Option<&'static str> {
    readout.limited.then_some(LIMIT_NOTE)
}

impl StageHandler for PlayMenuState {
    /// Walk the skin folder on the way in, so the options listed are the ones the play skin
    /// declares now, and build the list.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        ctx.shared.rescan_skins();
        self.refresh(ctx.shared);
        self.reveal_cursor();
    }

    /// Bring the preview up to this frame.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.preview.update(ctx);
        Transition::Stay
    }

    /// Escape writes the settings out and goes back to the screen the menu was opened over. A key
    /// pressed while a slider is being dragged ends the drag first: the key may move the cursor or
    /// the mode on, and the pointer must not go on carrying whatever row is then in that place.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if let Some(held) = shift_after(self.shift_held, &key) {
            self.shift_held = held;
            return Transition::Stay;
        }
        if !key.pressed {
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
    /// previews that mode. A press anywhere else -- the preview and the name of a group included --
    /// does nothing, but any press stores a number that was being typed.
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
        self.ensure_rows(ctx.shared);
        self.end_drag(ctx);
        self.commit_edit(ctx);
        match ctx.shared.hit_test(at) {
            Some(hot @ Hot::PlayMenuRow(index)) => {
                if let Some(row) = self.rows.get(index).copied().filter(|row| !row.is_heading()) {
                    self.sel = index;
                    self.press_row(ctx, row, hot, at, button, delta);
                }
            }
            Some(Hot::PlayMenuMode(index)) if is_left_press(button, pressed) && index < PREVIEW_MODES.len() => self.retarget(index, self.bpm),
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

    /// The menu's own half first, then the preview into its rectangle with the status in its place
    /// when no frame of a skin is there.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        self.ensure_rows(ctx.shared);
        let readout = self.target().readout(&ctx.shared.config.play);
        canvas.clear_bga();
        canvas.clear(theme().bg);
        draw_head(canvas, TITLE, &self.subtitle());
        draw_tabs(ctx.shared, canvas, PREVIEW_MODES.iter().map(|(_, name)| *name), Some(self.mode), Hot::PlayMenuMode);
        self.draw_rows(ctx.shared, canvas, &readout, ctx.now);
        self.draw_info(ctx.shared, canvas, &readout);
        draw_preview(&mut self.preview, ctx, canvas);
        draw_hint(canvas, self.numbers.edit_hint().unwrap_or(HINT));
    }

    /// The browser's lines, and what the preview is of and last left in its rectangle.
    fn debug_lines(&self, ctx: &FrameCtx<'_>) -> Vec<String> {
        let mut lines = ctx.shared.browser_debug_lines();
        lines.push(format!("PLAY PREVIEW {:?} {:?}", self.preview.target(), self.preview.status()));
        lines
    }
}

#[cfg(test)]
mod tests;
