//! The practice panel as it is worked: the cursor, the row that starts the slice, the rows that
//! hold a choice, and the rows that hold a number -- by a key, by a slider and by a typed figure.
//!
//! With [`CAPTURE_DIR_ENV`] naming a folder, the panel is also saved there as PNGs a person can
//! open: all of it with its sliders, a slider being dragged, a time being typed, and an END TIME
//! brought down onto START TIME.

use std::time::Instant;

use rbms_render::text_width;

use super::*;
use crate::number_row::{EDIT_HINT, EDIT_HINT_CLOCK, REFUSAL_SHOWN_FOR, knob_rect, steps_at};
use crate::stage::HeadlessCanvas;
use crate::stage::render_tests::app;

/// Environment variable naming the folder captures are saved in. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The sizes the panel is drawn at: the one it is laid out for, a full HD window, and one that is
/// not sixteen to nine.
const DRAWN_SIZES: [(u32, u32); 3] = [(CW, CH), (1920, 1080), (1280, 1024)];

/// The two sizes the panel is captured at.
const CAPTURED_SIZES: [(u32, u32); 2] = [(CW, CH), (1920, 1080)];

/// How long the chart the panel is opened on is, to its last timeline, and its own TOTAL.
const CHART_MS: i32 = 180_000;
const CHART_TOTAL: f64 = 260.0;

/// The rows that hold a number and the rows that hold a choice.
const NUMBER_ROWS: [PracticeElement; 6] = [
    PracticeElement::StartTime,
    PracticeElement::EndTime,
    PracticeElement::GaugeValue,
    PracticeElement::JudgeRank,
    PracticeElement::Total,
    PracticeElement::Freq,
];
const CHOICE_ROWS: [PracticeElement; 3] = [PracticeElement::GaugeType, PracticeElement::GaugeCategory, PracticeElement::Random];

/// How far past an end of a slider, and how far off its row, the pointer is taken while it drags.
const PAST_THE_SLIDER: f32 = 300.0;

/// How far inside a row's left edge a press is made on its label.
const INSIDE_THE_ROW: f32 = 8.0;

fn ctx(app: &mut App) -> FrameCtx<'_> {
    FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }
}

/// The panel over an app, with the canvas its frames are drawn on.
struct Screen {
    app: App,
    state: PracticeState,
    pixels: HeadlessCanvas,
    size: (u32, u32),
}

impl Screen {
    /// The panel on a long seven-key chart, with every number clear of both its ends.
    fn open() -> Screen {
        Screen::sized(DRAWN_SIZES[0])
    }

    fn sized(size: (u32, u32)) -> Screen {
        let mut panel = PracticePanel::new("md5".to_string(), MODE, CHART_MS, None, CHART_TOTAL);
        panel.property.start_ms = 20_000;
        panel.property.end_ms = 40_000;
        panel.property.start_gauge = 50;
        let mut screen = Screen { app: app(), state: PracticeState::new(panel, "a chart".to_string()), pixels: HeadlessCanvas::new(size.0, size.1), size };
        screen.frame();
        screen
    }

    /// One frame: the regions of the last are forgotten and the screen is drawn.
    fn frame(&mut self) {
        self.app.shared.hot.clear();
        self.state.draw(&mut ctx(&mut self.app), &mut Canvas::Headless(&mut self.pixels));
    }

    /// The place of one row of the property in the panel.
    fn at(element: PracticeElement) -> usize {
        (0..ROW_COUNT).find(|index| row_at(*index) == PracticeRow::Value(element)).unwrap_or_else(|| panic!("{element:?} is not a row of the panel"))
    }

    fn focus(&mut self, element: PracticeElement) {
        self.state.sel = Screen::at(element);
        self.frame();
    }

    fn key(&mut self, code: KeyCode) -> Transition {
        self.state.handle_key(&mut ctx(&mut self.app), KeyInput { code, pressed: true, released: false, text: None })
    }

    /// A key pressed with Shift held, and Shift let go of after it.
    fn shifted(&mut self, code: KeyCode) {
        self.key(KeyCode::ShiftLeft);
        self.key(code);
        self.state.handle_key(&mut ctx(&mut self.app), KeyInput { code: KeyCode::ShiftLeft, pressed: false, released: true, text: None });
    }

    /// One key pressed that types `text`.
    fn typed(&mut self, text: &'static str) -> Transition {
        self.state.handle_key(&mut ctx(&mut self.app), KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) })
    }

    /// Open the row to be typed into with Enter, type `typed` into it key by key, and press Enter.
    fn type_in(&mut self, element: PracticeElement, typed: &[&'static str]) {
        self.focus(element);
        assert!(matches!(self.key(KeyCode::Enter), Transition::Stay), "{element:?}: Enter on a number left the panel");
        for text in typed {
            assert!(matches!(self.typed(text), Transition::Stay));
        }
        assert!(matches!(self.key(KeyCode::Enter), Transition::Stay), "{element:?}: the Enter that stores a number left the panel");
    }

    fn press(&mut self, at: (f32, f32), button: MouseButton) -> Transition {
        self.state.handle_mouse(&mut ctx(&mut self.app), at, button, true)
    }

    fn drag(&mut self, at: (f32, f32)) {
        self.state.handle_mouse_drag(&mut ctx(&mut self.app), at);
    }

    fn let_go(&mut self, at: (f32, f32)) {
        self.state.handle_mouse(&mut ctx(&mut self.app), at, MouseButton::Left, false);
    }

    /// Where the frame drawn last drew one row of the property.
    fn row_of(&self, element: PracticeElement) -> Rect {
        hot_rect(&self.app.shared, Hot::PracticeRow(Screen::at(element))).unwrap_or_else(|| panic!("{element:?} was not drawn"))
    }

    fn slider_of(&self, element: PracticeElement) -> Rect {
        row_slider(self.row_of(element))
    }

    /// A point on the label of one row: on the row, and on neither its slider nor its number.
    fn on_the_label_of(&self, element: PracticeElement) -> (f32, f32) {
        let row = self.row_of(element);
        (row.x + INSIDE_THE_ROW, row.y + row.h * 0.5)
    }

    fn on_the_number_of(&self, element: PracticeElement) -> (f32, f32) {
        middle(row_number(self.row_of(element)))
    }

    /// The step one row that holds a number stands on.
    fn steps_of(&self, element: PracticeElement) -> i64 {
        self.state.panel().number(element).unwrap_or_else(|| panic!("{element:?} holds no number")).steps
    }

    /// The two times of the slice, in milliseconds.
    fn times(&self) -> (i32, i32) {
        (self.state.panel().property.start_ms, self.state.panel().property.end_ms)
    }

    fn reads(&self, element: PracticeElement) -> String {
        self.state.panel().value_text(element)
    }

    fn property(&self) -> crate::practice::PracticeProperty {
        self.state.panel().property.clone()
    }

    /// The colour of the frame drawn last at a point of the screen the panel is laid out for.
    fn pixel(&self, at: (f32, f32)) -> Color {
        let x = (at.0 * self.size.0 as f32 / CW as f32) as u32;
        let y = (at.1 * self.size.1 as f32 / CH as f32) as u32;
        self.pixels.pixel_at(x, y)
    }

    /// Draw a frame and save it as `<capture folder>/<name>-<size>.png`, when a folder was named.
    fn save(&mut self, name: &str) {
        self.frame();
        let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
            return;
        };
        let folder = std::path::Path::new(&folder);
        std::fs::create_dir_all(folder).expect("the capture folder can be created");
        let file = folder.join(format!("{name}-{}x{}.png", self.size.0, self.size.1));
        image::save_buffer(file, self.pixels.rgba(), self.size.0, self.size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
    }
}

fn middle(rect: Rect) -> (f32, f32) {
    (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
}

/// The pointer's place over a slider at `fraction` of the way along it: the middle of the knob.
fn along(slider: Rect, fraction: f32) -> (f32, f32) {
    middle(knob_rect(slider, fraction))
}

#[test]
fn the_cursor_starts_on_the_row_that_starts_the_slice_and_walks_every_row() {
    let mut screen = Screen::open();
    let mut seen = vec![screen.state.focused()];
    for _ in 1..ROW_COUNT {
        screen.key(KeyCode::ArrowDown);
        seen.push(screen.state.focused());
    }
    let mut expected = vec![PracticeRow::Start];
    expected.extend(PracticeElement::ALL.into_iter().map(PracticeRow::Value));
    assert_eq!(seen, expected, "down steps through the rows in order");
    screen.key(KeyCode::ArrowDown);
    assert_eq!(screen.state.focused(), PracticeRow::Start, "below the last row is the first");
    screen.key(KeyCode::ArrowUp);
    assert_eq!(screen.state.focused(), PracticeRow::Value(PracticeElement::Random), "above the first row is the last");
}

#[test]
fn escape_leaves_the_panel_without_starting_a_slice() {
    let mut screen = Screen::open();
    assert!(matches!(screen.key(KeyCode::Escape), Transition::Back));
}

#[test]
fn leaving_the_panel_writes_its_values_into_the_book() {
    let mut screen = Screen::open();
    screen.focus(PracticeElement::Freq);
    screen.key(KeyCode::ArrowRight);
    screen.state.on_exit(&mut ctx(&mut screen.app));
    assert_eq!(screen.app.shared.practice_book.get("md5").map(|saved| saved.freq), Some(101), "the chart's property is remembered as it was left");
}

/// Enter starts the slice from the row that starts it and from a row that holds a choice. With no
/// chart loaded the start must not panic; the panel reports and leaves, which is how a start is
/// told from a key that did nothing.
#[test]
fn enter_starts_the_slice_from_every_row_that_holds_no_number() {
    let mut screen = Screen::open();
    assert_eq!(screen.state.focused(), PracticeRow::Start);
    assert!(matches!(screen.key(KeyCode::Enter), Transition::Back), "Enter on the first row did not start the slice");
    assert!(matches!(screen.key(KeyCode::NumpadEnter), Transition::Back));
    for element in CHOICE_ROWS {
        let mut screen = Screen::open();
        screen.focus(element);
        let before = screen.property();
        assert!(matches!(screen.key(KeyCode::Enter), Transition::Back), "{element:?}: Enter did not start the slice");
        assert_eq!(screen.property(), before, "{element:?}: Enter moved the row");
    }
}

/// An arrow moves every row that holds a number by the smallest change it shows, and ten of them
/// with Shift held; the row stops at both its ends, and the zero key puts it back on what a fresh
/// panel holds.
#[test]
fn a_key_moves_every_number_row_by_its_smallest_change_and_ten_with_shift() {
    const NEAR_THE_END: i64 = 3;
    for element in NUMBER_ROWS {
        let mut screen = Screen::open();
        let shipped = screen.state.panel().number(element).expect("the row holds a number").shipped;
        screen.focus(element);
        let spec = screen.state.panel().number(element).expect("the row holds a number").spec;
        let start = screen.steps_of(element);
        assert!(matches!(screen.key(KeyCode::ArrowRight), Transition::Stay));
        assert_eq!(screen.steps_of(element), start + 1, "{element:?}: one press is not the smallest change");
        assert_eq!(screen.reads(element), spec.text(start + 1), "{element:?}");
        screen.shifted(KeyCode::ArrowRight);
        assert_eq!(screen.steps_of(element), start + 11, "{element:?}: Shift is not ten of them");
        screen.key(KeyCode::ArrowLeft);
        screen.shifted(KeyCode::ArrowLeft);
        assert_eq!(screen.steps_of(element), start, "{element:?}: the way back is not the way there");
        screen.key(KeyCode::ArrowRight);
        assert_eq!(screen.steps_of(element), start + 1, "{element:?}: Shift let go of still makes a step ten");

        screen.state.panel.set_number(element, spec.max - NEAR_THE_END);
        screen.shifted(KeyCode::ArrowRight);
        screen.key(KeyCode::ArrowRight);
        assert_eq!(screen.steps_of(element), spec.max, "{element:?} did not stop at its upper end");
        if element != PracticeElement::EndTime {
            screen.state.panel.set_number(element, spec.min + NEAR_THE_END);
            screen.shifted(KeyCode::ArrowLeft);
            screen.key(KeyCode::ArrowLeft);
            assert_eq!(screen.steps_of(element), spec.min, "{element:?} did not stop at its lower end");
        }

        let zero = if element == PracticeElement::Total { KeyCode::Numpad0 } else { KeyCode::Digit0 };
        assert!(matches!(screen.key(zero), Transition::Stay));
        assert_eq!(screen.steps_of(element), shipped, "{element:?}: the zero key did not put the row back");
    }
}

/// The steps the keys take, row by row, in what the property stores: a tenth of a second and a
/// second of a time, one and ten of everything else. The reference's coarse steps -- five of TOTAL,
/// five percent of the speed, two and a half seconds with its turbo key -- are not taken.
#[test]
fn the_steps_of_the_keys_are_the_finest_the_rows_show() {
    let mut screen = Screen::open();
    let before = screen.property();
    for element in NUMBER_ROWS {
        screen.focus(element);
        screen.key(KeyCode::ArrowRight);
    }
    let after = screen.property();
    assert_eq!((after.start_ms, after.end_ms), (before.start_ms + 100, before.end_ms + 100));
    assert_eq!((after.start_gauge, after.judge_rate, after.freq), (before.start_gauge + 1, before.judge_rate + 1, before.freq + 1));
    assert_eq!(after.total, before.total + 1.0);
    for element in NUMBER_ROWS {
        screen.focus(element);
        screen.shifted(KeyCode::ArrowRight);
    }
    let wide = screen.property();
    assert_eq!((wide.start_ms, wide.end_ms), (after.start_ms + 1_000, after.end_ms + 1_000));
    assert_eq!((wide.start_gauge, wide.judge_rate, wide.freq), (after.start_gauge + 10, after.judge_rate + 10, after.freq + 10));
    assert_eq!(wide.total, after.total + 10.0);
}

/// A row that holds a choice is stepped round its ring by the arrows as it always was, with Shift
/// held or not, and the zero key leaves it alone.
#[test]
fn a_choice_row_is_stepped_round_its_ring_by_the_arrows() {
    for element in CHOICE_ROWS {
        let mut screen = Screen::open();
        let mut model = screen.state.panel().clone();
        screen.focus(element);
        screen.key(KeyCode::ArrowRight);
        model.cycle(element, true);
        assert_eq!(screen.property(), model.property, "{element:?}: right is not the next of the ring");
        assert_ne!(screen.reads(element), Screen::open().reads(element), "{element:?} did not move");
        screen.shifted(KeyCode::ArrowRight);
        model.cycle(element, true);
        assert_eq!(screen.property(), model.property, "{element:?}: Shift made a step of a choice something else");
        screen.key(KeyCode::ArrowLeft);
        model.cycle(element, false);
        assert_eq!(screen.property(), model.property, "{element:?}: left is not the one before");
        assert!(matches!(screen.key(KeyCode::Digit0), Transition::Stay));
        assert_eq!(screen.property(), model.property, "{element:?}: the zero key moved a choice");
        assert!(screen.state.numbers.editing().is_none());
    }
    let mut screen = Screen::open();
    let before = screen.property();
    for code in [KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::Digit0] {
        assert!(matches!(screen.key(code), Transition::Stay));
    }
    assert_eq!(screen.property(), before, "the row that starts the slice was stepped");
}

/// Enter opens a row that holds a number to be typed into, without starting the slice or leaving
/// the panel, and Enter again stores what was typed.
#[test]
fn enter_opens_a_number_row_to_be_typed_into_and_enter_stores_it() {
    for (element, typed, steps, reads) in [
        (PracticeElement::GaugeValue, &["7", "5"][..], 75, "75"),
        (PracticeElement::JudgeRank, &["1", "2", "0", "%"][..], 120, "120"),
        (PracticeElement::Total, &["3", "1", "7"][..], 317, "317"),
        (PracticeElement::Freq, &["7", "3"][..], 73, "73"),
        (PracticeElement::StartTime, &["1", "2", ".", "3"][..], 123, "0:12.3"),
        (PracticeElement::EndTime, &["4", "5", ".", "6"][..], 456, "0:45.6"),
    ] {
        let mut screen = Screen::open();
        screen.focus(element);
        assert!(matches!(screen.key(KeyCode::Enter), Transition::Stay), "{element:?}: Enter left the panel");
        assert_eq!(screen.state.numbers.editing(), Some(element));
        let opened_on = screen.state.numbers.edit_of(element).map(|edit| (edit.text().to_string(), edit.is_all_selected()));
        let value = screen.state.panel().number(element).expect("the row holds a number");
        assert_eq!(opened_on, Some((value.spec.plain(value.steps), true)), "{element:?}: the editor did not open on the number, wholly selected");
        for text in typed {
            screen.typed(text);
        }
        assert_eq!(screen.steps_of(element), value.steps, "{element:?} moved while it was being typed into");
        assert!(matches!(screen.key(KeyCode::Enter), Transition::Stay));
        assert_eq!(screen.state.numbers.editing(), None);
        assert_eq!((screen.steps_of(element), screen.reads(element).as_str()), (steps, reads), "{element:?}");
    }
}

/// Escape while a number is being typed drops what was typed and leaves the panel up; the Escape
/// after it leaves the panel.
#[test]
fn escape_while_typing_drops_what_was_typed_and_does_not_leave_the_panel() {
    for element in NUMBER_ROWS {
        let mut screen = Screen::open();
        let before = screen.property();
        screen.focus(element);
        screen.key(KeyCode::Enter);
        screen.typed("9");
        screen.typed("9");
        assert!(matches!(screen.key(KeyCode::Escape), Transition::Stay), "{element:?}: Escape left the panel while a number was being typed");
        assert_eq!(screen.state.numbers.editing(), None);
        assert_eq!(screen.property(), before, "{element:?}: what was typed and dropped was stored");
        assert!(matches!(screen.key(KeyCode::Escape), Transition::Back));
    }
}

/// A figure typed outside what a row holds puts the row on the nearer of its two ends, and what is
/// not a number is not stored: the row says so for a while.
#[test]
fn a_typed_figure_is_held_to_the_row_and_what_is_no_number_is_refused() {
    for element in NUMBER_ROWS {
        let mut screen = Screen::open();
        let spec = screen.state.panel().number(element).expect("the row holds a number").spec;
        screen.type_in(element, &["9", "9", "9", "9", "9", "9"]);
        assert_eq!(screen.steps_of(element), spec.max, "{element:?}: a figure past the upper end did not land on it");
        if element != PracticeElement::EndTime {
            screen.type_in(element, &["-", "5"]);
            assert_eq!(screen.steps_of(element), spec.min, "{element:?}: a figure past the lower end did not land on it");
        }

        let before = screen.property();
        for typed in [&["-"][..], &["."][..], &["1", ".", "2", ".", "3"][..], &[][..]] {
            screen.focus(element);
            screen.key(KeyCode::Enter);
            screen.key(KeyCode::Backspace);
            for text in typed {
                screen.typed(text);
            }
            let now = Instant::now();
            assert!(matches!(screen.key(KeyCode::Enter), Transition::Stay));
            assert_eq!(screen.property(), before, "{element:?}: {typed:?} was stored");
            assert!(screen.state.numbers.is_refused(element, now), "{element:?}: the row does not say {typed:?} is not a number");
            assert!(!screen.state.numbers.is_refused(element, now + REFUSAL_SHOWN_FOR + REFUSAL_SHOWN_FOR), "{element:?} went on saying so");
        }
        screen.type_in(element, &["a", "b", "c"]);
        assert_eq!(screen.property(), before, "{element:?}: letters typed over the number replaced it");
    }
}

/// A time is typed as seconds or as minutes and seconds, and reads back as minutes and seconds.
#[test]
fn a_time_is_typed_as_seconds_or_as_minutes_and_seconds() {
    let mut screen = Screen::open();
    for (typed, start_ms, reads) in [
        (&["9", "0"][..], 90_000, "1:30.0"),
        (&["1", ":", "3", "0", ".", "5"][..], 90_500, "1:30.5"),
        (&["2", ":", "5"][..], 125_000, "2:05.0"),
        (&["0", ":", "0", "7", ".", "2", "6"][..], 7_300, "0:07.3"),
        (&["6", "1", ".", "2", "s"][..], 61_200, "1:01.2"),
    ] {
        screen.type_in(PracticeElement::StartTime, typed);
        assert_eq!((screen.times().0, screen.reads(PracticeElement::StartTime).as_str()), (start_ms, reads), "{typed:?}");
    }
    screen.focus(PracticeElement::StartTime);
    screen.key(KeyCode::Enter);
    assert_eq!(
        screen.state.numbers.edit_of(PracticeElement::StartTime).map(|edit| edit.text().to_string()).as_deref(),
        Some("1:01.2"),
        "a time does not open as it is written"
    );
    assert_eq!(screen.state.numbers.edit_hint(), Some(EDIT_HINT_CLOCK));
    screen.key(KeyCode::Escape);

    let before = screen.times();
    for typed in [&["1", ":"][..], &[":", "3", "0"][..], &["1", ":", "2", ":", "3"][..]] {
        screen.focus(PracticeElement::StartTime);
        screen.key(KeyCode::Enter);
        for text in typed {
            screen.typed(text);
        }
        screen.key(KeyCode::Enter);
        assert_eq!(screen.times(), before, "{typed:?} was read as a time");
    }

    screen.focus(PracticeElement::Freq);
    screen.key(KeyCode::Enter);
    assert_eq!(screen.state.numbers.edit_hint(), Some(EDIT_HINT));
    for text in ["1", ":", "5", "0"] {
        screen.typed(text);
    }
    screen.key(KeyCode::Enter);
    assert_eq!(screen.state.panel().property.freq, 150, "a colon was typed into a row that is no time");
}

/// The two times keep a second between them however they are typed: an END TIME typed below START
/// TIME stops a second above it, and a START TIME typed past END TIME carries it along.
#[test]
fn the_two_times_keep_a_second_between_them_when_they_are_typed() {
    let mut screen = Screen::open();
    screen.type_in(PracticeElement::EndTime, &["5"]);
    assert_eq!(screen.times(), (20_000, 21_000), "an END TIME typed below START TIME did not stop a second above it");
    screen.type_in(PracticeElement::EndTime, &["0", ":", "2", "0", ".", "5"]);
    assert_eq!(screen.times(), (20_000, 21_000));
    screen.type_in(PracticeElement::StartTime, &["1", ":", "0", "0"]);
    assert_eq!(screen.times(), (60_000, 61_000), "a START TIME typed past END TIME did not carry it along");
    screen.type_in(PracticeElement::StartTime, &["9", "9", ":", "0", "0"]);
    assert_eq!(screen.times(), (178_000, 179_000), "a START TIME typed past the chart did not stop where a slice may last start");
    screen.type_in(PracticeElement::EndTime, &["9", "9", ":", "0", "0"]);
    assert_eq!(screen.times(), (178_000, 181_000));
    screen.type_in(PracticeElement::StartTime, &["0"]);
    assert_eq!(screen.times(), (0, 181_000), "a START TIME brought down moved END TIME");
}

/// A key that moves the cursor stores what was typed and then moves the cursor.
#[test]
fn a_key_that_moves_the_cursor_stores_what_was_typed_first() {
    let mut screen = Screen::open();
    screen.focus(PracticeElement::Total);
    screen.key(KeyCode::Enter);
    screen.typed("3");
    screen.typed("0");
    screen.typed("0");
    assert!(matches!(screen.key(KeyCode::ArrowDown), Transition::Stay));
    assert_eq!(screen.state.panel().property.total, 300.0);
    assert_eq!((screen.state.focused(), screen.state.numbers.editing()), (PracticeRow::Value(PracticeElement::Freq), None));

    screen.key(KeyCode::Enter);
    screen.typed("7");
    for code in [KeyCode::Digit0, KeyCode::F8, KeyCode::End] {
        assert!(matches!(screen.key(code), Transition::Stay));
    }
    assert_eq!(screen.state.numbers.editing(), Some(PracticeElement::Freq), "a key the editor takes closed it");
    assert_eq!(screen.state.panel().property.freq, 100, "a key the editor takes reached the row behind it");
    screen.typed("0");
    screen.key(KeyCode::ArrowUp);
    assert_eq!((screen.state.panel().property.freq, screen.state.focused()), (70, PracticeRow::Value(PracticeElement::Total)));
}

/// A press on a slider puts its row where the press landed, and the row then goes with the pointer
/// for as long as the button is held, onto its ends when the pointer passes them and wherever on
/// the screen the pointer goes. With no button held the pointer moves nothing.
#[test]
fn a_slider_carries_its_row_with_the_pointer_until_it_is_let_go_of() {
    /// How far along its slider a row is first pressed: clear of where any row's knob stands.
    const PRESSED_AT: f32 = 0.75;
    for element in [PracticeElement::GaugeValue, PracticeElement::JudgeRank, PracticeElement::Total, PracticeElement::Freq, PracticeElement::StartTime] {
        let mut screen = Screen::open();
        let spec = screen.state.panel().number(element).expect("the row holds a number").track();
        let slider = screen.slider_of(element);
        let pressed_at = along(slider, PRESSED_AT);
        assert!(matches!(screen.press(pressed_at, MouseButton::Left), Transition::Stay));
        let under_the_press = steps_at(&spec, slider, pressed_at.0);
        assert!((under_the_press - spec.at_fraction(f64::from(PRESSED_AT))).abs() <= 1, "{element:?}: a place along the slider is not that far along the row");
        assert_eq!(screen.steps_of(element), under_the_press, "{element:?}: a press on the slider did not put the row where it landed");
        assert_eq!(screen.state.focused(), PracticeRow::Value(element), "{element:?}: the press did not take the cursor");
        assert!(screen.state.numbers.is_dragging(element));

        let mut last = spec.min;
        let mut x = slider.x - PAST_THE_SLIDER;
        while x < slider.x + slider.w + PAST_THE_SLIDER {
            screen.drag((x, slider.y));
            let now = screen.steps_of(element);
            assert!(now >= last, "{element:?} went back while the pointer went on");
            last = now;
            x += 1.0;
        }
        assert_eq!(last, spec.max, "{element:?} did not stop at its upper end");
        screen.drag((slider.x - PAST_THE_SLIDER, slider.y + PAST_THE_SLIDER));
        assert_eq!(screen.steps_of(element), spec.min, "{element:?} did not stop at its lower end, or the drag ended off the row");
        for fraction in [0.25, 0.75, 0.37] {
            screen.drag(along(slider, fraction));
            assert_eq!(screen.steps_of(element), steps_at(&spec, slider, along(slider, fraction).0), "{element:?} at {fraction}");
        }

        let held = screen.steps_of(element);
        screen.let_go(along(slider, 0.37));
        assert!(!screen.state.numbers.is_dragging(element));
        screen.drag(along(slider, 0.9));
        screen.let_go(along(slider, 0.9));
        assert_eq!(screen.steps_of(element), held, "{element:?}: the pointer moved the row with no button held on its slider");
    }
}

/// A press on the knob takes hold of it where it stands: the row does not move until the pointer
/// does, and a knob that was only pressed and let go of has changed nothing.
#[test]
fn a_knob_that_is_only_pressed_leaves_its_row_where_it_is() {
    for element in NUMBER_ROWS {
        let mut screen = Screen::open();
        let before = screen.property();
        let slider = screen.slider_of(element);
        let knob = knob_rect(slider, screen.state.panel().number(element).expect("the row holds a number").fraction());
        for across in [knob.x, knob.x + knob.w * 0.5, knob.x + knob.w - 1.0] {
            let at = (across, slider.y + slider.h * 0.5);
            assert!(matches!(screen.press(at, MouseButton::Left), Transition::Stay));
            assert!(screen.state.numbers.is_dragging(element), "{element:?}: a press on the knob did not take hold of it");
            screen.drag(at);
            screen.let_go(at);
            assert_eq!(screen.property(), before, "{element:?}: a knob pressed at {across} and let go of moved its row");
        }
        let at = (knob.x + 1.0, slider.y + slider.h * 0.5);
        screen.press(at, MouseButton::Left);
        screen.drag((at.0 + slider.w * 0.2, at.1));
        assert_ne!(screen.property(), before, "{element:?}: a knob that was pulled did not move its row");
    }
}

/// The two times keep a second between them under the pointer too: END TIME dragged down stops a
/// second above START TIME and stays there while the pointer goes on, and START TIME dragged up
/// carries END TIME with it.
#[test]
fn the_two_times_keep_a_second_between_them_when_they_are_dragged() {
    let mut screen = Screen::open();
    let end_slider = screen.slider_of(PracticeElement::EndTime);
    screen.press(along(end_slider, 0.5), MouseButton::Left);
    assert_eq!(screen.times(), (20_000, 91_000), "a press in the middle of END TIME is not the middle of its row");
    let mut last = screen.times().1;
    let mut x = end_slider.x + end_slider.w * 0.5;
    while x > end_slider.x - PAST_THE_SLIDER {
        screen.drag((x, end_slider.y));
        let (start, end) = screen.times();
        assert!(end <= last && end >= start + 1_000, "END TIME at {end} with START TIME at {start}");
        last = end;
        x -= 1.0;
    }
    assert_eq!(screen.times(), (20_000, 21_000), "END TIME did not stop a second above START TIME");
    screen.let_go((end_slider.x - PAST_THE_SLIDER, end_slider.y));

    let start_slider = screen.slider_of(PracticeElement::StartTime);
    screen.press(along(start_slider, 0.5), MouseButton::Left);
    assert_eq!(screen.times(), (89_000, 90_000), "START TIME dragged past END TIME did not carry it along");
    screen.drag(along(start_slider, 1.0));
    assert_eq!(screen.times(), (178_000, 179_000));
    screen.drag(along(start_slider, 0.0));
    assert_eq!(screen.times(), (0, 21_000), "END TIME is not back where it was when START TIME was taken hold of");
    screen.let_go(along(start_slider, 0.0));
    assert_eq!(screen.times(), (0, 21_000), "letting go of START TIME moved a time");
}

/// START TIME carried past END TIME and brought back in the same drag leaves END TIME where it was:
/// what a drag pushes out of its way it lets come back, so an end that was set is not lost to a
/// pointer that overshot. A drag that is let go of past END TIME keeps what it pushed, and the next
/// drag starts from there.
#[test]
fn start_time_dragged_past_end_time_and_back_leaves_end_time_where_it_was() {
    let mut screen = Screen::open();
    let slider = screen.slider_of(PracticeElement::StartTime);
    let stood_at = along(slider, screen.state.panel().number(PracticeElement::StartTime).expect("the row holds a number").fraction());
    screen.press(stood_at, MouseButton::Left);
    assert_eq!(screen.times(), (20_000, 40_000), "taking hold of the knob moved a time");
    screen.drag(along(slider, 0.8));
    let (start, end) = screen.times();
    assert!(start > 40_000 && end == start + 1_000, "START TIME at {start} did not carry END TIME at {end} a second ahead of it");
    screen.drag(along(slider, 0.15));
    assert_eq!(screen.times().1, 40_000, "END TIME did not come back once START TIME was clear of it");
    screen.drag(stood_at);
    assert_eq!(screen.times(), (20_000, 40_000), "the drag that came back to where it began left a time moved");
    screen.let_go(stood_at);
    assert_eq!(screen.times(), (20_000, 40_000));

    screen.press(along(slider, 0.8), MouseButton::Left);
    screen.let_go(along(slider, 0.8));
    assert_eq!(screen.times(), (start, start + 1_000), "a press past END TIME did not carry it along");
    screen.press(along(slider, 0.8), MouseButton::Left);
    screen.drag(stood_at);
    screen.let_go(stood_at);
    assert_eq!(screen.times(), (20_000, start + 1_000), "a later drag brought back an END TIME an earlier one had left pushed");
}

/// The slider of TOTAL and its arrows run over the totals charts have, so a place along the slider
/// is a total a chart could have and one pixel of it is a few of TOTAL. A figure typed into the row
/// reaches the rest of what the row may hold, and the row is stepped from there either way.
#[test]
fn the_slider_of_total_runs_over_the_totals_charts_have_and_a_typed_figure_goes_past_them() {
    /// The most of TOTAL one pixel of the slider may be.
    const MOST_PER_PIXEL: f64 = 5.0;
    let mut screen = Screen::open();
    let value = screen.state.panel().number(PracticeElement::Total).expect("the row holds a number");
    assert!(value.slider_max < value.spec.max, "the slider of TOTAL runs to the end of the row");
    let slider = screen.slider_of(PracticeElement::Total);
    let pressed = along(slider, 0.4);
    screen.press(pressed, MouseButton::Left);
    let under = screen.property().total;
    screen.drag((pressed.0 + 1.0, pressed.1));
    let beside = screen.property().total;
    assert!(beside > under && beside - under <= MOST_PER_PIXEL, "one pixel of the slider is {under} to {beside}");
    screen.drag((slider.x + slider.w + PAST_THE_SLIDER, pressed.1));
    assert_eq!(screen.steps_of(PracticeElement::Total), value.slider_max, "the slider did not stop where it ends");
    screen.let_go((slider.x + slider.w + PAST_THE_SLIDER, pressed.1));
    screen.focus(PracticeElement::Total);
    screen.key(KeyCode::ArrowRight);
    screen.shifted(KeyCode::ArrowRight);
    assert_eq!(screen.steps_of(PracticeElement::Total), value.slider_max, "an arrow carried the row past where its slider ends");

    screen.type_in(PracticeElement::Total, &["3", "0", "0", "0"]);
    assert_eq!(screen.property().total, 3_000.0, "a figure typed past the slider's end was not stored");
    assert_eq!(screen.state.panel().number(PracticeElement::Total).map(|value| value.fraction()), Some(1.0));
    screen.key(KeyCode::ArrowRight);
    assert_eq!(screen.property().total, 3_001.0, "a row past its slider's end is not stepped from where it stands");
    screen.type_in(PracticeElement::Total, &["9", "9", "9", "9", "9"]);
    assert_eq!(screen.steps_of(PracticeElement::Total), value.spec.max);
}

/// Tab moves the cursor on to the next row and Shift with it to the one before, and stores what
/// was being typed first: the keys the line of a row being typed into names do what it says.
#[test]
fn tab_moves_the_cursor_on_and_stores_what_was_typed_first() {
    let mut screen = Screen::open();
    screen.focus(PracticeElement::Total);
    assert!(matches!(screen.key(KeyCode::Tab), Transition::Stay));
    assert_eq!(screen.state.focused(), PracticeRow::Value(PracticeElement::Freq), "Tab did not move the cursor on");
    screen.shifted(KeyCode::Tab);
    assert_eq!(screen.state.focused(), PracticeRow::Value(PracticeElement::Total), "Shift and Tab did not move the cursor back");

    screen.key(KeyCode::Enter);
    for text in ["3", "0", "0"] {
        screen.typed(text);
    }
    assert!(matches!(screen.key(KeyCode::Tab), Transition::Stay));
    assert_eq!(screen.state.panel().property.total, 300.0, "Tab dropped what was typed");
    assert_eq!((screen.state.focused(), screen.state.numbers.editing()), (PracticeRow::Value(PracticeElement::Freq), None));
    screen.key(KeyCode::Enter);
    screen.typed("7");
    screen.typed("0");
    screen.shifted(KeyCode::Tab);
    assert_eq!((screen.state.panel().property.freq, screen.state.focused()), (70, PracticeRow::Value(PracticeElement::Total)));
    let before = screen.property();
    for _ in 0..ROW_COUNT {
        screen.key(KeyCode::Tab);
    }
    assert_eq!((screen.state.focused(), screen.property()), (PracticeRow::Value(PracticeElement::Total), before), "Tab round the panel changed a row");
}

/// A press on a row's number opens it to be typed into; a press of the left button anywhere else on
/// the row steps it forwards once and one of the right button steps it back once, on a row that
/// holds a number and on one that holds a choice alike. Any press stores a number being typed.
#[test]
fn a_press_on_a_row_steps_it_and_a_press_on_its_number_opens_it() {
    for element in NUMBER_ROWS {
        let mut screen = Screen::open();
        let start = screen.steps_of(element);
        assert!(matches!(screen.press(screen.on_the_label_of(element), MouseButton::Left), Transition::Stay));
        assert_eq!((screen.steps_of(element), screen.state.focused()), (start + 1, PracticeRow::Value(element)), "{element:?}: a press on the label");
        screen.press(screen.on_the_label_of(element), MouseButton::Right);
        screen.press(middle(screen.slider_of(element)), MouseButton::Right);
        screen.press(screen.on_the_number_of(element), MouseButton::Right);
        assert_eq!(screen.steps_of(element), start - 2, "{element:?}: a press of the right button is not one step back wherever on the row it lands");
        assert!(!screen.state.numbers.is_dragging(element) && screen.state.numbers.editing().is_none(), "{element:?}: the right button took hold of something");

        assert!(matches!(screen.press(screen.on_the_number_of(element), MouseButton::Left), Transition::Stay));
        assert_eq!(screen.state.numbers.editing(), Some(element), "{element:?}: a press on the number did not open it");
        assert_eq!(screen.steps_of(element), start - 2, "{element:?}: the press that opened the number moved it");
        screen.typed("6");
        screen.typed("0");
        let other = if element == PracticeElement::Freq { PracticeElement::Total } else { PracticeElement::Freq };
        let other_before = screen.steps_of(other);
        screen.press(screen.on_the_label_of(other), MouseButton::Left);
        assert_eq!(screen.state.numbers.editing(), None);
        assert_eq!(
            screen.reads(element),
            screen.state.panel().number(element).map(|value| value.spec.text(value.spec.parse("60").unwrap_or_default())).unwrap_or_default()
        );
        assert_eq!(screen.steps_of(other), other_before + 1, "{element:?}: the press that stored the number did not also step the row it landed on");
    }
    for element in CHOICE_ROWS {
        let mut screen = Screen::open();
        let mut model = screen.state.panel().clone();
        for at in [screen.on_the_label_of(element), middle(screen.slider_of(element)), screen.on_the_number_of(element)] {
            screen.press(at, MouseButton::Left);
            model.cycle(element, true);
            assert_eq!(screen.property(), model.property, "{element:?}: a press of the left button is not the next of the ring");
        }
        screen.press(screen.on_the_label_of(element), MouseButton::Right);
        model.cycle(element, false);
        assert_eq!(screen.property(), model.property, "{element:?}: a press of the right button is not the one before");
        assert!(screen.state.numbers.editing().is_none() && screen.state.numbers.dragged().is_none(), "{element:?} was taken for a number");
        assert!(matches!(screen.press(screen.on_the_label_of(element), MouseButton::Middle), Transition::Stay));
        assert_eq!(screen.property(), model.property, "{element:?}: the middle button stepped the row");
    }
}

/// The left button on the row that starts the slice starts it; the right button does not, and a
/// press off every row does nothing at all.
#[test]
fn a_press_on_the_first_row_starts_the_slice() {
    let mut screen = Screen::open();
    let start = hot_rect(&screen.app.shared, Hot::PracticeRow(0)).expect("the first row was drawn");
    let before = screen.property();
    screen.focus(PracticeElement::Freq);
    assert!(matches!(screen.press(middle(start), MouseButton::Right), Transition::Stay), "the right button started the slice");
    assert_eq!(screen.state.focused(), PracticeRow::Start, "the press did not take the cursor");
    assert!(matches!(screen.press((start.x - PAST_THE_SLIDER, start.y), MouseButton::Left), Transition::Stay), "a press off the panel did something");
    assert!(matches!(screen.press((start.x + 1.0, start.y - 4.0), MouseButton::Left), Transition::Stay));
    assert_eq!(screen.property(), before);
    assert!(matches!(screen.press(middle(start), MouseButton::Left), Transition::Back), "the left button on the first row did not start the slice");
}

/// A key pressed while a slider is held ends the drag, so the pointer does not go on carrying a
/// row the cursor has left; and leaving the panel lets go of the slider and drops a number that
/// was still being typed.
#[test]
fn a_key_ends_a_drag_and_leaving_the_panel_ends_whatever_was_under_way() {
    let mut screen = Screen::open();
    let slider = screen.slider_of(PracticeElement::Freq);
    screen.press(along(slider, 0.5), MouseButton::Left);
    let held = screen.steps_of(PracticeElement::Freq);
    screen.key(KeyCode::ArrowUp);
    assert!(!screen.state.numbers.is_dragging(PracticeElement::Freq));
    screen.drag(along(slider, 1.0));
    assert_eq!(screen.steps_of(PracticeElement::Freq), held, "the pointer carried a row after a key ended its drag");

    screen.press(along(slider, 0.2), MouseButton::Left);
    screen.focus(PracticeElement::Total);
    screen.key(KeyCode::Enter);
    screen.typed("9");
    let before = screen.property();
    screen.state.on_exit(&mut ctx(&mut screen.app));
    assert!(screen.state.numbers.editing().is_none() && screen.state.numbers.dragged().is_none());
    assert_eq!(screen.app.shared.practice_book.get("md5"), Some(before), "a number that was still being typed was stored on the way out");
}

/// The wheel moves nothing on the panel, over a slider or anywhere else.
#[test]
fn the_wheel_moves_no_row() {
    let mut screen = Screen::open();
    let before = screen.property();
    screen.app.shared.cursor = middle(screen.slider_of(PracticeElement::Freq));
    for lines in [1.0, -1.0, 12.0] {
        assert!(matches!(screen.state.handle_scroll(&mut ctx(&mut screen.app), lines), Transition::Stay));
    }
    assert_eq!((screen.property(), screen.state.focused()), (before, PracticeRow::Start));
}

/// What the rows are set to on the screen is what a slice is started with.
#[test]
fn what_is_set_on_the_panel_is_what_a_slice_is_started_with() {
    let mut screen = Screen::open();
    screen.type_in(PracticeElement::StartTime, &["0", ":", "1", "2", ".", "3"]);
    screen.type_in(PracticeElement::EndTime, &["4", "5", ".", "6"]);
    screen.type_in(PracticeElement::GaugeValue, &["4", "0"]);
    screen.type_in(PracticeElement::JudgeRank, &["8", "3"]);
    screen.type_in(PracticeElement::Total, &["3", "1", "7"]);
    screen.type_in(PracticeElement::Freq, &["7", "3"]);
    let session = screen.state.panel().clone().start();
    assert_eq!((session.start_us, session.end_us), (12_300_000, 45_600_000));
    assert_eq!((session.start_gauge, session.judge_rate_percent, session.freq_percent, session.total), (40.0, 83, 73, Some(317.0)));
}

/// The panel is laid out on the same screen whatever size its target is: every row is recorded
/// where it is drawn, a slider and a number are inside their row and clear of one another and of
/// the label, and the knob is painted where the row says it is, on a target that is sixteen to nine
/// and on one that is not.
#[test]
fn the_rows_are_where_the_pointer_finds_them_at_every_size() {
    let laid_out: Vec<Rect> = (0..ROW_COUNT).map(row_rect).collect();
    for size in DRAWN_SIZES {
        let mut screen = Screen::sized(size);
        let drawn: Vec<Rect> = (0..ROW_COUNT).map(|index| hot_rect(&screen.app.shared, Hot::PracticeRow(index)).expect("the row was drawn")).collect();
        assert_eq!(drawn, laid_out, "{size:?}: the rows are recorded somewhere else than at another size");
        for (index, row) in drawn.iter().enumerate() {
            assert_eq!(screen.app.shared.hit_test(middle(*row)), Some(Hot::PracticeRow(index)), "{size:?}");
        }
        assert!(drawn.last().is_some_and(|last| last.y + last.h < CH as f32), "{size:?}: the last row is off the screen");
        for element in NUMBER_ROWS {
            screen.focus(element);
            let (row, slider, number) = (screen.row_of(element), screen.slider_of(element), row_number(screen.row_of(element)));
            assert!(slider.x >= row.x + LABEL_INSET + text_width(element.label(), ROW_SCALE), "{size:?} {element:?}: the slider is under the label");
            assert!(slider.x + slider.w <= number.x && number.x + number.w <= row.x + row.w, "{size:?} {element:?}: the slider and the number overlap");
            assert!(text_width(&screen.reads(element), ROW_SCALE) <= NUMBER_W, "{size:?} {element:?}: the number is wider than its room");
            let knob = knob_rect(slider, screen.state.panel().number(element).expect("the row holds a number").fraction());
            assert_eq!(screen.pixel(middle(knob)), theme().focus, "{size:?} {element:?}: the knob of the row under the cursor is not where the row says");
            let above = (knob.x + knob.w * 0.5, slider.y + 1.0);
            assert_ne!(screen.pixel(above), theme().focus, "{size:?} {element:?}: the knob is as tall as its row");
        }
        for hint in [HINT_KEYS, HINT_MOUSE, EDIT_HINT, EDIT_HINT_CLOCK] {
            assert!(text_width(hint, HINT_SCALE) <= CW as f32 - row_rect(0).x, "{size:?}: {hint:?} runs off the screen");
        }
    }
}

/// The frames a person looks at: the whole panel with its sliders, a slider being dragged, a time
/// being typed, a figure that was refused, and an END TIME brought down onto START TIME.
#[test]
fn the_panel_is_drawn_with_its_sliders_and_captured() {
    for size in CAPTURED_SIZES {
        let mut screen = Screen::sized(size);
        screen.focus(PracticeElement::EndTime);
        screen.save("practice-panel");
        assert_eq!(
            screen.pixel(middle(knob_rect(
                screen.slider_of(PracticeElement::EndTime),
                screen.state.panel().number(PracticeElement::EndTime).map_or(0.0, |value| value.fraction())
            ))),
            theme().focus
        );

        let slider = screen.slider_of(PracticeElement::Freq);
        screen.press(along(slider, 0.2), MouseButton::Left);
        screen.drag(along(slider, 0.7));
        screen.save("practice-dragging");
        assert!(screen.state.numbers.is_dragging(PracticeElement::Freq));
        screen.let_go(along(slider, 0.7));

        screen.focus(PracticeElement::StartTime);
        screen.key(KeyCode::Enter);
        screen.save("practice-typing-opened");
        for text in ["1", ":", "0", "5", ".", "5"] {
            screen.typed(text);
        }
        screen.save("practice-typing-a-time");
        assert_eq!(screen.state.numbers.edit_of(PracticeElement::StartTime).map(|edit| edit.text().to_string()).as_deref(), Some("1:05.5"));
        screen.key(KeyCode::Enter);
        assert_eq!(screen.times(), (65_500, 66_500));

        screen.focus(PracticeElement::Total);
        screen.key(KeyCode::Enter);
        screen.typed("-");
        screen.key(KeyCode::Enter);
        screen.save("practice-refused");

        let end_slider = screen.slider_of(PracticeElement::EndTime);
        screen.press(along(end_slider, 0.9), MouseButton::Left);
        screen.drag(along(end_slider, 0.05));
        screen.save("practice-end-held-above-start");
        assert_eq!(screen.times(), (65_500, 66_500), "END TIME was dragged below START TIME");
        screen.let_go(along(end_slider, 0.05));
    }
}
