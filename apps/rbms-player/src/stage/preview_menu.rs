//! The parts of a menu that lists settings on its left and shows what they do on its right.
//!
//! Two screens are laid out this way: the skin menu ([`crate::stage::SkinMenuState`]) and the play
//! menu ([`crate::stage::PlayMenuState`]). What is here is what the two draw and move the same way,
//! so that they stay the same screen with different rows: where the title, the strip of tabs, the
//! list, the preview, the panel under it and the line of keys go, how a row of the list and the
//! scroll bar beside it are drawn, how the list is scrolled, what is written in the preview's place
//! when no frame of a skin is there, and how a value too long for its place is broken over lines.
//!
//! What is not here is what a row is. A menu builds its own rows, steps them through its own calls
//! and says what each is for; it hands this module the words to write.
//!
//! A row that holds a number is drawn with a slider before its number ([`ListEntry::number`]), and
//! a press on such a row is told by where on the row it landed ([`number_part_at`]): on the slider
//! it drags the row, and on the number it opens the row to be typed into. How a row is counted,
//! dragged and typed into is not this module's either ([`crate::number_row`]); it says where on a
//! row the slider and the number go.
//!
//! A row of a colour is drawn with a square of the colour it shows before its value
//! ([`ListEntry::color`]), and is typed into in hex where its value is.
//!
//! Everything is laid out in the fixed space every built-in screen is laid out in
//! ([`crate::stage::canvas::UI_SIZE`]).

use rbms_model::translations::Catalog;
use rbms_render::text::language;
use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_centered, draw_text_right, fit_text, text_width, theme};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::number_row::{NumberEdit, RowPart, SliderLook, draw_edit, draw_refusal, draw_refusal_note, draw_slider, hot_rect, row_part};
pub(crate) use crate::number_row::{ONE_STEP, SHIFT_STEPS};
use crate::skin_preview::region::with_region;
use crate::skin_preview::{PreviewStatus, SkinPreview};
use crate::stage::{Canvas, FrameCtx, KeyInput};
use crate::text::menus::preview_menu::PreviewMenuText;
use crate::text::settings::screen::NumberRowText;
use crate::{AppShared, CW, Hot};

/// The room kept between the screen's edges and everything on it.
pub(crate) const MARGIN: f32 = 24.0;

/// The gap between the list on the left and the preview on the right.
pub(crate) const COLUMN_GAP: f32 = 16.0;

/// Where the title is written, and how large.
pub(crate) const TITLE_Y: f32 = 16.0;
pub(crate) const TITLE_SCALE: f32 = 2.4;

/// Where what is written beside the title begins, and how large it is written.
const SUBTITLE_X: f32 = 132.0;
pub(crate) const SUBTITLE_Y: f32 = 26.0;
pub(crate) const SUBTITLE_SCALE: f32 = 1.6;

/// The top of the two columns, under the head.
pub(crate) const COLUMNS_TOP: f32 = 84.0;

/// The preview: as large as the screen leaves room for at the shape of a screen, against the right
/// edge. Sixteen by nine, on whole pixels at the size the screen is laid out for.
const PREVIEW_W: f32 = 832.0;
const PREVIEW_H: f32 = 468.0;

/// Where the preview goes, in the space the menu is laid out in.
pub(crate) const PREVIEW_RECT: Rect = Rect { x: CW as f32 - MARGIN - PREVIEW_W, y: COLUMNS_TOP, w: PREVIEW_W, h: PREVIEW_H };

/// The list of settings: from the left margin to the gap before the preview, and down to the hint.
pub(crate) const LIST_RECT: Rect = Rect { x: MARGIN, y: COLUMNS_TOP, w: PREVIEW_RECT.x - COLUMN_GAP - MARGIN, h: HINT_Y - HINT_GAP - COLUMNS_TOP };

/// What is said under the preview: about what is being previewed and the row under the cursor.
pub(crate) const INFO_RECT: Rect =
    Rect { x: PREVIEW_RECT.x, y: COLUMNS_TOP + PREVIEW_H + INFO_GAP, w: PREVIEW_W, h: HINT_Y - HINT_GAP - (COLUMNS_TOP + PREVIEW_H + INFO_GAP) };

/// The gap between the preview and what is said under it.
const INFO_GAP: f32 = 12.0;

/// Where the line of keys is written at the foot of the screen, how large, and the room above it.
pub(crate) const HINT_Y: f32 = 688.0;
pub(crate) const HINT_SCALE: f32 = 1.2;
pub(crate) const HINT_GAP: f32 = 12.0;

/// How thick the line round the preview is.
pub(crate) const PREVIEW_BORDER: f32 = 2.0;

/// The status written in the preview's place: how large its heading and what follows it are, and
/// how far the heading is above the first line of what follows.
pub(crate) const STATUS_SCALE: f32 = 1.8;
pub(crate) const STATUS_DETAIL_SCALE: f32 = 1.2;
const STATUS_LINE_GAP: f32 = 36.0;

/// How many lines what follows the heading is written over -- a reason a skin did not load is a
/// sentence with a file's name in it -- and how far one of them is from the next.
pub(crate) const STATUS_DETAIL_LINES: usize = 3;
pub(crate) const STATUS_DETAIL_PITCH: f32 = 18.0;

/// How far the detail's text is kept from the preview's own edges.
pub(crate) const STATUS_PAD: f32 = 24.0;

/// The strip of tabs above the preview: where it is, how tall, and the gap between two tabs. The
/// tabs share its width evenly.
pub(crate) const TAB_STRIP_Y: f32 = 24.0;
pub(crate) const TAB_H: f32 = 32.0;
const TAB_GAP: f32 = 6.0;

/// How large a tab's name is written, and how far below the tab's top.
pub(crate) const TAB_SCALE: f32 = 1.5;
const TAB_TEXT_Y: f32 = 8.0;

/// The rows of the list: how far they are kept from the list's own edges, how far one is from the
/// next, how tall one is, and how many are on show at once.
const LIST_PAD: f32 = 4.0;
const ROW_PITCH: f32 = 33.0;
const ROW_H: f32 = 29.0;
pub(crate) const VISIBLE_ROWS: usize = 17;

/// How far a row's label and value are kept from the row's own edges, and how far below its top
/// each is written.
pub(crate) const ROW_PAD: f32 = 10.0;
const LABEL_TEXT_Y: f32 = 9.0;
const VALUE_TEXT_Y: f32 = 8.0;

/// How large a row's label and its value are written.
pub(crate) const LABEL_SCALE: f32 = 1.3;
pub(crate) const VALUE_SCALE: f32 = 1.4;

/// The widest a row's value is drawn, as a share of the row, and the room kept between the label and
/// it. The label has what the value leaves.
pub(crate) const VALUE_MAX_SHARE: f32 = 0.55;
pub(crate) const LABEL_GAP: f32 = 12.0;

/// A row that holds a number: how wide the room its number is written in is, how wide the slider
/// before it, and the gap between the two. The number has a room of its own, whatever it reads, so
/// the slider stands in the same place on every row and does not move under a pointer dragging it
/// when the number grows a digit.
const NUMBER_VALUE_W: f32 = 58.0;
const SLIDER_W: f32 = 100.0;
const SLIDER_GAP: f32 = 10.0;

/// The square a colour row shows its colour in before its value: how large it is, how far it is from
/// the value, and how thick the line round it is, which keeps a dark colour apart from the row.
const SWATCH_SIZE: f32 = 16.0;
const SWATCH_GAP: f32 = 8.0;
const SWATCH_BORDER: f32 = 1.0;

/// The most bytes one character is written in, which is the buffer a character is measured from.
pub(crate) const UTF8_MAX_BYTES: usize = 4;

/// The scroll bar along the list's right edge: how wide it is, how far inside the edge, and the
/// shortest its thumb is drawn.
pub(crate) const SCROLL_BAR_W: f32 = 5.0;
pub(crate) const SCROLL_BAR_INSET: f32 = 3.0;
const SCROLL_THUMB_MIN_H: f32 = 16.0;

/// The room the scroll bar takes from the rows.
pub(crate) const SCROLL_BAR_ROOM: f32 = SCROLL_BAR_W + SCROLL_BAR_INSET * 2.0;

/// Where the cursor's place in the list is written, measured up from the list's foot, and how large.
pub(crate) const COUNTER_RISE: f32 = 20.0;
const COUNTER_SCALE: f32 = 1.2;

/// What is said under the preview: how far it is kept from the panel's edges, how large it is
/// written, how far one line is from the next, and where what leads the panel and the row under the
/// cursor begin, measured down from the panel's top.
pub(crate) const INFO_PAD: f32 = 16.0;
pub(crate) const INFO_SCALE: f32 = 1.3;
pub(crate) const INFO_LINE_PITCH: f32 = 17.0;
pub(crate) const INFO_LEAD_Y: f32 = 8.0;
pub(crate) const INFO_ROW_Y: f32 = 46.0;

/// How many lines what leads the panel is written over, and how many the value of the row under the
/// cursor is when it does not fit beside its label.
pub(crate) const INFO_LEAD_LINES: usize = 2;
pub(crate) const INFO_VALUE_LINES: usize = 2;

/// How tall a line of what is said under the preview is drawn, which is the room its last line
/// needs above the panel's foot.
pub(crate) const INFO_LINE_HEIGHT: f32 = 14.0;

/// The room kept between the heading of a line under the preview and what follows it.
const INFO_HEADING_GAP: f32 = 12.0;

/// The figures a panel can be led with ([`draw_info_figures`]): how far below the panel's top the
/// name of one and the figure itself are written, how large each is, and how large what is written
/// after a figure is.
const FIGURE_NAME_Y: f32 = 8.0;
const FIGURE_VALUE_Y: f32 = 21.0;
const FIGURE_NAME_SCALE: f32 = 1.1;
const FIGURE_VALUE_SCALE: f32 = 1.6;
const FIGURE_TAG_SCALE: f32 = 1.1;

/// How far what is written after a figure is from it, and how far below the top of the figure, so
/// the two stand on one line.
const FIGURE_TAG_GAP: f32 = 6.0;
const FIGURE_TAG_DROP: f32 = 5.0;

/// Where what is said of the figures as a whole is written under them, and how large.
const FIGURE_NOTE_Y: f32 = 39.0;
pub(crate) const FIGURE_NOTE_SCALE: f32 = 1.1;

/// Where the row under the cursor begins on a panel led with figures, which are taller than a line
/// of words.
pub(crate) const INFO_ROW_UNDER_FIGURES_Y: f32 = 54.0;

/// One step to the right on a row, which is what the right arrow, Enter and a press of the left
/// button all ask for, and one step to the left, which the left arrow and a press of the right
/// button ask for.
pub(crate) const ROW_STEP_FORWARD: i32 = 1;
pub(crate) const ROW_STEP_BACK: i32 = -1;

/// What a row that shows or hides other rows reads: whether its rows are on show, and how many they
/// are.
pub(crate) fn fold_value(open: bool, rows: usize) -> String {
    let shown = if open { PreviewMenuText::FoldShown } else { PreviewMenuText::FoldHidden };
    format!("{} ({rows})", shown.get(language()))
}

/// One step along the strip of tabs: forwards, and back.
pub(crate) const NEXT_TAB: i32 = 1;
pub(crate) const PREVIOUS_TAB: i32 = -1;

/// The most rows one turn of the wheel moves the list by, whatever the device reports.
const WHEEL_ROWS_MAX: f32 = 64.0;

/// How one entry of the list is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowLook {
    /// A row with a value that is changed here.
    Plain,
    /// A row whose value is worked out from the others and shown for what it is: the whole row is
    /// written fainter than one that can be changed, and stepping it does nothing.
    ReadOnly,
    /// The name of the group the rows under it belong to. It has no value, the cursor's light is
    /// never on it and a press on it lands on nothing.
    Heading,
}

/// One entry of the list as it is written: its label, its value already cut to the room it has
/// ([`fitted_value`]), how it is drawn, and its slider when it holds a number.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ListEntry<'a> {
    pub(crate) label: &'a str,
    pub(crate) value: String,
    pub(crate) look: RowLook,
    /// The slider of a row that holds a number, or `None` for a row that holds none. The value of
    /// such a row is written in the room a number has ([`number_value_room`]).
    pub(crate) number: Option<NumberCell<'a>>,
    /// The colour of a colour row, or `None` for a row that is no colour's.
    pub(crate) color: Option<ColorCell<'a>>,
}

/// What is drawn on a colour row besides its label and its value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColorCell<'a> {
    /// The colour the row shows, which its square is filled with.
    pub(crate) color: Color,
    /// The colour being typed into the row, which is drawn in place of the square and the value.
    pub(crate) edit: Option<&'a NumberEdit>,
    /// Whether the row is saying that what was typed into it was not a colour.
    pub(crate) refused: bool,
}

/// What is drawn on a row that holds a number besides its label and its value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NumberCell<'a> {
    /// How far along its slider the row stands, from nothing to one.
    pub(crate) fraction: f32,
    /// Whether the row is being dragged, and so drawn as one that is.
    pub(crate) dragged: bool,
    /// The figure being typed into the row, which is drawn in place of the slider and the number.
    pub(crate) edit: Option<&'a NumberEdit>,
    /// Whether the row is saying that what was typed into it was not a number.
    pub(crate) refused: bool,
}

/// One figure of the row a panel can be led with: what it is called, the figure and the colour it
/// is written in, and what is written small after it -- its unit, or what state it is in -- in a
/// colour of its own.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InfoFigure {
    pub(crate) name: &'static str,
    pub(crate) value: String,
    pub(crate) color: Color,
    pub(crate) tag: &'static str,
    pub(crate) tag_color: Color,
}

/// Where a list stands: how many entries it has, which the cursor is on, and which is the first on
/// show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ListWindow {
    pub(crate) len: usize,
    pub(crate) sel: usize,
    pub(crate) top: usize,
}

/// How wide one tab of a strip of `count` is: the tabs share the preview's width evenly.
pub(crate) const fn tab_width(count: usize) -> f32 {
    if count == 0 {
        return PREVIEW_W;
    }
    (PREVIEW_W - TAB_GAP * (count - 1) as f32) / count as f32
}

/// Where the tab at one place of a strip of `count` is drawn.
pub(crate) fn tab_rect(at: usize, count: usize) -> Rect {
    let width = tab_width(count);
    Rect::new(PREVIEW_RECT.x + at as f32 * (width + TAB_GAP), TAB_STRIP_Y, width, TAB_H)
}

/// The tab one step along a strip of `count` from `chosen`, round its ends. From a place the strip
/// does not have, either direction lands on the strip's first.
pub(crate) fn stepped_tab(chosen: Option<usize>, count: usize, delta: i32) -> usize {
    let Ok(count) = i32::try_from(count) else {
        return 0;
    };
    if count == 0 {
        return 0;
    }
    chosen.and_then(|at| i32::try_from(at).ok()).map_or(0, |at| (at + delta).rem_euclid(count) as usize)
}

/// Where the row in one slot of the list is drawn, the first slot being the top one on show.
pub(crate) fn row_rect(slot: usize) -> Rect {
    Rect::new(LIST_RECT.x + LIST_PAD, LIST_RECT.y + LIST_PAD + slot as f32 * ROW_PITCH, LIST_RECT.w - LIST_PAD - SCROLL_BAR_ROOM, ROW_H)
}

/// The room a row has for its label and its value together.
pub(crate) fn row_room() -> f32 {
    row_rect(0).w - ROW_PAD * 2.0
}

/// A row's value cut to the room a value has, at its end.
pub(crate) fn fitted_value(value: &str) -> String {
    fit_text(value, VALUE_SCALE, row_room() * VALUE_MAX_SHARE)
}

/// The room the number of a row that holds one is written in.
#[cfg(test)]
pub(crate) const fn number_value_room() -> f32 {
    NUMBER_VALUE_W
}

/// Where the number of a row that holds one is written, on the row drawn at `row`: against the
/// row's right edge. A press there opens the row to be typed into.
pub(crate) fn number_value_rect(row: Rect) -> Rect {
    Rect::new(row.x + row.w - ROW_PAD - NUMBER_VALUE_W, row.y, NUMBER_VALUE_W + ROW_PAD, row.h)
}

/// Where the slider of a row that holds a number is drawn, on the row drawn at `row`: before the
/// number, as tall as the row, so a press anywhere over it takes hold of it.
pub(crate) fn slider_rect(row: Rect) -> Rect {
    Rect::new(row.x + row.w - ROW_PAD - NUMBER_VALUE_W - SLIDER_GAP - SLIDER_W, row.y, SLIDER_W, row.h)
}

/// Where a figure being typed into a row is drawn, on the row drawn at `row`: over the slider and
/// the number both.
pub(crate) fn number_edit_rect(row: Rect) -> Rect {
    let slider = slider_rect(row);
    Rect::new(slider.x, row.y, row.x + row.w - ROW_PAD * 0.5 - slider.x, row.h)
}

/// Which part of a row that holds a number a press at `at` landed on, the row being what the last
/// frame made clickable as `hot`.
pub(crate) fn number_part_at(shared: &AppShared, hot: Hot, at: (f32, f32)) -> RowPart {
    hot_rect(shared, hot).map_or(RowPart::Rest, |row| row_part(slider_rect(row), number_value_rect(row), at))
}

/// The room the label of a row that holds a number has: what its slider leaves.
pub(crate) fn number_label_room() -> f32 {
    slider_rect(row_rect(0)).x - LABEL_GAP - (row_rect(0).x + ROW_PAD)
}

/// The whole length the scroll bar's thumb travels, which is drawn under it so the thumb's own
/// length reads against something.
pub(crate) fn scroll_track() -> Rect {
    Rect::new(LIST_RECT.x + LIST_RECT.w - SCROLL_BAR_INSET - SCROLL_BAR_W, LIST_RECT.y + LIST_PAD, SCROLL_BAR_W, VISIBLE_ROWS as f32 * ROW_PITCH)
}

/// Where the scroll bar's thumb is for a list of `len` rows whose first row on show is `top`: as
/// much of the bar as the rows on show are of the list, and as far down it as the list is scrolled.
pub(crate) fn scroll_thumb(top: usize, len: usize) -> Rect {
    scroll_thumb_in(scroll_track(), top, len, VISIBLE_ROWS)
}

/// Where the thumb of a scroll bar drawn along `track` is, for a list of `len` entries that shows
/// `visible` of them at once and whose first entry on show is `top`: as much of the track as the
/// entries on show are of the list, and as far down it as the list is scrolled.
pub(crate) fn scroll_thumb_in(track: Rect, top: usize, len: usize, visible: usize) -> Rect {
    let rows = len.max(1) as f32;
    let height = (track.h * visible as f32 / rows).clamp(SCROLL_THUMB_MIN_H.min(track.h), track.h);
    let travel = len.saturating_sub(visible).max(1) as f32;
    Rect::new(track.x, track.y + (track.h - height) * (top as f32 / travel).min(1.0), track.w, height)
}

/// Whether a point of the screen is inside a rectangle of it.
pub(crate) fn contains(rect: Rect, at: (f32, f32)) -> bool {
    at.0 >= rect.x && at.0 < rect.x + rect.w && at.1 >= rect.y && at.1 < rect.y + rect.h
}

/// The furthest a list of `len` rows can be scrolled: the last screenful.
pub(crate) fn last_top(len: usize) -> usize {
    last_top_of(len, VISIBLE_ROWS)
}

/// The furthest a list of `len` entries that shows `visible` of them at once can be scrolled.
pub(crate) fn last_top_of(len: usize, visible: usize) -> usize {
    len.saturating_sub(visible)
}

/// The first row on show once a list of `len` rows scrolled to `top` has been scrolled just far
/// enough to have the row `sel` on show.
pub(crate) fn revealed_top(sel: usize, top: usize, len: usize) -> usize {
    revealed_top_of(sel, top, len, VISIBLE_ROWS)
}

/// [`revealed_top`] for a list that shows `visible` entries at once.
pub(crate) fn revealed_top_of(sel: usize, top: usize, len: usize, visible: usize) -> usize {
    let top = if sel < top {
        sel
    } else if sel >= top + visible {
        sel + 1 - visible
    } else {
        top
    };
    top.min(last_top_of(len, visible))
}

/// The first row on show once the wheel has turned by `lines` over a list of `len` rows scrolled to
/// `top`. `carry` is the part of a line the wheel has turned that has not moved the list yet, which
/// is kept from one turn to the next. A turn that is not a number is the caller's to leave out.
pub(crate) fn wheel_scrolled(carry: &mut f32, top: usize, len: usize, lines: f32) -> usize {
    wheel_scrolled_over(carry, top, len, lines, VISIBLE_ROWS)
}

/// [`wheel_scrolled`] for a list that shows `visible` entries at once.
pub(crate) fn wheel_scrolled_over(carry: &mut f32, top: usize, len: usize, lines: f32, visible: usize) -> usize {
    *carry = (*carry + lines).clamp(-WHEEL_ROWS_MAX, WHEEL_ROWS_MAX);
    let whole = carry.trunc();
    *carry -= whole;
    if whole < 0.0 { top.saturating_sub((-whole) as usize) } else { (top + whole as usize).min(last_top_of(len, visible)) }
}

/// Which way a press of a mouse button steps the row it lands on: forwards with the left button,
/// and back with the right, so a step taken by mistake can be taken back with the mouse. `None` for
/// a button a menu does nothing on.
pub(crate) fn click_step(button: MouseButton) -> Option<i32> {
    match button {
        MouseButton::Left => Some(ROW_STEP_FORWARD),
        MouseButton::Right => Some(ROW_STEP_BACK),
        _ => None,
    }
}

/// Whether Shift is held once `key` has been taken in, given whether it was held before, or `None`
/// for a key that is no Shift.
pub(crate) fn shift_after(held: bool, key: &KeyInput<'_>) -> Option<bool> {
    matches!(key.code, KeyCode::ShiftLeft | KeyCode::ShiftRight).then_some(key.pressed || (held && !key.released))
}

/// `text` broken into at most `lines` lines no wider than `width` at `scale`: at a space where a line
/// has one, and anywhere where it has none. What does not fit the last line is cut short.
///
/// A break `text` brings with it is not one of them: a line feed, a tab or any other control
/// character is read as a space, so what is written never runs to more lines than it was given.
pub(crate) fn wrapped(text: &str, scale: f32, width: f32, lines: usize) -> Vec<String> {
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

/// What is written under a status's heading in the preview's place, broken over the lines it is
/// given.
pub(crate) fn status_detail_lines(status: &PreviewStatus) -> Vec<String> {
    status.detail().map_or_else(Vec::new, |detail| wrapped(detail, STATUS_DETAIL_SCALE, PREVIEW_RECT.w - STATUS_PAD * 2.0, STATUS_DETAIL_LINES))
}

/// Draw the head: the menu's title, and beside it what the menu is showing -- the screen being
/// configured, the mode being previewed -- cut to the room over the list.
pub(crate) fn draw_head(canvas: &mut Canvas<'_>, title: &str, subtitle: &str) {
    let colors = theme();
    draw_text(canvas, MARGIN, TITLE_Y, TITLE_SCALE, colors.text, title);
    draw_text(canvas, SUBTITLE_X, SUBTITLE_Y, SUBTITLE_SCALE, colors.text_dim, &head_subtitle(subtitle));
}

/// What is written beside the title for `subtitle`: all of it, or as much as the room over the list
/// holds.
pub(crate) fn head_subtitle(subtitle: &str) -> String {
    fit_text(subtitle, SUBTITLE_SCALE, LIST_RECT.x + LIST_RECT.w - SUBTITLE_X)
}

/// Draw the strip of tabs over the preview, the chosen one lit, and record each tab as clickable
/// under what `hot` makes of its place on the strip.
pub(crate) fn draw_tabs<'a>(
    shared: &mut AppShared,
    canvas: &mut Canvas<'_>,
    names: impl ExactSizeIterator<Item = &'a str>,
    chosen: Option<usize>,
    hot: fn(usize) -> Hot,
) {
    let colors = theme();
    let count = names.len();
    for (at, name) in names.enumerate() {
        let on = chosen == Some(at);
        let rect = tab_rect(at, count);
        canvas.fill_rect(rect, if on { colors.button } else { colors.panel });
        draw_text_centered(canvas, rect.x + rect.w * 0.5, rect.y + TAB_TEXT_Y, TAB_SCALE, if on { colors.title_focus } else { colors.text_dim }, name);
        shared.hot.push((rect, hot(at)));
    }
}

/// Draw the entries of a list that are on show, the cursor's lit, with the scroll bar and the
/// cursor's place in the list when there are more entries than fit, and record each drawn row as
/// clickable under what `hot` makes of its place in the whole list.
///
/// `entry` is asked for the entries on show and no others. A value is written as it is handed over:
/// cutting it to its room is the menu's ([`fitted_value`]), because a menu may have a row whose
/// value is cut another way. A label is cut to what its value leaves.
///
/// A row that holds a number has its slider before its number, each in a room of its own, and the
/// label has what the slider leaves. The row is clickable as any row is; which part of it a press
/// landed on is told from where the row was drawn ([`number_part_at`]). While the row is being
/// typed into, the figure typed is drawn where the slider and the number were.
///
/// A colour row has a square of its colour before its value, and the label has what the two leave.
/// While a colour is typed into it, the colour typed is drawn where a number's would be.
pub(crate) fn draw_list<'a>(shared: &mut AppShared, canvas: &mut Canvas<'_>, list: ListWindow, entry: impl Fn(usize) -> ListEntry<'a>, hot: fn(usize) -> Hot) {
    let colors = theme();
    canvas.fill_rect(LIST_RECT, colors.panel);
    let shown = list.top..(list.top + VISIBLE_ROWS).min(list.len);
    for (slot, index) in shown.enumerate() {
        let ListEntry { label, value, look, number, color } = entry(index);
        let rect = row_rect(slot);
        if look == RowLook::Heading {
            draw_text(canvas, rect.x + ROW_PAD, rect.y + LABEL_TEXT_Y, LABEL_SCALE, colors.accent, &fit_text(label, LABEL_SCALE, row_room()));
            continue;
        }
        let on = index == list.sel;
        if on {
            canvas.fill_rect(rect, colors.button);
        }
        let (label_color, value_color) = match (look, on) {
            (RowLook::ReadOnly, true) => (colors.text_dim, colors.text_dim),
            (RowLook::ReadOnly, false) => (colors.text_muted, colors.text_muted),
            (_, true) => (colors.text, colors.title_focus),
            (_, false) => (colors.text_dim, colors.text),
        };
        if let Some(cell) = color {
            draw_color_row(canvas, rect, label, &value, (label_color, value_color), cell);
            shared.hot.push((rect, hot(index)));
            continue;
        }
        let Some(cell) = number else {
            let label = fit_text(label, LABEL_SCALE, row_room() - text_width(&value, VALUE_SCALE) - LABEL_GAP);
            draw_text(canvas, rect.x + ROW_PAD, rect.y + LABEL_TEXT_Y, LABEL_SCALE, label_color, &label);
            draw_text_right(canvas, rect.x + rect.w - ROW_PAD, rect.y + VALUE_TEXT_Y, VALUE_SCALE, value_color, &value);
            shared.hot.push((rect, hot(index)));
            continue;
        };
        draw_text(canvas, rect.x + ROW_PAD, rect.y + LABEL_TEXT_Y, LABEL_SCALE, label_color, &fit_text(label, LABEL_SCALE, number_label_room()));
        let slider = slider_rect(rect);
        if let Some(edit) = cell.edit {
            draw_edit(canvas, number_edit_rect(rect), rect.y + VALUE_TEXT_Y, VALUE_SCALE, edit);
        } else if cell.refused {
            draw_refusal(canvas, rect.x + rect.w - ROW_PAD, rect.y + VALUE_TEXT_Y, VALUE_SCALE);
        } else {
            let slider_look = match (look, cell.dragged, on) {
                (RowLook::ReadOnly, _, _) => SliderLook::Locked,
                (_, true, _) => SliderLook::Dragged,
                (_, false, true) => SliderLook::Focused,
                (_, false, false) => SliderLook::Plain,
            };
            draw_slider(canvas, slider, cell.fraction, slider_look);
            draw_text_right(canvas, rect.x + rect.w - ROW_PAD, rect.y + VALUE_TEXT_Y, VALUE_SCALE, value_color, &fit_text(&value, VALUE_SCALE, NUMBER_VALUE_W));
        }
        shared.hot.push((rect, hot(index)));
    }
    if list.len <= VISIBLE_ROWS {
        return;
    }
    canvas.fill_rect(scroll_track(), colors.panel_hi);
    canvas.fill_rect(scroll_thumb(list.top, list.len), colors.text_muted);
    let counter = format!("{} / {}", list.sel + 1, list.len);
    draw_text_right(canvas, LIST_RECT.x + LIST_RECT.w - ROW_PAD, LIST_RECT.y + LIST_RECT.h - COUNTER_RISE, COUNTER_SCALE, colors.text_muted, &counter);
}

/// Draw a colour row at `rect`: its label, and the colour typed into it, the words that refuse what
/// was, or a square of its colour before its value. `colors` are the label's and the value's.
fn draw_color_row(canvas: &mut Canvas<'_>, rect: Rect, label: &str, value: &str, colors: (Color, Color), cell: ColorCell<'_>) {
    let (label_color, value_color) = colors;
    let right = rect.x + rect.w - ROW_PAD;
    if cell.edit.is_some() || cell.refused {
        draw_text(canvas, rect.x + ROW_PAD, rect.y + LABEL_TEXT_Y, LABEL_SCALE, label_color, &fit_text(label, LABEL_SCALE, number_label_room()));
        match cell.edit {
            Some(edit) => draw_edit(canvas, number_edit_rect(rect), rect.y + VALUE_TEXT_Y, VALUE_SCALE, edit),
            None => draw_refusal_note(canvas, right, rect.y + VALUE_TEXT_Y, VALUE_SCALE, NumberRowText::NotAColor.get(language())),
        }
        return;
    }
    let value_w = text_width(value, VALUE_SCALE);
    let square = swatch_rect(rect, value_w);
    let label = fit_text(label, LABEL_SCALE, square.x - LABEL_GAP - (rect.x + ROW_PAD));
    draw_text(canvas, rect.x + ROW_PAD, rect.y + LABEL_TEXT_Y, LABEL_SCALE, label_color, &label);
    canvas.fill_rect(square, theme().text_dim);
    let inner = Rect::new(square.x + SWATCH_BORDER, square.y + SWATCH_BORDER, square.w - SWATCH_BORDER * 2.0, square.h - SWATCH_BORDER * 2.0);
    canvas.fill_rect(inner, theme().bg);
    canvas.fill_rect(inner, cell.color);
    draw_text_right(canvas, right, rect.y + VALUE_TEXT_Y, VALUE_SCALE, value_color, value);
}

/// Where the square of a colour row drawn at `row` goes, before a value `value_w` wide written
/// against the row's right edge, in the middle of the row's height.
pub(crate) fn swatch_rect(row: Rect, value_w: f32) -> Rect {
    Rect::new(row.x + row.w - ROW_PAD - value_w - SWATCH_GAP - SWATCH_SIZE, row.y + ((row.h - SWATCH_SIZE) * 0.5).round(), SWATCH_SIZE, SWATCH_SIZE)
}

/// Fill the panel under the preview, before anything is written on it.
pub(crate) fn draw_info_panel(canvas: &mut Canvas<'_>) {
    canvas.fill_rect(INFO_RECT, theme().panel);
}

/// Write what leads the panel under the preview: a heading, and what is said under it in full, over
/// the lines it is given.
pub(crate) fn draw_info_lead(canvas: &mut Canvas<'_>, heading: &str, said: &str) {
    let colors = theme();
    let left = INFO_RECT.x + INFO_PAD;
    draw_text(canvas, left, INFO_RECT.y + INFO_LEAD_Y, INFO_SCALE, colors.text_muted, heading);
    let indent = text_width(heading, INFO_SCALE) + INFO_HEADING_GAP;
    for (at, line) in info_lead_lines(heading, said).iter().enumerate() {
        draw_text(canvas, left + indent, INFO_RECT.y + INFO_LEAD_Y + at as f32 * INFO_LINE_PITCH, INFO_SCALE, colors.text_dim, line);
    }
}

/// What is said under a heading at the top of the panel, broken over the lines [`draw_info_lead`]
/// writes it on beside that heading.
pub(crate) fn info_lead_lines(heading: &str, said: &str) -> Vec<String> {
    wrapped(said, INFO_SCALE, INFO_RECT.w - INFO_PAD * 2.0 - text_width(heading, INFO_SCALE) - INFO_HEADING_GAP, INFO_LEAD_LINES)
}

/// How far from the panel's left edge each of `count` figures begins: the figures share the panel's
/// width between its pads evenly.
pub(crate) fn figure_x(at: usize, count: usize) -> f32 {
    INFO_RECT.x + INFO_PAD + (INFO_RECT.w - INFO_PAD * 2.0) * at as f32 / count.max(1) as f32
}

/// Lead the panel under the preview with a row of figures, each under its name, and under the row
/// what is said of them as a whole when anything is.
///
/// A figure is written larger than the words round it and in a colour of its own, so the row is
/// read as numbers at a glance; what follows a figure is written small on the same line.
pub(crate) fn draw_info_figures(canvas: &mut Canvas<'_>, figures: &[InfoFigure], note: Option<&str>) {
    let colors = theme();
    for (at, figure) in figures.iter().enumerate() {
        let left = figure_x(at, figures.len());
        draw_text(canvas, left, INFO_RECT.y + FIGURE_NAME_Y, FIGURE_NAME_SCALE, colors.text_muted, figure.name);
        draw_text(canvas, left, INFO_RECT.y + FIGURE_VALUE_Y, FIGURE_VALUE_SCALE, figure.color, &figure.value);
        let after = left + text_width(&figure.value, FIGURE_VALUE_SCALE) + FIGURE_TAG_GAP;
        draw_text(canvas, after, INFO_RECT.y + FIGURE_VALUE_Y + FIGURE_TAG_DROP, FIGURE_TAG_SCALE, figure.tag_color, figure.tag);
    }
    if let Some(note) = note {
        let room = INFO_RECT.w - INFO_PAD * 2.0;
        draw_text(canvas, INFO_RECT.x + INFO_PAD, INFO_RECT.y + FIGURE_NOTE_Y, FIGURE_NOTE_SCALE, colors.warn, &fit_text(note, FIGURE_NOTE_SCALE, room));
    }
}

/// How wide one figure is with what is written after it, which is what has to fit before the next
/// figure begins.
#[cfg(test)]
pub(crate) fn figure_width(figure: &InfoFigure) -> f32 {
    let tag = if figure.tag.is_empty() { 0.0 } else { FIGURE_TAG_GAP + text_width(figure.tag, FIGURE_TAG_SCALE) };
    text_width(figure.name, FIGURE_NAME_SCALE).max(text_width(&figure.value, FIGURE_VALUE_SCALE) + tag)
}

/// Write one line where the row under the cursor is written, in its place: what the menu is waiting
/// for while it takes no key.
pub(crate) fn draw_info_note(canvas: &mut Canvas<'_>, note: &str) {
    draw_text(canvas, INFO_RECT.x + INFO_PAD, INFO_RECT.y + INFO_ROW_Y, INFO_SCALE, theme().accent, note);
}

/// Write the row under the cursor on the panel under the preview: its label and value in full and
/// what the row is for. A value that does not fit beside its label -- a folder's path -- is written
/// under it, over as many lines as it is given.
pub(crate) fn draw_info_row(canvas: &mut Canvas<'_>, label: &str, value: &str, help: &str) {
    draw_info_row_at(canvas, INFO_ROW_Y, label, value, help);
}

/// [`draw_info_row`] with the row begun `top` below the panel's top, for a panel whose lead is not
/// the two lines of words that one is laid out under.
pub(crate) fn draw_info_row_at(canvas: &mut Canvas<'_>, top: f32, label: &str, value: &str, help: &str) {
    draw_info_row_noted(canvas, top, label, value, help, None);
}

/// [`draw_info_row_at`] with one more line under what the row is for: something to be said of this
/// row in particular, written in the colour a note is. Where the panel has no room for both -- a
/// value that took lines of its own -- the note is written and what the row is for is not, since
/// that is said of every row of its kind and the note of this one alone.
pub(crate) fn draw_info_row_noted(canvas: &mut Canvas<'_>, top: f32, label: &str, value: &str, help: &str, note: Option<&str>) {
    let colors = theme();
    let left = INFO_RECT.x + INFO_PAD;
    let room = INFO_RECT.w - INFO_PAD * 2.0;
    let label = fit_text(label, INFO_SCALE, room);
    let mut y = INFO_RECT.y + top;
    draw_text(canvas, left, y, INFO_SCALE, colors.text, &label);
    if text_width(&label, INFO_SCALE) + INFO_HEADING_GAP + text_width(value, INFO_SCALE) <= room {
        draw_text_right(canvas, left + room, y, INFO_SCALE, colors.title_focus, value);
        y += INFO_LINE_PITCH;
    } else {
        y += INFO_LINE_PITCH;
        for part in wrapped(value, INFO_SCALE, room, INFO_VALUE_LINES) {
            draw_text(canvas, left, y, INFO_SCALE, colors.title_focus, &part);
            y += INFO_LINE_PITCH;
        }
    }
    let Some(note) = note else {
        draw_text(canvas, left, y, INFO_SCALE, colors.text_muted, &fit_text(help, INFO_SCALE, room));
        return;
    };
    if y + INFO_LINE_PITCH + INFO_LINE_HEIGHT <= INFO_RECT.y + INFO_RECT.h {
        draw_text(canvas, left, y, INFO_SCALE, colors.text_muted, &fit_text(help, INFO_SCALE, room));
        y += INFO_LINE_PITCH;
    }
    draw_text(canvas, left, y, INFO_SCALE, colors.accent, &fit_text(note, INFO_SCALE, room));
}

/// Fill the preview's rectangle and write `status` in it, for a frame the preview drew nothing
/// on: a heading, and under it what follows from it or why, over as many lines as it is given.
///
/// The fill goes through the region a frame of a skin would have been drawn in, so it covers the
/// very pixels that frame covers: the preview is put on whole pixels of the target, which at a
/// window that is not a whole multiple of the menu's size is not where the rectangle's own edges
/// fall.
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

/// Draw the right half: the line round the preview, the preview into its rectangle, then the status
/// in its place when no frame of a skin is there -- or nothing but the dark, for a wait the preview
/// asks to be left unsaid.
///
/// Called once a frame from a menu's `draw`, after the menu has cleared the frame.
pub(crate) fn draw_preview(preview: &mut SkinPreview, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
    let border = Rect::new(
        PREVIEW_RECT.x - PREVIEW_BORDER,
        PREVIEW_RECT.y - PREVIEW_BORDER,
        PREVIEW_RECT.w + PREVIEW_BORDER * 2.0,
        PREVIEW_RECT.h + PREVIEW_BORDER * 2.0,
    );
    canvas.fill_rect(border, theme().divider);
    preview.draw(ctx, canvas, PREVIEW_RECT);
    if preview.waits_in_the_dark() {
        draw_dark(canvas);
    } else if !preview.status().is_drawn() {
        draw_status(canvas, preview.status());
    }
}

/// Write the line of keys along the foot of the screen.
pub(crate) fn draw_hint(canvas: &mut Canvas<'_>, hint: &str) {
    draw_text(canvas, MARGIN, HINT_Y, HINT_SCALE, theme().text_muted, hint);
}
