//! The key-config editor: every key and every controller element the application reads, rebound one
//! cell at a time. The mode whose lanes are being edited is remembered between visits, so it lives
//! in [`AppShared`].
//!
//! The editor is a table ([`table`]). Each line is one thing that can be bound -- a lane, a control
//! of a run, a shortcut of the browser, a key to a screen -- under the heading of the screen that
//! reads it, with a column for the keyboard and one for the controller. The cursor is a line and a
//! column: Enter binds the cell under it to whatever is pressed next, Delete takes its binding away
//! and Backspace puts back the one it ships with. A key that something read at the same moment
//! already has is refused and the line says what has it ([`crate::keyconfig::Bindable::read_with`]);
//! a controller element is taken as it is and flagged, as it always was.
//!
//! A lane takes the first key pressed. A control or a shortcut takes a key pressed and let go, or
//! two keys held together: the first held while the second goes down
//! ([`crate::keyconfig::ChordCapture`]). The controller's cell takes elements the same way.
//!
//! The editor's own keys are fixed: they are what binds every other.
#![allow(clippy::wildcard_imports)]

#[cfg(test)]
mod chord_tests;
mod table;
#[cfg(test)]
mod tests;

use crate::gamepad::{AnalogMode, PadBinding};
use crate::keyconfig::{Bindable, Captured, ChordCapture, Clash, KeyBind, KeyConfig};
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::text::menus::key_editor::{KeyEditorNoticeText, KeyEditorText};
use crate::*;
use rbms_model::translations::Catalog;
use rbms_render::fit_text;
use rbms_render::text::language;
use table::{Bindings, Line, lines};

/// The editor's panel: how wide it is, and where its columns end. A value is set against the right
/// edge of its column.
const PANEL_W: f32 = 1080.0;
const KEYBOARD_RIGHT: f32 = 760.0;
const COLUMN_W: f32 = 300.0;
const CELL_PAD: f32 = 16.0;

/// Where the title, the two hint lines and the column headings stand.
const TITLE_Y: f32 = 24.0;
const TITLE_SCALE: f32 = 2.6;
const HINT_Y: f32 = 66.0;
const STATUS_Y: f32 = 88.0;
const HINT_SCALE: f32 = 1.1;
const COLUMNS_Y: f32 = 116.0;
const COLUMNS_SCALE: f32 = 1.1;

/// Vertical layout of the lines.
const ROW_TOP: f32 = 138.0;
const ROW_H: f32 = 24.0;
const ROW_GAP: f32 = 2.0;
const ROW_TEXT_Y: f32 = 6.0;
const ROW_SCALE: f32 = 1.4;
const HEADING_SCALE: f32 = 1.2;
const HEADING_TEXT_Y: f32 = 8.0;
const RULE_THICKNESS: f32 = 2.0;

/// How many lines fit on screen at once, and where the count of them stands.
const VISIBLE_ROWS: usize = 21;
const COUNTER_Y: f32 = 690.0;
const COUNTER_SCALE: f32 = 1.1;

/// What an unbound cell shows.
const UNBOUND: &str = "-";

/// What the restore row shows until Enter is pressed: the name of the key, which every language writes alike.
const RESTORE_VALUE: &str = "ENTER";

/// One step forwards and one back, for the rows that are stepped rather than bound.
const STEP_FORWARD: i32 = 1;
const STEP_BACK: i32 = -1;

/// Which of a line's two cells the cursor is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Column {
    #[default]
    Keyboard,
    Pad,
}

/// What the editor has to say about the last thing that was asked of it, shown until the next key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Notice {
    #[default]
    None,
    /// The key was refused: something read at the same moment has it.
    Taken(Bindable),
    /// The two keys were refused: something read at the same moment has the same two.
    ChordTaken(Bindable),
    /// The key was refused: the screen that reads the row keeps it for itself.
    Fixed,
    /// The key was refused: the key config file has no word for it.
    Unnamed,
    /// The row is the keyboard's alone.
    KeyboardOnly,
    /// A lane was cleared, which puts the key it ships with back.
    LaneKept,
    /// Enter once more puts everything back as shipped.
    RestoreArmed,
    Restored,
}

/// The editor's own state: the focused cell, whether the next key or controller element is being
/// captured for it, and what there is to say about the last request.
#[derive(Default)]
pub(crate) struct KeyConfigState {
    sel: usize,
    column: Column,
    capturing: bool,
    notice: Notice,
    /// What a capture of keys, and one of controller elements, holds waiting for a second.
    keys: ChordCapture<KeyCode>,
    elements: ChordCapture<PadBinding>,
}

impl KeyConfigState {
    pub(crate) fn new() -> KeyConfigState {
        KeyConfigState::default()
    }

    /// The line the cursor is on.
    fn focused(&self, shared: &AppShared) -> Option<Line> {
        lines(shared.kc_edit_mode).get(self.sel).copied()
    }

    /// The thing the cursor is on, when it is on one that can be bound.
    fn focused_target(&self, shared: &AppShared) -> Option<Bindable> {
        match self.focused(shared) {
            Some(Line::Bind { target, .. }) => Some(target),
            _ => None,
        }
    }

    /// Move the cursor `delta` lines, passing over the headings and stopping at either end.
    fn move_cursor(&mut self, shared: &AppShared, delta: i32) {
        let all = lines(shared.kc_edit_mode);
        let mut at = self.sel as i32;
        loop {
            at += delta;
            match usize::try_from(at).ok().and_then(|index| all.get(index).map(|line| (index, *line))) {
                Some((index, line)) if line.takes_the_cursor() => {
                    self.sel = index;
                    return;
                }
                Some(_) => {}
                None => return,
            }
        }
    }

    /// Move the cursor to the first line of the group after the one it is in (`forward`), or of the
    /// group before it.
    fn jump_group(&mut self, shared: &AppShared, forward: bool) {
        let all = lines(shared.kc_edit_mode);
        let headings: Vec<usize> = all.iter().enumerate().filter(|(_, line)| matches!(line, Line::Heading(_))).map(|(index, _)| index).collect();
        let own = headings.iter().rposition(|heading| *heading < self.sel);
        let to = match (forward, own) {
            (true, None) => headings.first().copied(),
            (true, Some(at)) => headings.get(at + 1).copied(),
            (false, Some(at)) if at > 0 => Some(headings[at - 1]),
            (false, _) => None,
        };
        match to {
            Some(heading) if heading + 1 < all.len() => self.sel = heading + 1,
            Some(_) => {}
            None if !forward => self.sel = 0,
            None => {}
        }
    }

    fn cycle_edit_mode(&mut self, shared: &mut AppShared, d: i32) {
        let all = Mode::ALL;
        let cur = all.iter().position(|m| m.key == shared.kc_edit_mode.key).unwrap_or(0);
        let focused = self.focused(shared);
        shared.kc_edit_mode = all[((cur as i32 + d).rem_euclid(all.len() as i32)) as usize];
        let rows = lines(shared.kc_edit_mode);
        self.sel = focused.and_then(|line| rows.iter().position(|row| *row == line)).unwrap_or_else(|| self.sel.min(rows.len() - 1));
        if !rows[self.sel].takes_the_cursor() {
            self.move_cursor(shared, STEP_BACK);
        }
    }

    /// Arm a capture of the focused cell, with nothing held waiting for a second.
    fn arm(&mut self) {
        self.capturing = true;
        self.keys = ChordCapture::default();
        self.elements = ChordCapture::default();
    }

    /// One key going down or up while the keyboard cell waits for its binding. Escape going down
    /// calls the capture off. A lane takes the first key that goes down; a control or a shortcut
    /// takes a key once it is let go, or two once the second goes down while the first is held.
    fn capture_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>) {
        let Some(target) = self.focused_target(shared) else {
            self.capturing = false;
            return;
        };
        if key.pressed && key.code == KeyCode::Escape {
            self.capturing = false;
            return;
        }
        let captured = match (target.takes_a_chord(), key.pressed, key.released) {
            (false, true, _) => Some(Captured::Single(key.code)),
            (true, true, _) => self.keys.press(key.code),
            (true, false, true) => self.keys.release(key.code),
            _ => None,
        };
        let bind = match captured {
            Some(Captured::Single(code)) => Some(KeyBind::Single(code)),
            Some(Captured::Chord(first, second)) => KeyBind::chord(first, second),
            None => None,
        };
        if let Some(bind) = bind {
            self.capturing = false;
            self.give_keys(shared, target, bind);
        }
    }

    /// Bind the captured key or keys to `target` unless something read at the same moment has
    /// them, the row's screen keeps one of them, or the key config file has no word for one.
    fn give_keys(&mut self, shared: &mut AppShared, target: Bindable, bind: KeyBind) {
        let mode = shared.kc_edit_mode;
        let chord = matches!(bind, KeyBind::Chord(..));
        self.notice = if !bind.is_named() {
            Notice::Unnamed
        } else {
            match shared.keyconfig.clash_of(mode, target, bind) {
                Some(Clash::With(other)) if chord => Notice::ChordTaken(other),
                Some(Clash::With(other)) => Notice::Taken(other),
                Some(Clash::Fixed) => Notice::Fixed,
                None => {
                    shared.keyconfig.bind_keys(mode, target, bind);
                    Notice::None
                }
            }
        };
    }

    /// Take the binding of the focused cell away. A lane is never without a key, so clearing one
    /// puts the key it ships with back, and the editor says so.
    fn clear(&mut self, shared: &mut AppShared) {
        let Some(target) = self.focused_target(shared) else {
            return;
        };
        let mode = shared.kc_edit_mode;
        match self.column {
            Column::Keyboard => {
                shared.keyconfig.bind(mode, target, None);
                if matches!(target, Bindable::Lane(_)) {
                    self.notice = Notice::LaneKept;
                }
            }
            Column::Pad => shared.keyconfig.pad.bind(mode, target, None),
        }
    }

    /// Put the focused cell back on what it ships with: its key, or on the controller nothing but
    /// the pad's own menu buttons for START and SELECT.
    fn restore(&mut self, shared: &mut AppShared) {
        let Some(target) = self.focused_target(shared) else {
            return;
        };
        let mode = shared.kc_edit_mode;
        match self.column {
            Column::Keyboard => shared.keyconfig.restore(mode, target),
            Column::Pad => shared.keyconfig.pad.bind(mode, target, None),
        }
    }

    /// Enter on the focused line: arm a capture, step a row that is stepped, or ask for and then
    /// carry out the restoring of everything.
    fn enter(&mut self, shared: &mut AppShared) {
        match self.focused(shared) {
            Some(Line::Bind { pad: false, .. }) if self.column == Column::Pad => self.notice = Notice::KeyboardOnly,
            Some(Line::Bind { .. }) => {
                if self.column == Column::Pad {
                    let AppShared { pad, keyconfig, .. } = shared;
                    if let Some(state) = pad.as_mut() {
                        state.drain(&keyconfig.pad);
                    }
                }
                self.arm();
            }
            Some(Line::Mode) => self.cycle_edit_mode(shared, STEP_FORWARD),
            Some(Line::PadDevice) => self.cycle_pad_device(shared, STEP_FORWARD),
            Some(Line::PadAnalog) => self.cycle_analog_mode(shared, STEP_FORWARD),
            Some(Line::RestoreAll) => {}
            Some(Line::Heading(_)) | None => {}
        }
    }

    /// Left or right on the focused line: step a row that is stepped, or move between the keyboard's
    /// cell and the controller's.
    fn sideways(&mut self, shared: &mut AppShared, delta: i32) {
        match self.focused(shared) {
            Some(Line::Mode) => self.cycle_edit_mode(shared, delta),
            Some(Line::PadDevice) => self.cycle_pad_device(shared, delta),
            Some(Line::PadAnalog) => self.cycle_analog_mode(shared, delta),
            Some(Line::Bind { .. }) => self.column = if delta > 0 { Column::Pad } else { Column::Keyboard },
            Some(Line::RestoreAll | Line::Heading(_)) | None => {}
        }
    }

    /// Step the controller the pad column binds against: the first entry is ANY, then every device
    /// gilrs has opened.
    fn cycle_pad_device(&self, shared: &mut AppShared, delta: i32) {
        let mut names: Vec<Option<String>> = vec![None];
        names.extend(shared.pad.as_ref().map(PadState::device_names).unwrap_or_default().into_iter().map(Some));
        let at = names.iter().position(|name| *name == shared.keyconfig.pad.device_name).unwrap_or(0);
        let next = (at as i32 + delta).rem_euclid(names.len() as i32) as usize;
        shared.keyconfig.pad.device_name = names[next].clone();
    }

    /// Step the turntable algorithm the analog scratch rows are read with.
    fn cycle_analog_mode(&self, shared: &mut AppShared, delta: i32) {
        let all = AnalogMode::ALL;
        let at = all.iter().position(|mode| *mode == shared.keyconfig.pad.analog_mode).unwrap_or(0);
        let next = (at as i32 + delta).rem_euclid(all.len() as i32) as usize;
        shared.keyconfig.pad.analog_mode = all[next];
    }

    /// Bind what the controller reports next to the focused cell. Returns once a binding has
    /// landed; until then the capture stays armed and the next frame asks again.
    fn capture_pad(&mut self, ctx: &mut FrameCtx<'_>) {
        let Some(target) = self.focused_target(ctx.shared).filter(|_| self.column == Column::Pad) else {
            return;
        };
        let AppShared { pad, keyconfig, .. } = &mut *ctx.shared;
        let Some(state) = pad.as_mut() else {
            return;
        };
        let changes = state.take_capture(&keyconfig.pad);
        self.capture_elements(ctx.shared, target, &changes);
    }

    /// Elements of the controller going down (`true`) or up while its cell waits for a binding,
    /// oldest first. A lane takes the first that goes down, a turntable for a scratch lane while an
    /// algorithm reads one; a control or a shortcut takes an element once it is let go, or two once
    /// the second goes down while the first is held.
    fn capture_elements(&mut self, shared: &mut AppShared, target: Bindable, changes: &[(PadBinding, bool)]) {
        let mode = shared.kc_edit_mode;
        for &(binding, pressed) in changes {
            let captured = match (target.takes_a_chord(), pressed) {
                (false, true) => Some(Captured::Single(binding)),
                (false, false) => None,
                (true, true) => self.elements.press(binding),
                (true, false) => self.elements.release(binding),
            };
            let pad = &mut shared.keyconfig.pad;
            match captured {
                None => continue,
                Some(Captured::Single(binding)) => {
                    let binding = match target {
                        Bindable::Lane(lane) if mode.is_scratch(lane) && pad.analog_mode != AnalogMode::Off => binding.as_analog_scratch().unwrap_or(binding),
                        _ => binding,
                    };
                    pad.bind(mode, target, Some(binding));
                }
                Some(Captured::Chord(first, second)) => pad.bind_chord(target, first, second),
            }
            self.capturing = false;
            return;
        }
    }

    /// The first line drawn, which keeps the cursor in the middle of what is on screen.
    fn first_visible(&self, count: usize) -> usize {
        self.sel.saturating_sub(VISIBLE_ROWS / 2).min(count.saturating_sub(VISIBLE_ROWS.min(count)))
    }

    /// What the second hint line says: what was just refused or done, or what the editor knows about
    /// the cell under the cursor -- what its binding clashes with, or the lane it shares a key with.
    fn status(&self, shared: &AppShared, bound: &Bindings) -> (String, Color) {
        let mode = shared.kc_edit_mode;
        let theme = rbms_render::theme();
        let in_red = |text: String| (text, Color::RED);
        match self.notice {
            Notice::Taken(other) => return in_red(KeyEditorNoticeText::Taken.fill(language(), &[&other.label(mode)])),
            Notice::ChordTaken(other) => return in_red(KeyEditorNoticeText::ChordTaken.fill(language(), &[&other.label(mode)])),
            Notice::Fixed => return in_red(KeyEditorNoticeText::Fixed.get(language()).to_string()),
            Notice::Unnamed => return in_red(KeyEditorNoticeText::Unnamed.get(language()).to_string()),
            Notice::KeyboardOnly => return in_red(KeyEditorNoticeText::KeyboardOnly.get(language()).to_string()),
            Notice::LaneKept => return (KeyEditorNoticeText::LaneKept.get(language()).to_string(), theme.warn),
            Notice::RestoreArmed => return (KeyEditorNoticeText::RestoreArmed.get(language()).to_string(), theme.warn),
            Notice::Restored => return (KeyEditorNoticeText::Restored.get(language()).to_string(), theme.text_dim),
            Notice::None => {}
        }
        let Some(target) = self.focused_target(shared) else {
            return (String::new(), theme.text_dim);
        };
        let clash = match self.column {
            Column::Keyboard => bound.key_clash(target),
            Column::Pad => bound.pad_clash(target).map(Clash::With),
        };
        match clash {
            Some(Clash::With(other)) => in_red(KeyEditorNoticeText::SharedWith.fill(language(), &[&other.label(mode)])),
            Some(Clash::Fixed) => in_red(KeyEditorNoticeText::FixedOnItsScreen.get(language()).to_string()),
            None => match (self.overlap(shared, bound, target), shared.keyconfig.lane_sharing(mode, target).filter(|_| self.column == Column::Keyboard)) {
                (Some(said), _) => (said, theme.warn),
                (None, Some(lane)) => (KeyEditorNoticeText::AlsoLane.fill(language(), &[&Bindable::Lane(lane).label(mode)]), theme.text_dim),
                (None, None) => (String::new(), theme.text_dim),
            },
        }
    }

    /// What the editor says of the cell under the cursor sharing a key with something read at the
    /// same moment, one of the two alone and the other held with a second: pressed first, the key
    /// does what it does alone.
    fn overlap(&self, shared: &AppShared, bound: &Bindings, target: Bindable) -> Option<String> {
        let mode = shared.kc_edit_mode;
        let (shared_key, other, mine_is_chord) = match self.column {
            Column::Keyboard => {
                let (code, other) = shared.keyconfig.chord_overlap(mode, target)?;
                (key_name(code).to_string(), other, matches!(bound.key_of(target), Some(KeyBind::Chord(..))))
            }
            Column::Pad => {
                let (element, other) = bound.pad_overlap(target)?;
                (element.label(), other, bound.pad_chord_of(target).is_some())
            }
        };
        let words = if mine_is_chord { KeyEditorNoticeText::ChordKeyAlone } else { KeyEditorNoticeText::KeyInChord };
        Some(words.fill(language(), &[&shared_key, &other.label(mode)]))
    }
}

/// A key or two keys as the editor draws them: each by its word in the key config file, or a dash
/// for none.
fn key_token(bind: Option<KeyBind>) -> String {
    bind.map_or_else(|| UNBOUND.to_string(), KeyBind::label)
}

/// A pad binding as the editor draws it, with the element held together with it: a dash for none.
fn pad_token(binding: Option<PadBinding>, second: Option<PadBinding>) -> String {
    binding.map_or_else(|| UNBOUND.to_string(), |first| PadBinding::chord_label(first, second))
}

impl StageHandler for KeyConfigState {
    /// A controller cell's capture arrives on the frame poll rather than as a window event, so it is
    /// collected here instead of in `handle_key`.
    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        if self.capturing {
            self.capture_pad(ctx);
        }
        Transition::Stay
    }

    /// The editor's own keys are fixed, and a key pressed to be bound is that key.
    fn reads_keys_as_they_are(&self) -> bool {
        true
    }

    /// The controller's ESC leaves the editor as Escape does, but for while the controller's cell
    /// waits for an element: the element pressed is the binding.
    fn takes_pad_escape(&self) -> bool {
        !(self.capturing && self.column == Column::Pad)
    }

    /// Key-config editor input. In capture mode the next key (except Esc), or the next two held
    /// together, are bound to the focused cell unless refused ([`KeyConfigState::capture_key`]);
    /// otherwise the editor's own keys move the cursor, switch the mode being edited, start a
    /// rebind, clear or restore a cell, or save and go back.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if self.capturing && self.column == Column::Keyboard {
            if key.pressed {
                self.notice = Notice::None;
            }
            self.capture_key(ctx.shared, &key);
            return Transition::Stay;
        }
        if !key.pressed {
            return Transition::Stay;
        }
        let armed = std::mem::take(&mut self.notice) == Notice::RestoreArmed;
        if self.capturing {
            if key.code == KeyCode::Escape {
                self.capturing = false;
            }
            return Transition::Stay;
        }
        match key.code {
            KeyCode::Escape => {
                ctx.shared.keyconfig.save(&ctx.shared.keyconfig_path);
                return Transition::Back;
            }
            KeyCode::ArrowUp => self.move_cursor(ctx.shared, STEP_BACK),
            KeyCode::ArrowDown => self.move_cursor(ctx.shared, STEP_FORWARD),
            KeyCode::PageUp => self.jump_group(ctx.shared, false),
            KeyCode::PageDown => self.jump_group(ctx.shared, true),
            KeyCode::ArrowLeft => self.sideways(ctx.shared, STEP_BACK),
            KeyCode::ArrowRight => self.sideways(ctx.shared, STEP_FORWARD),
            KeyCode::Tab => self.cycle_edit_mode(ctx.shared, STEP_FORWARD),
            KeyCode::Delete => self.clear(ctx.shared),
            KeyCode::Backspace => self.restore(ctx.shared),
            KeyCode::Enter | KeyCode::NumpadEnter if self.focused(ctx.shared) == Some(Line::RestoreAll) => {
                if armed {
                    ctx.shared.keyconfig.restore_everything();
                    self.notice = Notice::Restored;
                } else {
                    self.notice = Notice::RestoreArmed;
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => self.enter(ctx.shared),
            _ => {}
        }
        Transition::Stay
    }

    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        ctx.shared.prepare_skin(canvas, SKIN_TYPE_KEY_CONFIG);
        let edit_mode = ctx.shared.kc_edit_mode;
        let lane_tokens: Vec<String> = (0..edit_mode.key).map(|lane| ctx.shared.keyconfig.lane_token(edit_mode, lane)).collect();
        if ctx.shared.draw_keyconfig_skin(canvas, &lane_tokens) {
            return;
        }
        canvas.clear(th.bg);
        let keys: &KeyConfig = &ctx.shared.keyconfig;
        let bound = Bindings::read(keys, edit_mode);
        let x0 = (CW as f32 - PANEL_W) * 0.5;
        let keyboard_right = x0 + KEYBOARD_RIGHT;
        let pad_right = x0 + PANEL_W;
        draw_text(canvas, x0, TITLE_Y, TITLE_SCALE, th.text, KeyEditorText::Title.get(language()));
        draw_text_right(canvas, pad_right, TITLE_Y + HEADING_TEXT_Y, ROW_SCALE, th.accent, mode_short(edit_mode));
        let chord = self.focused_target(ctx.shared).is_some_and(Bindable::takes_a_chord);
        let (hint, hint_col) = match (self.capturing, self.column, chord) {
            (true, Column::Keyboard, false) => (KeyEditorText::CaptureKeyHint, th.accent),
            (true, Column::Keyboard, true) => (KeyEditorText::CaptureChordKeyHint, th.accent),
            (true, Column::Pad, false) => (KeyEditorText::CapturePadHint, th.accent),
            (true, Column::Pad, true) => (KeyEditorText::CaptureChordPadHint, th.accent),
            (false, ..) => (KeyEditorText::KeysHint, th.text_muted),
        };
        let hint = hint.get(language());
        draw_text(canvas, x0, HINT_Y, HINT_SCALE, hint_col, &fit_text(hint, HINT_SCALE, PANEL_W));
        let (status, status_col) = self.status(ctx.shared, &bound);
        draw_text(canvas, x0, STATUS_Y, HINT_SCALE, status_col, &fit_text(&status, HINT_SCALE, PANEL_W));
        draw_text(canvas, x0 + CELL_PAD, COLUMNS_Y, COLUMNS_SCALE, th.text_muted, KeyEditorText::ActionColumn.get(language()));
        draw_text_right(canvas, keyboard_right - CELL_PAD, COLUMNS_Y, COLUMNS_SCALE, th.text_muted, KeyEditorText::KeyboardColumn.get(language()));
        draw_text_right(canvas, pad_right - CELL_PAD, COLUMNS_Y, COLUMNS_SCALE, th.text_muted, KeyEditorText::Controller.get(language()));

        let rows = lines(edit_mode);
        let start = self.first_visible(rows.len());
        for (slot, index) in (start..(start + VISIBLE_ROWS).min(rows.len())).enumerate() {
            let y = ROW_TOP + slot as f32 * (ROW_H + ROW_GAP);
            let on = index == self.sel;
            let row_rect = Rect::new(x0, y, PANEL_W, ROW_H);
            let label_col = if on { th.text } else { th.text_dim };
            let value_col = if on { th.accent } else { th.text };
            let label_and_value = |canvas: &mut Canvas<'_>, label: &str, value: &str, right: f32| {
                canvas.fill_rect(row_rect, if on { th.button } else { th.panel });
                draw_text(canvas, x0 + CELL_PAD, y + ROW_TEXT_Y, ROW_SCALE, label_col, label);
                draw_text_right(canvas, right - CELL_PAD, y + ROW_TEXT_Y, ROW_SCALE, value_col, value);
            };
            match rows[index] {
                Line::Heading(title) => {
                    draw_text(canvas, x0, y + HEADING_TEXT_Y, HEADING_SCALE, th.accent, title);
                    canvas.fill_rect(Rect::new(x0, y + ROW_H - RULE_THICKNESS, PANEL_W, RULE_THICKNESS), th.divider);
                }
                Line::Mode => label_and_value(canvas, KeyEditorText::ModeLabel.get(language()), &format!("< {} >", mode_short(edit_mode)), keyboard_right),
                Line::PadDevice => label_and_value(
                    canvas,
                    KeyEditorText::PadDeviceLabel.get(language()),
                    &format!("< {} >", keys.pad.device_name.as_deref().unwrap_or(KeyEditorText::AnyDevice.get(language()))),
                    pad_right,
                ),
                Line::PadAnalog => {
                    label_and_value(canvas, KeyEditorText::PadAnalogLabel.get(language()), &format!("< {} >", keys.pad.analog_mode.label()), pad_right)
                }
                Line::RestoreAll => {
                    let value = if self.notice == Notice::RestoreArmed { KeyEditorText::RestoreArmedValue.get(language()) } else { RESTORE_VALUE };
                    label_and_value(canvas, KeyEditorText::RestoreLabel.get(language()), value, keyboard_right);
                }
                Line::Bind { target, pad: takes_a_pad } => {
                    canvas.fill_rect(row_rect, if on { th.panel_hi } else { th.panel });
                    draw_text(canvas, x0 + CELL_PAD, y + ROW_TEXT_Y, ROW_SCALE, label_col, &target.label(edit_mode));
                    let cells = [
                        (Column::Keyboard, keyboard_right, key_token(bound.key_of(target)), bound.key_clash(target).is_some()),
                        (
                            Column::Pad,
                            pad_right,
                            if takes_a_pad { pad_token(bound.pad_of(target), bound.pad_chord_of(target)) } else { String::new() },
                            bound.pad_clash(target).is_some(),
                        ),
                    ];
                    for (column, right, token, clashes) in cells {
                        let focused = on && column == self.column;
                        let waiting = focused && self.capturing;
                        if focused {
                            canvas.fill_rect(Rect::new(right - COLUMN_W, y, COLUMN_W, ROW_H), if waiting { th.focus } else { th.button });
                        }
                        let (text, color) = if waiting {
                            (KeyEditorText::Capturing.get(language()).to_string(), th.bg)
                        } else if clashes {
                            (token, Color::RED)
                        } else if focused {
                            (token, th.accent)
                        } else {
                            (token, th.text)
                        };
                        draw_text_right(canvas, right - CELL_PAD, y + ROW_TEXT_Y, ROW_SCALE, color, &text);
                    }
                }
            }
        }
        let counter = format!("{} / {}", self.sel + 1, rows.len());
        draw_text_right(canvas, pad_right, COUNTER_Y, COUNTER_SCALE, th.text_muted, &counter);
    }
}
