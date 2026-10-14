//! The settings screen as data plus one draw call.
//!
//! The screen is three columns under a title, in the grammar of the skin menu and the play menu
//! ([`crate::stage::preview_menu`]): a rail of the groups on the left with the ways to the other
//! screens at its foot, the rows of the open group in the middle under the headings of its
//! sections, and on the right a panel that says what the row under the cursor is for, what it
//! holds and ships as, and which keys it hears. The keys are written again along the foot of the
//! screen, with the keys of the screen itself after them.
//!
//! A row is drawn by the kind of value it holds ([`RowControl`]), so a switch, a choice, a number,
//! a line of text, a path and an action are told apart before any of them is read: a switch as a
//! switch beside its word, a choice of a few values as all of them side by side with the chosen one
//! lit, a choice of many as its value between two arrows, a number as a slider and its number
//! ([`crate::number_row`]), text in a field, a path cut short from its front beside the mark that
//! opens a picker, and an action as that mark alone. A row that is not what it ships as carries a
//! mark at its left edge, and so does its group on the rail.
//!
//! Nothing here moves on its own: a frame is what the scene says and no more, so the screen is the
//! same picture until a key or the pointer changes it.
//!
//! Which part of a row a press landed on is told from where the row was drawn
//! ([`control_part_at`]). Everything is laid out in the fixed space every built-in screen is laid
//! out in, and drawn through [`Renderer`] so a frame can be made headless in tests.

use rbms_render::{Rect, Renderer, draw_text, draw_text_centered, draw_text_right, fit_text, text_width, theme};

use crate::number_row::{NumberEdit, NumberValue, RowPart, SliderLook, draw_edit, draw_refusal, draw_slider, row_part};
use crate::stage::preview_menu::{
    COLUMN_GAP, HINT_GAP, HINT_SCALE, HINT_Y, MARGIN, SCROLL_BAR_INSET, SCROLL_BAR_ROOM, SCROLL_BAR_W, SUBTITLE_SCALE, SUBTITLE_Y, TITLE_SCALE, TITLE_Y,
    contains, scroll_thumb_in, wrapped,
};

/// How wide the space the screen is laid out in is.
const CANVAS_W: f32 = 1280.0;

/// The screen's title, and the room between it and what is written beside it.
const TITLE: &str = "SETTINGS";
const SUBTITLE_GAP: f32 = 20.0;

/// The top of the three columns, under the title, and their foot, above the line of keys.
const COLUMNS_TOP: f32 = 72.0;
const COLUMNS_BOTTOM: f32 = HINT_Y - HINT_GAP;

/// How wide the rail of groups on the left is, and how wide the panel on the right.
const RAIL_W: f32 = 188.0;
const INFO_W: f32 = 344.0;

/// The rail of groups, the list of rows and the panel that describes the row under the cursor.
const RAIL_RECT: Rect = Rect { x: MARGIN, y: COLUMNS_TOP, w: RAIL_W, h: COLUMNS_BOTTOM - COLUMNS_TOP };
const INFO_RECT: Rect = Rect { x: CANVAS_W - MARGIN - INFO_W, y: COLUMNS_TOP, w: INFO_W, h: COLUMNS_BOTTOM - COLUMNS_TOP };
const LIST_RECT: Rect =
    Rect { x: MARGIN + RAIL_W + COLUMN_GAP, y: COLUMNS_TOP, w: INFO_RECT.x - COLUMN_GAP - (MARGIN + RAIL_W + COLUMN_GAP), h: COLUMNS_BOTTOM - COLUMNS_TOP };

/// An entry of the rail: how far one is from the next, how tall one is, how far its name is kept
/// from its edges and below its top, and how large the name and the count beside it are written.
const RAIL_PITCH: f32 = 38.0;
const RAIL_ENTRY_H: f32 = 34.0;
const RAIL_PAD: f32 = 12.0;
const RAIL_NAME_Y: f32 = 10.0;
const RAIL_NAME_SCALE: f32 = 1.5;
const RAIL_COUNT_Y: f32 = 12.0;
const RAIL_COUNT_SCALE: f32 = 1.2;

/// The button under the groups that puts every row of the open group back to what it ships as: how
/// far below the last group it is, how tall, and how large and how far below its top its words are.
const RESET_GAP: f32 = 10.0;
const RESET_H: f32 = 30.0;
const RESET_SCALE: f32 = 1.2;
const RESET_TEXT_Y: f32 = 10.0;

/// What the button reads, and the key that does the same.
const RESET_GROUP_LABEL: &str = "RESET GROUP";
pub(crate) const RESET_GROUP_KEY: &str = "SHIFT+0";

/// The ways to the other screens at the foot of the rail: how many there are, the heading over
/// them with how far above the first it is written and how large, and how large the name of a
/// screen and the key that opens it are written on its button.
const LINK_COUNT: usize = 3;
const LINKS_HEADING: &str = "OTHER SCREENS";
const LINKS_HEADING_RISE: f32 = 16.0;
const LINKS_HEADING_SCALE: f32 = 1.1;
const LINK_NAME_SCALE: f32 = 1.4;
const LINK_KEY_SCALE: f32 = 1.2;
const LINK_KEY_Y: f32 = 12.0;

/// What the three buttons read, top to bottom: the play menu, the skin menu and the key editor.
const PLAY_MENU_LINK: &str = "PLAY MENU";
const SKIN_MENU_LINK: &str = "SKIN MENU";
const KEY_CONFIG_LINK: &str = "KEY CONFIG";

/// Which of the buttons each of them is, top to bottom.
const PLAY_MENU_PLACE: usize = 0;
const SKIN_MENU_PLACE: usize = 1;
const KEY_CONFIG_PLACE: usize = 2;

/// The entries of the list: how far they are kept from the list's own edges, how far one is from
/// the next, how tall a row is, and how many entries -- rows and the headings over them -- are on
/// show at once.
const LIST_PAD: f32 = 4.0;
const ROW_PITCH: f32 = 32.0;
const ROW_H: f32 = 28.0;
const VISIBLE_SLOTS: usize = 18;

/// A row: where its label begins, how far what is on its right is kept from its right edge, how
/// far below its top its label and its value are written, and how large each is.
const LABEL_X: f32 = 20.0;
const ROW_PAD_RIGHT: f32 = 10.0;
const LABEL_TEXT_Y: f32 = 8.0;
const VALUE_TEXT_Y: f32 = 8.0;
const LABEL_SCALE: f32 = 1.3;
const VALUE_SCALE: f32 = 1.4;

/// The room every row keeps on its right for what sets its value, and the gap between that room
/// and the label before it.
const CONTROL_W: f32 = 300.0;
const LABEL_GAP: f32 = 12.0;

/// The mark of a row that is not what it ships as: how far inside the row's left edge it is, and
/// how large.
const MARK_X: f32 = 8.0;
const MARK_SIZE: f32 = 5.0;

/// The heading over a section: where it begins, how far below the top of its slot, how large it is
/// written, and the gap before the line drawn from its end to the edge of the list.
const HEADING_X: f32 = 8.0;
const HEADING_TEXT_Y: f32 = 12.0;
const HEADING_SCALE: f32 = 1.2;
const HEADING_RULE_GAP: f32 = 10.0;
const HEADING_RULE_DROP: f32 = 6.0;
const RULE_H: f32 = 1.0;

/// The note beside a row: how large it is written and how far below the row's top.
const NOTE_SCALE: f32 = 1.1;
const NOTE_TEXT_Y: f32 = 10.0;

/// A row that holds a number: how wide the room its number is written in is, how wide the slider
/// before it, and the gap between the two. The number has a room of its own, whatever it reads, so
/// the slider stands in the same place on every row and does not move under a pointer dragging it.
const NUMBER_VALUE_W: f32 = 64.0;
const SLIDER_W: f32 = 200.0;
const SLIDER_GAP: f32 = 12.0;

/// A switch: how wide and how tall its track is, how far inside the track its knob is kept, the gap
/// between it and its word, and how wide the part of the row that turns it over is.
const SWITCH_W: f32 = 36.0;
const SWITCH_H: f32 = 16.0;
const SWITCH_INSET: f32 = 2.0;
const SWITCH_WORD_GAP: f32 = 10.0;
const SWITCH_HIT_W: f32 = 96.0;

/// A choice with all of its values on show: the most values that are shown that way, the widest a
/// value's cell is drawn, the gap between two cells, how tall a cell is, how large and how far below
/// the cell's top its value is written, and the room kept between a value and the cell's two sides.
const SEGMENTS_MAX: usize = 4;
const SEGMENT_MAX_W: f32 = 110.0;
const SEGMENT_GAP: f32 = 2.0;
const SEGMENT_H: f32 = 20.0;
const SEGMENT_SCALE: f32 = 1.2;
const SEGMENT_TEXT_Y: f32 = 5.0;
const SEGMENT_PAD: f32 = 5.0;

/// A choice of many values: how wide the room its value and the two arrows take is, and how much of
/// the room is kept clear between an arrow and the value.
const STEPPER_W: f32 = 220.0;
const STEPPER_PAD: f32 = 8.0;

/// The mark of something a press steps or opens -- an arrow of a choice, the mark of an action --
/// and the cell it is drawn in: how large the cell is, and how large and how far below the cell's
/// top the mark is written.
const CELL_SIZE: f32 = 20.0;
const CELL_SCALE: f32 = 1.3;
const CELL_TEXT_Y: f32 = 4.0;
const BACK_MARK: &str = "<";
const FORWARD_MARK: &str = ">";

/// A field text is typed into: how tall it is, how thick the line round it is while it is being
/// typed into, and how far inside it its text begins.
const FIELD_H: f32 = 22.0;
const FIELD_BORDER: f32 = 2.0;
const FIELD_PAD: f32 = 8.0;

/// What a path cut short from its front begins with.
const ELLIPSIS: &str = "…";

/// Where the cursor's place among the rows of the group is written, measured up from the list's
/// foot, and how large.
const COUNTER_RISE: f32 = 18.0;
const COUNTER_SCALE: f32 = 1.2;

/// The panel beside the list: how far what is on it is kept from its edges, and where each part of
/// it begins, measured down from its top -- the row's label, the kind of row it is, what it is for,
/// the line under that, what it holds, and the keys it hears.
const INFO_PAD: f32 = 16.0;
const INFO_LABEL_Y: f32 = 14.0;
const INFO_KIND_Y: f32 = 38.0;
const INFO_DESCRIPTION_Y: f32 = 58.0;
const INFO_RULE_Y: f32 = 170.0;
const INFO_FACTS_Y: f32 = 182.0;
const INFO_KEYS_Y: f32 = 404.0;

/// How large the parts of the panel are written, how far one line of it is from the next, and how
/// many lines what a row is for, the keys it hears, one of the things it holds, what is said of it
/// in particular and the status at the panel's foot are each written over.
const INFO_LABEL_SCALE: f32 = 1.6;
const INFO_SMALL_SCALE: f32 = 1.1;
const INFO_SCALE: f32 = 1.3;
const INFO_LINE_PITCH: f32 = 17.0;
const DESCRIPTION_LINES: usize = 6;
pub(crate) const INFO_KEY_LINES: usize = 5;
const FACT_LINES: usize = 4;
const REMARK_LINES: usize = 4;
const STATUS_LINES: usize = 3;

/// Where what a row holds is written beside the name of it, how far the name, which is written
/// smaller, is dropped to stand on the same line, the room between the last of them and what is
/// said of the row in particular, and how far the heading over the keys is above the first.
const FACT_VALUE_X: f32 = 78.0;
const FACT_NAME_DROP: f32 = 2.0;
const REMARK_GAP: f32 = 8.0;
const KEYS_HEADING: &str = "KEYS";
const KEYS_FIRST_Y: f32 = 18.0;

/// Where what a key does is written beside the key, and the room kept between the two.
const KEY_ACTION_X: f32 = 118.0;
const KEY_NAME_GAP: f32 = 8.0;

/// How far above the panel's foot the last line of the status ends, and the heading over the
/// status, which is as far above its first line as the heading over the keys is above the first key.
const STATUS_FOOT: f32 = 12.0;
const STATUS_HEADING: &str = "STATUS";

/// How thick the line round a panel drawn over the row list is.
const OVERLAY_BORDER: f32 = 2.0;

/// The inline rival list, drawn over the row list.
const RIVALS_W: f32 = 520.0;
const RIVALS_H: f32 = 420.0;
const RIVALS_X: f32 = LIST_RECT.x + (LIST_RECT.w - RIVALS_W) * 0.5;
const RIVALS_Y: f32 = 150.0;
const RIVALS_ROWS_TOP: f32 = 72.0;
const RIVALS_ROW_PITCH: f32 = 38.0;
const RIVALS_ROW_H: f32 = 32.0;

/// The inline list: how far its heading and its keys are kept from its left edge and below its top,
/// how far its rows are kept from its sides and its foot, and where a row's text begins and how far
/// below the row's top it is written.
const RIVALS_PAD: f32 = 16.0;
const RIVALS_TITLE_Y: f32 = 14.0;
const RIVALS_HINT_Y: f32 = 44.0;
const RIVALS_ROW_INSET: f32 = 8.0;
const RIVALS_ROW_TEXT_X: f32 = 20.0;
const RIVALS_ROW_TEXT_Y: f32 = 8.0;
const RIVALS_TITLE_SCALE: f32 = 1.6;
const RIVALS_HINT_SCALE: f32 = 1.2;
const RIVAL_SCALE: f32 = 1.5;

/// The question asked before every row of a group is put back, drawn over the row list: how large
/// its panel is, how far what is on it is kept from its edges, where its heading, what it says and
/// its two buttons begin, measured down from its top, and how tall and how far apart the buttons
/// are.
const CONFIRM_W: f32 = 440.0;
const CONFIRM_H: f32 = 136.0;
const CONFIRM_X: f32 = LIST_RECT.x + (LIST_RECT.w - CONFIRM_W) * 0.5;
const CONFIRM_Y: f32 = 250.0;
const CONFIRM_PAD: f32 = 16.0;
const CONFIRM_TITLE_Y: f32 = 16.0;
const CONFIRM_BODY_Y: f32 = 48.0;
const CONFIRM_BUTTONS_Y: f32 = 90.0;
const CONFIRM_BUTTON_H: f32 = 30.0;
const CONFIRM_BUTTON_GAP: f32 = 12.0;
const CONFIRM_TITLE_SCALE: f32 = 1.6;
const CONFIRM_BUTTON_SCALE: f32 = 1.3;
const CONFIRM_BUTTON_TEXT_Y: f32 = 9.0;

/// What the two buttons of the question read.
const CONFIRM_YES: &str = "ENTER  RESET";
const CONFIRM_NO: &str = "ESC  KEEP";

/// The keys of the screen while the question is up, written where the screen names its keys.
pub(crate) const CONFIRM_HINT: &str = "ENTER RESET THE GROUP   ESC KEEP IT AS IT IS";

/// Keyboard hint inside the inline rival list.
pub(crate) const RIVALS_HINT: &str = "UP DOWN MOVE   ENTER ADD   D REMOVE   ESC SAVE/CLOSE";

/// Title of the inline rival list.
const RIVALS_TITLE: &str = "RIVALS";

/// What sets the value of a row, which is how the row is drawn and what a press on it does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RowControl {
    /// On or off: a switch beside its word.
    Switch { on: bool },
    /// One of a few values, all of them on show side by side with the chosen one lit. `chosen` is
    /// `None` while the row holds something that is none of them.
    Segments { values: &'static [&'static str], chosen: Option<usize> },
    /// One of many values: the value between an arrow back and an arrow forward.
    Stepper,
    /// A number: a slider and the number, which is the row's [`SettingsLine::number`].
    Slider,
    /// A line of text typed into the row. `filled` is whether anything has been.
    Field { filled: bool },
    /// A file or a folder chosen with a picker: the path, cut short from its front, beside the mark
    /// that opens the picker. `chosen` is whether one has been.
    Path { chosen: bool },
    /// Something that is run or opened; the value beside the mark is what it reports, if anything.
    Action,
    /// Something the row reports and cannot change.
    Info,
}

/// One row of the settings screen as it is drawn.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct SettingsLine {
    pub(crate) label: String,
    pub(crate) value: String,
    /// What is said beside the row, or `None` for a row shown bare. A note is written small after
    /// the label, and the value of an annotated row is dimmed until the row is focused.
    pub(crate) note: Option<&'static str>,
    /// The number the row holds, or `None` for a row that holds none. A row that holds one is
    /// drawn with a slider before its value.
    pub(crate) number: Option<NumberValue>,
    /// What sets the row's value.
    pub(crate) control: RowControl,
    /// Whether the row is not what it ships as, which is marked at its left edge.
    pub(crate) changed: bool,
}

impl SettingsLine {
    /// A row that reports `value`, with nothing said beside it.
    #[cfg(test)]
    pub(crate) fn bare(label: &str, value: &str) -> SettingsLine {
        SettingsLine { label: label.to_string(), value: value.to_string(), note: None, number: None, control: RowControl::Info, changed: false }
    }
}

/// One group on the rail: its name, how many rows it lists, and how many of them are not what they
/// ship as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct RailGroup {
    pub(crate) name: &'static str,
    pub(crate) rows: usize,
    pub(crate) changed: usize,
}

/// One entry of the list: the heading over a section, or a row by its place in
/// [`SettingsScene::rows`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ListItem {
    Heading(&'static str),
    Row(usize),
}

/// What the panel beside the list says of the row under the cursor.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct InfoScene {
    pub(crate) label: String,
    /// What kind of row it is, in a word or two: a switch, a choice and where it stands in it.
    pub(crate) kind: String,
    /// What the row is for.
    pub(crate) description: &'static str,
    /// What the row holds, each under a name: its value, what it ships as, its range and its step.
    pub(crate) facts: Vec<(&'static str, String)>,
    /// What is said of this row in particular as it stands: what the note beside it means.
    pub(crate) remark: Option<&'static str>,
    /// A caution about the value the row holds now, written in the colour a caution is.
    pub(crate) caution: Option<&'static str>,
    /// The keys the row hears, each with what it does.
    pub(crate) keys: Vec<(String, &'static str)>,
}

/// The question asked before every row of a group is put back to what it ships as.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct ConfirmScene {
    pub(crate) title: String,
    pub(crate) body: String,
}

/// Everything the settings screen draws this frame, assembled before the render pass.
pub(crate) struct SettingsScene {
    pub(crate) groups: Vec<RailGroup>,
    /// The open group, by its place on the rail.
    pub(crate) group: usize,
    /// What the open group holds, written beside the title.
    pub(crate) summary: String,
    /// The headings and rows of the open group, top to bottom.
    pub(crate) entries: Vec<ListItem>,
    pub(crate) rows: Vec<SettingsLine>,
    /// The row under the cursor, by its place in `rows`.
    pub(crate) sel: usize,
    /// The first entry on show.
    pub(crate) top: usize,
    /// What the panel beside the list says, or `None` while the group lists nothing.
    pub(crate) info: Option<InfoScene>,
    /// The line of keys along the foot of the screen.
    pub(crate) hint: String,
    /// The keys that open the play menu, the skin menu and the key editor, as they are named.
    pub(crate) link_keys: [String; LINK_COUNT],
    /// The row being typed into as text, by its place in `rows`, with what is drawn in its field:
    /// what has been typed, already masked when the row is a secret.
    pub(crate) editor: Option<(usize, String)>,
    /// Last network result or error, or what the output stream was opened as, shown at the foot of
    /// the panel.
    pub(crate) status: String,
    /// The inline rival list, when it is open.
    pub(crate) rivals: Option<RivalsScene>,
    /// The question asked before a group is put back, while it is up.
    pub(crate) confirm: Option<ConfirmScene>,
    /// The row that holds a number and is being typed into, by its place in `rows`, with the figure
    /// typed so far.
    pub(crate) typed: Option<(usize, NumberEdit)>,
    /// The row whose slider is being dragged, by its place in `rows`.
    pub(crate) dragged: Option<usize>,
    /// The row saying that what was typed into it was not a number, by its place in `rows`.
    pub(crate) refused: Option<usize>,
}

/// The inline rival list's own rows and editor.
pub(crate) struct RivalsScene {
    pub(crate) rows: Vec<String>,
    pub(crate) sel: usize,
    pub(crate) editor: Option<String>,
}

/// A clickable region of the settings screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SettingsHot {
    /// A group on the rail, by its place on it.
    Tab(usize),
    /// The button at the foot of the rail that opens the play menu.
    PlayMenuLink,
    /// The button under it that opens the skin menu.
    SkinMenuLink,
    /// The button under that one that opens the key editor.
    KeyConfigLink,
    /// A row of the list, by its place among the rows of the open group.
    Row(usize),
    RivalRow(usize),
}

/// Which part of a row a press landed on.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum ControlPart {
    /// The slider of a row that holds a number, drawn where it is said.
    Slider(Rect),
    /// The number of a row that holds one: the press opens the row to be typed into.
    Number,
    /// A switch or its word: the press turns it over.
    Switch,
    /// One value of a choice with all its values on show, by its place among them.
    Segment(usize),
    /// The arrow back of a choice of many.
    Back,
    /// The arrow forward of a choice of many, or its value: the press takes the next.
    Forward,
    /// A field, a path or an action: the press opens or runs it.
    Open,
    /// Anywhere else on the row: the press moves the cursor there and nothing more.
    Label,
}

/// How many entries of the list -- rows and the headings over them -- are on show at once.
pub(crate) fn visible_rows() -> usize {
    VISIBLE_SLOTS
}

/// The list of rows, which is where the wheel scrolls it from.
pub(crate) fn list_rect() -> Rect {
    LIST_RECT
}

/// How many rival rows fit in the inline list.
pub(crate) fn rivals_visible_rows() -> usize {
    (((RIVALS_H - RIVALS_ROWS_TOP - RIVALS_ROW_INSET) / RIVALS_ROW_PITCH).floor() as usize).max(1)
}

fn rivals_scroll_start(len: usize, sel: usize) -> usize {
    let visible = rivals_visible_rows();
    if len <= visible {
        return 0;
    }
    sel.saturating_sub(visible / 2).min(len - visible)
}

/// Where the entry in one slot of the list is drawn, the first slot being the top one on show.
fn slot_rect(slot: usize) -> Rect {
    Rect::new(LIST_RECT.x + LIST_PAD, LIST_RECT.y + LIST_PAD + slot as f32 * ROW_PITCH, LIST_RECT.w - LIST_PAD - SCROLL_BAR_ROOM, ROW_H)
}

/// The right edge of what sets the value of the row drawn at `row`.
fn control_right(row: Rect) -> f32 {
    row.x + row.w - ROW_PAD_RIGHT
}

/// The room the row drawn at `row` keeps for what sets its value.
fn control_rect(row: Rect) -> Rect {
    Rect::new(control_right(row) - CONTROL_W, row.y, CONTROL_W, row.h)
}

/// Where the slider of a row that holds a number is, the row having been drawn at `row`: before the
/// number, as tall as the row, so a press anywhere over it takes hold of it.
pub(crate) fn row_slider(row: Rect) -> Rect {
    Rect::new(control_right(row) - NUMBER_VALUE_W - SLIDER_GAP - SLIDER_W, row.y, SLIDER_W, row.h)
}

/// Where the number of a row that holds one is written, the row having been drawn at `row`: against
/// the row's right edge. A press there opens the row to be typed into.
pub(crate) fn row_number(row: Rect) -> Rect {
    Rect::new(control_right(row) - NUMBER_VALUE_W, row.y, NUMBER_VALUE_W + ROW_PAD_RIGHT, row.h)
}

/// Where a figure being typed into a row is drawn: over the slider and the number both.
fn number_edit_rect(row: Rect) -> Rect {
    let slider = row_slider(row);
    Rect::new(slider.x, row.y, row.x + row.w - ROW_PAD_RIGHT * 0.5 - slider.x, row.h)
}

/// Which part of a row that holds a number a press at `at` landed on, the row having been drawn at
/// `row`.
pub(crate) fn number_part_at(row: Rect, at: (f32, f32)) -> RowPart {
    row_part(row_slider(row), row_number(row), at)
}

/// Where the track of a switch is, the row having been drawn at `row`.
fn switch_track(row: Rect) -> Rect {
    Rect::new(control_right(row) - SWITCH_W, row.y + ((row.h - SWITCH_H) * 0.5).round(), SWITCH_W, SWITCH_H)
}

/// The part of the row drawn at `row` that turns its switch over: the switch and its word.
pub(crate) fn row_switch(row: Rect) -> Rect {
    Rect::new(control_right(row) - SWITCH_HIT_W, row.y, SWITCH_HIT_W + ROW_PAD_RIGHT, row.h)
}

/// How wide one cell of a choice of `count` values with all of them on show is.
fn segment_width(count: usize) -> f32 {
    let count = count.max(1) as f32;
    ((CONTROL_W - SEGMENT_GAP * (count - 1.0)) / count).min(SEGMENT_MAX_W)
}

/// Where the cell of the value at `place` of a choice of `count` is, the row having been drawn at
/// `row`: the cells stand side by side against the row's right edge.
pub(crate) fn row_segment(row: Rect, place: usize, count: usize) -> Rect {
    let width = segment_width(count);
    let total = width * count as f32 + SEGMENT_GAP * count.saturating_sub(1) as f32;
    let left = control_right(row) - total + place as f32 * (width + SEGMENT_GAP);
    Rect::new(left, row.y + ((row.h - SEGMENT_H) * 0.5).round(), width, SEGMENT_H)
}

/// Whether every one of `values` can be written whole in a cell of a choice of that many, which is
/// what lets the choice be drawn with all of them on show.
pub(crate) fn segments_fit(values: &[&str]) -> bool {
    let room = segment_width(values.len()) - SEGMENT_PAD * 2.0;
    !values.is_empty() && values.len() <= SEGMENTS_MAX && values.iter().all(|value| text_width(value, SEGMENT_SCALE) <= room)
}

/// The cell a mark is drawn in at the right end of what sets a row's value.
fn end_cell(row: Rect) -> Rect {
    Rect::new(control_right(row) - CELL_SIZE, row.y + ((row.h - CELL_SIZE) * 0.5).round(), CELL_SIZE, CELL_SIZE)
}

/// The cell of the arrow back of a choice of many, the row having been drawn at `row`.
fn back_cell(row: Rect) -> Rect {
    Rect { x: control_right(row) - STEPPER_W, ..end_cell(row) }
}

/// The part of the row drawn at `row` that steps a choice of many back: its arrow, as tall as the
/// row.
pub(crate) fn row_back(row: Rect) -> Rect {
    let cell = back_cell(row);
    Rect::new(cell.x - STEPPER_PAD * 0.5, row.y, cell.w + STEPPER_PAD, row.h)
}

/// The part of the row drawn at `row` that steps a choice of many forward: its value and the arrow
/// after it.
pub(crate) fn row_forward(row: Rect) -> Rect {
    let back = row_back(row);
    Rect::new(back.x + back.w, row.y, row.x + row.w - (back.x + back.w), row.h)
}

/// The field text is typed into, the row having been drawn at `row`.
fn field_rect(row: Rect) -> Rect {
    let room = control_rect(row);
    Rect::new(room.x, row.y + ((row.h - FIELD_H) * 0.5).round(), room.w, FIELD_H)
}

/// The part of the row drawn at `row` that opens or runs it: the room kept for what sets its value.
pub(crate) fn row_open(row: Rect) -> Rect {
    let room = control_rect(row);
    Rect::new(room.x, row.y, room.w + ROW_PAD_RIGHT, row.h)
}

/// Which part of a row a press at `at` landed on, the row being set by `control` and having been
/// drawn at `row`.
pub(crate) fn control_part_at(row: Rect, control: RowControl, at: (f32, f32)) -> ControlPart {
    match control {
        RowControl::Slider => match number_part_at(row, at) {
            RowPart::Slider(area) => ControlPart::Slider(area),
            RowPart::Number => ControlPart::Number,
            RowPart::Rest => ControlPart::Label,
        },
        RowControl::Switch { .. } if contains(row_switch(row), at) => ControlPart::Switch,
        RowControl::Segments { values, .. } => {
            let hit = (0..values.len()).find(|place| {
                let cell = row_segment(row, *place, values.len());
                contains(Rect::new(cell.x, row.y, cell.w + SEGMENT_GAP, row.h), at)
            });
            hit.map_or(ControlPart::Label, ControlPart::Segment)
        }
        RowControl::Stepper if contains(row_back(row), at) => ControlPart::Back,
        RowControl::Stepper if contains(row_forward(row), at) => ControlPart::Forward,
        RowControl::Field { .. } | RowControl::Path { .. } | RowControl::Action if contains(row_open(row), at) => ControlPart::Open,
        _ => ControlPart::Label,
    }
}

/// The room an entry of the rail has for what is written at its right end, beside what is written
/// at its left: the entry between its two pads, less a gap as wide as a pad between the two.
fn rail_room() -> f32 {
    RAIL_W - RAIL_PAD * 2.0 - RAIL_PAD
}

/// Where the entry of the group at one place of the rail is drawn.
fn rail_entry(place: usize) -> Rect {
    Rect::new(RAIL_RECT.x, RAIL_RECT.y + place as f32 * RAIL_PITCH, RAIL_RECT.w, RAIL_ENTRY_H)
}

/// Where the button that puts every row of the open group back is, under a rail of `groups` groups.
pub(crate) fn reset_group_rect(groups: usize) -> Rect {
    Rect::new(RAIL_RECT.x, RAIL_RECT.y + groups as f32 * RAIL_PITCH + RESET_GAP, RAIL_RECT.w, RESET_H)
}

/// Where the button to another screen at one place of the foot of the rail is drawn.
fn link_rect(place: usize) -> Rect {
    let from_foot = (LINK_COUNT - place.min(LINK_COUNT - 1)) as f32;
    Rect::new(RAIL_RECT.x, COLUMNS_BOTTOM - from_foot * RAIL_PITCH + (RAIL_PITCH - RAIL_ENTRY_H), RAIL_RECT.w, RAIL_ENTRY_H)
}

/// Where the button to the key editor is on the screen as [`render_settings`] lays it out, for the
/// screen to test a press against.
pub(crate) fn key_config_link() -> Rect {
    link_rect(KEY_CONFIG_PLACE)
}

/// The panel of the question asked before a group is put back.
fn confirm_rect() -> Rect {
    Rect::new(CONFIRM_X, CONFIRM_Y, CONFIRM_W, CONFIRM_H)
}

/// How wide each of the two buttons of the question is.
const fn confirm_button_w() -> f32 {
    (CONFIRM_W - CONFIRM_PAD * 2.0 - CONFIRM_BUTTON_GAP) * 0.5
}

/// Where the button of the question that puts the group back is.
pub(crate) fn confirm_yes_rect() -> Rect {
    Rect::new(CONFIRM_X + CONFIRM_PAD, CONFIRM_Y + CONFIRM_BUTTONS_Y, confirm_button_w(), CONFIRM_BUTTON_H)
}

/// Where the button of the question that leaves the group as it is is.
pub(crate) fn confirm_no_rect() -> Rect {
    let yes = confirm_yes_rect();
    Rect { x: yes.x + yes.w + CONFIRM_BUTTON_GAP, ..yes }
}

/// `text` cut short from its front to fit `max_width` at `scale`, which is how a path is cut: the
/// end of a path is what tells one folder from another.
fn fit_tail(text: &str, scale: f32, max_width: f32) -> String {
    if max_width <= 0.0 || text_width(text, scale) <= max_width {
        return text.to_string();
    }
    let room = max_width - text_width(ELLIPSIS, scale);
    let mut kept = text;
    while !kept.is_empty() && text_width(kept, scale) > room {
        let first = kept.chars().next().map_or(kept.len(), char::len_utf8);
        kept = &kept[first..];
    }
    format!("{ELLIPSIS}{kept}")
}

/// The room the label of a row has before the room kept for what sets its value.
fn label_room(row: Rect) -> f32 {
    control_rect(row).x - LABEL_GAP - (row.x + LABEL_X)
}

/// Where a row's note starts and how much room it has: what is left of the row between its label
/// and the room kept for what sets its value, less a gap on either side.
fn note_span(row: Rect, label: &str) -> (f32, f32) {
    let left = row.x + LABEL_X + text_width(label, LABEL_SCALE) + LABEL_GAP;
    (left, control_rect(row).x - LABEL_GAP - left)
}

/// What the panel beside the list says of a row, broken over the lines it is written on.
fn description_lines(description: &str) -> Vec<String> {
    wrapped(description, INFO_SCALE, INFO_W - INFO_PAD * 2.0, DESCRIPTION_LINES)
}

/// Draw a mark in its cell.
fn draw_cell<R: Renderer>(r: &mut R, cell: Rect, mark: &str, on: bool) {
    let th = theme();
    r.fill_rect(cell, if on { th.button_active } else { th.panel_hi });
    draw_text_centered(r, cell.x + cell.w * 0.5, cell.y + CELL_TEXT_Y, CELL_SCALE, if on { th.title_focus } else { th.text_dim }, mark);
}

/// Draw what sets the value of one row that holds no number, on the row drawn at `row`.
fn draw_control<R: Renderer>(r: &mut R, row: Rect, line: &SettingsLine, on: bool, editor: Option<&str>) {
    let th = theme();
    let right = control_right(row);
    let text_y = row.y + VALUE_TEXT_Y;
    let value_color = match (on, line.note.is_some()) {
        (true, _) => th.title_focus,
        (false, true) => th.text_dim,
        (false, false) => th.text,
    };
    match line.control {
        RowControl::Switch { on: switched } => {
            let track = switch_track(row);
            let knob_w = (track.w * 0.5 - SWITCH_INSET).round();
            let knob_x = if switched { track.x + track.w - SWITCH_INSET - knob_w } else { track.x + SWITCH_INSET };
            r.fill_rect(track, if switched { th.text_dim } else { th.divider });
            r.fill_rect(Rect::new(knob_x, track.y + SWITCH_INSET, knob_w, track.h - SWITCH_INSET * 2.0), if switched { th.bg } else { th.text_muted });
            draw_text_right(r, track.x - SWITCH_WORD_GAP, text_y, VALUE_SCALE, if switched { value_color } else { th.text_muted }, &line.value);
        }
        RowControl::Segments { values, chosen } => {
            for (place, value) in values.iter().enumerate() {
                let cell = row_segment(row, place, values.len());
                let lit = chosen == Some(place);
                let fill = match (lit, on) {
                    (true, _) => th.text_dim,
                    (false, true) => th.button_active,
                    (false, false) => th.panel_hi,
                };
                r.fill_rect(cell, fill);
                draw_text_centered(r, cell.x + cell.w * 0.5, cell.y + SEGMENT_TEXT_Y, SEGMENT_SCALE, if lit { th.bg } else { th.text_muted }, value);
            }
        }
        RowControl::Stepper => {
            let (back, forward) = (back_cell(row), end_cell(row));
            draw_cell(r, back, BACK_MARK, on);
            draw_cell(r, forward, FORWARD_MARK, on);
            let room = forward.x - (back.x + back.w) - STEPPER_PAD * 2.0;
            let middle = (back.x + back.w + forward.x) * 0.5;
            draw_text_centered(r, middle, text_y, VALUE_SCALE, value_color, &fit_text(&line.value, VALUE_SCALE, room));
        }
        RowControl::Field { filled } => {
            let field = field_rect(row);
            let room = field.w - FIELD_PAD * 2.0;
            if let Some(buffer) = editor {
                r.fill_rect(field, th.accent);
                r.fill_rect(Rect::new(field.x + FIELD_BORDER, field.y + FIELD_BORDER, field.w - FIELD_BORDER * 2.0, field.h - FIELD_BORDER * 2.0), th.bg);
                draw_text(r, field.x + FIELD_PAD, text_y, VALUE_SCALE, th.text, &fit_tail(buffer, VALUE_SCALE, room));
            } else {
                r.fill_rect(field, th.bg);
                let color = if filled { value_color } else { th.text_muted };
                draw_text(r, field.x + FIELD_PAD, text_y, VALUE_SCALE, color, &fit_text(&line.value, VALUE_SCALE, room));
            }
        }
        RowControl::Path { chosen } => {
            let cell = end_cell(row);
            draw_cell(r, cell, FORWARD_MARK, on);
            let room = cell.x - STEPPER_PAD - control_rect(row).x;
            let color = if chosen { value_color } else { th.text_muted };
            draw_text_right(r, cell.x - STEPPER_PAD, text_y, VALUE_SCALE, color, &fit_tail(&line.value, VALUE_SCALE, room));
        }
        RowControl::Action => {
            let cell = end_cell(row);
            draw_cell(r, cell, FORWARD_MARK, on);
            if line.value != FORWARD_MARK {
                let room = cell.x - STEPPER_PAD - control_rect(row).x;
                draw_text_right(r, cell.x - STEPPER_PAD, text_y, VALUE_SCALE, value_color, &fit_text(&line.value, VALUE_SCALE, room));
            }
        }
        RowControl::Info | RowControl::Slider => {
            draw_text_right(r, right, text_y, VALUE_SCALE, if on { th.text_dim } else { th.text_muted }, &fit_text(&line.value, VALUE_SCALE, CONTROL_W));
        }
    }
}

/// Draw the slider and the number of one row that holds a number, or the figure being typed into
/// it, or that what was typed was not a number, on the row drawn at `row`.
fn draw_number<R: Renderer>(r: &mut R, row: Rect, line: &SettingsLine, number: &NumberValue, on: bool, state: (Option<&NumberEdit>, bool, bool)) {
    let th = theme();
    let (edit, dragged, refused) = state;
    let text_y = row.y + VALUE_TEXT_Y;
    if let Some(edit) = edit {
        draw_edit(r, number_edit_rect(row), text_y, VALUE_SCALE, edit);
        return;
    }
    if refused {
        draw_refusal(r, control_right(row), text_y, VALUE_SCALE);
        return;
    }
    let look = match (number.locked, dragged, on) {
        (true, _, _) => SliderLook::Locked,
        (false, true, _) => SliderLook::Dragged,
        (false, false, true) => SliderLook::Focused,
        (false, false, false) => SliderLook::Plain,
    };
    draw_slider(r, row_slider(row), number.fraction(), look);
    let color = match (on, line.note.is_some()) {
        (true, _) => th.title_focus,
        (false, true) => th.text_dim,
        (false, false) => th.text,
    };
    draw_text_right(r, control_right(row), text_y, VALUE_SCALE, color, &fit_text(&line.value, VALUE_SCALE, NUMBER_VALUE_W));
}

/// Draw the title and what the open group holds beside it.
fn draw_head<R: Renderer>(r: &mut R, scene: &SettingsScene) {
    let th = theme();
    draw_text(r, MARGIN, TITLE_Y, TITLE_SCALE, th.text, TITLE);
    let left = MARGIN + text_width(TITLE, TITLE_SCALE) + SUBTITLE_GAP;
    draw_text(r, left, SUBTITLE_Y, SUBTITLE_SCALE, th.text_dim, &fit_text(&scene.summary, SUBTITLE_SCALE, CANVAS_W - MARGIN - left));
}

/// Draw the rail: the groups with the open one lit, the button that puts the open group back, and
/// the buttons to the other screens at its foot.
fn draw_rail<R: Renderer>(r: &mut R, scene: &SettingsScene, hot: &mut Vec<(Rect, SettingsHot)>) {
    let th = theme();
    for (place, group) in scene.groups.iter().enumerate() {
        let on = place == scene.group;
        let rect = rail_entry(place);
        r.fill_rect(rect, if on { th.button } else { th.panel });
        draw_text(r, rect.x + RAIL_PAD, rect.y + RAIL_NAME_Y, RAIL_NAME_SCALE, if on { th.title_focus } else { th.text_dim }, group.name);
        let count = group.rows.to_string();
        let count_right = rect.x + rect.w - RAIL_PAD;
        draw_text_right(r, count_right, rect.y + RAIL_COUNT_Y, RAIL_COUNT_SCALE, if on { th.text_dim } else { th.text_muted }, &count);
        if group.changed > 0 {
            let mark_x = count_right - text_width(&count, RAIL_COUNT_SCALE) - MARK_X - MARK_SIZE;
            r.fill_rect(Rect::new(mark_x, rect.y + ((rect.h - MARK_SIZE) * 0.5).round(), MARK_SIZE, MARK_SIZE), th.accent);
        }
        hot.push((rect, SettingsHot::Tab(place)));
    }

    let changed = scene.groups.get(scene.group).map_or(0, |group| group.changed);
    let reset = reset_group_rect(scene.groups.len());
    r.fill_rect(reset, if changed > 0 { th.panel_hi } else { th.panel });
    let (name_color, key_color) = if changed > 0 { (th.accent, th.text_dim) } else { (th.text_muted, th.text_muted) };
    draw_text(r, reset.x + RAIL_PAD, reset.y + RESET_TEXT_Y, RESET_SCALE, name_color, RESET_GROUP_LABEL);
    draw_text_right(r, reset.x + reset.w - RAIL_PAD, reset.y + RESET_TEXT_Y, RESET_SCALE, key_color, RESET_GROUP_KEY);

    let first = link_rect(PLAY_MENU_PLACE);
    draw_text(r, first.x, first.y - LINKS_HEADING_RISE, LINKS_HEADING_SCALE, th.text_muted, LINKS_HEADING);
    let links = [
        (PLAY_MENU_PLACE, PLAY_MENU_LINK, SettingsHot::PlayMenuLink),
        (SKIN_MENU_PLACE, SKIN_MENU_LINK, SettingsHot::SkinMenuLink),
        (KEY_CONFIG_PLACE, KEY_CONFIG_LINK, SettingsHot::KeyConfigLink),
    ];
    for (place, name, link) in links {
        let rect = link_rect(place);
        r.fill_rect(rect, th.panel_hi);
        draw_text(r, rect.x + RAIL_PAD, rect.y + RAIL_NAME_Y, LINK_NAME_SCALE, th.accent, name);
        let key = format!("{} {FORWARD_MARK}", scene.link_keys[place]);
        let room = rail_room() - text_width(name, LINK_NAME_SCALE);
        draw_text_right(r, rect.x + rect.w - RAIL_PAD, rect.y + LINK_KEY_Y, LINK_KEY_SCALE, th.text_dim, &fit_text(&key, LINK_KEY_SCALE, room));
        hot.push((rect, link));
    }
}

/// Draw the entries of the open group that are on show, the cursor's row lit, with the scroll bar
/// when there are more entries than fit and the cursor's place among the rows.
fn draw_list<R: Renderer>(r: &mut R, scene: &SettingsScene, hot: &mut Vec<(Rect, SettingsHot)>) {
    let th = theme();
    r.fill_rect(LIST_RECT, th.panel);
    let shown = scene.top.min(scene.entries.len())..(scene.top + VISIBLE_SLOTS).min(scene.entries.len());
    for (slot, item) in scene.entries[shown].iter().enumerate() {
        let rect = slot_rect(slot);
        let index = match *item {
            ListItem::Heading(heading) => {
                draw_text(r, rect.x + HEADING_X, rect.y + HEADING_TEXT_Y, HEADING_SCALE, th.accent, heading);
                let rule_x = rect.x + HEADING_X + text_width(heading, HEADING_SCALE) + HEADING_RULE_GAP;
                let rule_y = rect.y + HEADING_TEXT_Y + HEADING_RULE_DROP;
                r.fill_rect(Rect::new(rule_x, rule_y, (rect.x + rect.w - rule_x).max(0.0), RULE_H), th.divider);
                continue;
            }
            ListItem::Row(index) => index,
        };
        let Some(line) = scene.rows.get(index) else {
            continue;
        };
        let on = index == scene.sel;
        if on {
            r.fill_rect(rect, th.button);
        }
        if line.changed {
            r.fill_rect(Rect::new(rect.x + MARK_X, rect.y + ((rect.h - MARK_SIZE) * 0.5).round(), MARK_SIZE, MARK_SIZE), th.accent);
        }
        let label = fit_text(&line.label, LABEL_SCALE, label_room(rect));
        draw_text(r, rect.x + LABEL_X, rect.y + LABEL_TEXT_Y, LABEL_SCALE, if on { th.text } else { th.text_dim }, &label);
        if let Some(note) = line.note {
            let (left, room) = note_span(rect, &label);
            if room > 0.0 {
                draw_text(r, left, rect.y + NOTE_TEXT_Y, NOTE_SCALE, if on { th.text_dim } else { th.text_muted }, &fit_text(note, NOTE_SCALE, room));
            }
        }
        match &line.number {
            Some(number) => {
                let edit = scene.typed.as_ref().filter(|(typed, _)| *typed == index).map(|(_, edit)| edit);
                draw_number(r, rect, line, number, on, (edit, scene.dragged == Some(index), scene.refused == Some(index)));
            }
            None => {
                let editor = scene.editor.as_ref().filter(|(edited, _)| *edited == index).map(|(_, buffer)| buffer.as_str());
                draw_control(r, rect, line, on, editor);
            }
        }
        hot.push((rect, SettingsHot::Row(index)));
    }

    if scene.entries.len() > VISIBLE_SLOTS {
        let track =
            Rect::new(LIST_RECT.x + LIST_RECT.w - SCROLL_BAR_INSET - SCROLL_BAR_W, LIST_RECT.y + LIST_PAD, SCROLL_BAR_W, VISIBLE_SLOTS as f32 * ROW_PITCH);
        r.fill_rect(track, th.panel_hi);
        r.fill_rect(scroll_thumb_in(track, scene.top, scene.entries.len(), VISIBLE_SLOTS), th.text_muted);
    }
    if !scene.rows.is_empty() {
        let counter = format!("{} / {}", scene.sel + 1, scene.rows.len());
        let right = LIST_RECT.x + LIST_RECT.w - SCROLL_BAR_ROOM - ROW_PAD_RIGHT;
        draw_text_right(r, right, LIST_RECT.y + LIST_RECT.h - COUNTER_RISE, COUNTER_SCALE, th.text_muted, &counter);
    }
}

/// Draw the panel beside the list: the row under the cursor by name, the kind of row it is, what it
/// is for, what it holds, what is said of it in particular, the keys it hears, and at the panel's
/// foot the status of the screen.
fn draw_info<R: Renderer>(r: &mut R, scene: &SettingsScene) {
    let th = theme();
    r.fill_rect(INFO_RECT, th.panel);
    let left = INFO_RECT.x + INFO_PAD;
    let room = INFO_RECT.w - INFO_PAD * 2.0;
    if let Some(info) = &scene.info {
        draw_text(r, left, INFO_RECT.y + INFO_LABEL_Y, INFO_LABEL_SCALE, th.text, &fit_text(&info.label, INFO_LABEL_SCALE, room));
        draw_text(r, left, INFO_RECT.y + INFO_KIND_Y, INFO_SMALL_SCALE, th.text_muted, &fit_text(&info.kind, INFO_SMALL_SCALE, room));
        for (at, line) in description_lines(info.description).iter().enumerate() {
            draw_text(r, left, INFO_RECT.y + INFO_DESCRIPTION_Y + at as f32 * INFO_LINE_PITCH, INFO_SCALE, th.text_dim, line);
        }
        r.fill_rect(Rect::new(left, INFO_RECT.y + INFO_RULE_Y, room, RULE_H), th.divider);

        let mut y = INFO_RECT.y + INFO_FACTS_Y;
        for (name, value) in &info.facts {
            draw_text(r, left, y + FACT_NAME_DROP, INFO_SMALL_SCALE, th.text_muted, name);
            for part in wrapped(value, INFO_SCALE, room - FACT_VALUE_X, FACT_LINES) {
                draw_text(r, left + FACT_VALUE_X, y, INFO_SCALE, th.text, &part);
                y += INFO_LINE_PITCH;
            }
        }
        y += REMARK_GAP;
        for (said, color) in [(info.caution, th.warn), (info.remark, th.accent)] {
            let Some(said) = said else {
                continue;
            };
            for part in wrapped(said, INFO_SCALE, room, REMARK_LINES) {
                if y + INFO_LINE_PITCH <= INFO_RECT.y + INFO_KEYS_Y {
                    draw_text(r, left, y, INFO_SCALE, color, &part);
                }
                y += INFO_LINE_PITCH;
            }
            y += REMARK_GAP;
        }

        let keys_top = INFO_RECT.y + INFO_KEYS_Y;
        draw_text(r, left, keys_top, INFO_SMALL_SCALE, th.text_muted, KEYS_HEADING);
        for (at, (key, action)) in info.keys.iter().take(INFO_KEY_LINES).enumerate() {
            let line_y = keys_top + KEYS_FIRST_Y + at as f32 * INFO_LINE_PITCH;
            draw_text(r, left, line_y, INFO_SCALE, th.text, &fit_text(key, INFO_SCALE, KEY_ACTION_X - KEY_NAME_GAP));
            draw_text(r, left + KEY_ACTION_X, line_y, INFO_SCALE, th.text_dim, &fit_text(action, INFO_SCALE, room - KEY_ACTION_X));
        }
    }
    let status = wrapped(&scene.status, INFO_SCALE, room, STATUS_LINES);
    let status_top = INFO_RECT.y + INFO_RECT.h - STATUS_FOOT - STATUS_LINES as f32 * INFO_LINE_PITCH;
    if !status.is_empty() {
        draw_text(r, left, status_top - KEYS_FIRST_Y, INFO_SMALL_SCALE, th.text_muted, STATUS_HEADING);
    }
    for (at, line) in status.iter().enumerate() {
        draw_text(r, left, status_top + at as f32 * INFO_LINE_PITCH, INFO_SCALE, th.accent, line);
    }
}

/// Draw the question asked before a group is put back, over the list.
fn draw_confirm<R: Renderer>(r: &mut R, confirm: &ConfirmScene) {
    let th = theme();
    let panel = confirm_rect();
    r.fill_rect(Rect::new(panel.x - OVERLAY_BORDER, panel.y - OVERLAY_BORDER, panel.w + OVERLAY_BORDER * 2.0, panel.h + OVERLAY_BORDER * 2.0), th.divider);
    r.fill_rect(panel, th.panel_hi);
    let room = panel.w - CONFIRM_PAD * 2.0;
    draw_text(r, panel.x + CONFIRM_PAD, panel.y + CONFIRM_TITLE_Y, CONFIRM_TITLE_SCALE, th.text, &fit_text(&confirm.title, CONFIRM_TITLE_SCALE, room));
    draw_text(r, panel.x + CONFIRM_PAD, panel.y + CONFIRM_BODY_Y, INFO_SCALE, th.text_dim, &fit_text(&confirm.body, INFO_SCALE, room));
    for (rect, words, fill, ink) in [(confirm_yes_rect(), CONFIRM_YES, th.text_dim, th.bg), (confirm_no_rect(), CONFIRM_NO, th.button_active, th.text)] {
        r.fill_rect(rect, fill);
        draw_text_centered(r, rect.x + rect.w * 0.5, rect.y + CONFIRM_BUTTON_TEXT_Y, CONFIRM_BUTTON_SCALE, ink, words);
    }
}

/// Draw the screen and return its clickable regions.
pub(crate) fn render_settings<R: Renderer>(r: &mut R, scene: &SettingsScene) -> Vec<(Rect, SettingsHot)> {
    let th = theme();
    let mut hot = Vec::new();
    r.clear(th.bg);
    draw_head(r, scene);
    draw_rail(r, scene, &mut hot);
    draw_list(r, scene, &mut hot);
    draw_info(r, scene);
    draw_text(r, MARGIN, HINT_Y, HINT_SCALE, th.text_muted, &fit_text(&scene.hint, HINT_SCALE, CANVAS_W - MARGIN * 2.0));
    if let Some(rivals) = &scene.rivals {
        hot.extend(render_rivals(r, rivals));
    }
    if let Some(confirm) = &scene.confirm {
        draw_confirm(r, confirm);
    }
    hot
}

fn render_rivals<R: Renderer>(r: &mut R, scene: &RivalsScene) -> Vec<(Rect, SettingsHot)> {
    let th = theme();
    let mut hot = Vec::new();
    let panel = Rect::new(RIVALS_X, RIVALS_Y, RIVALS_W, RIVALS_H);
    r.fill_rect(Rect::new(panel.x - OVERLAY_BORDER, panel.y - OVERLAY_BORDER, panel.w + OVERLAY_BORDER * 2.0, panel.h + OVERLAY_BORDER * 2.0), th.divider);
    r.fill_rect(panel, th.panel_hi);
    draw_text(r, RIVALS_X + RIVALS_PAD, RIVALS_Y + RIVALS_TITLE_Y, RIVALS_TITLE_SCALE, th.text, RIVALS_TITLE);
    draw_text(r, RIVALS_X + RIVALS_PAD, RIVALS_Y + RIVALS_HINT_Y, RIVALS_HINT_SCALE, th.text_muted, RIVALS_HINT);

    let start = rivals_scroll_start(scene.rows.len(), scene.sel);
    for (slot, index) in (start..(start + rivals_visible_rows()).min(scene.rows.len())).enumerate() {
        let y = RIVALS_Y + RIVALS_ROWS_TOP + slot as f32 * RIVALS_ROW_PITCH;
        let on = index == scene.sel;
        let rect = Rect::new(RIVALS_X + RIVALS_ROW_INSET, y, RIVALS_W - RIVALS_ROW_INSET * 2.0, RIVALS_ROW_H);
        r.fill_rect(rect, if on { th.button } else { th.panel });
        let is_add = index + 1 == scene.rows.len();
        let color = if is_add {
            th.accent
        } else if on {
            th.text
        } else {
            th.text_dim
        };
        let shown = match (&scene.editor, on && is_add) {
            (Some(buffer), true) => buffer.clone(),
            _ => scene.rows[index].clone(),
        };
        let room = RIVALS_W - RIVALS_ROW_TEXT_X * 2.0 - RIVALS_ROW_INSET;
        draw_text(r, RIVALS_X + RIVALS_ROW_TEXT_X, y + RIVALS_ROW_TEXT_Y, RIVAL_SCALE, color, &fit_text(&shown, RIVAL_SCALE, room));
        hot.push((rect, SettingsHot::RivalRow(index)));
    }
    hot
}

#[cfg(test)]
#[path = "stage/settings/view_tests.rs"]
mod tests;
