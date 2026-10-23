//! The PRACTICE panel: the range of a chart to repeat and the terms it is repeated under.
//!
//! The panel is a screen of its own rather than a mode of the browser, because it needs the parsed
//! chart — the range it edits is bounded by the chart's last timeline, and the TOTAL row starts
//! from the chart's own. So the practice key loads the chart exactly as PLAY does and lands here
//! instead, and a slice started from here returns here rather than to the result screen. It is
//! never drawn over a play screen: a slice is a screen opened over this one, and a skin's play
//! screen is never in the reference's practice state.
//!
//! Its first row starts the slice. Under it are the rows of the property ([`PracticeElement`]):
//! six hold a number and three a choice. A row that holds a number is set as such a row is on
//! every built-in screen ([`crate::number_row`]): an arrow moves it one step of the smallest change
//! it shows, and ten with Shift held; its slider is dragged; and Enter, or a press on the number,
//! opens it to be typed into -- a time as seconds or as minutes and seconds. A number that runs
//! into what another row leaves it stops there, however it was moved. A row that holds a choice is
//! stepped round its ring by an arrow or a press. Tab moves the cursor on as the down arrow does,
//! and back with Shift held, so the keys the line of a row being typed into names
//! ([`crate::number_row::EDIT_HINT`]) are this panel's too.
//!
//! Enter starts the slice from every row but one that holds a number, where it is the key that
//! opens the row to be typed into, as it is on the other screens.
//!
//! Nothing here is written anywhere while the panel is up. What the panel holds is the property of
//! this visit; it is written to the practice book when the panel is left, for a slice or for good,
//! and a slice is cut from it as it stands when the slice is started.
//!
//! Nothing on this screen is recorded or submitted: [`crate::practice::practice_block_reason`] is
//! folded into the IR gate, so a practice run updates no best and reaches no score server.
#![allow(clippy::wildcard_imports)]

use rbms_model::translations::Catalog;
use rbms_render::text::language;

use crate::number_row::{EditKey, NumberInput, RowPart, SliderLook, draw_edit, draw_refusal, draw_slider, hot_rect, row_part};
use crate::practice::{PracticeElement, PracticePanel, PracticeProperty, practice_path};
use crate::stage::preview_menu::{ROW_STEP_BACK, ROW_STEP_FORWARD, click_step, shift_after};
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::text::menus::practice::PracticeText;
use crate::*;

/// Width of the panel the rows are laid out in.
const PANEL_W: f32 = 640.0;

/// Where the first row sits and how far apart the rows are.
const ROW_TOP: f32 = 158.0;
const ROW_PITCH: f32 = 36.0;

/// Height of one row's plate.
const ROW_H: f32 = 30.0;

/// Text scales of the heading, the hint lines and the rows.
const HEADING_SCALE: f32 = 3.0;
const HINT_SCALE: f32 = 1.2;
const ROW_SCALE: f32 = 1.4;

/// Baselines of the heading, the chart's title and the two hint lines.
const HEADING_Y: f32 = 50.0;
const TITLE_Y: f32 = 92.0;
const HINT_Y: f32 = 116.0;
const HINT_SECOND_Y: f32 = 134.0;

/// Inset of a row's label from the plate's left edge, and of its value from the right.
const LABEL_INSET: f32 = 16.0;
const VALUE_INSET: f32 = 16.0;

/// How far a row's text sits below the top of its plate.
const TEXT_DROP: f32 = 7.0;

/// How wide the number of a row that holds one is given, how wide its slider is, and how far the
/// slider stands from the number.
const NUMBER_W: f32 = 92.0;
const SLIDER_W: f32 = 220.0;
const SLIDER_GAP: f32 = 14.0;

/// What is written where the value of the row that starts the slice is: the key that starts it.
const START_VALUE: &str = "ENTER";

/// One row of the panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PracticeRow {
    /// The row that starts the slice.
    Start,
    /// A row of the property.
    Value(PracticeElement),
}

/// How many rows the panel has: the one that starts the slice, and every row of the property.
pub(crate) const ROW_COUNT: usize = PracticeElement::ALL.len() + 1;

/// The row at one place of the panel, top to bottom. A place past the last row is the last row.
pub(crate) fn row_at(index: usize) -> PracticeRow {
    match index.checked_sub(1) {
        None => PracticeRow::Start,
        Some(at) => PracticeRow::Value(PracticeElement::ALL[at.min(PracticeElement::ALL.len() - 1)]),
    }
}

/// Where the row at one place of the panel is drawn.
fn row_rect(index: usize) -> Rect {
    Rect::new((CW as f32 - PANEL_W) * 0.5, ROW_TOP + index as f32 * ROW_PITCH, PANEL_W, ROW_H)
}

/// Where the number of a row that holds one is written, the row having been drawn at `row`: against
/// the row's right edge. A press there opens the row to be typed into.
pub(crate) fn row_number(row: Rect) -> Rect {
    Rect::new(row.x + row.w - VALUE_INSET - NUMBER_W, row.y, NUMBER_W + VALUE_INSET, row.h)
}

/// Where the slider of a row that holds a number is, the row having been drawn at `row`: before the
/// number, as tall as the row, so a press anywhere over it takes hold of it.
pub(crate) fn row_slider(row: Rect) -> Rect {
    Rect::new(row.x + row.w - VALUE_INSET - NUMBER_W - SLIDER_GAP - SLIDER_W, row.y, SLIDER_W, row.h)
}

/// Where a figure being typed into a row is drawn: over the slider and the number both.
fn row_edit(row: Rect) -> Rect {
    let slider = row_slider(row);
    Rect::new(slider.x, row.y, row.x + row.w - VALUE_INSET * 0.5 - slider.x, row.h)
}

/// The PRACTICE screen's own state: the panel being edited, the chart title it was opened on, where
/// the cursor is, and which row that holds a number is being typed into or dragged.
pub(crate) struct PracticeState {
    panel: PracticePanel,
    title: String,
    /// Either shift key is down, which is what makes one step of a number ten.
    shift_held: bool,
    slice_open: bool,
    /// The place of the row under the cursor.
    sel: usize,
    numbers: NumberInput<PracticeElement>,
    /// The property as it was when the slider being dragged was taken hold of: what a row carried
    /// out of the way of the one being dragged goes back to as the drag comes back.
    taken: PracticeProperty,
}

impl PracticeState {
    /// Open the panel on a chart that has already been loaded, with the cursor on the row that
    /// starts the slice.
    pub(crate) fn new(panel: PracticePanel, title: String) -> PracticeState {
        let taken = panel.property.clone();
        PracticeState { panel, title, shift_held: false, slice_open: false, sel: 0, numbers: NumberInput::new(), taken }
    }

    /// The panel this screen edits, for the tests that drive it.
    #[cfg(test)]
    pub(crate) fn panel(&self) -> &PracticePanel {
        &self.panel
    }

    /// The row under the cursor.
    pub(crate) fn focused(&self) -> PracticeRow {
        row_at(self.sel)
    }

    /// Move the cursor one row, wrapping at both ends so the list is a ring like every other panel
    /// here.
    fn move_cursor(&mut self, down: bool) {
        let step = if down { 1 } else { ROW_COUNT - 1 };
        self.sel = (self.sel.min(ROW_COUNT - 1) + step) % ROW_COUNT;
    }

    /// Start the slice the panel describes, or stay put when the chart is no longer loaded.
    fn start_slice(&mut self, shared: &mut AppShared) -> Transition {
        match shared.start_practice_slice(&mut self.panel) {
            Some(play) => {
                self.slice_open = true;
                Transition::Open(Stage::Play(Box::new(play)))
            }
            None => {
                notify(Level::Warn, PracticeText::ChartNoLongerLoaded.get(language()));
                Transition::Back
            }
        }
    }

    /// Store what was typed into the row being typed into, and close its editor. What is not a
    /// number is not stored, and the row says so for a while from `now`. A figure outside what the
    /// row may hold puts the row on the nearest value it may have.
    fn commit_edit(&mut self, now: std::time::Instant) {
        let Some((element, typed)) = self.numbers.take_edit() else {
            return;
        };
        let Some(value) = self.panel.number(element) else {
            return;
        };
        if let Some(steps) = self.numbers.typed_steps(element, &value.spec, &typed, now) {
            self.panel.set_number(element, steps);
        }
    }

    /// Carry the row being dragged to where the pointer is along its slider, as far as the row may
    /// go. What the row pushes out of its way on the way there comes back as the row does
    /// ([`PracticePanel::drag_number`]).
    fn drag_to(&mut self, x: f32) {
        let Some((element, _)) = self.numbers.dragged() else {
            return;
        };
        if let Some(steps) = self.panel.number(element).and_then(|value| self.numbers.dragged_to(&value, x)) {
            self.panel.drag_number(element, steps, &self.taken);
        }
    }

    /// Apply one left/right step to a row of the property. A row that holds a number moves one step
    /// of the smallest change it shows, and ten when `wide` asks; a row that holds a choice moves to
    /// the next of its ring, or the one before.
    fn step(&mut self, element: PracticeElement, delta: i32, wide: bool) {
        match self.panel.number(element) {
            Some(value) => {
                self.panel.set_number(element, value.stepped(delta, wide));
            }
            None => {
                self.panel.cycle(element, delta > 0);
            }
        }
    }

    /// A press of a button on a row of the property, the row at `index` of the panel. On a row that
    /// holds a number, a press of the left button on the slider begins a drag -- the row is put
    /// where the press landed, unless it landed on the knob, which is taken hold of where it stands
    /// -- and one on the number opens the row to be typed into. Any other press steps the row once,
    /// forwards or back as `delta` says.
    fn press_row(&mut self, shared: &AppShared, element: PracticeElement, index: usize, at: (f32, f32), button: MouseButton, delta: i32) {
        let part = hot_rect(shared, Hot::PracticeRow(index)).map_or(RowPart::Rest, |drawn| row_part(row_slider(drawn), row_number(drawn), at));
        match (self.panel.number(element).filter(|_| button == MouseButton::Left), part) {
            (Some(value), RowPart::Slider(area)) => {
                self.taken = self.panel.property.clone();
                if !self.numbers.begin_drag(element, area, &value, at.0) {
                    self.drag_to(at.0);
                }
            }
            (Some(value), RowPart::Number) => self.numbers.begin_edit(element, &value),
            (Some(_), RowPart::Rest) | (None, _) => self.step(element, delta, false),
        }
    }

    /// Enter on the row under the cursor: a row that holds a number is opened to be typed into, and
    /// from any other row the slice is started.
    fn enter_focused(&mut self, shared: &mut AppShared) -> Transition {
        let number = match self.focused() {
            PracticeRow::Start => None,
            PracticeRow::Value(element) => self.panel.number(element).map(|value| (element, value)),
        };
        match number {
            Some((element, value)) => {
                self.numbers.begin_edit(element, &value);
                Transition::Stay
            }
            None => self.start_slice(shared),
        }
    }

    /// Keys on the panel, once the modifier and a row being typed into have had their say.
    fn row_key(&mut self, shared: &mut AppShared, code: KeyCode) -> Transition {
        let focused = self.focused();
        match (code, focused) {
            (KeyCode::Escape, _) => return Transition::Back,
            (KeyCode::Enter | KeyCode::NumpadEnter, _) => return self.enter_focused(shared),
            (KeyCode::ArrowUp, _) => self.move_cursor(false),
            (KeyCode::ArrowDown, _) => self.move_cursor(true),
            (KeyCode::Tab, _) => self.move_cursor(!self.shift_held),
            (KeyCode::ArrowLeft, PracticeRow::Value(element)) => self.step(element, ROW_STEP_BACK, self.shift_held),
            (KeyCode::ArrowRight, PracticeRow::Value(element)) => self.step(element, ROW_STEP_FORWARD, self.shift_held),
            (KeyCode::Digit0 | KeyCode::Numpad0, PracticeRow::Value(element)) => {
                self.panel.reset_number(element);
            }
            _ => {}
        }
        Transition::Stay
    }

    /// Draw one row of the property on its plate: its label, and on its right its choice, or its
    /// slider and its number, or the figure being typed into it.
    fn draw_value_row(&self, canvas: &mut Canvas<'_>, rect: Rect, element: PracticeElement, on: bool, now: std::time::Instant) {
        let th = rbms_render::theme();
        let text_y = rect.y + TEXT_DROP;
        let right = rect.x + rect.w - VALUE_INSET;
        let colour = if on { th.text } else { th.text_dim };
        draw_text(canvas, rect.x + LABEL_INSET, text_y, ROW_SCALE, colour, element.label());
        let Some(number) = self.panel.number(element) else {
            draw_text_right(canvas, right, text_y, ROW_SCALE, colour, &self.panel.value_text(element));
            return;
        };
        if let Some(edit) = self.numbers.edit_of(element) {
            draw_edit(canvas, row_edit(rect), text_y, ROW_SCALE, edit);
        } else if self.numbers.is_refused(element, now) {
            draw_refusal(canvas, right, text_y, ROW_SCALE);
        } else {
            let look = match (self.numbers.is_dragging(element), on) {
                (true, _) => SliderLook::Dragged,
                (false, true) => SliderLook::Focused,
                (false, false) => SliderLook::Plain,
            };
            draw_slider(canvas, row_slider(rect), number.fraction(), look);
            draw_text_right(canvas, right, text_y, ROW_SCALE, colour, &self.panel.value_text(element));
        }
    }
}

impl StageHandler for PracticeState {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    /// Up/down move the cursor, and so does Tab (back with Shift held); left/right change the
    /// focused row (ten steps of a number with Shift held), the zero key puts a number back, Enter
    /// starts the slice or opens a number to be typed into, and Escape leaves the chart -- or, while
    /// a number is being typed, only drops what was typed. A key pressed while a slider is being dragged ends the drag first: the key may move
    /// the cursor on, and the pointer must not go on carrying a row the cursor has left.
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        if let Some(held) = shift_after(self.shift_held, &key) {
            self.shift_held = held;
            return Transition::Stay;
        }
        if !key.pressed {
            return Transition::Stay;
        }
        self.numbers.end_drag();
        match self.numbers.edit_key(&key) {
            Some(EditKey::Taken) => return Transition::Stay,
            Some(EditKey::Cancel) => {
                self.numbers.cancel_edit();
                return Transition::Stay;
            }
            Some(EditKey::Commit) => {
                self.commit_edit(ctx.now);
                return Transition::Stay;
            }
            Some(EditKey::CommitThenPass) => self.commit_edit(ctx.now),
            None => {}
        }
        self.row_key(ctx.shared, key.code)
    }

    /// A press on a row moves the cursor there. On the row that starts the slice, the left button
    /// starts it. On a row of the property a press steps the row once: forwards with the left
    /// button, and back with the right, so a step taken by mistake can be taken back with the mouse.
    /// On a row that holds a number, a press of the left button on the slider puts the row where
    /// the press landed and begins a drag, and one on the number opens the row to be typed into; the
    /// left button coming back up ends a drag. A press anywhere else does nothing, but any press
    /// stores a number that was being typed.
    fn handle_mouse(&mut self, ctx: &mut FrameCtx<'_>, at: (f32, f32), button: MouseButton, pressed: bool) -> Transition {
        if !pressed {
            if button == MouseButton::Left {
                self.numbers.end_drag();
            }
            return Transition::Stay;
        }
        let Some(delta) = click_step(button) else {
            return Transition::Stay;
        };
        self.numbers.end_drag();
        self.commit_edit(ctx.now);
        let Some(Hot::PracticeRow(index)) = ctx.shared.hit_test(at) else {
            return Transition::Stay;
        };
        self.sel = index.min(ROW_COUNT - 1);
        match row_at(index) {
            PracticeRow::Start if button == MouseButton::Left => return self.start_slice(ctx.shared),
            PracticeRow::Start => {}
            PracticeRow::Value(element) => self.press_row(ctx.shared, element, index, at, button, delta),
        }
        Transition::Stay
    }

    /// The cursor moving with a button held carries the row being dragged along its slider, wherever
    /// on the screen the pointer is.
    fn handle_mouse_drag(&mut self, _ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition {
        self.drag_to(at.0);
        Transition::Stay
    }

    /// The panel keeps its values between visits, so what the player set up is written out on the
    /// way out rather than on every keystroke. A number still being typed is dropped, and a slider
    /// still held is let go of where it is.
    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        self.numbers.cancel_edit();
        self.numbers.end_drag();
        self.panel.finish();
        ctx.shared.practice_book.put(self.panel.chart_key(), self.panel.property.clone());
        ctx.shared.practice_book.save(&practice_path(&ctx.shared.settings_path));
        if !self.slice_open {
            ctx.shared.release_practice_chart();
        }
    }

    fn on_enter(&mut self, _ctx: &mut FrameCtx<'_>) {
        self.slice_open = false;
    }

    /// The panel holds every key it is drawn with, so the option overlay does not open over it.
    fn holds_keys(&self, _ctx: &FrameCtx<'_>) -> bool {
        true
    }

    /// The heading, the chart's title and the keys of the panel, then each row on its plate, which
    /// is recorded as clickable under its place in the panel. While a row is being typed into, the
    /// keys written are the keys of the editor.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        let th = rbms_render::theme();
        canvas.clear_bga();
        canvas.clear(th.bg);
        let x0 = row_rect(0).x;
        draw_text(canvas, x0, HEADING_Y, HEADING_SCALE, th.text, PracticeText::Heading.get(language()));
        draw_text(canvas, x0, TITLE_Y, ROW_SCALE, th.text_dim, &self.title);
        match self.numbers.edit_hint() {
            Some(hint) => draw_text(canvas, x0, HINT_Y, HINT_SCALE, th.text_muted, hint),
            None => {
                draw_text(canvas, x0, HINT_Y, HINT_SCALE, th.text_muted, PracticeText::HintKeys.get(language()));
                draw_text(canvas, x0, HINT_SECOND_Y, HINT_SCALE, th.text_muted, PracticeText::HintMouse.get(language()));
            }
        }
        for index in 0..ROW_COUNT {
            let rect = row_rect(index);
            let on = index == self.sel;
            canvas.fill_rect(rect, if on { th.button } else { th.panel });
            match row_at(index) {
                PracticeRow::Start => {
                    draw_text(
                        canvas,
                        rect.x + LABEL_INSET,
                        rect.y + TEXT_DROP,
                        ROW_SCALE,
                        if on { th.text } else { th.accent },
                        PracticeText::Start.get(language()),
                    );
                    draw_text_right(canvas, rect.x + rect.w - VALUE_INSET, rect.y + TEXT_DROP, ROW_SCALE, th.text_muted, START_VALUE);
                }
                PracticeRow::Value(element) => self.draw_value_row(canvas, rect, element, on, ctx.now),
            }
            ctx.shared.hot.push((rect, Hot::PracticeRow(index)));
        }
    }
}

#[cfg(test)]
mod tests;
