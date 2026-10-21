//! A row that holds a number, as every built-in screen sets one: by a key, by a slider and by a
//! figure typed into the row.
//!
//! Four screens list such rows -- the settings screen, the skin menu, the play menu and the practice
//! panel -- and all of them count a row the same way ([`rbms_config::NumberSpec`]: whole steps of
//! the smallest change the row can hold), draw its slider with the same call ([`draw_slider`]), turn a place on the
//! slider into a step with the same arithmetic ([`steps_at`]), and take a typed figure with the
//! same editor ([`NumberEdit`]) and the same bookkeeping ([`NumberInput`]). What differs from one
//! screen to the next is what a row is and where its number is stored, which is the screen's: it
//! answers what a row holds ([`NumberValue`]) and writes the step it is handed. A screen answers
//! anew every time it is asked, so a row whose ends depend on the rows beside it -- the gauge a
//! practice slice starts at, under the gauge that is selected -- is counted between the ends it has
//! at that moment, and the screen holds the step it is handed to whatever else the row must keep to.
//!
//! A row that holds a length of time ([`rbms_config::NumberSpec::as_clock`]) is written as minutes
//! and seconds, and is typed either that way or as seconds ([`NumberEdit`], [`EDIT_HINT_CLOCK`]).
//!
//! A key moves a row one step, and ten with Shift held. A press on the slider puts the row where
//! the press landed and a drag carries it along with the pointer, wherever the pointer goes, until
//! the button comes back up. A press on the knob itself takes hold of the knob where it landed: the
//! row stays where it is until the pointer moves, and then moves by as far as the pointer does.
//! Enter, or a press on the number itself, opens the row to be typed into: the number it holds is
//! there and wholly selected, so the first character typed replaces it.
//!
//! The same editor takes a colour typed in hex into a colour row ([`NumberEdit::hex`],
//! [`NumberInput::begin_hex_edit`]): it is drawn, committed and called off as a number is, and a
//! figure that is no colour is refused the same way, with words of its own ([`COLOR_REFUSAL_NOTE`]).
//!
//! A row's slider and its arrows need not reach as far as a typed figure does
//! ([`NumberValue::slider_max`]): an opacity is dragged and stepped between none of a part and all
//! of it, and only a figure typed into it goes past all of it, for the part a skin lights by adding
//! to its alpha.

use std::time::{Duration, Instant};

use rbms_config::NumberSpec;
use rbms_render::{Rect, Renderer, draw_text, draw_text_right, text_width, theme};
use rbms_skin::dst::SkinColor;
use winit::keyboard::KeyCode;

use crate::stage::KeyInput;
use crate::textedit::{TextEdit, edit_key};
use crate::{AppShared, Hot};

/// How many steps one press of an arrow moves a row by, and by how many with Shift held.
pub(crate) const ONE_STEP: u8 = 1;
pub(crate) const SHIFT_STEPS: u8 = 10;

/// How thick the rail a slider's knob travels along is drawn, and how wide the knob is.
const RAIL_H: f32 = 4.0;
const KNOB_W: f32 = 8.0;

/// How much of the height it is given a slider's knob takes.
const KNOB_HEIGHT_SHARE: f32 = 0.56;

/// How far the line round the knob of a slider being dragged stands out from the knob.
const KNOB_HALO: f32 = 2.0;

/// How thick the line round a row being typed into is, how far inside it the typed figure begins,
/// and how far the fill behind a figure that is wholly selected reaches past it on either side.
const EDIT_BORDER: f32 = 2.0;
const EDIT_PAD: f32 = 8.0;
const SELECTION_PAD: f32 = 3.0;

/// How wide the caret of a row being typed into is drawn, how far it is kept from the top and the
/// foot of the line round the row, and how far it stands clear of the character before it.
const CARET_W: f32 = 2.0;
const CARET_INSET: f32 = 3.0;
const CARET_GAP: f32 = 2.0;

/// The most characters a figure typed into a row runs to, which is more than any row's longest.
const EDIT_MAX_CHARS: usize = 12;

/// The keys of a screen while a row is being typed into, written where the screen names its keys.
pub(crate) const EDIT_HINT: &str = "TYPE A NUMBER   ENTER SET   ESC CANCEL   UP DOWN TAB SET AND MOVE ON";

/// The same while the row being typed into holds a length of time.
pub(crate) const EDIT_HINT_CLOCK: &str = "TYPE SECONDS OR MIN:SEC   ENTER SET   ESC CANCEL   UP DOWN TAB SET AND MOVE ON";

/// The same while a colour is being typed into a colour row.
pub(crate) const EDIT_HINT_HEX: &str = "TYPE A HEX COLOR: RRGGBB OR RRGGBBAA   ENTER SET   ESC CANCEL   UP DOWN TAB SET AND MOVE ON";

/// What stands between the minutes and the seconds of a length of time as it is typed.
const CLOCK_SEPARATOR: char = ':';

/// What may stand before the digits of a colour as it is typed.
const HEX_PREFIX: char = '#';

/// What a row says in place of its number when what was typed into it was not one, and for how
/// long it says so.
pub(crate) const REFUSAL_NOTE: &str = "NOT A NUMBER";
pub(crate) const REFUSAL_SHOWN_FOR: Duration = Duration::from_millis(1500);

/// What a colour row says in place of its colour when what was typed into it was not one.
pub(crate) const COLOR_REFUSAL_NOTE: &str = "NOT A COLOR";

/// The number one row holds, as its screen reads it: how the row is counted, the step it stands
/// on, the step it ships on, how far its slider reaches, and whether it is worked out from other
/// rows and so cannot be set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct NumberValue {
    pub(crate) spec: NumberSpec,
    pub(crate) steps: i64,
    /// The step the row is put back to by the key that resets it.
    pub(crate) shipped: i64,
    /// The highest step the row's slider and its arrows carry it to. It is the row's upper end
    /// ([`NumberValue::of`]), except for a row only a typed figure takes further.
    pub(crate) slider_max: i64,
    /// Whether the row shows a number worked out from the others: its slider is drawn faint, and
    /// neither a drag nor a typed figure moves it.
    pub(crate) locked: bool,
}

impl NumberValue {
    /// A row that can be set, standing on `steps`, whose slider and arrows reach both its ends.
    pub(crate) fn of(spec: NumberSpec, steps: i64, shipped: i64) -> NumberValue {
        NumberValue { spec, steps, shipped, slider_max: spec.max, locked: false }
    }

    /// The same row with its slider and its arrows ending at `slider_max`, short of the row's
    /// upper end, which a typed figure still reaches.
    pub(crate) fn with_slider_to(self, slider_max: i64) -> NumberValue {
        NumberValue { slider_max: self.spec.clamp(slider_max), ..self }
    }

    /// The steps the row's slider runs over: the row's own, up to where the slider ends.
    pub(crate) fn track(&self) -> NumberSpec {
        NumberSpec { max: self.slider_max.clamp(self.spec.min, self.spec.max.max(self.spec.min)), ..self.spec }
    }

    /// The step one press of an arrow moves the row to: one step in the direction of `delta`, or
    /// ten when `wide` asks, stopping at the row's two ends.
    ///
    /// An arrow does not carry a row past where its slider ends. A row a typed figure put past it
    /// is stepped from where it stands, either way, as far as the row's own end.
    pub(crate) fn stepped(&self, delta: i32, wide: bool) -> i64 {
        let steps = if wide { SHIFT_STEPS } else { ONE_STEP };
        let next = self.spec.moved(self.steps, i64::from(delta.signum()) * i64::from(steps));
        if self.steps <= self.slider_max { next.min(self.slider_max) } else { next }
    }

    /// How far along its slider the row stands. A row past where its slider ends stands at the
    /// slider's end.
    pub(crate) fn fraction(&self) -> f32 {
        self.track().fraction(self.steps) as f32
    }
}

/// How a slider is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SliderLook {
    /// A row the cursor is not on.
    Plain,
    /// The row under the cursor.
    Focused,
    /// The row being dragged.
    Dragged,
    /// A row whose number is worked out from the others.
    Locked,
}

/// Where the knob of a slider drawn in `area` is at `fraction` of the way along it. The knob keeps
/// inside the area at both ends: its left edge is on the area's at nothing and its right edge on
/// the area's at one.
pub(crate) fn knob_rect(area: Rect, fraction: f32) -> Rect {
    let along = if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) };
    let height = (area.h * KNOB_HEIGHT_SHARE).round();
    Rect::new(area.x + (area.w - KNOB_W).max(0.0) * along, area.y + ((area.h - height) * 0.5).round(), KNOB_W.min(area.w), height)
}

/// The step of a row a pointer at `x` puts the slider drawn in `area` on, `spec` being the steps
/// the slider runs over ([`NumberValue::track`]): the step nearest to where the middle of the knob
/// would be under the pointer, and the slider's end for a pointer past either end of it.
pub(crate) fn steps_at(spec: &NumberSpec, area: Rect, x: f32) -> i64 {
    let travel = area.w - KNOB_W;
    if travel <= 0.0 {
        return spec.clamp(spec.min);
    }
    spec.at_fraction(f64::from((x - area.x - KNOB_W * 0.5) / travel))
}

/// Draw a slider in `area`: a rail across its middle, filled from its left end to the knob, and
/// the knob at `fraction` of the way along it.
pub(crate) fn draw_slider<R: Renderer>(r: &mut R, area: Rect, fraction: f32, look: SliderLook) {
    let colors = theme();
    let knob = knob_rect(area, fraction);
    let rail = Rect::new(area.x, area.y + ((area.h - RAIL_H) * 0.5).round(), area.w, RAIL_H);
    let (rail_color, filled, grip) = match look {
        SliderLook::Plain => (colors.divider, colors.text_muted, colors.text_dim),
        SliderLook::Focused => (colors.divider, colors.text_dim, colors.focus),
        SliderLook::Dragged => (colors.divider, colors.focus, colors.focus),
        SliderLook::Locked => (colors.panel_hi, colors.panel_hi, colors.text_muted),
    };
    r.fill_rect(rail, rail_color);
    r.fill_rect(Rect::new(rail.x, rail.y, (knob.x + knob.w * 0.5 - rail.x).max(0.0), rail.h), filled);
    if look == SliderLook::Dragged {
        r.fill_rect(Rect::new(knob.x - KNOB_HALO, knob.y - KNOB_HALO, knob.w + KNOB_HALO * 2.0, knob.h + KNOB_HALO * 2.0), colors.text);
    }
    r.fill_rect(knob, grip);
}

/// Draw a row being typed into, in `field`: a line round it, the figure typed so far from its left
/// edge with its top at `text_y`, and either the fill behind a figure that is wholly selected or
/// the caret where the next character lands.
pub(crate) fn draw_edit<R: Renderer>(r: &mut R, field: Rect, text_y: f32, scale: f32, edit: &NumberEdit) {
    let colors = theme();
    r.fill_rect(field, colors.accent);
    r.fill_rect(Rect::new(field.x + EDIT_BORDER, field.y + EDIT_BORDER, field.w - EDIT_BORDER * 2.0, field.h - EDIT_BORDER * 2.0), colors.bg);
    let left = field.x + EDIT_PAD;
    let (top, height) = (field.y + EDIT_BORDER + CARET_INSET, field.h - (EDIT_BORDER + CARET_INSET) * 2.0);
    if edit.is_all_selected() && !edit.text().is_empty() {
        r.fill_rect(Rect::new(left - SELECTION_PAD, top, text_width(edit.text(), scale) + SELECTION_PAD * 2.0, height), colors.button);
        draw_text(r, left, text_y, scale, colors.title_focus, edit.text());
        return;
    }
    draw_text(r, left, text_y, scale, colors.text, edit.text());
    let before: String = edit.text().chars().take(edit.caret()).collect();
    r.fill_rect(Rect::new(left + text_width(&before, scale) + CARET_GAP, top, CARET_W, height), colors.focus);
}

/// Write, with its right edge at `right`, what a row says in place of its number for a while after
/// what was typed into it was refused.
pub(crate) fn draw_refusal<R: Renderer>(r: &mut R, right: f32, text_y: f32, scale: f32) {
    draw_refusal_note(r, right, text_y, scale, REFUSAL_NOTE);
}

/// [`draw_refusal`] in the words `note`, for a row that is refused something other than a number.
pub(crate) fn draw_refusal_note<R: Renderer>(r: &mut R, right: f32, text_y: f32, scale: f32, note: &str) {
    draw_text_right(r, right, text_y, scale, theme().accent, note);
}

/// What a key did to a row being typed into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EditKey {
    /// The key was the editor's: a character, a delete, a move of the caret, or a key it takes and
    /// does nothing with so the screen behind does not act on it.
    Taken,
    /// Enter: what was typed is to be stored.
    Commit,
    /// Escape: what was typed is dropped, and the screen is not left.
    Cancel,
    /// A key that moves the cursor or the screen on: what was typed is to be stored, and then the
    /// key does what it always does.
    CommitThenPass,
}

/// What a figure being typed into a row is written as, which is what the editor takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditKind {
    /// A number: digits, a sign and a decimal point.
    Number,
    /// A length of time: a number, and the colon between its minutes and its seconds.
    Clock,
    /// A colour: hex digits, after a `#` or not.
    Hex,
}

/// A figure being typed into a row.
///
/// It opens on the number the row holds, wholly selected: the first character typed replaces it,
/// a delete empties it, and a move of the caret lets go of the selection and leaves the number to
/// be corrected. Only what a number is written with is taken -- digits, a sign and a decimal point,
/// and for a length of time the colon between its minutes and its seconds -- so a unit typed after
/// it is dropped as it is typed. A colour opened with [`NumberEdit::hex`] takes hex digits and a
/// `#` in their place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NumberEdit {
    line: TextEdit,
    all_selected: bool,
    kind: EditKind,
}

impl NumberEdit {
    /// An editor opened on the number a row holds, wholly selected.
    pub(crate) fn over(spec: &NumberSpec, steps: i64) -> NumberEdit {
        let kind = if spec.clock { EditKind::Clock } else { EditKind::Number };
        NumberEdit { line: TextEdit::from_text(spec.plain(steps)), all_selected: true, kind }
    }

    /// An editor opened on the colour a colour row shows, written as `hex`, wholly selected.
    pub(crate) fn hex(hex: &str) -> NumberEdit {
        NumberEdit { line: TextEdit::from_text(hex.to_owned()), all_selected: true, kind: EditKind::Hex }
    }

    /// Whether the row being typed into holds a length of time.
    #[cfg(test)]
    pub(crate) fn is_clock(&self) -> bool {
        self.kind == EditKind::Clock
    }

    /// Whether what is being typed is a colour.
    #[cfg(test)]
    pub(crate) fn is_hex(&self) -> bool {
        self.kind == EditKind::Hex
    }

    /// Whether `ch` is something the figure being typed is written with.
    fn takes(&self, ch: char) -> bool {
        match self.kind {
            EditKind::Number => ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.'),
            EditKind::Clock => ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.' | CLOCK_SEPARATOR),
            EditKind::Hex => ch.is_ascii_hexdigit() || ch == HEX_PREFIX,
        }
    }

    /// What has been typed.
    pub(crate) fn text(&self) -> &str {
        self.line.text()
    }

    /// How many characters are before the caret.
    pub(crate) fn caret(&self) -> usize {
        self.line.cursor()
    }

    /// Whether the whole figure is selected, so the next character typed replaces it.
    pub(crate) fn is_all_selected(&self) -> bool {
        self.all_selected
    }

    /// Type `text` in at the caret, keeping only what a number is written with, in place of the
    /// whole figure while it is selected.
    fn type_in(&mut self, text: &str) {
        let typed: String = text.chars().filter(|ch| self.takes(*ch)).collect();
        if typed.is_empty() {
            return;
        }
        if self.all_selected {
            self.line.take();
            self.all_selected = false;
        }
        let room = EDIT_MAX_CHARS.saturating_sub(self.line.text().chars().count());
        self.line.insert(&typed.chars().take(room).collect::<String>());
    }

    /// Apply one pressed key, and answer what it was to the editor.
    pub(crate) fn key(&mut self, key: &KeyInput<'_>) -> EditKey {
        match key.code {
            KeyCode::Enter | KeyCode::NumpadEnter => return EditKey::Commit,
            KeyCode::Escape => return EditKey::Cancel,
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Tab => return EditKey::CommitThenPass,
            KeyCode::Backspace | KeyCode::Delete if self.all_selected => {
                self.line.take();
            }
            KeyCode::ArrowLeft | KeyCode::Home if self.all_selected => self.line.home(),
            KeyCode::ArrowRight | KeyCode::End if self.all_selected => self.line.end(),
            KeyCode::Backspace | KeyCode::Delete | KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Home | KeyCode::End => {
                edit_key(&mut self.line, key, false)
            }
            _ => {
                if let Some(text) = key.text {
                    self.type_in(text);
                }
                return EditKey::Taken;
            }
        }
        self.all_selected = false;
        EditKey::Taken
    }
}

/// What a screen keeps about the rows it lists that hold a number: which one is being typed into,
/// which one is being dragged and along what, and which one refused what was typed into it.
///
/// `K` is whatever the screen tells its rows apart by. One row at a time is typed into and one at a
/// time is dragged.
#[derive(Debug)]
pub(crate) struct NumberInput<K> {
    edit: Option<(K, NumberEdit)>,
    drag: Option<Drag<K>>,
    refused: Option<(K, Instant)>,
}

/// A drag of one row along its slider.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drag<K> {
    row: K,
    /// The slider as it was drawn when the drag began.
    area: Rect,
    /// How far from the middle of the knob the press that began the drag landed, when it landed on
    /// the knob: the knob is carried by where it was taken hold of, not by its middle. Nothing for a
    /// press anywhere else on the slider, which brings the middle of the knob under the pointer.
    grip: f32,
}

impl<K: Copy + PartialEq> NumberInput<K> {
    pub(crate) fn new() -> NumberInput<K> {
        NumberInput { edit: None, drag: None, refused: None }
    }

    /// The row being typed into.
    pub(crate) fn editing(&self) -> Option<K> {
        self.edit.as_ref().map(|(row, _)| *row)
    }

    /// The editor of `row`, while it is the row being typed into.
    pub(crate) fn edit_of(&self, row: K) -> Option<&NumberEdit> {
        self.edit.as_ref().filter(|(edited, _)| *edited == row).map(|(_, edit)| edit)
    }

    /// Open `row` to be typed into, on the number it holds. A row whose number is worked out from
    /// the others is not opened.
    pub(crate) fn begin_edit(&mut self, row: K, value: &NumberValue) {
        if !value.locked {
            self.refused = None;
            self.edit = Some((row, NumberEdit::over(&value.spec, value.steps)));
        }
    }

    /// Hand one pressed key to the row being typed into, or `None` when no row is.
    pub(crate) fn edit_key(&mut self, key: &KeyInput<'_>) -> Option<EditKey> {
        self.edit.as_mut().map(|(_, edit)| edit.key(key))
    }

    /// Open `row`, a colour row, to have a colour typed into it, on the colour it shows written as
    /// `hex`.
    pub(crate) fn begin_hex_edit(&mut self, row: K, hex: &str) {
        self.refused = None;
        self.edit = Some((row, NumberEdit::hex(hex)));
    }

    /// Close the editor and drop what was typed.
    pub(crate) fn cancel_edit(&mut self) {
        self.edit = None;
    }

    /// Close the editor and hand back the row it was on with what was typed, for the screen to
    /// store ([`NumberInput::typed_steps`]).
    pub(crate) fn take_edit(&mut self) -> Option<(K, String)> {
        self.edit.take().map(|(row, mut edit)| (row, edit.line.take()))
    }

    /// The step what was typed into `row` stands for, or `None` when it is not a number -- which
    /// the row then says for a while, from `now`.
    pub(crate) fn typed_steps(&mut self, row: K, spec: &NumberSpec, typed: &str, now: Instant) -> Option<i64> {
        let steps = spec.parse(typed);
        if steps.is_none() {
            self.refused = Some((row, now + REFUSAL_SHOWN_FOR));
        }
        steps
    }

    /// The colour what was typed into `row` writes ([`SkinColor::from_hex`]), or `None` when it is
    /// not one -- which the row then says for a while, from `now`.
    pub(crate) fn typed_color(&mut self, row: K, typed: &str, now: Instant) -> Option<SkinColor> {
        let color = SkinColor::from_hex(typed.trim());
        if color.is_none() {
            self.refused = Some((row, now + REFUSAL_SHOWN_FOR));
        }
        color
    }

    /// The line of keys a screen writes while a row is being typed into: the one for a length of
    /// time while the row holds one, the one for a colour while a colour is typed, and `None` while
    /// no row is being typed into.
    pub(crate) fn edit_hint(&self) -> Option<&'static str> {
        self.edit.as_ref().map(|(_, edit)| match edit.kind {
            EditKind::Number => EDIT_HINT,
            EditKind::Clock => EDIT_HINT_CLOCK,
            EditKind::Hex => EDIT_HINT_HEX,
        })
    }

    /// Whether `row` is still saying that what was typed into it was not a number.
    pub(crate) fn is_refused(&self, row: K, now: Instant) -> bool {
        self.refused.is_some_and(|(refused, until)| refused == row && now < until)
    }

    /// Begin dragging `row`, which holds `value`, along the slider drawn in `area`, with a press at
    /// `x`. Answers whether the press took hold of the knob where it stands: the row is then not
    /// moved until the pointer is, and otherwise it is brought under the pointer at once
    /// ([`NumberInput::dragged_to`]).
    pub(crate) fn begin_drag(&mut self, row: K, area: Rect, value: &NumberValue, x: f32) -> bool {
        let knob = knob_rect(area, value.fraction());
        let on_the_knob = x >= knob.x && x < knob.x + knob.w;
        let grip = if on_the_knob { x - (knob.x + knob.w * 0.5) } else { 0.0 };
        self.drag = Some(Drag { row, area, grip });
        on_the_knob
    }

    /// The row being dragged, and the slider it is dragged along as it was when the drag began.
    pub(crate) fn dragged(&self) -> Option<(K, Rect)> {
        self.drag.map(|drag| (drag.row, drag.area))
    }

    /// The step a pointer at `x` carries the row being dragged to, `value` being what that row
    /// holds, or `None` when no row is being dragged. The knob follows the pointer from where it was
    /// taken hold of, along the steps the row's slider runs over.
    pub(crate) fn dragged_to(&self, value: &NumberValue, x: f32) -> Option<i64> {
        self.drag.map(|drag| steps_at(&value.track(), drag.area, x - drag.grip))
    }

    /// Whether `row` is the row being dragged.
    pub(crate) fn is_dragging(&self, row: K) -> bool {
        self.drag.is_some_and(|drag| drag.row == row)
    }

    /// End the drag, and answer whether there was one.
    pub(crate) fn end_drag(&mut self) -> bool {
        self.drag.take().is_some()
    }
}

/// Which part of a row that holds a number a press landed on.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum RowPart {
    /// The slider, drawn where it is said: the press puts the row where it landed and begins a drag
    /// along it.
    Slider(Rect),
    /// The number: the press opens the row to be typed into.
    Number,
    /// Anywhere else on the row: the press steps the row, as it does a row that holds no number.
    Rest,
}

/// Which part of a row a press at `at` landed on, the row's slider being drawn in `slider` and its
/// number written in `number`.
pub(crate) fn row_part(slider: Rect, number: Rect, at: (f32, f32)) -> RowPart {
    let inside = |rect: Rect| at.0 >= rect.x && at.0 < rect.x + rect.w && at.1 >= rect.y && at.1 < rect.y + rect.h;
    if inside(slider) {
        RowPart::Slider(slider)
    } else if inside(number) {
        RowPart::Number
    } else {
        RowPart::Rest
    }
}

/// Where the last frame drew what it made clickable as `hot`: the row a press landed on, which is
/// what the row's slider and its number are placed from.
pub(crate) fn hot_rect(shared: &AppShared, hot: Hot) -> Option<Rect> {
    shared.hot.iter().rev().find(|(_, entry)| *entry == hot).map(|(rect, _)| *rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERCENT: NumberSpec = NumberSpec::whole(0, 100, "%");
    const PLACE: NumberSpec = NumberSpec::whole(-2000, 2000, "").with_sign();
    const CLOCK: NumberSpec = NumberSpec::fine(0, 6000, 10, 1, "").as_clock();
    const AREA: Rect = Rect { x: 200.0, y: 40.0, w: 108.0, h: 29.0 };

    fn press(code: KeyCode) -> KeyInput<'static> {
        KeyInput { code, pressed: true, released: false, text: None }
    }

    fn typed(text: &'static str) -> KeyInput<'static> {
        KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) }
    }

    fn value(steps: i64) -> NumberValue {
        NumberValue::of(PERCENT, steps, 100)
    }

    #[test]
    fn an_arrow_moves_one_step_and_ten_with_shift_and_stops_at_the_ends() {
        assert_eq!(value(40).stepped(1, false), 41);
        assert_eq!(value(40).stepped(-1, false), 39);
        assert_eq!(value(40).stepped(1, true), 50);
        assert_eq!(value(95).stepped(1, true), 100);
        assert_eq!(value(4).stepped(-1, true), 0);
        assert_eq!(value(100).stepped(1, false), 100);
    }

    /// The knob is drawn whole at both ends of the slider, and the pointer over the middle of the
    /// knob reads back as the step the knob is drawn for.
    #[test]
    fn the_knob_keeps_inside_its_slider_and_reads_back_under_the_pointer() {
        let (first, last) = (knob_rect(AREA, 0.0), knob_rect(AREA, 1.0));
        assert_eq!(first.x, AREA.x);
        assert_eq!(last.x + last.w, AREA.x + AREA.w);
        assert!(first.y >= AREA.y && first.y + first.h <= AREA.y + AREA.h);
        for steps in PERCENT.min..=PERCENT.max {
            let knob = knob_rect(AREA, PERCENT.fraction(steps) as f32);
            assert_eq!(steps_at(&PERCENT, AREA, knob.x + knob.w * 0.5), steps);
        }
    }

    #[test]
    fn a_pointer_past_either_end_of_the_slider_is_the_end_of_the_row() {
        assert_eq!(steps_at(&PERCENT, AREA, AREA.x - 500.0), 0);
        assert_eq!(steps_at(&PERCENT, AREA, AREA.x), 0);
        assert_eq!(steps_at(&PERCENT, AREA, AREA.x + AREA.w), 100);
        assert_eq!(steps_at(&PERCENT, AREA, AREA.x + AREA.w + 500.0), 100);
        assert_eq!(steps_at(&PLACE, AREA, AREA.x + AREA.w * 0.5), 0, "the middle of a row that runs both ways is nothing");
        assert_eq!(steps_at(&PERCENT, Rect::new(0.0, 0.0, KNOB_W, 10.0), 4.0), 0, "a slider with no room to travel stands on its lower end");
    }

    #[test]
    fn an_editor_opens_on_the_number_wholly_selected_and_the_first_character_replaces_it() {
        let mut edit = NumberEdit::over(&PERCENT, 40);
        assert_eq!((edit.text(), edit.is_all_selected()), ("40", true));
        assert_eq!(edit.key(&typed("7")), EditKey::Taken);
        assert_eq!((edit.text(), edit.is_all_selected()), ("7", false));
        edit.key(&typed("5"));
        assert_eq!(edit.text(), "75");
    }

    #[test]
    fn a_move_of_the_caret_lets_go_of_the_selection_and_keeps_the_number() {
        let mut edit = NumberEdit::over(&PERCENT, 40);
        edit.key(&press(KeyCode::ArrowRight));
        assert_eq!((edit.text(), edit.is_all_selected(), edit.caret()), ("40", false, 2));
        edit.key(&typed("0"));
        assert_eq!(edit.text(), "400");
        let mut edit = NumberEdit::over(&PERCENT, 40);
        edit.key(&press(KeyCode::Home));
        edit.key(&typed("1"));
        assert_eq!(edit.text(), "140");
        let mut edit = NumberEdit::over(&PERCENT, 40);
        edit.key(&press(KeyCode::Backspace));
        assert_eq!((edit.text(), edit.is_all_selected()), ("", false));
        edit.key(&press(KeyCode::Backspace));
        assert_eq!(edit.text(), "", "a delete of nothing is not an error");
    }

    #[test]
    fn only_what_a_number_is_written_with_is_typed() {
        let mut edit = NumberEdit::over(&PLACE, 0);
        for text in ["-", "1", "2", ".", "5", "p", "x", "%", " ", "m", "s"] {
            assert_eq!(edit.key(&typed(text)), EditKey::Taken);
        }
        assert_eq!(edit.text(), "-12.5");
        let mut untouched = NumberEdit::over(&PERCENT, 40);
        untouched.key(&typed("%"));
        assert_eq!((untouched.text(), untouched.is_all_selected()), ("40", true), "a unit typed over a selected number replaced it");
        let mut long = NumberEdit::over(&PERCENT, 0);
        for _ in 0..40 {
            long.key(&typed("9"));
        }
        assert_eq!(long.text().chars().count(), EDIT_MAX_CHARS);
    }

    /// A length of time opens as it is written and takes the colon it is written with, which no
    /// other row does; what is typed into it reads as minutes and seconds or as seconds alone.
    #[test]
    fn a_length_of_time_is_typed_with_its_colon() {
        let mut edit = NumberEdit::over(&CLOCK, 834);
        assert_eq!((edit.text(), edit.is_all_selected(), edit.is_clock()), ("1:23.4", true, true));
        for text in ["2", ":", "0", "5", ".", "5", "s"] {
            assert_eq!(edit.key(&typed(text)), EditKey::Taken);
        }
        assert_eq!(edit.text(), "2:05.5");
        assert_eq!(CLOCK.parse(edit.text()), Some(1255));

        let mut plain = NumberEdit::over(&PERCENT, 40);
        assert!(!plain.is_clock());
        plain.key(&typed(":"));
        assert_eq!((plain.text(), plain.is_all_selected()), ("40", true), "a colon typed over a number that is no length of time replaced it");
        plain.key(&typed("1"));
        plain.key(&typed(":"));
        assert_eq!(plain.text(), "1");

        let mut input: NumberInput<usize> = NumberInput::new();
        assert_eq!(input.edit_hint(), None);
        input.begin_edit(0, &NumberValue::of(CLOCK, 834, 0));
        assert_eq!(input.edit_hint(), Some(EDIT_HINT_CLOCK));
        assert_eq!(input.edit_key(&typed("9")), Some(EditKey::Taken));
        assert_eq!(input.edit_key(&typed("0")), Some(EditKey::Taken));
        let (row, text) = input.take_edit().expect("a row was being typed into");
        assert_eq!(input.typed_steps(row, &CLOCK, &text, Instant::now()), Some(900), "seconds alone are not a length of time");
        input.begin_edit(1, &value(40));
        assert_eq!(input.edit_hint(), Some(EDIT_HINT));
    }

    /// A colour is typed in hex, with its `#` or without, in either case; anything else typed is
    /// dropped as it is typed, and what is no colour is refused for a while as a number is.
    #[test]
    fn a_colour_is_typed_in_hex_and_what_is_no_colour_is_refused() {
        let now = Instant::now();
        let mut input: NumberInput<usize> = NumberInput::new();
        input.begin_hex_edit(2, "#30E050FF");
        assert_eq!(input.edit_hint(), Some(EDIT_HINT_HEX));
        let edit = input.edit_of(2).expect("the colour row is being typed into");
        assert_eq!((edit.text(), edit.is_all_selected(), edit.is_hex()), ("#30E050FF", true, true));
        for text in ["#", "a", "b", "g", "-", " ", "c", "1", "2", "3"] {
            assert_eq!(input.edit_key(&typed(text)), Some(EditKey::Taken));
        }
        let (row, text) = input.take_edit().expect("a row was being typed into");
        assert_eq!((row, text.as_str()), (2, "#abc123"));
        assert_eq!(input.typed_color(row, &text, now), Some(SkinColor::rgba(0xAB, 0xC1, 0x23, u8::MAX)));
        assert!(!input.is_refused(2, now));
        assert_eq!(input.typed_color(row, "0000007f", now), Some(SkinColor::rgba(0, 0, 0, 0x7F)), "eight digits are not read with their alpha");
        assert_eq!(input.typed_color(row, "#12345", now), None);
        assert!(input.is_refused(2, now) && !input.is_refused(2, now + REFUSAL_SHOWN_FOR), "a figure that is no colour is not said to be");

        let mut number = NumberEdit::over(&PERCENT, 40);
        number.key(&typed("f"));
        assert_eq!(number.text(), "40", "a hex digit was typed into a number");
        assert!(!number.is_hex());
    }

    #[test]
    fn the_keys_that_end_an_edit_say_how() {
        let mut edit = NumberEdit::over(&PERCENT, 40);
        assert_eq!(edit.key(&press(KeyCode::Enter)), EditKey::Commit);
        assert_eq!(edit.key(&press(KeyCode::NumpadEnter)), EditKey::Commit);
        assert_eq!(edit.key(&press(KeyCode::Escape)), EditKey::Cancel);
        for code in [KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::PageUp, KeyCode::PageDown, KeyCode::Tab] {
            assert_eq!(edit.key(&press(code)), EditKey::CommitThenPass, "{code:?}");
        }
        assert_eq!(edit.key(&press(KeyCode::F8)), EditKey::Taken, "a key that is nothing to a number is kept from the screen behind");
        assert_eq!(edit.text(), "40", "a key that ends the edit changed what was typed");
    }

    #[test]
    fn a_refused_figure_is_said_for_a_while_and_a_locked_row_is_not_opened() {
        let now = Instant::now();
        let mut input: NumberInput<usize> = NumberInput::new();
        input.begin_edit(3, &NumberValue { locked: true, ..value(40) });
        assert_eq!(input.editing(), None);
        input.begin_edit(3, &value(40));
        assert_eq!(input.editing(), Some(3));
        assert!(input.edit_of(3).is_some() && input.edit_of(4).is_none());
        assert_eq!(input.edit_key(&typed("-")), Some(EditKey::Taken));
        let (row, text) = input.take_edit().expect("a row was being typed into");
        assert_eq!((row, text.as_str()), (3, "-"));
        assert_eq!(input.editing(), None);
        assert_eq!(input.typed_steps(row, &PERCENT, &text, now), None);
        assert!(input.is_refused(3, now) && !input.is_refused(4, now));
        assert!(!input.is_refused(3, now + REFUSAL_SHOWN_FOR), "the row went on saying so");
        assert_eq!(input.typed_steps(row, &PERCENT, "250%", now), Some(100));
        assert_eq!(input.edit_key(&typed("1")), None, "a key reached an editor that is not open");
    }

    #[test]
    fn one_row_at_a_time_is_dragged() {
        let mut input: NumberInput<usize> = NumberInput::new();
        assert!(!input.end_drag());
        assert_eq!(input.dragged_to(&value(40), AREA.x), None, "a pointer carried a row nobody took hold of");
        assert!(!input.begin_drag(2, AREA, &value(100), AREA.x + AREA.w * 0.5), "a press off the knob took hold of it");
        assert_eq!(input.dragged(), Some((2, AREA)));
        assert!(input.is_dragging(2) && !input.is_dragging(1));
        assert_eq!(input.dragged_to(&value(100), AREA.x + AREA.w * 0.5), Some(50), "a press off the knob does not bring the knob under it");
        assert!(input.end_drag());
        assert_eq!(input.dragged(), None);
    }

    /// The knob is carried by where it was taken hold of: a press anywhere across it leaves the row
    /// on its step, and the row then moves by as far as the pointer does.
    #[test]
    fn a_knob_is_carried_by_where_it_was_taken_hold_of() {
        const PULLED: f32 = 30.0;
        for spec in [PERCENT, PLACE] {
            for steps in [spec.min, (spec.min + spec.max) / 3, spec.max] {
                let held = NumberValue::of(spec, steps, 0);
                let knob = knob_rect(AREA, held.fraction());
                let middle = knob.x + knob.w * 0.5;
                for across in [knob.x, knob.x + 1.0, middle, knob.x + knob.w - 1.0] {
                    let mut input: NumberInput<usize> = NumberInput::new();
                    assert!(input.begin_drag(0, AREA, &held, across), "{spec:?} at {steps}: a press at {across} is not on the knob at {}", knob.x);
                    assert_eq!(input.dragged_to(&held, across), Some(steps), "{spec:?}: the knob moved under a pointer that did not");
                    for pulled in [PULLED, -PULLED] {
                        assert_eq!(input.dragged_to(&held, across + pulled), Some(steps_at(&spec, AREA, middle + pulled)), "{spec:?} at {steps}");
                    }
                }
                let mut input: NumberInput<usize> = NumberInput::new();
                assert!(!input.begin_drag(0, AREA, &held, knob.x + knob.w), "the knob is taken hold of from past its right edge");
                assert!(!input.begin_drag(0, AREA, &held, knob.x - 1.0), "the knob is taken hold of from before its left edge");
            }
        }
    }

    /// A row whose slider ends short of its upper end is dragged and stepped as far as the slider
    /// goes, stands at the slider's end when a typed figure put it further, and is stepped from
    /// there either way.
    #[test]
    fn a_slider_that_ends_short_of_its_row_is_as_far_as_a_drag_and_an_arrow_go() {
        const WIDE: NumberSpec = NumberSpec::whole(0, 200, "%");
        let at = |steps| NumberValue::of(WIDE, steps, 100).with_slider_to(100);
        assert_eq!((at(50).fraction(), at(100).fraction(), at(150).fraction()), (0.5, 1.0, 1.0));
        assert_eq!(steps_at(&at(50).track(), AREA, AREA.x + AREA.w), 100, "the end of the slider is past where the slider ends");
        assert_eq!((at(99).stepped(1, false), at(100).stepped(1, false), at(95).stepped(1, true)), (100, 100, 100), "an arrow went past the slider's end");
        assert_eq!((at(100).stepped(-1, false), at(100).stepped(-1, true)), (99, 90));
        assert_eq!(
            (at(150).stepped(1, false), at(150).stepped(-1, false), at(195).stepped(1, true)),
            (151, 149, 200),
            "a row past its slider is not stepped from where it stands"
        );
        assert_eq!(at(104).stepped(-1, true), 94, "a row past its slider does not come back onto it");
        assert_eq!(WIDE.parse("150"), Some(150), "a typed figure does not reach past the slider");

        let mut input: NumberInput<usize> = NumberInput::new();
        let knob = knob_rect(AREA, at(150).fraction());
        assert!(input.begin_drag(0, AREA, &at(150), knob.x + 1.0), "the knob of a row past its slider is not at the slider's end");
        assert_eq!(input.dragged_to(&at(150), knob.x + 1.0 - AREA.w), Some(0));
        assert_eq!(NumberValue::of(WIDE, 150, 100).with_slider_to(900).slider_max, 200, "a slider ends past its row");
        assert_eq!(value(40).track(), PERCENT, "a row whose slider reaches its end runs over other steps than its own");
    }
}
